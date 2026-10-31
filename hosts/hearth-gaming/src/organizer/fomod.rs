//! FOMOD installers (fomod/ModuleConfig.xml and info.xml), the way MO2's installer_fomod reads them: install steps,
//! their groups and plugins, plugin types (Required, Recommended, Optional, NotUsable, CouldBeUsable, fixed or by
//! pattern), condition flags, file and flag dependencies, conditional file installs, and file priorities.
//!
//! The parsed installer is data the UI renders (with a controller: one step at a time, A to pick); the UI sends back
//! its choices as `[[[plugin indexes] per group] per step]` and `evaluate` says which steps show, what each plugin's
//! type is with those choices, what's wrong, and which files go where. Unset choices take the installer's defaults.

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cond {
    /// and, or, file, flag, game, fomm, other.
    pub op: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Cond>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub file: String,
    /// Active, Inactive or Missing.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub state: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub flag: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileOp { pub source: String, pub destination: String, pub folder: bool, pub priority: i64 }

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plugin {
    pub name: String,
    pub description: String,
    /// The picture, as a path inside the installer and as a file on disk (once unpacked).
    pub image: String,
    pub image_path: String,
    pub files: Vec<FileOp>,
    pub flags: Vec<(String, String)>,
    /// The type when no pattern matches, and the patterns (first match wins).
    pub default_type: String,
    pub patterns: Vec<(Cond, String)>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub name: String,
    /// SelectExactlyOne, SelectAtMostOne, SelectAtLeastOne, SelectAll, SelectAny.
    #[serde(rename = "type")]
    pub kind: String,
    pub plugins: Vec<Plugin>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Step { pub name: String, pub visible: Option<Cond>, pub groups: Vec<Group> }

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CondFiles { pub when: Cond, pub files: Vec<FileOp> }

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info { pub name: String, pub author: String, pub version: String, pub website: String, pub description: String }

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fomod {
    pub name: String,
    pub image: String,
    pub info: Info,
    pub requires: Option<Cond>,
    pub required: Vec<FileOp>,
    pub steps: Vec<Step>,
    pub conditional: Vec<CondFiles>,
}

// ------------------------------------------------------------------ reading

/// The XML's text: UTF-16 (with or without its byte-order mark) or UTF-8, without the <?xml?> line.
pub fn decode_xml(b: &[u8]) -> String {
    let utf16 = |b: &[u8], le: bool| String::from_utf16_lossy(&b.chunks_exact(2).map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) }).collect::<Vec<_>>());
    let s = if b.starts_with(&[0xFF, 0xFE]) { utf16(&b[2..], true) }
        else if b.starts_with(&[0xFE, 0xFF]) { utf16(&b[2..], false) }
        else if b.starts_with(&[0xEF, 0xBB, 0xBF]) { String::from_utf8_lossy(&b[3..]).to_string() }
        else if b.len() >= 2 && b[0] != 0 && b[1] == 0 { utf16(b, true) }
        else if b.len() >= 2 && b[0] == 0 && b[1] != 0 { utf16(b, false) }
        else { String::from_utf8_lossy(b).to_string() };
    let t = s.trim_start_matches('\u{feff}').trim_start();
    if t.starts_with("<?xml") { if let Some(i) = t.find("?>") { return t[i + 2..].to_string(); } }
    t.to_string()
}

fn kids<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Vec<roxmltree::Node<'a, 'i>> {
    n.children().filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case(name)).collect()
}
fn kid<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> { kids(n, name).into_iter().next() }
fn text_of(n: Option<roxmltree::Node>) -> String { n.and_then(|x| x.text()).unwrap_or("").trim().to_string() }
fn attr(n: roxmltree::Node, name: &str) -> String { n.attributes().find(|a| a.name().eq_ignore_ascii_case(name)).map(|a| a.value().to_string()).unwrap_or_default() }
fn attr_opt(n: roxmltree::Node, name: &str) -> Option<String> { n.attributes().find(|a| a.name().eq_ignore_ascii_case(name)).map(|a| a.value().to_string()) }

fn parse_doc(text: &str) -> Result<roxmltree::Document<'_>, String> {
    let opt = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    roxmltree::Document::parse_with_options(text, opt).map_err(|e| format!("The installer's ModuleConfig.xml can't be read ({e})"))
}

fn parse_cond(n: roxmltree::Node) -> Cond {
    let op = attr_opt(n, "operator").unwrap_or_else(|| "And".into()).to_lowercase();
    let children = n.children().filter(|c| c.is_element()).map(|c| match c.tag_name().name().to_ascii_lowercase().as_str() {
        "filedependency" => Cond { op: "file".into(), file: attr(c, "file"), state: attr(c, "state"), ..Default::default() },
        "flagdependency" => Cond { op: "flag".into(), flag: attr(c, "flag"), value: attr(c, "value"), ..Default::default() },
        "gamedependency" => Cond { op: "game".into(), value: attr(c, "version"), ..Default::default() },
        "fommdependency" => Cond { op: "fomm".into(), value: attr(c, "version"), ..Default::default() },
        "dependencies" => parse_cond(c),
        _ => Cond { op: "other".into(), ..Default::default() },
    }).collect();
    Cond { op: if op == "or" { "or".into() } else { "and".into() }, children, ..Default::default() }
}

fn parse_files(n: Option<roxmltree::Node>) -> Vec<FileOp> {
    let Some(n) = n else { return Vec::new() };
    n.children().filter(|c| c.is_element()).filter_map(|c| {
        let folder = match c.tag_name().name().to_ascii_lowercase().as_str() { "file" => false, "folder" => true, _ => return None };
        let source = attr(c, "source").replace('\\', "/").trim_matches('/').to_string();
        let destination = match attr_opt(c, "destination") {
            None => source.clone(),
            Some(d) => {
                let d = d.replace('\\', "/").trim_matches('/').to_string();
                // An empty destination for a file: the file at the top, under its own name.
                if d.is_empty() && !folder { source.rsplit('/').next().unwrap_or("").to_string() } else { d }
            }
        };
        Some(FileOp { source, destination, folder, priority: attr(c, "priority").trim().parse().unwrap_or(0) })
    }).collect()
}

/// installSteps, optionalFileGroups and plugins are sorted by name unless their order says Explicit (the schema's
/// default is Ascending, and MO2 follows it).
fn order<T>(v: &mut [T], how: &str, name: impl Fn(&T) -> String) {
    match how.to_ascii_lowercase().as_str() {
        "explicit" => {}
        "descending" => v.sort_by_key(|x| std::cmp::Reverse(name(x).to_lowercase())),
        _ => v.sort_by_key(|x| name(x).to_lowercase()),
    }
}

/// Reads ModuleConfig.xml (and info.xml, if given). `root`: the folder the installer's paths start from, for the
/// pictures' files.
pub fn parse(config: &[u8], info: Option<&[u8]>, root: Option<&Path>) -> Result<Fomod, String> {
    let text = decode_xml(config);
    let doc = parse_doc(&text)?;
    let c = doc.root_element();
    if !c.tag_name().name().eq_ignore_ascii_case("config") { return Err("That isn't a FOMOD installer (no <config>).".into()); }
    let image_path = |p: &str| if p.is_empty() { String::new() } else { root.and_then(|r| super::vfs::ci_resolve(r, p)).map(|x| x.to_string_lossy().to_string()).unwrap_or_default() };
    let mut f = Fomod {
        name: text_of(kid(c, "moduleName")),
        image: kid(c, "moduleImage").map(|i| attr(i, "path")).unwrap_or_default(),
        requires: kid(c, "moduleDependencies").map(parse_cond),
        required: parse_files(kid(c, "requiredInstallFiles")),
        ..Default::default()
    };
    if let Some(steps) = kid(c, "installSteps") {
        for s in kids(steps, "installStep") {
            let mut step = Step { name: attr(s, "name"), visible: kid(s, "visible").map(|v| match kid(v, "dependencies") { Some(d) if v.children().filter(|x| x.is_element()).count() == 1 => parse_cond(d), _ => parse_cond(v) }), groups: Vec::new() };
            if let Some(groups) = kid(s, "optionalFileGroups") {
                for g in kids(groups, "group") {
                    let mut group = Group { name: attr(g, "name"), kind: attr(g, "type"), plugins: Vec::new() };
                    if let Some(pl) = kid(g, "plugins") {
                        for p in kids(pl, "plugin") {
                            let img = kid(p, "image").map(|i| attr(i, "path").replace('\\', "/")).unwrap_or_default();
                            let mut plugin = Plugin { name: attr(p, "name"), description: text_of(kid(p, "description")), image_path: image_path(&img), image: img,
                                files: parse_files(kid(p, "files")), default_type: "Optional".into(), ..Default::default() };
                            if let Some(cf) = kid(p, "conditionFlags") { plugin.flags = kids(cf, "flag").into_iter().map(|x| (attr(x, "name"), x.text().unwrap_or("").trim().to_string())).collect(); }
                            if let Some(td) = kid(p, "typeDescriptor") {
                                if let Some(t) = kid(td, "type") { plugin.default_type = attr(t, "name"); }
                                if let Some(dt) = kid(td, "dependencyType") {
                                    if let Some(d) = kid(dt, "defaultType") { plugin.default_type = attr(d, "name"); }
                                    if let Some(ps) = kid(dt, "patterns") {
                                        plugin.patterns = kids(ps, "pattern").into_iter().map(|pt| (kid(pt, "dependencies").map(parse_cond).unwrap_or_default(), kid(pt, "type").map(|t| attr(t, "name")).unwrap_or_default())).collect();
                                    }
                                }
                            }
                            group.plugins.push(plugin);
                        }
                        order(&mut group.plugins, &attr(pl, "order"), |p| p.name.clone());
                    }
                    step.groups.push(group);
                }
                order(&mut step.groups, &attr(groups, "order"), |g| g.name.clone());
            }
            f.steps.push(step);
        }
        order(&mut f.steps, &attr(steps, "order"), |s| s.name.clone());
    }
    if let Some(cfi) = kid(c, "conditionalFileInstalls") {
        if let Some(ps) = kid(cfi, "patterns") {
            f.conditional = kids(ps, "pattern").into_iter().map(|pt| CondFiles { when: kid(pt, "dependencies").map(parse_cond).unwrap_or_default(), files: parse_files(kid(pt, "files")) }).collect();
        }
    }
    if let Some(info) = info {
        let t = decode_xml(info);
        if let Ok(d) = parse_doc(&t) {
            let r = d.root_element();
            f.info = Info { name: text_of(kid(r, "Name")), author: text_of(kid(r, "Author")), version: text_of(kid(r, "Version")), website: text_of(kid(r, "Website")), description: text_of(kid(r, "Description")) };
        }
    }
    if f.name.is_empty() { f.name = f.info.name.clone(); }
    Ok(f)
}

/// fomod/ModuleConfig.xml in an unpacked mod (at its top or up to two folders down): the folder its paths start
/// from, the config and info.xml.
pub fn find(dir: &Path) -> Option<(PathBuf, PathBuf, Option<PathBuf>)> {
    fn look(d: &Path, depth: usize) -> Option<(PathBuf, PathBuf, Option<PathBuf>)> {
        if let Some(fd) = super::vfs::ci_find(d, "fomod").filter(|p| p.is_dir()) {
            if let Some(cfg) = super::vfs::ci_find(&fd, "ModuleConfig.xml").filter(|p| p.is_file()) {
                return Some((d.to_path_buf(), cfg, super::vfs::ci_find(&fd, "info.xml").filter(|p| p.is_file())));
            }
        }
        if depth == 0 { return None; }
        let subs: Vec<PathBuf> = std::fs::read_dir(d).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        if subs.len() != 1 { return None; }
        look(&subs[0], depth - 1)
    }
    look(dir, 2)
}

// ------------------------------------------------------------------ choosing

pub type Choices = Vec<Vec<Vec<usize>>>;

/// What a plugin or file is in the game now: Active, Inactive (there, off) or Missing.
pub type FileState<'a> = &'a dyn Fn(&str) -> &'static str;

pub fn eval(c: &Cond, flags: &BTreeMap<String, String>, files: FileState) -> bool {
    match c.op.as_str() {
        "and" => c.children.iter().all(|x| eval(x, flags, files)),
        "or" => c.children.is_empty() || c.children.iter().any(|x| eval(x, flags, files)),
        "file" => files(&c.file).eq_ignore_ascii_case(&c.state),
        "flag" => flags.get(&c.flag).map(|s| s.as_str()).unwrap_or("") == c.value,
        _ => true,
    }
}

pub fn plugin_type(p: &Plugin, flags: &BTreeMap<String, String>, files: FileState) -> String {
    p.patterns.iter().find(|(c, _)| eval(c, flags, files)).map(|(_, t)| t.clone()).unwrap_or_else(|| p.default_type.clone())
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupEval { pub selected: Vec<usize>, pub types: Vec<String>, #[serde(skip_serializing_if = "String::is_empty")] pub problem: String }
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepEval { pub visible: bool, pub groups: Vec<GroupEval> }
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Eval {
    pub steps: Vec<StepEval>,
    pub flags: BTreeMap<String, String>,
    /// What stops the install (a group needing a choice, the mod's own requirements).
    pub errors: Vec<String>,
    /// The files to install, lowest priority first (later ones replace earlier ones).
    pub files: Vec<FileOp>,
    /// The choices as they stand (defaults filled in), to send back.
    pub choices: Choices,
}

/// Choices from the UI's JSON ([[[0], [1, 2]], ...]); anything else is "no choices".
pub fn choices_from(v: &serde_json::Value) -> Choices {
    v.as_array().map(|steps| steps.iter().map(|g| g.as_array().map(|gs| gs.iter().map(|p| p.as_array().map(|ps| ps.iter().filter_map(|x| x.as_u64().map(|n| n as usize)).collect()).unwrap_or_default()).collect()).unwrap_or_default()).collect()).unwrap_or_default()
}

pub fn evaluate(f: &Fomod, choices: &Choices, files: FileState) -> Eval {
    let mut e = Eval::default();
    if let Some(r) = &f.requires { if !eval(r, &e.flags, files) { e.errors.push("This mod's installer says something it needs isn't installed (or something it can't work with is).".into()); } }
    let mut picked: Vec<&Plugin> = Vec::new();
    for (si, step) in f.steps.iter().enumerate() {
        let visible = step.visible.as_ref().map(|c| eval(c, &e.flags, files)).unwrap_or(true);
        let mut se = StepEval { visible, groups: Vec::new() };
        let mut step_choices = Vec::new();
        for (gi, g) in step.groups.iter().enumerate() {
            let types: Vec<String> = g.plugins.iter().map(|p| plugin_type(p, &e.flags, files)).collect();
            let usable = |i: usize| !types[i].eq_ignore_ascii_case("NotUsable");
            let mut ge = GroupEval { types: types.clone(), ..Default::default() };
            if !visible { se.groups.push(ge); step_choices.push(Vec::new()); continue; }
            let given = choices.get(si).and_then(|s| s.get(gi));
            let mut sel: Vec<usize> = match given {
                Some(v) => { let mut out = Vec::new(); for &i in v { if i < g.plugins.len() && !out.contains(&i) { out.push(i); } } out }
                None => {
                    let mut d: Vec<usize> = (0..g.plugins.len()).filter(|&i| types[i].eq_ignore_ascii_case("Recommended")).collect();
                    let one = g.kind.eq_ignore_ascii_case("SelectExactlyOne") || g.kind.eq_ignore_ascii_case("SelectAtMostOne");
                    if one { d.truncate(1); }
                    if d.is_empty() && (g.kind.eq_ignore_ascii_case("SelectExactlyOne") || g.kind.eq_ignore_ascii_case("SelectAtLeastOne")) {
                        if let Some(i) = (0..g.plugins.len()).find(|&i| usable(i)) { d.push(i); }
                    }
                    d
                }
            };
            // Required plugins are always in; SelectAll takes them all; NotUsable ones can't be.
            for i in 0..g.plugins.len() { if (types[i].eq_ignore_ascii_case("Required") || g.kind.eq_ignore_ascii_case("SelectAll")) && usable(i) && !sel.contains(&i) { sel.push(i); } }
            if sel.iter().any(|&i| !usable(i)) { ge.problem = "Some of these can't be used here.".into(); sel.retain(|&i| usable(i)); }
            sel.sort_unstable();
            let n = sel.len();
            let k = g.kind.to_ascii_lowercase();
            let any_usable = (0..g.plugins.len()).any(usable);
            let bad = match k.as_str() { "selectexactlyone" => n != 1 && any_usable, "selectatmostone" => n > 1, "selectatleastone" => n == 0 && any_usable, _ => false };
            if bad { ge.problem = match k.as_str() { "selectexactlyone" => "Choose one.".into(), "selectatmostone" => "Choose one at most.".into(), _ => "Choose at least one.".into() }; e.errors.push(format!("{}: {}", if g.name.is_empty() { &step.name } else { &g.name }, ge.problem)); }
            for &i in &sel { let p = &g.plugins[i]; for (k, v) in &p.flags { e.flags.insert(k.clone(), v.clone()); } picked.push(p); }
            step_choices.push(sel.clone());
            ge.selected = sel;
            se.groups.push(ge);
        }
        e.choices.push(step_choices);
        e.steps.push(se);
    }
    let mut ops: Vec<FileOp> = f.required.clone();
    for p in picked { ops.extend(p.files.iter().cloned()); }
    for c in &f.conditional { if eval(&c.when, &e.flags, files) { ops.extend(c.files.iter().cloned()); } }
    ops.sort_by_key(|o| o.priority);
    e.files = ops;
    e
}

/// Puts the chosen files into `dest` (a new mod's folder). Paths in the installer are matched ignoring case, and
/// files that land on the same path ignoring case become one. Returns how many files were put in, and the problems.
pub fn apply(files: &[FileOp], root: &Path, dest: &Path) -> (usize, Vec<String>) {
    let mut n = 0;
    let mut problems = Vec::new();
    std::fs::create_dir_all(dest).ok();
    for op in files {
        let Some(src) = super::vfs::ci_resolve(root, &op.source) else { problems.push(format!("The installer names {} but it isn't in the mod.", op.source)); continue };
        let dst_rel = if op.destination.is_empty() { None } else { match crate::util::clean_rel(&op.destination) { Some(r) => Some(r), None => { problems.push(format!("{} would go outside the mod's folder; left out.", op.destination)); continue } } };
        let base = match &dst_rel { Some(r) => r.to_string_lossy().to_string(), None => String::new() };
        if src.is_dir() {
            let mut all = Vec::new();
            crate::util::files_under(&src, &mut all);
            for f in all {
                let Ok(r) = f.strip_prefix(&src) else { continue };
                let rel = if base.is_empty() { r.to_string_lossy().to_string() } else { format!("{base}/{}", r.to_string_lossy()) };
                let to = super::vfs::ci_join(dest, &rel);
                match crate::mods::copy_file(&f, &to) { Ok(_) => n += 1, Err(e) => problems.push(format!("{rel}: {e}")) }
            }
        } else if src.is_file() {
            if base.is_empty() { problems.push(format!("{} has nowhere to go.", op.source)); continue; }
            let to = super::vfs::ci_join(dest, &base);
            match crate::mods::copy_file(&src, &to) { Ok(_) => n += 1, Err(e) => problems.push(format!("{base}: {e}")) }
        }
    }
    (n, problems)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub const CONFIG: &str = r#"<?xml version="1.0" encoding="UTF-16"?>
<config xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="http://qconsulting.ca/fo3/ModConfig5.0.xsd">
  <moduleName>Test Weathers</moduleName>
  <moduleDependencies operator="And"><fileDependency file="Skyrim.esm" state="Active"/></moduleDependencies>
  <requiredInstallFiles><folder source="00 Core" destination=""/><file source="readme.txt" destination="docs/readme.txt"/></requiredInstallFiles>
  <installSteps order="Explicit">
    <installStep name="Main">
      <optionalFileGroups order="Explicit">
        <group name="Weather style" type="SelectExactlyOne">
          <plugins order="Explicit">
            <plugin name="Bright"><description>Brighter skies</description><image path="fomod\bright.png"/>
              <files><file source="opt\Bright.esp" destination="TestWeathers.esp" priority="1"/></files>
              <conditionFlags><flag name="style">bright</flag></conditionFlags>
              <typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
            <plugin name="Dark"><description>Darker nights</description>
              <files><file source="opt\Dark.esp" destination="TestWeathers.esp" priority="1"/></files>
              <conditionFlags><flag name="style">dark</flag></conditionFlags>
              <typeDescriptor><type name="Recommended"/></typeDescriptor></plugin>
          </plugins>
        </group>
        <group name="Patches" type="SelectAny">
          <plugins>
            <plugin name="Z Patch"><description/><files><file source="opt/z.esp"/></files>
              <typeDescriptor><dependencyType><defaultType name="NotUsable"/><patterns>
                <pattern><dependencies operator="And"><fileDependency file="Dawnguard.esm" state="Active"/></dependencies><type name="Recommended"/></pattern>
              </patterns></dependencyType></typeDescriptor></plugin>
            <plugin name="A Patch"><description/><files><file source="opt/a.esp"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
    <installStep name="Dark extras">
      <visible><flagDependency flag="style" value="dark"/></visible>
      <optionalFileGroups><group name="Extras" type="SelectAll"><plugins><plugin name="Stars"><description/><files><folder source="opt/stars" destination="textures/sky"/></files><typeDescriptor><type name="Required"/></typeDescriptor></plugin></plugins></group></optionalFileGroups>
    </installStep>
  </installSteps>
  <conditionalFileInstalls><patterns>
    <pattern><dependencies operator="Or"><flagDependency flag="style" value="bright"/><flagDependency flag="style" value="sunny"/></dependencies><files><file source="opt/bright.ini" destination="TestWeathers.ini"/></files></pattern>
  </patterns></conditionalFileInstalls>
</config>"#;

    fn utf16(s: &str) -> Vec<u8> { let mut b = vec![0xFF, 0xFE]; for u in s.encode_utf16() { b.extend_from_slice(&u.to_le_bytes()); } b }

    #[test]
    fn parses_steps_groups_and_types() {
        let f = parse(&utf16(CONFIG), Some(b"<fomod><Name>Test Weathers</Name><Author>Dragongamer9</Author><Version>1.0</Version></fomod>"), None).unwrap();
        assert_eq!((f.name.as_str(), f.info.author.as_str()), ("Test Weathers", "Dragongamer9"));
        assert_eq!(f.required, vec![FileOp { source: "00 Core".into(), destination: "".into(), folder: true, priority: 0 }, FileOp { source: "readme.txt".into(), destination: "docs/readme.txt".into(), folder: false, priority: 0 }]);
        assert_eq!(f.steps.len(), 2);
        let g = &f.steps[0].groups;
        assert_eq!((g[0].name.as_str(), g[0].kind.as_str()), ("Weather style", "SelectExactlyOne"));
        assert_eq!(g[0].plugins[0].files[0], FileOp { source: "opt/Bright.esp".into(), destination: "TestWeathers.esp".into(), folder: false, priority: 1 });
        assert_eq!(g[0].plugins[1].flags, vec![("style".to_string(), "dark".to_string())]);
        // No order given: plugins sorted by name (the schema's default).
        assert_eq!(g[1].plugins.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["A Patch", "Z Patch"]);
        assert_eq!(g[1].plugins[1].patterns.len(), 1);
        assert_eq!(g[1].plugins[0].files[0].destination, "opt/a.esp", "no destination: the source path");
        assert_eq!(f.steps[1].visible.as_ref().unwrap().children[0], Cond { op: "flag".into(), flag: "style".into(), value: "dark".into(), ..Default::default() });
        assert_eq!(f.conditional[0].when.op, "or");
    }

    #[test]
    fn choices_conditions_and_files() {
        let f = parse(CONFIG.as_bytes(), None, None).unwrap();
        let skyrim_only = |n: &str| if n.eq_ignore_ascii_case("Skyrim.esm") { "Active" } else { "Missing" };
        // Defaults: Dark is recommended; the Dawnguard patch isn't usable; the dark step shows with its Required plugin.
        let e = evaluate(&f, &Vec::new(), &skyrim_only);
        assert!(e.errors.is_empty(), "{:?}", e.errors);
        assert_eq!(e.steps[0].groups[0].selected, vec![1]);
        assert_eq!(e.steps[0].groups[1].types, vec!["Optional", "NotUsable"]);
        assert!(e.steps[1].visible);
        assert_eq!(e.steps[1].groups[0].selected, vec![0]);
        assert_eq!(e.flags.get("style").map(|s| s.as_str()), Some("dark"));
        assert!(e.files.iter().any(|o| o.source == "opt/stars"));
        assert!(!e.files.iter().any(|o| o.source == "opt/bright.ini"));
        assert_eq!(e.choices, vec![vec![vec![1], vec![]], vec![vec![0]]]);
        // Bright: the dark step hides, the conditional ini comes in; asking for the unusable patch is refused.
        let e = evaluate(&f, &vec![vec![vec![0], vec![0, 1]]], &skyrim_only);
        assert!(!e.steps[1].visible);
        assert_eq!(e.steps[0].groups[1].selected, vec![0]);
        assert!(!e.steps[0].groups[1].problem.is_empty());
        assert!(e.files.iter().any(|o| o.source == "opt/bright.ini") && !e.files.iter().any(|o| o.source == "opt/stars"));
        // Priorities: the plugin's priority-1 file comes after the priority-0 ones.
        assert_eq!(e.files.last().unwrap().priority, 1);
        // With Dawnguard the patch is recommended; choosing none in an exactly-one group is an error.
        let with_dg = |n: &str| if n.eq_ignore_ascii_case("Skyrim.esm") || n.eq_ignore_ascii_case("Dawnguard.esm") { "Active" } else { "Missing" };
        let e = evaluate(&f, &vec![vec![vec![], vec![1]]], &with_dg);
        assert_eq!(e.steps[0].groups[1].types[1], "Recommended");
        assert_eq!(e.errors.len(), 1, "{:?}", e.errors);
        // The mod's own requirement: Skyrim.esm active.
        let none = |_: &str| "Missing";
        assert!(evaluate(&f, &Vec::new(), &none).errors.iter().any(|x| x.contains("needs")));
        assert_eq!(choices_from(&serde_json::json!([[[1], []], [[0]]])), vec![vec![vec![1], vec![]], vec![vec![0]]]);
    }

    #[test]
    fn applies_files_case_insensitively() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("x");
        let put = |p: &str, s: &str| { let f = root.join(p); std::fs::create_dir_all(f.parent().unwrap()).unwrap(); std::fs::write(f, s).unwrap(); };
        put("00 Core/Textures/a.dds", "core a");
        put("00 Core/meshes/m.nif", "core m");
        put("readme.txt", "read me");
        put("Opt/dark.esp", "dark");
        put("Opt/Stars/s1.dds", "s1");
        put("fomod/ModuleConfig.xml", CONFIG);
        let (froot, cfg, info) = find(t.path()).unwrap();
        assert_eq!(froot, root);
        assert!(info.is_none());
        let f = parse(&std::fs::read(cfg).unwrap(), None, Some(&froot)).unwrap();
        let e = evaluate(&f, &Vec::new(), &|n: &str| if n == "Skyrim.esm" { "Active" } else { "Missing" });
        let dest = t.path().join("mod");
        let (n, problems) = apply(&e.files, &froot, &dest);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(n, 5);
        assert_eq!(std::fs::read_to_string(dest.join("TestWeathers.esp")).unwrap(), "dark");
        assert_eq!(std::fs::read_to_string(dest.join("Textures/sky/s1.dds")).unwrap(), "s1", "textures/sky joins the core's Textures folder");
        assert_eq!(std::fs::read_to_string(dest.join("docs/readme.txt")).unwrap(), "read me");
        assert!(dest.join("meshes/m.nif").is_file());
        let bad = vec![FileOp { source: "readme.txt".into(), destination: "../escape.txt".into(), folder: false, priority: 0 }];
        let (n, p) = apply(&bad, &froot, &dest);
        assert_eq!(n, 0);
        assert_eq!(p.len(), 1);
        assert!(!t.path().join("escape.txt").exists());
    }
}

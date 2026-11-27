//! The community's installers inside Hearth (board 08, Get a program): Bottles' programs list (bottlesdevs/programs,
//! GPL-3) says, for each program people checked, what to put into the bottle and how, in its own YAML manifest
//! (`<Category>/<name>.yml`: Dependencies, Parameters, Steps, Executable). Bottles' own command can't run them, so
//! Hearth runs the manifest itself, step by step, with the bottle's own Wine, showing each step as it goes:
//!   Dependencies     the extras, with winetricks (Bottles' names mapped to winetricks' verbs)
//!   Parameters       the bottle's settings, in its bottle.yml (one line at a time, .bak kept)
//!   Steps            install_exe / install_msi (downloaded, its checksum checked, run in the bottle and waited for),
//!                    run_winecommand, set_register_key, register_font, replace_font, override_dll (Bottles'
//!                    DLL_Overrides), set_windows / use_windows, update_driveC (a file put into C:)
//!   Executable       the program added to the bottle's programs (Bottles' External_Programs)
//! A step Hearth doesn't know, or one that runs the manifest's own shell script, is skipped and said so (never run
//! blindly). A snapshot of the bottle is taken first when the bottle asks for one before installs.

use hearth_oss_kit_a::yaml;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const BASE: &str = "https://raw.githubusercontent.com/bottlesdevs/programs/main";

/// A path piece for the address (letters, digits, - _ . ~ as they are; the rest %XX).
pub fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Manifest {
    pub name: String,
    pub deps: Vec<String>,
    pub params: Vec<(String, String)>,
    pub steps: Vec<Vec<(String, String)>>,
    pub exe: Vec<(String, String)>,
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}
fn kv(l: &str) -> Option<(String, String)> {
    let (k, v) = l.trim().trim_start_matches("- ").split_once(':')?;
    Some((k.trim().to_string(), yaml::unquote(v.trim())))
}

/// Reads a manifest (the parts Hearth runs; everything else in it is left alone).
pub fn parse(y: &str) -> Manifest {
    let mut m = Manifest { name: yaml::yget(y, &["Name"]).unwrap_or_default(), deps: yaml::ylist(y, &["Dependencies"]), ..Manifest::default() };
    m.params = yaml::ymap(y, &["Parameters"]).into_iter().map(|(k, v)| (k, yaml::unquote(&v))).collect();
    m.exe = yaml::ymap(y, &["Executable"]).into_iter().map(|(k, v)| (k, yaml::unquote(&v))).collect();
    // Steps: a list of maps ("- action: install_exe" and the keys under it)
    let lines: Vec<&str> = y.lines().collect();
    if let Some(i) = lines.iter().position(|l| l.trim_end() == "Steps:" && indent(l) == 0) {
        let mut cur: Option<Vec<(String, String)>> = None;
        let mut item_ind = usize::MAX;
        for l in &lines[i + 1..] {
            if l.trim().is_empty() || l.trim_start().starts_with('#') {
                continue;
            }
            let t = l.trim_start();
            // the next top-level key ends the list (its items may sit at the left edge too: "- action: …")
            if indent(l) == 0 && !t.starts_with("- ") {
                break;
            }
            if t.starts_with("- ") && indent(l) <= item_ind {
                item_ind = indent(l);
                if let Some(c) = cur.take() {
                    m.steps.push(c);
                }
                cur = Some(kv(t).into_iter().collect());
            } else if let (Some(c), Some(p)) = (cur.as_mut(), kv(t)) {
                c.push(p);
            }
        }
        if let Some(c) = cur {
            m.steps.push(c);
        }
    }
    m
}

pub fn get<'a>(step: &'a [(String, String)], k: &str) -> &'a str {
    step.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str()).unwrap_or("")
}

/// Bottles' dependency names → winetricks' verbs (what Hearth can install; the rest is said).
pub fn verb(dep: &str) -> Option<&'static str> {
    Some(match dep {
        "vcredist2022" | "vcredist2019" | "vcredist2017" | "vcredist2015" => "vcrun2022",
        "vcredist2013" => "vcrun2013",
        "vcredist2012" => "vcrun2012",
        "vcredist2010" => "vcrun2010",
        "vcredist2008" => "vcrun2008",
        "vcredist2005" => "vcrun2005",
        "dotnet48" => "dotnet48",
        "dotnet472" => "dotnet472",
        "dotnet462" => "dotnet462",
        "dotnet40" => "dotnet40",
        "dotnet35" => "dotnet35",
        "dotnetcore3" => "dotnetcoredesktop3",
        "dotnetdesktop6" | "dotnet6" => "dotnetdesktop6",
        "dotnetdesktop7" => "dotnetdesktop7",
        "dotnetdesktop8" => "dotnetdesktop8",
        "allfonts" => "allfonts",
        "corefonts" | "arial32" | "times32" | "courie32" | "verdan32" | "webdin32" | "trebuc32" | "georgi32" | "impact32" | "comic32" | "andale32" => "corefonts",
        "d3dx9" => "d3dx9",
        "d3dx10" => "d3dx10",
        "d3dx11" => "d3dx11_43",
        "d3dcompiler_47" => "d3dcompiler_47",
        "mf" | "mfplat" => "mf",
        "xact" => "xact",
        "xinput" => "xinput",
        "gdiplus" => "gdiplus",
        "msxml3" => "msxml3",
        "msxml6" => "msxml6",
        "vbrun6" => "vb6run",
        "physx" => "physx",
        "faudio" => "faudio",
        "wmp11" => "wmp11",
        "quartz" => "quartz",
        "riched20" => "riched20",
        "mdac28" => "mdac28",
        "cjkfonts" => "cjkfonts",
        "webview2" => "webview2",
        _ => return None,
    })
}

/// A step in words, as the installer's list shows it.
pub fn step_words(s: &[(String, String)]) -> String {
    let file = |k: &str| Path::new(get(s, k)).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    match get(s, "action") {
        "install_exe" | "install_msi" => format!("Download and run {}", if get(s, "file_name").is_empty() { file("url") } else { get(s, "file_name").to_string() }),
        "run_winecommand" => format!("Run {}", get(s, "command")),
        "set_register_key" => format!("Set {} in the registry", get(s, "value")),
        "register_font" | "replace_font" => format!("The font {}", get(s, "name")),
        "override_dll" => format!("Use {} {}", get(s, "dll"), get(s, "type")),
        "set_windows" | "use_windows" => format!("Act like {}", get(s, "version")),
        "update_driveC" => format!("Put {} into C:", file("url")),
        "run_script" => "The installer's own script (skipped: Hearth doesn't run it)".into(),
        o => format!("Bottles' step {o} (skipped)"),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum St {
    Waiting,
    Doing,
    Done,
    Skipped(String),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct Line {
    pub words: String,
    pub st: St,
}

/// What an installer is doing (shared with its thread; the window reads it each half second).
pub type Progress = Arc<Mutex<Vec<Line>>>;

pub struct Ctx {
    pub bottle: String,
    pub dir: PathBuf,
    pub yml: PathBuf,
    pub wine: String,
}

fn sh(cmd: &[String]) -> Result<(), String> {
    let o = hearth_ui::run::blocking(cmd);
    if o.ok { Ok(()) } else { Err(o.why()) }
}
fn wine(c: &Ctx, args: &[&str]) -> Result<(), String> {
    let mut v = vec!["env".to_string(), format!("WINEPREFIX={}", c.dir.display()), "WINEDEBUG=-all".into(), c.wine.clone()];
    v.extend(args.iter().map(|s| s.to_string()));
    sh(&v)
}
pub fn reg_add(c: &Ctx, key: &str, value: &str, data: &str, kind: &str) -> Result<(), String> {
    wine(c, &["reg", "add", key, "/v", value, "/t", if kind.is_empty() { "REG_SZ" } else { kind }, "/d", data, "/f"])
}

/// A downloaded file in Hearth's cache, its MD5 checked when the manifest gives one.
fn download(url: &str, name: &str, md5: &str) -> Result<PathBuf, String> {
    if !url.starts_with("https://") {
        return Err("only https:// downloads".into());
    }
    let d = hearth_oss_kit_a::home().join(".cache/hearth/bottles-installers");
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let safe: String = (if name.is_empty() { url.rsplit('/').next().unwrap_or("setup.exe") } else { name }).chars().map(|c| if c.is_alphanumeric() || ".-_".contains(c) { c } else { '_' }).collect();
    let f = d.join(&safe);
    sh(&["curl".into(), "-fsSL".into(), "--max-time".into(), "1800".into(), "-o".into(), f.to_string_lossy().to_string(), url.into()])?;
    if !md5.trim().is_empty() {
        let o = hearth_ui::run::blocking(&["md5sum".into(), f.to_string_lossy().to_string()]);
        let got = o.out.split_whitespace().next().unwrap_or("").to_lowercase();
        if got != md5.trim().to_lowercase() {
            let _ = std::fs::remove_file(&f);
            return Err("the download isn't the file the installer was checked with".into());
        }
    }
    Ok(f)
}

fn change_yml(c: &Ctx, f: impl FnOnce(&str) -> String) -> Result<(), String> {
    hearth_oss_kit_a::settings_file::update_text(&c.yml, f).map(|_| ()).map_err(|e| e.to_string())
}

fn find_in(dir: &Path, name: &str, depth: usize) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut subs = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            subs.push(p);
        } else if e.file_name().to_string_lossy().eq_ignore_ascii_case(name) {
            return Some(p);
        }
    }
    if depth == 0 {
        return None;
    }
    subs.into_iter().find_map(|s| find_in(&s, name, depth - 1))
}

/// One step of the manifest, in the bottle.
pub fn do_step(c: &Ctx, s: &[(String, String)]) -> Result<Option<String>, String> {
    match get(s, "action") {
        "install_exe" | "install_msi" => {
            let f = download(get(s, "url"), get(s, "file_name"), get(s, "file_checksum"))?;
            let fs = f.to_string_lossy().to_string();
            let args: Vec<&str> = get(s, "arguments").split_whitespace().collect();
            let mut a: Vec<&str> = if get(s, "action") == "install_msi" { vec!["msiexec", "/i", fs.as_str()] } else { vec![fs.as_str()] };
            a.extend(args);
            wine(c, &a)?;
            Ok(None)
        }
        "run_winecommand" => {
            let mut a: Vec<&str> = get(s, "command").split_whitespace().collect();
            a.extend(get(s, "arguments").split_whitespace());
            if a.is_empty() {
                return Ok(Some("nothing to run".into()));
            }
            wine(c, &a)?;
            Ok(None)
        }
        "set_register_key" => {
            reg_add(c, get(s, "key"), get(s, "value"), get(s, "data"), get(s, "key_type"))?;
            Ok(None)
        }
        "register_font" => {
            reg_add(c, "HKLM\\Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts", get(s, "name"), get(s, "file"), "REG_SZ")?;
            Ok(None)
        }
        "replace_font" => {
            for f in get(s, "font").split(',') {
                reg_add(c, "HKCU\\Software\\Wine\\Fonts\\Replacements", f.trim(), get(s, "replace"), "REG_SZ")?;
            }
            Ok(None)
        }
        "override_dll" => {
            let (dll, kind) = (get(s, "dll").to_string(), get(s, "type").to_string());
            change_yml(c, |t| yaml::yset(t, &["DLL_Overrides", &dll], Some(&yaml::quote(&kind))))?;
            reg_add(c, "HKCU\\Software\\Wine\\DllOverrides", &dll, &kind, "REG_SZ")?;
            Ok(None)
        }
        "set_windows" | "use_windows" => {
            let w = get(s, "version").to_string();
            change_yml(c, |t| yaml::yset(t, &["Windows"], Some(&yaml::quote(&w))))?;
            wine(c, &["winecfg", "-v", &w])?;
            Ok(None)
        }
        "update_driveC" => {
            let f = download(get(s, "url"), get(s, "file_name"), get(s, "file_checksum"))?;
            let rel = get(s, "dest").trim_start_matches(['/', '\\']).replace('\\', "/");
            if rel.contains("..") {
                return Err("a place outside C:".into());
            }
            let to = c.dir.join("drive_c").join(rel);
            if let Some(p) = to.parent() {
                std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            std::fs::copy(&f, &to).map_err(|e| e.to_string())?;
            Ok(None)
        }
        "run_script" => Ok(Some("Hearth doesn't run an installer's own scripts".into())),
        o => Ok(Some(format!("Hearth doesn't know Bottles' step {o}"))),
    }
}

/// The whole manifest, in order, each line's state in `prog` as it goes. Ends with the program in the bottle's list.
pub fn run(c: Ctx, m: Manifest, prog: Progress) {
    let set = |i: usize, st: St| {
        if let Ok(mut g) = prog.lock() {
            if let Some(l) = g.get_mut(i) {
                l.st = st;
            }
        }
    };
    let mut i = 0;
    let mut failed = false;
    for d in &m.deps {
        set(i, St::Doing);
        match verb(d) {
            Some(v) => {
                let cmd = vec!["env".to_string(), format!("WINEPREFIX={}", c.dir.display()), format!("WINE={}", c.wine), "winetricks".into(), "-q".into(), v.into()];
                match sh(&cmd) {
                    Ok(()) => {
                        let v = v.to_string();
                        let _ = change_yml(&c, |t| {
                            let have = yaml::ylist(t, &["Installed_Dependencies"]);
                            if have.iter().any(|x| *x == v) {
                                return t.to_string();
                            }
                            let mut items = have;
                            items.push(v);
                            let body: String = items.iter().map(|x| format!("- {x}\n")).collect();
                            let mut out = yaml::yset(t, &["Installed_Dependencies"], None);
                            out.push_str(&format!("Installed_Dependencies:\n{body}"));
                            out
                        });
                        set(i, St::Done);
                    }
                    Err(e) => {
                        set(i, St::Failed(e));
                        failed = true;
                    }
                }
            }
            None => set(i, St::Skipped(format!("winetricks has no {d}"))),
        }
        i += 1;
        if failed {
            return;
        }
    }
    if !m.params.is_empty() {
        set(i, St::Doing);
        let ps = m.params.clone();
        match change_yml(&c, |t| ps.iter().fold(t.to_string(), |t, (k, v)| yaml::yset(&t, &["Parameters", k], Some(&yaml::quote(v))))) {
            Ok(()) => set(i, St::Done),
            Err(e) => set(i, St::Failed(e)),
        }
        i += 1;
    }
    for s in &m.steps {
        set(i, St::Doing);
        match do_step(&c, s) {
            Ok(None) => set(i, St::Done),
            Ok(Some(why)) => set(i, St::Skipped(why)),
            Err(e) => {
                set(i, St::Failed(e));
                return;
            }
        }
        i += 1;
    }
    // the program in the bottle's own list
    set(i, St::Doing);
    let file = m.exe.iter().find(|(k, _)| k == "file").map(|(_, v)| v.clone()).unwrap_or_default();
    let name = m.exe.iter().find(|(k, _)| k == "name").map(|(_, v)| v.clone()).unwrap_or_else(|| m.name.clone());
    let found = if file.is_empty() { None } else { find_in(&c.dir.join("drive_c"), &file, 5) };
    match found {
        Some(path) => {
            let id = format!("hearth-{}", name.to_lowercase().chars().map(|ch| if ch.is_alphanumeric() { ch } else { '-' }).collect::<String>());
            let ps = path.to_string_lossy().to_string();
            let r = change_yml(&c, |t| {
                let t = yaml::yset(t, &["External_Programs", &id, "id"], Some(&yaml::quote(&id)));
                let t = yaml::yset(&t, &["External_Programs", &id, "name"], Some(&yaml::quote(&name)));
                let t = yaml::yset(&t, &["External_Programs", &id, "executable"], Some(&yaml::quote(&file)));
                yaml::yset(&t, &["External_Programs", &id, "path"], Some(&yaml::quote(&ps)))
            });
            set(i, match r {
                Ok(()) => St::Done,
                Err(e) => St::Failed(e),
            });
        }
        None => set(i, St::Skipped(format!("{file} wasn't found in C: (run it with Run a program…)"))),
    }
}

/// The lines the installer's list starts with (all Waiting).
pub fn lines_of(m: &Manifest) -> Vec<Line> {
    let mut v: Vec<Line> = m.deps.iter().map(|d| Line { words: format!("Extra: {d}"), st: St::Waiting }).collect();
    if !m.params.is_empty() {
        v.push(Line { words: format!("The bottle's settings ({})", m.params.iter().map(|p| p.0.as_str()).collect::<Vec<_>>().join(", ")), st: St::Waiting });
    }
    v.extend(m.steps.iter().map(|s| Line { words: step_words(s), st: St::Waiting }));
    v.push(Line { words: format!("{} in this bottle's programs", m.exe.iter().find(|(k, _)| k == "name").map(|(_, v)| v.as_str()).unwrap_or(m.name.as_str())), st: St::Waiting });
    v
}

/// The Wine registry for "Controller and USB" (Wine's own bus driver, winebus): on, controllers (SDL, the kernel's
/// input devices) and USB devices (hidraw) reach the bottle's programs; off, they're kept apart from it.
pub fn devices_reg(on: bool) -> String {
    let (hid, inp, sdl) = if on { (0, 0, 1) } else { (1, 1, 0) };
    format!("REGEDIT4\r\n\r\n[HKEY_LOCAL_MACHINE\\System\\CurrentControlSet\\Services\\winebus]\r\n\"DisableHidraw\"=dword:{hid:08x}\r\n\"DisableInput\"=dword:{inp:08x}\r\n\"Enable SDL\"=dword:{sdl:08x}\r\n\"Map Controllers\"=dword:{sdl:08x}\r\n\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    const NPP: &str = "Name: Notepad++\nDescription: A text editor\nGrade: Platinum\nArch: win64\nDependencies:\n- vcredist2019\n- arial32\nParameters:\n  dxvk: false\nSteps:\n- action: install_exe\n  file_name: npp.exe\n  url: https://example.org/npp.exe\n  file_checksum: abc\n  arguments: /S\n- action: run_script\n  script: echo hi\n- action: override_dll\n  dll: d3d9\n  type: native,builtin\nExecutable:\n  name: Notepad++\n  file: notepad++.exe\n  path: Notepad++/notepad++.exe\n";
    #[test]
    fn a_manifest_is_read_step_by_step() {
        let m = parse(NPP);
        assert_eq!(m.name, "Notepad++");
        assert_eq!(m.deps, vec!["vcredist2019", "arial32"]);
        assert_eq!(m.steps.len(), 3);
        assert_eq!(get(&m.steps[0], "arguments"), "/S");
        assert_eq!(get(&m.steps[2], "type"), "native,builtin");
        assert_eq!(step_words(&m.steps[0]), "Download and run npp.exe");
        assert!(step_words(&m.steps[1]).contains("skipped"));
        assert_eq!(verb("vcredist2019"), Some("vcrun2022"));
        assert_eq!(lines_of(&m).len(), 2 + 1 + 3 + 1);
        assert!(devices_reg(false).contains("\"DisableHidraw\"=dword:00000001"));
        assert!(devices_reg(true).contains("\"Enable SDL\"=dword:00000001"));
    }
}

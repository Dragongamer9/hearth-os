//! Bethesda plugins (.esp, .esm, .esl): their headers (the TES4 record: master and light flags, the masters they
//! need, author and description), the load order (plugins.txt / loadorder.txt), missing masters, a sort helper
//! and the BSA/BA2 archives they load.
//!
//! The sort helper is not LOOT: LOOT's masterlist (rules per plugin, from github.com/loot) isn't read yet. It keeps
//! your order and only fixes what the game itself requires: the game's own masters first, master files before the
//! others, and every plugin after the masters it needs.

use super::kinds::Kind;
use super::mo2::{self, PluginLine};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const RECORD_MASTER: u32 = 0x1;
pub const RECORD_LOCALIZED: u32 = 0x80;

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub master_flag: bool,
    pub light_flag: bool,
    pub localized: bool,
    pub masters: Vec<String>,
    pub author: String,
    pub description: String,
    pub version: f32,
    pub records: u32,
}

fn u16le(b: &[u8]) -> u16 { u16::from_le_bytes([b[0], b[1]]) }
fn u32le(b: &[u8]) -> u32 { u32::from_le_bytes([b[0], b[1], b[2], b[3]]) }
/// A zero-ended string in the game's code page (Windows-1252; read as Latin-1, which matches for letters).
fn zstr(b: &[u8]) -> String { b.iter().take_while(|&&c| c != 0).map(|&c| c as char).collect() }

/// Reads a plugin's header from its first bytes.
pub fn parse_header(bytes: &[u8], header_size: usize, light_flag: u32) -> Result<Header, String> {
    if header_size < 20 || bytes.len() < header_size { return Err("too short to be a plugin".into()); }
    if &bytes[0..4] != b"TES4" { return Err("not a plugin (it has no TES4 header)".into()); }
    let size = u32le(&bytes[4..8]) as usize;
    let flags = u32le(&bytes[8..12]);
    let end = header_size.saturating_add(size).min(bytes.len());
    let mut h = Header { master_flag: flags & RECORD_MASTER != 0, light_flag: light_flag != 0 && flags & light_flag != 0, localized: flags & RECORD_LOCALIZED != 0, ..Default::default() };
    let mut i = header_size;
    let mut big: Option<usize> = None;
    while i + 6 <= end {
        let ty = [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]];
        let mut len = u16le(&bytes[i + 4..i + 6]) as usize;
        i += 6;
        if let Some(b) = big.take() { len = b; }
        if &ty == b"XXXX" {
            // The next field is longer than 65535 bytes: its real size is here.
            if len >= 4 && i + 4 <= end { big = Some(u32le(&bytes[i..i + 4]) as usize); }
            i += len;
            continue;
        }
        if i + len > end { break; }
        let d = &bytes[i..i + len];
        match &ty {
            b"HEDR" if len >= 8 => { h.version = f32::from_le_bytes([d[0], d[1], d[2], d[3]]); h.records = u32le(&d[4..8]); }
            b"CNAM" => h.author = zstr(d),
            b"SNAM" => h.description = zstr(d),
            b"MAST" => { let m = zstr(d); if !m.is_empty() { h.masters.push(m); } }
            _ => {}
        }
        i += len;
    }
    Ok(h)
}

/// Reads a plugin file's header (only its first record is read).
pub fn read_header(path: &Path, kind: &Kind) -> Result<Header, String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut head = vec![0u8; kind.header.max(24)];
    let n = f.read(&mut head).map_err(|e| e.to_string())?;
    head.truncate(n);
    if head.len() < kind.header { return Err("too short to be a plugin".into()); }
    let size = u32le(&head[4..8]) as usize;
    let want = (kind.header + size).min(16 << 20);
    let mut all = head;
    if want > all.len() {
        let mut rest = vec![0u8; want - all.len()];
        let n = f.read(&mut rest).map_err(|e| e.to_string())?;
        rest.truncate(n);
        all.extend(rest);
    }
    parse_header(&all, kind.header, kind.light)
}

pub fn is_plugin(name: &str) -> bool {
    let l = name.to_lowercase();
    l.ends_with(".esp") || l.ends_with(".esm") || l.ends_with(".esl")
}
fn ext(name: &str) -> String { name.rsplit('.').next().unwrap_or("").to_lowercase() }

/// One plugin in the load order.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plugin {
    pub name: String,
    pub active: bool,
    /// The game always loads it (its own masters, Creation Club content): it can't be turned off or moved.
    pub implicit: bool,
    /// Loads with the masters (the master flag, or .esm/.esl in games with light plugins).
    pub master: bool,
    pub light: bool,
    /// The mod it comes from ("" = the game's own Data folder).
    #[serde(rename = "mod")]
    pub from: String,
    pub masters: Vec<String>,
    pub author: String,
    pub description: String,
    /// "05", "FE:003" for active plugins.
    pub load_index: String,
    /// Masters it needs that aren't there, aren't on, or load after it.
    pub missing: Vec<MissingMaster>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub problem: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MissingMaster { pub master: String, pub why: &'static str }

/// The plugins at the top of a game's Data view: lower-case name → (its name, (the mod it comes from, its file)).
pub type Present = BTreeMap<String, (String, (String, std::path::PathBuf))>;

/// The plugins in a game's Data view, in load order. `saved`: plugins.txt's lines, `loadorder`: loadorder.txt
/// (older games), `ccc`: the game's Creation Club list (always loaded).
pub fn load_order(kind: &Kind, present: &Present, saved: &[PluginLine], loadorder: &[String], ccc: &[String]) -> Vec<Plugin> {
    let mut names: Vec<String> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut add = |n: &str, names: &mut Vec<String>| { let k = n.to_lowercase(); if present.contains_key(&k) && seen.insert(k) { names.push(n.to_string()); } };
    for n in kind.implicit.iter().copied() { add(n, &mut names); }
    for n in ccc { add(n.as_str(), &mut names); }
    if kind.star { for p in saved { add(p.name.as_str(), &mut names); } }
    else { for n in loadorder { add(n.as_str(), &mut names); } for p in saved { add(p.name.as_str(), &mut names); } }
    // New plugins (not in the lists yet) go at the end, in name order, as MO2 adds them.
    for (_, (shown, _)) in present.iter() { add(shown.as_str(), &mut names); }
    let active_saved: BTreeSet<String> = saved.iter().filter(|p| p.active).map(|p| p.name.to_lowercase()).collect();
    let implicit: BTreeSet<String> = kind.implicit.iter().map(|s| s.to_lowercase()).chain(ccc.iter().map(|s| s.to_lowercase())).collect();
    let mut out: Vec<Plugin> = names.iter().map(|n| {
        let k = n.to_lowercase();
        let (shown, (from, file)) = present.get(&k).map(|(s, x)| (s.clone(), x.clone())).unwrap_or_default();
        let (h, problem) = match read_header(&file, kind) { Ok(h) => (h, String::new()), Err(e) => (Header::default(), e) };
        let e = ext(&shown);
        let light = h.light_flag || (kind.light != 0 && e == "esl");
        let master = h.master_flag || (kind.light != 0 && (e == "esm" || e == "esl"));
        Plugin { active: implicit.contains(&k) || active_saved.contains(&k), implicit: implicit.contains(&k), master, light, name: shown, from, masters: h.masters, author: h.author, description: h.description, problem, ..Default::default() }
    }).collect();
    // The game loads master files before the others, whatever the list says.
    let (m, o): (Vec<Plugin>, Vec<Plugin>) = out.drain(..).partition(|p| p.master || p.implicit);
    out = m;
    out.extend(o);
    check(&mut out);
    out
}

/// Load indexes and missing masters.
pub fn check(list: &mut [Plugin]) {
    let (mut full, mut light) = (0u32, 0u32);
    for p in list.iter_mut() {
        p.load_index = if !p.active { String::new() } else if p.light { light += 1; format!("FE:{:03X}", light - 1) } else { full += 1; format!("{:02X}", full - 1) };
    }
    let pos: BTreeMap<String, (usize, bool)> = list.iter().enumerate().map(|(i, p)| (p.name.to_lowercase(), (i, p.active))).collect();
    for (i, p) in list.iter_mut().enumerate() {
        p.missing = p.masters.iter().filter_map(|m| match pos.get(&m.to_lowercase()) {
            None => Some(MissingMaster { master: m.clone(), why: "missing" }),
            Some((_, false)) if p.active => Some(MissingMaster { master: m.clone(), why: "inactive" }),
            Some((j, _)) if *j > i => Some(MissingMaster { master: m.clone(), why: "loadsAfter" }),
            _ => None,
        }).collect();
    }
}

/// The sort helper: your order, with the game's own plugins first, masters before the others and every plugin after
/// the masters it needs (a stable topological order; a loop is left as it is).
pub fn sort(list: &[Plugin]) -> Vec<Plugin> {
    let (fixed, rest): (Vec<Plugin>, Vec<Plugin>) = list.iter().cloned().partition(|p| p.implicit);
    let (masters, others): (Vec<Plugin>, Vec<Plugin>) = rest.into_iter().partition(|p| p.master);
    let mut out = fixed;
    for group in [masters, others] {
        let mut pending = group;
        while !pending.is_empty() {
            let placed: BTreeSet<String> = out.iter().map(|p| p.name.to_lowercase()).collect();
            let waiting: BTreeSet<String> = pending.iter().map(|p| p.name.to_lowercase()).collect();
            let i = pending.iter().position(|p| p.masters.iter().all(|m| { let k = m.to_lowercase(); placed.contains(&k) || !waiting.contains(&k) })).unwrap_or(0);
            out.push(pending.remove(i));
        }
    }
    check(&mut out);
    out
}

/// What goes in plugins.txt (and loadorder.txt): the game's implicit plugins aren't listed in plugins.txt.
pub fn to_lines(list: &[Plugin]) -> (Vec<PluginLine>, Vec<String>) {
    let lines = list.iter().filter(|p| !p.implicit).map(|p| PluginLine { name: p.name.clone(), active: p.active }).collect();
    let order = list.iter().map(|p| p.name.clone()).collect();
    (lines, order)
}

/// Writes a profile's plugins.txt and loadorder.txt.
pub fn save(profile: &Path, kind: &Kind, list: &[Plugin]) -> Result<(), String> {
    let (lines, order) = to_lines(list);
    crate::util::write_atomic(&profile.join("plugins.txt"), mo2::write_plugins(&lines, kind.star).as_bytes()).map_err(|e| format!("Couldn't save plugins.txt ({e})"))?;
    crate::util::write_atomic(&profile.join("loadorder.txt"), mo2::write_loadorder(&order).as_bytes()).map_err(|e| format!("Couldn't save loadorder.txt ({e})"))
}

/// BSA/BA2 archives in the view, with the plugin that loads each ("Mod.esp" loads "Mod.bsa" and "Mod - Textures.bsa").
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Archive { pub name: String, #[serde(rename = "mod")] pub from: String, pub plugin: Option<String>, pub loaded: bool }

pub fn archives(files: &[(String, String)], plugins: &[Plugin]) -> Vec<Archive> {
    files.iter().filter(|(n, _)| { let e = ext(n); e == "bsa" || e == "ba2" }).map(|(n, from)| {
        let stem = n[..n.rfind('.').unwrap_or(n.len())].to_lowercase();
        let p = plugins.iter().find(|p| {
            let ps = p.name[..p.name.rfind('.').unwrap_or(p.name.len())].to_lowercase();
            stem == ps || stem.starts_with(&format!("{ps} - "))
        });
        Archive { name: n.clone(), from: from.clone(), plugin: p.map(|p| p.name.clone()), loaded: p.map(|p| p.active).unwrap_or(false) }
    }).collect()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A tiny plugin: a TES4 record with HEDR, CNAM, SNAM and MAST/DATA fields (24-byte record header).
    fn field(out: &mut Vec<u8>, ty: &[u8], data: &[u8]) { out.extend_from_slice(ty); out.extend_from_slice(&(data.len() as u16).to_le_bytes()); out.extend_from_slice(data); }
    fn z(s: &str) -> Vec<u8> { let mut v = s.as_bytes().to_vec(); v.push(0); v }
    pub fn esp(flags: u32, masters: &[&str], author: &str) -> Vec<u8> {
        let mut fields = Vec::new();
        let mut hedr = 1.7f32.to_le_bytes().to_vec();
        hedr.extend_from_slice(&3u32.to_le_bytes());
        hedr.extend_from_slice(&0x800u32.to_le_bytes());
        field(&mut fields, b"HEDR", &hedr);
        field(&mut fields, b"CNAM", &z(author));
        field(&mut fields, b"SNAM", &z("A test plugin"));
        for m in masters { field(&mut fields, b"MAST", &z(m)); field(&mut fields, b"DATA", &0u64.to_le_bytes()); }
        let mut out = b"TES4".to_vec();
        out.extend_from_slice(&(fields.len() as u32).to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&[0u8; 4]);
        out.extend_from_slice(&[0u8; 4]);
        out.extend_from_slice(&44u16.to_le_bytes());
        out.extend_from_slice(&[0u8; 2]);
        out.extend_from_slice(&fields);
        // A record after the header, which must not be read.
        out.extend_from_slice(b"GRUP\x10\x00\x00\x00MAST");
        out
    }

    #[test]
    fn headers() {
        let h = parse_header(&esp(0x201, &["Skyrim.esm", "Update.esm"], "Dragongamer9"), 24, 0x200).unwrap();
        assert!(h.master_flag && h.light_flag && !h.localized);
        assert_eq!(h.masters, vec!["Skyrim.esm", "Update.esm"]);
        assert_eq!((h.author.as_str(), h.description.as_str(), h.records), ("Dragongamer9", "A test plugin", 3));
        assert!((h.version - 1.7).abs() < 1e-6);
        // Light flags only count in games that have light plugins.
        assert!(!parse_header(&esp(0x200, &[], ""), 24, 0).unwrap().light_flag);
        assert!(parse_header(b"TES3xxxxxxxxxxxxxxxxxxxxxxxx", 24, 0).is_err());
        assert!(parse_header(b"TES4", 24, 0).is_err());
        // An XXXX field gives the real size of the next one.
        let mut big = b"TES4".to_vec();
        let mut f = Vec::new();
        f.extend_from_slice(b"XXXX"); f.extend_from_slice(&4u16.to_le_bytes()); f.extend_from_slice(&11u32.to_le_bytes());
        f.extend_from_slice(b"MAST"); f.extend_from_slice(&0u16.to_le_bytes()); f.extend_from_slice(b"Big.esm\0abc");
        big.extend_from_slice(&(f.len() as u32).to_le_bytes()); big.extend_from_slice(&[0u8; 16]); big.extend_from_slice(&f);
        assert_eq!(parse_header(&big, 24, 0).unwrap().masters, vec!["Big.esm"]);
        // Oblivion's 20-byte header.
        let mut ob = esp(1, &["Oblivion.esm"], "x");
        ob.drain(20..24);
        assert_eq!(parse_header(&ob, 20, 0).unwrap().masters, vec!["Oblivion.esm"]);
    }

    fn present(dir: &Path, files: &[(&str, Vec<u8>, &str)]) -> Present {
        let mut m = BTreeMap::new();
        for (n, b, from) in files { let p = dir.join(n); std::fs::write(&p, b).unwrap(); m.insert(n.to_lowercase(), (n.to_string(), (from.to_string(), p))); }
        m
    }

    #[test]
    fn order_masters_and_sort() {
        let t = tempfile::tempdir().unwrap();
        let k = Kind::by_id("skyrimse");
        let p = present(t.path(), &[
            ("Skyrim.esm", esp(1, &[], "Bethesda"), ""), ("Update.esm", esp(1, &["Skyrim.esm"], "Bethesda"), ""),
            ("Patch.esp", esp(0, &["Skyrim.esm", "Lib.esm"], "a"), "Patch"), ("Lib.esm", esp(1, &["Skyrim.esm"], "b"), "Lib"),
            ("Small.esp", esp(0x200, &["Skyrim.esm"], "c"), "Small"), ("Needs.esp", esp(0, &["Gone.esm", "Off.esp"], "d"), "Needs"), ("Off.esp", esp(0, &[], "e"), "Off"),
        ]);
        let saved = mo2::parse_plugins("*Patch.esp\r\n*Lib.esm\r\n*Small.esp\r\n*Needs.esp\r\nOff.esp\r\n", true);
        let l = load_order(&k, &p, &saved, &[], &[]);
        let names: Vec<&str> = l.iter().map(|p| p.name.as_str()).collect();
        // Masters first (Lib.esm moves up), the rest in your order.
        assert_eq!(names, vec!["Skyrim.esm", "Update.esm", "Lib.esm", "Patch.esp", "Small.esp", "Needs.esp", "Off.esp"]);
        assert!(l[0].implicit && l[0].active);
        assert_eq!((l[2].load_index.as_str(), l[4].load_index.as_str(), l[4].light), ("02", "FE:000", true));
        assert_eq!(l[5].missing, vec![MissingMaster { master: "Gone.esm".into(), why: "missing" }, MissingMaster { master: "Off.esp".into(), why: "inactive" }]);
        assert!(l[3].missing.is_empty());
        assert_eq!(l[3].from, "Patch");
        // Sorting: a plugin moved above its master goes back after it.
        let mut bad = l.clone();
        let needs = bad.remove(5);
        bad.insert(3, Plugin { masters: vec!["Off.esp".into()], active: true, ..needs });
        let off = bad.pop().unwrap();
        bad.insert(4, Plugin { active: true, ..off });
        // Off.esp now after Needs.esp? Needs.esp is at 3, Off.esp at 4: it loads after.
        check(&mut bad);
        assert_eq!(bad[3].missing, vec![MissingMaster { master: "Off.esp".into(), why: "loadsAfter" }]);
        let s = sort(&bad);
        let sn: Vec<&str> = s.iter().map(|p| p.name.as_str()).collect();
        assert!(sn.iter().position(|n| *n == "Off.esp") < sn.iter().position(|n| *n == "Needs.esp"), "{sn:?}");
        assert!(s.iter().all(|p| p.missing.is_empty()), "{s:?}");
        // plugins.txt leaves the game's own masters out.
        let (lines, order) = to_lines(&s);
        assert!(!lines.iter().any(|x| x.name == "Skyrim.esm") && order[0] == "Skyrim.esm");
    }

    #[test]
    fn old_games_use_loadorder() {
        let t = tempfile::tempdir().unwrap();
        let k = Kind::by_id("falloutnv");
        let p = present(t.path(), &[("FalloutNV.esm", esp(1, &[], ""), ""), ("B.esp", esp(0, &["FalloutNV.esm"], ""), "B"), ("A.esp", esp(0, &["FalloutNV.esm"], ""), "A")]);
        let l = load_order(&k, &p, &mo2::parse_plugins("B.esp\r\n", false), &["FalloutNV.esm".into(), "B.esp".into(), "A.esp".into()], &[]);
        assert_eq!(l.iter().map(|p| (p.name.as_str(), p.active)).collect::<Vec<_>>(), vec![("FalloutNV.esm", true), ("B.esp", true), ("A.esp", false)]);
    }

    #[test]
    fn archives_follow_plugins() {
        let pl = vec![Plugin { name: "Mod.esp".into(), active: true, ..Default::default() }, Plugin { name: "Off.esp".into(), ..Default::default() }];
        let a = archives(&[("Mod.bsa".into(), "M".into()), ("Mod - Textures.bsa".into(), "M".into()), ("Off - Main.ba2".into(), "O".into()), ("Lone.bsa".into(), "L".into()), ("x.esp".into(), "x".into())], &pl);
        assert_eq!(a.len(), 4);
        assert!(a[0].loaded && a[1].loaded && a[1].plugin.as_deref() == Some("Mod.esp"));
        assert!(!a[2].loaded && a[2].plugin.as_deref() == Some("Off.esp"));
        assert!(a[3].plugin.is_none());
    }
}

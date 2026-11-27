//! What the Mod Manager works out itself, without a window (tested in CI): Sort with LOOT's masterlist and its reasons
//! (board 1), snapshots of a whole setup and going back (board 4), a save's plugins (board 4), and crash logs read in
//! plain words (board 5). The mods, plugins and conflicts themselves are the Hearth Mod Organizer's (hearth-gaming).

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ LOOT's masterlist (GPL-3)

/// The masterlist Sort uses: LOOT's own (https://github.com/loot), fetched at a pinned commit and used only when its
/// SHA-256 is the pinned one. `PIN` is (game folder at github.com/loot, commit, sha256); an empty commit means not
/// pinned yet, and Sort then uses only what the plugins say themselves (their masters).
pub const PIN: &[(&str, &str, &str, &str)] = &[
    // (kinds.rs id, loot repo, commit, sha256): branch v0.26 as fetched by CI on Nov 26, 2026
    ("skyrimse", "skyrimse", "d8bd34be6321e32bd7d38a5770ec45eb91fa4337", "78c9fa462f1f26b7f8105c019b7bac4354bcafa6bb36848369a6f745177a7f65"),
    ("fallout4", "fallout4", "b519c56dab5791dc7141d9ef178552817664bfe8", "c56da222811981021029e61482f55086c25f6b32cc1922e92ff4afd45491b3b7"),
];

pub fn masterlist_url(kind: &str) -> Option<(String, String)> {
    let (_, repo, commit, sha) = PIN.iter().find(|p| p.0 == kind)?;
    if commit.is_empty() {
        return None;
    }
    Some((format!("https://raw.githubusercontent.com/loot/{repo}/{commit}/masterlist.yaml"), sha.to_string()))
}

pub fn sha256_hex(b: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

/// One plugin's rules from the masterlist.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rule {
    pub after: Vec<String>,
    pub req: Vec<String>,
    pub inc: Vec<String>,
    pub group: String,
    pub msg: Vec<String>,
}

fn unq(s: &str) -> String {
    let s = s.trim().trim_end_matches(',').trim();
    let s = s.strip_prefix("name:").map(str::trim).unwrap_or(s);
    s.trim_matches(|c| c == '\'' || c == '"').to_string()
}

fn inline_list(v: &str) -> Vec<String> {
    let v = v.trim();
    if let Some(inner) = v.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
        return inner.split(',').map(unq).filter(|x| !x.is_empty()).collect();
    }
    if v.is_empty() {
        Vec::new()
    } else {
        vec![unq(v)]
    }
}

/// The `plugins:` part of a LOOT masterlist: each plugin's after / req / inc / group and its messages' words. A small
/// reader for the shapes LOOT's masterlists use (flow lists, block lists, `- name:` items, `- <<: *anchor` skipped).
pub fn parse_masterlist(text: &str) -> BTreeMap<String, Rule> {
    let mut out: BTreeMap<String, Rule> = BTreeMap::new();
    let mut in_plugins = false;
    let mut cur: Option<(String, Rule)> = None;
    let mut key = String::new();
    let mut item_indent = 0usize;
    let flush = |cur: &mut Option<(String, Rule)>, out: &mut BTreeMap<String, Rule>| {
        if let Some((n, r)) = cur.take() {
            if !n.is_empty() && !n.contains('\\') {
                out.insert(n.to_lowercase(), r);
            }
        }
    };
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let ind = line.len() - line.trim_start().len();
        let l = line.trim_start();
        if ind == 0 {
            flush(&mut cur, &mut out);
            in_plugins = l.starts_with("plugins:");
            continue;
        }
        if !in_plugins {
            continue;
        }
        if let Some(rest) = l.strip_prefix("- name:") {
            if cur.is_none() || ind <= item_indent {
                flush(&mut cur, &mut out);
                item_indent = ind;
                cur = Some((unq(rest), Rule::default()));
                key.clear();
                continue;
            }
        }
        let Some((_, r)) = cur.as_mut() else { continue };
        if let Some((k, v)) = l.split_once(':').filter(|(k, _)| !k.starts_with('-') && !k.contains(' ') && !k.contains('\'')) {
            key = k.to_string();
            let vals = inline_list(v);
            match k {
                "after" => r.after.extend(vals),
                "req" => r.req.extend(vals),
                "inc" => r.inc.extend(vals),
                "group" => r.group = vals.first().cloned().unwrap_or_default(),
                "msg" | "content" | "text" => {
                    if !v.trim().is_empty() && !v.trim().starts_with('[') {
                        r.msg.push(unq(v));
                    }
                }
                _ => {}
            }
            continue;
        }
        if let Some(item) = l.strip_prefix("- ") {
            if item.starts_with("<<") {
                continue;
            }
            let v = unq(item);
            if v.is_empty() || v.contains(": ") {
                continue;
            }
            match key.as_str() {
                "after" => r.after.push(v),
                "req" => r.req.push(v),
                "inc" => r.inc.push(v),
                _ => {}
            }
        }
    }
    flush(&mut cur, &mut out);
    out
}

/// A plugin as Sort sees it (the organizer's list: name, on, a master, its masters).
#[derive(Debug, Clone, PartialEq)]
pub struct P {
    pub name: String,
    pub active: bool,
    pub master: bool,
    pub implicit: bool,
    pub masters: Vec<String>,
}

pub fn plugins_of(summary: &Value) -> Vec<P> {
    summary["plugins"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|p| P {
                    name: p["name"].as_str().unwrap_or("").to_string(),
                    active: p["active"].as_bool().unwrap_or(false),
                    master: p["master"].as_bool().unwrap_or(false),
                    implicit: p["implicit"].as_bool().unwrap_or(false),
                    masters: p["masters"].as_array().map(|m| m.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// One move Sort would make, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    pub plugin: String,
    /// "after" or "before"
    pub way: &'static str,
    pub other: String,
    pub why: String,
}

/// Sort: your own order kept wherever nothing says otherwise. A plugin moves only when one of its masters, or a
/// plugin the masterlist says it loads after, is below it; masters stay before the rest. The moves and their
/// reasons, and the new order.
pub fn sort(list: &[P], rules: &BTreeMap<String, Rule>) -> (Vec<String>, Vec<Move>) {
    let present: BTreeSet<String> = list.iter().map(|p| p.name.to_lowercase()).collect();
    let needs = |p: &P| -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = p.masters.iter().filter(|m| present.contains(&m.to_lowercase())).map(|m| (m.clone(), "it's a patch for it".to_string())).collect();
        if let Some(r) = rules.get(&p.name.to_lowercase()) {
            for a in &r.after {
                if present.contains(&a.to_lowercase()) && !v.iter().any(|x| x.0.eq_ignore_ascii_case(a)) {
                    v.push((a.clone(), "masterlist: load after".to_string()));
                }
            }
        }
        v
    };
    let (fixed, rest): (Vec<&P>, Vec<&P>) = list.iter().partition(|p| p.implicit);
    let (masters, others): (Vec<&P>, Vec<&P>) = rest.into_iter().partition(|p| p.master);
    let mut out: Vec<String> = fixed.iter().map(|p| p.name.clone()).collect();
    for group in [masters, others] {
        let mut pending: Vec<&P> = group;
        while !pending.is_empty() {
            let placed: BTreeSet<String> = out.iter().map(|n| n.to_lowercase()).collect();
            let waiting: BTreeSet<String> = pending.iter().map(|p| p.name.to_lowercase()).collect();
            let i = pending.iter().position(|p| needs(p).iter().all(|(m, _)| placed.contains(&m.to_lowercase()) || !waiting.contains(&m.to_lowercase()))).unwrap_or(0);
            out.push(pending.remove(i).name.clone());
        }
    }
    // what moved, compared with the order now: a plugin that was above something it needs and is now below it
    let before: Vec<String> = list.iter().map(|p| p.name.clone()).collect();
    let pos = |v: &[String], n: &str| v.iter().position(|x| x.eq_ignore_ascii_case(n));
    let mut moves = Vec::new();
    for p in list {
        let mut best: Option<(usize, String, String)> = None;
        for (m, w) in needs(p) {
            if let (Some(b0), Some(b1), Some(a1)) = (pos(&before, &p.name), pos(&before, &m), pos(&out, &m)) {
                if b0 < b1 && best.as_ref().map(|b| a1 > b.0).unwrap_or(true) {
                    best = Some((a1, m.clone(), w));
                }
            }
        }
        if let Some((_, m, w)) = best {
            let real = list.iter().find(|x| x.name.eq_ignore_ascii_case(&m)).map(|x| x.name.clone()).unwrap_or(m);
            moves.push(Move { plugin: p.name.clone(), way: "after", other: real, why: w });
        }
    }
    (out, moves)
}

/// A plugin the masterlist says needs another that isn't on (its `req`), or one that can't be used with another on
/// (its `inc`): Health's lines.
pub fn masterlist_warnings(list: &[P], rules: &BTreeMap<String, Rule>) -> Vec<String> {
    let on: BTreeSet<String> = list.iter().filter(|p| p.active).map(|p| p.name.to_lowercase()).collect();
    let mut out = Vec::new();
    for p in list.iter().filter(|p| p.active) {
        if let Some(r) = rules.get(&p.name.to_lowercase()) {
            for q in &r.req {
                if !q.contains('/') && !on.contains(&q.to_lowercase()) {
                    out.push(format!("{} needs {} (LOOT's masterlist)", p.name, q));
                }
            }
            for q in &r.inc {
                if on.contains(&q.to_lowercase()) {
                    out.push(format!("{} and {} can't be on together (LOOT's masterlist)", p.name, q));
                }
            }
        }
    }
    out
}

// ------------------------------------------------------------------ snapshots

/// A snapshot of a whole setup: the profile's lists and INIs, each mod's version, kept beside the instance's Mod
/// Organizer 2 files (`.hearth/snapshots/<time>/`). Tiny: mod files are never copied (they stay in mods/ and on the
/// home server).
#[derive(Debug, Clone, Default)]
pub struct Snap {
    pub dir: PathBuf,
    pub at: i64,
    pub why: String,
    pub name: String,
    pub profile: String,
    pub mods: usize,
    pub by_hand: bool,
    pub crashed: bool,
}

pub const SNAP_FILES: &[&str] = &["modlist.txt", "plugins.txt", "loadorder.txt", "settings.ini"];

pub fn snaps_dir(instance: &Path) -> PathBuf {
    instance.join(".hearth").join("snapshots")
}

fn write_atomic(p: &Path, b: &[u8]) -> Result<(), String> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let tmp = p.with_extension("hearth-tmp");
    std::fs::write(&tmp, b).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, p).map_err(|e| e.to_string())
}

/// The mods' versions (meta.ini's `version`), by folder name.
pub fn versions(mods_dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Ok(rd) = std::fs::read_dir(mods_dir) {
        for e in rd.flatten() {
            let ini = std::fs::read_to_string(e.path().join("meta.ini")).unwrap_or_default();
            let v = ini.lines().find_map(|l| l.trim().strip_prefix("version=")).unwrap_or("").trim().to_string();
            out.insert(e.file_name().to_string_lossy().to_string(), v);
        }
    }
    out
}

/// Takes a snapshot of the profile now. `why` says what's about to change ("before Lux went in").
pub fn take(instance: &Path, profile_dir: &Path, mods_dir: &Path, why: &str, name: &str, by_hand: bool, now: i64) -> Result<Snap, String> {
    let dir = snaps_dir(instance).join(now.to_string());
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't make the snapshot's folder ({e})"))?;
    for f in SNAP_FILES {
        if let Ok(b) = std::fs::read(profile_dir.join(f)) {
            write_atomic(&dir.join(f), &b)?;
        }
    }
    // the profile's own INIs (Skyrim.ini, SkyrimPrefs.ini…)
    if let Ok(rd) = std::fs::read_dir(profile_dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.to_lowercase().ends_with(".ini") && n != "settings.ini" {
                if let Ok(b) = std::fs::read(e.path()) {
                    write_atomic(&dir.join("inis").join(&n), &b)?;
                }
            }
        }
    }
    let list = std::fs::read_to_string(profile_dir.join("modlist.txt")).unwrap_or_default();
    let on = list.lines().filter(|l| l.starts_with('+') && !l.ends_with("_separator")).count();
    let profile = profile_dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let meta = json!({"at": now, "why": why, "name": name, "profile": profile, "mods": on, "byHand": by_hand, "versions": versions(mods_dir)});
    write_atomic(&dir.join("snapshot.json"), serde_json::to_string_pretty(&meta).unwrap_or_default().as_bytes())?;
    Ok(Snap { dir, at: now, why: why.into(), name: name.into(), profile, mods: on, by_hand, crashed: false })
}

/// Every snapshot, newest first.
pub fn list(instance: &Path) -> Vec<Snap> {
    let mut v: Vec<Snap> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(snaps_dir(instance)) {
        for e in rd.flatten() {
            let m: Value = std::fs::read_to_string(e.path().join("snapshot.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
            if m.is_null() {
                continue;
            }
            v.push(Snap {
                dir: e.path(),
                at: m["at"].as_i64().unwrap_or(0),
                why: m["why"].as_str().unwrap_or("").into(),
                name: m["name"].as_str().unwrap_or("").into(),
                profile: m["profile"].as_str().unwrap_or("").into(),
                mods: m["mods"].as_u64().unwrap_or(0) as usize,
                by_hand: m["byHand"].as_bool().unwrap_or(false),
                crashed: m["crashed"].as_bool().unwrap_or(false),
            });
        }
    }
    v.sort_by(|a, b| b.at.cmp(&a.at));
    v
}

fn modlist_state(text: &str) -> BTreeMap<String, bool> {
    text.lines().filter(|l| (l.starts_with('+') || l.starts_with('-')) && !l.ends_with("_separator")).map(|l| (l[1..].to_string(), l.starts_with('+'))).collect()
}

/// What going back to `s` changes, line by line (board 4's "Going back to … changes").
pub fn diff(s: &Snap, profile_dir: &Path, mods_dir: &Path) -> Vec<(String, String)> {
    let then = modlist_state(&std::fs::read_to_string(s.dir.join("modlist.txt")).unwrap_or_default());
    let now = modlist_state(&std::fs::read_to_string(profile_dir.join("modlist.txt")).unwrap_or_default());
    let off: Vec<&String> = now.iter().filter(|(k, on)| **on && !then.get(*k).copied().unwrap_or(false)).map(|(k, _)| k).collect();
    let back_on: Vec<&String> = then.iter().filter(|(k, on)| **on && !now.get(*k).copied().unwrap_or(false)).map(|(k, _)| k).collect();
    let mut out = Vec::new();
    let names = |v: &[&String]| -> String {
        let mut s = v.iter().take(2).map(|x| x.as_str()).collect::<Vec<_>>().join(", ");
        if v.len() > 2 {
            s.push_str(&format!(" and {} more", v.len() - 2));
        }
        s
    };
    let mut m = Vec::new();
    if !off.is_empty() {
        m.push(format!("{} off · kept, not deleted", names(&off)));
    }
    if !back_on.is_empty() {
        m.push(format!("{} back on", names(&back_on)));
    }
    out.push(("Mods".into(), if m.is_empty() { "the same".into() } else { m.join(" · ") }));
    let meta: Value = std::fs::read_to_string(s.dir.join("snapshot.json")).ok().and_then(|x| serde_json::from_str(&x).ok()).unwrap_or(Value::Null);
    let cur = versions(mods_dir);
    let mut vs = Vec::new();
    if let Some(o) = meta["versions"].as_object() {
        for (k, v) in o {
            let v = v.as_str().unwrap_or("");
            if let Some(c) = cur.get(k) {
                if !v.is_empty() && c != v {
                    vs.push(format!("{k} back to {v}"));
                }
            }
        }
    }
    out.push(("Versions".into(), if vs.is_empty() { "the same".into() } else { vs.into_iter().take(2).collect::<Vec<_>>().join(", ") }));
    let lo = |p: &Path| std::fs::read_to_string(p).unwrap_or_default().lines().map(str::to_string).collect::<Vec<_>>();
    let (a, b) = (lo(&s.dir.join("loadorder.txt")), lo(&profile_dir.join("loadorder.txt")));
    let moved = a.iter().enumerate().filter(|(i, n)| b.get(*i) != Some(*n)).count();
    out.push(("Load order".into(), if moved == 0 { "the same".into() } else { format!("{moved} plugins back where they were") }));
    let mut ini = Vec::new();
    if let Ok(rd) = std::fs::read_dir(s.dir.join("inis")) {
        for e in rd.flatten() {
            let then = std::fs::read_to_string(e.path()).unwrap_or_default();
            let now = std::fs::read_to_string(profile_dir.join(e.file_name())).unwrap_or_default();
            let n = then.lines().zip(now.lines()).filter(|(x, y)| x != y).count() + then.lines().count().abs_diff(now.lines().count());
            if n > 0 {
                ini.push(format!("{}: {n} line{} back", e.file_name().to_string_lossy(), if n == 1 { "" } else { "s" }));
            }
        }
    }
    out.push(("Profile INIs".into(), if ini.is_empty() { "the same".into() } else { ini.join(", ") }));
    out
}

/// Goes back to `s`: tonight's setup becomes a snapshot first (so you can come forward again), then the profile's
/// lists and INIs are put back as they were. Mod files are never deleted.
pub fn restore(s: &Snap, instance: &Path, profile_dir: &Path, mods_dir: &Path, now: i64) -> Result<Snap, String> {
    let before = take(instance, profile_dir, mods_dir, &format!("before going back to {}", if s.name.is_empty() { "an earlier setup" } else { s.name.as_str() }), "", false, now)?;
    for f in SNAP_FILES {
        if let Ok(b) = std::fs::read(s.dir.join(f)) {
            write_atomic(&profile_dir.join(f), &b)?;
        }
    }
    if let Ok(rd) = std::fs::read_dir(s.dir.join("inis")) {
        for e in rd.flatten() {
            if let Ok(b) = std::fs::read(e.path()) {
                write_atomic(&profile_dir.join(e.file_name()), &b)?;
            }
        }
    }
    Ok(before)
}

// ------------------------------------------------------------------ saves

/// LZ4's block format (what Skyrim SE compresses its saves with).
pub fn lz4_block(src: &[u8], out_len: usize) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(out_len);
    let mut i = 0usize;
    while i < src.len() {
        let token = src[i];
        i += 1;
        let mut lit = (token >> 4) as usize;
        if lit == 15 {
            loop {
                let b = *src.get(i)?;
                i += 1;
                lit += b as usize;
                if b != 255 {
                    break;
                }
            }
        }
        out.extend_from_slice(src.get(i..i + lit)?);
        i += lit;
        if i >= src.len() || out.len() >= out_len {
            break;
        }
        let off = u16::from_le_bytes([*src.get(i)?, *src.get(i + 1)?]) as usize;
        i += 2;
        if off == 0 || off > out.len() {
            return None;
        }
        let mut ml = (token & 15) as usize;
        if ml == 15 {
            loop {
                let b = *src.get(i)?;
                i += 1;
                ml += b as usize;
                if b != 255 {
                    break;
                }
            }
        }
        ml += 4;
        let start = out.len() - off;
        for k in 0..ml {
            let b = out[start + k];
            out.push(b);
        }
        if out.len() > out_len {
            return None;
        }
    }
    Some(out)
}

/// What a Skyrim SE save says about itself.
#[derive(Debug, Clone, Default)]
pub struct Save {
    pub name: String,
    pub level: u32,
    pub place: String,
    pub plugins: Vec<String>,
}

/// Reads a Skyrim Special Edition save (.ess): the player, the place and the plugins it was made with.
pub fn read_save(b: &[u8]) -> Option<Save> {
    let magic = b"TESV_SAVEGAME";
    if !b.starts_with(magic) {
        return None;
    }
    let mut i = magic.len();
    let u32_at = |b: &[u8], i: usize| -> Option<u32> { Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?)) };
    let u16_at = |b: &[u8], i: usize| -> Option<u16> { Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?)) };
    let wstr = |b: &[u8], i: &mut usize| -> Option<String> {
        let n = u16_at(b, *i)? as usize;
        *i += 2;
        let s = String::from_utf8_lossy(b.get(*i..*i + n)?).to_string();
        *i += n;
        Some(s)
    };
    let _header_size = u32_at(b, i)?;
    i += 4;
    let version = u32_at(b, i)?;
    i += 8; // version, save number
    let name = wstr(b, &mut i)?;
    let level = u32_at(b, i)?;
    i += 4;
    let place = wstr(b, &mut i)?;
    let _date = wstr(b, &mut i)?;
    let _race = wstr(b, &mut i)?;
    i += 2 + 4 + 4 + 8; // sex, exp, level-up exp, file time
    let w = u32_at(b, i)? as usize;
    let h = u32_at(b, i + 4)? as usize;
    i += 8;
    let compression = if version >= 12 {
        let c = u16_at(b, i)?;
        i += 2;
        c
    } else {
        0
    };
    i += w * h * if version >= 12 { 4 } else { 3 };
    let body: Vec<u8> = if compression == 2 {
        let ulen = u32_at(b, i)? as usize;
        let clen = u32_at(b, i + 4)? as usize;
        lz4_block(b.get(i + 8..i + 8 + clen)?, ulen)?
    } else if compression == 0 {
        b.get(i..)?.to_vec()
    } else {
        return Some(Save { name, level, place, plugins: Vec::new() });
    };
    // form version, plugin info size, the plugins
    let mut j = 1 + 4;
    let n = *body.get(j)? as usize;
    j += 1;
    let mut plugins = Vec::new();
    for _ in 0..n {
        plugins.push(wstr(&body, &mut j)?);
    }
    Some(Save { name, level, place, plugins })
}

// ------------------------------------------------------------------ crash logs

/// A crash log read in plain words (board 5).
#[derive(Debug, Clone, Default)]
pub struct Crash {
    pub file: PathBuf,
    pub game: String,
    pub logger: String,
    pub exception: String,
    pub at: String,
    /// the modules named in the call stack, top first, without the game's own and Windows'
    pub suspects: Vec<String>,
    /// lines that matter (their numbers in `lines`)
    pub marked: BTreeSet<usize>,
    pub lines: Vec<String>,
    pub objects: Vec<String>,
    pub plugins: Option<(usize, usize)>,
}

const SYSTEM: &[&str] = &["skyrimse.exe", "skyrimvr.exe", "fallout4.exe", "fallout4vr.exe", "ntdll.dll", "kernelbase.dll", "kernel32.dll", "ucrtbase.dll", "msvcp140.dll", "vcruntime140.dll", "user32.dll", "win32u.dll", "d3d11.dll", "dxgi.dll", "dxvk", "wined3d", "crashlogger.dll", "skse64_1_6_1170.dll", "skse64_loader.exe", "f4se_1_10_163.dll"];

fn is_system(m: &str) -> bool {
    let l = m.to_lowercase();
    SYSTEM.iter().any(|s| l == *s || l.starts_with(s)) || l.starts_with("api-ms-") || l.starts_with("nvwgf") || l.starts_with("amdxx")
}

/// The module in "SkyUI_SE.dll+0024410" or "(SkyUI_SE.dll+0024410)".
fn module_of(s: &str) -> Option<String> {
    for w in s.split(|c: char| c.is_whitespace() || c == '(' || c == ')') {
        if let Some((m, _)) = w.split_once('+') {
            let l = m.to_lowercase();
            if l.ends_with(".dll") || l.ends_with(".exe") {
                return Some(m.to_string());
            }
        }
    }
    None
}

/// Reads a Crash Logger SSE / F4SE log, a Trainwreck log or a .NET Script Framework crash log.
pub fn read_crash(text: &str) -> Crash {
    let mut c = Crash { lines: text.lines().map(|l| l.replace('\t', "  ")).collect(), ..Default::default() };
    let mut sect = String::new();
    for (n, l) in c.lines.clone().iter().enumerate() {
        let t = l.trim();
        if n < 3 && c.game.is_empty() && (t.starts_with("Skyrim") || t.starts_with("Fallout")) {
            c.game = t.to_string();
        }
        if t.to_lowercase().contains("crashlogger") || t.starts_with("Trainwreck") || t.contains("NET Script Framework") {
            if c.logger.is_empty() {
                c.logger = t.to_string();
            }
        }
        if let Some(rest) = t.strip_prefix("Unhandled exception").or_else(|| t.strip_prefix("Unhandled native exception occurred at")) {
            c.exception = rest.split('"').nth(1).unwrap_or("").to_string();
            c.at = module_of(rest).unwrap_or_default();
            c.marked.insert(n);
            continue;
        }
        if t.ends_with(':') && t.chars().next().map(|x| x.is_ascii_uppercase()).unwrap_or(false) && t.len() < 40 {
            sect = t.trim_end_matches(':').to_uppercase();
            continue;
        }
        let l2 = t.to_lowercase();
        if sect.contains("CALL STACK") || sect.contains("CALLSTACK") {
            if let Some(m) = module_of(t) {
                if !is_system(&m) {
                    if !c.suspects.iter().any(|s| s.eq_ignore_ascii_case(&m)) {
                        c.suspects.push(m);
                    }
                    c.marked.insert(n);
                } else if c.marked.is_empty() || n == c.marked.iter().next_back().copied().unwrap_or(0) + 1 {
                    // the frame where it stopped
                    if c.suspects.is_empty() {
                        c.marked.insert(n);
                    }
                }
            }
        } else if sect.contains("RELEVANT OBJECTS") && !t.is_empty() {
            c.objects.push(t.to_string());
            if c.objects.len() == 1 {
                c.marked.insert(n);
            }
        } else if sect.starts_with("PLUGINS") && c.plugins.is_none() {
            let _ = l2;
        }
    }
    // PLUGINS: the count, and missing masters if the logger says so
    let pcount = c.lines.iter().skip_while(|l| !l.trim().to_uppercase().starts_with("PLUGINS")).skip(1).take_while(|l| !l.trim().is_empty() && !l.trim().ends_with(':')).count();
    if pcount > 0 {
        c.plugins = Some((pcount, 0));
    }
    c
}

/// Which mod has this module (a mod's SKSE/Plugins/<dll>), from the organizer's mods folder.
pub fn mod_with_file(mods_dir: &Path, module: &str) -> Option<String> {
    let want = module.to_lowercase();
    let rd = std::fs::read_dir(mods_dir).ok()?;
    for e in rd.flatten() {
        for sub in ["SKSE/Plugins", "skse/plugins", "F4SE/Plugins", "f4se/plugins", "SKSE/plugins"] {
            if let Ok(r2) = std::fs::read_dir(e.path().join(sub)) {
                if r2.flatten().any(|f| f.file_name().to_string_lossy().to_lowercase() == want) {
                    return Some(e.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

/// The exception's name in plain words.
pub fn exception_words(e: &str) -> &'static str {
    match e {
        "EXCEPTION_ACCESS_VIOLATION" => "read memory it shouldn't have",
        "EXCEPTION_STACK_OVERFLOW" => "ran out of room for its work (a loop that never ended)",
        "EXCEPTION_ILLEGAL_INSTRUCTION" => "ran an instruction this processor doesn't have",
        "EXCEPTION_INT_DIVIDE_BY_ZERO" => "divided by zero",
        _ => "stopped on an error",
    }
}

/// Crash logs, newest first: Crash Logger's and .NET Script Framework's folders in the game's prefix (My Games) and
/// in Hearth's own crash folder.
pub fn crash_logs(roots: &[PathBuf]) -> Vec<(PathBuf, std::time::SystemTime)> {
    let mut v = Vec::new();
    for r in roots {
        if let Ok(rd) = std::fs::read_dir(r) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_lowercase();
                if (n.starts_with("crash-") || n.starts_with("crash_") || n.contains("crashlog")) && (n.ends_with(".log") || n.ends_with(".txt")) {
                    if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                        v.push((e.path(), m));
                    }
                }
            }
        }
    }
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    const ML: &str = "groups:\n  - name: 'Late Loaders'\nplugins:\n  - name: 'Lux - Ordinator Patch.esp'\n    after: [ 'Ordinator - Perks of Skyrim.esp' ]\n    req: [ 'Lux.esp' ]\n  - name: 'Alternate Start - Live Another Life.esp'\n    after:\n      - 'unofficial skyrim special edition patch.esp'\n    msg:\n      - <<: *patchIncluded\n  - name: 'Cathedral Landscapes.esp'\n    group: 'Late Loaders'\n    inc: [ 'Old Grass.esp' ]\n";

    fn p(n: &str, master: bool, masters: &[&str]) -> P {
        P { name: n.into(), active: true, master, implicit: false, masters: masters.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn masterlist_rules() {
        let r = parse_masterlist(ML);
        assert_eq!(r["lux - ordinator patch.esp"].after, vec!["Ordinator - Perks of Skyrim.esp"]);
        assert_eq!(r["lux - ordinator patch.esp"].req, vec!["Lux.esp"]);
        assert_eq!(r["alternate start - live another life.esp"].after, vec!["unofficial skyrim special edition patch.esp"]);
        assert_eq!(r["cathedral landscapes.esp"].group, "Late Loaders");
        assert!(!r.contains_key("late loaders"));
    }

    #[test]
    fn sort_keeps_your_order() {
        let rules = parse_masterlist(ML);
        let list = vec![
            p("unofficial skyrim special edition patch.esp", true, &[]),
            p("Lux - Ordinator Patch.esp", false, &["Lux.esp"]),
            p("Alternate Start - Live Another Life.esp", false, &[]),
            p("Lux.esp", false, &[]),
            p("Ordinator - Perks of Skyrim.esp", false, &[]),
            p("Mine.esp", false, &[]),
        ];
        let (order, moves) = sort(&list, &rules);
        let at = |n: &str| order.iter().position(|x| x == n).unwrap();
        assert!(at("Lux - Ordinator Patch.esp") > at("Lux.esp"));
        assert!(at("Lux - Ordinator Patch.esp") > at("Ordinator - Perks of Skyrim.esp"));
        assert!(at("Alternate Start - Live Another Life.esp") < at("Lux.esp"), "kept where you put it");
        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].plugin, "Lux - Ordinator Patch.esp");
        assert_eq!(at("Mine.esp"), 5);
    }

    #[test]
    fn crash_log_in_plain_words() {
        let log = "Skyrim SSE v1.6.1170\nCrashLoggerSSE v1-15\n\nUnhandled exception \"EXCEPTION_ACCESS_VIOLATION\" at 0x7FF6A41C2D10 SkyrimSE.exe+0C42D10\n\nPROBABLE CALL STACK:\n\t[0] 0x7FF6A41C2D10 SkyrimSE.exe+0C42D10\n\t[1] 0x7FFB1E3A4410 SkyUI_SE.dll+0024410\n\t[2] 0x7FFB1E3A1B02 SkyUI_SE.dll+0021B02\n\t[3] 0x7FFB1E3A1B02 MCMHelper.dll+0001B02\n\nPOSSIBLE RELEVANT OBJECTS:\n\tMagicMenu (open)\n\nPLUGINS:\n\tSkyrim.esm\n\tUpdate.esm\n";
        let c = read_crash(log);
        assert_eq!(c.exception, "EXCEPTION_ACCESS_VIOLATION");
        assert_eq!(c.suspects, vec!["SkyUI_SE.dll", "MCMHelper.dll"]);
        assert_eq!(c.objects, vec!["MagicMenu (open)"]);
        assert_eq!(c.plugins, Some((2, 0)));
        assert!(c.marked.contains(&3));
    }

    #[test]
    fn lz4_round() {
        // "abcabcabcabc": 3 literals, then a match of 9 at offset 3
        let src = [0x35, b'a', b'b', b'c', 3, 0];
        assert_eq!(lz4_block(&src, 12).unwrap(), b"abcabcabcabc");
    }

    #[test]
    fn snapshots_go_back() {
        let d = tempfile::tempdir().unwrap();
        let (inst, prof, mods) = (d.path().join("i"), d.path().join("i/profiles/P"), d.path().join("i/mods"));
        std::fs::create_dir_all(&prof).unwrap();
        std::fs::create_dir_all(mods.join("Lux")).unwrap();
        std::fs::write(mods.join("Lux/meta.ini"), "[General]\nversion=5.4\n").unwrap();
        std::fs::write(prof.join("modlist.txt"), "+SkyUI\n-Lux\n").unwrap();
        std::fs::write(prof.join("Skyrim.ini"), "a=1\n").unwrap();
        let s = take(&inst, &prof, &mods, "before Lux went in", "Tuesday's setup", false, 100).unwrap();
        std::fs::write(prof.join("modlist.txt"), "+SkyUI\n+Lux\n").unwrap();
        std::fs::write(prof.join("Skyrim.ini"), "a=2\n").unwrap();
        let d2 = diff(&s, &prof, &mods);
        assert!(d2[0].1.contains("Lux off"), "{d2:?}");
        assert!(d2[3].1.contains("Skyrim.ini: 1 line back"), "{d2:?}");
        restore(&s, &inst, &prof, &mods, 200).unwrap();
        assert_eq!(std::fs::read_to_string(prof.join("modlist.txt")).unwrap(), "+SkyUI\n-Lux\n");
        assert_eq!(list(&inst).len(), 2, "tonight's setup is a snapshot too");
    }
}

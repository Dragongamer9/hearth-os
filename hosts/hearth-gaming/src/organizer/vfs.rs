//! The organizer's view of the game's Data folder, without changing the game's files.
//!
//! MO2 builds its view with USVFS (hooking the game's file calls on Windows). On Linux Hearth builds it for real:
//! - resolve(): every enabled mod's files merged by priority (higher wins), matched without regard to case the way
//!   Windows (and Wine) see them, using the game's own spelling of folders it already has; `.mohidden` files and
//!   folders and each mod's meta.ini are left out;
//! - overlay (preferred): the winners are hard-linked into a layer (instance/.hearth/layer) and fuse-overlayfs lays
//!   it over the game's Data folder, with the instance's overwrite folder on top for whatever the game writes, so
//!   the game folder itself is never changed;
//! - links (fallback, when fuse-overlayfs isn't there or won't mount): mods.rs's link deploy puts the winners in the
//!   Data folder, setting aside every game file it replaces; un-deploying puts them all back and moves anything the
//!   game wrote there into overwrite, leaving the folder as it was.
//! Profile files (plugins.txt, the profile's own inis and saves) are swapped into the game's Proton folder while it
//! runs and put back afterwards. What's deployed is written down first (instance/.hearth/active.json), so a crash
//! is undone at the next run or at sign-in (hearth-mods --remount).

use super::Instance;
use crate::mods::{self, overlay, Ledger, Want};
use crate::util;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ case-insensitive paths

/// A child of `dir` named `name` ignoring case (the exact name first).
pub fn ci_find(dir: &Path, name: &str) -> Option<PathBuf> {
    if name.is_empty() { return Some(dir.to_path_buf()); }
    let exact = dir.join(name);
    if std::fs::symlink_metadata(&exact).is_ok() { return Some(exact); }
    let rd = std::fs::read_dir(dir).ok()?;
    for e in rd.flatten() { if e.file_name().to_string_lossy().eq_ignore_ascii_case(name) { return Some(e.path()); } }
    None
}
/// `rel` under `base`, each part matched ignoring case; None when a part isn't there.
pub fn ci_resolve(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for part in rel.split(['/', '\\']).filter(|p| !p.is_empty() && *p != ".") {
        if part == ".." { return None; }
        cur = ci_find(&cur, part)?;
    }
    Some(cur)
}
/// `rel` under `base`, reusing the spelling of the parts that are already there (the rest as given).
pub fn ci_join(base: &Path, rel: &str) -> PathBuf {
    let mut cur = base.to_path_buf();
    let mut found = true;
    for part in rel.split(['/', '\\']).filter(|p| !p.is_empty() && *p != "." && *p != "..") {
        if found { if let Some(p) = ci_find(&cur, part) { cur = p; continue; } found = false; }
        cur.push(part);
    }
    cur
}

// ------------------------------------------------------------------ the merged view

/// Something with files for the view: a mod's folder, or overwrite (always last, so it wins).
#[derive(Debug, Clone)]
pub struct Source { pub name: String, pub dir: PathBuf }

#[derive(Debug, Clone)]
pub struct Entry {
    /// The path in the view, in its one spelling.
    pub rel: String,
    /// The source that wins it, and the ones it beats (lowest first).
    pub winner: usize,
    pub losers: Vec<usize>,
    pub src: PathBuf,
}

#[derive(Debug, Default)]
pub struct View {
    /// lower-case path → entry.
    pub files: BTreeMap<String, Entry>,
    /// Files left out with .mohidden: (source, path).
    pub hidden: Vec<(usize, String)>,
    /// The game's own files in the folder: lower-case path → its spelling.
    pub base: BTreeMap<String, String>,
}

fn is_hidden(rel: &str) -> bool { rel.split('/').any(|p| p.to_ascii_lowercase().ends_with(".mohidden")) }

/// The one spelling of a path: folders as first seen (the game's own first), the file as the game has it.
fn canonical(dirs: &mut HashMap<String, String>, base: &BTreeMap<String, String>, rel: &str) -> String {
    let parts: Vec<&str> = rel.split('/').filter(|p| !p.is_empty()).collect();
    let mut out = String::new();
    for (i, p) in parts.iter().enumerate() {
        let cand = if out.is_empty() { p.to_string() } else { format!("{out}/{p}") };
        let key = cand.to_lowercase();
        out = if i + 1 == parts.len() { base.get(&key).cloned().unwrap_or(cand) } else { dirs.entry(key).or_insert(cand).clone() };
    }
    out
}

fn rel_files(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    util::files_under(dir, &mut files);
    files.into_iter().filter_map(|f| { let r = f.strip_prefix(dir).ok()?.to_string_lossy().replace('\\', "/"); if r.is_empty() { None } else { Some((r, f)) } }).collect()
}

/// Merges the sources (lowest priority first) over the game's folder `base`.
pub fn resolve(base: Option<&Path>, sources: &[Source]) -> View {
    let mut dirs: HashMap<String, String> = HashMap::new();
    let mut view = View::default();
    if let Some(b) = base {
        let none = BTreeMap::new();
        for (rel, _) in rel_files(b) {
            let c = canonical(&mut dirs, &none, &rel);
            view.base.insert(c.to_lowercase(), c);
        }
    }
    for (i, s) in sources.iter().enumerate() {
        for (rel, f) in rel_files(&s.dir) {
            if rel.eq_ignore_ascii_case("meta.ini") { continue; }
            if is_hidden(&rel) { view.hidden.push((i, rel)); continue; }
            let c = canonical(&mut dirs, &view.base, &rel);
            let key = c.to_lowercase();
            match view.files.get_mut(&key) {
                Some(e) => { let w = e.winner; e.losers.push(w); e.winner = i; e.src = f; }
                None => { view.files.insert(key, Entry { rel: c, winner: i, losers: Vec::new(), src: f }); }
            }
        }
    }
    view
}

/// Each source's conflicts: the sources it beats, the ones it loses to, and how many files each way.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Conflicts { pub wins_over: BTreeSet<usize>, pub loses_to: BTreeSet<usize>, pub winning: usize, pub losing: usize, pub files: usize, pub hidden: usize }

pub fn conflicts(view: &View, n: usize) -> Vec<Conflicts> {
    let mut out = vec![Conflicts::default(); n];
    for e in view.files.values() {
        if e.winner < n {
            out[e.winner].files += 1;
            if !e.losers.is_empty() { out[e.winner].winning += 1; out[e.winner].wins_over.extend(e.losers.iter().copied()); }
        }
        for &l in &e.losers { if l < n { out[l].files += 1; out[l].losing += 1; out[l].loses_to.insert(e.winner); } }
    }
    for (i, _) in &view.hidden { if *i < n { out[*i].hidden += 1; } }
    out
}

/// One source's files with who wins each (for the conflicts screen).
pub fn files_of(view: &View, i: usize, names: &[String]) -> Value {
    let name = |j: usize| names.get(j).cloned().unwrap_or_default();
    let mut list: Vec<Value> = view.files.values().filter(|e| e.winner == i || e.losers.contains(&i)).map(|e| {
        if e.winner == i { json!({"path": e.rel, "status": if e.losers.is_empty() { "alone" } else { "wins" }, "over": e.losers.iter().map(|&j| name(j)).collect::<Vec<_>>(), "replacesGameFile": view.base.contains_key(&e.rel.to_lowercase())}) }
        else { json!({"path": e.rel, "status": "loses", "to": name(e.winner)}) }
    }).collect();
    list.extend(view.hidden.iter().filter(|(j, _)| *j == i).map(|(_, r)| json!({"path": r, "status": "hidden"})));
    Value::Array(list)
}

/// Links the view's winners (all but `skip`) into `layer`, made afresh. Hard links (no space, instant), symbolic
/// links where hard links can't go (another drive).
pub fn build_layer(view: &View, skip: Option<usize>, layer: &Path) -> Result<usize, String> {
    if layer.exists() { std::fs::remove_dir_all(layer).map_err(|e| format!("Couldn't clear the old view ({e})"))?; }
    std::fs::create_dir_all(layer).map_err(|e| e.to_string())?;
    let mut n = 0;
    for e in view.files.values() {
        if Some(e.winner) == skip { continue; }
        let Some(rel) = util::clean_rel(&e.rel) else { continue };
        let dest = layer.join(rel);
        if let Some(d) = dest.parent() { std::fs::create_dir_all(d).map_err(|er| format!("Couldn't build the view ({er})"))?; }
        if std::fs::hard_link(&e.src, &dest).is_err() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(&e.src, &dest).map_err(|er| format!("Couldn't link {} ({er})", e.rel))?;
            #[cfg(not(unix))]
            std::fs::copy(&e.src, &dest).map_err(|er| format!("Couldn't copy {} ({er})", e.rel))?;
        }
        n += 1;
    }
    Ok(n)
}

// ------------------------------------------------------------------ deploying

/// What's deployed, written before anything is changed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Active {
    /// overlay or hardlink.
    pub mode: String,
    /// The game folder the view is on (its Data folder).
    pub target: PathBuf,
    /// The hearth-mods that runs the game (0: deployed by hand).
    pub pid: u32,
    pub since: u64,
    pub profile: String,
    pub swaps: Vec<Swap>,
    /// Links: what's in place, and every file and folder that was in the target before.
    pub ledger: Ledger,
    pub base: Vec<String>,
    pub notes: Vec<String>,
}

/// A file (or the saves folder) of the profile put in the game's place for the run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Swap {
    pub path: PathBuf,
    /// Where the game's own file went meanwhile.
    pub backup: Option<PathBuf>,
    /// Copied back here afterwards (the profile's own inis, which the game may change).
    pub back_to: Option<PathBuf>,
    /// A link to the profile's saves folder.
    pub link: bool,
}

fn active_file(inst: &Instance) -> PathBuf { inst.hearth_dir().join("active.json") }
pub fn read_active(inst: &Instance) -> Option<Active> {
    std::fs::read(active_file(inst)).ok().and_then(|b| serde_json::from_slice(&b).ok())
}
fn write_active(inst: &Instance, a: &Active) -> Result<(), String> {
    let f = active_file(inst);
    if let Some(d) = f.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    util::write_atomic(&f, serde_json::to_string_pretty(a).unwrap_or_default().as_bytes()).map_err(|e| format!("Couldn't note what's deployed ({e})"))
}

/// Is the hearth-mods that deployed it still running?
pub fn alive(pid: u32) -> bool {
    if pid == 0 { return false; }
    if pid == std::process::id() { return true; }
    std::fs::read(format!("/proc/{pid}/cmdline")).map(|c| String::from_utf8_lossy(&c).contains("hearth-mods")).unwrap_or(false)
}

pub fn status(inst: &Instance) -> Value {
    match read_active(inst) {
        None => Value::Null,
        Some(a) => json!({"mode": a.mode, "target": a.target, "since": a.since, "profile": a.profile, "running": alive(a.pid), "byHand": a.pid == 0,
                          "mounted": a.mode == "overlay" && overlay::mounted(Some(&a.target)), "notes": a.notes}),
    }
}

/// Every file and folder under `dir` (folders end in /), links not followed.
fn list_tree(dir: &Path) -> Vec<String> {
    fn walk(d: &Path, root: &Path, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(t) = e.file_type() else { continue };
            let rel = p.strip_prefix(root).map(|r| r.to_string_lossy().to_string()).unwrap_or_default();
            if t.is_dir() { out.push(format!("{rel}/")); walk(&p, root, out); } else { out.push(rel); }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// Puts the profile's view in place. `pid`: the hearth-mods running the game (0: by hand). `overlay_ok`: try
/// fuse-overlayfs first.
pub fn deploy(inst: &Instance, pid: u32, overlay_ok: bool) -> Result<Active, String> {
    if let Some(a) = read_active(inst) {
        if alive(a.pid) { return Err("The game is running through the organizer: close it first.".into()); }
        // Left from before (by hand, or a crash): taken out first.
        undeploy(inst, false)?;
    }
    let target = inst.data_target().ok_or("The game's folder isn't there. Check where the game is (or install it first).")?;
    if overlay::mounted(Some(&target)) { return Err("Something is already laid over the game's folder (Hearth Mods' own overlay?). Turn that off first.".into()); }
    if let Some(g) = inst.game_dir.as_deref() { if overlay::mounted(Some(g)) { return Err("Hearth Mods' overlay is on this game's folder: switch that game's mods to Links or turn them off first.".into()); } }
    let sources = inst.sources()?;
    let ow = sources.len().saturating_sub(1);
    let view = resolve(Some(&target), &sources);
    let hd = inst.hearth_dir();
    std::fs::create_dir_all(&hd).map_err(|e| e.to_string())?;
    let mut a = Active { target: target.clone(), pid, since: util::now_ms(), profile: inst.profile.clone(), ..Default::default() };
    if overlay_ok && overlay::available() {
        let layer = hd.join("layer");
        build_layer(&view, Some(ow), &layer)?;
        a.mode = "overlay".into();
        write_active(inst, &a)?;
        if let Err(e) = overlay::mount_at(&target, &[layer.clone(), target.clone()], &inst.overwrite, &hd.join("work")) {
            a.notes.push(format!("The overlay didn't work ({e}), so links are used instead."));
            a.mode.clear();
            let _ = std::fs::remove_dir_all(&layer);
        }
    }
    if a.mode.is_empty() {
        a.mode = "hardlink".into();
        a.base = list_tree(&target);
        write_active(inst, &a)?;
        let want: BTreeMap<String, Want> = view.files.values().filter_map(|e| {
            let rel = util::clean_rel(&e.rel)?;
            let name = sources.get(e.winner).map(|s| s.name.clone()).unwrap_or_default();
            Some((target.join(rel).to_string_lossy().to_string(), Want { by: name.clone(), label: name, src: e.src.clone(), root: target.clone() }))
        }).collect();
        let mut ledger = Ledger::default();
        let rep = {
            let snap = a.clone();
            mods::sync_files(&hd.join("deploy"), &mut ledger, &want, true, &mut |l: &Ledger| { let mut x = snap.clone(); x.ledger = l.clone(); let _ = write_active(inst, &x); })
        };
        a.ledger = ledger;
        a.notes.extend(rep.problems);
        write_active(inst, &a)?;
    }
    swap_profile_files(inst, &mut a);
    write_active(inst, &a)?;
    Ok(a)
}

/// Takes the view away and puts everything back. `patient`: the game may still be closing (keep trying to take the
/// overlay off for a while).
pub fn undeploy(inst: &Instance, patient: bool) -> Result<Value, String> {
    let Some(mut a) = read_active(inst) else { return Ok(json!({"undeployed": false})) };
    let hd = inst.hearth_dir();
    let mut notes = Vec::new();
    match a.mode.as_str() {
        "overlay" => {
            let tries = if patient { 120 } else { 3 };
            let mut n = 0;
            loop {
                match overlay::unmount(Some(&a.target)) {
                    Ok(()) => break,
                    Err(e) => { n += 1; if n >= tries { return Err(e); } std::thread::sleep(std::time::Duration::from_secs(1)); }
                }
            }
            let _ = std::fs::remove_dir_all(hd.join("layer"));
        }
        "hardlink" => {
            let mut ledger = std::mem::take(&mut a.ledger);
            let rep = mods::sync_files(&hd.join("deploy"), &mut ledger, &BTreeMap::new(), true, &mut |_: &Ledger| {});
            a.ledger = ledger;
            if !rep.kept_changed.is_empty() { notes.push(format!("The game changed {} of the mods' files in place; those copies are kept in {}", rep.kept_changed.len(), hd.join("deploy/changed").display())); }
            if !a.ledger.deployed.is_empty() || !a.ledger.originals.is_empty() {
                let _ = write_active(inst, &a);
                return Err(format!("Couldn't take every mod file out of the game folder: {}", rep.problems.join("; ")));
            }
            let moved = keep_new_files(&a.target, &a.base, &inst.overwrite);
            if moved > 0 { notes.push(format!("{moved} file{} the game wrote went to overwrite", if moved == 1 { "" } else { "s" })); }
        }
        _ => {}
    }
    let mut failed = Vec::new();
    let mut kept = Vec::new();
    for s in std::mem::take(&mut a.swaps).into_iter().rev() {
        if let Err(e) = restore_swap(&s) { failed.push(e); kept.insert(0, s); }
    }
    a.swaps = kept;
    if !failed.is_empty() {
        a.mode.clear();
        let _ = write_active(inst, &a);
        return Err(format!("Couldn't put the game's own files back in its Proton folder: {}", failed.join("; ")));
    }
    let _ = std::fs::remove_file(active_file(inst));
    Ok(json!({"undeployed": true, "mode": a.mode, "notes": notes}))
}

/// Files in `target` that weren't there before (and aren't mods'): moved into overwrite; folders made for them go.
fn keep_new_files(target: &Path, base: &[String], overwrite: &Path) -> usize {
    let before: BTreeSet<&str> = base.iter().map(|s| s.as_str()).collect();
    let now = list_tree(target);
    let mut moved = 0;
    for rel in now.iter().filter(|r| !r.ends_with('/') && !before.contains(r.as_str())) {
        let from = target.join(rel);
        let to = ci_join(overwrite, rel);
        if mods::move_file(&from, &to).is_ok() { moved += 1; }
    }
    let mut dirs: Vec<&String> = now.iter().filter(|r| r.ends_with('/') && !before.contains(r.as_str())).collect();
    dirs.sort_by_key(|d| std::cmp::Reverse(d.len()));
    for d in dirs { let _ = std::fs::remove_dir(target.join(d.trim_end_matches('/'))); }
    moved
}

// ------------------------------------------------------------------ profile files in the Proton folder

fn swap_profile_files(inst: &Instance, a: &mut Active) {
    let k = inst.kind;
    if k.folder.is_empty() { return; }
    let Some(user) = inst.prefix_user() else {
        if k.has_plugins() { a.notes.push("The game's Proton folder isn't there yet, so the profile's plugin list couldn't be given to it. Start the game once without mods, then again.".into()); }
        return;
    };
    let prof = inst.profile_dir();
    let hd = inst.hearth_dir().join("swap");
    let local = ci_join(&user, &format!("AppData/Local/{}", k.folder));
    if k.has_plugins() {
        swap_file(inst, a, &hd, &prof.join("plugins.txt"), &ci_join(&local, "Plugins.txt"), None);
        if !k.star { swap_file(inst, a, &hd, &prof.join("loadorder.txt"), &ci_join(&local, "loadorder.txt"), None); }
    }
    let set = inst.profile_settings();
    let mygames = ci_join(&user, &format!("Documents/My Games/{}", k.folder));
    if set.local_inis {
        for ini in k.inis { if let Some(src) = ci_find(&prof, ini) { if src.is_file() { swap_file(inst, a, &hd, &src, &ci_join(&mygames, ini), Some(src.clone())); } } }
    }
    if set.local_saves {
        let saves = prof.join("saves");
        let dest = ci_join(&mygames, "Saves");
        let r = (|| -> Result<Swap, String> {
            std::fs::create_dir_all(&saves).map_err(|e| e.to_string())?;
            if let Some(d) = dest.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
            let backup = if std::fs::symlink_metadata(&dest).is_ok() {
                let b = dest.with_file_name(format!("Saves.hearth-{}", util::now_ms()));
                std::fs::rename(&dest, &b).map_err(|e| e.to_string())?;
                Some(b)
            } else { None };
            #[cfg(unix)]
            if let Err(e) = std::os::unix::fs::symlink(&saves, &dest) { if let Some(b) = &backup { let _ = std::fs::rename(b, &dest); } return Err(e.to_string()); }
            Ok(Swap { path: dest.clone(), backup, back_to: None, link: true })
        })();
        match r { Ok(s) => { a.swaps.push(s); let _ = write_active(inst, a); } Err(e) => a.notes.push(format!("The profile's own saves couldn't be used ({e}).")) }
    }
}

fn swap_file(inst: &Instance, a: &mut Active, store: &Path, src: &Path, dest: &Path, back_to: Option<PathBuf>) {
    if !src.is_file() { return; }
    let r = (|| -> Result<Swap, String> {
        let backup = if std::fs::symlink_metadata(dest).is_ok() {
            let b = store.join(format!("{}-{}", a.swaps.len(), dest.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
            mods::move_file(dest, &b).map_err(|e| e.to_string())?;
            Some(b)
        } else { None };
        if let Err(e) = mods::copy_file(src, dest) { if let Some(b) = &backup { let _ = mods::move_file(b, dest); } return Err(e.to_string()); }
        Ok(Swap { path: dest.to_path_buf(), backup, back_to, link: false })
    })();
    match r {
        Ok(s) => { a.swaps.push(s); let _ = write_active(inst, a); }
        Err(e) => a.notes.push(format!("Couldn't give the game the profile's {} ({e}).", src.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())),
    }
}

fn restore_swap(s: &Swap) -> Result<(), String> {
    if s.link {
        if std::fs::symlink_metadata(&s.path).map(|m| m.file_type().is_symlink()).unwrap_or(false) { std::fs::remove_file(&s.path).map_err(|e| e.to_string())?; }
        if let Some(b) = &s.backup { if b.exists() { std::fs::rename(b, &s.path).map_err(|e| format!("{} ({e})", s.path.display()))?; } }
        return Ok(());
    }
    if let Some(back) = &s.back_to { if s.path.is_file() { let _ = mods::copy_file(&s.path, back); } }
    if std::fs::symlink_metadata(&s.path).is_ok() { std::fs::remove_file(&s.path).map_err(|e| format!("{} ({e})", s.path.display()))?; }
    if let Some(b) = &s.backup { if b.exists() { mods::move_file(b, &s.path).map_err(|e| format!("{} ({e})", s.path.display()))?; } }
    Ok(())
}

// ------------------------------------------------------------------ running

/// Something to start through the organizer (MO2's executables list).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Executable {
    pub title: String,
    pub binary: PathBuf,
    pub args: String,
    pub cwd: Option<PathBuf>,
    /// The script extender's loader (SKSE, F4SE...).
    pub extender: bool,
    /// "game" (Hearth found it) or "mo2" (the instance's ModOrganizer.ini).
    pub from: String,
    pub present: bool,
}
impl Executable {
    pub fn windows(&self) -> bool { self.binary.extension().map(|e| e.eq_ignore_ascii_case("exe") || e.eq_ignore_ascii_case("bat")).unwrap_or(false) }
}

/// The command that starts it: Windows programs with umu-run in the game's Proton folder (as Hearth starts Steam
/// games itself: GAMEID umu-<app>, STORE steam, PROTONPATH), Linux programs as they are.
pub fn command_for(inst: &Instance, ex: &Executable) -> (String, Vec<String>, Vec<(String, String)>, PathBuf) {
    let args: Vec<String> = split_args(&ex.args);
    let cwd = ex.cwd.clone().filter(|c| c.is_dir()).or_else(|| ex.binary.parent().map(Path::to_path_buf)).unwrap_or_else(|| PathBuf::from("/"));
    if !ex.windows() { return (ex.binary.to_string_lossy().to_string(), args, Vec::new(), cwd); }
    let mut env = Vec::new();
    let pfx = inst.wine_prefix();
    let compat = if pfx.file_name().map(|n| n == "pfx").unwrap_or(false) { pfx.parent().map(Path::to_path_buf).unwrap_or_else(|| pfx.clone()) } else { pfx.clone() };
    env.push(("WINEPREFIX".to_string(), pfx.to_string_lossy().to_string()));
    env.push(("STEAM_COMPAT_DATA_PATH".to_string(), compat.to_string_lossy().to_string()));
    let app = inst.steam_app.or(if inst.kind.steam > 0 { Some(inst.kind.steam) } else { None });
    env.push(("GAMEID".to_string(), app.map(|a| format!("umu-{a}")).unwrap_or_else(|| "umu-default".into())));
    if app.is_some() { env.push(("STORE".to_string(), "steam".to_string())); }
    env.push(("PROTONPATH".to_string(), std::env::var("HEARTH_PROTON").ok().filter(|p| !p.is_empty()).unwrap_or_else(|| "GE-Proton".into())));
    let mut a = vec![ex.binary.to_string_lossy().to_string()];
    a.extend(args);
    ("umu-run".to_string(), a, env, cwd)
}

/// "a \"b c\" d" → [a, b c, d]
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let (mut cur, mut q, mut any) = (String::new(), false, false);
    for c in s.chars() {
        match c {
            '"' => { q = !q; any = true; }
            ' ' | '\t' if !q => { if any || !cur.is_empty() { out.push(std::mem::take(&mut cur)); } any = false; }
            _ => cur.push(c),
        }
    }
    if any || !cur.is_empty() { out.push(cur); }
    out
}

fn has(cmd: &str) -> bool { std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(cmd).is_file())).unwrap_or(false) }

/// Processes (other than this one) working in the game's folder: their working folder or command line is in it.
fn game_processes(dir: &Path) -> Vec<u32> {
    let me = std::process::id();
    let want = dir.to_string_lossy().to_string();
    let Ok(rd) = std::fs::read_dir("/proc") else { return Vec::new() };
    rd.flatten().filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok()).filter(|&p| p != me).filter(|p| {
        let cwd = std::fs::read_link(format!("/proc/{p}/cwd")).map(|c| c.starts_with(dir)).unwrap_or(false);
        cwd || std::fs::read(format!("/proc/{p}/cmdline")).map(|c| String::from_utf8_lossy(&c).replace('\\', "/").contains(&want)).unwrap_or(false)
    }).collect()
}

/// Mounts the view, starts the program and waits for it (and anything it started in the game's folder) to end,
/// then takes the view away again. Used by `hearth-mods --organizer <instance> run` (the library's Play).
pub fn run(inst: &Instance, ex: &Executable) -> Result<Value, String> {
    if !ex.binary.is_file() { return Err(format!("{} isn't there ({}).", ex.title, ex.binary.display())); }
    let a = deploy(inst, std::process::id(), true)?;
    let (cmd, args, env, cwd) = command_for(inst, ex);
    let via_steam = cmd == "umu-run" && !has("umu-run");
    let game_dir = inst.game_dir.clone().unwrap_or_else(|| a.target.clone());
    let started = if via_steam {
        // No umu-launcher: Steam starts the game itself (only its own exe: script extenders need umu).
        match inst.steam_app.or(if inst.kind.steam > 0 { Some(inst.kind.steam) } else { None }) {
            Some(app) if has("steam") => std::process::Command::new("steam").arg("-applaunch").arg(app.to_string()).stdin(std::process::Stdio::null()).spawn().map_err(|e| e.to_string()),
            _ => Err("umu-launcher isn't installed, so Windows programs can't be started from here.".to_string()),
        }
    } else {
        let mut c = std::process::Command::new(&cmd);
        c.args(&args).current_dir(&cwd).stdin(std::process::Stdio::null());
        for (k, v) in &env { c.env(k, v); }
        c.spawn().map_err(|e| format!("Couldn't start {} ({e})", ex.title))
    };
    let mut result = json!({"ok": true, "title": ex.title, "mode": a.mode, "notes": a.notes});
    match started {
        Ok(mut child) => {
            let st = child.wait();
            result["exit"] = json!(st.ok().and_then(|s| s.code()));
            // Steam hands the game over: wait for it to appear (two minutes at most), then for it to end.
            if via_steam {
                let t0 = std::time::Instant::now();
                while game_processes(&game_dir).is_empty() && t0.elapsed().as_secs() < 120 { std::thread::sleep(std::time::Duration::from_secs(2)); }
            }
            let t0 = std::time::Instant::now();
            while !game_processes(&game_dir).is_empty() && t0.elapsed().as_secs() < 24 * 3600 { std::thread::sleep(std::time::Duration::from_secs(2)); }
        }
        Err(e) => { result["ok"] = json!(false); result["error"] = json!(e); }
    }
    match undeploy(inst, true) {
        Ok(u) => result["undeploy"] = u,
        Err(e) => { result["undeploy"] = json!({"undeployed": false, "error": e}); }
    }
    Ok(result)
}

/// At sign-in (hearth-mods --remount) and before a run: a view left by a crash or by hand is taken away.
pub fn recover(inst: &Instance) -> Option<Result<Value, String>> {
    let a = read_active(inst)?;
    if alive(a.pid) { return None; }
    Some(undeploy(inst, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn put(p: &Path, t: &str) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, t).unwrap(); }

    #[test]
    fn case_insensitive_paths() {
        let t = tempfile::tempdir().unwrap();
        put(&t.path().join("Data/Textures/Sky.dds"), "x");
        assert_eq!(ci_resolve(t.path(), "data/textures/SKY.DDS"), Some(t.path().join("Data/Textures/Sky.dds")));
        assert_eq!(ci_resolve(t.path(), "data/../x"), None);
        assert_eq!(ci_join(t.path(), "DATA/textures/new/A.dds"), t.path().join("Data/Textures/new/A.dds"));
        assert_eq!(split_args("-a \"b c\" d"), vec!["-a", "b c", "d"]);
    }

    #[test]
    fn priority_case_and_hidden() {
        let t = tempfile::tempdir().unwrap();
        let game = t.path().join("Data");
        put(&game.join("Textures/Sky.dds"), "game");
        put(&game.join("Skyrim.esm"), "esm");
        let m = |n: &str| t.path().join("mods").join(n);
        put(&m("A").join("textures/sky.dds"), "A");
        put(&m("A").join("textures/a.dds"), "A");
        put(&m("A").join("meta.ini"), "[General]");
        put(&m("B").join("TEXTURES/SKY.DDS"), "B");
        put(&m("B").join("Meshes/b.nif"), "B");
        put(&m("B").join("textures/hidden.dds.mohidden"), "B");
        put(&m("C").join("meshes/B.NIF"), "C");
        let src = vec![Source { name: "A".into(), dir: m("A") }, Source { name: "B".into(), dir: m("B") }, Source { name: "C".into(), dir: m("C") }];
        let v = resolve(Some(&game), &src);
        let sky = &v.files["textures/sky.dds"];
        assert_eq!((sky.rel.as_str(), sky.winner, sky.losers.clone()), ("Textures/Sky.dds", 1, vec![0]), "the game's spelling, B wins over A");
        assert_eq!(v.files["textures/a.dds"].rel, "Textures/a.dds");
        assert_eq!(v.files["meshes/b.nif"].rel, "Meshes/b.nif");
        assert_eq!(v.files["meshes/b.nif"].winner, 2);
        assert!(!v.files.contains_key("meta.ini"));
        assert_eq!(v.hidden, vec![(1, "textures/hidden.dds.mohidden".to_string())]);
        let c = conflicts(&v, 3);
        assert_eq!(c[0], Conflicts { wins_over: BTreeSet::new(), loses_to: [1].into(), winning: 0, losing: 1, files: 2, hidden: 0 });
        assert_eq!(c[1].wins_over, [0].into());
        assert_eq!(c[1].loses_to, [2].into());
        assert_eq!((c[1].winning, c[1].losing, c[1].hidden), (1, 1, 1));
        assert_eq!(c[2].wins_over, [1].into());
        let names: Vec<String> = vec!["A".into(), "B".into(), "C".into()];
        let f = files_of(&v, 1, &names);
        assert!(f.as_array().unwrap().iter().any(|x| x["path"] == "Textures/Sky.dds" && x["status"] == "wins" && x["replacesGameFile"] == true));
        assert!(f.as_array().unwrap().iter().any(|x| x["path"] == "Meshes/b.nif" && x["status"] == "loses" && x["to"] == "C"));
        // The layer has the winners, in one spelling.
        let layer = t.path().join("layer");
        assert_eq!(build_layer(&v, None, &layer).unwrap(), 3);
        assert_eq!(std::fs::read_to_string(layer.join("Textures/Sky.dds")).unwrap(), "B");
        assert_eq!(std::fs::read_to_string(layer.join("Meshes/b.nif")).unwrap(), "C");
        assert!(!layer.join("textures").exists() && !layer.join("TEXTURES").exists());
        assert_eq!(build_layer(&v, Some(2), &layer).unwrap(), 2, "a skipped source (overwrite) stays out");
    }
}

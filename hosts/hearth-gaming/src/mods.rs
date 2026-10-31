//! Hearth Mods: a mod manager for any game.
//!
//! Each game has a folder in Hearth's data folder (`mods/<game>/`): `state.json`, every installed mod's files
//! (`files/<mod>/`), the game's own files that mods replaced (`originals/`) and files the game changed while a mod
//! had put them there (`changed/`). Mods are copied into place ("deployed"); turning one off removes its copies and
//! puts the replaced originals back, so nothing of the game is lost. When two mods have the same file, the one
//! lower in the list wins (the load order).
//!
//! How mods are put in place is chosen per game (`State::deploy`):
//! - "hardlink" (the default): each file is a hard link to the mod's own copy, so it takes no extra space and is
//!   instant; across drives (where links can't go) it's a copy. A game that edits a modded file in place edits the
//!   mod's copy too (like Vortex).
//! - "copy": real copies (the way Hearth Mods always worked).
//! - "overlay" (Beta Experimental): the game's folder is never touched. Mods for the game folder are linked into a
//!   layer (`layer/`) that fuse-overlayfs lays over the game folder; whatever the game writes there goes to `upper/`.
//!   The overlay is put back at sign-in (hearth-mods --remount). Mods for the Proton folder or your home are linked.
//! Wabbajack modlists are a fourth way: Mod Organizer 2 manages them in their own folder (wabbajack.rs).

use crate::games::GameEntry;
use crate::util;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModRec {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    /// Where it goes: a target kind (`targets`) and a folder inside it.
    pub target: String,
    #[serde(default)]
    pub sub: String,
    /// Its files, relative to where it goes, with / between parts.
    pub files: Vec<String>,
    pub size: u64,
    pub added: u64,
    #[serde(default)]
    pub source: String,
    /// Where it came from on Nexus Mods (for update checks).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nexus: Option<NexusRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NexusRef { pub domain: String, pub mod_id: u64, pub file_id: u64, #[serde(default)] pub version: String }

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Placed {
    #[serde(rename = "mod")]
    pub by: String,
    pub size: u64,
    pub mtime: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct State {
    pub game: String,
    pub title: String,
    pub mods: Vec<ModRec>,
    /// Every file a mod has put in place (full path) and what it was when put there.
    pub deployed: BTreeMap<String, Placed>,
    /// The game's own files that a mod replaced: full path → kept file in originals/.
    pub originals: BTreeMap<String, String>,
    /// Folders made for mods' files (removed again when empty).
    pub dirs: BTreeSet<String>,
    pub next: u64,
    /// hardlink (default, also when empty), copy or overlay.
    #[serde(default)]
    pub deploy: String,
    /// The Nexus game, when it's known (for downloads and update checks).
    #[serde(default)]
    pub nexus: String,
}

pub const DEPLOY_MODES: [(&str, &str); 3] = [
    ("hardlink", "Links (no extra space; the default)"),
    ("copy", "Copies (the mods' files are copied in)"),
    ("overlay", "Overlay (Beta Experimental: the game folder is never changed)"),
];

/// One game's mods.
pub struct GameMods { pub dir: PathBuf, pub state: State }

/// What a deploy did, for the page.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report { pub placed: usize, pub removed: usize, pub restored: usize, pub kept_changed: Vec<String>, pub problems: Vec<String> }

/// Where mods can go. (id, label, needs a Proton/Wine prefix)
pub const TARGETS: [(&str, &str, bool); 8] = [
    ("game", "Game folder", false),
    ("prefix-documents", "Documents (in the game's Proton folder)", true),
    ("prefix-roaming", "AppData › Roaming (in the game's Proton folder)", true),
    ("prefix-local", "AppData › Local (in the game's Proton folder)", true),
    ("prefix-locallow", "AppData › LocalLow (in the game's Proton folder)", true),
    ("prefix-drive", "C: drive (in the game's Proton folder)", true),
    ("home-data", "Your data folder (~/.local/share), for Linux games", false),
    ("home-config", "Your settings folder (~/.config), for Linux games", false),
];

/// A target kind's folder for this game, if it has one.
pub fn target_root(g: &GameEntry, kind: &str) -> Option<PathBuf> {
    let place = g.place();
    let user = || place.prefix_user();
    match kind {
        "game" => g.install.clone(),
        "prefix-documents" => user().map(|u| u.join("Documents")),
        "prefix-roaming" => user().map(|u| u.join("AppData/Roaming")),
        "prefix-local" => user().map(|u| u.join("AppData/Local")),
        "prefix-locallow" => user().map(|u| u.join("AppData/LocalLow")),
        "prefix-drive" => g.prefix.as_ref().map(|p| p.join("drive_c")),
        "home-data" => Some(place.xdg_data.clone()),
        "home-config" => Some(place.xdg_config.clone()),
        _ => None,
    }
}

/// The targets this game has (a game without a prefix has no prefix targets).
pub fn targets_for(g: &GameEntry) -> Vec<Value> {
    TARGETS.iter().filter_map(|(id, label, _)| target_root(g, id).map(|p| json!({"id": id, "label": label, "path": p}))).collect()
}

/// Where a mod's files go: its target's folder and the folder inside it. The home targets need a folder inside.
pub fn mod_root(g: &GameEntry, m: &ModRec) -> Result<PathBuf, String> {
    let root = target_root(g, &m.target).ok_or_else(|| if m.target.starts_with("prefix") { "The game's Proton folder isn't there yet: start the game once, then turn the mod on.".to_string() } else { "This game has no folder for that.".to_string() })?;
    if m.sub.is_empty() { if m.target.starts_with("home") { return Err("A mod for your data or settings folder needs a folder inside it.".into()); } return Ok(root); }
    let sub = util::clean_rel(&m.sub).ok_or("That folder isn't inside the game's folder.")?;
    Ok(root.join(sub))
}

impl GameMods {
    pub fn mode(&self) -> &str { match self.state.deploy.as_str() { "copy" => "copy", "overlay" => "overlay", _ => "hardlink" } }
    fn layer(&self) -> PathBuf { self.dir.join("layer") }
    /// Where a mod's files go under this game's way of deploying: with the overlay, mods for the game folder go into
    /// the layer laid over it.
    pub fn root_of(&self, g: &GameEntry, m: &ModRec) -> Result<PathBuf, String> {
        if self.mode() == "overlay" && m.target == "game" {
            let sub = if m.sub.is_empty() { PathBuf::new() } else { util::clean_rel(&m.sub).ok_or("That folder isn't inside the game's folder.")? };
            return Ok(self.layer().join(sub));
        }
        mod_root(g, m)
    }
    pub fn open(data_root: &Path, g: &GameEntry) -> GameMods {
        let dir = data_root.join("mods").join(util::id_key(&g.id));
        let mut state: State = std::fs::read(dir.join("state.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        state.game = g.id.clone();
        state.title = g.title.clone();
        GameMods { dir, state }
    }
    pub fn save(&self) -> Result<(), String> {
        util::write_atomic(&self.dir.join("state.json"), serde_json::to_string_pretty(&self.state).unwrap_or_default().as_bytes()).map_err(|e| format!("Couldn't save the mod list ({e})"))
    }
    fn files_of(&self, id: &str) -> PathBuf { self.dir.join("files").join(id) }
    pub fn find(&self, id: &str) -> Result<usize, String> { self.state.mods.iter().position(|m| m.id == id).ok_or_else(|| "That mod isn't installed anymore.".to_string()) }

    /// What the page shows: the mods in load order, with conflicts, and where mods can go.
    pub fn summary(&self, g: &GameEntry) -> Value {
        let desired = self.desired(g).0;
        let names: BTreeMap<&str, &str> = self.state.mods.iter().map(|m| (m.id.as_str(), m.name.as_str())).collect();
        let mods: Vec<Value> = self.state.mods.iter().enumerate().map(|(i, m)| {
            let root = self.root_of(g, m).ok();
            let (mut beaten, mut wins) = (BTreeSet::new(), 0usize);
            if m.enabled { if let Some(r) = &root {
                for f in &m.files {
                    match desired.get(&r.join(f).to_string_lossy().to_string()) {
                        Some(w) if w != &m.id => { beaten.insert(names.get(w.as_str()).copied().unwrap_or("another mod")); }
                        _ => wins += 1,
                    }
                }
            } }
            json!({"id": m.id, "name": m.name, "enabled": m.enabled, "order": i, "target": m.target, "sub": m.sub, "root": root, "files": m.files.len(),
                   "size": m.size, "added": m.added, "source": m.source, "overriddenBy": beaten, "winning": wins, "nexus": m.nexus,
                   "targetLabel": TARGETS.iter().find(|t| t.0 == m.target).map(|t| t.1).unwrap_or("")})
        }).collect();
        json!({"game": g.id, "title": g.title, "mods": mods, "targets": targets_for(g), "deployed": self.state.deployed.len(), "originals": self.state.originals.len(), "folder": self.dir,
               "deploy": self.mode(), "modes": DEPLOY_MODES.iter().map(|m| json!({"id": m.0, "label": m.1})).collect::<Vec<_>>(), "nexus": self.state.nexus,
               "overlay": if self.mode() == "overlay" { json!({"mounted": overlay::mounted(g.install.as_deref()), "upper": self.dir.join("upper"), "changed": util::tree_size(&self.dir.join("upper"))}) } else { Value::Null }})
    }

    /// Every file the turned-on mods want in place → the mod that wins it. Mods whose folder is missing are left out
    /// (with why).
    fn desired(&self, g: &GameEntry) -> (BTreeMap<String, String>, Vec<String>) {
        let mut want = BTreeMap::new();
        let mut problems = Vec::new();
        for m in self.state.mods.iter().filter(|m| m.enabled) {
            match self.root_of(g, m) {
                Ok(root) => for f in &m.files { if let Some(rel) = util::clean_rel(f) { want.insert(root.join(rel).to_string_lossy().to_string(), m.id.clone()); } },
                Err(e) => problems.push(format!("{}: {e}", m.name)),
            }
        }
        (want, problems)
    }

    /// Makes the game's folder match the mod list: removes what's no longer wanted (putting originals back) and puts
    /// in what is (keeping the originals it replaces).
    pub fn deploy(&mut self, g: &GameEntry) -> Report {
        let overlay = self.mode() == "overlay";
        if overlay || overlay::mounted(g.install.as_deref()) {
            // The layer changes under a live overlay: take it down first (the game mustn't be running).
            if let Err(e) = overlay::unmount(g.install.as_deref()) { return Report { problems: vec![e], ..Default::default() }; }
        }
        let mut rep = self.place(g);
        if overlay {
            if let Err(e) = overlay::mount(g.install.as_deref(), &self.layer(), &self.dir) { rep.problems.push(e); }
        }
        rep
    }

    /// The overlay again after a restart (hearth-mods --remount, at sign-in).
    pub fn remount(&self, g: &GameEntry) -> Result<(), String> {
        if self.mode() != "overlay" || overlay::mounted(g.install.as_deref()) { return Ok(()); }
        overlay::mount(g.install.as_deref(), &self.layer(), &self.dir)
    }

    /// Changes how this game's mods are put in place: every mod comes out the old way and goes back in the new way.
    pub fn set_mode(&mut self, g: &GameEntry, mode: &str) -> Result<Report, String> {
        if !DEPLOY_MODES.iter().any(|m| m.0 == mode) { return Err("Choose how mods are put in place.".into()); }
        if mode == self.mode() { return Ok(Report::default()); }
        if mode == "overlay" && !overlay::available() { return Err("The overlay needs fuse-overlayfs (it comes with Hearth OS).".into()); }
        let on: Vec<bool> = self.state.mods.iter().map(|m| m.enabled).collect();
        for m in self.state.mods.iter_mut() { m.enabled = false; }
        let out = self.deploy(g);
        let stuck = !self.state.deployed.is_empty();
        for (m, e) in self.state.mods.iter_mut().zip(on) { m.enabled = e; }
        if stuck || !out.problems.is_empty() {
            self.save()?;
            return Err(format!("Couldn't take the mods out to switch: {}", out.problems.join("; ")));
        }
        self.state.deploy = mode.into();
        self.save()?;
        Ok(self.deploy(g))
    }

    fn place(&mut self, g: &GameEntry) -> Report {
        let linking = self.mode() != "copy";
        let (want, problems) = self.desired(g);
        let mut rep = Report { problems, ..Default::default() };
        let _ = std::fs::create_dir_all(self.dir.join("originals"));
        // 1. What's in place but no longer wanted (or wanted from another mod) comes out.
        let current: Vec<(String, Placed)> = self.state.deployed.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        for (abs, placed) in current {
            if want.get(&abs) == Some(&placed.by) && same_file(Path::new(&abs), &placed) { continue; }
            let p = PathBuf::from(&abs);
            if let Ok(md) = std::fs::symlink_metadata(&p) {
                if md.is_file() && (md.len() != placed.size || util::mtime_ms(&md) != placed.mtime) {
                    // The game changed this file while the mod's copy was there: keep it rather than lose it.
                    let keep = self.dir.join("changed").join(format!("{}-{}", util::now_ms(), p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
                    let _ = std::fs::create_dir_all(keep.parent().unwrap());
                    if move_file(&p, &keep).is_ok() { rep.kept_changed.push(abs.clone()); }
                } else if md.is_file() || md.file_type().is_symlink() { if std::fs::remove_file(&p).is_ok() { rep.removed += 1; } }
            }
            self.state.deployed.remove(&abs);
            if let Some(o) = self.state.originals.remove(&abs) {
                match move_file(&self.dir.join("originals").join(&o), &p) {
                    Ok(()) => rep.restored += 1,
                    Err(e) => { rep.problems.push(format!("Couldn't put back the game's {abs} ({e}); it's kept in {}", self.dir.join("originals").join(&o).display())); self.state.originals.insert(abs.clone(), o); }
                }
            }
        }
        let _ = self.save();
        // 2. What's wanted goes in.
        for (abs, by) in &want {
            if self.state.deployed.get(abs).map(|d| &d.by == by).unwrap_or(false) { continue; }
            let Ok(mi) = self.find(by) else { continue };
            let m = self.state.mods[mi].clone();
            let Ok(root) = self.root_of(g, &m) else { continue };
            let Ok(rel) = Path::new(abs).strip_prefix(&root) else { continue };
            let src = self.files_of(&m.id).join(rel);
            let p = PathBuf::from(abs);
            if let Ok(md) = std::fs::symlink_metadata(&p) {
                if md.is_dir() { rep.problems.push(format!("{}: {abs} is a folder in the game, not a file; left alone", m.name)); continue; }
                if !self.state.originals.contains_key(abs) {
                    let name = format!("o{}", self.state.next);
                    self.state.next += 1;
                    if let Err(e) = move_file(&p, &self.dir.join("originals").join(&name)) { rep.problems.push(format!("{}: couldn't set aside the game's {abs} ({e}); left alone", m.name)); continue; }
                    self.state.originals.insert(abs.clone(), name);
                }
            }
            if let Some(parent) = p.parent() { self.make_dirs(parent, &root); }
            match if linking { link_file(&src, &p) } else { copy_file(&src, &p) } {
                Ok(md) => { self.state.deployed.insert(abs.clone(), Placed { by: by.clone(), size: md.len(), mtime: util::mtime_ms(&md) }); rep.placed += 1; }
                Err(e) => rep.problems.push(format!("{}: couldn't copy {} ({e})", m.name, rel.display())),
            }
        }
        // 3. Folders made for mods that are empty again go.
        let mut dirs: Vec<String> = self.state.dirs.iter().cloned().collect();
        dirs.sort_by_key(|d| std::cmp::Reverse(d.len()));
        for d in dirs {
            let p = Path::new(&d);
            if !p.exists() { self.state.dirs.remove(&d); continue; }
            let empty = std::fs::read_dir(p).map(|mut r| r.next().is_none()).unwrap_or(false);
            if empty && std::fs::remove_dir(p).is_ok() { self.state.dirs.remove(&d); }
        }
        let _ = self.save();
        rep
    }

    /// Makes the folders up to `dir` that are missing (inside `root` only), remembering them.
    fn make_dirs(&mut self, dir: &Path, root: &Path) {
        let mut missing = Vec::new();
        let mut cur = Some(dir);
        while let Some(d) = cur {
            if d.exists() || !d.starts_with(root) { break; }
            missing.push(d.to_path_buf());
            cur = d.parent();
        }
        if !root.exists() { let _ = std::fs::create_dir_all(root); }
        for d in missing.iter().rev() {
            if std::fs::create_dir(d).is_ok() { self.state.dirs.insert(d.to_string_lossy().to_string()); }
        }
    }

    pub fn set_enabled(&mut self, g: &GameEntry, id: &str, on: bool) -> Result<Report, String> {
        let i = self.find(id)?;
        self.state.mods[i].enabled = on;
        self.save()?;
        Ok(self.deploy(g))
    }

    /// Moves a mod up (-1) or down (+1) the load order.
    pub fn move_by(&mut self, g: &GameEntry, id: &str, by: i64) -> Result<Report, String> {
        let i = self.find(id)? as i64;
        let j = (i + by).clamp(0, self.state.mods.len() as i64 - 1);
        if i != j { let m = self.state.mods.remove(i as usize); self.state.mods.insert(j as usize, m); }
        self.save()?;
        Ok(self.deploy(g))
    }

    /// Turns a mod off (putting the game's files back) and deletes its copy.
    pub fn remove(&mut self, g: &GameEntry, id: &str) -> Result<Report, String> {
        let i = self.find(id)?;
        self.state.mods[i].enabled = false;
        let rep = self.deploy(g);
        if self.state.deployed.values().any(|d| d.by == id) { return Err("Some of its files couldn't be taken out; it's still installed (and off).".into()); }
        self.state.mods.remove(i);
        let _ = std::fs::remove_dir_all(self.files_of(id));
        self.save()?;
        Ok(rep)
    }

    /// Installs a mod unpacked by `stage` (moved into the mod store) and turns it on if asked.
    pub fn install(&mut self, g: &GameEntry, staged: &Staged, name: &str, target: &str, sub: &str, enable: bool) -> Result<Report, String> {
        if !TARGETS.iter().any(|t| t.0 == target) { return Err("Choose where the mod goes.".into()); }
        if !sub.is_empty() && util::clean_rel(sub).is_none() { return Err("That folder isn't inside the game's folder.".into()); }
        let id = format!("m{}", self.state.next);
        self.state.next += 1;
        let dest = self.files_of(&id);
        std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
        if dest.exists() { let _ = std::fs::remove_dir_all(&dest); }
        if std::fs::rename(&staged.root, &dest).is_err() {
            copy_tree(&staged.root, &dest).map_err(|e| format!("Couldn't keep the mod's files ({e})"))?;
        }
        let _ = std::fs::remove_dir_all(&staged.dir);
        let mut files = Vec::new();
        util::files_under(&dest, &mut files);
        let rels: Vec<String> = files.iter().filter_map(|f| f.strip_prefix(&dest).ok()).map(|r| r.to_string_lossy().replace('\\', "/")).collect();
        if rels.is_empty() { let _ = std::fs::remove_dir_all(&dest); return Err("That mod has no files in it.".into()); }
        let name = if name.trim().is_empty() { staged.name.clone() } else { name.trim().chars().take(120).collect() };
        self.state.mods.push(ModRec { id, name, enabled: enable, target: target.into(), sub: sub.trim_matches('/').into(), size: files.iter().map(|f| std::fs::metadata(f).map(|m| m.len()).unwrap_or(0)).sum(),
            files: rels, added: util::now_ms(), source: staged.source.clone(), nexus: staged.nexus.clone() });
        self.save()?;
        Ok(if enable { self.deploy(g) } else { Report::default() })
    }

    /// Keeps which Nexus game this is (chosen once, or found from the library).
    pub fn set_nexus_game(&mut self, domain: &str) -> Result<(), String> { self.state.nexus = domain.to_ascii_lowercase(); self.save() }
}

/// fuse-overlayfs over a game's folder: the mods' layer on top, the game's own files under it, the game's own
/// changes in upper/. Only one overlay per game folder.
pub mod overlay {
    use std::path::Path;

    pub fn available() -> bool { which("fuse-overlayfs") }
    fn which(cmd: &str) -> bool {
        std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(cmd).is_file())).unwrap_or(false)
    }

    /// Is something mounted on this folder? (/proc/self/mountinfo, with its \040-style escapes.)
    pub fn mounted(game: Option<&Path>) -> bool {
        let Some(g) = game else { return false };
        let want = g.to_string_lossy().trim_end_matches('/').to_string();
        std::fs::read_to_string("/proc/self/mountinfo").map(|t| t.lines().any(|l| l.split(' ').nth(4).map(unescape).as_deref() == Some(want.as_str()))).unwrap_or(false)
    }
    pub fn unescape(s: &str) -> String {
        let mut out = String::new();
        let b = s.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'\\' && i + 3 < b.len() {
                if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 4], 8) { out.push(v as char); i += 4; continue; }
            }
            out.push(b[i] as char);
            i += 1;
        }
        out
    }
    /// A folder for fuse-overlayfs' options: , : and \ escaped.
    pub fn opt_path(p: &Path) -> String {
        p.to_string_lossy().chars().flat_map(|c| if matches!(c, ',' | ':' | '\\') { vec!['\\', c] } else { vec![c] }).collect()
    }

    pub fn mount(game: Option<&Path>, layer: &Path, dir: &Path) -> Result<(), String> {
        let game = game.ok_or("This game has no folder.")?;
        if mounted(Some(game)) { return Ok(()); }
        // Nothing in the layer: nothing to lay over.
        if std::fs::read_dir(layer).map(|mut r| r.next().is_none()).unwrap_or(true) { return Ok(()); }
        if !available() { return Err("The overlay needs fuse-overlayfs (it comes with Hearth OS).".into()); }
        let (upper, work) = (dir.join("upper"), dir.join("work"));
        std::fs::create_dir_all(&upper).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
        let opts = format!("lowerdir={}:{},upperdir={},workdir={}", opt_path(layer), opt_path(game), opt_path(&upper), opt_path(&work));
        let out = std::process::Command::new("fuse-overlayfs").arg("-o").arg(opts).arg(game).output().map_err(|e| format!("Couldn't start fuse-overlayfs ({e})"))?;
        if out.status.success() && mounted(Some(game)) { Ok(()) } else { Err(format!("Couldn't lay the mods over the game folder ({})", String::from_utf8_lossy(&out.stderr).trim())) }
    }

    pub fn unmount(game: Option<&Path>) -> Result<(), String> {
        let Some(game) = game else { return Ok(()) };
        if !mounted(Some(game)) { return Ok(()); }
        for cmd in ["fusermount3", "fusermount"] {
            if let Ok(o) = std::process::Command::new(cmd).arg("-u").arg(game).output() {
                if o.status.success() && !mounted(Some(game)) { return Ok(()); }
                let e = String::from_utf8_lossy(&o.stderr).to_string();
                if e.contains("busy") { return Err("The game (or a program in its folder) is still running: close it, then try again.".into()); }
            }
        }
        Err("Couldn't take the overlay off the game folder.".into())
    }
}

/// Puts a hard link to `from` at `to` (a temporary name, then a rename); a copy where links can't go (another drive).
pub fn link_file(from: &Path, to: &Path) -> std::io::Result<std::fs::Metadata> {
    if let Some(d) = to.parent() { std::fs::create_dir_all(d)?; }
    let tmp = to.with_file_name(format!(".{}.hearth-mods", to.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    let _ = std::fs::remove_file(&tmp);
    if std::fs::hard_link(from, &tmp).is_err() { return copy_file(from, to); }
    if let Err(e) = std::fs::rename(&tmp, to) { let _ = std::fs::remove_file(&tmp); return Err(e); }
    std::fs::metadata(to)
}

fn same_file(p: &Path, placed: &Placed) -> bool {
    std::fs::symlink_metadata(p).map(|m| m.is_file() && m.len() == placed.size && util::mtime_ms(&m) == placed.mtime).unwrap_or(false)
}

/// Moves a file (across drives too: copy, then delete).
pub fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(d) = to.parent() { std::fs::create_dir_all(d)?; }
    if std::fs::rename(from, to).is_ok() { return Ok(()); }
    copy_file(from, to)?;
    std::fs::remove_file(from)
}

/// Copies a file all at once (a temporary file, then a rename). Returns the copy's details.
pub fn copy_file(from: &Path, to: &Path) -> std::io::Result<std::fs::Metadata> {
    if let Some(d) = to.parent() { std::fs::create_dir_all(d)?; }
    let tmp = to.with_file_name(format!(".{}.hearth-mods", to.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    if let Err(e) = std::fs::copy(from, &tmp) { let _ = std::fs::remove_file(&tmp); return Err(e); }
    if let Err(e) = std::fs::rename(&tmp, to) { let _ = std::fs::remove_file(&tmp); return Err(e); }
    std::fs::metadata(to)
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        let t = e.file_type()?;
        if t.is_dir() { copy_tree(&e.path(), &to.join(e.file_name()))?; } else if t.is_file() { std::fs::copy(e.path(), to.join(e.file_name()))?; }
    }
    Ok(())
}

// ------------------------------------------------------------------ unpacking a mod

/// A mod unpacked and waiting for "Install": where its files are, and what's at its top.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Staged {
    pub token: String,
    pub dir: PathBuf,
    /// Where the mod's files start (inside dir; a single wrapping folder is skipped).
    pub root: PathBuf,
    pub name: String,
    pub source: String,
    pub top: Vec<String>,
    pub files: usize,
    pub size: u64,
    /// The wrapping folder skipped, if any ("MyMod-1.2/").
    pub unwrapped: Option<String>,
    /// Where it probably goes.
    pub suggest_target: String,
    pub suggest_sub: String,
    /// Set when the file came from Nexus Mods (the download queue).
    #[serde(default)]
    pub nexus: Option<NexusRef>,
}

pub fn is_archive(p: &Path) -> bool {
    let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    [".zip", ".7z", ".rar", ".tar", ".tar.gz", ".tgz", ".tar.xz", ".tar.zst", ".tar.bz2"].iter().any(|e| n.ends_with(e))
}

/// Unpacks a mod (a zip, 7z or rar file, or a folder) into the game's staging folder, and looks at it.
pub fn stage(data_root: &Path, g: &GameEntry, source: &Path) -> Result<Staged, String> {
    let base = data_root.join("mods").join(util::id_key(&g.id)).join("staging");
    // Leftovers from before (a mod looked at but not installed) go.
    if let Ok(rd) = std::fs::read_dir(&base) { for e in rd.flatten() { let _ = std::fs::remove_dir_all(e.path()); } }
    let token = format!("s{}", util::now_ms());
    let dir = base.join(&token);
    let out = dir.join("x");
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let r = (|| -> Result<(), String> {
        if source.is_dir() { return copy_tree(source, &out).map_err(|e| format!("Couldn't copy the folder ({e})")); }
        let lower = source.to_string_lossy().to_lowercase();
        if lower.ends_with(".zip") { return unzip(source, &out); }
        if !is_archive(source) { return Err("Choose a .zip, .7z or .rar file, or a folder.".into()); }
        // 7z, rar and tar files: bsdtar (libarchive, in Hearth OS), or 7z.
        let ok = std::process::Command::new("bsdtar").arg("-xf").arg(source).arg("-C").arg(&out).status().map(|s| s.success()).unwrap_or(false)
            || std::process::Command::new("7z").arg("x").arg("-y").arg(format!("-o{}", out.display())).arg(source).stdout(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false);
        if ok { Ok(()) } else { Err("Couldn't unpack that file (it needs bsdtar or 7-Zip, and may be damaged).".into()) }
    })();
    if let Err(e) = r { let _ = std::fs::remove_dir_all(&dir); return Err(e); }
    drop_links(&out);
    let game_root = g.install.clone();
    let mut root = out.clone();
    let mut unwrapped = None;
    for _ in 0..3 {
        let entries: Vec<PathBuf> = std::fs::read_dir(&root).map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default();
        if entries.len() != 1 || !entries[0].is_dir() { break; }
        let n = entries[0].file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        // A single folder the game also has (Data, BepInEx, Mods...) is where the files go, not a wrapper.
        if game_root.as_ref().map(|r| has_name(r, &n)).unwrap_or(false) || KNOWN.iter().any(|k| k.eq_ignore_ascii_case(&n)) { break; }
        unwrapped = Some(match unwrapped { Some(u) => format!("{u}/{n}"), None => n });
        root = entries[0].clone();
    }
    let mut files = Vec::new();
    util::files_under(&root, &mut files);
    if files.is_empty() { let _ = std::fs::remove_dir_all(&dir); return Err("There are no files in it.".into()); }
    let mut top: Vec<String> = std::fs::read_dir(&root).map(|rd| rd.flatten().map(|e| format!("{}{}", e.file_name().to_string_lossy(), if e.path().is_dir() { "/" } else { "" })).collect()).unwrap_or_default();
    top.sort();
    let name = source.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Mod".into());
    let name = ["zip", "7z", "rar", "tgz", "gz", "xz", "zst", "bz2", "tar"].iter().fold(name, |n, e| n.strip_suffix(&format!(".{e}")).map(str::to_string).unwrap_or(n));
    let (st, ss) = suggest(g, &top);
    Ok(Staged { token, dir: dir.clone(), root, name, source: source.to_string_lossy().into(), files: files.len(), size: files.iter().map(|f| std::fs::metadata(f).map(|m| m.len()).unwrap_or(0)).sum(),
        top, unwrapped, suggest_target: st, suggest_sub: ss, nexus: None })
}

/// Folder names that mean "these go into the game folder as they are".
const KNOWN: [&str; 12] = ["Data", "BepInEx", "Mods", "mods", "plugins", "scripts", "Content", "bin", "x64", "Binaries", "Plugins", "Paks"];

fn has_name(dir: &Path, n: &str) -> bool {
    std::fs::read_dir(dir).map(|rd| rd.flatten().any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(n))).unwrap_or(false)
}

/// Where a mod probably goes, from what's at its top: the game's folder (most mods), unless its game folder has
/// a "Mods" folder and the mod is just files (then into Mods).
fn suggest(g: &GameEntry, top: &[String]) -> (String, String) {
    let Some(root) = &g.install else { return ("game".into(), String::new()) };
    let matches = top.iter().any(|t| has_name(root, t.trim_end_matches('/')));
    if !matches && top.iter().all(|t| !t.ends_with('/')) {
        for m in ["Mods", "mods"] { if root.join(m).is_dir() { return ("game".into(), m.into()); } }
    }
    ("game".into(), String::new())
}

fn unzip(file: &Path, out: &Path) -> Result<(), String> {
    let f = std::fs::File::open(file).map_err(|e| format!("Couldn't open it ({e})"))?;
    let mut a = zip::ZipArchive::new(f).map_err(|_| "That zip file is damaged.".to_string())?;
    for i in 0..a.len() {
        let mut e = a.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = e.enclosed_name() else { continue };
        let Some(rel) = util::clean_rel(&rel.to_string_lossy()) else { continue };
        let p = out.join(rel);
        if e.is_dir() { std::fs::create_dir_all(&p).map_err(|e| e.to_string())?; continue; }
        if let Some(d) = p.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
        let mut w = std::fs::File::create(&p).map_err(|e| e.to_string())?;
        std::io::copy(&mut e, &mut w).map_err(|e| format!("Couldn't unpack it ({e})"))?;
    }
    Ok(())
}

/// Links in an unpacked mod could point anywhere: they're removed.
fn drop_links(dir: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(t) = e.file_type() else { continue };
        if t.is_symlink() { let _ = std::fs::remove_file(e.path()); } else if t.is_dir() { drop_links(&e.path()); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn put(p: &Path, text: &str) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, text).unwrap(); }
    fn read(p: &Path) -> String { std::fs::read_to_string(p).unwrap_or_else(|_| "<missing>".into()) }
    fn make_zip(p: &Path, files: &[(&str, &str)]) {
        let mut z = zip::ZipWriter::new(std::fs::File::create(p).unwrap());
        for (n, t) in files { z.start_file(n.to_string(), zip::write::SimpleFileOptions::default()).unwrap(); z.write_all(t.as_bytes()).unwrap(); }
        z.finish().unwrap();
    }

    #[test]
    fn mods_go_in_and_come_out_leaving_the_game_as_it_was() {
        let t = tempfile::tempdir().unwrap();
        let game = t.path().join("Game");
        put(&game.join("Data/a.txt"), "original a");
        put(&game.join("game.exe"), "exe");
        let g = GameEntry { id: "steam:1".into(), title: "Game".into(), install: Some(game.clone()), kind: "game".into(), ..Default::default() };
        let data = t.path().join("data");
        // A zip with a wrapping folder: "Cool Mod/Data/a.txt" and "Cool Mod/Data/new/b.txt".
        let z1 = t.path().join("Cool Mod.zip");
        make_zip(&z1, &[("Cool Mod/Data/a.txt", "mod one a"), ("Cool Mod/Data/new/b.txt", "mod one b"), ("../evil.txt", "nope")]);
        let mut gm = GameMods::open(&data, &g);
        let s = stage(&data, &g, &z1).unwrap();
        assert_eq!((s.name.as_str(), s.unwrapped.as_deref(), s.top.clone()), ("Cool Mod", Some("Cool Mod"), vec!["Data/".to_string()]));
        let r = gm.install(&g, &s, "", "game", "", true).unwrap();
        assert_eq!(r.placed, 2, "{r:?}");
        assert_eq!(read(&game.join("Data/a.txt")), "mod one a");
        assert_eq!(read(&game.join("Data/new/b.txt")), "mod one b");
        assert!(!t.path().join("evil.txt").exists());
        // A second mod, from a folder, with the same file: lower in the list wins.
        let f2 = t.path().join("Other");
        put(&f2.join("Data/a.txt"), "mod two a");
        let s2 = stage(&data, &g, &f2).unwrap();
        gm.install(&g, &s2, "Other mod", "game", "", true).unwrap();
        assert_eq!(read(&game.join("Data/a.txt")), "mod two a");
        let sum = gm.summary(&g);
        assert_eq!(sum["mods"][0]["overriddenBy"][0], "Other mod");
        // Load order: move the second mod up; now the first wins.
        let id2 = gm.state.mods[1].id.clone();
        gm.move_by(&g, &id2, -1).unwrap();
        assert_eq!(read(&game.join("Data/a.txt")), "mod one a");
        // Turn both off: the game is exactly as it was.
        let id1 = gm.state.mods[1].id.clone();
        gm.set_enabled(&g, &id1, false).unwrap();
        assert_eq!(read(&game.join("Data/a.txt")), "mod two a");
        gm.set_enabled(&g, &id2, false).unwrap();
        assert_eq!(read(&game.join("Data/a.txt")), "original a");
        assert!(!game.join("Data/new").exists(), "the folder made for the mod is gone");
        assert!(gm.state.originals.is_empty() && gm.state.deployed.is_empty());
        // On again, then removed for good.
        gm.set_enabled(&g, &id1, true).unwrap();
        assert_eq!(read(&game.join("Data/new/b.txt")), "mod one b");
        gm.remove(&g, &id1).unwrap();
        assert_eq!(read(&game.join("Data/a.txt")), "original a");
        assert!(!game.join("Data/new/b.txt").exists());
        assert_eq!(gm.state.mods.len(), 1);
        // The list survives reopening.
        assert_eq!(GameMods::open(&data, &g).state.mods.len(), 1);
    }

    #[test]
    fn a_file_the_game_changed_is_kept() {
        let t = tempfile::tempdir().unwrap();
        let game = t.path().join("Game");
        put(&game.join("cfg.ini"), "original");
        let g = GameEntry { id: "manual:x".into(), title: "X".into(), install: Some(game.clone()), kind: "game".into(), ..Default::default() };
        let data = t.path().join("data");
        let f = t.path().join("M");
        put(&f.join("cfg.ini"), "modded");
        let mut gm = GameMods::open(&data, &g);
        let s = stage(&data, &g, &f).unwrap();
        gm.install(&g, &s, "M", "game", "", true).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        put(&game.join("cfg.ini"), "modded, then changed by the game");
        let id = gm.state.mods[0].id.clone();
        let r = gm.set_enabled(&g, &id, false).unwrap();
        assert_eq!(r.kept_changed.len(), 1);
        assert_eq!(read(&game.join("cfg.ini")), "original");
    }

    #[cfg(unix)]
    #[test]
    fn links_by_default_copies_when_asked_and_switching_keeps_the_game_whole() {
        use std::os::unix::fs::MetadataExt;
        let t = tempfile::tempdir().unwrap();
        let game = t.path().join("Game");
        put(&game.join("Data/a.txt"), "original a");
        let g = GameEntry { id: "steam:3".into(), title: "Game".into(), install: Some(game.clone()), kind: "game".into(), ..Default::default() };
        let data = t.path().join("data");
        let f = t.path().join("M");
        put(&f.join("Data/a.txt"), "mod a");
        put(&f.join("Data/b.txt"), "mod b");
        let mut gm = GameMods::open(&data, &g);
        assert_eq!(gm.mode(), "hardlink");
        let s = stage(&data, &g, &f).unwrap();
        gm.install(&g, &s, "M", "game", "", true).unwrap();
        let id = gm.state.mods[0].id.clone();
        let store = gm.dir.join("files").join(&id).join("Data/b.txt");
        assert_eq!(std::fs::metadata(game.join("Data/b.txt")).unwrap().ino(), std::fs::metadata(&store).unwrap().ino(), "a link, not a copy");
        let r = gm.set_mode(&g, "copy").unwrap();
        assert!(r.problems.is_empty(), "{r:?}");
        assert_eq!(gm.mode(), "copy");
        assert_eq!(read(&game.join("Data/a.txt")), "mod a");
        assert_ne!(std::fs::metadata(game.join("Data/b.txt")).unwrap().ino(), std::fs::metadata(&store).unwrap().ino(), "now a copy");
        gm.set_enabled(&g, &id, false).unwrap();
        assert_eq!(read(&game.join("Data/a.txt")), "original a");
        assert!(!game.join("Data/b.txt").exists());
        assert!(gm.set_mode(&g, "sideways").is_err());
        assert_eq!(GameMods::open(&data, &g).mode(), "copy", "the choice is kept");
    }

    #[test]
    fn overlay_paths() {
        assert_eq!(overlay::unescape("/home/a\\040b/Game"), "/home/a b/Game");
        assert_eq!(overlay::opt_path(Path::new("/g/a,b:c")), "/g/a\\,b\\:c");
        assert!(!overlay::mounted(Some(Path::new("/definitely/not/mounted"))));
        let t = tempfile::tempdir().unwrap();
        let g = GameEntry { id: "manual:o".into(), title: "O".into(), install: Some(t.path().join("O")), kind: "game".into(), ..Default::default() };
        let mut gm = GameMods::open(&t.path().join("data"), &g);
        gm.state.deploy = "overlay".into();
        let m = ModRec { target: "game".into(), sub: "Data".into(), ..Default::default() };
        assert_eq!(gm.root_of(&g, &m).unwrap(), gm.dir.join("layer/Data"), "game-folder mods go into the layer");
        let d = ModRec { target: "home-data".into(), sub: "x".into(), ..Default::default() };
        assert!(gm.root_of(&g, &d).unwrap().ends_with("x"));
    }

    #[test]
    fn prefix_targets_need_a_prefix() {
        let t = tempfile::tempdir().unwrap();
        let g = GameEntry { id: "steam:2".into(), title: "Y".into(), install: Some(t.path().join("Y")), kind: "game".into(), ..Default::default() };
        assert_eq!(targets_for(&g).len(), 3);
        let m = ModRec { target: "prefix-documents".into(), ..Default::default() };
        assert!(mod_root(&g, &m).unwrap_err().contains("start the game once"));
        let h = ModRec { target: "home-data".into(), ..Default::default() };
        assert!(mod_root(&g, &h).is_err());
    }
}

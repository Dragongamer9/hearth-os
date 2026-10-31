//! The Hearth Mod Organizer: Hearth's own mod organizer, a native Linux engine that works the way Mod Organizer 2
//! does and with its files (MO2: github.com/ModOrganizer2, GPL-3.0 like Hearth; its design and file formats are
//! followed here, the code is Hearth's own, in Rust).
//!
//! - An instance per game, laid out as MO2's: ModOrganizer.ini, mods/ (each mod in its own folder with meta.ini),
//!   downloads/, profiles/ (modlist.txt, plugins.txt, loadorder.txt, settings.ini, the profile's own inis and saves)
//!   and overwrite/. Hearth's own notes go in .hearth/. An existing MO2 instance (a Wabbajack list's folder) is used
//!   in place, so MO2 itself still opens it.
//! - The game's Data folder is never changed: the enabled mods are merged by priority and laid over it with
//!   fuse-overlayfs while the game runs (vfs.rs), with links as the fallback.
//! - Conflicts, .mohidden files, Bethesda plugins (plugins.rs), FOMOD (fomod.rs), BAIN and plain archives.
//! - `command()` is the JSON interface: `hearth-mods --organizer …` (apps/mods.rs), the Desktop Mods app and Game
//!   Mode's mod screens use it (docs/MODS.md, "Hearth Mod Organizer").
//!
//! The instance list is mods/organizer/instances.json in Hearth's data folder. Each instance also gets a library
//! entry (mods/modlists/, read by hearth-core library.rs) whose Play runs the game through the organizer.

pub mod fomod;
pub mod kinds;
pub mod mo2;
pub mod plugins;
pub mod vfs;

use crate::games::GameEntry;
use crate::util;
use kinds::Kind;
use mo2::{Ini, ModLine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ instances

/// An instance in Hearth's list.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Rec {
    pub id: String,
    pub title: String,
    pub dir: PathBuf,
    /// kinds.rs id (skyrimse, fallout4, generic...).
    pub kind: String,
    pub game_dir: Option<PathBuf>,
    /// The Hearth library game it's for ("steam:489830"), if any.
    pub library_id: String,
    /// The Proton/Wine prefix (the folder with drive_c in it).
    pub prefix: Option<PathBuf>,
    pub steam_app: Option<u64>,
    /// The Wabbajack install it came from (wabbajack.rs job id).
    pub wabbajack: String,
    /// What the library's Play starts: "hearth" (this organizer) or "mo2" (Windows Mod Organizer 2 under Proton).
    pub launcher: String,
    pub added: u64,
}

/// Where things are: Hearth's data folder (the instance list, library entries) and the games in the library.
pub struct Ctx { pub data: PathBuf, pub games: Vec<GameEntry>, pub exe: PathBuf }

impl Ctx {
    pub fn system(games: Vec<GameEntry>) -> Ctx {
        Ctx { data: util::data_dir(), games, exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("hearth-mods")) }
    }
    pub fn base(&self) -> PathBuf { self.data.join("mods").join("organizer") }
    fn modlists(&self) -> PathBuf { self.data.join("mods").join("modlists") }
    fn reg_file(&self) -> PathBuf { self.base().join("instances.json") }
    pub fn recs(&self) -> Vec<Rec> {
        serde_json::from_value(util::read_json(&self.reg_file())["instances"].clone()).unwrap_or_default()
    }
    fn save_recs(&self, v: &[Rec]) -> Result<(), String> { util::write_json(&self.reg_file(), &json!({"instances": v})) }
    fn put_rec(&self, r: &Rec) -> Result<(), String> {
        let mut all = self.recs();
        match all.iter_mut().find(|x| x.id == r.id) { Some(x) => *x = r.clone(), None => all.push(r.clone()) }
        self.save_recs(&all)
    }
    /// An instance by its id, its library game, or its name.
    pub fn find(&self, key: &str) -> Result<Rec, String> {
        let all = self.recs();
        all.iter().find(|r| r.id == key).or_else(|| all.iter().find(|r| !r.library_id.is_empty() && r.library_id == key)).or_else(|| all.iter().find(|r| r.title.eq_ignore_ascii_case(key)))
            .cloned().ok_or_else(|| format!("There's no mod organizer instance called {key}."))
    }
    pub fn open(&self, key: &str) -> Result<Instance, String> { Instance::load(self.find(key)?) }
    fn unique_id(&self, want: &str) -> String {
        let base: String = want.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>().split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
        let base = if base.is_empty() || ["instances", "games", "create", "import", "recover"].contains(&base.as_str()) { format!("instance-{base}").trim_end_matches('-').to_string() } else { base };
        let all = self.recs();
        let mut id = base.clone();
        let mut n = 2;
        while all.iter().any(|r| r.id == id) { id = format!("{base}-{n}"); n += 1; }
        id
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ProfileSettings { pub local_saves: bool, pub local_inis: bool }

/// An instance, opened.
#[derive(Debug, Clone)]
pub struct Instance {
    pub rec: Rec,
    pub dir: PathBuf,
    pub ini: Ini,
    pub kind: Kind,
    pub game_dir: Option<PathBuf>,
    pub prefix: Option<PathBuf>,
    pub steam_app: Option<u64>,
    pub mods_dir: PathBuf,
    pub downloads_dir: PathBuf,
    pub profiles_dir: PathBuf,
    pub overwrite: PathBuf,
    pub profile: String,
}

impl Instance {
    pub fn load(rec: Rec) -> Result<Instance, String> {
        let dir = rec.dir.clone();
        if !dir.is_dir() { return Err(format!("The instance's folder ({}) isn't there anymore.", dir.display())); }
        let ini = Ini::read(&dir.join("ModOrganizer.ini"));
        let prefix = rec.prefix.clone();
        let base = ini.get("Settings", "base_directory").filter(|s| !s.is_empty()).and_then(|v| mo2::win_to_linux(&v, prefix.as_deref())).unwrap_or_else(|| dir.clone());
        let setting = |key: &str, default: &str| -> PathBuf {
            let v = ini.get("Settings", key).filter(|s| !s.trim().is_empty()).unwrap_or_else(|| format!("%BASE_DIR%/{default}"));
            if let Some(rest) = v.strip_prefix("%BASE_DIR%") { return base.join(rest.trim_start_matches(['/', '\\']).replace('\\', "/")); }
            mo2::win_to_linux(&v, prefix.as_deref()).unwrap_or_else(|| base.join(default))
        };
        let (mods_dir, downloads_dir, profiles_dir, overwrite) = (setting("mod_directory", "mods"), setting("download_directory", "downloads"), setting("profiles_directory", "profiles"), setting("overwrite_directory", "overwrite"));
        let kind = Kind::by_id(&rec.kind);
        let game_dir = rec.game_dir.clone().or_else(|| ini.get("General", "gamePath").and_then(|g| mo2::win_to_linux(&g, prefix.as_deref())));
        let mut inst = Instance { steam_app: rec.steam_app, rec, dir, ini, kind, game_dir, prefix, mods_dir, downloads_dir, profiles_dir, overwrite, profile: String::new() };
        let profiles = inst.profiles();
        let want = inst.ini.get("General", "selected_profile").unwrap_or_default();
        inst.profile = if profiles.contains(&want) { want } else if profiles.iter().any(|p| p == "Default") { "Default".into() } else { profiles.first().cloned().unwrap_or_else(|| "Default".into()) };
        Ok(inst)
    }
    pub fn hearth_dir(&self) -> PathBuf { self.dir.join(".hearth") }
    pub fn profile_dir(&self) -> PathBuf { self.profiles_dir.join(&self.profile) }
    /// The folder mods go into (the game's Data folder for Bethesda games).
    pub fn data_target(&self) -> Option<PathBuf> {
        let g = self.game_dir.as_ref().filter(|g| g.is_dir())?;
        if self.kind.data.is_empty() { return Some(g.clone()); }
        vfs::ci_find(g, self.kind.data).filter(|d| d.is_dir())
    }
    /// The Windows user folder in the prefix.
    pub fn prefix_user(&self) -> Option<PathBuf> {
        let users = self.prefix.as_ref()?.join("drive_c/users");
        let su = users.join("steamuser");
        if su.is_dir() { return Some(su); }
        let me = users.join(std::env::var("USER").unwrap_or_default());
        if me.is_dir() { return Some(me); }
        std::fs::read_dir(&users).ok()?.flatten().map(|e| e.path()).find(|p| p.is_dir() && p.file_name().map(|n| n != "Public").unwrap_or(false))
    }
    /// The prefix Windows programs run in (the game's own; the instance's when the game has none yet).
    pub fn wine_prefix(&self) -> PathBuf { self.prefix.clone().unwrap_or_else(|| self.hearth_dir().join("prefix").join("pfx")) }
    pub fn profiles(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(&self.profiles_dir).map(|rd| rd.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
        v.sort_by_key(|s| s.to_lowercase());
        v
    }
    pub fn profile_settings(&self) -> ProfileSettings {
        let i = Ini::read(&self.profile_dir().join("settings.ini"));
        ProfileSettings { local_saves: i.get_bool("General", "LocalSaves").unwrap_or(false), local_inis: i.get_bool("General", "LocalSettings").unwrap_or(false) }
    }

    // -------------------------------------------------------------- the mod list

    fn mod_folders(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(&self.mods_dir).map(|rd| rd.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| !n.starts_with('.')).collect()).unwrap_or_default();
        v.sort_by_key(|s| s.to_lowercase());
        v
    }
    /// The profile's mods, lowest priority first: modlist.txt's order; mods it doesn't list yet come last, off;
    /// lines for mods whose folder is gone are left out (as MO2 does).
    pub fn modlist(&self) -> Vec<ModLine> {
        let text = std::fs::read(self.profile_dir().join("modlist.txt")).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
        let folders = self.mod_folders();
        let mut list: Vec<ModLine> = mo2::parse_modlist(&text).into_iter().filter(|m| m.state == '*' || folders.iter().any(|f| f == &m.name)).collect();
        for f in folders { if !list.iter().any(|m| m.name == f) { list.push(ModLine { name: f, state: '-' }); } }
        list
    }
    pub fn save_modlist(&self, list: &[ModLine]) -> Result<(), String> {
        std::fs::create_dir_all(self.profile_dir()).map_err(|e| e.to_string())?;
        util::write_atomic(&self.profile_dir().join("modlist.txt"), mo2::write_modlist(list).as_bytes()).map_err(|e| format!("Couldn't save modlist.txt ({e})"))
    }
    fn find_mod(list: &[ModLine], name: &str) -> Result<usize, String> {
        list.iter().position(|m| m.name == name).or_else(|| list.iter().position(|m| m.name.eq_ignore_ascii_case(name))).ok_or_else(|| format!("There's no mod called {name} in this profile."))
    }
    /// The view's sources: the enabled mods, lowest priority first, then overwrite.
    pub fn sources(&self) -> Result<Vec<vfs::Source>, String> {
        let _ = std::fs::create_dir_all(&self.overwrite);
        let mut v: Vec<vfs::Source> = self.modlist().into_iter().filter(|m| m.state == '+').map(|m| vfs::Source { dir: self.mods_dir.join(&m.name), name: m.name }).filter(|s| s.dir.is_dir()).collect();
        v.push(vfs::Source { name: "Overwrite".into(), dir: self.overwrite.clone() });
        Ok(v)
    }
    pub fn set_enabled(&self, name: &str, on: bool) -> Result<(), String> {
        let mut l = self.modlist();
        let i = Self::find_mod(&l, name)?;
        if l[i].state == '*' { return Err("That's the game's own content: it's always on.".into()); }
        l[i].state = if on { '+' } else { '-' };
        self.save_modlist(&l)
    }
    /// Moves a mod to a priority (0 = lowest; higher wins).
    pub fn move_mod(&self, name: &str, to: i64) -> Result<(), String> {
        let mut l = self.modlist();
        let i = Self::find_mod(&l, name)?;
        let m = l.remove(i);
        let j = to.clamp(0, l.len() as i64) as usize;
        l.insert(j, m);
        self.save_modlist(&l)
    }
    pub fn remove_mod(&self, name: &str) -> Result<(), String> {
        let l = self.modlist();
        let i = Self::find_mod(&l, name)?;
        let real = l[i].name.clone();
        if l[i].state == '*' { return Err("That's the game's own content.".into()); }
        let dir = self.mods_dir.join(&real);
        if dir.parent() != Some(self.mods_dir.as_path()) { return Err("That mod's folder isn't in the instance.".into()); }
        std::fs::remove_dir_all(&dir).map_err(|e| format!("Couldn't delete the mod's folder ({e})"))?;
        for p in self.profiles() {
            let f = self.profiles_dir.join(&p).join("modlist.txt");
            let Ok(t) = std::fs::read_to_string(&f) else { continue };
            let left: Vec<ModLine> = mo2::parse_modlist(&t).into_iter().filter(|m| m.name != real).collect();
            let _ = util::write_atomic(&f, mo2::write_modlist(&left).as_bytes());
        }
        Ok(())
    }
    /// Hides a mod's file or folder from the game (MO2's .mohidden), or shows it again.
    pub fn hide(&self, name: &str, rel: &str, hide: bool) -> Result<(), String> {
        let l = self.modlist();
        let i = Self::find_mod(&l, name)?;
        let r = util::clean_rel(rel).ok_or("That isn't a file in the mod.")?;
        let base = self.mods_dir.join(&l[i].name);
        let r = r.to_string_lossy().to_string();
        let (from, to) = if hide { (r.clone(), format!("{r}.mohidden")) } else { (r.clone(), r.strip_suffix(".mohidden").map(str::to_string).unwrap_or_else(|| r.clone())) };
        let src = vfs::ci_resolve(&base, &from).ok_or("That file isn't in the mod.")?;
        let dest = src.with_file_name(Path::new(&to).file_name().ok_or("bad name")?);
        if src == dest { return Ok(()); }
        std::fs::rename(&src, &dest).map_err(|e| e.to_string())
    }

    // -------------------------------------------------------------- profiles

    pub fn create_profile(&self, name: &str, from: Option<&str>) -> Result<(), String> {
        let n = mo2::clean_name(name);
        let dir = self.profiles_dir.join(&n);
        if dir.exists() { return Err(format!("There's already a profile called {n}.")); }
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        match from {
            Some(f) => {
                let src = self.profiles_dir.join(f);
                if !src.is_dir() { let _ = std::fs::remove_dir(&dir); return Err(format!("There's no profile called {f}.")); }
                for e in std::fs::read_dir(&src).map_err(|e| e.to_string())?.flatten() { if e.path().is_file() { let _ = std::fs::copy(e.path(), dir.join(e.file_name())); } }
            }
            None => write_empty_profile(&dir)?,
        }
        Ok(())
    }
    pub fn select_profile(&mut self, name: &str) -> Result<(), String> {
        let p = self.profiles().into_iter().find(|p| p == name || p.eq_ignore_ascii_case(name)).ok_or_else(|| format!("There's no profile called {name}."))?;
        mo2::set_in_file(&self.dir.join("ModOrganizer.ini"), "General", "selected_profile", &mo2::qt_bytearray(&p))?;
        self.ini = Ini::read(&self.dir.join("ModOrganizer.ini"));
        self.profile = p;
        Ok(())
    }
    pub fn set_profile_settings(&self, local_saves: Option<bool>, local_inis: Option<bool>) -> Result<(), String> {
        let f = self.profile_dir().join("settings.ini");
        let mut i = Ini::read(&f);
        if let Some(v) = local_saves { i.set_raw("General", "LocalSaves", if v { "true" } else { "false" }); }
        if let Some(v) = local_inis {
            i.set_raw("General", "LocalSettings", if v { "true" } else { "false" });
            // The profile starts from the game's current inis (as MO2 does).
            if v { if let Some(user) = self.prefix_user() {
                let mg = vfs::ci_join(&user, &format!("Documents/My Games/{}", self.kind.folder));
                for ini in self.kind.inis { if vfs::ci_find(&self.profile_dir(), ini).is_none() { if let Some(src) = vfs::ci_find(&mg, ini).filter(|p| p.is_file()) { let _ = std::fs::copy(src, self.profile_dir().join(ini)); } } }
            } }
        }
        i.write(&f)
    }

    // -------------------------------------------------------------- plugins

    /// The plugins at the top of the view: the game's own and the enabled mods' (the winner of each).
    pub fn present_plugins(&self, view: &vfs::View, sources: &[vfs::Source]) -> plugins::Present {
        let mut m = plugins::Present::new();
        if let Some(t) = self.data_target() {
            for (k, rel) in &view.base { if !rel.contains('/') && plugins::is_plugin(rel) { m.insert(k.clone(), (rel.clone(), (String::new(), t.join(rel)))); } }
        }
        for (k, e) in &view.files {
            if !e.rel.contains('/') && plugins::is_plugin(&e.rel) {
                let shown = m.get(k).map(|x| x.0.clone()).unwrap_or_else(|| e.rel.clone());
                m.insert(k.clone(), (shown, (sources.get(e.winner).map(|s| s.name.clone()).unwrap_or_default(), e.src.clone())));
            }
        }
        m
    }
    pub fn plugin_list(&self, view: &vfs::View, sources: &[vfs::Source]) -> Vec<plugins::Plugin> {
        if !self.kind.has_plugins() { return Vec::new(); }
        let read = |n: &str| std::fs::read(self.profile_dir().join(n)).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
        let saved = mo2::parse_plugins(&read("plugins.txt"), self.kind.star);
        let lo = mo2::parse_loadorder(&read("loadorder.txt"));
        let ccc: Vec<String> = if self.kind.ccc.is_empty() { Vec::new() } else {
            self.game_dir.as_ref().and_then(|g| vfs::ci_find(g, self.kind.ccc)).and_then(|f| std::fs::read_to_string(f).ok()).map(|t| mo2::parse_loadorder(&t)).unwrap_or_default()
        };
        plugins::load_order(&self.kind, &self.present_plugins(view, sources), &saved, &lo, &ccc)
    }
    fn current_plugins(&self) -> Vec<plugins::Plugin> {
        let s = self.sources().unwrap_or_default();
        let v = vfs::resolve(self.data_target().as_deref(), &s);
        self.plugin_list(&v, &s)
    }
    pub fn set_plugin(&self, name: &str, on: bool) -> Result<(), String> {
        let mut l = self.current_plugins();
        let p = l.iter_mut().find(|p| p.name.eq_ignore_ascii_case(name)).ok_or_else(|| format!("There's no plugin called {name}."))?;
        if p.implicit { return Err("The game always loads that one.".into()); }
        p.active = on;
        plugins::check(&mut l);
        plugins::save(&self.profile_dir(), &self.kind, &l)
    }
    /// Moves a plugin to a place in the load order (masters stay before the others, as the game loads them).
    pub fn move_plugin(&self, name: &str, to: i64) -> Result<(), String> {
        let mut l = self.current_plugins();
        let i = l.iter().position(|p| p.name.eq_ignore_ascii_case(name)).ok_or_else(|| format!("There's no plugin called {name}."))?;
        if l[i].implicit { return Err("The game always loads that one first.".into()); }
        let p = l.remove(i);
        let first = l.iter().take_while(|p| p.implicit).count();
        l.insert((to.max(0) as usize).clamp(first, l.len()), p);
        let (m, o): (Vec<plugins::Plugin>, Vec<plugins::Plugin>) = l.into_iter().partition(|p| p.master || p.implicit);
        let mut l = m;
        l.extend(o);
        plugins::check(&mut l);
        plugins::save(&self.profile_dir(), &self.kind, &l)
    }
    pub fn sort_plugins(&self) -> Result<Vec<plugins::Plugin>, String> {
        let s = plugins::sort(&self.current_plugins());
        plugins::save(&self.profile_dir(), &self.kind, &s)?;
        Ok(s)
    }

    // -------------------------------------------------------------- executables

    pub fn executables(&self) -> Vec<vfs::Executable> {
        let mut v: Vec<vfs::Executable> = Vec::new();
        let k = self.kind;
        if let Some(g) = self.game_dir.as_ref() {
            let mut add = |title: String, file: &str, extender: bool| {
                if file.is_empty() { return; }
                if let Some(p) = vfs::ci_find(g, file).filter(|p| p.is_file()) { v.push(vfs::Executable { title, binary: p, extender, from: "game".into(), present: true, ..Default::default() }); }
            };
            add(k.extender_name.to_string(), k.extender, true);
            add(k.mo2_name().to_string(), k.exe, false);
            add(format!("{} Launcher", k.mo2_name()), k.launcher, false);
        }
        let n: usize = self.ini.get("customExecutables", "size").and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        for i in 1..=n.min(200) {
            let g = |key: &str| self.ini.get("customExecutables", &format!("{i}\\{key}")).unwrap_or_default();
            let (title, bin) = (g("title"), g("binary"));
            if title.is_empty() || bin.is_empty() { continue; }
            let Some(binary) = mo2::win_to_linux(&bin, self.prefix.as_deref()) else { continue };
            if v.iter().any(|x| x.binary == binary || x.title.eq_ignore_ascii_case(&title)) { continue; }
            let cwd = Some(g("workingDirectory")).filter(|s| !s.is_empty()).and_then(|c| mo2::win_to_linux(&c, self.prefix.as_deref()));
            let extender = !k.extender.is_empty() && binary.file_name().map(|f| f.to_string_lossy().eq_ignore_ascii_case(k.extender)).unwrap_or(false);
            v.push(vfs::Executable { present: binary.is_file(), title, binary, args: g("arguments"), cwd, extender, from: "mo2".into() });
        }
        v
    }
    /// The one to run: the title asked for, else the script extender, else the game.
    pub fn executable(&self, title: Option<&str>) -> Result<vfs::Executable, String> {
        let all = self.executables();
        if let Some(t) = title { return all.into_iter().find(|e| e.title.eq_ignore_ascii_case(t)).ok_or_else(|| format!("There's nothing called {t} to start.")); }
        all.iter().find(|e| e.extender && e.present).or_else(|| all.iter().find(|e| e.present)).cloned().ok_or_else(|| "There's nothing to start: the game's program isn't in its folder.".into())
    }

    // -------------------------------------------------------------- guards

    /// Changes wait while the game runs through the organizer.
    pub fn guard(&self) -> Result<(), String> {
        match vfs::read_active(self) { Some(a) if vfs::alive(a.pid) => Err("The game is running through the organizer: close it first.".into()), _ => Ok(()) }
    }
    /// After a change, a view deployed by hand is made again.
    pub fn refresh_deploy(&self) -> Result<(), String> {
        if let Some(a) = vfs::read_active(self) {
            if !vfs::alive(a.pid) { let overlay = a.mode == "overlay"; vfs::undeploy(self, false)?; vfs::deploy(self, 0, overlay)?; }
        }
        Ok(())
    }

    // -------------------------------------------------------------- what the screens show

    pub fn public(&self) -> Value {
        json!({"id": self.rec.id, "title": self.rec.title, "dir": self.dir, "game": self.kind.id, "gameName": if self.kind.is_generic() { self.rec.title.clone() } else { self.kind.mo2_name().to_string() },
               "gameDir": self.game_dir, "dataDir": self.data_target(), "prefix": self.prefix, "libraryId": self.rec.library_id, "wabbajack": self.rec.wabbajack,
               "launcher": if self.rec.launcher.is_empty() { "hearth" } else { self.rec.launcher.as_str() }, "profile": self.profile,
               "modsDir": self.mods_dir, "downloadsDir": self.downloads_dir, "overwriteDir": self.overwrite, "plugins": self.kind.has_plugins()})
    }
    pub fn profiles_public(&self) -> Value {
        Value::Array(self.profiles().into_iter().map(|p| {
            let i = Ini::read(&self.profiles_dir.join(&p).join("settings.ini"));
            json!({"name": p, "active": p == self.profile, "localSaves": i.get_bool("General", "LocalSaves").unwrap_or(false), "localInis": i.get_bool("General", "LocalSettings").unwrap_or(false)})
        }).collect())
    }

    /// Everything the mod screens show, in one answer.
    pub fn summary(&self) -> Value {
        let list = self.modlist();
        let sources = self.sources().unwrap_or_default();
        let target = self.data_target();
        let view = vfs::resolve(target.as_deref(), &sources);
        let conf = vfs::conflicts(&view, sources.len());
        let names: Vec<String> = sources.iter().map(|s| s.name.clone()).collect();
        let at: HashMap<&str, usize> = names.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
        let plugin_list = self.plugin_list(&view, &sources);
        let mut by_mod: HashMap<String, Vec<String>> = HashMap::new();
        for p in &plugin_list { by_mod.entry(p.from.clone()).or_default().push(p.name.clone()); }
        let names_of = |s: &std::collections::BTreeSet<usize>| s.iter().filter_map(|&i| names.get(i).cloned()).collect::<Vec<_>>();
        let mut mods: Vec<Value> = list.iter().enumerate().map(|(prio, m)| {
            let kind = if m.state == '*' { "unmanaged" } else if m.name.ends_with("_separator") { "separator" } else { "mod" };
            let meta = Ini::read(&self.mods_dir.join(&m.name).join("meta.ini"));
            let c = at.get(m.name.as_str()).and_then(|&i| conf.get(i));
            json!({"name": if kind == "separator" { m.name.trim_end_matches("_separator").to_string() } else { m.name.clone() }, "id": m.name, "priority": prio, "enabled": m.state != '-', "kind": kind,
                   "version": meta.get("General", "version").unwrap_or_default(), "nexusId": meta.get("General", "modid").and_then(|s| s.parse::<u64>().ok()).filter(|&n| n > 0),
                   "installedFrom": meta.get("General", "installationFile").unwrap_or_default(), "plugins": by_mod.get(&m.name).cloned().unwrap_or_default(),
                   "files": c.map(|c| json!(c.files)), "conflicts": c.map(|c| json!({"winsOver": names_of(&c.wins_over), "losesTo": names_of(&c.loses_to), "winning": c.winning, "losing": c.losing})),
                   "hidden": c.map(|c| c.hidden).unwrap_or(0)})
        }).collect();
        let ow = conf.last().cloned().unwrap_or_default();
        mods.push(json!({"name": "Overwrite", "id": "Overwrite", "priority": list.len(), "enabled": true, "kind": "overwrite", "files": ow.files,
                         "conflicts": {"winsOver": names_of(&ow.wins_over), "losesTo": [], "winning": ow.winning, "losing": 0}}));
        let mut root_files: Vec<(String, String)> = view.base.values().filter(|r| !r.contains('/')).map(|r| (r.clone(), String::new())).collect();
        root_files.retain(|(r, _)| !view.files.contains_key(&r.to_lowercase()));
        root_files.extend(view.files.values().filter(|e| !e.rel.contains('/')).map(|e| (e.rel.clone(), names.get(e.winner).cloned().unwrap_or_default())));
        let archives = plugins::archives(&root_files, &plugin_list);
        let mut warnings: Vec<String> = Vec::new();
        if self.game_dir.as_ref().map(|g| !g.is_dir()).unwrap_or(true) { warnings.push("The game's folder isn't there: check where the game is.".into()); }
        else if target.is_none() { warnings.push(format!("The game has no {} folder.", self.kind.data)); }
        if self.kind.is_generic() { warnings.push("Hearth doesn't know this game: mods go into its folder as they are, and it has no plugin list.".into()); }
        if self.kind.has_plugins() && self.prefix_user().is_none() { warnings.push("The game's Proton folder isn't there yet: start the game once without mods.".into()); }
        let missing = plugin_list.iter().filter(|p| p.active && !p.missing.is_empty()).count();
        if missing > 0 { warnings.push(format!("{missing} active plugin{} need masters that are missing, off or load after them.", if missing == 1 { "" } else { "s" })); }
        json!({"ok": true, "instance": self.public(), "profiles": self.profiles_public(), "mods": mods, "plugins": plugin_list, "archives": archives,
               "executables": self.executables(), "deployed": vfs::status(self), "warnings": warnings,
               "sort": {"loot": false, "note": "Sorting keeps your order and fixes what the game requires (masters first, every plugin after its masters). LOOT's masterlist isn't used yet."}})
    }

    // -------------------------------------------------------------- installing

    fn staging(&self) -> PathBuf { self.hearth_dir().join("staging") }
    /// Unpacks a mod archive (or copies a folder) to look at it. Returns its token.
    pub fn stage(&self, source: &Path) -> Result<String, String> {
        // Leftovers older than a day go.
        if let Ok(rd) = std::fs::read_dir(self.staging()) {
            for e in rd.flatten() { if e.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).map(|d| d.as_secs() > 86400).unwrap_or(false) { let _ = std::fs::remove_dir_all(e.path()); } }
        }
        let token = format!("s{}", util::now_ms());
        let dir = self.staging().join(&token);
        crate::mods::unpack_to(source, &dir.join("x")).inspect_err(|_| { let _ = std::fs::remove_dir_all(&dir); })?;
        let _ = std::fs::write(dir.join("source.txt"), source.to_string_lossy().as_bytes());
        Ok(token)
    }
    fn staged(&self, token: &str) -> Option<(PathBuf, String)> {
        if token.is_empty() || !token.chars().all(|c| c.is_ascii_alphanumeric()) { return None; }
        let dir = self.staging().join(token);
        if !dir.join("x").is_dir() { return None; }
        Some((dir.join("x"), std::fs::read_to_string(dir.join("source.txt")).unwrap_or_default()))
    }
    pub fn cancel_stage(&self, token: &str) {
        if !token.is_empty() && token.chars().all(|c| c.is_ascii_alphanumeric()) { let _ = std::fs::remove_dir_all(self.staging().join(token)); }
    }

    /// Active / Inactive / Missing for FOMOD conditions, from the current view.
    fn file_states(&self) -> HashMap<String, &'static str> {
        let s = self.sources().unwrap_or_default();
        let v = vfs::resolve(self.data_target().as_deref(), &s);
        let mut m: HashMap<String, &'static str> = HashMap::new();
        for k in v.base.keys().chain(v.files.keys()) { m.insert(k.clone(), "Inactive"); }
        for p in self.plugin_list(&v, &s) { if p.active { m.insert(p.name.to_lowercase(), "Active"); } }
        m
    }

    /// Installs a mod from an archive or folder (or a staged token): FOMOD installers and BAIN packages answer with
    /// what to choose first (`needs`), unless `choices` is given.
    pub fn install(&self, what: &str, name: Option<&str>, choices: Option<&Value>, enable: bool) -> Result<Value, String> {
        let (token, fresh) = match self.staged(what.trim_start_matches("token:")) { Some(_) => (what.trim_start_matches("token:").to_string(), false), None => (self.stage(Path::new(what))?, true) };
        let _ = fresh;
        let (root, source) = self.staged(&token).ok_or("The unpacked mod isn't there anymore: choose it again.")?;
        let src_name = Path::new(&source).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let stem = ["zip", "7z", "rar", "tgz", "gz", "xz", "zst", "bz2", "tar"].iter().fold(src_name.clone(), |n, e| n.strip_suffix(&format!(".{e}")).map(str::to_string).unwrap_or(n));
        let mut notes: Vec<String> = Vec::new();
        let want_name = name.filter(|n| !n.trim().is_empty()).map(str::to_string);
        // FOMOD
        if let Some((froot, cfg, info)) = fomod::find(&root) {
            let f = fomod::parse(&std::fs::read(&cfg).map_err(|e| e.to_string())?, info.and_then(|i| std::fs::read(i).ok()).as_deref(), Some(&froot))?;
            let states = self.file_states();
            let fs = |n: &str| -> &'static str { states.get(&n.to_lowercase()).copied().unwrap_or("Missing") };
            let mod_name = mo2::clean_name(&want_name.clone().unwrap_or_else(|| if f.name.is_empty() { stem.clone() } else { f.name.clone() }));
            let Some(ch) = choices else {
                let e = fomod::evaluate(&f, &Vec::new(), &fs);
                return Ok(json!({"ok": true, "needs": "fomod", "token": token, "name": mod_name, "fomod": f, "evaluation": e}));
            };
            let e = fomod::evaluate(&f, &fomod::choices_from(ch), &fs);
            if !e.errors.is_empty() { return Err(e.errors.join(" ")); }
            let dest = self.new_mod_dir(&mod_name);
            let (n, problems) = fomod::apply(&e.files, &froot, &dest);
            notes.extend(problems);
            if n == 0 { let _ = std::fs::remove_dir_all(&dest); return Err(format!("Those choices install no files. {}", notes.join(" "))); }
            return self.finish_install(&token, &dest, &src_name, enable, n, notes);
        }
        // BAIN
        if let Some(pk) = bain_packages(&root) {
            let mod_name = mo2::clean_name(&want_name.clone().unwrap_or_else(|| stem.clone()));
            let Some(ch) = choices else {
                return Ok(json!({"ok": true, "needs": "bain", "token": token, "name": mod_name, "packages": pk.iter().map(|p| json!({"name": p, "default": p.starts_with("00") || p.to_lowercase().contains("core")})).collect::<Vec<_>>()}));
            };
            let picked: Vec<String> = ch.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string).or_else(|| x.as_u64().and_then(|i| pk.get(i as usize).cloned()))).collect()).unwrap_or_default();
            let picked: Vec<&String> = pk.iter().filter(|p| picked.contains(p)).collect();
            if picked.is_empty() { return Err("Choose at least one part of the package.".into()); }
            let dest = self.new_mod_dir(&mod_name);
            let mut n = 0;
            for p in picked {
                let from = root.join(p);
                let mut files = Vec::new();
                util::files_under(&from, &mut files);
                for f in files {
                    let Ok(r) = f.strip_prefix(&from) else { continue };
                    if crate::mods::copy_file(&f, &vfs::ci_join(&dest, &r.to_string_lossy())).is_ok() { n += 1; }
                }
            }
            return self.finish_install(&token, &dest, &src_name, enable, n, notes);
        }
        // A plain archive: its Data folder.
        let (data, mut more) = data_root(&root, &self.kind);
        notes.append(&mut more);
        let mod_name = mo2::clean_name(&want_name.unwrap_or_else(|| stem.clone()));
        let dest = self.new_mod_dir(&mod_name);
        if std::fs::rename(&data, &dest).is_err() {
            crate::mods::copy_tree(&data, &dest).map_err(|e| format!("Couldn't keep the mod's files ({e})"))?;
        }
        let mut files = Vec::new();
        util::files_under(&dest, &mut files);
        if files.is_empty() { let _ = std::fs::remove_dir_all(&dest); self.cancel_stage(&token); return Err("That mod has no files in it.".into()); }
        let n = files.len();
        self.finish_install(&token, &dest, &src_name, enable, n, notes)
    }
    fn new_mod_dir(&self, name: &str) -> PathBuf {
        let mut d = self.mods_dir.join(name);
        let mut i = 2;
        while d.exists() { d = self.mods_dir.join(format!("{name} ({i})")); i += 1; }
        d
    }
    fn finish_install(&self, token: &str, dest: &Path, src_name: &str, enable: bool, files: usize, notes: Vec<String>) -> Result<Value, String> {
        let name = dest.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let mut meta = Ini::read(&dest.join("meta.ini"));
        meta.set("General", "gameName", if self.kind.is_generic() { &self.rec.title } else { self.kind.mo2_name() });
        if meta.raw("General", "modid").is_none() { meta.set_raw("General", "modid", "0"); }
        if meta.raw("General", "version").is_none() { meta.set_raw("General", "version", ""); }
        meta.set("General", "installationFile", src_name);
        meta.set_raw("General", "installedBy", "Hearth");
        meta.write(&dest.join("meta.ini"))?;
        let mut l = self.modlist();
        l.retain(|m| m.name != name);
        l.push(ModLine { name: name.clone(), state: if enable { '+' } else { '-' } });
        self.save_modlist(&l)?;
        self.cancel_stage(token);
        Ok(json!({"ok": true, "installed": name, "files": files, "enabled": enable, "notes": notes}))
    }
}

fn write_empty_profile(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let w = |n: &str, t: String| util::write_atomic(&dir.join(n), t.as_bytes()).map_err(|e| e.to_string());
    if !dir.join("modlist.txt").exists() { w("modlist.txt", mo2::write_modlist(&[]))?; }
    if !dir.join("plugins.txt").exists() { w("plugins.txt", format!("{}\r\n", mo2::HEADER))?; }
    if !dir.join("settings.ini").exists() { w("settings.ini", "[General]\r\nLocalSaves=false\r\nLocalSettings=false\r\nAutomaticArchiveInvalidation=false\r\n".into())?; }
    Ok(())
}

// ------------------------------------------------------------------ what's in an archive

/// Folder names that mean "this is a Data folder" (Bethesda games), and files that do.
const DATA_DIRS: &[&str] = &["meshes", "textures", "scripts", "interface", "sound", "music", "strings", "seq", "skse", "f4se", "nvse", "obse", "sksevr", "f4sevr", "sfse",
    "materials", "lodsettings", "grass", "shadersfx", "video", "facegen", "distantlod", "lsdata", "mcm", "calientetools", "dyndolod", "netscriptframework", "source",
    "dialogueviews", "trees", "lod", "vis", "shaders", "fonts", "menus", "geometries", "particles", "terrain", "planetdata", "tools", "nemesis_engine", "platform", "config"];
const DATA_EXTS: &[&str] = &["esp", "esm", "esl", "bsa", "ba2"];
const DOC_EXTS: &[&str] = &["txt", "md", "pdf", "jpg", "jpeg", "png", "html", "htm", "rtf", "url", "docx", "webp"];

fn entries(dir: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut d = Vec::new();
    let mut f = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) { for e in rd.flatten() { let p = e.path(); if p.is_dir() { d.push(p); } else if p.is_file() { f.push(p); } } }
    d.sort();
    f.sort();
    (d, f)
}
fn ext_of(p: &Path) -> String { p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default() }
fn name_of(p: &Path) -> String { p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default() }
fn looks_like_data(dir: &Path) -> bool {
    let (d, f) = entries(dir);
    d.iter().any(|x| DATA_DIRS.contains(&name_of(x).as_str())) || f.iter().any(|x| DATA_EXTS.contains(&ext_of(x).as_str()))
}

/// Where a plain archive's Data folder is: a "Data" folder in it, or the folder that looks like one (with
/// meshes/, textures/, plugins...), through wrapping folders. Files beside a Data folder are for the game's own
/// folder (a script extender's loader...): the organizer doesn't change the game folder, so they're named in a note.
pub fn data_root(root: &Path, kind: &Kind) -> (PathBuf, Vec<String>) {
    if kind.data.is_empty() {
        // Other games: through single wrapping folders only.
        let mut cur = root.to_path_buf();
        for _ in 0..3 { let (d, f) = entries(&cur); if d.len() == 1 && f.iter().all(|x| DOC_EXTS.contains(&ext_of(x).as_str())) && f.is_empty() { cur = d[0].clone(); } else { break; } }
        return (cur, Vec::new());
    }
    let mut cur = root.to_path_buf();
    for _ in 0..4 {
        if looks_like_data(&cur) { return (cur, Vec::new()); }
        if let Some(d) = vfs::ci_find(&cur, kind.data).filter(|p| p.is_dir()) {
            let (dirs, files) = entries(&cur);
            let beside: Vec<String> = dirs.iter().filter(|x| **x != d && name_of(x) != "fomod").chain(files.iter().filter(|x| !DOC_EXTS.contains(&ext_of(x).as_str())))
                .map(|x| x.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()).collect();
            let notes = if beside.is_empty() { Vec::new() } else { vec![format!("These are for the game's own folder, which Hearth's organizer doesn't change; put them there yourself if the mod needs them: {}", beside.join(", "))] };
            return (d, notes);
        }
        let (dirs, files) = entries(&cur);
        let real_files = files.iter().filter(|x| !DOC_EXTS.contains(&ext_of(x).as_str())).count();
        if dirs.len() == 1 && real_files == 0 { cur = dirs[0].clone(); continue; }
        break;
    }
    (cur, vec!["It doesn't look like a Data folder (no meshes, textures or plugins at its top); it's installed as it is.".into()])
}

/// A BAIN package: numbered sub-packages ("00 Core", "01 Option") at its top, each a Data folder of its own.
pub fn bain_packages(root: &Path) -> Option<Vec<String>> {
    let (dirs, files) = entries(root);
    if files.iter().any(|x| !DOC_EXTS.contains(&ext_of(x).as_str())) { return None; }
    let numbered: Vec<String> = dirs.iter().map(|d| d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
        .filter(|n| n.len() > 2 && n.as_bytes()[0].is_ascii_digit() && n.as_bytes()[1].is_ascii_digit()).collect();
    if numbered.len() < 2 || !numbered.iter().all(|n| looks_like_data(&root.join(n))) { return None; }
    let mut v = numbered;
    v.sort();
    Some(v)
}

// ------------------------------------------------------------------ making and importing instances

/// A new instance for a game in Hearth's library (or the one it already has).
pub fn create(ctx: &Ctx, game: &GameEntry, kind: Option<&str>, title: Option<&str>) -> Result<Rec, String> {
    if let Some(r) = ctx.recs().into_iter().find(|r| r.library_id == game.id) { return Ok(r); }
    let install = game.install.clone().filter(|p| p.is_dir()).ok_or("Install the game first.")?;
    let k = kind.map(Kind::by_id).or_else(|| game.steam_app.and_then(Kind::by_steam)).or_else(|| Kind::by_folder(&install)).unwrap_or(kinds::GENERIC);
    let title = title.map(str::to_string).filter(|t| !t.trim().is_empty()).unwrap_or_else(|| game.title.clone());
    let id = ctx.unique_id(if k.is_generic() { &title } else { k.id });
    let dir = ctx.base().join(&id);
    for d in ["mods", "downloads", "overwrite", "profiles"] { std::fs::create_dir_all(dir.join(d)).map_err(|e| format!("Couldn't make the instance ({e})"))?; }
    write_empty_profile(&dir.join("profiles/Default"))?;
    let mut ini = Ini::default();
    ini.set("General", "gameName", if k.is_generic() { &title } else { k.mo2_name() });
    ini.set_raw("General", "gamePath", &mo2::qt_bytearray(&mo2::linux_to_win(&install)));
    ini.set_raw("General", "selected_profile", &mo2::qt_bytearray("Default"));
    ini.set_raw("General", "first_start", "false");
    ini.set_raw("Settings", "hearth_instance", "true");
    ini.write(&dir.join("ModOrganizer.ini"))?;
    let prefix = game.prefix.clone().or_else(|| match (&game.library_root, game.steam_app) { (Some(l), Some(a)) => Some(l.join("steamapps/compatdata").join(a.to_string()).join("pfx")), _ => None });
    let rec = Rec { id, title, dir, kind: k.id.into(), game_dir: Some(install), library_id: game.id.clone(), prefix, steam_app: game.steam_app.or(if k.steam > 0 { Some(k.steam) } else { None }),
                    launcher: "hearth".into(), added: util::now_ms(), ..Default::default() };
    ctx.put_rec(&rec)?;
    let _ = library_entry(ctx, &rec);
    Ok(rec)
}

/// Uses an existing MO2 instance (a folder with ModOrganizer.ini: a Wabbajack list, or your own) in place.
pub fn import(ctx: &Ctx, dir: &Path, title: Option<&str>, wabbajack: &str, prefix: Option<PathBuf>) -> Result<(Rec, Vec<String>), String> {
    let ini_file = dir.join("ModOrganizer.ini");
    if !ini_file.is_file() { return Err("That folder has no ModOrganizer.ini: choose a Mod Organizer 2 instance's folder.".into()); }
    let ini = Ini::read(&ini_file);
    let mut notes = Vec::new();
    let game_name = ini.get("General", "gameName").unwrap_or_default();
    let mut game_dir = ini.get("General", "gamePath").and_then(|g| mo2::win_to_linux(&g, prefix.as_deref())).filter(|g| g.is_dir());
    // Wabbajack lists built on their own copy of the game ("Stock Game", "Game Root") run on that copy.
    for own in ["Stock Game", "Game Root", "Stock Folder", "Skyrim Stock"] { if let Some(d) = vfs::ci_find(dir, own).filter(|d| d.is_dir()) { game_dir = Some(d); break; } }
    let k = Kind::by_name(&game_name).or_else(|| game_dir.as_deref().and_then(Kind::by_folder)).unwrap_or(kinds::GENERIC);
    if game_dir.is_none() {
        // The library's copy of the game.
        game_dir = ctx.games.iter().find(|g| k.steam > 0 && g.steam_app.and_then(Kind::by_steam).map(|x| x.id == k.id).unwrap_or(false)).and_then(|g| g.install.clone());
        if game_dir.is_some() { notes.push("Its game folder wasn't found from ModOrganizer.ini, so the library's copy of the game is used.".into()); }
    }
    let game_dir = game_dir.ok_or("Where the game is couldn't be worked out from ModOrganizer.ini (and it isn't in the library).")?;
    if k.is_generic() { notes.push(format!("Hearth doesn't know \"{game_name}\": mods go into the game folder as they are.")); }
    let existing = ctx.recs().into_iter().find(|r| r.dir == dir);
    let title = title.map(str::to_string).filter(|t| !t.trim().is_empty()).or_else(|| existing.as_ref().map(|r| r.title.clone())).unwrap_or_else(|| dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Mod Organizer".into()));
    let library = ctx.games.iter().find(|g| k.steam > 0 && g.steam_app.and_then(Kind::by_steam).map(|x| x.id == k.id).unwrap_or(false));
    let prefix = prefix.or_else(|| library.and_then(|g| g.prefix.clone()));
    let rec = Rec {
        id: existing.as_ref().map(|r| r.id.clone()).unwrap_or_else(|| ctx.unique_id(&title)),
        title, dir: dir.to_path_buf(), kind: k.id.into(), game_dir: Some(game_dir), library_id: String::new(), prefix,
        steam_app: if k.steam > 0 { Some(k.steam) } else { None }, wabbajack: wabbajack.into(),
        launcher: existing.as_ref().map(|r| r.launcher.clone()).filter(|l| !l.is_empty()).unwrap_or_else(|| "hearth".into()),
        added: existing.as_ref().map(|r| r.added).unwrap_or_else(util::now_ms),
    };
    ctx.put_rec(&rec)?;
    let inst = Instance::load(rec.clone())?;
    if inst.profiles().is_empty() { write_empty_profile(&inst.profiles_dir.join("Default"))?; }
    Ok((rec, notes))
}

/// The library entry for an instance (mods/modlists/<id>.json and .desktop, read by hearth-core): Play runs
/// `hearth-mods --organizer <id> run`. A Wabbajack list keeps its own entry; its "launch" names the Hearth or the
/// Mod Organizer 2 .desktop, as chosen.
fn hearth_desktop(ctx: &Ctx, rec: &Rec) -> Result<PathBuf, String> {
    let dir = ctx.modlists();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = if rec.wabbajack.is_empty() { format!("org-{}.desktop", rec.id) } else { format!("{}-hearth.desktop", rec.wabbajack) };
    let f = dir.join(name);
    let q = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
    let text = format!("[Desktop Entry]\nType=Application\nName={}\nExec={} --organizer {} run\nPath={}\n", rec.title.replace('\n', " "), q(&ctx.exe.to_string_lossy()), q(&rec.id), rec.dir.to_string_lossy());
    util::write_atomic(&f, text.as_bytes()).map_err(|e| e.to_string())?;
    Ok(f)
}
pub fn library_entry(ctx: &Ctx, rec: &Rec) -> Result<(), String> {
    let desktop = hearth_desktop(ctx, rec)?;
    if rec.wabbajack.is_empty() {
        let note = format!("{} (Hearth Mod Organizer)", rec.title);
        return util::write_json(&ctx.modlists().join(format!("org-{}.json", rec.id)), &json!({"id": format!("org-{}", rec.id), "title": note, "launch": desktop, "folder": rec.dir, "game": rec.kind, "added": rec.added, "note": "Hearth Mod Organizer"}));
    }
    let f = ctx.modlists().join(format!("{}.json", rec.wabbajack));
    let mut v = util::read_json(&f);
    if !v.is_object() { return Ok(()); }
    let mo2 = ctx.modlists().join(format!("{}.desktop", rec.wabbajack));
    let native = rec.launcher != "mo2";
    v["launch"] = json!(if native || !mo2.is_file() { desktop } else { mo2 });
    v["note"] = json!(if native { "Wabbajack modlist (Hearth Mod Organizer)" } else { "Wabbajack modlist (Mod Organizer 2)" });
    v["organizer"] = json!(rec.id);
    util::write_json(&f, &v)
}

/// Chooses what Play starts for an instance: "hearth" or "mo2" (Wabbajack lists only).
pub fn set_launcher(ctx: &Ctx, key: &str, which: &str) -> Result<Rec, String> {
    let mut rec = ctx.find(key)?;
    if !matches!(which, "hearth" | "mo2") { return Err("Choose hearth or mo2.".into()); }
    if which == "mo2" && rec.wabbajack.is_empty() { return Err("Only Wabbajack modlists have Mod Organizer 2 to fall back on.".into()); }
    rec.launcher = which.into();
    ctx.put_rec(&rec)?;
    library_entry(ctx, &rec)?;
    Ok(rec)
}

/// Called when a Wabbajack list has been installed (wabbajack.rs set_up): the list becomes an organizer instance
/// and Play runs it natively, with Mod Organizer 2 as the fallback. Returns a note for the page.
pub fn adopt_wabbajack(job: &str, title: &str, install_dir: &Path, prefix_dir: &Path) -> String {
    let ctx = Ctx::system(Vec::new());
    match import(&ctx, install_dir, Some(title), job, Some(prefix_dir.join("pfx"))) {
        Ok((rec, _)) => match library_entry(&ctx, &rec) {
            Ok(()) if rec.launcher == "mo2" => "Play starts Mod Organizer 2 (as you chose).".into(),
            Ok(()) => "Play runs it through Hearth's own Mod Organizer. If something's wrong, switch it to Mod Organizer 2 in the organizer.".into(),
            Err(e) => format!("Play starts Mod Organizer 2 (Hearth's organizer couldn't make its entry: {e})."),
        },
        Err(e) => format!("Play starts Mod Organizer 2 (Hearth's organizer couldn't read the list: {e})."),
    }
}

/// Every instance's view left by a crash or by hand is taken away (hearth-mods --remount, at sign-in).
pub fn recover_all(ctx: &Ctx) -> Vec<(String, Result<Value, String>)> {
    ctx.recs().into_iter().filter_map(|r| { let id = r.id.clone(); let inst = Instance::load(r).ok()?; vfs::recover(&inst).map(|res| (id, res)) }).collect()
}

// ------------------------------------------------------------------ the command interface

const VALUE_FLAGS: &[&str] = &["--name", "--choices", "--from", "--by", "--exe", "--kind", "--prefix", "--local-saves", "--local-inis", "--wabbajack"];

fn positional(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if VALUE_FLAGS.contains(&args[i].as_str()) { i += 2; continue; }
        if args[i].starts_with("--") { i += 1; continue; }
        out.push(args[i].clone());
        i += 1;
    }
    out
}
fn on_off(v: Option<String>) -> Result<Option<bool>, String> {
    match v.as_deref() { None => Ok(None), Some("on" | "true" | "1" | "yes") => Ok(Some(true)), Some("off" | "false" | "0" | "no") => Ok(Some(false)), Some(x) => Err(format!("{x}: say on or off.")) }
}
fn json_arg(v: Option<String>) -> Result<Option<Value>, String> {
    let Some(s) = v else { return Ok(None) };
    let text = match s.strip_prefix('@') { Some(f) => std::fs::read_to_string(f).map_err(|e| format!("Couldn't read {f} ({e})"))?, None => s };
    serde_json::from_str(&text).map(Some).map_err(|e| format!("The choices aren't JSON ({e})"))
}

/// `hearth-mods --organizer <args…>`: answers with JSON (docs/MODS.md lists the commands).
pub fn command(ctx: &Ctx, args: &[String]) -> Result<Value, String> {
    let pos = positional(args);
    let flag = |n: &str| util::arg_value(args, n);
    let first = pos.first().map(|s| s.as_str()).unwrap_or("instances");
    match first {
        "instances" => {
            let list: Vec<Value> = ctx.recs().into_iter().map(|r| match Instance::load(r.clone()) {
                Ok(i) => { let mut v = i.public(); v["deployed"] = vfs::status(&i); v["mods"] = json!(i.modlist().iter().filter(|m| m.state != '*').count()); v["modsOn"] = json!(i.modlist().iter().filter(|m| m.state == '+').count()); v }
                Err(e) => json!({"id": r.id, "title": r.title, "dir": r.dir, "error": e}),
            }).collect();
            return Ok(json!({"ok": true, "instances": list}));
        }
        "games" => return Ok(json!({"ok": true, "games": kinds::all_public()})),
        "create" => {
            let gid = pos.get(1).ok_or("Say which library game (its id, like steam:489830).")?;
            let g = ctx.games.iter().find(|g| &g.id == gid).ok_or("That game isn't in the library (or isn't installed).")?;
            let r = create(ctx, g, flag("--kind").as_deref(), flag("--name").as_deref())?;
            return Ok(json!({"ok": true, "created": r.id, "summary": Instance::load(r)?.summary()}));
        }
        "import" => {
            let dir = pos.get(1).ok_or("Say which folder (a Mod Organizer 2 instance).")?;
            let (r, notes) = import(ctx, Path::new(dir), flag("--name").as_deref(), &flag("--wabbajack").unwrap_or_default(), flag("--prefix").map(PathBuf::from))?;
            let _ = library_entry(ctx, &r);
            return Ok(json!({"ok": true, "imported": r.id, "notes": notes, "summary": Instance::load(r)?.summary()}));
        }
        "recover" => {
            let r: Vec<Value> = recover_all(ctx).into_iter().map(|(id, res)| match res { Ok(v) => json!({"id": id, "result": v}), Err(e) => json!({"id": id, "error": e}) }).collect();
            return Ok(json!({"ok": true, "recovered": r}));
        }
        _ => {}
    }
    let mut inst = ctx.open(first)?;
    let sub = pos.get(1).map(|s| s.as_str()).unwrap_or("summary");
    let arg = |i: usize, what: &str| pos.get(i).cloned().ok_or_else(|| format!("Say which {what}."));
    let mutating = matches!(sub, "enable" | "disable" | "move" | "remove" | "hide" | "unhide" | "plugin-enable" | "plugin-disable" | "plugin-move" | "sort" | "profile" | "profile-create" | "profile-settings" | "install");
    if mutating { inst.guard()?; }
    let mut extra = json!({});
    match sub {
        "summary" => return Ok(inst.summary()),
        "mods" | "plugins" | "archives" | "executables" | "profiles" | "status" | "warnings" => {
            let s = inst.summary();
            let key = match sub { "status" => "deployed", k => k };
            let mut o = json!({"ok": true, "instance": s["instance"]});
            o[sub] = s[key].clone();
            return Ok(o);
        }
        "conflicts" => {
            let sources = inst.sources()?;
            let view = vfs::resolve(inst.data_target().as_deref(), &sources);
            let names: Vec<String> = sources.iter().map(|s| s.name.clone()).collect();
            if let Some(m) = pos.get(2) {
                let i = names.iter().position(|n| n.eq_ignore_ascii_case(m)).ok_or_else(|| format!("{m} isn't on (only mods that are on have conflicts)."))?;
                return Ok(json!({"ok": true, "mod": names[i], "files": vfs::files_of(&view, i, &names)}));
            }
            let c = vfs::conflicts(&view, names.len());
            let list: Vec<Value> = names.iter().zip(c.iter()).map(|(n, c)| json!({"name": n, "files": c.files, "winning": c.winning, "losing": c.losing, "hidden": c.hidden,
                "winsOver": c.wins_over.iter().map(|&i| names[i].clone()).collect::<Vec<_>>(), "losesTo": c.loses_to.iter().map(|&i| names[i].clone()).collect::<Vec<_>>()})).collect();
            return Ok(json!({"ok": true, "conflicts": list}));
        }
        "enable" | "disable" => inst.set_enabled(&arg(2, "mod")?, sub == "enable")?,
        "move" => {
            let m = arg(2, "mod")?;
            let l = inst.modlist();
            let cur = Instance::find_mod(&l, &m)? as i64;
            let to = match flag("--by").and_then(|b| b.parse::<i64>().ok()) { Some(b) => cur + b, None => arg(3, "priority")?.parse::<i64>().map_err(|_| "Say the priority as a number (0 is the lowest).".to_string())? };
            inst.move_mod(&m, to)?;
        }
        "remove" => inst.remove_mod(&arg(2, "mod")?)?,
        "hide" | "unhide" => inst.hide(&arg(2, "mod")?, &arg(3, "file")?, sub == "hide")?,
        "plugin-enable" | "plugin-disable" => inst.set_plugin(&arg(2, "plugin")?, sub == "plugin-enable")?,
        "plugin-move" => inst.move_plugin(&arg(2, "plugin")?, arg(3, "place")?.parse::<i64>().map_err(|_| "Say the place as a number.".to_string())?)?,
        "sort" => { inst.sort_plugins()?; }
        "profile" => inst.select_profile(&arg(2, "profile")?)?,
        "profile-create" => { let n = arg(2, "name")?; inst.create_profile(&n, flag("--from").as_deref())?; extra["created"] = json!(mo2::clean_name(&n)); }
        "profile-settings" => inst.set_profile_settings(on_off(flag("--local-saves"))?, on_off(flag("--local-inis"))?)?,
        "install" => {
            let what = arg(2, "file, folder or token")?;
            let r = inst.install(&what, flag("--name").as_deref(), json_arg(flag("--choices"))?.as_ref(), !util::has_arg(args, "--off"))?;
            if r.get("needs").is_some() { return Ok(r); }
            extra = r;
        }
        "fomod" => {
            // Unpacks and reads the installer without installing (the token is for install … --choices).
            let what = arg(2, "file or folder")?;
            return inst.install(&what, flag("--name").as_deref(), None, true);
        }
        "fomod-eval" => {
            let token = arg(2, "token")?;
            let (root, _) = inst.staged(&token).ok_or("The unpacked mod isn't there anymore: choose it again.")?;
            let (froot, cfg, info) = fomod::find(&root).ok_or("That mod has no FOMOD installer.")?;
            let f = fomod::parse(&std::fs::read(&cfg).map_err(|e| e.to_string())?, info.and_then(|i| std::fs::read(i).ok()).as_deref(), Some(&froot))?;
            let states = inst.file_states();
            let fs = |n: &str| -> &'static str { states.get(&n.to_lowercase()).copied().unwrap_or("Missing") };
            let ch = json_arg(flag("--choices"))?.map(|v| fomod::choices_from(&v)).unwrap_or_default();
            return Ok(json!({"ok": true, "token": token, "evaluation": fomod::evaluate(&f, &ch, &fs)}));
        }
        "cancel" => { inst.cancel_stage(&arg(2, "token")?); return Ok(json!({"ok": true})); }
        "deploy" => {
            refuse_simple_list(ctx, &inst)?;
            let a = vfs::deploy(&inst, 0, !util::has_arg(args, "--links"))?;
            return Ok(json!({"ok": true, "deployed": vfs::status(&inst), "notes": a.notes}));
        }
        "undeploy" => return Ok(json!({"ok": true, "result": vfs::undeploy(&inst, false)?})),
        "run" => {
            refuse_simple_list(ctx, &inst)?;
            let ex = inst.executable(flag("--exe").as_deref())?;
            return vfs::run(&inst, &ex);
        }
        "launcher" => { let r = set_launcher(ctx, first, &arg(2, "launcher (hearth or mo2)")?)?; return Ok(json!({"ok": true, "launcher": r.launcher})); }
        "forget" => {
            if vfs::read_active(&inst).is_some() { return Err("Undeploy it first.".into()); }
            let all: Vec<Rec> = ctx.recs().into_iter().filter(|r| r.id != inst.rec.id).collect();
            ctx.save_recs(&all)?;
            let _ = std::fs::remove_file(ctx.modlists().join(format!("org-{}.json", inst.rec.id)));
            let _ = std::fs::remove_file(ctx.modlists().join(format!("org-{}.desktop", inst.rec.id)));
            return Ok(json!({"ok": true, "forgot": inst.rec.id, "note": "Its folder is kept."}));
        }
        other => return Err(format!("{other}: that isn't an organizer command (docs/MODS.md has them).")),
    }
    if mutating {
        inst = Instance::load(inst.rec.clone())?;
        if let Err(e) = inst.refresh_deploy() { extra["deployProblem"] = json!(e); }
    }
    let mut out = json!({"ok": true, "summary": inst.summary()});
    if let Value::Object(m) = extra { for (k, v) in m { out[k] = v; } }
    Ok(out)
}

/// Hearth Mods' own list for the same game mustn't be deployed at the same time.
fn refuse_simple_list(ctx: &Ctx, inst: &Instance) -> Result<(), String> {
    if inst.rec.library_id.is_empty() { return Ok(()); }
    let Some(g) = ctx.games.iter().find(|g| g.id == inst.rec.library_id) else { return Ok(()) };
    let gm = crate::mods::GameMods::open(&ctx.data, g);
    if !gm.state.ledger.deployed.is_empty() || crate::mods::overlay::mounted(g.install.as_deref()) {
        return Err("This game has mods on in Hearth Mods' simple list: turn those off first (both can't be in the game at once).".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;

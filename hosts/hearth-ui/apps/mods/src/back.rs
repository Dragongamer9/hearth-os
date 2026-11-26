//! The Mod Manager's way to Hearth Mods' own logic (hearth-gaming's mods_app: the Hearth Mod Organizer, Nexus Mods,
//! downloads, Wabbajack, Play modded, Add to Steam), always off the window's thread, and what it keeps of its own
//! (snapshots beside each instance, the updates it found, Describe it's last answer, the sources' searches).

use crate::logic;
use hearth_gaming::mods_app;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// One call by name (the same names the old page and Game Mode use), on a thread of its own.
pub fn call(name: &str, args: Value) -> impl std::future::Future<Output = Result<Value, String>> + Send + 'static {
    let name = name.to_string();
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("mods-call".into()).spawn(move || {
        let _ = tx.send(mods_app::route(&name, &args));
    });
    async move { rx.await.unwrap_or_else(|_| Err("Hearth Mods stopped answering.".into())) }
}

/// Any work off the window's thread.
pub fn work<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> impl std::future::Future<Output = T> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("mods-work".into()).spawn(move || {
        let _ = tx.send(f());
    });
    async move { rx.await.expect("the Mod Manager's work stopped") }
}

/// An organizer command (`hearth-mods --organizer <instance> …`).
pub fn organizer(args: Vec<String>) -> impl std::future::Future<Output = Result<Value, String>> + Send + 'static {
    call("organizer", json!({ "args": args }))
}

/// The Mod Manager's own folder: ~/.local/share/nimbus/mods/manager.
pub fn own_dir() -> PathBuf {
    hearth_gaming::util::data_dir().join("mods").join("manager")
}

pub fn read_own(name: &str) -> Value {
    hearth_ui::settings_file::read_json(&own_dir().join(name))
}

pub fn write_own(name: &str, v: &Value) -> Result<(), String> {
    let _ = std::fs::create_dir_all(own_dir());
    hearth_ui::settings_file::write_json(&own_dir().join(name), v).map_err(|e| e.to_string())
}

/// One of your games: a library game, an organizer instance, or both.
#[derive(Debug, Clone, Default)]
pub struct Game {
    pub key: String,
    pub title: String,
    pub mods: usize,
    pub on: usize,
    pub inst: Option<Value>,
    pub lib: Value,
    pub beacon: bool,
    /// 0 nothing, 1 something to look at (amber), 2 updates (blue)
    pub dot: u8,
}

impl Game {
    pub fn inst_id(&self) -> Option<String> {
        self.inst.as_ref().and_then(|i| i["id"].as_str()).map(str::to_string)
    }
    pub fn short(&self) -> String {
        self.title.replace("Special Edition", "SE").replace("The Elder Scrolls V: ", "")
    }
    /// How its mods go in (board 8).
    pub fn how(&self) -> String {
        let t = self.title.to_lowercase();
        if self.beacon || t.contains("middle-earth") || t.contains("bfme") {
            "through Beacon".into()
        } else if self.inst.is_some() {
            "Hearth Mod Organizer · plugins".into()
        } else if t.contains("cyberpunk") {
            "REDmod and archives".into()
        } else if t.contains("baldur") {
            "mod.io and Nexus · load order".into()
        } else if t.contains("stardew") {
            "SMAPI mods".into()
        } else if t.contains("valheim") {
            "BepInEx · Thunderstore".into()
        } else if t.contains("minecraft") {
            "Fabric · modpacks".into()
        } else {
            "into its folder".into()
        }
    }
}

/// Every game: the library's (Hearth Mods' own list) with the organizer's instances joined in.
pub fn load_world() -> Result<(Vec<Game>, Value), String> {
    let list = mods_app::route("list", &Value::Null).unwrap_or(Value::Array(Vec::new()));
    let inst = mods_app::route("organizer", &json!({"args": ["instances"]})).unwrap_or(Value::Null);
    let insts: Vec<Value> = inst["instances"].as_array().cloned().unwrap_or_default();
    let mut out: Vec<Game> = Vec::new();
    for g in list.as_array().cloned().unwrap_or_default() {
        let id = g["id"].as_str().unwrap_or("").to_string();
        if g["kind"].as_str() == Some("emulator") || id.starts_with("emu:") {
            continue;
        }
        let i = insts.iter().find(|i| i["libraryId"].as_str() == Some(id.as_str())).cloned();
        let (mods, on) = match &i {
            Some(i) => (i["mods"].as_u64().unwrap_or(0) as usize, i["modsOn"].as_u64().unwrap_or(0) as usize),
            None => (g["mods"].as_u64().unwrap_or(0) as usize, g["modsOn"].as_u64().unwrap_or(0) as usize),
        };
        out.push(Game { key: id, title: g["title"].as_str().unwrap_or("").to_string(), mods, on, beacon: g["beacon"].as_bool().unwrap_or(false), inst: i, lib: g, dot: 0 });
    }
    for i in insts {
        let lid = i["libraryId"].as_str().unwrap_or("");
        if !lid.is_empty() && out.iter().any(|g| g.key == lid) {
            continue;
        }
        let id = i["id"].as_str().unwrap_or("").to_string();
        out.push(Game { key: format!("instance:{id}"), title: i["title"].as_str().unwrap_or(&id).to_string(), mods: i["mods"].as_u64().unwrap_or(0) as usize, on: i["modsOn"].as_u64().unwrap_or(0) as usize, inst: Some(i), lib: Value::Null, beacon: false, dot: 0 });
    }
    // modded ones first, then by name
    out.sort_by(|a, b| (b.mods > 0).cmp(&(a.mods > 0)).then(b.mods.cmp(&a.mods)).then(a.title.to_lowercase().cmp(&b.title.to_lowercase())));
    Ok((out, inst))
}

/// What a game's page shows (board 1), read in one go.
#[derive(Debug, Clone, Default)]
pub struct Detail {
    pub key: String,
    /// the organizer's summary (mods with separators, plugins, profiles, executables, conflicts), or Hearth Mods'
    /// own list for a game without an instance
    pub s: Value,
    pub organizer: bool,
    pub snaps: Vec<logic::Snap>,
    pub rules: std::collections::BTreeMap<String, logic::Rule>,
    pub masterlist: String,
    pub crashes: Vec<(PathBuf, logic::Crash, u64)>,
    pub saves: Vec<(PathBuf, logic::Save, u64)>,
    pub game_files: (usize, usize),
}

fn mtime(p: &Path) -> u64 {
    std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0)
}

/// The masterlist as kept (checked when it came: only the pinned one is ever kept).
pub fn masterlist_file(kind: &str) -> PathBuf {
    own_dir().join("loot").join(format!("{kind}.yaml"))
}

/// Fetches LOOT's masterlist at its pinned commit and keeps it only if its SHA-256 is the pinned one.
pub fn fetch_masterlist(kind: &str) -> Result<String, String> {
    let (url, sha) = logic::masterlist_url(kind).ok_or("LOOT's masterlist isn't pinned for this game yet, so Sort uses the plugins' own masters.")?;
    let (code, _, body) = hearth_gaming::nexus::curl_get(&url, &[], 40)?;
    if code != 200 {
        return Err(format!("LOOT's masterlist didn't come (HTTP {code})."));
    }
    let got = logic::sha256_hex(body.as_bytes());
    if got != sha {
        return Err("LOOT's masterlist wasn't the pinned one (its SHA-256 differs), so it wasn't used.".into());
    }
    let f = masterlist_file(kind);
    let _ = std::fs::create_dir_all(f.parent().unwrap_or(Path::new("/")));
    hearth_ui::settings_file::write_text(&f, &body).map_err(|e| e.to_string())?;
    Ok(body)
}

/// Where a Bethesda game's crash loggers write, in its Proton prefix (My Games/<game>/SKSE or F4SE) and NetScriptFramework's folder.
pub fn crash_roots(inst: &Value) -> Vec<PathBuf> {
    let mut v = vec![own_dir().join("crashes")];
    let game = inst["gameName"].as_str().unwrap_or("");
    if let Some(pfx) = inst["prefix"].as_str() {
        let users = Path::new(pfx).join("drive_c/users");
        if let Ok(rd) = std::fs::read_dir(&users) {
            for u in rd.flatten() {
                for docs in ["Documents", "My Documents"] {
                    let mg = u.path().join(docs).join("My Games").join(game);
                    v.push(mg.join("SKSE"));
                    v.push(mg.join("F4SE"));
                }
            }
        }
    }
    if let Some(g) = inst["gameDir"].as_str() {
        v.push(Path::new(g).join("Data/NetScriptFramework/Crash"));
    }
    v
}

pub fn saves_dirs(inst: &Value, profile_dir: &Path, local: bool) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if local {
        v.push(profile_dir.join("saves"));
    }
    let game = inst["gameName"].as_str().unwrap_or("");
    if let Some(pfx) = inst["prefix"].as_str() {
        if let Ok(rd) = std::fs::read_dir(Path::new(pfx).join("drive_c/users")) {
            for u in rd.flatten() {
                v.push(u.path().join("Documents/My Games").join(game).join("Saves"));
            }
        }
    }
    v
}

pub fn load_detail(g: &Game) -> Detail {
    let mut d = Detail { key: g.key.clone(), ..Default::default() };
    if let Some(id) = g.inst_id() {
        d.organizer = true;
        d.s = mods_app::route("organizer", &json!({"args": [id.clone(), "summary"]})).unwrap_or_else(|e| json!({"error": e}));
        let inst = d.s["instance"].clone();
        let dir = PathBuf::from(inst["dir"].as_str().unwrap_or(""));
        d.snaps = logic::list(&dir);
        let kind = inst["game"].as_str().unwrap_or("").to_string();
        let ml = std::fs::read_to_string(masterlist_file(&kind)).unwrap_or_default();
        d.masterlist = if ml.is_empty() { String::new() } else { format!("LOOT's masterlist, {}", hearth_ui::time::day(mtime(&masterlist_file(&kind)) as i64)) };
        d.rules = logic::parse_masterlist(&ml);
        for (f, _) in logic::crash_logs(&crash_roots(&inst)).into_iter().take(8) {
            let text = std::fs::read_to_string(&f).unwrap_or_default();
            let c = logic::read_crash(&text);
            let t = mtime(&f);
            d.crashes.push((f, c, t));
        }
        let profile = inst["profile"].as_str().unwrap_or("").to_string();
        let pdir = PathBuf::from(inst["dir"].as_str().unwrap_or("")).join("profiles").join(&profile);
        let local = d.s["profiles"].as_array().and_then(|a| a.iter().find(|p| p["active"].as_bool() == Some(true))).and_then(|p| p["localSaves"].as_bool()).unwrap_or(false);
        let mut saves = Vec::new();
        for sd in saves_dirs(&inst, &pdir, local) {
            if let Ok(rd) = std::fs::read_dir(&sd) {
                for e in rd.flatten() {
                    if e.path().extension().map(|x| x == "ess" || x == "fos").unwrap_or(false) {
                        let b = std::fs::read(e.path()).unwrap_or_default();
                        if let Some(s) = logic::read_save(&b) {
                            saves.push((e.path(), s, mtime(&e.path())));
                        }
                    }
                }
            }
        }
        saves.sort_by(|a, b| b.2.cmp(&a.2));
        saves.truncate(12);
        d.saves = saves;
        let dep = &d.s["deployed"];
        d.game_files = (dep["notes"].as_array().map(|a| a.len()).unwrap_or(0), 0);
    } else if !g.key.is_empty() {
        d.s = mods_app::route("mods_of", &json!({"id": g.key})).unwrap_or_else(|e| json!({"error": e}));
    }
    d
}

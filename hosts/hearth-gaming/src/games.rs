//! The games Saves and Mods work with: the installed games in Hearth's library (hearth-core: Steam, Epic and GOG
//! through Hearth, your own games, Beacon's), with where each one's folder and Proton/Wine prefix are; games
//! installed with Heroic; and the emulators' save folders.

use crate::locate::Place;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEntry {
    pub id: String,
    pub title: String,
    /// steam, epic, gog, manual, beacon, heroic-epic, heroic-gog, emulator...
    pub source: String,
    /// The store, for the manifest's rules: steam, epic, gog or "".
    pub store: String,
    /// The game's own folder.
    pub install: Option<PathBuf>,
    /// The folder with drive_c in it (Proton's pfx, or a Wine prefix), when the game has one.
    pub prefix: Option<PathBuf>,
    pub steam_app: Option<u64>,
    pub gog_id: Option<u64>,
    /// Steam's library folder the game is in.
    pub library_root: Option<PathBuf>,
    pub steam_dir: Option<PathBuf>,
    /// Beacon's games: Beacon manages their mods.
    pub beacon: bool,
    /// Emulators: their save folders (there's no manifest entry for these).
    pub save_dirs: Vec<PathBuf>,
    /// "game", or "emulator" for an emulator's saves.
    pub kind: String,
}

impl GameEntry {
    /// This game's places, for finding and restoring its saves.
    pub fn place(&self) -> Place {
        let mut roots = Vec::new();
        if let Some(r) = &self.library_root { roots.push(r.clone()); }
        if let Some(s) = &self.steam_dir { if !roots.contains(s) { roots.push(s.clone()); } }
        if roots.is_empty() { if let Some(p) = self.install.as_ref().and_then(|i| i.parent()) { roots.push(p.to_path_buf()); } }
        Place {
            home: crate::util::home(), xdg_data: crate::util::xdg_data(), xdg_config: crate::util::xdg_config(), store: self.store.clone(),
            base: self.install.clone(), roots, prefix: self.prefix.clone(), steam_dir: if self.store == "steam" { self.steam_dir.clone() } else { None },
            steam_app: self.steam_app, user: std::env::var("USER").unwrap_or_default(),
        }
    }
    pub fn folder_name(&self) -> Option<String> { self.install.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()) }
}

/// The folder in a prefix that has drive_c: the prefix itself or its pfx (Proton).
pub fn prefix_root(p: &Path) -> Option<PathBuf> {
    [p.join("pfx"), p.to_path_buf()].into_iter().find(|x| x.join("drive_c").is_dir())
}

/// A game's Steam library folder: <library>/steamapps/common/<game>.
pub fn steam_library_of(install: &Path) -> Option<PathBuf> {
    let common = install.parent()?;
    let steamapps = common.parent()?;
    if common.file_name()? != "common" || steamapps.file_name()? != "steamapps" { return None; }
    steamapps.parent().map(Path::to_path_buf)
}

// ------------------------------------------------------------------ emulators

/// Emulators' save folders (installed natively or from Flathub): (id, title, folders that exist).
pub fn emulators(home: &Path, xdg_data: &Path, xdg_config: &Path) -> Vec<GameEntry> {
    let fp = |app: &str, rest: &str| home.join(".var/app").join(app).join(rest);
    let list: Vec<(&str, &str, Vec<PathBuf>)> = vec![
        ("retroarch", "RetroArch", vec![xdg_config.join("retroarch/saves"), xdg_config.join("retroarch/states"),
            fp("org.libretro.RetroArch", "config/retroarch/saves"), fp("org.libretro.RetroArch", "config/retroarch/states")]),
        ("dolphin", "Dolphin (GameCube and Wii)", vec![xdg_data.join("dolphin-emu/GC"), xdg_data.join("dolphin-emu/Wii/title"), xdg_data.join("dolphin-emu/StateSaves"),
            fp("org.DolphinEmu.dolphin-emu", "data/dolphin-emu/GC"), fp("org.DolphinEmu.dolphin-emu", "data/dolphin-emu/Wii/title"), fp("org.DolphinEmu.dolphin-emu", "data/dolphin-emu/StateSaves")]),
        ("pcsx2", "PCSX2 (PlayStation 2)", vec![xdg_config.join("PCSX2/memcards"), xdg_config.join("PCSX2/sstates"), fp("net.pcsx2.PCSX2", "config/PCSX2/memcards"), fp("net.pcsx2.PCSX2", "config/PCSX2/sstates")]),
        ("duckstation", "DuckStation (PlayStation)", vec![xdg_data.join("duckstation/memcards"), xdg_data.join("duckstation/savestates"), fp("org.duckstation.DuckStation", "data/duckstation/memcards"), fp("org.duckstation.DuckStation", "data/duckstation/savestates")]),
        ("ppsspp", "PPSSPP (PSP)", vec![xdg_config.join("ppsspp/PSP/SAVEDATA"), xdg_config.join("ppsspp/PSP/PPSSPP_STATE"), fp("org.ppsspp.PPSSPP", "config/ppsspp/PSP/SAVEDATA"), fp("org.ppsspp.PPSSPP", "config/ppsspp/PSP/PPSSPP_STATE")]),
        ("rpcs3", "RPCS3 (PlayStation 3)", vec![xdg_config.join("rpcs3/dev_hdd0/home/00000001/savedata"), fp("net.rpcs3.RPCS3", "config/rpcs3/dev_hdd0/home/00000001/savedata")]),
        ("cemu", "Cemu (Wii U)", vec![xdg_data.join("Cemu/mlc01/usr/save"), fp("info.cemu.Cemu", "data/Cemu/mlc01/usr/save")]),
        ("ryujinx", "Ryujinx (Switch)", vec![xdg_config.join("Ryujinx/bis/user/save"), fp("org.ryujinx.Ryujinx", "config/Ryujinx/bis/user/save")]),
        ("melonds", "melonDS (Nintendo DS)", vec![xdg_config.join("melonDS/saves"), fp("net.kuribo64.melonDS", "config/melonDS/saves")]),
        ("xemu", "xemu (Xbox)", vec![xdg_data.join("xemu/xemu"), fp("app.xemu.xemu", "data/xemu/xemu")]),
    ];
    list.into_iter().filter_map(|(id, title, dirs)| {
        let dirs: Vec<PathBuf> = dirs.into_iter().filter(|d| d.is_dir()).collect();
        if dirs.is_empty() { return None; }
        Some(GameEntry { id: format!("emulator:{id}"), title: format!("{title} saves"), source: "emulator".into(), kind: "emulator".into(), save_dirs: dirs, ..Default::default() })
    }).collect()
}

// ------------------------------------------------------------------ Heroic

/// Games installed with Heroic (Epic through legendary, GOG), with their Wine/Proton prefixes.
pub fn heroic(home: &Path, xdg_config: &Path) -> Vec<GameEntry> {
    let mut out = Vec::new();
    for dir in [xdg_config.join("heroic"), home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic")] {
        if !dir.is_dir() { continue; }
        let prefix_of = |app: &str| -> Option<PathBuf> {
            let v = crate::util::read_json(&dir.join("GamesConfig").join(format!("{app}.json")));
            v[app]["winePrefix"].as_str().map(PathBuf::from).and_then(|p| prefix_root(&p))
        };
        let epic = crate::util::read_json(&dir.join("legendaryConfig/legendary/installed.json"));
        for (app, e) in epic.as_object().cloned().unwrap_or_default() {
            let Some(path) = e["install_path"].as_str().map(PathBuf::from).filter(|p| p.is_dir()) else { continue };
            if e["is_dlc"] == true { continue; }
            let title = e["title"].as_str().map(str::to_string).unwrap_or_else(|| app.clone());
            out.push(GameEntry { id: format!("heroic-epic:{app}"), title, source: "heroic-epic".into(), store: "epic".into(), prefix: prefix_of(&app), install: Some(path), kind: "game".into(), ..Default::default() });
        }
        let gog = crate::util::read_json(&dir.join("gog_store/installed.json"));
        for e in gog["installed"].as_array().cloned().unwrap_or_default() {
            let (Some(app), Some(path)) = (e["appName"].as_str(), e["install_path"].as_str().map(PathBuf::from).filter(|p| p.is_dir())) else { continue };
            let info = crate::util::read_json(&path.join(format!("goggame-{app}.info")));
            let title = info["name"].as_str().map(str::to_string).unwrap_or_else(|| path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| app.to_string()));
            out.push(GameEntry { id: format!("heroic-gog:{app}"), title, source: "heroic-gog".into(), store: "gog".into(), prefix: prefix_of(app), install: Some(path),
                gog_id: app.parse().ok(), kind: "game".into(), ..Default::default() });
        }
    }
    out
}

// ------------------------------------------------------------------ Hearth's library

/// Every game Saves and Mods can work with, A to Z (emulators last).
#[cfg(target_os = "linux")]
pub fn list() -> Vec<GameEntry> {
    use hearth_core::library::Library;
    let home = crate::util::home();
    let store = hearth_core::store::Store::open(hearth_core::paths::data_dir().join("library.json"));
    let mut lib = Library::new(store, home.clone());
    lib.scan_quietly();
    // Beacon's games are read in the background: wait a moment for them (as `hearth --list-games` does).
    if lib.store.source_on("beacon") {
        let slot = lib.beacon_slot();
        for _ in 0..40 {
            if let Some(s) = slot.lock().unwrap_or_else(|e| e.into_inner()).take() { lib.beacon = s; lib.apply_beacon(); break; }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    let steam_dir = hearth_core::steam::steam_path(&home);
    let mut out: Vec<GameEntry> = Vec::new();
    for g in lib.games.values() {
        if !g.installed || g.cloud || g.source == "emu" || g.source == "xbox" { continue; }
        let (store_kind, rest) = g.id.split_once(':').unwrap_or(("", g.id.as_str()));
        let mut install = g.install_path.clone();
        if let Some(p) = install.clone() { if p.is_file() { install = p.parent().map(Path::to_path_buf); } }
        let mut e = GameEntry { id: g.id.clone(), title: g.title.clone(), source: g.source.to_string(), install, kind: "game".into(), steam_dir: steam_dir.clone(), ..Default::default() };
        e.store = match g.source { "steam" | "epic" | "gog" => g.source.to_string(), _ => String::new() };
        match g.source {
            "steam" => {
                e.steam_app = rest.parse().ok();
                e.library_root = e.install.as_deref().and_then(steam_library_of);
                if let (Some(lr), Some(app)) = (&e.library_root, e.steam_app) { e.prefix = prefix_root(&lr.join("steamapps/compatdata").join(app.to_string())); }
            }
            "gog" => { e.gog_id = rest.parse().ok(); e.prefix = prefix_root(&hearth_core::stores::prefix_of("gog", rest)); }
            "epic" => e.prefix = prefix_root(&hearth_core::stores::prefix_of("epic", rest)),
            "beacon" => {
                e.beacon = true;
                e.steam_app = g.beacon.as_ref().and_then(|b| b.steam_appid).map(u64::from);
                if e.steam_app.is_some() { e.store = "steam".into(); }
            }
            _ => e.prefix = prefix_root(&hearth_core::stores::prefix_of(store_kind, rest)),
        }
        out.push(e);
    }
    // Heroic's games that aren't already in Hearth's library.
    let have: std::collections::HashSet<String> = out.iter().map(|g| crate::manifest::norm(&g.title)).collect();
    for h in heroic(&home, &crate::util::xdg_config()) { if !have.contains(&crate::manifest::norm(&h.title)) { out.push(h); } }
    out.sort_by_key(|g| g.title.to_lowercase());
    out.extend(emulators(&home, &crate::util::xdg_data(), &crate::util::xdg_config()));
    out
}

#[cfg(not(target_os = "linux"))]
pub fn list() -> Vec<GameEntry> { Vec::new() }

/// Opens Beacon's own window (full screen in Game Mode). Beacon manages the Battle for Middle-earth games' mods.
#[cfg(target_os = "linux")]
pub fn open_beacon() -> Result<(), String> {
    let home = crate::util::home();
    let path = std::env::var("PATH").unwrap_or_default();
    let roots = [home.join(".local/share/flatpak"), PathBuf::from("/var/lib/flatpak")];
    let app = hearth_core::beacon::locate_app(&home, &path, &roots).ok_or("Beacon isn't installed. Get it from Hearth's Library › Game stores › Beacon.")?;
    hearth_core::beacon::open_app(&app, hearth_core::signin_handoff::game_mode())
}
#[cfg(not(target_os = "linux"))]
pub fn open_beacon() -> Result<(), String> { Err("Beacon opens from Hearth here.".into()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn steam_folders() {
        assert_eq!(steam_library_of(Path::new("/g/SteamLibrary/steamapps/common/Celeste")), Some(PathBuf::from("/g/SteamLibrary")));
        assert_eq!(steam_library_of(Path::new("/g/Games/Celeste")), None);
    }
    #[test]
    fn heroic_games_and_emulators() {
        let t = tempfile::tempdir().unwrap();
        let home = t.path().join("home");
        let cfg = home.join(".config");
        let game = home.join("Games/Heroic/Hades");
        std::fs::create_dir_all(&game).unwrap();
        let pfx = home.join("Games/Heroic/Prefixes/default/Hades");
        std::fs::create_dir_all(pfx.join("pfx/drive_c")).unwrap();
        std::fs::create_dir_all(cfg.join("heroic/legendaryConfig/legendary")).unwrap();
        std::fs::create_dir_all(cfg.join("heroic/GamesConfig")).unwrap();
        std::fs::write(cfg.join("heroic/legendaryConfig/legendary/installed.json"), format!(r#"{{"Min": {{"title": "Hades", "install_path": "{}"}}}}"#, game.display())).unwrap();
        std::fs::write(cfg.join("heroic/GamesConfig/Min.json"), format!(r#"{{"Min": {{"winePrefix": "{}"}}}}"#, pfx.display())).unwrap();
        let h = heroic(&home, &cfg);
        assert_eq!(h.len(), 1);
        assert_eq!((h[0].id.as_str(), h[0].store.as_str(), h[0].title.as_str()), ("heroic-epic:Min", "epic", "Hades"));
        assert_eq!(h[0].prefix, Some(pfx.join("pfx")));
        std::fs::create_dir_all(cfg.join("retroarch/saves")).unwrap();
        let e = emulators(&home, &home.join(".local/share"), &cfg);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].id, "emulator:retroarch");
    }
}

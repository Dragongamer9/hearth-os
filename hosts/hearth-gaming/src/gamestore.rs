//! Hearth Games (`hearth-gamestore`, docs/HEARTH-GAMES.md): Hearth's own store app for GOG, Epic and Battle.net.
//!
//! It is a window on Hearth's own store engine (hearth-core), not a Heroic fork: GOG installs with gogdl, Epic with
//! legendary (the same open-source tools Heroic uses, pinned with their checksums in hearth-core tools.rs), and
//! Battle.net is Blizzard's own app set up through Faugus Launcher (hearth-core battlenet.rs). The window's process
//! keeps one hearth-core Bridge, so sign-ins, the library, the download queue and uninstalls are exactly Hearth's: a
//! game installed here is in Hearth's library, Hearth Sync puts it in Steam Game Mode, and it starts there and here
//! the same way (`nimbus --play-now <id>`, desktop_games::play_now).
//!
//! Recomps (native PC ports of console games, docs/RECOMPS.md) are a tab of their own. Their engine is hearth-core's
//! recomps.rs, reached through the same Bridge (its `recomps.*` calls): the catalogue, installs from each project's
//! GitHub releases, checking your copy of the game, updates, uninstall. Installed ports are Hearth library entries
//! (`recomp:<id>`), so Hearth Sync keeps putting them in Steam Game Mode, and Play starts them the same way.
//!
//! This module is what doesn't need a window: which of Hearth's calls the page may make and with which arguments
//! (check), the drives new games can go to (drives) and choosing one (use_drive), which tab or store the window opens
//! on (start_view). apps/gamestore.rs is the window.

use serde_json::{json, Value};

/// The stores Hearth Games signs in to itself (on the store's own page). Battle.net signs in in its own app.
pub const STORES: &[&str] = &["gog", "epic"];

/// The library sources Hearth Games shows and works on.
pub const SOURCES: &[&str] = &["gog", "epic", "battlenet"];

/// Every Bridge call the page may make, with the arguments it may carry; anything else is refused.
pub const CALLS: &[(&str, &[&str])] = &[
    ("accounts.status", &[]), ("accounts.signIn", &["store"]), ("accounts.signOut", &["store"]), ("accounts.refresh", &["store"]),
    ("library.get", &["rescan"]), ("library.downloads", &[]),
    ("game.install", &["id"]), ("game.cancelInstall", &["id"]), ("game.pauseInstall", &["id"]), ("game.resumeInstall", &["id"]),
    ("game.installFirst", &["id"]), ("game.uninstall", &["id"]), ("game.info", &["id"]),
    ("downloads.pauseAll", &[]), ("downloads.resumeAll", &[]),
    ("battlenet.status", &[]), ("battlenet.install", &[]), ("battlenet.signInDesktop", &[]),
    // Recomps (hearth-core recomps.rs): ids are the catalogue's (zelda64, custom-owner-name), not library ids.
    ("recomps.list", &[]), ("recomps.details", &["id", "fresh"]), ("recomps.places", &[]), ("recomps.checkUpdates", &[]),
    ("recomps.install", &["id", "place"]), ("recomps.cancel", &["id"]), ("recomps.uninstall", &["id", "keep"]),
    ("recomps.pickGame", &["id"]), ("recomps.useGame", &["id", "path", "anyway"]), ("recomps.skip", &["id", "tag"]),
];

/// The tabs the window has (`--tab <name>` opens on one).
pub const TABS: &[&str] = &["library", "downloads", "stores", "recomps"];

/// A port's id in Hearth Recomps' catalogue: the same rule as hearth-core recomps.rs (lowercase letters, digits, '-').
pub fn recomp_id_ok(id: &str) -> bool {
    !id.is_empty() && id.len() <= 80 && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// What Play may start: one of this app's store games, or an installed port's library entry (recomp:<id>), which is
/// what the port's Steam Game Mode shortcut starts too.
pub fn playable(id: &str) -> bool {
    game_id_ok(id) || id.strip_prefix("recomp:").map(recomp_id_ok).unwrap_or(false)
}

/// "Use it anyway" only for the file Hearth itself just checked for that port (the engine's job for it says which),
/// never a path the page made up.
pub fn use_game_ok(job: &Value, path: &str) -> bool {
    !path.is_empty() && job["busy"] != true && job["file"].as_str() == Some(path)
}

/// Which tab and store the window opens on: `--tab recomps` / `--recomps`, `--store gog|epic|battlenet` (the library on
/// that store). Unknown values are ignored.
pub fn start_view(args: &[String]) -> (Option<String>, Option<String>) {
    let after = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let store = after("--store").filter(|s| SOURCES.contains(&s.as_str()));
    let tab = after("--tab").filter(|t| TABS.contains(&t.as_str()))
        .or_else(|| args.iter().any(|a| a == "--recomps").then(|| "recomps".to_string()))
        .or_else(|| store.as_ref().map(|_| "library".to_string()));
    (tab, store)
}

/// A game id Hearth Games works on: one of its stores' (gog:1207658924, epic:Fortnite, battlenet:wow), nothing else.
pub fn game_id_ok(id: &str) -> bool {
    let Some((src, rest)) = id.split_once(':') else { return false };
    SOURCES.contains(&src) && !rest.is_empty() && id.len() <= 200
        && rest.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':'))
}

/// The call and its arguments, cleaned: only the listed keys, plain short values, a known store, an id of one of
/// these stores. Err: why it's refused.
pub fn check(method: &str, args: &Value) -> Result<(String, Value), String> {
    let (m, keys) = CALLS.iter().find(|(n, _)| *n == method).ok_or_else(|| format!("{method:.40} isn't something Hearth Games asks"))?;
    if !(args.is_object() || args.is_null()) { return Err("The arguments have to be a JSON object".into()); }
    let mut out = serde_json::Map::new();
    for k in keys.iter() {
        let v = &args[*k];
        let max = if *k == "path" { 4096 } else { 200 };
        let ok = match v {
            Value::Null => continue,
            Value::Bool(_) => true,
            Value::String(s) => s.len() <= max && !s.chars().any(|c| c.is_control()),
            _ => false,
        };
        if !ok { return Err(format!("That isn't something Hearth Games sends ({k})")); }
        out.insert(k.to_string(), v.clone());
    }
    if keys.contains(&"store") {
        let s = out.get("store").and_then(Value::as_str).unwrap_or("");
        if !STORES.contains(&s) { return Err("Hearth Games signs in to GOG and Epic Games".into()); }
    }
    let recomp = m.starts_with("recomps.");
    if keys.contains(&"id") {
        let id = out.get("id").and_then(Value::as_str).unwrap_or("");
        if recomp { if !recomp_id_ok(id) { return Err("That isn't one of Hearth Recomps' ports".into()); } }
        else if !game_id_ok(id) { return Err("That isn't a GOG, Epic or Battle.net game".into()); }
    }
    // An install goes to this device ("" or none) or a drive Hearth Recomps offers (recomps.places: /run/media, /media).
    if let Some(p) = out.get("place").and_then(Value::as_str) {
        if !p.is_empty() && (p.contains("..") || !(p.starts_with("/run/media/") || p.starts_with("/media/"))) {
            return Err("Pick one of the places offered".into());
        }
    }
    if let Some(p) = out.get("path") {
        if !p.as_str().map(|p| p.starts_with('/')).unwrap_or(false) { return Err("That isn't a file on this device".into()); }
    }
    Ok((m.to_string(), Value::Object(out)))
}

/// The library's answer with only this app's games (GOG, Epic, Battle.net and Battle.net's own entry).
pub fn only_ours(mut lib: Value) -> Value {
    if let Some(a) = lib["games"].as_array_mut() {
        a.retain(|g| g["source"].as_str().map(|s| SOURCES.contains(&s)).unwrap_or(false));
    }
    lib
}

/// The downloads' answer with only this app's games.
pub fn only_our_downloads(mut d: Value) -> Value {
    for k in ["active", "queue", "recent", "items", "downloads"] {
        if let Some(a) = d[k].as_array_mut() {
            a.retain(|g| g["id"].as_str().map(game_id_ok).unwrap_or(true));
        }
    }
    d
}

/// The drives new GOG and Epic games can go to, with the one in use now.
#[cfg(target_os = "linux")]
pub fn drives(lib: &hearth_core::library::Library) -> Value {
    use std::os::unix::fs::MetadataExt;
    let ctx = hearth_core::diskspace::ctx_from(lib);
    let root = lib.install_root();
    let root_dev = std::fs::metadata(&root).or_else(|_| std::fs::metadata(root.parent().unwrap_or(&root))).map(|m| m.dev()).ok();
    let list: Vec<Value> = hearth_core::diskspace::drives(&ctx).into_iter().filter(|d| !d.read_only).map(|d| json!({
        "name": d.name, "kind": d.kind, "mount": d.mount.to_string_lossy(), "free": d.free, "size": d.size,
        "current": Some(d.dev) == root_dev,
    })).collect();
    json!({"installRoot": root.to_string_lossy(), "drives": list})
}

/// "Put new games on this drive": Hearth's install folder (new Epic and GOG installs) on a writable drive it lists,
/// the same rule as Settings › Storage and the Steam pages' "Use this disk for games" (core_service.rs games_disk).
/// Saved in library.json's installRoot, merged into the file (Store::update), nothing else touched.
#[cfg(target_os = "linux")]
pub fn use_drive(lib: &mut hearth_core::library::Library, mount: &str) -> Result<Value, String> {
    use std::os::unix::fs::MetadataExt;
    let ctx = hearth_core::diskspace::ctx_from(lib);
    let d = hearth_core::diskspace::drives(&ctx).into_iter().find(|d| d.mount.to_string_lossy() == mount).ok_or("That drive isn't one Hearth can put games on.")?;
    if d.read_only { return Err("That drive is read only.".into()); }
    let root = lib.install_root();
    let base = hearth_core::diskspace::games_base(&d.mount, d.dev, &root);
    std::fs::create_dir_all(&base).map_err(|e| format!("Couldn't make {} ({e})", base.display()))?;
    if std::fs::metadata(&base).map(|m| m.dev() != d.dev).unwrap_or(true) { return Err("That folder isn't on that drive.".into()); }
    let dir = base.to_string_lossy().to_string();
    lib.store.update(|x| { x.insert("installRoot".into(), json!(dir)); }).map_err(|e| e.to_string())?;
    hearth_core::bridge::log(&format!("hearth games: new Epic and GOG installs go to {} ({})", base.display(), d.name));
    Ok(drives(lib))
}

/// "%20" -> " " in a nimbus-art://file/ address (bytes, so a name with non-ASCII letters survives).
fn percent_decode(s: &str) -> String {
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) { out.push(h * 16 + l); i += 3; continue; }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The library's pictures (nimbus-art://cache/<name> for Hearth's art cache, nimbus-art://file/<path> for a picture on
/// this device), as Hearth's own window serves them (hearth-rs main.rs art_path): a picture file and its type, and only
/// picture files.
#[cfg(target_os = "linux")]
pub fn art_file(uri: &str) -> Option<(std::path::PathBuf, &'static str)> {
    let path = if let Some(name) = uri.splitn(2, "://cache/").nth(1) {
        hearth_core::artcache::file(name.split('?').next().unwrap_or(name))?
    } else {
        std::path::PathBuf::from(percent_decode(uri.splitn(2, "://file/").nth(1)?.split('?').next()?))
    };
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let mime = match ext.as_str() { "png" => "image/png", "jpg" | "jpeg" => "image/jpeg", "webp" => "image/webp", "gif" => "image/gif", "avif" => "image/avif", _ => return None };
    path.is_file().then_some((path, mime))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_its_own_calls_and_arguments() {
        assert!(check("library.get", &json!({"rescan": true})).is_ok());
        assert!(check("game.install", &json!({"id": "gog:1207658924"})).is_ok());
        assert!(check("game.install", &json!({"id": "epic:Fortnite", "path": "/etc"})).unwrap().1.get("path").is_none());
        // Steam's games stay with Steam; anything else Hearth's core does isn't this app's.
        assert!(check("game.uninstall", &json!({"id": "steam:620"})).is_err());
        assert!(check("game.install", &json!({"id": "manual:x"})).is_err());
        assert!(check("logs.send", &json!({})).is_err());
        assert!(check("login.password", &json!({})).is_err());
        assert!(check("remote.login", &json!({"user": "a", "pass": "b"})).is_err());
        // Sign-ins: GOG and Epic only (Battle.net signs in in its own app; Steam in Steam).
        assert!(check("accounts.signIn", &json!({"store": "gog"})).is_ok());
        assert!(check("accounts.signIn", &json!({"store": "steam"})).is_err());
        assert!(check("accounts.signIn", &json!({})).is_err());
        assert!(check("game.install", &json!({"id": 5})).is_err());
        assert!(check("game.install", &json!({"id": "gog:1\n2"})).is_err());
    }

    #[test]
    fn recomps_calls_go_to_the_engine_with_their_own_ids() {
        assert!(check("recomps.list", &json!({})).is_ok());
        assert!(check("recomps.details", &json!({"id": "zelda64", "fresh": true})).is_ok());
        assert!(check("recomps.install", &json!({"id": "zelda64"})).is_ok());
        assert!(check("recomps.install", &json!({"id": "custom-zelda64recomp-zelda64recomp"})).is_ok());
        assert!(check("recomps.install", &json!({"id": "zelda64", "place": ""})).is_ok());
        assert!(check("recomps.install", &json!({"id": "zelda64", "place": "/run/media/you/SD/Hearth Recomps"})).is_ok());
        assert!(check("recomps.install", &json!({"id": "zelda64", "place": "/etc"})).is_err());
        assert!(check("recomps.install", &json!({"id": "zelda64", "place": "/run/media/../../etc"})).is_err());
        // The engine's own ids, not library ids or store ids; nothing that walks out of its folder.
        assert!(check("recomps.install", &json!({"id": "recomp:zelda64"})).is_err());
        assert!(check("recomps.install", &json!({"id": "gog:1"})).is_err());
        assert!(check("recomps.install", &json!({"id": "../x"})).is_err());
        assert!(check("recomps.uninstall", &json!({"id": "zelda64", "keep": true})).unwrap().1["keep"] == true);
        assert!(check("recomps.uninstall", &json!({"id": "zelda64", "keep": "rm -rf"})).is_ok(), "a string keep is just not true");
        assert!(check("recomps.pickGame", &json!({"id": "zelda64"})).is_ok());
        let long = format!("/home/you/{}/Majora.z64", "a".repeat(300));
        assert!(check("recomps.useGame", &json!({"id": "zelda64", "path": long, "anyway": true})).is_ok(), "long paths are fine");
        assert!(check("recomps.useGame", &json!({"id": "zelda64", "path": "relative.z64", "anyway": true})).is_err());
        // What the old page could do beyond what this app offers stays out (adding projects, hiding, picking folders).
        for m in ["recomps.addCustom", "recomps.removeCustom", "recomps.adopt", "recomps.setExe", "recomps.hide", "recomps.backup"] {
            assert!(check(m, &json!({"id": "zelda64"})).is_err(), "{m}");
        }
        // And store calls still only take store ids.
        assert!(check("game.install", &json!({"id": "zelda64"})).is_err());
    }

    #[test]
    fn use_it_anyway_only_for_the_file_just_checked() {
        let job = json!({"busy": false, "mismatch": true, "error": "It doesn't match", "file": "/home/you/mm.z64"});
        assert!(use_game_ok(&job, "/home/you/mm.z64"));
        assert!(!use_game_ok(&job, "/etc/shadow"));
        assert!(!use_game_ok(&json!(null), "/home/you/mm.z64"));
        assert!(!use_game_ok(&json!({"busy": true, "file": "/home/you/mm.z64"}), "/home/you/mm.z64"));
        assert!(!use_game_ok(&job, ""));
    }

    #[test]
    fn play_starts_store_games_and_installed_ports() {
        assert!(playable("gog:1") && playable("recomp:zelda64") && playable("recomp:custom-a-b"));
        assert!(!playable("steam:620") && !playable("recomp:") && !playable("recomp:../x") && !playable("manual:x") && !playable("zelda64"));
    }

    #[test]
    fn opens_on_the_tab_asked_for() {
        let a = |v: &[&str]| start_view(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(a(&["hearth-gamestore"]), (None, None));
        assert_eq!(a(&["hearth-gamestore", "--tab", "recomps"]), (Some("recomps".into()), None));
        assert_eq!(a(&["hearth-gamestore", "--recomps"]), (Some("recomps".into()), None));
        assert_eq!(a(&["hearth-gamestore", "--store", "epic"]), (Some("library".into()), Some("epic".into())));
        assert_eq!(a(&["hearth-gamestore", "--tab", "nope", "--store", "steam"]), (None, None));
        assert_eq!(a(&["hearth-gamestore", "--fullscreen", "--tab", "downloads"]), (Some("downloads".into()), None));
    }

    #[test]
    fn picture_addresses() {
        assert_eq!(percent_decode("a%20b%C3%A9.png"), "a bé.png");
        assert_eq!(percent_decode("100%"), "100%");
        #[cfg(target_os = "linux")]
        {
            let t = tempfile::tempdir().unwrap();
            let png = t.path().join("c over.png");
            std::fs::write(&png, b"x").unwrap();
            let txt = t.path().join("notes.txt");
            std::fs::write(&txt, b"x").unwrap();
            let enc = |p: &std::path::Path| format!("nimbus-art://file/{}", p.to_string_lossy().replace(' ', "%20"));
            assert_eq!(art_file(&enc(&png)).map(|x| x.1), Some("image/png"));
            assert!(art_file(&enc(&txt)).is_none(), "only pictures");
        }
    }

    #[test]
    fn ids() {
        assert!(game_id_ok("gog:1207658924") && game_id_ok("epic:Sugar") && game_id_ok("battlenet:wow") && game_id_ok("battlenet:app"));
        assert!(!game_id_ok("steam:620") && !game_id_ok("gog:") && !game_id_ok("gog") && !game_id_ok("gog:../x") && !game_id_ok("epic:a b"));
    }

    #[test]
    fn the_library_shows_only_these_stores() {
        let v = only_ours(json!({"games": [{"id": "gog:1", "source": "gog"}, {"id": "steam:2", "source": "steam"}, {"id": "battlenet:app", "source": "battlenet"}]}));
        let ids: Vec<&str> = v["games"].as_array().unwrap().iter().map(|g| g["id"].as_str().unwrap()).collect();
        assert_eq!(ids, vec!["gog:1", "battlenet:app"]);
        let d = only_our_downloads(json!({"active": [{"id": "steam:2"}, {"id": "epic:x"}]}));
        assert_eq!(d["active"].as_array().unwrap().len(), 1);
    }
}

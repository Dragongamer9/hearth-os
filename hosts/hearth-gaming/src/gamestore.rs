//! Hearth Games (`hearth-gamestore`, docs/HEARTH-GAMES.md): Hearth's own store app for GOG, Epic and Battle.net.
//!
//! It is a window on Hearth's own store engine (hearth-core), not a Heroic fork: GOG installs with gogdl, Epic with
//! legendary (the same open-source tools Heroic uses, pinned with their checksums in hearth-core tools.rs), and
//! Battle.net is Blizzard's own app set up through Faugus Launcher (hearth-core battlenet.rs). The window's process
//! keeps one hearth-core Bridge, so sign-ins, the library, the download queue and uninstalls are exactly Hearth's: a
//! game installed here is in Hearth's library, Hearth Sync puts it in Steam Game Mode, and it starts there and here
//! the same way (`nimbus --play-now <id>`, desktop_games::play_now).
//!
//! This module is what doesn't need a window: which of Hearth's calls the page may make and with which arguments
//! (check), the drives new games can go to (drives) and choosing one (use_drive). apps/gamestore.rs is the window.

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
];

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
        let ok = match v {
            Value::Null => continue,
            Value::Bool(_) => true,
            Value::String(s) => s.len() <= 200 && !s.chars().any(|c| c.is_control()),
            _ => false,
        };
        if !ok { return Err(format!("That isn't something Hearth Games sends ({k})")); }
        out.insert(k.to_string(), v.clone());
    }
    if keys.contains(&"store") {
        let s = out.get("store").and_then(Value::as_str).unwrap_or("");
        if !STORES.contains(&s) { return Err("Hearth Games signs in to GOG and Epic Games".into()); }
    }
    if keys.contains(&"id") {
        let id = out.get("id").and_then(Value::as_str).unwrap_or("");
        if !game_id_ok(id) { return Err("That isn't a GOG, Epic or Battle.net game".into()); }
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

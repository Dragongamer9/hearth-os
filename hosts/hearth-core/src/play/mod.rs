//! Hearth's own take on Hydra Launcher's features (https://github.com/hydralauncher/hydra, MIT), written from
//! scratch for Hearth: see docs/HYDRA-FEATURES.md for the full checklist. The page reaches it with `play.*` calls.
//!  - Discover (discover.rs), game page details and time to beat (details.rs), achievements from official
//!    services (achievements.rs), custom artwork from SteamGridDB (artwork.rs), your gaming profile shared with
//!    Hearth friends (activity.rs), finding games in a folder (finder.rs), theme files (themes.rs), and a download
//!    manager for legitimate downloads (downloads.rs, sources.rs).
//!  - Every online service that needs a key uses the player's own, typed in Settings (keys.rs), off until then.
//! Not built, on purpose: community "download sources" / repack feeds, warez or file-hoster sites, debrid services,
//! torrents, and reading achievement files written by cracks or Steam emulators. Hearth only downloads games from
//! legitimate sources the player is entitled to.

pub mod achievements;
pub mod activity;
pub mod artwork;
pub mod details;
pub mod discover;
pub mod downloads;
pub mod finder;
pub mod keys;
pub mod sources;
pub mod themes;

use crate::bridge::Bridge;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The library as the page sees it (library.rs dto), for the parts that suggest or summarise.
fn dtos(br: &Bridge) -> Vec<Value> { br.library.games.values().map(|g| br.library.dto(g)).collect() }

/// A library game by the call's "id": (id, title, source).
fn game(br: &Bridge, a: &Value) -> Result<(String, String, String), String> {
    let id = a["id"].as_str().filter(|s| !s.is_empty()).ok_or("Missing 'id'")?;
    let g = br.library.games.get(id).ok_or("That game isn't in the library anymore.")?;
    Ok((id.to_string(), g.title.clone(), g.source.to_string()))
}

/// Paths of games added by hand (so Find games leaves them out).
fn manual_paths(br: &Bridge) -> HashSet<String> {
    br.library.store.data["manual"].as_array().into_iter().flatten().filter_map(|m| m["path"].as_str().map(str::to_string)).collect()
}

/// Adds a file to the library like Library › Add a game (an AppImage or Linux launcher is made runnable first).
pub fn add_file(br: &mut Bridge, file: &str, title: Option<&str>) -> Result<Value, String> {
    let p = Path::new(file);
    if !p.is_file() { return Err(format!("File not found: {file}")); }
    if manual_paths(br).contains(file) { return Err("That's in your library already.".into()); }
    let low = file.to_ascii_lowercase();
    if [".appimage", ".sh", ".x86_64", ".x86"].iter().any(|e| low.ends_with(e)) {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(m) = std::fs::metadata(p) { let mut perm = m.permissions(); perm.set_mode(perm.mode() | 0o755); let _ = std::fs::set_permissions(p, perm); }
    }
    let id = format!("manual:{:012x}", crate::library::now_ms() * 1000 + (std::process::id() as u64 % 1000) + br.library.store.data["manual"].as_array().map(|a| a.len() as u64).unwrap_or(0));
    let title = title.map(str::to_string).filter(|t| !t.trim().is_empty()).unwrap_or_else(|| crate::manual::title_for(file));
    let m = json!({"id": id, "title": title, "path": file, "added": crate::library::now_ms()});
    br.library.store.update(|d| { crate::store::array_in(d, "manual").push(m); }).map_err(|e| e.to_string())?;
    br.library.scan();
    br.library.games.get(&id).map(|g| br.library.dto(g)).ok_or("Couldn't add it".into())
}

/// Unpacks an archive into a folder of its own next to it (never outside it: bsdtar, tar and unzip refuse absolute
/// paths and "..").
pub fn unpack(file: &Path) -> Result<PathBuf, String> {
    let name = file.file_name().map(|n| n.to_string_lossy().to_string()).ok_or("No file")?;
    let (stem, ext) = downloads::split_ext(&name);
    let dir = downloads::unique(file.parent().unwrap_or(Path::new("/tmp")), &stem, &HashSet::new());
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let (f, d) = (file.to_string_lossy().to_string(), dir.to_string_lossy().to_string());
    let long = std::time::Duration::from_secs(1800);
    let low = ext.to_ascii_lowercase();
    let r = if crate::system::has("bsdtar") { crate::system::run_timeout("bsdtar", &["-xf", &f, "-C", &d], long) }
        else if low == ".zip" && crate::system::has("unzip") { crate::system::run_timeout("unzip", &["-q", "-o", &f, "-d", &d], long) }
        else if low == ".zip" || low == ".7z" { crate::system::run_timeout("python3", &["-I", "-m", "zipfile", "-e", &f, &d], long) }
        else { crate::system::run_timeout("tar", &["-xf", &f, "-C", &d], long) };
    if let Err(e) = r { let _ = std::fs::remove_dir_all(&dir); return Err(format!("Couldn't unpack it: {e}")); }
    Ok(dir)
}

pub fn bridge_call(br: &mut Bridge, method: &str, a: &Value) -> Result<Value, String> {
    let cmd = method.trim_start_matches("play.");
    let s = |k: &str| a[k].as_str().unwrap_or("").to_string();
    let now = crate::library::now_ms();
    match cmd {
        // Settings › Games › Integrations
        "keys.get" => Ok(keys::status(&keys::load())),
        "keys.set" => keys::set_in(&keys::path(), &s("service"), &s("field"), &s("value")),
        "keys.on" => keys::set_on_in(&keys::path(), &s("service"), a["on"] == true),
        "keys.clear" => keys::clear_in(&keys::path(), &s("service")),
        // A game's page
        "details" => { let (id, title, source) = game(br, a)?; br.later(move || Ok(details::for_game(&id, &title, &source))) }
        "achievements" => { let (id, title, source) = game(br, a)?; br.later(move || Ok(achievements::for_game(&id, &title, &source))) }
        // Artwork: search SteamGridDB, fetch a picture into art/custom, then use it for one of the game's kinds.
        "art.search" => {
            let (_, title, _) = game(br, a)?;
            let q = a["query"].as_str().map(str::trim).filter(|q| !q.is_empty()).map(str::to_string).unwrap_or(title);
            br.later(move || artwork::search(&q))
        }
        "art.fetch" => { let url = s("url"); br.later(move || artwork::save(&url).map(|p| json!({"file": p.to_string_lossy()}))) }
        "art.use" => {
            let (id, _, _) = game(br, a)?;
            let file = PathBuf::from(s("file"));
            let ok = std::fs::canonicalize(&file).ok().zip(std::fs::canonicalize(artwork::folder()).ok()).map(|(f, d)| f.starts_with(d)).unwrap_or(false);
            if !ok { return Err("That picture isn't one Hearth downloaded.".into()); }
            let f = file.to_string_lossy().to_string();
            artwork::set(&mut br.library.store, &id, &s("kind"), Some(f.as_str()))?;
            br.library.games.get(&id).map(|g| br.library.dto(g)).ok_or("Gone".into())
        }
        "art.clear" => {
            let (id, _, _) = game(br, a)?;
            match a["kind"].as_str().filter(|k| !k.is_empty()) { Some(k) => artwork::set(&mut br.library.store, &id, k, None)?, None => artwork::clear(&mut br.library.store, &id)? }
            br.library.games.get(&id).map(|g| br.library.dto(g)).ok_or("Gone".into())
        }
        // Discover
        "discover" => { let games = dtos(br); br.later(move || Ok(discover::build(&games, now))) }
        // Your profile, and your friends' (Hearth Friends)
        "activity.mine" => Ok(activity::mine(&dtos(br), br.library.running_id().as_deref(), now)),
        "activity.publish" => {
            let sh = activity::shared(&activity::mine(&dtos(br), br.library.running_id().as_deref(), now));
            br.later(move || crate::matrix_friends::call("setActivity", &json!({"activity": sh})))
        }
        "activity.friends" => br.later(|| crate::matrix_friends::call("activity", &json!({}))),
        "activity.share" => { let on = a["on"] == true; br.later(move || crate::matrix_friends::call("settings", &json!({"shareActivity": on}))) }
        // The download manager
        "dl.list" => Ok(downloads::with(|m| { m.tick(); m.view() })),
        "dl.add" => { let url = s("url"); let name = a["name"].as_str().map(str::to_string); downloads::with(|m| m.add(&url, name.as_deref(), "url", None)) }
        "dl.pause" => downloads::with(|m| m.pause(&s("id")).map(|_| m.view())),
        "dl.resume" => downloads::with(|m| m.resume(&s("id")).map(|_| m.view())),
        "dl.cancel" => downloads::with(|m| m.cancel(&s("id")).map(|_| m.view())),
        "dl.first" => downloads::with(|m| m.first(&s("id")).map(|_| m.view())),
        "dl.clear" => Ok(downloads::with(|m| { m.clear_done(); m.view() })),
        "dl.settings" => downloads::with(|m| m.set_settings(a["limitKBps"].as_u64(), a["parallel"].as_u64(), a["folder"].as_str()).map(|_| m.view())),
        "dl.addToLibrary" => {
            let j = downloads::with(|m| m.get(&s("id"))).ok_or("That download isn't in the list.")?;
            if j["state"] != "done" { return Err("It hasn't finished downloading.".into()); }
            let title = a["title"].as_str().map(str::to_string);
            add_file(br, j["file"].as_str().unwrap_or(""), title.as_deref())
        }
        "dl.unpack" => {
            let j = downloads::with(|m| m.get(&s("id"))).ok_or("That download isn't in the list.")?;
            if j["state"] != "done" { return Err("It hasn't finished downloading.".into()); }
            let (file, known) = (PathBuf::from(j["file"].as_str().unwrap_or("")), manual_paths(br));
            br.later(move || { let dir = unpack(&file)?; Ok(json!({"folder": dir.to_string_lossy(), "found": finder::scan(&[dir], &known)})) })
        }
        // Free and open-source games, and itch.io
        "foss.list" => Ok(sources::foss_list()),
        "foss.get" => { let id = s("id"); br.later(move || { let r = sources::foss_resolve(&id)?; downloads::with(|m| m.add(r["url"].as_str().unwrap_or(""), r["name"].as_str(), "foss", r["size"].as_u64().filter(|n| *n > 0))) }) }
        "itch.library" => br.later(sources::itch_library),
        "itch.search" => { let q = s("q"); br.later(move || sources::itch_search(&q)) }
        "itch.uploads" => { let (g, k) = (a["game"].as_u64().ok_or("Missing 'game'")?, a["keyId"].as_u64()); br.later(move || sources::itch_uploads(g, k)) }
        "itch.get" => {
            let (u, k, name, size) = (a["upload"].as_u64().ok_or("Missing 'upload'")?, a["keyId"].as_u64(), s("name"), a["size"].as_u64());
            br.later(move || { let url = sources::itch_download_url(u, k)?; downloads::with(|m| m.add(&url, Some(&name), "itch", size.filter(|n| *n > 0))) })
        }
        // Library › Find games in a folder
        "finder.scan" => {
            let known = manual_paths(br);
            let dirs = match a["path"].as_str().filter(|p| !p.is_empty()) { Some(p) => vec![PathBuf::from(p)], None => finder::default_dirs(&crate::paths::home(), &downloads::with(|m| m.folder())) };
            br.later(move || Ok(json!({"dirs": dirs.iter().map(|d| d.to_string_lossy().to_string()).collect::<Vec<_>>(), "found": finder::scan(&dirs, &known)})))
        }
        "finder.pick" => br.pick_later(method, a, crate::picker::Ask::Folder("Choose a folder to look for games in".into(), crate::paths::home().to_string_lossy().to_string())),
        "finder.add" => {
            let mut added = vec![];
            let mut errors = vec![];
            for p in a["paths"].as_array().cloned().unwrap_or_default().iter().filter_map(|p| p.as_str()).take(100) {
                match add_file(br, p, None) { Ok(d) => added.push(d), Err(e) => errors.push(e) }
            }
            Ok(json!({"added": added, "errors": errors}))
        }
        // Theme files
        "themes.list" => Ok(themes::list()),
        _ => Err(format!("{method} isn't a Hearth call")),
    }
}

/// The rest of a call that opened a folder picker (Find games in a folder).
pub fn picked(br: &mut Bridge, method: &str, _a: &Value, path: Option<String>) -> Result<Value, String> {
    match (method, path.filter(|p| !p.is_empty())) {
        ("play.finder.pick", Some(p)) => { let dir = PathBuf::from(&p); Ok(json!({"dirs": [p], "found": finder::scan(&[dir], &manual_paths(br))})) }
        ("play.finder.pick", None) => Ok(Value::Null),
        _ => Err(format!("Nothing waits for a picker in {method}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_into_its_own_folder() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src/Game");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("start.sh"), b"#!/bin/sh\n").unwrap();
        let tar = d.path().join("Free Game.tar.gz");
        crate::system::run("tar", &["-czf", &tar.to_string_lossy(), "-C", &d.path().join("src").to_string_lossy(), "Game"]).unwrap();
        let out = unpack(&tar).unwrap();
        assert_eq!(out, d.path().join("Free Game"));
        assert!(out.join("Game/start.sh").is_file());
        let again = unpack(&tar).unwrap();
        assert_eq!(again, d.path().join("Free Game (2)"), "never over an earlier one");
        std::fs::write(d.path().join("broken.zip"), b"not a zip").unwrap();
        assert!(unpack(&d.path().join("broken.zip")).is_err());
        assert!(!d.path().join("broken").exists(), "a failed unpack leaves nothing behind");
    }
}

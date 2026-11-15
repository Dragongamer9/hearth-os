//! A game's cloud saves and DLC, for its page in Hearth Games (stage two, 1.13; docs/HEARTH-GAMES.md).
//!
//! Cloud saves: GOG's and Epic's own (gogdl save-sync, legendary sync-saves; stores.rs syncs them by itself before you
//! play and after). Here: whether the game has them, the game's switch (its game options' "cloudSaves", on unless you
//! turn it off), the last sync (each sync notes it: note_sync) and the folder, and "Bring saves down / Send saves up
//! now".
//!
//! DLC: what your account has for the game, which is installed, installing it and (Epic) uninstalling it. Epic:
//! legendary's info (owned_dlc) and its installed records (is_dlc); a DLC installs into its game with
//! `legendary install <dlc>`. GOG: gogdl's info (dlcs) and each DLC's goggame-<id>.info in the game's folder; installs
//! with `gogdl download <game> --with-dlcs --dlcs <ids>` into the same folder (GOG DLC can't be taken out on its own:
//! uninstalling the game removes it). One DLC job at a time; the page asks again to see how it's going.

use crate::stores::{epic, gog};
use crate::tools;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

fn data() -> PathBuf { crate::paths::data_dir() }
fn read_json(p: &Path) -> Value { std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null) }
fn secs(n: u64) -> Option<std::time::Duration> { Some(std::time::Duration::from_secs(n)) }
fn sync_file() -> PathBuf { data().join("cloud-saves.json") }

/// Each sync, noted for the page ("Last synced 5 minutes ago · sent up after you played").
pub fn note_sync(id: &str, direction: &str, ok: bool) {
    let mut v = read_json(&sync_file());
    if !v.is_object() { v = json!({}); }
    v[id] = json!({"at": crate::library::now_ms(), "dir": direction, "ok": ok});
    let _ = crate::paths::write_atomic(&sync_file(), v.to_string().as_bytes());
}

/// {supported, on, last, folder}.
pub fn saves(g: &crate::library::Game, on: bool) -> Value {
    let last = read_json(&sync_file())[&g.id].clone();
    let last = if last.is_object() { last } else { Value::Null };
    let (supported, folder) = match g.source {
        "gog" => {
            let id = g.id.trim_start_matches("gog:");
            match g.install_path.as_ref().map(|p| gog::save_locations(id, p)) {
                Some(Ok(l)) if !l.is_empty() => (true, Some(l[0].1.to_string_lossy().to_string())),
                _ => (false, None),
            }
        }
        "epic" => {
            let app = g.id.trim_start_matches("epic:");
            let meta = read_json(&epic::config().join("metadata").join(format!("{app}.json")));
            match meta["metadata"]["customAttributes"]["CloudSaveFolder"]["value"].as_str() {
                Some(tpl) => {
                    let inst = read_json(&epic::config().join("installed.json"))[app].clone();
                    let acct = read_json(&epic::config().join("user.json"))["account_id"].as_str().map(str::to_string);
                    (true, crate::stores::save_path(tpl, &crate::stores::prefix_of("epic", app), inst["install_path"].as_str(), acct.as_deref()).map(|p| p.to_string_lossy().to_string()))
                }
                None => (false, None),
            }
        }
        _ => (false, None),
    };
    json!({"supported": supported, "on": on, "last": last, "folder": folder})
}

/// "Bring saves down now" / "Send saves up now".
pub fn sync_now(g: &crate::library::Game, direction: &str) -> Result<Value, String> {
    if direction != "up" && direction != "down" { return Err("up or down".into()); }
    match g.source {
        "gog" => {
            let dir = g.install_path.clone().ok_or("The game isn't installed.")?;
            let n = gog::sync_saves(g.id.trim_start_matches("gog:"), &dir, direction)?;
            if n == 0 { return Err("GOG didn't sync anything (the game has no cloud saves, or GOG didn't answer).".into()); }
            Ok(json!({"ok": true, "places": n}))
        }
        "epic" => {
            let ok = epic::sync_saves(g.id.trim_start_matches("epic:"), direction)?;
            if !ok { return Err("Epic didn't sync the saves (the game has no cloud saves, or Epic didn't answer).".into()); }
            Ok(json!({"ok": true}))
        }
        _ => Err("This store doesn't keep saves in the cloud.".into()),
    }
}

// ------------------------------------------------------------------ DLC

/// DLC jobs running now: game id → what's happening ("Installing", "Removing").
static BUSY: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
fn busy_set(dlc: &str, what: Option<&str>) {
    let mut b = BUSY.lock().unwrap_or_else(|p| p.into_inner());
    let m = b.get_or_insert_with(HashMap::new);
    match what { Some(w) => { m.insert(dlc.to_string(), w.to_string()); } None => { m.remove(dlc); } }
}
fn busy() -> HashMap<String, String> { BUSY.lock().unwrap_or_else(|p| p.into_inner()).clone().unwrap_or_default() }
fn last_error() -> PathBuf { data().join("dlc-error.json") }

/// Epic: the game's DLC on your account (legendary info's owned_dlc) and which are installed.
pub fn epic_dlc_from(info: &Value, installed: &Value) -> Vec<Value> {
    let owned = info["game"]["owned_dlc"].as_array().cloned().unwrap_or_default();
    owned.iter().filter_map(|d| {
        let app = d["app_name"].as_str()?;
        let inst = &installed[app];
        Some(json!({"id": format!("epic:{app}"), "title": d["title"].as_str().unwrap_or(app), "installed": inst.is_object() && inst["is_dlc"] == true,
            "sizeBytes": inst["install_size"].as_u64(), "canUninstall": true}))
    }).collect()
}

/// GOG: the game's DLC (gogdl info's dlcs) and which are in the game's folder (goggame-<id>.info).
pub fn gog_dlc_from(info: &Value, dir: Option<&Path>) -> Vec<Value> {
    info["dlcs"].as_array().cloned().unwrap_or_default().iter().filter_map(|d| {
        let id = d["id"].as_str().map(str::to_string).or_else(|| d["id"].as_u64().map(|n| n.to_string()))
            .or_else(|| d["app_name"].as_str().map(str::to_string))?;
        let installed = dir.map(|p| p.join(format!("goggame-{id}.info")).exists()).unwrap_or(false);
        let size = d["size"]["*"]["download_size"].as_u64().or(d["size"].as_u64()).or(d["perLangSize"]["*"]["download_size"].as_u64());
        Some(json!({"id": format!("gog:{id}"), "title": d["title"].as_str().unwrap_or(&id), "installed": installed, "sizeBytes": size, "canUninstall": false}))
    }).collect()
}

pub fn dlc(g: &crate::library::Game) -> Result<Value, String> {
    let list = match g.source {
        "epic" => {
            let app = g.id.trim_start_matches("epic:");
            let r = tools::run(&tools::LEGENDARY, &["info", app, "--json"], &epic::env(), None, None, secs(120))?;
            let info = crate::owned::json_in(&r.out, '{').ok_or_else(|| format!("Epic didn't say: {}", tools::problem(&r)))?;
            epic_dlc_from(&info, &read_json(&epic::config().join("installed.json")))
        }
        "gog" => {
            let id = g.id.trim_start_matches("gog:");
            gog::token()?;
            let env = vec![("GOGDL_CONFIG_PATH", gog::dir().to_string_lossy().to_string())];
            let r = tools::run(&tools::GOGDL, &["--auth-config-path", &gog::auth().to_string_lossy(), "info", id, "--platform", "windows", "--lang", "en-US"], &env, None, None, secs(120))?;
            let info = crate::owned::json_in(&r.out, '{').ok_or_else(|| format!("GOG didn't say: {}", tools::problem(&r)))?;
            gog_dlc_from(&info, g.install_path.as_deref())
        }
        _ => vec![],
    };
    let b = busy();
    let err = read_json(&last_error());
    Ok(json!({"dlc": list, "busy": b, "error": if err[&g.id].is_string() && b.is_empty() { err[&g.id].clone() } else { Value::Null }}))
}

/// Installs the game's DLC (`want` empty: every one you have that isn't installed), in the background.
pub fn install_dlc(g: &crate::library::Game, want: Vec<String>) -> Result<Value, String> {
    if !busy().is_empty() { return Err("A DLC is being installed already: one at a time.".into()); }
    let all = dlc(g)?;
    let todo: Vec<String> = all["dlc"].as_array().cloned().unwrap_or_default().iter()
        .filter(|d| d["installed"] != true).filter_map(|d| d["id"].as_str().map(str::to_string))
        .filter(|id| want.is_empty() || want.contains(id)).collect();
    if todo.is_empty() { return Err("Nothing to install: it's installed already.".into()); }
    for d in &todo { busy_set(d, Some("Installing")); }
    let (game, source, dir) = (g.id.clone(), g.source, g.install_path.clone());
    std::thread::spawn(move || {
        let res: Result<(), String> = (|| match source {
            "epic" => {
                for d in &todo {
                    let app = d.trim_start_matches("epic:");
                    let r = tools::run(&tools::LEGENDARY, &["-y", "install", app, "--skip-sdl"], &epic::env(), None, None, None)?;
                    if r.code != 0 { return Err(format!("Epic install of the DLC failed: {}", tools::problem(&r))); }
                    busy_set(d, None);
                }
                Ok(())
            }
            "gog" => {
                let dir = dir.ok_or("The game isn't installed.")?;
                let base = dir.parent().ok_or("The game's folder has no parent")?;
                let ids: Vec<&str> = todo.iter().map(|d| d.trim_start_matches("gog:")).collect();
                gog::token()?;
                let env = vec![("GOGDL_CONFIG_PATH", gog::dir().to_string_lossy().to_string())];
                let r = tools::run(&tools::GOGDL, &["--auth-config-path", &gog::auth().to_string_lossy(), "download", game.trim_start_matches("gog:"), "--platform", "windows",
                    "--path", &base.to_string_lossy(), "--with-dlcs", "--dlcs", &ids.join(","), "--lang", "en-US"], &env, None, None, None)?;
                if r.code != 0 { return Err(format!("GOG install of the DLC failed: {}", tools::problem(&r))); }
                Ok(())
            }
            _ => Err("This store's DLC isn't installed by Hearth.".into()),
        })();
        let mut errs = read_json(&last_error());
        if !errs.is_object() { errs = json!({}); }
        match res {
            Ok(()) => { errs.as_object_mut().map(|o| o.remove(&game)); crate::bridge::log(&format!("dlc: {game}: installed {}", todo.join(", "))); }
            Err(e) => { errs[&game] = json!(e.clone()); crate::bridge::log(&format!("dlc: {game}: {e}")); }
        }
        let _ = crate::paths::write_atomic(&last_error(), errs.to_string().as_bytes());
        for d in &todo { busy_set(d, None); }
        crate::stores::notify(json!({"type": "internal", "what": "rescan", "reply": null}));
    });
    Ok(json!({"ok": true, "installing": true}))
}

/// Epic only: takes one DLC out of its game.
pub fn uninstall_dlc(g: &crate::library::Game, dlc: &str) -> Result<Value, String> {
    if g.source != "epic" { return Err("GOG's DLC goes with its game: uninstall the game to remove it.".into()); }
    let app = dlc.strip_prefix("epic:").ok_or("That isn't an Epic DLC.")?;
    let inst = read_json(&epic::config().join("installed.json"));
    if inst[app]["is_dlc"] != true { return Err("That DLC isn't installed.".into()); }
    let r = tools::run(&tools::LEGENDARY, &["-y", "uninstall", app], &epic::env(), None, None, secs(600))?;
    if read_json(&epic::config().join("installed.json"))[app].is_object() { return Err(format!("Couldn't remove the DLC: {}", tools::problem(&r))); }
    Ok(json!({"ok": true}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn epic_dlc() {
        let info = json!({"game": {"owned_dlc": [{"app_name": "a1", "title": "Hearts"}, {"app_name": "a2", "title": "Blood"}]}});
        let inst = json!({"a1": {"is_dlc": true, "install_size": 4200}, "base": {"is_dlc": false}});
        let l = epic_dlc_from(&info, &inst);
        assert_eq!(l.len(), 2);
        assert_eq!(l[0]["id"], "epic:a1");
        assert_eq!(l[0]["installed"], true);
        assert_eq!(l[0]["sizeBytes"], 4200);
        assert_eq!(l[1]["installed"], false);
    }
    #[test]
    fn gog_dlc() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("goggame-11.info"), "{}").unwrap();
        let info = json!({"dlcs": [{"id": "11", "title": "Hearts"}, {"id": 12, "title": "Blood", "size": {"*": {"download_size": 900}}}]});
        let l = gog_dlc_from(&info, Some(dir.path()));
        assert_eq!(l[0]["installed"], true);
        assert_eq!(l[1]["id"], "gog:12");
        assert_eq!(l[1]["installed"], false);
        assert_eq!(l[1]["sizeBytes"], 900);
        assert_eq!(l[1]["canUninstall"], false);
    }
}

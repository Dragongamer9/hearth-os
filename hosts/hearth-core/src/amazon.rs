//! Amazon Games (Prime Gaming's free games you claimed, and games bought on Amazon Games) in Hearth Games, 1.13
//! (docs/HEARTH-GAMES.md, stage two): through nile (github.com/imLinguin/nile, GPL-3.0, the tool Heroic uses), pinned
//! in tools.rs with its checksum and carried in the image like gogdl and legendary.
//!
//! Signing in is on Amazon's own page: `nile auth --login --non-interactive` makes the page's address (and a one-time
//! code verifier, kept in nile's own folder until the sign-in finishes); the page sends you back to amazon.com with
//! `openid.oa2.authorization_code` in the address, which Hearth catches and hands to `nile register`. Hearth never sees
//! your password. Games install with `nile install --base-path <drive>/Amazon`, start with Proton (nile's own launch
//! instructions, `nile launch --json`: the game's program, its arguments and Amazon's SDK variables) and uninstall with
//! `nile uninstall`. Amazon games have no cloud saves or DLC through nile.

use crate::owned::{json_in, Owned};
use crate::tools::{self, Output};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub const CACHE: &str = "amazon-owned.json";

/// nile's folder: Hearth's own (NILE_CONFIG_PATH), next to legendary's and gogdl's; nile adds "nile".
pub fn base() -> PathBuf { crate::paths::data_dir() }
pub fn config() -> PathBuf { base().join("nile") }
pub fn env() -> Vec<(&'static str, String)> { vec![("NILE_CONFIG_PATH", base().to_string_lossy().into())] }
fn read_json(p: &Path) -> Value { std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null) }
fn secs(n: u64) -> Option<std::time::Duration> { Some(std::time::Duration::from_secs(n)) }

pub fn signed_in() -> bool { read_json(&config().join("current_user.json"))["user_id"].is_string() }
pub fn display_name() -> Option<String> { read_json(&config().join("current_user.json"))["name"].as_str().filter(|s| !s.is_empty()).map(str::to_string) }

fn pending() -> PathBuf { config().join("hearth-login.json") }

/// The page's address: nile makes it (a code verifier and a device serial with it, kept until the sign-in finishes).
pub fn sign_in() -> crate::stores::SignIn {
    let url = (|| -> Option<String> {
        let r = tools::run(&tools::NILE, &["auth", "--login", "--non-interactive"], &env(), None, None, secs(60)).ok()?;
        let v = json_in(&r.out, '{')?;
        let url = v["url"].as_str()?.to_string();
        std::fs::create_dir_all(config()).ok()?;
        crate::paths::write_atomic(&pending(), v.to_string().as_bytes()).ok()?;
        Some(url)
    })().unwrap_or_else(|| "https://gaming.amazon.com/".to_string());
    crate::stores::SignIn { store: "amazon", title: "Sign in to Amazon", url,
        done: |u| if u.contains("openid.oa2.authorization_code=") { crate::http::query_param(u, "openid.oa2.authorization_code") } else { None },
        script: None }
}

pub fn finish_sign_in(code: &str) -> Result<(), String> {
    let p = read_json(&pending());
    let (Some(cid), Some(ver), Some(serial)) = (p["client_id"].as_str(), p["code_verifier"].as_str(), p["serial"].as_str()) else {
        return Err("The Amazon sign-in didn't start properly: try Sign in again.".into());
    };
    let r = tools::run(&tools::NILE, &["register", "--code", code, "--client-id", cid, "--code-verifier", ver, "--serial", serial], &env(), None, None, secs(120))?;
    let _ = std::fs::remove_file(pending());
    if !signed_in() { return Err(format!("Amazon sign-in didn't finish: {}", tools::problem(&r))); }
    Ok(())
}

pub fn sign_out() {
    if tools::is_ready(&tools::NILE) { let _ = tools::run(&tools::NILE, &["auth", "--logout"], &env(), None, None, secs(60)); }
    let _ = std::fs::remove_file(config().join("current_user.json"));
    Owned::remove(CACHE);
}

/// A nile library entry's art: the product's own pictures.
fn art_of(p: &Value) -> serde_json::Map<String, Value> {
    let mut art = serde_json::Map::new();
    let d = &p["productDetail"]["details"];
    if let Some(u) = d["logoUrl"].as_str() { art.insert("logo".into(), json!(u)); }
    if let Some(u) = d["backgroundUrl1"].as_str().or(d["backgroundUrl2"].as_str()) { art.insert("hero".into(), json!(u)); art.insert("wide".into(), json!(u)); }
    if let Some(u) = p["productDetail"]["iconUrl"].as_str() { art.insert("cover".into(), json!(u)); }
    for s in d["screenshots"].as_array().cloned().unwrap_or_default().iter().take(1) { if let Some(u) = s.as_str() { art.entry("wide").or_insert(json!(u)); } }
    art
}

/// Your Amazon games (nile library sync, then its list).
pub fn refresh_owned() -> Result<Owned, String> {
    let r = tools::run(&tools::NILE, &["library", "sync"], &env(), None, None, secs(300))?;
    if r.code != 0 && !config().join("library.json").exists() { return Err(format!("Couldn't get your Amazon games: {}", tools::problem(&r))); }
    let lib = read_json(&config().join("library.json"));
    let mut o = Owned { fetched: crate::library::now_ms(), games: vec![] };
    for g in lib.as_array().cloned().unwrap_or_default() {
        let (Some(id), Some(title)) = (g["id"].as_str(), g["product"]["title"].as_str()) else { continue };
        o.games.push(json!({"id": id, "title": title.trim(), "art": art_of(&g["product"]), "added": 0, "lastPlayed": 0, "note": null}));
    }
    o.save(CACHE);
    Ok(o)
}

/// Amazon games installed through nile (its installed.json: id, path, version, size).
pub fn installed(owned: &Owned) -> Vec<crate::library::Game> {
    let mut out = Vec::new();
    for e in read_json(&config().join("installed.json")).as_array().cloned().unwrap_or_default() {
        let (Some(id), Some(path)) = (e["id"].as_str(), e["path"].as_str()) else { continue };
        if !Path::new(path).is_dir() { continue; }
        let o = owned.find(id);
        let title = o.and_then(|o| o["title"].as_str()).unwrap_or(id).to_string();
        let art = o.map(|o| crate::stores::art_of(&o["art"])).unwrap_or_default();
        out.push(crate::stores::game(format!("amazon:{id}"), &title, "amazon", path, e["size"].as_u64().filter(|s| *s > 0), art, true));
    }
    out
}

pub fn install(id: &str, root: &Path, job: &Arc<Mutex<crate::stores::Job>>, cancel: Arc<AtomicBool>) -> Result<(), String> {
    let base = root.join("Amazon");
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let j = job.clone();
    let r: Output = tools::run(&tools::NILE, &["install", id, "--base-path", &base.to_string_lossy()], &env(),
        Some(Arc::new(move |l: &str| j.lock().unwrap().read_line(l, false))), Some(cancel), None)?;
    if !installed(&Owned::default()).iter().any(|g| g.id == format!("amazon:{id}")) {
        return Err(format!("Amazon install failed: {}", tools::problem(&r)));
    }
    Ok(())
}

pub fn uninstall(id: &str) -> Result<(), String> {
    let r = tools::run(&tools::NILE, &["uninstall", id], &env(), None, None, secs(600))?;
    if installed(&Owned::default()).iter().any(|g| g.id == format!("amazon:{id}")) { return Err(format!("Couldn't uninstall: {}", tools::problem(&r))); }
    crate::stores::remove_prefix("amazon", id);
    Ok(())
}

/// nile's launch instructions (the game's own fuel.json, read by nile) and Proton, like Epic and GOG.
pub fn launch(id: &str, opts: &Value, game_id: u32) -> Result<u32, String> {
    let r = tools::run(&tools::NILE, &["launch", id, "--json"], &env(), None, None, secs(60))?;
    let p = json_in(&r.out, '{').ok_or_else(|| format!("Couldn't start the game: {}", tools::problem(&r)))?;
    let exe = p["command"]["instruction"].as_str().unwrap_or("").to_string();
    if !Path::new(&exe).is_file() { return Err(format!("The game's program is missing ({exe}). Try reinstalling it.")); }
    let args: Vec<String> = p["command"]["arguments"].as_array().cloned().unwrap_or_default().iter().filter_map(|a| a.as_str().map(str::to_string)).collect();
    let env: Vec<(String, String)> = p["env"].as_object().map(|o| o.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect()).unwrap_or_default();
    let dir = p["game_directory"].as_str().unwrap_or("/").to_string();
    let cwd = Path::new(&exe).parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from(&dir));
    crate::stores::proton_launch("amazon", id, &exe, args, &cwd, env, opts, game_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sign_in_code_from_the_address() {
        let f = |u: &str| if u.contains("openid.oa2.authorization_code=") { crate::http::query_param(u, "openid.oa2.authorization_code") } else { None };
        assert_eq!(f("https://www.amazon.com/?openid.assoc_handle=amzn_sonic_games_launcher&openid.oa2.authorization_code=ANabc123&x=1").as_deref(), Some("ANabc123"));
        assert_eq!(f("https://www.amazon.com/ap/signin?x=1"), None);
    }
    #[test]
    fn art_from_nile_entry() {
        let p = json!({"productDetail": {"iconUrl": "https://a/icon.png", "details": {"logoUrl": "https://a/logo.png", "backgroundUrl1": "https://a/bg.jpg"}}});
        let a = art_of(&p);
        assert_eq!(a["cover"], "https://a/icon.png");
        assert_eq!(a["hero"], "https://a/bg.jpg");
        assert_eq!(a["logo"], "https://a/logo.png");
    }
}

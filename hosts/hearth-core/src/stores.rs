//! The store accounts in Library › Stores (Epic, GOG, Xbox; Steam signs in through Steam itself), their game
//! lists, Hearth's own Epic and GOG installs (download queue), starting their games with Proton, and Xbox Cloud
//! Gaming's catalog. Port of hosts/linux/src/stores/*.js and library/xcloud.js; same files on disk, so the
//! Electron host and this one share sign-ins and installs.

use crate::http;
use crate::owned::{https, json_in, ms, Owned};
use crate::tools::{self, Output};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn data() -> PathBuf { crate::paths::data_dir() }
fn read_json(p: &Path) -> Value { std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null) }

/// A sign-in page: where it starts, and how to tell it's done (a code in the address, or for Epic, in the page).
pub struct SignIn { pub title: &'static str, pub url: String, pub done: fn(&str) -> Option<String>, pub script: Option<&'static str> }

// ------------------------------------------------------------------ Proton (umu)

pub fn prefix_of(store: &str, id: &str) -> PathBuf {
    let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect();
    data().join("prefixes").join(format!("{store}-{safe}"))
}

/// Starts a Windows game with Proton through umu-run, in its own Wine prefix. Returns the process id.
/// `game_id` (appid.rs) goes in as HEARTH_GAME_ID: umu passes it on to Proton and Wine untouched (it rewrites
/// SteamGameId and SteamAppId from GAMEID, which is why those aren't used), so every process of the game carries it.
pub fn proton_launch(store: &str, id: &str, exe: &str, args: Vec<String>, cwd: &Path, env: Vec<(String, String)>, opts: &Value, game_id: u32) -> Result<u32, String> {
    let umu = tools::ensure(&tools::UMU)?;
    let prefix = prefix_of(store, id);
    std::fs::create_dir_all(&prefix).map_err(|e| e.to_string())?;
    let mut a = vec![exe.to_string()]; a.extend(args);
    let (cmd, argv, mut e) = crate::manual::apply(opts, &umu, a, true);
    e.extend(env);
    e.push(("WINEPREFIX".into(), prefix.to_string_lossy().into()));
    e.push(("GAMEID".into(), std::env::var("NIMBUS_GAMEID").unwrap_or_else(|_| "0".into())));
    e.push(("STORE".into(), match store { "epic" => "egs", "gog" => "gog", _ => "none" }.into()));
    e.push(("PROTON_VERB".into(), "waitforexitandrun".into()));
    if game_id != 0 { e.push(crate::appid::env(game_id)); }
    crate::bridge::log(&format!("proton: {store}:{id} {exe} (HEARTH_GAME_ID={game_id})"));
    std::process::Command::new(cmd).args(argv).envs(e).current_dir(cwd)
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(crate::procs::detach).map_err(|err| format!("Couldn't start the game: {err}"))
}
pub fn remove_prefix(store: &str, id: &str) { let p = prefix_of(store, id); if p.starts_with(data().join("prefixes")) { let _ = std::fs::remove_dir_all(p); } }

// ------------------------------------------------------------------ Epic (legendary)

pub mod epic {
    use super::*;
    const CLIENT_ID: &str = "34a02cf8f4414e29b15921876da36f9a";
    pub fn config() -> PathBuf { data().join("legendary") }
    pub fn env() -> Vec<(&'static str, String)> { vec![("LEGENDARY_CONFIG_PATH", config().to_string_lossy().into())] }
    pub const CACHE: &str = "epic-owned.json";

    pub fn signed_in() -> bool { config().join("user.json").exists() }
    pub fn display_name() -> Option<String> { read_json(&config().join("user.json"))["displayName"].as_str().map(str::to_string) }
    /// Your Epic friends (Epic's friends service, signed in with the same account legendary uses). Epic doesn't
    /// share online status outside its own apps, so they show without one.
    pub fn friends() -> Result<Vec<Value>, String> {
        let call = |u: &Value| -> Result<http::Reply, String> {
            let acct = u["account_id"].as_str().ok_or("Sign in to Epic first.")?;
            let tok = u["access_token"].as_str().unwrap_or("");
            http::get(&format!("https://friends-public-service-prod.ol.epicgames.com/friends/api/v1/{acct}/summary"), &[("Authorization", &format!("bearer {tok}"))])
        };
        if super::expiry::is_expired("epic") { return Err(super::expiry::message("epic")); }
        let mut u = read_json(&config().join("user.json"));
        let mut r = call(&u)?;
        if r.status == 401 {
            // An old token: legendary renews it on any online command.
            let b = before();
            if let Ok(o) = tools::run(&tools::LEGENDARY, &["status"], &env(), None, None, Some(std::time::Duration::from_secs(60))) {
                if check(&o, &b) { return Err(super::expiry::message("epic")); }
            }
            u = read_json(&config().join("user.json"));
            r = call(&u)?;
            // Still refused with a freshly renewed token: Epic doesn't accept this sign-in anymore.
            if r.status == 401 && b.0 { super::expiry::mark("epic", b.1); return Err(super::expiry::message("epic")); }
        }
        if !r.ok() { return Err(format!("Epic didn't share your friends ({})", r.status)); }
        let ids: Vec<String> = r.json()["friends"].as_array().cloned().unwrap_or_default().iter().filter_map(|f| f["accountId"].as_str().map(str::to_string)).collect();
        let mut names = std::collections::HashMap::new();
        for chunk in ids.chunks(90) {
            let q: Vec<String> = chunk.iter().map(|i| format!("accountId={i}")).collect();
            let tok = u["access_token"].as_str().unwrap_or("");
            if let Ok(a) = http::get_json(&format!("https://account-public-service-prod.ol.epicgames.com/account/api/public/account?{}", q.join("&")), &[("Authorization", &format!("bearer {tok}"))]) {
                for x in a.as_array().cloned().unwrap_or_default() {
                    if let (Some(id), Some(n)) = (x["id"].as_str(), x["displayName"].as_str()) { names.insert(id.to_string(), n.to_string()); }
                }
            }
        }
        Ok(ids.iter().map(|id| json!({"id": id, "platform": "epic", "name": names.get(id).cloned().unwrap_or_else(|| "Epic friend".into()),
            "state": "unknown", "canMessage": false, "canJoin": false})).collect())
    }

    /// Who is signed in before an online legendary command: (signed in, name). legendary deletes user.json when
    /// Epic refuses the saved sign-in, so the name is read first.
    fn before() -> (bool, Option<String>) { (signed_in(), display_name()) }
    /// After an online legendary command: marks the sign-in expired when Epic refused it (see expiry).
    fn check(r: &Output, (had, name): &(bool, Option<String>)) -> bool {
        let refused = *had && super::expiry::legendary_refused(&format!("{}\n{}", r.err, r.out), !signed_in());
        if refused { super::expiry::mark("epic", name.clone()); }
        refused
    }

    fn redirect() -> String { format!("https://www.epicgames.com/id/api/redirect?clientId={CLIENT_ID}&responseType=code") }
    /// The sign-in page. After signing in, Epic shows a small JSON page with a one-time code: the script hands
    /// it back (as a hearth-signin:// address the app catches); before that, it sends you back to the login.
    pub fn sign_in() -> SignIn {
        SignIn { title: "Sign in to Epic Games", url: format!("https://www.epicgames.com/id/login?redirectUrl={}", http::urlencode(&redirect())),
            done: |u| if u.starts_with("hearth-signin://") { Some(http::query_param(u, "code").unwrap_or_default()) } else { None },
            script: Some(r#"(() => { if (!location.href.toLowerCase().startsWith('https://www.epicgames.com/id/api/redirect')) return;
                const go = () => { let c = ''; try { c = JSON.parse(document.body.innerText).authorizationCode || ''; } catch (e) {}
                  location.href = c ? 'hearth-signin://done?code=' + encodeURIComponent(c) : 'https://www.epicgames.com/id/login'; };
                if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', go); else go(); })();"#) }
    }
    pub fn finish_sign_in(code: &str) -> Result<(), String> {
        let r = tools::run(&tools::LEGENDARY, &["auth", "--code", code], &env(), None, None, Some(std::time::Duration::from_secs(120)))?;
        if !signed_in() { return Err(format!("Epic sign-in didn't finish: {}", tools::problem(&r))); }
        Ok(())
    }
    pub fn sign_out() {
        if tools::is_ready(&tools::LEGENDARY) { let _ = tools::run(&tools::LEGENDARY, &["auth", "--delete"], &env(), None, None, Some(std::time::Duration::from_secs(60))); }
        let _ = std::fs::remove_file(config().join("user.json"));
        Owned::remove(CACHE);
    }
    /// Your Epic library, with art.
    pub fn refresh_owned() -> Result<Owned, String> {
        let b = before();
        let r = tools::run(&tools::LEGENDARY, &["list", "--json"], &env(), None, None, Some(std::time::Duration::from_secs(240)))?;
        if check(&r, &b) { return Err(super::expiry::message("epic")); }
        let list = json_in(&r.out, '[').filter(|_| r.code == 0).ok_or_else(|| format!("Couldn't get your Epic library: {}", tools::problem(&r)))?;
        let mut o = Owned { fetched: crate::library::now_ms(), games: vec![] };
        for g in list.as_array().cloned().unwrap_or_default() {
            let (Some(app), Some(title)) = (g["app_name"].as_str(), g["app_title"].as_str()) else { continue };
            let meta = &g["metadata"];
            let mut art = serde_json::Map::new();
            for img in meta["keyImages"].as_array().cloned().unwrap_or_default() {
                let Some(url) = img["url"].as_str() else { continue };
                match img["type"].as_str().unwrap_or("") {
                    "DieselGameBoxTall" => { art.insert("cover".into(), json!(url)); }
                    "DieselGameBox" => { art.insert("hero".into(), json!(url)); art.entry("wide").or_insert(json!(url)); }
                    "DieselGameBoxWide" => { art.insert("wide".into(), json!(url)); }
                    "DieselGameBoxLogo" => { art.insert("logo".into(), json!(url)); }
                    "Thumbnail" => { art.entry("cover").or_insert(json!(url)); }
                    _ => {}
                }
            }
            let custom = &meta["customAttributes"];
            let third = custom["ThirdPartyManagedApp"]["value"].as_str().or(custom["ThirdPartyManagedProvider"]["value"].as_str());
            o.games.push(json!({"id": app, "title": title.trim(), "art": art, "added": ms(meta["creationDate"].as_str().unwrap_or("")), "lastPlayed": 0,
                                "note": third.map(|t| format!("Installs through {t}"))}));
        }
        o.save(CACHE);
        Ok(o)
    }
    /// Games installed with legendary, from legendary's records.
    pub fn installed(owned: &Owned) -> Vec<crate::library::Game> {
        let recs = read_json(&config().join("installed.json"));
        let mut out = Vec::new();
        for (app, e) in recs.as_object().cloned().unwrap_or_default() {
            let (Some(path), Some(title)) = (e["install_path"].as_str(), e["title"].as_str()) else { continue };
            if e["is_dlc"] == true || !Path::new(path).is_dir() { continue; }
            let art = owned.find(&app).map(|o| super::art_of(&o["art"])).unwrap_or_default();
            out.push(super::game(format!("epic:{app}"), title, "epic", path, e["install_size"].as_u64().filter(|s| *s > 0), art, true));
        }
        out
    }
    pub fn install(app: &str, root: &Path, job: &Arc<Mutex<super::Job>>, cancel: Arc<AtomicBool>) -> Result<(), String> {
        let base = root.join("Epic");
        std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
        let j = job.clone();
        let b = before();
        let r = tools::run(&tools::LEGENDARY, &["-y", "install", app, "--base-path", &base.to_string_lossy(), "--skip-sdl", "--skip-dlcs", "--platform", "Windows"], &env(),
            Some(Arc::new(move |l: &str| j.lock().unwrap().read_line(l, false))), Some(cancel), None)?;
        if !installed(&Owned::default()).iter().any(|g| g.id == format!("epic:{app}")) {
            if check(&r, &b) { return Err(super::expiry::message("epic")); }
            return Err(format!("Epic install failed: {}", tools::problem(&r)));
        }
        Ok(())
    }
    /// Registers a game copied from another drive (Settings › Import games).
    pub fn import_game(app: &str, dir: &str) -> Result<(), String> {
        let r = tools::run(&tools::LEGENDARY, &["-y", "import", app, dir, "--platform", "Windows", "--disable-check"], &env(), None, None, Some(std::time::Duration::from_secs(600)))?;
        if !installed(&Owned::default()).iter().any(|g| g.id == format!("epic:{app}")) { return Err(format!("Epic didn't accept it: {}", tools::problem(&r))); }
        Ok(())
    }
    pub fn uninstall(app: &str) -> Result<(), String> {
        let r = tools::run(&tools::LEGENDARY, &["-y", "uninstall", app], &env(), None, None, Some(std::time::Duration::from_secs(600)))?;
        if installed(&Owned::default()).iter().any(|g| g.id == format!("epic:{app}")) { return Err(format!("Couldn't uninstall: {}", tools::problem(&r))); }
        super::remove_prefix("epic", app);
        Ok(())
    }
    /// Asks legendary how to start the game (with a fresh Epic token) and starts it with Proton.
    pub fn launch(app: &str, opts: &Value, game_id: u32) -> Result<u32, String> {
        let t = Some(std::time::Duration::from_secs(60));
        let b = before();
        let mut r: Output = tools::run(&tools::LEGENDARY, &["launch", app, "--json", "--skip-version-check"], &env(), None, None, t)?;
        let mut p = json_in(&r.out, '{');
        let mut expired = false;
        if p.is_none() {
            // An expired sign-in is noted (Library › Stores offers Sign in again); the game still tries to start.
            expired = check(&r, &b);
            // Offline, Epic is down, or the sign-in expired: games that allow it start without the online token.
            r = tools::run(&tools::LEGENDARY, &["launch", app, "--json", "--offline", "--skip-version-check"], &env(), None, None, t)?;
            p = json_in(&r.out, '{');
        }
        let p = p.ok_or_else(|| if expired || super::expiry::is_expired("epic") { format!("{} This game needs Epic online to start.", super::expiry::message("epic")) }
            else { format!("Couldn't start the game: {}", tools::problem(&r)) })?;
        if opts["cloudSaves"] != false && !super::expiry::is_expired("epic") { let _ = sync_saves(app, "down"); }
        let dir = p["game_directory"].as_str().unwrap_or("");
        let exe = Path::new(dir).join(p["game_executable"].as_str().unwrap_or(""));
        if !exe.is_file() { return Err(format!("The game's program is missing ({}). Try reinstalling it.", exe.display())); }
        let mut args = Vec::new();
        for k in ["game_parameters", "user_parameters", "egl_parameters"] { for a in p[k].as_array().cloned().unwrap_or_default() { if let Some(s) = a.as_str() { args.push(s.to_string()); } } }
        let env: Vec<(String, String)> = p["environment"].as_object().map(|o| o.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect()).unwrap_or_default();
        let cwd = p["working_directory"].as_str().filter(|s| !s.is_empty()).unwrap_or(dir);
        super::proton_launch("epic", app, &exe.to_string_lossy(), args, Path::new(cwd), env, opts, game_id)
    }
    /// Epic cloud saves: "down" before playing, "up" after (games with cloud saves).
    pub fn sync_saves(app: &str, direction: &str) -> Result<bool, String> {
        let meta = read_json(&config().join("metadata").join(format!("{app}.json")));
        let tpl = meta["metadata"]["customAttributes"]["CloudSaveFolder"]["value"].as_str().map(str::to_string);
        let Some(tpl) = tpl else { return Ok(false) };
        let inst = read_json(&config().join("installed.json"))[app].clone();
        let account = read_json(&config().join("user.json"))["account_id"].as_str().map(str::to_string);
        let Some(save) = super::save_path(&tpl, &prefix_of("epic", app), inst["install_path"].as_str(), account.as_deref()) else { return Ok(false) };
        std::fs::create_dir_all(&save).map_err(|e| e.to_string())?;
        let flag = if direction == "down" { "--skip-upload" } else { "--skip-download" };
        let b = before();
        let r = tools::run(&tools::LEGENDARY, &["-y", "sync-saves", app, "--save-path", &save.to_string_lossy(), flag], &env(), None, None, Some(std::time::Duration::from_secs(120)))?;
        if r.code != 0 && check(&r, &b) { return Err(super::expiry::message("epic")); }
        crate::bridge::log(&format!("epic saves: {app} {direction} {}", if r.code == 0 { "ok".into() } else { tools::problem(&r) }));
        Ok(r.code == 0)
    }
}

/// Fills Epic's save-folder placeholders ({AppData}/Game/Saved) for the game's own Wine prefix.
pub fn save_path(tpl: &str, prefix: &Path, install: Option<&str>, account: Option<&str>) -> Option<PathBuf> {
    let drive = [prefix.join("drive_c"), prefix.join("pfx/drive_c")].into_iter().find(|d| d.exists()).unwrap_or(prefix.join("drive_c"));
    let user = drive.join("users/steamuser");
    let mut out = String::new();
    let mut rest = tpl;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let j = rest[i..].find('}')? + i;
        let key = rest[i + 1..j].to_lowercase();
        let v = match key.as_str() {
            "appdata" | "localappdata" => user.join("AppData/Local").to_string_lossy().to_string(),
            "roamingappdata" => user.join("AppData/Roaming").to_string_lossy().to_string(),
            "userdir" => user.join("Documents").to_string_lossy().to_string(),
            "userprofile" => user.to_string_lossy().to_string(),
            "usersavedgames" => user.join("Saved Games").to_string_lossy().to_string(),
            "installdir" => install?.to_string(),
            "epicid" => account?.to_string(),
            _ => return None,
        };
        out.push_str(&v);
        rest = &rest[j + 1..];
    }
    out.push_str(rest);
    if out.contains('%') { return None; }
    Some(PathBuf::from(out.replace('\\', "/")))
}

// ------------------------------------------------------------------ GOG (web API + gogdl)

pub mod gog {
    use super::*;
    const CLIENT_ID: &str = "46899977096215655";
    const CLIENT_SECRET: &str = "9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9";
    const REDIRECT: &str = "https://embed.gog.com/on_login_success?origin=client";
    pub const CACHE: &str = "gog-owned.json";
    fn dir() -> PathBuf { data().join("gog") }
    fn auth() -> PathBuf { dir().join("auth.json") }
    pub fn signed_in() -> bool { auth().exists() }
    pub fn profile() -> (Option<String>, Option<String>) { let p = read_json(&dir().join("profile.json")); (p["name"].as_str().map(str::to_string), p["avatar"].as_str().map(str::to_string)) }

    pub fn sign_in() -> SignIn {
        SignIn { title: "Sign in to GOG",
            url: format!("https://auth.gog.com/auth?client_id={CLIENT_ID}&redirect_uri={}&response_type=code&layout=client2", http::urlencode(REDIRECT)),
            done: |u| if u.to_lowercase().starts_with("https://embed.gog.com/on_login_success") { Some(http::query_param(u, "code").unwrap_or_default()) } else { None },
            script: None }
    }
    fn save_token(mut t: Value) -> Result<(), String> {
        if !t["access_token"].is_string() { return Err("GOG didn't accept the sign-in. Try again.".into()); }
        t["loginTime"] = json!(crate::library::now_ms() as f64 / 1000.0);
        crate::paths::write_atomic(&auth(), json!({CLIENT_ID: t}).to_string().as_bytes()).map_err(|e| e.to_string())?;
        let _ = crate::system::run("chmod", &["600", &auth().to_string_lossy()]);
        Ok(())
    }
    pub fn finish_sign_in(code: &str) -> Result<(), String> {
        let url = format!("https://auth.gog.com/token?client_id={CLIENT_ID}&client_secret={CLIENT_SECRET}&grant_type=authorization_code&redirect_uri={}&code={}", http::urlencode(REDIRECT), http::urlencode(code));
        save_token(http::get_json(&url, &[])?)?;
        load_profile();
        let _ = tools::ensure(&tools::GOGDL);
        Ok(())
    }
    pub fn sign_out() { let _ = std::fs::remove_file(auth()); let _ = std::fs::remove_file(dir().join("profile.json")); Owned::remove(CACHE); }
    /// A valid access token, refreshed when it has expired.
    pub fn token() -> Result<String, String> { token_with(false) }
    /// `renew`: refresh even if the saved token should still be good (GOG answered 401 to it).
    /// One refresh at a time: GOG replaces the refresh token, so two at once would make the second look refused.
    fn token_with(renew: bool) -> Result<String, String> {
        static ONE: Mutex<()> = Mutex::new(());
        let _one = ONE.lock().unwrap_or_else(|p| p.into_inner());
        let mut used = String::new();
        for _ in 0..2 {
            let all = read_json(&auth());
            let t = &all[CLIENT_ID];
            if !t.is_object() { return Err("Sign in to GOG first.".into()); }
            let refresh = t["refresh_token"].as_str().unwrap_or("").to_string();
            let now = crate::library::now_ms() as f64 / 1000.0;
            let fresh = now < t["loginTime"].as_f64().unwrap_or(0.0) + t["expires_in"].as_f64().unwrap_or(0.0) - 120.0;
            // (Renewing: unless someone else (gogdl, the other Hearth) already renewed it since the 401.)
            if fresh && (!renew || (!used.is_empty() && refresh != used)) { return Ok(t["access_token"].as_str().unwrap_or("").to_string()); }
            if !used.is_empty() && refresh == used { break; }
            let url = format!("https://auth.gog.com/token?client_id={CLIENT_ID}&client_secret={CLIENT_SECRET}&grant_type=refresh_token&refresh_token={}", http::urlencode(&refresh));
            let r = http::get(&url, &[]).map_err(|e| format!("Couldn't reach GOG to renew your sign-in ({e}). Check the connection and try again."))?;
            let body = r.json();
            if r.ok() && body["access_token"].is_string() {
                let tok = body["access_token"].as_str().unwrap_or("").to_string();
                save_token(body)?;
                super::expiry::clear("gog");
                return Ok(tok);
            }
            if !super::expiry::token_refused(r.status, &r.text) { return Err(format!("GOG didn't renew your sign-in just now (HTTP {}). Try again in a little while.", r.status)); }
            // Refused. If gogdl (or the other Hearth) renewed it in the meantime with the same token, use theirs.
            used = refresh;
        }
        super::expiry::mark("gog", profile().0);
        Err(super::expiry::message("gog"))
    }
    fn api(url: &str) -> Result<Value, String> {
        let get = |t: &str| http::get(url, &[("Authorization", &format!("Bearer {t}"))]);
        let mut r = get(&token()?)?;
        // GOG no longer takes a token that hasn't run out: renew it once (a refused renewal marks it expired).
        if r.status == 401 { r = get(&token_with(true)?)?; }
        if !r.ok() { return Err(format!("HTTP {}", r.status)); }
        Ok(r.json())
    }
    pub fn load_profile() {
        if let Ok(u) = api("https://embed.gog.com/userData.json") {
            let av = &u["avatars"];
            let avatar = av["menu_user_av_big2"].as_str().or(av["menu_user_av_big"].as_str()).map(https);
            let _ = crate::paths::write_atomic(&dir().join("profile.json"), json!({"name": u["username"], "avatar": avatar}).to_string().as_bytes());
        }
    }
    pub fn refresh_owned(old: &Owned) -> Result<Owned, String> {
        if profile().0.is_none() { load_profile(); }
        let mut list = Vec::new();
        let (mut page, mut pages) = (1, 1);
        while page <= pages && page <= 100 {
            let r = api(&format!("https://embed.gog.com/account/getFilteredProducts?mediaType=1&sortBy=title&page={page}"))?;
            pages = r["totalPages"].as_u64().unwrap_or(1);
            for p in r["products"].as_array().cloned().unwrap_or_default() {
                if p["worksOn"]["Windows"] != true || p["id"].is_null() { continue; }
                let id = p["id"].to_string().trim_matches('"').to_string();
                let mut art = serde_json::Map::new();
                if let Some(img) = p["image"].as_str() { let w = format!("{}_product_card_v2_mobile_slider_639.jpg", https(img)); art.insert("wide".into(), json!(w)); art.insert("hero".into(), json!(w)); }
                // Better art from GOG's game database, kept between refreshes.
                if let Some(prev) = old.find(&id).filter(|g| g["art"]["cover"].is_string()) { art = prev["art"].as_object().cloned().unwrap_or_default(); }
                else if let Ok(db) = http::get_json(&format!("https://gamesdb.gog.com/platforms/gog/external_releases/{id}"), &[]) {
                    let f = |k: &str| db["game"][k]["url_format"].as_str().map(|u| u.replace("{formatter}", "").replace("{ext}", "jpg"));
                    if let Some(u) = f("vertical_cover") { art.insert("cover".into(), json!(u)); }
                    if let Some(u) = f("background") { art.insert("hero".into(), json!(u)); }
                    if let Some(u) = f("logo") { art.insert("wide".into(), json!(u)); }
                }
                list.push(json!({"id": id, "title": p["title"].as_str().unwrap_or(&id), "art": art, "added": 0, "lastPlayed": 0, "note": null}));
            }
            page += 1;
        }
        let o = Owned { fetched: crate::library::now_ms(), games: list };
        o.save(CACHE);
        Ok(o)
    }
    /// GOG games Hearth installed (library.json "gogInstalls": id → folder), from each game's goggame-<id>.info.
    pub fn installed(records: &Value, owned: &Owned) -> Vec<crate::library::Game> {
        let mut out = Vec::new();
        for (id, dir) in records.as_object().cloned().unwrap_or_default() {
            let Some(dir) = dir.as_str() else { continue };
            let info = read_json(&Path::new(dir).join(format!("goggame-{id}.info")));
            if info.is_null() { continue; }
            let tasks = info["playTasks"].as_array().cloned().unwrap_or_default();
            let Some(task) = tasks.iter().find(|t| t["isPrimary"] == true).or_else(|| tasks.iter().find(|t| t["type"] == "FileTask")).cloned() else { continue };
            let Some(tp) = task["path"].as_str() else { continue };
            let exe = Path::new(dir).join(tp.replace('\\', "/"));
            let o = owned.find(&id);
            let title = info["name"].as_str().or(o.and_then(|o| o["title"].as_str())).unwrap_or(&id).to_string();
            let mut g = super::game(format!("gog:{id}"), &title, "gog", dir, None, o.map(|o| super::art_of(&o["art"])).unwrap_or_default(), true);
            g.exe = Some(exe.to_string_lossy().into());
            g.args = task["arguments"].as_str().unwrap_or("").to_string();
            g.work_dir = Some(task["workingDir"].as_str().filter(|w| !w.is_empty()).map(|w| Path::new(dir).join(w.replace('\\', "/"))).unwrap_or_else(|| exe.parent().unwrap_or(Path::new(dir)).to_path_buf()));
            out.push(g);
        }
        out
    }
    pub fn install(id: &str, root: &Path, job: &Arc<Mutex<super::Job>>, cancel: Arc<AtomicBool>) -> Result<String, String> {
        token()?;
        let base = root.join("GOG");
        std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
        let j = job.clone();
        let env = vec![("GOGDL_CONFIG_PATH", dir().to_string_lossy().to_string())];
        let r = tools::run(&tools::GOGDL, &["--auth-config-path", &auth().to_string_lossy(), "download", id, "--platform", "windows", "--path", &base.to_string_lossy(), "--skip-dlcs", "--lang", "en-US"],
            &env, Some(Arc::new(move |l: &str| j.lock().unwrap().read_line(l, true))), Some(cancel), None)?;
        let found = std::fs::read_dir(&base).ok().into_iter().flatten().flatten().map(|e| e.path()).find(|d| d.join(format!("goggame-{id}.info")).exists());
        found.map(|d| d.to_string_lossy().to_string()).ok_or_else(|| format!("GOG install failed: {}", tools::problem(&r)))
    }
    pub fn launch(g: &crate::library::Game, opts: &Value, game_id: u32) -> Result<u32, String> {
        let id = g.id.trim_start_matches("gog:");
        // Cloud saves come down first (games with GOG cloud saves).
        if opts["cloudSaves"] != false && !super::expiry::is_expired("gog") {
            if let Some(dir) = g.install_path.as_ref() { if let Err(e) = sync_saves(id, dir, "down") { crate::bridge::log(&format!("gog saves: {e}")); } }
        }
        let args = crate::manual::split_args(&g.args);
        let cwd = g.work_dir.clone().unwrap_or_else(|| "/".into());
        super::proton_launch("gog", id, g.exe.as_deref().unwrap_or(""), args, &cwd, vec![], opts, game_id)
    }

    /// Where a GOG game keeps its saves, from GOG's own settings for it (the same ones GOG Galaxy uses):
    /// (name, folder in the game's Wine prefix). Games without cloud saves give an empty list.
    pub fn save_locations(id: &str, install: &Path) -> Result<Vec<(String, PathBuf)>, String> {
        let info = read_json(&install.join(format!("goggame-{id}.info")));
        let client = info["clientId"].as_str().ok_or("no GOG client id for this game")?.to_string();
        let cfg = http::get_json(&format!("https://remote-config.gog.com/components/galaxy_client/clients/{client}?component_version=2.0.45"), &[])?;
        let cloud = &cfg["content"]["Windows"]["cloudStorage"];
        if cloud["enabled"] != true { return Ok(vec![]); }
        let mut out = Vec::new();
        let list = cloud["locations"].as_array().cloned().unwrap_or_default();
        let prefix = super::prefix_of("gog", id);
        if list.is_empty() {
            let def = format!("%LocalAppData%/GOG.com/Galaxy/Applications/{client}/Storage/Shared/Files");
            if let Some(p) = resolve(&def, &prefix, install) { out.push(("__default".to_string(), p)); }
        }
        for l in list {
            let (Some(name), Some(loc)) = (l["name"].as_str(), l["location"].as_str()) else { continue };
            if let Some(p) = resolve(loc, &prefix, install) { out.push((name.to_string(), p)); }
        }
        Ok(out)
    }

    /// GOG's folder placeholders (<?INSTALL?>, %AppData%, %Documents%...) for the game's Wine prefix.
    pub fn resolve(loc: &str, prefix: &Path, install: &Path) -> Option<PathBuf> {
        let user = prefix.join("drive_c/users/steamuser");
        let map: [(&str, PathBuf); 9] = [
            ("<?INSTALL?>", install.to_path_buf()), ("%INSTALL%", install.to_path_buf()),
            ("%LocalAppData%", user.join("AppData/Local")), ("%AppData%", user.join("AppData/Roaming")), ("%LocalAppDataLow%", user.join("AppData/LocalLow")),
            ("%Documents%", user.join("Documents")), ("%UserProfile%", user.clone()), ("%SavedGames%", user.join("Saved Games")), ("%Public%", prefix.join("drive_c/users/Public")),
        ];
        let mut out = loc.replace('\\', "/");
        for (k, v) in map.iter() {
            // ASCII lowercasing keeps every byte where it was, so the place found is the place in `out` (to_lowercase
            // moved them after letters like İ, and the replace then cut a character in half: a panic).
            if let Some(i) = out.to_ascii_lowercase().find(&k.to_ascii_lowercase()) { out.replace_range(i..i + k.len(), &v.to_string_lossy()); }
        }
        if out.contains('%') || out.contains("<?") { return None; }
        Some(PathBuf::from(out))
    }

    fn sync_file() -> PathBuf { dir().join("saves.json") }

    /// GOG cloud saves: "down" before playing, "up" after. Remembers when each place was last synced, as
    /// gogdl needs. Returns how many places were synced.
    pub fn sync_saves(id: &str, install: &Path, direction: &str) -> Result<usize, String> {
        if !signed_in() { return Ok(0); }
        token()?;
        let locs = save_locations(id, install)?;
        if locs.is_empty() { return Ok(0); }
        let mut stamps = read_json(&sync_file());
        if !stamps.is_object() { stamps = json!({}); }
        let mut n = 0;
        for (name, path) in locs {
            std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
            let ts = stamps[id][&name].as_f64().unwrap_or(0.0).to_string();
            let skip = if direction == "down" { "--skip-upload" } else { "--skip-download" };
            let env = vec![("GOGDL_CONFIG_PATH", dir().to_string_lossy().to_string())];
            let r = tools::run(&tools::GOGDL, &["--auth-config-path", &auth().to_string_lossy(), "save-sync", &path.to_string_lossy(), id, "--os", "windows", "--ts", &ts, "--name", &name, skip],
                &env, None, None, Some(std::time::Duration::from_secs(180)))?;
            if r.code != 0 { crate::bridge::log(&format!("gog saves: {id} {name} {direction}: {}", tools::problem(&r))); continue; }
            // gogdl ends with the new sync time.
            if let Some(t) = r.out.lines().rev().find_map(|l| l.trim().parse::<f64>().ok()) {
                if !stamps[id].is_object() { stamps[id] = json!({}); }
                stamps[id][&name] = json!(t);
            }
            n += 1;
        }
        let _ = crate::paths::write_atomic(&sync_file(), stamps.to_string().as_bytes());
        crate::bridge::log(&format!("gog saves: {id} {direction}: {n} place(s)"));
        Ok(n)
    }
}

// ------------------------------------------------------------------ GOG cloud saves

// ------------------------------------------------------------------ Xbox (Microsoft account → Xbox Live)

pub mod xbox {
    use super::*;
    const CLIENT_ID: &str = "000000004C12AE6F";
    const SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";
    const REDIRECT: &str = "https://login.live.com/oauth20_desktop.srf";
    pub const CACHE: &str = "xbox-titles.json";
    fn secret() -> PathBuf { data().join("accounts/xbox.bin") }

    /// The saved sign-in (refresh token, gamertag, picture). Only the plain-file form ("P1") is readable here;
    /// one the Electron host locked with the desktop keyring means signing in once more.
    pub fn saved() -> Option<Value> {
        let b = std::fs::read(secret()).ok()?;
        if b.len() < 2 || &b[..2] != b"P1" { return None; }
        serde_json::from_slice(&b[2..]).ok()
    }
    fn persist(v: &Value) -> Result<(), String> {
        let mut bytes = b"P1".to_vec(); bytes.extend(v.to_string().as_bytes());
        crate::paths::write_atomic(&secret(), &bytes).map_err(|e| e.to_string())?;
        let _ = crate::system::run("chmod", &["600", &secret().to_string_lossy()]);
        Ok(())
    }
    pub fn signed_in() -> bool { saved().is_some() }

    pub fn sign_in() -> SignIn {
        SignIn { title: "Sign in with your Microsoft account (Xbox)",
            url: format!("https://login.live.com/oauth20_authorize.srf?client_id={CLIENT_ID}&response_type=code&display=touch&scope={}&redirect_uri={}", http::urlencode(SCOPE), http::urlencode(REDIRECT)),
            done: |u| if u.to_lowercase().starts_with(REDIRECT) { Some(http::query_param(u, "code").unwrap_or_default()) } else { None },
            script: None }
    }
    /// Microsoft's token endpoint: (access, refresh), or (refused, what to say). `refused`: Microsoft turned the
    /// code or refresh token down (see expiry::token_refused); otherwise it's a passing problem.
    fn msa_token(form: &[(&str, &str)]) -> Result<(String, String), (bool, String)> {
        let mut body: Vec<String> = form.iter().map(|(k, v)| format!("{k}={}", http::urlencode(v))).collect();
        for (k, v) in [("client_id", CLIENT_ID), ("redirect_uri", REDIRECT), ("scope", SCOPE)] { body.push(format!("{k}={}", http::urlencode(v))); }
        let r = http::post("https://login.live.com/oauth20_token.srf", &body.join("&"), &[("Content-Type", "application/x-www-form-urlencoded")])
            .map_err(|e| (false, format!("Couldn't reach Microsoft to sign in ({e}). Check the connection and try again.")))?;
        let j = r.json();
        match (j["access_token"].as_str(), j["refresh_token"].as_str()) {
            (Some(a), Some(rf)) if r.ok() => Ok((a.into(), rf.into())),
            _ if super::expiry::token_refused(r.status, &r.text) => Err((true, j["error_description"].as_str().unwrap_or("").to_string())),
            _ => Err((false, format!("Microsoft didn't answer the sign-in just now (HTTP {}). Try again in a little while.", r.status))),
        }
    }
    /// A fresh Microsoft account token from the saved sign-in, with the new refresh token saved (Microsoft
    /// replaces it each time). One at a time, so two renewals at once can't make the second look refused.
    /// A refusal marks the sign-in expired. Returns (access token, the saved sign-in).
    fn renew() -> Result<(String, Value), String> {
        static ONE: Mutex<()> = Mutex::new(());
        let _one = ONE.lock().unwrap_or_else(|p| p.into_inner());
        let mut tried = String::new();
        loop {
            let mut saved = saved().ok_or("Sign in to Xbox first.")?;
            let rt = saved["refreshToken"].as_str().unwrap_or("").to_string();
            // Refused, and nobody (the other Hearth) has saved a newer one since: expired.
            if rt == tried {
                super::expiry::mark("xbox", saved["gamertag"].as_str().map(str::to_string));
                return Err(super::expiry::message("xbox"));
            }
            match msa_token(&[("grant_type", "refresh_token"), ("refresh_token", &rt)]) {
                Ok((access, refresh)) => {
                    saved["refreshToken"] = json!(refresh);
                    persist(&saved)?;
                    super::expiry::clear("xbox");
                    return Ok((access, saved));
                }
                Err((true, why)) => { crate::bridge::log(&format!("xbox: Microsoft refused the saved sign-in: {why}")); tried = rt; }
                Err((false, e)) => return Err(e),
            }
        }
    }
    /// Microsoft account token → Xbox user token → Xbox Live (XSTS): (uhs, token, xuid, gamertag)
    fn xsts(msa: &str) -> Result<(String, String, String, Option<String>), String> {
        let hdr = [("Content-Type", "application/json"), ("x-xbl-contract-version", "1")];
        let mut user = None;
        for ticket in [msa.to_string(), format!("t={msa}"), format!("d={msa}")] {
            let r = http::post("https://user.auth.xboxlive.com/user/authenticate", &json!({"RelyingParty": "http://auth.xboxlive.com", "TokenType": "JWT",
                "Properties": {"AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": ticket}}).to_string(), &hdr)?;
            if r.ok() { if let Some(t) = r.json()["Token"].as_str() { user = Some(t.to_string()); break; } }
        }
        let user = user.ok_or("Xbox Live didn't accept this Microsoft account.")?;
        let x = http::post("https://xsts.auth.xboxlive.com/xsts/authorize", &json!({"RelyingParty": "http://xboxlive.com", "TokenType": "JWT", "Properties": {"UserTokens": [user], "SandboxId": "RETAIL"}}).to_string(), &hdr)?;
        if !x.ok() {
            return Err(if x.text.contains("2148916233") { "This Microsoft account doesn't have an Xbox profile yet. Create one at xbox.com, then try again.".into() }
                else if x.text.contains("2148916238") { "This is a child account. An adult needs to add it to a Microsoft family first.".into() }
                else { format!("Xbox Live sign-in failed ({}).", x.status) });
        }
        let j = x.json();
        let xui = &j["DisplayClaims"]["xui"][0];
        match (j["Token"].as_str(), xui["uhs"].as_str(), xui["xid"].as_str()) {
            (Some(t), Some(u), Some(id)) => Ok((u.into(), t.into(), id.into(), xui["gtg"].as_str().map(str::to_string))),
            _ => Err("Xbox Live sign-in failed (no profile in the response).".into()),
        }
    }
    fn get(url: &str, contract: &str, auth: &(String, String, String, Option<String>)) -> Result<Value, String> {
        http::get_json(url, &[("Authorization", &format!("XBL3.0 x={};{}", auth.0, auth.1)), ("x-xbl-contract-version", contract), ("Accept-Language", "en-US")])
    }
    fn profile(saved: &mut Value, auth: &(String, String, String, Option<String>)) {
        if let Ok(j) = get(&format!("https://profile.xboxlive.com/users/xuid({})/profile/settings?settings=GameDisplayPicRaw,Gamertag,ModernGamertag", auth.2), "2", auth) {
            for s in j["profileUsers"][0]["settings"].as_array().cloned().unwrap_or_default() {
                let Some(v) = s["value"].as_str() else { continue };
                match s["id"].as_str().unwrap_or("") { "GameDisplayPicRaw" => saved["gamerpic"] = json!(https(v)), "ModernGamertag" => saved["gamertag"] = json!(v),
                    "Gamertag" => if saved["gamertag"].is_null() { saved["gamertag"] = json!(v) }, _ => {} }
            }
        }
    }
    pub fn finish_sign_in(code: &str) -> Result<(), String> {
        let (access, refresh) = msa_token(&[("grant_type", "authorization_code"), ("code", code)])
            .map_err(|(refused, e)| if refused { "Microsoft didn't accept the sign-in. Try again.".to_string() } else { e })?;
        let auth = xsts(&access)?;
        let mut saved = json!({"refreshToken": refresh, "xuid": auth.2, "gamertag": auth.3});
        profile(&mut saved, &auth);
        persist(&saved)
    }
    pub fn sign_out() { let _ = std::fs::remove_file(secret()); Owned::remove(CACHE); }
    /// Xbox Live sign-in for the friends list, kept for 50 minutes (the tokens last longer).
    fn auth_cached() -> Result<(String, String, String, Option<String>), String> {
        static A: std::sync::OnceLock<Mutex<Option<(u64, (String, String, String, Option<String>))>>> = std::sync::OnceLock::new();
        let a = A.get_or_init(Default::default);
        let now = crate::library::now_ms();
        if let Some((at, auth)) = a.lock().unwrap().clone() { if now.saturating_sub(at) < 50 * 60_000 { return Ok(auth); } }
        // The friends list asks often: with an expired sign-in it waits for Sign in again (or a working refresh).
        if super::expiry::is_expired("xbox") { return Err(super::expiry::message("xbox")); }
        let (access, _) = renew()?;
        let auth = xsts(&access)?;
        *a.lock().unwrap() = Some((now, auth.clone()));
        Ok(auth)
    }
    /// Your Xbox friends with what they're doing (Xbox's people hub).
    pub fn friends() -> Result<Vec<Value>, String> {
        let auth = auth_cached()?;
        let j = get("https://peoplehub.xboxlive.com/users/me/people/social/decoration/presencedetail", "5", &auth)?;
        Ok(j["people"].as_array().cloned().unwrap_or_default().iter().filter(|p| p["isFollowedByCaller"] != false || p["isFollowingCaller"] == true).map(|p| {
            let details = p["presenceDetails"].as_array().cloned().unwrap_or_default();
            let game = details.iter().find(|d| d["IsGame"] == true && d["IsPrimary"] != false).or_else(|| details.iter().find(|d| d["IsGame"] == true));
            let online = p["presenceState"].as_str() == Some("Online");
            let state = if game.is_some() && online { "ingame" } else if online { "online" } else { "offline" };
            let name = p["modernGamertag"].as_str().filter(|s| !s.is_empty()).or(p["gamertag"].as_str()).unwrap_or("Xbox friend");
            let gname = game.and_then(|g| g["PresenceText"].as_str().map(str::to_string)).or_else(|| p["presenceText"].as_str().filter(|_| game.is_some()).map(str::to_string));
            json!({"id": p["xuid"], "platform": "xbox", "name": name, "avatar": p["displayPicRaw"].as_str().map(|u| crate::artcache::url(&https(u))),
                   "state": state, "game": gname, "message": p["presenceText"].as_str().filter(|t| !t.is_empty() && *t != "Offline"),
                   "titleId": game.and_then(|g| g["TitleId"].as_str()), "canMessage": false, "canJoin": false})
        }).collect())
    }
    /// Games you've played on Xbox (title history), with Xbox art.
    pub fn refresh_owned() -> Result<Owned, String> {
        let (access, mut saved) = renew()?;   // Microsoft rotates refresh tokens
        let auth = xsts(&access)?;
        if saved["gamerpic"].is_null() { profile(&mut saved, &auth); persist(&saved)?; }
        let j = get(&format!("https://titlehub.xboxlive.com/users/xuid({})/titles/titlehistory/decoration/image,detail?maxItems=1000", auth.2), "2", &auth)?;
        let mut o = Owned { fetched: crate::library::now_ms(), games: vec![] };
        for t in j["titles"].as_array().cloned().unwrap_or_default() {
            if t["type"] != "Game" || !t["name"].is_string() { continue; }
            let imgs = t["images"].as_array().cloned().unwrap_or_default();
            let img = |types: &[&str]| types.iter().find_map(|ty| imgs.iter().find(|i| i["type"] == *ty).and_then(|i| i["url"].as_str()).map(https));
            let mut art = serde_json::Map::new();
            if let Some(c) = img(&["Poster", "BoxArt"]).or(t["displayImage"].as_str().map(https)) { art.insert("cover".into(), json!(c)); }
            if let Some(h) = img(&["SuperHeroArt", "TitledHeroArt", "BrandedKeyArt"]) { art.insert("hero".into(), json!(h)); }
            if let Some(w) = img(&["BrandedKeyArt", "TitledHeroArt", "SuperHeroArt"]) { art.insert("wide".into(), json!(w)); }
            if let Some(l) = img(&["Logo"]) { art.insert("logo".into(), json!(l)); }
            o.games.push(json!({"id": t["pfn"].as_str().or(t["titleId"].as_str()), "title": t["name"], "art": art, "added": 0,
                                "lastPlayed": ms(t["titleHistory"]["lastTimePlayed"].as_str().unwrap_or("")), "note": null}));
        }
        o.save(CACHE);
        Ok(o)
    }
}

// ------------------------------------------------------------------ Xbox Cloud Gaming catalog

pub mod xcloud {
    use super::*;
    const BASE: &str = "https://catalog.gamepass.com";
    const ALL_GAMES: &str = "1bf84c2b-0643-4591-893f-d9edb703f692";
    pub const GALLERY: &str = "https://play.xbox.com/gallery/all-games";
    fn cache() -> PathBuf { data().join("xcloud-catalog.json") }

    pub fn locale() -> (String, String) {
        let l = std::env::var("LC_ALL").ok().filter(|s| !s.is_empty()).or(std::env::var("LANG").ok()).unwrap_or_default();
        let b = l.as_bytes();
        if b.len() >= 5 && b[0].is_ascii_lowercase() && b[1].is_ascii_lowercase() && b[2] == b'_' && b[3].is_ascii_uppercase() && b[4].is_ascii_uppercase() {
            (l[3..5].to_string(), format!("{}-{}", &l[..2], l[3..5].to_lowercase()))
        } else { ("US".into(), "en-us".into()) }
    }
    pub fn slug(title: &str) -> String {
        let s: String = title.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace() || *c == '-').collect();
        let s = s.split(|c: char| c.is_whitespace() || c == '-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
        if s.is_empty() { "_".into() } else { s }
    }
    fn img(p: &Value, keys: &[&str]) -> Option<String> { keys.iter().find_map(|k| p[*k]["URL"].as_str().map(https)) }
    fn fetch() -> Result<Value, String> {
        let (market, language) = locale();
        let sigl = http::get_json(&format!("{BASE}/sigls/v2?id={ALL_GAMES}&market={market}&language={language}"), &[])?;
        let mut ids: Vec<String> = Vec::new();
        for x in sigl.as_array().cloned().unwrap_or_default() { if let Some(id) = x["id"].as_str() { if !ids.iter().any(|i| i == id) { ids.push(id.into()); } } }
        let mut games = Vec::new();
        for chunk in ids.chunks(50) {
            // A minute per part (in the background): the catalog is big, and 30 s cut slow connections off halfway.
            let r = http::request("POST", &format!("{BASE}/v3/products?market={market}&language={language}&hydration=RemoteHighSapphire0"),
                &[("Content-Type", "application/json"), ("ms-cv", "cv"), ("calling-app-name", "Nimbus"), ("calling-app-version", "1")], Some(&json!({"Products": chunk}).to_string()), 60)?;
            for (id, p) in r.json()["Products"].as_object().cloned().unwrap_or_default() {
                let Some(title) = p["ProductTitle"].as_str() else { continue };
                if p["XCloudOfferings"].is_object() && p["XCloudOfferings"]["CLOUDGAMING"].is_null() { continue; }
                games.push(json!({"id": id, "title": title, "cover": img(&p, &["Image_Poster", "Image_BoxArt", "Image_Tile"]), "wide": img(&p, &["Image_Tile", "Image_Hero", "Image_SuperHero"]),
                                  "hero": img(&p, &["Image_Hero", "Image_SuperHero", "Image_Tile"]), "logo": img(&p, &["Image_Logo"])}));
            }
        }
        Ok(json!({"market": market, "fetched": crate::library::now_ms(), "games": games}))
    }
    /// The catalog as saved. When it's older than a day (or from another country) a fresh copy is fetched in
    /// the background, and the library is rescanned once it's in; a failed refresh keeps the old copy.
    pub fn catalog() -> Vec<Value> {
        static FETCHING: AtomicBool = AtomicBool::new(false);
        let cached = read_json(&cache());
        let fresh = cached["market"] == locale().0.as_str() && crate::library::now_ms().saturating_sub(cached["fetched"].as_u64().unwrap_or(0)) < 86_400_000;
        if !fresh && !FETCHING.swap(true, Ordering::SeqCst) {
            std::thread::spawn(|| {
                match fetch() {
                    Ok(c) if c["games"].as_array().map(|g| !g.is_empty()).unwrap_or(false) => {
                        let _ = crate::paths::write_atomic(&cache(), c.to_string().as_bytes());
                        crate::bridge::log(&format!("xcloud: catalog {} games", c["games"].as_array().map(|g| g.len()).unwrap_or(0)));
                        super::notify(json!({"type": "internal", "what": "rescan", "reply": null, "changed": true}));
                    }
                    Ok(_) => crate::bridge::log("xcloud: catalog came back empty"),
                    Err(e) => crate::bridge::log(&format!("xcloud: catalog refresh failed: {e}")),
                }
                FETCHING.store(false, Ordering::SeqCst);
            });
        }
        cached["games"].as_array().cloned().unwrap_or_default()
    }
    pub fn scan() -> Vec<crate::library::Game> {
        let mut out = vec![{ let mut g = super::game("xbox:cloud".into(), "Xbox Cloud Gaming", "xbox", "", None, vec![], true); g.cloud = true; g.stream_url = Some(GALLERY.into()); g }];
        for e in catalog() {
            let (Some(id), Some(title)) = (e["id"].as_str(), e["title"].as_str()) else { continue };
            let art = ["cover", "wide", "hero", "logo"].iter().filter_map(|k| e[*k].as_str().map(|u| (*k, u.to_string()))).collect();
            let mut g = super::game(format!("xbox:{id}"), title, "xbox", "", None, art, false);
            g.cloud = true; g.state = "cloud"; g.install_path = None;
            g.stream_url = Some(format!("https://play.xbox.com/products/{id}/{}", slug(title)));
            out.push(g);
        }
        out
    }
}

// ------------------------------------------------------------------ the download queue

/// One Epic or GOG download, with progress read from the tool's output.
#[derive(Debug, Clone, Default)]
pub struct Job { pub game_id: String, pub title: String, pub uri: String, pub root: PathBuf, pub state: String, pub paused: bool, pub cancelled: bool, pub progress: Option<f64>, pub bytes_done: Option<u64>, pub bytes_total: Option<u64>, pub speed: u64 }

impl Job {
    /// One output line. bytes_in_line: gogdl's x/y are bytes; legendary's are chunks.
    pub fn read_line(&mut self, line: &str, bytes_in_line: bool) {
        use std::sync::OnceLock;
        static RE: OnceLock<[regex::Regex; 3]> = OnceLock::new();
        let [percent, speed, size] = RE.get_or_init(|| [
            regex::Regex::new(r"= Progress: ([\d.]+)%?(?: \((\d+)/(\d+)\)| (\d+)/(\d+))?").unwrap(),
            regex::Regex::new(r"\+ Download\s+- ([\d.]+) MiB/s").unwrap(),
            regex::Regex::new(r"Download size: ([\d.]+) MiB").unwrap()]);
        if let Some(m) = percent.captures(line) {
            self.state = "downloading".into();
            let p = (m[1].parse::<f64>().unwrap_or(0.0) / 100.0).clamp(0.0, 1.0);
            self.progress = Some(p);
            if let (true, Some(a), Some(b)) = (bytes_in_line, m.get(4), m.get(5)) { self.bytes_done = a.as_str().parse().ok(); self.bytes_total = b.as_str().parse().ok(); }
            else if let Some(t) = self.bytes_total { self.bytes_done = Some((t as f64 * p).round() as u64); }
        } else if let Some(m) = speed.captures(line) {
            self.speed = (m[1].parse::<f64>().unwrap_or(0.0) * 1048576.0).round() as u64;
        } else if let (false, Some(m)) = (bytes_in_line, size.captures(line)) {
            self.bytes_total = Some((m[1].parse::<f64>().unwrap_or(0.0) * 1048576.0).round() as u64);
        }
    }
}

pub type Entry = (Arc<Mutex<Job>>, Arc<AtomicBool>);

/// The download queue: one download at a time, the rest wait (or are paused).
pub struct Queue { pub jobs: Vec<Entry>, pub running: bool }

impl Default for Queue { fn default() -> Queue { Queue::new() } }

impl Queue {
    pub fn new() -> Queue { Queue { jobs: vec![], running: false } }
    fn pos(&self, id: &str) -> Option<usize> { self.jobs.iter().position(|(j, _)| j.lock().unwrap().game_id == id) }
    pub fn get(&self, id: &str) -> Option<Arc<Mutex<Job>>> { self.pos(id).map(|i| self.jobs[i].0.clone()) }
    pub fn state(&self, id: &str) -> Option<Value> {
        self.get(id).map(|j| { let j = j.lock().unwrap(); json!({"state": j.state, "progress": j.progress, "bytesDone": j.bytes_done, "bytesTotal": j.bytes_total, "bytesPerSec": j.speed}) })
    }
    pub fn ids(&self) -> Vec<String> { self.jobs.iter().map(|(j, _)| j.lock().unwrap().game_id.clone()).collect() }
    pub fn add(&mut self, job: Job) -> Result<(), String> {
        if self.pos(&job.game_id).is_some() { return Err(format!("{} is already in the download queue.", job.title)); }
        self.jobs.push((Arc::new(Mutex::new(job)), Arc::new(AtomicBool::new(false))));
        Ok(())
    }
    /// (found, removed now). A waiting download goes at once; the one downloading stops and goes when its tool has.
    pub fn cancel(&mut self, id: &str) -> (bool, bool) {
        let Some(pos) = self.pos(id) else { return (false, false) };
        self.jobs[pos].1.store(true, Ordering::SeqCst);
        let mut j = self.jobs[pos].0.lock().unwrap();
        j.cancelled = true;
        let waiting = j.state == "queued" || j.state == "paused";
        drop(j);
        if waiting { self.jobs.remove(pos); }
        (true, waiting)
    }
    /// Stops a download where it is (the files so far are kept; resuming carries on from there).
    pub fn pause(&mut self, id: &str) -> bool {
        let Some(pos) = self.pos(id) else { return false };
        let mut j = self.jobs[pos].0.lock().unwrap();
        if j.paused { return false; }
        j.paused = true;
        if j.state == "queued" { j.state = "paused".into(); } else { self.jobs[pos].1.store(true, Ordering::SeqCst); }
        true
    }
    pub fn resume(&mut self, id: &str) -> bool {
        let Some(pos) = self.pos(id) else { return false };
        let mut j = self.jobs[pos].0.lock().unwrap();
        if !j.paused { return false; }
        j.paused = false; j.state = "queued".into(); j.speed = 0;
        drop(j);
        self.jobs[pos].1 = Arc::new(AtomicBool::new(false));
        true
    }
    /// Puts a download first in line (it starts when the current one finishes).
    pub fn first(&mut self, id: &str) -> bool {
        let Some(pos) = self.pos(id) else { return false };
        let e = self.jobs.remove(pos); self.jobs.insert(0, e); true
    }
}

/// Runs the queue in the background until it's empty. When a download ends, the host hears
/// {type:'internal', what:'installDone', id, error, gogDir}; when one pauses, {what:'queueChanged'}.
pub fn pump(queue: Arc<Mutex<Queue>>) {
    { let mut q = queue.lock().unwrap(); if q.running { return; } q.running = true; }
    std::thread::spawn(move || loop {
        let next = {
            let mut q = queue.lock().unwrap();
            let e = q.jobs.iter().find(|(j, _)| !j.lock().unwrap().paused).cloned();
            if e.is_none() { q.running = false; return; }
            e.unwrap()
        };
        let (job, cancel) = next;
        let (id, uri, root) = { let mut j = job.lock().unwrap(); j.state = "downloading".into(); (j.game_id.clone(), j.uri.clone(), j.root.clone()) };
        crate::bridge::log(&format!("install {id}: starting"));
        let res: Result<Option<String>, String> = if let Some(app) = uri.strip_prefix("nimbus-epic:") { epic::install(app, &root, &job, cancel.clone()).map(|_| None) }
            else if let Some(g) = uri.strip_prefix("nimbus-gog:") { gog::install(g, &root, &job, cancel.clone()).map(Some) }
            else { Err("Hearth can't install this game itself.".into()) };
        let mut q = queue.lock().unwrap();
        let Some(pos) = q.jobs.iter().position(|(j, _)| Arc::ptr_eq(j, &job)) else { continue };
        let (cancelled, paused) = { let j = job.lock().unwrap(); (j.cancelled, j.paused) };
        let stopped = cancel.load(Ordering::SeqCst) && res.is_err();
        if stopped && !cancelled {
            // Paused (or paused and resumed before the tool stopped): stays in the queue.
            let mut j = job.lock().unwrap();
            j.speed = 0;
            j.state = if paused { "paused".into() } else { "queued".into() };
            drop(j);
            if !Arc::ptr_eq(&q.jobs[pos].1, &cancel) || !paused { q.jobs[pos].1 = Arc::new(AtomicBool::new(false)); }
            drop(q);
            crate::bridge::log(&format!("install {id}: paused"));
            notify(json!({"type": "internal", "what": "queueChanged"}));
            continue;
        }
        q.jobs.remove(pos);
        drop(q);
        let error = if cancelled && res.is_err() { Some("cancelled".to_string()) } else { res.as_ref().err().cloned() };
        crate::bridge::log(&format!("install {id}: finished: {}", error.as_deref().unwrap_or("ok")));
        notify(json!({"type": "internal", "what": "installDone", "id": id, "error": error, "gogDir": res.ok().flatten()}));
    });
}

// ------------------------------------------------------------------ accounts: busy, errors, background work

/// Sends a message to the host from background work (set once by the app).
static NOTIFY: std::sync::OnceLock<crate::system::bt_scan::Emit> = std::sync::OnceLock::new();
pub fn set_notify(e: crate::system::bt_scan::Emit) { let _ = NOTIFY.set(e); }
pub fn notify(v: Value) { if let Some(e) = NOTIFY.get() { e(v); } }

#[derive(Default)]
pub struct AccState { pub busy: std::collections::HashSet<String>, pub errors: std::collections::HashMap<String, String> }

pub const STORES: [&str; 3] = ["epic", "gog", "xbox"];
pub fn signed_in(s: &str) -> bool { match s { "epic" => epic::signed_in(), "gog" => gog::signed_in(), "xbox" => xbox::signed_in(), _ => false } }
pub fn cache_of(s: &str) -> &'static str { match s { "epic" => epic::CACHE, "gog" => gog::CACHE, _ => xbox::CACHE } }
pub fn sign_in_page(s: &str) -> Option<SignIn> { match s { "epic" => Some(epic::sign_in()), "gog" => Some(gog::sign_in()), "xbox" => Some(xbox::sign_in()), _ => None } }
pub fn finish_sign_in(s: &str, code: &str) -> Result<(), String> {
    match s { "epic" => epic::finish_sign_in(code), "gog" => gog::finish_sign_in(code), "xbox" => xbox::finish_sign_in(code), _ => Err("Unknown store".into()) }
}
pub fn sign_out(s: &str) -> Result<(), String> {
    match s { "epic" => epic::sign_out(), "gog" => gog::sign_out(), "xbox" => xbox::sign_out(), _ => return Err("Unknown store".into()) }
    expiry::clear(s);
    crate::bridge::log(&format!("accounts: signed out of {s}"));
    Ok(())
}

/// Downloads a store's game list again, in the background. Errors are kept and shown on the Stores page.
/// When it's done the host hears {what:'rescan', reply} (reply: a call id to answer once the library is rescanned).
pub fn refresh(s: &'static str, acc: Arc<Mutex<AccState>>, reply: Value) {
    {
        let mut a = acc.lock().unwrap();
        if !signed_in(s) || a.busy.contains(s) { drop(a); notify(json!({"type": "internal", "what": "rescan", "reply": reply})); return; }
        a.busy.insert(s.to_string());
    }
    notify(json!({"type": "event", "name": "accounts.changed", "data": {}}));
    std::thread::spawn(move || {
        let r = match s { "epic" => epic::refresh_owned(), "gog" => gog::refresh_owned(&Owned::load(gog::CACHE)), _ => xbox::refresh_owned() };
        let mut a = acc.lock().unwrap();
        a.busy.remove(s);
        match r {
            Ok(o) => { a.errors.remove(s); expiry::clear(s); crate::bridge::log(&format!("accounts: {s} list has {} games", o.games.len())); }
            // An expired sign-in shows as that (with Sign in again), not as an error.
            Err(e) if expiry::is_expired(s) => { crate::bridge::log(&format!("accounts: {s} refresh FAILED (sign-in expired) {e}")); a.errors.remove(s); }
            Err(e) => { crate::bridge::log(&format!("accounts: {s} refresh FAILED {e}")); a.errors.insert(s.to_string(), e); }
        }
        drop(a);
        notify(json!({"type": "event", "name": "accounts.changed", "data": {}}));
        notify(json!({"type": "internal", "what": "rescan", "reply": reply}));
    });
}
/// At startup: lists older than 12 hours are fetched again.
pub fn refresh_stale(acc: &Arc<Mutex<AccState>>) {
    for s in STORES {
        if signed_in(s) && !expiry::is_expired(s) && crate::library::now_ms().saturating_sub(Owned::load(cache_of(s)).fetched) > 12 * 3600 * 1000 { refresh(s, acc.clone(), Value::Null); }
    }
}

// ------------------------------------------------------------------ expired sign-ins

/// A store sign-in that stopped working: the store refused the saved sign-in (its refresh token was revoked or
/// ran out), as opposed to a passing problem (no connection, the store down, slow or rate-limiting). Library ›
/// Stores shows "Sign-in expired" with Sign in again, and the page hears `accounts.expired` once per expiry.
/// Kept in accounts/expired.json (per profile, like the sign-ins themselves) until the next sign-in, a working
/// refresh, or signing out. The rule that matters most: a network outage never marks a sign-in as expired.
pub mod expiry {
    use super::*;

    /// OAuth `error` codes that mean the saved sign-in is no good anymore (RFC 6749 §5.2, and Microsoft's).
    const DEAD: [&str; 6] = ["invalid_grant", "invalid_token", "unauthorized_client", "interaction_required", "login_required", "consent_required"];

    fn oauth_error(body: &str) -> Option<String> {
        serde_json::from_str::<Value>(body).ok().and_then(|j| j["error"].as_str().map(|e| e.trim().to_lowercase()))
    }

    /// A token endpoint's answer to renewing a sign-in (auth.gog.com/token, login.live.com/oauth20_token.srf).
    /// Only a clear refusal counts: 400/401/403 naming one of the codes above, or a bare 401. No answer (status 0),
    /// timeouts, rate limits, server errors, and a 400/403 without such a code (a captive portal or a proxy can
    /// send those) are temporary. So is a 401 that names some other code ("temporarily_unavailable").
    pub fn token_refused(status: u16, body: &str) -> bool {
        match (status, oauth_error(body)) {
            (400 | 401 | 403, Some(e)) => DEAD.contains(&e.as_str()),
            (401, None) => true,
            _ => false,
        }
    }

    /// legendary's output after an online command. legendary says so when Epic refused the saved sign-in
    /// ("Stored credentials are no longer valid! Please login again.") and only then deletes user.json; "Login
    /// failed, cannot continue!" alone is also what it prints without a connection, so that counts only when
    /// user.json went away during the command. Any sign of a network problem wins.
    pub fn legendary_refused(out: &str, user_file_gone: bool) -> bool {
        let t = out.to_lowercase();
        const NETWORK: [&str; 10] = ["http request for login failed", "connectionerror", "connection error", "connection aborted", "timed out", "timeout",
            "max retries exceeded", "name or service not known", "temporary failure in name resolution", "please try again later"];
        if NETWORK.iter().any(|n| t.contains(n)) { return false; }
        const DEAD_EPIC: [&str; 6] = ["stored credentials are no longer valid", "no saved credentials", "please login again", "please log in again",
            "invalid_refresh_token", "errors.com.epicgames.account.auth_token"];
        DEAD_EPIC.iter().any(|d| t.contains(d)) || (user_file_gone && t.contains("login failed"))
    }

    pub fn label(store: &str) -> &'static str { match store { "epic" => "Epic Games", "gog" => "GOG", "xbox" => "Microsoft (Xbox)", _ => "store" } }
    /// What a call that failed because of it says.
    pub fn message(store: &str) -> String { format!("Your {} sign-in expired. Sign in again under Library › Stores.", label(store)) }

    fn file() -> PathBuf { data().join("accounts/expired.json") }
    static LOCK: Mutex<()> = Mutex::new(());
    fn load() -> Value { let v = read_json(&file()); if v.is_object() { v } else { json!({}) } }
    fn save(v: &Value) { let _ = crate::paths::write_atomic(&file(), v.to_string().as_bytes()); }

    /// The record for a store ({at, name}), when its sign-in has expired.
    pub fn get(store: &str) -> Option<Value> { let e = load()["expired"][store].clone(); if e.is_object() { Some(e) } else { None } }
    pub fn is_expired(store: &str) -> bool { get(store).is_some() }
    /// The account name last seen for a store (for "Sign in again as <name>" after legendary deleted user.json).
    pub fn known_name(store: &str) -> Option<String> { load()["names"][store].as_str().map(str::to_string) }

    /// Marks it in `v` (the file's contents). None when it was already marked: one notice per expiry.
    pub fn apply_mark(v: &mut Value, store: &str, name: Option<String>, now: u64) -> Option<Value> {
        if v["expired"][store].is_object() { return None; }
        let name = name.or_else(|| v["names"][store].as_str().map(str::to_string));
        if !v["expired"].is_object() { v["expired"] = json!({}); }
        let rec = json!({"at": now, "name": name});
        v["expired"][store] = rec.clone();
        Some(rec)
    }
    /// Clears it in `v`. Whether there was anything to clear.
    pub fn apply_clear(v: &mut Value, store: &str) -> bool {
        v.get_mut("expired").and_then(|e| e.as_object_mut()).map(|e| e.remove(store).is_some()).unwrap_or(false)
    }

    /// The store refused the saved sign-in. The first time (per expiry) it's logged and the page hears
    /// `accounts.expired` {store, name, at}, then `accounts.changed`. Returns whether it was new.
    pub fn mark(store: &str, name: Option<String>) -> bool {
        let rec = {
            let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
            let mut v = load();
            let Some(rec) = apply_mark(&mut v, store, name, crate::library::now_ms()) else { return false };
            save(&v);
            rec
        };
        crate::bridge::log(&format!("accounts: {store} sign-in EXPIRED (the store refused the saved sign-in)"));
        notify(json!({"type": "event", "name": "accounts.expired", "data": {"store": store, "name": rec["name"], "at": rec["at"]}}));
        notify(json!({"type": "event", "name": "accounts.changed", "data": {}}));
        true
    }
    /// Signed in again, signed out, or the store answered with the saved sign-in after all.
    pub fn clear(store: &str) -> bool {
        let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let mut v = load();
        if !apply_clear(&mut v, store) { return false; }
        save(&v);
        drop(_one);
        crate::bridge::log(&format!("accounts: {store} sign-in works again"));
        notify(json!({"type": "event", "name": "accounts.changed", "data": {}}));
        true
    }
    /// Remembers the account's name (written only when it changes).
    pub fn remember_name(store: &str, name: &str) {
        if name.is_empty() { return; }
        let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let mut v = load();
        if v["names"][store].as_str() == Some(name) { return; }
        if !v["names"].is_object() { v["names"] = json!({}); }
        v["names"][store] = json!(name);
        save(&v);
    }
}

// ------------------------------------------------------------------ helpers for library games

pub fn art_of(v: &Value) -> Vec<(&'static str, String)> {
    ["cover", "hero", "wide", "logo"].iter().filter_map(|k| v[*k].as_str().map(|u| (*k, u.to_string()))).collect()
}
pub fn game(id: String, title: &str, source: &'static str, path: &str, size: Option<u64>, art: Vec<(&'static str, String)>, installed: bool) -> crate::library::Game {
    let added = std::fs::metadata(path).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0);
    crate::library::Game { id, title: title.to_string(), source, install_path: if path.is_empty() { None } else { Some(path.into()) }, size_bytes: size, added,
        store_minutes: 0, store_last_played: 0, installed, state: if installed { "installed" } else { "notInstalled" }, bytes_done: None, bytes_total: None,
        art, launch_uri: None, exe: None, args: String::new(), system: None, system_key: None, can_uninstall: installed && (source == "epic" || source == "gog"), ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_lines() {
        let mut j = Job::default();
        j.read_line("[cli] INFO: Download size: 1024.00 MiB", false);
        assert_eq!(j.bytes_total, Some(1073741824));
        j.read_line("[DLManager] INFO: = Progress: 25.50% (1200/4700), Running for 00:01:00", false);
        assert!((j.progress.unwrap() - 0.255).abs() < 1e-9); assert_eq!(j.bytes_done, Some(273804165));
        j.read_line("[DLManager] INFO:  + Download\t- 12.50 MiB/s (raw) / 20.00 MiB/s (decompressed)", false);
        assert_eq!(j.speed, 13107200);
        let mut g = Job::default();
        g.read_line("= Progress: 50 100/200", true);
        assert_eq!((g.progress, g.bytes_done, g.bytes_total), (Some(0.5), Some(100), Some(200)));
    }
    #[test]
    fn epic_save_folders() {
        let p = Path::new("/p/epic-A");
        assert_eq!(save_path("{AppData}/A/Saved", p, None, None).unwrap(), PathBuf::from("/p/epic-A/drive_c/users/steamuser/AppData/Local/A/Saved"));
        assert_eq!(save_path("{UserDir}\\My Games\\{EpicID}", p, None, Some("acc")).unwrap(), PathBuf::from("/p/epic-A/drive_c/users/steamuser/Documents/My Games/acc"));
        assert!(save_path("{Mystery}/x", p, None, None).is_none());
        assert!(save_path("{EpicID}/x", p, None, None).is_none());
    }
    #[test]
    fn cloud_names() {
        assert_eq!(xcloud::slug("Forza Horizon 5: Premium"), "forza-horizon-5-premium");
        assert_eq!((epic::sign_in().done)("hearth-signin://done?code=abc").as_deref(), Some("abc"));
        assert_eq!((gog::sign_in().done)("https://embed.gog.com/on_login_success?origin=client&code=xyz").as_deref(), Some("xyz"));
        assert_eq!((gog::sign_in().done)("https://auth.gog.com/login"), None);
    }
    #[test]
    fn gog_save_folders() {
        let (p, i) = (Path::new("/p/gog-1"), Path::new("/games/Witcher"));
        assert_eq!(gog::resolve("<?INSTALL?>/saves", p, i).unwrap(), PathBuf::from("/games/Witcher/saves"));
        assert_eq!(gog::resolve("%Documents%\\The Witcher 3\\gamesaves", p, i).unwrap(), PathBuf::from("/p/gog-1/drive_c/users/steamuser/Documents/The Witcher 3/gamesaves"));
        assert!(gog::resolve("%Mystery%/x", p, i).is_none());
        // Letters whose lowercase is longer (İ) before a placeholder: the placeholder is still the one replaced.
        assert_eq!(gog::resolve("İstanbul/%AppData%/x", p, i).unwrap(), PathBuf::from("İstanbul/p/gog-1/drive_c/users/steamuser/AppData/Roaming/x"));
        assert_eq!(gog::resolve("%AppData%/İİ/x", p, i).unwrap(), PathBuf::from("/p/gog-1/drive_c/users/steamuser/AppData/Roaming/İİ/x"));
        assert_eq!(gog::resolve("<?INSTALL?>/%appdata%", Path::new("/p/gog-1"), Path::new("/games/İİİ")).unwrap(),
                   PathBuf::from("/games/İİİ//p/gog-1/drive_c/users/steamuser/AppData/Roaming"));
    }
    #[test]
    fn expired_token_replies() {
        // The store refused the saved sign-in: expired.
        assert!(expiry::token_refused(400, r#"{"error":"invalid_grant","error_description":"The refresh token is invalid."}"#));
        assert!(expiry::token_refused(400, r#"{"error":"invalid_grant","error_description":"AADSTS70008: The refresh token has expired due to inactivity."}"#));
        assert!(expiry::token_refused(401, r#"{"error":"invalid_token"}"#));
        assert!(expiry::token_refused(400, r#"{"error":"interaction_required"}"#));
        assert!(expiry::token_refused(403, r#"{"error":"unauthorized_client"}"#));
        assert!(expiry::token_refused(401, ""));
        assert!(expiry::token_refused(401, "Unauthorized"));
        // Passing problems: never expired.
        assert!(!expiry::token_refused(0, ""));                                     // no connection (curl got nothing)
        assert!(!expiry::token_refused(500, r#"{"error":"invalid_grant"}"#));       // the store is broken, not the sign-in
        assert!(!expiry::token_refused(502, "<html>Bad Gateway</html>"));
        assert!(!expiry::token_refused(503, ""));
        assert!(!expiry::token_refused(504, ""));
        assert!(!expiry::token_refused(429, r#"{"error":"too_many_requests"}"#));
        assert!(!expiry::token_refused(408, ""));
        assert!(!expiry::token_refused(511, "<html>Sign in to the Wi-Fi</html>"));  // captive portal
        assert!(!expiry::token_refused(403, "<html>Access denied</html>"));         // a proxy or firewall page
        assert!(!expiry::token_refused(400, "<html>Bad request</html>"));
        assert!(!expiry::token_refused(400, r#"{"error":"invalid_request"}"#));
        assert!(!expiry::token_refused(401, r#"{"error":"temporarily_unavailable"}"#));
        assert!(!expiry::token_refused(200, r#"{"access_token":"a","refresh_token":"b"}"#));
    }
    #[test]
    fn expired_legendary_output() {
        assert!(expiry::legendary_refused("[Core] ERROR: Stored credentials are no longer valid! Please login again.\n[cli] ERROR: Login failed, cannot continue!", false));
        assert!(expiry::legendary_refused("[Core] ERROR: errors.com.epicgames.account.auth_token.invalid_refresh_token", false));
        // legendary deleted user.json during the command: it only does that when Epic refused the sign-in.
        assert!(expiry::legendary_refused("[cli] ERROR: Login failed, cannot continue!", true));
        // Without a connection: never expired, even if user.json is missing.
        assert!(!expiry::legendary_refused("[cli] ERROR: Login failed, cannot continue!", false));
        assert!(!expiry::legendary_refused("[Core] ERROR: HTTP request for login failed: ConnectionError(MaxRetryError(\"HTTPSConnectionPool(host='account-public-service-prod03.ol.epicgames.com', port=443): Max retries exceeded\")), please try again later.\n[cli] ERROR: Login failed, cannot continue!", true));
        assert!(!expiry::legendary_refused("requests.exceptions.ReadTimeout: HTTPSConnectionPool(...): Read timed out. (read timeout=10)", false));
        assert!(!expiry::legendary_refused("[cli] INFO: Game is up to date", false));
        assert!(!expiry::legendary_refused("", false));
    }
    #[test]
    fn expiry_is_noticed_once() {
        let mut v = json!({"names": {"gog": "geralt"}});
        let rec = expiry::apply_mark(&mut v, "gog", None, 1000).expect("first time");
        assert_eq!(rec, json!({"at": 1000, "name": "geralt"}));               // the remembered name
        assert!(expiry::apply_mark(&mut v, "gog", Some("x".into()), 2000).is_none());   // no second notice
        assert_eq!(v["expired"]["gog"]["at"], 1000);
        assert_eq!(expiry::apply_mark(&mut v, "xbox", Some("Tag".into()), 3000).unwrap()["name"], "Tag");
        assert!(expiry::apply_clear(&mut v, "gog"));
        assert!(!expiry::apply_clear(&mut v, "gog"));
        assert!(expiry::apply_mark(&mut v, "gog", None, 4000).is_some());      // a later expiry is noticed again
        assert!(!expiry::apply_clear(&mut json!({}), "epic"));
    }
}

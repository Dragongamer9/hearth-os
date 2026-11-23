//! Looking for updates (docs/HEARTH-STORE.md, "Finding updates"): only when the Store opens, and at most once a day
//! from the small user timer (hearth-store-check.timer runs `hearth-store-app --check`), which skips it while a game
//! runs or on a metered connection. A few small requests: each app's newest release on GitHub and its SHA256SUMS.txt,
//! and (only when Hearth OS has the owner's key built in) the signed list of apps from hearthos.fun.

use crate::catalog::{self, Catalog, Kind};
use crate::net::{self, Fail};
use crate::release::{self, Rel, Seen};
use serde_json::Value;

pub const CATALOG_URL: &str = "https://hearthos.fun/store/catalog.json";

/// The newest release for one app (and its SHA-256), from GitHub, the list's pin, or CI's pretend releases.
pub fn latest(e: &catalog::Entry, test: bool) -> Result<Option<Rel>, Fail> {
    if let Some(r) = release::from_pin(e) {
        return Ok(Some(r));
    }
    if e.github.is_empty() {
        return Ok(None);
    }
    let answer: Value = match crate::paths::fixture() {
        Some(f) => {
            if f.join("offline").exists() {
                return Err(Fail::Offline);
            }
            let all: Value = std::fs::read(f.join("releases.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
            match all.get(&e.id) {
                Some(v) => v.clone(),
                None => return Ok(None),
            }
        }
        None => match net::fetch(&release::api_url(e, test), 2_000_000, 20) {
            Ok(b) => serde_json::from_slice(&b).map_err(|_| Fail::Other("GitHub's answer couldn't be read".into()))?,
            Err(Fail::Missing) => return Ok(None),
            Err(x) => return Err(x),
        },
    };
    let Some(mut r) = release::pick(e, &answer, test) else { return Ok(None) };
    if r.sha256.is_empty() && !r.sums_url.is_empty() {
        if let Ok(b) = net::fetch(&r.sums_url, 64_000, 20) {
            r.sha256 = release::sum_for(&String::from_utf8_lossy(&b), &r.asset).unwrap_or_default();
        }
    }
    Ok(Some(r))
}

/// Every app's newest release, merged into what was seen before (an app that couldn't be asked keeps what was seen).
pub fn check_all(cat: &Catalog, test: bool) -> Seen {
    let mut seen = release::read_seen();
    let mut asked = 0;
    let mut offline = 0;
    for e in cat.apps.iter().filter(|e| e.kind == Kind::Install) {
        if e.github.is_empty() && e.pin.is_none() {
            continue;
        }
        asked += 1;
        match latest(e, test) {
            Ok(Some(r)) => {
                seen.apps.insert(e.id.clone(), release::to_json(&r));
            }
            Ok(None) => {
                seen.apps.remove(&e.id);
            }
            Err(Fail::Offline) => offline += 1,
            Err(_) => {}
        }
    }
    seen.offline = asked > 0 && offline == asked;
    if !seen.offline {
        seen.checked = crate::files::now();
    }
    seen
}

/// The signed list from hearthos.fun, kept when it's the owner's and newer (catalog::accept_refresh). Asked only when
/// the image has the key. True: a newer list is in use from now on.
pub fn refresh_catalog(cat: &Catalog) -> bool {
    if crate::paths::fixture().is_some() {
        return false;
    }
    let Some(key) = catalog::built_in_key(&crate::paths::signing_key()) else { return false };
    let Ok(text) = net::fetch(CATALOG_URL, 1_000_000, 20) else { return false };
    let Ok(sig) = net::fetch(&format!("{CATALOG_URL}.sig"), 1_000, 20) else { return false };
    catalog::accept_refresh(&key, &text, &String::from_utf8_lossy(&sig), cat.serial).unwrap_or(false)
}

/// Updates ready to tell about: (app name, version, updates itself).
pub fn ready(cat: &Catalog, seen: &Seen) -> Vec<(String, String, bool)> {
    let mut v = Vec::new();
    for e in cat.apps.iter().filter(|e| e.kind == Kind::Install) {
        let (Some(i), Some(r)) = (crate::files::read_installed(&e.id), seen.rel(&e.id)) else { continue };
        if crate::newer(&r.version, &i.version) {
            v.push((e.name.clone(), r.version.clone(), e.self_updates));
        }
    }
    v
}

/// The daily check (`hearth-store-app --check`, from the user timer): nothing while a game runs or on a metered
/// connection, or when "Look for updates" is "Only when the Store opens". One notice per new version, if "Tell me
/// when an update is ready" is on. Writes only the Store's cache. Prints what it did.
pub fn daily() -> i32 {
    let prefs = crate::prefs::load();
    if !prefs.daily {
        println!("hearth-store: looking only when the Store opens (Store settings); nothing done");
        return 0;
    }
    if net::game_running() {
        println!("hearth-store: a game is running; looking later");
        return 0;
    }
    if net::metered() {
        println!("hearth-store: on a metered connection; looking later");
        return 0;
    }
    let mut cat = catalog::load();
    if refresh_catalog(&cat) {
        cat = catalog::load();
    }
    let mut seen = check_all(&cat, prefs.test);
    let mut new_told = Vec::new();
    if prefs.notify {
        for (name, ver, own) in ready(&cat, &seen) {
            let k = format!("{name} {ver}");
            if seen.told.contains(&k) {
                continue;
            }
            let body = if own { format!("Open {name} to update it.") } else { "Open the Hearth Store to update it.".to_string() };
            if net::run_short(5, &["notify-send", "-a", "Hearth Store", "-i", "system-software-update", &format!("{name} {ver} is ready"), &body]).is_some() {
                new_told.push(k);
            }
        }
    }
    seen.told.extend(new_told);
    // "Install updates by themselves" (store-w11-6; off unless the person turned it on): Hearth parts' quick updates
    // only (each signed and checked by hearth-part, the built-in one kept to fall back on), never test versions, never
    // while a game runs (checked above), and the person is told what changed.
    if prefs.auto && !seen.offline {
        let (hearth, _) = crate::this_pc();
        for e in cat.apps.iter().filter(|e| !e.test) {
            let Some(p) = e.part.as_ref() else { continue };
            let Some(up) = crate::parts::offer(p, crate::parts::have(&p.id).as_ref(), &hearth) else { continue };
            match crate::parts::install(up) {
                Ok(()) => {
                    println!("hearth-store: {} updated to {} by itself", e.name, up.version);
                    let _ = net::run_short(5, &["notify-send", "-a", "Hearth Store", "-i", "system-software-update", &format!("{} is on {}", e.name, up.version), "Updated by itself (Store settings › Install updates by themselves)."]);
                }
                Err(err) => println!("hearth-store: {} {} didn't install: {err}", e.name, up.version),
            }
        }
    }
    let _ = release::write_seen(&seen);
    println!("hearth-store: checked {} apps{}", seen.apps.len(), if seen.offline { " (offline)" } else { "" });
    0
}

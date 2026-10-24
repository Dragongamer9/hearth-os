//! Store sign-ins kept through updates. An update must never sign anyone out, so at every start Hearth:
//!  1. puts back a store's sign-in that went missing, from wherever it still is on this device (an unfinished
//!     profile switch, Hearth's own sign-in backups, ~/.local/share/nimbus when Hearth's data folder is elsewhere,
//!     the ~/.old-hearth-<date> folder a clean install moved things to), unless you signed out of it yourself;
//!  2. backs the sign-ins up (when they changed since the last backup), before anything else this version does;
//!  3. says in its log, per store, whether it's signed in and why (file names only, never what's in them).
//! Anything that changes sign-in storage (switching profiles, restoring a Hearth backup, Hearth setting Steam's
//! account, signing out) backs them up first too.
//!
//! The backups: ~/.local/share/nimbus/signin-backup/<time>/, folders 0700 and files 0600, the newest few kept. They
//! stay on this device: Hearth's backup files (backup.rs SKIP), Hearth Link and "Send logs" never include them, and
//! no token, cookie or account name is ever logged. Steam's own sign-in stays Steam's: Hearth never reads or copies
//! Steam's saved sign-in (local.vdf, config.vdf); only the two files that say which account signs in
//! (registry.vdf, loginusers.vdf, no secrets in them) are copied before Hearth itself changes them.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Each store's sign-in files in Hearth's data folder; the first one is the sign-in itself.
pub const FILES: [(&str, &[&str]); 3] = [
    ("epic", &["legendary/user.json"]),
    ("gog", &["gog/auth.json", "gog/profile.json"]),
    ("xbox", &["accounts/xbox.bin"]),
];
/// Hearth's other small files about sign-ins (expired sign-ins, sign-outs), backed up with them.
const EXTRA: [&str; 2] = ["accounts/expired.json", "accounts/signed-out.json"];
pub const BACKUP_DIR: &str = "signin-backup";
const KEEP: usize = 8;
const SIGNED_OUT: &str = "accounts/signed-out.json";
/// Hearth's Tauri identifier: up to 0.6.4 every sign-in window used this app's own WebKit folder
/// (~/.local/share/io.github.dragongamer9.hearth, its "cookies" file).
pub const LEGACY_WEB_ID: &str = "io.github.dragongamer9.hearth";

fn key_file(store: &str) -> Option<&'static str> { FILES.iter().find(|(s, _)| *s == store).map(|(_, f)| f[0]) }
fn read_json(p: &Path) -> Value { std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null) }
fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }
fn log(text: &str) { crate::bridge::log(&format!("sign-ins: {text}")) }

/// Whether these bytes are a usable sign-in for the store (shape only: the right keys, never their values).
pub fn usable(store: &str, bytes: &[u8]) -> bool {
    let has = |v: &Value, k: &str| v[k].as_str().map(|s| !s.is_empty()).unwrap_or(false);
    match store {
        "epic" => serde_json::from_slice::<Value>(bytes).map(|v| has(&v, "refresh_token") || has(&v, "access_token")).unwrap_or(false),
        "gog" => serde_json::from_slice::<Value>(bytes).ok().and_then(|v| v.as_object().cloned())
            .map(|o| o.values().any(|t| has(t, "refresh_token") || has(t, "access_token"))).unwrap_or(false),
        "xbox" => bytes.len() > 2 && &bytes[..2] == b"P1" && serde_json::from_slice::<Value>(&bytes[2..]).map(|v| has(&v, "refreshToken")).unwrap_or(false),
        _ => false,
    }
}

/// A store's sign-in in this data folder: (signed in, why), for the log and the tests. Never reads out a value.
pub fn state(data: &Path, store: &str) -> (bool, String) {
    let Some(rel) = key_file(store) else { return (false, "not a store Hearth signs in to".into()) };
    let f = data.join(rel);
    let bytes = match std::fs::read(&f) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return (false, match signed_out_at(data, store) { Some(t) => format!("signed out in Hearth ({}), no {rel}", when(t)), None => format!("no {rel}") });
        }
        Err(e) => return (false, format!("{rel} is there but can't be read ({}): check who owns it", e.kind())),
    };
    if !usable(store, &bytes) {
        if store == "xbox" && bytes.len() > 1 && &bytes[..2] != b"P1" { return (false, format!("{rel} is locked with the desktop keyring (the Electron host's form): sign in once more")); }
        return (false, format!("{rel} is there but isn't a sign-in ({} bytes)", bytes.len()));
    }
    let expired = read_json(&data.join("accounts/expired.json"))["expired"][store]["at"].as_u64();
    match expired {
        Some(t) => (true, format!("{rel}, but the store refused it {}: Sign in again", when(t))),
        None => (true, rel.to_string()),
    }
}

fn when(ms: u64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_millis_opt(ms as i64).single().map(|t| t.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| ms.to_string())
}

// ------------------------------------------------------------------ signed out on purpose

/// When you signed out of a store in Hearth (so a sign-in found elsewhere isn't put back), if you did.
pub fn signed_out_at(data: &Path, store: &str) -> Option<u64> { read_json(&data.join(SIGNED_OUT))[store].as_u64() }
fn set_signed_out(data: &Path, store: &str, at: Option<u64>) {
    let f = data.join(SIGNED_OUT);
    let mut v = read_json(&f);
    if !v.is_object() { v = json!({}); }
    match at { Some(t) => { v[store] = json!(t); } None => { if v.get(store).is_none() { return; } v.as_object_mut().unwrap().remove(store); } }
    let _ = write_private(&f, v.to_string().as_bytes());
}
/// Signing out in Hearth (stores::sign_out).
pub fn mark_signed_out(store: &str) { set_signed_out(&crate::paths::data_dir(), store, Some(now_ms())); }
/// Signed in again.
pub fn clear_signed_out(store: &str) { set_signed_out(&crate::paths::data_dir(), store, None); }

// ------------------------------------------------------------------ private files

#[cfg(unix)]
fn private_dir(p: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(p)?;
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700))
}
/// A file only you can read (0600), written all at once.
#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    if let Some(d) = path.parent() { std::fs::create_dir_all(d)?; }
    let tmp = path.with_file_name(format!(".{}.{}.tmp", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), std::process::id()));
    let r = (|| {
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
        f.write_all(bytes)?;
        drop(f);
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        std::fs::rename(&tmp, path)
    })();
    if r.is_err() { let _ = std::fs::remove_file(&tmp); }
    r
}

#[cfg(not(unix))]
fn private_dir(p: &Path) -> std::io::Result<()> { std::fs::create_dir_all(p) }
#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> { crate::paths::write_atomic(path, bytes) }

/// A private (0600) file, written all at once (storefront.rs: a store's carried-over web sign-in).
pub fn write_private_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> { write_private(path, bytes) }

// ------------------------------------------------------------------ backups

/// What goes in a backup: (name in the backup, file on this device). The sign-in files, Hearth's notes about them,
/// the store windows' web sign-ins (their cookie files only, not their caches) and, up to 0.6.4's layout, the app's
/// own WebKit cookie file. `steam`: also Steam's two "who signs in" files (before Hearth changes them).
pub fn backup_items(data: &Path, legacy_web: Option<&Path>, steam: Option<(Option<PathBuf>, PathBuf)>) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = Vec::new();
    for (_, files) in FILES { for f in files.iter() { v.push((f.to_string(), data.join(f))); } }
    for f in EXTRA { v.push((f.to_string(), data.join(f))); }
    if let Ok(rd) = std::fs::read_dir(data.join("accounts/web")) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.ends_with(".id") { v.push((format!("accounts/web/{n}"), e.path())); continue; }
            for c in ["webkit/cookies", "webkit/cookies.sqlite"] { v.push((format!("accounts/web/{n}/{c}"), e.path().join(c))); }
        }
    }
    if let Some(l) = legacy_web { v.push(("web-legacy/cookies".into(), l.join("cookies"))); }
    if let Some((reg, users)) = steam {
        if let Some(r) = reg { v.push(("steam/registry.vdf".into(), r)); }
        v.push(("steam/loginusers.vdf".into(), users));
    }
    v.into_iter().filter(|(_, p)| p.is_file()).collect()
}

/// Backups, newest first: (folder, its notes {at, why, version, profile, files}).
pub fn backups(data: &Path) -> Vec<(PathBuf, Value)> {
    let mut out: Vec<(PathBuf, Value)> = std::fs::read_dir(data.join(BACKUP_DIR)).into_iter().flatten().flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| { let m = read_json(&e.path().join("backup.json")); (e.path(), m) })
        .filter(|(_, m)| m["at"].is_u64())
        .collect();
    out.sort_by(|a, b| b.1["at"].as_u64().cmp(&a.1["at"].as_u64()).then(b.0.cmp(&a.0)));
    out
}

/// Copies the sign-ins into a new backup, unless they're the same as in the newest one. Returns its folder.
pub fn backup_in(data: &Path, items: &[(String, PathBuf)], why: &str, version: &str, profile: &str) -> Option<PathBuf> {
    if items.is_empty() { return None; }
    let read: Vec<(String, Vec<u8>)> = items.iter().filter_map(|(n, p)| std::fs::read(p).ok().map(|b| (n.clone(), b))).collect();
    if read.is_empty() { return None; }
    let root = data.join(BACKUP_DIR);
    if let Some((last, meta)) = backups(data).into_iter().next() {
        let same_files = meta["files"].as_array().map(|f| f.len() == read.len()).unwrap_or(false);
        if same_files && meta["profile"] == profile && read.iter().all(|(n, b)| std::fs::read(last.join(n)).ok().as_deref() == Some(b.as_slice())) { return None; }
    }
    let at = now_ms();
    let dir = root.join(format!("{at}-{}", std::process::id()));
    if private_dir(&root).is_err() || private_dir(&dir).is_err() { log("couldn't make a backup folder"); return None; }
    for (n, b) in &read {
        let dst = dir.join(n);
        if let Some(p) = dst.parent() { let _ = private_dir(p); }
        if write_private(&dst, b).is_err() { log(&format!("couldn't back up {n}")); }
    }
    let meta = json!({"at": at, "why": why, "version": version, "profile": profile, "files": read.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>()});
    let _ = write_private(&dir.join("backup.json"), meta.to_string().as_bytes());
    // The newest few stay.
    for (old, _) in backups(data).into_iter().skip(KEEP) { if old.starts_with(&root) { let _ = std::fs::remove_dir_all(old); } }
    Some(dir)
}

/// The app's own WebKit folder (~/.local/share/io.github.dragongamer9.hearth): every sign-in window's up to 0.6.4.
pub fn legacy_web_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| crate::paths::home().join(".local/share")).join(LEGACY_WEB_ID)
}
fn active_profile(data: &Path) -> String { crate::profiles::Profiles::open(data).active() }

/// Backs up the sign-ins now (before something that changes them). `why` goes in the backup's notes and the log.
pub fn backup(why: &str) {
    let data = crate::paths::data_dir();
    let items = backup_items(&data, Some(&legacy_web_dir()), None);
    if let Some(d) = backup_in(&data, &items, why, env!("CARGO_PKG_VERSION"), &active_profile(&data)) {
        log(&format!("backed up {} file(s) ({why}) in {}", items.len(), d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    }
}
/// Before Hearth writes Steam's registry.vdf and loginusers.vdf (profiles::set_steam_account).
pub fn backup_with_steam(why: &str, reg: Option<PathBuf>, users: PathBuf) {
    let data = crate::paths::data_dir();
    let items = backup_items(&data, Some(&legacy_web_dir()), Some((reg, users)));
    if let Some(d) = backup_in(&data, &items, why, env!("CARGO_PKG_VERSION"), &active_profile(&data)) {
        log(&format!("backed up {} file(s) ({why}) in {}", items.len(), d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    }
}

// ------------------------------------------------------------------ putting missing sign-ins back

/// Where a missing sign-in may still be, in order: (what to call it in the log, a folder laid out like Hearth's
/// data folder). Only this person's: their profile's own folder (a switch that didn't finish leaves their files
/// there), their backups, Hearth's data folder in its usual place when $XDG_DATA_HOME points elsewhere, and the
/// folders a clean install moved things into.
pub fn sources(data: &Path, home: &Path, profile: &str) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = Vec::new();
    if !profile.is_empty() && profile.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        v.push(("a profile switch that didn't finish".into(), data.join("profiles").join(profile)));
    }
    for (dir, meta) in backups(data) {
        if meta["profile"].as_str().unwrap_or("main") == profile { v.push((format!("the sign-in backup from {}", when(meta["at"].as_u64().unwrap_or(0))), dir)); }
    }
    let usual = home.join(".local/share/nimbus");
    let same = |a: &Path, b: &Path| std::fs::canonicalize(a).ok().zip(std::fs::canonicalize(b).ok()).map(|(x, y)| x == y).unwrap_or(a == b);
    if !same(&usual, data) { v.push(("~/.local/share/nimbus".into(), usual)); }
    let mut old: Vec<PathBuf> = std::fs::read_dir(home).into_iter().flatten().flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(".old-hearth-")).map(|e| e.path()).collect();
    old.sort();
    old.reverse();
    for o in old {
        let name = o.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        v.push((format!("~/{name} (a clean install moved it there)"), o.join(".local/share/nimbus")));
    }
    v
}

/// Puts back each store's sign-in that's missing (or unreadable as a sign-in) from the first source that still has
/// a usable one. Not a store you signed out of in Hearth, nor one the store refused (it would be refused again).
/// Returns what was put back: (store, where from).
pub fn restore_missing_in(data: &Path, home: &Path, profile: &str) -> Vec<(String, String)> {
    let mut done = Vec::new();
    let expired = read_json(&data.join("accounts/expired.json"));
    for (store, files) in FILES {
        if state(data, store).0 || signed_out_at(data, store).is_some() || expired["expired"][store].is_object() { continue; }
        for (label, src) in sources(data, home, profile) {
            let Ok(bytes) = std::fs::read(src.join(files[0])) else { continue };
            if !usable(store, &bytes) { continue; }
            if write_private(&data.join(files[0]), &bytes).is_err() { continue; }
            // The store's other files (GOG's profile name and picture) come along when they're missing here.
            for f in files.iter().skip(1) {
                if !data.join(f).exists() { if let Ok(b) = std::fs::read(src.join(f)) { let _ = write_private(&data.join(f), &b); } }
            }
            done.push((store.to_string(), label));
            break;
        }
    }
    done
}

/// At every start, before anything else (Bridge::new): missing sign-ins put back, a backup when they changed, one
/// line per store in the log.
pub fn at_start(version: &str) {
    let data = crate::paths::data_dir();
    let home = crate::paths::home();
    let profile = active_profile(&data);
    for (store, from) in restore_missing_in(&data, &home, &profile) { log(&format!("{store}: the sign-in was missing; put back from {from}")); }
    let items = backup_items(&data, Some(&legacy_web_dir()), None);
    if let Some(d) = backup_in(&data, &items, &format!("start of Hearth {version}"), version, &profile) {
        log(&format!("backed up {} file(s) in {}", items.len(), d.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    }
    for line in report(&data, &home) { log(&line); }
}

/// One line per store: signed in or not, and why (file names, counts and dates only).
pub fn report(data: &Path, home: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for (store, _) in FILES {
        let (yes, why) = state(data, store);
        out.push(format!("signed in to {store}: {} ({why})", if yes { "yes" } else { "no" }));
    }
    // Steam keeps its own sign-in; Hearth only says whether its files name an account.
    out.push(match crate::steam::steam_path(home) {
        None => format!("signed in to steam: unknown ({})", crate::steam::no_folder_reason(home)),
        Some(steam) => match crate::steam::find_account(home, &steam, !crate::steam::several_profiles()) {
            Ok((_, how)) => format!("signed in to steam: yes, as far as Steam's files say (found by {how}; Steam itself keeps the sign-in)"),
            Err(e) => format!("signed in to steam: no account Steam signs in as ({e})"),
        },
    });
    let web: Vec<String> = FILES.iter().map(|(s, _)| {
        let id = std::fs::read_to_string(data.join(format!("accounts/web/{s}.id"))).unwrap_or_default();
        let has = !id.trim().is_empty() && data.join(format!("accounts/web/{s}-{}/webkit/cookies", id.trim())).is_file();
        format!("{s} {}", if has { "yes" } else { "no" })
    }).collect();
    out.push(format!("store windows' web sign-ins: {}", web.join(", ")));
    out.push(format!("backups of the sign-ins: {} (in {BACKUP_DIR}, private, never uploaded)", backups(data).len()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPIC: &str = r#"{"access_token": "zz-epic-access-SECRET", "refresh_token": "zz-epic-refresh-SECRET", "account_id": "a1", "displayName": "Zqxplayer"}"#;
    const GOG: &str = r#"{"46899977096215655": {"access_token": "zz-gog-access-SECRET", "refresh_token": "zz-gog-refresh-SECRET", "expires_in": 3600, "loginTime": 1}}"#;
    const XBOX: &str = r#"P1{"refreshToken": "zz-xbox-refresh-SECRET", "gamertag": "Zqxtag"}"#;

    fn put(p: &Path, s: &str) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, s).unwrap(); }
    #[cfg(unix)]
    fn mode(p: &Path) -> u32 { use std::os::unix::fs::PermissionsExt; std::fs::metadata(p).unwrap().permissions().mode() & 0o777 }

    /// A home folder as Hearth 0.6.3 left it: Epic, GOG and Xbox signed in, the sign-in windows' WebKit cookies in
    /// the app's own folder, Steam (Debian's steam-installer, as on Peppermint) signed in.
    fn era_063(home: &Path) -> PathBuf {
        let data = home.join(".local/share/nimbus");
        put(&data.join("legendary/user.json"), EPIC);
        put(&data.join("gog/auth.json"), GOG);
        put(&data.join("gog/profile.json"), r#"{"name": "Zqxgog", "avatar": null}"#);
        put(&data.join("accounts/xbox.bin"), XBOX);
        put(&data.join("library.json"), r#"{"favorites": []}"#);
        put(&home.join(".local/share").join(LEGACY_WEB_ID).join("cookies"),
            "# Netscape HTTP Cookie File\n.gog.com\tTRUE\t/\tTRUE\t1999999999\tgog-al\tzz-gog-cookie-SECRET\n#HttpOnly_.epicgames.com\tTRUE\t/\tTRUE\t1999999999\tEPIC_SSO\tzz-epic-cookie-SECRET\n.login.live.com\tTRUE\t/\tTRUE\t1999999999\tMSPAuth\tzz-ms-cookie-SECRET\n.example.org\tTRUE\t/\tFALSE\t1999999999\tother\tx\n");
        let deb = home.join(".steam/debian-installation");
        put(&deb.join("config/loginusers.vdf"), r#""users" { "76561197960265729" { "AccountName" "zqxsam" "MostRecent" "1" "Timestamp" "1700000000" } }"#);
        put(&deb.join("steamapps/libraryfolders.vdf"), r#""libraryfolders" { }"#);
        put(&home.join(".steam/registry.vdf"), r#""Registry" { "HKCU" { "Software" { "Valve" { "Steam" { "AutoLoginUser" "zqxsam" } } } } }"#);
        data
    }

    #[test]
    fn knows_a_sign_in_by_its_shape_only() {
        assert!(usable("epic", EPIC.as_bytes()) && usable("gog", GOG.as_bytes()) && usable("xbox", XBOX.as_bytes()));
        assert!(!usable("epic", b"{}") && !usable("gog", b"{\"1\": {}}") && !usable("xbox", b"K1abc") && !usable("xbox", b"P1{}") && !usable("steam", EPIC.as_bytes()));
    }

    /// The update from 0.6.3: every store is still signed in afterwards, nothing in the report gives a secret away,
    /// and the first start backs everything up privately.
    #[test]
    fn an_update_from_063_keeps_every_store_signed_in() {
        let h = tempfile::tempdir().unwrap();
        let data = era_063(h.path());
        let before: Vec<Vec<u8>> = FILES.iter().map(|(_, f)| std::fs::read(data.join(f[0])).unwrap()).collect();
        // What a start of the new version does to them (no source needed: nothing is missing).
        assert!(restore_missing_in(&data, h.path(), "main").is_empty());
        let items = backup_items(&data, Some(&h.path().join(".local/share").join(LEGACY_WEB_ID)), None);
        let b = backup_in(&data, &items, "start of Hearth test", "test", "main").expect("a first backup");
        for (i, (store, f)) in FILES.iter().enumerate() {
            assert!(state(&data, store).0, "{store} signed in");
            assert_eq!(std::fs::read(data.join(f[0])).unwrap(), before[i], "{store}'s file unchanged");
            assert_eq!(std::fs::read(b.join(f[0])).unwrap(), before[i], "{store}'s file backed up");
        }
        assert!(b.join("web-legacy/cookies").is_file());
        #[cfg(unix)]
        {
            assert_eq!(mode(&data.join(BACKUP_DIR)), 0o700);
            assert_eq!(mode(&b), 0o700);
            assert_eq!(mode(&b.join("gog/auth.json")), 0o600);
            assert_eq!(mode(&b.join("backup.json")), 0o600);
        }
        // The same sign-ins again: no second backup.
        assert!(backup_in(&data, &items, "again", "test", "main").is_none());
        let report = report(&data, h.path()).join("\n");
        for s in ["epic: yes", "gog: yes", "xbox: yes", "steam: yes"] { assert!(report.contains(s), "{report}"); }
        for secret in ["SECRET", "zqx", "Zqx", "76561197960265729"] { assert!(!report.contains(secret), "{report}"); }
        let meta = std::fs::read_to_string(b.join("backup.json")).unwrap();
        assert!(!meta.contains("SECRET"), "{meta}");
    }

    /// Sign-ins gone from Hearth's data folder come back from wherever they still are, but never one you signed
    /// out of, one the store refused, or another person's.
    #[test]
    fn missing_sign_ins_come_back() {
        let h = tempfile::tempdir().unwrap();
        let data = era_063(h.path());
        let items = backup_items(&data, None, None);
        backup_in(&data, &items, "start", "test", "main").unwrap();
        for (_, f) in FILES { std::fs::remove_file(data.join(f[0])).unwrap(); }
        std::fs::remove_file(data.join("gog/profile.json")).unwrap();
        let back = restore_missing_in(&data, h.path(), "main");
        assert_eq!(back.len(), 3, "{back:?}");
        for (store, _) in FILES { assert!(state(&data, store).0, "{store} back"); }
        assert!(data.join("gog/profile.json").is_file(), "GOG's name and picture too");
        #[cfg(unix)]
        assert_eq!(mode(&data.join("gog/auth.json")), 0o600);

        // Signed out on purpose, or refused by the store: left signed out.
        std::fs::remove_file(data.join("legendary/user.json")).unwrap();
        set_signed_out(&data, "epic", Some(1));
        std::fs::remove_file(data.join("gog/auth.json")).unwrap();
        put(&data.join("accounts/expired.json"), r#"{"expired": {"gog": {"at": 1, "name": null}}}"#);
        assert!(restore_missing_in(&data, h.path(), "main").is_empty());
        assert!(state(&data, "epic").1.starts_with("signed out in Hearth"), "{}", state(&data, "epic").1);
        // Signing in again clears the mark.
        set_signed_out(&data, "epic", None);
        assert_eq!(restore_missing_in(&data, h.path(), "main").len(), 1);

        // Another profile's backup is never used for this one.
        let h2 = tempfile::tempdir().unwrap();
        let d2 = era_063(h2.path());
        backup_in(&d2, &backup_items(&d2, None, None), "start", "test", "kid").unwrap();
        std::fs::remove_file(d2.join("legendary/user.json")).unwrap();
        assert!(restore_missing_in(&d2, h2.path(), "main").is_empty());
    }

    /// From the other places a sign-in can be left: Hearth's usual folder when $XDG_DATA_HOME points elsewhere, an
    /// unfinished profile switch, a clean install's ~/.old-hearth-<date>.
    #[test]
    fn found_where_they_were_left() {
        let h = tempfile::tempdir().unwrap();
        era_063(h.path());
        let elsewhere = h.path().join("xdg/nimbus");
        std::fs::create_dir_all(&elsewhere).unwrap();
        let back = restore_missing_in(&elsewhere, h.path(), "main");
        assert_eq!(back.iter().map(|(s, _)| s.as_str()).collect::<Vec<_>>(), vec!["epic", "gog", "xbox"]);
        assert!(back.iter().all(|(_, f)| f == "~/.local/share/nimbus"), "{back:?}");

        let h = tempfile::tempdir().unwrap();
        let data = h.path().join(".local/share/nimbus");
        put(&data.join("profiles/main/gog/auth.json"), GOG);
        put(&h.path().join(".old-hearth-2026-10-01/.local/share/nimbus/legendary/user.json"), EPIC);
        let back = restore_missing_in(&data, h.path(), "main");
        assert_eq!(back.len(), 2, "{back:?}");
        assert_eq!((back[0].0.as_str(), back[1].0.as_str()), ("epic", "gog"));
        assert!(back[0].1.contains(".old-hearth-2026-10-01") && back[1].1.contains("profile switch"), "{back:?}");
    }

    /// Only the newest backups stay.
    #[test]
    fn old_backups_go() {
        let h = tempfile::tempdir().unwrap();
        let data = era_063(h.path());
        for i in 0..(KEEP + 3) {
            put(&data.join("gog/profile.json"), &format!(r#"{{"name": "n{i}"}}"#));
            backup_in(&data, &backup_items(&data, None, None), "t", "test", "main").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(3));
        }
        assert_eq!(backups(&data).len(), KEEP);
    }
}

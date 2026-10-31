//! Hearth Recomps: Hearth's own store for native PC ports of console games (N64Recomp projects like Zelda 64:
//! Recompiled, Xbox 360 recompilations like Unleashed Recompiled, decompilation ports like Ship of Harkinian).
//! Inspired by RecompStore (elripalda, GPL-3.0, a PS5 homebrew store), RecompLauncher and SirDiabo's GithubLauncher
//! (formerly N64RecompLauncher); written from scratch for Hearth, with its own catalogue (docs/RECOMPS.md).
//!
//! - The catalogue: os/system_files/usr/share/hearth/recomps.json (in the image), built in here too, refreshed once a
//!   day from Hearth OS's own GitHub repo, plus GitHub projects you add yourself (marked "added by you").
//! - Each port comes from its own GitHub releases (the public REST API, no sign-in: answers are kept for 6 hours, an
//!   unchanged answer is asked with its ETag, and GitHub's hourly limit is respected).
//! - Installs go to ~/.local/share/nimbus/recomps/<id>/app (or a folder on a USB drive or SD card). Updates are copied
//!   over the old version: files the port made (saves, mods, settings, your game file) are never deleted or replaced.
//! - You supply your own game file (a ROM or disc image) with Hearth's file picker. Hearth checks it against the hash
//!   the project documents or checks itself, and puts it where the port looks. Hearth never downloads game files.

use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The catalogue as this version of Hearth was built with it.
pub const BUILTIN: &str = include_str!("../../../os/system_files/usr/share/hearth/recomps.json");
const SYSTEM: &str = "/usr/share/hearth/recomps.json";
/// Newer catalogues come from Hearth OS's own repository (its main branch: what was released).
const REFRESH_URL: &str = "https://raw.githubusercontent.com/Dragongamer9/hearth-os/main/os/system_files/usr/share/hearth/recomps.json";
const API: &str = "https://api.github.com/repos";
/// How long a project's releases are kept before GitHub is asked again.
const KEEP_MS: u64 = 6 * 3600 * 1000;
/// Until when GitHub said its hourly limit is used up (ms since 1970; 0: not limited).
static LIMITED_UNTIL: AtomicU64 = AtomicU64::new(0);
/// Files Hearth makes in a folder it installed into (only such folders are ever deleted whole).
const MARKER: &str = ".hearth-recomps";

pub fn root() -> PathBuf { crate::paths::data_dir().join("recomps") }
fn now() -> u64 { crate::library::now_ms() }
fn id_ok(id: &str) -> bool { !id.is_empty() && id.len() <= 80 && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') }
pub fn repo_ok(r: &str) -> bool {
    let mut p = r.split('/');
    let ok = |s: Option<&str>| s.map(|s| !s.is_empty() && s.len() <= 100 && s != "." && s != ".." && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))).unwrap_or(false);
    ok(p.next()) && ok(p.next()) && p.next().is_none()
}
fn read_json(p: &Path) -> Value { std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null) }
fn write_json(p: &Path, v: &Value) -> Result<(), String> {
    if let Some(d) = p.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    crate::paths::write_atomic(p, serde_json::to_string_pretty(v).unwrap_or_default().as_bytes()).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- the catalogue

/// A catalogue file that's safe to use: entries with a good id, a title and a GitHub owner/repo.
pub fn valid_catalogue(v: &Value) -> bool {
    let Some(list) = v["recomps"].as_array() else { return false };
    !list.is_empty() && list.iter().all(|e| e["id"].as_str().map(id_ok).unwrap_or(false) && e["title"].as_str().map(|t| !t.is_empty()).unwrap_or(false)
        && e["repo"].as_str().map(repo_ok).unwrap_or(false))
}
/// The newest of the catalogues Hearth has (built in, the image's, the one refreshed from GitHub), by its date.
pub fn catalogue() -> Value {
    let mut best: Value = serde_json::from_str(BUILTIN).unwrap_or(json!({"recomps": []}));
    best["source"] = json!("built in");
    for (p, src) in [(PathBuf::from(SYSTEM), "Hearth OS"), (root().join("catalogue.json"), "refreshed")] {
        let v = read_json(&p);
        if valid_catalogue(&v) && v["updated"].as_str().unwrap_or("") > best["updated"].as_str().unwrap_or("") { best = v; best["source"] = json!(src); }
    }
    // Projects you added yourself (GitHub owner/repo), after the catalogue's.
    let custom = read_json(&root().join("custom.json"));
    if let (Some(list), Some(extra)) = (best["recomps"].as_array_mut(), custom.as_array()) {
        for e in extra { if e["id"].as_str().map(id_ok).unwrap_or(false) && !list.iter().any(|x| x["id"] == e["id"]) { list.push(e.clone()); } }
    }
    best
}
pub fn entry(id: &str) -> Option<Value> { catalogue()["recomps"].as_array()?.iter().find(|e| e["id"] == id).cloned() }

/// A GitHub project you add yourself: owner/repo, or its github.com address. Marked as added by you (not checked).
pub fn custom_entry(input: &str, title: Option<&str>) -> Result<Value, String> {
    let t = input.trim().trim_end_matches('/').trim_end_matches(".git");
    let repo = t.strip_prefix("https://github.com/").or_else(|| t.strip_prefix("http://github.com/")).or_else(|| t.strip_prefix("github.com/")).unwrap_or(t);
    let repo: String = repo.split('/').take(2).collect::<Vec<_>>().join("/");
    if !repo_ok(&repo) { return Err("Type the project as owner/name, like Zelda64Recomp/Zelda64Recomp".into()); }
    let slug: String = repo.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let name = title.map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or_else(|| repo.split('/').nth(1).unwrap_or("").to_string());
    Ok(json!({"id": format!("custom-{}", slug.trim_matches('-')).chars().take(80).collect::<String>(), "title": name.chars().filter(|c| !c.is_control()).take(80).collect::<String>(),
        "game": "", "console": "other", "kind": "custom", "repo": repo, "linux": null, "needs": null, "files": [], "setup": "in-app", "custom": true}))
}
pub fn add_custom(input: &str, title: Option<&str>) -> Result<Value, String> {
    let e = custom_entry(input, title)?;
    if catalogue()["recomps"].as_array().map(|l| l.iter().any(|x| x["repo"].as_str().map(|r| r.eq_ignore_ascii_case(e["repo"].as_str().unwrap_or(""))).unwrap_or(false))).unwrap_or(false) {
        return Err("That project is in Hearth Recomps already".into());
    }
    // It has to have releases to install from (one question to GitHub).
    let (rel, _) = releases(e["repo"].as_str().unwrap_or(""), true)?;
    if rel.is_empty() { return Err("That project has no releases on GitHub to install".into()); }
    let p = root().join("custom.json");
    let mut list = read_json(&p).as_array().cloned().unwrap_or_default();
    list.push(e.clone());
    write_json(&p, &Value::Array(list))?;
    Ok(e)
}
pub fn remove_custom(id: &str) -> Result<(), String> {
    if installed_state(id).is_some() { return Err("Uninstall it first".into()); }
    let p = root().join("custom.json");
    let list: Vec<Value> = read_json(&p).as_array().cloned().unwrap_or_default().into_iter().filter(|e| e["id"] != id).collect();
    write_json(&p, &Value::Array(list))
}

/// Gets a newer catalogue from Hearth OS's repository (at most once a day; a bad or older one is ignored).
pub fn refresh_catalogue(force: bool) {
    let stamp = root().join("catalogue.checked");
    let last: u64 = std::fs::read_to_string(&stamp).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    if !force && now().saturating_sub(last) < 24 * 3600 * 1000 { return; }
    let _ = std::fs::create_dir_all(root());
    let _ = std::fs::write(&stamp, now().to_string());
    let Ok(r) = crate::http::get(REFRESH_URL, &[]) else { return };
    if !r.ok() { return; }
    let v = r.json();
    if valid_catalogue(&v) && v["updated"].as_str().unwrap_or("") > catalogue()["updated"].as_str().unwrap_or("") {
        let _ = write_json(&root().join("catalogue.json"), &v);
        crate::bridge::log(&format!("recomps: newer catalogue ({})", v["updated"].as_str().unwrap_or("")));
    }
}

// ---------------------------------------------------------------- GitHub releases

/// curl's answer with its headers (-D -): (status, headers in lower case, body).
pub fn split_reply(out: &str) -> (u16, HashMap<String, String>, String) {
    let (rest, code) = out.rsplit_once('\n').unwrap_or((out, "0"));
    let mut text = rest;
    let mut headers = HashMap::new();
    // A redirect or "100 Continue" gives more than one header block: the last one is the answer's.
    while text.starts_with("HTTP/") {
        let (head, body) = match text.find("\r\n\r\n") { Some(i) => (&text[..i], &text[i + 4..]), None => match text.find("\n\n") { Some(i) => (&text[..i], &text[i + 2..]), None => (text, "") } };
        headers.clear();
        for l in head.lines().skip(1) { if let Some((k, v)) = l.split_once(':') { headers.insert(k.trim().to_lowercase(), v.trim().to_string()); } }
        text = body;
    }
    (code.trim().parse().unwrap_or(0), headers, text.to_string())
}

/// A release, kept small: {tag, name, notes, published, prerelease, url, assets: [{name, size, url, digest}]}.
pub fn parse_releases(v: &Value) -> Vec<Value> {
    v.as_array().map(|a| a.iter().filter(|r| r["draft"] != true && r["tag_name"].is_string()).map(|r| {
        let notes: String = r["body"].as_str().unwrap_or("").chars().take(20000).collect();
        let assets: Vec<Value> = r["assets"].as_array().map(|a| a.iter().map(|x| json!({"name": x["name"], "size": x["size"].as_u64().unwrap_or(0),
            "url": x["browser_download_url"], "digest": x["digest"]})).collect()).unwrap_or_default();
        json!({"tag": r["tag_name"], "name": r["name"].as_str().filter(|s| !s.trim().is_empty()).unwrap_or(r["tag_name"].as_str().unwrap_or("")),
            "notes": notes, "published": r["published_at"], "prerelease": r["prerelease"] == true, "url": r["html_url"], "assets": assets})
    }).collect()).unwrap_or_default()
}
/// The release to install by default: the newest that isn't a pre-release, or the newest when there are only those.
pub fn latest(rel: &[Value]) -> Option<Value> { rel.iter().find(|r| r["prerelease"] != true).or(rel.first()).cloned() }

/// When GitHub's hourly limit for this connection is used up: Some(time it resets, ms).
pub fn limited_until() -> Option<u64> { let u = LIMITED_UNTIL.load(Ordering::SeqCst); if u > now() { Some(u) } else { None } }
fn limited_msg(until: u64) -> String {
    let t = chrono::DateTime::from_timestamp_millis(until as i64).map(|d| d.with_timezone(&chrono::Local).format("%H:%M").to_string()).unwrap_or_default();
    format!("GitHub only answers so many questions an hour without a sign-in, and that's used up. Try again after {t}.")
}

/// A project's releases (newest first) and whether they're from earlier (GitHub couldn't be asked). Kept for 6 hours
/// unless `fresh`; a kept answer is asked again with its ETag, which GitHub doesn't count towards the limit.
pub fn releases(repo: &str, fresh: bool) -> Result<(Vec<Value>, bool), String> {
    if !repo_ok(repo) { return Err("That isn't a GitHub project".into()); }
    let cache = root().join("cache").join(format!("{}.json", repo.replace('/', "__")));
    let kept = read_json(&cache);
    let have = kept["releases"].as_array().cloned();
    if let Some(r) = &have { if !fresh && now().saturating_sub(kept["fetched"].as_u64().unwrap_or(0)) < KEEP_MS { return Ok((r.clone(), false)); } }
    if let Some(u) = limited_until() { return have.map(|r| (r, true)).ok_or_else(|| limited_msg(u)); }
    let url = format!("{API}/{repo}/releases?per_page=20");
    let mut headers = vec![("Accept", "application/vnd.github+json"), ("X-GitHub-Api-Version", "2022-11-28")];
    let etag = kept["etag"].as_str().unwrap_or("").to_string();
    if have.is_some() && !etag.is_empty() { headers.push(("If-None-Match", etag.as_str())); }
    let cfg = crate::http::curl_config(&url, &headers, None);
    let out = crate::system::run_input("curl", &["-sS", "-m", "30", "-A", "Hearth", "-D", "-", "-w", "\n%{http_code}", "-K", "-"], Some(&cfg), Duration::from_secs(40));
    let Ok(out) = out else { return have.map(|r| (r, true)).ok_or_else(|| "No connection to GitHub".to_string()) };
    let (code, h, body) = split_reply(&out);
    let save = |rel: &Vec<Value>, etag: &str| { let _ = write_json(&cache, &json!({"fetched": now(), "etag": etag, "releases": rel})); };
    match code {
        200 => {
            let rel = parse_releases(&serde_json::from_str(&body).unwrap_or(Value::Null));
            save(&rel, h.get("etag").map(String::as_str).unwrap_or(""));
            Ok((rel, false))
        }
        304 => { let r = have.unwrap_or_default(); save(&r, &etag); Ok((r, false)) }
        403 | 429 if h.get("x-ratelimit-remaining").map(|v| v == "0").unwrap_or(true) || code == 429 => {
            let reset = h.get("x-ratelimit-reset").and_then(|v| v.parse::<u64>().ok()).map(|s| s * 1000).unwrap_or(now() + 15 * 60 * 1000);
            LIMITED_UNTIL.store(reset.max(now() + 60_000), Ordering::SeqCst);
            crate::bridge::log(&format!("recomps: GitHub's hourly limit reached (until {reset})"));
            have.map(|r| (r, true)).ok_or_else(|| limited_msg(reset))
        }
        404 => Err("GitHub doesn't know this project any more (it may have moved or been taken down).".into()),
        _ => have.map(|r| (r, true)).ok_or_else(|| format!("GitHub didn't answer (HTTP {code})")),
    }
}

// ---------------------------------------------------------------- which download

/// What a release file is: (platform "linux" | "windows", arch "x86_64" | "aarch64", how good a pick it is), or None
/// for files that aren't a build for a PC (sources, checksums, macOS, Android, Flatpak bundles, 32-bit, installers).
pub fn classify(name: &str) -> Option<(&'static str, &'static str, u8)> {
    let n = name.to_lowercase();
    const SKIP: [&str; 30] = ["source", "-src", "_src", "symbols", "debug", ".pdb", ".sha", ".md5", ".sig", ".asc", ".txt", ".json", ".apk", ".aab", ".dmg",
        ".pkg", ".ipa", "macos", "mac-", "-mac", "_mac", "osx", "darwin", "android", "ios.", "flatpak", ".deb", ".rpm", "wasm", ".msi"];
    if SKIP.iter().any(|s| n.contains(s)) || n.ends_with(".html") || n.contains("-ios") || n.contains("_ios") { return None; }
    let kind = if n.ends_with(".appimage") { 5 } else if n.contains("appimage") { 4 }
        else if n.ends_with(".tar.gz") || n.ends_with(".tgz") || n.ends_with(".tar.xz") || n.ends_with(".tar.bz2") || n.ends_with(".tar.zst") { 3 }
        else if n.ends_with(".zip") { 3 } else if n.ends_with(".7z") { 2 } else if n.ends_with(".exe") { 1 } else { return None };
    let linux = n.contains("linux") || n.contains("appimage") || n.contains("steamdeck") || n.contains("steam-deck") || n.contains("steam_deck") || n.contains("ubuntu");
    let windows = !linux && (n.contains("windows") || n.contains("win64") || n.contains("-win") || n.contains("_win") || n.contains("win-") || n.contains("win_") || n.starts_with("win") || n.ends_with(".exe"));
    let platform = if linux { "linux" } else if windows { "windows" } else { return None };
    let arch = if n.contains("arm64") || n.contains("aarch64") || n.contains("arm-64") { "aarch64" }
        else if n.contains("win32") || n.contains("i686") || n.contains("i386") || n.contains("x86_32") || (n.contains("x86") && !n.contains("x86_64") && !n.contains("x86-64")) { return None }
        else { "x86_64" };
    // The Steam Deck builds of a port work everywhere, but the plain build is the one made for any PC.
    let deck = n.contains("deck");
    Some((platform, arch, kind * 2 - if deck { 1 } else { 0 }))
}
/// The file to download from a release for this PC: a Linux build if there's one for its processor, otherwise the
/// Windows build (x86_64 PCs only), which runs with Proton. An entry can name its files with a pattern of its own.
pub fn pick_asset(assets: &[Value], arch: &str, entry: &Value) -> Option<(Value, &'static str)> {
    let custom = |plat: &str| entry["assets"][plat].as_str().and_then(|p| regex::Regex::new(p).ok());
    let mut best: Option<(u8, Value, &'static str)> = None;
    for a in assets {
        let name = a["name"].as_str().unwrap_or("");
        let Some((plat, ar, score)) = classify(name) else { continue };
        if let Some(re) = custom(plat) { if !re.is_match(name) { continue; } }
        let fits = ar == arch || (plat == "windows" && arch == "x86_64" && ar == "x86_64");
        if !fits { continue; }
        let s = if plat == "linux" { 100 + score } else { score };
        if best.as_ref().map(|b| s > b.0).unwrap_or(true) { best = Some((s, a.clone(), plat)); }
    }
    best.map(|(_, a, p)| (a, p))
}
pub fn arch() -> &'static str { if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" } }

// ---------------------------------------------------------------- what's installed

fn state_file(id: &str) -> PathBuf { root().join(id).join("hearth.json") }
/// An installed port's state: {version, asset, platform, dir, exe, files, installed, game, pinned, skipped, adopted}.
pub fn installed_state(id: &str) -> Option<Value> {
    if !id_ok(id) { return None; }
    let s = read_json(&state_file(id));
    if s["dir"].is_string() { Some(s) } else { None }
}
fn save_state(id: &str, s: &Value) -> Result<(), String> { write_json(&state_file(id), s) }
fn update_state(id: &str, f: impl FnOnce(&mut Value)) -> Result<Value, String> {
    let mut s = read_json(&state_file(id));
    if !s.is_object() { s = json!({}); }
    f(&mut s);
    save_state(id, &s)?;
    Ok(s)
}
fn prefs() -> Value { let p = read_json(&root().join("prefs.json")); if p.is_object() { p } else { json!({}) } }
pub fn set_hidden(id: &str, on: bool) -> Result<(), String> {
    let mut p = prefs();
    let mut list: Vec<Value> = p["hidden"].as_array().cloned().unwrap_or_default();
    list.retain(|x| x != id);
    if on { list.push(json!(id)); }
    p["hidden"] = Value::Array(list);
    write_json(&root().join("prefs.json"), &p)
}

/// The program the library starts: Hearth's start script for Linux builds, the .exe itself for Windows builds.
fn launcher_of(id: &str, s: &Value) -> Option<String> {
    let exe = s["exe"].as_str()?;
    let dir = s["dir"].as_str()?;
    let full = Path::new(dir).join(exe);
    if !full.exists() { return None; }
    if s["platform"] == "windows" { Some(full.to_string_lossy().to_string()) } else {
        let run = root().join(id).join("run.sh");
        if run.exists() { Some(run.to_string_lossy().to_string()) } else { None }
    }
}
fn icon_file(id: &str) -> PathBuf { root().join(id).join("icon.png") }

/// The library's entries: each installed port with Play (Library › Recomps).
pub fn games() -> Vec<crate::library::Game> {
    let cat = catalogue();
    let mut out = vec![];
    for e in std::fs::read_dir(root()).into_iter().flatten().flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        let Some(s) = installed_state(&id) else { continue };
        let Some(exe) = launcher_of(&id, &s) else { continue };
        let ent = cat["recomps"].as_array().and_then(|l| l.iter().find(|x| x["id"] == id.as_str()).cloned()).unwrap_or(Value::Null);
        let title = ent["title"].as_str().or(s["title"].as_str()).unwrap_or(&id).to_string();
        let mut art = vec![];
        let icon = icon_file(&id);
        if icon.exists() { let i = icon.to_string_lossy().to_string(); art.push(("cover", i.clone())); art.push(("icon", i)); }
        let args = if s["platform"] == "windows" { ent["args"].as_str().unwrap_or("").to_string() } else { String::new() };
        out.push(crate::library::Game { id: format!("recomp:{id}"), title, source: "recomps", install_path: s["dir"].as_str().map(PathBuf::from),
            installed: true, state: "installed", exe: Some(exe), args, art, ..Default::default() });
    }
    out
}

// ---------------------------------------------------------------- install folders

/// Places to install to: this device, and each USB drive, SD card or second drive that's mounted: [{label, path, free}]
pub fn places() -> Vec<Value> {
    let free = |p: &Path| crate::system::run("df", &["-B1", "--output=avail", &p.to_string_lossy()]).ok()
        .and_then(|o| o.lines().nth(1).and_then(|l| l.trim().parse::<u64>().ok())).unwrap_or(0);
    let _ = std::fs::create_dir_all(root());
    let mut out = vec![json!({"label": "This device", "path": "", "free": free(&root())})];
    let user = std::env::var("USER").unwrap_or_default();
    for base in [PathBuf::from("/run/media").join(&user), PathBuf::from("/media").join(&user)] {
        for d in std::fs::read_dir(&base).into_iter().flatten().flatten() {
            let p = d.path();
            if p.is_dir() { out.push(json!({"label": d.file_name().to_string_lossy(), "path": p.join("Hearth Recomps").to_string_lossy(), "free": free(&p)})); }
        }
    }
    out
}
/// Where a port installs: its own folder in Hearth's data, or <drive>/Hearth Recomps/<id> on a drive from `places`.
pub fn install_dir(id: &str, place: Option<&str>) -> Result<PathBuf, String> {
    match place.map(str::trim).filter(|p| !p.is_empty()) {
        None => Ok(root().join(id).join("app")),
        Some(p) => {
            let pp = Path::new(p);
            if p.contains("..") || !(pp.starts_with("/run/media") || pp.starts_with("/media")) { return Err("Pick one of the places offered".into()); }
            Ok(pp.join(id))
        }
    }
}

// ---------------------------------------------------------------- unpacking and copying

fn unpack(file: &Path, into: &Path) -> Result<(), String> {
    let n = file.file_name().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
    let f = file.to_string_lossy().to_string();
    let to = into.to_string_lossy().to_string();
    let long = Duration::from_secs(900);
    std::fs::create_dir_all(into).map_err(|e| e.to_string())?;
    let r = if n.ends_with(".appimage") || n.ends_with(".exe") {
        std::fs::copy(file, into.join(file.file_name().unwrap())).map(|_| String::new()).map_err(|e| e.to_string())
    } else if n.ends_with(".zip") {
        if crate::system::has("unzip") { crate::system::run_timeout("unzip", &["-q", "-o", &f, "-d", &to], long) }
        else if crate::system::has("bsdtar") { crate::system::run_timeout("bsdtar", &["-xf", &f, "-C", &to], long) }
        else { crate::system::run_timeout("python3", &["-m", "zipfile", "-e", &f, &to], long) }
    } else if n.ends_with(".7z") {
        if crate::system::has("7z") { crate::system::run_timeout("7z", &["x", "-y", &format!("-o{to}"), &f], long) }
        else if crate::system::has("7za") { crate::system::run_timeout("7za", &["x", "-y", &format!("-o{to}"), &f], long) }
        else if crate::system::has("bsdtar") { crate::system::run_timeout("bsdtar", &["-xf", &f, "-C", &to], long) }
        else { Err("Unpacking .7z files needs 7-Zip (7z) or bsdtar".into()) }
    } else {
        crate::system::run_timeout("tar", &["-xf", &f, "-C", &to], long)
    };
    r.map(|_| ()).map_err(|e| format!("Couldn't unpack the download: {e}"))?;
    // GitHub Actions builds often come as a zip with a tar inside: unpack that too.
    let inner: Vec<PathBuf> = std::fs::read_dir(into).into_iter().flatten().flatten().map(|e| e.path()).collect();
    if inner.len() == 1 {
        let p = &inner[0];
        let pn = p.file_name().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
        if p.is_file() && (pn.ends_with(".tar") || pn.ends_with(".tar.gz") || pn.ends_with(".tgz") || pn.ends_with(".tar.xz")) {
            crate::system::run_timeout("tar", &["-xf", &p.to_string_lossy(), "-C", &to], long).map_err(|e| format!("Couldn't unpack the download: {e}"))?;
            let _ = std::fs::remove_file(p);
        }
    }
    Ok(())
}
/// The folder whose contents are the port: the unpacked folder, or the one folder inside it.
fn content_root(dir: &Path) -> PathBuf {
    let items: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    if items.len() == 1 && items[0].is_dir() && !items[0].is_symlink() { items[0].clone() } else { dir.to_path_buf() }
}
fn walk(dir: &Path, base: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 12 { return; }
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        let ft = match e.file_type() { Ok(t) => t, Err(_) => continue };
        if ft.is_dir() { walk(&p, base, out, depth + 1); } else if let Ok(rel) = p.strip_prefix(base) { out.push(rel.to_path_buf()); }
    }
}
/// Folders a port keeps your own things in: an update never replaces a file in them that's already there.
fn users_folder(rel: &Path) -> bool {
    let first = rel.components().next().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).unwrap_or_default();
    ["saves", "save", "savedata", "saved", "mods", "dlc", "config", "userdata", "user"].contains(&first.as_str())
}
/// Copies a new version over the installed one. Files of the old version are replaced; anything else already there
/// (saves, mods, settings, your game file, a file you put there with the same name) is kept. Nothing is deleted.
/// Returns the new version's files.
pub fn merge(from: &Path, to: &Path, old_files: &HashSet<String>) -> Result<Vec<String>, String> {
    let mut files = vec![];
    walk(from, from, &mut files, 0);
    let mut out = vec![];
    for rel in files {
        let key = rel.to_string_lossy().to_string();
        let src = from.join(&rel);
        let dst = to.join(&rel);
        let exists = dst.exists() || dst.is_symlink();
        let ours = old_files.contains(&key);
        if exists && !old_files.is_empty() {
            if !ours { continue; }                              // yours (a save, a setting): kept, and stays yours
            if users_folder(&rel) { out.push(key); continue; }  // the release's, but in a folder of yours: kept
        }
        if let Some(d) = dst.parent() { std::fs::create_dir_all(d).map_err(|e| format!("Couldn't make {}: {e}", d.display()))?; }
        if dst.is_dir() && !dst.is_symlink() { continue; }
        if exists { let _ = std::fs::remove_file(&dst); }
        if std::fs::rename(&src, &dst).is_err() {
            if src.is_symlink() {
                let t = std::fs::read_link(&src).map_err(|e| e.to_string())?;
                std::os::unix::fs::symlink(t, &dst).map_err(|e| e.to_string())?;
            } else {
                std::fs::copy(&src, &dst).map_err(|e| format!("Couldn't copy {}: {e}", rel.display()))?;
            }
        }
        out.push(key);
    }
    Ok(out)
}

/// Programs in an installed port that could be the game, best first (AppImages, then programs near the top whose
/// name is like the port's), as paths inside its folder.
pub fn executables(dir: &Path, platform: &str, hint: &str) -> Vec<String> {
    let mut files = vec![];
    walk(dir, dir, &mut files, 9);
    let hint: String = hint.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let bad = ["unins", "crash", "redist", "vc_", "dxsetup", "setup", "updater", "chrome-sandbox", "sandbox", "helper", "7z", "upx"];
    let mut c: Vec<(i64, String)> = files.into_iter().filter_map(|rel| {
        let name = rel.file_name()?.to_string_lossy().to_lowercase();
        if bad.iter().any(|b| name.contains(b)) { return None; }
        let full = dir.join(&rel);
        let depth = rel.components().count() as i64;
        let score = if platform == "windows" {
            if !name.ends_with(".exe") { return None; }
            0
        } else if name.ends_with(".appimage") { 50 } else if name.ends_with(".sh") && depth == 1 { 5 } else {
            if name.contains(".so") || name.ends_with(".dll") || name.ends_with(".exe") { return None; }
            let mut head = [0u8; 4];
            let ok = std::fs::File::open(&full).and_then(|mut f| f.read_exact(&mut head)).is_ok();
            if !ok || head != [0x7f, b'E', b'L', b'F'] { return None; }
            20
        };
        let stem: String = name.split('.').next().unwrap_or("").chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        let like = !stem.is_empty() && !hint.is_empty() && (hint.contains(&stem) || stem.contains(&hint));
        Some((score + if like { 30 } else { 0 } - depth * 4, rel.to_string_lossy().to_string()))
    }).collect();
    c.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    c.into_iter().map(|x| x.1).collect()
}

fn sh_quote(s: &str) -> String { format!("'{}'", s.replace('\'', "'\\''")) }
/// Hearth's start script for a Linux build: in the port's folder (ports look for their files there), AppImages
/// without FUSE unpacked as they start, and the port's arguments.
pub fn run_script(title: &str, dir: &str, exe: &str, args: &str) -> String {
    let a = crate::manual::split_args(args).iter().map(|x| sh_quote(x)).collect::<Vec<_>>().join(" ");
    format!("#!/bin/sh\n# Starts {} for Hearth (made by Hearth Recomps; replaced on each install or update).\ncd {} || exit 1\n\
        if ! command -v fusermount >/dev/null 2>&1 && ! command -v fusermount3 >/dev/null 2>&1; then export APPIMAGE_EXTRACT_AND_RUN=1; fi\n\
        exec {} {}\"$@\"\n", title.replace('\n', " "), sh_quote(dir), sh_quote(&format!("./{exe}")), if a.is_empty() { String::new() } else { format!("{a} ") })
}
fn write_run(id: &str, title: &str, s: &Value, args: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let (Some(dir), Some(exe)) = (s["dir"].as_str(), s["exe"].as_str()) else { return Ok(()) };
    if s["platform"] == "windows" { return Ok(()); }
    let full = Path::new(dir).join(exe);
    if let Ok(m) = std::fs::metadata(&full) { let mut p = m.permissions(); p.set_mode(p.mode() | 0o755); let _ = std::fs::set_permissions(&full, p); }
    let run = root().join(id).join("run.sh");
    std::fs::write(&run, run_script(title, dir, exe, args)).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&run, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- your game file

/// N64 ROMs come in three byte orders (.z64, .v64, .n64): turned into .z64 (what the ports and their hashes use).
/// False when it isn't an N64 ROM.
pub fn n64_normalize(d: &mut [u8]) -> bool {
    if d.len() < 4 { return false; }
    match [d[0], d[1], d[2], d[3]] {
        [0x80, 0x37, 0x12, 0x40] => true,
        [0x37, 0x80, 0x40, 0x12] => { for c in d.chunks_exact_mut(2) { c.swap(0, 1); } true }
        [0x40, 0x12, 0x37, 0x80] => { for c in d.chunks_exact_mut(4) { c.reverse(); } true }
        _ => false,
    }
}
fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

/// The hashes of a game file that an entry documents ({"sha1": [...], "md5": [...], "sha256": [...], "xxh3": [...]}):
/// which one matched, or Err with what's wrong. N64 ROMs are checked as .z64 whatever their byte order, and XXH3 the
/// way N64Recomp checks it (the .z64 padded to whole 4 bytes).
pub fn check_file(path: &Path, hashes: &Value, n64: bool, progress: &dyn Fn(f64)) -> Result<Option<String>, String> {
    let want: Vec<(&str, Vec<String>)> = ["xxh3", "sha1", "md5", "sha256"].iter().filter_map(|k| {
        let l: Vec<String> = hashes[*k].as_array()?.iter().filter_map(|v| v.as_str().map(|s| s.trim().to_lowercase())).collect();
        if l.is_empty() { None } else { Some((*k, l)) }
    }).collect();
    if want.is_empty() { return Ok(None); }
    let size = std::fs::metadata(path).map_err(|e| format!("Can't read that file: {e}"))?.len();
    let got = if n64 && size <= 128 << 20 {
        let mut d = std::fs::read(path).map_err(|e| format!("Can't read that file: {e}"))?;
        if !n64_normalize(&mut d) { return Err("That isn't a Nintendo 64 ROM".into()); }
        progress(0.5);
        hash_all(&want, &mut std::io::Cursor::new(d), 0, &|_| {})?
    } else {
        let f = std::fs::File::open(path).map_err(|e| format!("Can't read that file: {e}"))?;
        hash_all(&want, &mut std::io::BufReader::with_capacity(1 << 20, f), size, progress)?
    };
    for (k, l) in &want { if let Some(h) = got.get(*k) { if l.contains(h) { return Ok(Some(k.to_string())); } } }
    Err("This copy doesn't match the version the port needs".into())
}
fn hash_all(want: &[(&str, Vec<String>)], r: &mut dyn Read, size: u64, progress: &dyn Fn(f64)) -> Result<HashMap<String, String>, String> {
    use sha2::Digest;
    let on = |k: &str| want.iter().any(|w| w.0 == k);
    let mut s1 = sha1_smol::Sha1::new();
    let mut m5 = md5::Md5::new();
    let mut s256 = sha2::Sha256::new();
    let mut x3 = xxhash_rust::xxh3::Xxh3::new();
    let mut buf = vec![0u8; 1 << 20];
    let (mut done, mut last) = (0u64, 0u64);
    loop {
        let n = r.read(&mut buf).map_err(|e| format!("Can't read that file: {e}"))?;
        if n == 0 { break; }
        let b = &buf[..n];
        if on("sha1") { s1.update(b); }
        if on("md5") { m5.update(b); }
        if on("sha256") { s256.update(b); }
        if on("xxh3") { x3.update(b); }
        done += n as u64;
        if size > 0 && done - last > 64 << 20 { last = done; progress(done as f64 / size as f64); }
    }
    // N64Recomp pads the ROM to whole 4 bytes before it hashes it.
    if on("xxh3") && done % 4 != 0 { x3.update(&vec![0u8; (4 - done % 4) as usize]); }
    let mut out = HashMap::new();
    if on("sha1") { out.insert("sha1".into(), s1.digest().to_string()); }
    if on("md5") { out.insert("md5".into(), hex(&m5.finalize())); }
    if on("sha256") { out.insert("sha256".into(), hex(&s256.finalize())); }
    if on("xxh3") { out.insert("xxh3".into(), format!("{:016x}", x3.digest())); }
    Ok(out)
}

fn config_home() -> PathBuf { std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| crate::paths::home().join(".config")) }
/// Where an N64Recomp port keeps the ROM it checked: ~/.config/<program>/<game id>.z64. With it there, the port
/// starts without asking for the ROM (it checks it again itself, and asks if it doesn't match).
pub fn n64_rom_path(e: &Value) -> Option<PathBuf> {
    let p = e["n64"]["programId"].as_str()?;
    let g = e["n64"]["gameId"].as_str()?;
    let safe = |s: &str| !s.is_empty() && !s.contains('/') && !s.contains("..");
    if !safe(p) || !safe(g) { return None; }
    Some(config_home().join(p).join(format!("{g}.z64")))
}
/// The files a .cue sheet names (its tracks), beside it.
pub fn cue_tracks(cue: &str) -> Vec<String> {
    cue.lines().filter_map(|l| {
        let l = l.trim();
        if !l.to_uppercase().starts_with("FILE ") { return None; }
        let rest = l[5..].trim();
        let name = if let Some(r) = rest.strip_prefix('"') { r.split('"').next()? } else { rest.split_whitespace().next()? };
        if name.contains('/') || name.contains("..") { None } else { Some(name.to_string()) }
    }).collect()
}
/// Puts your game file where the port looks for it: in its folder (as a link: nothing is copied), under the name or
/// in the folder the port documents. A file you put there yourself is left alone.
pub fn place_beside(e: &Value, app: &Path, file: &Path) -> Result<Vec<PathBuf>, String> {
    let name = e["besideName"].as_str().unwrap_or("");
    if name.contains("..") || name.starts_with('/') { return Err("Bad catalogue entry".into()); }
    let mut files = vec![file.to_path_buf()];
    if file.extension().map(|x| x.to_string_lossy().eq_ignore_ascii_case("cue")).unwrap_or(false) {
        let dir = file.parent().unwrap_or(Path::new("/"));
        for t in cue_tracks(&std::fs::read_to_string(file).unwrap_or_default()) { files.push(dir.join(t)); }
    }
    let mut made = vec![];
    for (i, f) in files.iter().enumerate() {
        let fname = f.file_name().ok_or("Bad file")?;
        let dst = if name.is_empty() || name.ends_with('/') { app.join(name).join(fname) } else if i == 0 { app.join(name) } else { app.join(Path::new(name).parent().unwrap_or(Path::new(""))).join(fname) };
        if let Some(d) = dst.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
        if dst.is_symlink() { let _ = std::fs::remove_file(&dst); } else if dst.exists() { continue; }
        std::os::unix::fs::symlink(f, &dst).map_err(|e| format!("Couldn't put it in the port's folder: {e}"))?;
        made.push(dst);
    }
    Ok(made)
}

// ---------------------------------------------------------------- shortcuts and backups

fn desktop_file(id: &str) -> PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| crate::paths::home().join(".local/share")).join("applications").join(format!("hearth-recomp-{id}.desktop"))
}
fn desktop_quote(s: &str) -> String { format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('`', "\\`").replace('$', "\\$")) }
/// A menu entry for Desktop Mode (and other launchers; Steam's "Add a non-Steam game" lists it too).
pub fn desktop_entry(title: &str, game: &str, launcher: &str, windows: bool, dir: &str, icon: Option<&str>) -> String {
    let exec = if windows { format!("umu-run {}", desktop_quote(launcher)) } else { desktop_quote(launcher) };
    let clean = |s: &str| s.chars().filter(|c| !c.is_control()).collect::<String>();
    format!("[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec={exec}\nPath={}\nIcon={}\nCategories=Game;\nX-Hearth-Recomp=true\n",
        clean(title), clean(if game.is_empty() { "Hearth Recomps" } else { game }), clean(dir), icon.map(clean).unwrap_or_else(|| "applications-games".into()))
}
pub fn set_shortcut(id: &str, on: bool) -> Result<bool, String> {
    let f = desktop_file(id);
    if !on { let _ = std::fs::remove_file(&f); return Ok(false); }
    let s = installed_state(id).ok_or("Install it first")?;
    let e = entry(id).unwrap_or(Value::Null);
    let launcher = launcher_of(id, &s).ok_or("Its program is missing: install it again")?;
    let icon = icon_file(id);
    let text = desktop_entry(e["title"].as_str().unwrap_or(id), e["game"].as_str().unwrap_or(""), &launcher, s["platform"] == "windows",
        s["dir"].as_str().unwrap_or(""), if icon.exists() { icon.to_str() } else { None });
    if let Some(d) = f.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
    std::fs::write(&f, text).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Your own files in an installed port's folder: everything that didn't come with the release (saves, mods,
/// settings), leaving out links to your game file.
pub fn user_files(dir: &Path, release: &HashSet<String>) -> Vec<String> {
    let mut files = vec![];
    walk(dir, dir, &mut files, 0);
    files.into_iter().map(|p| p.to_string_lossy().to_string()).filter(|p| !release.contains(p) && p != MARKER && !dir.join(p).is_symlink()).collect()
}
/// Backs up your saves, mods and settings: the port's own folder in ~/.config (N64Recomp ports, without the ROM), and
/// your files in its install folder. Returns the archive (in ~/.local/share/nimbus/recomps/backups).
pub fn backup(id: &str) -> Result<String, String> {
    let s = installed_state(id).ok_or("It isn't installed")?;
    let e = entry(id).unwrap_or(Value::Null);
    let out = root().join("backups");
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let stamp = chrono::Local::now().format("%Y-%m-%d-%H%M%S");
    let file = out.join(format!("{id}-{stamp}.tar.gz"));
    let release: HashSet<String> = s["files"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let dir = PathBuf::from(s["dir"].as_str().unwrap_or(""));
    let mine = user_files(&dir, &release);
    let mut list = String::new();
    for f in &mine { list.push_str(&format!("app/{f}\n")); }
    // A folder with links to what to keep, so one archive holds both places.
    let stage = root().join(id).join("backup-stage");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
    let _ = std::os::unix::fs::symlink(&dir, stage.join("app"));
    if let Some(cfg) = n64_rom_path(&e).and_then(|p| p.parent().map(Path::to_path_buf)).filter(|p| p.is_dir()) {
        let _ = std::os::unix::fs::symlink(&cfg, stage.join("config"));
        list.push_str("config/\n");
    }
    if list.is_empty() { let _ = std::fs::remove_dir_all(&stage); return Err("There's nothing of yours to back up yet".into()); }
    let r = crate::system::run_input("tar", &["-czhf", &file.to_string_lossy(), "--exclude=*.z64", "-C", &stage.to_string_lossy(), "-T", "-"], Some(&list), Duration::from_secs(600));
    let _ = std::fs::remove_dir_all(&stage);
    r.map_err(|e| format!("Couldn't back it up: {e}"))?;
    Ok(file.to_string_lossy().to_string())
}

/// Uninstalls: a folder Hearth made is deleted (saves in it too, so `keep` backs them up first); a folder you pointed
/// Hearth at ("I have it already") is only forgotten. An N64Recomp port's saves in ~/.config always stay.
pub fn uninstall(id: &str, keep: bool) -> Result<Option<String>, String> {
    let s = installed_state(id).ok_or("It isn't installed")?;
    let kept = if keep { backup(id).ok() } else { None };
    let dir = PathBuf::from(s["dir"].as_str().unwrap_or(""));
    if s["adopted"] != true && dir.join(MARKER).exists() && dir.ends_with(id) || dir == root().join(id).join("app") {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("Couldn't remove it: {e}"))?;
    }
    let _ = std::fs::remove_file(desktop_file(id));
    for f in ["hearth.json", "run.sh", "icon.png"] { let _ = std::fs::remove_file(root().join(id).join(f)); }
    let _ = std::fs::remove_dir_all(root().join(id).join("download"));
    Ok(kept)
}

// ---------------------------------------------------------------- the jobs (installing, checking your copy)

/// Installs, updates and checks run in the background and tell the page how they're going ("recomps" events).
pub struct Recomps {
    pub jobs: Arc<Mutex<Map<String, Value>>>,
    updates: Arc<Mutex<(u64, Map<String, Value>)>>,
    cancel: Arc<Mutex<HashSet<String>>>,
}
impl Default for Recomps { fn default() -> Self { Self::new() } }

fn emit_job(emit: &crate::system::bt_scan::Emit, jobs: &Arc<Mutex<Map<String, Value>>>, id: &str) {
    let j = jobs.lock().unwrap().get(id).cloned().unwrap_or(Value::Null);
    emit(json!({"type": "event", "name": "recomps", "data": {"id": id, "job": j}}));
}
fn set_job(jobs: &Arc<Mutex<Map<String, Value>>>, id: &str, f: impl FnOnce(&mut Value)) {
    let mut m = jobs.lock().unwrap();
    let j = m.entry(id.to_string()).or_insert_with(|| json!({}));
    f(j);
}

impl Recomps {
    pub fn new() -> Recomps { Recomps { jobs: Arc::new(Mutex::new(Map::new())), updates: Arc::new(Mutex::new((0, Map::new()))), cancel: Arc::new(Mutex::new(HashSet::new())) } }

    fn busy(&self, id: &str) -> bool { self.jobs.lock().unwrap().get(id).map(|j| j["busy"] == true).unwrap_or(false) }

    /// The catalogue with what's installed: {recomps: [entry + {installed, version, platform, update, job, ...}],
    /// consoles, updated, source, limitedUntil, arch}. "copy" is your game file: {path, verified, hash, placed}.
    pub fn list(&self) -> Value {
        let mut cat = catalogue();
        let hidden = prefs()["hidden"].clone();
        let jobs = self.jobs.lock().unwrap().clone();
        let ups = self.updates.lock().unwrap().1.clone();
        if let Some(list) = cat["recomps"].as_array_mut() {
            for e in list.iter_mut() {
                let id = e["id"].as_str().unwrap_or("").to_string();
                let s = installed_state(&id);
                e["hidden"] = json!(hidden.as_array().map(|h| h.iter().any(|x| x == id.as_str())).unwrap_or(false));
                e["job"] = jobs.get(&id).cloned().unwrap_or(Value::Null);
                if let Some(s) = s {
                    e["installed"] = json!(launcher_of(&id, &s).is_some());
                    for k in ["version", "platform", "dir", "exe", "pinned", "skipped", "adopted", "asset"] { e[k] = s[k].clone(); }
                    e["copy"] = s["game"].clone();   // your game file ("game" is the game's name)
                    e["shortcut"] = json!(desktop_file(&id).exists());
                    let up = ups.get(&id).cloned().unwrap_or(Value::Null);
                    e["update"] = if s["pinned"].is_string() || up == s["skipped"] || up == s["version"] { Value::Null } else { up };
                    e["icon"] = if icon_file(&id).exists() { json!(crate::library::art_url(&icon_file(&id).to_string_lossy())) } else { e["icon"].clone() };
                } else { e["installed"] = json!(false); }
            }
        }
        cat["limitedUntil"] = json!(limited_until());
        cat["arch"] = json!(arch());
        cat["tools"] = json!({"proton": crate::system::has("umu-run") || crate::system::has("wine")});
        cat
    }

    /// One port's releases, for its page: {releases (newest first), latest, pick (the download for this PC), stale}.
    pub fn details(id: &str, fresh: bool) -> Result<Value, String> {
        let e = entry(id).ok_or("Hearth Recomps doesn't know that one")?;
        let (rel, stale) = releases(e["repo"].as_str().unwrap_or(""), fresh)?;
        let latest = latest(&rel);
        let pick = latest.as_ref().and_then(|r| pick_asset(r["assets"].as_array().map(|a| a.as_slice()).unwrap_or(&[]), arch(), &e))
            .map(|(a, p)| json!({"asset": a, "platform": p}));
        Ok(json!({"id": id, "releases": rel, "latest": latest, "pick": pick, "stale": stale, "limitedUntil": limited_until()}))
    }

    /// Looks for updates to installed ports (at most every 6 hours, one project at a time, stopping at GitHub's
    /// limit), and refreshes the catalogue once a day. In the background: the page hears "recomps" events.
    pub fn check_updates(&self, force: bool, emit: crate::system::bt_scan::Emit) {
        {
            let mut u = self.updates.lock().unwrap();
            if !force && now().saturating_sub(u.0) < KEEP_MS { return; }
            u.0 = now();
        }
        let ups = self.updates.clone();
        std::thread::spawn(move || {
            refresh_catalogue(force);
            let ids: Vec<String> = std::fs::read_dir(root()).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|id| installed_state(id).is_some()).collect();
            for id in ids {
                if limited_until().is_some() { break; }
                let Some(e) = entry(&id) else { continue };
                if let Ok((rel, _)) = releases(e["repo"].as_str().unwrap_or(""), force) {
                    if let Some(l) = latest(&rel) { ups.lock().unwrap().1.insert(id.clone(), l["tag"].clone()); }
                }
                std::thread::sleep(Duration::from_millis(800));
            }
            emit(json!({"type": "event", "name": "recomps", "data": {"updates": true}}));
        });
    }

    pub fn cancel(&self, id: &str) { self.cancel.lock().unwrap().insert(id.to_string()); }

    /// Installs a port, or updates it in place: the release `tag` (default: the newest, or the one you chose to keep),
    /// its file `asset` (default: the best for this PC), into `place` (first install only).
    pub fn install(&self, id: &str, tag: Option<String>, asset: Option<String>, place: Option<String>, emit: crate::system::bt_scan::Emit) -> Result<Value, String> {
        let e = entry(id).ok_or("Hearth Recomps doesn't know that one")?;
        if self.busy(id) { return Ok(json!({"busy": true})); }
        let prev = installed_state(id);
        let dir = match prev.as_ref().and_then(|s| s["dir"].as_str()) { Some(d) => PathBuf::from(d), None => install_dir(id, place.as_deref())? };
        set_job(&self.jobs, id, |j| { *j = json!({"busy": true, "stage": "Asking GitHub for the release…", "progress": null, "error": null}); });
        self.cancel.lock().unwrap().remove(id);
        let (jobs, cancel, id) = (self.jobs.clone(), self.cancel.clone(), id.to_string());
        let ups = self.updates.clone();
        std::thread::spawn(move || {
            emit_job(&emit, &jobs, &id);
            let stage = |t: &str, p: Option<f64>| { set_job(&jobs, &id, |j| { j["stage"] = json!(t); j["progress"] = json!(p); }); emit_job(&emit, &jobs, &id); };
            let r = (|| -> Result<String, String> {
                let repo = e["repo"].as_str().unwrap_or("");
                let (rel, _) = releases(repo, prev.is_some())?;
                let want = tag.clone().or_else(|| prev.as_ref().and_then(|s| s["pinned"].as_str().map(str::to_string)));
                let release = match &want { Some(t) => rel.iter().find(|r| r["tag"] == t.as_str()).cloned().ok_or("That version isn't on GitHub any more")?, None => latest(&rel).ok_or("This project has no releases to install yet")? };
                let assets = release["assets"].as_array().cloned().unwrap_or_default();
                let (a, platform) = match &asset {
                    Some(n) => {
                        let a = assets.iter().find(|x| x["name"] == n.as_str()).cloned().ok_or("That file isn't in the release")?;
                        let plat = classify(n).map(|c| c.0).unwrap_or(if n.to_lowercase().contains("win") { "windows" } else { "linux" });
                        (a, plat)
                    }
                    None => pick_asset(&assets, arch(), &e).ok_or_else(|| format!("{} has no build for this PC in its newest release ({}).", e["title"].as_str().unwrap_or(""), release["tag"].as_str().unwrap_or("")))?,
                };
                if platform == "windows" && !crate::system::has("umu-run") && !crate::system::has("wine") {
                    return Err("This port only has a Windows build, which needs Proton: install umu-launcher (Hearth OS has it) and try again".into());
                }
                let name = a["name"].as_str().unwrap_or("download").replace('/', "_");
                let url = a["url"].as_str().unwrap_or("").to_string();
                if !url.starts_with("https://github.com/") && !url.starts_with("https://objects.githubusercontent.com/") { return Err("That download isn't on GitHub".into()); }
                let size = a["size"].as_u64().unwrap_or(0);
                let work = root().join(&id).join("download");
                let _ = std::fs::remove_dir_all(&work);
                std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
                let file = work.join(&name);
                stage(&format!("Downloading {}…", release["tag"].as_str().unwrap_or("")), Some(0.0));
                let mut child = std::process::Command::new("curl").args(["-sSL", "--fail", "--speed-limit", "1024", "--speed-time", "60", "-A", "Hearth", "-o"]).arg(&file).arg(&url)
                    .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map_err(|e| format!("Couldn't download it: {e}"))?;
                loop {
                    if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
                        if !st.success() { return Err("The download didn't finish. Check the internet connection and try again.".into()); }
                        break;
                    }
                    if cancel.lock().unwrap().remove(&id) { let _ = child.kill(); let _ = child.wait(); return Err("Stopped".into()); }
                    if size > 0 { let got = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0); stage(&format!("Downloading {}…", release["tag"].as_str().unwrap_or("")), Some((got as f64 / size as f64).min(1.0))); }
                    std::thread::sleep(Duration::from_millis(500));
                }
                // Checked: the size GitHub gave, and its SHA-256 when it has one ("digest").
                let got = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
                if size > 0 && got != size { return Err(format!("The download is the wrong size ({got} bytes, not {size}): try again")); }
                if let Some(d) = a["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")) {
                    stage("Checking the download…", None);
                    let h = check_file(&file, &json!({"sha256": [d]}), false, &|_| {});
                    if h.is_err() { return Err("The download doesn't match what GitHub says it should be: try again".into()); }
                }
                stage("Unpacking…", None);
                let unpacked = work.join("unpacked");
                unpack(&file, &unpacked)?;
                let content = content_root(&unpacked);
                stage("Installing…", None);
                std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't make {}: {e}", dir.display()))?;
                if prev.is_none() { let _ = std::fs::write(dir.join(MARKER), "Installed by Hearth Recomps. Uninstall deletes this folder.\n"); }
                let old: HashSet<String> = prev.as_ref().and_then(|s| s["files"].as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
                // An adopted folder (your own copy) has no list of release files: its files are the release's.
                let old = if old.is_empty() && prev.as_ref().map(|s| s["adopted"] == true).unwrap_or(false) { let mut f = vec![]; walk(&dir, &dir, &mut f, 0); f.into_iter().map(|p| p.to_string_lossy().to_string()).collect() } else { old };
                let files = merge(&content, &dir, &old)?;
                let _ = std::fs::remove_dir_all(&work);
                // The program to start: the one chosen before if it's still there, otherwise the best guess.
                let hint = format!("{} {}", e["n64"]["programId"].as_str().unwrap_or(""), repo.split('/').nth(1).unwrap_or(""));
                let keep_exe = prev.as_ref().and_then(|s| s["exe"].as_str()).filter(|x| dir.join(x).exists()).map(str::to_string);
                let exe = keep_exe.or_else(|| executables(&dir, platform, &hint).into_iter().next()).ok_or("Couldn't find the game's program in the download")?;
                let ver = release["tag"].as_str().unwrap_or("").to_string();
                let s = update_state(&id, |s| {
                    s["dir"] = json!(dir.to_string_lossy()); s["version"] = json!(ver); s["asset"] = json!(name); s["platform"] = json!(platform);
                    s["exe"] = json!(exe); s["files"] = json!(files); s["installed"] = json!(now()); s["title"] = e["title"].clone();
                })?;
                write_run(&id, e["title"].as_str().unwrap_or(&id), &s, e["args"].as_str().unwrap_or(""))?;
                // The project's own icon, for the library and the menu entry.
                if let Some(u) = e["icon"].as_str().filter(|u| u.starts_with("https://raw.githubusercontent.com/")) {
                    if !icon_file(&id).exists() { let _ = crate::system::run_timeout("curl", &["-sSL", "--fail", "-m", "30", "-o", &icon_file(&id).to_string_lossy(), u], Duration::from_secs(40)); }
                }
                // Your game file, where the port looks (again: a new version may have a new folder layout).
                if e["setup"] == "beside" { if let Some(g) = s["game"]["path"].as_str() { let _ = place_beside(&e, &dir, Path::new(g)); } }
                if desktop_file(&id).exists() { let _ = set_shortcut(&id, true); }
                ups.lock().unwrap().1.insert(id.clone(), json!(ver.clone()));
                Ok(ver)
            })();
            if let Err(err) = &r { crate::bridge::log(&format!("recomps: {id}: install failed: {err}")); let _ = std::fs::remove_dir_all(root().join(&id).join("download")); }
            else { crate::bridge::log(&format!("recomps: {id}: installed {}", r.as_deref().unwrap_or(""))); }
            set_job(&jobs, &id, |j| { *j = json!({"busy": false, "done": r.is_ok(), "version": r.as_ref().ok(), "error": r.err()}); });
            emit_job(&emit, &jobs, &id);
            crate::stores::notify(json!({"type": "internal", "what": "rescan"}));
        });
        Ok(json!({"busy": true}))
    }

    /// Your game file for a port (from the file picker): checked against the project's hash in the background, then
    /// put where the port looks. A file that doesn't match is refused unless `anyway`.
    pub fn set_game(&self, id: &str, path: &str, anyway: bool, emit: crate::system::bt_scan::Emit) -> Result<Value, String> {
        let e = entry(id).ok_or("Hearth Recomps doesn't know that one")?;
        let p = PathBuf::from(path);
        if !p.is_file() { return Err("Pick a file".into()); }
        if self.busy(id) { return Err("Wait for it to finish first".into()); }
        let exts: Vec<String> = e["files"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_lowercase)).collect()).unwrap_or_default();
        let low = path.to_lowercase();
        if !anyway && !exts.is_empty() && !exts.iter().any(|x| low.ends_with(x.as_str())) {
            return Err(format!("{} needs a {} file", e["title"].as_str().unwrap_or(""), exts.join(", ")));
        }
        set_job(&self.jobs, id, |j| { *j = json!({"busy": true, "stage": "Checking your copy…", "progress": null, "error": null}); });
        let (jobs, id) = (self.jobs.clone(), id.to_string());
        std::thread::spawn(move || {
            emit_job(&emit, &jobs, &id);
            let n64 = e["console"] == "n64";
            let r = (|| -> Result<Value, String> {
                let prog = |f: f64| { set_job(&jobs, &id, |j| { j["progress"] = json!(f); }); emit_job(&emit, &jobs, &id); };
                let matched = match check_file(&p, &e["hash"], n64, &prog) { Ok(m) => m, Err(err) if anyway => { crate::bridge::log(&format!("recomps: {id}: used anyway: {err}")); None } Err(err) => return Err(err) };
                let verified = if matched.is_some() { json!(true) } else if e["hash"].is_object() { json!(false) } else { Value::Null };
                let mut placed = vec![];
                if e["setup"] == "n64recomp" && matched.as_deref() == Some("xxh3") {
                    if let Some(dst) = n64_rom_path(&e) {
                        let mut d = std::fs::read(&p).map_err(|e| e.to_string())?;
                        n64_normalize(&mut d);
                        d.resize((d.len() + 3) & !3, 0);
                        if let Some(dir) = dst.parent() { std::fs::create_dir_all(dir).map_err(|e| e.to_string())?; }
                        crate::paths::write_atomic(&dst, &d).map_err(|e| format!("Couldn't put the ROM where the port keeps it: {e}"))?;
                        placed.push(dst.to_string_lossy().to_string());
                    }
                }
                if e["setup"] == "beside" {
                    if let Some(dir) = installed_state(&id).and_then(|s| s["dir"].as_str().map(PathBuf::from)) {
                        placed.extend(place_beside(&e, &dir, &p)?.into_iter().map(|x| x.to_string_lossy().to_string()));
                    }
                }
                let g = json!({"path": p.to_string_lossy(), "verified": verified, "hash": matched, "placed": placed, "at": now()});
                update_state(&id, |s| { s["game"] = g.clone(); })?;
                Ok(g)
            })();
            set_job(&jobs, &id, |j| { *j = json!({"busy": false, "game": r.as_ref().ok(), "error": r.as_ref().err(), "mismatch": r.as_ref().err().map(|e| e.contains("doesn't match")).unwrap_or(false), "file": p.to_string_lossy()}); });
            emit_job(&emit, &jobs, &id);
        });
        Ok(json!({"busy": true}))
    }
}

/// "I have it already": a folder with the port in it (installed by hand, or by another launcher). Hearth starts it
/// from there and updates it in place, and never deletes that folder.
pub fn adopt(id: &str, folder: &str) -> Result<Value, String> {
    let e = entry(id).ok_or("Hearth Recomps doesn't know that one")?;
    let dir = PathBuf::from(folder);
    if !dir.is_dir() { return Err("Pick the folder the port is in".into()); }
    let hint = e["repo"].as_str().unwrap_or("").split('/').nth(1).unwrap_or("").to_string();
    let (platform, exe) = match executables(&dir, "linux", &hint).into_iter().next() { Some(x) => ("linux", x), None => ("windows", executables(&dir, "windows", &hint).into_iter().next().ok_or("There's no program for the port in that folder")?) };
    let s = update_state(id, |s| { s["dir"] = json!(folder); s["exe"] = json!(exe); s["platform"] = json!(platform); s["version"] = json!(null); s["adopted"] = json!(true); s["files"] = json!([]); s["title"] = e["title"].clone(); })?;
    write_run(id, e["title"].as_str().unwrap_or(id), &s, e["args"].as_str().unwrap_or(""))?;
    Ok(s)
}
/// Which program starts the port (when Hearth guessed wrong, or a port has several).
pub fn set_exe(id: &str, exe: &str) -> Result<Value, String> {
    let s = installed_state(id).ok_or("It isn't installed")?;
    let dir = PathBuf::from(s["dir"].as_str().unwrap_or(""));
    if exe.contains("..") || exe.starts_with('/') || !dir.join(exe).is_file() { return Err("Pick one of the programs offered".into()); }
    let platform = if exe.to_lowercase().ends_with(".exe") { "windows" } else { "linux" };
    let s = update_state(id, |s| { s["exe"] = json!(exe); s["platform"] = json!(platform); })?;
    let e = entry(id).unwrap_or(Value::Null);
    write_run(id, e["title"].as_str().unwrap_or(id), &s, e["args"].as_str().unwrap_or(""))?;
    Ok(s)
}
pub fn exe_choices(id: &str) -> Result<Value, String> {
    let s = installed_state(id).ok_or("It isn't installed")?;
    let dir = PathBuf::from(s["dir"].as_str().unwrap_or(""));
    let mut l = executables(&dir, "linux", "");
    l.extend(executables(&dir, "windows", ""));
    Ok(json!({"current": s["exe"], "choices": l.into_iter().take(30).collect::<Vec<_>>()}))
}
/// Skip this update (until the next one), or keep a version (no updates offered until you let go).
pub fn set_skip(id: &str, tag: Option<&str>) -> Result<Value, String> { update_state(id, |s| { s["skipped"] = json!(tag); }) }
pub fn set_pin(id: &str, tag: Option<&str>) -> Result<Value, String> {
    if installed_state(id).is_none() { return Err("Install it first".into()); }
    update_state(id, |s| { s["pinned"] = json!(tag); })
}
/// Library ids: "recomp:<id>".
pub fn id_of(library_id: &str) -> Option<&str> { library_id.strip_prefix("recomp:") }

#[cfg(test)]
mod tests {
    use super::*;

    fn rel_json() -> Value {
        // Trimmed from GitHub's answer for a release with Linux, Windows, Flatpak and source files (the shape of
        // https://api.github.com/repos/<owner>/<repo>/releases).
        json!([
          {"tag_name": "v1.3.0-rc1", "name": "", "draft": false, "prerelease": true, "published_at": "2026-10-01T00:00:00Z", "html_url": "https://github.com/o/r/releases/tag/v1.3.0-rc1", "body": "Testing", "assets": []},
          {"tag_name": "v1.2.1", "name": "1.2.1", "draft": false, "prerelease": false, "published_at": "2026-09-01T00:00:00Z", "html_url": "https://github.com/o/r/releases/tag/v1.2.1",
           "body": "## Fixes\n- Saves load again", "assets": [
            {"name": "Zelda64Recompiled-Flatpak-X64.zip", "size": 10, "browser_download_url": "https://github.com/o/r/releases/download/v1.2.1/a.zip"},
            {"name": "Zelda64Recompiled-AppImage-ARM64.zip", "size": 11, "browser_download_url": "https://github.com/o/r/releases/download/v1.2.1/b.zip"},
            {"name": "Zelda64Recompiled-AppImage-X64.zip", "size": 12, "browser_download_url": "https://github.com/o/r/releases/download/v1.2.1/c.zip", "digest": "sha256:abc"},
            {"name": "Zelda64Recompiled-Linux-X64.tar.gz", "size": 13, "browser_download_url": "https://github.com/o/r/releases/download/v1.2.1/d.tar.gz"},
            {"name": "Zelda64Recompiled-Windows.zip", "size": 14, "browser_download_url": "https://github.com/o/r/releases/download/v1.2.1/e.zip"},
            {"name": "Source code.tar.gz", "size": 15, "browser_download_url": "https://github.com/o/r/archive/v1.2.1.tar.gz"}]},
          {"tag_name": "v0.9", "draft": true, "assets": []}
        ])
    }

    #[test]
    fn releases_are_read_and_the_newest_stable_is_picked() {
        let r = parse_releases(&rel_json());
        assert_eq!(r.len(), 2, "drafts are left out");
        assert_eq!(r[0]["name"], "v1.3.0-rc1", "a release without a name is called by its tag");
        let l = latest(&r).unwrap();
        assert_eq!(l["tag"], "v1.2.1");
        assert_eq!(l["notes"], "## Fixes\n- Saves load again");
        assert_eq!(l["assets"][2]["digest"], "sha256:abc");
        // Only pre-releases: the newest one.
        assert_eq!(latest(&r[..1]).unwrap()["tag"], "v1.3.0-rc1");
        assert!(latest(&[]).is_none());
    }

    #[test]
    fn the_right_download_for_each_pc() {
        let r = parse_releases(&rel_json());
        let a = r[1]["assets"].as_array().unwrap().clone();
        let (x, p) = pick_asset(&a, "x86_64", &Value::Null).unwrap();
        assert_eq!((x["name"].as_str().unwrap(), p), ("Zelda64Recompiled-AppImage-X64.zip", "linux"));
        let (x, p) = pick_asset(&a, "aarch64", &Value::Null).unwrap();
        assert_eq!((x["name"].as_str().unwrap(), p), ("Zelda64Recompiled-AppImage-ARM64.zip", "linux"));
        // Only a Windows build: through Proton (x86_64 PCs only).
        let win: Vec<Value> = a.iter().filter(|x| x["name"].as_str().unwrap().contains("Windows") || x["name"].as_str().unwrap().contains("Source")).cloned().collect();
        let (x, p) = pick_asset(&win, "x86_64", &Value::Null).unwrap();
        assert_eq!((x["name"].as_str().unwrap(), p), ("Zelda64Recompiled-Windows.zip", "windows"));
        assert!(pick_asset(&win, "aarch64", &Value::Null).is_none());
        // An entry can name its file.
        let (x, _) = pick_asset(&a, "x86_64", &json!({"assets": {"linux": "Linux-X64\\.tar\\.gz$"}})).unwrap();
        assert_eq!(x["name"], "Zelda64Recompiled-Linux-X64.tar.gz");
        // Names seen on the catalogue's projects.
        assert_eq!(classify("SoH-Blair-Echo-Linux.zip").map(|c| c.0), Some("linux"));
        assert_eq!(classify("starship.AppImage").map(|c| c.0), Some("linux"));
        assert_eq!(classify("2ship-win64.zip").map(|c| c.0), Some("windows"));
        assert_eq!(classify("TroubleMakers-SteamDeck-AppImage-X64.zip").map(|c| c.0), Some("linux"));
        assert!(classify("TroubleMakers-SteamDeck-AppImage-X64.zip").unwrap().2 < classify("TroubleMakers-AppImage-X64.zip").unwrap().2, "the plain build wins over the Deck one");
        assert_eq!(classify("UnleashedRecomp-Windows.zip").map(|c| c.1), Some("x86_64"));
        for n in ["Source code.zip", "game-macos.zip", "x.apk", "Banjo.flatpak", "app-win32.zip", "notes.txt", "x.dmg", "SHA256SUMS.sha256"] { assert!(classify(n).is_none(), "{n}"); }
    }

    #[test]
    fn github_answers_with_headers() {
        let out = "HTTP/2 200\r\netag: W/\"abc\"\r\nx-ratelimit-remaining: 57\r\n\r\n[{\"tag_name\":\"v1\"}]\n200";
        let (code, h, body) = split_reply(out);
        assert_eq!(code, 200);
        assert_eq!(h["etag"], "W/\"abc\"");
        assert_eq!(h["x-ratelimit-remaining"], "57");
        assert_eq!(body, "[{\"tag_name\":\"v1\"}]");
        let (code, h, body) = split_reply("HTTP/2 403\r\nx-ratelimit-remaining: 0\r\nx-ratelimit-reset: 1790000000\r\n\r\n{\"message\":\"API rate limit exceeded\"}\n403");
        assert_eq!((code, h["x-ratelimit-reset"].as_str()), (403, "1790000000"));
        assert!(body.contains("rate limit"));
        assert_eq!(split_reply("HTTP/2 304\r\netag: x\r\n\r\n\n304").0, 304);
    }

    #[test]
    fn the_catalogue_is_good() {
        let v: Value = serde_json::from_str(BUILTIN).expect("recomps.json is JSON");
        assert!(valid_catalogue(&v), "every entry has an id, a title and a GitHub owner/repo");
        let list = v["recomps"].as_array().unwrap();
        assert!(list.len() >= 100, "the catalogue has {} entries", list.len());
        let mut ids = HashSet::new();
        for e in list {
            let id = e["id"].as_str().unwrap();
            assert!(ids.insert(id), "{id} twice");
            assert!(v["consoles"][e["console"].as_str().unwrap()].is_string(), "{id}: unknown console");
            assert!(["recomp", "decomp", "port"].contains(&e["kind"].as_str().unwrap()), "{id}: kind");
            assert!(["n64recomp", "beside", "in-app"].contains(&e["setup"].as_str().unwrap()), "{id}: setup");
            if e["setup"] == "n64recomp" { assert!(n64_rom_path(e).is_some() && e["hash"]["xxh3"][0].as_str().map(|h| h.len() == 16).unwrap_or(false), "{id}: N64Recomp needs its program, game id and hash"); }
            for (k, len) in [("sha1", 40), ("md5", 32), ("sha256", 64), ("xxh3", 16)] {
                for h in e["hash"][k].as_array().into_iter().flatten() { let h = h.as_str().unwrap(); assert!(h.len() == len && h.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()), "{id}: bad {k}"); }
            }
            if let Some(i) = e["icon"].as_str() { assert!(i.starts_with(&format!("https://raw.githubusercontent.com/{}/", e["repo"].as_str().unwrap())), "{id}: the icon is the project's own"); }
            // Never a link to a ROM site: the only addresses are the projects' own.
            let text = e.to_string().to_lowercase();
            for bad in ["romsfun", "vimm", "emuparadise", "coolrom", "romspure", "archive.org", "cdromance", "myrient"] { assert!(!text.contains(bad), "{id} links to a ROM site"); }
        }
        let z = list.iter().find(|e| e["id"] == "zelda64").unwrap();
        assert_eq!(z["repo"], "Zelda64Recomp/Zelda64Recomp");
        assert_eq!(z["hash"]["xxh3"][0], "ef18b4a9e2386169");
    }

    #[test]
    fn n64_roms_in_any_byte_order() {
        let z = vec![0x80, 0x37, 0x12, 0x40, 1, 2, 3, 4];
        let mut v = vec![0x37, 0x80, 0x40, 0x12, 2, 1, 4, 3];
        let mut n = vec![0x40, 0x12, 0x37, 0x80, 4, 3, 2, 1];
        assert!(n64_normalize(&mut v) && v == z);
        assert!(n64_normalize(&mut n) && n == z);
        let mut zz = z.clone();
        assert!(n64_normalize(&mut zz) && zz == z);
        assert!(!n64_normalize(&mut vec![1, 2, 3, 4]));
    }

    #[test]
    fn your_copy_is_checked_against_the_projects_hash() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("game.bin");
        std::fs::write(&f, b"abc").unwrap();
        // Well-known hashes of "abc".
        let ok = check_file(&f, &json!({"sha1": ["a9993e364706816aba3e25717850c26c9cd0d89d"]}), false, &|_| {}).unwrap();
        assert_eq!(ok.as_deref(), Some("sha1"));
        assert_eq!(check_file(&f, &json!({"md5": ["900150983cd24fb0d6963f7d28e17f72"]}), false, &|_| {}).unwrap().as_deref(), Some("md5"));
        assert_eq!(check_file(&f, &json!({"sha256": ["ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"]}), false, &|_| {}).unwrap().as_deref(), Some("sha256"));
        assert!(check_file(&f, &json!({"sha1": ["0000000000000000000000000000000000000000"]}), false, &|_| {}).is_err());
        assert_eq!(check_file(&f, &Value::Null, false, &|_| {}).unwrap(), None, "no documented hash: accepted, not checked");
        // N64: the same ROM byte-swapped (.v64) matches the .z64's hash; XXH3 over the ROM padded to 4 bytes.
        let z: Vec<u8> = [0x80u8, 0x37, 0x12, 0x40].iter().cycle().take(64).cloned().collect();
        let mut v = z.clone();
        for c in v.chunks_exact_mut(2) { c.swap(0, 1); }
        let zf = d.path().join("rom.v64");
        std::fs::write(&zf, &v).unwrap();
        let x = format!("{:016x}", xxhash_rust::xxh3::xxh3_64(&z));
        assert_eq!(check_file(&zf, &json!({"xxh3": [x]}), true, &|_| {}).unwrap().as_deref(), Some("xxh3"));
        assert!(check_file(&f, &json!({"xxh3": ["0000000000000000"]}), true, &|_| {}).is_err(), "not an N64 ROM");
    }

    #[test]
    fn updates_keep_saves_mods_and_settings() {
        let d = tempfile::tempdir().unwrap();
        let (new, app) = (d.path().join("new"), d.path().join("app"));
        for (p, t) in [("game", "v2"), ("assets/a.bin", "v2"), ("saves/slot1.sav", "fresh"), ("config.ini", "default"), ("mods/readme.txt", "v2")] {
            let f = new.join(p); std::fs::create_dir_all(f.parent().unwrap()).unwrap(); std::fs::write(f, t).unwrap();
        }
        for (p, t) in [("game", "v1"), ("assets/a.bin", "v1"), ("saves/slot1.sav", "MY SAVE"), ("config.ini", "MY SETTINGS"), ("mods/cool.o2r", "MY MOD"), ("mods/readme.txt", "v1")] {
            let f = app.join(p); std::fs::create_dir_all(f.parent().unwrap()).unwrap(); std::fs::write(f, t).unwrap();
        }
        // v1 came with game, assets/a.bin and mods/readme.txt; the rest is yours.
        let old: HashSet<String> = ["game", "assets/a.bin", "mods/readme.txt"].iter().map(|s| s.to_string()).collect();
        let files = merge(&new, &app, &old).unwrap();
        let read = |p: &str| std::fs::read_to_string(app.join(p)).unwrap();
        assert_eq!(read("game"), "v2");
        assert_eq!(read("assets/a.bin"), "v2");
        assert_eq!(read("saves/slot1.sav"), "MY SAVE");
        assert_eq!(read("config.ini"), "MY SETTINGS", "a file you changed that wasn't the release's stays");
        assert_eq!(read("mods/cool.o2r"), "MY MOD");
        assert_eq!(read("mods/readme.txt"), "v1", "nothing in mods/ is replaced");
        assert!(files.contains(&"game".to_string()));
        let mine = user_files(&app, &files.iter().cloned().collect());
        assert!(mine.contains(&"mods/cool.o2r".to_string()) && !mine.contains(&"game".to_string()));
        // A first install copies everything.
        let fresh = d.path().join("fresh");
        let n2 = d.path().join("n2");
        std::fs::create_dir_all(n2.join("data")).unwrap();
        std::fs::write(n2.join("data/x"), "1").unwrap();
        assert_eq!(merge(&n2, &fresh, &HashSet::new()).unwrap(), vec!["data/x".to_string()]);
    }

    #[test]
    fn the_program_to_start() {
        let d = tempfile::tempdir().unwrap();
        let w = |p: &str, b: &[u8]| { let f = d.path().join(p); std::fs::create_dir_all(f.parent().unwrap()).unwrap(); std::fs::write(f, b).unwrap(); };
        w("lib/libfoo.so.1", b"\x7fELF....");
        w("Zelda64Recompiled", b"\x7fELF....");
        w("tools/crashpad_handler", b"\x7fELF....");
        w("readme.txt", b"hi");
        w("unins000.exe", b"MZ");
        w("Zelda64Recompiled.exe", b"MZ");
        assert_eq!(executables(d.path(), "linux", "Zelda64Recompiled")[0], "Zelda64Recompiled");
        assert_eq!(executables(d.path(), "windows", "Zelda64Recompiled"), vec!["Zelda64Recompiled.exe".to_string()]);
        w("Starship.AppImage", b"\x7fELF");
        assert_eq!(executables(d.path(), "linux", "starship")[0], "Starship.AppImage");
        let s = run_script("Zelda 64: Recompiled", "/home/me/it's here", "Zelda64Recompiled", "--fullscreen");
        assert!(s.starts_with("#!/bin/sh") && s.contains("cd '/home/me/it'\\''s here'") && s.contains("exec './Zelda64Recompiled' '--fullscreen' \"$@\""));
    }

    #[test]
    fn your_game_file_goes_where_the_port_looks() {
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        let rom = d.path().join("Perfect Dark (USA).z64");
        std::fs::write(&rom, b"x").unwrap();
        place_beside(&json!({"besideName": "data/pd.ntsc-final.z64"}), &app, &rom).unwrap();
        assert!(app.join("data/pd.ntsc-final.z64").is_symlink());
        place_beside(&json!({}), &app, &rom).unwrap();
        assert!(app.join("Perfect Dark (USA).z64").is_symlink());
        // A disc image as bin + cue: the cue and its tracks.
        let cue = d.path().join("Game.cue");
        std::fs::write(&cue, "FILE \"Game (Track 1).bin\" BINARY\n  TRACK 01 MODE2/2352\nFILE \"Game (Track 2).bin\" BINARY\n").unwrap();
        assert_eq!(cue_tracks(&std::fs::read_to_string(&cue).unwrap()), vec!["Game (Track 1).bin", "Game (Track 2).bin"]);
        place_beside(&json!({"besideName": "isos/"}), &app, &cue).unwrap();
        assert!(app.join("isos/Game.cue").is_symlink() && app.join("isos/Game (Track 2).bin").is_symlink());
        assert!(place_beside(&json!({"besideName": "../x"}), &app, &rom).is_err());
        let e = json!({"n64": {"programId": "Zelda64Recompiled", "gameId": "mm.n64.us.1.0"}});
        assert!(n64_rom_path(&e).unwrap().ends_with("Zelda64Recompiled/mm.n64.us.1.0.z64"));
        assert!(n64_rom_path(&json!({"n64": {"programId": "../x", "gameId": "y"}})).is_none());
    }

    #[test]
    fn projects_you_add_and_menu_entries() {
        let e = custom_entry("https://github.com/Some-One/Cool.Port/", None).unwrap();
        assert_eq!((e["repo"].as_str().unwrap(), e["id"].as_str().unwrap(), e["custom"].as_bool()), ("Some-One/Cool.Port", "custom-some-one-cool-port", Some(true)));
        assert!(custom_entry("not a repo", None).is_err() && custom_entry("a/b/../c", None).is_ok_and(|e| e["repo"] == "a/b"));
        assert!(!repo_ok("../etc") && !repo_ok("a/..") && repo_ok("harbourmasters/2ship2harkinian"));
        let d = desktop_entry("Ship of Harkinian", "Ocarina of Time", "/home/me/.local/share/nimbus/recomps/shipwright/run.sh", false, "/x", None);
        assert!(d.contains("Exec=\"/home/me/.local/share/nimbus/recomps/shipwright/run.sh\"") && d.contains("Categories=Game;") && !d.to_lowercase().contains("xodus"));
        let w = desktop_entry("X", "", "/g/Game $1.exe", true, "/g", Some("/i.png"));
        assert!(w.contains("Exec=umu-run \"/g/Game \\$1.exe\"") && w.contains("Icon=/i.png"));
        assert_eq!(id_of("recomp:zelda64"), Some("zelda64"));
    }
}

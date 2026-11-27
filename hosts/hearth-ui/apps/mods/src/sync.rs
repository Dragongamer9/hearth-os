//! Your other devices (boards 1, 3, 4, 7 and 8): each of your Hearth devices keeps a short note of its mods in the home
//! server's Mods folder (`<Mods>/.hearth/devices/<device>.json`: its games, their mod counts and profiles, the updates
//! it has waiting, its crashes, its last fps), so every device sees the others ("synced with the Ally", "Also on…",
//! the devices on each game's card). Asking another device to do something (add a mod there as well, take a game's
//! profile, measure fps) is a small file in its inbox there (`<Mods>/.hearth/inbox/<device>/`), done the next time its
//! Mod Manager opens, never without it. Hearth Link's own list (`hearth-link call status.devices`) says which are on.
//! Mod files themselves are kept once in that Mods folder: a download that's already there is taken from there.
//! Nothing here is a secret; nothing leaves your home server.

use crate::sources;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn read(p: &Path) -> Value {
    hearth_ui::settings_file::read_json(p)
}

fn write(p: &Path, v: &Value) -> Result<(), String> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("The home server's Mods folder can't be written here ({e})"))?;
    }
    hearth_ui::settings_file::write_json(p, v).map(|_| ()).map_err(|e| e.to_string())
}

/// The home server's own folder for these notes (inside its Mods folder), when there is one.
pub fn root() -> Option<PathBuf> {
    sources::home_server().map(|m| m.join(".hearth"))
}

fn key(name: &str) -> String {
    hearth_gaming::util::safe_name(&name.to_lowercase().replace(' ', "-"))
}

/// What kind of device this is: the ROG Ally, a laptop, the truck's laptop, or a PC.
pub fn kind() -> &'static str {
    let r = |f: &str| std::fs::read_to_string(format!("/sys/class/dmi/id/{f}")).unwrap_or_default().trim().to_lowercase();
    let product = r("product_name");
    if product.contains("ally") || product.contains("rc71") || product.contains("rc72") {
        return "ALLY";
    }
    match r("chassis_type").as_str() {
        "8" | "9" | "10" | "14" | "31" | "32" => "LAPTOP",
        _ => "PC",
    }
}

/// This device's name as the other devices see it: the one you gave it (Settings), else what it is.
pub fn here() -> String {
    let own = crate::back::read_own("device.json");
    if let Some(n) = own["name"].as_str().filter(|n| !n.trim().is_empty()) {
        return n.trim().to_string();
    }
    match kind() {
        "ALLY" => "ROG Ally".into(),
        "LAPTOP" => "Laptop".into(),
        _ if Path::new("/proc/driver/nvidia").exists() => "Nvidia PC".into(),
        _ => hearth_gaming::util::host_name(),
    }
}

/// Hearth Link's devices (name → on), when Hearth Link answers within a few seconds.
fn link_devices() -> Vec<(String, bool)> {
    let out = std::process::Command::new("timeout").args(["4", "hearth-link", "call", "status.devices"]).output();
    let Ok(o) = out else { return Vec::new() };
    let v: Value = serde_json::from_slice(&o.stdout).unwrap_or(Value::Null);
    v["devices"].as_array().cloned().unwrap_or_default().iter().map(|d| (d["name"].as_str().unwrap_or("").to_string(), d["online"].as_bool().unwrap_or(false))).filter(|d| !d.0.is_empty()).collect()
}

/// Every other device's note, newest first, each with its state worked out ("away", "N updates", "").
pub fn others() -> Vec<Value> {
    let Some(r) = root() else { return Vec::new() };
    let me = key(&here());
    let link = link_devices();
    let now = crate::now();
    let mut v: Vec<Value> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(r.join("devices")) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if !n.ends_with(".json") || n.trim_end_matches(".json") == me {
                continue;
            }
            let mut d = read(&e.path());
            let name = d["name"].as_str().unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }
            let on = link.iter().find(|l| l.0.eq_ignore_ascii_case(&name)).map(|l| l.1);
            let at = d["at"].as_i64().unwrap_or(0);
            let ups = d["updates"].as_array().map(|u| u.len()).unwrap_or(0);
            let state = if on == Some(false) && now - at > 3 * 86400 || on.is_none() && now - at > 3 * 86400 {
                "away".to_string()
            } else if ups > 0 {
                format!("{ups} update{}", if ups == 1 { "" } else { "s" })
            } else {
                String::new()
            };
            d["state"] = json!(state);
            d["online"] = json!(on.unwrap_or(now - at < 600));
            let games: Vec<String> = d["games"].as_array().cloned().unwrap_or_default().iter().filter(|g| g["mods"].as_u64().unwrap_or(0) > 0).filter_map(|g| g["title"].as_str().map(short)).collect();
            d["gamesWords"] = json!(games.join(", "));
            v.push(d);
        }
    }
    v.sort_by(|a, b| b["at"].as_i64().cmp(&a["at"].as_i64()));
    v
}

pub fn short(t: &str) -> String {
    t.replace("Special Edition", "SE").replace("The Elder Scrolls V: ", "").replace("The Battle for Middle-earth II", "BFME II")
}

/// The devices that have this game modded (its title), for "Also on…" and the cards.
pub fn with_game<'a>(devs: &'a [Value], title: &str) -> Vec<&'a Value> {
    devs.iter().filter(|d| d["games"].as_array().map(|g| g.iter().any(|x| x["title"].as_str().map(|t| t.eq_ignore_ascii_case(title)).unwrap_or(false) && x["mods"].as_u64().unwrap_or(0) > 0)).unwrap_or(false)).collect()
}

/// The device's own short tag on the boards (PC, ALLY, LAPTOP, TRUCK).
pub fn tag_of(d: &Value) -> String {
    let n = d["name"].as_str().unwrap_or("").to_lowercase();
    if n.contains("truck") {
        "TRUCK".into()
    } else {
        d["kind"].as_str().unwrap_or("PC").to_string()
    }
}

/// Writes this device's note (what the others read).
pub fn publish(games: Value, updates: Value, crashes: Value, measured: Value) -> Result<(), String> {
    let Some(r) = root() else { return Ok(()) };
    let me = here();
    let mut v = read(&r.join("devices").join(format!("{}.json", key(&me))));
    v["name"] = json!(me);
    v["kind"] = json!(kind());
    v["at"] = json!(crate::now());
    v["games"] = games;
    v["updates"] = updates;
    v["crashes"] = crashes;
    if !measured.is_null() {
        v["measured"] = measured;
    }
    // memory and space, for Describe it's "On the Ally"
    let mi = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let kb = |k: &str| mi.lines().find(|l| l.starts_with(k)).and_then(|l| l.split_whitespace().nth(1)).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0) * 1024;
    v["memory"] = json!({"total": kb("MemTotal:"), "used": kb("MemTotal:").saturating_sub(kb("MemAvailable:"))});
    if let Some(f) = hearth_gaming::wabbajack::free_space(&hearth_oss_kit_a::home()) {
        v["free"] = json!(f);
    }
    write(&r.join("devices").join(format!("{}.json", key(&me))), &v)
}

/// Asks another device to do something the next time its Mod Manager opens.
pub fn send(to: &str, req: Value) -> Result<String, String> {
    let r = root().ok_or("Set up Hearth's home server (Settings › Home server) first: your devices meet in its Mods folder.")?;
    let mut req = req;
    req["from"] = json!(here());
    req["at"] = json!(crate::now());
    let f = r.join("inbox").join(key(to)).join(format!("{}.json", hearth_gaming::util::now_ms()));
    write(&f, &req)?;
    Ok(format!("Sent to {to}: it's done the next time the Mod Manager opens there."))
}

/// What the other devices asked this one to do (taken out of the inbox; kept in done/ beside it).
pub fn take_inbox() -> Vec<Value> {
    let Some(r) = root() else { return Vec::new() };
    let dir = r.join("inbox").join(key(&here()));
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().map(|x| x == "json").unwrap_or(false)).collect();
        files.sort();
        for f in files {
            let req = read(&f);
            let done = dir.join("done");
            let _ = std::fs::create_dir_all(&done);
            let _ = std::fs::rename(&f, done.join(f.file_name().unwrap_or_default()));
            if req["kind"].is_string() {
                v.push(req);
            }
        }
    }
    v
}

/// A mod file that's already on the home server (downloaded once by any device): where it is.
pub fn on_server(domain: &str, file_name: &str, size: u64) -> Option<PathBuf> {
    let m = sources::home_server()?;
    for p in [m.join(domain).join(file_name), m.join(file_name), m.join("nexus").join(file_name)] {
        if let Ok(md) = std::fs::metadata(&p) {
            if md.is_file() && (size == 0 || md.len() == size) {
                return Some(p);
            }
        }
    }
    None
}

/// Counts a file taken from the home server instead of downloaded (board 8's "Saved this month").
pub fn note_saved(bytes: u64) {
    let month = hearth_ui::time::day(crate::now()).split(' ').next().unwrap_or("").to_string();
    let mut s = crate::back::read_own("server.json");
    if s["month"].as_str() != Some(month.as_str()) {
        s["month"] = json!(month);
        s["savedThisMonth"] = json!(0);
    }
    s["savedThisMonth"] = json!(s["savedThisMonth"].as_u64().unwrap_or(0) + bytes);
    let _ = crate::back::write_own("server.json", &s);
}

/// Before a Nexus download: the same file on the home server is used instead (and counted). Else the download goes
/// into the home server's Mods folder (one download for every device), or Hearth Mods' own folder without one.
pub fn nexus_get(domain: String, mod_id: u64, file_id: u64, game: String) -> Result<Value, String> {
    if let Some(acct) = hearth_gaming::nexus::account() {
        let f = hearth_gaming::nexus::file_info(&acct.key, &domain, mod_id, file_id).unwrap_or(Value::Null);
        let name = f["file_name"].as_str().unwrap_or("").to_string();
        let size = f["size_in_bytes"].as_u64().unwrap_or(0);
        if !name.is_empty() {
            if let Some(p) = on_server(&domain, &name, size) {
                note_saved(size.max(std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0)));
                return Ok(json!({"ok": true, "fromServer": p, "notes": [format!("{name} is already on your home server: taken from there, not downloaded again.")]}));
            }
        }
    }
    let dir = sources::home_server().map(|m| m.join(&domain)).filter(|d| std::fs::create_dir_all(d).is_ok() && writable(d));
    hearth_gaming::mods_app::route("download_file", &json!({"domain": domain, "modId": mod_id, "fileId": file_id, "game": game, "dir": dir.map(|d| d.to_string_lossy().to_string()).unwrap_or_default()}))
}

fn writable(d: &Path) -> bool {
    let t = d.join(".hearth-write-test");
    let ok = std::fs::write(&t, b"").is_ok();
    let _ = std::fs::remove_file(&t);
    ok
}

/// What the home server keeps for mods: every file in its Mods folder (all versions), and the old versions.
pub fn server_files() -> (u64, usize) {
    let Some(m) = sources::home_server() else { return (0, 0) };
    let mut files = Vec::new();
    hearth_gaming::util::files_under(&m, &mut files);
    let files: Vec<PathBuf> = files.into_iter().filter(|f| !f.components().any(|c| c.as_os_str() == ".hearth")).collect();
    (files.iter().filter_map(|f| std::fs::metadata(f).ok()).map(|m| m.len()).sum(), files.len())
}

// ------------------------------------------------------------------ fps from MangoHud's own logs

/// MangoHud's log folders: where its config says (output_folder), and where Hearth asks it to log a measuring start.
fn mango_dirs() -> Vec<PathBuf> {
    let home = hearth_oss_kit_a::home();
    let mut v = vec![crate::back::own_dir().join("fps"), home.join("mangologs"), home.join(".local/share/MangoHud")];
    let conf = std::fs::read_to_string(home.join(".config/MangoHud/MangoHud.conf")).unwrap_or_default();
    for l in conf.lines() {
        if let Some(p) = l.trim().strip_prefix("output_folder=") {
            v.push(PathBuf::from(p.trim().replace('~', &home.to_string_lossy())));
        }
    }
    v
}

/// The fps of one MangoHud log (its CSV: a header, then one row a frame with fps first): (average, 1% low, frames).
pub fn read_mango(text: &str) -> Option<(f64, f64, usize)> {
    let mut lines = text.lines();
    // MangoHud writes system info, then the column names, then the frames
    let mut col = None;
    for l in lines.by_ref() {
        let cols: Vec<&str> = l.split(',').map(str::trim).collect();
        if let Some(i) = cols.iter().position(|c| *c == "fps") {
            col = Some(i);
            break;
        }
    }
    let i = col?;
    let mut fps: Vec<f64> = lines.filter_map(|l| l.split(',').nth(i).and_then(|x| x.trim().parse::<f64>().ok())).filter(|f| *f > 0.0).collect();
    if fps.is_empty() {
        return None;
    }
    let avg = fps.iter().sum::<f64>() / fps.len() as f64;
    fps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let low = fps[(fps.len() / 100).min(fps.len() - 1)];
    Some((avg, low, fps.len()))
}

/// This device's fps for a game's exe, from MangoHud's newest logs (each a start; the place is the save made last in
/// it): {settings, places: [{name, fps, low}], at}.
pub fn measured_here(exe: &str, saves: &[(PathBuf, crate::logic::Save, u64)]) -> Value {
    let stem = exe.trim_end_matches(".exe").to_lowercase();
    let mut logs: Vec<(PathBuf, u64)> = Vec::new();
    for d in mango_dirs() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_lowercase();
                if n.ends_with(".csv") && !n.contains("summary") && (stem.is_empty() || n.starts_with(&stem)) {
                    let t = e.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
                    logs.push((e.path(), t));
                }
            }
        }
    }
    logs.sort_by(|a, b| b.1.cmp(&a.1));
    let mut places: Vec<Value> = Vec::new();
    for (f, t) in logs.into_iter().take(6) {
        let Some((avg, low, _)) = read_mango(&std::fs::read_to_string(&f).unwrap_or_default()) else { continue };
        let place = saves.iter().filter(|s| s.2 <= t + 60).max_by_key(|s| s.2).map(|s| s.1.place.clone()).unwrap_or_else(|| hearth_ui::time::when(t as i64));
        if !places.iter().any(|p| p["name"].as_str() == Some(place.as_str())) {
            places.push(json!({"name": place, "fps": avg.round(), "low": low.round(), "at": t}));
        }
    }
    if places.is_empty() {
        return Value::Null;
    }
    json!({"places": places, "settings": crate::back::read_own("device.json")["settings"].as_str().unwrap_or(""), "at": crate::now()})
}

/// The environment for a measuring start: MangoHud on, logging the first 60 seconds into the Mod Manager's own folder.
pub fn measure_env() -> Vec<(String, String)> {
    let dir = crate::back::own_dir().join("fps");
    let _ = std::fs::create_dir_all(&dir);
    vec![("MANGOHUD".into(), "1".into()), ("MANGOHUD_CONFIG".into(), format!("no_display,autostart_log=1,log_duration=60,output_folder={}", dir.display()))]
}

#[cfg(test)]
mod tests {
    #[test]
    fn mangohud_log() {
        let t = "os,cpu,gpu\nLinux,Z1,780M\nfps,frametime,cpu_load\n60,16.6,30\n62,16.1,31\n30,33.3,40\n58,17.2,30\n";
        let (avg, low, n) = super::read_mango(t).unwrap();
        assert_eq!(n, 4);
        assert!((avg - 52.5).abs() < 0.01);
        assert_eq!(low, 30.0);
    }
}

//! Xbox PC games through Xodus Nexus (experimental): an independent Linux launcher for Microsoft Store and PC Game
//! Pass games, built on the Xodus project (not Microsoft's). Hearth OS ships it, like Beacon (os-bases/*/build.sh put
//! its AppImage in /usr/lib/xodus-nexus and `xodus-nexus` in /usr/bin). Elsewhere, or for a newer version than the
//! image has, Hearth downloads its AppImage from its GitHub releases into your data folder; that copy wins.
//! The library shows Nexus as one entry that opens it full screen, plus a tile for each game you made a shortcut for
//! in Nexus. Nexus downloads its own backend and runtime from its Settings. https://github.com/domsvoboda/xodus-nexus

use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const ID: &str = "xodus:nexus";
const RELEASES: &str = "https://api.github.com/repos/domsvoboda/xodus-nexus/releases/latest";
pub const PROJECT: &str = "https://github.com/domsvoboda/xodus-nexus";

/// Where Hearth OS's image keeps Nexus, and the program that starts it (the same script as RUN_SH).
pub const SYSTEM_DIR: &str = "/usr/lib/xodus-nexus";
pub const SYSTEM_RUNNER: &str = "/usr/bin/xodus-nexus";

pub fn dir() -> PathBuf { crate::paths::data_dir().join("xodus-nexus") }
fn appimage() -> PathBuf { dir().join("Xodus-Nexus.AppImage") }
fn own_runner() -> PathBuf { dir().join("run.sh") }
/// Your own copy (downloaded by Hearth), which wins over the image's.
fn own_copy() -> bool { appimage().exists() && own_runner().exists() }
/// The copy Hearth OS ships.
pub fn system_copy() -> bool { std::path::Path::new(SYSTEM_DIR).join("Xodus-Nexus.AppImage").is_file() && std::path::Path::new(SYSTEM_RUNNER).is_file() }
/// What the library starts: a small script that runs the AppImage the way it works in gamescope.
pub fn runner() -> PathBuf { if own_copy() || !system_copy() { own_runner() } else { PathBuf::from(SYSTEM_RUNNER) } }
fn version_file() -> PathBuf { if own_copy() || !system_copy() { dir().join("version") } else { PathBuf::from(SYSTEM_DIR).join("version") } }
fn folder_file() -> PathBuf { dir().join("games-folder") }
/// Where you chose to keep Xbox games (Nexus is pointed at it in its Settings).
pub fn games_folder() -> Option<String> { std::fs::read_to_string(folder_file()).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) }

/// Places for the games folder: this device, and each USB drive, SD card or second drive that's mounted, with
/// free space: [{label, path, free}]
pub fn places() -> Vec<Value> {
    let free = |p: &std::path::Path| crate::system::run("df", &["-B1", "--output=avail", &p.to_string_lossy()]).ok()
        .and_then(|o| o.lines().nth(1).and_then(|l| l.trim().parse::<u64>().ok())).unwrap_or(0);
    let home = crate::paths::home();
    let mut out = vec![json!({"label": "This device", "path": home.join("Games/Xbox").to_string_lossy(), "free": free(&home)})];
    let user = std::env::var("USER").unwrap_or_default();
    for base in [PathBuf::from("/run/media").join(&user), PathBuf::from("/media").join(&user)] {
        for d in std::fs::read_dir(&base).into_iter().flatten().flatten() {
            let p = d.path();
            if !p.is_dir() { continue; }
            out.push(json!({"label": d.file_name().to_string_lossy(), "path": p.join("Xbox Games").to_string_lossy(), "free": free(&p)}));
        }
    }
    out
}
/// Makes the games folder and remembers it.
pub fn set_folder(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    let home = crate::paths::home();
    let ok = p.starts_with(&home) || p.starts_with("/run/media") || p.starts_with("/media");
    if !ok || path.contains("..") { return Err("Pick one of the places offered".into()); }
    std::fs::create_dir_all(p).map_err(|e| format!("Couldn't make the folder: {e}"))?;
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    std::fs::write(folder_file(), path).map_err(|e| e.to_string())
}
/// Desktop Mode opens Nexus once it starts (its first sign-in, where the controller types).
pub fn open_in_desktop() -> Result<(), String> {
    if !installed() { return Err("Add Xodus Nexus first".into()); }
    let f = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("hearth-open-after");
    std::fs::write(f, runner().to_string_lossy().as_bytes()).map_err(|e| e.to_string())
}

pub fn installed() -> bool { own_copy() || system_copy() }
pub fn version() -> Option<String> { std::fs::read_to_string(version_file()).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) }

/// The library entry (only when installed).
pub fn game() -> Option<crate::library::Game> {
    if !installed() { return None; }
    let exe = runner().to_string_lossy().to_string();
    Some(crate::library::Game { id: ID.into(), title: "Xbox PC Games (Xodus Nexus)".into(), source: "xodus", install_path: Some(dir()),
        installed: true, state: "installed", exe: Some(exe), ..Default::default() })
}

/// Nexus and the games you made shortcuts for in it: what the library shows under Xbox PC (only when installed).
pub fn games() -> Vec<crate::library::Game> {
    let Some(nexus) = game() else { return vec![] };
    let mut out = vec![nexus];
    let home = crate::paths::home();
    let apps = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share")).join("applications");
    for folder in [apps, home.join("Desktop")] {
        for e in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
            let p = e.path();
            if p.extension().map(|x| x != "desktop").unwrap_or(true) { continue; }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            let Some((id, title)) = shortcut(&e.file_name().to_string_lossy(), &text) else { continue };
            if out.iter().any(|g| g.id == id) { continue; }
            out.push(crate::library::Game { id, title, source: "xodus", installed: true, state: "installed", exe: Some(p.to_string_lossy().to_string()), ..Default::default() });
        }
    }
    out
}

/// A game shortcut Nexus made (a game's menu › Create shortcuts): (library id, title), or None for anything else.
/// Nexus doesn't document what its shortcuts look like, so this only takes .desktop files whose Exec names Xodus,
/// and leaves out Nexus's own menu entry. The shortcut itself is what Play starts, exactly as Nexus wrote it.
pub fn shortcut(file_name: &str, text: &str) -> Option<(String, String)> {
    let d = crate::manual::parse_desktop(text);
    let exec = d.get("Exec")?.to_lowercase();
    let name = d.get("Name").map(|n| n.trim().to_string()).filter(|n| !n.is_empty())?;
    let stem = file_name.trim_end_matches(".desktop").to_lowercase();
    if !exec.contains("xodus") || stem.starts_with("xodus") || name.to_lowercase().starts_with("xodus") { return None; }
    let slug: String = stem.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).take(80).collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() { return None; }
    Some((format!("xodus:game:{slug}"), name.chars().filter(|c| !c.is_control()).take(120).collect()))
}

/// Nexus's own window is driven with a mouse, so it starts with the "Mouse only" controller layout. Xbox games have
/// controller support of their own and start Off, like any game.
pub fn pointer_by_default(library_id: &str) -> bool { library_id == ID }

const RUN_SH: &str = r#"#!/bin/sh
# Starts Xodus Nexus for Hearth (made by Hearth; replaced on each update).
here="$(dirname "$(readlink -f "$0")")"
# AppImages need FUSE; without it, unpack and run instead.
if ! command -v fusermount >/dev/null 2>&1 && ! command -v fusermount3 >/dev/null 2>&1; then export APPIMAGE_EXTRACT_AND_RUN=1; fi
# WebKit's newer GPU path draws a black window on some Nvidia drivers.
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
# Sign-in pages open in your default browser (Hearth Browser unless you picked another in Settings › Default apps),
# not whichever browser the app finds first.
export BROWSER="${BROWSER:-xdg-open}"
exec "$here/Xodus-Nexus.AppImage" "$@"
"#;

/// The newest release: (version, AppImage download address, size).
fn latest() -> Result<(String, String, u64), String> {
    let r = crate::http::get_json(RELEASES, &[("Accept", "application/vnd.github+json")])?;
    let tag = r["tag_name"].as_str().unwrap_or("").trim_start_matches('v').to_string();
    let asset = r["assets"].as_array().and_then(|a| a.iter().find(|x| {
        let n = x["name"].as_str().unwrap_or("").to_lowercase();
        n.ends_with(".appimage") && (n.contains("x86_64") || n.contains("amd64"))
    }).cloned()).ok_or("The newest Xodus Nexus release has no AppImage for this PC yet.")?;
    Ok((tag, asset["browser_download_url"].as_str().unwrap_or("").to_string(), asset["size"].as_u64().unwrap_or(0)))
}

/// The newer version to offer, if any.
fn update_of(have: Option<String>, latest: &Option<String>) -> Option<String> {
    match (have, latest) { (Some(v), Some(l)) if !l.is_empty() && &v != l => Some(l.clone()), _ => None }
}

/// Install and update, in the background, telling the page how it's going ("xodus" events).
pub struct Xodus { pub state: Arc<Mutex<Value>>, checked: Arc<Mutex<(u64, Option<String>)>> }
impl Default for Xodus { fn default() -> Self { Self::new() } }
impl Xodus {
    pub fn new() -> Xodus { Xodus { state: Arc::new(Mutex::new(json!({"busy": false, "progress": null, "error": null}))), checked: Arc::new(Mutex::new((0, None))) } }

    /// {installed, version, latest, busy, progress, error, project}. Looks for a newer version at most every 6 hours.
    pub fn status(&self, look: bool) -> Value {
        let mut s = self.state.lock().unwrap().clone();
        let now = crate::library::now_ms();
        let latest = {
            let mut c = self.checked.lock().unwrap();
            if look && installed() && now.saturating_sub(c.0) > 6 * 3600 * 1000 {
                // Asked of GitHub in the background (up to 30 s; it held the bridge): the page hears an "xodus" event.
                c.0 = now;
                let (checked, state) = (self.checked.clone(), self.state.clone());
                std::thread::spawn(move || {
                    let l = latest().ok().map(|l| l.0);
                    checked.lock().unwrap().1 = l.clone();
                    let busy = state.lock().unwrap()["busy"].clone();
                    crate::stores::notify(json!({"type": "event", "name": "xodus", "data": {"busy": busy, "update": update_of(version(), &l)}}));
                });
            }
            c.1.clone()
        };
        s["installed"] = json!(installed()); s["version"] = json!(version()); s["project"] = json!(PROJECT); s["folder"] = json!(games_folder());
        // Hearth OS's copy can't be removed (only your own download), and Nexus's game shortcuts count as its games.
        s["preinstalled"] = json!(system_copy()); s["own"] = json!(own_copy()); s["games"] = json!(games().len().saturating_sub(1));
        s["update"] = json!(update_of(version(), &latest));
        s
    }

    /// Downloads the newest AppImage (first install or update).
    pub fn install(&self, emit: crate::system::bt_scan::Emit) -> Value {
        {
            let mut s = self.state.lock().unwrap();
            if s["busy"] == true { drop(s); return self.status(false); }
            s["busy"] = json!(true); s["progress"] = json!(0); s["error"] = Value::Null;
        }
        let state = self.state.clone();
        std::thread::spawn(move || {
            let push = |st: &Arc<Mutex<Value>>| { let mut v = st.lock().unwrap().clone(); v["installed"] = json!(installed()); v["version"] = json!(version()); emit(json!({"type": "event", "name": "xodus", "data": v})); };
            push(&state);
            let r = (|| -> Result<(), String> {
                let (ver, url, size) = latest()?;
                std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
                let part = dir().join("Xodus-Nexus.AppImage.part");
                let _ = std::fs::remove_file(&part);
                // A download that stalls (under 1 KB/s for a minute) stops: it used to wait forever, with the install
                // showing as busy (and not startable again) until Hearth restarted.
                let mut child = std::process::Command::new("curl").args(["-sSL", "--fail", "--speed-limit", "1024", "--speed-time", "60", "-o"]).arg(&part).arg(&url)
                    .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map_err(|e| format!("Couldn't download it: {e}"))?;
                loop {
                    if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
                        if !st.success() { let _ = std::fs::remove_file(&part); return Err("The download didn't finish. Check the internet connection and try again.".into()); }
                        break;
                    }
                    if size > 0 { let got = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0); state.lock().unwrap()["progress"] = json!((got as f64 / size as f64).min(1.0)); push(&state); }
                    std::thread::sleep(Duration::from_millis(500));
                }
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
                std::fs::rename(&part, appimage()).map_err(|e| e.to_string())?;
                std::fs::write(own_runner(), RUN_SH).map_err(|e| e.to_string())?;
                std::fs::set_permissions(own_runner(), std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
                std::fs::write(dir().join("version"), &ver).map_err(|e| e.to_string())?;
                Ok(())
            })();
            { let mut s = state.lock().unwrap(); s["busy"] = json!(false); s["progress"] = Value::Null; s["error"] = match r { Ok(()) => Value::Null, Err(e) => json!(e) }; }
            push(&state);
        });
        self.status(false)
    }
}

/// Removes Hearth's download (Nexus's own data, sign-in and installed games stay where Nexus keeps them). On Hearth OS
/// the image's copy stays, so this goes back to it.
pub fn remove() -> Result<(), String> {
    if dir().exists() { std::fs::remove_dir_all(dir()).map_err(|e| format!("Couldn't remove it: {e}"))?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn game_shortcuts_from_nexus() {
        use super::shortcut;
        let game = "[Desktop Entry]\nType=Application\nName=Forza Horizon 5\nExec=/home/me/Apps/Xodus-Nexus.AppImage --launch 9NKX70BBCDRN\n";
        assert_eq!(shortcut("Forza Horizon 5.desktop", game), Some(("xodus:game:forza-horizon-5".into(), "Forza Horizon 5".into())));
        // Nexus's own entry, other apps, and broken files aren't games.
        assert_eq!(shortcut("xodus-nexus.desktop", "[Desktop Entry]\nName=Xodus Nexus\nExec=xodus-nexus %U\n"), None);
        assert_eq!(shortcut("nexus.desktop", "[Desktop Entry]\nName=Xodus Nexus\nExec=/opt/Xodus-Nexus.AppImage\n"), None);
        assert_eq!(shortcut("firefox.desktop", "[Desktop Entry]\nName=Firefox\nExec=firefox %u\n"), None);
        assert_eq!(shortcut("x.desktop", "[Desktop Entry]\nExec=xodus run\n"), None);
        assert!(super::pointer_by_default("xodus:nexus") && !super::pointer_by_default("xodus:game:forza-horizon-5"));
    }

    #[test]
    fn runner_script_starts_the_appimage() {
        assert!(super::RUN_SH.starts_with("#!/bin/sh") && super::RUN_SH.contains("exec \"$here/Xodus-Nexus.AppImage\""));
        assert!(super::game().is_none() || super::installed());
    }
}

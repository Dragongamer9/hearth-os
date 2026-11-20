//! Desktop icons for games (Hearth Games' "Add to desktop", the owner, Nov 19 2026; docs/HEARTH-GAMES.md).
//!
//! A GOG, Epic, Amazon or Battle.net game installed through Hearth Games can have an icon on the desktop: a launcher
//! (`<Game name>.desktop`) in your Desktop folder (xdg-user-dir DESKTOP, read from ~/.config/user-dirs.dirs the way that
//! command reads it; else ~/Desktop). It has the game's name, the game's art as its picture (a copy kept in Hearth's
//! data folder, ~/.local/share/nimbus/desktop-icons, so it stays when the art cache is cleaned), and starts the game the
//! way its Steam Game Mode shortcut does (`nimbus --play-now <id>`, desktop_games::play_now). It's executable (KDE
//! Plasma's desktop and Firehold on Plasma open an executable launcher in your Desktop folder without asking whether
//! to trust it; GNOME-style desktops read metadata::trusted, set too where gio is there). Mantel's desktop (labwc,
//! KWin and Firehold's Desk: Desktop.qml with hearth-desk) shows and starts any launcher there.
//!
//! Where it lands: nothing here places icons. Each desktop puts a new file in its next free spot and keeps every
//! other icon where it was saved (Mantel: hearth-desk's layout; Plasma: its folder view, in the order you set).
//!
//! What's Hearth's: every icon Hearth makes is written down in desktop-icons/made.json (the file, and a SHA-256 of
//! exactly what Hearth wrote). "Remove from desktop" and uninstalling the game take off only that file, and only while
//! it is still byte for byte what Hearth wrote: a file you made, copied, renamed over it or changed yourself is never
//! deleted (you're told it was left). Uninstalling a game takes its icon off only this way.

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// The key every launcher Hearth makes carries: the game's library id.
pub const MARK: &str = "X-Hearth-Game";
/// The stores whose games can have a desktop icon (Hearth Games' own).
pub const SOURCES: &[&str] = &["gog", "epic", "amazon", "battlenet"];
/// Hearth Games' own icon, when a game has no art.
pub const FALLBACK_ICON: &str = "hearth-gamestore";

/// Where everything lives (the real places in `real()`, a test's own in the tests).
#[derive(Debug, Clone)]
pub struct Places {
    /// The Desktop folder, or None when it's set to your home folder itself (Hearth then adds nothing there).
    pub desktop: Option<PathBuf>,
    /// Hearth's folder for these: made.json and the icons' pictures.
    pub data: PathBuf,
    /// The program the launcher starts.
    pub exec: String,
}

impl Places {
    pub fn record(&self) -> PathBuf { self.data.join("made.json") }
    pub fn real() -> Places {
        let home = crate::paths::home();
        let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config"));
        let dirs = std::fs::read_to_string(cfg.join("user-dirs.dirs")).ok();
        let env = std::env::var("XDG_DESKTOP_DIR").ok();
        Places { desktop: desktop_dir(&home, dirs.as_deref(), env.as_deref()), data: crate::paths::data_dir().join("desktop-icons"), exec: "nimbus".into() }
    }
}

/// The Desktop folder as `xdg-user-dir DESKTOP` gives it: XDG_DESKTOP_DIR from user-dirs.dirs ("$HOME/…" or an absolute
/// path), else the variable already set, else ~/Desktop. None when that's your home folder itself (xdg-user-dir's
/// answer when there's no Desktop folder): the desktop would show everything in Home, so Hearth adds nothing (as Mantel's
/// hearth-desk desktop_dir).
pub fn desktop_dir(home: &Path, user_dirs: Option<&str>, env: Option<&str>) -> Option<PathBuf> {
    let from_file = user_dirs.and_then(|t| t.lines().rev().find_map(|l| {
        let l = l.trim();
        if l.starts_with('#') { return None; }
        let v = l.strip_prefix("XDG_DESKTOP_DIR=")?.trim();
        Some(v.trim_matches('"').to_string())
    }));
    let raw = from_file.or_else(|| env.map(str::to_string)).filter(|v| !v.trim().is_empty());
    let p = match raw {
        Some(v) => {
            if v == "$HOME" { home.to_path_buf() }
            else if let Some(rest) = v.strip_prefix("$HOME/") { home.join(rest) }
            else if v.starts_with('/') { PathBuf::from(v) }
            else { return Some(home.join("Desktop")) }   // not a path xdg-user-dir would give: the usual one
        }
        None => home.join("Desktop"),
    };
    let norm = |p: &Path| p.components().collect::<PathBuf>();
    if norm(&p) == norm(home) { return None; }
    Some(p)
}

/// A game id a desktop icon may start: one of Hearth Games' stores, plain letters, digits and . _ - : only (so the
/// launcher's Exec line needs no quoting, and nothing in it can be read as more than one word).
pub fn id_ok(id: &str) -> bool {
    let Some((src, rest)) = id.split_once(':') else { return false };
    SOURCES.contains(&src) && !rest.is_empty() && id.len() <= 200
        && rest.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':'))
}

/// The launcher's file name (without .desktop) for a game: its name, without anything a file name can't hold or that
/// would hide it ("/", a leading dot, control characters), at most 80 characters.
pub fn file_stem(title: &str) -> String {
    let s: String = title.chars().map(|c| if c == '/' || c == '\\' || c.is_control() { ' ' } else { c }).collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let s = s.trim_start_matches('.').trim();
    let s: String = s.chars().take(80).collect();
    let s = s.trim().to_string();
    if s.is_empty() { "Game".into() } else { s }
}

/// A value for a desktop entry's string key: one line, backslashes escaped (the Desktop Entry spec).
fn entry_value(v: &str) -> String {
    v.chars().filter(|c| !c.is_control()).collect::<String>().replace('\\', "\\\\")
}

/// Exactly what the launcher says.
pub fn contents(id: &str, title: &str, icon: &str, exec: &str) -> String {
    let name = entry_value(title.trim());
    let name = if name.is_empty() { "Game".to_string() } else { name };
    format!("[Desktop Entry]\nType=Application\nVersion=1.0\nName={name}\nComment=Play {name} with Hearth\nIcon={}\nExec={exec} --play-now {id}\n\
             Terminal=false\nCategories=Game;\nStartupNotify=false\n{MARK}={id}\n", entry_value(icon))
}

fn sha(b: &[u8]) -> String { Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect() }

/// The picture's file name for a game in Hearth's folder ("gog_1207658924.jpg").
pub fn icon_name(id: &str, ext: &str) -> String {
    let base: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '_' }).collect();
    format!("{base}.{ext}")
}

/// The art a desktop icon should show, best first: a square icon, then the cover, then the wide pictures.
pub fn art_order(art: &[(&'static str, String)]) -> Vec<String> {
    ["icon", "square", "cover", "portrait", "logo", "hero", "wide"].iter()
        .filter_map(|k| art.iter().find(|(kind, _)| kind == k).map(|(_, v)| v.clone())).collect()
}

/// The game's art copied into Hearth's folder (a file already on this device, the art cache's copy, or downloaded
/// now). None: no art could be had (the launcher uses Hearth Games' icon instead).
fn keep_picture(p: &Places, id: &str, art: &[(&'static str, String)]) -> Option<PathBuf> {
    for v in art_order(art) {
        let l = v.to_ascii_lowercase();
        let file = if l.starts_with("http://") || l.starts_with("https://") { crate::artcache::fetch_now(&v) }
            else { Some(PathBuf::from(v.strip_prefix("file://").unwrap_or(&v))).filter(|f| f.is_file()) };
        let Some(file) = file else { continue };
        let ext = file.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        if !["png", "jpg", "jpeg", "webp", "svg"].contains(&ext.as_str()) { continue; }
        if std::fs::create_dir_all(&p.data).is_err() { return None; }
        let to = p.data.join(icon_name(id, &ext));
        let tmp = p.data.join(format!(".{}.part", icon_name(id, &ext)));
        if std::fs::copy(&file, &tmp).is_ok() && std::fs::rename(&tmp, &to).is_ok() { return Some(to); }
        let _ = std::fs::remove_file(&tmp);
    }
    None
}

/// What Hearth wrote down for a game: {file, sha256, icon}.
fn made(p: &Places, id: &str) -> Value { crate::settings_file::read_json(&p.record())["games"][id].clone() }

/// Whether the file at `path` is the launcher Hearth made for `id`, untouched: a plain file (not a link), still byte
/// for byte what Hearth wrote (`want` is its SHA-256) and carrying the game's mark.
pub fn is_ours(path: &Path, id: &str, want: &str) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return false };
    if !meta.file_type().is_file() || meta.len() > 64 * 1024 { return false; }
    let Ok(bytes) = std::fs::read(path) else { return false };
    !want.is_empty() && sha(&bytes) == want && String::from_utf8_lossy(&bytes).lines().any(|l| l.trim() == format!("{MARK}={id}"))
}

/// Whether the game has its icon on the desktop now (Hearth's, still there).
pub fn on_desktop_in(p: &Places, id: &str) -> bool {
    let m = made(p, id);
    m["file"].as_str().map(|f| is_ours(Path::new(f), id, m["sha256"].as_str().unwrap_or(""))).unwrap_or(false)
}

/// Every game with Hearth's icon on the desktop now, and the ones waiting for one when they're installed.
pub fn status_in(p: &Places) -> Value {
    let rec = crate::settings_file::read_json(&p.record());
    let on: Vec<Value> = rec["games"].as_object().map(|o| o.iter().filter(|(id, m)|
        m["file"].as_str().map(|f| is_ours(Path::new(f), id, m["sha256"].as_str().unwrap_or(""))).unwrap_or(false))
        .map(|(id, _)| json!(id)).collect()).unwrap_or_default();
    let waiting: Vec<Value> = rec["onInstall"].as_object().map(|o| o.iter().filter(|(_, v)| **v == true).map(|(id, _)| json!(id)).collect()).unwrap_or_default();
    json!({"available": p.desktop.is_some(), "desktop": p.desktop.as_ref().map(|d| d.to_string_lossy().to_string()), "on": on, "onInstall": waiting})
}

/// A free name in the Desktop folder: "<stem>.desktop", else "<stem> 2.desktop"... (never a file that's there).
fn free_name(dir: &Path, stem: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.desktop"));
    if std::fs::symlink_metadata(&first).is_err() { return first; }
    (2..1000).map(|n| dir.join(format!("{stem} {n}.desktop"))).find(|f| std::fs::symlink_metadata(f).is_err())
        .unwrap_or_else(|| dir.join(format!("{stem} {}.desktop", crate::library::now_ms())))
}

/// metadata::trusted for desktops that read it (GNOME's); best effort, never waits more than a few seconds.
fn mark_trusted(file: &Path) {
    if cfg!(test) { return; }
    let Ok(mut c) = std::process::Command::new("gio").arg("set").arg(file).args(["metadata::trusted", "true"])
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn() else { return };
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_secs(3) {
        if let Ok(Some(_)) = c.try_wait() { return; }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let _ = c.kill();
    let _ = c.wait();
}

/// "Add to desktop": the launcher for this installed game. Already there (Hearth's, untouched): left as it is.
pub fn add_in(p: &Places, id: &str, title: &str, installed: bool, art: &[(&'static str, String)]) -> Result<Value, String> {
    if !id_ok(id) { return Err("Only GOG, Epic, Amazon and Battle.net games get a desktop icon here.".into()); }
    if !installed { return Err(format!("{title} isn't installed yet.")); }
    let dir = p.desktop.clone().ok_or("Your Desktop folder is set to your home folder, so Hearth doesn't add icons there.")?;
    if on_desktop_in(p, id) {
        let m = made(p, id);
        return Ok(json!({"ok": true, "already": true, "file": m["file"], "onDesktop": true}));
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't make your Desktop folder ({e})"))?;
    let icon = keep_picture(p, id, art).map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| FALLBACK_ICON.into());
    let text = contents(id, title, &icon, &p.exec);
    let file = free_name(&dir, &file_stem(title));
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        // create_new: never over a file that's there (even one that appeared just now).
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o755).open(&file)
            .map_err(|e| format!("Couldn't put the icon on your desktop ({e})"))?;
        f.write_all(text.as_bytes()).and_then(|_| f.sync_all()).map_err(|e| { let _ = std::fs::remove_file(&file); format!("Couldn't put the icon on your desktop ({e})") })?;
    }
    // Executable for its owner and readable by all (KDE's "trusted launcher" rule), whatever the umask made it.
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755));
    mark_trusted(&file);
    let f = file.to_string_lossy().to_string();
    let rec = json!({"file": f, "sha256": sha(text.as_bytes()), "icon": icon, "title": title, "at": crate::library::now_ms()});
    crate::settings_file::update_json(&p.record(), |m| {
        crate::store::object_in(m, "games").insert(id.to_string(), rec.clone());
        if let Some(o) = m.get_mut("onInstall").and_then(Value::as_object_mut) { o.remove(id); }
    }).map_err(|e| { let _ = std::fs::remove_file(&file); format!("Couldn't note the icon ({e})") })?;
    crate::bridge::log(&format!("desktop icons: {id} ({title}) added as {f}"));
    Ok(json!({"ok": true, "file": f, "onDesktop": true}))
}

/// "Remove from desktop" (and uninstalling): takes off only the launcher Hearth made, only while it's untouched.
/// Ok({removed, kept?}): kept says why a file was left (you changed it, or it isn't Hearth's any more).
pub fn remove_in(p: &Places, id: &str) -> Result<Value, String> {
    let m = made(p, id);
    let mut out = json!({"ok": true, "removed": false, "onDesktop": false});
    if let Some(f) = m["file"].as_str() {
        let path = Path::new(f);
        if is_ours(path, id, m["sha256"].as_str().unwrap_or("")) {
            std::fs::remove_file(path).map_err(|e| format!("Couldn't take the icon off your desktop ({e})"))?;
            out["removed"] = json!(true);
            crate::bridge::log(&format!("desktop icons: {id} removed {f}"));
        } else if std::fs::symlink_metadata(path).is_ok() {
            out["kept"] = json!(format!("{} was changed after Hearth made it, so it's left on your desktop.", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
            crate::bridge::log(&format!("desktop icons: {id}: {f} isn't the file Hearth made any more; left as it is"));
        }
    }
    // Hearth's own copy of the picture, only from Hearth's own folder.
    if let Some(icon) = m["icon"].as_str().map(PathBuf::from) {
        let base = icon_name(id, "x");
        let base = base.trim_end_matches(".x");
        if icon.parent() == Some(p.data.as_path()) && icon.file_stem().map(|n| n.to_string_lossy() == base).unwrap_or(false) {
            let _ = std::fs::remove_file(&icon);
        }
    }
    if !m.is_null() || !crate::settings_file::read_json(&p.record())["onInstall"][id].is_null() {
        crate::settings_file::update_json(&p.record(), |r| {
            if let Some(o) = r.get_mut("games").and_then(Value::as_object_mut) { o.remove(id); }
            if let Some(o) = r.get_mut("onInstall").and_then(Value::as_object_mut) { o.remove(id); }
        }).map_err(|e| format!("Couldn't note it ({e})"))?;
    }
    Ok(out)
}

/// The "Add a desktop icon" tick box when installing: the icon is made once the install is done (by whichever part of
/// Hearth finishes it: Hearth Games, Game Mode, Steam Game Mode's core service).
pub fn set_on_install_in(p: &Places, id: &str, on: bool) -> Result<(), String> {
    if !id_ok(id) { return Err("That isn't a GOG, Epic, Amazon or Battle.net game".into()); }
    crate::settings_file::update_json(&p.record(), |m| {
        if on { crate::store::object_in(m, "onInstall").insert(id.to_string(), json!(true)); }
        else if let Some(o) = m.get_mut("onInstall").and_then(Value::as_object_mut) { o.remove(id); }
    }).map(|_| ()).map_err(|e| e.to_string())
}

/// A game finished installing: its icon, if the tick box asked for one.
pub fn installed_in(p: &Places, g: &crate::library::Game) -> Option<Value> {
    if crate::settings_file::read_json(&p.record())["onInstall"][&g.id] != true { return None; }
    let r = add_in(p, &g.id, &g.title, g.installed, &g.art);
    if let Err(e) = &r { crate::bridge::log(&format!("desktop icons: {} installed, but its icon couldn't be made: {e}", g.id)); }
    r.ok()
}

// ---- the real places
pub fn add(g: &crate::library::Game) -> Result<Value, String> { add_in(&Places::real(), &g.id, &g.title, g.installed, &g.art) }
pub fn remove(id: &str) -> Result<Value, String> { remove_in(&Places::real(), id) }
pub fn status() -> Value { status_in(&Places::real()) }
pub fn set_on_install(id: &str, on: bool) -> Result<(), String> { set_on_install_in(&Places::real(), id, on) }
/// Library::install_done: nothing to do unless the tick box asked (reads one small file).
pub fn installed(g: &crate::library::Game) -> Option<Value> {
    if !id_ok(&g.id) { return None; }
    installed_in(&Places::real(), g)
}
/// Library::uninstall: the game is gone, so is Hearth's icon for it (only Hearth's, untouched).
pub fn game_gone(id: &str) {
    if !id_ok(id) { return; }
    let p = Places::real();
    if !p.record().exists() { return; }
    if let Err(e) = remove_in(&p, id) { crate::bridge::log(&format!("desktop icons: {id} uninstalled, icon not removed: {e}")); }
}

/// Hearth Games' own choice for the install tick box (~/.config/nimbus/hearth-games.json "desktopIconOnInstall"),
/// written only when you change it. Unset: off (the owner decides whether it starts on).
pub fn tick_default_in(cfg: &Path) -> bool { crate::settings_file::read_json(&cfg.join("hearth-games.json"))["desktopIconOnInstall"] == true }
pub fn set_tick_default_in(cfg: &Path, on: bool) -> Result<(), String> {
    let mut m = Map::new();
    m.insert("desktopIconOnInstall".into(), json!(on));
    crate::settings_file::merge_json(&cfg.join("hearth-games.json"), &m).map(|_| ()).map_err(|e| e.to_string())
}
pub fn tick_default() -> bool { tick_default_in(&crate::paths::config_dir()) }
pub fn set_tick_default(on: bool) -> Result<(), String> { set_tick_default_in(&crate::paths::config_dir(), on) }

#[cfg(test)]
mod tests {
    use super::*;

    fn places(t: &Path) -> Places {
        Places { desktop: Some(t.join("home/Desktop")), data: t.join("data/desktop-icons"), exec: "nimbus".into() }
    }

    #[test]
    fn the_desktop_folder_as_xdg_user_dir_gives_it() {
        let h = Path::new("/home/chris");
        assert_eq!(desktop_dir(h, None, None), Some(h.join("Desktop")));
        assert_eq!(desktop_dir(h, Some("# comment\nXDG_DESKTOP_DIR=\"$HOME/Schreibtisch\"\n"), None), Some(h.join("Schreibtisch")));
        assert_eq!(desktop_dir(h, Some("XDG_DESKTOP_DIR=\"/data/desk\"\n"), None), Some(PathBuf::from("/data/desk")));
        assert_eq!(desktop_dir(h, None, Some("/srv/d")), Some(PathBuf::from("/srv/d")), "the variable when the file says nothing");
        assert_eq!(desktop_dir(h, Some("XDG_DESKTOP_DIR=\"$HOME/D\"\n"), Some("/srv/d")), Some(h.join("D")), "the file wins, as in xdg-user-dir");
        assert_eq!(desktop_dir(h, Some("XDG_DESKTOP_DIR=\"$HOME/\"\n"), None), None, "home itself: nothing is added");
        assert_eq!(desktop_dir(h, Some("XDG_DESKTOP_DIR=\"$HOME\"\n"), None), None);
    }

    #[test]
    fn the_launcher_says_exactly_this() {
        let c = contents("gog:1207658924", "Witcher Tales", "/home/chris/.local/share/nimbus/desktop-icons/gog_1207658924.jpg", "nimbus");
        assert_eq!(c, "[Desktop Entry]\nType=Application\nVersion=1.0\nName=Witcher Tales\nComment=Play Witcher Tales with Hearth\n\
                       Icon=/home/chris/.local/share/nimbus/desktop-icons/gog_1207658924.jpg\nExec=nimbus --play-now gog:1207658924\n\
                       Terminal=false\nCategories=Game;\nStartupNotify=false\nX-Hearth-Game=gog:1207658924\n");
        // A name can't add lines or keys; backslashes are escaped.
        let c = contents("epic:Sugar", "Bad\nExec=rm -rf ~ \\o/", "hearth-gamestore", "nimbus");
        assert!(c.contains("Name=BadExec=rm -rf ~ \\\\o/\n"), "{c}");
        assert_eq!(c.lines().filter(|l| l.starts_with("Exec=")).count(), 1);
        assert!(c.contains("\nExec=nimbus --play-now epic:Sugar\n"));
        assert!(c.contains("\nIcon=hearth-gamestore\n"));
        // Only Hearth Games' stores, only ids the Exec line can carry as one word.
        assert!(id_ok("gog:1") && id_ok("epic:Fortnite") && id_ok("amazon:amzn1.adg.product.x-y") && id_ok("battlenet:wow"));
        assert!(!id_ok("steam:620") && !id_ok("manual:x") && !id_ok("gog:") && !id_ok("gog:a b") && !id_ok("gog:a;b") && !id_ok("gog:$(x)") && !id_ok("gog:'x'"));
        assert_eq!(file_stem("Baldur's Gate 3: Deluxe/Edition"), "Baldur's Gate 3: Deluxe Edition");
        assert_eq!(file_stem("..hidden"), "hidden");
        assert_eq!(file_stem("  \n "), "Game");
        assert_eq!(file_stem(&"x".repeat(300)).len(), 80);
        assert_eq!(art_order(&[("hero", "h".into()), ("cover", "c".into()), ("icon", "i".into())]), vec!["i", "c", "h"]);
    }

    #[test]
    fn add_makes_a_trusted_launcher_with_the_games_art() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        let p = places(t.path());
        let cover = t.path().join("cover.png");
        std::fs::write(&cover, b"\x89PNG fake").unwrap();
        assert!(add_in(&p, "gog:1", "Witcher Tales", false, &[]).unwrap_err().contains("isn't installed"));
        assert!(add_in(&p, "steam:620", "Portal 2", true, &[]).is_err());
        let r = add_in(&p, "gog:1", "Witcher Tales", true, &[("cover", cover.to_string_lossy().to_string())]).unwrap();
        let f = PathBuf::from(r["file"].as_str().unwrap());
        assert_eq!(f, t.path().join("home/Desktop/Witcher Tales.desktop"));
        let text = std::fs::read_to_string(&f).unwrap();
        let icon = p.data.join("gog_1.png");
        assert!(icon.is_file(), "the art is kept in Hearth's folder");
        assert_eq!(text, contents("gog:1", "Witcher Tales", &icon.to_string_lossy(), "nimbus"));
        assert_eq!(std::fs::metadata(&f).unwrap().permissions().mode() & 0o777, 0o755, "executable: Plasma opens it without asking");
        assert!(on_desktop_in(&p, "gog:1"));
        assert_eq!(status_in(&p)["on"], json!(["gog:1"]));
        // A second Add leaves it as it is.
        assert_eq!(add_in(&p, "gog:1", "Witcher Tales", true, &[]).unwrap()["already"], true);
        assert_eq!(std::fs::read_dir(t.path().join("home/Desktop")).unwrap().count(), 1);
        // No art: Hearth Games' icon.
        let r = add_in(&p, "epic:b", "Neon Drift", true, &[("cover", "/nowhere/x.png".into())]).unwrap();
        assert!(std::fs::read_to_string(r["file"].as_str().unwrap()).unwrap().contains("\nIcon=hearth-gamestore\n"));
        // No Desktop folder to use: nothing written.
        let none = Places { desktop: None, ..p.clone() };
        assert!(add_in(&none, "gog:2", "Pixel Keep", true, &[]).unwrap_err().contains("home folder"));
    }

    #[test]
    fn a_file_of_yours_is_never_overwritten_or_removed() {
        let t = tempfile::tempdir().unwrap();
        let p = places(t.path());
        let desk = t.path().join("home/Desktop");
        std::fs::create_dir_all(&desk).unwrap();
        // Your own launcher with the game's name, and one of yours that even carries Hearth's mark.
        let yours = desk.join("Witcher Tales.desktop");
        std::fs::write(&yours, "[Desktop Entry]\nName=My Witcher\nExec=/opt/witcher\n").unwrap();
        let copy = desk.join("copied.desktop");
        std::fs::write(&copy, contents("gog:1", "Witcher Tales", "x", "nimbus")).unwrap();
        let r = add_in(&p, "gog:1", "Witcher Tales", true, &[]).unwrap();
        assert_eq!(r["file"].as_str().unwrap(), desk.join("Witcher Tales 2.desktop").to_string_lossy());
        assert_eq!(std::fs::read_to_string(&yours).unwrap(), "[Desktop Entry]\nName=My Witcher\nExec=/opt/witcher\n", "yours untouched");
        // Remove takes off only Hearth's.
        let r = remove_in(&p, "gog:1").unwrap();
        assert_eq!(r["removed"], true);
        assert!(!desk.join("Witcher Tales 2.desktop").exists());
        assert!(yours.exists() && copy.exists(), "your files stay");
        assert!(!on_desktop_in(&p, "gog:1"));
        assert!(!p.data.join("gog_1.png").exists());
        // Removing again, or for a game Hearth never made an icon for: nothing happens.
        assert_eq!(remove_in(&p, "gog:1").unwrap()["removed"], false);
        assert_eq!(remove_in(&p, "gog:9").unwrap()["removed"], false);
        assert!(yours.exists() && copy.exists());

        // You changed Hearth's icon (renamed it in the file): it's yours now, left on the desktop.
        let r = add_in(&p, "gog:1", "Witcher Tales", true, &[]).unwrap();
        let f = PathBuf::from(r["file"].as_str().unwrap());
        let edited = std::fs::read_to_string(&f).unwrap().replace("Name=Witcher Tales", "Name=Witcher (mine)");
        std::fs::write(&f, edited).unwrap();
        let r = remove_in(&p, "gog:1").unwrap();
        assert_eq!(r["removed"], false);
        assert!(r["kept"].as_str().unwrap().contains("changed after Hearth made it"));
        assert!(f.exists());

        // Your file put where Hearth's was (Hearth's deleted, yours saved under its name): left.
        let r = add_in(&p, "epic:b", "Neon Drift", true, &[]).unwrap();
        let f = PathBuf::from(r["file"].as_str().unwrap());
        std::fs::remove_file(&f).unwrap();
        std::fs::write(&f, "[Desktop Entry]\nName=Neon Drift\nExec=/home/me/neon.sh\n").unwrap();
        assert_eq!(remove_in(&p, "epic:b").unwrap()["removed"], false);
        assert!(f.exists());

        // A link where Hearth's was (to a file with exactly Hearth's text): links are never followed or removed.
        let r = add_in(&p, "amazon:c", "Rivet City", true, &[]).unwrap();
        let f = PathBuf::from(r["file"].as_str().unwrap());
        let real = t.path().join("elsewhere.desktop");
        std::fs::rename(&f, &real).unwrap();
        std::os::unix::fs::symlink(&real, &f).unwrap();
        assert_eq!(remove_in(&p, "amazon:c").unwrap()["removed"], false);
        assert!(real.exists() && std::fs::symlink_metadata(&f).is_ok());
    }

    #[test]
    fn the_install_tick_box_and_uninstalling() {
        let t = tempfile::tempdir().unwrap();
        let p = places(t.path());
        let g = |installed| crate::library::Game { id: "gog:5".into(), title: "Lantern Road".into(), source: "gog", installed,
            state: if installed { "installed" } else { "notInstalled" }, ..Default::default() };
        // Not asked: installing makes nothing.
        assert!(installed_in(&p, &g(true)).is_none());
        assert!(!t.path().join("home/Desktop").exists());
        set_on_install_in(&p, "gog:5", true).unwrap();
        assert_eq!(status_in(&p)["onInstall"], json!(["gog:5"]));
        let r = installed_in(&p, &g(true)).unwrap();
        assert!(Path::new(r["file"].as_str().unwrap()).is_file());
        assert_eq!(status_in(&p)["onInstall"], json!([]), "asked once");
        // Uninstalled: Hearth's icon goes; the record is cleared.
        remove_in(&p, "gog:5").unwrap();
        assert!(!Path::new(r["file"].as_str().unwrap()).exists());
        assert_eq!(status_in(&p)["on"], json!([]));
        // The tick box turned off before the install finished: nothing.
        set_on_install_in(&p, "gog:5", true).unwrap();
        set_on_install_in(&p, "gog:5", false).unwrap();
        assert!(installed_in(&p, &g(true)).is_none());
        // Its remembered choice: off until you change it, then what you chose.
        let cfg = t.path().join("cfg");
        assert!(!tick_default_in(&cfg));
        assert!(!cfg.join("hearth-games.json").exists(), "reading writes nothing");
        set_tick_default_in(&cfg, true).unwrap();
        assert!(tick_default_in(&cfg));
        std::fs::write(cfg.join("hearth-games.json"), "{\"other\": 1, \"desktopIconOnInstall\": true}").unwrap();
        set_tick_default_in(&cfg, false).unwrap();
        let v = crate::settings_file::read_json(&cfg.join("hearth-games.json"));
        assert_eq!(v, json!({"other": 1, "desktopIconOnInstall": false}), "merged: nothing else changes");
    }
}

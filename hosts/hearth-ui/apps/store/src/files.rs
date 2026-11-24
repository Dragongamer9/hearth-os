//! Installing, updating, checking, repairing and uninstalling, all in your home folder (docs/HEARTH-STORE.md):
//!
//! * install: the release's AppImage is downloaded as a hidden file in the app's folder
//!   (~/.local/share/hearth/store/apps/<app>/), checked against the release's SHA-256 (deleted if it doesn't
//!   match), then renamed to its place, with its menu entry and icon;
//! * update: the same download next to the old file, which keeps working meanwhile; then one rename puts the new one
//!   in place (the old one is kept as <file>.previous, by a second name for the same file, until the new version has
//!   started fine), so the app is never left half updated;
//! * repair: the installed version checked against its release's SHA-256, that it may run, that AppImages can start
//!   (FUSE), its menu entry and icon; only what's wrong is fixed (the same version downloaded again and checked);
//! * uninstall: its folder, menu entry and icon go; its settings and data (the app's own folders) stay, or go to the
//!   Trash (never deleted) when "Keep my settings and data" is turned off.
//!
//! Never touches the system, boot files, sessions or services, and never an app's settings.

use crate::catalog::{Entry, Kind};
use crate::paths;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// What the Store installed (store.json in the app's folder).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Installed {
    pub version: String,
    /// The AppImage's name in the folder (Entry::file).
    pub file: String,
    pub sha256: String,
    pub size: u64,
    pub url: String,
    pub page: String,
    pub when: i64,
    /// The file as the Store left it (size, modified): when these change, the app has updated itself.
    pub fsize: u64,
    pub mtime: i64,
}

pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Written whole or not at all: a temporary file beside it, flushed, then renamed over it.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".{}.tmp-{}", path.file_name().and_then(|n| n.to_str()).unwrap_or("file"), std::process::id()));
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

pub fn app_file(e: &Entry) -> PathBuf {
    paths::app_dir(&e.id).join(&e.file)
}
pub fn previous_file(e: &Entry) -> PathBuf {
    paths::app_dir(&e.id).join(format!("{}.previous", e.file))
}
/// Where a download goes first (hidden, next to the app).
pub fn part_file(e: &Entry) -> PathBuf {
    paths::app_dir(&e.id).join(format!(".{}.download", e.file))
}
fn state_file(id: &str) -> PathBuf {
    paths::app_dir(id).join("store.json")
}

fn stat(p: &Path) -> Option<(u64, i64)> {
    let m = std::fs::metadata(p).ok()?;
    let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
    Some((m.len(), t))
}

pub fn read_installed(id: &str) -> Option<Installed> {
    let v: Value = serde_json::from_slice(&std::fs::read(state_file(id)).ok()?).ok()?;
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let n = |k: &str| v.get(k).and_then(Value::as_i64).unwrap_or(0);
    let version = s("version");
    if version.is_empty() {
        return None;
    }
    Some(Installed { version, file: s("file"), sha256: s("sha256"), size: n("size") as u64, url: s("url"), page: s("page"), when: n("when"), fsize: n("fsize") as u64, mtime: n("mtime") })
}

pub fn write_installed(id: &str, i: &Installed) -> std::io::Result<()> {
    let v = json!({"version": i.version, "file": i.file, "sha256": i.sha256, "size": i.size, "url": i.url, "page": i.page, "when": i.when, "fsize": i.fsize, "mtime": i.mtime});
    write_atomic(&state_file(id), serde_json::to_string_pretty(&v).unwrap_or_default().as_bytes())
}

/// The file isn't the one the Store left (it updated itself, or something changed it).
pub fn changed_since(e: &Entry, i: &Installed) -> bool {
    match stat(&app_file(e)) {
        Some((size, mtime)) => size != i.fsize || mtime != i.mtime,
        None => false,
    }
}

/// Its SHA-256 (hex), read in 1 MB pieces.
pub fn sha256_file(p: &Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    let mut f = std::fs::File::open(p)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// The download matches the release's SHA-256. If it doesn't (or it can't be read), it's deleted: never used.
pub fn check_download(p: &Path, expected: &str) -> Result<(), String> {
    if expected.len() != 64 {
        let _ = std::fs::remove_file(p);
        return Err("The release has no SHA-256 for this file, so it wasn't used.".into());
    }
    match sha256_file(p) {
        Ok(h) if h.eq_ignore_ascii_case(expected) => Ok(()),
        Ok(_) => {
            let _ = std::fs::remove_file(p);
            Err("The download didn't match the release's SHA-256 list, so it was deleted. Nothing changed.".into())
        }
        Err(e) => {
            let _ = std::fs::remove_file(p);
            Err(format!("The download couldn't be read ({e}), so it was deleted. Nothing changed."))
        }
    }
}

/// The checked download takes the app's place in one step (rename). The file in use before is kept as .previous
/// (a second name for the same file, made before the rename) until the new version has started fine.
pub fn put_in_place(e: &Entry, downloaded: &Path) -> std::io::Result<bool> {
    let target = app_file(e);
    let prev = previous_file(e);
    std::fs::set_permissions(downloaded, std::fs::Permissions::from_mode(0o755))?;
    let kept = if target.exists() {
        let _ = std::fs::remove_file(&prev);
        std::fs::hard_link(&target, &prev).or_else(|_| std::fs::copy(&target, &prev).map(|_| ()))?;
        true
    } else {
        false
    };
    std::fs::rename(downloaded, &target)?;
    Ok(kept)
}

/// After the new version started fine: the old one goes.
pub fn drop_previous(e: &Entry) {
    let _ = std::fs::remove_file(previous_file(e));
}

/// Records what's installed now (the file as it is, so a later change is noticed).
pub fn record(e: &Entry, version: &str, sha256: &str, size: u64, url: &str, page: &str) -> std::io::Result<Installed> {
    let (fsize, mtime) = stat(&app_file(e)).unwrap_or((size, 0));
    let i = Installed { version: version.into(), file: e.file.clone(), sha256: sha256.to_lowercase(), size, url: url.into(), page: page.into(), when: now(), fsize, mtime };
    write_installed(&e.id, &i)?;
    Ok(i)
}

// ------------------------------------------------------------------ the menu entry and icon
/// AppImages start with FUSE 2 (in the Fedora image); without it they're unpacked to run.
pub fn fuse_ok() -> bool {
    if let Some(f) = paths::fixture() {
        return f.join("fuse").exists();
    }
    let lib = ["/usr/lib64/libfuse.so.2", "/lib64/libfuse.so.2", "/usr/lib/x86_64-linux-gnu/libfuse.so.2", "/usr/lib/libfuse.so.2"].iter().any(|p| Path::new(p).exists());
    lib && Path::new("/dev/fuse").exists()
}

/// A path as the Exec line wants it (quoted, with " ` $ \ escaped).
fn exec_quote(p: &Path) -> String {
    let s = p.to_string_lossy();
    let mut o = String::from("\"");
    for c in s.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            o.push('\\');
        }
        o.push(c);
    }
    o.push('"');
    o
}

fn one_line(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
}

pub fn desktop_text(e: &Entry, fuse: bool) -> String {
    let exe = app_file(e);
    let exec = if fuse { format!("{} %U", exec_quote(&exe)) } else { format!("env APPIMAGE_EXTRACT_AND_RUN=1 {} %U", exec_quote(&exe)) };
    format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec={}\nTryExec={}\nIcon={}\nCategories=Utility;\nTerminal=false\nX-Hearth-Store={}\n",
        one_line(&e.name),
        one_line(if e.tagline.is_empty() { &e.card } else { &e.tagline }),
        exec,
        exe.display(),
        paths::icon_file(&e.id).display(),
        e.id
    )
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn colour(c: &str, d: &str) -> String {
    let ok = c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit());
    if ok { c.to_string() } else { d.to_string() }
}

/// The app's icon as the Store draws it: a rounded square in its two colours with its letters.
pub fn icon_svg(e: &Entry) -> String {
    let letters: String = e.icon.chars().take(3).collect();
    let size = match letters.chars().count() { 0 | 1 => 30, 2 => 24, _ => 19 };
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"64\" viewBox=\"0 0 64 64\"><defs><linearGradient id=\"g\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\"><stop offset=\"0\" stop-color=\"{}\"/><stop offset=\"1\" stop-color=\"{}\"/></linearGradient></defs><rect x=\"2\" y=\"2\" width=\"60\" height=\"60\" rx=\"14\" fill=\"url(#g)\"/><text x=\"32\" y=\"{}\" text-anchor=\"middle\" font-family=\"Inter, sans-serif\" font-weight=\"800\" font-size=\"{}\" fill=\"#ffffff\">{}</text></svg>\n",
        colour(&e.from, "#ff8a3d"),
        colour(&e.to, "#d9531e"),
        32 + size * 36 / 100,
        size,
        xml(&letters)
    )
}

/// The menu entry and icon, written again (install, update, repair).
pub fn write_menu(e: &Entry) -> std::io::Result<()> {
    write_atomic(&paths::icon_file(&e.id), icon_svg(e).as_bytes())?;
    write_atomic(&paths::desktop_file(&e.id), desktop_text(e, fuse_ok()).as_bytes())
}

// ------------------------------------------------------------------ check and repair
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrong {
    /// The AppImage isn't the release's (its SHA-256 doesn't match).
    Changed,
    /// It's gone.
    Missing,
    /// It isn't allowed to run (not executable).
    NotRunnable,
    MenuMissing,
    IconMissing,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Check {
    /// The app against its release's SHA-256: Some(true) matches, Some(false) doesn't, None no SHA-256 known.
    pub matches: Option<bool>,
    pub runnable: bool,
    pub menu: bool,
    pub icon: bool,
    pub fuse: bool,
    /// (what, how it's wrong): what's shown in "What's wrong".
    pub wrong: Vec<(String, Wrong)>,
}

impl Check {
    pub fn needs_download(&self) -> bool {
        self.wrong.iter().any(|(_, w)| matches!(w, Wrong::Changed | Wrong::Missing))
    }
    pub fn menu_fixes(&self) -> usize {
        self.wrong.iter().filter(|(_, w)| matches!(w, Wrong::MenuMissing | Wrong::IconMissing)).count()
    }
}

/// Checks the installed app (reads it whole for its SHA-256: run it off the window's thread).
pub fn check(e: &Entry, expected_sha: &str) -> Check {
    let f = app_file(e);
    let mut c = Check { fuse: fuse_ok(), ..Default::default() };
    match std::fs::metadata(&f) {
        Err(_) => {
            c.matches = Some(false);
            c.wrong.push((e.file.clone(), Wrong::Missing));
        }
        Ok(m) => {
            c.matches = if expected_sha.len() == 64 { Some(sha256_file(&f).map(|h| h.eq_ignore_ascii_case(expected_sha)).unwrap_or(false)) } else { None };
            if c.matches == Some(false) {
                c.wrong.push((e.file.clone(), Wrong::Changed));
            }
            c.runnable = m.permissions().mode() & 0o100 != 0;
            if !c.runnable {
                c.wrong.push(("The same AppImage".into(), Wrong::NotRunnable));
            }
        }
    }
    let want = desktop_text(e, c.fuse);
    c.menu = std::fs::read_to_string(paths::desktop_file(&e.id)).map(|t| t == want).unwrap_or(false);
    if !c.menu {
        c.wrong.push((format!("hearth-store-{}.desktop", e.id), Wrong::MenuMissing));
    }
    c.icon = paths::icon_file(&e.id).exists();
    if !c.icon {
        c.wrong.push((format!("hearth-store-{}.svg", e.id), Wrong::IconMissing));
    }
    c
}

/// The fixes that need no download: allowed to run again, the menu entry and icon written again.
pub fn fix_local(e: &Entry) -> std::io::Result<()> {
    let f = app_file(e);
    if f.exists() {
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755))?;
    }
    write_menu(e)
}

// ------------------------------------------------------------------ uninstall
/// The app's own folders (its settings and data) that are there.
pub fn data_dirs(e: &Entry) -> Vec<PathBuf> {
    e.data
        .iter()
        .map(|d| paths::expand(d))
        .filter(|p| p.starts_with(hearth_ui::paths::home()) && p != &hearth_ui::paths::home() && p.components().all(|c| !matches!(c, std::path::Component::ParentDir)) && p.exists())
        .collect()
}

/// How much its settings and data take (at most 200 000 files are counted).
pub fn data_size(e: &Entry) -> u64 {
    let mut total = 0u64;
    let mut n = 0usize;
    let mut stack: Vec<PathBuf> = data_dirs(e);
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for x in rd.flatten() {
            n += 1;
            if n > 200_000 {
                return total;
            }
            let Ok(m) = x.metadata() else { continue };
            if m.is_dir() {
                stack.push(x.path());
            } else {
                total += m.len();
            }
        }
    }
    total
}

fn url_path(p: &Path) -> String {
    let mut o = String::new();
    for b in p.to_string_lossy().bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

/// Moves `p` to the Trash (freedesktop.org's, as Files and Dolphin do): never deleted, can be put back from there.
pub fn trash(p: &Path) -> std::io::Result<PathBuf> {
    let t = paths::trash();
    let (files, info) = (t.join("files"), t.join("info"));
    std::fs::create_dir_all(&files)?;
    std::fs::create_dir_all(&info)?;
    let base = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "item".into());
    let mut name = base.clone();
    let mut k = 2;
    while files.join(&name).exists() || info.join(format!("{name}.trashinfo")).exists() {
        name = format!("{base}.{k}");
        k += 1;
    }
    let tm = hearth_ui::time::local(hearth_ui::time::now());
    let date = format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", tm.year, tm.mon + 1, tm.day, tm.hour, tm.min, tm.sec);
    let abs = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    write_atomic(&info.join(format!("{name}.trashinfo")), format!("[Trash Info]\nPath={}\nDeletionDate={date}\n", url_path(&abs)).as_bytes())?;
    let dest = files.join(&name);
    if let Err(err) = std::fs::rename(p, &dest) {
        let _ = std::fs::remove_file(info.join(format!("{name}.trashinfo")));
        return Err(err);
    }
    Ok(dest)
}

/// Uninstalls: the app's folder (its AppImage, the one kept from before, store.json), its menu entry and icon. With
/// `keep`, its settings and data stay where they are; without, they go to the Trash. Returns what went to the Trash.
pub fn uninstall(e: &Entry, keep: bool) -> std::io::Result<Vec<PathBuf>> {
    let dir = paths::app_dir(&e.id);
    if dir.starts_with(paths::apps()) && dir != paths::apps() && dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    let _ = std::fs::remove_file(paths::desktop_file(&e.id));
    let _ = std::fs::remove_file(paths::icon_file(&e.id));
    let mut moved = Vec::new();
    if !keep {
        for d in data_dirs(e) {
            moved.push(trash(&d)?);
        }
    }
    Ok(moved)
}

// ------------------------------------------------------------------ is it running?
/// The app's processes: those started from its AppImage (the AppImage's runtime sets APPIMAGE=<its path>). Only this
/// user's processes can be read, which is all that matters.
pub fn running(e: &Entry) -> Vec<i32> {
    let want = format!("APPIMAGE={}", app_file(e).display());
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir("/proc") else { return out };
    for x in rd.flatten() {
        let Some(pid) = x.file_name().to_str().and_then(|s| s.parse::<i32>().ok()) else { continue };
        let Ok(env) = std::fs::read(x.path().join("environ")) else { continue };
        if env.split(|b| *b == 0).any(|v| v == want.as_bytes()) {
            out.push(pid);
        }
    }
    out
}

/// Asks the app to close (as closing its window does: it saves first), waiting up to `secs`. True: it closed.
pub fn close(e: &Entry, secs: u64) -> bool {
    let pids = running(e);
    for p in &pids {
        // SAFETY: a signal to a process of this user, found above.
        unsafe { libc::kill(*p, libc::SIGTERM) };
    }
    let until = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    while std::time::Instant::now() < until {
        if running(e).is_empty() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    running(e).is_empty()
}

/// Starts the installed app (from its menu entry's command), not waiting for it.
pub fn start(e: &Entry) -> bool {
    let f = app_file(e);
    let mut c = std::process::Command::new(&f);
    if !fuse_ok() {
        c.env("APPIMAGE_EXTRACT_AND_RUN", "1");
    }
    c.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().is_ok()
}

// ------------------------------------------------------------------ Settings › Where apps go › Change…
/// A folder moved as it is: renamed when it's on the same drive, else copied (permissions and times kept), every
/// file checked against the original, and only then the original removed.
fn move_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    let tmp = dst.with_file_name(format!(".{}.moving", dst.file_name().and_then(|n| n.to_str()).unwrap_or("app")));
    let _ = std::fs::remove_dir_all(&tmp);
    let r = copy_tree(src, &tmp).and_then(|_| same_tree(src, &tmp)).and_then(|_| std::fs::rename(&tmp, dst));
    if let Err(e) = r {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e);
    }
    std::fs::remove_dir_all(src)
}

fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for x in std::fs::read_dir(src)? {
        let x = x?;
        let (from, to) = (x.path(), dst.join(x.file_name()));
        let ft = x.file_type()?;
        if ft.is_symlink() {
            std::os::unix::fs::symlink(std::fs::read_link(&from)?, &to)?;
        } else if ft.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
            let m = std::fs::metadata(&from)?;
            std::fs::File::options().write(true).open(&to)?.set_modified(m.modified()?)?;
        }
    }
    Ok(())
}

/// Every file in `b` is the same as in `a` (sizes and SHA-256).
fn same_tree(a: &Path, b: &Path) -> std::io::Result<()> {
    for x in std::fs::read_dir(a)? {
        let x = x?;
        let (pa, pb) = (x.path(), b.join(x.file_name()));
        let ft = x.file_type()?;
        if ft.is_dir() {
            same_tree(&pa, &pb)?;
        } else if ft.is_file() && (std::fs::metadata(&pb)?.len() != x.metadata()?.len() || sha256_file(&pa)? != sha256_file(&pb)?) {
            return Err(std::io::Error::other(format!("{} didn't copy right", pa.display())));
        }
    }
    Ok(())
}

/// Moves every app the Store installed into `to` (a folder in the home folder), then remembers it and points the
/// menu entries there. Nothing is lost on the way: an app that's open stops it before anything moves, and if any
/// step fails, what was moved goes back and the setting isn't changed.
pub fn move_apps(apps: &[Entry], to: &Path) -> Result<String, String> {
    let from = paths::apps();
    let home = hearth_ui::paths::home();
    if !to.is_absolute() || !to.starts_with(&home) || to == home {
        return Err("Pick a folder inside your home folder: the Store never puts apps anywhere else.".into());
    }
    if to == from || to.starts_with(&from) {
        return Err("That's where your apps already are.".into());
    }
    let installed: Vec<&Entry> = apps.iter().filter(|e| e.kind == Kind::Install && read_installed(&e.id).is_some()).collect();
    if let Some(e) = installed.iter().find(|e| !running(e).is_empty()) {
        return Err(format!("Close {} first: an app can't move while it's open. Nothing was moved.", e.name));
    }
    std::fs::create_dir_all(to).map_err(|e| format!("Couldn't make {}: {e}", paths::tilde(to)))?;
    let mut names = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&from) {
        for x in rd.flatten() {
            if x.file_type().map(|t| t.is_dir()).unwrap_or(false) && !x.file_name().to_string_lossy().starts_with('.') {
                names.push(x.file_name());
            }
        }
    }
    if let Some(n) = names.iter().find(|n| to.join(n).exists()) {
        return Err(format!("{} already has a folder called {}; pick another one. Nothing was moved.", paths::tilde(to), n.to_string_lossy()));
    }
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    let undo = |moved: &[(PathBuf, PathBuf)]| {
        for (a, b) in moved.iter().rev() {
            let _ = move_dir(b, a);
        }
    };
    for n in &names {
        let (a, b) = (from.join(n), to.join(n));
        if let Err(e) = move_dir(&a, &b) {
            undo(&moved);
            return Err(format!("{} couldn't move ({e}); everything is back where it was.", n.to_string_lossy()));
        }
        moved.push((a, b));
    }
    if let Err(e) = crate::prefs::set_apps_dir(to) {
        undo(&moved);
        crate::paths::set_apps(None);
        return Err(format!("{e} Everything is back where it was."));
    }
    for e in &installed {
        let _ = write_menu(e);
    }
    let _ = std::fs::remove_dir(&from);
    Ok(format!("Your apps are in {} now{}.", paths::tilde(to), if names.is_empty() { "; new ones go there too".to_string() } else { format!(" ({} moved)", names.len()) }))
}

/// The folder picker: Hearth's file chooser, else KDE's or GTK's.
pub fn pick_folder() -> Option<PathBuf> {
    for cmd in [vec!["hearth-file-chooser", "--folder"], vec!["kdialog", "--getexistingdirectory"], vec!["zenity", "--file-selection", "--directory"]] {
        let Ok(o) = std::process::Command::new(cmd[0]).args(&cmd[1..]).output() else { continue };
        if o.status.success() {
            let p = PathBuf::from(String::from_utf8_lossy(&o.stdout).trim());
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Home(tempfile::TempDir);
    fn home() -> Home {
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", d.path());
        std::env::set_var("XDG_DATA_HOME", d.path().join(".local/share"));
        std::env::set_var("XDG_CACHE_HOME", d.path().join(".cache"));
        std::env::set_var("XDG_CONFIG_HOME", d.path().join(".config"));
        Home(d)
    }

    fn entry() -> Entry {
        Entry::from_json(&json!({"id": "hearth-ai", "name": "Hearth AI", "icon": "☾", "from": "#a78bfa", "to": "#6d28d9", "file": "Hearth-AI.AppImage", "data": ["~/.config/Hearth AI"]}), false).unwrap()
    }

    fn sha(b: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn a_download_that_doesnt_match_is_deleted() {
        let _h = home();
        let e = entry();
        std::fs::create_dir_all(paths::app_dir(&e.id)).unwrap();
        let p = part_file(&e);
        std::fs::write(&p, b"new version").unwrap();
        assert!(check_download(&p, &sha(b"something else")).is_err());
        assert!(!p.exists(), "a file that fails its check is never left to be used");
        std::fs::write(&p, b"new version").unwrap();
        assert!(check_download(&p, "").is_err());
        assert!(!p.exists());
        std::fs::write(&p, b"new version").unwrap();
        assert!(check_download(&p, &sha(b"new version").to_uppercase()).is_ok());
        assert!(p.exists());
    }

    #[test]
    fn an_update_swaps_in_one_rename_and_keeps_the_old_one() {
        let _h = home();
        let e = entry();
        std::fs::create_dir_all(paths::app_dir(&e.id)).unwrap();
        std::fs::write(part_file(&e), b"0.5.0").unwrap();
        assert!(!put_in_place(&e, &part_file(&e)).unwrap(), "a first install keeps nothing from before");
        record(&e, "0.5.0", &sha(b"0.5.0"), 5, "https://x/0.5.0", "").unwrap();
        std::fs::write(part_file(&e), b"0.6.0 new").unwrap();
        assert!(put_in_place(&e, &part_file(&e)).unwrap());
        assert_eq!(std::fs::read(app_file(&e)).unwrap(), b"0.6.0 new");
        assert_eq!(std::fs::read(previous_file(&e)).unwrap(), b"0.5.0", "the old version is kept until the new one started");
        assert!(!part_file(&e).exists());
        assert!(std::fs::metadata(app_file(&e)).unwrap().permissions().mode() & 0o111 != 0);
        drop_previous(&e);
        assert!(!previous_file(&e).exists());
        assert!(changed_since(&e, &read_installed(&e.id).unwrap()), "the file isn't the one recorded");
        let i = record(&e, "0.6.0", &sha(b"0.6.0"), 5, "https://x/0.6.0", "").unwrap();
        assert!(!changed_since(&e, &i));
        assert_eq!(read_installed(&e.id).unwrap().version, "0.6.0");
    }

    #[test]
    fn repair_finds_what_is_wrong_and_fixes_it() {
        let _h = home();
        let e = entry();
        std::fs::create_dir_all(paths::app_dir(&e.id)).unwrap();
        std::fs::write(app_file(&e), b"3.13.0").unwrap();
        std::fs::set_permissions(app_file(&e), std::fs::Permissions::from_mode(0o755)).unwrap();
        write_menu(&e).unwrap();
        let good = check(&e, &sha(b"3.13.0"));
        assert_eq!(good.matches, Some(true));
        assert!(good.wrong.is_empty(), "{:?}", good.wrong);
        // Changed, not allowed to run, the menu entry gone.
        std::fs::write(app_file(&e), b"changed").unwrap();
        std::fs::set_permissions(app_file(&e), std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::remove_file(paths::desktop_file(&e.id)).unwrap();
        let bad = check(&e, &sha(b"3.13.0"));
        let kinds: Vec<Wrong> = bad.wrong.iter().map(|w| w.1).collect();
        assert_eq!(kinds, vec![Wrong::Changed, Wrong::NotRunnable, Wrong::MenuMissing]);
        assert!(bad.needs_download());
        assert_eq!(bad.menu_fixes(), 1);
        // Repair: the same version again (here: the right bytes), checked, swapped in; the local fixes.
        std::fs::write(part_file(&e), b"3.13.0").unwrap();
        check_download(&part_file(&e), &sha(b"3.13.0")).unwrap();
        put_in_place(&e, &part_file(&e)).unwrap();
        fix_local(&e).unwrap();
        assert!(check(&e, &sha(b"3.13.0")).wrong.is_empty());
        // A missing app.
        std::fs::remove_file(app_file(&e)).unwrap();
        assert_eq!(check(&e, &sha(b"3.13.0")).wrong[0].1, Wrong::Missing);
        // Unknown SHA-256: not called wrong, not called right.
        std::fs::write(app_file(&e), b"x").unwrap();
        assert_eq!(check(&e, "").matches, None);
    }

    #[test]
    fn uninstall_keeps_data_or_moves_it_to_the_trash() {
        let h = home();
        let e = entry();
        let data = h.0.path().join(".config/Hearth AI");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("memories.db"), b"her memories").unwrap();
        std::fs::create_dir_all(paths::app_dir(&e.id)).unwrap();
        std::fs::write(app_file(&e), b"app").unwrap();
        write_menu(&e).unwrap();
        assert_eq!(data_size(&e), 12);
        assert!(uninstall(&e, true).unwrap().is_empty());
        assert!(!paths::app_dir(&e.id).exists());
        assert!(!paths::desktop_file(&e.id).exists());
        assert!(!paths::icon_file(&e.id).exists());
        assert!(data.join("memories.db").exists(), "kept by default");
        let moved = uninstall(&e, false).unwrap();
        assert_eq!(moved.len(), 1);
        assert!(!data.exists());
        assert_eq!(std::fs::read(moved[0].join("memories.db")).unwrap(), b"her memories", "in the Trash, not deleted");
        let info = std::fs::read_to_string(paths::trash().join("info/Hearth AI.trashinfo")).unwrap();
        assert!(info.contains("Path=") && info.contains("Hearth%20AI"), "{info}");
    }

    #[test]
    fn data_outside_the_home_folder_is_never_touched() {
        let _h = home();
        let e = Entry::from_json(&json!({"id": "x", "name": "X", "data": ["/etc", "~/../../etc", "~/", "~/.."]}), false).unwrap();
        assert!(data_dirs(&e).is_empty());
    }

    #[test]
    fn the_menu_entry_points_at_the_appimage() {
        let _h = home();
        let e = entry();
        let t = desktop_text(&e, true);
        assert!(t.contains(&format!("Exec=\"{}\" %U", app_file(&e).display())), "{t}");
        assert!(desktop_text(&e, false).contains("APPIMAGE_EXTRACT_AND_RUN=1"));
        assert!(icon_svg(&e).contains("#a78bfa") && icon_svg(&e).contains("☾"));
    }

    #[test]
    fn moving_the_apps_keeps_every_file_and_can_go_back() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a/one");
        std::fs::create_dir_all(a.join("sub")).unwrap();
        std::fs::write(a.join("app.AppImage"), b"bytes").unwrap();
        std::fs::write(a.join("sub/x"), b"y").unwrap();
        let tmp = d.path().join("b");
        std::fs::create_dir_all(&tmp).unwrap();
        copy_tree(&a, &tmp.join("one")).unwrap();
        same_tree(&a, &tmp.join("one")).unwrap();
        std::fs::write(tmp.join("one/sub/x"), b"z").unwrap();
        assert!(same_tree(&a, &tmp.join("one")).is_err(), "a changed file is caught");
        move_dir(&a, &d.path().join("c")).unwrap();
        assert!(!a.exists() && d.path().join("c/app.AppImage").exists());
    }
}

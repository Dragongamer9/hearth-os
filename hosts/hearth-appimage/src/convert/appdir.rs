//! Turning an unpacked package into an AppDir: its menu entry and icon at the top, links that pointed at /opt or
//! /usr made relative, launcher scripts pointed inside the AppImage, and AppRun, which starts the app with its own
//! libraries.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A .desktop file's [Desktop Entry] group, keys in order.
#[derive(Debug, Clone, Default)]
pub struct Desktop {
    pub keys: Vec<(String, String)>,
}

impl Desktop {
    pub fn parse(text: &str) -> Desktop {
        let mut d = Desktop::default();
        let mut inside = false;
        for line in text.lines() {
            let l = line.trim();
            if l.starts_with('[') { inside = l == "[Desktop Entry]"; continue; }
            if !inside || l.starts_with('#') { continue; }
            if let Some((k, v)) = l.split_once('=') { d.keys.push((k.trim().to_string(), v.trim().to_string())); }
        }
        d
    }
    pub fn get(&self, k: &str) -> Option<&str> { self.keys.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str()) }
    pub fn set(&mut self, k: &str, v: &str) {
        if let Some(e) = self.keys.iter_mut().find(|(a, _)| a == k) { e.1 = v.to_string(); } else { self.keys.push((k.into(), v.into())); }
    }
    pub fn remove(&mut self, k: &str) { self.keys.retain(|(a, _)| a != k); }
    pub fn text(&self) -> String {
        let mut s = String::from("[Desktop Entry]\n");
        for (k, v) in &self.keys { s.push_str(&format!("{k}={v}\n")); }
        s
    }
}

/// The program an Exec= line starts (first word, `env A=B` and quotes taken off), and the rest of the line.
pub fn exec_program(exec: &str) -> (String, String) {
    let mut words = shell_words(exec);
    if words.first().map(|w| w == "env").unwrap_or(false) {
        words.remove(0);
        while words.first().map(|w| w.contains('=') && !w.starts_with('/')).unwrap_or(false) { words.remove(0); }
    }
    if words.is_empty() { return (String::new(), String::new()); }
    let prog = words.remove(0);
    (prog, words.join(" "))
}

fn shell_words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut q: Option<char> = None;
    let mut any = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match (q, c) {
            (None, '"') | (None, '\'') => { q = Some(c); any = true; }
            (Some(x), c) if c == x => q = None,
            (_, '\\') => { if let Some(n) = chars.next() { cur.push(n); any = true; } }
            (None, c) if c.is_whitespace() => { if any || !cur.is_empty() { out.push(std::mem::take(&mut cur)); any = false; } }
            (_, c) => { cur.push(c); any = true; }
        }
    }
    if any || !cur.is_empty() { out.push(cur); }
    out
}

/// The app's menu entries (usr/share/applications and anywhere else in a tarball), the best one first: a shown
/// Application, named like the package if one is.
pub fn find_desktops(appdir: &Path, name_hint: &str) -> Vec<(PathBuf, Desktop)> {
    let mut v: Vec<(PathBuf, Desktop, i32)> = Vec::new();
    for e in WalkDir::new(appdir).follow_links(false).into_iter().flatten() {
        if !e.file_type().is_file() || e.path().extension().map(|x| x != "desktop").unwrap_or(true) { continue; }
        let Ok(t) = fs::read_to_string(e.path()) else { continue };
        let d = Desktop::parse(&t);
        if d.get("Type").map(|t| t != "Application").unwrap_or(false) { continue; }
        if d.get("Exec").is_none() { continue; }
        let mut score = 0;
        if d.get("NoDisplay") == Some("true") || d.get("Hidden") == Some("true") { score -= 10; }
        let stem = e.path().file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let hint = name_hint.to_ascii_lowercase();
        if !hint.is_empty() && (stem == hint || stem.ends_with(&format!(".{hint}"))) { score += 5; }
        if !hint.is_empty() && stem.contains(&hint) { score += 2; }
        if e.path().to_string_lossy().contains("/usr/share/applications/") { score += 1; }
        if d.get("MimeType").is_some() { score += 1; }
        v.push((e.path().to_path_buf(), d, score));
    }
    v.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    v.into_iter().map(|(p, d, _)| (p, d)).collect()
}

/// Where in the AppDir an absolute path from the package is (`/opt/foo/foo` → appdir/opt/foo/foo).
pub fn inside(appdir: &Path, abs: &str) -> PathBuf { appdir.join(abs.trim_start_matches('/')) }

/// Finds the program for a command name: an absolute path mapped inside, or a name in the app's bin folders.
pub fn find_program(appdir: &Path, prog: &str) -> Option<PathBuf> {
    if prog.is_empty() { return None; }
    if prog.starts_with('/') {
        let p = inside(appdir, prog);
        return if fs::symlink_metadata(&p).is_ok() { Some(p) } else { None };
    }
    let name = Path::new(prog).file_name()?.to_string_lossy().into_owned();
    for d in ["usr/bin", "usr/games", "usr/local/bin", "bin", "usr/sbin", "app/bin"] {
        let p = appdir.join(d).join(&name);
        if fs::symlink_metadata(&p).is_ok() { return Some(p); }
    }
    // A tarball's program, anywhere (shallow first).
    let mut best: Option<(usize, PathBuf)> = None;
    for e in WalkDir::new(appdir).max_depth(5).follow_links(false).into_iter().flatten() {
        if e.file_name().to_string_lossy() == name && e.file_type().is_file() {
            let depth = e.depth();
            if best.as_ref().map(|b| depth < b.0).unwrap_or(true) { best = Some((depth, e.path().to_path_buf())); }
        }
    }
    best.map(|b| b.1)
}

fn executable(p: &Path) -> bool {
    fs::metadata(p).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

/// With no menu entry: the app's main program, guessed. A program in a bin folder or at the top of a tarball,
/// named like the package if one is, else the biggest one (real programs are big, helpers small).
pub fn guess_program(appdir: &Path, name_hint: &str) -> Option<PathBuf> {
    let hint = name_hint.to_ascii_lowercase();
    let mut cands: Vec<(i64, PathBuf)> = Vec::new();
    for e in WalkDir::new(appdir).max_depth(6).follow_links(false).into_iter().flatten() {
        let p = e.path();
        if !e.file_type().is_file() || !executable(p) { continue; }
        let name = e.file_name().to_string_lossy().to_ascii_lowercase();
        if name.contains(".so") || name.ends_with(".sh") && e.depth() > 2 { continue; }
        let rel = p.strip_prefix(appdir).unwrap_or(p).to_string_lossy().into_owned();
        if rel.contains("/lib/") && !rel.starts_with("usr/lib/") && super::libs::is_elf(p) && name.starts_with("lib") { continue; }
        let elf = super::libs::read_elf(p);
        if elf.as_ref().map(|e| e.is_lib).unwrap_or(false) { continue; }
        let script = elf.is_none() && fs::read(p).map(|b| b.starts_with(b"#!")).unwrap_or(false);
        if elf.is_none() && !script { continue; }
        let size = fs::metadata(p).map(|m| m.len() as i64).unwrap_or(0);
        let mut score = size.min(400 << 20) / 1024; // KiB
        if !hint.is_empty() && name == hint { score += 10 << 20; }
        else if !hint.is_empty() && name.contains(&hint) { score += 2 << 20; }
        if rel.starts_with("usr/bin/") || rel.starts_with("usr/games/") || rel.starts_with("bin/") { score += 1 << 20; }
        if ["chrome-sandbox", "chrome_crashpad_handler", "crashpad_handler", "uninstall", "update"].iter().any(|x| name.starts_with(x)) { score -= 20 << 20; }
        cands.push((score, p.to_path_buf()));
    }
    cands.sort_by(|a, b| b.0.cmp(&a.0));
    cands.into_iter().next().map(|c| c.1)
}

/// Links that pointed at absolute paths (/opt/foo/foo, /usr/lib/x) made relative, so they resolve inside the
/// AppImage. Links to things the package doesn't have (/etc/alternatives/...) stay as they are.
pub fn fix_links(appdir: &Path) -> usize {
    let mut fixed = 0;
    let links: Vec<PathBuf> = WalkDir::new(appdir).follow_links(false).into_iter().flatten()
        .filter(|e| e.file_type().is_symlink()).map(|e| e.into_path()).collect();
    for l in links {
        let Ok(to) = fs::read_link(&l) else { continue };
        let Some(to_s) = to.to_str() else { continue };
        if !to_s.starts_with('/') { continue; }
        let target = inside(appdir, to_s);
        if fs::symlink_metadata(&target).is_err() { continue; }
        let Some(from_dir) = l.parent() else { continue };
        let rel = relative(from_dir, &target);
        if fs::remove_file(&l).is_ok() && symlink(&rel, &l).is_ok() { fixed += 1; }
    }
    fixed
}

/// `to` written relative to the folder `from` (both under the same root, plain paths).
pub fn relative(from: &Path, to: &Path) -> PathBuf {
    let f: Vec<_> = from.components().collect();
    let t: Vec<_> = to.components().collect();
    let common = f.iter().zip(&t).take_while(|(a, b)| a == b).count();
    let mut p = PathBuf::new();
    for _ in common..f.len() { p.push(".."); }
    for c in &t[common..] { p.push(c); }
    p
}

/// The package's own top folders (opt/<x>, usr/lib/<x>, usr/share/<x>, ...), not the shared ones every package
/// puts files in. Their absolute paths in launcher scripts are pointed inside the AppImage.
pub fn own_folders(appdir: &Path) -> Vec<String> {
    const SHARED: &[&str] = &[
        "applications", "icons", "pixmaps", "doc", "man", "info", "locale", "mime", "metainfo", "appdata", "lintian",
        "menu", "bash-completion", "zsh", "fish", "glib-2.0", "dbus-1", "polkit-1", "fonts", "themes", "licenses",
        "gnome-shell", "kservices5", "help", "mime-info", "application-registry", "systemd", "udev", "sysusers.d",
        "tmpfiles.d", "x86_64-linux-gnu", "aarch64-linux-gnu", "debug", "python3", "python3.12", "python3.13", "perl5",
        "gtk-3.0", "gtk-4.0", "qt5", "qt6", "gstreamer-1.0", "dri", "girepository-1.0", "pkgconfig", "cmake", "sounds",
    ];
    let mut out = Vec::new();
    for base in ["opt", "usr/lib", "usr/lib64", "usr/share", "usr/libexec"] {
        let Ok(rd) = fs::read_dir(appdir.join(base)) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !e.path().is_dir() || SHARED.contains(&name.as_str()) || name.starts_with("python") { continue; }
            out.push(format!("/{base}/{name}"));
        }
    }
    out.sort();
    out
}

/// In a shell script, every absolute path into one of the package's own folders becomes "$APPDIR/...".
/// Only where a path starts (after a space, quote, =, : or line start), and not inside single quotes on that line.
pub fn rewrite_script(text: &str, folders: &[String]) -> (String, usize) {
    let mut n = 0;
    let mut out = String::with_capacity(text.len() + 64);
    for (i, line) in text.split_inclusive('\n').enumerate() {
        if i == 0 && line.starts_with("#!") { out.push_str(line); continue; }
        let mut s = String::with_capacity(line.len());
        let b = line.as_bytes();
        let mut j = 0;
        let mut single = false;
        while j < b.len() {
            let c = b[j] as char;
            if c == '\'' { single = !single; }
            let starts = j == 0 || matches!(b[j - 1] as char, ' ' | '\t' | '"' | '=' | ':' | '(' | '`');
            if c == '/' && starts && !single {
                let rest = &line[j..];
                if let Some(f) = folders.iter().find(|f| rest.starts_with(f.as_str()) && rest[f.len()..].chars().next().map(|x| matches!(x, '/' | '"' | ' ' | '\n' | ':' | ';' | ')' | '\'' | '`')).unwrap_or(true)) {
                    s.push_str("$APPDIR");
                    s.push_str(f);
                    j += f.len();
                    n += 1;
                    continue;
                }
            }
            s.push(c);
            j += 1;
        }
        // Lines are ASCII-only for the matching above; anything else is copied as it was.
        if line.is_ascii() { out.push_str(&s); } else { out.push_str(line); }
    }
    (out, n)
}

fn is_shell_script(p: &Path) -> bool {
    let Ok(b) = fs::read(p) else { return false };
    if b.len() > 1 << 20 { return false; }
    let first = b.split(|c| *c == b'\n').next().unwrap_or(&[]);
    let first = String::from_utf8_lossy(first);
    first.starts_with("#!") && (first.contains("/sh") || first.contains("bash") || first.contains("dash") || first.contains("zsh"))
}

/// The launcher scripts in the app's bin folders and at the top of its own folders, pointed inside the AppImage.
pub fn fix_scripts(appdir: &Path) -> usize {
    let folders = own_folders(appdir);
    if folders.is_empty() { return 0; }
    let mut n = 0;
    let mut places: Vec<PathBuf> = ["usr/bin", "usr/games", "usr/sbin", "bin"].iter().map(|d| appdir.join(d)).collect();
    for f in &folders { places.push(inside(appdir, f)); }
    for dir in places {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if fs::symlink_metadata(&p).map(|m| !m.is_file()).unwrap_or(true) || !is_shell_script(&p) { continue; }
            let Ok(t) = fs::read_to_string(&p) else { continue };
            let (new, k) = rewrite_script(&t, &folders);
            if k > 0 && fs::write(&p, new).is_ok() { n += k; }
        }
    }
    n
}

/// The icon a menu entry names: an absolute path inside, else the biggest one in the icon theme folders or pixmaps.
pub fn find_icon(appdir: &Path, icon: &str) -> Option<PathBuf> {
    if icon.is_empty() { return None; }
    if icon.starts_with('/') {
        let p = inside(appdir, icon);
        return if p.is_file() { Some(p) } else { None };
    }
    let stem = icon.trim_end_matches(".png").trim_end_matches(".svg").trim_end_matches(".xpm");
    let mut best: Option<(u32, PathBuf)> = None;
    // Links aren't walked into (one to /usr would walk this whole computer), but an icon that is a link counts.
    for e in WalkDir::new(appdir).follow_links(false).into_iter().flatten() {
        let p = e.path();
        if !(e.file_type().is_file() || (e.file_type().is_symlink() && p.is_file())) { continue; }
        let Some(file_stem) = p.file_stem().map(|s| s.to_string_lossy().into_owned()) else { continue };
        let ext = p.extension().map(|s| s.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        if file_stem != stem || !matches!(ext.as_str(), "png" | "svg") { continue; }
        let s = p.to_string_lossy();
        let size = if s.contains("/scalable/") || ext == "svg" { 1000 } else {
            s.split('/').find_map(|seg| seg.split_once('x').and_then(|(a, b)| if a == b { a.parse().ok() } else { None })).unwrap_or(48)
        };
        // A 512 png is better for the AppImage's own icon than an svg some tools can't show; 1000 only beats < 256.
        let rank = if size == 1000 { 300 } else { size };
        if best.as_ref().map(|b| rank > b.0).unwrap_or(true) { best = Some((rank, p.to_path_buf())); }
    }
    best.map(|b| b.1)
}

/// Hearth's plain package icon, for apps that bring none.
pub const FALLBACK_ICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect x="24" y="24" width="208" height="208" rx="48" fill="#2a1d15"/><path d="M128 56l64 32v80l-64 32-64-32V88z" fill="#ff8a3d"/><path d="M64 88l64 32 64-32M128 120v80" fill="none" stroke="#2a1206" stroke-width="10" stroke-linejoin="round"/></svg>"##;

/// AppRun: the libraries and programs inside first, then the app's program, with everything it was given.
pub fn write_apprun(appdir: &Path, program: &Path, args: &str, lib_dirs: &[PathBuf]) -> io::Result<()> {
    let rel = program.strip_prefix(appdir).unwrap_or(program).to_string_lossy().into_owned();
    let libs: Vec<String> = lib_dirs.iter().map(|d| format!("$APPDIR/{}", d.display())).collect();
    let mut extra = String::new();
    if appdir.join("usr/share/glib-2.0/schemas/gschemas.compiled").exists() {
        extra.push_str("export GSETTINGS_SCHEMA_DIR=\"$APPDIR/usr/share/glib-2.0/schemas${GSETTINGS_SCHEMA_DIR:+:$GSETTINGS_SCHEMA_DIR}\"\n");
    }
    for qt in ["usr/lib/qt6/plugins", "usr/lib64/qt6/plugins", "usr/lib/x86_64-linux-gnu/qt6/plugins", "usr/lib/qt5/plugins", "usr/lib64/qt5/plugins", "usr/lib/x86_64-linux-gnu/qt5/plugins"] {
        if appdir.join(qt).is_dir() { extra.push_str(&format!("export QT_PLUGIN_PATH=\"$APPDIR/{qt}${{QT_PLUGIN_PATH:+:$QT_PLUGIN_PATH}}\"\n")); break; }
    }
    // Fixed arguments from the menu entry's Exec= (field codes like %U are the files given, so they go).
    let fixed: Vec<String> = shell_words(args).into_iter().filter(|w| !(w.len() == 2 && w.starts_with('%'))).map(|w| sh_quote(&w)).collect();
    let fixed = if fixed.is_empty() { String::new() } else { format!(" {}", fixed.join(" ")) };
    let s = format!(r#"#!/bin/sh
# Made by Hearth OS's AppImage Maker: starts the app with the libraries inside this AppImage.
HERE="$(dirname "$(readlink -f "$0")")"
export APPDIR="${{APPDIR:-$HERE}}"
export LD_LIBRARY_PATH="{libs}${{LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}}"
export PATH="$APPDIR/usr/bin:$APPDIR/usr/sbin:$APPDIR/usr/games:$APPDIR/bin:$PATH"
export XDG_DATA_DIRS="$APPDIR/usr/share:${{XDG_DATA_DIRS:-/usr/local/share:/usr/share}}"
{extra}exec "$APPDIR/{rel}"{fixed} "$@"
"#, libs = libs.join(":"));
    let p = appdir.join("AppRun");
    let _ = fs::remove_file(&p);
    fs::write(&p, s)?;
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755))
}

fn sh_quote(s: &str) -> String {
    if s.chars().all(|c| c.is_ascii_alphanumeric() || "-_=./,:+@".contains(c)) { s.to_string() } else { format!("'{}'", s.replace('\'', "'\\''")) }
}

/// A desktop id from a name: lowercase letters, digits and dashes.
pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() { out.push(c.to_ascii_lowercase()); }
        else if !out.ends_with('-') && !out.is_empty() { out.push('-'); }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "app".into() } else { out }
}

/// Counts of what is in the AppDir, for the report.
pub fn sizes(appdir: &Path) -> BTreeMap<&'static str, u64> {
    let mut m = BTreeMap::new();
    for e in WalkDir::new(appdir).follow_links(false).into_iter().flatten() {
        if e.file_type().is_file() {
            *m.entry("files").or_insert(0) += 1;
            *m.entry("bytes").or_insert(0) += e.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_lines() {
        assert_eq!(exec_program("/opt/Foo/foo --no-sandbox %U"), ("/opt/Foo/foo".into(), "--no-sandbox %U".into()));
        assert_eq!(exec_program("env GDK_BACKEND=x11 \"/opt/My App/app\" %F"), ("/opt/My App/app".into(), "%F".into()));
        assert_eq!(exec_program("hello"), ("hello".into(), "".into()));
    }

    #[test]
    fn desktop_round_trip() {
        let mut d = Desktop::parse("[Desktop Entry]\nName=Demo\nExec=demo %U\nIcon=demo\n[Desktop Action new]\nExec=demo --new\n");
        assert_eq!(d.get("Exec"), Some("demo %U"));
        d.set("Icon", "x");
        d.remove("Name");
        assert_eq!(d.text(), "[Desktop Entry]\nExec=demo %U\nIcon=x\n");
    }

    #[test]
    fn scripts_point_inside() {
        let folders = vec!["/opt/demo".to_string(), "/usr/share/code".to_string()];
        let (s, n) = rewrite_script("#!/bin/sh\nexec /opt/demo/demo \"$@\"\nVSCODE_PATH=\"/usr/share/code\"\necho '/opt/demo'\n/opt/demolition\n", &folders);
        assert_eq!(n, 2);
        assert!(s.contains("exec $APPDIR/opt/demo/demo"));
        assert!(s.contains("VSCODE_PATH=\"$APPDIR/usr/share/code\""));
        assert!(s.contains("echo '/opt/demo'"));
        assert!(s.contains("\n/opt/demolition"));
    }

    #[test]
    fn relative_links() {
        assert_eq!(relative(Path::new("/a/usr/bin"), Path::new("/a/opt/x/x")), PathBuf::from("../../opt/x/x"));
        assert_eq!(slug("My Cool App 2!"), "my-cool-app-2");
    }
}

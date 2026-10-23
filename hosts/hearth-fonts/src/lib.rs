//! Hearth Fonts without the window: the installed fonts (fontconfig's fc-list on Linux), what a font file is
//! (fc-scan), and installing a font for you (into ~/.local/share/fonts, then fc-cache) or removing one you installed.
//! Fonts that came with the system are never removed. Windows (Ember) lists its font folders; installing there
//! comes later.

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Face {
    pub style: String,
    pub file: String,
    pub weight: u32,
    pub italic: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Family {
    pub name: String,
    pub faces: Vec<Face>,
    /// Installed by you (in your fonts folder), so it can be removed.
    pub mine: bool,
    /// Mostly symbols or icons (shown with its name instead of a sentence).
    pub symbols: bool,
}

pub const FONT_EXTS: &[&str] = &["ttf", "otf", "ttc", "otc"];
pub const MAX_FONT_BYTES: u64 = 64 * 1024 * 1024;

pub fn home() -> PathBuf {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_else(std::env::temp_dir)
}

/// Where your own fonts go: ~/.local/share/fonts (fontconfig reads it), or Windows' per-user font folder.
pub fn user_dir() -> PathBuf {
    if cfg!(windows) {
        return std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(home).join("Microsoft/Windows/Fonts");
    }
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".local/share")).join("fonts")
}

/// Folders whose fonts count as yours (the old ~/.fonts too).
pub fn user_dirs() -> Vec<PathBuf> {
    let mut v = vec![user_dir()];
    if !cfg!(windows) { v.push(home().join(".fonts")); }
    v
}

pub fn is_mine(file: &Path, dirs: &[PathBuf]) -> bool {
    dirs.iter().any(|d| file.starts_with(d))
}

/// fontconfig's weight (0-215) as CSS's (100-900).
pub fn css_weight(fc: u32) -> u32 {
    match fc {
        0..=20 => 100, 21..=45 => 200, 46..=55 => 300, 56..=90 => 400, 91..=140 => 500, 141..=190 => 600,
        191..=202 => 700, 203..=207 => 800, _ => 900,
    }
}

/// fc-list's lines ("family|style|file|weight|slant|spacing|charset-size") grouped into families, sorted by name.
pub fn parse_fc_list(text: &str, dirs: &[PathBuf]) -> Vec<Family> {
    let mut map: BTreeMap<String, Family> = BTreeMap::new();
    for line in text.lines() {
        let p: Vec<&str> = line.split('|').collect();
        if p.len() < 3 { continue; }
        let family = p[0].split(',').next().unwrap_or("").trim();
        let file = p[2].trim();
        if family.is_empty() || file.is_empty() { continue; }
        let style = p[1].split(',').next().unwrap_or("").trim();
        let weight = p.get(3).and_then(|w| w.trim().parse::<f64>().ok()).map(|w| css_weight(w as u32)).unwrap_or(400);
        let italic = p.get(4).and_then(|s| s.trim().parse::<u32>().ok()).map(|s| s > 0).unwrap_or_else(|| style.to_lowercase().contains("italic") || style.to_lowercase().contains("oblique"));
        let key = family.to_lowercase();
        let f = map.entry(key).or_insert_with(|| Family { name: family.to_string(), faces: Vec::new(), mine: false, symbols: false });
        let mine = is_mine(Path::new(file), dirs);
        f.mine |= mine;
        let lname = family.to_lowercase();
        f.symbols |= ["symbol", "emoji", "icons", "dingbat", "awesome", "material"].iter().any(|w| lname.contains(w));
        if !f.faces.iter().any(|x| x.file == file && x.style == style) {
            f.faces.push(Face { style: if style.is_empty() { "Regular".into() } else { style.to_string() }, file: file.to_string(), weight, italic });
        }
    }
    let mut v: Vec<Family> = map.into_values().collect();
    for f in &mut v { f.faces.sort_by(|a, b| (a.italic, a.weight, &a.style).cmp(&(b.italic, b.weight, &b.style))); }
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    v
}

pub const FC_FORMAT: &str = "%{family}|%{style}|%{file}|%{weight}|%{slant}\n";

/// Every installed font family.
pub fn list() -> Result<Vec<Family>, String> {
    #[cfg(windows)]
    { return Ok(windows_list()); }
    #[cfg(not(windows))]
    {
        let out = Command::new("fc-list").arg("--format").arg(FC_FORMAT).output().map_err(|_| "fontconfig (fc-list) isn't installed".to_string())?;
        Ok(parse_fc_list(&String::from_utf8_lossy(&out.stdout), &user_dirs()))
    }
}

#[cfg(windows)]
fn windows_list() -> Vec<Family> {
    let sys = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Windows")).join("Fonts");
    let mut lines = String::new();
    for d in [sys, user_dir()] {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if !has_font_ext(&p) { continue; }
                let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                lines.push_str(&format!("{stem}|Regular|{}\n", p.display()));
            }
        }
    }
    parse_fc_list(&lines, &[user_dir()])
}

pub fn has_font_ext(p: &Path) -> bool {
    p.extension().map(|e| FONT_EXTS.contains(&e.to_string_lossy().to_lowercase().as_str())).unwrap_or(false)
}

/// The first bytes say it's a TrueType/OpenType font or collection.
pub fn looks_like_font(head: &[u8]) -> bool {
    head.len() >= 4 && matches!(&head[..4], [0, 1, 0, 0] | b"OTTO" | b"true" | b"ttcf" | b"typ1")
}

/// A font file you opened: checked to be one, then described by fontconfig (fc-scan).
pub fn inspect(path: &Path) -> Result<Family, String> {
    check_file(path)?;
    #[cfg(not(windows))]
    {
        let out = Command::new("fc-scan").arg("--format").arg(FC_FORMAT).arg(path).output().map_err(|_| "fontconfig (fc-scan) isn't installed".to_string())?;
        let mut fams = parse_fc_list(&String::from_utf8_lossy(&out.stdout), &user_dirs());
        if fams.is_empty() { return Err("fontconfig can't read this font.".into()); }
        let mut f = fams.remove(0);
        for other in fams { f.faces.extend(other.faces); }
        Ok(f)
    }
    #[cfg(windows)]
    {
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        Ok(Family { name: stem, faces: vec![Face { style: "Regular".into(), file: path.display().to_string(), weight: 400, italic: false }], mine: false, symbols: false })
    }
}

pub fn check_file(path: &Path) -> Result<(), String> {
    let meta = std::fs::metadata(path).map_err(|_| "That file isn't there.".to_string())?;
    if !meta.is_file() { return Err("That isn't a file.".into()); }
    if meta.len() > MAX_FONT_BYTES { return Err("That file is too big to be a font.".into()); }
    if !has_font_ext(path) { return Err("Only .ttf, .otf and .ttc fonts can be installed.".into()); }
    let mut head = [0u8; 4];
    use std::io::Read;
    std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut head)).map_err(|e| e.to_string())?;
    if !looks_like_font(&head) { return Err("That file isn't a font, whatever its name says.".into()); }
    Ok(())
}

/// A name for the copy in your fonts folder that doesn't overwrite another font: "Name.ttf", "Name-2.ttf"...
pub fn target_name(dir: &Path, src: &Path) -> PathBuf {
    let stem: String = src.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "font".into())
        .chars().map(|c| if c == '/' || c == '\\' || c.is_control() { '_' } else { c }).collect();
    let ext = src.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_else(|| "ttf".into());
    let mut p = dir.join(format!("{stem}.{ext}"));
    let mut n = 2;
    while p.exists() { p = dir.join(format!("{stem}-{n}.{ext}")); n += 1; }
    p
}

/// Installs a font for you. Returns where it went. An identical copy already there counts as installed.
pub fn install_into(src: &Path, dir: &Path) -> Result<PathBuf, String> {
    check_file(src)?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let bytes = std::fs::read(src).map_err(|e| e.to_string())?;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if std::fs::metadata(e.path()).map(|m| m.len() == bytes.len() as u64).unwrap_or(false) && std::fs::read(e.path()).map(|b| b == bytes).unwrap_or(false) {
                return Ok(e.path());
            }
        }
    }
    let dest = target_name(dir, src);
    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
    Ok(dest)
}

/// Removes a font you installed. Anything outside your fonts folders is refused.
pub fn uninstall_from(file: &Path, dirs: &[PathBuf]) -> Result<(), String> {
    let real = std::fs::canonicalize(file).map_err(|_| "That font isn't there any more.".to_string())?;
    let ok = dirs.iter().filter_map(|d| std::fs::canonicalize(d).ok()).any(|d| real.starts_with(&d) && real != d);
    if !ok { return Err("Only fonts you installed can be removed; this one came with the system.".into()); }
    if !has_font_ext(&real) { return Err("That isn't a font file.".into()); }
    std::fs::remove_file(&real).map_err(|e| e.to_string())
}

/// fontconfig reads your fonts folder again, so apps see the change.
pub fn refresh_cache() {
    #[cfg(not(windows))]
    { let _ = Command::new("fc-cache").arg("-f").arg(user_dir()).status(); }
}

pub fn install(src: &Path) -> Result<PathBuf, String> {
    if cfg!(windows) { return Err("Installing fonts from Hearth Fonts comes to Ember later. Right-click the file and choose Install.".into()); }
    let p = install_into(src, &user_dir())?;
    refresh_cache();
    Ok(p)
}

pub fn uninstall(file: &Path) -> Result<(), String> {
    if cfg!(windows) { return Err("Removing fonts from Hearth Fonts comes to Ember later.".into()); }
    uninstall_from(file, &user_dirs())?;
    refresh_cache();
    Ok(())
}

#[derive(Serialize, Debug)]
pub struct Entry { pub name: String, pub path: String, pub dir: bool }

/// A folder for the page's own file picker: its folders and font files (hidden ones left out), folders first.
/// "" starts in Downloads (or home).
pub fn browse(dir: &str) -> Result<(String, Option<String>, Vec<Entry>), String> {
    let d = if dir.is_empty() {
        let dl = home().join("Downloads");
        if dl.is_dir() { dl } else { home() }
    } else { PathBuf::from(dir) };
    let d = std::fs::canonicalize(&d).map_err(|_| "That folder isn't there.".to_string())?;
    let mut v: Vec<Entry> = std::fs::read_dir(&d).map_err(|e| e.to_string())?.flatten().filter_map(|e| {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { return None; }
        let p = e.path();
        let dir = p.is_dir();
        if !dir && !has_font_ext(&p) { return None; }
        Some(Entry { name, path: p.display().to_string(), dir })
    }).collect();
    v.sort_by(|a, b| (!a.dir, a.name.to_lowercase()).cmp(&(!b.dir, b.name.to_lowercase())));
    Ok((d.display().to_string(), d.parent().map(|p| p.display().to_string()), v))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FC: &str = "DejaVu Sans|Book|/usr/share/fonts/dejavu/DejaVuSans.ttf|80|0\n\
DejaVu Sans|Bold|/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf|200|0\n\
DejaVu Sans,DejaVu Sans Condensed|Condensed Oblique,Oblique|/usr/share/fonts/dejavu/DejaVuSansCondensed-Oblique.ttf|80|110\n\
Lobster|Regular|/home/me/.local/share/fonts/Lobster.ttf|80|0\n\
Noto Color Emoji|Regular|/usr/share/fonts/noto/NotoColorEmoji.ttf|80|0\n\
broken line\n";

    #[test]
    fn families() {
        let f = parse_fc_list(FC, &[PathBuf::from("/home/me/.local/share/fonts")]);
        assert_eq!(f.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), vec!["DejaVu Sans", "Lobster", "Noto Color Emoji"]);
        assert_eq!(f[0].faces.len(), 3);
        assert_eq!(f[0].faces[0].style, "Book");
        assert_eq!(f[0].faces[1].weight, 700);
        assert!(f[0].faces[2].italic);
        assert!(!f[0].mine && f[1].mine);
        assert!(f[2].symbols);
    }

    #[test]
    fn font_files() {
        assert!(looks_like_font(&[0, 1, 0, 0, 9]));
        assert!(looks_like_font(b"OTTO"));
        assert!(!looks_like_font(b"%PDF"));
        assert!(has_font_ext(Path::new("/a/B.TTF")));
        assert!(!has_font_ext(Path::new("/a/b.woff2")));
    }

    #[test]
    fn install_and_remove() {
        let d = std::env::temp_dir().join(format!("hearth-fonts-test-{}", std::process::id()));
        let src_dir = d.join("dl");
        let fonts = d.join("fonts");
        std::fs::create_dir_all(&src_dir).unwrap();
        let src = src_dir.join("My Font.ttf");
        std::fs::write(&src, [0u8, 1, 0, 0, 1, 2, 3]).unwrap();
        let a = install_into(&src, &fonts).unwrap();
        assert_eq!(a, fonts.join("My Font.ttf"));
        // The same file again: already there, no second copy.
        assert_eq!(install_into(&src, &fonts).unwrap(), a);
        // A different font with the same name: a new name.
        std::fs::write(&src, [0u8, 1, 0, 0, 9]).unwrap();
        assert_eq!(install_into(&src, &fonts).unwrap(), fonts.join("My Font-2.ttf"));
        // Not a font.
        let fake = src_dir.join("fake.ttf");
        std::fs::write(&fake, b"%PDF-1.7").unwrap();
        assert!(install_into(&fake, &fonts).is_err());
        // Removing: yours yes, the system's no.
        assert!(uninstall_from(&src, &[fonts.clone()]).is_err());
        uninstall_from(&a, &[fonts.clone()]).unwrap();
        assert!(!a.exists());
        assert!(uninstall_from(&fonts.join("../dl/My Font.ttf"), &[fonts.clone()]).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn browsing() {
        let d = std::env::temp_dir().join(format!("hearth-fonts-browse-{}", std::process::id()));
        std::fs::create_dir_all(d.join("Sub")).unwrap();
        std::fs::write(d.join("a.otf"), b"OTTO").unwrap();
        std::fs::write(d.join("notes.txt"), b"x").unwrap();
        std::fs::write(d.join(".hidden.ttf"), b"x").unwrap();
        let (_, parent, e) = browse(&d.display().to_string()).unwrap();
        assert!(parent.is_some());
        assert_eq!(e.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), vec!["Sub", "a.otf"]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn weights() {
        assert_eq!(css_weight(80), 400);
        assert_eq!(css_weight(200), 700);
        assert_eq!(css_weight(0), 100);
        assert_eq!(css_weight(210), 900);
    }
}

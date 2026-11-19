//! What Hearth Files shows: a folder's entries as hearth-files lists them, and the words for them (FileManager.qml's
//! kindOf, kindName with Windows' type names, fmtSize, fmtDate, natural sorting).

use serde_json::Value;

pub const HOME_PAGE: &str = "hearth:home";
pub const THIS_PC: &str = "hearth:thispc";
pub const APPS_PAGE: &str = "hearth:apps";

/// One thing in a folder (hearth-files `entry`), a recent file, a This PC item, an app or one of Hearth (C:)'s names.
#[derive(Debug, Clone, Default)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub dir: bool,
    pub size: u64,
    pub mtime: i64,
    pub mime: String,
    pub is_link: bool,
    pub exec: bool,
    pub tags: Vec<String>,
    /// Search results and recent files: the folder it's in (~/Downloads).
    pub where_: String,
    /// Recent files: when it was opened (ISO text).
    pub opened: String,
    /// In the Trash: where it came from.
    pub orig_path: String,
    pub appimage: bool,
    pub in_menu: bool,
    /// Hearth (C:)'s view: what this name stands for.
    pub cv: Option<crate::drive::CView>,
    /// This PC: "folder" | "drive" | "net", with its item.
    pub pc: String,
    pub item: Value,
    /// Installed apps: the app.
    pub app: Option<Value>,
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

impl Entry {
    pub fn from_json(v: &Value) -> Entry {
        Entry {
            name: s(v, "name"),
            path: s(v, "path"),
            dir: v.get("dir").and_then(Value::as_bool).unwrap_or(false),
            size: v.get("size").and_then(Value::as_u64).unwrap_or(0),
            mtime: v.get("mtime").and_then(Value::as_i64).unwrap_or(0),
            mime: s(v, "mime"),
            is_link: v.get("isLink").and_then(Value::as_bool).unwrap_or(false),
            exec: v.get("exec").and_then(Value::as_bool).unwrap_or(false),
            tags: v.get("tags").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default(),
            where_: s(v, "where"),
            opened: s(v, "opened"),
            orig_path: s(v, "origPath"),
            appimage: v.get("appimage").and_then(Value::as_bool).unwrap_or(false),
            in_menu: v.get("inMenu").and_then(Value::as_bool).unwrap_or(false),
            ..Default::default()
        }
    }
}

/// hearth-files list: the folder and what's in it.
#[derive(Debug, Clone, Default)]
pub struct Listing {
    pub path: String,
    pub name: String,
    pub entries: Vec<Entry>,
    pub writable: bool,
    pub trash: bool,
    pub free: Option<u64>,
    pub total: Option<u64>,
    pub error: String,
    pub select: String,
}

impl Listing {
    pub fn from_json(v: &Value) -> Listing {
        Listing {
            path: s(v, "path"),
            name: s(v, "name"),
            entries: v.get("entries").and_then(Value::as_array).map(|a| a.iter().map(Entry::from_json).collect()).unwrap_or_default(),
            writable: v.get("writable").and_then(Value::as_bool).unwrap_or(false),
            trash: v.get("trash").and_then(Value::as_bool).unwrap_or(false),
            free: v.get("free").and_then(Value::as_u64),
            total: v.get("total").and_then(Value::as_u64),
            error: String::new(),
            select: s(v, "select"),
        }
    }
    pub fn page(path: &str, name: &str) -> Listing {
        Listing { path: path.into(), name: name.into(), ..Default::default() }
    }
}

pub fn base(p: &str) -> String {
    let s = p.trim_end_matches('/');
    match s.rfind('/') {
        Some(i) => {
            let b = &s[i + 1..];
            if b.is_empty() { "/".into() } else { b.to_string() }
        }
        None => if s.is_empty() { "/".into() } else { s.to_string() },
    }
}

pub fn parent_of(p: &str) -> String {
    let s = p.trim_end_matches('/');
    match s.rfind('/') {
        Some(i) if i > 0 => s[..i].to_string(),
        _ => "/".into(),
    }
}

pub fn under(p: &str, root: &str) -> bool {
    p == root || p.starts_with(&format!("{}/", root.trim_end_matches('/')))
}

pub fn ext_of(e: &Entry) -> String {
    if e.dir {
        return String::new();
    }
    match e.name.rfind('.') {
        Some(i) if i > 0 => e.name[i + 1..].to_lowercase(),
        _ => String::new(),
    }
}

/// folder | doc | text | image | video | audio | archive | app | disc | file
pub fn kind_of(e: &Entry) -> &'static str {
    if e.dir {
        return "folder";
    }
    let m = e.mime.as_str();
    let has = |xs: &[&str]| xs.iter().any(|x| m.contains(x));
    if m.starts_with("image/") {
        return "image";
    }
    if m.starts_with("video/") {
        return "video";
    }
    if m.starts_with("audio/") {
        return "audio";
    }
    if has(&["iso9660", "cd-image", "x-raw-disk"]) {
        return "disc";
    }
    if has(&["zip", "x-7z", "rar", "x-tar", "gzip", "x-xz", "zstd", "bzip", "x-compressed", "x-rpm"]) {
        return "archive";
    }
    if has(&["appimage", "ms-dos-executable", "x-msi", "x-desktop", "flatpak", "x-executable"]) || e.exec {
        return "app";
    }
    if has(&["pdf", "msword", "opendocument", "officedocument", "epub", "rtf"]) {
        return "doc";
    }
    if m.starts_with("text/") || has(&["json", "xml", "javascript", "shellscript", "x-yaml", "toml"]) {
        return "text";
    }
    // hearth-files names types by their extension; a few it may not know.
    match ext_of(e).as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" => "image",
        "mp4" | "mkv" | "webm" | "avi" | "mov" => "video",
        "mp3" | "flac" | "ogg" | "wav" | "m4a" => "audio",
        "iso" => "disc",
        "zip" | "7z" | "rar" | "gz" | "xz" | "tar" | "zst" | "bz2" => "archive",
        "pdf" | "odt" | "ods" | "odp" | "docx" | "doc" | "xlsx" | "pptx" => "doc",
        "txt" | "md" | "py" | "sh" | "json" | "toml" | "yaml" | "yml" | "rs" | "c" | "h" | "cpp" | "js" | "css" | "html" | "log" | "conf" | "ini" | "cfg" | "qml" => "text",
        _ => "file",
    }
}

/// Windows' names for types (the Type column, the details pane, the Save window): "File folder", "Python file".
pub fn win_type(x: &str) -> Option<&'static str> {
    Some(match x {
        "py" => "Python file",
        "pdf" => "PDF document",
        "jpg" | "jpeg" => "JPEG image",
        "png" => "PNG image",
        "gif" => "GIF image",
        "webp" => "WebP image",
        "svg" => "SVG image",
        "bmp" => "Bitmap image",
        "txt" | "log" => "Text document",
        "md" => "Markdown file",
        "json" => "JSON file",
        "xml" => "XML document",
        "html" | "htm" => "HTML document",
        "css" => "CSS file",
        "js" => "JavaScript file",
        "sh" => "Shell script",
        "zip" => "Compressed (zipped) folder",
        "7z" => "7-Zip archive",
        "rar" => "RAR archive",
        "gz" => "GZ archive",
        "xz" => "XZ archive",
        "tar" => "TAR archive",
        "exe" => "Application",
        "msi" => "Windows Installer package",
        "iso" => "Disc image",
        "mp4" => "MP4 video",
        "mkv" => "MKV video",
        "webm" => "WebM video",
        "avi" => "AVI video",
        "mov" => "QuickTime movie",
        "mp3" => "MP3 audio",
        "flac" => "FLAC audio",
        "ogg" => "OGG audio",
        "wav" => "WAV audio",
        "m4a" => "M4A audio",
        "odt" => "OpenDocument text",
        "ods" => "OpenDocument spreadsheet",
        "odp" => "OpenDocument presentation",
        "docx" => "Microsoft Word document",
        "doc" => "Microsoft Word 97–2003 document",
        "xlsx" => "Microsoft Excel worksheet",
        "pptx" => "Microsoft PowerPoint presentation",
        "csv" => "CSV file",
        "deb" => "Debian package",
        "rpm" => "RPM package",
        "appimage" => "AppImage app",
        "desktop" => "Shortcut",
        "ini" | "cfg" | "conf" => "Configuration settings",
        "ttf" => "TrueType font",
        "otf" => "OpenType font",
        "qml" => "QML file",
        "rs" => "Rust source file",
        "c" => "C source file",
        "cpp" => "C++ source file",
        "h" => "C header file",
        "toml" => "TOML file",
        "yaml" | "yml" => "YAML file",
        _ => return None,
    })
}

pub fn kind_name(e: &Entry) -> String {
    if let Some(cv) = &e.cv {
        return cv.kind_text.clone();
    }
    if e.dir {
        return "File folder".into();
    }
    let x = ext_of(e);
    if let Some(w) = win_type(&x) {
        return w.into();
    }
    if !x.is_empty() {
        return format!("{} file", x.to_uppercase());
    }
    "File".into()
}

/// 2 KB, 2.4 MB, 4.1 GB (Windows rounds files up to whole KB; Hearth's own words keep one decimal).
pub fn fmt_size(n: u64) -> String {
    if n < 1024 {
        return format!("{n} {}", if n == 1 { "byte" } else { "bytes" });
    }
    let u = ["KB", "MB", "GB", "TB"];
    let mut v = n as f64 / 1024.0;
    let mut i = 0;
    while v >= 1024.0 && i < u.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        return format!("{} KB", v.ceil() as u64);
    }
    if v >= 100.0 { format!("{} {}", v.round() as u64, u[i]) } else if v >= 10.0 { format!("{:.0} {}", v, u[i]) } else { format!("{:.1} {}", v, u[i]) }
}

/// Free space, the drive way: 96 GB, 3.6 GB, 1.2 TB.
pub fn fmt_space(n: u64) -> String {
    let u = ["bytes", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < u.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        return format!("{n} bytes");
    }
    if v >= 10.0 { format!("{:.0} {}", v, u[i]) } else { format!("{:.1} {}", v, u[i]) }
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// Nov 11, 21:40 (this year), Nov 11, 2025 (before), Today 02:10, Yesterday 21:17 (`relative`: the Home page's).
pub fn fmt_date(ts: i64, relative: bool) -> String {
    if ts <= 0 {
        return String::new();
    }
    let t = hearth_ui::time::local(ts);
    let n = hearth_ui::time::local(hearth_ui::time::now());
    let y = hearth_ui::time::local(hearth_ui::time::now() - 86_400);
    let hm = format!("{:02}:{:02}", t.hour, t.min);
    if relative {
        if t.year == n.year && t.mon == n.mon && t.day == n.day {
            return format!("Today {hm}");
        }
        if t.year == y.year && t.mon == y.mon && t.day == y.day {
            return format!("Yesterday {hm}");
        }
    }
    if t.year == n.year {
        format!("{} {}, {hm}", MONTHS[t.mon], t.day)
    } else {
        format!("{} {}, {}", MONTHS[t.mon], t.day, t.year)
    }
}

/// Nov 12, 2026, 18:02 (Properties).
pub fn fmt_full(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    let t = hearth_ui::time::local(ts);
    format!("{} {}, {}, {:02}:{:02}", MONTHS[t.mon], t.day, t.year, t.hour, t.min)
}

/// "2026-11-18T02:10:00Z" (recently-used.xbel) → seconds.
pub fn parse_iso(s: &str) -> i64 {
    let b = s.as_bytes();
    if b.len() < 19 {
        return 0;
    }
    let num = |a: usize, n: usize| -> i64 { s.get(a..a + n).and_then(|x| x.parse().ok()).unwrap_or(0) };
    let (y, mo, d, h, mi, se) = (num(0, 4), num(5, 2), num(8, 2), num(11, 2), num(14, 2), num(17, 2));
    // days from civil (Howard Hinnant)
    let yy = if mo <= 2 { y - 1 } else { y };
    let era = (if yy >= 0 { yy } else { yy - 399 }) / 400;
    let yoe = yy - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    days * 86400 + h * 3600 + mi * 60 + se
}

pub fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// "file 9" before "file 10"; case doesn't matter.
pub fn natural_key(name: &str) -> Vec<(bool, String)> {
    let low = name.to_lowercase();
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut digit = false;
    for c in low.chars() {
        let d = c.is_ascii_digit();
        if !cur.is_empty() && d != digit {
            out.push((digit, std::mem::take(&mut cur)));
        }
        digit = d;
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push((digit, cur));
    }
    out
}

pub fn natural_cmp(x: &[(bool, String)], y: &[(bool, String)]) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    for (a, b) in x.iter().zip(y.iter()) {
        if a == b {
            continue;
        }
        if a.0 && b.0 {
            let na = a.1.trim_start_matches('0');
            let nb = b.1.trim_start_matches('0');
            let o = na.len().cmp(&nb.len()).then_with(|| na.cmp(nb)).then_with(|| a.1.len().cmp(&b.1.len()));
            if o != Ordering::Equal {
                return o;
            }
            continue;
        }
        return a.1.cmp(&b.1);
    }
    x.len().cmp(&y.len())
}

pub fn natural(a: &str, b: &str) -> std::cmp::Ordering {
    natural_cmp(&natural_key(a), &natural_key(b)).then_with(|| a.cmp(b))
}

pub fn is_archive(e: &Entry) -> bool {
    if e.dir {
        return false;
    }
    let n = e.name.to_lowercase();
    [".zip", ".tar", ".tgz", ".txz", ".tbz", ".tbz2", ".tzst", ".gz", ".xz", ".bz2", ".zst", ".7z", ".rar", ".cbz", ".cbr", ".jar", ".apk"].iter().any(|x| n.ends_with(x))
}

pub fn archive_stem(name: &str) -> String {
    let low = name.to_lowercase();
    for x in [".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst"] {
        if low.ends_with(x) && name.len() > x.len() {
            return name[..name.len() - x.len()].to_string();
        }
    }
    match name.rfind('.') {
        Some(i) if i > 0 => name[..i].to_string(),
        _ => name.to_string(),
    }
}

/// The name as shown: without its extension when View › File name extensions is off.
pub fn shown_name(e: &Entry, show_ext: bool) -> String {
    if show_ext || e.dir {
        return e.name.clone();
    }
    match e.name.rfind('.') {
        Some(i) if i > 0 => e.name[..i].to_string(),
        _ => e.name.clone(),
    }
}

/// A glob ("*.tar.gz", "*.[jJ][pP][gG]", "Makefile") matched ignoring case (the picker's filters).
pub fn glob_match(glob: &str, name: &str) -> bool {
    fn m(g: &[char], n: &[char]) -> bool {
        if g.is_empty() {
            return n.is_empty();
        }
        match g[0] {
            '*' => (0..=n.len()).any(|i| m(&g[1..], &n[i..])),
            '?' => !n.is_empty() && m(&g[1..], &n[1..]),
            '[' => {
                if let Some(j) = g.iter().position(|c| *c == ']').filter(|j| *j > 1) {
                    if n.is_empty() {
                        return false;
                    }
                    let set = &g[1..j];
                    let (neg, set) = if set.first() == Some(&'!') { (true, &set[1..]) } else { (false, set) };
                    let c = n[0];
                    let mut hit = false;
                    let mut i = 0;
                    while i < set.len() {
                        if i + 2 < set.len() && set[i + 1] == '-' {
                            if set[i] <= c && c <= set[i + 2] {
                                hit = true;
                            }
                            i += 3;
                        } else {
                            if set[i] == c {
                                hit = true;
                            }
                            i += 1;
                        }
                    }
                    hit != neg && m(&g[j + 1..], &n[1..])
                } else {
                    !n.is_empty() && n[0] == '[' && m(&g[1..], &n[1..])
                }
            }
            c => !n.is_empty() && n[0] == c && m(&g[1..], &n[1..]),
        }
    }
    let g: Vec<char> = glob.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    m(&g, &n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words() {
        let e = Entry { name: "swaptest.py".into(), ..Default::default() };
        assert_eq!(kind_name(&e), "Python file");
        let d = Entry { name: "Desktop".into(), dir: true, ..Default::default() };
        assert_eq!(kind_name(&d), "File folder");
        assert_eq!(fmt_size(2048), "2 KB");
        assert_eq!(fmt_size(2_516_582), "2.4 MB");
        assert_eq!(fmt_space(96 * 1024 * 1024 * 1024), "96 GB");
        assert_eq!(base("/a/b/"), "b");
        assert_eq!(parent_of("/a"), "/");
        assert!(natural("file 9", "file 10").is_lt());
        assert!(glob_match("*.[jJ][pP][gG]", "Photo.JPG"));
        assert!(glob_match("*.tar.gz", "a.tar.gz"));
        assert!(!glob_match("*.png", "a.jpg"));
        assert_eq!(archive_stem("a.tar.gz"), "a");
        assert_eq!(parse_iso("1970-01-02T00:00:00Z"), 86400);
    }
}

//! Hearth Office: Hearth OS's word processor (Write), spreadsheet (Sheets) and presentations (Slides). The apps
//! themselves are web pages (ui/), drawn by the system's web view (WebKitGTK on Linux, WebView2 on Windows); this
//! side gives them what a page can't do on its own: read and write files, keep the recent-files list and the
//! recovery copies, print and export PDFs. Everything here works without a window, so the tests can check it.

use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Which of the three apps a window is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum App {
    Write,
    Sheets,
    Slides,
}

impl App {
    pub fn id(self) -> &'static str {
        match self { App::Write => "write", App::Sheets => "sheets", App::Slides => "slides" }
    }
    pub fn title(self) -> &'static str {
        match self { App::Write => "Hearth Write", App::Sheets => "Hearth Sheets", App::Slides => "Hearth Slides" }
    }
    pub fn parse(s: &str) -> Option<App> {
        match s.to_ascii_lowercase().as_str() {
            "write" | "writer" | "word" | "docs" | "hearth-write" => Some(App::Write),
            "sheets" | "sheet" | "calc" | "spreadsheet" | "hearth-sheets" => Some(App::Sheets),
            "slides" | "slide" | "impress" | "presentation" | "hearth-slides" => Some(App::Slides),
            _ => None,
        }
    }
    /// The app that opens a file, from its extension.
    pub fn for_file(path: &str) -> Option<App> {
        let ext = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "docx" | "odt" | "rtf" | "txt" | "md" | "markdown" | "html" | "htm" | "hwrite" | "doc" => Some(App::Write),
            "xlsx" | "xlsm" | "xls" | "ods" | "csv" | "tsv" | "hsheet" => Some(App::Sheets),
            "pptx" | "hslides" | "odp" => Some(App::Slides),
            _ => None,
        }
    }
    /// From the name the program was started as (hearth-write, hearth-sheets.exe...).
    pub fn from_program(argv0: &str) -> Option<App> {
        // Either kind of slash, whichever system the name came from.
        let base = argv0.rsplit(|c| c == '/' || c == '\\').next()?.to_ascii_lowercase();
        let stem = base.strip_suffix(".exe").unwrap_or(&base);
        match stem {
            "hearth-write" => Some(App::Write),
            "hearth-sheets" => Some(App::Sheets),
            "hearth-slides" => Some(App::Slides),
            _ => None,
        }
    }
}

/// How the program was started.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Launch {
    pub app: App,
    /// A file to open (absolute).
    pub file: Option<String>,
    /// `--convert IN OUT`: open IN, save it as OUT (the format from OUT's extension), then quit. For scripts
    /// and for the CI test that runs the real window.
    pub convert: Option<(String, String)>,
    pub fullscreen: bool,
    /// Slides: start presenting the file right away.
    pub present: bool,
}

pub const USAGE: &str = "Hearth Office: Hearth OS's word processor, spreadsheet and presentations.

Usage: hearth-office [--app write|sheets|slides] [FILE]
       hearth-write [FILE]   hearth-sheets [FILE]   hearth-slides [FILE]

  --app NAME            which app to open (otherwise from FILE's type, or the name it was started as)
  --convert IN OUT      open IN and save it as OUT (format from OUT's extension), then quit
  --present             Slides: start presenting FILE
  --fullscreen          start full screen
  -h, --help            this help
  -V, --version         the version
";

pub enum Parsed {
    Run(Launch),
    Help,
    Version,
}

fn absolute(p: &str) -> String {
    // Desktop files hand over file:// URIs (%U); those become plain paths.
    let p = if let Some(rest) = p.strip_prefix("file://") { percent_decode(rest) } else { p.to_string() };
    let pb = PathBuf::from(&p);
    if pb.is_absolute() { return p; }
    std::env::current_dir().map(|d| d.join(pb).to_string_lossy().into_owned()).unwrap_or(p)
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or(""), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Reads the command line (without the program name; `argv0` is the program name).
pub fn parse(argv0: &str, args: impl IntoIterator<Item = String>) -> Result<Parsed, String> {
    let mut app: Option<App> = None;
    let mut file: Option<String> = None;
    let mut convert = None;
    let mut fullscreen = false;
    let mut present = false;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => return Ok(Parsed::Help),
            "-V" | "--version" => return Ok(Parsed::Version),
            "--fullscreen" => fullscreen = true,
            "--present" => present = true,
            "--app" => {
                let v = it.next().ok_or("--app needs a name: write, sheets or slides")?;
                app = Some(App::parse(&v).ok_or_else(|| format!("there's no app called {v}: write, sheets or slides"))?);
            }
            "--convert" => {
                let i = it.next().ok_or("--convert needs a file to open and a file to save")?;
                let o = it.next().ok_or("--convert needs a file to save as well")?;
                convert = Some((absolute(&i), absolute(&o)));
            }
            s if s.starts_with("--app=") => {
                let v = &s[6..];
                app = Some(App::parse(v).ok_or_else(|| format!("there's no app called {v}: write, sheets or slides"))?);
            }
            s if s.starts_with('-') && s.len() > 1 => return Err(format!("unknown option {s}")),
            _ => {
                if file.is_none() { file = Some(absolute(&a)); }
            }
        }
    }
    let from_file = convert.as_ref().map(|(i, _)| i.clone()).or_else(|| file.clone());
    let app = app
        .or_else(|| App::from_program(argv0))
        .or_else(|| from_file.as_deref().and_then(App::for_file))
        .unwrap_or(App::Write);
    Ok(Parsed::Run(Launch { app, file, convert, fullscreen, present }))
}

// ------------------------------------------------------------------ where things are kept

/// ~/.config/hearth-office (settings and recent files).
pub fn config_dir() -> PathBuf {
    std::env::var_os("HEARTH_OFFICE_CONFIG").map(PathBuf::from)
        .or_else(|| dirs::config_dir().map(|d| d.join("hearth-office")))
        .unwrap_or_else(|| PathBuf::from(".hearth-office"))
}

/// ~/.local/share/hearth-office/recovery: what you were working on, saved every little while until you save it.
pub fn recovery_dir() -> PathBuf {
    std::env::var_os("HEARTH_OFFICE_DATA").map(|p| PathBuf::from(p).join("recovery"))
        .or_else(|| dirs::data_local_dir().map(|d| d.join("hearth-office").join("recovery")))
        .unwrap_or_else(|| PathBuf::from(".hearth-office-recovery"))
}

/// Writes a file whole or not at all: to a temporary file next to it first, then renamed over it.
pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() { if !dir.as_os_str().is_empty() { std::fs::create_dir_all(dir)?; } }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    let tmp = path.with_file_name(format!(".{name}.hearth-saving"));
    std::fs::write(&tmp, data)?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => { let _ = std::fs::remove_file(&tmp); Err(e) }
    }
}

fn read_json(path: &Path) -> Value {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null)
}

pub fn settings_load() -> Value {
    let v = read_json(&config_dir().join("settings.json"));
    if v.is_object() { v } else { json!({}) }
}

pub fn settings_save(v: &Value) -> std::io::Result<()> {
    write_atomic(&config_dir().join("settings.json"), &serde_json::to_vec_pretty(v)?)
}

// ------------------------------------------------------------------ recent files

pub const RECENT_MAX: usize = 25;

/// The recent files list with `path` moved to the top (for `app`), at most RECENT_MAX per app.
pub fn recent_with(list: &Value, app: App, path: &str, now: u64) -> Value {
    let mut items: Vec<Value> = list.as_array().cloned().unwrap_or_default();
    items.retain(|i| i.get("path").and_then(Value::as_str) != Some(path));
    let name = Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string());
    items.insert(0, json!({"path": path, "name": name, "app": app.id(), "time": now}));
    let mut per = std::collections::HashMap::<String, usize>::new();
    items.retain(|i| {
        let a = i.get("app").and_then(Value::as_str).unwrap_or("").to_string();
        let n = per.entry(a).or_insert(0);
        *n += 1;
        *n <= RECENT_MAX
    });
    Value::Array(items)
}

pub fn recent_path() -> PathBuf { config_dir().join("recent.json") }

pub fn recent_load() -> Value {
    let v = read_json(&recent_path());
    if v.is_array() { v } else { json!([]) }
}

/// The recent files for one app, newest first, each marked with whether it's still there.
pub fn recent_for(app: App) -> Value {
    let items: Vec<Value> = recent_load().as_array().cloned().unwrap_or_default().into_iter()
        .filter(|i| i.get("app").and_then(Value::as_str) == Some(app.id()))
        .map(|mut i| {
            let exists = i.get("path").and_then(Value::as_str).map(|p| Path::new(p).exists()).unwrap_or(false);
            i["exists"] = json!(exists);
            i
        })
        .collect();
    Value::Array(items)
}

pub fn recent_add(app: App, path: &str) -> std::io::Result<()> {
    let v = recent_with(&recent_load(), app, path, now());
    write_atomic(&recent_path(), &serde_json::to_vec_pretty(&v)?)
}

pub fn recent_remove(path: &str) -> std::io::Result<()> {
    let mut items: Vec<Value> = recent_load().as_array().cloned().unwrap_or_default();
    items.retain(|i| i.get("path").and_then(Value::as_str) != Some(path));
    write_atomic(&recent_path(), &serde_json::to_vec_pretty(&Value::Array(items))?)
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// ------------------------------------------------------------------ recovery copies

/// A recovery id is the page's own (letters, digits, - and _): anything else could reach outside the folder.
pub fn safe_id(id: &str) -> Option<String> {
    if id.is_empty() || id.len() > 80 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') { return None; }
    Some(id.to_string())
}

/// Keeps a recovery copy: `meta` (app, title, path) and `body` (the document in Hearth's own format).
pub fn recovery_save(id: &str, meta: &Value, body: &str) -> Result<(), String> {
    let id = safe_id(id).ok_or("bad recovery id")?;
    let dir = recovery_dir();
    let mut m = meta.clone();
    if !m.is_object() { m = json!({}); }
    m["time"] = json!(now());
    m["id"] = json!(id);
    write_atomic(&dir.join(format!("{id}.body")), body.as_bytes()).map_err(|e| e.to_string())?;
    write_atomic(&dir.join(format!("{id}.json")), &serde_json::to_vec(&m).unwrap_or_default()).map_err(|e| e.to_string())
}

pub fn recovery_clear(id: &str) {
    if let Some(id) = safe_id(id) {
        let dir = recovery_dir();
        let _ = std::fs::remove_file(dir.join(format!("{id}.json")));
        let _ = std::fs::remove_file(dir.join(format!("{id}.body")));
    }
}

/// Recovery copies left for `app` (by windows that closed without saving, or crashed), newest first.
pub fn recovery_list(app: App) -> Value {
    let mut out: Vec<Value> = std::fs::read_dir(recovery_dir()).into_iter().flatten().flatten()
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .map(|e| read_json(&e.path()))
        .filter(|m| m.get("app").and_then(Value::as_str) == Some(app.id()))
        .collect();
    out.sort_by_key(|m| std::cmp::Reverse(m.get("time").and_then(Value::as_u64).unwrap_or(0)));
    Value::Array(out)
}

pub fn recovery_load(id: &str) -> Result<String, String> {
    let id = safe_id(id).ok_or("bad recovery id")?;
    std::fs::read_to_string(recovery_dir().join(format!("{id}.body"))).map_err(|e| e.to_string())
}

/// Only web and mail links open from a document.
pub fn link_ok(url: &str) -> bool {
    let l = url.trim().to_ascii_lowercase();
    l.starts_with("http://") || l.starts_with("https://") || l.starts_with("mailto:")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(argv0: &str, a: &[&str]) -> Launch {
        match parse(argv0, a.iter().map(|s| s.to_string())).unwrap() { Parsed::Run(l) => l, _ => panic!("not a run") }
    }

    #[test]
    fn picks_the_app() {
        assert_eq!(run("hearth-office", &[]).app, App::Write);
        assert_eq!(run("/usr/bin/hearth-sheets", &[]).app, App::Sheets);
        assert_eq!(run("C:\\x\\hearth-slides.exe", &[]).app, App::Slides);
        assert_eq!(run("hearth-office", &["/tmp/a.XLSX"]).app, App::Sheets);
        assert_eq!(run("hearth-office", &["/tmp/a.pptx"]).app, App::Slides);
        assert_eq!(run("hearth-office", &["--app", "slides", "/tmp/a.docx"]).app, App::Slides);
        assert_eq!(run("hearth-office", &["--app=sheets"]).app, App::Sheets);
        assert_eq!(run("hearth-write", &["/tmp/a.csv"]).app, App::Write);
        assert!(parse("x", ["--app".to_string(), "paint".to_string()]).is_err());
        assert!(parse("x", ["--nope".to_string()]).is_err());
    }

    #[test]
    #[cfg(unix)] // the paths below are Unix ones
    fn files_and_convert() {
        let l = run("hearth-office", &["file:///tmp/My%20Doc.docx"]);
        assert_eq!(l.file.as_deref(), Some("/tmp/My Doc.docx"));
        let l = run("hearth-office", &["--convert", "/tmp/in.xlsx", "/tmp/out.csv"]);
        assert_eq!(l.app, App::Sheets);
        assert_eq!(l.convert, Some(("/tmp/in.xlsx".into(), "/tmp/out.csv".into())));
        let l = run("hearth-office", &["rel.txt"]);
        assert!(Path::new(l.file.as_deref().unwrap()).is_absolute());
        assert!(matches!(parse("x", ["--help".to_string()]), Ok(Parsed::Help)));
    }

    #[test]
    fn recent_list() {
        let mut v = json!([]);
        for i in 0..30 { v = recent_with(&v, App::Write, &format!("/d/{i}.docx"), i); }
        v = recent_with(&v, App::Sheets, "/d/s.xlsx", 99);
        v = recent_with(&v, App::Write, "/d/5.docx", 100);
        let a = v.as_array().unwrap();
        assert_eq!(a[0]["path"], "/d/5.docx");
        assert_eq!(a[0]["name"], "5.docx");
        assert_eq!(a.iter().filter(|i| i["app"] == "write").count(), RECENT_MAX);
        assert_eq!(a.iter().filter(|i| i["path"] == "/d/5.docx").count(), 1);
        assert_eq!(a.iter().filter(|i| i["app"] == "sheets").count(), 1);
    }

    #[test]
    fn recovery_round_trip() {
        let dir = std::env::temp_dir().join(format!("hearth-office-test-{}", std::process::id()));
        std::env::set_var("HEARTH_OFFICE_DATA", &dir);
        std::env::set_var("HEARTH_OFFICE_CONFIG", dir.join("cfg"));
        recovery_save("w-1", &json!({"app": "write", "title": "Letter"}), "{\"html\":\"<p>hi</p>\"}").unwrap();
        recovery_save("s-1", &json!({"app": "sheets", "title": "Budget"}), "{}").unwrap();
        assert!(recovery_save("../x", &json!({}), "").is_err());
        let l = recovery_list(App::Write);
        assert_eq!(l.as_array().unwrap().len(), 1);
        assert_eq!(l[0]["title"], "Letter");
        assert_eq!(recovery_load("w-1").unwrap(), "{\"html\":\"<p>hi</p>\"}");
        recovery_clear("w-1");
        assert_eq!(recovery_list(App::Write).as_array().unwrap().len(), 0);
        // Recent files and settings land in the config folder.
        let f = dir.join("doc.docx");
        std::fs::write(&f, b"x").unwrap();
        recent_add(App::Write, f.to_str().unwrap()).unwrap();
        let r = recent_for(App::Write);
        assert_eq!(r[0]["exists"], true);
        recent_remove(f.to_str().unwrap()).unwrap();
        assert_eq!(recent_for(App::Write).as_array().unwrap().len(), 0);
        settings_save(&json!({"theme": "light"})).unwrap();
        assert_eq!(settings_load()["theme"], "light");
        write_atomic(&dir.join("a/b.txt"), b"hello").unwrap();
        assert_eq!(std::fs::read(dir.join("a/b.txt")).unwrap(), b"hello");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn links() {
        assert!(link_ok("https://hearthos.fun"));
        assert!(link_ok("mailto:a@b.c"));
        assert!(!link_ok("file:///etc/passwd"));
        assert!(!link_ok("javascript:alert(1)"));
    }
}

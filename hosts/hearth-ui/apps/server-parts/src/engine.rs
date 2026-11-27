//! Running a part's engine: `python3 -I <parts>/<engine>.py ARGS`, its answer one JSON line ({"ok": true, ...} or
//! {"ok": false, "error": "..."}), off the window's thread. Input (never a key or a password: those go straight to the
//! keyring with secret-tool) goes on its stdin.
//!
//! Where the engines are: next to the Home server helper that's in use, so the Store's copy of the helper (a
//! root-owned system part, /var/lib/hearth/system-parts/hearth-home-server-helper/<version>/) brings its engines with
//! it; else the ones built into Hearth OS (/usr/libexec/nimbus/hearth-server-parts). The files are root's either way,
//! so nothing you run can change what an engine does. $HEARTH_SERVER_PARTS_DIR names another folder (the tests).
//!
//! CI's pictures: $HEARTH_SERVER_FIXTURES is a folder of made-up answers, `<engine>-<first argument>.json`
//! (dash-status.json); anything else answers {"ok": true}. Nothing runs then. Never set on a PC.

use hearth_ui::run::Output;
use std::path::PathBuf;

const BUILT_IN: &str = "/usr/libexec/nimbus/hearth-server-parts";
const HELPER: &str = "/usr/libexec/nimbus/hearth-home-server-helper";
const SYSTEM_PART: &str = "/usr/libexec/hearth/hearth-system-part";

pub fn fixtures() -> Option<PathBuf> {
    std::env::var_os("HEARTH_SERVER_FIXTURES").filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// The engines' folder (worked out once).
pub fn dir() -> PathBuf {
    static D: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    D.get_or_init(|| {
        if let Some(d) = std::env::var_os("HEARTH_SERVER_PARTS_DIR").filter(|v| !v.is_empty()) {
            return PathBuf::from(d);
        }
        if std::path::Path::new(SYSTEM_PART).is_file() {
            let o = hearth_ui::run::blocking(&[SYSTEM_PART.to_string(), "path".into(), crate::parts::HELPER_PART.into(), HELPER.into()]);
            let p = PathBuf::from(o.out.trim());
            if o.ok && p.is_absolute() {
                if let Some(d) = p.parent().map(|d| d.join("hearth-server-parts")) {
                    if d.join("common.py").is_file() {
                        return d;
                    }
                }
            }
        }
        PathBuf::from(BUILT_IN)
    })
    .clone()
}

/// The full command for `engine args`.
pub fn cmd(engine: &str, args: &[String]) -> Vec<String> {
    let mut c = vec!["python3".to_string(), "-I".into(), dir().join(format!("{engine}.py")).to_string_lossy().to_string()];
    c.extend(args.iter().cloned());
    c
}

/// Is this part's engine on this PC (its file is there)?
pub fn present(engine: &str) -> bool {
    if fixtures().is_some() {
        return true;
    }
    !engine.is_empty() && dir().join(format!("{engine}.py")).is_file()
}

fn fixture(engine: &str, args: &[String]) -> Option<Output> {
    let d = fixtures()?;
    let first = args.first().cloned().unwrap_or_default();
    let f = d.join(format!("{engine}-{first}.json"));
    let out = std::fs::read_to_string(&f).unwrap_or_else(|_| "{\"ok\": true, \"fixture\": true}".into());
    Some(Output { ok: true, code: 0, out, err: String::new() })
}

pub fn blocking(engine: &str, args: &[String], input: &str) -> Output {
    if let Some(o) = fixture(engine, args) {
        return o;
    }
    if !present(engine) {
        return Output { ok: false, code: -1, out: String::new(), err: "This part isn't on this PC yet: get the Home server helper's update in the Hearth Store".into() };
    }
    let mut full = vec!["timeout".to_string(), "-k".into(), "5".into(), "120".into()];
    full.extend(cmd(engine, args));
    hearth_oss::proc::blocking(&full, input.as_bytes(), &[])
}

/// `engine args` on its own thread.
pub fn spawn(engine: &str, args: Vec<String>, input: String) -> impl std::future::Future<Output = Output> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let e = engine.to_string();
    let _ = std::thread::Builder::new().name("hearth-server-part".into()).spawn(move || {
        let _ = tx.send(blocking(&e, &args, &input));
    });
    async move { rx.await.unwrap_or(Output { ok: false, code: -1, out: String::new(), err: "it stopped".into() }) }
}

/// What an answer says went wrong, in words ("" when it went well).
pub fn problem(o: &Output) -> String {
    match o.json() {
        Some(v) if v.get("ok").and_then(serde_json::Value::as_bool) == Some(false) => v.get("error").and_then(serde_json::Value::as_str).unwrap_or("That didn't work").to_string(),
        Some(_) if o.ok => String::new(),
        _ => {
            let w = o.why();
            if w.is_empty() { "That didn't work".into() } else { w }
        }
    }
}

/// The admin helper for what needs the system (installing a Store system part, its test version), through pkexec:
/// polkit asks for the admin password.
pub fn admin(args: &[&str]) -> Vec<String> {
    let mut c = vec!["pkexec".to_string(), "/usr/libexec/nimbus/hearth-admin".into()];
    c.extend(args.iter().map(|s| s.to_string()));
    c
}

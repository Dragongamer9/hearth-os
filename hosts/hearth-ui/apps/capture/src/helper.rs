//! Mantel's hearth-capture-helper does the work behind Capture here too (the same backend, so Debian's Mantel and
//! this app keep the same recent list, the same Pictures › Screenshots names and the same choices): taking the
//! picture, saving, copying, the recent list, reading text and QR codes, Share and the recorder. Each run is one
//! short-lived thread with a time limit; nothing polls.
//!
//! On KDE Plasma the picture itself comes from KWin's org.kde.KWin.ScreenShot2 (kwin.rs): the helper asks
//! hearth-screens, which asks the program in HEARTH_KWIN_CAST. Debian's Mantel on KWin has its own small trusted
//! program for that (/usr/libexec/hearth/hearth-kwin-cast); the Fedora edition doesn't build it, so there it is this
//! app (`hearth-capture-app shot …`), which KWin trusts through its menu entry (hearth-capture.desktop's
//! X-KDE-DBUS-Restricted-Interfaces).

use serde_json::Value;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn helper_path() -> String {
    std::env::var("HEARTH_CAPTURE_HELPER").unwrap_or_else(|_| hearth_ui::run::shell_helper("hearth-capture-helper"))
}

/// True on KDE Plasma (KWin): the session says so.
pub fn on_kwin() -> bool {
    match std::env::var("HEARTH_WM").ok().as_deref().map(str::trim) {
        Some("kwin") => return true,
        Some("labwc") => return false,
        _ => {}
    }
    std::env::var("XDG_CURRENT_DESKTOP").map(|d| d.to_uppercase().contains("KDE")).unwrap_or(false)
}

/// The environment the helper runs in: KWin named when it's Plasma, and this program as the one that asks KWin for
/// pictures when Hearth's own small helper for that isn't installed.
fn env_for(cmd: &mut Command) {
    if on_kwin() {
        cmd.env("HEARTH_WM", "kwin");
        let cast = "/usr/libexec/hearth/hearth-kwin-cast";
        let usable = std::fs::metadata(cast).map(|m| m.is_file()).unwrap_or(false);
        if std::env::var_os("HEARTH_KWIN_CAST").is_none() && !usable {
            if let Ok(me) = std::env::current_exe() {
                cmd.env("HEARTH_KWIN_CAST", me);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Out {
    pub ok: bool,
    pub out: String,
    pub err: String,
}

impl Out {
    /// The last JSON line it printed ({} when none).
    pub fn json(&self) -> Value {
        for l in self.out.lines().rev() {
            if let Ok(v) = serde_json::from_str::<Value>(l.trim()) {
                return v;
            }
        }
        Value::Object(Default::default())
    }
}

/// Runs PROG ARGS with INPUT on its standard input, for at most LIMIT; killed when it takes longer.
pub fn run_prog(prog: &str, args: &[String], input: Option<Vec<u8>>, limit: Duration) -> Out {
    let mut cmd = Command::new(prog);
    cmd.args(args).stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::piped());
    env_for(&mut cmd);
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Out { ok: false, out: String::new(), err: format!("{prog}: {e}") },
    };
    if let Some(data) = input {
        if let Some(mut si) = child.stdin.take() {
            std::thread::spawn(move || {
                let _ = si.write_all(&data);
            });
        }
    }
    let mut so = child.stdout.take();
    let mut se = child.stderr.take();
    let t_out = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(o) = so.as_mut() {
            let _ = o.read_to_end(&mut s);
        }
        s
    });
    let t_err = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(o) = se.as_mut() {
            let _ = o.read_to_end(&mut s);
        }
        s
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Some(st),
            Ok(None) if start.elapsed() < limit => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let out = String::from_utf8_lossy(&t_out.join().unwrap_or_default()).to_string();
    let err = String::from_utf8_lossy(&t_err.join().unwrap_or_default()).to_string();
    match status {
        Some(st) => Out { ok: st.success(), out, err },
        None => Out { ok: false, out, err: "it took too long and was stopped".into() },
    }
}

/// hearth-capture-helper ARGS (python3), waited for on a thread of its own.
pub fn call(args: Vec<String>, input: Option<Vec<u8>>, secs: u64) -> impl std::future::Future<Output = Value> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("capture-helper".into()).spawn(move || {
        let mut a = vec![helper_path()];
        a.extend(args);
        let o = run_prog("python3", &a, input, Duration::from_secs(secs));
        let mut v = o.json();
        if !v.is_object() {
            v = Value::Object(Default::default());
        }
        if v.as_object().map(|m| m.is_empty()).unwrap_or(true) && !o.ok {
            let why = o.err.trim().lines().last().unwrap_or("").to_string();
            v = serde_json::json!({"ok": false, "error": if why.is_empty() { "Capture's helper didn't answer".to_string() } else { why }});
        }
        let _ = tx.send(v);
    });
    async move { rx.await.unwrap_or_else(|_| serde_json::json!({"ok": false, "error": "it stopped"})) }
}

/// The screens that are on, for one area picker on each (Mantel's Capture: Quickshell's screens): Mantel's
/// hearth-screens get, which on KDE Plasma reads KDE's own screen settings (kscreen-doctor). Each: KWin's name for
/// it and where it is on the desktop in logical pixels (its mode over its scale, turned with it). Empty when they
/// can't be read (then the picker is on the screen you're on, as before).
pub fn screens() -> impl std::future::Future<Output = Vec<(String, crate::draw::R)>> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("capture-screens".into()).spawn(move || {
        let hs = hearth_ui::run::shell_helper("hearth-screens");
        let o = run_prog("python3", &[hs, "get".to_string()], None, Duration::from_secs(12));
        let v: Value = o.out.lines().rev().find_map(|l| serde_json::from_str::<Value>(l.trim()).ok().filter(Value::is_array)).unwrap_or(Value::Null);
        let _ = tx.send(parse_screens(&v));
    });
    async move { rx.await.unwrap_or_default() }
}

pub fn parse_screens(v: &Value) -> Vec<(String, crate::draw::R)> {
    let mut out = Vec::new();
    for o in v.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
        if o.get("enabled").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        let name = s(o, "name").to_string();
        let Some(m) = o.get("modes").and_then(Value::as_array).and_then(|l| l.iter().find(|m| b(m, "current"))) else { continue };
        let scale = o.get("scale").and_then(Value::as_f64).filter(|x| *x > 0.0).unwrap_or(1.0);
        let (mut w, mut h) = (n(m, "width") as f64 / scale, n(m, "height") as f64 / scale);
        if matches!(s(o, "transform"), "90" | "270" | "flipped-90" | "flipped-270") {
            std::mem::swap(&mut w, &mut h);
        }
        let pos = o.get("position").cloned().unwrap_or(Value::Null);
        if name.is_empty() || w < 1.0 || h < 1.0 {
            continue;
        }
        out.push((name, crate::draw::R::new(n(&pos, "x") as f32, n(&pos, "y") as f32, w.round() as f32, h.round() as f32)));
    }
    out
}

pub fn a(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Starts a program and doesn't wait (the recorder's pop-up, opening a link).
pub fn detached(prog: &str, args: &[String]) {
    let mut cmd = Command::new(prog);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    env_for(&mut cmd);
    let _ = cmd.spawn();
}

pub fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}
pub fn b(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}
pub fn n(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64))).unwrap_or(0)
}

/// Where the editor's own pictures go: $XDG_RUNTIME_DIR/hearth-capture (in RAM, the helper's folder; `clean` takes
/// the ones older than a day away).
pub fn runtime_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    let d = base.join("hearth-capture");
    let _ = std::fs::create_dir_all(&d);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o700));
    }
    d
}

#[cfg(test)]
mod tests {
    #[test]
    fn screens_are_read_in_logical_pixels() {
        let v = serde_json::json!([
            {"name": "DP-1", "enabled": true, "scale": 2.0, "transform": "normal", "position": {"x": 0, "y": 0},
             "modes": [{"width": 3840, "height": 2160, "current": true}]},
            {"name": "HDMI-A-1", "enabled": true, "scale": 1.0, "transform": "90", "position": {"x": 1920, "y": 0},
             "modes": [{"width": 1920, "height": 1080, "current": false}, {"width": 1280, "height": 720, "current": true}]},
            {"name": "eDP-1", "enabled": false, "modes": [{"width": 1920, "height": 1080, "current": true}]}
        ]);
        let l = super::parse_screens(&v);
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].0, "DP-1");
        assert_eq!((l[0].1.w, l[0].1.h), (1920.0, 1080.0));
        assert_eq!((l[1].1.x, l[1].1.w, l[1].1.h), (1920.0, 720.0, 1280.0));
    }
}

pub fn now_ms() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

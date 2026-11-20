//! `hearth-capture-app shot …`: a picture from KWin's org.kde.KWin.ScreenShot2 on KDE Plasma, the way Plasma means
//! it to be done. KWin answers that interface only for a program whose own .desktop file asks for it
//! (X-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2), found by the program's path: Capture's menu entry
//! (hearth-capture.desktop, Exec=/usr/bin/hearth-capture-app) asks for it, as Spectacle's does for Spectacle.
//!
//! The same command and output as Debian's hearth-kwin-cast shot (hosts/linux/desktop/kwin-helpers), so Hearth's
//! hearth-screens (Capture's helper, Print Screen, hearth-screenshot) uses this one where that isn't installed
//! (HEARTH_KWIN_CAST):
//!   shot screen NAME | window UUID | active-window | active-screen | workspace | area X Y W H [CURSOR 0|1 [FULL 0|1]]
//!   → one line "W H STRIDE FORMAT SCALE" (FORMAT: Qt's QImage format number), then the raw pixels.
//!   Exit 1 failed, 2 KWin refused, 3 not asked by Hearth's own hearth-screens, 4 no active window to take.
//! Like hearth-kwin-cast it answers only Hearth's hearth-screens (a root-owned file in a root-owned folder), so no
//! other program gets the screen just by running this one.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::fs::MetadataExt;
use zbus::zvariant::{Fd, OwnedValue, Value};

fn trusted_parent() -> bool {
    let ppid = unsafe { libc::getppid() };
    let Ok(raw) = std::fs::read(format!("/proc/{ppid}/cmdline")) else { return false };
    let parts: Vec<&[u8]> = raw.split(|b| *b == 0).filter(|p| !p.is_empty()).collect();
    // argv[0] = the interpreter, then maybe its options (-I, -E...), then the script.
    let Some(script) = parts.iter().skip(1).find(|p| !p.starts_with(b"-")) else { return false };
    let Ok(real) = std::fs::canonicalize(String::from_utf8_lossy(script).to_string()) else { return false };
    if real.file_name().and_then(|n| n.to_str()) != Some("hearth-screens") {
        return false;
    }
    let owned_by_root = |p: &std::path::Path| std::fs::metadata(p).map(|m| m.uid() == 0 && m.mode() & 0o022 == 0).unwrap_or(false);
    owned_by_root(&real) && real.parent().map(owned_by_root).unwrap_or(false)
}

fn u32_of(d: &HashMap<String, OwnedValue>, k: &str) -> u32 {
    match d.get(k) {
        Some(v) => u32::try_from(v).ok().or_else(|| i32::try_from(v).ok().map(|i| i.max(0) as u32)).unwrap_or(0),
        None => 0,
    }
}

const USAGE: &str = "hearth-capture-app: shot screen NAME | window UUID | active-window | active-screen | workspace | area X Y W H [CURSOR 0|1 [FULL 0|1]]";

/// The raw picture: (w, h, stride, format, scale, pixels), or (exit code, why).
pub fn capture(args: &[String]) -> Result<(u32, u32, u32, u32, f64, Vec<u8>), (i32, String)> {
    let kind = args.first().map(String::as_str).unwrap_or("");
    let rest = match kind {
        "screen" | "window" => 2,
        "area" => 5,
        _ => 1,
    };
    let num = |i: usize, d: i64| args.get(i).and_then(|v| v.parse::<i64>().ok()).unwrap_or(d);
    let cursor = num(rest, 0) != 0;
    let full = num(rest + 1, 1) != 0;
    let mut opts: HashMap<&str, Value> = HashMap::new();
    opts.insert("include-cursor", Value::from(cursor));
    opts.insert("native-resolution", Value::from(full));
    if kind == "window" || kind == "active-window" {
        opts.insert("include-decoration", Value::from(true));
        opts.insert("include-shadow", Value::from(false));
    }
    let mut fds = [0i32; 2];
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return Err((1, "no pipe".into()));
    }
    let (rd, wr) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
    let reader = std::thread::spawn(move || {
        let mut f = std::fs::File::from(rd);
        let mut v = Vec::new();
        let _ = f.read_to_end(&mut v);
        v
    });
    let conn = zbus::blocking::connection::Builder::session()
        .and_then(|b| b.method_timeout(std::time::Duration::from_secs(15)).build())
        .map_err(|e| (1, format!("no session bus: {e}")))?;
    let dest = Some("org.kde.KWin");
    let path = "/org/kde/KWin/ScreenShot2";
    let iface = Some("org.kde.KWin.ScreenShot2");
    let fd = Fd::from(&wr);
    let (method, reply) = match kind {
        "screen" if args.len() >= 2 => ("CaptureScreen", conn.call_method(dest, path, iface, "CaptureScreen", &(args[1].as_str(), opts, fd))),
        "window" if args.len() >= 2 => ("CaptureWindow", conn.call_method(dest, path, iface, "CaptureWindow", &(args[1].as_str(), opts, fd))),
        "active-window" => ("CaptureActiveWindow", conn.call_method(dest, path, iface, "CaptureActiveWindow", &(opts, fd))),
        "active-screen" => ("CaptureActiveScreen", conn.call_method(dest, path, iface, "CaptureActiveScreen", &(opts, fd))),
        "workspace" => ("CaptureWorkspace", conn.call_method(dest, path, iface, "CaptureWorkspace", &(opts, fd))),
        "area" if args.len() >= 5 => {
            let (x, y, w, h) = (num(1, 0) as i32, num(2, 0) as i32, num(3, 0).max(0) as u32, num(4, 0).max(0) as u32);
            ("CaptureArea", conn.call_method(dest, path, iface, "CaptureArea", &(x, y, w, h, opts, fd)))
        }
        _ => return Err((1, USAGE.into())),
    };
    // Our copy of the write end closes here: the reader sees the end of the picture once KWin has written it.
    drop(wr);
    let msg = match reply {
        Ok(m) => m,
        Err(e) => {
            let why = format!("{e}");
            let code = if why.contains("NoAuthorized") || why.contains("not authorized") || why.contains("Unauthorized") {
                2
            } else if why.contains("NoActiveWindow") {
                4
            } else {
                1
            };
            return Err((code, format!("{method} failed: {why}")));
        }
    };
    let d: HashMap<String, OwnedValue> = msg.body().deserialize().map_err(|e| (1, format!("KWin's answer: {e}")))?;
    let data = reader.join().unwrap_or_default();
    let (w, h, stride, fmt) = (u32_of(&d, "width"), u32_of(&d, "height"), u32_of(&d, "stride"), u32_of(&d, "format"));
    let scale = d.get("scale").and_then(|v| f64::try_from(v).ok()).unwrap_or(1.0);
    let need = stride as u64 * h as u64;
    if w == 0 || h == 0 || stride < w * 4 || need > data.len() as u64 || need > 1u64 << 31 {
        return Err((1, format!("KWin's picture was cut short ({} bytes for {w}x{h})", data.len())));
    }
    Ok((w, h, stride, fmt, scale, data[..need as usize].to_vec()))
}

/// The `shot` command (main.rs): prints the picture as hearth-kwin-cast does; its exit code.
pub fn shot_main(args: &[String]) -> i32 {
    if !trusted_parent() {
        eprintln!("hearth-capture-app: shot answers only Hearth's hearth-screens");
        return 3;
    }
    match capture(args) {
        Ok((w, h, stride, fmt, scale, data)) => {
            let mut o = std::io::stdout().lock();
            let ok = write!(o, "{w} {h} {stride} {fmt} {scale}\n").is_ok() && o.write_all(&data).is_ok() && o.flush().is_ok();
            if ok {
                0
            } else {
                1
            }
        }
        Err((code, why)) => {
            eprintln!("hearth-capture-app: {why}");
            code
        }
    }
}

// ------------------------------------------------------------------ where Capture's small windows go on KWin
/// A Wayland app can't place its own windows, so on Plasma a one-off KWin script puts Capture's pills where Mantel
/// puts them (the countdown and the scrolling capture's pill at the top or bottom of the screen in the middle, the
/// capture dot at the top right), keeps them above, out of the taskbar and, where KWin can, out of screenshots and
/// screen sharing. Elsewhere (and when KWin says no) they stay where the desktop put them.
#[derive(Debug, Clone, Copy)]
pub enum Place {
    Top(f32),
    Bottom(f32),
    TopRight(f32, f32),
}

/// The screen a pill goes on: the one named (a scrolling capture's area is on it), else the one you're on.
fn screen_expr(screen: &str) -> String {
    let active = "(workspace.activeScreen ? workspace.activeScreen.geometry : workspace.clientArea(KWin.FullScreenArea, w))";
    if screen.is_empty() {
        return active.to_string();
    }
    format!("(function () {{ var o = workspace.screens; for (var j = 0; j < o.length; j++) if (String(o[j].name) === {}) return o[j].geometry; return {active}; }})()", js_str(screen))
}

/// A JavaScript string literal (names come from KDE's screen settings; nothing else is put in a script).
fn js_str(v: &str) -> String {
    serde_json::Value::from(v).to_string()
}

fn script_for(app_id: &str, w: f32, h: f32, place: Place, screen: &str) -> String {
    let (xexpr, yexpr) = match place {
        Place::Top(m) => ("s.x + (s.width - W) / 2".to_string(), format!("s.y + {m}")),
        Place::Bottom(m) => ("s.x + (s.width - W) / 2".to_string(), format!("s.y + s.height - H - {m}")),
        Place::TopRight(top, right) => (format!("s.x + s.width - W - {right}"), format!("s.y + {top}")),
    };
    let id = js_str(app_id);
    let sx = screen_expr(screen);
    // Out of screenshots and screen sharing (KWin's excludeFromCapture). CI's pictures of where the pills go set
    // HEARTH_CAPTURE_CI_IN_PICTURES, as KWin's own screenshots are then the only way to see them.
    let exclude = if std::env::var_os("HEARTH_CAPTURE_CI_IN_PICTURES").is_some() { "" } else { "\n    if ('excludeFromCapture' in w) w.excludeFromCapture = true;" };
    format!(
        "(function () {{\n  var l = workspace.windowList();\n  for (var i = 0; i < l.length; i++) {{\n    var w = l[i];\n    if (String(w.resourceClass) !== {id} && String(w.desktopFileName) !== {id}) continue;\n    var s = {sx};\n    var W = {w}, H = {h};\n    w.frameGeometry = {{ x: Math.round({xexpr}), y: Math.round({yexpr}), width: W, height: H }};\n    w.keepAbove = true; w.skipTaskbar = true; w.skipPager = true; w.skipSwitcher = true;{exclude}\n  }}\n}})();\n"
    )
}

/// The area pickers, one on each screen (Mantel's: a layer-shell window per screen, which a Wayland app can't ask
/// for itself on KWin): each picker window (its own name, hearth-capture-pick-N) sent to its screen and made full
/// screen there, kept above and out of the taskbar and Alt+Tab. In the pictures they're still there (they show the
/// still picture you pick on).
fn script_pickers(list: &[(String, String)]) -> String {
    let pairs: Vec<String> = list.iter().map(|(id, name)| format!("{}: {}", js_str(id), js_str(name))).collect();
    format!(
        "(function () {{\n  var want = {{ {} }};\n  var outs = workspace.screens;\n  var l = workspace.windowList();\n  for (var i = 0; i < l.length; i++) {{\n    var w = l[i];\n    var name = want[String(w.resourceClass)];\n    if (name === undefined) name = want[String(w.desktopFileName)];\n    if (name === undefined) continue;\n    var o = null;\n    for (var j = 0; j < outs.length; j++) if (String(outs[j].name) === name) o = outs[j];\n    if (!o) continue;\n    if (w.output !== o) workspace.sendClientToScreen(w, o);\n    var g = o.geometry;\n    if (!w.fullScreen) w.frameGeometry = {{ x: g.x, y: g.y, width: g.width, height: g.height }};\n    w.fullScreen = true;\n    w.keepAbove = true; w.skipTaskbar = true; w.skipPager = true; w.skipSwitcher = true;\n  }}\n}})();\n",
        pairs.join(", ")
    )
}

/// Runs a one-off KWin script (loaded, run, unloaded; its file gone), on KWin only.
fn run_script(script: &str) {
    let name = format!("hearth-capture-place-{}", crate::helper::now_ms());
    let path = crate::helper::runtime_dir().join(format!("{name}.js"));
    if std::fs::write(&path, script).is_err() {
        return;
    }
    let Ok(conn) = zbus::blocking::connection::Builder::session().and_then(|b| b.method_timeout(std::time::Duration::from_secs(5)).build()) else {
        let _ = std::fs::remove_file(&path);
        return;
    };
    let path_s = path.to_string_lossy().to_string();
    let r = conn.call_method(Some("org.kde.KWin"), "/Scripting", Some("org.kde.kwin.Scripting"), "loadScript", &(path_s.as_str(), name.as_str()));
    if let Ok(m) = r {
        if let Ok(id) = m.body().deserialize::<i32>() {
            // KWin 6: /Scripting/Script<id>; KWin 5: /<id>.
            let p6 = format!("/Scripting/Script{id}");
            if conn.call_method(Some("org.kde.KWin"), p6.as_str(), Some("org.kde.kwin.Script"), "run", &()).is_err() {
                let p5 = format!("/{id}");
                let _ = conn.call_method(Some("org.kde.KWin"), p5.as_str(), Some("org.kde.kwin.Script"), "run", &());
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        let _ = conn.call_method(Some("org.kde.KWin"), "/Scripting", Some("org.kde.kwin.Scripting"), "unloadScript", &(name.as_str(),));
    }
    let _ = std::fs::remove_file(&path);
}

pub fn place(app_id: &str, w: f32, h: f32, place: Place, screen: &str) {
    if !crate::helper::on_kwin() {
        return;
    }
    // The window is on the screen first.
    std::thread::sleep(std::time::Duration::from_millis(250));
    run_script(&script_for(app_id, w, h, place, screen));
}

pub fn place_async(app_id: &'static str, w: f32, h: f32, p: Place, screen: String) -> impl std::future::Future<Output = ()> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("capture-place".into()).spawn(move || {
        place(app_id, w, h, p, &screen);
        let _ = tx.send(());
    });
    async move {
        let _ = rx.await;
    }
}

/// The pickers on their screens: once they're up, and again a moment later for one that was slow to show (the
/// script only moves a picker that isn't where it belongs).
pub fn place_pickers_async(list: Vec<(String, String)>) -> impl std::future::Future<Output = ()> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("capture-place".into()).spawn(move || {
        if crate::helper::on_kwin() && !list.is_empty() {
            let js = script_pickers(&list);
            std::thread::sleep(std::time::Duration::from_millis(300));
            run_script(&js);
            std::thread::sleep(std::time::Duration::from_millis(900));
            run_script(&js);
        }
        let _ = tx.send(());
    });
    async move {
        let _ = rx.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pill_script_names_the_window_and_the_place() {
        let s = script_for("hearth-capture-pill", 260.0, 64.0, Place::Top(40.0), "");
        assert!(s.contains("\"hearth-capture-pill\"") && s.contains("s.y + 40") && s.contains("excludeFromCapture") && s.contains("activeScreen"));
        let s = script_for("hearth-capture-dot", 200.0, 34.0, Place::TopRight(56.0, 12.0), "");
        assert!(s.contains("s.width - W - 12") && s.contains("s.y + 56"));
        let s = script_for("hearth-capture-pill", 300.0, 56.0, Place::Bottom(24.0), "HDMI-A-1");
        assert!(s.contains("\"HDMI-A-1\"") && s.contains("s.y + s.height - H - 24"));
    }

    #[test]
    fn each_picker_goes_full_screen_on_its_own_screen() {
        let s = script_pickers(&[("hearth-capture-pick-0".into(), "DP-1".into()), ("hearth-capture-pick-1".into(), "HDMI-A-1\"';".into())]);
        assert!(s.contains("\"hearth-capture-pick-0\": \"DP-1\""));
        assert!(s.contains("sendClientToScreen") && s.contains("w.fullScreen = true"));
        // A name is only ever a string in the script.
        assert!(s.contains("\"HDMI-A-1\\\"';\""));
    }
}

//! What the Personalization, Accounts, Privacy & security and Accessibility pages share (their own, beside
//! widgets.rs): running Hearth's helpers the way Mantel's SetJob does (text for the helper on its stdin and in
//! HEARTH_INPUT, never on the command line; a timeout on every run), KDE Plasma's own scripting (plasmashell's
//! evaluateScript, for the panel and the wallpapers on the Fedora edition), a line-by-line run for the long ones
//! (a browser sign-in, adding a fingerprint), and the few pieces of page these pages need that widgets.rs hasn't got
//! (their icons, a drop-down list, chips, banners).

use crate::widgets::*;
use hearth_ui::run;
use hearth_ui::ui::{self, glyph, t, t1, tm, ts, G};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::widget::{column, container, mouse_area, row, scrollable, text_input, Column, Row, Space};
use iced::{mouse, Color, Length, Padding, Point, Rectangle, Renderer, Theme};
use serde_json::Value;
use std::future::Future;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

/// One of Hearth's programs in /usr/libexec/nimbus ($HEARTH_LIBEXEC instead: CI runs the repo's copies).
pub fn lx(name: &str) -> String {
    let d = std::env::var("HEARTH_LIBEXEC").unwrap_or_else(|_| "/usr/libexec/nimbus".into());
    format!("{d}/{name}")
}

// ------------------------------------------------------------------ where we are
/// KDE Plasma is the desktop (the Fedora edition's Desktop Mode).
pub fn on_plasma() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP").map(|d| d.split(':').any(|x| x.eq_ignore_ascii_case("KDE"))).unwrap_or(false)
}

/// What Hearth's helpers are told about the desktop. On the Fedora edition KDE Plasma runs on KWin, so the helpers'
/// KWin side (hearth-kwin-settings, hearth-access) is the one to use: HEARTH_WM=kwin, as hearth-screenshot does when
/// it finds itself on KDE. Mantel on KWin (Debian) sets it itself.
pub fn helper_env() -> Vec<(String, String)> {
    if std::env::var_os("HEARTH_WM").is_none() && on_plasma() {
        vec![("HEARTH_WM".into(), "kwin".into())]
    } else {
        Vec::new()
    }
}

/// `python3 <helper> args…` for one of Hearth's helpers in the shell folder (or an absolute path), with a timeout.
pub fn job(helper: &str, args: &[&str]) -> Vec<String> {
    job_t(helper, args, 60)
}
pub fn job_t(helper: &str, args: &[&str], secs: u32) -> Vec<String> {
    // CI's pictures: what a helper would answer, from $HEARTH_JOB_FIXTURES/<helper>-<args joined by _>.json.
    if let Some(d) = std::env::var_os("HEARTH_JOB_FIXTURES") {
        let f = std::path::Path::new(&d).join(format!("{}-{}.json", base(helper), args.join("_")));
        if f.is_file() {
            return vec!["cat".into(), f.to_string_lossy().to_string()];
        }
    }
    let path = if helper.starts_with('/') { helper.to_string() } else { run::shell_helper(helper) };
    let mut v = vec!["timeout".into(), "-k".into(), "5".into(), secs.to_string(), "python3".into(), path];
    v.extend(args.iter().map(|s| s.to_string()));
    v
}
/// `pkexec hearth-admin args…` (Hearth's polkit rule lets the person at the device do this without a prompt).
pub fn admin(args: &[&str]) -> Vec<String> {
    let mut v = vec!["timeout".into(), "-k".into(), "5".into(), "600".into(), "pkexec".into(), lx("hearth-admin")];
    v.extend(args.iter().map(|s| s.to_string()));
    v
}
pub fn admin_of(path: &str, args: &[&str]) -> Vec<String> {
    let mut v = vec!["timeout".into(), "-k".into(), "5".into(), "600".into(), "pkexec".into(), path.into()];
    v.extend(args.iter().map(|s| s.to_string()));
    v
}
pub fn sh_t(script: &str, secs: u32) -> Vec<String> {
    vec!["timeout".into(), "-k".into(), "2".into(), secs.to_string(), "sh".into(), "-c".into(), script.into()]
}

/// Runs `cmd` on its own thread with `input` on its stdin (and in HEARTH_INPUT, which only you can read), as SetJob.
pub fn run_in(cmd: Vec<String>, input: String) -> impl Future<Output = run::Output> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-settings-job".into()).spawn(move || {
        let _ = tx.send(blocking_in(&cmd, &input));
    });
    async move { rx.await.unwrap_or(run::Output { ok: false, code: -1, out: String::new(), err: "it stopped".into() }) }
}

fn command(cmd: &[String], input: &str) -> Command {
    let mut c = Command::new(&cmd[0]);
    c.args(&cmd[1..]);
    for (k, v) in helper_env() {
        c.env(k, v);
    }
    c.env("HEARTH_INPUT", input);
    c.stdin(if input.is_empty() { Stdio::null() } else { Stdio::piped() });
    c
}

fn blocking_in(cmd: &[String], input: &str) -> run::Output {
    if cmd.is_empty() {
        return run::Output { ok: false, code: -1, out: String::new(), err: "nothing to run".into() };
    }
    let mut c = command(cmd, input);
    c.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match c.spawn() {
        Ok(ch) => ch,
        Err(e) => return run::Output { ok: false, code: -1, out: String::new(), err: format!("{}: {e}", cmd[0]) },
    };
    if !input.is_empty() {
        if let Some(mut si) = child.stdin.take() {
            let _ = si.write_all(input.as_bytes());
        }
    }
    match child.wait_with_output() {
        Ok(o) => run::Output { ok: o.status.success(), code: o.status.code().unwrap_or(-1), out: String::from_utf8_lossy(&o.stdout).to_string(), err: String::from_utf8_lossy(&o.stderr).to_string() },
        Err(e) => run::Output { ok: false, code: -1, out: String::new(), err: e.to_string() },
    }
}

/// What a helper answered, as SetJob reads it: its "error: …" line, else its last JSON line.
pub fn answer(o: &run::Output) -> Result<Value, String> {
    let all = format!("{}\n{}", o.out, o.err);
    if let Some(e) = all.lines().find_map(|l| l.strip_prefix("error: ")) {
        return Err(e.trim().to_string());
    }
    if let Some(v) = o.out.lines().rev().find_map(|l| if l.trim_start().starts_with('{') { serde_json::from_str::<Value>(l.trim()).ok() } else { None }) {
        return Ok(v);
    }
    if o.code == 126 || o.code == 127 {
        return Err("Not allowed (Hearth's polkit rule is missing)".into());
    }
    if o.code == 124 {
        return Err("It took too long and was stopped".into());
    }
    if o.ok {
        Ok(Value::Null)
    } else {
        let w = o.why();
        Err(if w.is_empty() { "It didn't work".into() } else { w })
    }
}

// ------------------------------------------------------------------ a run, line by line
#[derive(Debug, Clone)]
pub enum Line {
    Progress(f32, String),
    Json(Value),
    Err(String),
    End(i32),
}

/// A run whose lines come as they're printed ("progress: <0-100> <what>", JSON lines, "error: …"), then End.
/// The pid is kept so Cancel can stop it.
pub fn lines(cmd: Vec<String>, input: String) -> (futures::channel::mpsc::UnboundedReceiver<Line>, Arc<AtomicI32>) {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let pid = Arc::new(AtomicI32::new(0));
    let p2 = pid.clone();
    let _ = std::thread::Builder::new().name("hearth-settings-lines".into()).spawn(move || {
        if cmd.is_empty() {
            let _ = tx.unbounded_send(Line::End(-1));
            return;
        }
        let mut c = command(&cmd, &input);
        c.stdout(Stdio::piped()).stderr(Stdio::null());
        let mut child = match c.spawn() {
            Ok(ch) => ch,
            Err(e) => {
                let _ = tx.unbounded_send(Line::Err(format!("{}: {e}", cmd[0])));
                let _ = tx.unbounded_send(Line::End(-1));
                return;
            }
        };
        p2.store(child.id() as i32, Ordering::Relaxed);
        if !input.is_empty() {
            if let Some(mut si) = child.stdin.take() {
                let _ = si.write_all(input.as_bytes());
            }
        }
        if let Some(out) = child.stdout.take() {
            for l in BufReader::new(out).lines().map_while(Result::ok) {
                let l = l.trim().to_string();
                let msg = if let Some(r) = l.strip_prefix("progress: ") {
                    let mut it = r.splitn(2, ' ');
                    let n = it.next().and_then(|x| x.parse::<f32>().ok()).unwrap_or(-100.0) / 100.0;
                    Line::Progress(n, it.next().unwrap_or("").to_string())
                } else if let Some(e) = l.strip_prefix("error: ") {
                    Line::Err(e.to_string())
                } else if l.starts_with('{') {
                    match serde_json::from_str::<Value>(&l) {
                        Ok(v) => Line::Json(v),
                        Err(_) => continue,
                    }
                } else {
                    continue;
                };
                let _ = tx.unbounded_send(msg);
            }
        }
        let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
        p2.store(0, Ordering::Relaxed);
        let _ = tx.unbounded_send(Line::End(code));
    });
    (rx, pid)
}

/// Stops a run started with `lines` (Cancel).
pub fn stop(pid: &Arc<AtomicI32>) {
    let p = pid.swap(0, Ordering::Relaxed);
    if p > 0 {
        unsafe {
            libc::kill(p, libc::SIGTERM);
        }
    }
}

// ------------------------------------------------------------------ KDE Plasma's scripting (Fedora edition)
/// plasmashell runs `js` (KDE's desktop scripting: panels(), desktops()) and hands back what it print()ed. In CI,
/// HEARTH_PLASMA_FIXTURE is a file of what a read prints (the pictures); nothing is ever sent then.
pub fn plasma(js: &str) -> Vec<String> {
    if let Some(f) = std::env::var_os("HEARTH_PLASMA_FIXTURE") {
        return vec!["cat".into(), f.to_string_lossy().to_string()];
    }
    vec![
        "timeout".into(), "10".into(), "dbus-send".into(), "--session".into(), "--print-reply".into(), "--dest=org.kde.plasmashell".into(),
        "/PlasmaShell".into(), "org.kde.PlasmaShell.evaluateScript".into(), format!("string:{js}"),
    ]
}
/// A change through plasmashell's scripting (never sent while pictures are taken from a fixture).
pub fn plasma_do(js: &str) -> Vec<String> {
    if std::env::var_os("HEARTH_PLASMA_FIXTURE").is_some() {
        return vec!["true".into()];
    }
    plasma(js)
}
/// What a plasmashell script print()ed as JSON (dbus-send shows it as `string "…"`).
pub fn plasma_json(o: &run::Output) -> Option<Value> {
    if !o.ok {
        return None;
    }
    let s = o.out.trim();
    let body = match (s.find("string \""), s.rfind('"')) {
        (Some(a), Some(b)) if b > a + 8 => &s[a + 8..b],
        _ => s,
    };
    serde_json::from_str(body.trim()).ok()
}
/// plasmashell's error, said plainly.
pub fn plasma_why(o: &run::Output) -> String {
    if o.ok {
        return String::new();
    }
    let w = o.why();
    if w.contains("ServiceUnknown") || w.contains("was not provided") || w.contains("No such file") || o.code == 127 {
        "KDE Plasma's desktop isn't running here".into()
    } else {
        w
    }
}
/// A JavaScript string literal.
pub fn js_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

// ------------------------------------------------------------------ icons these pages use
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ic {
    G(G),
    Lock,
    Key,
    Image,
    Trash,
    Mouse,
    Kbd,
    Eye,
    Bell,
    Finger,
    Window,
    Desktops,
    Motion,
    Cloud,
    Globe,
    Vault,
    Cpu,
    Box,
    Sparkle,
    Wave,
    Edit,
    Sync,
    Calendar,
    Text,
    Camera,
    Menu,
    Warn,
    Undo,
    Mail,
}

struct Draw(Ic, Color, f32);

fn arc(cx: f32, cy: f32, r: f32, a0: f32, a1: f32, n: usize) -> Vec<(f32, f32)> {
    (0..=n).map(|i| { let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians(); (cx + r * a.cos(), cy + r * a.sin()) }).collect()
}
fn rect(x: f32, y: f32, w: f32, h: f32) -> Vec<(f32, f32)> {
    vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)]
}

fn ic_lines(i: Ic) -> (Vec<Vec<(f32, f32)>>, Vec<(f32, f32, f32)>) {
    let v = |p: &[(f32, f32)]| p.to_vec();
    match i {
        Ic::G(_) => (vec![], vec![]),
        Ic::Lock => (vec![rect(5.0, 11.0, 14.0, 10.0), arc(12.0, 11.0, 4.5, 180.0, 360.0, 12)], vec![(12.0, 16.0, 1.1)]),
        Ic::Key => (vec![v(&[(11.0, 11.0), (20.0, 11.0), (20.0, 14.0)]), v(&[(16.5, 11.0), (16.5, 14.0)])], vec![(7.0, 11.0, 4.0)]),
        Ic::Image => (vec![rect(3.0, 5.0, 18.0, 14.0), v(&[(3.0, 17.0), (9.0, 11.0), (13.0, 15.0), (16.0, 12.0), (21.0, 17.0)])], vec![(16.0, 8.5, 1.4)]),
        Ic::Trash => (vec![v(&[(4.0, 6.0), (20.0, 6.0)]), v(&[(9.0, 6.0), (9.0, 3.5), (15.0, 3.5), (15.0, 6.0)]), v(&[(6.0, 6.0), (7.0, 21.0), (17.0, 21.0), (18.0, 6.0)]), v(&[(10.0, 10.0), (10.0, 17.0)]), v(&[(14.0, 10.0), (14.0, 17.0)])], vec![]),
        Ic::Mouse => (vec![v(&[(7.0, 8.0), (7.0, 16.0)]), v(&[(17.0, 8.0), (17.0, 16.0)]), arc(12.0, 8.0, 5.0, 180.0, 360.0, 10), arc(12.0, 16.0, 5.0, 0.0, 180.0, 10), v(&[(12.0, 6.0), (12.0, 9.5)])], vec![]),
        Ic::Kbd => (vec![rect(2.0, 6.0, 20.0, 12.0), v(&[(7.0, 14.5), (17.0, 14.5)])], vec![(6.0, 9.5, 0.8), (9.5, 9.5, 0.8), (13.0, 9.5, 0.8), (16.5, 9.5, 0.8)]),
        Ic::Eye => {
            let mut a = arc(12.0, 19.0, 11.0, 215.0, 325.0, 14);
            a.extend(arc(12.0, 5.0, 11.0, 35.0, 145.0, 14));
            (vec![a], vec![(12.0, 12.0, 3.2)])
        }
        Ic::Bell => (vec![v(&[(5.0, 17.0), (7.0, 14.0), (7.0, 10.0)]), arc(12.0, 10.0, 5.0, 180.0, 360.0, 10), v(&[(17.0, 10.0), (17.0, 14.0), (19.0, 17.0), (5.0, 17.0)]), v(&[(10.0, 20.0), (14.0, 20.0)])], vec![]),
        Ic::Finger => (vec![arc(12.0, 13.0, 8.0, 200.0, 340.0, 12), arc(12.0, 13.0, 5.0, 180.0, 380.0, 14), v(&[(12.0, 11.0), (12.0, 20.0)])], vec![]),
        Ic::Window => (vec![rect(3.0, 4.0, 18.0, 16.0), v(&[(3.0, 8.5), (21.0, 8.5)])], vec![(6.0, 6.2, 0.7), (8.5, 6.2, 0.7)]),
        Ic::Desktops => (vec![rect(2.0, 6.0, 13.0, 10.0), v(&[(6.0, 3.0), (22.0, 3.0), (22.0, 13.0)]), v(&[(5.0, 20.0), (12.0, 20.0)])], vec![]),
        Ic::Motion => (vec![v(&[(3.0, 8.0), (10.0, 8.0)]), v(&[(2.0, 12.0), (9.0, 12.0)]), v(&[(3.0, 16.0), (10.0, 16.0)])], vec![(16.0, 12.0, 5.0)]),
        Ic::Cloud => {
            let mut a = arc(8.0, 15.0, 4.0, 90.0, 270.0, 8);
            a.extend(arc(13.0, 11.0, 5.0, 200.0, 340.0, 10));
            a.extend(arc(17.0, 15.0, 4.0, 270.0, 450.0, 8));
            a.push((8.0, 19.0));
            (vec![a], vec![])
        }
        Ic::Globe => (vec![v(&[(3.0, 12.0), (21.0, 12.0)]), v(&[(12.0, 3.0), (12.0, 21.0)]), arc(19.0, 12.0, 10.0, 155.0, 205.0, 8), arc(5.0, 12.0, 10.0, -25.0, 25.0, 8)], vec![(12.0, 12.0, 9.0)]),
        Ic::Vault => (vec![rect(3.0, 4.0, 18.0, 16.0), v(&[(12.0, 9.0), (12.0, 15.0)]), v(&[(9.0, 12.0), (15.0, 12.0)])], vec![(12.0, 12.0, 4.0)]),
        Ic::Cpu => (vec![rect(6.0, 6.0, 12.0, 12.0), rect(9.5, 9.5, 5.0, 5.0), v(&[(9.0, 3.0), (9.0, 6.0)]), v(&[(15.0, 3.0), (15.0, 6.0)]), v(&[(9.0, 18.0), (9.0, 21.0)]), v(&[(15.0, 18.0), (15.0, 21.0)]), v(&[(3.0, 9.0), (6.0, 9.0)]), v(&[(3.0, 15.0), (6.0, 15.0)]), v(&[(18.0, 9.0), (21.0, 9.0)]), v(&[(18.0, 15.0), (21.0, 15.0)])], vec![]),
        Ic::Box => (vec![v(&[(12.0, 3.0), (20.0, 7.0), (20.0, 17.0), (12.0, 21.0), (4.0, 17.0), (4.0, 7.0), (12.0, 3.0)]), v(&[(4.0, 7.0), (12.0, 11.0), (20.0, 7.0)]), v(&[(12.0, 11.0), (12.0, 21.0)])], vec![]),
        Ic::Sparkle => (vec![v(&[(12.0, 3.0), (13.5, 10.5), (21.0, 12.0), (13.5, 13.5), (12.0, 21.0), (10.5, 13.5), (3.0, 12.0), (10.5, 10.5), (12.0, 3.0)])], vec![]),
        Ic::Wave => (vec![(0..=24).map(|i| { let x = 2.0 + 20.0 * i as f32 / 24.0; (x, 12.0 + 4.0 * (x * 0.6).sin()) }).collect()], vec![]),
        Ic::Edit => (vec![v(&[(4.0, 20.0), (5.0, 15.0), (16.0, 4.0), (20.0, 8.0), (9.0, 19.0), (4.0, 20.0)]), v(&[(14.0, 6.0), (18.0, 10.0)])], vec![]),
        Ic::Sync => (vec![arc(12.0, 12.0, 8.0, 200.0, 340.0, 12), v(&[(18.5, 4.5), (19.5, 8.0), (16.0, 8.5)]), arc(12.0, 12.0, 8.0, 20.0, 160.0, 12), v(&[(5.5, 19.5), (4.5, 16.0), (8.0, 15.5)])], vec![]),
        Ic::Calendar => (vec![rect(3.0, 5.0, 18.0, 16.0), v(&[(3.0, 10.0), (21.0, 10.0)]), v(&[(8.0, 3.0), (8.0, 7.0)]), v(&[(16.0, 3.0), (16.0, 7.0)])], vec![]),
        Ic::Text => (vec![v(&[(5.0, 5.0), (19.0, 5.0)]), v(&[(12.0, 5.0), (12.0, 20.0)]), v(&[(9.0, 20.0), (15.0, 20.0)])], vec![]),
        Ic::Camera => (vec![v(&[(3.0, 8.0), (7.0, 8.0), (9.0, 5.0), (15.0, 5.0), (17.0, 8.0), (21.0, 8.0), (21.0, 19.0), (3.0, 19.0), (3.0, 8.0)])], vec![(12.0, 13.0, 3.8)]),
        Ic::Menu => (vec![v(&[(4.0, 6.0), (20.0, 6.0)]), v(&[(4.0, 12.0), (20.0, 12.0)]), v(&[(4.0, 18.0), (20.0, 18.0)])], vec![]),
        Ic::Warn => (vec![v(&[(12.0, 3.0), (22.0, 20.0), (2.0, 20.0), (12.0, 3.0)]), v(&[(12.0, 9.0), (12.0, 14.0)])], vec![(12.0, 17.0, 1.0)]),
        Ic::Undo => (vec![v(&[(8.0, 5.0), (4.0, 9.0), (8.0, 13.0)]), {
            let mut a = vec![(4.0, 9.0), (14.0, 9.0)];
            a.extend(arc(14.0, 14.5, 5.5, 270.0, 450.0, 10));
            a.push((8.0, 20.0));
            a
        }], vec![]),
        Ic::Mail => (vec![rect(3.0, 6.0, 18.0, 13.0), v(&[(3.0, 6.0), (12.0, 13.0), (21.0, 6.0)])], vec![]),
    }
}

impl<M> canvas::Program<M> for Draw {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let k = bounds.width.min(bounds.height) / 24.0;
        let (ls, cs) = ic_lines(self.0);
        let stroke = || Stroke::default().with_color(self.1).with_width(self.2 * k.max(0.5)).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round);
        for l in ls {
            if l.len() < 2 {
                continue;
            }
            let path = Path::new(|b| {
                b.move_to(Point::new(l[0].0 * k, l[0].1 * k));
                for pt in &l[1..] {
                    b.line_to(Point::new(pt.0 * k, pt.1 * k));
                }
            });
            f.stroke(&path, stroke());
        }
        for (x, y, r) in cs {
            let path = Path::circle(Point::new(x * k, y * k), r * k);
            if r < 2.0 {
                f.fill(&path, self.1);
            } else {
                f.stroke(&path, stroke());
            }
        }
        vec![f.into_geometry()]
    }
}

pub fn icon<'a, M: 'a>(i: Ic, c: Color, size: f32, w: f32) -> E<'a, M> {
    match i {
        Ic::G(g) => glyph(g, c, size, w),
        _ => canvas::Canvas::new(Draw(i, c, w)).width(Length::Fixed(size)).height(Length::Fixed(size)).into(),
    }
}

pub fn ibox<'a, M: 'a>(p: &Pal, i: Ic) -> E<'a, M> {
    let s = sz(40.0);
    container(icon(i, p.text, sz(18.0), 1.8)).center_x(Length::Fixed(s)).center_y(Length::Fixed(s)).style(ui::boxed(Pal::mix(p.panel, p.text, 0.06), p.r(8.0), None)).into()
}

// ------------------------------------------------------------------ rows
fn words<'a, M: 'a>(p: &Pal, title: String, sub: String, title_c: Color) -> Column<'a, M> {
    let mut w = Column::new().spacing(3.0).push(tm(title, sz(15.0), title_c));
    if !sub.is_empty() {
        w = w.push(t(sub, sz(13.0), p.muted));
    }
    w
}
fn boxed_row<'a, M: 'a>(p: &Pal, inner: impl Into<E<'a, M>>, dim: bool) -> container::Container<'a, M> {
    container(inner)
        .padding(Padding { top: sz(14.0), right: 20.0, bottom: sz(14.0), left: 20.0 })
        .width(Length::Fill)
        .style(ui::boxed(if dim { Pal::mix(p.panel, p.bg, 0.45) } else { p.panel }, p.r(10.0), None))
}

/// A setting's row (as widgets::srow) with one of these pages' icons.
pub fn irow<'a, M: 'a>(p: &Pal, i: Ic, title: impl Into<String>, sub: impl Into<String>, control: E<'a, M>) -> E<'a, M> {
    boxed_row(p, row![ibox(p, i), words(p, title.into(), sub.into(), p.text).width(Length::Fill), control].spacing(16.0).align_y(iced::Center), false).into()
}
/// A row greyed out: its control can't be used here (with why in its line).
pub fn irow_off<'a, M: 'a>(p: &Pal, i: Ic, title: impl Into<String>, sub: impl Into<String>, control: E<'a, M>) -> E<'a, M> {
    let pm = Pal { text: p.muted, ..*p };
    boxed_row(p, row![ibox(&pm, i), words(&pm, title.into(), sub.into(), p.muted).width(Length::Fill), control].spacing(16.0).align_y(iced::Center), true).into()
}
/// A row for a setting that can be used or not (`on`: usable).
pub fn irow_if<'a, M: 'a>(p: &Pal, on: bool, i: Ic, title: impl Into<String>, sub: impl Into<String>, control: E<'a, M>) -> E<'a, M> {
    if on { irow(p, i, title, sub, control) } else { irow_off(p, i, title, sub, control) }
}
/// A row that opens something: its value (if any) and › on the right.
pub fn ilink<'a, M: Clone + 'a>(p: &Pal, i: Ic, title: impl Into<String>, sub: impl Into<String>, value: impl Into<String>, msg: M) -> E<'a, M> {
    let value: String = value.into();
    let mut right = Row::new().spacing(12.0).align_y(iced::Center);
    if !value.is_empty() {
        right = right.push(t1(value, sz(13.0), p.muted));
    }
    right = right.push(glyph(G::Chevron, p.muted, 14.0, 2.0));
    mouse_area(irow(p, i, title, sub, right.into())).on_press(msg).interaction(mouse::Interaction::Pointer).into()
}
/// A row that does something when pressed (no ›), red when it deletes or removes.
pub fn iact<'a, M: Clone + 'a>(p: &Pal, i: Ic, title: impl Into<String>, sub: impl Into<String>, danger: bool, msg: Option<M>) -> E<'a, M> {
    let c = if msg.is_none() { p.muted } else if danger { p.danger } else { p.text };
    let pi = Pal { text: c, ..*p };
    let r = boxed_row(p, row![ibox(&pi, i), words(p, title.into(), sub.into(), c).width(Length::Fill)].spacing(16.0).align_y(iced::Center), msg.is_none());
    match msg {
        Some(m) => mouse_area(r).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => r.into(),
    }
}
/// A row only to read (its chip on the right).
pub fn iinfo<'a, M: 'a>(p: &Pal, i: Ic, title: impl Into<String>, sub: impl Into<String>, chip_: Option<(String, Kind)>) -> E<'a, M> {
    let right: E<'a, M> = match chip_ {
        Some((l, k)) if !l.is_empty() => chip(p, l, k),
        _ => Space::new().width(Length::Fixed(0.0)).into(),
    };
    irow(p, i, title, sub, right)
}
/// Rows under the one they belong to (an account's options), moved in a little.
pub fn indent<'a, M: 'a>(inner: impl Into<E<'a, M>>) -> E<'a, M> {
    container(inner).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 52.0 }).width(Length::Fill).into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Ok,
    Warn,
    Bad,
    Off,
}
pub fn chip<'a, M: 'a>(p: &Pal, label: impl Into<String>, k: Kind) -> E<'a, M> {
    let c = match k {
        Kind::Ok => p.good,
        Kind::Warn => p.warn,
        Kind::Bad => p.danger,
        Kind::Off => p.muted,
    };
    container(t1(label.into(), sz(12.0), c).font(ui::weight(iced::font::Weight::Semibold)))
        .padding(Padding { top: 3.0, right: 10.0, bottom: 3.0, left: 10.0 })
        .style(ui::boxed(Pal::a(c, 0.14), 11.0, Some((Pal::a(c, 0.45), 1.0))))
        .into()
}

/// A box across the page (KWin isn't here, no keyring…), with what to do about it.
pub fn banner<'a, M: Clone + 'a>(p: &Pal, i: Ic, title: impl Into<String>, sub: impl Into<String>, bad: bool, action: Option<(&'a str, M)>) -> E<'a, M> {
    let c = if bad { p.danger } else { p.accent };
    let mut r = row![icon(i, c, sz(20.0), 1.8), column![ts(title.into(), sz(15.0), p.text), t(sub.into(), sz(13.0), p.muted)].spacing(3.0).width(Length::Fill)].spacing(14.0).align_y(iced::Center);
    if let Some((l, m)) = action {
        r = r.push(ui::btn(p, l, ui::Kind::Normal, false, Some(m)));
    }
    container(r).padding(16.0).width(Length::Fill).style(ui::boxed(Pal::mix(p.panel, c, 0.08), p.r(10.0), Some((Pal::a(c, 0.5), 1.0)))).into()
}

/// The page's own line (what happened), in the flame colour, or red when it went wrong.
pub fn said<'a, M: 'a>(p: &Pal, s: &str, bad: bool) -> E<'a, M> {
    container(t(s.to_string(), sz(13.0), if bad { p.danger } else { p.flame })).padding(Padding { top: 4.0, right: 0.0, bottom: 4.0, left: 4.0 }).width(Length::Fill).into()
}
/// A small grey line (under a group).
pub fn small<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    container(t(s.into(), sz(12.0), p.muted)).padding(Padding { top: 2.0, right: 0.0, bottom: 4.0, left: 4.0 }).width(Length::Fill).into()
}

/// A drop-down's button: what's chosen, and ▾. Pressing it opens the list (`pick_dialog`).
pub fn dropdown<'a, M: Clone + 'a>(p: &Pal, label: impl Into<String>, msg: Option<M>) -> E<'a, M> {
    let on = msg.is_some();
    let c = container(row![t1(label.into(), sz(13.0), if on { p.text } else { p.muted }), glyph(G::Down, p.muted, 12.0, 2.0)].spacing(10.0).align_y(iced::Center))
        .padding(Padding { top: sz(7.0), right: 12.0, bottom: sz(7.0), left: 14.0 })
        .style(ui::boxed(Pal::mix(p.panel, p.bg, 0.4), p.r(8.0), Some((p.line, 1.0))));
    match msg {
        Some(m) => mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => c.into(),
    }
}

/// A drop-down's list, over the page: each choice (with its line), the chosen one marked. Esc or outside closes it.
pub fn pick_dialog<'a, M: Clone + 'a>(p: &Pal, title: &str, items: Vec<(String, String, bool, M)>, close: M) -> E<'a, M> {
    let n = items.len();
    let mut c = Column::new().spacing(4.0);
    for (label, sub, on, m) in items {
        let mut w = Column::new().spacing(2.0).push(if on { ts(label, sz(14.0), p.accent) } else { tm(label, sz(14.0), p.text) });
        if !sub.is_empty() {
            w = w.push(t(sub, sz(12.0), p.muted));
        }
        let mark: E<'a, M> = if on { glyph(G::Check, p.accent, 16.0, 2.4) } else { Space::new().width(Length::Fixed(16.0)).into() };
        c = c.push(
            mouse_area(container(row![w.width(Length::Fill), mark].spacing(10.0).align_y(iced::Center)).padding(Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 12.0 }).width(Length::Fill).style(ui::boxed(if on { Pal::mix(p.panel, p.accent, 0.1) } else { Color::TRANSPARENT }, p.r(8.0), None)))
                .on_press(m)
                .interaction(mouse::Interaction::Pointer),
        );
    }
    let body = column![
        row![ts(title.to_string(), sz(17.0), p.text), Space::new().width(Length::Fill), ui::btn(p, "Close", ui::Kind::Normal, false, Some(close))].align_y(iced::Center),
        scrollable(c).height(Length::Fixed((n as f32 * sz(58.0)).min(460.0))),
    ]
    .spacing(12.0);
    ui::dialog_box(p, 520.0, body)
}

/// A slider with its value written next to it; it changes the setting as it moves (the page saves it a moment later).
pub fn slide<'a, M: Clone + 'a>(p: &Pal, v: f32, range: std::ops::RangeInclusive<f32>, step: f32, value: impl Into<String>, on: impl Fn(f32) -> M + 'a) -> E<'a, M> {
    row![iced::widget::slider(range, v, on).step(step).width(Length::Fixed(sz(240.0))), container(t1(value.into(), sz(13.0), p.muted)).width(Length::Fixed(sz(70.0)))].spacing(12.0).align_y(iced::Center).into()
}

/// A text field for a password: what's typed shows as dots.
pub fn secret<'a, M: Clone + 'a>(p: &Pal, placeholder: &str, value: &str, on: impl Fn(String) -> M + 'a) -> text_input::TextInput<'a, M> {
    field(p, placeholder, value, on).secure(true)
}

/// A small picture made of colour (a wallpaper or a look, drawn: these pages draw no picture files).
pub fn swatch<'a, M: 'a>(bg: Color, accent: Color, text: Color, w: f32, h: f32, lit: Option<Color>) -> E<'a, M> {
    container(column![
        Space::new().height(Length::Fill),
        container(row![container(Space::new()).width(Length::Fixed(8.0)).height(Length::Fixed(8.0)).style(ui::boxed(accent, 4.0, None)), container(Space::new()).width(Length::Fixed(w * 0.35)).height(Length::Fixed(4.0)).style(ui::boxed(Pal::a(text, 0.5), 2.0, None))].spacing(6.0).align_y(iced::Center))
            .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
            .width(Length::Fill)
            .style(ui::boxed(Pal::a(Color::BLACK, 0.35), 0.0, None)),
    ])
    .width(Length::Fixed(w))
    .height(Length::Fixed(h))
    .style(ui::boxed(bg, 8.0, Some(match lit { Some(c) => (c, 2.5), None => (Pal::a(text, 0.12), 1.0) })))
    .into()
}

/// A colour from a hash of a name (a wallpaper's tile with no picture shown): steady, in the look's range.
pub fn name_colour(p: &Pal, s: &str) -> Color {
    let mut h: u32 = 2166136261;
    for b in s.bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    let hue = (h % 360) as f32;
    hsv(hue, 0.45, if p.dark { 0.42 } else { 0.78 })
}
pub fn hsv(h: f32, s: f32, v: f32) -> Color {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h as i32) / 60 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::from_rgb(r + m, g + m, b + m)
}
pub fn hex_of(c: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", (c.r * 255.0).round() as u8, (c.g * 255.0).round() as u8, (c.b * 255.0).round() as u8)
}
pub fn hue_of(hex: &str) -> f32 {
    let Some(c) = hearth_ui::theme::hex(hex) else { return 0.58 };
    let (r, g, b) = (c.r, c.g, c.b);
    let mx = r.max(g).max(b);
    let d = mx - r.min(g).min(b);
    if d == 0.0 {
        return 0.0;
    }
    let h = if mx == r { ((g - b) / d) % 6.0 } else if mx == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
    ((h / 6.0) + 1.0) % 1.0
}

/// Choices side by side (widgets::seg) from (label, value) pairs.
pub fn choice<'a, M: Clone + 'a, V: PartialEq + Clone>(p: &Pal, opts: &[(&str, V)], cur: &V, on: impl Fn(V) -> M) -> E<'a, M> {
    seg(p, opts.iter().map(|(l, v)| (l.to_string(), v == cur, Some(on(v.clone())))).collect())
}

/// The file picker of the desktop (KDE's kdialog, else GNOME's zenity), as Mantel's "Choose a picture…". Exit 3: none.
pub fn file_picker(title: &str, folder: bool, filter: &str) -> Vec<String> {
    let t = data_q(title);
    let script = if folder {
        format!("if command -v kdialog >/dev/null; then kdialog --title {t} --getexistingdirectory \"$HOME/Pictures\"; elif command -v zenity >/dev/null; then zenity --file-selection --directory --title={t} --filename=\"$HOME/Pictures/\"; else exit 3; fi")
    } else {
        format!("if command -v kdialog >/dev/null; then kdialog --title {t} --getopenfilename \"$HOME/Pictures\" {}; elif command -v zenity >/dev/null; then zenity --file-selection --title={t} --filename=\"$HOME/Pictures/\" --file-filter='Pictures | *.jpg *.jpeg *.png *.webp'; else exit 3; fi", data_q(filter))
    };
    sh_t(&script, 900)
}
fn data_q(s: &str) -> String {
    crate::data::q(s)
}

/// A setting's value of desktop.json as JSON (an object or array kept as it is).
pub fn obj(sh: &crate::shared::Shared, k: &str) -> serde_json::Map<String, Value> {
    sh.desktop.get(k).and_then(Value::as_object).cloned().unwrap_or_default()
}
pub fn s_of(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}
pub fn b_of(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}
pub fn n_of(v: &Value, k: &str, d: f64) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(d)
}
pub fn arr<'v>(v: &'v Value, k: &str) -> Vec<&'v Value> {
    v.get(k).and_then(Value::as_array).map(|a| a.iter().collect()).unwrap_or_default()
}
pub fn strs(v: &Value, k: &str) -> Vec<String> {
    arr(v, k).into_iter().filter_map(|x| x.as_str().map(String::from)).collect()
}

/// A file's name without its folder.
pub fn base(f: &str) -> String {
    f.rsplit('/').next().unwrap_or(f).to_string()
}

/// A page's column that can start further down (`hearth-settings-app --page PAGE:from:N`: CI's pictures of the
/// lower part of a long page). Pushed to like a Column.
pub struct Col<'a, M> {
    v: Vec<E<'a, M>>,
    skip: usize,
}
impl<'a, M: 'a> Col<'a, M> {
    pub fn new(skip: usize) -> Self {
        Col { v: Vec::new(), skip }
    }
    pub fn push(mut self, e: impl Into<E<'a, M>>) -> Self {
        self.v.push(e.into());
        self
    }
}
impl<'a, M: 'a> From<Col<'a, M>> for E<'a, M> {
    fn from(c: Col<'a, M>) -> E<'a, M> {
        let skip = c.skip.min(c.v.len().saturating_sub(1));
        Column::with_children(c.v.into_iter().skip(skip)).spacing(10.0).into()
    }
}
/// `from:N` in the part a page was opened at.
pub fn skip_of(part: &str) -> usize {
    part.split(',').find_map(|t| t.strip_prefix("from:").and_then(|n| n.parse().ok())).unwrap_or(0)
}
/// The part's other words (`wallpaper,from:8` → wallpaper).
pub fn part_word(part: &str) -> String {
    part.split(',').find(|t| !t.starts_with("from:")).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn answers_like_setjob() {
        let o = |out: &str, ok: bool| run::Output { ok, code: if ok { 0 } else { 1 }, out: out.into(), err: String::new() };
        assert_eq!(answer(&o("log: x\n{\"a\": 1}\n", true)).unwrap()["a"], 1);
        assert_eq!(answer(&o("error: no way\n", false)).unwrap_err(), "no way");
        let d = run::Output { ok: true, code: 0, out: "method return time=1\n   string \"{\"panels\": []}\"\n".into(), err: String::new() };
        assert!(plasma_json(&d).unwrap()["panels"].is_array());
        assert_eq!(js_str("a\"b"), "\"a\\\"b\"");
        assert!((hue_of("#ff0000") - 0.0).abs() < 0.01);
        assert_eq!(hex_of(hsv(0.0, 1.0, 1.0)), "#ff0000");
    }
}

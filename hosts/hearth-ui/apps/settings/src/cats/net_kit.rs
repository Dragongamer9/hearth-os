//! What every Network & internet and Bluetooth & devices page shares (Mantel's SetJob, SetRow, SetChoice,
//! SetDropdown, SetExpander and SetBanner, in Settings' look): running one of Mantel's helpers (hearth-network,
//! hearth-bt, hearth-system, hearth-phone…, the same files Mantel uses, so Debian's Mantel and these pages share every
//! setting) with text for it on its stdin, never on its command line; its one JSON line or its "error: …" line back;
//! and the rows the pages are drawn with.
//!
//! On KDE Plasma (the Fedora edition) KWin is the compositor, so the helpers are told so (HEARTH_WM=kwin, as
//! hearth-screenshot does): hearth-system then puts the keyboard and the mouse in KWin's kxkbrc / kcminputrc
//! (hearth-kwin-settings) instead of labwc's files.

use crate::widgets::*;
use hearth_ui::run::{self, Output};
use hearth_ui::ui::{self, btn, glyph, t, t1, tm, Kind, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, Column, Row, Space};
use iced::{mouse, Color, Length, Padding};
use serde_json::Value;
use std::future::Future;
use std::io::Write;
use std::process::{Command, Stdio};

// ------------------------------------------------------------------ running Mantel's helpers
/// A helper's answer: its JSON (an empty object when it printed none) or why it failed, in its own words.
#[derive(Debug, Clone)]
pub enum Res {
    Ok(Value),
    Err(String),
}

impl Res {
    pub fn val(&self) -> Option<&Value> {
        match self {
            Res::Ok(v) => Some(v),
            Res::Err(_) => None,
        }
    }
    pub fn err(&self) -> Option<&str> {
        match self {
            Res::Ok(_) => None,
            Res::Err(e) => Some(e.as_str()),
        }
    }
}

pub fn a(v: &[&str]) -> Vec<String> {
    run::args(v)
}

/// On KDE Plasma, KWin is the window manager: the helpers are told so, as Mantel on KWin tells them.
fn plasma_env(c: &mut Command) {
    let wm = std::env::var("HEARTH_WM").unwrap_or_default();
    let desk = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    if wm.is_empty() && desk.to_uppercase().contains("KDE") {
        c.env("HEARTH_WM", "kwin");
    }
}

/// Runs `cmd` with `input` on its stdin (and in HEARTH_INPUT, which only you can read, as Mantel's SetJob does), for
/// at most `secs` seconds (coreutils' timeout: nothing here can hang the page).
pub fn exec_blocking(cmd: &[String], input: &str, secs: u32) -> Output {
    if cmd.is_empty() {
        return Output { ok: false, code: -1, out: String::new(), err: "nothing to run".into() };
    }
    let mut c = Command::new("timeout");
    c.arg("-k").arg("5").arg(secs.to_string()).args(cmd);
    c.env("HEARTH_INPUT", input);
    plasma_env(&mut c);
    c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    match c.spawn() {
        Ok(mut ch) => {
            if let Some(mut si) = ch.stdin.take() {
                let _ = si.write_all(input.as_bytes());
            }
            match ch.wait_with_output() {
                Ok(o) => Output {
                    ok: o.status.success(),
                    code: o.status.code().unwrap_or(-1),
                    out: String::from_utf8_lossy(&o.stdout).to_string(),
                    err: String::from_utf8_lossy(&o.stderr).to_string(),
                },
                Err(e) => Output { ok: false, code: -1, out: String::new(), err: format!("{}: {e}", cmd[0]) },
            }
        }
        Err(e) => Output { ok: false, code: -1, out: String::new(), err: format!("{}: {e}", cmd[0]) },
    }
}

/// `exec_blocking` on its own thread (one short-lived thread per run; nothing polls).
pub fn exec(cmd: Vec<String>, input: String, secs: u32) -> impl Future<Output = Output> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-settings-job".into()).spawn(move || {
        let _ = tx.send(exec_blocking(&cmd, &input, secs));
    });
    async move { rx.await.unwrap_or(Output { ok: false, code: -1, out: String::new(), err: "it stopped".into() }) }
}

/// What a helper answered, read as Mantel's SetJob reads it: "error: …" is the reason, a line starting with "{" is
/// the answer, "progress: …" and "log: …" lines are skipped.
pub fn parse(o: &Output) -> Res {
    let mut result: Option<Value> = None;
    let mut err = String::new();
    let mut tail = String::new();
    for l in o.out.lines().chain(o.err.lines()) {
        let l = l.trim_end();
        if l.starts_with("progress:") || l.starts_with("log: ") {
            continue;
        }
        if let Some(e) = l.strip_prefix("error: ") {
            err = e.to_string();
            continue;
        }
        if l.starts_with('{') {
            if let Ok(v) = serde_json::from_str::<Value>(l) {
                result = Some(v);
                continue;
            }
        }
        if !l.trim().is_empty() {
            tail = l.trim().to_string();
        }
    }
    if !err.is_empty() {
        return Res::Err(err);
    }
    if o.code == 124 && result.is_none() {
        return Res::Err("It didn't answer in time".into());
    }
    if !o.ok && result.is_none() {
        return Res::Err(if tail.is_empty() { "That didn't work".into() } else { tail });
    }
    Res::Ok(result.unwrap_or_else(|| Value::Object(Default::default())))
}

/// One of Mantel's helpers (a file in its shell folder), as you: `python3 helper args`, `input` on its stdin.
pub fn job(helper: &str, args: Vec<String>, input: String) -> impl Future<Output = Res> + Send + 'static {
    job_for(helper, args, input, 120)
}

pub fn job_for(helper: &str, args: Vec<String>, input: String, secs: u32) -> impl Future<Output = Res> + Send + 'static {
    let mut cmd = vec!["python3".to_string(), run::shell_helper(helper)];
    cmd.extend(args);
    let f = exec(cmd, input, secs);
    async move { parse(&f.await) }
}

/// Hearth's admin helper through pkexec (Hearth's polkit rule lets the person at the device do it without a prompt).
pub fn admin(args: Vec<String>) -> impl Future<Output = Res> + Send + 'static {
    let mut cmd = vec!["pkexec".to_string(), crate::data::ADMIN.to_string()];
    cmd.extend(args);
    let f = exec(cmd, String::new(), 180);
    async move {
        let o = f.await;
        let r = parse(&o);
        let refused = o.code == 126 || o.code == 127;
        match r {
            Res::Err(_) if refused && !o.out.contains("error: ") && !o.err.contains("error: ") => Res::Err("Not allowed (Hearth's polkit rule is missing, or the request was cancelled).".into()),
            other => other,
        }
    }
}

// ------------------------------------------------------------------ reading a helper's JSON
static NULL: Value = Value::Null;

pub fn null() -> &'static Value {
    &NULL
}

/// `v.a.b` ("a.b"), or null.
pub fn at<'a>(v: &'a Value, path: &str) -> &'a Value {
    let mut c = v;
    for k in path.split('.') {
        if k.is_empty() {
            continue;
        }
        c = match c {
            Value::Object(m) => m.get(k).unwrap_or(&NULL),
            Value::Array(l) => k.parse::<usize>().ok().and_then(|i| l.get(i)).unwrap_or(&NULL),
            _ => &NULL,
        };
    }
    c
}
pub fn st(v: &Value, k: &str) -> String {
    match at(v, k) {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}
pub fn bo(v: &Value, k: &str) -> bool {
    at(v, k).as_bool().unwrap_or(false)
}
/// True unless it's there and false (Mantel's `x !== false`).
pub fn not_false(v: &Value, k: &str) -> bool {
    at(v, k).as_bool() != Some(false)
}
pub fn nu(v: &Value, k: &str) -> Option<f64> {
    at(v, k).as_f64()
}
pub fn ar<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    at(v, k).as_array().map(|l| l.as_slice()).unwrap_or(&[])
}
pub fn strs(v: &Value, k: &str) -> Vec<String> {
    ar(v, k).iter().filter_map(|x| x.as_str().map(String::from)).collect()
}
pub fn has(v: &Value, k: &str) -> bool {
    !at(v, k).is_null()
}
/// The non-empty ones, joined with " · ".
pub fn dots(parts: &[String]) -> String {
    parts.iter().filter(|s| !s.trim().is_empty()).cloned().collect::<Vec<_>>().join(" · ")
}

// ------------------------------------------------------------------ the line under a page's controls
#[derive(Debug, Clone, Default)]
pub struct Say {
    pub text: String,
    pub bad: bool,
}

impl Say {
    pub fn ok(&mut self, t: impl Into<String>) {
        self.text = t.into();
        self.bad = false;
    }
    pub fn bad(&mut self, t: impl Into<String>) {
        self.text = t.into();
        self.bad = true;
    }
    pub fn clear(&mut self) {
        self.text.clear();
        self.bad = false;
    }
    /// A helper's answer: `ok` when it worked, its reason when it didn't.
    pub fn res(&mut self, r: &Res, ok: &str) {
        match r {
            Res::Ok(_) => {
                if !ok.is_empty() {
                    self.ok(ok)
                }
            }
            Res::Err(e) => self.bad(e.clone()),
        }
    }
}

/// Mantel's message line: the flame colour, or red when something went wrong. Nothing when empty.
pub fn said<'a, M: 'a>(p: &Pal, s: &Say) -> E<'a, M> {
    if s.text.is_empty() {
        return Space::new().height(Length::Fixed(0.0)).into();
    }
    container(t(s.text.clone(), sz(13.0), if s.bad { p.danger } else { p.flame })).padding(Padding { top: 2.0, right: 0.0, bottom: 2.0, left: 4.0 }).into()
}

/// A muted line of words in the page (Mantel's plain Text).
pub fn words<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    container(t(s.into(), sz(13.0), p.muted)).padding(Padding { top: 2.0, right: 0.0, bottom: 2.0, left: 4.0 }).into()
}

// ------------------------------------------------------------------ rows
/// A small chip: ok (green), warn, bad (red), off (grey), anything else the accent.
pub fn chipk<'a, M: 'a>(p: &Pal, label: impl Into<String>, kind: &str) -> E<'a, M> {
    let (bg, fg) = match kind {
        "ok" => (Pal::a(p.good, 0.18), p.good),
        "warn" => (Pal::a(p.warn, 0.2), p.warn),
        "bad" => (Pal::a(p.danger, 0.18), p.danger),
        "off" => (Pal::mix(p.panel2, p.text, 0.1), p.muted),
        _ => (Pal::a(p.accent, 0.2), p.accent),
    };
    ui::chip(label.into(), bg, fg)
}

/// Mantel's SetRow: its icon, name (red for one that removes something), the line under it, what's on the right, and
/// pressing it sends `msg`. A row without a message is only shown.
pub fn xrow<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, right: Option<E<'a, M>>, danger: bool, msg: Option<M>) -> E<'a, M> {
    let sub: String = sub.into();
    let mut w = Column::new().spacing(3.0).push(tm(title.into(), sz(15.0), if danger { p.danger } else { p.text }));
    if !sub.is_empty() {
        w = w.push(t(sub, sz(13.0), p.muted));
    }
    let mut r = Row::new().spacing(16.0).align_y(iced::Center).push(icon_box(p, g)).push(w.width(Length::Fill));
    if let Some(x) = right {
        r = r.push(x);
    }
    let c = container(r).padding(Padding { top: sz(14.0), right: 20.0, bottom: sz(14.0), left: 20.0 }).width(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), None));
    match msg {
        Some(m) => mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => c.into(),
    }
}

/// A row you press (no chevron).
pub fn press<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, msg: Option<M>) -> E<'a, M> {
    xrow(p, g, title, sub, None, false, msg)
}
/// A row that removes something (red).
pub fn danger_row<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, msg: Option<M>) -> E<'a, M> {
    xrow(p, g, title, sub, None, true, msg)
}
/// A row that opens another page or part (›).
pub fn go_row<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, msg: M) -> E<'a, M> {
    xrow(p, g, title, sub, Some(glyph(G::Chevron, p.muted, 14.0, 2.0)), false, Some(msg))
}
/// A row with a value on the right (Default, 85 %, Shown below).
pub fn val_row<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, value: impl Into<String>, msg: Option<M>) -> E<'a, M> {
    let v: String = value.into();
    let right: Option<E<'a, M>> = if v.is_empty() { None } else { Some(t1(v, sz(13.0), p.muted).into()) };
    xrow(p, g, title, sub, right, false, msg)
}
/// A row with a chip on the right (Connected, Blocked, Waiting).
pub fn chip_row<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, chip: &str, kind: &str, msg: Option<M>) -> E<'a, M> {
    let right: Option<E<'a, M>> = if chip.is_empty() { None } else { Some(chipk(p, chip.to_string(), kind)) };
    xrow(p, g, title, sub, right, false, msg)
}
/// A row only shown (Mantel's interactive: false).
pub fn info_row<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>) -> E<'a, M> {
    xrow(p, g, title, sub, None, false, None)
}
/// A switch row; `enabled: false` shows it greyed (Needs Mantel on KWin, Needs KWin 6.7).
pub fn switch_row<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, on: bool, enabled: bool, msg: M) -> E<'a, M> {
    if enabled {
        srow(p, g, title, sub, switch(p, on, Some(msg)))
    } else {
        let q = dim(p);
        srow(&q, g, title, sub, switch(&q, on, None))
    }
}
/// Choices side by side (Mantel's SetChoice).
pub fn choice<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, items: Vec<(String, bool, Option<M>)>) -> E<'a, M> {
    srow(p, g, title, sub, seg(p, items))
}
/// The same, greyed and not changeable.
pub fn choice_dim<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, items: Vec<(String, bool)>) -> E<'a, M> {
    let q = dim(p);
    srow(&q, g, title, sub, seg(&q, items.into_iter().map(|(l, on)| (l, on, None)).collect()))
}
/// A look with every colour faded towards the panel (a greyed row).
pub fn dim(p: &Pal) -> Pal {
    let mut q = *p;
    q.text = Pal::mix(p.text, p.panel, 0.55);
    q.muted = Pal::mix(p.muted, p.panel, 0.5);
    q.accent = Pal::mix(p.accent, p.panel, 0.55);
    q
}
/// Mantel's SetDropdown: the row with what's picked on a button; pressed, the choices open under it.
pub fn dropdown<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, current: impl Into<String>, open: bool, toggle: M, opts: Vec<(String, bool, M)>) -> E<'a, M> {
    let pp = *p;
    let cur = container(row![t1(current.into(), sz(13.0), p.text), glyph(G::Down, if open { p.accent } else { p.muted }, 12.0, 2.0)].spacing(10.0).align_y(iced::Center))
        .padding(Padding { top: sz(6.0), right: 12.0, bottom: sz(6.0), left: 14.0 })
        .style(ui::boxed(Pal::mix(p.panel, p.bg, 0.4), p.r(8.0), Some((p.line, 1.0))));
    let mut c = Column::new().spacing(6.0).push(srow(p, g, title, sub, mouse_area(cur).on_press(toggle).interaction(mouse::Interaction::Pointer).into()));
    if open {
        let mut l = Column::new().spacing(4.0);
        for (label, on, m) in opts {
            let fg = if on { pp.accent } else { pp.text };
            l = l.push(
                mouse_area(container(row![t(label, sz(14.0), fg).width(Length::Fill), if on { glyph(G::Check, pp.accent, 14.0, 2.2) } else { Space::new().width(Length::Fixed(14.0)).into() }].align_y(iced::Center))
                    .padding(Padding { top: 8.0, right: 14.0, bottom: 8.0, left: 14.0 })
                    .width(Length::Fill)
                    .style(ui::boxed(if on { Pal::mix(pp.panel, pp.accent, 0.08) } else { pp.panel }, pp.r(8.0), None)))
                .on_press(m)
                .interaction(mouse::Interaction::Pointer),
            );
        }
        c = c.push(indent(l));
    }
    c.into()
}
/// Mantel's SetExpander: a row that folds its own settings open under it.
pub fn expander<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, value: &str, chip: Option<(&str, &str)>, open: bool, toggle: M, body: Option<E<'a, M>>) -> E<'a, M> {
    let mut right = Row::new().spacing(10.0).align_y(iced::Center);
    if let Some((c, k)) = chip {
        if !c.is_empty() {
            right = right.push(chipk(p, c.to_string(), k));
        }
    }
    if !value.is_empty() {
        right = right.push(t1(value.to_string(), sz(13.0), p.muted));
    }
    right = right.push(glyph(if open { G::Down } else { G::Chevron }, p.muted, 14.0, 2.0));
    let mut c = Column::new().spacing(6.0).push(xrow(p, g, title, sub, Some(right.into()), false, Some(toggle)));
    if open {
        if let Some(b) = body {
            c = c.push(indent(b));
        }
    }
    c.into()
}
/// Under a row, set in (Mantel's Layout.leftMargin: 52).
pub fn indent<'a, M: 'a>(inner: impl Into<E<'a, M>>) -> E<'a, M> {
    container(inner).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 52.0 }).width(Length::Fill).into()
}
/// Mantel's SetBanner: a tinted card with an icon, a title, the line under it and its button.
pub fn banner<'a, M: Clone + 'a>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, bad: bool, action: Option<(String, M)>) -> E<'a, M> {
    let tint = if bad { p.danger } else { p.accent };
    let mut r = Row::new().spacing(16.0).align_y(iced::Center).push(glyph(g, tint, sz(22.0), 1.8)).push(column![tm(title.into(), sz(15.0), p.text), t(sub.into(), sz(13.0), p.muted)].spacing(3.0).width(Length::Fill));
    if let Some((l, m)) = action {
        r = r.push(btn(p, l, Kind::Primary, false, Some(m)));
    }
    container(r).padding(Padding { top: sz(14.0), right: 20.0, bottom: sz(14.0), left: 20.0 }).width(Length::Fill).style(ui::boxed(Pal::mix(p.panel, tint, 0.12), p.r(10.0), Some((Pal::a(tint, 0.45), 1.0)))).into()
}
/// A text field, secret or not, Enter sending `submit`.
pub fn input<'a, M: Clone + 'a>(p: &Pal, placeholder: &str, value: &str, secret: bool, on: impl Fn(String) -> M + 'a, submit: Option<M>) -> E<'a, M> {
    let mut f = field(p, placeholder, value, on);
    if secret {
        f = f.secure(true);
    }
    if let Some(m) = submit {
        f = f.on_submit(m);
    }
    f.into()
}
/// Buttons in a line (Mantel's Flow of SetButtons).
pub fn buttons<'a, M: Clone + 'a>(p: &Pal, items: Vec<(String, Kind, Option<M>)>) -> E<'a, M> {
    let mut r = Row::new().spacing(8.0).align_y(iced::Center);
    for (l, k, m) in items {
        r = r.push(btn(p, l, k, false, m));
    }
    r.into()
}
/// How full something is, in its own colour.
pub fn level_bar<'a, M: 'a>(p: &Pal, f: f32, c: Color) -> E<'a, M> {
    ui::bar(vec![(f.clamp(0.0, 1.0), c)], Pal::mix(p.panel2, p.text, 0.08), 10.0, 5.0)
}

// ------------------------------------------------------------------ reading again while a page is open
static CURRENT: std::sync::Mutex<&'static str> = std::sync::Mutex::new("");

/// The page Settings shows now. The frame asks every category for a dialog each time it draws (cats::*::dialog,
/// with the App), so net.rs and bt.rs note the page there: a timer or Back of one of these pages acts only while
/// it's the one showing.
pub fn set_current(page: &'static str) {
    if let Ok(mut c) = CURRENT.lock() {
        *c = page;
    }
}
pub fn current() -> &'static str {
    CURRENT.lock().map(|c| *c).unwrap_or("")
}
pub fn showing(pages: &[&str]) -> bool {
    let c = current();
    pages.iter().any(|p| *p == c)
}

/// A page that reads again every few seconds while it's open (Mantel's Timer { running: page.visible }): the timer
/// stops by itself once another page shows, so nothing keeps running for a page you've left.
#[derive(Default)]
pub struct Live {
    pub page: &'static str,
    on: bool,
    drawn: std::cell::Cell<Option<std::time::Instant>>,
}

impl Live {
    pub fn start(&mut self, page: &'static str) {
        self.page = page;
        self.on = true;
        self.drawn.set(Some(std::time::Instant::now()));
    }
    pub fn stop(&mut self) {
        self.on = false;
    }
    /// The page was drawn (called from its view).
    pub fn seen(&self) {
        self.drawn.set(Some(std::time::Instant::now()));
    }
    /// On a tick: is the page still the one showing? (Otherwise the timer stops.)
    pub fn still(&mut self, _secs: u64) -> bool {
        let here = current() == self.page || (current().is_empty() && self.drawn.get().is_some());
        if !here {
            self.on = false;
        }
        self.on && here
    }
    pub fn running(&self, page: &str) -> bool {
        self.on && self.page == page
    }
}

/// `~/…` for a path in the home folder.
pub fn tilde(path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() && path.starts_with(&home) {
        format!("~{}", &path[home.len()..])
    } else {
        path.to_string()
    }
}

/// A folder picked with KDE's own dialog (kdialog), or GNOME's (zenity); "" when cancelled; Err when neither is here.
pub fn pick_cmd(title: &str, folder: bool, start: &str, filter: &str) -> Vec<String> {
    let k = if folder { format!("kdialog --title {} --getexistingdirectory {}", crate::data::q(title), crate::data::q(start)) } else { format!("kdialog --title {} --getopenfilename {} {}", crate::data::q(title), crate::data::q(start), crate::data::q(filter)) };
    let z = if folder { format!("zenity --file-selection --directory --title={} --filename={}", crate::data::q(title), crate::data::q(&format!("{start}/"))) } else { format!("zenity --file-selection --title={} --filename={}", crate::data::q(title), crate::data::q(&format!("{start}/"))) };
    crate::data::sh(&format!("if command -v kdialog >/dev/null; then {k}; elif command -v zenity >/dev/null; then {z}; else exit 3; fi"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn out(ok: bool, code: i32, o: &str, e: &str) -> Output {
        Output { ok, code, out: o.into(), err: e.into() }
    }
    #[test]
    fn reads_helpers_like_setjob() {
        assert!(matches!(parse(&out(true, 0, "log: x\n{\"a\": 1}\n", "")), Res::Ok(v) if v["a"] == 1));
        assert!(matches!(parse(&out(false, 1, "", "error: No such VPN\n")), Res::Err(e) if e == "No such VPN"));
        assert!(matches!(parse(&out(false, 2, "Traceback\nValueError: x\n", "")), Res::Err(e) if e == "ValueError: x"));
        assert!(matches!(parse(&out(true, 0, "", "")), Res::Ok(v) if v.is_object()));
        assert!(matches!(parse(&out(false, 124, "", "")), Res::Err(_)));
    }
    #[test]
    fn reads_json_paths() {
        let v: Value = serde_json::json!({"a": {"b": [1, {"c": "x"}]}, "t": true});
        assert_eq!(st(&v, "a.b.1.c"), "x");
        assert!(bo(&v, "t"));
        assert!(not_false(&v, "missing"));
        assert!(at(&v, "a.z.q").is_null());
        assert_eq!(dots(&["a".into(), "".into(), "b".into()]), "a · b");
    }
}

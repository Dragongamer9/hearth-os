//! What the Apps and Gaming pages share: running Hearth's helpers the way Mantel's SetJob.qml does (python3 <helper>
//! ARGS, text for it on stdin and in HEARTH_INPUT, "progress: N what" lines, "error: ..." and the last JSON line),
//! always with a time limit; Hearth's core (`nimbus --settings-call`, HearthCall.qml); reading the answers; and the
//! rows Mantel's pages are made of (SetRow with a value or a chip, SetSwitch, SetChoice, SetExpander, SetCardGroup).
//! Opening a page only reads: every change is a helper the person asked for.

use crate::widgets::*;
use hearth_ui::ui::{self, chip, glyph, t, tm, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Task};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

/// What a running helper sends: a progress line (0..1, or -1 when it gave no number), then its end.
#[derive(Debug, Clone)]
pub enum Ev {
    Line(f32, String),
    Done(Res),
}

/// How a helper ended: its JSON answer (Null if none) or why it failed.
#[derive(Debug, Clone, Default)]
pub struct Res {
    pub ok: bool,
    pub v: Value,
    pub err: String,
    /// Everything it printed (for programs that don't answer in JSON).
    pub out: String,
}

/// Mantel's helpers' folder (hosts/linux/desktop/shell, /usr/share/nimbus/desktop/shell on the image).
pub fn shell(name: &str) -> String {
    hearth_ui::run::shell_helper(name)
}

/// Hearth's system helpers (/usr/libexec/nimbus; $HEARTH_LIBEXEC_DIR for CI's pictures).
pub fn libexec(name: &str) -> String {
    let d = std::env::var("HEARTH_LIBEXEC_DIR").unwrap_or_else(|_| "/usr/libexec/nimbus".into());
    format!("{d}/{name}")
}

pub fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// `python3 HELPER ARGS` (a file in Mantel's folder, or an absolute path), stopped after `secs`.
pub fn py(helper: &str, args: &[&str], secs: u32) -> Vec<String> {
    pyv(helper, strs(args), secs)
}
pub fn pyv(helper: &str, args: Vec<String>, secs: u32) -> Vec<String> {
    let path = if helper.starts_with('/') { helper.to_string() } else { shell(helper) };
    let mut v = vec!["timeout".into(), secs.to_string(), "python3".into(), path];
    v.extend(args);
    v
}
/// A program as it is, stopped after `secs`.
pub fn prog(args: Vec<String>, secs: u32) -> Vec<String> {
    let mut v = vec!["timeout".into(), secs.to_string()];
    v.extend(args);
    v
}
/// Through pkexec and hearth-admin (Hearth's polkit rule lets the person at the device do it without a prompt).
pub fn admin(args: &[&str], secs: u32) -> Vec<String> {
    let mut v = vec!["timeout".into(), secs.to_string(), "pkexec".into(), libexec("hearth-admin")];
    v.extend(strs(args));
    v
}

/// CI's pictures only: with $HEARTH_SETTINGS_FIXTURES set, a helper's answer can come from a file there named after
/// it and its first two words ("hearth-hhd-page.json"), so a page can be pictured as it looks on a real device.
fn fixture(cmd: &[String]) -> Option<Value> {
    let dir = std::env::var_os("HEARTH_SETTINGS_FIXTURES")?;
    let mut it = cmd.iter().skip_while(|a| a.as_str() == "timeout" || a.as_str() == "python3" || a.parse::<u32>().is_ok());
    let base = it.next()?.rsplit('/').next()?.to_string();
    let rest: Vec<&str> = it.map(String::as_str).filter(|a| !a.starts_with('{')).take(2).collect();
    let mut name = base;
    for r in rest {
        name.push('-');
        name.push_str(r);
    }
    let name: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || "-._".contains(c) { c } else { '_' }).collect();
    let t = std::fs::read_to_string(std::path::Path::new(&dir).join(format!("{name}.json"))).ok()?;
    serde_json::from_str(&t).ok()
}

fn run_lines(cmd: Vec<String>, input: Option<String>, tx: futures::channel::mpsc::UnboundedSender<Ev>) {
    if let Some(v) = fixture(&cmd) {
        let _ = tx.unbounded_send(Ev::Done(Res { ok: true, v, err: String::new(), out: String::new() }));
        return;
    }
    let name = cmd.iter().rev().find(|a| a.contains('/')).map(|a| a.rsplit('/').next().unwrap_or("").to_string()).unwrap_or_else(|| cmd.first().cloned().unwrap_or_default());
    let done = |r: Res| {
        let _ = tx.unbounded_send(Ev::Done(r));
    };
    if cmd.is_empty() {
        return done(Res { ok: false, v: Value::Null, err: "Nothing to run".into(), out: String::new() });
    }
    let mut c = Command::new(&cmd[0]);
    c.args(&cmd[1..]).stdout(Stdio::piped()).stderr(Stdio::piped());
    c.env("HEARTH_INPUT", input.clone().unwrap_or_default());
    c.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
    let mut child = match c.spawn() {
        Ok(ch) => ch,
        Err(e) => return done(Res { ok: false, v: Value::Null, err: format!("Couldn't start {name} ({e})"), out: String::new() }),
    };
    if let (Some(i), Some(mut sin)) = (input, child.stdin.take()) {
        let _ = sin.write_all(i.as_bytes());
    }
    let errs = child.stderr.take().map(|mut e| {
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = e.read_to_string(&mut s);
            s
        })
    });
    let (mut json, mut err, mut tail, mut all) = (Value::Null, String::new(), String::new(), String::new());
    if let Some(out) = child.stdout.take() {
        for l in BufReader::new(out).lines().map_while(Result::ok) {
            if all.len() < 8 << 20 {
                all.push_str(&l);
                all.push('\n');
            }
            if let Some(rest) = l.strip_prefix("progress: ") {
                let (n, what) = rest.split_once(' ').unwrap_or((rest, ""));
                let f = n.trim().parse::<f32>().map(|x| (x / 100.0).clamp(0.0, 1.0)).unwrap_or(-1.0);
                let _ = tx.unbounded_send(Ev::Line(f, what.to_string()));
            } else if let Some(e) = l.strip_prefix("error: ") {
                err = e.to_string();
            } else if l.starts_with('{') || l.starts_with('[') {
                match serde_json::from_str::<Value>(&l) {
                    Ok(v) => json = v,
                    Err(_) => tail = l.trim().to_string(),
                }
            } else if !l.trim().is_empty() && !l.starts_with("log: ") {
                tail = l.trim().to_string();
            }
        }
    }
    let code = child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
    let stderr = errs.and_then(|h| h.join().ok()).unwrap_or_default();
    if json.is_null() {
        if let Ok(v) = serde_json::from_str::<Value>(all.trim()) {
            json = v;
        }
    }
    let ok = code == 0 && err.is_empty();
    if !ok && err.is_empty() {
        let last_err = stderr.trim().lines().last().unwrap_or("").trim_start_matches("error: ").to_string();
        err = if code == 124 {
            "That took too long and was stopped".into()
        } else if code == 127 && tail.is_empty() {
            format!("{name} isn't on this system")
        } else if !tail.is_empty() {
            tail
        } else if !last_err.is_empty() {
            last_err
        } else {
            "That didn't work".into()
        };
    }
    done(Res { ok, v: json, err, out: all })
}

/// Runs `cmd` (text for it on stdin, if any) off the window's thread; every progress line and its end come as `f(..)`.
pub fn job<M: Send + 'static>(cmd: Vec<String>, input: Option<String>, f: impl Fn(Ev) -> M + Send + 'static) -> Task<M> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let _ = std::thread::Builder::new().name("hearth-settings-job".into()).spawn(move || run_lines(cmd, input, tx));
    Task::run(rx, f)
}

/// Only its end.
pub fn read<M: Send + 'static>(cmd: Vec<String>, f: impl Fn(Res) -> M + Send + 'static) -> Task<M> {
    read_in(cmd, None, f)
}
/// Only its end, with text for it (on stdin and in HEARTH_INPUT, never on the command line).
pub fn read_in<M: Send + 'static>(cmd: Vec<String>, input: Option<String>, f: impl Fn(Res) -> M + Send + 'static) -> Task<M> {
    use futures::StreamExt;
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let _ = std::thread::Builder::new().name("hearth-settings-read".into()).spawn(move || run_lines(cmd, input, tx));
    Task::stream(rx.filter_map(move |e| {
        futures::future::ready(match e {
            Ev::Done(r) => Some(f(r)),
            Ev::Line(..) => None,
        })
    }))
}

/// Hearth's core, as HearthCall.qml asks it: `nimbus --settings-call METHOD JSON` answers {"ok": true, "value": ..}.
pub fn core<M: Send + 'static>(method: &str, args: Value, secs: u32, f: impl Fn(Result<Value, String>) -> M + Send + 'static) -> Task<M> {
    let cmd = prog(vec!["nimbus".into(), "--settings-call".into(), method.into(), args.to_string()], secs);
    read(cmd, move |r| {
        if r.v.get("ok").and_then(Value::as_bool) == Some(true) {
            f(Ok(r.v.get("value").cloned().unwrap_or(Value::Null)))
        } else if let Some(e) = r.v.get("error").and_then(Value::as_str) {
            f(Err(e.to_string()))
        } else if r.err.contains("isn't on this system") {
            f(Err("The Hearth app isn't on this system".into()))
        } else {
            f(Err(if r.err.is_empty() { "Hearth couldn't do that".into() } else { r.err }))
        }
    })
}

/// Opens a program and doesn't wait (an app, a folder, Game Mode).
pub fn open(cmd: &[&str]) {
    hearth_ui::run::detached(cmd);
}
/// What has no app of its own yet opens in the Hearth app's window at that page (HearthCall.openPage).
pub fn open_hearth_page(page: &str) {
    hearth_ui::run::detached(&["nimbus", "--windowed", "--page", page]);
}
pub fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

// ---------------------------------------------------------------- reading the answers (JavaScript's truthiness)

pub fn gs(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}
pub fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().map(|x| x != 0.0).unwrap_or(false),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}
pub fn gb(v: &Value, k: &str) -> bool {
    truthy(v.get(k))
}
/// False only when it's there and false (`x !== false`).
pub fn not_false(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool) != Some(false)
}
pub fn gn(v: &Value, k: &str) -> f64 {
    match v.get(k) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0.0),
        Some(Value::Bool(true)) => 1.0,
        _ => 0.0,
    }
}
pub fn ga<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(Value::as_array).map(|a| a.as_slice()).unwrap_or(&[])
}
pub fn go<'a>(v: &'a Value, k: &str) -> &'a Value {
    static NULL: Value = Value::Null;
    v.get(k).unwrap_or(&NULL)
}
pub fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}
pub fn mb(b: f64) -> String {
    if b <= 0.0 {
        String::new()
    } else if b > 1e9 {
        format!("{:.1} GB", b / 1e9)
    } else {
        format!("{} MB", (b / 1e6).round())
    }
}

// ---------------------------------------------------------------- the line under the page's rows

/// A page's message: what was done (in the accent) or why it didn't work (red).
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
    }
}

/// A running job's line and bar (SetJob's progress).
#[derive(Debug, Clone, Default)]
pub struct Busy {
    pub on: bool,
    pub frac: f32,
    pub line: String,
}
impl Busy {
    pub fn start(&mut self) {
        *self = Busy { on: true, frac: -1.0, line: String::new() };
    }
    pub fn step(&mut self, f: f32, l: String) {
        self.frac = f;
        if !l.is_empty() {
            self.line = l;
        }
    }
    pub fn stop(&mut self) {
        *self = Busy::default();
    }
}

pub fn say_view<'a, M: 'a>(p: &Pal, s: &Say) -> Option<E<'a, M>> {
    if s.text.is_empty() {
        return None;
    }
    Some(container(t(s.text.clone(), sz(13.0), if s.bad { p.danger } else { p.flame })).padding(Padding { top: 2.0, right: 0.0, bottom: 2.0, left: 4.0 }).into())
}

/// Plain words on the page (muted, or red).
pub fn words<'a, M: 'a>(p: &Pal, s: impl Into<String>, bad: bool) -> E<'a, M> {
    container(t(s.into(), sz(13.0), if bad { p.danger } else { p.muted })).padding(Padding { top: 2.0, right: 0.0, bottom: 2.0, left: 4.0 }).width(Length::Fill).into()
}
pub fn small<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    container(t(s.into(), sz(12.0), p.muted)).padding(Padding { top: 2.0, right: 0.0, bottom: 2.0, left: 4.0 }).width(Length::Fill).into()
}

pub fn busy_view<'a, M: 'a>(p: &Pal, b: &Busy, what: &str) -> Option<E<'a, M>> {
    if !b.on {
        return None;
    }
    let label = if b.line.is_empty() { what.to_string() } else { b.line.clone() };
    let label = if b.frac >= 0.0 { format!("{label}  {}%", (b.frac * 100.0).round()) } else { label };
    let f = if b.frac >= 0.0 { b.frac.max(0.03) } else { 0.03 };
    Some(column![t(label, sz(14.0), p.text), ui::bar(vec![(f, p.flame)], p.panel3, 8.0, 4.0)].spacing(6.0).width(Length::Fill).into())
}

// ---------------------------------------------------------------- rows

/// What Mantel's SetRow chips say how something is: ok (green), warn, off, new (the accent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Chip {
    Ok,
    Warn,
    Off,
    New,
}
pub fn chip_of<'a, M: 'a>(p: &Pal, label: impl Into<String>, k: Chip) -> E<'a, M> {
    let (bg, fg) = match k {
        Chip::Ok => (Pal::a(p.good, 0.18), p.good),
        Chip::Warn => (Pal::a(p.warn, 0.2), p.warn),
        Chip::Off => (Pal::mix(p.panel2, p.text, 0.1), p.muted),
        Chip::New => (Pal::a(p.accent, 0.2), p.accent),
    };
    chip(label.into(), bg, fg)
}

/// A full row: icon, name (red for a danger row), the line under it, a chip, a value, a chevron; clickable with `msg`.
pub struct R<M> {
    pub g: G,
    pub title: String,
    pub sub: String,
    pub chip: Option<(String, Chip)>,
    pub value: String,
    pub chev: bool,
    pub danger: bool,
    pub msg: Option<M>,
    pub indent: bool,
}
pub fn r<M>(g: G, title: impl Into<String>, sub: impl Into<String>) -> R<M> {
    R { g, title: title.into(), sub: sub.into(), chip: None, value: String::new(), chev: false, danger: false, msg: None, indent: false }
}
impl<M: Clone + Send + 'static> R<M> {
    pub fn on(mut self, m: M) -> Self {
        self.msg = Some(m);
        self
    }
    pub fn on_maybe(mut self, m: Option<M>) -> Self {
        self.msg = m;
        self
    }
    pub fn chev(mut self) -> Self {
        self.chev = true;
        self
    }
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
    pub fn danger_if(mut self, d: bool) -> Self {
        self.danger = d;
        self
    }
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = v.into();
        self
    }
    pub fn chip(mut self, l: impl Into<String>, k: Chip) -> Self {
        let l = l.into();
        if !l.is_empty() {
            self.chip = Some((l, k));
        }
        self
    }
    pub fn indent(mut self) -> Self {
        self.indent = true;
        self
    }
    pub fn view<'a>(self, p: &Pal) -> E<'a, M>
    where
        M: 'a,
    {
        let mut right = Row::new().spacing(10.0).align_y(iced::Center);
        if let Some((l, k)) = self.chip {
            right = right.push(chip_of(p, l, k));
        }
        if !self.value.is_empty() {
            right = right.push(t(self.value, sz(13.0), if self.danger { p.danger } else { p.muted }));
        }
        if self.chev {
            right = right.push(glyph(G::Chevron, p.muted, 14.0, 2.0));
        }
        let e = line_row(p, self.g, self.title, if self.danger { p.danger } else { p.text }, self.sub, right.into());
        let e: E<'a, M> = match self.msg {
            Some(m) => mouse_area(e).on_press(m).interaction(mouse::Interaction::Pointer).into(),
            None => e,
        };
        if self.indent {
            container(e).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 52.0 }).into()
        } else {
            e
        }
    }
}

/// widgets::srow with the name in a colour of its own (red rows).
pub fn line_row<'a, M: 'a>(p: &Pal, g: G, title: String, color: Color, sub: String, control: E<'a, M>) -> E<'a, M> {
    let mut w = Column::new().spacing(3.0).push(tm(title, sz(15.0), color));
    if !sub.is_empty() {
        w = w.push(t(sub, sz(13.0), p.muted));
    }
    container(row![icon_box(p, g), w.width(Length::Fill), control].spacing(16.0).align_y(iced::Center))
        .padding(Padding { top: sz(14.0), right: 20.0, bottom: sz(14.0), left: 20.0 })
        .width(Length::Fill)
        .style(ui::boxed(p.panel, p.r(10.0), None))
        .into()
}

/// SetSwitch: a row with a switch (None: shown, can't be changed).
pub fn sw<'a, M: Clone + Send + 'static>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, on: bool, msg: Option<M>) -> E<'a, M> {
    srow(p, g, title, sub, switch(p, on, msg))
}
/// SetChoice: a row with the choices side by side.
pub fn choice<'a, M: Clone + Send + 'static>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, items: Vec<(String, bool, Option<M>)>) -> E<'a, M> {
    srow(p, g, title, sub, seg(p, items))
}
/// SetChoice with its choices under the name, for long lists (they'd crowd the row).
pub fn choice_wide<'a, M: Clone + Send + 'static>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, items: Vec<(String, bool, Option<M>)>) -> E<'a, M> {
    let mut w = Column::new().spacing(3.0).push(tm(title.into(), sz(15.0), p.text));
    let sub: String = sub.into();
    if !sub.is_empty() {
        w = w.push(t(sub, sz(13.0), p.muted));
    }
    container(column![row![icon_box(p, g), w.width(Length::Fill)].spacing(16.0).align_y(iced::Center), container(seg(p, items)).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 56.0 })].spacing(10.0))
        .padding(Padding { top: sz(14.0), right: 20.0, bottom: sz(14.0), left: 20.0 })
        .width(Length::Fill)
        .style(ui::boxed(p.panel, p.r(10.0), None))
        .into()
}
/// SetExpander's top row: › closed, ⌄ open.
pub fn expander<'a, M: Clone + Send + 'static>(p: &Pal, g: G, title: impl Into<String>, sub: impl Into<String>, chip: Option<(String, Chip)>, value: String, open: bool, msg: M) -> E<'a, M> {
    let mut right = Row::new().spacing(10.0).align_y(iced::Center);
    if let Some((l, k)) = chip {
        right = right.push(chip_of(p, l, k));
    }
    if !value.is_empty() {
        right = right.push(t(value, sz(13.0), p.muted));
    }
    right = right.push(glyph(if open { G::Down } else { G::Chevron }, p.muted, 14.0, 2.0));
    mouse_area(line_row(p, g, title.into(), p.text, sub.into(), right.into())).on_press(msg).interaction(mouse::Interaction::Pointer).into()
}
/// What an open expander holds, set in under it.
pub fn inside<'a, M: 'a>(items: Vec<E<'a, M>>) -> E<'a, M> {
    container(Column::with_children(items).spacing(8.0)).padding(Padding { top: 0.0, right: 0.0, bottom: 6.0, left: 36.0 }).width(Length::Fill).into()
}
/// SetCardGroup: a heading and its rows.
pub fn group<'a, M: 'a>(p: &Pal, title: impl Into<String>, items: Vec<E<'a, M>>) -> E<'a, M> {
    let mut c = Column::new().spacing(8.0).push(head(p, title));
    for i in items {
        c = c.push(i);
    }
    c.into()
}
/// A small button in a row (Save, Remove, Install…).
pub fn button<'a, M: Clone + Send + 'static>(p: &Pal, label: impl Into<String>, primary: bool, msg: Option<M>) -> E<'a, M> {
    crate::widgets::btn(p, label.into(), if primary { ui::Kind::Primary } else { ui::Kind::Normal }, false, msg)
}
pub fn buttons<'a, M: 'a>(items: Vec<E<'a, M>>) -> E<'a, M> {
    Row::with_children(items).spacing(10.0).align_y(iced::Center).into()
}
/// A labelled text field (SetField with its name over it).
pub fn labelled<'a, M: Clone + Send + 'static>(p: &Pal, label: &str, ph: &str, value: &str, on: impl Fn(String) -> M + 'a, submit: Option<M>) -> E<'a, M> {
    let f = field(p, ph, value, on);
    let f = match submit {
        Some(m) => f.on_submit(m),
        None => f,
    };
    column![container(t(label.to_string(), sz(12.0), p.muted)).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 4.0 }), f].spacing(3.0).into()
}
pub fn field_on<'a, M: Clone + Send + 'static>(p: &Pal, ph: &str, value: &str, on: impl Fn(String) -> M + 'a, submit: M) -> E<'a, M> {
    field(p, ph, value, on).on_submit(submit).into()
}
pub fn gap<'a, M: 'a>(h: f32) -> E<'a, M> {
    Space::new().height(Length::Fixed(h)).into()
}
/// A page: its parts one under the other.
pub fn page<'a, M: 'a>(items: Vec<E<'a, M>>) -> E<'a, M> {
    Column::with_children(items).spacing(10.0).width(Length::Fill).into()
}
/// Push `e` when it's there.
pub fn opt<'a, M: 'a>(v: &mut Vec<E<'a, M>>, e: Option<E<'a, M>>) {
    if let Some(e) = e {
        v.push(e);
    }
}

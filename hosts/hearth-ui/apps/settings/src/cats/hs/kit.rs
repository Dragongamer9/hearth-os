//! What Home server, Hearth Link, Updates & recovery and Home share: running Mantel's own helpers the way its SetJob
//! does (one JSON line back, or "error: ..."; text for the helper on its stdin and in HEARTH_INPUT, never on the
//! command line; a time limit on every run), the JSON they answer read like the QML pages read it, and the pieces
//! Mantel's Settings pages are made of (SetRow, SetSwitch, SetChoice, SetHeader, SetBanner, SetField, SetExpander,
//! SetCardGroup) drawn in the new Settings' look, so each SetHome*.qml page ports line by line.
//!
//! A page is described by its reads (which helper answers into which slot), what Enter in each of its fields does,
//! and whether something on it is still moving (an install, a download) so it's read again every 3 s while it is.
//! Nothing is read again at rest (red flag 2: no polling while nothing changes), and opening a page writes nothing
//! (red flag 9): every change is one of the helper's own commands, which save only the key that changed.
//!
//! In CI, HEARTH_SETTINGS_FIXTURES names a JSON file of believable answers ("helper args" → answer) so the pictures
//! show filled pages; never set on a PC.

use crate::shared::Shared;
use crate::widgets::{field, head, note, seg, switch, sz};
use hearth_ui::run;
use hearth_ui::ui::{self, chip, glyph, t, t1, tm, ts, G};
use hearth_ui::Pal;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::widget::{column, container, mouse_area, row, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Point, Rectangle, Renderer, Subscription, Task, Theme};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::process::{Command, Stdio};

pub type E<'a, M> = Element<'a, M>;

// ================================================================== the helpers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum H {
    #[default]
    Hs,
    ServerMode,
    Apps,
    Colibri,
    AiShare,
    Link,
    DevStatus,
    Files,
    Desk,
    Admin,
    Decky,
    Nimbus,
    Unlock,
    Restore,
    Io,
    Sh,
}

pub const ADMIN: &str = "/usr/libexec/nimbus/hearth-admin";
pub const DECKY: &str = "/usr/libexec/nimbus/hearth-decky";

pub fn helper_name(h: H) -> &'static str {
    match h {
        H::Hs => "hearth-home-server",
        H::ServerMode => "hearth-server-mode",
        H::Apps => "hearth-server-apps",
        H::Colibri => "hearth-colibri",
        H::AiShare => "hearth-ai-share",
        H::Link => "hearth-link-settings",
        H::DevStatus => "hearth-device-status",
        H::Files => "hearth-files",
        H::Desk => "hearth-desk",
        H::Admin => "hearth-admin",
        H::Decky => "hearth-decky",
        H::Nimbus => "nimbus",
        H::Unlock => "hearth-unlock",
        H::Restore => "hearth_restore.py",
        H::Io => "hearth_settings_io.py",
        H::Sh => "sh",
    }
}

/// The command for `helper args`: Mantel's helpers (next to its QML, run with python3 as SetJob runs them),
/// hearth-admin through pkexec (Hearth's polkit rule lets the person at the PC do it without a prompt), hearth-decky
/// as you, `nimbus` (hearth-core's --settings-call), hearth-unlock, or a short sh script.
pub fn cmd(h: H, args: &[String]) -> Vec<String> {
    let mut c: Vec<String> = match h {
        H::Admin => vec!["pkexec".into(), ADMIN.into()],
        H::Decky => vec!["python3".into(), DECKY.into()],
        H::Nimbus => vec!["nimbus".into()],
        H::Unlock => vec!["/usr/bin/hearth-unlock".into()],
        H::Sh => vec!["sh".into(), "-c".into()],
        other => vec!["python3".into(), run::shell_helper(helper_name(other))],
    };
    c.extend(args.iter().cloned());
    c
}

pub fn sv(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

fn fixtures() -> Option<&'static Value> {
    static F: std::sync::OnceLock<Option<Value>> = std::sync::OnceLock::new();
    F.get_or_init(|| {
        let p = std::env::var_os("HEARTH_SETTINGS_FIXTURES")?;
        serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()
    })
    .as_ref()
}

/// CI's stand-in answer for `helper args`, if there's one ("hearth-home-server status").
fn fixture(h: H, args: &[String]) -> Option<run::Output> {
    let f = fixtures()?;
    let key = format!("{} {}", helper_name(h), args.join(" "));
    // A key ending in "*" stands for every command that starts with it (the sh scripts).
    let key = key.trim();
    let v = f.get(key).or_else(|| f.as_object()?.iter().find(|(k, _)| k.ends_with('*') && key.starts_with(k.trim_end_matches('*'))).map(|x| x.1))?;
    Some(match v {
        Value::String(s) if s.starts_with("error: ") => run::Output { ok: false, code: 1, out: s.clone(), err: String::new() },
        Value::String(s) => run::Output { ok: true, code: 0, out: s.clone(), err: String::new() },
        other => run::Output { ok: true, code: 0, out: other.to_string(), err: String::new() },
    })
}

/// Runs it with a time limit (coreutils `timeout`, then a kill 5 s later), `input` on its stdin and in HEARTH_INPUT.
pub fn blocking(h: H, args: &[String], input: Option<&str>, secs: u32) -> run::Output {
    if let Some(o) = fixture(h, args) {
        return o;
    }
    let c = cmd(h, args);
    let mut full = vec!["timeout".to_string(), "-k".into(), "5".into(), secs.max(1).to_string()];
    full.extend(c);
    let mut command = Command::new(&full[0]);
    command.args(&full[1..]).stdout(Stdio::piped()).stderr(Stdio::piped());
    match input {
        Some(i) => {
            command.stdin(Stdio::piped()).env("HEARTH_INPUT", i);
        }
        None => {
            command.stdin(Stdio::null()).env_remove("HEARTH_INPUT");
        }
    }
    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => return run::Output { ok: false, code: -1, out: String::new(), err: format!("{}: {e}", helper_name(h)) },
    };
    if let (Some(i), Some(mut sin)) = (input, child.stdin.take()) {
        let _ = sin.write_all(i.as_bytes());
    }
    match child.wait_with_output() {
        Ok(o) => run::Output {
            ok: o.status.success(),
            code: o.status.code().unwrap_or(-1),
            out: String::from_utf8_lossy(&o.stdout).to_string(),
            err: String::from_utf8_lossy(&o.stderr).to_string(),
        },
        Err(e) => run::Output { ok: false, code: -1, out: String::new(), err: e.to_string() },
    }
}

pub fn spawn(h: H, args: Vec<String>, input: Option<String>, secs: u32) -> impl std::future::Future<Output = run::Output> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-settings-job".into()).spawn(move || {
        let _ = tx.send(blocking(h, &args, input.as_deref(), secs));
    });
    async move { rx.await.unwrap_or(run::Output { ok: false, code: -1, out: String::new(), err: "it stopped".into() }) }
}

/// A line a long job printed (hearth-admin update: "progress: 42 Downloading", "log: ..."), or its end.
#[derive(Debug, Clone)]
pub enum Line {
    Out(String),
    End(run::Output),
}

/// Runs it and hands back each line as it comes (for progress), then the end.
pub fn stream(h: H, args: Vec<String>, secs: u32) -> futures::channel::mpsc::UnboundedReceiver<Line> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let _ = std::thread::Builder::new().name("hearth-settings-stream".into()).spawn(move || {
        if let Some(o) = fixture(h, &args) {
            for l in o.out.lines() {
                let _ = tx.unbounded_send(Line::Out(l.to_string()));
            }
            let _ = tx.unbounded_send(Line::End(o));
            return;
        }
        let mut full = vec!["timeout".to_string(), "-k".into(), "5".into(), secs.to_string()];
        full.extend(cmd(h, &args));
        let child = Command::new(&full[0]).args(&full[1..]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.unbounded_send(Line::End(run::Output { ok: false, code: -1, out: String::new(), err: e.to_string() }));
                return;
            }
        };
        let mut all = String::new();
        if let Some(out) = child.stdout.take() {
            for l in std::io::BufReader::new(out).lines().map_while(Result::ok) {
                all.push_str(&l);
                all.push('\n');
                let _ = tx.unbounded_send(Line::Out(l));
            }
        }
        let mut err = String::new();
        if let Some(mut e) = child.stderr.take() {
            let _ = e.read_to_string(&mut err);
        }
        let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
        let _ = tx.unbounded_send(Line::End(run::Output { ok: code == 0, code, out: all, err }));
    });
    rx
}

/// What a helper said went wrong: its "error: ..." line, else its last line.
pub fn why(o: &run::Output) -> String {
    if let Some(l) = o.out.lines().rev().find(|l| l.starts_with("error: ")) {
        return l.trim_start_matches("error: ").trim().to_string();
    }
    if o.code == 126 || o.code == 127 {
        return "Not allowed (Hearth's polkit rule is missing).".into();
    }
    if o.code == 124 || o.code == 137 {
        return "It took too long and was stopped".into();
    }
    let w = o.why();
    if w.is_empty() { "It didn't work".into() } else { w }
}

/// The answer as JSON: its last JSON line, or {"error": why} when it failed.
pub fn answer(o: &run::Output) -> Value {
    if o.ok {
        // A command that worked and printed no JSON (bootc's own words) answered nothing in particular.
        return o.json().unwrap_or_else(|| Value::Object(Default::default()));
    }
    serde_json::json!({ "error": why(o) })
}

// ================================================================== reading the answers like the QML does
pub static NULL: Value = Value::Null;

/// `v` at "a.b.0.c" (Null when any part is missing).
pub fn at<'v>(v: &'v Value, path: &str) -> &'v Value {
    let mut cur = v;
    if path.is_empty() {
        return cur;
    }
    for k in path.split('.') {
        cur = match cur {
            Value::Object(m) => m.get(k).unwrap_or(&NULL),
            Value::Array(a) => k.parse::<usize>().ok().and_then(|i| a.get(i)).unwrap_or(&NULL),
            _ => &NULL,
        };
    }
    cur
}
/// JavaScript's `!!x`.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}
pub fn b(v: &Value, path: &str) -> bool {
    truthy(at(v, path))
}
/// `x !== false` (missing counts as on).
pub fn nf(v: &Value, path: &str) -> bool {
    at(v, path) != &Value::Bool(false)
}
pub fn s(v: &Value, path: &str) -> String {
    vs(at(v, path))
}
pub fn vs(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else {
                let f = n.as_f64().unwrap_or(0.0);
                if f.fract() == 0.0 { format!("{}", f as i64) } else { format!("{f}") }
            }
        }
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}
pub fn n(v: &Value, path: &str) -> f64 {
    match at(v, path) {
        Value::Number(x) => x.as_f64().unwrap_or(0.0),
        Value::String(x) => x.parse().unwrap_or(0.0),
        Value::Bool(true) => 1.0,
        _ => 0.0,
    }
}
pub fn arr<'v>(v: &'v Value, path: &str) -> &'v [Value] {
    match at(v, path) {
        Value::Array(a) => a.as_slice(),
        _ => &[],
    }
}
pub fn keys(v: &Value, path: &str) -> Vec<String> {
    match at(v, path) {
        Value::Object(m) => m.keys().cloned().collect(),
        _ => Vec::new(),
    }
}
pub fn has(v: &Value, path: &str) -> bool {
    !at(v, path).is_null()
}
/// "a" or "b" by number: 1 file / 2 files.
pub fn plural(k: f64, one: &str, many: &str) -> String {
    format!("{} {}", vs(&serde_json::json!(k)), if k == 1.0 { one } else { many })
}
pub fn join(parts: &[String], sep: &str) -> String {
    parts.iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(sep)
}

/// Sizes as the QML pages write them: 1.2 TB, 301 GB, 40 MB.
pub fn size(b: f64) -> String {
    if b >= 1e12 {
        format!("{:.1} TB", b / 1e12)
    } else if b >= 1e9 {
        format!("{} GB", (b / 1e9).round())
    } else if b >= 1e6 {
        format!("{} MB", (b / 1e6).round())
    } else {
        format!("{} KB", (b / 1e3).round())
    }
}
pub fn size1(b: f64) -> String {
    if b >= 1e12 {
        format!("{:.1} TB", b / 1e12)
    } else if b >= 1e9 {
        format!("{:.1} GB", b / 1e9)
    } else if b >= 1e6 {
        format!("{} MB", (b / 1e6).round())
    } else {
        format!("{} KB", (b / 1e3).round())
    }
}
/// "2 min ago", "3 h ago", "4 days ago" from seconds since 1970.
pub fn ago(secs: f64) -> String {
    if secs <= 0.0 {
        return "never".into();
    }
    let m = ((hearth_ui::time::now() as f64 - secs) / 60.0).round();
    if m < 2.0 {
        "just now".into()
    } else if m < 60.0 {
        format!("{m} min ago")
    } else if m < 1440.0 {
        format!("{} h ago", (m / 60.0).round())
    } else {
        format!("{} days ago", (m / 1440.0).round())
    }
}
/// A date and time in words ("Nov 18, 9:41 PM") from seconds since 1970.
pub fn when(secs: f64) -> String {
    if secs <= 0.0 {
        return String::new();
    }
    hearth_ui::time::when(secs as i64)
}
/// "Nov 12, 2026".
pub fn date(secs: f64) -> String {
    if secs <= 0.0 {
        return String::new();
    }
    format!("{}, {}", hearth_ui::time::day(secs as i64), hearth_ui::time::local(secs as i64).year)
}
/// JavaScript ms → s.
pub fn ms(v: f64) -> f64 {
    if v > 1e11 { v / 1000.0 } else { v }
}

// ================================================================== a page's state and what it asks for
/// A read: the slot it fills, the helper and its words.
#[derive(Debug, Clone)]
pub struct Rd {
    pub slot: String,
    pub h: H,
    pub args: Vec<String>,
}
pub fn rd(slot: &str, h: H, args: &[&str]) -> Rd {
    Rd { slot: slot.into(), h, args: sv(args) }
}

/// Something to run when a row is pressed, and what to do with its answer.
#[derive(Debug, Clone, Default)]
pub struct Act {
    pub h: H,
    pub args: Vec<String>,
    pub input: Option<String>,
    /// Said when it worked: "{a.b}" is filled from the answer, "{note|…}" the answer's note or the rest.
    pub ok: String,
    /// Said while it runs.
    pub wait: String,
    pub set: Vec<(String, Value)>,
    pub set_from: Vec<(String, String)>,
    pub fail_set: Vec<(String, Value)>,
    /// The error goes into this local (not the page's message).
    pub fail_to: Option<String>,
    pub store: Option<String>,
    pub copy: Option<String>,
    pub then: Option<Box<Act>>,
    pub secs: u32,
    pub no_reload: bool,
    pub reload_also: Vec<Rd>,
    /// Lines as they come (progress: N text, log: text, error: text), into the page's `prog`.
    pub stream: bool,
    /// The page to open afterwards (worked or not).
    pub go: Option<&'static str>,
    /// `then` runs only when the answer has this (a folder was picked); its words' "{a.b}" are filled from the answer.
    pub then_if: Option<String>,
}

pub fn act(h: H, args: &[&str]) -> Act {
    Act { h, args: sv(args), secs: 120, ..Act::default() }
}
pub fn act_v(h: H, args: Vec<String>) -> Act {
    Act { h, args, secs: 120, ..Act::default() }
}
impl Act {
    pub fn ok(mut self, s: impl Into<String>) -> Act {
        self.ok = s.into();
        self
    }
    pub fn wait(mut self, s: impl Into<String>) -> Act {
        self.wait = s.into();
        self
    }
    pub fn input(mut self, s: impl Into<String>) -> Act {
        self.input = Some(s.into());
        self
    }
    pub fn set(mut self, k: &str, v: Value) -> Act {
        self.set.push((k.into(), v));
        self
    }
    pub fn set_from(mut self, k: &str, path: &str) -> Act {
        self.set_from.push((k.into(), path.into()));
        self
    }
    pub fn fail_set(mut self, k: &str, v: Value) -> Act {
        self.fail_set.push((k.into(), v));
        self
    }
    pub fn fail_to(mut self, k: &str) -> Act {
        self.fail_to = Some(k.into());
        self
    }
    pub fn store(mut self, slot: &str) -> Act {
        self.store = Some(slot.into());
        self
    }
    pub fn copy(mut self, path: &str) -> Act {
        self.copy = Some(path.into());
        self
    }
    pub fn then(mut self, a: Act) -> Act {
        self.then = Some(Box::new(a));
        self
    }
    pub fn secs(mut self, s: u32) -> Act {
        self.secs = s;
        self
    }
    pub fn no_reload(mut self) -> Act {
        self.no_reload = true;
        self
    }
    pub fn also(mut self, r: Rd) -> Act {
        self.reload_also.push(r);
        self
    }
    pub fn stream(mut self) -> Act {
        self.stream = true;
        self
    }
    pub fn then_if(mut self, path: &str, a: Act) -> Act {
        self.then_if = Some(path.into());
        self.then = Some(Box::new(a));
        self
    }
    pub fn go(mut self, page: &'static str) -> Act {
        self.go = Some(page);
        self
    }
    pub fn msg(self) -> Msg {
        Msg::Run(Box::new(self))
    }
}

/// A long job's progress (hearth-admin update).
#[derive(Debug, Clone, Default)]
pub struct Prog {
    pub running: bool,
    pub what: String,
    pub pct: f32,
    pub line: String,
    pub error: String,
    pub answer: Value,
}

#[derive(Default)]
pub struct St {
    pub page: &'static str,
    /// What the page's reads answered, by slot.
    pub d: HashMap<String, Value>,
    /// The page's own state (what's open, picked, asked twice), gone when the page closes.
    pub l: HashMap<String, Value>,
    /// What's typed in its fields.
    pub t: HashMap<String, String>,
    pub msg: String,
    pub bad: bool,
    pub busy: u32,
    pub prog: Prog,
}

impl St {
    pub fn get(&self, slot: &str) -> &Value {
        self.d.get(slot).unwrap_or(&NULL)
    }
    pub fn loc(&self, k: &str) -> &Value {
        self.l.get(k).unwrap_or(&NULL)
    }
    pub fn ls(&self, k: &str) -> String {
        vs(self.loc(k))
    }
    pub fn lb(&self, k: &str) -> bool {
        truthy(self.loc(k))
    }
    pub fn ln(&self, k: &str) -> f64 {
        n(self.loc(k), "")
    }
    pub fn txt(&self, k: &str) -> String {
        self.t.get(k).cloned().unwrap_or_default()
    }
    pub fn say(&mut self, s: impl Into<String>, bad: bool) {
        self.msg = s.into();
        self.bad = bad;
    }
    pub fn setl(&mut self, k: &str, v: Value) {
        if v.is_null() {
            self.l.remove(k);
        } else {
            self.l.insert(k.into(), v);
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    Got(String, run::Output),
    Run(Box<Act>),
    Did(Box<Act>, run::Output),
    Line(Box<Act>, Line),
    Local(String, Value),
    /// Several at once (Pick a device and close what was open).
    Locals(Vec<(String, Value)>),
    Text(String, String),
    Submit(String),
    Go(&'static str),
    Open(String),
    Show(String),
    Copy(String),
    Detached(Vec<String>),
    Say(String, bool),
    Tick,
}

/// A page's own parts: its reads (and the ones that follow a read), what Enter in its fields does, and whether
/// something on it is moving.
pub struct Hooks {
    pub reads: fn(&str, &St) -> Vec<Rd>,
    pub follow: fn(&str, &str, &St) -> Vec<Rd>,
    pub submit: fn(&mut St, &str, String) -> Result<Vec<Act>, String>,
    pub live: fn(&str, &St) -> bool,
}

pub fn no_follow(_p: &str, _slot: &str, _st: &St) -> Vec<Rd> {
    Vec::new()
}
pub fn no_submit(_st: &mut St, _k: &str, _t: String) -> Result<Vec<Act>, String> {
    Ok(Vec::new())
}

fn read(r: Rd) -> Task<Msg> {
    let slot = r.slot.clone();
    Task::perform(spawn(r.h, r.args, None, 60), move |o| Msg::Got(slot.clone(), o))
}

fn reads(v: Vec<Rd>) -> Task<Msg> {
    Task::batch(v.into_iter().map(read))
}

/// `part` is what `hearth-settings PAGE:part` asked for: "a=b,c" opens the page with a = "b" and c on (a share's
/// settings open, an expander open), as the QML pages' anchors do.
pub fn open(st: &mut St, page: &'static str, part: &str, hk: &Hooks) -> Task<Msg> {
    let running = std::mem::take(&mut st.prog);
    let off = st.l.remove("offAt");
    *st = St { page, ..St::default() };
    // A long job (an update) and its shutdown countdown keep going when its page is opened again.
    if running.running {
        st.prog = running;
    }
    if let Some(o) = off {
        st.l.insert("offAt".into(), o);
    }
    for kv in part.split(',').filter(|x| !x.is_empty()) {
        match kv.split_once('=') {
            Some((k, v)) => st.setl(k, Value::String(v.into())),
            None => st.setl(kv, Value::Bool(true)),
        }
    }
    reads((hk.reads)(page, st))
}

fn fill(tpl: &str, v: &Value) -> String {
    let mut out = String::new();
    let mut rest = tpl;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('}') {
            Some(j) => {
                let inner = &after[..j];
                let (path, dflt) = match inner.split_once('|') {
                    Some((p, d)) => (p, Some(d)),
                    None => (inner, None),
                };
                let val = match at(v, path) {
                    Value::Array(a) => a.iter().map(vs).collect::<Vec<_>>().join(", "),
                    x => vs(x),
                };
                out.push_str(if val.is_empty() { dflt.unwrap_or("") } else { &val });
                rest = &after[j + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn start(st: &mut St, a: Act) -> Task<Msg> {
    st.busy += 1;
    if !a.wait.is_empty() {
        st.say(a.wait.clone(), false);
    }
    let boxed = Box::new(a);
    if boxed.stream {
        st.prog = Prog { running: true, what: boxed.args.first().cloned().unwrap_or_default(), pct: -1.0, ..Prog::default() };
        let rx = stream(boxed.h, boxed.args.clone(), boxed.secs.max(60));
        let b2 = boxed.clone();
        return Task::run(rx, move |l| Msg::Line(b2.clone(), l));
    }
    let (h, args, input, secs) = (boxed.h, boxed.args.clone(), boxed.input.clone(), boxed.secs);
    Task::perform(spawn(h, args, input, secs), move |o| Msg::Did(boxed.clone(), o))
}

fn done(sh: &mut Shared, st: &mut St, a: Box<Act>, o: run::Output, hk: &Hooks) -> Task<Msg> {
    st.busy = st.busy.saturating_sub(1);
    if let Some(g) = a.go {
        sh.go(g);
    }
    let v = answer(&o);
    let mut more = Vec::new();
    if o.ok && !b(&v, "error") {
        for (k, x) in &a.set {
            // "@now+N": N seconds from now (the shutdown after an update).
            let x = match x.as_str().and_then(|t| t.strip_prefix("@now+")).and_then(|t| t.parse::<i64>().ok()) {
                Some(secs) => Value::from(hearth_ui::time::now() + secs),
                None => x.clone(),
            };
            st.setl(k, x);
        }
        for (k, p) in &a.set_from {
            st.setl(k, at(&v, p).clone());
        }
        if let Some(slot) = &a.store {
            st.d.insert(slot.clone(), if slot.ends_with("text") { Value::String(o.out.clone()) } else { v.clone() });
            let f = (hk.follow)(st.page, slot, st);
            if !f.is_empty() {
                more.push(reads(f));
            }
        }
        if !a.ok.is_empty() {
            st.say(fill(&a.ok, &v), false);
        } else if !a.wait.is_empty() && st.msg == a.wait {
            st.msg.clear();
        }
        if let Some(p) = &a.copy {
            let c = vs(at(&v, p));
            if !c.is_empty() {
                more.push(iced::clipboard::write(c));
            }
        }
        if let Some(t) = a.then.clone() {
            if a.then_if.as_ref().map(|p| b(&v, p)).unwrap_or(true) {
                let mut t = *t;
                t.args = t.args.iter().map(|x| if x.contains('{') { fill(x, &v) } else { x.clone() }).collect();
                more.push(start(st, t));
            }
        }
    } else {
        let e = if b(&v, "error") { s(&v, "error") } else { why(&o) };
        for (k, x) in &a.fail_set {
            st.setl(k, x.clone());
        }
        match &a.fail_to {
            Some(k) => st.setl(k, Value::String(e)),
            None => st.say(e, true),
        }
    }
    if !a.no_reload {
        more.push(reads((hk.reads)(st.page, st)));
    }
    if !a.reload_also.is_empty() {
        more.push(reads(a.reload_also.clone()));
    }
    Task::batch(more)
}

pub fn update(sh: &mut Shared, st: &mut St, m: Msg, hk: &Hooks) -> Task<Msg> {
    match m {
        Msg::Got(slot, o) => {
            // Slots named "...text" keep what was printed as it is (qrencode's picture in characters).
            let v = if slot.ends_with("text") { if o.ok { Value::String(o.out.clone()) } else { Value::Null } } else { answer(&o) };
            st.d.insert(slot.clone(), v);
            let f = (hk.follow)(st.page, &slot, st);
            if !f.is_empty() {
                return reads(f);
            }
        }
        Msg::Run(a) => return start(st, *a),
        Msg::Did(a, o) => return done(sh, st, a, o, hk),
        Msg::Line(a, l) => match l {
            Line::Out(l) => {
                let l = l.trim().to_string();
                if let Some(x) = l.strip_prefix("log: ") {
                    st.prog.line = x.to_string();
                } else if let Some(x) = l.strip_prefix("error: ") {
                    st.prog.error = x.to_string();
                } else if let Some(x) = l.strip_prefix("progress: ") {
                    let (num, text) = x.split_once(' ').unwrap_or((x, ""));
                    if let Ok(p) = num.parse::<f32>() {
                        st.prog.pct = p / 100.0;
                    }
                    if !text.is_empty() {
                        st.prog.line = text.to_string();
                    }
                } else if l.starts_with('{') {
                    if let Ok(v) = serde_json::from_str::<Value>(&l) {
                        st.prog.answer = v;
                    }
                }
            }
            Line::End(o) => {
                st.prog.running = false;
                if !o.ok && st.prog.error.is_empty() {
                    st.prog.error = why(&o);
                }
                return done(sh, st, a, o, hk);
            }
        },
        Msg::Local(k, v) => st.setl(&k, v),
        Msg::Locals(v) => {
            for (k, x) in v {
                st.setl(&k, x);
            }
        }
        Msg::Text(k, v) => {
            st.t.insert(k, v);
        }
        Msg::Submit(k) => {
            let text = st.txt(&k);
            match (hk.submit)(st, &k, text) {
                Ok(acts) => {
                    if !acts.is_empty() {
                        st.t.insert(k, String::new());
                    }
                    return Task::batch(acts.into_iter().map(|a| start(st, a)).collect::<Vec<_>>());
                }
                Err(e) => st.say(e, true),
            }
        }
        Msg::Go(id) => sh.go(id),
        Msg::Open(u) => run::detached(&["xdg-open", &u]),
        Msg::Show(p) => {
            let portal = run::shell_helper("hearth-portal");
            run::detached(&["python3", &portal, "show", &p]);
        }
        Msg::Copy(c) => {
            st.say("Copied ✓", false);
            return iced::clipboard::write(c);
        }
        Msg::Detached(c) => {
            let r: Vec<&str> = c.iter().map(|x| x.as_str()).collect();
            run::detached(&r);
        }
        Msg::Say(t, bad) => st.say(t, bad),
        Msg::Tick => {
            // A shutdown counting down (Updates & recovery's "Shut down when it's done"): no reading meanwhile.
            if st.l.contains_key("offAt") {
                if hearth_ui::time::now() as f64 >= st.ln("offAt") {
                    st.l.remove("offAt");
                    run::detached(&["systemctl", "poweroff"]);
                }
                return Task::none();
            }
            if st.busy == 0 {
                return reads((hk.reads)(st.page, st));
            }
        }
    }
    Task::none()
}

pub fn subscription(st: &St, hk: &Hooks) -> Subscription<Msg> {
    if !st.page.is_empty() && (hk.live)(st.page, st) {
        let every = if st.l.contains_key("offAt") { 1 } else { 3 };
        iced::time::every(std::time::Duration::from_secs(every)).map(|_| Msg::Tick)
    } else {
        Subscription::none()
    }
}

/// A QR code from `qrencode -t ASCII` (two characters a square, "#" dark), drawn black on white.
struct Qr {
    rows: Vec<Vec<bool>>,
}
impl<M> canvas::Program<M> for Qr {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        f.fill_rectangle(Point::ORIGIN, bounds.size(), Color::WHITE);
        let n = self.rows.len().max(1) as f32;
        let k = bounds.width.min(bounds.height) / n;
        for (y, rw) in self.rows.iter().enumerate() {
            for (x, on) in rw.iter().enumerate() {
                if *on {
                    f.fill_rectangle(Point::new(x as f32 * k, y as f32 * k), iced::Size::new(k + 0.3, k + 0.3), Color::BLACK);
                }
            }
        }
        vec![f.into_geometry()]
    }
}
pub fn qr<'a, M: 'a>(ascii: &str, px: f32) -> E<'a, M> {
    let rows: Vec<Vec<bool>> = ascii
        .lines()
        .filter(|l| !l.trim().is_empty() || l.len() > 2)
        .map(|l| l.as_bytes().chunks(2).map(|c| c[0] == b'#').collect())
        .collect();
    canvas::Canvas::new(Qr { rows }).width(Length::Fixed(px)).height(Length::Fixed(px)).into()
}

/// SetPinPad: the digits typed so far as dots, and 1–9, ⌫, 0, ✓ (✓ sends Submit(key) once there are enough).
pub fn pinpad<'a>(p: &Pal, st: &St, key: &str, hint: &str, min: usize, max: usize) -> E<'a, Msg> {
    let p = *p;
    let cur = st.txt(key);
    let ready = cur.len() >= min && cur.len() <= max;
    let dots: String = if cur.is_empty() { "—".into() } else { "●".repeat(cur.len()) };
    let keyb = |label: &str, m: Option<Msg>| -> E<'a, Msg> {
        let fg = if m.is_some() { p.text } else { p.muted };
        let c = container(tm(label.to_string(), sz(16.0), fg)).center_x(Length::Fixed(sz(58.0))).center_y(Length::Fixed(sz(40.0))).style(ui::boxed(Pal::mix(p.panel, p.text, 0.06), p.r(8.0), None));
        match m {
            Some(x) => mouse_area(c).on_press(x).interaction(mouse::Interaction::Pointer).into(),
            None => c.into(),
        }
    };
    let mut grid = Column::new().spacing(6.0);
    let keys = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "⌫", "0", "✓"];
    for rowk in keys.chunks(3) {
        let mut rw = Row::new().spacing(6.0);
        for k in rowk {
            let m = match *k {
                "⌫" => (!cur.is_empty()).then(|| Msg::Text(key.into(), cur[..cur.len() - 1].to_string())),
                "✓" => ready.then(|| Msg::Submit(key.into())),
                d => (cur.len() < max).then(|| Msg::Text(key.into(), format!("{cur}{d}"))),
            };
            rw = rw.push(keyb(k, m));
        }
        grid = grid.push(rw);
    }
    let typed = tfield(&p, st, key, "Or type it, then Enter", true);
    container(row![grid, column![t(hint.to_string(), sz(13.0), p.muted), ts(dots, sz(20.0), p.text), typed].spacing(10.0).width(Length::Fill)].spacing(22.0))
        .padding(Padding { top: 10.0, right: 18.0, bottom: 10.0, left: 56.0 })
        .width(Length::Fill)
        .into()
}

// ================================================================== the pieces
pub fn lv(k: &str, v: Value) -> Msg {
    Msg::Local(k.into(), v)
}
pub fn toggle(st: &St, k: &str) -> Msg {
    Msg::Local(k.into(), Value::Bool(!st.lb(k)))
}
/// Picks `v` in `k`, or nothing when it's already picked (QML's `sel = sel === id ? "" : id`).
pub fn pick(st: &St, k: &str, v: &str) -> Msg {
    Msg::Local(k.into(), if st.ls(k) == v { Value::Null } else { Value::String(v.into()) })
}

/// A row's icon (Mantel's icon names).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum I {
    G(G),
    Sun,
    Bell,
    BellOff,
    Batt,
    Lock,
    Trash,
    Globe,
    Cloud,
    Chat,
    Film,
    Mic,
    Cam,
    File,
    Back,
    Forward,
    Eye,
    Box,
    Bolt,
    Wave,
    Clipboard,
    Open,
    Undo,
    Cpu,
    Warn,
    Calendar,
    Sparkle,
    Wired,
    Headphones,
    Edit,
    Image,
}

pub fn icon_named(name: &str) -> I {
    match name {
        "wifi" => I::G(G::Wifi),
        "bt" => I::G(G::Bt),
        "display" | "desktops" => I::G(G::Display),
        "folder" => I::G(G::Folder),
        "home" => I::G(G::Home),
        "shield" => I::G(G::Shield),
        "drive" | "store" | "archive" => I::G(G::Drive),
        "user" => I::G(G::User),
        "users" | "family" => I::G(G::Users),
        "server" => I::G(G::Server),
        "pad" => I::G(G::Pad),
        "power" => I::G(G::Power),
        "update" | "sync" | "restart" => I::G(G::Update),
        "check" => I::G(G::Check),
        "plus" => I::G(G::Plus),
        "close" => I::G(G::Close),
        "clock" => I::G(G::Clock),
        "search" => I::G(G::Search),
        "link" => I::G(G::Link),
        "play" => I::G(G::Play),
        "gear" => I::G(G::Gear),
        "info" => I::G(G::Info),
        "moon" | "sleep" => I::G(G::Moon),
        "vol" => I::G(G::Sound),
        "rocket" => I::G(G::Rocket),
        "grid" => I::G(G::Grid),
        "palette" => I::G(G::Palette),
        "sun" => I::Sun,
        "bell" => I::Bell,
        "bell-off" => I::BellOff,
        "batt" | "batt-charging" => I::Batt,
        "lock" => I::Lock,
        "trash" => I::Trash,
        "globe" => I::Globe,
        "cloud" => I::Cloud,
        "chat" => I::Chat,
        "film" => I::Film,
        "mic" => I::Mic,
        "cam" => I::Cam,
        "file" | "doc" => I::File,
        "back" => I::Back,
        "forward" => I::Forward,
        "eye" => I::Eye,
        "box" => I::Box,
        "bolt" => I::Bolt,
        "wave" => I::Wave,
        "clipboard" => I::Clipboard,
        "open" => I::Open,
        "undo" => I::Undo,
        "cpu" => I::Cpu,
        "warn" => I::Warn,
        "calendar" => I::Calendar,
        "sparkle" => I::Sparkle,
        "wired" => I::Wired,
        "headphones" => I::Headphones,
        "edit" => I::Edit,
        "image" => I::Image,
        _ => I::G(G::Gear),
    }
}

type Pts = Vec<Vec<(f32, f32)>>;
fn arcp(cx: f32, cy: f32, r: f32, a0: f32, a1: f32) -> Vec<(f32, f32)> {
    (0..=16).map(|i| {
        let a = (a0 + (a1 - a0) * i as f32 / 16.0).to_radians();
        (cx + r * a.cos(), cy + r * a.sin())
    }).collect()
}
fn shape(i: I) -> (Pts, Vec<(f32, f32, f32)>) {
    let v = |p: &[(f32, f32)]| p.to_vec();
    match i {
        I::G(_) => (vec![], vec![]),
        I::Sun => {
            let mut l = Vec::new();
            for k in 0..8 {
                let a = (k as f32 * 45.0f32).to_radians();
                l.push(vec![(12.0 + 7.0 * a.cos(), 12.0 + 7.0 * a.sin()), (12.0 + 10.0 * a.cos(), 12.0 + 10.0 * a.sin())]);
            }
            (l, vec![(12.0, 12.0, 4.0)])
        }
        I::Bell => (vec![v(&[(6.0, 17.0), (6.0, 11.0), (8.0, 6.5), (12.0, 5.0), (16.0, 6.5), (18.0, 11.0), (18.0, 17.0)]), v(&[(4.0, 17.0), (20.0, 17.0)]), v(&[(10.0, 20.0), (14.0, 20.0)])], vec![]),
        I::BellOff => (vec![v(&[(6.0, 17.0), (6.0, 11.0), (8.0, 6.5), (12.0, 5.0), (16.0, 6.5), (18.0, 11.0), (18.0, 17.0)]), v(&[(4.0, 17.0), (20.0, 17.0)]), v(&[(10.0, 20.0), (14.0, 20.0)]), v(&[(3.0, 3.0), (21.0, 21.0)])], vec![]),
        I::Batt => (vec![v(&[(4.0, 7.0), (18.0, 7.0), (18.0, 17.0), (4.0, 17.0), (4.0, 7.0)]), v(&[(20.5, 10.0), (20.5, 14.0)]), v(&[(7.0, 10.0), (7.0, 14.0)]), v(&[(10.0, 10.0), (10.0, 14.0)])], vec![]),
        I::Lock => (vec![v(&[(5.0, 11.0), (19.0, 11.0), (19.0, 21.0), (5.0, 21.0), (5.0, 11.0)]), arcp(12.0, 8.0, 4.0, 180.0, 360.0), v(&[(8.0, 8.0), (8.0, 11.0)]), v(&[(16.0, 8.0), (16.0, 11.0)])], vec![(12.0, 16.0, 1.2)]),
        I::Trash => (vec![v(&[(4.0, 7.0), (20.0, 7.0)]), v(&[(6.0, 7.0), (7.0, 21.0), (17.0, 21.0), (18.0, 7.0)]), v(&[(9.0, 7.0), (10.0, 3.5), (14.0, 3.5), (15.0, 7.0)]), v(&[(10.0, 11.0), (10.0, 17.0)]), v(&[(14.0, 11.0), (14.0, 17.0)])], vec![]),
        I::Globe => (vec![v(&[(3.0, 12.0), (21.0, 12.0)]), arcp(12.0, 12.0, 9.0, -90.0, 90.0).iter().map(|p| (12.0 + (p.0 - 12.0) * 0.45, p.1)).collect(), arcp(12.0, 12.0, 9.0, 90.0, 270.0).iter().map(|p| (12.0 + (p.0 - 12.0) * 0.45, p.1)).collect()], vec![(12.0, 12.0, 9.0)]),
        I::Cloud => (vec![{
            let mut p = arcp(8.0, 15.0, 4.0, 90.0, 270.0);
            p.extend(arcp(12.5, 10.5, 5.0, 200.0, 340.0));
            p.extend(arcp(17.0, 15.0, 4.0, 270.0, 450.0));
            p.push((8.0, 19.0));
            p
        }], vec![]),
        I::Chat => (vec![v(&[(4.0, 5.0), (20.0, 5.0), (20.0, 16.0), (11.0, 16.0), (6.0, 20.0), (7.0, 16.0), (4.0, 16.0), (4.0, 5.0)])], vec![]),
        I::Film => (vec![v(&[(4.0, 4.0), (20.0, 4.0), (20.0, 20.0), (4.0, 20.0), (4.0, 4.0)]), v(&[(8.0, 4.0), (8.0, 20.0)]), v(&[(16.0, 4.0), (16.0, 20.0)]), v(&[(4.0, 12.0), (20.0, 12.0)])], vec![]),
        I::Mic => (vec![v(&[(9.5, 5.0), (12.0, 3.0), (14.5, 5.0), (14.5, 12.0), (12.0, 14.0), (9.5, 12.0), (9.5, 5.0)]), arcp(12.0, 11.0, 6.0, 0.0, 180.0), v(&[(12.0, 17.0), (12.0, 21.0)])], vec![]),
        I::Cam => (vec![v(&[(3.0, 8.0), (8.0, 8.0), (9.5, 5.0), (14.5, 5.0), (16.0, 8.0), (21.0, 8.0), (21.0, 19.0), (3.0, 19.0), (3.0, 8.0)])], vec![(12.0, 13.5, 3.5)]),
        I::File => (vec![v(&[(6.0, 3.0), (14.0, 3.0), (19.0, 8.0), (19.0, 21.0), (6.0, 21.0), (6.0, 3.0)]), v(&[(14.0, 3.0), (14.0, 8.0), (19.0, 8.0)])], vec![]),
        I::Back => (vec![v(&[(19.0, 12.0), (5.0, 12.0)]), v(&[(11.0, 6.0), (5.0, 12.0), (11.0, 18.0)])], vec![]),
        I::Forward => (vec![v(&[(5.0, 12.0), (19.0, 12.0)]), v(&[(13.0, 6.0), (19.0, 12.0), (13.0, 18.0)])], vec![]),
        I::Eye => (vec![{
            let mut p = arcp(12.0, 20.0, 12.0, 205.0, 335.0);
            p.extend(arcp(12.0, 4.0, 12.0, 25.0, 155.0));
            p
        }], vec![(12.0, 12.0, 3.0)]),
        I::Box => (vec![v(&[(3.0, 7.0), (12.0, 3.0), (21.0, 7.0), (21.0, 17.0), (12.0, 21.0), (3.0, 17.0), (3.0, 7.0)]), v(&[(3.0, 7.0), (12.0, 11.0), (21.0, 7.0)]), v(&[(12.0, 11.0), (12.0, 21.0)])], vec![]),
        I::Bolt => (vec![v(&[(13.0, 2.0), (5.0, 13.0), (11.0, 13.0), (10.0, 22.0), (19.0, 10.0), (13.0, 10.0), (13.0, 2.0)])], vec![]),
        I::Wave => (vec![v(&[(3.0, 12.0), (5.0, 12.0)]), v(&[(7.0, 8.0), (7.0, 16.0)]), v(&[(10.0, 4.0), (10.0, 20.0)]), v(&[(13.0, 7.0), (13.0, 17.0)]), v(&[(16.0, 10.0), (16.0, 14.0)]), v(&[(19.0, 8.0), (19.0, 16.0)])], vec![]),
        I::Clipboard => (vec![v(&[(6.0, 5.0), (18.0, 5.0), (18.0, 21.0), (6.0, 21.0), (6.0, 5.0)]), v(&[(9.0, 3.0), (15.0, 3.0), (15.0, 7.0), (9.0, 7.0), (9.0, 3.0)])], vec![]),
        I::Open => (vec![v(&[(11.0, 5.0), (5.0, 5.0), (5.0, 19.0), (19.0, 19.0), (19.0, 13.0)]), v(&[(13.0, 5.0), (19.0, 5.0), (19.0, 11.0)]), v(&[(19.0, 5.0), (11.0, 13.0)])], vec![]),
        I::Undo => (vec![{
            let mut p = vec![(5.0, 10.0)];
            p.extend(arcp(13.0, 14.0, 7.0, 220.0, 450.0));
            p
        }, v(&[(4.0, 5.0), (5.0, 10.0), (10.0, 10.0)])], vec![]),
        I::Cpu => (vec![v(&[(6.0, 6.0), (18.0, 6.0), (18.0, 18.0), (6.0, 18.0), (6.0, 6.0)]), v(&[(10.0, 10.0), (14.0, 10.0), (14.0, 14.0), (10.0, 14.0), (10.0, 10.0)]),
                        v(&[(9.0, 2.0), (9.0, 6.0)]), v(&[(15.0, 2.0), (15.0, 6.0)]), v(&[(9.0, 18.0), (9.0, 22.0)]), v(&[(15.0, 18.0), (15.0, 22.0)]),
                        v(&[(2.0, 9.0), (6.0, 9.0)]), v(&[(2.0, 15.0), (6.0, 15.0)]), v(&[(18.0, 9.0), (22.0, 9.0)]), v(&[(18.0, 15.0), (22.0, 15.0)])], vec![]),
        I::Warn => (vec![v(&[(12.0, 3.0), (22.0, 20.0), (2.0, 20.0), (12.0, 3.0)]), v(&[(12.0, 9.0), (12.0, 14.0)])], vec![(12.0, 17.0, 1.1)]),
        I::Calendar => (vec![v(&[(4.0, 6.0), (20.0, 6.0), (20.0, 20.0), (4.0, 20.0), (4.0, 6.0)]), v(&[(4.0, 10.0), (20.0, 10.0)]), v(&[(8.0, 3.0), (8.0, 7.0)]), v(&[(16.0, 3.0), (16.0, 7.0)])], vec![]),
        I::Sparkle => (vec![v(&[(12.0, 3.0), (14.0, 10.0), (21.0, 12.0), (14.0, 14.0), (12.0, 21.0), (10.0, 14.0), (3.0, 12.0), (10.0, 10.0), (12.0, 3.0)])], vec![]),
        I::Wired => (vec![v(&[(8.0, 3.0), (16.0, 3.0), (16.0, 10.0), (8.0, 10.0), (8.0, 3.0)]), v(&[(12.0, 10.0), (12.0, 21.0)]), v(&[(10.0, 6.5), (10.0, 6.6)]), v(&[(14.0, 6.5), (14.0, 6.6)])], vec![]),
        I::Headphones => (vec![arcp(12.0, 13.0, 8.0, 180.0, 360.0), v(&[(4.0, 13.0), (4.0, 19.0), (8.0, 19.0), (8.0, 14.0), (4.0, 14.0)]), v(&[(20.0, 13.0), (20.0, 19.0), (16.0, 19.0), (16.0, 14.0), (20.0, 14.0)])], vec![]),
        I::Edit => (vec![v(&[(4.0, 20.0), (5.0, 15.0), (16.0, 4.0), (20.0, 8.0), (9.0, 19.0), (4.0, 20.0)]), v(&[(14.0, 6.0), (18.0, 10.0)])], vec![]),
        I::Image => (vec![v(&[(3.0, 5.0), (21.0, 5.0), (21.0, 19.0), (3.0, 19.0), (3.0, 5.0)]), v(&[(3.0, 17.0), (9.0, 11.0), (14.0, 16.0), (17.0, 13.0), (21.0, 17.0)])], vec![(16.0, 9.0, 1.5)]),
    }
}

struct Shape {
    i: I,
    c: Color,
    w: f32,
}

impl<M> canvas::Program<M> for Shape {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let k = bounds.width.min(bounds.height) / 24.0;
        let (ls, cs) = shape(self.i);
        let stroke = || Stroke::default().with_color(self.c).with_width(self.w * k.max(0.5)).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round);
        for l in ls {
            if l.len() < 2 {
                continue;
            }
            let path = Path::new(|bd| {
                bd.move_to(Point::new(l[0].0 * k, l[0].1 * k));
                for pt in &l[1..] {
                    bd.line_to(Point::new(pt.0 * k, pt.1 * k));
                }
            });
            f.stroke(&path, stroke());
        }
        for (x, y, r) in cs {
            let path = Path::circle(Point::new(x * k, y * k), r * k);
            if r < 2.0 {
                f.fill(&path, self.c);
            } else {
                f.stroke(&path, stroke());
            }
        }
        vec![f.into_geometry()]
    }
}

pub fn icon<'a, M: 'a>(i: I, c: Color, size: f32) -> E<'a, M> {
    match i {
        I::G(g) => glyph(g, c, size, 1.8),
        other => canvas::Canvas::new(Shape { i: other, c, w: 1.8 }).width(Length::Fixed(size)).height(Length::Fixed(size)).into(),
    }
}

pub fn ibox<'a, M: 'a>(p: &Pal, i: I, tone: Option<Color>) -> E<'a, M> {
    let s = sz(40.0);
    let (bg, fg) = match tone {
        Some(c) => (Pal::mix(p.panel, c, 0.16), c),
        None => (Pal::mix(p.panel, p.text, 0.06), p.text),
    };
    container(icon(i, fg, sz(18.0))).center_x(Length::Fixed(s)).center_y(Length::Fixed(s)).style(ui::boxed(bg, p.r(8.0), None)).into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ck {
    Ok,
    Warn,
    Bad,
    Off,
}
pub fn ck(kind: &str) -> Ck {
    match kind {
        "ok" => Ck::Ok,
        "warn" => Ck::Warn,
        "bad" => Ck::Bad,
        _ => Ck::Off,
    }
}
pub fn chip_of<'a, M: 'a>(p: &Pal, label: impl Into<String>, k: Ck) -> E<'a, M> {
    let c = match k {
        Ck::Ok => p.good,
        Ck::Warn => p.warn,
        Ck::Bad => p.danger,
        Ck::Off => p.muted,
    };
    chip(label.into(), Pal::mix(p.panel, c, 0.2), c)
}

/// A SetRow: icon, title, the line under it, a value, a chip, › for more, and what pressing it does.
pub struct R<'a, M> {
    pub icon: Option<I>,
    pub title: String,
    pub sub: String,
    pub value: String,
    pub chip: Option<(String, Ck)>,
    pub badge: String,
    pub chev: bool,
    pub danger: bool,
    pub inset: bool,
    pub on: Option<M>,
    pub control: Option<E<'a, M>>,
}

pub fn r<'a, M>(icon: &str, title: impl Into<String>) -> R<'a, M> {
    R { icon: if icon.is_empty() { None } else { Some(icon_named(icon)) }, title: title.into(), sub: String::new(), value: String::new(), chip: None, badge: String::new(), chev: false, danger: false, inset: false, on: None, control: None }
}

impl<'a, M: Clone + 'a> R<'a, M> {
    pub fn sub(mut self, s: impl Into<String>) -> Self {
        self.sub = s.into();
        self
    }
    pub fn value(mut self, s: impl Into<String>) -> Self {
        self.value = s.into();
        self
    }
    pub fn chip(mut self, s: impl Into<String>, k: Ck) -> Self {
        let s = s.into();
        if !s.is_empty() {
            self.chip = Some((s, k));
        }
        self
    }
    pub fn badge(mut self, s: &str) -> Self {
        self.badge = s.into();
        self
    }
    pub fn chev(mut self) -> Self {
        self.chev = true;
        self
    }
    pub fn danger(mut self, d: bool) -> Self {
        self.danger = d;
        self
    }
    pub fn inset(mut self, d: bool) -> Self {
        self.inset = d;
        self
    }
    pub fn on(mut self, m: M) -> Self {
        self.on = Some(m);
        self
    }
    pub fn on_maybe(mut self, m: Option<M>) -> Self {
        self.on = m;
        self
    }
    pub fn control(mut self, c: E<'a, M>) -> Self {
        self.control = Some(c);
        self
    }
    /// SetSwitch: the switch on the right; pressing the row flips it.
    pub fn switch(mut self, p: &Pal, checked: bool, m: Option<M>) -> Self {
        self.control = Some(switch(p, checked, None));
        self.on = m;
        self
    }
    pub fn el(self, p: &Pal) -> E<'a, M> {
        let p = *p;
        let tone = if self.danger { Some(p.danger) } else { None };
        let mut title_row = Row::new().spacing(8.0).align_y(iced::Center).push(tm(self.title, sz(15.0), if self.danger { p.danger } else { p.text }));
        if !self.badge.is_empty() {
            title_row = title_row.push(chip(self.badge.clone(), p.accent, p.ink));
        }
        let mut words = Column::new().spacing(3.0).push(title_row);
        if !self.sub.is_empty() {
            words = words.push(t(self.sub, sz(13.0), p.muted));
        }
        let mut line = Row::new().spacing(16.0).align_y(iced::Center);
        if let Some(i) = self.icon {
            line = line.push(ibox(&p, i, tone));
        } else if self.inset {
            line = line.push(Space::new().width(Length::Fixed(sz(40.0))));
        }
        line = line.push(words.width(Length::Fill));
        if !self.value.is_empty() {
            line = line.push(t1(self.value, sz(13.0), if self.on.is_some() && self.control.is_none() { p.accent } else { p.muted }));
        }
        if let Some((c, k)) = self.chip {
            line = line.push(chip_of(&p, c, k));
        }
        if let Some(c) = self.control {
            line = line.push(c);
        } else if self.chev {
            line = line.push(glyph(G::Chevron, p.muted, 14.0, 2.0));
        }
        let bg = if self.inset { Pal::mix(p.panel, p.bg, 0.35) } else { p.panel };
        let boxr = container(line)
            .padding(Padding { top: sz(12.0), right: 20.0, bottom: sz(12.0), left: if self.inset { 34.0 } else { 20.0 } })
            .width(Length::Fill)
            .style(ui::boxed(bg, p.r(10.0), None));
        match self.on {
            Some(m) => mouse_area(boxr).on_press(m).interaction(mouse::Interaction::Pointer).into(),
            None => boxr.into(),
        }
    }
}

/// Choices side by side that wrap when they don't fit (SetChoice's buttons).
pub fn choices<'a, M: Clone + 'a>(p: &Pal, items: Vec<(String, bool, Option<M>)>) -> E<'a, M> {
    let total: usize = items.iter().map(|x| x.0.chars().count() + 4).sum();
    if total <= 44 {
        return seg(p, items);
    }
    let p = *p;
    let mut r = Row::new().spacing(6.0);
    for (label, on, msg) in items {
        let fg = if on { p.ink } else if msg.is_some() { p.text } else { p.muted };
        let cell = container(t1(label, sz(13.0), fg).font(if on { ui::weight(iced::font::Weight::Semibold) } else { hearth_ui::app::INTER }))
            .padding(Padding { top: sz(6.0), right: 14.0, bottom: sz(6.0), left: 14.0 })
            .style(ui::boxed(if on { p.accent } else { Pal::mix(p.panel, p.bg, 0.4) }, p.r(6.0), None));
        r = r.push(match msg {
            Some(m) if !on => E::from(mouse_area(cell).on_press(m).interaction(mouse::Interaction::Pointer)),
            _ => cell.into(),
        });
    }
    r.wrap().vertical_spacing(6.0).into()
}

/// SetChoice: a row with its title and line, and the choices (on the right, or under it when they're long).
pub fn choice<'a, M: Clone + 'a>(p: &Pal, ic: &str, title: impl Into<String>, sub: impl Into<String>, opts: Vec<(String, bool, Option<M>)>) -> E<'a, M> {
    let p = *p;
    let title: String = title.into();
    let sub: String = sub.into();
    let total: usize = opts.iter().map(|x| x.0.chars().count() + 4).sum();
    let mut words = Column::new().spacing(3.0).push(tm(title, sz(15.0), p.text));
    if !sub.is_empty() {
        words = words.push(t(sub, sz(13.0), p.muted));
    }
    let lead: E<'a, M> = if ic.is_empty() { Space::new().width(Length::Fixed(0.0)).into() } else { ibox(&p, icon_named(ic), None) };
    let inner: E<'a, M> = if total <= 44 {
        row![lead, words.width(Length::Fill), choices(&p, opts)].spacing(16.0).align_y(iced::Center).into()
    } else {
        column![row![lead, words.width(Length::Fill)].spacing(16.0).align_y(iced::Center), choices(&p, opts)].spacing(10.0).into()
    };
    container(inner).padding(Padding { top: sz(12.0), right: 20.0, bottom: sz(12.0), left: 20.0 }).width(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), None)).into()
}

/// The options of a SetChoice from (value, label, disabled), `cur` picked, `f` what picking one sends.
pub fn opts<M: Clone>(list: &[(&str, &str, bool)], cur: &str, f: impl Fn(&str) -> M) -> Vec<(String, bool, Option<M>)> {
    list.iter().map(|(v, l, dis)| (l.to_string(), *v == cur, if *dis { None } else { Some(f(v)) })).collect()
}

/// Plain words on the page (the intro under the title, a note).
pub fn words<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    container(t(s.into(), sz(13.0), p.muted)).padding(Padding { top: 2.0, right: 4.0, bottom: 6.0, left: 4.0 }).width(Length::Fill).into()
}
pub fn words_c<'a, M: 'a>(s: impl Into<String>, c: Color) -> E<'a, M> {
    container(t(s.into(), sz(13.0), c)).padding(Padding { top: 2.0, right: 4.0, bottom: 6.0, left: 4.0 }).width(Length::Fill).into()
}
pub fn step_title<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    container(ts(s.into(), sz(14.0), p.text)).padding(Padding { top: 6.0, right: 4.0, bottom: 2.0, left: 4.0 }).into()
}
/// The page's message at the bottom (the QML pages' `msg`): in the accent, red when it went wrong.
pub fn message<'a, M: 'a>(p: &Pal, st: &St) -> E<'a, M> {
    if st.msg.is_empty() {
        return Space::new().height(Length::Fixed(0.0)).into();
    }
    note(p, st.msg.clone(), st.bad)
}

/// SetBanner: something needs a look, and the button that goes there.
pub fn banner<'a, M: Clone + 'a>(p: &Pal, kind: &str, ic: &str, title: impl Into<String>, sub: impl Into<String>, action: &str, m: Option<M>) -> E<'a, M> {
    let p = *p;
    let tone = match kind {
        "bad" => p.danger,
        "info" => p.accent,
        _ => p.warn,
    };
    let mut r = row![ibox(&p, icon_named(ic), Some(tone)), column![ts(title.into(), sz(15.0), p.text), t(sub.into(), sz(13.0), p.muted)].spacing(3.0).width(Length::Fill)].spacing(16.0).align_y(iced::Center);
    if !action.is_empty() {
        r = r.push(ui::btn(&p, action.to_string(), ui::Kind::Primary, false, m.clone()));
    }
    let c = container(r).padding(Padding { top: sz(14.0), right: 20.0, bottom: sz(14.0), left: 20.0 }).width(Length::Fill).style(ui::boxed(Pal::mix(p.panel, tone, 0.12), p.r(10.0), Some((Pal::a(tone, 0.5), 1.0))));
    match m {
        Some(x) => mouse_area(c).on_press(x).interaction(mouse::Interaction::Pointer).into(),
        None => c.into(),
    }
}

/// SetField: a text box; Enter sends Submit(key).
pub fn tfield<'a>(p: &Pal, st: &St, key: &str, placeholder: impl Into<String>, secret: bool) -> E<'a, Msg> {
    let k = key.to_string();
    let k2 = key.to_string();
    let ph: String = placeholder.into();
    let f = field(p, &ph, &st.txt(key), move |v| Msg::Text(k.clone(), v)).on_submit(Msg::Submit(k2)).secure(secret);
    container(f).padding(Padding { top: 2.0, right: 0.0, bottom: 2.0, left: 0.0 }).width(Length::Fill).into()
}
/// An indented one (under the row it belongs to).
pub fn tfield_in<'a>(p: &Pal, st: &St, key: &str, placeholder: impl Into<String>, secret: bool) -> E<'a, Msg> {
    container(tfield(p, st, key, placeholder, secret)).padding(Padding { top: 4.0, right: 18.0, bottom: 4.0, left: 56.0 }).into()
}

/// SetExpander: a row that opens the rows under it (`open` in the local `key`).
pub fn expander<'a>(p: &Pal, st: &St, key: &str, ic: &str, title: impl Into<String>, sub: impl Into<String>, value: &str, chip: Option<(String, Ck)>, inner: Vec<E<'a, Msg>>) -> E<'a, Msg> {
    let open = st.lb(key);
    let mut head_r = r(ic, title).sub(sub).value(value).on(lv(key, Value::Bool(!open))).control(glyph(if open { G::Down } else { G::Chevron }, p.muted, 14.0, 2.0));
    if let Some((c, k)) = chip {
        head_r = head_r.chip(c, k);
    }
    let mut c = Column::new().spacing(3.0).push(head_r.el(p));
    if open {
        let mut body = Column::new().spacing(3.0);
        for e in inner {
            body = body.push(e);
        }
        c = c.push(container(body).padding(Padding { top: 0.0, right: 0.0, bottom: 4.0, left: 28.0 }));
    }
    c.into()
}

/// SetCardGroup: a heading, then its rows close together.
pub fn group<'a, M: 'a>(p: &Pal, title: &str, rows: Vec<E<'a, M>>) -> E<'a, M> {
    let mut c = Column::new().spacing(3.0);
    if !title.is_empty() {
        c = c.push(head(p, title.to_string()));
    }
    for e in rows {
        c = c.push(e);
    }
    c.into()
}

/// SetLinkCard: a row that opens another page, its name and line from the page list.
pub fn link_card<'a>(p: &Pal, target: &'static str, value: &str, sub: Option<String>, chip: Option<(String, Ck)>) -> E<'a, Msg> {
    let pg = crate::registry::page(target);
    let label = pg.map(|x| x.label).unwrap_or(target);
    let desc = sub.unwrap_or_else(|| pg.map(|x| x.desc.to_string()).unwrap_or_default());
    let ic = match target {
        "hs-shares" => "folder",
        "hs-files" => "link",
        "hs-people" => "users",
        "hs-family" => "family",
        "hs-services" => "box",
        "hs-apps" => "box",
        "hs-server" => "server",
        "hs-gaming" => "pad",
        "hs-aidevices" => "desktops",
        "hs-drives" => "drive",
        "hs-backups" => "cloud",
        "hs-safety" => "shield",
        "hs-phone" => "bell",
        "hs-alerts" => "bell",
        "hs-tailscale" => "globe",
        "remote" => "play",
        _ => "gear",
    };
    let mut row_ = r(ic, label).sub(desc).value(value).chev().on(Msg::Go(target));
    if let Some((c, k)) = chip {
        row_ = row_.chip(c, k);
    }
    row_.el(p)
}

/// A Column of elements with the rows' spacing.
pub fn col<'a, M: 'a>(items: Vec<E<'a, M>>) -> E<'a, M> {
    let mut c = Column::new().spacing(6.0);
    for e in items {
        c = c.push(e);
    }
    c.into()
}

/// A bar (how full).
pub fn meter<'a, M: 'a>(p: &Pal, frac: f64, warn: bool) -> E<'a, M> {
    let f = frac.clamp(0.0, 1.0) as f32;
    ui::bar(vec![(f, if warn { p.warn } else { p.accent })], Pal::mix(p.panel2, p.text, 0.08), 8.0, 4.0)
}

/// The rows the page shows (builders push into it; `None` rows are left out, as QML's `visible: false`).
pub struct Rows<'a, M> {
    pub v: Vec<E<'a, M>>,
}
impl<'a, M: 'a> Rows<'a, M> {
    pub fn new() -> Self {
        Rows { v: Vec::new() }
    }
    pub fn add(&mut self, e: E<'a, M>) {
        self.v.push(e);
    }
    pub fn when(&mut self, cond: bool, f: impl FnOnce() -> E<'a, M>) {
        if cond {
            self.v.push(f());
        }
    }
    pub fn el(self) -> E<'a, M> {
        let mut c = Column::new().spacing(6.0);
        for e in self.v {
            c = c.push(e);
        }
        c.into()
    }
}

/// A heading (SetHeader).
pub fn h<'a, M: 'a>(p: &Pal, s: impl Into<String>) -> E<'a, M> {
    head(p, s.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn values_and_templates() {
        let v: Value = serde_json::from_str(r#"{"a":{"b":[{"c":"x"}],"n":3,"f":false},"share":{"name":"Photos"}}"#).unwrap();
        assert_eq!(s(&v, "a.b.0.c"), "x");
        assert_eq!(n(&v, "a.n"), 3.0);
        assert!(!b(&v, "a.f") && !nf(&v, "a.f") && nf(&v, "a.missing"));
        assert_eq!(fill("Shared ✓ {share.name}{nothing|, ok}", &v), "Shared ✓ Photos, ok");
        assert_eq!(size(301e9), "301 GB");
        assert_eq!(size(1.6e12), "1.6 TB");
        let o = run::Output { ok: false, code: 1, out: "note\nerror: Use 8 characters\n".into(), err: String::new() };
        assert_eq!(why(&o), "Use 8 characters");
        assert_eq!(s(&answer(&o), "error"), "Use 8 characters");
    }
}

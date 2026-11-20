//! The Briefcase in Hearth Files (docs/HEARTH-FILES.md › Briefcase; Mantel's Briefcase.qml and BriefcaseDialog.qml):
//! every file change runs in hearth-briefcase (desktop/shell, beside hearth-files), the way the rest of Files runs
//! hearth-files; this keeps what Files shows (each item's status, the lock, auto-sync) and draws the dialogs (Update
//! All, passwords, Customize, choosing a Briefcase, a job's progress). Passwords go to the helper on its stdin, never on
//! its command line or in its environment, and are forgotten as soon as they're sent.
//!
//! Light: a Briefcase's status is asked for when it's opened, when something in it changes (the folder's own watch),
//! after each change made here, and once a minute only while an unlocked, locked Briefcase is shown (its "Locks in"
//! countdown). Nothing else runs.

use crate::app::*;
use crate::icons::{ico, Ic};
use crate::model::{self, base, parent_of, Entry};
use futures::channel::mpsc;
use hearth_ui::ui::{self, gap, t, t1, tb};
use hearth_ui::Pal;
use iced::widget::{button, column, container, mouse_area, row, scrollable, text_input, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Shadow, Task, Theme};
use serde_json::{json, Map, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn sv(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}
fn bv(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}

pub fn helper() -> String {
    std::env::var("HEARTH_BRIEFCASE_HELPER").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| hearth_ui::run::shell_helper("hearth-briefcase"))
}

// ------------------------------------------------------------------ the helper
fn fail(e: &str) -> Value {
    json!({ "ok": false, "error": e, "code": "error" })
}

/// The answer in what it printed: its last JSON line with "ok".
fn answer(out: &str) -> Value {
    for l in out.lines().rev() {
        let l = l.trim();
        if l.starts_with('{') {
            if let Ok(v) = serde_json::from_str::<Value>(l) {
                if v.get("ok").is_some() {
                    return v;
                }
            }
        }
    }
    fail("Something went wrong")
}

fn spawn(args: &[String], input: &str) -> std::io::Result<std::process::Child> {
    let mut c = Command::new("python3");
    c.arg(helper()).args(args).stdout(Stdio::piped()).stderr(Stdio::null()).stdin(if input.is_empty() { Stdio::null() } else { Stdio::piped() });
    let mut ch = c.spawn()?;
    if !input.is_empty() {
        if let Some(mut w) = ch.stdin.take() {
            let _ = w.write_all(input.as_bytes());
        }
    }
    Ok(ch)
}

/// hearth-briefcase ARGS (with INPUT, passwords, on its stdin): its answer, at most two minutes.
pub fn call(args: Vec<String>, input: String) -> impl std::future::Future<Output = Value> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-briefcase".into()).stack_size(256 * 1024).spawn(move || {
        let v = match spawn(&args, &input) {
            Err(e) => fail(&format!("hearth-briefcase: {e}")),
            Ok(mut ch) => {
                let mut out = ch.stdout.take();
                let reader = std::thread::spawn(move || {
                    let mut s = String::new();
                    if let Some(o) = out.as_mut() {
                        let _ = o.read_to_string(&mut s);
                    }
                    s
                });
                let start = Instant::now();
                loop {
                    match ch.try_wait() {
                        Ok(Some(_)) | Err(_) => break,
                        Ok(None) => {}
                    }
                    if start.elapsed() > Duration::from_secs(120) {
                        let _ = ch.kill();
                        let _ = ch.wait();
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(if start.elapsed() < Duration::from_millis(300) { 5 } else { 40 }));
                }
                answer(&reader.join().unwrap_or_default())
            }
        };
        let _ = tx.send(v);
    });
    async move { rx.await.unwrap_or_else(|_| fail("It stopped")) }
}

#[derive(Debug, Clone)]
pub enum BLine {
    Pid(u64, i32),
    Progress(u64, Value),
    Done(u64, Value),
}

/// A job with progress (Update, adding, sending, locking): its lines as they come; SIGTERM (Stop) ends it cleanly.
fn job_stream(seq: u64, args: Vec<String>, input: String) -> mpsc::UnboundedReceiver<BLine> {
    let (tx, rx) = mpsc::unbounded();
    let _ = std::thread::Builder::new().name("hearth-briefcase-job".into()).stack_size(256 * 1024).spawn(move || {
        let mut ch = match spawn(&args, &input) {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.unbounded_send(BLine::Done(seq, fail(&format!("hearth-briefcase: {e}"))));
                return;
            }
        };
        let _ = tx.unbounded_send(BLine::Pid(seq, ch.id() as i32));
        let mut fin = Value::Null;
        if let Some(out) = ch.stdout.take() {
            for l in BufReader::new(out).lines() {
                let Ok(l) = l else { break };
                if let Some(p) = l.strip_prefix("progress: ") {
                    if let Ok(v) = serde_json::from_str::<Value>(p) {
                        let _ = tx.unbounded_send(BLine::Progress(seq, v));
                    }
                } else if l.trim_start().starts_with('{') && fin.is_null() {
                    if let Ok(v) = serde_json::from_str::<Value>(l.trim()) {
                        if v.get("ok").is_some() {
                            fin = v;
                        }
                    }
                }
            }
        }
        let _ = ch.wait();
        let _ = tx.unbounded_send(BLine::Done(seq, if fin.is_null() { fail("Something went wrong") } else { fin }));
    });
    rx
}

// ------------------------------------------------------------------ state
#[derive(Debug, Clone, Default)]
pub struct Pw {
    pub a: String,
    pub b: String,
    pub c: String,
    pub remember: bool,
    pub error: String,
    pub path: String,
    pub name: String,
    pub then: Option<Box<BMsg>>,
}

#[derive(Debug, Clone)]
pub struct Cust {
    pub path: String,
    pub locked: bool,
    pub orig: Value,
    pub look: Value,
    pub sync_custom: bool,
    pub lock_custom: bool,
}

#[derive(Debug, Clone)]
pub struct PickRow {
    pub label: String,
    pub sub: String,
    pub look: Value,
    pub locked: bool,
    pub raw: Value,
}

/// What choosing a row does.
#[derive(Debug, Clone)]
pub enum PickThen {
    AddTo(Vec<String>),
    Transfer(String, Vec<String>, bool),
    SwapIn(String, String),
    SwapWith(String, String, String),
    Send(String),
    NewAt(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct BPick {
    pub title: String,
    pub list: Vec<PickRow>,
    pub then: PickThen,
}

/// What happens when a job ends.
#[derive(Debug, Clone)]
pub enum JobThen {
    Update,
    Add(String, Vec<String>),
    Transfer(String, String, Vec<String>, bool),
    Swap(String, String),
    LockRemove(String),
    Sync,
    Send(String),
    Lock(String, String),
}

#[derive(Debug, Clone)]
pub struct BJob {
    pub verb: String,
    pub done: f64,
    pub total: f64,
    pub file: String,
    pub cancel: bool,
    pub seq: u64,
    pub pid: i32,
    pub then: JobThen,
}

#[derive(Debug, Clone, Default)]
pub struct Bc {
    pub known: Vec<Value>,
    pub st: Option<Value>,
    pub st_at: Option<Instant>,
    /// "" | update | password | setlock | passwd | customize | pick | job | message
    pub mode: String,
    pub row: usize,
    pub plan: Option<Value>,
    pub pw: Pw,
    pub cust: Option<Cust>,
    pub pick: Option<BPick>,
    pub job: Option<BJob>,
    pub msg: Option<(String, String)>,
    pub seq: u64,
    /// When this folder's status was last asked for (opening it, or it changed).
    pub asked: Option<(String, Instant)>,
}

/// What an answer from hearth-briefcase leads to.
#[derive(Debug, Clone)]
pub enum BThen {
    Known,
    Status(String, bool),
    Say(String),
    Find,
    Split,
    Lock(String),
    Forget,
    LockAsNew(String),
    LockAsAdded(String),
    MadeNew,
    NewFor(Vec<String>),
    Keyring(String),
    Unlocked,
    Passwd,
    Plan(String, Vec<String>),
    CustOpen,
    CustSaved(String),
    SwapList(String, String, String, String),
    ResetList,
}

#[derive(Debug, Clone)]
pub enum BMsg {
    Run(String, Vec<String>),
    Did(BThen, Value),
    Line(BLine),
    Tick,
    // the dialogs
    Row(usize),
    Press(usize, i32),
    Field(&'static str, String),
    Ok,
    Cancel,
}

// ------------------------------------------------------------------ what Files shows
pub fn in_briefcase(a: &App) -> bool {
    a.listing.as_ref().map(|l| l.briefcase.is_object()).unwrap_or(false) && !a.on_home()
}
fn st(a: &App) -> &Value {
    a.bc.st.as_ref().unwrap_or(&Value::Null)
}
pub fn locked(a: &App) -> bool {
    bv(st(a), "locked")
}
pub fn unlocked(a: &App) -> bool {
    bv(st(a), "unlocked")
}
fn bc_name(a: &App) -> String {
    sv(st(a), "name")
}

pub fn refresh_known() -> Task<Msg> {
    Task::perform(call(vec!["list".into()], String::new()), |v| Msg::Bc(BMsg::Did(BThen::Known, v)))
}

pub fn refresh(a: &mut App) -> Task<Msg> {
    if !in_briefcase(a) {
        a.bc.st = None;
        return Task::none();
    }
    let at = a.path.clone();
    Task::perform(call(vec!["status".into(), at.clone()], String::new()), move |v| Msg::Bc(BMsg::Did(BThen::Status(at.clone(), false), v)))
}

/// A folder was opened: a Briefcase's status; a locked one asks for its password; an unlocked one opens what's inside.
pub fn opened(a: &mut App) -> Task<Msg> {
    if !in_briefcase(a) {
        a.bc.st = None;
        a.bc.asked = None;
        return Task::none();
    }
    let at = a.path.clone();
    // The folder updates itself when something in it changes: its status is asked again at most every few seconds
    // (never a loop of asking, whatever the helper writes).
    if let Some((p, t)) = &a.bc.asked {
        if *p == at && t.elapsed() < Duration::from_secs(3) {
            return Task::none();
        }
    }
    a.bc.asked = Some((at.clone(), Instant::now()));
    Task::perform(call(vec!["status".into(), at.clone()], String::new()), move |v| Msg::Bc(BMsg::Did(BThen::Status(at.clone(), true), v)))
}

pub fn item_of<'a>(a: &'a App, e: &Entry) -> Option<&'a Value> {
    st(a).get("items").and_then(Value::as_array)?.iter().find(|i| sv(i, "path") == e.path || sv(i, "name") == e.name)
}

pub fn status_label(a: &App, e: &Entry) -> String {
    match item_of(a, e) {
        None => if in_briefcase(a) && !e.name.starts_with('.') { "Not synced".into() } else { String::new() },
        Some(it) => match sv(it, "status").as_str() {
            "ok" => "Up-to-date".into(),
            "newer-b" => "Newer in Briefcase".into(),
            "newer-o" => "Newer at original".into(),
            "both" => "Changes on both sides".into(),
            "conflict" => "Changed in both".into(),
            _ => sv(it, "label"),
        },
    }
}

/// A small badge on an item's icon: the colour of its status, and a glyph.
pub fn badge_for(a: &App, e: &Entry) -> Option<(Color, &'static str)> {
    let it = item_of(a, e)?;
    let p = &a.p;
    Some(match sv(it, "status").as_str() {
        "ok" => (Color::from_rgb8(0x5f, 0xb9, 0x8f), "✓"),
        "newer-b" => (p.accent, "→"),
        "newer-o" => (p.accent, "←"),
        "both" => (p.accent, "⇄"),
        "conflict" => (p.danger, "!"),
        "away" => (p.muted, "…"),
        "orphan" => (Color::from_rgb8(0x7d, 0x6a, 0x5c), "×"),
        _ => (p.muted, "?"),
    })
}

pub fn color_of(a: &App, look: &Value) -> Color {
    let c = sv(look, "color");
    match c.as_str() {
        "" | "accent" => a.p.accent,
        "ember" => Color::from_rgb8(0xff, 0x8a, 0x3d),
        "red" => Color::from_rgb8(0xe0, 0x66, 0x66),
        "green" => Color::from_rgb8(0x5f, 0xb9, 0x8f),
        "blue" => Color::from_rgb8(0x4f, 0x9d, 0xde),
        "purple" => Color::from_rgb8(0xb3, 0x6a, 0xd9),
        "gold" => Color::from_rgb8(0xe0, 0xb2, 0x4f),
        "slate" => Color::from_rgb8(0x8a, 0x9a, 0xa8),
        x => hearth_ui::theme::hex(x).filter(|_| x.len() == 7).unwrap_or(a.p.accent),
    }
}

pub fn icon_of(look: &Value) -> Ic {
    match sv(look, "icon").as_str() {
        "satchel" => Ic::BcSatchel,
        "suitcase" => Ic::BcSuitcase,
        "backpack" => Ic::BcBackpack,
        "box" => Ic::BcBox,
        _ => Ic::BcBriefcase,
    }
}

/// A Briefcase's icon: its style in its colour, its badge and a padlock when it's locked.
pub fn bc_icon<'a>(a: &App, look: &Value, locked: bool, size: f32) -> Element<'a, Msg> {
    let p = a.p;
    let mut layers: Vec<Element<'a, Msg>> = vec![ico(icon_of(look), color_of(a, look), size)];
    let badge = sv(look, "badge");
    if !badge.is_empty() {
        layers.push(container(t1(badge, (size * 0.3).max(9.0), p.text).font(ui::weight(iced::font::Weight::Bold))).align_right(Length::Fixed(size)).align_top(Length::Fixed(size)).into());
    }
    if locked {
        layers.push(container(ico::<Msg>(Ic::Lock, p.text, (size * 0.4).max(10.0))).align_right(Length::Fixed(size)).align_bottom(Length::Fixed(size)).into());
    }
    iced::widget::Stack::with_children(layers).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
}

fn ago(t: i64) -> String {
    if t <= 0 {
        return String::new();
    }
    let s = (hearth_ui::time::now() - t).max(0);
    if s < 60 { "just now".into() } else if s < 3600 { format!("{} min ago", (s as f64 / 60.0).round()) } else if s < 86400 { format!("{} h ago", (s as f64 / 3600.0).round()) } else { format!("{} days ago", (s as f64 / 86400.0).round()) }
}
fn plural(n: u64, w: &str) -> String {
    format!("{n} {w}{}", if n == 1 { "" } else { "s" })
}

/// "Syncing…" / "Synced 2 min ago" / "2 files need you", for the details pane and the status bar.
pub fn sync_line(a: &App) -> String {
    let s = st(a);
    let Some(sy) = s.get("sync").filter(|x| x.is_object()) else { return String::new() };
    let set = sy.get("settings").cloned().unwrap_or(Value::Null);
    let off = sv(&set, "mode") == "off";
    if bv(s, "locked") && !bv(s, "unlocked") {
        return if off { String::new() } else { "Auto-sync waits until it's unlocked".into() };
    }
    if bv(sy, "syncing") {
        return "Syncing…".into();
    }
    let needs = sy.get("needs").and_then(Value::as_u64).unwrap_or(0);
    if needs > 0 {
        return format!("{} need{} you", plural(needs, "file"), if needs == 1 { "s" } else { "" });
    }
    if off {
        return "Auto-sync off · Update All to sync".into();
    }
    let last = sy.get("last").and_then(Value::as_f64).unwrap_or(0.0) as i64;
    if bv(sy, "away") && last == 0 {
        return "Waiting for originals to come back".into();
    }
    if last > 0 { format!("Synced {}", ago(last)) } else { "Auto-sync on".into() }
}

pub fn lock_line(a: &App) -> String {
    let s = st(a);
    if !bv(s, "locked") {
        return String::new();
    }
    if !bv(s, "unlocked") {
        return "Locked".into();
    }
    match sv(s, "lockState").as_str() {
        "never" => return "Unlocked · locks only when you lock it".into(),
        "open" => return "Unlocked · a file in it is open".into(),
        _ => {}
    }
    let Some(lock_in) = s.get("lockIn").and_then(Value::as_f64) else { return "Unlocked".into() };
    let gone = a.bc.st_at.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
    let m = ((lock_in - gone) / 60.0).round().max(1.0);
    format!("Unlocked · Locks in {}", if m >= 60.0 { format!("{} h", (m / 6.0).round() / 10.0) } else { format!("{m} min") })
}

/// The details pane: the Briefcase's note, lock and sync; an item's status and where its original is.
pub fn detail_rows(a: &App, e: Option<&Entry>) -> Vec<(String, String)> {
    if !in_briefcase(a) || a.bc.st.is_none() {
        return vec![];
    }
    let s = st(a);
    let Some(e) = e else {
        let mut r = vec![];
        let note = s.get("look").map(|l| sv(l, "note")).unwrap_or_default();
        if !note.is_empty() {
            r.push(("Note".into(), note));
        }
        let ll = lock_line(a);
        if !ll.is_empty() {
            r.push(("Lock".into(), ll));
        }
        let sl = sync_line(a);
        if !sl.is_empty() {
            r.push(("Sync".into(), sl));
        }
        let n = s.get("items").and_then(Value::as_array).map(|x| x.len()).unwrap_or(0) as u64;
        let ok = s.get("counts").and_then(|c| c.get("ok")).and_then(Value::as_u64).unwrap_or(0);
        if n > 0 {
            r.push(("Items".into(), format!("{n} · {ok} up-to-date{}", if n > ok { format!(" · {} need a look", n - ok) } else { String::new() })));
        }
        if bv(s, "damaged") {
            r.push(("List".into(), "Damaged: kept aside (Start the list again)".into()));
        }
        return r;
    };
    match item_of(a, e) {
        None => if e.name.starts_with('.') { vec![] } else { vec![("Briefcase".into(), "Not linked to an original".into())] },
        Some(it) => vec![("Status".into(), sv(it, "label")), ("Original".into(), sv(it, "origin"))],
    }
}

// ------------------------------------------------------------------ menus
fn items_in(a: &App, t: &[String]) -> Vec<String> {
    let items = st(a).get("items").and_then(Value::as_array).cloned().unwrap_or_default();
    t.iter().filter(|p| items.iter().any(|i| sv(i, "path") == **p)).cloned().collect()
}
fn others(a: &App) -> Vec<Value> {
    let here = sv(st(a), "path");
    a.bc.known.iter().filter(|k| !bv(k, "missing") && (a.bc.st.is_none() || sv(k, "path") != here)).cloned().collect()
}

pub fn menu_items(a: &App, e: Option<&Entry>, n: usize, t: &[String]) -> Vec<MenuItem> {
    let mut out = vec![];
    if a.picking() || a.in_trash() || (a.on_home() && e.is_none()) || a.on_special() {
        return out;
    }
    let head = |out: &mut Vec<MenuItem>| {
        if out.is_empty() {
            out.push(MenuItem::sep());
            out.push(MenuItem::heading("BRIEFCASE"));
        }
    };
    if in_briefcase(a) {
        let mine = items_in(a, t);
        if e.is_some() && !mine.is_empty() {
            head(&mut out);
            out.push(MenuItem::new("bc-update-sel", &if mine.len() > 1 { format!("Update {} items", mine.len()) } else { "Update".into() }).icon(Ic::Refresh));
            if mine.len() == 1 {
                out.push(MenuItem::new("bc-find", "Find original").icon(Ic::Folder));
            }
            out.push(MenuItem::new("bc-split", "Split from original").icon(Ic::Link));
            if !others(a).is_empty() {
                out.push(MenuItem::new("bc-moveto", "Move to Briefcase  ›").icon(Ic::BcBriefcase));
                out.push(MenuItem::new("bc-copyto", "Copy to Briefcase  ›").icon(Ic::BcBriefcase));
                if mine.len() == 1 {
                    out.push(MenuItem::new("bc-swap", "Swap with…").icon(Ic::BcBriefcase));
                }
            }
        }
        if e.is_none() {
            head(&mut out);
            out.extend(folder_items(a));
        }
        return out;
    }
    if let Some(e) = e {
        if e.briefcase.is_object() && n == 1 {
            head(&mut out);
            let p = e.path.clone();
            out.push(MenuItem::new(&format!("bc-update-all:{p}"), "Update All").icon(Ic::Refresh));
            out.push(if bv(&e.briefcase, "locked") { MenuItem::new(&format!("bc-unlock:{p}"), "Unlock…").icon(Ic::Lock) } else { MenuItem::new(&format!("bc-setlock:{p}"), "Lock this Briefcase…").icon(Ic::Lock) });
            out.push(MenuItem::new(&format!("bc-customize:{p}"), "Customize…").icon(icon_of(&e.briefcase)));
            out.push(MenuItem::new(&format!("bc-send:{p}"), "Send Briefcase to…").icon(Ic::Share));
            return out;
        }
    }
    if n > 0 {
        head(&mut out);
        out.push(MenuItem::new("bc-addto", "Add to Briefcase  ›").icon(Ic::BcBriefcase));
    }
    out
}

fn folder_items(a: &App) -> Vec<MenuItem> {
    let s = st(a);
    let mut out = vec![MenuItem::new("bc-update-all", "Update All").icon(Ic::Refresh)];
    if bv(s, "locked") {
        if bv(s, "unlocked") {
            out.push(MenuItem::new("bc-lock", "Lock").icon(Ic::Lock));
            out.push(MenuItem::new("bc-passwd", "Change password…").icon(Ic::Lock));
            out.push(MenuItem::new("bc-unlockremove", "Remove lock…").icon(Ic::Lock));
        } else {
            out.push(MenuItem::new("bc-unlock", "Unlock…").icon(Ic::Lock));
        }
        out.push(MenuItem::new("bc-forget", "Forget remembered password").icon(Ic::Close));
    } else {
        out.push(MenuItem::new("bc-setlock", "Lock this Briefcase…").icon(Ic::Lock));
    }
    out.push(MenuItem::new("bc-customize", "Customize…").icon(icon_of(s.get("look").unwrap_or(&Value::Null))));
    out.push(MenuItem::new("bc-syncnow", "Sync now").icon(Ic::Refresh));
    out.push(MenuItem::new("bc-send", "Send Briefcase to…").icon(Ic::Share));
    if bv(s, "damaged") {
        out.push(MenuItem::new("bc-resetlist", "Start the list again…").icon(Ic::Undo).danger());
    }
    out
}

/// The command bar's Briefcase buttons (inside a Briefcase): Update All, Unlock / Lock, Customize.
pub fn tool_shown(a: &App, t: &str) -> bool {
    if !in_briefcase(a) || a.picking() {
        return false;
    }
    match t {
        "bc-unlock" => locked(a) && !unlocked(a),
        "bc-lock" => !locked(a) || unlocked(a),
        _ => true,
    }
}

pub fn command(a: &mut App, t: &str) -> Task<Msg> {
    if t == "bc-lock" && !locked(a) {
        return run(a, "bc-setlock", vec![]);
    }
    run(a, t, vec![])
}

fn done(a: &mut App, r: &Value, ok_text: &str) -> Option<Task<Msg>> {
    if !bv(r, "ok") {
        let e = sv(r, "error");
        return Some(a.say(if e.is_empty() { "Something went wrong" } else { &e }, true));
    }
    if !ok_text.is_empty() {
        return Some(a.say(ok_text, false));
    }
    None
}
fn bc_call(args: Vec<String>, input: String, then: BThen) -> Task<Msg> {
    Task::perform(call(args, input), move |v| Msg::Bc(BMsg::Did(then.clone(), v)))
}
fn s_(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

fn run_job(a: &mut App, verb: &str, args: Vec<String>, input: String, then: JobThen) -> Task<Msg> {
    a.bc.seq += 1;
    let seq = a.bc.seq;
    a.bc.job = Some(BJob { verb: verb.into(), done: 0.0, total: 0.0, file: String::new(), cancel: false, seq, pid: 0, then });
    a.bc.mode = "job".into();
    a.bc.row = 0;
    a.overlay = Some(Overlay::Bc);
    Task::run(job_stream(seq, args, input), |l| Msg::Bc(BMsg::Line(l)))
}

fn cancel_job(a: &mut App) {
    if let Some(j) = a.bc.job.as_mut() {
        j.cancel = true;
        if j.pid > 0 {
            unsafe {
                libc::kill(j.pid, libc::SIGTERM);
            }
        }
    }
}

pub fn close(a: &mut App) {
    a.bc.mode.clear();
    a.bc.plan = None;
    a.bc.pick = None;
    a.bc.cust = None;
    a.bc.msg = None;
    a.bc.pw = Pw::default();
    if matches!(a.overlay, Some(Overlay::Bc)) {
        a.overlay = None;
    }
}

fn open_dialog(a: &mut App, mode: &str) {
    a.bc.mode = mode.into();
    a.bc.row = 0;
    a.overlay = Some(Overlay::Bc);
}

fn choose(a: &mut App, title: &str, list: Vec<Value>, then: PickThen) -> Task<Msg> {
    if list.is_empty() {
        return a.say("There's no other Briefcase yet. Make one with New › Briefcase", false);
    }
    let rows = list
        .into_iter()
        .map(|k| {
            let look = k.get("look").cloned().unwrap_or(Value::Null);
            let lname = sv(&look, "name");
            let label = if !lname.is_empty() { lname } else if !sv(&k, "name").is_empty() { sv(&k, "name") } else { base(&sv(&k, "path")) };
            let sub = if !sv(&k, "sub").is_empty() { sv(&k, "sub") } else if bv(&k, "locked") { if bv(&k, "unlocked") { "Unlocked".into() } else { "Locked".into() } } else { parent_of(&sv(&k, "path")) };
            let locked = bv(&k, "locked") && !bv(&k, "unlocked");
            PickRow { label, sub, look, locked, raw: k }
        })
        .collect();
    a.bc.pick = Some(BPick { title: title.into(), list: rows, then });
    open_dialog(a, "pick");
    Task::none()
}

fn ask_password(a: &mut App, path: String, name: String, then: Option<BMsg>) -> Task<Msg> {
    let nm = if name.is_empty() { base(&path) } else { name };
    a.bc.pw = Pw { path: path.clone(), name: nm, then: then.map(Box::new), ..Default::default() };
    open_dialog(a, "password");
    // A password this device remembers (only if you asked it to): tried first, quietly.
    bc_call(vec!["unlock".into(), path.clone(), "--keyring".into()], String::new(), BThen::Keyring(path))
}

fn open_set_lock(a: &mut App, path: String) -> Task<Msg> {
    let name = if a.bc.st.is_some() && path == sv(st(a), "path") { bc_name(a) } else { base(&path) };
    a.bc.pw = Pw { path, name, ..Default::default() };
    open_dialog(a, "setlock");
    a.bc.row = 0;
    iced::widget::operation::focus(PW_A)
}

fn open_update(path: String, names: Vec<String>) -> Task<Msg> {
    let mut args = vec!["plan".to_string(), path.clone()];
    args.extend(names.iter().cloned());
    bc_call(args, String::new(), BThen::Plan(path, names))
}

fn add_to(a: &mut App, path: String, paths: Vec<String>) -> Task<Msg> {
    if paths.is_empty() {
        return Task::none();
    }
    let mut args = vec!["add".to_string(), path.clone()];
    args.extend(paths.iter().cloned());
    run_job(a, "add", args, String::new(), JobThen::Add(path, paths))
}

fn transfer(a: &mut App, from: String, to: String, names: Vec<String>, mv: bool) -> Task<Msg> {
    let mut args = vec!["transfer".to_string(), from.clone(), to.clone()];
    args.extend(names.iter().cloned());
    args.push(if mv { "--move" } else { "--copy" }.into());
    run_job(a, if mv { "move" } else { "copy" }, args, String::new(), JobThen::Transfer(from, to, names, mv))
}

/// Everything the menus and buttons do. T: what's picked (paths). An id may carry the Briefcase: "bc-unlock:/path".
pub fn run(a: &mut App, id: &str, t: Vec<String>) -> Task<Msg> {
    let (verb, arg) = match id.find(':') {
        Some(i) => (&id[..i], id[i + 1..].to_string()),
        None => (id, String::new()),
    };
    let here = if a.bc.st.is_some() { sv(st(a), "path") } else { a.path.clone() };
    let target = if arg.starts_with('/') { arg.clone() } else { here.clone() };
    match verb {
        "bc-new" => {
            let parent = if a.listing.is_some() && a.writable() && a.path.starts_with('/') { a.path.clone() } else { a.home.clone() };
            return bc_call(s_(&["new", &parent, "Briefcase"]), String::new(), BThen::MadeNew);
        }
        "bc-update-all" => return open_update(target, vec![]),
        "bc-update-sel" => {
            let names = items_in(a, &t).iter().map(|p| base(p)).collect();
            return open_update(here, names);
        }
        "bc-find" => {
            let name = items_in(a, &t).first().map(|p| base(p)).unwrap_or_default();
            return bc_call(s_(&["find", &here, &name]), String::new(), BThen::Find);
        }
        "bc-split" => {
            let names: Vec<String> = items_in(a, &t).iter().map(|p| base(p)).collect();
            let title = format!("Split {} from the original?", if names.len() == 1 { format!("“{}”", names[0]) } else { format!("{} items", names.len()) });
            let mut args = vec!["split".to_string(), here];
            args.extend(names);
            a.confirm(&title, "The copy in the Briefcase stays, but it won't be updated any more (Orphan).", "Split", Yes::Then(Msg::Bc(BMsg::Run(format!("bc-do-split\n{}", args.join("\n")), vec![]))));
            if let Some(Overlay::Confirm(c)) = &mut a.overlay {
                c.danger = false;
            }
        }
        "bc-addto" => {
            let mut list: Vec<Value> = a.bc.known.iter().filter(|k| !bv(k, "missing")).cloned().collect();
            list.push(json!({ "name": "New Briefcase…", "sub": "Pick where it goes", "newBc": true }));
            return choose(a, "Add to Briefcase", list, PickThen::AddTo(t));
        }
        "bc-addto-path" => return add_to(a, arg, t),
        "bc-moveto" | "bc-copyto" => {
            let names: Vec<String> = items_in(a, &t).iter().map(|p| base(p)).collect();
            let mv = verb == "bc-moveto";
            let o = others(a);
            return choose(a, if mv { "Move to Briefcase" } else { "Copy to Briefcase" }, o, PickThen::Transfer(here, names, mv));
        }
        "bc-swap" => {
            let name = items_in(a, &t).first().map(|p| base(p)).unwrap_or_default();
            let o = others(a);
            return choose(a, &format!("Swap “{name}” with an item in…"), o, PickThen::SwapIn(here, name));
        }
        "bc-unlock" => {
            let name = if arg.is_empty() { bc_name(a) } else { base(&arg) };
            let then = if arg.is_empty() { BMsg::Run(format!("bc-go:{target}"), vec![]) } else { BMsg::Run("bc-known".into(), vec![]) };
            return ask_password(a, target, name, Some(then));
        }
        "bc-go" => return a.navigate(&arg, ""),
        "bc-known" => return refresh_known(),
        "bc-lock" => return bc_call(s_(&["lock", &target]), String::new(), BThen::Lock(target)),
        "bc-setlock" => return open_set_lock(a, target),
        "bc-passwd" => {
            let name = bc_name(a);
            a.bc.pw = Pw { path: target, name, ..Default::default() };
            open_dialog(a, "passwd");
            return iced::widget::operation::focus(PW_A);
        }
        "bc-unlockremove" => {
            a.confirm(
                &format!("Remove the lock from “{}”?", bc_name(a)),
                "Its files are decrypted into the Briefcase folder and checked; only then does the locked copy go to the Trash. Anyone with the folder can read them afterwards.",
                "Remove lock",
                Yes::Then(Msg::Bc(BMsg::Run(format!("bc-do-unlockremove:{target}"), vec![]))),
            );
        }
        "bc-do-unlockremove" => return run_job(a, "unlock", s_(&["lock-remove", &target]), String::new(), JobThen::LockRemove(target)),
        "bc-forget" => return bc_call(s_(&["forget", &target]), String::new(), BThen::Forget),
        "bc-customize" => return bc_call(s_(&["status", &target]), String::new(), BThen::CustOpen),
        "bc-syncnow" => return run_job(a, "sync", s_(&["sync-now", &target]), String::new(), JobThen::Sync),
        "bc-send" => {
            let mut drives: Vec<Value> = a.raw_places.iter().filter(|p| sv(p, "section") == "drives" && !sv(p, "path").is_empty()).map(|p| json!({ "name": sv(p, "label"), "path": sv(p, "path"), "sub": sv(p, "path") })).collect();
            drives.push(json!({ "name": "Home folder", "path": a.home, "sub": a.home }));
            let name = if arg.is_empty() { bc_name(a) } else { base(&arg) };
            return choose(a, &format!("Send “{name}” to…"), drives, PickThen::Send(target));
        }
        "bc-resetlist" => {
            a.confirm(
                "Start this Briefcase's list again?",
                "The damaged list is kept aside (nothing is deleted). The files stay, but they forget their originals.",
                "Start again",
                Yes::Then(Msg::Bc(BMsg::Run(format!("bc-do-resetlist:{here}"), vec![]))),
            );
        }
        "bc-do-resetlist" => return bc_call(s_(&["reset-list", &target]), String::new(), BThen::ResetList),
        _ => {
            if let Some(rest) = id.strip_prefix("bc-do-split\n") {
                let args: Vec<String> = rest.split('\n').map(String::from).collect();
                return bc_call(args, String::new(), BThen::Split);
            }
        }
    }
    Task::none()
}

/// Lock as a Briefcase (mockup files-7): a new Briefcase next to the folder, holding it (kept in step with the
/// original), then its lock is set up (your password). The folder itself isn't moved or changed.
pub fn lock_as_briefcase(path: String) -> Task<Msg> {
    let parent = parent_of(&path);
    let name = format!("{} (Briefcase)", base(&path));
    bc_call(vec!["new".into(), parent, name], String::new(), BThen::LockAsNew(path))
}

/// Dropped on a Briefcase (its folder, its icon, its place): added, or moved / copied from another Briefcase.
pub fn briefcase_at(a: &App, dest: &str) -> Option<String> {
    if in_briefcase(a) && a.bc.st.is_some() && (dest == a.path || dest == sv(st(a), "path")) {
        return Some(sv(st(a), "path"));
    }
    if let Some(k) = a.bc.known.iter().find(|k| sv(k, "path") == dest || sv(k, "content") == dest) {
        return Some(sv(k, "path"));
    }
    a.entries.iter().find(|e| e.path == dest && e.briefcase.is_object()).map(|e| e.path.clone())
}

pub fn drop(a: &mut App, paths: &[String], dest: &str) -> Option<Task<Msg>> {
    let to = briefcase_at(a, dest)?;
    let st_path = sv(st(a), "path");
    let from = if in_briefcase(a) && a.bc.st.is_some() && st_path != to && paths.iter().all(|p| parent_of(p) == a.path) { st_path } else { String::new() };
    if !from.is_empty() && items_in(a, paths).len() == paths.len() {
        let names = paths.iter().map(|p| base(p)).collect();
        let mv = !a.mods.control();
        return Some(transfer(a, from, to, names, mv));
    }
    Some(add_to(a, to, paths.to_vec()))
}

fn pick_choose(a: &mut App, i: usize) -> Task<Msg> {
    let Some(pk) = a.bc.pick.clone() else { return Task::none() };
    close(a);
    let Some(row) = pk.list.get(i) else { return Task::none() };
    let k = row.raw.clone();
    let kpath = sv(&k, "path");
    match pk.then {
        PickThen::AddTo(t) => {
            if bv(&k, "newBc") {
                return new_briefcase_for(a, t);
            }
            add_to(a, kpath, t)
        }
        PickThen::Transfer(from, names, mv) => transfer(a, from, kpath, names, mv),
        PickThen::SwapIn(here, name) => bc_call(s_(&["status", &kpath]), String::new(), BThen::SwapList(here, name, kpath, sv(&k, "name"))),
        PickThen::SwapWith(here, name, kp) => {
            let other = sv(&k, "name");
            run_job(a, "swap", s_(&["swap", &here, &name, &kp, &other]), String::new(), JobThen::Swap(name, other))
        }
        PickThen::Send(target) => run_job(a, "send", s_(&["send", &target, &kpath]), String::new(), JobThen::Send(sv(&k, "name"))),
        PickThen::NewAt(paths) => bc_call(s_(&["new", &kpath, "Briefcase"]), String::new(), BThen::NewFor(paths)),
    }
}

/// "Add to Briefcase › New Briefcase…": where it goes (this folder, your usual folders, your drives).
fn new_briefcase_for(a: &mut App, paths: Vec<String>) -> Task<Msg> {
    let mut wh: Vec<Value> = vec![];
    let mut seen: Vec<String> = vec![];
    let mut put = |name: String, path: String, wh: &mut Vec<Value>| {
        if !path.is_empty() && !seen.contains(&path) {
            seen.push(path.clone());
            wh.push(json!({ "name": name, "path": path, "sub": path }));
        }
    };
    if a.listing.is_some() && a.writable() && !in_briefcase(a) && a.path.starts_with('/') {
        let n = a.listing.as_ref().map(|l| l.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| base(&a.path));
        put(format!("This folder ({n})"), a.path.clone(), &mut wh);
    }
    put("Home folder".into(), a.home.clone(), &mut wh);
    for q in &a.raw_places {
        if sv(q, "section") == "places" && q.get("child") != Some(&Value::Bool(true)) && (sv(q, "kind") == "documents" || sv(q, "kind") == "desktop") {
            put(sv(q, "label"), sv(q, "path"), &mut wh);
        }
    }
    for q in &a.raw_places {
        if sv(q, "section") == "drives" && !sv(q, "path").is_empty() && sv(q, "path") != "/" && q.get("mounted") != Some(&Value::Bool(false)) {
            put(sv(q, "label"), sv(q, "path"), &mut wh);
        }
    }
    choose(a, "Where should the new Briefcase go?", wh, PickThen::NewAt(paths))
}

// ------------------------------------------------------------------ the dialogs' rows
#[derive(Debug, Clone, Copy, PartialEq)]
enum RK {
    Field,
    Check,
    Choice,
    Button,
    Row,
}

fn rows(a: &App) -> Vec<(RK, String)> {
    let b = &a.bc;
    let r = |k: RK, id: &str| (k, id.to_string());
    match b.mode.as_str() {
        "password" => vec![r(RK::Field, "a"), r(RK::Check, "remember"), r(RK::Button, "cancel"), r(RK::Button, "ok")],
        "setlock" => vec![r(RK::Field, "a"), r(RK::Field, "b"), r(RK::Check, "remember"), r(RK::Button, "cancel"), r(RK::Button, "ok")],
        "passwd" => vec![r(RK::Field, "a"), r(RK::Field, "b"), r(RK::Field, "c"), r(RK::Button, "cancel"), r(RK::Button, "ok")],
        "pick" => {
            let mut v: Vec<(RK, String)> = b.pick.as_ref().map(|p| (0..p.list.len()).map(|i| (RK::Row, i.to_string())).collect()).unwrap_or_default();
            v.push(r(RK::Button, "cancel"));
            v
        }
        "update" => {
            let n = b.plan.as_ref().and_then(|p| p.get("rows")).and_then(Value::as_array).map(|x| x.len()).unwrap_or(0);
            let mut v: Vec<(RK, String)> = (0..n).map(|i| (RK::Row, i.to_string())).collect();
            v.push(r(RK::Button, "cancel"));
            v.push(r(RK::Button, "ok"));
            v
        }
        "customize" => {
            let c = b.cust.as_ref();
            let sc = c.map(|c| c.sync_custom).unwrap_or(false);
            let lc = c.map(|c| c.lock_custom && c.locked).unwrap_or(false);
            let mut v = vec![r(RK::Field, "name"), r(RK::Choice, "color"), r(RK::Choice, "icon"), r(RK::Field, "badge"), r(RK::Field, "note"), r(RK::Choice, "view"), r(RK::Choice, "sort"), r(RK::Choice, "autosync")];
            if sc {
                v.push(r(RK::Field, "syncmin"));
            }
            v.push(r(RK::Choice, "autolock"));
            if lc {
                v.push(r(RK::Field, "lockmin"));
            }
            v.extend([r(RK::Check, "onSleep"), r(RK::Check, "onEject"), r(RK::Check, "showInNav"), r(RK::Button, "cancel"), r(RK::Button, "ok")]);
            v
        }
        "job" => vec![r(RK::Button, "cancel")],
        "message" => vec![r(RK::Button, "ok")],
        _ => vec![],
    }
}

fn lit(a: &App, id: &str) -> bool {
    let rs = rows(a);
    rs.get(a.bc.row.min(rs.len().saturating_sub(1))).map(|r| r.1 == id).unwrap_or(false)
}

pub const PW_A: &str = "bc-pw-a";
pub const PW_B: &str = "bc-pw-b";
pub const PW_C: &str = "bc-pw-c";

fn field_id(mode: &str, id: &str) -> &'static str {
    match (mode, id) {
        ("customize", "name") => "bc-cust-name",
        ("customize", "badge") => "bc-cust-badge",
        ("customize", "note") => "bc-cust-note",
        ("customize", "syncmin") => "bc-cust-syncmin",
        ("customize", "lockmin") => "bc-cust-lockmin",
        (_, "b") => PW_B,
        (_, "c") => PW_C,
        _ => PW_A,
    }
}

fn focus_row(a: &App) -> Task<Msg> {
    let rs = rows(a);
    match rs.get(a.bc.row) {
        Some((RK::Field, id)) => iced::widget::operation::focus(field_id(&a.bc.mode, id)),
        _ => iced::widget::operation::focus("files-none"),
    }
}

const ICONS: [&str; 5] = ["briefcase", "satchel", "suitcase", "backpack", "box"];
const COLORS: [&str; 8] = ["accent", "ember", "red", "green", "blue", "purple", "gold", "slate"];
const VIEWS: [&str; 7] = ["", "large", "grid", "small", "tiles", "compact", "details"];
const SORTS: [&str; 5] = ["", "name", "date", "size", "type"];
const SYNC: [(&str, i64, &str); 7] = [("instant", 0, "Right away"), ("interval", 5, "Every 5 minutes"), ("interval", 15, "Every 15 minutes"), ("interval", 30, "Every 30 minutes"), ("interval", -1, "Every … minutes (custom)"), ("onlock", 0, "When I lock it"), ("off", 0, "Off (only Update All)")];
const LOCKS: [(i64, &str); 8] = [(0, "Never (only when I lock it)"), (1, "1 minute"), (5, "5 minutes"), (15, "15 minutes"), (30, "30 minutes"), (60, "1 hour"), (240, "4 hours"), (-1, "Custom (minutes)")];

fn cyc(list: &[&str], v: &str, d: i32) -> String {
    let n = list.len() as i32;
    let i = list.iter().position(|x| *x == v).unwrap_or(0) as i32;
    list[((i + d) % n + n) as usize % n as usize].to_string()
}

fn sync_index(c: &Cust) -> usize {
    let s = c.look.get("autosync").cloned().unwrap_or(Value::Null);
    let mode = sv(&s, "mode");
    let min = s.get("minutes").and_then(Value::as_i64).unwrap_or(0);
    match mode.as_str() {
        "interval" => if c.sync_custom { 4 } else { [0i64, 5, 15, 30].iter().position(|m| *m == min).unwrap_or(1).max(1) },
        "instant" => 0,
        "onlock" => 5,
        _ => 6,
    }
}
fn set_sync_index(c: &mut Cust, i: i32) {
    let n = SYNC.len() as i32;
    let ch = SYNC[((i % n + n) % n) as usize];
    let mut o = c.look.get("autosync").cloned().filter(|v| v.is_object()).unwrap_or(json!({}));
    o["mode"] = json!(ch.0);
    if ch.1 > 0 {
        o["minutes"] = json!(ch.1);
    }
    c.sync_custom = ch.1 < 0;
    c.look["autosync"] = o;
}
fn lock_index(c: &Cust) -> usize {
    if c.lock_custom {
        return 7;
    }
    let m = c.look.get("autolock").and_then(|l| l.get("minutes")).and_then(Value::as_i64).unwrap_or(0);
    [0i64, 1, 5, 15, 30, 60, 240].iter().position(|x| *x == m).unwrap_or(0)
}
fn set_lock_index(c: &mut Cust, i: i32) {
    if !c.locked {
        return;
    }
    let n = LOCKS.len() as i32;
    let ch = LOCKS[((i % n + n) % n) as usize];
    let mut o = c.look.get("autolock").cloned().filter(|v| v.is_object()).unwrap_or(json!({}));
    if ch.0 >= 0 {
        o["minutes"] = json!(ch.0);
    }
    c.lock_custom = ch.0 < 0;
    c.look["autolock"] = o;
}

fn press(a: &mut App, i: usize, d: i32) -> Task<Msg> {
    let rs = rows(a);
    let Some((k, id)) = rs.get(i).cloned() else { return Task::none() };
    a.bc.row = i;
    match k {
        RK::Row if a.bc.mode == "pick" => return pick_choose(a, id.parse().unwrap_or(0)),
        RK::Row => {
            // Update All: the next (or the one before) of the row's choices.
            let ri: usize = id.parse().unwrap_or(0);
            if let Some(rows) = a.bc.plan.as_mut().and_then(|p| p.get_mut("rows")).and_then(Value::as_array_mut) {
                if let Some(r) = rows.get_mut(ri) {
                    let choices: Vec<String> = r.get("choices").and_then(Value::as_array).map(|c| c.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                    if !choices.is_empty() {
                        let at = choices.iter().position(|c| *c == sv(r, "action")).unwrap_or(0) as i32;
                        let n = choices.len() as i32;
                        r["action"] = json!(choices[(((at + d) % n + n) % n) as usize]);
                    }
                }
            }
        }
        RK::Check => {
            match id.as_str() {
                "remember" => a.bc.pw.remember = !a.bc.pw.remember,
                "onSleep" | "onEject" => {
                    if let Some(c) = a.bc.cust.as_mut() {
                        if c.locked {
                            let on = c.look.get("autolock").and_then(|l| l.get(&id)).and_then(Value::as_bool).unwrap_or(false);
                            let mut o = c.look.get("autolock").cloned().filter(|v| v.is_object()).unwrap_or(json!({}));
                            o[id.as_str()] = json!(!on);
                            c.look["autolock"] = o;
                        }
                    }
                }
                "showInNav" => {
                    if let Some(c) = a.bc.cust.as_mut() {
                        let on = bv(&c.look, "showInNav");
                        c.look["showInNav"] = json!(!on);
                    }
                }
                _ => {}
            }
        }
        RK::Choice => {
            if let Some(c) = a.bc.cust.as_mut() {
                match id.as_str() {
                    "color" => c.look["color"] = json!(cyc(&COLORS, &sv(&c.look, "color"), d)),
                    "icon" => c.look["icon"] = json!(cyc(&ICONS, &sv(&c.look, "icon"), d)),
                    "view" => c.look["view"] = json!(cyc(&VIEWS, &sv(&c.look, "view"), d)),
                    "sort" => c.look["sort"] = json!(cyc(&SORTS, &sv(&c.look, "sort"), d)),
                    "autosync" => {
                        let i = sync_index(c) as i32 + d;
                        set_sync_index(c, i);
                    }
                    "autolock" => {
                        let i = lock_index(c) as i32 + d;
                        set_lock_index(c, i);
                    }
                    _ => {}
                }
            }
        }
        RK::Field => return focus_row(a),
        RK::Button => {
            if id == "cancel" {
                if a.bc.mode == "job" {
                    cancel_job(a);
                } else {
                    close(a);
                }
                return Task::none();
            }
            return ok(a);
        }
    }
    Task::none()
}

fn ok(a: &mut App) -> Task<Msg> {
    match a.bc.mode.as_str() {
        "password" => {
            let p = a.bc.pw.clone();
            if p.a.is_empty() {
                a.bc.pw.error = "Type the password".into();
                return Task::none();
            }
            let input = format!("{}\n", p.a);
            a.bc.pw.a.clear();
            a.bc.pw.error = "Checking…".into();
            let mut args = vec!["unlock".to_string(), p.path.clone()];
            if p.remember {
                args.push("--remember".into());
            }
            bc_call(args, input, BThen::Unlocked)
        }
        "setlock" => {
            let p = a.bc.pw.clone();
            if p.a.chars().count() < 4 {
                a.bc.pw.error = "Use at least 4 characters".into();
                return Task::none();
            }
            if p.a != p.b {
                a.bc.pw.error = "The two passwords aren't the same".into();
                return Task::none();
            }
            let input = format!("{}\n", p.a);
            a.bc.pw.a.clear();
            a.bc.pw.b.clear();
            let mut args = vec!["lock-setup".to_string(), p.path.clone()];
            if p.remember {
                args.push("--remember".into());
            }
            run_job(a, "lock", args, input, JobThen::Lock(p.path, p.name))
        }
        "passwd" => {
            let p = a.bc.pw.clone();
            if p.a.is_empty() || p.b.chars().count() < 4 {
                a.bc.pw.error = "Type the old password and a new one (4 characters or more)".into();
                return Task::none();
            }
            if p.b != p.c {
                a.bc.pw.error = "The two new passwords aren't the same".into();
                return Task::none();
            }
            let input = format!("{}\n{}\n", p.a, p.b);
            a.bc.pw.a.clear();
            a.bc.pw.b.clear();
            a.bc.pw.c.clear();
            a.bc.pw.error = "Changing…".into();
            bc_call(s_(&["passwd", &p.path]), input, BThen::Passwd)
        }
        "update" => {
            let Some(p) = a.bc.plan.clone() else { return Task::none() };
            close(a);
            let rows: Vec<Value> = p.get("rows").and_then(Value::as_array).cloned().unwrap_or_default().iter().map(|r| json!({ "id": r.get("id"), "action": r.get("action"), "bsig": r.get("bsig"), "osig": r.get("osig"), "size": r.get("size") })).collect();
            let path = sv(&p, "path");
            run_job(a, "update", vec!["apply".into(), path, json!({ "rows": rows }).to_string()], String::new(), JobThen::Update)
        }
        "customize" => save_customize(a),
        "message" => {
            close(a);
            Task::none()
        }
        "pick" => {
            let rs = rows(a);
            if let Some((RK::Row, id)) = rs.get(a.bc.row) {
                let i = id.parse().unwrap_or(0);
                return pick_choose(a, i);
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

/// Customize: only what you change is saved, into the Briefcase's own list (so it travels with it).
fn save_customize(a: &mut App) -> Task<Msg> {
    let Some(c) = a.bc.cust.clone() else { return Task::none() };
    let (was, now) = (&c.orig, &c.look);
    let mut ch = Map::new();
    for k in ["name", "color", "icon", "badge", "note", "view", "sort", "showInNav"] {
        if was.get(k) != now.get(k) {
            ch.insert(k.into(), now.get(k).cloned().unwrap_or(Value::Null));
        }
    }
    for k in ["autosync", "autolock"] {
        let mut d = Map::new();
        if let Some(o) = now.get(k).and_then(Value::as_object) {
            for (s, v) in o {
                if was.get(k).and_then(|w| w.get(s)) != Some(v) {
                    d.insert(s.clone(), v.clone());
                }
            }
        }
        if !d.is_empty() {
            ch.insert(k.into(), Value::Object(d));
        }
    }
    let path = c.path.clone();
    close(a);
    if ch.is_empty() {
        return Task::none();
    }
    bc_call(vec!["customize".into(), path.clone(), Value::Object(ch).to_string()], String::new(), BThen::CustSaved(path))
}

/// The keys while a Briefcase dialog is open (Up / Down move, Left / Right change a choice, Enter presses, Esc closes).
pub fn key(a: &mut App, named: Option<iced::keyboard::key::Named>, shift: bool) -> Task<Msg> {
    use iced::keyboard::key::Named;
    let rs = rows(a);
    let n = rs.len();
    let at = rs.get(a.bc.row).cloned();
    let field = matches!(at, Some((RK::Field, _)));
    match named {
        Some(Named::ArrowUp) => {
            a.bc.row = a.bc.row.saturating_sub(1);
            focus_row(a)
        }
        Some(Named::Tab) if shift => {
            a.bc.row = a.bc.row.saturating_sub(1);
            focus_row(a)
        }
        Some(Named::ArrowDown) | Some(Named::Tab) => {
            a.bc.row = (a.bc.row + 1).min(n.saturating_sub(1));
            focus_row(a)
        }
        Some(Named::ArrowLeft) | Some(Named::ArrowRight) if !field => {
            let d = if matches!(named, Some(Named::ArrowLeft)) { -1 } else { 1 };
            match at {
                Some((RK::Button, _)) => {
                    let j = a.bc.row as i32 + d;
                    if j >= 0 && (j as usize) < n && rs[j as usize].0 == RK::Button {
                        a.bc.row = j as usize;
                    }
                    Task::none()
                }
                Some(_) => {
                    let i = a.bc.row;
                    press(a, i, d)
                }
                None => Task::none(),
            }
        }
        Some(Named::Enter) => {
            if field {
                return ok(a);
            }
            let i = a.bc.row;
            press(a, i, 1)
        }
        Some(Named::Space) if !field => {
            let i = a.bc.row;
            press(a, i, 1)
        }
        Some(Named::Escape) | Some(Named::Backspace) if !field || matches!(named, Some(Named::Escape)) => {
            if a.bc.mode == "job" {
                cancel_job(a);
            } else {
                close(a);
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

// ------------------------------------------------------------------ answers
pub fn update(a: &mut App, m: BMsg) -> Task<Msg> {
    match m {
        BMsg::Run(id, t) => return run(a, &id, t),
        BMsg::Tick => {
            if in_briefcase(a) && locked(a) && unlocked(a) && a.overlay.is_none() {
                return refresh(a);
            }
        }
        BMsg::Row(i) => {
            a.bc.row = i;
            return focus_row(a);
        }
        BMsg::Press(i, d) => return press(a, i, d),
        BMsg::Field(id, text) => {
            if let Some(i) = rows(a).iter().position(|r| r.0 == RK::Field && r.1 == id) {
                a.bc.row = i;
            }
            match id {
                "a" => {
                    a.bc.pw.a = text;
                    a.bc.pw.error.clear();
                }
                "b" => {
                    a.bc.pw.b = text;
                    a.bc.pw.error.clear();
                }
                "c" => {
                    a.bc.pw.c = text;
                    a.bc.pw.error.clear();
                }
                "name" | "note" => {
                    if let Some(c) = a.bc.cust.as_mut() {
                        c.look[id] = json!(text);
                    }
                }
                "badge" => {
                    if let Some(c) = a.bc.cust.as_mut() {
                        c.look["badge"] = json!(text.chars().take(4).collect::<String>());
                    }
                }
                "syncmin" | "lockmin" => {
                    if let Some(c) = a.bc.cust.as_mut() {
                        if let Ok(n) = text.trim().parse::<i64>() {
                            if n > 0 {
                                let (k, max) = if id == "syncmin" { ("autosync", 1440) } else { ("autolock", 10080) };
                                let mut o = c.look.get(k).cloned().filter(|v| v.is_object()).unwrap_or(json!({}));
                                o["minutes"] = json!(n.min(max));
                                c.look[k] = o;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        BMsg::Ok => return ok(a),
        BMsg::Cancel => {
            if a.bc.mode == "job" {
                cancel_job(a);
            } else {
                close(a);
            }
        }
        BMsg::Line(l) => match l {
            BLine::Pid(seq, pid) => {
                if let Some(j) = a.bc.job.as_mut().filter(|j| j.seq == seq) {
                    j.pid = pid;
                }
            }
            BLine::Progress(seq, v) => {
                if let Some(j) = a.bc.job.as_mut().filter(|j| j.seq == seq) {
                    j.done = v.get("done").and_then(Value::as_f64).unwrap_or(0.0);
                    j.total = v.get("total").and_then(Value::as_f64).unwrap_or(0.0);
                    j.file = sv(&v, "file");
                }
            }
            BLine::Done(seq, r) => {
                let Some(j) = a.bc.job.clone().filter(|j| j.seq == seq) else { return Task::none() };
                a.bc.job = None;
                if a.bc.mode == "job" {
                    close(a);
                }
                let t = job_done(a, j, r);
                let p = a.path.clone();
                let cur = a.current().map(|e| e.name.clone()).unwrap_or_default();
                let mut tasks = vec![t, refresh(a), a.refresh_places(), refresh_known()];
                if !p.is_empty() && a.overlay.is_none() {
                    tasks.push(a.load(&p, &cur, false));
                }
                return Task::batch(tasks);
            }
        },
        BMsg::Did(then, r) => return did(a, then, r),
    }
    Task::none()
}

fn job_done(a: &mut App, j: BJob, r: Value) -> Task<Msg> {
    let cancelled = j.cancel || bv(&r, "cancelled");
    let again = |a: &mut App, msg: BMsg| -> Option<Task<Msg>> {
        if sv(&r, "code") == "locked" {
            return Some(ask_password(a, sv(&r, "path"), sv(&r, "name"), Some(msg)));
        }
        None
    };
    match j.then {
        JobThen::Update => {
            if !bv(&r, "ok") {
                let e = sv(&r, "error");
                return a.say(if e.is_empty() { "Something went wrong" } else { &e }, true);
            }
            let n = r.get("done").and_then(Value::as_array).map(|x| x.len()).unwrap_or(0) as u64;
            let mut text = if cancelled { format!("Stopped after {}", plural(n, "change")) } else if n > 0 { format!("{} made", plural(n, "change")) } else { "Nothing needed doing".into() };
            if r.get("trashed").and_then(Value::as_array).map(|x| !x.is_empty()).unwrap_or(false) {
                text += " · replaced versions are in the Trash";
            }
            if r.get("replaced").and_then(Value::as_array).map(|x| !x.is_empty()).unwrap_or(false) {
                text += " · replaced versions kept in .briefcase-replaced (no Trash on that shared folder)";
            }
            let sk: Vec<Value> = r.get("skipped").and_then(Value::as_array).cloned().unwrap_or_default().into_iter().filter(|s| sv(s, "why") != "kept both").collect();
            if let Some(f) = sk.first() {
                text += &format!(" · {} skipped ({})", plural(sk.len() as u64, "item"), sv(f, "why"));
            }
            a.say(&text, false)
        }
        JobThen::Add(path, paths) => {
            if let Some(t) = again(a, BMsg::Run(format!("bc-addto-path:{path}"), paths)) {
                return t;
            }
            let added: Vec<Value> = r.get("added").and_then(Value::as_array).cloned().unwrap_or_default();
            let ok_text = if added.len() == 1 { format!("“{}” is in the Briefcase", sv(&added[0], "name")) } else if !added.is_empty() { format!("{} items are in the Briefcase", added.len()) } else { String::new() };
            done(a, &r, &ok_text).unwrap_or_else(Task::none)
        }
        JobThen::Transfer(from, to, names, mv) => {
            if sv(&r, "code") == "locked" {
                let _ = (&from, &to);
                return ask_password(a, sv(&r, "path"), sv(&r, "name"), None);
            }
            let what = if names.len() == 1 { format!("“{}”", names[0]) } else { format!("{} items", names.len()) };
            done(a, &r, &format!("{} {what}: still linked to the original", if mv { "Moved" } else { "Copied" })).unwrap_or_else(Task::none)
        }
        JobThen::Swap(x, y) => {
            if sv(&r, "code") == "locked" {
                return ask_password(a, sv(&r, "path"), sv(&r, "name"), None);
            }
            done(a, &r, &format!("Swapped “{x}” and “{y}”")).unwrap_or_else(Task::none)
        }
        JobThen::LockRemove(target) => {
            if let Some(t) = done(a, &r, "The lock was removed") {
                if bv(&r, "ok") {
                    return Task::batch(vec![t, a.navigate(&target, "")]);
                }
                return t;
            }
            Task::none()
        }
        JobThen::Sync => {
            if !bv(&r, "ok") {
                return done(a, &r, "").unwrap_or_else(Task::none);
            }
            let synced = r.get("synced").and_then(Value::as_u64).unwrap_or(0);
            let needs = r.get("needs").and_then(Value::as_u64).unwrap_or(0);
            let text = if synced > 0 {
                format!("{} synced{}", plural(synced, "file"), if needs > 0 { format!(" · {} need you", plural(needs, "file")) } else { String::new() })
            } else if needs > 0 {
                format!("{} need you: use Update All", plural(needs, "file"))
            } else {
                "Everything is up-to-date".into()
            };
            a.say(&text, false)
        }
        JobThen::Send(name) => done(a, &r, &format!("Sent to {name}: it keeps its list and works there too")).unwrap_or_else(Task::none),
        JobThen::Lock(path, name) => {
            if let Some(t) = done(a, &r, "") {
                return t;
            }
            a.bc.msg = Some((
                format!("“{name}” is locked"),
                format!("Its files are encrypted inside the folder. The unlocked copy went to the Trash (empty the Trash to remove it{}).", if path.contains("/media/") { ", including the drive's own Trash" } else { "" }),
            ));
            open_dialog(a, "message");
            a.navigate(&path, "")
        }
    }
}

fn did(a: &mut App, then: BThen, r: Value) -> Task<Msg> {
    match then {
        BThen::Known => {
            if bv(&r, "ok") {
                a.bc.known = r.get("briefcases").and_then(Value::as_array).cloned().unwrap_or_default();
            }
            Task::none()
        }
        BThen::Status(at, first) => {
            if a.path != at {
                return Task::none();
            }
            a.bc.st = if bv(&r, "ok") { Some(r.clone()) } else { None };
            a.bc.st_at = Some(Instant::now());
            if !first || a.bc.st.is_none() {
                return Task::none();
            }
            let content = sv(&r, "content");
            if bv(&r, "locked") && bv(&r, "unlocked") && !content.is_empty() && content != at {
                return a.load(&content, "", false);
            }
            if bv(&r, "locked") && !bv(&r, "unlocked") {
                let (p, n) = (sv(&r, "path"), sv(&r, "name"));
                return ask_password(a, p.clone(), n, Some(BMsg::Run(format!("bc-go:{p}"), vec![])));
            }
            Task::perform(call(vec!["touch".into(), at], String::new()), |_| Msg::NoteClear(u64::MAX))
        }
        BThen::Say(s) => done(a, &r, &s).unwrap_or_else(Task::none),
        BThen::Find => {
            if let Some(t) = done(a, &r, "") {
                return t;
            }
            let p = sv(&r, "path");
            a.navigate(&p, "")
        }
        BThen::Split => {
            let t = done(a, &r, "Split from the original").unwrap_or_else(Task::none);
            Task::batch(vec![t, refresh(a)])
        }
        BThen::Lock(target) => {
            let t = done(a, &r, "Locked").unwrap_or_else(Task::none);
            if bv(&r, "ok") {
                return Task::batch(vec![t, a.navigate(&target, ""), a.refresh_places()]);
            }
            t
        }
        BThen::Forget => done(a, &r, "This device no longer remembers the password").unwrap_or_else(Task::none),
        BThen::MadeNew => {
            let t = done(a, &r, &format!("Made “{}”", base(&sv(&r, "path")))).unwrap_or_else(Task::none);
            Task::batch(vec![t, refresh_known(), a.refresh_places(), a.reload()])
        }
        BThen::LockAsNew(src) => {
            if let Some(t) = done(a, &r, &format!("Made “{}”", base(&sv(&r, "path")))) {
                if !bv(&r, "ok") {
                    return t;
                }
                let bc = sv(&r, "path");
                return Task::batch(vec![t, bc_call(vec!["add".into(), bc.clone(), src], String::new(), BThen::LockAsAdded(bc))]);
            }
            Task::none()
        }
        BThen::LockAsAdded(bc) => {
            let t = open_set_lock(a, bc);
            Task::batch(vec![refresh_known(), a.refresh_places(), a.reload(), t])
        }
        BThen::NewFor(paths) => {
            if let Some(t) = done(a, &r, &format!("Made “{}”", base(&sv(&r, "path")))) {
                if !bv(&r, "ok") {
                    return t;
                }
                let p = sv(&r, "path");
                let add = add_to(a, p, paths);
                return Task::batch(vec![t, refresh_known(), a.refresh_places(), add]);
            }
            Task::none()
        }
        BThen::Keyring(path) => {
            if a.bc.mode != "password" || a.bc.pw.path != path {
                return Task::none();
            }
            if bv(&r, "ok") {
                let pw = a.bc.pw.clone();
                close(a);
                let t = a.say(&format!("“{}” is unlocked", pw.name), false);
                let mut tasks = vec![t, refresh_known()];
                if let Some(m) = pw.then {
                    tasks.push(update(a, *m));
                }
                return Task::batch(tasks);
            }
            iced::widget::operation::focus(PW_A)
        }
        BThen::Unlocked => {
            if !bv(&r, "ok") {
                let e = sv(&r, "error");
                a.bc.pw.error = if e.is_empty() { "Couldn't unlock".into() } else { e };
                return Task::none();
            }
            let pw = a.bc.pw.clone();
            close(a);
            let t = a.say(&format!("“{}” is unlocked", pw.name), false);
            let mut tasks = vec![t, refresh_known(), a.refresh_places()];
            match pw.then {
                Some(m) => tasks.push(update(a, *m)),
                None => {
                    let c = sv(&r, "content");
                    tasks.push(a.navigate(&c, ""));
                }
            }
            Task::batch(tasks)
        }
        BThen::Passwd => {
            if !bv(&r, "ok") {
                let e = sv(&r, "error");
                a.bc.pw.error = if e.is_empty() { "Couldn't change it".into() } else { e };
                return Task::none();
            }
            close(a);
            a.say("The password was changed", false)
        }
        BThen::Plan(path, names) => {
            if sv(&r, "code") == "locked" {
                let mut ids = vec![format!("bc-update-all:{path}")];
                ids.extend(names);
                return ask_password(a, sv(&r, "path"), sv(&r, "name"), Some(BMsg::Run(ids[0].clone(), vec![])));
            }
            if let Some(t) = done(a, &r, "") {
                return t;
            }
            let none = r.get("rows").and_then(Value::as_array).map(|x| x.is_empty()).unwrap_or(true) && r.get("info").and_then(Value::as_array).map(|x| x.is_empty()).unwrap_or(true);
            if none {
                return a.say(&format!("Everything in “{}” is up-to-date", sv(&r, "name")), false);
            }
            a.bc.plan = Some(r);
            open_dialog(a, "update");
            Task::none()
        }
        BThen::CustOpen => {
            if let Some(t) = done(a, &r, "") {
                return t;
            }
            let look = r.get("look").cloned().unwrap_or(json!({}));
            let sm = look.get("autosync").and_then(|x| x.get("minutes")).and_then(Value::as_i64).unwrap_or(0);
            let lm = look.get("autolock").and_then(|x| x.get("minutes")).and_then(Value::as_i64).unwrap_or(0);
            let sync_custom = look.get("autosync").map(|x| sv(x, "mode")).unwrap_or_default() == "interval" && ![5, 15, 30].contains(&sm);
            let lock_custom = ![0, 1, 5, 15, 30, 60, 240].contains(&lm);
            a.bc.cust = Some(Cust { path: sv(&r, "path"), locked: bv(&r, "locked"), orig: look.clone(), look, sync_custom, lock_custom });
            open_dialog(a, "customize");
            iced::widget::operation::focus("bc-cust-name")
        }
        BThen::CustSaved(path) => {
            let t = done(a, &r, "Saved").unwrap_or_else(Task::none);
            if !bv(&r, "ok") {
                return t;
            }
            let np = sv(&r, "path");
            let mut tasks = vec![t, refresh_known(), a.refresh_places()];
            if !np.is_empty() && np != path {
                tasks.push(a.navigate(&np, ""));
            } else {
                tasks.push(refresh(a));
                tasks.push(a.reload());
            }
            Task::batch(tasks)
        }
        BThen::SwapList(here, name, kpath, kname) => {
            if bv(&r, "locked") && !bv(&r, "unlocked") {
                return ask_password(a, kpath, kname, None);
            }
            if let Some(t) = done(a, &r, "") {
                return t;
            }
            let list: Vec<Value> = r.get("items").and_then(Value::as_array).cloned().unwrap_or_default().iter().map(|i| json!({ "name": sv(i, "name"), "path": kpath, "sub": sv(i, "label") })).collect();
            choose(a, &format!("Swap “{name}” with"), list, PickThen::SwapWith(here, name, kpath))
        }
        BThen::ResetList => {
            let t = done(a, &r, "A new list was started; the old one is kept aside").unwrap_or_else(Task::none);
            Task::batch(vec![t, refresh(a)])
        }
    }
}

// ------------------------------------------------------------------ drawing the dialogs (BriefcaseDialog.qml)
fn pill<'a>(p: &Pal, label: String, primary: bool, focused: bool, msg: Msg) -> Element<'a, Msg> {
    let p = *p;
    button(container(tb(label, 14.0, if primary { p.ink } else { p.text })).center_y(Length::Fixed(24.0)))
        .padding(Padding { top: 8.0, right: 18.0, bottom: 8.0, left: 18.0 })
        .on_press(msg)
        .style(move |_: &Theme, st: button::Status| {
            let hover = matches!(st, button::Status::Hovered | button::Status::Pressed);
            let bg = if primary { if hover { Pal::mix(p.accent, Color::WHITE, 0.1) } else { p.accent } } else if hover { Pal::mix(p.panel3, p.text, 0.06) } else { p.panel3 };
            button::Style { background: Some(Background::Color(bg)), text_color: p.text, border: Border { color: if focused { p.text } else { Color::TRANSPARENT }, width: 2.0, radius: p.r(20.0).into() }, shadow: Shadow::default(), snap: true }
        })
        .into()
}

fn row_style(p: Pal, on: bool, base_bg: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(Background::Color(if on { p.panel3 } else { base_bg })), border: Border { color: if on { p.accent } else { Color::TRANSPARENT }, width: 2.0, radius: p.r(10.0).into() }, ..container::Style::default() }
}

fn grow<'a>(p: &Pal, dim: bool, k: &str, w: Element<'a, Msg>) -> Element<'a, Msg> {
    row![label(p, k, dim), container(w).width(Length::Fill)].spacing(16.0).align_y(iced::Center).into()
}

fn label<'a>(p: &Pal, s: &str, dim: bool) -> Element<'a, Msg> {
    container(t1(s.to_string(), 13.0, if dim { Pal::a(p.muted, 0.5) } else { p.muted })).width(Length::Fixed(110.0)).into()
}

fn field<'a>(a: &'a App, id: &'static str, value: &str, hint: &str, secret: bool) -> Element<'a, Msg> {
    let p = a.p;
    let on = lit(a, id);
    let fid = field_id(&a.bc.mode, id);
    let input = text_input(hint, value)
        .id(fid)
        .on_input(move |s| Msg::Bc(BMsg::Field(id, s)))
        .on_submit(Msg::Bc(BMsg::Ok))
        .secure(secret)
        .size(15.0)
        .padding(Padding { top: 10.0, right: 14.0, bottom: 10.0, left: 14.0 })
        .style(move |_t, st| {
            let focused = matches!(st, text_input::Status::Focused { .. });
            text_input::Style {
                background: Background::Color(p.panel3),
                border: Border { color: if focused || on { p.accent } else { Color::TRANSPARENT }, width: 2.0, radius: p.r(10.0).into() },
                icon: p.muted,
                placeholder: p.muted,
                value: p.text,
                selection: Pal::a(p.accent, 0.35),
            }
        });
    input.into()
}

fn check<'a>(a: &'a App, id: &str, text: &str, on: bool, dim: bool) -> Element<'a, Msg> {
    let p = a.p;
    let lt = lit(a, id);
    let box_ = container(if on { t1("✓", 13.0, p.ink).font(ui::weight(iced::font::Weight::Bold)) } else { t1(" ", 13.0, p.ink) })
        .center_x(Length::Fixed(20.0))
        .center_y(Length::Fixed(20.0))
        .style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 5.0, Some((if on { p.accent } else { p.muted }, 2.0))));
    let r = row![box_, t1(text.to_string(), 14.0, if dim { Pal::a(p.text, 0.5) } else { p.text })].spacing(10.0).align_y(iced::Center).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 });
    let i = rows(a).iter().position(|r| r.1 == id).unwrap_or(0);
    mouse_area(container(r).width(Length::Fill).height(Length::Fixed(40.0)).center_y(Length::Fixed(40.0)).style(row_style(p, lt, Color::TRANSPARENT)))
        .on_press(Msg::Bc(BMsg::Press(i, 1)))
        .interaction(mouse::Interaction::Pointer)
        .into()
}

fn picker<'a>(a: &'a App, id: &str, text: String, swatch: Option<Color>, dim: bool) -> Element<'a, Msg> {
    let p = a.p;
    let lt = lit(a, id);
    let i = rows(a).iter().position(|r| r.1 == id).unwrap_or(0);
    let mut mid = Row::new().spacing(8.0).align_y(iced::Center);
    if let Some(c) = swatch {
        mid = mid.push(container(Space::new().width(Length::Fixed(14.0)).height(Length::Fixed(14.0))).style(ui::boxed(c, 7.0, None)));
    }
    mid = mid.push(t1(text, 14.0, if dim { Pal::a(p.text, 0.5) } else { p.text }));
    let prev = mouse_area(container(t1("‹", 16.0, p.muted)).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 12.0 })).on_press(Msg::Bc(BMsg::Press(i, -1))).interaction(mouse::Interaction::Pointer);
    let next = mouse_area(container(t1("›", 16.0, p.muted)).padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 10.0 })).on_press(Msg::Bc(BMsg::Press(i, 1))).interaction(mouse::Interaction::Pointer);
    let body = mouse_area(container(mid).center_x(Length::Fill)).on_press(Msg::Bc(BMsg::Press(i, 1))).interaction(mouse::Interaction::Pointer);
    container(row![prev, body, next].align_y(iced::Center))
        .width(Length::Fill)
        .height(Length::Fixed(40.0))
        .center_y(Length::Fixed(40.0))
        .style(move |_: &Theme| container::Style { background: Some(Background::Color(p.panel3)), border: Border { color: if lt { p.accent } else { Color::TRANSPARENT }, width: 2.0, radius: p.r(10.0).into() }, ..container::Style::default() })
        .into()
}

pub fn dialog(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let b = &a.bc;
    let mode = b.mode.as_str();
    let title = match mode {
        "password" => format!("Unlock “{}”", b.pw.name),
        "setlock" => format!("Lock “{}” with a password", b.pw.name),
        "passwd" => format!("Change the password of “{}”", b.pw.name),
        "pick" => b.pick.as_ref().map(|x| x.title.clone()).unwrap_or_default(),
        "update" => b.plan.as_ref().map(|x| format!("Update “{}”", sv(x, "name"))).unwrap_or_default(),
        "customize" => b.cust.as_ref().map(|c| { let n = sv(&c.look, "name"); format!("Customize “{}”", if n.is_empty() { base(&c.path) } else { n }) }).unwrap_or_default(),
        "job" => match b.job.as_ref().map(|j| j.verb.as_str()).unwrap_or("") {
            "update" => "Updating…",
            "add" => "Adding to the Briefcase…",
            "send" => "Sending the Briefcase…",
            "lock" => "Encrypting and checking…",
            "unlock" => "Decrypting and checking…",
            "move" => "Moving…",
            "copy" => "Copying…",
            "swap" => "Swapping…",
            "sync" => "Syncing…",
            _ => "Working…",
        }
        .to_string(),
        "message" => b.msg.as_ref().map(|m| m.0.clone()).unwrap_or_default(),
        _ => String::new(),
    };
    let mut head = Row::new().spacing(12.0).align_y(iced::Center);
    if mode == "customize" {
        if let Some(c) = &b.cust {
            head = head.push(bc_icon(a, &c.look, c.locked, 44.0));
        }
    }
    if ["password", "setlock", "passwd"].contains(&mode) {
        head = head.push(ico::<Msg>(Ic::Lock, p.accent, 26.0));
    }
    head = head.push(container(tb(ui::fit(&title, 60), 20.0, p.text)).width(Length::Fill));
    let mut col = Column::new().spacing(12.0).push(head);
    let wide = mode == "update" || mode == "customize";
    match mode {
        "password" | "setlock" | "passwd" => {
            if mode == "setlock" {
                col = col.push(t("If you forget this password, the files inside can't be recovered. Hearth can't reset it.", 14.0, p.danger).font(ui::weight(iced::font::Weight::Semibold)));
                col = col.push(t("The files are encrypted inside the Briefcase folder and checked; only then does the unlocked copy go to the Trash. It opens with the same password on a USB stick or another Hearth device.", 13.0, p.muted));
            }
            if mode == "password" {
                col = col.push(t("This Briefcase is locked: its files are encrypted. Type its password to open it.", 13.0, p.muted));
            }
            col = col.push(field(a, "a", &b.pw.a, if mode == "passwd" { "Old password" } else if mode == "setlock" { "New password" } else { "Password" }, true));
            if mode != "password" {
                col = col.push(field(a, "b", &b.pw.b, if mode == "passwd" { "New password" } else { "The same password again" }, true));
            }
            if mode == "passwd" {
                col = col.push(field(a, "c", &b.pw.c, "The new password again", true));
            }
            if mode != "passwd" {
                col = col.push(check(a, "remember", "Remember on this device (kept in the system keyring)", b.pw.remember, false));
            }
            if !b.pw.error.is_empty() {
                col = col.push(t(b.pw.error.clone(), 13.0, if b.pw.error.ends_with('…') { p.muted } else { p.danger }));
            }
        }
        "pick" => {
            let mut list = Column::new().spacing(2.0);
            for (i, r) in b.pick.as_ref().map(|x| x.list.clone()).unwrap_or_default().into_iter().enumerate() {
                let on = lit(a, &i.to_string());
                let mut rr = Row::new().spacing(12.0).align_y(iced::Center);
                if r.look.is_object() {
                    rr = rr.push(bc_icon(a, &r.look, r.locked, 30.0));
                }
                rr = rr.push(column![t1(r.label.clone(), 15.0, p.text), t1(ui::fit(&r.sub, 70), 12.0, p.muted)].spacing(0.0).width(Length::Fill));
                let cell = container(rr.padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 12.0 })).width(Length::Fill).height(Length::Fixed(54.0)).center_y(Length::Fixed(54.0)).style(row_style(p, on, Color::TRANSPARENT));
                list = list.push(mouse_area(cell).on_press(Msg::Bc(BMsg::Press(i, 1))).interaction(mouse::Interaction::Pointer));
            }
            col = col.push(scrollable(list).height(Length::Shrink));
        }
        "update" => {
            col = col.push(t("Hearth Files will do this. Change any row (click it, or Left / Right), then press Update. Replaced versions go to the Trash.", 13.0, p.muted));
            let plan = b.plan.clone().unwrap_or(Value::Null);
            let mut list = Column::new().spacing(6.0);
            for (i, r) in plan.get("rows").and_then(Value::as_array).cloned().unwrap_or_default().iter().enumerate() {
                let on = lit(a, &i.to_string());
                let kind = sv(r, "kind");
                let kname = match kind.as_str() {
                    "newer-b" => "Newer in Briefcase",
                    "new-b" => "New in Briefcase",
                    "newer-o" => "Newer at original",
                    "new-o" => "New at original",
                    "conflict" => "Changed in both",
                    "deleted-o" => "Deleted at original: keep or delete?",
                    "deleted-b" => "Deleted in Briefcase: keep or delete?",
                    "changed-b-deleted-o" => "Changed here, deleted at original",
                    "changed-o-deleted-b" => "Changed at original, deleted here",
                    k => k,
                }
                .to_string();
                let act = sv(r, "action");
                let aname = match act.as_str() {
                    "to-original" => "Replace →",
                    "to-briefcase" => "← Replace",
                    "skip" => "Skip",
                    "keep-both" => "Keep both",
                    "delete-briefcase" => "Delete here too",
                    "delete-original" => "Delete there too",
                    "adopt" => "Same",
                    "forget" => "Forget",
                    x => x,
                }
                .to_string();
                let red = kind.starts_with("conflict") || kind.starts_with("changed-");
                let item = format!("{}{}", sv(r, "item"), if sv(r, "rel").is_empty() { String::new() } else { format!(" › {}", sv(r, "rel")) });
                let (abg, afg) = if act == "skip" { (p.panel3, p.text) } else if act.starts_with("delete") { (p.danger, Color::WHITE) } else { (p.accent, p.ink) };
                let chip = mouse_area(container(tb(format!("‹  {aname}  ›"), 13.0, afg)).padding(Padding { top: 0.0, right: 14.0, bottom: 0.0, left: 14.0 }).center_y(Length::Fixed(34.0)).style(ui::boxed(abg, p.r(17.0), None)))
                    .on_press(Msg::Bc(BMsg::Press(i, 1)))
                    .interaction(mouse::Interaction::Pointer);
                let rr = row![column![t1(ui::fit(&item, 70), 14.0, p.text), t1(kname, 12.0, if red { p.danger } else { p.muted })].spacing(0.0).width(Length::Fill), chip]
                    .spacing(12.0)
                    .align_y(iced::Center)
                    .padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 14.0 });
                list = list.push(container(rr).width(Length::Fill).height(Length::Fixed(52.0)).center_y(Length::Fixed(52.0)).style(row_style(p, on, p.panel2)));
            }
            for x in plan.get("info").and_then(Value::as_array).cloned().unwrap_or_default() {
                list = list.push(t(format!("“{}”: {}{}", sv(&x, "item"), sv(&x, "label"), if sv(&x, "status") == "away" { " · it waits until then" } else { "" }), 13.0, p.muted));
            }
            col = col.push(scrollable(list).height(Length::Shrink));
        }
        "customize" => {
            if let Some(c) = &b.cust {
                let l = &c.look;
                let dim = !c.locked;
                let color_name = match sv(l, "color").as_str() {
                    "" | "accent" => "Your accent".to_string(),
                    "ember" => "Ember".into(),
                    "red" => "Red".into(),
                    "green" => "Green".into(),
                    "blue" => "Blue".into(),
                    "purple" => "Purple".into(),
                    "gold" => "Gold".into(),
                    "slate" => "Slate".into(),
                    x => x.to_string(),
                };
                let icon_name = match sv(l, "icon").as_str() {
                    "satchel" => "Satchel",
                    "suitcase" => "Suitcase",
                    "backpack" => "Backpack",
                    "box" => "Box",
                    _ => "Classic briefcase",
                };
                let view_name = match sv(l, "view").as_str() {
                    "large" => "Large icons",
                    "grid" => "Medium icons",
                    "small" => "Small icons",
                    "tiles" => "Tiles",
                    "compact" => "List",
                    "details" => "Details",
                    "xlarge" => "Extra large icons",
                    _ => "Like other folders",
                };
                let sort_name = match sv(l, "sort").as_str() {
                    "name" => "Name",
                    "date" => "Modified",
                    "size" => "Size",
                    "type" => "Type",
                    _ => "Like other folders",
                };
                let mut grid = Column::new().spacing(8.0);
                grid = grid.push(grow(&p, false, "Name", field(a, "name", &sv(l, "name"), "", false)));
                grid = grid.push(grow(&p, false, "Colour", picker(a, "color", color_name, Some(color_of(a, l)), false)));
                grid = grid.push(grow(&p, false, "Icon", picker(a, "icon", icon_name.into(), None, false)));
                grid = grid.push(grow(&p, false, "Badge", field(a, "badge", &sv(l, "badge"), "An emoji or up to 4 letters (optional)", false)));
                grid = grid.push(grow(&p, false, "Note", field(a, "note", &sv(l, "note"), "Shown in its details pane (optional)", false)));
                grid = grid.push(grow(&p, false, "View", picker(a, "view", view_name.into(), None, false)));
                grid = grid.push(grow(&p, false, "Sort", picker(a, "sort", sort_name.into(), None, false)));
                grid = grid.push(grow(&p, false, "Auto-sync", picker(a, "autosync", SYNC[sync_index(c)].2.to_string(), None, false)));
                if c.sync_custom {
                    let m = l.get("autosync").and_then(|x| x.get("minutes")).map(|v| v.to_string()).unwrap_or_default();
                    grid = grid.push(grow(&p, false, "Every", field(a, "syncmin", &m, "minutes", false)));
                }
                let lock_text = if c.locked { LOCKS[lock_index(c)].1.to_string() } else { "Needs a password (Lock this Briefcase…)".into() };
                grid = grid.push(grow(&p, dim, "Auto-lock", picker(a, "autolock", lock_text, None, dim)));
                if c.lock_custom && c.locked {
                    let m = l.get("autolock").and_then(|x| x.get("minutes")).map(|v| v.to_string()).unwrap_or_default();
                    grid = grid.push(grow(&p, false, "After", field(a, "lockmin", &m, "minutes", false)));
                }
                let al = l.get("autolock").cloned().unwrap_or(Value::Null);
                grid = grid.push(grow(&p, false, "", check(a, "onSleep", "Lock when the PC sleeps", bv(&al, "onSleep"), dim)));
                grid = grid.push(grow(&p, false, "", check(a, "onEject", "Lock when the USB stick is ejected", bv(&al, "onEject"), dim)));
                grid = grid.push(grow(&p, false, "", check(a, "showInNav", "Show in navigation pane", bv(l, "showInNav"), false)));
                col = col.push(grid);
                col = col.push(t("Kept in the Briefcase itself, so it looks the same on a USB stick or another Hearth device. The files inside aren't touched.", 12.0, p.muted));
            }
        }
        "job" => {
            let j = b.job.clone();
            let frac = j.as_ref().map(|j| if j.total > 0.0 { (j.done / j.total).min(1.0) as f32 } else { 0.05 }).unwrap_or(0.05);
            col = col.push(container(ui::bar(vec![(frac, p.accent)], p.panel3, 8.0, 4.0)).width(Length::Fill));
            let line = match &j {
                Some(j) if j.cancel => "Stopping…".to_string(),
                Some(j) => format!("{}{}", j.file, if j.total > 0.0 { format!("   {} of {}", model::fmt_size(j.done as u64), model::fmt_size(j.total as u64)) } else { String::new() }),
                None => String::new(),
            };
            col = col.push(t(line, 13.0, p.muted));
        }
        "message" => {
            col = col.push(t(b.msg.as_ref().map(|m| m.1.clone()).unwrap_or_default(), 13.0, p.muted));
        }
        _ => {}
    }
    let rs = rows(a);
    let has_field = rs.iter().any(|r| r.0 == RK::Field);
    let hint = format!("Ⓐ choose · ◀ ▶ change · Ⓑ {}{}", if mode == "job" { "stop" } else { "cancel" }, if has_field { " · Ⓨ keyboard" } else { "" });
    let mut btns = Row::new().spacing(10.0).align_y(iced::Center).push(container(t1(hint, 12.0, p.muted)).width(Length::Fill).clip(true));
    let ci = rs.iter().position(|r| r.1 == "cancel").unwrap_or(0);
    let oi = rs.iter().position(|r| r.1 == "ok").unwrap_or(0);
    if mode != "message" {
        btns = btns.push(pill(&p, if mode == "job" { "Stop".into() } else { "Cancel".into() }, false, lit(a, "cancel"), Msg::Bc(BMsg::Press(ci, 1))));
    }
    if mode != "pick" && mode != "job" {
        let l = match mode {
            "password" => "Unlock",
            "setlock" => "Lock",
            "passwd" => "Change",
            "update" => "Update",
            "customize" => "Save",
            _ => "OK",
        };
        btns = btns.push(pill(&p, l.into(), true, lit(a, "ok"), Msg::Bc(BMsg::Press(oi, 1))));
    }
    col = col.push(container(btns).padding(Padding { top: 6.0, ..Padding::ZERO }));
    let card = container(scrollable(col.padding(22.0)).height(Length::Shrink))
        .width(Length::Fixed(if wide { 760.0 } else { 560.0 }))
        .max_height(720.0)
        .style(ui::boxed(p.panel, p.r(14.0), Some((p.line, 1.0))));
    let _ = gap::<Msg>(1.0, 1.0);
    card.into()
}

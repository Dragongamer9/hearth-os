//! Every KDE System Settings module (KCM) that Hearth Settings had no page for, as a page in Hearth's look (the
//! owner, Nov 19, 2026: "Build them all into Hearth Settings", so KDE's own System Settings is never needed on the
//! Fedora edition). The pages and their rows are described once, in hosts/linux/desktop/shell/kde-settings.json:
//! this module draws them and hearth-kde-settings (the same folder) reads and changes KDE's own files the way each
//! KCM does (only the keys a page lists, only the keys changed: atomic, a .bak kept, a file that can't be read never
//! written over; then KDE's "config changed" signal and the program that reads it asked to read it again). Where a
//! KCM applies a choice with one of KDE's tools (plasma-apply-colorscheme, ...), the page runs the same tool.
//! Opening a page only reads. On labwc the pages still read and save KDE's files and say which only KDE Plasma uses.
//! Every KCM and its Hearth page: docs/mockups/checklists/settings-kde-all.md.

use crate::cats::apps::kit::{choice, choice_wide, page as column_of, read, read_in, say_view, small, sw, words, Res, Say};
use crate::nav::{self, Kind};
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::ui::{t, tm, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, scrollable, Column, Space};
use iced::{mouse, Border, Color, Element, Length, Padding, Subscription, Task, Theme};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::OnceLock;

/// The pages, as kde-settings.json lists them (a test checks they agree).
pub const PAGES: &[&str] = &[
    "kde-settings", "kde-workspace", "kde-style", "kde-plasma-style", "kde-colors", "kde-global-theme",
    "kde-decorations", "kde-screenlock", "kde-session", "kde-renderer", "kde-feedback", "kde-tabbox", "kde-krunner",
    "kde-webshortcuts", "kde-updates", "kde-trash", "kde-netpref", "kde-kwin-scripts", "kde-kded", "kde-activities",
    "kde-locations", "kde-device-actions", "kde-nighttime", "kde-plasmakeyboard", "kde-gamecontroller", "kde-cddb",
    "kde-krdp", "kde-info", "kde-shortcuts", "kde-push", "kde-colord", "kde-fontinst", "kde-store", "kde-hdr",
];

#[path = "kde_pad.rs"]
pub mod pad_test;

/// The page on the screen now (kept by dialog(), which the frame asks every time it draws): the live controller test
/// listens only while its page is shown.
static SHOWN: std::sync::Mutex<&'static str> = std::sync::Mutex::new("");

const SPEC_TEXT: &str = include_str!("../../../../../linux/desktop/shell/kde-settings.json");

fn spec() -> &'static Value {
    static S: OnceLock<Value> = OnceLock::new();
    S.get_or_init(|| serde_json::from_str(SPEC_TEXT).unwrap_or(Value::Null))
}

fn page_spec(id: &str) -> Option<&'static Value> {
    spec().get("pages")?.as_array()?.iter().find(|p| p.get("id").and_then(Value::as_str) == Some(id))
}

fn rows_of(id: &str) -> &'static [Value] {
    page_spec(id).and_then(|p| p.get("rows")).and_then(Value::as_array).map(|v| v.as_slice()).unwrap_or(&[])
}

fn row_of(id: &str, i: usize) -> Option<&'static Value> {
    rows_of(id).get(i)
}

fn s<'v>(v: &'v Value, k: &str) -> &'v str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

/// A value as KDE's text ("true", "5", "Breeze").
fn text_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        x => x.to_string(),
    }
}

/// Runs a helper with a password on its stdin only (not in its environment or on its command line), off the window's
/// thread; its last JSON line or why it failed.
fn secret_run<M: Send + 'static>(cmd: Vec<String>, secret: String, f: impl Fn(Res) -> M + Send + 'static) -> Task<M> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-settings-secret".into()).spawn(move || {
        let mut res = Res { ok: false, v: Value::Null, err: "It didn't start".into(), out: String::new() };
        if let Ok(mut child) = Command::new(&cmd[0]).args(&cmd[1..]).env_remove("HEARTH_INPUT").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            if let Some(mut sin) = child.stdin.take() {
                let _ = sin.write_all(secret.as_bytes());
            }
            drop(secret);
            if let Ok(o) = child.wait_with_output() {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                let err = String::from_utf8_lossy(&o.stderr).to_string();
                let v = out.lines().rev().find_map(|l| serde_json::from_str::<Value>(l).ok()).unwrap_or(Value::Null);
                res = Res { ok: o.status.success(), v, err: err.trim().lines().last().unwrap_or("").to_string(), out };
            }
        }
        let _ = tx.send(res);
    });
    Task::perform(async move { rx.await.unwrap_or_default() }, f)
}

fn helper(args: &[&str], secs: u32) -> Vec<String> {
    crate::cats::apps::kit::py("hearth-kde-settings", args, secs)
}

/// The page's id as the registry's own (`'static`) string.
fn sid(id: &str) -> &'static str {
    crate::registry::page(id).map(|p| p.id).unwrap_or("home")
}

fn icon(name: &str) -> G {
    match name {
        "folder" => G::Folder,
        "mouse" => G::Mouse,
        "keyboard" => G::Keyboard,
        "list" => G::List,
        "info" => G::Info,
        "display" => G::Display,
        "pad" => G::Pad,
        "rocket" => G::Rocket,
        "clock" => G::Clock,
        "grid" => G::Grid,
        "palette" => G::Palette,
        "menu" => G::Menu,
        "moon" => G::Moon,
        "close" => G::Close,
        "maximize" => G::Maximize,
        "minimize" => G::Minimize,
        "shield" => G::Shield,
        "camera" => G::Camera,
        "play" => G::Play,
        "user" => G::User,
        "users" => G::Users,
        "power" => G::Power,
        "cog" => G::Cog,
        "sound" => G::Sound,
        "drive" => G::Drive,
        "wifi" => G::Wifi,
        "search" => G::Search,
        "store" => G::Store,
        "link" => G::Link,
        "server" => G::Server,
        "update" => G::Update,
        "access" => G::Access,
        "tasks" => G::Tasks,
        "pulse" => G::Pulse,
        "bt" => G::Bt,
        _ => G::Gear,
    }
}

/// One page's reads and what's going on with it.
#[derive(Default)]
pub struct Pg {
    /// hearth-kde-settings get PAGE: {files, mine, locked, bad, lists, info, current, status}.
    pub data: Value,
    pub read: bool,
    pub saving: bool,
    pub say: Say,
    /// A slider being moved (saved when it's let go).
    pub temp: HashMap<usize, f32>,
    /// A text field being typed in (saved with Save or Enter).
    pub text: HashMap<usize, String>,
}

#[derive(Default)]
pub struct State {
    pub pages: HashMap<&'static str, Pg>,
    /// `--page PAGE:pick`: the page's first list opens once it's read (pictures, tests).
    pub want_pick: bool,
    /// `--page PAGE:end`: the page scrolls to its end once it's read (pictures of long pages).
    pub want_end: bool,
    /// Shortcuts: waiting for the new keys of (page, component, action).
    pub capture: Option<(&'static str, String, String)>,
    /// Shortcuts: the component shown (its id).
    pub open_comp: Option<String>,
    /// Font management: fonts loaded for their preview (family names, kept for the app's life), and the one shown.
    pub fonts_loaded: Vec<&'static str>,
    pub font_shown: Option<String>,
    /// Device actions: the one being edited (file, action).
    pub editing: Option<(String, String)>,
    /// Controllers in KDE: what the live test read last.
    pub pads: Vec<pad_test::Pad>,
    /// Calibrate HDR.
    pub hdr: Hdr,
    /// Activities: waiting for an activity's shortcut keys (its id).
    pub act_capture: Option<(&'static str, String)>,
    /// "Get New…": the kind, what the store answered, the search, the order, and what's being installed.
    pub store_kind: &'static str,
    pub store: Value,
    pub store_q: String,
    pub store_sort: &'static str,
    pub store_busy: String,
    pub store_say: Say,
    /// `--page PAGE:part` for the pictures: calibrate (HDR's steps), preview (a font), edit (a device action).
    pub want: String,
    /// A long list open over the page: (its page, its row).
    pub pick: Option<(&'static str, usize)>,
}

/// Calibrate HDR (Hearth's own three steps, as Mantel's HdrCalibrate.qml): the brightest small spot (peak), the
/// brightest whole screen (average) and the darkest (lowest), each with its test picture over the whole window, the
/// screen changing as you move it (hearth-screens apply: KWin's brightness overrides, nothing saved); Finish asks
/// "Keep these settings?" for 15 s (a wrong one goes back by itself); Cancel puts back what was there.
#[derive(Default)]
pub struct Hdr {
    pub info: Value,
    pub name: String,
    pub on: bool,
    pub step: usize,
    pub vals: [f32; 3],
    pub start0: Value,
    pub keep: Option<u32>,
    pub say: Say,
    /// B / Esc in the calibration: put back what was there (sent once from the subscription).
    pub cancel_due: bool,
}

pub const HDR_STEPS: [(&str, &str, f32, f32, &str); 3] = [
    ("Brightest spot", "Move it down until the small square in the middle only just stands out from the box around it.", 100.0, 2000.0, "nits"),
    ("Brightest across the screen", "Move it down until the white looks as bright as it can without flickering or dimming after a moment.", 80.0, 1500.0, "nits"),
    ("Darkest", "Move it until you can just see every one of the dark squares, and the background stays black.", 0.0, 0.5, "nits"),
];

#[derive(Debug, Clone)]
pub enum Msg {
    Got(&'static str, Res),
    /// A row's new value (a switch's "true" / "false", a choice's option number, a list's value).
    Switch(&'static str, usize, bool),
    Opt(&'static str, usize, usize),
    Picked(&'static str, usize, String),
    Pick(Option<(&'static str, usize)>),
    Num(&'static str, usize, f32),
    NumDone(&'static str, usize),
    Text(&'static str, usize, String),
    TextSave(&'static str, usize),
    File(&'static str, usize),
    FileGot(&'static str, usize, Res),
    Firmware(&'static str, bool),
    Saved(&'static str, usize, Res),
    Applied(&'static str, Res),
    Go(&'static str),
    /// A KWin script (any `<id>Enabled` plugin row) on or off.
    Plugin(&'static str, usize, String, bool),
    /// A KDE background service on or off (now and at sign-in).
    Kded(&'static str, String, bool),
    /// One of the helper's own commands for this page (activity add NAME, userdir-set KEY PATH, ...), then read again.
    Run(&'static str, Vec<String>, Option<String>),
    Folder(&'static str, String),
    FolderGot(&'static str, String, Res),
    /// A file to hand to one of the helper's commands (a font to install, a colour profile to import).
    FileFor(&'static str, &'static str, &'static str),
    /// Shortcuts: wait for the new keys of this one (component, action); the keys pressed.
    Capture(&'static str, String, String),
    Key(iced::keyboard::Event),
    OpenComp(Option<String>),
    End,
    FontShow(String, String),
    FontBytes(String, Option<Vec<u8>>),
    FontLoaded(String, bool),
    Edit(Option<(String, String)>),
    Pads(Vec<pad_test::Pad>),
    HdrGot(Res),
    HdrStart,
    HdrSet(f32),
    HdrApply,
    HdrStep(i32),
    HdrReset,
    HdrFinish,
    HdrCancel,
    HdrApplied(Res),
    HdrKeep(bool),
    HdrTick,
    /// A helper command that gets a password typed on the page (on its stdin only): (page, args, the field's key).
    Secret(&'static str, Vec<String>, usize),
    /// Activities: wait for the keys of this activity's own shortcut.
    ActCapture(&'static str, String),
    /// "Get New…": the KDE Store for a kind (lookandfeel, colorschemes, plasma-themes, aurorae).
    Store(&'static str),
    StoreGot(Res),
    StoreSearch(String),
    StoreSort(&'static str),
    StoreFind,
    StoreRun(Vec<String>),
    StoreDone(Res),
    Pics,
    FileForGot(&'static str, &'static str, Res),
}

pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    st.want_pick = sh.part == "pick";
    st.want_end = sh.part == "end";
    st.want = sh.part.clone();
    if page == "kde-store" {
        st.store_kind = match sh.part.as_str() {
            "colorschemes" => "colorschemes",
            "plasma-themes" => "plasma-themes",
            "aurorae" => "aurorae",
            _ => "lookandfeel",
        };
        if st.store_sort.is_empty() {
            st.store_sort = "down";
        }
        let (k, q, o) = (st.store_kind.to_string(), st.store_q.clone(), st.store_sort.to_string());
        return Task::batch([read(helper(&["get", page], 30), move |r| Msg::Got(page, r)), read(helper(&["store", &k, &q, &o], 90), Msg::StoreGot)]);
    }
    // `--page kde-shortcuts:COMPONENT`: that app's shortcuts open.
    if page == "kde-shortcuts" && !sh.part.is_empty() && sh.part != "pick" && sh.part != "end" {
        st.open_comp = Some(sh.part.clone());
    }
    let pg = st.pages.entry(page).or_default();
    pg.say.clear();
    pg.temp.clear();
    st.pick = None;
    let t = read(helper(&["get", page], 30), move |r| Msg::Got(page, r));
    if page == "kde-hdr" {
        st.hdr.say.clear();
        return Task::batch([t, read(crate::cats::apps::kit::py("hearth-screens", &["info"], 30), Msg::HdrGot)]);
    }
    t
}

/// The first screen with HDR on that KWin knows the brightness of (or the one picked before).
fn hdr_out(h: &Hdr) -> Option<&Value> {
    let outs = h.info.get("outputs").and_then(Value::as_array)?;
    outs.iter().find(|o| s(o, "name") == h.name).or_else(|| outs.iter().find(|o| o.pointer("/hdrCal/capable").and_then(Value::as_bool).unwrap_or(false)))
}

fn hdr_apply(h: &Hdr, cal: Value) -> Task<Msg> {
    let mut outs = Map::new();
    outs.insert(h.name.clone(), serde_json::json!({ "hdrCal": cal }));
    let arg = serde_json::json!({ "outputs": outs }).to_string();
    read(crate::cats::apps::kit::py("hearth-screens", &["apply", &arg], 30), Msg::HdrApplied)
}

fn hdr_now(h: &Hdr) -> Value {
    serde_json::json!({"peak": h.vals[0].round(), "avg": h.vals[1].round().min(h.vals[0].round()), "min": (h.vals[2] * 10000.0).round() / 10000.0})
}

fn win_mode(full: bool) -> Task<Msg> {
    iced::window::latest().and_then(move |id| iced::window::set_mode(id, if full { iced::window::Mode::Fullscreen } else { iced::window::Mode::Windowed }))
}

// ---------------------------------------------------------------- reading the page's values

/// The value KDE uses for file [group] key (yours, else the system's), or the row's default.
fn val(pg: &Pg, file: &str, group: &str, key: &str, def: &Value) -> String {
    match pg.data.get("files").and_then(|f| f.get(file)).and_then(|f| f.get(group)).and_then(|g| g.get(key)) {
        Some(v) => text_of(v),
        None => text_of(def),
    }
}

fn row_val(pg: &Pg, r: &Value) -> String {
    let def = r.get("def").cloned().unwrap_or(Value::Null);
    if let Some(cur) = r.get("current").and_then(Value::as_str) {
        return pg.data.get("current").and_then(|c| c.get(cur)).map(text_of).unwrap_or_default();
    }
    let v = val(pg, s(r, "file"), s(r, "group"), s(r, "key"), &def);
    if !s(r, "key2").is_empty() {
        let d2 = text_of(&def);
        let d2 = d2.split_once('|').map(|x| x.1.to_string()).unwrap_or_default();
        let v2 = val(pg, s(r, "file"), s(r, "group"), s(r, "key2"), &Value::String(d2));
        return format!("{v}|{v2}");
    }
    v
}

fn truthy(v: &str) -> bool {
    v.eq_ignore_ascii_case("true") || v == "1"
}

/// A switch row: on now? ("onoff": [on, off] values for keys that aren't true / false; "invert": the key says off).
fn switch_on(pg: &Pg, r: &Value) -> bool {
    let cur = row_val(pg, r);
    let on = match r.get("onoff").and_then(Value::as_array) {
        Some(v) => v.first().map(text_of).as_deref() == Some(cur.as_str()),
        None => truthy(&cur),
    };
    on != r.get("invert").and_then(Value::as_bool).unwrap_or(false)
}

/// What a switch row writes to be on / off.
fn switch_value(r: &Value, on: bool) -> Value {
    let key_on = on != r.get("invert").and_then(Value::as_bool).unwrap_or(false);
    match r.get("onoff").and_then(Value::as_array) {
        Some(v) => Value::String(v.get(if key_on { 0 } else { 1 }).map(text_of).unwrap_or_default()),
        None => Value::Bool(key_on),
    }
}

/// The default a key has on this page (the row that changes it), for "needs".
fn def_of(page: &str, file: &str, group: &str, key: &str) -> Value {
    rows_of(page).iter().find(|r| s(r, "file") == file && s(r, "group") == group && s(r, "key") == key).and_then(|r| r.get("def")).cloned().unwrap_or(Value::Null)
}

/// A row that only counts when another setting is set a way (Border size only with the theme's own size off).
fn needs_ok(page: &str, pg: &Pg, r: &Value) -> bool {
    let Some(n) = r.get("needs").and_then(Value::as_array) else { return true };
    let f: Vec<&str> = n.iter().filter_map(Value::as_str).collect();
    if f.len() < 4 {
        return true;
    }
    let v = val(pg, f[0], f[1], f[2], &def_of(page, f[0], f[1], f[2]));
    if f[3] == "true" || f[3] == "false" {
        truthy(&v) == (f[3] == "true")
    } else {
        v == f[3]
    }
}

fn locked(pg: &Pg, r: &Value) -> bool {
    pg.data.get("locked").and_then(|l| l.get(s(r, "file"))).and_then(|l| l.get(s(r, "group"))).and_then(Value::as_array).map(|a| a.iter().any(|k| k.as_str() == Some(s(r, "key")) || k.as_str() == Some("*"))).unwrap_or(false)
}

fn plasma_now(pg: &Pg) -> bool {
    pg.data.pointer("/status/plasma").and_then(Value::as_bool).unwrap_or(false)
}

/// A choice's option that's chosen now: its value matches and so do the other keys it sets.
fn chosen(pg: &Pg, r: &Value) -> Option<usize> {
    let opts = r.get("opts")?.as_array()?;
    let cur = row_val(pg, r);
    opts.iter().position(|o| {
        let Some(a) = o.as_array() else { return false };
        let v = a.first().map(text_of).unwrap_or_default();
        let same = v == cur || (truthy(&v) && truthy(&cur)) || (v.eq_ignore_ascii_case("false") && cur.eq_ignore_ascii_case("false"));
        let extras = a.get(2).and_then(Value::as_object).map(|m| m.iter().all(|(k, ev)| val(pg, s(r, "file"), s(r, "group"), k, &def_of_extra(r, k)) == text_of(ev))).unwrap_or(true);
        same && extras
    })
}

/// The value an option's other key has when nobody set it: the first option's (KDE's default layout).
fn def_of_extra(r: &Value, k: &str) -> Value {
    let def = text_of(r.get("def").unwrap_or(&Value::Null));
    r.get("opts").and_then(Value::as_array).and_then(|o| o.iter().find(|o| o.get(0).map(text_of).as_deref() == Some(def.as_str()))).and_then(|o| o.get(2)).and_then(|m| m.get(k)).cloned().unwrap_or(Value::Null)
}

fn list(pg: &Pg, r: &Value) -> Vec<(String, String)> {
    let none: Vec<(String, String)> = r.get("none").and_then(Value::as_str).map(|l| vec![(String::new(), l.to_string())]).unwrap_or_default();
    let mut v = none;
    v.extend(list_only(pg, r));
    v
}

fn list_only(pg: &Pg, r: &Value) -> Vec<(String, String)> {
    pg.data
        .get("lists")
        .and_then(|l| l.get(s(r, "from")))
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_array()).map(|x| (x.first().map(text_of).unwrap_or_default(), x.get(1).map(text_of).unwrap_or_default())).collect())
        .unwrap_or_default()
}

fn label_of(items: &[(String, String)], v: &str) -> String {
    items.iter().find(|(k, _)| k == v).map(|(_, l)| l.clone()).unwrap_or_else(|| if v.is_empty() || v == "|" { "Not set".into() } else { v.trim_end_matches('|').split('|').last().unwrap_or(v).trim_start_matches("__aurorae__svg__").to_string() })
}

// ---------------------------------------------------------------- changing

/// {group: {key: value}} for the helper, written to `file`.
fn change(file: &str, group: &str, kv: Map<String, Value>) -> (String, String) {
    let mut outer = Map::new();
    outer.insert(group.to_string(), Value::Object(kv));
    (file.to_string(), Value::Object(outer).to_string())
}

fn save(st: &mut State, page: &'static str, i: usize, file: &str, input: String, shown: Vec<(String, String, String)>) -> Task<Msg> {
    let pg = st.pages.entry(page).or_default();
    // Shown at once (what was picked), saved now.
    for (g, k, v) in shown {
        if pg.data.get("files").is_none() {
            pg.data["files"] = Value::Object(Map::new());
        }
        let files = &mut pg.data["files"];
        if files.get(file).is_none() {
            files[file] = Value::Object(Map::new());
        }
        if files[file].get(&g).is_none() {
            files[file][&g] = Value::Object(Map::new());
        }
        files[file][&g][&k] = Value::String(v);
    }
    pg.saving = true;
    pg.say.clear();
    read_in(helper(&["set", file], 30), Some(input), move |r| Msg::Saved(page, i, r))
}

fn kv(pairs: Vec<(String, Value)>) -> Map<String, Value> {
    pairs.into_iter().collect()
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Go(id) => sh.go(id),
        Msg::Got(page, r) => {
            let pg = st.pages.entry(page).or_default();
            pg.read = true;
            if r.ok {
                pg.data = r.v;
            } else {
                pg.say.bad(if r.err.is_empty() { "KDE's settings couldn't be read".to_string() } else { r.err });
            }
            if std::mem::take(&mut st.want_pick) {
                st.pick = rows_of(page).iter().position(|r| s(r, "type") == "pick").map(|i| (page, i));
            }
            let want = std::mem::take(&mut st.want);
            if want == "preview" {
                let f = st.pages.get(page).and_then(|pg| pg.data.pointer("/current/fonts")).cloned().unwrap_or(Value::Null);
                let first = f.get("files").and_then(Value::as_object).and_then(|m| m.iter().next().map(|(k, v)| (k.clone(), text_of(v))));
                if let Some((fam, file)) = first {
                    return update(sh, st, Msg::FontShow(fam, file));
                }
            }
            if want == "edit" {
                let a = st.pages.get(page).and_then(|pg| pg.data.pointer("/current/deviceactions")).and_then(Value::as_array).and_then(|l| l.iter().find(|x| x.get("system").and_then(Value::as_bool).unwrap_or(false)).cloned());
                if let Some(a) = a {
                    st.editing = Some((s(&a, "file").to_string(), s(&a, "action").to_string()));
                }
            }
            if want.starts_with("calibrate") {
                st.want = want;
            }
            if std::mem::take(&mut st.want_end) {
                return Task::perform(hearth_ui::run::after(400), |_| Msg::End);
            }
        }
        Msg::Switch(page, i, on) => {
            let Some(r) = row_of(page, i) else { return Task::none() };
            if s(r, "type") == "firmware" {
                return update(sh, st, Msg::Firmware(page, on));
            }
            let w = switch_value(r, on);
            let (file, input) = change(s(r, "file"), s(r, "group"), kv(vec![(s(r, "key").into(), w.clone())]));
            sh.reread_history = true;
            return save(st, page, i, &file, input, vec![(s(r, "group").into(), s(r, "key").into(), text_of(&w))]);
        }
        Msg::Opt(page, i, o) => {
            let Some(r) = row_of(page, i) else { return Task::none() };
            let Some(opt) = r.get("opts").and_then(Value::as_array).and_then(|a| a.get(o)).and_then(Value::as_array) else { return Task::none() };
            let v = opt.first().map(text_of).unwrap_or_default();
            let mut pairs = vec![(s(r, "key").to_string(), Value::String(v.clone()))];
            let mut shown = vec![(s(r, "group").to_string(), s(r, "key").to_string(), v)];
            if let Some(extra) = opt.get(2).and_then(Value::as_object) {
                for (k, ev) in extra {
                    pairs.push((k.clone(), Value::String(text_of(ev))));
                    shown.push((s(r, "group").to_string(), k.clone(), text_of(ev)));
                }
            }
            let (file, input) = change(s(r, "file"), s(r, "group"), kv(pairs));
            sh.reread_history = true;
            return save(st, page, i, &file, input, shown);
        }
        Msg::Pick(p) => st.pick = p,
        Msg::Picked(page, i, v) => {
            st.pick = None;
            let Some(r) = row_of(page, i) else { return Task::none() };
            if let Some(tool) = r.get("tool").and_then(Value::as_str) {
                let pg = st.pages.entry(page).or_default();
                pg.saving = true;
                pg.say.clear();
                let tool = tool.to_string();
                return read(helper(&["apply", &tool, &v], 60), move |r| Msg::Applied(page, r));
            }
            let (pairs, shown) = if !s(r, "key2").is_empty() {
                let (a, b) = v.split_once('|').unwrap_or((v.as_str(), ""));
                (vec![(s(r, "key").to_string(), Value::String(a.into())), (s(r, "key2").to_string(), Value::String(b.into()))], vec![(s(r, "group").to_string(), s(r, "key").to_string(), a.to_string()), (s(r, "group").to_string(), s(r, "key2").to_string(), b.to_string())])
            } else {
                (vec![(s(r, "key").to_string(), Value::String(v.clone()))], vec![(s(r, "group").to_string(), s(r, "key").to_string(), v.clone())])
            };
            let (file, input) = change(s(r, "file"), s(r, "group"), kv(pairs));
            sh.reread_history = true;
            return save(st, page, i, &file, input, shown);
        }
        Msg::Num(page, i, v) => {
            st.pages.entry(page).or_default().temp.insert(i, v);
        }
        Msg::NumDone(page, i) => {
            let Some(r) = row_of(page, i) else { return Task::none() };
            let Some(v) = st.pages.entry(page).or_default().temp.remove(&i) else { return Task::none() };
            let text = if r.get("int").and_then(Value::as_bool).unwrap_or(false) { format!("{}", v.round() as i64) } else { format!("{}", (v * 100.0).round() / 100.0) };
            let (file, input) = change(s(r, "file"), s(r, "group"), kv(vec![(s(r, "key").into(), Value::String(text.clone()))]));
            sh.reread_history = true;
            return save(st, page, i, &file, input, vec![(s(r, "group").into(), s(r, "key").into(), text)]);
        }
        Msg::Text(page, i, v) => {
            st.pages.entry(page).or_default().text.insert(i, v);
        }
        Msg::TextSave(page, i) => {
            let Some(r) = row_of(page, i) else { return Task::none() };
            let Some(v) = st.pages.entry(page).or_default().text.remove(&i) else { return Task::none() };
            // A number or a time of day, checked before it's saved (KDE would read anything else as 0 or midnight).
            let v = v.trim().replace('−', "-");
            let v = if r.get("number").and_then(Value::as_bool).unwrap_or(false) {
                match v.replace(',', ".").parse::<f64>() {
                    Ok(n) if n.is_finite() && n.abs() <= 180.0 => format!("{n}"),
                    _ => {
                        st.pages.entry(page).or_default().say.bad(format!("\"{v}\" isn't a number between -180 and 180"));
                        return Task::none();
                    }
                }
            } else if r.get("time").and_then(Value::as_bool).unwrap_or(false) {
                let parts: Vec<u32> = v.split(':').filter_map(|x| x.trim().parse().ok()).collect();
                match parts.as_slice() {
                    [h, m] | [h, m, _] if *h < 24 && *m < 60 => format!("{h:02}:{m:02}:00"),
                    _ => {
                        st.pages.entry(page).or_default().say.bad(format!("\"{v}\" isn't a time like 18:00"));
                        return Task::none();
                    }
                }
            } else {
                v
            };
            let (file, input) = change(s(r, "file"), s(r, "group"), kv(vec![(s(r, "key").into(), Value::String(v.clone()))]));
            sh.reread_history = true;
            return save(st, page, i, &file, input, vec![(s(r, "group").into(), s(r, "key").into(), v)]);
        }
        Msg::File(page, i) => {
            return read(crate::cats::pers::kit::file_picker("Choose a picture", false, "*.jpg *.jpeg *.png *.webp"), move |r| Msg::FileGot(page, i, r));
        }
        Msg::FileGot(page, i, r) => {
            let path = r.out.trim().lines().last().unwrap_or("").trim().to_string();
            if !r.ok || !path.starts_with('/') {
                return Task::none(); // the picker was closed
            }
            let Some(row) = row_of(page, i) else { return Task::none() };
            let v = format!("file://{path}");
            let (file, input) = change(s(row, "file"), s(row, "group"), kv(vec![(s(row, "key").into(), Value::String(v.clone()))]));
            sh.reread_history = true;
            return save(st, page, i, &file, input, vec![(s(row, "group").into(), s(row, "key").into(), v)]);
        }
        Msg::Firmware(page, on) => {
            let pg = st.pages.entry(page).or_default();
            pg.saving = true;
            return read(helper(&["firmware-set", if on { "on" } else { "off" }], 20), move |r| Msg::Applied(page, r));
        }
        Msg::Saved(page, i, r) => {
            let pg = st.pages.entry(page).or_default();
            pg.saving = false;
            if r.ok {
                if let (Some(f), Some(v)) = (r.v.get("file").and_then(Value::as_str), r.v.get("values")) {
                    if pg.data.get("files").is_some() {
                        pg.data["files"][f] = v.clone();
                    }
                    if let Some(l) = r.v.get("locked") {
                        pg.data["locked"][f] = l.clone();
                    }
                }
                // A KCM that applies its scheme again after a change (the accent colour: KDE's colour scheme tool).
                if let Some(after) = row_of(page, i).and_then(|r| r.get("after")).and_then(Value::as_str) {
                    if after == "colorscheme" {
                        let scheme = val(pg, "kdeglobals", "General", "ColorScheme", &Value::String("BreezeLight".into()));
                        pg.saving = true;
                        return read(helper(&["apply", "colorscheme", &scheme], 60), move |r| Msg::Applied(page, r));
                    }
                }
            } else {
                pg.say.bad(r.err);
                return read(helper(&["get", page], 30), move |r| Msg::Got(page, r));
            }
        }
        Msg::Plugin(page, i, id, on) => {
            let Some(r) = row_of(page, i) else { return Task::none() };
            let key = format!("{id}Enabled");
            let (file, input) = change(s(r, "file"), s(r, "group"), kv(vec![(key.clone(), Value::Bool(on))]));
            sh.reread_history = true;
            return save(st, page, i, &file, input, vec![(s(r, "group").into(), key, on.to_string())]);
        }
        Msg::Kded(page, name, on) => {
            let pg = st.pages.entry(page).or_default();
            pg.saving = true;
            sh.reread_history = true;
            return read(helper(&["kded-set", &name, if on { "on" } else { "off" }], 20), move |r| Msg::Applied(page, r));
        }
        Msg::Run(page, args, input) => {
            let pg = st.pages.entry(page).or_default();
            pg.saving = true;
            pg.text.clear();
            sh.reread_history = true;
            if args.first().map(|x| x == "device-action-edit").unwrap_or(false) {
                st.editing = None;
            }
            let a: Vec<&str> = args.iter().map(String::as_str).collect();
            return read_in(helper(&a, 30), input, move |r| Msg::Applied(page, r));
        }
        Msg::Folder(page, key) => {
            return read(crate::cats::pers::kit::file_picker("Choose a folder", true, ""), move |r| Msg::FolderGot(page, key.clone(), r));
        }
        Msg::FolderGot(page, key, r) => {
            let path = r.out.trim().lines().last().unwrap_or("").trim().to_string();
            if r.ok && path.starts_with('/') {
                return update(sh, st, Msg::Run(page, vec!["userdir-set".into(), key, path], None));
            }
        }
        Msg::Capture(page, comp, action) => {
            st.capture = Some((page, comp, action));
        }
        Msg::Key(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
            if let Some((page, id)) = st.act_capture.clone() {
                if matches!(key.as_ref(), iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)) {
                    st.act_capture = None;
                    return Task::none();
                }
                if let Some(text) = shortcut_text(&key, modifiers) {
                    st.act_capture = None;
                    return update(sh, st, Msg::Run(page, vec!["activity".into(), "shortcut".into(), id, text], None));
                }
                return Task::none();
            }
            let Some((page, comp, action)) = st.capture.clone() else { return Task::none() };
            if matches!(key.as_ref(), iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)) {
                st.capture = None;
                return Task::none();
            }
            if let Some(text) = shortcut_text(&key, modifiers) {
                st.capture = None;
                if let Some(file) = comp.strip_prefix("services][").filter(|f| f.starts_with("net.local.")) {
                    // One of your own commands: its keys (the command may have had none yet).
                    return update(sh, st, Msg::Run(page, vec!["command-keys".into(), file.to_string(), text], None));
                }
                return update(sh, st, Msg::Run(page, vec!["shortcut-set".into(), comp, action, text], None));
            }
        }
        Msg::Key(_) => {}
        Msg::OpenComp(c) => st.open_comp = c,
        Msg::End => {
            return iced::widget::operation::scroll_to(iced::widget::Id::new(crate::nav::PAGE), iced::widget::operation::AbsoluteOffset { x: None, y: Some(1.0e6) });
        }
        Msg::Secret(page, args, key) => {
            let pg = st.pages.entry(page).or_default();
            let Some(secret) = pg.text.remove(&key) else { return Task::none() };
            pg.saving = true;
            pg.text.clear();
            sh.reread_history = true;
            let a: Vec<&str> = args.iter().map(String::as_str).collect();
            return secret_run(helper(&a, 150), secret, move |r| Msg::Applied(page, r));
        }
        Msg::ActCapture(page, id) => st.act_capture = Some((page, id)),
        Msg::Store(kind) => {
            st.store_kind = kind;
            st.store = Value::Null;
            st.store_say.clear();
            if st.store_sort.is_empty() {
                st.store_sort = "down";
            }
            sh.go("kde-store");
            return update(sh, st, Msg::StoreFind);
        }
        Msg::StoreFind => {
            st.store = Value::Null;
            let (k, q, o) = (st.store_kind.to_string(), st.store_q.clone(), st.store_sort.to_string());
            return read(helper(&["store", &k, &q, &o], 90), Msg::StoreGot);
        }
        Msg::StoreGot(r) => {
            if r.ok {
                st.store = r.v;
                let thumbs: Vec<(String, crate::pics::Cut)> = st.store.get("items").and_then(Value::as_array).map(|a| a.iter().map(|x| (s(x, "thumb").to_string(), crate::pics::WALL)).collect()).unwrap_or_default();
                if let Some(f) = crate::pics::load(thumbs) {
                    return Task::perform(f, |_| Msg::Pics);
                }
            } else {
                st.store = serde_json::json!({"items": []});
                st.store_say.bad(r.err);
            }
        }
        Msg::Pics => {}
        Msg::StoreSearch(q) => st.store_q = q,
        Msg::StoreSort(o) => {
            st.store_sort = o;
            return update(sh, st, Msg::StoreFind);
        }
        Msg::StoreRun(args) => {
            st.store_busy = args.get(2).cloned().unwrap_or_default();
            st.store_say.clear();
            sh.reread_history = true;
            let a: Vec<&str> = args.iter().map(String::as_str).collect();
            return read(helper(&a, 600), Msg::StoreDone);
        }
        Msg::StoreDone(r) => {
            st.store_busy.clear();
            if r.ok {
                st.store_say.ok("Done: it's in the list on its page now");
            } else {
                st.store_say.bad(r.err);
            }
            return update(sh, st, Msg::StoreFind);
        }
        Msg::FontShow(family, file) => {
            if family.is_empty() {
                st.font_shown = None;
                return Task::none();
            }
            st.font_shown = Some(family.clone());
            if st.fonts_loaded.iter().any(|f| *f == family) {
                return Task::none();
            }
            let (tx, rx) = futures::channel::oneshot::channel();
            let _ = std::thread::Builder::new().name("hearth-settings-font".into()).spawn(move || {
                // A font file of at most 40 MB (bigger ones are only listed).
                let ok = std::fs::metadata(&file).map(|m| m.len() < 40 << 20).unwrap_or(false);
                let _ = tx.send(if ok { std::fs::read(&file).ok() } else { None });
            });
            return Task::perform(async move { rx.await.ok().flatten() }, move |b| Msg::FontBytes(family.clone(), b));
        }
        Msg::FontBytes(family, bytes) => {
            let Some(b) = bytes else { return Task::none() };
            return iced::font::load(b).map(move |r| Msg::FontLoaded(family.clone(), r.is_ok()));
        }
        Msg::FontLoaded(family, ok) => {
            if ok && !st.fonts_loaded.iter().any(|f| *f == family) {
                // Kept for as long as Settings is open (iced draws a font by its family's name).
                st.fonts_loaded.push(Box::leak(family.into_boxed_str()));
            }
        }
        Msg::Edit(e) => {
            st.editing = e;
            if let Some(pg) = st.pages.get_mut("kde-device-actions") {
                pg.text.retain(|k, _| *k < 2000);
            }
        }
        Msg::Pads(p) => st.pads = p,
        Msg::HdrGot(r) => {
            if r.ok {
                st.hdr.info = r.v;
                if let Some(o) = hdr_out(&st.hdr).cloned() {
                    st.hdr.name = s(&o, "name").to_string();
                    let w = std::mem::take(&mut st.want);
                    if w.starts_with("calibrate") {
                        let t = update(sh, st, Msg::HdrStart);
                        if w == "calibrate3" {
                            st.hdr.step = 2;
                        }
                        return t;
                    }
                }
            } else {
                st.hdr.say.bad(r.err);
            }
        }
        Msg::HdrStart => {
            let Some(o) = hdr_out(&st.hdr).cloned() else { return Task::none() };
            let c = o.get("hdrCal").cloned().unwrap_or(Value::Null);
            let f = |k: &str| c.get(k).and_then(Value::as_f64).map(|x| x as f32);
            st.hdr.start0 = serde_json::json!({"peak": c.get("peakSet"), "avg": c.get("avgSet"), "min": c.get("minSet")});
            let peak = f("peakSet").or(f("peak")).unwrap_or(600.0);
            st.hdr.vals = [peak, f("avgSet").or(f("avg")).unwrap_or(peak.min(400.0)), f("minSet").or(f("min")).unwrap_or(0.005)];
            st.hdr.step = 0;
            st.hdr.on = true;
            st.hdr.say.clear();
            return win_mode(true);
        }
        Msg::HdrSet(v) => {
            let (_, _, lo, hi, _) = HDR_STEPS[st.hdr.step];
            st.hdr.vals[st.hdr.step] = v.clamp(lo, hi);
        }
        Msg::HdrApply => return hdr_apply(&st.hdr, hdr_now(&st.hdr)),
        Msg::HdrStep(d) => {
            let n = st.hdr.step as i32 + d;
            if n < 0 {
                return update(sh, st, Msg::HdrCancel);
            }
            if n > 2 {
                return update(sh, st, Msg::HdrFinish);
            }
            st.hdr.step = n as usize;
        }
        Msg::HdrReset => {
            let c = hdr_out(&st.hdr).and_then(|o| o.get("hdrCal")).cloned().unwrap_or(Value::Null);
            let d = [600.0, 400.0, 0.0];
            let k = ["peak", "avg", "min"][st.hdr.step];
            st.hdr.vals[st.hdr.step] = c.get(k).and_then(Value::as_f64).map(|x| x as f32).unwrap_or(d[st.hdr.step]);
            return hdr_apply(&st.hdr, hdr_now(&st.hdr));
        }
        Msg::HdrFinish => {
            st.hdr.on = false;
            st.hdr.keep = Some(15);
            sh.reread_history = true;
            return Task::batch([win_mode(false), hdr_apply(&st.hdr, hdr_now(&st.hdr))]);
        }
        Msg::HdrCancel => {
            st.hdr.on = false;
            st.hdr.cancel_due = false;
            return Task::batch([win_mode(false), hdr_apply(&st.hdr, st.hdr.start0.clone())]);
        }
        Msg::HdrApplied(r) => {
            if !r.ok {
                st.hdr.say.bad(r.err);
            } else if let Some(p) = r.v.get("problem").and_then(Value::as_str).filter(|p| !p.is_empty()) {
                st.hdr.say.bad(p.to_string());
            }
            if r.ok && r.v.get("outputs").is_some() {
                st.hdr.info = r.v;
            }
        }
        Msg::HdrKeep(yes) => {
            st.hdr.keep = None;
            if yes {
                let names = serde_json::json!([st.hdr.name]).to_string();
                st.hdr.say.ok("Kept: KWin uses these from now on");
                return read(crate::cats::apps::kit::py("hearth-screens", &["keep", &names], 30), Msg::HdrApplied);
            }
            st.hdr.say.ok("Put back as it was");
            return hdr_apply(&st.hdr, st.hdr.start0.clone());
        }
        Msg::HdrTick => match st.hdr.keep {
            Some(n) if n > 1 => st.hdr.keep = Some(n - 1),
            Some(_) => return update(sh, st, Msg::HdrKeep(false)),
            None => {}
        },
        Msg::FileFor(page, cmd, filter) => {
            let what = if cmd == "font-install" { "Choose a font" } else { "Choose a colour profile" };
            return read(crate::cats::pers::kit::file_picker(what, false, filter), move |r| Msg::FileForGot(page, cmd, r));
        }
        Msg::FileForGot(page, cmd, r) => {
            let path = r.out.trim().lines().last().unwrap_or("").trim().to_string();
            if r.ok && path.starts_with('/') {
                return update(sh, st, Msg::Run(page, vec![cmd.to_string(), path], None));
            }
        }
        Msg::Applied(page, r) => {
            let pg = st.pages.entry(page).or_default();
            pg.saving = false;
            if !r.ok {
                pg.say.bad(r.err);
            }
            sh.reread_history = true;
            // What the tool changed, read again.
            return read(helper(&["get", page], 30), move |r| Msg::Got(page, r));
        }
    }
    Task::none()
}

/// Only while a shortcut waits for its keys (nothing at rest).
pub fn subscription(st: &State) -> Subscription<Msg> {
    let mut v = Vec::new();
    if st.capture.is_some() || st.act_capture.is_some() {
        v.push(iced::keyboard::listen().map(Msg::Key));
    }
    // The live controller test, only while its page is on the screen.
    if *SHOWN.lock().unwrap_or_else(|e| e.into_inner()) == "kde-gamecontroller" {
        v.push(Subscription::run(pad_test::watch).map(Msg::Pads));
    }
    if st.hdr.cancel_due {
        v.push(Subscription::run(|| futures::stream::once(async {})).map(|_| Msg::HdrCancel));
    }
    // "Keep these settings?" after Calibrate HDR counts down (15 s at most).
    if st.hdr.keep.is_some() {
        v.push(iced::time::every(std::time::Duration::from_secs(1)).map(|_| Msg::HdrTick));
    }
    Subscription::batch(v)
}

pub fn back(st: &mut State) -> bool {
    if st.hdr.on {
        st.hdr.cancel_due = true;
        return true;
    }
    st.pick.take().is_some() || st.capture.take().is_some() || st.act_capture.take().is_some() || st.open_comp.take().is_some() || st.editing.take().is_some()
}

/// A key pressed as KDE writes it ("Meta+Shift+S"); None for a modifier alone (still waiting) or a key that can't be
/// one.
pub fn shortcut_text(key: &iced::keyboard::Key, m: iced::keyboard::Modifiers) -> Option<String> {
    use iced::keyboard::key::Named as N;
    use iced::keyboard::Key;
    let k = match key.as_ref() {
        Key::Character(c) => c.to_uppercase(),
        Key::Named(n) => match n {
            N::Space => "Space".into(),
            N::Tab => "Tab".into(),
            N::Enter => "Return".into(),
            N::Backspace => "Backspace".into(),
            N::Delete => "Del".into(),
            N::Insert => "Ins".into(),
            N::Home => "Home".into(),
            N::End => "End".into(),
            N::PageUp => "PgUp".into(),
            N::PageDown => "PgDown".into(),
            N::ArrowLeft => "Left".into(),
            N::ArrowRight => "Right".into(),
            N::ArrowUp => "Up".into(),
            N::ArrowDown => "Down".into(),
            N::PrintScreen => "Print".into(),
            N::Pause => "Pause".into(),
            N::F1 => "F1".into(),
            N::F2 => "F2".into(),
            N::F3 => "F3".into(),
            N::F4 => "F4".into(),
            N::F5 => "F5".into(),
            N::F6 => "F6".into(),
            N::F7 => "F7".into(),
            N::F8 => "F8".into(),
            N::F9 => "F9".into(),
            N::F10 => "F10".into(),
            N::F11 => "F11".into(),
            N::F12 => "F12".into(),
            _ => return None,
        },
        _ => return None,
    };
    let mut s = String::new();
    if m.logo() {
        s.push_str("Meta+");
    }
    if m.control() {
        s.push_str("Ctrl+");
    }
    if m.alt() {
        s.push_str("Alt+");
    }
    if m.shift() {
        s.push_str("Shift+");
    }
    s.push_str(&k);
    Some(s)
}

// ---------------------------------------------------------------- drawing

type E<'a> = Element<'a, Msg>;

fn act(m: Msg) -> nav::Act {
    Box::new(m)
}

/// The sub-line, with "used on KDE Plasma" when this row is only Plasma's and this isn't Plasma.
fn sub_of(pg: &Pg, page_plasma: bool, r: &Value) -> String {
    let sub = s(r, "sub").to_string();
    let plasma_row = r.get("plasma").and_then(Value::as_bool).unwrap_or(false);
    if plasma_row && !page_plasma && !plasma_now(pg) {
        if sub.is_empty() {
            "Used on KDE Plasma".into()
        } else {
            format!("{sub}. Used on KDE Plasma")
        }
    } else {
        sub
    }
}

fn a_kde_capture(c: &Option<(&'static str, String)>, id: &str) -> bool {
    c.as_ref().map(|(_, x)| x == id).unwrap_or(false)
}

/// A long list (themes, styles): ‹ name › to step through it, or a click (A) to see them all.
fn pick_row<'a>(p: &Pal, page: &'static str, i: usize, g: G, title: String, sub: String, items: &[(String, String)], cur: &str, on: bool) -> E<'a> {
    let idx = items.iter().position(|(k, _)| k == cur);
    let prev = idx.and_then(|x| x.checked_sub(1)).and_then(|x| items.get(x)).map(|(k, _)| Msg::Picked(page, i, k.clone()));
    let next = match idx {
        Some(x) => items.get(x + 1),
        None => items.first(),
    }
    .map(|(k, _)| Msg::Picked(page, i, k.clone()));
    let (prev, next) = if on { (prev, next) } else { (None, None) };
    let open = if on && !items.is_empty() { Some(Msg::Pick(Some((page, i)))) } else { None };
    let label = label_of(items, cur);
    let slot = nav::control(Kind::Choice, open.iter().cloned().map(act).collect(), prev.iter().cloned().map(act).collect(), next.iter().cloned().map(act).collect(), String::new(), label.clone());
    let pp = *p;
    let arrow = |sym: &'static str, m: Option<Msg>| -> E<'a> {
        let lit = m.is_some();
        let a = container(t(sym, sz(20.0), if lit { pp.accent } else { Pal::a(pp.muted, 0.5) })).padding(Padding { top: 0.0, right: 6.0, bottom: 2.0, left: 6.0 });
        match m {
            Some(m) => mouse_area(a).on_press(m).interaction(mouse::Interaction::Pointer).into(),
            None => a.into(),
        }
    };
    let name = container(tm(label, sz(14.0), if on { pp.text } else { pp.muted })).padding(Padding { top: sz(6.0), right: 12.0, bottom: sz(6.0), left: 12.0 }).style(hearth_ui::ui::boxed(Pal::mix(pp.panel, pp.bg, 0.4), pp.r(8.0), Some((pp.line, 1.0))));
    let name: E<'a> = match open {
        Some(m) => mouse_area(name).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => name.into(),
    };
    let ctl = container(row![arrow("‹", prev), name, arrow("›", next)].spacing(4.0).align_y(iced::Center)).id(nav::slot_id(slot)).style(move |_: &Theme| {
        let lit = nav::slot_lit(slot);
        container::Style { border: Border { color: if lit { pp.accent } else { Color::TRANSPARENT }, width: if lit { 2.0 } else { 0.0 }, radius: pp.r(8.0).into() }, ..container::Style::default() }
    });
    srow(p, g, title, sub, ctl.into())
}

/// A button on the right of a row (Choose a picture…, Save).
fn button_row<'a>(p: &Pal, g: G, title: String, sub: String, label: &'a str, msg: Option<Msg>) -> E<'a> {
    nav::control(Kind::Press, msg.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
    srow(p, g, title, sub, btn(p, label, hearth_ui::ui::Kind::Normal, false, msg))
}

fn swatch_row<'a>(p: &Pal, page: &'static str, i: usize, g: G, title: String, sub: String, opts: &[Value], cur: Option<usize>, on: bool) -> E<'a> {
    // The same choice as a row of colour dots (A / left / right like any choice).
    let mut dots = iced::widget::Row::new().spacing(8.0).align_y(iced::Center);
    for (o, x) in opts.iter().enumerate() {
        let v = x.get(0).map(text_of).unwrap_or_default();
        let c: Vec<f32> = v.split(',').filter_map(|n| n.trim().parse::<f32>().ok()).collect();
        let fill = if c.len() == 3 { Color::from_rgb(c[0] / 255.0, c[1] / 255.0, c[2] / 255.0) } else { Pal::mix(p.panel, p.text, 0.12) };
        let lit = cur == Some(o);
        let dot = container(if c.len() != 3 { E::from(t("A", sz(11.0), p.muted)) } else { Space::new().into() })
            .center_x(Length::Fixed(sz(26.0)))
            .center_y(Length::Fixed(sz(26.0)))
            .style(hearth_ui::ui::boxed(fill, sz(13.0), Some((if lit { p.text } else { p.line }, if lit { 2.5 } else { 1.0 }))));
        dots = dots.push(if on { E::from(mouse_area(dot).on_press(Msg::Opt(page, i, o)).interaction(mouse::Interaction::Pointer)) } else { dot.into() });
    }
    let name = cur.and_then(|o| opts.get(o)).and_then(|x| x.get(1)).map(text_of).unwrap_or_else(|| "Your own".into());
    let idx = cur.unwrap_or(0);
    let (idx_prev, idx_next) = (idx.checked_sub(1), if idx + 1 < opts.len() { Some(idx + 1) } else { None });
    let prev = if on { idx_prev.map(|o| Msg::Opt(page, i, o)) } else { None };
    let next = if on { idx_next.map(|o| Msg::Opt(page, i, o)) } else { None };
    let round = if on { Some(Msg::Opt(page, i, (cur.unwrap_or(0) + 1) % opts.len().max(1))) } else { None };
    nav::control(Kind::Choice, round.into_iter().map(act).collect(), prev.into_iter().map(act).collect(), next.into_iter().map(act).collect(), String::new(), name.clone());
    srow(p, g, title, if sub.is_empty() { name } else { format!("{sub} · {name}") }, dots.into())
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<E<'a>> {
    if !PAGES.contains(&page) {
        return None;
    }
    let p = a.sh.p;
    let empty = Pg::default();
    let pg = a.kde.pages.get(page).unwrap_or(&empty);
    let ps = page_spec(page)?;
    let page_plasma = ps.get("plasma").and_then(Value::as_bool).unwrap_or(false);
    let mut v: Vec<E<'a>> = Vec::new();
    let intro = s(ps, "intro");
    if !intro.is_empty() {
        v.push(small(&p, intro.to_string()));
    }
    if page_plasma && pg.read && !plasma_now(pg) {
        v.push(words(&p, "These are KDE Plasma's settings. You're not on KDE Plasma now: they're saved, and used the next time you are.", false));
    }
    if !pg.read {
        v.push(words(&p, "Reading KDE's settings…", false));
    }
    if let Some(bad) = pg.data.get("bad").and_then(Value::as_object) {
        for (f, why) in bad {
            v.push(words(&p, format!("Your {f} couldn't be read ({}). Hearth won't write over it, so nothing in it is lost; its settings here can't be changed until it's fixed.", text_of(why)), true));
        }
    }
    let ready = pg.read && pg.data.get("files").is_some();
    for (i, r) in rows_of(page).iter().enumerate() {
        let g = icon(s(r, "icon"));
        let title = s(r, "title").to_string();
        let sub = sub_of(pg, page_plasma, r);
        let bad_file = pg.data.get("bad").and_then(|b| b.get(s(r, "file"))).is_some();
        let lock = locked(pg, r);
        let on = ready && !bad_file && !lock && needs_ok(page, pg, r);
        let sub = if lock { format!("{}{}Set by whoever set up this PC, so it can't be changed here", sub, if sub.is_empty() { "" } else { ". " }) } else { sub };
        match s(r, "type") {
            "head" => v.push(head(&p, title)),
            "note" => v.push(small(&p, title)),
            "switch" => {
                let is_on = switch_on(pg, r);
                v.push(sw(&p, g, title, sub, is_on, if on { Some(Msg::Switch(page, i, !is_on)) } else { None }));
            }
            "choice" => {
                let opts = r.get("opts").and_then(Value::as_array).map(|x| x.as_slice()).unwrap_or(&[]);
                let cur = chosen(pg, r);
                if r.get("swatches").and_then(Value::as_bool).unwrap_or(false) {
                    v.push(swatch_row(&p, page, i, g, title, sub, opts, cur, on));
                    continue;
                }
                let items: Vec<(String, bool, Option<Msg>)> = opts.iter().enumerate().map(|(o, x)| (x.get(1).map(text_of).unwrap_or_default(), cur == Some(o), if on { Some(Msg::Opt(page, i, o)) } else { None })).collect();
                let wide = r.get("wide").and_then(Value::as_bool).unwrap_or(false) || items.len() > 3;
                v.push(if wide { choice_wide(&p, g, title, sub, items) } else { choice(&p, g, title, sub, items) });
            }
            "number" => {
                let def = r.get("def").cloned().unwrap_or(Value::Null);
                let cur: f32 = pg.temp.get(&i).copied().unwrap_or_else(|| row_val(pg, r).parse::<f32>().unwrap_or_else(|_| def.as_f64().unwrap_or(0.0) as f32));
                let (lo, hi) = (r.get("min").and_then(Value::as_f64).unwrap_or(0.0) as f32, r.get("max").and_then(Value::as_f64).unwrap_or(100.0) as f32);
                let step = r.get("step").and_then(Value::as_f64).unwrap_or(1.0) as f32;
                let unit = s(r, "unit");
                let shown = if r.get("percent").and_then(Value::as_bool).unwrap_or(false) { format!("{:.0} %", 100.0 * cur / hi) } else if r.get("int").and_then(Value::as_bool).unwrap_or(false) { format!("{} {unit}", cur.round() as i64) } else { format!("{cur:.1} {unit}") };
                let line = if sub.is_empty() { shown.trim().to_string() } else { format!("{} · {}", shown.trim(), sub) };
                let int = r.get("int").and_then(Value::as_bool).unwrap_or(false) && lo >= 0.0;
                let ctl: E<'a> = if !on {
                    t(shown.trim().to_string(), sz(13.0), p.muted).into()
                } else if int {
                    // Whole numbers: the value shows as one (500, not 500.0).
                    let (l, h) = (lo as u32, hi as u32);
                    let _ = step;
                    slider_u32((cur.round().max(0.0) as u32).clamp(l, h), l..=h, move |x| Msg::Num(page, i, x as f32), Msg::NumDone(page, i))
                } else {
                    slider_f32(cur.clamp(lo, hi), lo..=hi, step, move |x| Msg::Num(page, i, x), Msg::NumDone(page, i))
                };
                v.push(srow(&p, g, title, line, ctl));
            }
            "pick" => {
                let items = list(pg, r);
                let cur = row_val(pg, r);
                let on = on || (ready && r.get("tool").is_some() && r.get("file").is_none());
                let can = on && !items.is_empty();
                v.push(pick_row(&p, page, i, g, title, if items.is_empty() && pg.read { format!("{}{}Nothing to pick from is installed here", sub, if sub.is_empty() { "" } else { ". " }) } else { sub }, &items, &cur, can));
                // A page that's mostly this one choice lists every option too (as KDE's grid of themes does).
                if r.get("list_all").and_then(Value::as_bool).unwrap_or(false) && !items.is_empty() {
                    v.push(head(&p, "Installed"));
                    for (k, label) in items.iter() {
                        let here = *k == cur;
                        let m = if can && !here { Some(Msg::Picked(page, i, k.clone())) } else { None };
                        nav::control(Kind::Press, m.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                        let ctl: E<'a> = if here { row![t("In use", sz(13.0), p.accent), hearth_ui::ui::glyph(G::Check, p.accent, 16.0, 2.4)].spacing(8.0).align_y(iced::Center).into() } else { btn(&p, "Use", hearth_ui::ui::Kind::Normal, false, m) };
                        v.push(srow(&p, g, label.clone(), String::new(), ctl));
                    }
                }
            }
            "text" => {
                let cur = pg.text.get(&i).cloned().unwrap_or_else(|| row_val(pg, r));
                let dirty = pg.text.contains_key(&i);
                let f = field(&p, "", &cur, move |x| Msg::Text(page, i, x)).on_submit(Msg::TextSave(page, i)).width(Length::Fixed(sz(280.0)));
                let save_msg = if on && dirty { Some(Msg::TextSave(page, i)) } else { None };
                nav::control(Kind::Press, save_msg.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                let ctl: E<'a> = row![f, btn(&p, "Save", hearth_ui::ui::Kind::Normal, false, save_msg)].spacing(8.0).align_y(iced::Center).into();
                v.push(srow(&p, g, title, sub, ctl));
            }
            "file" => {
                let cur = row_val(pg, r);
                let name = cur.trim_start_matches("file://").rsplit('/').next().unwrap_or("").to_string();
                let line = if name.is_empty() { if sub.is_empty() { "KDE's own picture".to_string() } else { sub } } else { name };
                v.push(button_row(&p, g, title, line, "Choose a picture…", if on { Some(Msg::File(page, i)) } else { None }));
            }
            "firmware" => {
                let fw = pg.data.pointer("/current/firmware");
                let can = fw.and_then(|f| f.get("can")).and_then(Value::as_bool).unwrap_or(false);
                let is_on = fw.and_then(|f| f.get("on")).and_then(Value::as_bool).unwrap_or(false);
                let sub = if can || !pg.read { sub } else { "This PC can't be told to (it needs UEFI and a system that allows it)".into() };
                v.push(sw(&p, g, title, sub, is_on, if ready && can { Some(Msg::Switch(page, i, !is_on)) } else { None }));
            }
            "info" => {
                let rows = pg.data.get("info").and_then(|x| x.get(s(r, "from"))).and_then(Value::as_array).cloned().unwrap_or_default();
                let mut c = Column::new().spacing(8.0);
                if rows.is_empty() {
                    c = c.push(t(if pg.read { "Nothing to show" } else { "Reading…" }, sz(14.0), p.muted));
                }
                for x in rows.iter().filter_map(Value::as_array) {
                    c = c.push(fact(&p, x.first().map(text_of).unwrap_or_default(), x.get(1).map(text_of).unwrap_or_default()));
                }
                if !title.is_empty() {
                    v.push(head(&p, title));
                }
                v.push(card(&p, c));
            }
            "hub" => {
                // Every other KDE page, under its category's name.
                for (cid, clabel, _) in crate::registry::CATEGORIES.iter() {
                    let here: Vec<&Value> = spec()["pages"].as_array().map(|a| a.iter().filter(|x| s(x, "cat") == *cid && s(x, "id") != page && !x.get("hidden").and_then(Value::as_bool).unwrap_or(false)).collect()).unwrap_or_default();
                    if here.is_empty() {
                        continue;
                    }
                    v.push(head(&p, *clabel));
                    for x in here {
                        v.push(link_row(&p, crate::frame::cat_glyph(cid), s(x, "label").to_string(), s(x, "desc").to_string(), Msg::Go(sid(s(x, "id")))));
                    }
                }
            }
            "plugins" => {
                // Every installed one (kwinrc [Plugins] <id>Enabled), its name and what it does.
                let items = pg.data.get("lists").and_then(|l| l.get(s(r, "from"))).and_then(Value::as_array).cloned().unwrap_or_default();
                if items.is_empty() && pg.read {
                    v.push(small(&p, "None are installed."));
                }
                for x in items.iter().filter_map(Value::as_array) {
                    let id = x.first().map(text_of).unwrap_or_default();
                    let name = x.get(1).map(text_of).unwrap_or_else(|| id.clone());
                    let what = x.get(2).map(text_of).unwrap_or_default();
                    let def = x.get(3).and_then(Value::as_bool).unwrap_or(false);
                    let cur = val(pg, s(r, "file"), s(r, "group"), &format!("{id}Enabled"), &Value::String(def.to_string()));
                    let is_on = truthy(&cur);
                    v.push(sw(&p, g, name, what, is_on, if on { Some(Msg::Plugin(page, i, id.clone(), !is_on)) } else { None }));
                }
            }
            "kded" => {
                let k = pg.data.pointer("/current/kded").cloned().unwrap_or(Value::Null);
                let mods = k.get("modules").and_then(Value::as_array).cloned().unwrap_or_default();
                if pg.read && !k.get("kded").and_then(Value::as_bool).unwrap_or(false) {
                    v.push(words(&p, "KDE's background services aren't running here (they run on KDE Plasma): what starts at sign-in can still be set.", false));
                }
                if mods.is_empty() && pg.read {
                    v.push(small(&p, "None are installed."));
                }
                let (mut running, mut stopped) = (Vec::new(), Vec::new());
                for m in mods.iter() {
                    let name = s(m, "name").to_string();
                    let auto = m.get("autoload").and_then(Value::as_bool).unwrap_or(true);
                    let run_now = m.get("running").and_then(Value::as_bool).unwrap_or(false);
                    let what = s(m, "what");
                    let line = format!("{}{}{}", if run_now { "Running now" } else { "Not running" }, if what.is_empty() { "" } else { " · " }, what);
                    let e = sw(&p, g, s(m, "label").to_string(), line, auto, if ready { Some(Msg::Kded(page, name, !auto)) } else { None });
                    if run_now { running.push(e) } else { stopped.push(e) }
                }
                if !running.is_empty() {
                    v.push(head(&p, "Running"));
                    v.extend(running);
                }
                if !stopped.is_empty() {
                    v.push(head(&p, "Not running"));
                    v.extend(stopped);
                }
            }
            "activities" => {
                let a_capture = a.kde.act_capture.clone();
                let a = pg.data.pointer("/current/activities").cloned().unwrap_or(Value::Null);
                let list = a.get("list").and_then(Value::as_array).cloned().unwrap_or_default();
                let cur = s(&a, "current").to_string();
                let live = a.get("running").and_then(Value::as_bool).unwrap_or(false);
                if pg.read && !live {
                    v.push(words(&p, "Activities run on KDE Plasma: open this page there to change them.", false));
                }
                let n = list.len();
                for (k, x) in list.iter().enumerate() {
                    let id = s(x, "id").to_string();
                    let name = s(x, "name").to_string();
                    let tkey = 1000 + i * 100 + k;
                    let typed = pg.text.get(&tkey).cloned();
                    let shown = typed.clone().unwrap_or_else(|| name.clone());
                    let fld = field(&p, "Name", &shown, move |t| Msg::Text(page, tkey, t)).width(Length::Fixed(sz(220.0)));
                    let fld = match &typed {
                        Some(tn) if live && !tn.trim().is_empty() => fld.on_submit(Msg::Run(page, vec!["activity".into(), "rename".into(), id.clone(), tn.clone()], None)),
                        _ => fld,
                    };
                    let is_cur = id == cur;
                    let sw_msg = if live && !is_cur { Some(Msg::Run(page, vec!["activity".into(), "switch".into(), id.clone()], None)) } else { None };
                    let rm_msg = if live && n > 1 { Some(Msg::Run(page, vec!["activity".into(), "remove".into(), id.clone()], None)) } else { None };
                    nav::control(Kind::Press, sw_msg.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                    nav::control(Kind::Press, rm_msg.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                    let ctl: E<'a> = row![fld, btn(&p, if is_cur { "In use" } else { "Switch to it" }, hearth_ui::ui::Kind::Normal, false, sw_msg), btn(&p, "Remove", hearth_ui::ui::Kind::Normal, false, rm_msg)].spacing(8.0).align_y(iced::Center).into();
                    v.push(srow(&p, g, name.clone(), if is_cur { "The activity you're in".to_string() } else { String::new() }, ctl));
                    // Its icon, its own shortcut, and whether it remembers what's opened in it (KDE's activity page).
                    let icon_now = s(x, "icon").to_string();
                    let icons = [("activities", "Activities"), ("user-home", "Home"), ("applications-games", "Games"), ("applications-office", "Work"), ("applications-internet", "Web"), ("applications-multimedia", "Media"), ("applications-graphics", "Art"), ("applications-development", "Code"), ("folder-music", "Music"), ("weather-clear-night", "Night")];
                    let items: Vec<(String, bool, Option<Msg>)> = icons.iter().map(|(k, l)| (l.to_string(), icon_now == *k, if live { Some(Msg::Run(page, vec!["activity".into(), "icon".into(), id.clone(), k.to_string()], None)) } else { None })).collect();
                    v.push(container(choice_wide(&p, G::Palette, format!("{name}: its icon"), "", items)).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 40.0 }).into());
                    let private = x.get("private").and_then(Value::as_bool).unwrap_or(false);
                    v.push(container(sw(&p, G::Clock, format!("{name}: remember what's opened in it"), "Off: files and apps used in it don't show in Recent files or the search", !private, if live { Some(Msg::Run(page, vec!["activity".into(), "private".into(), id.clone(), if private { "off".into() } else { "on".into() }], None)) } else { None })).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 40.0 }).into());
                    let keys: Vec<String> = x.get("shortcut").and_then(Value::as_array).map(|k| k.iter().map(text_of).collect()).unwrap_or_default();
                    let waiting = a_kde_capture(&a_capture, &id);
                    let ch = if live { Some(Msg::ActCapture(page, id.clone())) } else { None };
                    let cl = if live && !keys.is_empty() { Some(Msg::Run(page, vec!["activity".into(), "shortcut".into(), id.clone(), String::new()], None)) } else { None };
                    nav::control(Kind::Press, ch.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                    nav::control(Kind::Press, cl.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                    let line = if waiting { "Press the new shortcut… (Esc: cancel)".to_string() } else if keys.is_empty() { "No shortcut".to_string() } else { keys.join(" or ") };
                    v.push(container(srow(&p, G::Keyboard, format!("{name}: shortcut to switch to it"), line, row![btn(&p, "Change", hearth_ui::ui::Kind::Normal, false, ch), btn(&p, "None", hearth_ui::ui::Kind::Normal, false, cl)].spacing(6.0).into())).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 40.0 }).into());
                }
                let nkey = 1000 + i * 100 + 99;
                let new_name = pg.text.get(&nkey).cloned().unwrap_or_default();
                let add = if live && !new_name.trim().is_empty() { Some(Msg::Run(page, vec!["activity".into(), "add".into(), new_name.clone()], None)) } else { None };
                let mut fld = field(&p, "A name, like Work or Games", &new_name, move |t| Msg::Text(page, nkey, t)).width(Length::Fixed(sz(260.0)));
                if let Some(m) = add.clone() {
                    fld = fld.on_submit(m);
                }
                nav::control(Kind::Press, add.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                v.push(srow(&p, G::Plus, "New activity", "", row![fld, btn(&p, "Add", hearth_ui::ui::Kind::Primary, false, add)].spacing(8.0).align_y(iced::Center).into()));
            }
            "userdirs" => {
                let dirs = pg.data.pointer("/current/userdirs").and_then(Value::as_array).cloned().unwrap_or_default();
                for d in dirs.iter() {
                    let key = s(d, "key").to_string();
                    let m = if ready { Some(Msg::Folder(page, key)) } else { None };
                    v.push(button_row(&p, g, s(d, "label").to_string(), s(d, "path").to_string(), "Change…", m));
                }
            }
            "deviceactions" => {
                let acts = pg.data.pointer("/current/deviceactions").and_then(Value::as_array).cloned().unwrap_or_default();
                if acts.is_empty() && pg.read {
                    v.push(small(&p, "No device actions are installed here."));
                }
                let mut last = String::new();
                let mut sorted: Vec<&Value> = acts.iter().collect();
                sorted.sort_by(|a, b| s(a, "kinds").cmp(s(b, "kinds")).then(s(a, "name").cmp(s(b, "name"))));
                for x in sorted {
                    let kinds = s(x, "kinds").to_string();
                    if kinds != last {
                        v.push(head(&p, format!("When you plug in: {kinds}")));
                        last = kinds;
                    }
                    let mine = x.get("mine").and_then(Value::as_bool).unwrap_or(false);
                    let changed = x.get("changed").and_then(Value::as_bool).unwrap_or(false);
                    let (file, action) = (s(x, "file").to_string(), s(x, "action").to_string());
                    let line = s(x, "exec").to_string();
                    let editing = a.kde.editing.as_ref() == Some(&(file.clone(), action.clone()));
                    if editing {
                        // KDE's way: your own copy of the system's action (same name, in your folder) is used instead;
                        // the system's file is never changed.
                        let (kn, kc) = (2000 + 1, 2000 + 2);
                        let name = pg.text.get(&kn).cloned().unwrap_or_else(|| s(x, "name").to_string());
                        let cmd = pg.text.get(&kc).cloned().unwrap_or_else(|| line.clone());
                        v.push(srow(&p, G::Cog, "Its name", "What the pop-up shows", field(&p, "", &name, move |t| Msg::Text(page, kn, t)).width(Length::Fixed(sz(320.0))).into()));
                        v.push(srow(&p, G::Cog, "The command", "%f is the device's folder, %i its id", field(&p, "", &cmd, move |t| Msg::Text(page, kc, t)).width(Length::Fixed(sz(320.0))).into()));
                        let input = serde_json::json!({"file": file, "action": action, "name": name, "exec": cmd}).to_string();
                        let save = if ready && !name.trim().is_empty() && !cmd.trim().is_empty() { Some(Msg::Run(page, vec!["device-action-edit".into()], Some(input))) } else { None };
                        for m in [&save, &Some(Msg::Edit(None))] {
                            nav::control(Kind::Press, m.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                        }
                        v.push(srow(&p, G::Check, "Save your version", "The system's own stays as it is: Reset brings it back", row![btn(&p, "Save", hearth_ui::ui::Kind::Primary, false, save), btn(&p, "Cancel", hearth_ui::ui::Kind::Normal, false, Some(Msg::Edit(None)))].spacing(8.0).into()));
                    } else if mine {
                        let m = if ready { Some(Msg::Run(page, vec!["device-action-remove".into(), file.clone()], None)) } else { None };
                        v.push(button_row(&p, g, s(x, "name").to_string(), format!("Yours · {line}"), "Remove", m));
                    } else {
                        let ed = if ready { Some(Msg::Edit(Some((file.clone(), action.clone())))) } else { None };
                        let rs = if ready && changed { Some(Msg::Run(page, vec!["device-action-reset".into(), file.clone()], None)) } else { None };
                        nav::control(Kind::Press, ed.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                        if changed {
                            nav::control(Kind::Press, rs.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                        }
                        let ctl: E<'a> = if changed { row![btn(&p, "Edit", hearth_ui::ui::Kind::Normal, false, ed), btn(&p, "Reset to the system version", hearth_ui::ui::Kind::Normal, false, rs)].spacing(8.0).into() } else { btn(&p, "Edit", hearth_ui::ui::Kind::Normal, false, ed) };
                        v.push(srow(&p, g, s(x, "name").to_string(), if changed { format!("Changed by you · {line}") } else { line }, ctl));
                    }
                }
                // Your own action.
                v.push(head(&p, "Add your own"));
                let (kn, kc, kk) = (1000 + i * 100 + 1, 1000 + i * 100 + 2, 1000 + i * 100 + 3);
                let name = pg.text.get(&kn).cloned().unwrap_or_default();
                let cmd = pg.text.get(&kc).cloned().unwrap_or_default();
                let kind = pg.text.get(&kk).cloned().unwrap_or_else(|| "StorageVolume".into());
                v.push(srow(&p, G::Plus, "Its name", "What the pop-up shows", field(&p, "Back up my photos", &name, move |t| Msg::Text(page, kn, t)).width(Length::Fixed(sz(300.0))).into()));
                v.push(srow(&p, G::Cog, "The command", "%f is the device's folder", field(&p, "rsync -a %f/DCIM ~/Pictures/", &cmd, move |t| Msg::Text(page, kc, t)).width(Length::Fixed(sz(300.0))).into()));
                let kinds = [("StorageVolume", "Drives"), ("Camera", "Cameras"), ("PortableMediaPlayer", "Phones and music players"), ("OpticalDisc", "Discs")];
                v.push(choice_wide(&p, G::Drive, "For", "", kinds.iter().map(|(k, l)| (l.to_string(), kind == *k, Some(Msg::Text(page, kk, k.to_string())))).collect()));
                let input = serde_json::json!({"name": name, "exec": cmd, "kind": kind}).to_string();
                let add = if ready && !name.trim().is_empty() && !cmd.trim().is_empty() { Some(Msg::Run(page, vec!["device-action-add".into()], Some(input))) } else { None };
                v.push(button_row(&p, G::Plus, "Add this action".into(), String::new(), "Add", add));
            }
            "hdrcal" => {
                let h = &a.kde.hdr;
                let o = hdr_out(h).cloned().unwrap_or(Value::Null);
                let capable = o.pointer("/hdrCal/capable").and_then(Value::as_bool).unwrap_or(false);
                if let Some(n) = h.keep {
                    let keep = Msg::HdrKeep(true);
                    let back = Msg::HdrKeep(false);
                    nav::control(Kind::Press, vec![act(keep.clone())], Vec::new(), Vec::new(), String::new(), String::new());
                    nav::control(Kind::Press, vec![act(back.clone())], Vec::new(), Vec::new(), String::new(), String::new());
                    v.push(srow(&p, G::Display, "Keep these settings?", format!("They go back by themselves in {n} s"), row![btn(&p, "Keep", hearth_ui::ui::Kind::Primary, false, Some(keep)), btn(&p, "Go back", hearth_ui::ui::Kind::Normal, false, Some(back))].spacing(8.0).into()));
                }
                if h.info.is_null() {
                    v.push(small(&p, "Reading the screens…"));
                } else if !capable {
                    v.push(words(&p, "No screen here has HDR on with a brightness KWin knows. Turn HDR on for a screen in Display, then come back.", false));
                } else {
                    let c = o.get("hdrCal").cloned().unwrap_or(Value::Null);
                    let val = |set: &str, says: &str, unit: &str| match c.get(set).and_then(Value::as_f64) {
                        Some(x) => format!("{x} {unit} (calibrated)"),
                        None => format!("{} {unit} (what the screen says)", c.get(says).map(text_of).unwrap_or_else(|| "?".into())),
                    };
                    v.push(fact(&p, "Screen", format!("{} ({})", s(&o, "description"), s(&o, "name"))));
                    v.push(fact(&p, "Brightest spot", val("peakSet", "peak", "nits")));
                    v.push(fact(&p, "Brightest across the screen", val("avgSet", "avg", "nits")));
                    v.push(fact(&p, "Darkest", val("minSet", "min", "nits")));
                    let start = if h.keep.is_none() { Some(Msg::HdrStart) } else { None };
                    v.push(button_row(&p, G::Display, "Calibrate this screen".into(), "Three steps over the whole screen, each with a test picture: the screen changes as you move it".into(), "Start", start));
                }
                if let Some(e) = say_view(&p, &h.say) {
                    v.push(e);
                }
            }
            "padtest" => {
                if a.kde.pads.is_empty() {
                    v.push(card(&p, t("Plug in a controller or switch one on: its buttons, sticks and triggers show here as you use them.", sz(14.0), p.muted)));
                }
                for pad in a.kde.pads.iter() {
                    v.push(pad_card(&p, pad));
                }
            }
            "shortcuts" => {
                let sc = pg.data.pointer("/current/shortcuts").cloned().unwrap_or(Value::Null);
                let comps = sc.get("components").and_then(Value::as_array).cloned().unwrap_or_default();
                let live = sc.get("running").and_then(Value::as_bool).unwrap_or(false);
                if pg.read && !live {
                    v.push(words(&p, "KDE's shortcuts are changed through KDE Plasma's shortcut service: open this page on KDE Plasma to change them. Here they're shown as they're saved.", false));
                }
                if !s(&sc, "why").is_empty() {
                    v.push(words(&p, s(&sc, "why").to_string(), true));
                }
                if let Some((_, c, a)) = &a.kde.capture {
                    v.push(words(&p, format!("Press the new shortcut for {} (Esc: cancel)…", comps.iter().find(|x| s(x, "id") == c).and_then(|x| x.get("actions")).and_then(Value::as_array).and_then(|l| l.iter().find(|y| s(y, "id") == a)).map(|y| s(y, "name").to_string()).unwrap_or_default()), false));
                }
                // Your own commands with a shortcut (KDE's "Add New › Command or Script").
                let cmds = pg.data.pointer("/current/commands").and_then(Value::as_array).cloned().unwrap_or_default();
                v.push(head(&p, "Your own commands"));
                for c in cmds.iter() {
                    let file = s(c, "file").to_string();
                    let keys: Vec<String> = c.get("keys").and_then(Value::as_array).map(|k| k.iter().map(text_of).collect()).unwrap_or_default();
                    let ch = if live { Some(Msg::Capture(page, format!("services][{file}"), "_launch".into())) } else { None };
                    let rm = if ready { Some(Msg::Run(page, vec!["command-remove".into(), file.clone()], None)) } else { None };
                    nav::control(Kind::Press, ch.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                    nav::control(Kind::Press, rm.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                    v.push(srow(&p, G::Rocket, s(c, "name").to_string(), format!("{} · {}", if keys.is_empty() { "No shortcut".to_string() } else { keys.join(" or ") }, s(c, "exec")), row![btn(&p, "Change", hearth_ui::ui::Kind::Normal, false, ch), btn(&p, "Remove", hearth_ui::ui::Kind::Normal, false, rm)].spacing(6.0).into()));
                }
                let (cn, ce, ck) = (1000 + i * 100 + 80, 1000 + i * 100 + 81, 1000 + i * 100 + 82);
                let cname = pg.text.get(&cn).cloned().unwrap_or_default();
                let cexec = pg.text.get(&ce).cloned().unwrap_or_default();
                let ckeys = pg.text.get(&ck).cloned().unwrap_or_default();
                v.push(srow(&p, G::Plus, "A new command: its name", "", field(&p, "Open my notes", &cname, move |t| Msg::Text(page, cn, t)).width(Length::Fixed(sz(300.0))).into()));
                v.push(srow(&p, G::Cog, "What it runs", "A command or a script", field(&p, "kate ~/notes.txt", &cexec, move |t| Msg::Text(page, ce, t)).width(Length::Fixed(sz(300.0))).into()));
                v.push(srow(&p, G::Keyboard, "Its shortcut", "Like Meta+N (Meta is the Windows key)", field(&p, "Meta+N", &ckeys, move |t| Msg::Text(page, ck, t)).width(Length::Fixed(sz(300.0))).into()));
                let input = serde_json::json!({"name": cname, "exec": cexec, "keys": ckeys}).to_string();
                let add = if ready && !cname.trim().is_empty() && !cexec.trim().is_empty() { Some(Msg::Run(page, vec!["command-set".into()], Some(input))) } else { None };
                v.push(button_row(&p, G::Plus, "Add this command".into(), String::new(), "Add", add));
                let qkey = 1000 + i * 100 + 50;
                let qraw = pg.text.get(&qkey).cloned().unwrap_or_default();
                let q = qraw.to_lowercase();
                v.push(srow(&p, G::Search, "Find a shortcut", "By what it does or its keys", field(&p, "Screenshot, Meta+L…", &qraw, move |t| Msg::Text(page, qkey, t)).width(Length::Fixed(sz(300.0))).into()));
                let open = a.kde.open_comp.clone();
                for c in comps.iter() {
                    let cid = s(c, "id").to_string();
                    let acts = c.get("actions").and_then(Value::as_array).cloned().unwrap_or_default();
                    let matching: Vec<&Value> = acts.iter().filter(|x| q.is_empty() || s(x, "name").to_lowercase().contains(&q) || x.get("keys").map(|k| k.to_string().to_lowercase().contains(&q)).unwrap_or(false)).collect();
                    if matching.is_empty() {
                        continue;
                    }
                    let shown = !q.is_empty() || open.as_deref() == Some(cid.as_str());
                    if !shown {
                        v.push(link_row(&p, G::Keyboard, s(c, "name").to_string(), format!("{} shortcuts", acts.len()), Msg::OpenComp(Some(cid.clone()))));
                        continue;
                    }
                    v.push(head(&p, s(c, "name").to_string()));
                    for x in matching {
                        let aid = s(x, "id").to_string();
                        let keys: Vec<String> = x.get("keys").and_then(Value::as_array).map(|k| k.iter().map(text_of).collect()).unwrap_or_default();
                        let dflt: Vec<String> = x.get("default").and_then(Value::as_array).map(|k| k.iter().map(text_of).collect()).unwrap_or_default();
                        let ch = if live { Some(Msg::Capture(page, cid.clone(), aid.clone())) } else { None };
                        let df = if live && keys != dflt { Some(Msg::Run(page, vec!["shortcut-set".into(), cid.clone(), aid.clone(), dflt.join("\t")], None)) } else { None };
                        let cl = if live && !keys.is_empty() { Some(Msg::Run(page, vec!["shortcut-set".into(), cid.clone(), aid.clone(), String::new()], None)) } else { None };
                        for m in [&ch, &df, &cl] {
                            nav::control(Kind::Press, m.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                        }
                        let ctl: E<'a> = row![btn(&p, "Change", hearth_ui::ui::Kind::Normal, false, ch), btn(&p, "Default", hearth_ui::ui::Kind::Normal, false, df), btn(&p, "None", hearth_ui::ui::Kind::Normal, false, cl)].spacing(6.0).align_y(iced::Center).into();
                        let line = if keys.is_empty() { "No shortcut".to_string() } else { keys.join("  or  ") };
                        let line = if keys != dflt && !dflt.is_empty() { format!("{line} · KDE's: {}", dflt.join(" or ")) } else { line };
                        v.push(srow(&p, G::Keyboard, s(x, "name").to_string(), line, ctl));
                    }
                }
            }
            "rdpusers" => {
                let ru = pg.data.pointer("/current/rdpusers").cloned().unwrap_or(Value::Null);
                let users: Vec<String> = ru.get("users").and_then(Value::as_array).map(|a| a.iter().map(text_of).collect()).unwrap_or_default();
                v.push(head(&p, "Other people who may connect"));
                if users.is_empty() {
                    v.push(small(&p, "None yet."));
                }
                for u in users.iter() {
                    let m = if ready { Some(Msg::Run(page, vec!["rdp-user-remove".into(), u.clone()], None)) } else { None };
                    v.push(button_row(&p, G::User, u.clone(), "Its password is in KDE Wallet".into(), "Remove", m));
                }
                let (uk, pk) = (1000 + i * 100 + 70, 1000 + i * 100 + 71);
                let uname = pg.text.get(&uk).cloned().unwrap_or_default();
                v.push(srow(&p, G::Plus, "Their name", "What they type in their RDP app", field(&p, "", &uname, move |t| Msg::Text(page, uk, t)).width(Length::Fixed(sz(280.0))).into()));
                let typed = pg.text.get(&pk).map(|x| !x.is_empty()).unwrap_or(false);
                v.push(srow(&p, G::Shield, "Their password", "Kept in KDE Wallet, never anywhere else", field(&p, "", pg.text.get(&pk).map(String::as_str).unwrap_or(""), move |t| Msg::Text(page, pk, t)).secure(true).width(Length::Fixed(sz(280.0))).into()));
                let add = if ready && typed && !uname.trim().is_empty() { Some(Msg::Secret(page, vec!["rdp-user-add".into(), uname.trim().to_string()], pk)) } else { None };
                v.push(button_row(&p, G::Plus, "Add this person".into(), String::new(), "Add", add));
            }
            "getnew" => {
                let kind: &'static str = match s(r, "kind") {
                    "lookandfeel" => "lookandfeel",
                    "colorschemes" => "colorschemes",
                    "plasma-themes" => "plasma-themes",
                    _ => "aurorae",
                };
                v.push(link_row(&p, G::Store, "Get New…", "More from the KDE Store, installed for you only", Msg::Store(kind)));
            }
            "store" => {
                let st = &a.kde;
                let kinds = [("lookandfeel", "Global themes"), ("colorschemes", "Colour schemes"), ("plasma-themes", "Plasma styles"), ("aurorae", "Title bars")];
                let label = kinds.iter().find(|(k, _)| *k == st.store_kind).map(|(_, l)| *l).unwrap_or("Global themes");
                v.push(choice_wide(&p, G::Store, "From the KDE Store", label, kinds.iter().map(|(k, l)| (l.to_string(), st.store_kind == *k, Some(Msg::Store(*k)))).collect()));
                v.push(srow(&p, G::Search, "Find", "", row![field(&p, "Search the KDE Store", &st.store_q, Msg::StoreSearch).on_submit(Msg::StoreFind).width(Length::Fixed(sz(260.0))), btn(&p, "Search", hearth_ui::ui::Kind::Normal, false, Some(Msg::StoreFind))].spacing(8.0).into()));
                let sorts = [("down", "Most downloaded"), ("new", "Newest"), ("high", "Best rated")];
                v.push(choice(&p, G::List, "Order", "", sorts.iter().map(|(k, l)| (l.to_string(), st.store_sort == *k, Some(Msg::StoreSort(*k)))).collect()));
                if let Some(e) = say_view(&p, &st.store_say) {
                    v.push(e);
                }
                if st.store.is_null() {
                    v.push(small(&p, "Asking the KDE Store…"));
                }
                let items = st.store.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
                for x in items.iter() {
                    let id = s(x, "id").to_string();
                    let inst = x.get("installed").and_then(Value::as_bool).unwrap_or(false);
                    let busy = st.store_busy == id;
                    let m = if busy || !st.store_busy.is_empty() { None } else if inst { Some(Msg::StoreRun(vec!["store-remove".into(), st.store_kind.to_string(), id.clone()])) } else { Some(Msg::StoreRun(vec!["store-install".into(), st.store_kind.to_string(), id.clone()])) };
                    nav::control(Kind::Press, m.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                    let it = nav::item(s(x, "name"), "", false);
                    let pic: E<'a> = match crate::pics::get(s(x, "thumb"), crate::pics::WALL) {
                        Some(h) => crate::pics::tile(h, 190.0, 96.0),
                        None => container(Space::new()).width(Length::Fixed(190.0)).height(Length::Fixed(96.0)).style(hearth_ui::ui::boxed(Pal::mix(p.panel, p.text, 0.08), p.r(8.0), None)).into(),
                    };
                    let words_ = column![tm(s(x, "name").to_string(), sz(15.0), p.text), t(format!("by {} · {} downloads{}", s(x, "by"), s(x, "downloads"), if s(x, "score").is_empty() { String::new() } else { format!(" · rated {}%", s(x, "score")) }), sz(12.0), p.muted), t(s(x, "summary").to_string(), sz(12.0), p.muted)].spacing(3.0).width(Length::Fill);
                    let b = btn(&p, if busy { if inst { "Removing…" } else { "Installing…" } } else if inst { "Remove" } else { "Install" }, if inst { hearth_ui::ui::Kind::Normal } else { hearth_ui::ui::Kind::Primary }, false, m);
                    let body = container(row![pic, words_, b].spacing(16.0).align_y(iced::Center)).id(it.key).padding(12.0).width(Length::Fill).style(row_style(p, p.panel, p.r(10.0), it.lit));
                    v.push(ring(&p, it.lit, p.r(10.0), body));
                }
                v.push(small(&p, "From store.kde.org (the KDE Store), as KDE's own Get New… window shows it. Installed in your own folders only; Remove takes away just what was installed here."));
            }
            "push" => {
                let pu = pg.data.pointer("/current/push").cloned().unwrap_or(Value::Null);
                let live = pu.get("running").and_then(Value::as_bool).unwrap_or(false);
                if pg.read && !live {
                    v.push(words(&p, "KDE's push service isn't running here (it runs on KDE Plasma).", false));
                }
                let cur = s(&pu, "pushProviderId").to_string();
                let ukey = 1000 + i * 100 + 60;
                let url = pg.text.get(&ukey).cloned().unwrap_or_else(|| s(&pu, "url").to_string());
                let names = [("Ntfy", "ntfy"), ("Autopush", "Mozilla Autopush"), ("Gotify", "Gotify"), ("NextPush", "NextPush (Nextcloud)")];
                let items: Vec<(String, bool, Option<Msg>)> = names.iter().map(|(k, l)| (l.to_string(), cur == *k, if live && (*k == "Ntfy" || *k == "Autopush") { Some(Msg::Run(page, vec!["push-set".into(), k.to_string(), url.clone()], None)) } else { None })).collect();
                v.push(choice_wide(&p, G::Server, "Push service", if cur.is_empty() { "None picked: apps can't get notices from online services".to_string() } else { "Apps' notices from online services come through it".to_string() }, items));
                let save = if live && (cur == "Ntfy" || cur == "Autopush") { Some(Msg::Run(page, vec!["push-set".into(), cur.clone(), url.clone()], None)) } else { None };
                nav::control(Kind::Press, save.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                v.push(srow(&p, G::Link, "Its server", "Empty: the service's own", row![field(&p, "https://ntfy.sh", &url, move |t| Msg::Text(page, ukey, t)).width(Length::Fixed(sz(280.0))), btn(&p, "Save", hearth_ui::ui::Kind::Normal, false, save)].spacing(8.0).align_y(iced::Center).into()));
                // Gotify and NextPush: your server, and its token or app password (handed to KDE's push service, which
                // keeps it; never shown, saved or logged by Hearth).
                let (gu, nu, nn, sk) = (1000 + i * 100 + 61, 1000 + i * 100 + 62, 1000 + i * 100 + 63, 1000 + i * 100 + 64);
                v.push(head(&p, "Gotify or NextPush (your own server)"));
                let gurl = pg.text.get(&gu).cloned().unwrap_or_else(|| if cur == "Gotify" || cur == "NextPush" { s(&pu, "url").to_string() } else { String::new() });
                v.push(srow(&p, G::Link, "Server", "Your Gotify or Nextcloud's web address", field(&p, "https://push.example.org", &gurl, move |t| Msg::Text(page, gu, t)).width(Length::Fixed(sz(300.0))).into()));
                let nname = pg.text.get(&nn).cloned().unwrap_or_default();
                v.push(srow(&p, G::User, "Nextcloud name", "Only for NextPush", field(&p, "", &nname, move |t| Msg::Text(page, nn, t)).width(Length::Fixed(sz(300.0))).into()));
                let typed = pg.text.get(&sk).map(|x| !x.is_empty()).unwrap_or(false);
                let dots = "•".repeat(pg.text.get(&sk).map(|x| x.chars().count()).unwrap_or(0));
                let _ = (&nu, &dots);
                v.push(srow(&p, G::Shield, "Token or app password", "Gotify's client token, or an app password from Nextcloud's security settings", field(&p, "", pg.text.get(&sk).map(String::as_str).unwrap_or(""), move |t| Msg::Text(page, sk, t)).secure(true).width(Length::Fixed(sz(300.0))).into()));
                let ok = live && typed && gurl.starts_with("http");
                let g_msg = if ok { Some(Msg::Secret(page, vec!["push-signin".into(), "Gotify".into(), gurl.clone()], sk)) } else { None };
                let n_msg = if ok && !nname.trim().is_empty() { Some(Msg::Secret(page, vec!["push-signin".into(), "NextPush".into(), gurl.clone(), nname.clone()], sk)) } else { None };
                for m in [&g_msg, &n_msg] {
                    nav::control(Kind::Press, m.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                }
                v.push(srow(&p, G::Check, "Use it", "The token or password goes to KDE's push service only", row![btn(&p, "Use Gotify", hearth_ui::ui::Kind::Normal, false, g_msg), btn(&p, "Use NextPush", hearth_ui::ui::Kind::Normal, false, n_msg)].spacing(8.0).into()));
            }
            "fonts" => {
                let f = pg.data.pointer("/current/fonts").cloned().unwrap_or(Value::Null);
                let yours = f.get("yours").and_then(Value::as_array).cloned().unwrap_or_default();
                let inst = if ready { Some(Msg::FileFor(page, "font-install", "*.ttf *.otf *.ttc *.woff2")) } else { None };
                v.push(button_row(&p, G::Plus, "Install a font".into(), "A .ttf, .otf or .ttc file: for you only, every app can use it".into(), "Choose a font…", inst));
                // The preview: the font picked below, drawn in itself.
                if let Some(fam) = &a.kde.font_shown {
                    let loaded = a.kde.fonts_loaded.iter().find(|x| **x == fam.as_str()).copied();
                    let sample = |text: &'static str, size: f32| -> E<'a> {
                        match loaded {
                            Some(name) => iced::widget::text(text).size(sz(size)).color(p.text).font(iced::Font { family: iced::font::Family::Name(name), ..iced::Font::DEFAULT }).into(),
                            None => t("Loading…", sz(14.0), p.muted).into(),
                        }
                    };
                    v.push(card(&p, column![
                        row![tm(fam.clone(), sz(15.0), p.text).width(Length::Fill), btn(&p, "Close", hearth_ui::ui::Kind::Normal, false, Some(Msg::FontShow(String::new(), String::new())))].align_y(iced::Center),
                        sample("The quick brown fox jumps over the lazy dog", 28.0),
                        sample("ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz 0123456789 !?&@€", 18.0),
                        sample("Hearth OS · Sphinx of black quartz, judge my vow.", 14.0),
                    ].spacing(10.0)));
                }
                v.push(head(&p, "Fonts you installed"));
                if yours.is_empty() {
                    v.push(small(&p, if pg.read { "None yet." } else { "Reading…" }));
                }
                for u in yours.iter() {
                    let file = s(u, "file").to_string();
                    let off = u.get("off").and_then(Value::as_bool).unwrap_or(false);
                    let name = format!("{}{}{}", s(u, "family"), if s(u, "style").is_empty() { "" } else { " · " }, s(u, "style"));
                    let tog = if ready { Some(Msg::Run(page, vec!["font-off".into(), file.clone(), if off { "on".into() } else { "off".into() }], None)) } else { None };
                    let pv = Some(Msg::FontShow(s(u, "family").to_string(), file.clone()));
                    nav::control(Kind::Press, pv.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                    let rm = if ready { Some(Msg::Run(page, vec!["font-remove".into(), file.clone()], None)) } else { None };
                    nav::control(Kind::Toggle(!off), tog.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                    nav::control(Kind::Press, rm.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                    let ctl: E<'a> = row![btn(&p, "Preview", hearth_ui::ui::Kind::Normal, false, pv), btn(&p, if off { "Turn on" } else { "Turn off" }, hearth_ui::ui::Kind::Normal, false, tog), btn(&p, "Remove", hearth_ui::ui::Kind::Normal, false, rm)].spacing(8.0).align_y(iced::Center).into();
                    v.push(srow(&p, G::Palette, name, format!("{}{}", if off { "Turned off · " } else { "" }, file.rsplit('/').next().unwrap_or("")), ctl));
                }
                // Every font on this PC: pick one to see it.
                let fams = f.get("families").and_then(Value::as_array).cloned().unwrap_or_default();
                let files = f.get("files").cloned().unwrap_or(Value::Null);
                if !fams.is_empty() {
                    v.push(head(&p, format!("Every font on this PC ({})", fams.len())));
                    for fam in fams.iter().map(text_of) {
                        let file = files.get(&fam).map(text_of).unwrap_or_default();
                        let m = if file.is_empty() { None } else { Some(Msg::FontShow(fam.clone(), file)) };
                        let here = a.kde.font_shown.as_deref() == Some(fam.as_str());
                        nav::control(Kind::Press, m.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                        let ctl: E<'a> = if here { t("Shown above", sz(13.0), p.accent).into() } else { btn(&p, "Preview", hearth_ui::ui::Kind::Normal, false, m) };
                        v.push(srow(&p, G::Palette, fam.clone(), String::new(), ctl));
                    }
                }
            }
            "colord" => {
                let c = pg.data.pointer("/current/colord").cloned().unwrap_or(Value::Null);
                if pg.read && !c.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    v.push(words(&p, format!("{}.", if s(&c, "why").is_empty() { "Colour management isn't running" } else { s(&c, "why") }), false));
                }
                let profiles = c.get("profiles").and_then(Value::as_array).cloned().unwrap_or_default();
                let devices = c.get("devices").and_then(Value::as_array).cloned().unwrap_or_default();
                for d in devices.iter() {
                    let dpath = s(d, "path").to_string();
                    let has: Vec<String> = d.get("profiles").and_then(Value::as_array).map(|a| a.iter().map(text_of).collect()).unwrap_or_default();
                    let cur = has.first().cloned().unwrap_or_default();
                    let kind = match s(d, "kind") { "display" => "Screen", "printer" => "Printer", "camera" => "Camera", "scanner" => "Scanner", "webcam" => "Webcam", k => k };
                    v.push(head(&p, format!("{kind}: {}", s(d, "name"))));
                    if profiles.is_empty() {
                        v.push(small(&p, "No colour profiles are installed."));
                    }
                    for pr in profiles.iter() {
                        let ppath = s(pr, "path").to_string();
                        let here = ppath == cur;
                        let m = if ready && !here { Some(Msg::Run(page, vec!["colord-default".into(), dpath.clone(), ppath.clone()], None)) } else { None };
                        nav::control(Kind::Press, m.iter().cloned().map(act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
                        let ctl: E<'a> = if here { row![t("In use", sz(13.0), p.accent), hearth_ui::ui::glyph(G::Check, p.accent, 16.0, 2.4)].spacing(8.0).align_y(iced::Center).into() } else { btn(&p, "Use", hearth_ui::ui::Kind::Normal, false, m) };
                        v.push(srow(&p, G::Palette, s(pr, "title").to_string(), s(pr, "file").rsplit('/').next().unwrap_or("").to_string(), ctl));
                    }
                }
                let imp = if ready { Some(Msg::FileFor(page, "colord-import", "*.icc *.icm")) } else { None };
                v.push(button_row(&p, G::Plus, "Add a colour profile".into(), "An .icc file from your screen's or printer's maker, or from a calibration".into(), "Choose a file…", imp));
            }
            "link" => {
                let to = sid(s(r, "page"));
                v.push(link_row(&p, g, title, sub, Msg::Go(to)));
            }
            _ => {}
        }
    }
    if pg.saving {
        v.push(small(&p, "Saving…"));
    }
    if let Some(e) = say_view(&p, &pg.say) {
        v.push(e);
    }
    Some(column_of(v))
}

/// Calibrate HDR over the whole window: the step's test picture on black, and a card with its value.
fn hdr_view(a: &App) -> E<'_> {
    let p = a.sh.p;
    let h = &a.kde.hdr;
    let (title, sub, lo, hi, unit) = HDR_STEPS[h.step];
    let v = h.vals[h.step];
    let white = Color::WHITE;
    let picture: E<'_> = match h.step {
        0 => container(container(Space::new()).width(Length::Fixed(140.0)).height(Length::Fixed(140.0)).style(hearth_ui::ui::boxed(Color::from_rgb8(0xf4, 0xf4, 0xf4), 0.0, None)))
            .center_x(Length::Fixed(350.0))
            .center_y(Length::Fixed(350.0))
            .style(hearth_ui::ui::boxed(white, 0.0, None))
            .into(),
        1 => container(Space::new()).width(Length::Fill).height(Length::Fill).style(hearth_ui::ui::boxed(white, 0.0, None)).into(),
        _ => {
            let mut r = iced::widget::Row::new().spacing(24.0);
            for k in [1u8, 2, 3, 4, 6, 8] {
                r = r.push(container(Space::new()).width(Length::Fixed(110.0)).height(Length::Fixed(110.0)).style(hearth_ui::ui::boxed(Color::from_rgb8(k, k, k), 8.0, None)));
            }
            r.into()
        }
    };
    let shown = if h.step == 2 { format!("{v:.3} {unit}") } else { format!("{} {unit}", v.round()) };
    let screen_says = hdr_out(h).and_then(|o| o.get("hdrCal")).and_then(|c| c.get(["peak", "avg", "min"][h.step])).map(text_of).unwrap_or_else(|| "?".into());
    let step = (hi - lo) / 200.0;
    let card = column![
        row![tm(format!("Calibrate HDR · {} of 3 · {title}", h.step + 1), sz(20.0), p.text).width(Length::Fill), tm(shown, sz(20.0), p.accent)].align_y(iced::Center),
        t(sub, sz(14.0), p.muted),
        srow(&p, G::Display, "Brightness", format!("The screen says {screen_says} nits"), slider_f32(v, lo..=hi, step, Msg::HdrSet, Msg::HdrApply)),
        row![
            Space::new().width(Length::Fill),
            btn(&p, "Cancel", hearth_ui::ui::Kind::Normal, false, Some(Msg::HdrCancel)),
            btn(&p, "What the screen says", hearth_ui::ui::Kind::Normal, false, Some(Msg::HdrReset)),
            btn(&p, "Back", hearth_ui::ui::Kind::Normal, false, if h.step > 0 { Some(Msg::HdrStep(-1)) } else { None }),
            btn(&p, if h.step < 2 { "Next" } else { "Finish" }, hearth_ui::ui::Kind::Primary, false, Some(Msg::HdrStep(1))),
        ]
        .spacing(10.0),
    ]
    .spacing(10.0);
    for m in [Msg::HdrCancel, Msg::HdrReset, Msg::HdrStep(-1), Msg::HdrStep(1)] {
        nav::control(Kind::Press, vec![act(m)], Vec::new(), Vec::new(), String::new(), String::new());
    }
    let card = container(card).padding(20.0).width(Length::Fixed(760.0)).style(hearth_ui::ui::boxed(p.panel, p.r(18.0), Some((p.line, 1.0))));
    let pic: E<'_> = if h.step == 1 { picture } else { container(picture).center_x(Length::Fill).center_y(Length::Fill).into() };
    container(iced::widget::stack![pic, container(card).center_x(Length::Fill).align_bottom(Length::Fill).padding(48.0)])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(hearth_ui::ui::boxed(Color::BLACK, 0.0, None))
        .into()
}

/// One controller in the live test: its buttons lit while held, both sticks, the triggers and the D-pad.
fn pad_card<'a>(p: &Pal, pad: &pad_test::Pad) -> E<'a> {
    let lit = |on: bool, label: String| -> E<'a> {
        container(t(label, sz(13.0), if on { p.ink } else { p.text }))
            .padding(Padding { top: 6.0, right: 12.0, bottom: 6.0, left: 12.0 })
            .style(hearth_ui::ui::boxed(if on { p.accent } else { Pal::mix(p.panel, p.text, 0.08) }, p.r(8.0), None))
            .into()
    };
    // Two rows, so they fit beside the page's list in the controller look too.
    let (mut b1, mut b2) = (iced::widget::Row::new().spacing(6.0), iced::widget::Row::new().spacing(6.0));
    for (k, (code, name)) in pad_test::BUTTONS.iter().enumerate() {
        let b = lit(pad.held.contains(code), name.to_string());
        if k < 6 {
            b1 = b1.push(b);
        } else {
            b2 = b2.push(b);
        }
    }
    let buttons = column![b1, b2].spacing(6.0);
    let pp = *p;
    let stick = |x: f32, y: f32, name: &'static str| -> E<'a> {
        let size = 92.0;
        let dot = 16.0;
        let left = ((x + 1.0) / 2.0 * (size - dot)).clamp(0.0, size - dot);
        let top = ((y + 1.0) / 2.0 * (size - dot)).clamp(0.0, size - dot);
        let field = container(container(container(Space::new()).width(Length::Fixed(dot)).height(Length::Fixed(dot)).style(hearth_ui::ui::boxed(pp.accent, dot / 2.0, None))).padding(Padding { top, right: 0.0, bottom: 0.0, left }))
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .style(hearth_ui::ui::boxed(Pal::mix(pp.panel, pp.text, 0.06), size / 2.0, Some((pp.line, 1.0))));
        column![field, t(format!("{name}  {x:+.2}, {y:+.2}"), sz(12.0), pp.muted)].spacing(6.0).align_x(iced::Center).into()
    };
    let trigger = |v: f32, name: &'static str| -> E<'a> {
        let w = 140.0;
        column![
            t(format!("{name}  {:.0} %", v * 100.0), sz(12.0), pp.muted),
            container(container(Space::new()).width(Length::Fixed((w * v).max(2.0))).height(Length::Fixed(10.0)).style(hearth_ui::ui::boxed(pp.accent, 5.0, None))).width(Length::Fixed(w)).style(hearth_ui::ui::boxed(Pal::mix(pp.panel, pp.text, 0.08), 5.0, None)),
        ]
        .spacing(6.0)
        .into()
    };
    let dpad = row![lit(pad.hat[0] < 0, "←".into()), column![lit(pad.hat[1] < 0, "↑".into()), lit(pad.hat[1] > 0, "↓".into())].spacing(4.0), lit(pad.hat[0] > 0, "→".into())].spacing(4.0).align_y(iced::Center);
    card(
        p,
        column![
            tm(pad.name.clone(), sz(15.0), p.text),
            buttons,
            row![stick(pad.axes[0], pad.axes[1], "Left stick"), stick(pad.axes[2], pad.axes[3], "Right stick"), column![trigger(pad.axes[4], "LT"), trigger(pad.axes[5], "RT")].spacing(14.0), column![t("D-pad", sz(12.0), p.muted), dpad].spacing(6.0)].spacing(28.0).align_y(iced::Center),
        ]
        .spacing(14.0),
    )
}

/// A long list open over the page: every choice, the chosen one marked.
pub fn dialog<'a>(a: &'a App) -> Option<(E<'a>, Option<Msg>)> {
    if let Ok(mut w) = SHOWN.lock() {
        *w = a.page;
    }
    if a.page == "kde-hdr" && a.kde.hdr.on {
        return Some((hdr_view(a), None));
    }
    let (page, i) = a.kde.pick?;
    if page != a.page {
        return None;
    }
    let p = a.sh.p;
    let pg = a.kde.pages.get(page)?;
    let r = row_of(page, i)?;
    let items = list(pg, r);
    let cur = row_val(pg, r);
    let n = items.len();
    let mut c = Column::new().spacing(4.0);
    for (k, label) in items {
        let on = k == cur;
        let msg = Msg::Picked(page, i, k.clone());
        nav::control(Kind::Press, vec![act(msg.clone())], Vec::new(), Vec::new(), String::new(), String::new());
        let it = nav::item(&label, "", false);
        let mark: E<'a> = if on { hearth_ui::ui::glyph(G::Check, p.accent, 16.0, 2.4) } else { Space::new().width(Length::Fixed(16.0)).into() };
        let body = container(row![tm(label, sz(14.0), if on { p.accent } else { p.text }).width(Length::Fill), mark].spacing(10.0).align_y(iced::Center))
            .id(it.key)
            .padding(Padding { top: 8.0, right: 12.0, bottom: 8.0, left: 12.0 })
            .width(Length::Fill)
            .style(row_style(p, if on { Pal::mix(p.panel, p.accent, 0.1) } else { p.panel }, p.r(8.0), it.lit));
        c = c.push(mouse_area(ring(&p, it.lit, p.r(8.0), body)).on_press(msg).interaction(mouse::Interaction::Pointer));
    }
    let body = column![
        row![tm(s(r, "title").to_string(), sz(17.0), p.text), Space::new().width(Length::Fill), btn(&p, "Close", hearth_ui::ui::Kind::Normal, false, Some(Msg::Pick(None)))].align_y(iced::Center),
        scrollable(c).height(Length::Fixed((n as f32 * sz(44.0)).clamp(60.0, 460.0))),
    ]
    .spacing(12.0);
    let d = container(body).padding(20.0).width(Length::Fixed(sz(460.0))).style(hearth_ui::ui::boxed(p.panel, p.r(14.0), Some((p.line, 1.0))));
    Some((d.into(), Some(Msg::Pick(None))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spec_and_the_pages_agree() {
        let ids: Vec<&str> = spec()["pages"].as_array().unwrap().iter().map(|p| p["id"].as_str().unwrap()).collect();
        assert_eq!(ids, PAGES.to_vec(), "PAGES must list kde-settings.json's pages in order");
        for id in PAGES {
            assert!(crate::registry::page(id).is_some(), "{id} isn't in pages_list.rs: run tools/settings-registry-rs.py");
            for r in rows_of(id) {
                let ty = s(r, "type");
                assert!(["rdpusers", "getnew", "store", "hdrcal", "padtest", "shortcuts", "push", "fonts", "colord", "plugins", "kded", "activities", "userdirs", "deviceactions", "hub", "head", "note", "switch", "choice", "number", "pick", "text", "file", "firmware", "info", "link"].contains(&ty), "{id}: row type {ty}");
                if ["switch", "choice", "number", "text", "file"].contains(&ty) {
                    assert!(!s(r, "file").is_empty() && !s(r, "group").is_empty() && !s(r, "key").is_empty(), "{id}: {} has no file/group/key", s(r, "title"));
                }
                if ty == "link" {
                    assert!(crate::registry::page(s(r, "page")).is_some(), "{id}: link to {}", s(r, "page"));
                }
                if ty == "choice" {
                    assert!(r["opts"].as_array().map(|o| o.len() >= 2).unwrap_or(false), "{id}: {} needs options", s(r, "title"));
                }
            }
        }
    }

    #[test]
    fn a_choice_with_other_keys_is_chosen_only_when_they_match() {
        let mut pg = Pg::default();
        pg.read = true;
        pg.data = serde_json::json!({"files": {"kwinrc": {"org.kde.kdecoration2": {"ButtonsOnRight": "X", "ButtonsOnLeft": ""}}}});
        let i = rows_of("kde-decorations").iter().position(|r| s(r, "key") == "ButtonsOnRight").unwrap();
        let r = row_of("kde-decorations", i).unwrap();
        assert_eq!(chosen(&pg, r), Some(2));
        pg.data["files"]["kwinrc"]["org.kde.kdecoration2"]["ButtonsOnLeft"] = Value::String("MS".into());
        assert_eq!(chosen(&pg, r), None, "Close only with a menu on the left is your own layout");
        // Nothing set: KDE's default (HIAX with MS on the left).
        pg.data = serde_json::json!({"files": {"kwinrc": {}}});
        assert_eq!(chosen(&pg, r), Some(0));
    }

    #[test]
    fn needs_greys_a_row_until_its_switch_is_set() {
        let mut pg = Pg::default();
        pg.data = serde_json::json!({"files": {"kwinrc": {}}});
        let r = rows_of("kde-decorations").iter().find(|r| s(r, "key") == "BorderSize").unwrap();
        assert!(!needs_ok("kde-decorations", &pg, r), "the theme's own border size is on by default");
        pg.data["files"]["kwinrc"]["org.kde.kdecoration2"] = serde_json::json!({"BorderSizeAuto": "false"});
        assert!(needs_ok("kde-decorations", &pg, r));
    }
}

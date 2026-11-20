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
    "kde-settings", "kde-workspace", "kde-style", "kde-plasma-style", "kde-colors", "kde-global-theme", "kde-decorations", "kde-screenlock", "kde-session", "kde-renderer", "kde-feedback",
];

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
    /// A long list open over the page: (its page, its row).
    pub pick: Option<(&'static str, usize)>,
}

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
}

pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    st.want_pick = sh.part == "pick";
    let pg = st.pages.entry(page).or_default();
    pg.say.clear();
    pg.temp.clear();
    st.pick = None;
    read(helper(&["get", page], 30), move |r| Msg::Got(page, r))
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
        }
        Msg::Switch(page, i, on) => {
            let Some(r) = row_of(page, i) else { return Task::none() };
            if s(r, "type") == "firmware" {
                return update(sh, st, Msg::Firmware(page, on));
            }
            let (file, input) = change(s(r, "file"), s(r, "group"), kv(vec![(s(r, "key").into(), Value::Bool(on))]));
            sh.reread_history = true;
            return save(st, page, i, &file, input, vec![(s(r, "group").into(), s(r, "key").into(), on.to_string())]);
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

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn back(st: &mut State) -> bool {
    st.pick.take().is_some()
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
                let cur = row_val(pg, r);
                let is_on = truthy(&cur);
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
                let ctl: E<'a> = if on { slider_f32(cur.clamp(lo, hi), lo..=hi, step, move |x| Msg::Num(page, i, x), Msg::NumDone(page, i)) } else { t(shown.trim().to_string(), sz(13.0), p.muted).into() };
                v.push(srow(&p, g, title, line, ctl));
            }
            "pick" => {
                let items = list(pg, r);
                let cur = row_val(pg, r);
                let on = on || (ready && r.get("tool").is_some() && r.get("file").is_none());
                v.push(pick_row(&p, page, i, g, title, if items.is_empty() && pg.read { format!("{}{}Nothing to pick from is installed here", sub, if sub.is_empty() { "" } else { ". " }) } else { sub }, &items, &cur, on && !items.is_empty()));
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
                    let here: Vec<&Value> = spec()["pages"].as_array().map(|a| a.iter().filter(|x| s(x, "cat") == *cid && s(x, "id") != page).collect()).unwrap_or_default();
                    if here.is_empty() {
                        continue;
                    }
                    v.push(head(&p, *clabel));
                    for x in here {
                        v.push(link_row(&p, crate::frame::cat_glyph(cid), s(x, "label").to_string(), s(x, "desc").to_string(), Msg::Go(sid(s(x, "id")))));
                    }
                }
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

/// A long list open over the page: every choice, the chosen one marked.
pub fn dialog<'a>(a: &'a App) -> Option<(E<'a>, Option<Msg>)> {
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
                assert!(["hub", "head", "note", "switch", "choice", "number", "pick", "text", "file", "firmware", "info", "link"].contains(&ty), "{id}: row type {ty}");
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

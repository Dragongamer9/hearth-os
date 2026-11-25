//! Settings › AI (the approved mockups docs/mockups/settings-cleanup settings-09 and ai-1 to ai-4, Nov 23, 2026; the
//! owner: "fix the AI problem"): which AI answers and a test of it, the engines on this PC with their models, sharing
//! this PC's AI with your devices (who uses it, Stop), what uses your AI, and when something's wrong what happened in
//! plain words with one button that fixes it. On a device with no engine (a laptop) the same page says whose AI it
//! uses, over Hearth Link, found by itself.
//!
//! Reads: hearth-ai-check status / models / test (read only), hearth-ai-share status, hearth-link-settings ai-status,
//! hearth-home-server status. Changes go through their own tools: hearth_ai_engines.py set ollamaModel (Change model),
//! hearth-home-server set aiOn (Ollama on or off), hearth-link-settings ai-serve / ai-stop / ai-allow (sharing, Stop),
//! hearth-home-server-helper app-fix (Fix it, as admin). Nothing changes by itself (red flag 9).
//!
//! Describe it uses (the owner, Nov 24, 2026: Describe it sat on "Your AI is making it…" on his laptop's slow CPU Ollama
//! while his PC's AI was there): where Describe it asks first, Fastest available / This device / My other devices
//! (hearth-ai-check use describe {"from": …}), and which AI that is now, with how fast it writes when it's slow.
//! Shared from your other devices: every AI your other Hearth devices share (hearth-ai-check shared: the same list as
//! Settings › Home server › AI devices, from hearth-ai-share devices, with Hearth Link's), each with its state, engine,
//! models, graphics, speed when measured, whether Describe it uses it, and "Use this for Describe it" (hearth-ai-check
//! use-shared: the same pick, through use). A device Hearth Link couldn't confirm as yours says so, with Pair again.
//! Read when the page opens and on Refresh; nothing polls.

use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, t, tb, Kind as BtnKind};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Subscription, Task};
use serde_json::Value;

pub const PAGES: &[&str] = &["sec-ai"];

#[derive(Debug, Default)]
pub struct State {
    pub st: Value,
    pub share: Value,
    pub link: Value,
    pub models: Value,
    pub hs: Value,
    pub test: Option<Value>,
    pub testing: bool,
    /// Change model's sheet is open, with the model picked in it.
    pub picking: Option<String>,
    pub msg: String,
    pub busy: bool,
    /// Your paired devices (hearth-link-settings status): the ones not sharing their AI show in Where your AI comes from.
    pub peers: Value,
    /// What uses your AI: whose picker is open.
    pub menu: Option<&'static str>,
    /// Where your AI comes from: the group being dragged, and the one it's over.
    pub drag: Option<String>,
    pub over: Option<String>,
    /// Test it as Describe it does it (a laptop's view): when it was asked, and what came back.
    pub dtest: Option<Value>,
    pub dstart: Option<std::time::Instant>,
    pub dsecs: f32,
    /// Shared from your other devices (hearth-ai-check shared), and whether it's being looked up again
    pub shared: Value,
    pub sh_busy: bool,
}

/// What Test it asks (ai-2): a whole desktop, as Describe it would.
pub const TEST_ASK: &str = "make it like a cozy cabin with a clock and my games";

/// The uses with their own AI picker (ai-1): name, line, their id in ai-uses.json (hearth-ai-check use).
pub const USES: [(&str, &str, &str); 4] = [
    ("Describe it", "Desktop, panels and blocks", "describe"),
    ("Ithil", "Hearth AI on this PC", "ithil"),
    ("AI photos", "Finds people and things in Photos", "photos"),
    ("Sort my Downloads", "Hearth Files", "sortdl"),
];

#[derive(Debug, Clone)]
pub enum Msg {
    St(run::Output),
    Share(run::Output),
    Link(run::Output),
    Models(run::Output),
    Hs(run::Output),
    Test,
    Tested(run::Output),
    Pick(bool),
    PickModel(String),
    UseModel,
    Ollama(bool),
    Sharing(bool),
    Stop(String, bool),
    Fix(String),
    Did(run::Output),
    Go(&'static str),
    Open(Vec<String>),
    Copy(String),
    Peers(run::Output),
    Menu(Option<&'static str>),
    /// A use's AI picked: "hearth" or the AI's JSON (hearth-ai-check use USE …)
    UseFor(&'static str, String),
    DragStart(String),
    DragOver(String),
    DragEnd,
    DTested(run::Output),
    ShowIt,
    Speed(run::Output),
    /// Describe it uses… picked: fastest, here or link
    DescribeFrom(&'static str),
    Shared(run::Output),
    RefreshShared,
    /// Use this for Describe it: that device's AI (hearth-ai-check use-shared DEVICE)
    UseShared(String),
    UsedShared(run::Output),
}

/// Describe it uses… (Settings › AI): where it asks first, in hearth-ai-check's words.
pub const DESCRIBE_FROM: [(&str, &str); 3] = [("fastest", "Fastest available"), ("here", "This device"), ("link", "My other devices")];

fn py(name: &str, args: &[&str]) -> Vec<String> {
    let mut v = vec!["python3".to_string(), run::shell_helper(name)];
    v.extend(args.iter().map(|s| s.to_string()));
    v
}

fn load() -> Task<Msg> {
    Task::batch([
        Task::perform(run::spawn(py("hearth-ai-check", &["status"])), Msg::St),
        Task::perform(run::spawn(py("hearth-ai-share", &["status"])), Msg::Share),
        Task::perform(run::spawn(py("hearth-link-settings", &["ai-status"])), Msg::Link),
        Task::perform(run::spawn(py("hearth-ai-check", &["models"])), Msg::Models),
        Task::perform(run::spawn(py("hearth-home-server", &["status"])), Msg::Hs),
        Task::perform(run::spawn(py("hearth-link-settings", &["status"])), Msg::Peers),
        Task::perform(run::spawn(py("hearth-ai-check", &["shared"])), Msg::Shared),
    ])
}

pub fn open(_sh: &mut Shared, st: &mut State, _page: &'static str) -> Task<Msg> {
    st.msg.clear();
    st.picking = None;
    load()
}

fn json_of(o: &run::Output) -> Value {
    o.json().unwrap_or(Value::Null)
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::St(o) => st.st = json_of(&o),
        Msg::Share(o) => st.share = json_of(&o),
        Msg::Link(o) => st.link = json_of(&o),
        Msg::Models(o) => st.models = json_of(&o),
        Msg::Hs(o) => st.hs = json_of(&o),
        Msg::Test => {
            st.testing = true;
            if !here_engine(st) {
                // A device without an engine of its own (ai-2): Describe it's own ask, to the AI that answers, its
                // screen checked by Hearth (hearth-layout describe-screen); nothing is put in place.
                st.dstart = Some(std::time::Instant::now());
                return Task::perform(run::spawn(vec!["env".into(), "HEARTH_DESCRIBE_ASK_AI=1".into(), "hearth-layout".into(), "describe-screen".into(), TEST_ASK.into(), "null".into()]), Msg::DTested);
            }
            return Task::perform(run::spawn(py("hearth-ai-check", &["test"])), Msg::Tested);
        }
        Msg::Tested(o) => {
            st.testing = false;
            st.test = Some(json_of(&o));
            // the answering model's speed, measured now that it's loaded (Change model shows it)
            return Task::perform(run::spawn(py("hearth-ai-check", &["speed"])), Msg::Speed);
        }
        Msg::DTested(o) => {
            st.testing = false;
            st.dsecs = st.dstart.take().map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0);
            let mut v = json_of(&o);
            if v.is_null() {
                v = serde_json::json!({ "ok": false, "error": if o.why().is_empty() { "hearth-layout didn't answer".to_string() } else { o.why() } });
            }
            st.dtest = Some(v);
        }
        Msg::ShowIt => {
            // the editor (Build anything) opens with it; nothing is kept unless you press Keep this there
            if let Some(scr) = st.dtest.as_ref().map(|v| v["screen"].clone()).filter(|x| x.is_object()) {
                let dir = run::shell_dir().to_string_lossy().to_string();
                let j = scr.to_string();
                run::detached(&["quickshell", "ipc", "-p", &dir, "call", "screen", "editJson", &j, "Your AI's test"]);
            }
        }
        Msg::Pick(open) => {
            st.picking = if open { Some(answering_model(st)) } else { None };
            if open {
                // the models already loaded are timed (nothing new is loaded for it); the others say so
                return Task::perform(run::spawn(py("hearth-ai-check", &["speed"])), Msg::Speed);
            }
        }
        Msg::Speed(_) => return Task::perform(run::spawn(py("hearth-ai-check", &["models"])), Msg::Models),
        Msg::Peers(o) => st.peers = json_of(&o),
        Msg::Menu(m) => st.menu = if st.menu == m { None } else { m },
        Msg::UseFor(u, what) => {
            st.menu = None;
            st.busy = true;
            return Task::perform(run::spawn(py("hearth-ai-check", &["use", u, &what])), Msg::Did);
        }
        Msg::DragStart(k) => {
            st.drag = Some(k.clone());
            st.over = Some(k);
        }
        Msg::DragOver(k) => {
            if st.drag.is_some() {
                st.over = Some(k);
            }
        }
        Msg::DragEnd => {
            let (Some(d), Some(o)) = (st.drag.take(), st.over.take()) else { return Task::none() };
            let mut order = order_of(st);
            if d != o {
                if let (Some(i), Some(j)) = (order.iter().position(|x| *x == d), order.iter().position(|x| *x == o)) {
                    let x = order.remove(i);
                    order.insert(j, x);
                    st.busy = true;
                    return Task::perform(run::spawn(py("hearth-ai-check", &["order", &order.join(",")])), Msg::Did);
                }
            }
        }
        Msg::PickModel(m) => st.picking = Some(m),
        Msg::UseModel => {
            if let Some(m) = st.picking.take() {
                st.busy = true;
                return Task::perform(run::spawn(py("hearth_ai_engines.py", &["set", "ollamaModel", &m])), Msg::Did);
            }
        }
        Msg::Ollama(on) => {
            st.busy = true;
            return Task::perform(run::spawn(py("hearth-home-server", &["set", "aiOn", if on { "on" } else { "off" }])), Msg::Did);
        }
        Msg::Sharing(on) => {
            st.busy = true;
            return Task::perform(run::spawn(py("hearth-link-settings", &["ai-serve", if on { "on" } else { "off" }])), Msg::Did);
        }
        Msg::Stop(id, stop) => {
            st.busy = true;
            return Task::perform(run::spawn(py("hearth-link-settings", &[if stop { "ai-stop" } else { "ai-allow" }, &id])), Msg::Did);
        }
        Msg::Fix(app) => {
            st.busy = true;
            return Task::perform(run::spawn(vec!["pkexec".into(), "/usr/libexec/nimbus/hearth-home-server-helper".into(), "app-fix".into(), app]), Msg::Did);
        }
        Msg::Did(o) => {
            st.busy = false;
            st.msg = if o.ok { String::new() } else { o.why() };
            return load();
        }
        Msg::Go(id) => sh.go(id),
        Msg::Open(cmd) => {
            let v: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
            run::detached(&v);
        }
        Msg::Copy(s) => return iced::clipboard::write(s),
        Msg::Shared(o) => {
            st.sh_busy = false;
            st.shared = json_of(&o);
        }
        Msg::RefreshShared => {
            st.sh_busy = true;
            return Task::perform(run::spawn(py("hearth-ai-check", &["shared"])), Msg::Shared);
        }
        Msg::UseShared(dev) => {
            st.busy = true;
            return Task::perform(run::spawn(py("hearth-ai-check", &["use-shared", &dev])), Msg::UsedShared);
        }
        Msg::UsedShared(o) => {
            st.busy = false;
            let v = json_of(&o);
            st.msg = if v["ok"] == Value::Bool(true) { String::new() } else if !s(&v, "error").is_empty() { s(&v, "error") } else { o.why() };
            return load();
        }
        Msg::DescribeFrom(k) => {
            st.busy = true;
            return Task::perform(run::spawn(py("hearth-ai-check", &["use", "describe", &serde_json::json!({ "from": k }).to_string()])), Msg::Did);
        }
    }
    Task::none()
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn back(st: &mut State) -> bool {
    if st.picking.is_some() {
        st.picking = None;
        return true;
    }
    false
}

fn s(v: &Value, k: &str) -> String {
    let mut cur = v;
    for part in k.split('.') {
        cur = &cur[part];
    }
    match cur {
        Value::String(x) => x.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn b(v: &Value, k: &str) -> bool {
    let mut cur = v;
    for part in k.split('.') {
        cur = &cur[part];
    }
    cur.as_bool().unwrap_or(false)
}

fn arr<'a>(v: &'a Value, k: &str) -> Vec<&'a Value> {
    v[k].as_array().map(|a| a.iter().collect()).unwrap_or_default()
}

/// Where Hearth's AI comes from, first that answers (hearth-ai-check status "order").
fn order_of(st: &State) -> Vec<String> {
    let mut o: Vec<String> = arr(&st.st, "order").iter().filter_map(|x| x.as_str()).filter(|x| ["here", "link", "online"].contains(x)).map(str::to_string).collect();
    for k in ["here", "link", "online"] {
        if !o.iter().any(|x| x == k) {
            o.push(k.into());
        }
    }
    o
}

fn answering_model(st: &State) -> String {
    s(&st.st, "answering.model")
}

fn gb(bytes: f64) -> String {
    format!("{:.1} GB", bytes / 1e9)
}

/// "4 min ago", "yesterday": when a device last asked.
fn ago(now: u64, then: u64) -> String {
    if then == 0 || now < then {
        return String::new();
    }
    let d = now - then;
    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", d / 60),
        3600..=86399 => format!("{} h ago", d / 3600),
        86400..=172799 => "yesterday".into(),
        _ => format!("{} days ago", d / 86400),
    }
}

/// The engine on this PC that Hearth's AI uses, when there is one (Ollama or Colibri, answering or on).
fn here_engine(st: &State) -> bool {
    arr(&st.st, "engines").iter().any(|e| b(e, "on") || b(e, "answering"))
}

fn top<'a>(p: &Pal, st: &'a State, host: &str) -> Column<'a, Msg> {
    let mut c = Column::new().spacing(18.0);
    // When something's wrong (ai-3): what happened in plain words, and one button that fixes it.
    for pr in arr(&st.st, "problems") {
        let kind = s(pr, "kind");
        if kind == "none" {
            continue;
        }
        let what = s(pr, "what");
        let why = s(pr, "why");
        let fix = s(pr, "fix");
        let id = s(pr, "id");
        let mut btns = vec![btn(p, "Show what happened", BtnKind::Normal, false, Some(Msg::Open(vec!["hearth-tools".into(), "--app".into(), "logs".into()])))];
        let fix_msg = match fix.as_str() {
            "app-fix" | "restart" => Some(Msg::Fix(id.trim_start_matches("hearth-").to_string())),
            "models" => Some(Msg::Go("hs-ai")),
            _ => None,
        };
        if let Some(m) = fix_msg {
            btns.push(btn(p, "Fix it", BtnKind::Primary, false, if st.busy { None } else { Some(m) }));
        }
        c = c.push(tinted(p, p.danger, sec_dot(p.danger), what, why, btns));
    }
    let answering = !st.st["answering"].is_null();
    if !answering && b(&st.hs, "pictures.on") {
        c = c.push(tinted(p, p.warn, sec_dot(p.warn), "AI photos is waiting for your AI".into(), "It starts by itself once an AI answers. Nothing to do.".into(), vec![sec_chip(p, "waiting", false)]));
    }
    // The hero: which AI answers.
    let model = answering_model(st);
    let from = s(&st.st, "answering.from");
    let shared_with = arr(&st.link, "clients").iter().filter(|x| !b(x, "stopped")).count();
    let tested = match &st.test {
        Some(v) if b(v, "ok") => format!("answered a test in {} s", s(v, "seconds")),
        Some(v) => s(v, "error"),
        None => String::new(),
    };
    let test_btn = btn(p, if st.testing { "Testing…" } else { "Test it" }, BtnKind::Normal, false, if st.testing { None } else { Some(Msg::Test) });
    if answering && from == "link" {
        let pc = arr(&st.link, "servers").first().map(|x| s(x, "name")).unwrap_or_else(|| "your PC".into());
        let line = [Some("Over Hearth Link".to_string()), Some(model.clone()), (!tested.is_empty()).then(|| tested.clone())].into_iter().flatten().collect::<Vec<_>>().join(" · ");
        c = c.push(sec_banner(p, sec_dot(p.good), format!("Using {pc}'s AI"), line, vec![test_btn, btn(p, "Use another", BtnKind::Normal, false, Some(Msg::Go("link")))]));
    } else if answering {
        let where_ = if from == "engine" { format!("{} on this PC", s(&st.st, "answering.label")) } else { s(&st.st, "answering.label") };
        let mut bits = vec![where_, model.clone()];
        if !tested.is_empty() {
            bits.push(tested.clone());
        }
        if shared_with > 0 {
            bits.push(format!("shared with {shared_with} of your devices"));
        }
        let change = btn(p, "Change model", BtnKind::Primary, false, if from == "engine" { Some(Msg::Pick(true)) } else { None });
        c = c.push(sec_banner(p, sec_dot(p.good), "Your AI is answering".into(), bits.join(" · "), vec![test_btn, change]));
    } else {
        c = c.push(sec_banner(p, sec_dot(p.danger), "No AI is answering".into(), "Describe it, Ithil and AI photos need one. Pick whichever is easiest below.".into(), vec![test_btn]));
    }
    let _ = host;
    c
}

/// A coloured banner (red: couldn't start; yellow: waiting).
fn tinted<'a>(p: &Pal, col: Color, lead: Element<'a, Msg>, title: String, line: String, buttons: Vec<Element<'a, Msg>>) -> Element<'a, Msg> {
    let mut r = Row::new().spacing(18.0).align_y(iced::Center).push(lead).push(column![tb(title, 20.0, p.text), t(line, 14.0, p.muted)].spacing(4.0).width(Length::Fill));
    for x in buttons {
        r = r.push(x);
    }
    container(r).padding(Padding { top: 18.0, right: 24.0, bottom: 18.0, left: 20.0 }).width(Length::Fill).style(ui::boxed(Pal::mix(p.panel, col, 0.12), p.r(14.0), Some((Pal::a(col, 0.45), 1.0)))).into()
}

fn engines_card<'a>(p: &Pal, st: &'a State, gpu: &str) -> Element<'a, Msg> {
    let mut rows: Vec<Element<'a, Msg>> = Vec::new();
    let models = arr(&st.models, "models");
    let total: f64 = models.iter().map(|m| m["size"].as_f64().unwrap_or(0.0)).sum();
    for e in arr(&st.st, "engines") {
        let id = s(e, "id");
        let label = s(e, "label");
        let ans = b(e, "answering");
        let on = b(e, "on") || ans;
        if id == "ollama" {
            let mut line = vec![format!("{} models", if ans { arr(e, "models").len() } else { models.len() })];
            if total > 0.0 {
                line.push(gb(total));
            }
            line.push("starts a minute after the PC does".into());
            let mut name = Row::new().spacing(8.0).align_y(iced::Center).push(tb(label.clone(), 15.0, p.text));
            name = name.push(sec_chip(p, if ans { "running" } else if on { "starting" } else { "off" }, ans));
            if b(e, "default") {
                name = name.push(container(t("Hearth's AI uses this", 12.0, p.accent)).padding(Padding { top: 2.0, right: 10.0, bottom: 2.0, left: 10.0 }).style(ui::boxed(Pal::a(p.accent, 0.12), 12.0, Some((Pal::a(p.accent, 0.5), 1.0)))));
            }
            let sw = switch(p, on, if st.busy { None } else { Some(Msg::Ollama(!on)) });
            rows.push(engine_row(p, "O", name.into(), line.join(" · "), sw));
        } else {
            let name = row![tb(label.clone(), 15.0, p.text), sec_chip(p, if ans { "running" } else { "off" }, ans)].spacing(8.0).align_y(iced::Center);
            let line = if s(e, "writing").is_empty() && !ans { "Faster on big models · pick a model to turn it on".to_string() } else { s(e, "writing") };
            rows.push(engine_row(p, "C", name.into(), line, btn(p, "Pick a model", BtnKind::Normal, false, Some(Msg::Go("hs-ai")))));
        }
    }
    let names: Vec<String> = models.iter().map(|m| if m["writes"].as_bool() == Some(false) { format!("{} (search only)", s(m, "name")) } else { s(m, "name") }).collect();
    rows.push(vrow(p, "Models", &if names.is_empty() { "None yet: get one in Manage".to_string() } else { names.join(" · ") }, btn(p, "Manage", BtnKind::Normal, false, Some(Msg::Go("hs-ai"))), None, true));
    let vram = st.models["vram"].as_f64().unwrap_or(0.0);
    let head_right = if vram > 0.0 { format!("{gpu} · {:.0} GB", vram / 1073741824.0) } else { gpu.to_string() };
    titled(p, "Engines on this PC", &head_right, rows)
}

fn engine_row<'a>(p: &Pal, letter: &str, name: Element<'a, Msg>, line: String, right: Element<'a, Msg>) -> Element<'a, Msg> {
    let badge = container(tb(letter.to_string(), 16.0, p.accent)).center_x(Length::Fixed(44.0)).center_y(Length::Fixed(44.0)).style(ui::boxed(Pal::mix(p.panel, p.accent, 0.1), p.r(10.0), None));
    let r = row![badge, column![name, t(line, 13.0, p.muted)].spacing(3.0).width(Length::Fill), right].spacing(16.0).align_y(iced::Center);
    column![container(r).padding(Padding { top: 12.0, right: 6.0, bottom: 12.0, left: 6.0 }), ui::hline(Pal::a(p.text, 0.07))].into()
}

/// A card with its heading on the left and a note on the right (ENGINES ON THIS PC · RTX 5070 · 12 GB).
fn titled<'a>(p: &Pal, head: &str, right: &str, rows: Vec<Element<'a, Msg>>) -> Element<'a, Msg> {
    crate::secs::heading(head);
    let mut c = Column::new().push(container(row![tb(head.to_uppercase(), 12.5, p.muted), Space::new().width(Length::Fill), tb(right.to_uppercase(), 12.5, p.muted)]).padding(Padding { top: 0.0, right: 6.0, bottom: 8.0, left: 6.0 }));
    for r in rows {
        c = c.push(r);
    }
    container(c).padding(Padding { top: 22.0, right: 18.0, bottom: 14.0, left: 18.0 }).width(Length::Fill).style(ui::boxed(p.panel, p.r(14.0), Some((Pal::a(p.text, 0.06), 1.0)))).into()
}

fn sharing_card<'a>(p: &Pal, st: &'a State) -> Element<'a, Msg> {
    let mut rows: Vec<Element<'a, Msg>> = Vec::new();
    let on = b(&st.link, "serve");
    rows.push(vrow(p, "Share this PC's AI with my devices", "Over Hearth Link, encrypted · nothing opened on your network", switch(p, on, if st.busy { None } else { Some(Msg::Sharing(!on)) }), None, false));
    let now = st.link["now"].as_u64().unwrap_or(0);
    for x in arr(&st.link, "clients") {
        let stopped = b(x, "stopped");
        let id = s(x, "id");
        let model = s(x, "model");
        let when = ago(now, x["last"].as_u64().unwrap_or(0));
        let line = [if model.is_empty() { None } else { Some(model) }, if when.is_empty() { None } else { Some(format!("last asked {when}")) }].into_iter().flatten().collect::<Vec<_>>().join(" · ");
        let name = s(x, "name");
        let title_el = row![tb(name.clone(), 15.0, p.text), sec_chip(p, if stopped { "stopped" } else { "using it" }, !stopped)].spacing(8.0).align_y(iced::Center);
        let r = row![column![title_el, t(line, 13.0, p.muted)].spacing(3.0).width(Length::Fill), btn(p, if stopped { "Let it again" } else { "Stop" }, BtnKind::Normal, false, Some(Msg::Stop(id, !stopped)))].spacing(14.0).align_y(iced::Center);
        rows.push(column![container(r).padding(Padding { top: 12.0, right: 6.0, bottom: 12.0, left: 6.0 }), ui::hline(Pal::a(p.text, 0.07))].into());
    }
    let cc = s(&st.share, "commandCenter");
    let addr = if cc.is_empty() { s(&st.share, "local") } else { format!("{cc}/v1/hearth/ai") };
    rows.push(vrow(p, "Hearth AI Command Center", &addr, btn(p, "Copy", BtnKind::Normal, false, Some(Msg::Copy(addr.clone()))), None, true));
    titled(p, "Sharing", "", rows)
}

/// Where a laptop's AI comes from (ai-2): your PC over Hearth Link, this laptop, another device; the first that answers.
fn sources_card<'a>(p: &Pal, st: &'a State) -> Element<'a, Msg> {
    let mut rows: Vec<Element<'a, Msg>> = Vec::new();
    let mut n = 1;
    let using = s(&st.link, "using.server");
    for kind in order_of(st) {
        let mut g = Column::new();
        match kind.as_str() {
            "link" => {
                let start = n;
                let servers = arr(&st.link, "servers");
                for sv in &servers {
                    let id = s(sv, "id");
                    let models = arr(sv, "models").len();
                    let name = row![tb(format!("{} (your PC)", s(sv, "name")), 15.0, p.text), sec_chip(p, "answering", true)].spacing(8.0).align_y(iced::Center);
                    let right: Element<'a, Msg> = if id == using { sec_chip(p, "using", false) } else { Space::new().into() };
                    g = g.push(numbered(p, n, name.into(), format!("{models} models · paired in Hearth Link"), right));
                    n += 1;
                }
                // your other devices that don't share their AI
                for pe in arr(&st.peers, "peers") {
                    if s(pe, "relation") != "mine" && !s(pe, "relation").is_empty() {
                        continue;
                    }
                    let id = s(pe, "id");
                    if servers.iter().any(|x| s(x, "id") == id) {
                        continue;
                    }
                    let nm = s(pe, "name");
                    let name = row![tb(nm.clone(), 15.0, p.text), sec_chip(p, "not sharing", false)].spacing(8.0).align_y(iced::Center);
                    g = g.push(numbered(p, n, name.into(), format!("Turn on sharing on the {nm} to use it here"), Space::new().into()));
                    n += 1;
                }
                if n == start {
                    g = g.push(numbered(p, n, tb("Your devices (Hearth Link)".to_string(), 15.0, p.text).into(), "None shares its AI with this device yet".into(), Space::new().into()));
                    n += 1;
                }
            }
            "here" => {
                let here = arr(&st.st, "engines").iter().any(|e| b(e, "answering"));
                let name = row![tb("This device".to_string(), 15.0, p.text), sec_chip(p, if here { "answering" } else { "no engine" }, here)].spacing(8.0).align_y(iced::Center);
                g = g.push(numbered(p, n, name.into(), if here { "An engine on this device".into() } else { "Ollama isn't installed · small models only on this device".into() }, if here { Space::new().into() } else { btn(p, "Install Ollama", BtnKind::Normal, false, Some(Msg::Go("hs-ai"))) }));
                n += 1;
            }
            _ => {
                let on = &st.st["online"];
                let (label, line, ok) = if s(on, "provider").is_empty() {
                    ("An online AI".to_string(), "With your own key (Settings › Home server › AI chat)".to_string(), false)
                } else {
                    (format!("{} (online)", s(on, "label")), if b(on, "key") { format!("{} · your key", s(on, "model")) } else { "Its key isn't in the password store".into() }, b(on, "key"))
                };
                let name = row![tb(label, 15.0, p.text), sec_chip(p, if ok { "ready" } else { "off" }, ok)].spacing(8.0).align_y(iced::Center);
                g = g.push(numbered(p, n, name.into(), line, Space::new().into()));
                n += 1;
            }
        }
        // drag it by its handle onto another to change the order (saved: hearth-ai-check order)
        let lit = st.over.as_deref() == Some(kind.as_str()) && st.drag.is_some();
        let handle = mouse_area(container(t("⠿", 18.0, p.muted)).padding(Padding { top: 0.0, right: 4.0, bottom: 0.0, left: 0.0 })).on_press(Msg::DragStart(kind.clone())).interaction(mouse::Interaction::Grab);
        let body = container(row![handle, g.width(Length::Fill)].spacing(6.0).align_y(iced::Center)).style(ui::boxed(if lit { Pal::mix(p.panel, p.accent, 0.08) } else { Color::TRANSPARENT }, p.r(10.0), None));
        rows.push(mouse_area(body).on_enter(Msg::DragOver(kind.clone())).on_release(Msg::DragEnd).into());
    }
    rows.push(container(t("If your PC is asleep or away, Hearth tries the next one, and says which answered. Drag to change the order.", 13.0, p.muted)).padding(Padding { top: 10.0, right: 6.0, bottom: 0.0, left: 6.0 }).into());
    titled(p, "Where your AI comes from", "First that answers", rows)
}

fn numbered<'a>(p: &Pal, n: usize, name: Element<'a, Msg>, line: String, right: Element<'a, Msg>) -> Element<'a, Msg> {
    let badge = container(tb(n.to_string(), 16.0, p.accent)).center_x(Length::Fixed(44.0)).center_y(Length::Fixed(44.0)).style(ui::boxed(Pal::mix(p.panel, p.accent, 0.08), p.r(10.0), None));
    let r = row![badge, column![name, t(line, 13.0, p.muted)].spacing(3.0).width(Length::Fill), right].spacing(16.0).align_y(iced::Center);
    column![container(r).padding(Padding { top: 12.0, right: 6.0, bottom: 12.0, left: 6.0 }), ui::hline(Pal::a(p.text, 0.07))].into()
}

fn test_card<'a>(p: &Pal, st: &'a State) -> Element<'a, Msg> {
    let mut rows: Vec<Element<'a, Msg>> = Vec::new();
    match &st.dtest {
        Some(v) if b(v, "ok") => {
            rows.push(vrow(p, &format!("\"{TEST_ASK}\""), &format!("Describe it, sent to {}", s(v, "engine")), Space::new().into(), None, false));
            let blocks = v["screen"]["blocks"].as_array().map(|a| a.len()).unwrap_or(0);
            let r = row![
                sec_chip(p, "ok", true),
                column![tb(format!("A whole desktop came back in {:.1} s", st.dsecs), 15.0, p.text), t(format!("{blocks} blocks · checked by Hearth · nothing changed on your desktop"), 13.0, p.muted)].spacing(3.0).width(Length::Fill),
                btn(p, "Show it", BtnKind::Normal, false, Some(Msg::ShowIt))
            ]
            .spacing(14.0)
            .align_y(iced::Center);
            rows.push(column![container(r).padding(Padding { top: 12.0, right: 6.0, bottom: 12.0, left: 6.0 }), ui::hline(Pal::a(p.text, 0.07))].into());
        }
        Some(v) => rows.push(vrow(p, "It didn't come back", &s(v, "error"), btn(p, "Test it", BtnKind::Normal, false, Some(Msg::Test)), None, false)),
        None => rows.push(vrow(p, if st.testing { "Testing…" } else { "Not tested yet" }, &format!("Test it asks your AI \"{TEST_ASK}\", as Describe it does, and checks what comes back"), btn(p, "Test it", BtnKind::Normal, false, if st.testing { None } else { Some(Msg::Test) }), None, false)),
    }
    rows.push(vrow(p, "Thinking models", "Qwen3 is asked to answer straight away; its thinking is skipped", sec_chip(p, "on", true), None, true));
    titled(p, "Test it", "", rows)
}

/// What uses your AI (ai-1, ai-2): Describe it, Ithil, AI photos, Sort my Downloads, each with its own AI picker
/// (Hearth's AI, or any AI that answers: this device's engines, your devices' over Hearth Link, an online AI with your
/// key; saved in ai-uses.json by hearth-ai-check use) and whether it's ready.
fn uses_card<'a>(p: &Pal, st: &'a State, laptop: bool) -> Element<'a, Msg> {
    let which = if laptop { format!("{}'s AI", arr(&st.link, "servers").first().map(|x| s(x, "name")).unwrap_or_else(|| "Your PC".into())) } else { "Hearth's AI".to_string() };
    let choices = arr(&st.st, "choices");
    let mut r = Row::new().spacing(14.0);
    for (name, line, id) in USES {
        let u = &st.st["uses"][id];
        let own = b(u, "own");
        let ai = if own { s(u, "label") } else { which.clone() };
        let ready = if u.is_object() { b(u, "ready") } else { !st.st["answering"].is_null() };
        let picker = container(t(format!("{ai} ▾"), 14.0, p.text)).padding(Padding { top: 8.0, right: 14.0, bottom: 8.0, left: 14.0 }).style(ui::boxed(Pal::mix(p.panel, p.text, 0.05), p.r(8.0), Some((Pal::a(p.text, 0.1), 1.0))));
        let picker = mouse_area(picker).on_press(Msg::Menu(Some(id))).interaction(mouse::Interaction::Pointer);
        let mut col = Column::new().spacing(4.0).push(tb(name.to_string(), 15.0, p.text));
        if !laptop {
            col = col.push(t(line.to_string(), 13.0, p.muted));
        }
        col = col.push(Space::new().height(Length::Fixed(8.0))).push(row![picker, sec_chip(p, if ready { "ready" } else { "waiting" }, ready)].spacing(10.0).align_y(iced::Center));
        if st.menu == Some(id) {
            // the picker's list: Hearth's AI first, then each AI that answers now
            let mut list = Column::new().spacing(2.0);
            let item = |label: String, on: bool, m: Msg| -> Element<'a, Msg> {
                let c = container(t(format!("{}{label}", if on { "✓ " } else { "" }), 14.0, if on { p.accent } else { p.text })).padding(Padding { top: 6.0, right: 10.0, bottom: 6.0, left: 10.0 }).width(Length::Fill);
                mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer).into()
            };
            list = list.push(item(which.clone(), !own, Msg::UseFor(id, "hearth".into())));
            for c in &choices {
                let label = if s(c, "from") == "online" { s(c, "label") } else { format!("{} · {}", s(c, "label"), s(c, "model")) };
                let pick = if s(c, "from") == "online" { serde_json::json!({ "online": s(c, "provider") }) } else { serde_json::json!({ "url": s(c, "base"), "model": s(c, "model"), "label": s(c, "label") }) };
                let on = own && s(u, "url").trim_end_matches('/') == s(c, "base").trim_end_matches('/') && (s(c, "from") == "online") == !s(u, "online").is_empty();
                list = list.push(item(label, on, Msg::UseFor(id, pick.to_string())));
            }
            if choices.is_empty() {
                list = list.push(t("No other AI answers right now", 13.0, p.muted));
            }
            col = col.push(container(list).padding(6.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(8.0), Some((Pal::a(p.text, 0.12), 1.0)))));
        }
        r = r.push(container(col).padding(18.0).width(Length::FillPortion(1)).style(ui::boxed(Pal::mix(p.panel, p.text, 0.02), p.r(12.0), Some((Pal::a(p.text, 0.07), 1.0)))));
    }
    let mut c = Column::new().spacing(12.0).push(container(row![tb("WHAT USES YOUR AI", 12.5, p.muted), Space::new().width(Length::Fill), tb(if laptop { "" } else { "EACH ONE CAN HAVE ITS OWN" }, 12.5, p.muted)]).padding(Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 6.0 })).push(r);
    if !laptop {
        c = c.push(t("\"Hearth's AI\" is the one at the top of this page. Pick another for one thing only and the rest stay as they are. Nothing here changes by itself.", 13.0, p.muted));
    }
    container(c).padding(22.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(14.0), Some((Pal::a(p.text, 0.06), 1.0)))).into()
}

/// Describe it uses… (the owner, Nov 24): where Describe it asks first, and which AI that is now.
fn describe_card<'a>(p: &Pal, st: &'a State) -> Element<'a, Msg> {
    let u = &st.st["uses"]["describe"];
    let from = if s(u, "from").is_empty() { if b(u, "own") { String::new() } else { "fastest".into() } } else { s(u, "from") };
    let items: Vec<(String, bool, Option<Msg>)> = DESCRIBE_FROM.iter().map(|(k, label)| (label.to_string(), from == *k, if st.busy { None } else { Some(Msg::DescribeFrom(*k)) })).collect();
    let now = if st.st.is_null() {
        "Checking…".to_string()
    } else if b(u, "ready") {
        let via = if s(u, "via").is_empty() { s(u, "label") } else { s(u, "via") };
        let mut x = format!("Now: {via}");
        if !s(u, "model").is_empty() {
            x.push_str(&format!(" · {}", s(u, "model")));
        }
        if b(u, "slow") {
            x.push_str(&match u["words"].as_f64() {
                Some(w) => format!(" · slow on this device (about {} words a second)", w.round() as i64),
                None => " · slow on this device".into(),
            });
        }
        x
    } else {
        "Nothing answers there right now: the fastest that answers is asked instead".into()
    };
    let r = row![
        column![tb("Where Describe it asks first".to_string(), 15.0, p.text), t(now, 13.0, p.muted)].spacing(3.0).width(Length::Fill),
        seg(p, items)
    ]
    .spacing(16.0)
    .align_y(iced::Center);
    let rows: Vec<Element<'a, Msg>> = vec![
        r.into(),
        Space::new().height(Length::Fixed(8.0)).into(),
        t("Fastest available: your other devices over Hearth Link before a slow engine on this device (under 8 words a second, or no graphics card). Nothing here changes by itself.", 13.0, p.muted).into(),
    ];
    titled(p, "Describe it uses", "", rows)
}

/// Shared from your other devices (the owner, Nov 24): every AI your other Hearth devices share, read only, each with
/// "Use this for Describe it". The words are Settings › Home server › AI devices' own (hs::services).
fn shared_card<'a>(p: &Pal, st: &'a State) -> Element<'a, Msg> {
    use super::hs::services::{gpu_word, reason_word};
    let mut rows: Vec<Element<'a, Msg>> = Vec::new();
    let devs = arr(&st.shared, "devices");
    if st.shared.is_null() {
        rows.push(t("Looking for your devices…", 13.0, p.muted).into());
    } else if devs.is_empty() {
        rows.push(t("None of your other devices shares its AI yet. Turn on Share this device's AI on it (Settings › Home server › AI devices), or pair it in Hearth Link.", 13.0, p.muted).into());
    }
    for d in devs {
        let name = s(d, "name");
        let state = s(d, "state");
        let dev = s(d, "device");
        let (word, good) = match state.as_str() {
            "ready" => ("Ready".to_string(), true),
            "playing" => ("Playing".to_string(), false),
            "busy" => ("Busy".to_string(), false),
            "unconfirmed" => ("Can't confirm".to_string(), false),
            "offline" => ("Offline".to_string(), false),
            other => (reason_word(other).to_string(), false),
        };
        let mut title = Row::new().spacing(8.0).align_y(iced::Center).push(tb(name.clone(), 15.0, p.text)).push(sec_chip(p, if word.is_empty() { "Not sharing" } else { word.as_str() }, good));
        if b(d, "inUse") {
            title = title.push(container(t("Describe it uses this", 12.0, p.accent)).padding(Padding { top: 2.0, right: 10.0, bottom: 2.0, left: 10.0 }).style(ui::boxed(Pal::a(p.accent, 0.12), 12.0, Some((Pal::a(p.accent, 0.5), 1.0)))));
        }
        let (line, right): (Element<'a, Msg>, Element<'a, Msg>) = if state == "unconfirmed" {
            (t(format!("Can't confirm this is your {name} — pair it again"), 13.0, p.danger).into(), btn(p, "Pair again", BtnKind::Normal, false, Some(Msg::Go("link"))))
        } else {
            let mut bits: Vec<String> = Vec::new();
            let models: Vec<String> = arr(d, "models").iter().filter_map(|m| m.as_str().map(str::to_string)).collect();
            if !s(d, "engine").is_empty() {
                bits.push(match s(d, "engine").as_str() { "ollama" => "Ollama".to_string(), "colibri" => "Colibri".to_string(), e => e.to_string() });
            }
            if !models.is_empty() {
                bits.push(format!("{}{}", models.iter().take(2).cloned().collect::<Vec<_>>().join(", "), if models.len() > 2 { format!(" +{}", models.len() - 2) } else { String::new() }));
            }
            if !s(d, "gpu").is_empty() {
                let vram = d["vramMB"].as_f64().unwrap_or(0.0);
                bits.push(if vram > 0.0 { format!("{} · {:.0} GB", gpu_word(&s(d, "gpu")), vram / 1024.0) } else { gpu_word(&s(d, "gpu")).to_string() });
            }
            if let Some(w) = d["words"].as_f64() {
                bits.push(format!("about {} words a second", w.round() as i64));
            }
            bits.push(match s(d, "via").as_str() { "tailscale" => "through Tailscale".to_string(), "hearth link" => "over Hearth Link".to_string(), _ => String::new() });
            bits.retain(|x| !x.is_empty());
            let right: Element<'a, Msg> = if b(d, "canUse") && !b(d, "inUse") { btn(p, "Use this for Describe it", BtnKind::Normal, false, if st.busy { None } else { Some(Msg::UseShared(dev.clone())) }) } else { Space::new().into() };
            (t(bits.join(" · "), 13.0, p.muted).into(), right)
        };
        let r = row![column![title, line].spacing(3.0).width(Length::Fill), right].spacing(14.0).align_y(iced::Center);
        rows.push(column![container(r).padding(Padding { top: 12.0, right: 6.0, bottom: 12.0, left: 6.0 }), ui::hline(Pal::a(p.text, 0.07))].into());
    }
    rows.push(container(row![t("Looked up when this page opens; nothing is asked in the background.", 13.0, p.muted).width(Length::Fill), btn(p, if st.sh_busy { "Looking…" } else { "Refresh" }, BtnKind::Normal, false, if st.sh_busy { None } else { Some(Msg::RefreshShared) })].spacing(12.0).align_y(iced::Center)).padding(Padding { top: 10.0, right: 6.0, bottom: 0.0, left: 6.0 }).into());
    titled(p, "Shared from your other devices", "", rows)
}

/// If no AI answers (ai-3): use your PC's, turn on Ollama here, or another address.
fn none_card<'a>(p: &Pal, st: &'a State) -> Element<'a, Msg> {
    let pc = arr(&st.link, "servers").first().map(|x| (s(x, "name"), arr(x, "models").len()));
    let opt = |title: &str, line: String, label: &'static str, m: Msg, primary: bool| -> Element<'a, Msg> {
        container(column![tb(title.to_string(), 15.0, p.text), t(line, 13.0, p.muted), Space::new().height(Length::Fixed(6.0)), btn(p, label, if primary { BtnKind::Primary } else { BtnKind::Normal }, false, Some(m))].spacing(4.0))
            .padding(16.0)
            .width(Length::FillPortion(1))
            .style(ui::boxed(Pal::mix(p.panel, p.text, 0.02), p.r(12.0), Some((Pal::a(p.text, 0.07), 1.0))))
            .into()
    };
    let mut r = Row::new().spacing(12.0);
    if let Some((name, n)) = pc.clone() {
        r = r.push(opt("Use your PC's AI", format!("{name} is paired and has {n} models."), "Use it", Msg::Go("link"), true));
    }
    r = r.push(opt("Turn on Ollama here", "About 2 GB, then a model. Slower on this device.".into(), "Install", Msg::Go("hs-ai"), pc.is_none()));
    r = r.push(opt("Another address", "Any OpenAI-style AI on your network.".into(), "Add", Msg::Go("desktop-layout"), false));
    titled(p, "If no AI answers", "Other ways to get one", vec![t("Describe it, Ithil and AI photos need one. Pick whichever is easiest:", 14.0, p.muted).into(), Space::new().height(Length::Fixed(10.0)).into(), r.into()])
}

/// What Hearth checked just now (ai-3): each engine and device with its state, the model for writing, search-only ones.
fn checked_card<'a>(p: &Pal, st: &'a State) -> Element<'a, Msg> {
    let mut rows: Vec<Element<'a, Msg>> = Vec::new();
    let problems = arr(&st.st, "problems");
    for e in arr(&st.st, "engines") {
        let id = s(e, "id");
        let pr = problems.iter().find(|x| s(x, "id").trim_start_matches("hearth-") == id);
        let (label, good) = if b(e, "answering") { ("answering".to_string(), true) } else if let Some(x) = pr { (format!("couldn't start{}", if s(x, "kind") == "selinux" { ": SELinux" } else { "" }), false) } else { ("off".to_string(), false) };
        rows.push(vrow(p, &format!("{} on this PC", s(e, "label")), "", sec_chip(p, &label, good), None, false));
    }
    if b(&st.st, "link.answering") {
        let pc = arr(&st.link, "servers").first().map(|x| s(x, "name")).unwrap_or_else(|| "Your PC".into());
        rows.push(vrow(p, &format!("{pc} over Hearth Link"), "", sec_chip(p, "answering", true), None, false));
    }
    let m = answering_model(st);
    if !m.is_empty() {
        rows.push(vrow(p, "Model for writing", "", sec_chip(p, &m, true), None, false));
    }
    for x in arr(&st.models, "models") {
        if x["writes"].as_bool() == Some(false) {
            rows.push(vrow(p, &s(x, "name"), "", sec_chip(p, "search only, not for writing", false), None, false));
        }
    }
    rows.push(t("Send logs adds this list, so a problem can be found without terminal commands.", 13.0, p.muted).into());
    titled(p, "What Hearth checked", "Just now", rows)
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    if page != "sec-ai" {
        return None;
    }
    let p = a.sh.p;
    let st = &a.ai;
    // its parts as blocks (secs.rs Plain): what's on top, then each titled card
    let mut c = crate::secs::Plain::new(0).spacing(18.0).push(top(&p, st, &a.sh.info.host));
    if !st.msg.is_empty() {
        c = c.push(note(&p, st.msg.clone(), true));
    }
    // Where Describe it asks first, near the top (the owner's laptop, Nov 24)
    c = c.push(describe_card(&p, st));
    c = c.push(shared_card(&p, st));
    let answering = !st.st["answering"].is_null();
    let has_problem = arr(&st.st, "problems").iter().any(|x| s(x, "kind") != "none");
    if !answering || has_problem {
        c = c.push(row![container(if answering { engines_card(&p, st, &a.sh.info.gpu) } else { none_card(&p, st) }).width(Length::FillPortion(1)), container(checked_card(&p, st)).width(Length::FillPortion(1))].spacing(18.0));
        for pr in arr(&st.st, "problems") {
            if s(pr, "kind") == "selinux" || s(pr, "kind") == "failed" {
                let app = s(pr, "id").trim_start_matches("hearth-").to_string();
                let name = s(pr, "what").replace(" couldn't start", "");
                c = c.push(titled(&p, "Restarting by itself", "", vec![vrow(&p, &format!("{name} tried 5 times in 10 minutes, then stopped"), "So it never keeps your PC busy retrying. Fix it, or turn it off and on, starts it again.", btn(&p, "Try again", BtnKind::Normal, false, Some(Msg::Fix(app))), None, true)]));
                break;
            }
        }
    } else if here_engine(st) {
        c = c.push(row![container(engines_card(&p, st, &a.sh.info.gpu)).width(Length::FillPortion(1)), container(sharing_card(&p, st)).width(Length::FillPortion(1))].spacing(18.0));
    } else {
        c = c.push(row![container(sources_card(&p, st)).width(Length::FillPortion(1)), container(test_card(&p, st)).width(Length::FillPortion(1))].spacing(18.0));
    }
    c = c.push(uses_card(&p, st, !here_engine(st)));
    Some(c.into())
}

/// Change model (ai-4): a sheet over the page, each model with its size, whether it fits, speed, thinks, uncensored.
pub fn dialog<'a>(a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    let st = &a.ai;
    let picked = st.picking.as_ref()?;
    if a.page != "sec-ai" {
        return None;
    }
    let p = a.sh.p;
    let mut c = Column::new().spacing(8.0).push(tb("Model for Hearth's AI", 20.0, p.text)).push(t("What Describe it, Ithil and the rest write with. Ones that can't write are greyed out.", 14.0, p.muted));
    for m in arr(&st.models, "models") {
        let name = s(m, "name");
        let writes = m["writes"].as_bool().unwrap_or(true);
        let on = *picked == name;
        let size = gb(m["size"].as_f64().unwrap_or(0.0));
        let mut bits = vec![size];
        if !writes {
            bits.push("Search only (AI photos uses it) · can't write".into());
        } else {
            if b(m, "gpu") {
                bits.push("fits all on the graphics card · fastest".into());
            } else if b(m, "ram") {
                bits.push("fits with your RAM".into());
            } else {
                bits.push("too big for this PC".into());
            }
            // measured here (hearth-ai-check speed: Ollama's own eval_duration), never guessed
            match m["speed"].as_f64() {
                Some(w) => bits.push(format!("about {} words a second", w.round() as i64)),
                None => bits.push("speed not measured yet".into()),
            }
            if b(m, "uncensored") {
                bits.push("uncensored".into());
            }
        }
        let col = if writes { p.text } else { p.muted };
        let radio = container(if on { container(Space::new()).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(ui::boxed(p.accent, 5.0, None)).into() } else { Element::from(Space::new()) }).center_x(Length::Fixed(22.0)).center_y(Length::Fixed(22.0)).style(ui::boxed(Color::TRANSPARENT, 11.0, Some((if on { p.accent } else { p.muted }, 2.0))));
        let mut title = Row::new().spacing(8.0).align_y(iced::Center).push(tb(name.clone(), 16.0, col));
        if b(m, "best") {
            title = title.push(sec_chip(&p, "best here", true));
        }
        let mut line = Row::new().spacing(10.0).align_y(iced::Center).push(radio).push(column![title, t(bits.join(" · "), 13.0, p.muted)].spacing(3.0).width(Length::Fill));
        if b(m, "thinks") {
            line = line.push(sec_chip(&p, "thinks · skipped", false));
        }
        let boxed = container(line).padding(14.0).width(Length::Fill).style(ui::boxed(if on { Pal::mix(p.panel, p.accent, 0.06) } else { Color::TRANSPARENT }, p.r(10.0), if on { Some((p.accent, 1.5)) } else { None }));
        c = c.push(if writes { Element::from(mouse_area(boxed).on_press(Msg::PickModel(name.clone())).interaction(mouse::Interaction::Pointer)) } else { boxed.into() });
    }
    if arr(&st.models, "models").is_empty() {
        c = c.push(t("Ollama has no models yet, or isn't answering. Get one first.", 14.0, p.muted));
    }
    let use_label = format!("Use {picked}");
    c = c.push(row![btn(&p, "Get another model…", BtnKind::Normal, false, Some(Msg::Go("hs-ai"))), Space::new().width(Length::Fill), btn(&p, "Cancel", BtnKind::Normal, false, Some(Msg::Pick(false))), btn(&p, use_label, BtnKind::Primary, false, if picked.is_empty() { None } else { Some(Msg::UseModel) })].spacing(12.0).align_y(iced::Center));
    let d = container(c).padding(24.0).width(Length::Fixed(720.0)).style(ui::boxed(p.panel, p.r(16.0), Some((Pal::a(p.text, 0.1), 1.0))));
    Some((d.into(), Some(Msg::Pick(false))))
}

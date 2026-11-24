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
}

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
}

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
            return Task::perform(run::spawn(py("hearth-ai-check", &["test"])), Msg::Tested);
        }
        Msg::Tested(o) => {
            st.testing = false;
            st.test = Some(json_of(&o));
        }
        Msg::Pick(open) => {
            st.picking = if open { Some(answering_model(st)) } else { None };
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
        c = c.push(tinted(p, p.danger, dot(p.danger), what, why, btns));
    }
    let answering = !st.st["answering"].is_null();
    if !answering && b(&st.hs, "pictures.on") {
        c = c.push(tinted(p, p.warn, dot(p.warn), "AI photos is waiting for your AI".into(), "It starts by itself once an AI answers. Nothing to do.".into(), vec![chip(p, "waiting", false)]));
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
        c = c.push(banner(p, dot(p.good), format!("Using {pc}'s AI"), line, vec![test_btn, btn(p, "Use another", BtnKind::Normal, false, Some(Msg::Go("link")))]));
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
        c = c.push(banner(p, dot(p.good), "Your AI is answering".into(), bits.join(" · "), vec![test_btn, change]));
    } else {
        c = c.push(banner(p, dot(p.danger), "No AI is answering".into(), "Describe it, Ithil and AI photos need one. Pick whichever is easiest below.".into(), vec![test_btn]));
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
            name = name.push(chip(p, if ans { "running" } else if on { "starting" } else { "off" }, ans));
            if b(e, "default") {
                name = name.push(container(t("Hearth's AI uses this", 12.0, p.accent)).padding(Padding { top: 2.0, right: 10.0, bottom: 2.0, left: 10.0 }).style(ui::boxed(Pal::a(p.accent, 0.12), 12.0, Some((Pal::a(p.accent, 0.5), 1.0)))));
            }
            let sw = switch(p, on, if st.busy { None } else { Some(Msg::Ollama(!on)) });
            rows.push(engine_row(p, "O", name.into(), line.join(" · "), sw));
        } else {
            let name = row![tb(label.clone(), 15.0, p.text), chip(p, if ans { "running" } else { "off" }, ans)].spacing(8.0).align_y(iced::Center);
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
        let title_el = row![tb(name.clone(), 15.0, p.text), chip(p, if stopped { "stopped" } else { "using it" }, !stopped)].spacing(8.0).align_y(iced::Center);
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
    for sv in arr(&st.link, "servers") {
        let id = s(sv, "id");
        let models = arr(sv, "models").len();
        let name = row![tb(format!("{} (your PC)", s(sv, "name")), 15.0, p.text), chip(p, "answering", true)].spacing(8.0).align_y(iced::Center);
        let right: Element<'a, Msg> = if id == using { chip(p, "using", false) } else { Space::new().into() };
        rows.push(numbered(p, n, name.into(), format!("{models} models · paired in Hearth Link"), right));
        n += 1;
    }
    let here = arr(&st.st, "engines").iter().any(|e| b(e, "answering"));
    let name = row![tb("This device".to_string(), 15.0, p.text), chip(p, if here { "answering" } else { "no engine" }, here)].spacing(8.0).align_y(iced::Center);
    rows.push(numbered(p, n, name.into(), if here { "An engine on this device".into() } else { "Ollama isn't installed · small models only on this device".into() }, if here { Space::new().into() } else { btn(p, "Install Ollama", BtnKind::Normal, false, Some(Msg::Go("hs-ai"))) }));
    rows.push(container(t("If your PC is asleep or away, Hearth tries the next one, and says which answered.", 13.0, p.muted)).padding(Padding { top: 10.0, right: 6.0, bottom: 0.0, left: 6.0 }).into());
    titled(p, "Where your AI comes from", "First that answers", rows)
}

fn numbered<'a>(p: &Pal, n: usize, name: Element<'a, Msg>, line: String, right: Element<'a, Msg>) -> Element<'a, Msg> {
    let badge = container(tb(n.to_string(), 16.0, p.accent)).center_x(Length::Fixed(44.0)).center_y(Length::Fixed(44.0)).style(ui::boxed(Pal::mix(p.panel, p.accent, 0.08), p.r(10.0), None));
    let r = row![badge, column![name, t(line, 13.0, p.muted)].spacing(3.0).width(Length::Fill), right].spacing(16.0).align_y(iced::Center);
    column![container(r).padding(Padding { top: 12.0, right: 6.0, bottom: 12.0, left: 6.0 }), ui::hline(Pal::a(p.text, 0.07))].into()
}

fn test_card<'a>(p: &Pal, st: &'a State) -> Element<'a, Msg> {
    let mut rows: Vec<Element<'a, Msg>> = Vec::new();
    match &st.test {
        Some(v) if b(v, "ok") => {
            rows.push(vrow(p, "\"Say ok.\"", &format!("sent to {} · {}", s(v, "engine"), s(v, "model")), Space::new().into(), None, false));
            let r = row![chip(p, "ok", true), column![tb(format!("It answered in {} s", s(v, "seconds")), 15.0, p.text), t(format!("\"{}\" · checked by Hearth · nothing changed", s(v, "said")), 13.0, p.muted)].spacing(3.0).width(Length::Fill)].spacing(14.0).align_y(iced::Center);
            rows.push(column![container(r).padding(Padding { top: 12.0, right: 6.0, bottom: 12.0, left: 6.0 }), ui::hline(Pal::a(p.text, 0.07))].into());
        }
        Some(v) => rows.push(vrow(p, "It didn't answer", &s(v, "error"), Space::new().into(), None, false)),
        None => rows.push(vrow(p, "Not tested yet", "Test it asks your AI one short question and times the answer", btn(p, "Test it", BtnKind::Normal, false, Some(Msg::Test)), None, false)),
    }
    rows.push(vrow(p, "Thinking models", "Qwen3 is asked to answer straight away; its thinking is skipped", chip(p, "on", true), None, true));
    titled(p, "Test it", "", rows)
}

/// What uses your AI: Describe it, Ithil, AI photos, Sort my Downloads, each with the AI it uses and whether it's ready.
fn uses_card<'a>(p: &Pal, st: &'a State, laptop: bool) -> Element<'a, Msg> {
    let answering = !st.st["answering"].is_null();
    let which = if laptop { format!("{}'s AI", arr(&st.link, "servers").first().map(|x| s(x, "name")).unwrap_or_else(|| "Your PC".into())) } else { "Hearth's AI".to_string() };
    let items: [(&str, &str, String, &'static str); 4] = [
        ("Describe it", "Desktop, panels and blocks", which.clone(), "desktop-layout"),
        ("Ithil", "Hearth AI on this PC", which.clone(), "ai"),
        ("AI photos", "Finds people and things in Photos", format!("nomic-embed-text + {which}"), "hs-services"),
        ("Sort my Downloads", "Hearth Files", which.clone(), "ai"),
    ];
    let mut r = Row::new().spacing(14.0);
    for (name, line, ai, go) in items {
        let picker = container(t(format!("{ai} ▾"), 14.0, p.text)).padding(Padding { top: 8.0, right: 14.0, bottom: 8.0, left: 14.0 }).style(ui::boxed(Pal::mix(p.panel, p.text, 0.05), p.r(8.0), Some((Pal::a(p.text, 0.1), 1.0))));
        let picker = mouse_area(picker).on_press(Msg::Go(go)).interaction(mouse::Interaction::Pointer);
        let mut col = Column::new().spacing(4.0).push(tb(name.to_string(), 15.0, p.text));
        if !laptop {
            col = col.push(t(line.to_string(), 13.0, p.muted));
        }
        col = col.push(Space::new().height(Length::Fixed(8.0))).push(row![picker, chip(p, if answering { "ready" } else { "waiting" }, answering)].spacing(10.0).align_y(iced::Center));
        r = r.push(container(col).padding(18.0).width(Length::FillPortion(1)).style(ui::boxed(Pal::mix(p.panel, p.text, 0.02), p.r(12.0), Some((Pal::a(p.text, 0.07), 1.0)))));
    }
    let mut c = Column::new().spacing(12.0).push(container(row![tb("WHAT USES YOUR AI", 12.5, p.muted), Space::new().width(Length::Fill), tb(if laptop { "" } else { "EACH ONE CAN HAVE ITS OWN" }, 12.5, p.muted)]).padding(Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 6.0 })).push(r);
    if !laptop {
        c = c.push(t("\"Hearth's AI\" is the one at the top of this page. Pick another for one thing only and the rest stay as they are. Nothing here changes by itself.", 13.0, p.muted));
    }
    container(c).padding(22.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(14.0), Some((Pal::a(p.text, 0.06), 1.0)))).into()
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
    if let Some((name, n)) = pc {
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
        rows.push(vrow(p, &format!("{} on this PC", s(e, "label")), "", chip(p, &label, good), None, false));
    }
    if b(&st.st, "link.answering") {
        let pc = arr(&st.link, "servers").first().map(|x| s(x, "name")).unwrap_or_else(|| "Your PC".into());
        rows.push(vrow(p, &format!("{pc} over Hearth Link"), "", chip(p, "answering", true), None, false));
    }
    let m = answering_model(st);
    if !m.is_empty() {
        rows.push(vrow(p, "Model for writing", "", chip(p, &m, true), None, false));
    }
    for x in arr(&st.models, "models") {
        if x["writes"].as_bool() == Some(false) {
            rows.push(vrow(p, &s(x, "name"), "", chip(p, "search only, not for writing", false), None, false));
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
    let mut c = top(&p, st, &a.sh.info.host);
    if !st.msg.is_empty() {
        c = c.push(note(&p, st.msg.clone(), true));
    }
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
            if b(m, "uncensored") {
                bits.push("uncensored".into());
            }
        }
        let col = if writes { p.text } else { p.muted };
        let radio = container(if on { container(Space::new()).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(ui::boxed(p.accent, 5.0, None)).into() } else { Element::from(Space::new()) }).center_x(Length::Fixed(22.0)).center_y(Length::Fixed(22.0)).style(ui::boxed(Color::TRANSPARENT, 11.0, Some((if on { p.accent } else { p.muted }, 2.0))));
        let mut title = Row::new().spacing(8.0).align_y(iced::Center).push(tb(name.clone(), 16.0, col));
        if b(m, "best") {
            title = title.push(chip(&p, "best here", true));
        }
        let mut line = Row::new().spacing(10.0).align_y(iced::Center).push(radio).push(column![title, t(bits.join(" · "), 13.0, p.muted)].spacing(3.0).width(Length::Fill));
        if b(m, "thinks") {
            line = line.push(chip(&p, "thinks · skipped", false));
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

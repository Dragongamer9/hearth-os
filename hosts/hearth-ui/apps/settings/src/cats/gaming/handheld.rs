//! Gaming › Handheld (Mantel's SetHandheld.qml): Hearth's own page for Handheld Daemon (HHD, which the Fedora edition
//! runs on handhelds). Every setting HHD offers for this device (controller emulation, gyro, back buttons, power
//! limit and fan, lights, HHD's own options), drawn from HHD's settings tree, so a newer HHD's options appear by
//! themselves; advanced ones behind a switch. hearth-hhd page / apply talk to HHD's local API and check each change.
//! HHD changes things by itself too (Steam's power settings, its shortcuts), so while HHD runs and the page is open
//! it's read again every 6 s, as Mantel's is. Without HHD: says so, nothing else.

use crate::cats::apps::kit::*;
use crate::cats::apps::kit::button;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{self, t, tm, ts, G};
use hearth_ui::Pal;
use iced::widget::{column, container, row};
use iced::{Length, Padding, Task};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default)]
pub struct St {
    pub hh: Value,
    pub err: String,
    pub advanced: bool,
    pub picking: String,
    pub armed: String,
    pub pending: HashMap<String, Value>,
    pub loading: bool,
}

#[derive(Debug, Clone)]
pub enum M {
    Got(Res),
    Load,
    Tick,
    Apply(String, Value),
    Applied(Res),
    Advanced,
    Pick(String),
    Unpick,
    Arm(String),
    Num(String, f64),
    NumDone(String),
}

fn load() -> Task<M> {
    read(py("hearth-hhd", &["page"], 20), M::Got)
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.picking.clear();
    st.armed.clear();
    st.loading = true;
    load()
}

pub fn live(st: &St) -> bool {
    gb(&st.hh, "running") && st.picking.is_empty()
}

pub fn back(st: &mut St) -> bool {
    if st.picking.is_empty() {
        false
    } else {
        st.picking.clear();
        true
    }
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Got(r) => {
            st.loading = false;
            if r.ok {
                st.hh = r.v;
                st.err.clear();
            } else {
                st.err = r.err;
            }
        }
        M::Load => {
            st.hh = Value::Null;
            st.loading = true;
            return load();
        }
        M::Tick => {
            if !st.loading && st.pending.is_empty() {
                st.loading = true;
                return load();
            }
        }
        M::Apply(key, val) => {
            st.pending.insert(key.clone(), val.clone());
            sh.reread_history = true;
            return read(py("hearth-hhd", &["apply", &key, &val.to_string()], 30), M::Applied);
        }
        M::Applied(r) => {
            st.pending.clear();
            if r.ok {
                st.hh = r.v;
                st.err.clear();
            } else {
                st.err = r.err;
                return load();
            }
        }
        M::Advanced => st.advanced = !st.advanced,
        M::Pick(k) => st.picking = k,
        M::Unpick => st.picking.clear(),
        M::Arm(k) => {
            if st.armed == k {
                st.armed.clear();
                return update(sh, st, M::Apply(k, Value::Bool(true)));
            }
            st.armed = k;
        }
        M::Num(k, n) => {
            st.pending.insert(k, serde_json::json!(n));
        }
        M::NumDone(k) => {
            if let Some(v) = st.pending.get(&k).cloned() {
                return update(sh, st, M::Apply(k, v));
            }
        }
    }
    Task::none()
}

fn val(st: &St, d: &Value) -> Value {
    st.pending.get(&gs(d, "key")).cloned().unwrap_or_else(|| go(d, "value").clone())
}
fn vstr(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        x => x.to_string(),
    }
}
fn icon(d: &Value) -> G {
    let k = gs(d, "key").to_lowercase();
    let has = |w: &[&str]| w.iter().any(|x| k.contains(x));
    if has(&["tdp", "power", "boost", "cpu", "gpu", "charge"]) {
        G::Power
    } else if has(&["fan"]) {
        G::Cog
    } else if has(&["rgb", "led", "light"]) {
        G::Palette
    } else if has(&["batt"]) {
        G::Power
    } else if has(&["display", "screen", "bright"]) {
        G::Display
    } else if has(&["touch", "mouse"]) {
        G::Pad
    } else if has(&["keyboard", "osk"]) {
        G::Grid
    } else if has(&["controller", "gamepad", "button", "stick", "trigger", "gyro", "rumble", "vibr", "swap", "layout"]) {
        G::Pad
    } else {
        G::Gear
    }
}
fn opt_raw(o: &Value) -> Value {
    o.get("raw").cloned().unwrap_or_else(|| go(o, "id").clone())
}

fn item<'a>(p: &Pal, st: &St, d: &Value) -> E<'a, M> {
    let key = gs(d, "key");
    let title = gs(d, "title");
    let hint = gs(d, "hint");
    let cur = val(st, d);
    let opts = ga(d, "options");
    match gs(d, "kind").as_str() {
        "bool" => {
            let on = truthy(Some(&cur));
            sw(p, icon(d), title, hint, on, Some(M::Apply(key, Value::Bool(!on))))
        }
        "choice" => {
            let few = opts.len() >= 2 && opts.len() <= 4 && opts.iter().map(|o| gs(o, "label").chars().count()).sum::<usize>() <= 36;
            let c = vstr(&cur);
            if few {
                choice(p, icon(d), title, hint, opts.iter().map(|o| (gs(o, "label"), gs(o, "id") == c, Some(M::Apply(key.clone(), opt_raw(o))))).collect())
            } else {
                let label = opts.iter().find(|o| gs(o, "id") == c).map(|o| gs(o, "label")).unwrap_or(c);
                r(icon(d), title, hint).value(label).chev().on(M::Pick(key)).view(p)
            }
        }
        "number" => {
            let (min, max) = (gn(d, "min"), gn(d, "max"));
            let step = if gn(d, "step") > 0.0 { gn(d, "step") } else { 1.0 };
            let n = cur.as_f64().or_else(|| vstr(&cur).parse().ok()).unwrap_or(min).clamp(min, max.max(min));
            let shown = if gb(d, "int") { format!("{}", n.round()) } else { format!("{}", (n * 100.0).round() / 100.0) };
            let unit = gs(d, "unit");
            let (k1, k2) = (key.clone(), key.clone());
            let int = gb(d, "int");
            let sl: E<M> = iced::widget::slider(min..=max.max(min + step), n, move |x: f64| M::Num(k1.clone(), if int { x.round() } else { (x * 1000.0).round() / 1000.0 })).step(step).on_release(M::NumDone(k2)).width(Length::Fixed(sz(260.0))).into();
            srow(p, if key.contains("tdp") { G::Power } else { G::Gear }, title, if unit.is_empty() { shown } else { format!("{shown} {unit}") }, sl)
        }
        "action" => r(G::Update, title, hint).value(if st.armed == key { "Press again to confirm" } else { "" }).on(M::Arm(key)).view(p),
        "group" => container(tm(title, sz(13.0), p.text)).padding(Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 4.0 }).into(),
        "color" => {
            let c = vstr(&go(d, "value").clone());
            let dot = container(iced::widget::Space::new()).width(Length::Fixed(22.0)).height(Length::Fixed(22.0)).style(ui::boxed(hearth_ui::theme::hex(&c).unwrap_or(p.panel2), 11.0, Some((p.line, 1.0))));
            srow(p, icon(d), title, hint, row![dot, t(c, sz(13.0), p.muted)].spacing(10.0).align_y(iced::Center).into())
        }
        _ => {
            let s = vstr(go(d, "value")).trim().to_string();
            if s.chars().count() <= 28 && !s.contains('\n') {
                r(icon(d), title, hint).value(s).view(p)
            } else {
                r(icon(d), title, s.split_whitespace().collect::<Vec<_>>().join(" ")).view(p)
            }
        }
    }
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let h = &st.hh;
    let running = gb(h, "running");
    let mut v: Vec<E<M>> = Vec::new();
    if !st.picking.is_empty() {
        let found = ga(h, "sections").iter().flat_map(|s| ga(s, "items").iter()).find(|i| gs(i, "key") == st.picking);
        v.push(r(G::Arrow, "Back", "").on(M::Unpick).view(&p));
        if let Some(it) = found {
            v.push(head(&p, gs(it, "title")));
            if gb(it, "hint") {
                v.push(small(&p, gs(it, "hint")));
            }
            let c = vstr(&val(st, it));
            for o in ga(it, "options") {
                let on = gs(o, "id") == c;
                v.push(r(if on { G::Check } else { G::Dots }, gs(o, "label"), "").value(if on { "In use" } else { "" }).on(M::Apply(gs(it, "key"), opt_raw(o))).view(&p));
            }
        }
        return page(v);
    }
    let title = if h.is_null() {
        if st.err.is_empty() { "Looking for Handheld Daemon…".to_string() } else { "Couldn't read Handheld Daemon".into() }
    } else if running {
        format!("Handheld Daemon{}", if gb(h, "version") { format!("  ·  {}", gs(h, "version").chars().take(16).collect::<String>()) } else { String::new() })
    } else {
        "Handheld Daemon isn't running".into()
    };
    let sub = if running {
        let c: Vec<String> = ga(h, "controllers").iter().filter_map(|x| x.as_str().map(String::from)).collect();
        format!("{}Its settings for this device. Changes apply right away, in games too.", if c.is_empty() { String::new() } else { format!("{}. ", c.join(", ")) })
    } else if !h.is_null() {
        "Handheld Daemon makes a handheld's built-in controls, power limit, fan and lights work (ROG Ally, Legion Go, Steam Deck, MSI Claw and more). It isn't running here, so there's nothing to set: on a desktop or laptop that's expected.".into()
    } else {
        st.err.clone()
    };
    let mark = container(hearth_ui::ui::glyph(G::Pad, if running { p.ink } else { p.muted }, 24.0, 1.8)).center_x(Length::Fixed(44.0)).center_y(Length::Fixed(44.0)).style(ui::boxed(if running { p.accent } else { p.panel3 }, p.r(12.0), None));
    v.push(container(row![mark, column![tm(title, sz(16.0), p.text), t(sub, sz(12.0), if h.is_null() && !st.err.is_empty() { p.danger } else { p.muted })].spacing(3.0).width(Length::Fill)].spacing(14.0).align_y(iced::Center)).padding(16.0).width(Length::Fill).style(ui::boxed(p.panel2, p.r(12.0), None)).into());
    if !running {
        v.push(r(G::Update, "Check again", "").on(M::Load).view(&p));
        return page(v);
    }
    v.push(small(&p, "In Steam Game Mode, Steam has its own Performance and controller settings; the sections below say where Steam takes over."));
    if !st.err.is_empty() {
        v.push(words(&p, st.err.clone(), true));
    }
    let adv = ga(h, "sections").iter().flat_map(|s| ga(s, "items").iter()).filter(|i| gb(i, "advanced") && gs(i, "kind") != "group").count();
    if adv > 0 {
        v.push(sw(&p, G::Gear, "Show advanced settings", format!("{adv} more, for tinkering (Handheld Daemon's advanced and expert options)"), st.advanced, Some(M::Advanced)));
    }
    for s in ga(h, "sections") {
        let shown: Vec<&Value> = ga(s, "items").iter().filter(|i| st.advanced || !gb(i, "advanced")).collect();
        if !shown.iter().any(|i| gs(i, "kind") != "group") {
            continue;
        }
        let mut g: Vec<E<M>> = Vec::new();
        let note = gs(s, "steamNote");
        if !note.is_empty() {
            g.push(container(row![ts("STEAM", sz(11.0), p.accent), t(note, sz(12.0), p.text)].spacing(10.0).align_y(iced::Center)).padding(Padding { top: 9.0, right: 12.0, bottom: 9.0, left: 12.0 }).width(Length::Fill).style(ui::boxed(Pal::a(p.accent, 0.12), p.r(10.0), Some((Pal::a(p.accent, 0.35), 1.0)))).into());
        }
        for d in shown {
            let depth = (gn(d, "depth") as i64).clamp(0, 3) as f32;
            g.push(container(item(&p, st, d)).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: depth * 22.0 }).into());
        }
        v.push(group(&p, gs(s, "title"), g));
    }
    page(v)
}

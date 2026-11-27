//! The Server home board (board 11, docs/mockups/server-expansion/11-server-home.png): the overall state with Fix it,
//! Open the dashboard and Server mode; one tile for each part (what it's doing, its graph, "Updates in the Store" and
//! its state there); your devices; and on the right Updates for the server (the first part with an update ready, its
//! Update, What's new and Try the test version; Update server apps by themselves, off unless you turn it on; Only when
//! nobody's playing or watching) and the rows that are also here.
//!
//! Settings › Home server and the Home server helper's Server home both draw it from the same pieces ([`pieces`]), each
//! in its own blocks; every number comes from the parts' engines (`<engine> summary`) and the Store's own list.

use crate::draw::{self, pad, st, E};
use crate::parts::{self, Part};
use crate::store::{self, Up};
use crate::{arr, b, f, nums, s, strs};
use hearth_oss::w::{self, Sz, Tg};
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, mouse_area, row, Column, Row, Space};
use iced::{mouse, Color, Length};
use serde_json::Value;

/// What the board shows, read once (and again after a change).
#[derive(Debug, Clone, Default)]
pub struct HomeData {
    /// each part's `summary` (Null: its engine isn't on this PC yet)
    pub sums: Vec<(&'static str, Value)>,
    /// the Store's state of each part's Store part, and the Home server helper app's own
    pub store: Vec<Up>,
    pub loaded: bool,
}

impl HomeData {
    pub fn sum(&self, id: &str) -> &Value {
        self.sums.iter().find(|x| x.0 == id).map(|x| &x.1).unwrap_or(&Value::Null)
    }
    pub fn up(&self, id: &str) -> Option<&Up> {
        self.store.iter().find(|u| u.id == id)
    }
    /// The overall line comes from the Dashboard's summary.
    pub fn overall(&self) -> &Value {
        crate::at(self.sum("dash"), "overall")
    }
    /// The Store parts with an update ready, the Home server helper's first.
    pub fn ready(&self) -> Vec<&Up> {
        self.store.iter().filter(|u| u.ready).collect()
    }
}

/// Every part's summary and the Store's states (blocking: call it off the window's thread, as [`load`] does).
pub fn read() -> HomeData {
    let mut d = HomeData { loaded: true, ..Default::default() };
    for p in parts::sorted() {
        let v = if crate::engine::present(p.engine) { crate::engine::blocking(p.engine, &["summary".to_string()], "").json().unwrap_or(Value::Null) } else { Value::Null };
        d.sums.push((p.id, v));
    }
    let mut ids: Vec<&str> = vec![parts::HELPER_PART, parts::APP_PART];
    for p in parts::PARTS {
        if !ids.contains(&p.store) {
            ids.push(p.store);
        }
    }
    for id in ids {
        d.store.push(store::state(id));
    }
    d
}

pub fn load() -> impl std::future::Future<Output = HomeData> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-server-home".into()).spawn(move || {
        let _ = tx.send(read());
    });
    async move { rx.await.unwrap_or_default() }
}

/// What the board's buttons ask for (the caller does it: Settings and the helper each in their own way).
#[derive(Debug, Clone, PartialEq)]
pub enum HAct {
    /// a part's page in the Home server helper
    Open(&'static str),
    /// the first thing that needs a look: its fix
    Fix,
    Dashboard,
    ServerMode,
    /// install the Store part's update (its id), or its test version
    Update(String),
    Test(String),
    /// What's new: shown under it, or hidden again
    Notes(String),
    /// Update server apps by themselves (off unless you turn it on), Only when nobody's playing or watching
    Auto(bool),
    Idle(bool),
    /// a row of Also here (a Settings page)
    Also(&'static str),
    EditLayout,
}

fn click<'a, M: Clone + 'a>(el: impl Into<E<'a, M>>, m: M) -> E<'a, M> {
    mouse_area(el.into()).on_press(m).interaction(mouse::Interaction::Pointer).into()
}

/// The overall card: the dot, "12 of 13 services healthy · 3 PCs · 8 devices", what needs a look and the rest, Fix
/// it, Open the dashboard, Server mode.
pub fn state_card<'a, M: Clone + 'a>(p: &Pal, d: &'a HomeData, m: impl Fn(HAct) -> M + Clone + 'a) -> E<'a, M> {
    let o = d.overall();
    let head = if !d.loaded {
        "Looking at the server…".to_string()
    } else if o.is_null() {
        "The Dashboard part isn't on this PC yet".to_string()
    } else {
        s(o, "headline")
    };
    let line = if o.is_null() && d.loaded { "Get the Home server helper's update in the Hearth Store: every part comes from there".to_string() } else { s(o, "line") };
    let tone = if o.is_null() { "off".to_string() } else { s(o, "state") };
    let fix = !arr(o, "needs").is_empty();
    let r = row![
        st(&tone, 16.0),
        // the words take what's left, so the three buttons always stay in view (Settings' narrower page too)
        column![tb(head, 18.0, p.text), t(line, 12.0, p.muted)].spacing(3.0).width(Length::Fill),
        w::chip(p, "Fix it", false, Sz::Md, if fix { Some(m(HAct::Fix)) } else { None }),
        w::chip(p, "Open the dashboard", false, Sz::Md, Some(m(HAct::Dashboard))),
        w::chip(p, "Server mode", true, Sz::Md, Some(m(HAct::ServerMode))),
    ]
    .spacing(18.0)
    .align_y(iced::Center);
    let bg = if p.dark { Color::from_rgb8(0x23, 0x1a, 0x14) } else { Pal::mix(p.panel, p.accent, 0.06) };
    container(r).padding(pad(16.0, 20.0, 16.0, 20.0)).width(Length::Fill).style(ui::boxed(bg, p.r(12.0), Some((Pal::mix(p.line, p.accent, 0.25), 1.0)))).into()
}

/// The footer every tile has: "● Updates in the Store" and its state there.
pub fn store_line<'a, M: Clone + 'a>(p: &Pal, d: &HomeData, part: &Part, here: bool, m: impl Fn(HAct) -> M) -> E<'a, M> {
    let ready = d.up(part.store).map(|u| u.ready).unwrap_or(false) || d.up(parts::APP_PART).map(|u| u.ready && part.id == "dash").unwrap_or(false);
    let tag: E<'a, M> = if !here {
        click(w::tag(p, "Get it", Tg::Acc), m(HAct::Update(part.store.to_string())))
    } else if ready {
        click(w::tag(p, "Update ready", Tg::Acc), m(HAct::Update(part.store.to_string())))
    } else {
        w::tag(p, "Up to date", Tg::Ok)
    };
    let line = container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(ui::boxed(Pal::mix(p.panel, p.line, 0.6), 0.0, None));
    column![line, container(row![t("●", 12.0, p.accent), t("Updates in the Store", 12.0, p.text).wrapping(Wrapping::None), Space::new().width(Length::Fill), tag].spacing(8.0).align_y(iced::Center)).padding(pad(10.0, 0.0, 0.0, 0.0))].into()
}

/// One part's tile: its mark, name and line; its big number; its two lines (and a red one when something's wrong);
/// its graph; Updates in the Store.
pub fn tile<'a, M: Clone + 'a>(p: &Pal, d: &'a HomeData, part: &'static Part, sel: bool, m: impl Fn(HAct) -> M + Clone + 'a) -> E<'a, M> {
    let v = d.sum(part.id);
    let here = !v.is_null();
    let head = row![w::mark(part.mark, part.grad, 44.0), column![tb(part.name, 16.0, p.text), t(part.line, 12.0, p.muted)].spacing(2.0)].spacing(10.0).align_y(iced::Center);
    let mut c = Column::new().spacing(8.0).push(head);
    if here {
        c = c.push(row![tb(s(v, "big"), 26.0, p.text).wrapping(Wrapping::None), container(t(s(v, "of"), 14.0, p.muted)).padding(pad(0.0, 0.0, 4.0, 0.0))].spacing(6.0).align_y(iced::alignment::Vertical::Bottom));
        let mut lines = Column::new().spacing(3.0);
        for l in strs(v, "lines") {
            lines = lines.push(t(l, 12.0, p.muted));
        }
        let alert = s(v, "alert");
        if !alert.is_empty() {
            lines = lines.push(t(alert, 12.0, w::red()));
        }
        c = c.push(lines);
        c = c.push(container(draw::spark(nums(v, "spark"), draw::hex(part.color), true, Length::Fill, 44.0)).padding(pad(4.0, 0.0, 0.0, 0.0)));
    } else {
        c = c.push(tb("—", 26.0, p.muted)).push(t(if d.loaded { "Not on this PC yet: it comes from the Hearth Store" } else { "Looking…" }, 12.0, p.muted));
    }
    c = c.push(Space::new().height(Length::Fill)).push(store_line(p, d, part, here, m.clone()));
    let (bg, line) = if sel { (Pal::mix(p.panel, p.accent, 0.08), (p.accent, 2.0)) } else { (p.panel, (p.line, 1.0)) };
    let card = container(c).padding(16.0).width(Length::Fill).height(Length::Fill).style(ui::boxed(bg, p.r(14.0), Some(line)));
    click(card, m(HAct::Open(part.id)))
}

/// Your devices: each with its dot and what it's doing, from the Dashboard's summary (Tailscale and Hearth Link).
pub fn devices<'a, M: Clone + 'a>(p: &Pal, d: &'a HomeData) -> E<'a, M> {
    let o = d.overall();
    let devs = arr(o, "devices");
    let mut r = Row::new().spacing(8.0);
    for dv in &devs {
        let note = s(dv, "note");
        let label = if note.is_empty() { s(dv, "name") } else { format!("{} · {}", s(dv, "name"), note) };
        let chip = container(row![st(&s(dv, "state"), 10.0), t(label, 12.0, p.text).wrapping(Wrapping::None)].spacing(8.0).align_y(iced::Center))
            .padding(pad(0.0, 10.0, 0.0, 10.0))
            .center_y(Length::Fixed(28.0))
            .style(ui::boxed(p.panel2, p.r(8.0), Some((p.line, 1.0))));
        r = r.push(chip);
    }
    if devs.is_empty() {
        r = r.push(t(if d.loaded { "No devices seen yet: they show up here once they're on Tailscale or Hearth Link" } else { "Looking…" }, 12.0, p.muted));
    }
    let top = row![tb(format!("YOUR DEVICES · {}", devs.len()), 11.0, p.muted), Space::new().width(Length::Fill), t(s(o, "devicesLine"), 12.0, p.muted)].align_y(iced::Center);
    container(column![top, r.wrap()].spacing(10.0)).padding(pad(12.0, 16.0, 12.0, 16.0)).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0)))).into()
}

/// "Each tile is a block: drag to reorder, hide the ones you don't use · Edit layout".
pub fn foot<'a, M: Clone + 'a>(p: &Pal, m: impl Fn(HAct) -> M) -> E<'a, M> {
    row![t("Each tile is a block: drag to reorder, hide the ones you don't use · ", 12.0, p.muted), click(tb("Edit layout", 12.0, p.accent), m(HAct::EditLayout))].align_y(iced::Center).into()
}

/// Updates for the server (the right column's top): the first part with an update, Update, What's new, Try the test
/// version; the two switches; the line about parts that need the system.
pub fn updates<'a, M: Clone + 'a>(p: &Pal, d: &'a HomeData, notes_open: &str, m: impl Fn(HAct) -> M + Clone + 'a) -> E<'a, M> {
    let o = d.overall();
    let mut c = Column::new().spacing(0.0).push(tb("Updates for the server", 18.0, p.text)).push(container(t("Every part comes from the Hearth Store and updates there, never as a Hearth OS update.", 12.0, p.muted)).padding(pad(4.0, 0.0, 0.0, 0.0)));
    let ready = d.ready();
    if let Some(u) = ready.first() {
        let (mark, grad, name) = name_of(&u.id);
        let what = if u.notes.is_empty() { format!("Version {} is ready (you have {}).", u.version, if u.have.is_empty() { "the built-in one".into() } else { u.have.clone() }) } else { u.notes.first().cloned().unwrap_or_default() };
        let mut bxc = Column::new().spacing(0.0).push(row![w::mark(mark, grad, 22.0), tb(name, 12.0, p.text), Space::new().width(Length::Fill), w::tag(p, "Ready", Tg::Acc)].spacing(8.0).align_y(iced::Center));
        bxc = bxc.push(container(t(what, 12.0, p.muted)).padding(pad(6.0, 0.0, 0.0, 0.0)));
        if notes_open == u.id {
            for n in u.notes.iter().skip(1) {
                bxc = bxc.push(t(format!("· {n}"), 12.0, p.muted));
            }
        }
        let mut btns = row![w::chip(p, "Update", true, Sz::Sm, Some(m(HAct::Update(u.id.clone())))), w::chip(p, "What's new", false, Sz::Sm, Some(m(HAct::Notes(u.id.clone()))))].spacing(6.0);
        if u.test {
            btns = btns.push(w::chip(p, "Try the test version", false, Sz::Sm, Some(m(HAct::Test(u.id.clone())))));
        }
        bxc = bxc.push(container(btns).padding(pad(10.0, 0.0, 0.0, 0.0)));
        if ready.len() > 1 {
            bxc = bxc.push(container(t(format!("and {} more ready", ready.len() - 1), 12.0, p.muted)).padding(pad(6.0, 0.0, 0.0, 0.0)));
        }
        c = c.push(container(draw::bx(p, bxc)).padding(pad(12.0, 0.0, 0.0, 0.0)));
    } else if d.loaded {
        c = c.push(container(draw::bx(p, row![st("ok", 10.0), t("Every server part is up to date", 12.0, p.text)].spacing(8.0).align_y(iced::Center))).padding(pad(12.0, 0.0, 0.0, 0.0)));
    }
    let auto = b(o, "updates.auto");
    let idle = if crate::at(o, "updates.idle").is_null() { true } else { b(o, "updates.idle") };
    c = c.push(container(draw::lr(t("Update server apps by themselves", 12.0, p.text), w::sw(p, auto, Some(m(HAct::Auto(!auto)))))).padding(pad(12.0, 0.0, 0.0, 0.0)));
    c = c.push(container(draw::lr(t("Only when nobody's playing or watching", 12.0, p.text), w::sw(p, idle, Some(m(HAct::Idle(!idle)))))).padding(pad(10.0, 0.0, 0.0, 0.0)));
    c = c.push(container(t("Parts that need the system (a service, the firewall, a TV tuner) install with your password once, from inside the app.", 12.0, p.muted)).padding(pad(8.0, 0.0, 0.0, 0.0)));
    c.into()
}

/// "Also here": the older Home server pages, each with its value.
pub fn also<'a, M: Clone + 'a>(p: &Pal, rows: Vec<(&'static str, String, &'static str)>, m: impl Fn(HAct) -> M) -> E<'a, M> {
    let mut c = Column::new().push(container(tb("ALSO HERE", 11.0, p.muted)).padding(pad(20.0, 0.0, 4.0, 0.0)));
    let n = rows.len();
    for (i, (label, val, go)) in rows.into_iter().enumerate() {
        let r = row![t(label, 13.0, p.text), Space::new().width(Length::Fill), t(format!("{val} ›"), 13.0, p.muted).wrapping(Wrapping::None)].align_y(iced::Center);
        c = c.push(click(draw::under(p, r, i + 1 == n), m(HAct::Also(go))));
    }
    c.into()
}

/// A Store part's mark and name, for Updates for the server.
pub fn name_of(id: &str) -> (&'static str, &'static str, String) {
    if let Some(p) = parts::PARTS.iter().find(|p| p.store == id && p.store != parts::HELPER_PART) {
        return (p.mark, p.grad, p.name.to_string());
    }
    match id {
        parts::APP_PART => ("HS", "or", "Home server helper".into()),
        parts::HELPER_PART => ("HS", "gy", "The server's parts (Dashboard, Model vault, Backups…)".into()),
        other => ("SP", "gy", other.to_string()),
    }
}

/// A count for a sidebar row (the part's summary's "count").
pub fn count(d: &HomeData, id: &str) -> String {
    s(d.sum(id), "count")
}

/// The overall state's numbers, for a title bar tag ("12 OF 13 HEALTHY").
pub fn healthy(d: &HomeData) -> (f64, f64) {
    let o = d.overall();
    (f(o, "healthy"), f(o, "total"))
}

/// The board laid out as drawn (Settings › Home server and Server home): the state, the tiles 4 across, your
/// devices, the footnote; `side` is the right column.
pub fn tiles_grid<'a, M: 'a>(mut tiles: Vec<E<'a, M>>, cols: usize, h: f32) -> E<'a, M> {
    let mut c = Column::new().spacing(14.0);
    while !tiles.is_empty() {
        let rest = tiles.split_off(tiles.len().min(cols));
        let mut r = Row::new().spacing(14.0).height(Length::Fixed(h));
        let n = tiles.len();
        for t in tiles {
            r = r.push(container(t).width(Length::FillPortion(1)).height(Length::Fill));
        }
        for _ in n..cols {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        c = c.push(r);
        tiles = rest;
    }
    c.into()
}

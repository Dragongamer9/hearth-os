//! The opened pages of Server mode's own tiles, as Mantel draws them: the AI team (AiTeamDetail.qml with AiMesh.qml),
//! Ithil (IthilDetail.qml with her brain, brain.rs), the game download cache (CacheDetail.qml), the ad blocker
//! (AdsDetail.qml) and quiet hours & rules (RulesDetail.qml), with their shared pieces (ServerViewHead, ServerPanel,
//! ServerSwitch, ServerKeys). Everything they show comes from `hearth-server-mode dash`; their buttons go through
//! main.rs (hearth-server-mode dns … / rules … / service … / ithil-*), as ServerDash.qml's featureAct does.

use crate::brain;
use crate::draw::{self, a as alpha, icon, rgb, Memo};
use crate::{arr, f, s, truthy, App, Msg};
use hearth_ui::app::INTER;
use hearth_ui::ui::weight;
use iced::font::Weight;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::text::{Alignment, Rich, Span, Wrapping};
use iced::widget::{column, container, mouse_area, row, stack, text, text_input, Column, Row, Space};
use iced::{mouse, Color, Element, Font, Length, Padding, Point, Rectangle, Renderer, Size, Theme};
use serde_json::Value;

type E<'a> = Element<'a, Msg>;

fn boxed(bg: Color, r: f32, line: Option<(Color, f32)>) -> impl Fn(&Theme) -> container::Style {
    hearth_ui::ui::boxed(bg, r, line)
}
fn tx<'a>(v: impl Into<String>, size: f32, c: Color) -> iced::widget::Text<'a> {
    text(v.into()).size(size).color(c).font(INTER).wrapping(Wrapping::None)
}
fn txw<'a>(v: impl Into<String>, size: f32, c: Color) -> iced::widget::Text<'a> {
    text(v.into()).size(size).color(c).font(INTER).wrapping(Wrapping::Word)
}
fn txs<'a>(v: impl Into<String>, size: f32, c: Color) -> iced::widget::Text<'a> {
    tx(v, size, c).font(weight(Weight::Semibold))
}
fn txb<'a>(v: impl Into<String>, size: f32, c: Color) -> iced::widget::Text<'a> {
    tx(v, size, c).font(weight(Weight::Bold))
}
fn sp<'a>(v: impl Into<String>, size: f32, c: Color, bold: bool) -> Span<'a, (), Font> {
    Span::new(v.into()).size(size).color(c).font(if bold { weight(Weight::Bold) } else { INTER })
}
fn rich<'a>(v: Vec<Span<'a, (), Font>>) -> E<'a> {
    Rich::<'a, (), Msg>::from_iter(v).into()
}
fn fill<'a>() -> E<'a> {
    Space::new().width(Length::Fill).into()
}
fn vfill<'a>() -> E<'a> {
    Space::new().height(Length::Fill).into()
}
fn click<'a>(e: E<'a>, m: Msg) -> E<'a> {
    mouse_area(e).on_press(m).interaction(mouse::Interaction::Pointer).into()
}
fn rowsv(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().map(|x| match x { Value::String(s) => s.clone(), Value::Number(n) => n.to_string(), _ => String::new() }).collect()).unwrap_or_default()
}

// The feature pages' own colours (ServerPanel / ServerSwitch / ServerViewHead).
const TEXT: u32 = 0xfff4df;
const MUTED: u32 = 0xc9b79f;
const OK: u32 = 0x6fe39a;
const WARN: u32 = 0xf0c060;
const ORANGE: u32 = 0xff8c42;
const ACC: u32 = 0xffb347;

/// ServerViewHead: the icon, the title, the line under it, the chip; ringed when the feature itself is picked.
fn head<'a>(k: f32, ic: &str, title: &str, sub: &str, chip: &str, chip_c: Color, picked: bool) -> E<'a> {
    let r = row![
        container(icon(ic, rgb(ORANGE), 30.0 * k)).center_x(Length::Fixed(58.0 * k)).center_y(Length::Fixed(58.0 * k)).style(boxed(rgb(0x3a2614), 12.0 * k, None)),
        column![txs(title.to_string(), 38.0 * k, rgb(TEXT)), container(tx(sub.to_string(), 18.0 * k, rgb(MUTED))).width(Length::Fill).clip(true)].spacing(2.0).width(Length::Fill),
        container(tx(chip.to_string(), 19.0 * k, chip_c)).padding(Padding { top: 0.0, right: 14.0 * k, bottom: 0.0, left: 14.0 * k }).center_y(Length::Fixed(38.0 * k)).style(boxed(alpha(chip_c, 0.16), 8.0 * k, None)),
    ]
    .spacing(18.0 * k)
    .align_y(iced::Center);
    container(r).padding(8.0 * k).width(Length::Fill).style(boxed(Color::TRANSPARENT, 12.0 * k, if picked { Some((rgb(ACC), 2.5 * k)) } else { None })).into()
}

/// ServerPanel: its title in small capitals and its rows.
fn panel<'a>(k: f32, title: &str, body: Vec<E<'a>>) -> E<'a> {
    let mut c = Column::new().spacing(10.0 * k).push(txs(title.to_uppercase(), 16.0 * k, rgb(MUTED)));
    for b in body {
        c = c.push(b);
    }
    container(c).padding(22.0 * k).width(Length::Fill).style(boxed(rgb(0x211b14), 14.0 * k, Some((rgb(0x3a2f22), 1.0)))).into()
}

/// ServerSwitch: a row's name, its chip, the switch, the ring when picked.
fn switch<'a>(k: f32, label: E<'a>, sub: &str, chip: &str, chip_kind: &str, on: bool, picked: bool, m: Msg) -> E<'a> {
    let cc = match chip_kind { "warn" => rgb(WARN), "muted" => rgb(MUTED), _ => rgb(OK) };
    let mut left = Column::new().spacing(2.0).width(Length::Fill).push(container(label).width(Length::Fill).clip(true));
    if !sub.is_empty() {
        left = left.push(txw(sub.to_string(), 13.0 * k, rgb(MUTED)));
    }
    let mut r = Row::new().spacing(14.0 * k).align_y(iced::Center).push(left);
    if !chip.is_empty() {
        r = r.push(container(tx(chip.to_string(), 13.0 * k, cc)).padding(Padding { top: 0.0, right: 9.0 * k, bottom: 0.0, left: 9.0 * k }).center_y(Length::Fixed(26.0 * k)).style(boxed(alpha(cc, 0.14), 6.0 * k, None)));
    }
    let knob = container(Space::new()).width(Length::Fixed(22.0 * k)).height(Length::Fixed(22.0 * k)).style(boxed(if on { rgb(0x1a120a) } else { rgb(0x9a9088) }, 11.0 * k, None));
    let sw = container(knob)
        .padding(Padding { top: 0.0, right: 4.0 * k, bottom: 0.0, left: 4.0 * k })
        .width(Length::Fixed(54.0 * k))
        .height(Length::Fixed(30.0 * k))
        .align_y(iced::alignment::Vertical::Center)
        .align_x(if on { iced::alignment::Horizontal::Right } else { iced::alignment::Horizontal::Left })
        .style(boxed(if on { rgb(ORANGE) } else { rgb(0x3a3330) }, 15.0 * k, None));
    r = r.push(sw);
    let body = container(r)
        .padding(Padding { top: 8.0 * k, right: if picked { 12.0 * k } else { 0.0 }, bottom: 8.0 * k, left: if picked { 12.0 * k } else { 0.0 } })
        .width(Length::Fill)
        .style(boxed(Color::TRANSPARENT, 10.0 * k, if picked { Some((rgb(ACC), 2.5 * k)) } else { None }));
    let mut c = Column::new().push(click(body.into(), m));
    if !picked {
        c = c.push(container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(boxed(rgb(0x3a2f22), 0.0, None)));
    }
    c.into()
}

/// ServerKeys: the controller keys along the bottom.
fn keys<'a>(k: f32, ks: &[(&str, String, Msg)], back: &str) -> E<'a> {
    let key = |l: &str| -> E<'a> {
        let menu = l == "☰";
        container(txb(l.to_string(), if menu { 20.0 } else { 14.0 } * k, rgb(TEXT)))
            .center_x(Length::Fixed(26.0 * k))
            .center_y(Length::Fixed(26.0 * k))
            .style(boxed(Color::TRANSPARENT, if menu { 4.0 * k } else { 13.0 * k }, if menu { None } else { Some((rgb(MUTED), 1.5)) }))
            .into()
    };
    let mut r = Row::new().spacing(30.0 * k).align_y(iced::Center);
    for (l, t, m) in ks {
        r = r.push(click(row![key(l), tx(t.clone(), 19.0 * k, rgb(TEXT))].spacing(8.0 * k).align_y(iced::Center).into(), m.clone()));
    }
    r = r.push(fill()).push(click(row![key("B"), tx(back.to_string(), 19.0 * k, rgb(TEXT))].spacing(8.0 * k).align_y(iced::Center).into(), Msg::Back));
    r.into()
}

fn gbs(b: f64) -> String {
    if b >= 1e12 {
        format!("{} TB", (b / 1e11).round() / 10.0)
    } else if b >= 10e9 {
        format!("{} GB", (b / 1e9).round())
    } else {
        format!("{} GB", (b / 1e8).round() / 10.0)
    }
}
fn rate(r: f64) -> String {
    if r >= 1e6 { format!("{} MB/s", (r / 1e6).round()) } else { format!("{} KB/s", (r / 1e3).round()) }
}
fn thousands(n: f64) -> String {
    let s = format!("{}", n.round() as i64);
    let mut o = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 && c != '-' {
            o.push(',');
        }
        o.push(c);
    }
    o.chars().rev().collect()
}

// ================================================================ the game download cache
pub const STORES: [&str; 4] = ["steam", "epic", "blizzard", "xbox"];

pub fn cache_page<'a>(a: &'a App, t: &Value, k: f32) -> E<'a> {
    let c = t.get("cache").cloned().unwrap_or(Value::Null);
    let v = c.get("view").cloned().unwrap_or(Value::Null);
    let on = truthy(&c, "on");
    let chip = if on { if truthy(&c, "running") { "On" } else { "Starting" } } else { "Off" };
    let fv = |kk: &str| f(&v, kk).unwrap_or(0.0);
    // the days
    let bars = arr(&v, "bars");
    let most = bars.iter().map(|b| f(b, "saved").unwrap_or(0.0)).fold(1.0f64, f64::max);
    let mut days = Row::new().spacing(10.0 * k).height(Length::Fixed(170.0 * k)).align_y(iced::alignment::Vertical::Bottom);
    for b in &bars {
        let h = (130.0 * k * (f(b, "saved").unwrap_or(0.0) / most) as f32).max(4.0 * k);
        days = days.push(
            column![
                vfill(),
                container(Space::new()).width(Length::Fill).height(Length::Fixed(h)).style(boxed(rgb(0xf2994a), 3.0 * k, None)),
                tx(s(b, "day"), 14.0 * k, rgb(MUTED))
            ]
            .spacing(4.0 * k)
            .width(Length::FillPortion(1))
            .height(Length::Fill),
        );
    }
    let saved = panel(k, "Saved this week", vec![
        row![txb(gbs(fv("savedWeek")), 54.0 * k, rgb(TEXT)), tx("not downloaded again", 21.0 * k, rgb(TEXT))].spacing(12.0 * k).align_y(iced::alignment::Vertical::Bottom).into(),
        txw(format!("Your internet only gave each game once: {} came from the internet, {} went to your devices from here.", gbs(fv("internetWeek")), gbs(fv("devicesWeek"))), 15.0 * k, rgb(MUTED)).into(),
        days.into(),
    ]);
    let mut now_rows: Vec<E> = vec![];
    for n in arr(&v, "now") {
        now_rows.push(row![container(tx(format!("{} · {}", s(&n, "device"), s(&n, "what")), 18.0 * k, rgb(TEXT))).width(Length::Fill).clip(true), tx(format!("{} · {}", s(&n, "from"), rate(f(&n, "rate").unwrap_or(0.0))), 17.0 * k, rgb(MUTED))].spacing(10.0 * k).into());
    }
    if now_rows.is_empty() {
        now_rows.push(tx(if on { "Nobody is downloading right now." } else { "Off: your devices download from the internet as usual." }, 16.0 * k, rgb(MUTED)).into());
    }
    let right_now = panel(k, "Right now", now_rows);
    let mut stores: Vec<E> = vec![];
    for (i, id) in STORES.iter().enumerate() {
        let name = c.get("storeNames").map(|n| s(n, id)).filter(|x| !x.is_empty()).unwrap_or_else(|| id.to_string());
        let son = c.get("stores").map(|x| truthy(x, id)).unwrap_or(false);
        stores.push(switch(k, tx(name, 18.0 * k, rgb(TEXT)).into(), "", "", "ok", son, a.fsel == i + 1, Msg::FeatRow(i + 1)));
    }
    let size = f(&c, "sizeGB").unwrap_or(0.0) * 1e9;
    let used = fv("used");
    let drive = { let d = s(&v, "drive"); if d.is_empty() { "This PC".into() } else { d } };
    let space = panel(k, "Space", vec![
        rich(vec![sp(format!("{drive} · "), 18.0 * k, rgb(TEXT), false), sp(gbs(used), 18.0 * k, rgb(TEXT), true), sp(format!(" of {} for the cache", gbs(size)), 18.0 * k, rgb(TEXT), false)]),
        hearth_ui::ui::bar(vec![((used / size.max(1.0)).min(1.0) as f32, rgb(0xf2994a))], rgb(0x14100c), 10.0 * k, 5.0 * k),
        tx("The oldest games leave the cache first when it's full.", 15.0 * k, rgb(MUTED)).into(),
    ]);
    let away = panel(k, "If this PC isn't there", vec![txw("Your devices download from the internet as normal. Nothing waits for the server, and nothing breaks.", 16.0 * k, rgb(MUTED)).into()]);
    let mut rightc = Column::new().spacing(20.0 * k).width(Length::FillPortion(10)).push(panel(k, "Stores", stores)).push(space).push(away);
    if a.ask_empty {
        rightc = rightc.push(txw("Press X again to empty the cache (your devices download each game from the internet once more).", 16.0 * k, rgb(WARN)));
    }
    column![
        head(k, "update", "Game download cache", "LANCache on this PC · on your home network (and Tailscale) · off until you turn it on", chip, if on { rgb(OK) } else { rgb(MUTED) }, a.fsel == 0),
        row![column![saved, right_now].spacing(20.0 * k).width(Length::FillPortion(12)), rightc].spacing(20.0 * k),
        vfill(),
        keys(k, &[("A", "Turn on or off".into(), Msg::FeatKey("enter".into())), ("X", "Empty the cache (asks twice)".into(), Msg::FeatKey("x".into())), ("Y", "Settings".into(), Msg::FeatKey("y".into()))], "Back to the dashboard"),
    ]
    .spacing(20.0 * k)
    .padding(32.0 * k)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

// ================================================================ the ad blocker
pub fn ads_devices(t: &Value) -> Vec<Value> {
    t.get("ads").and_then(|x| x.get("view")).map(|v| arr(v, "devices")).unwrap_or_default()
}

pub fn ads_page<'a>(a: &'a App, t: &Value, k: f32) -> E<'a> {
    let ad = t.get("ads").cloned().unwrap_or(Value::Null);
    let v = ad.get("view").cloned().unwrap_or(Value::Null);
    let today = v.get("today").cloned().unwrap_or(Value::Null);
    let on = truthy(&ad, "on");
    let paused = f(&v, "pausedUntil").unwrap_or(0.0) > 0.0;
    let chip = if on { if paused { "Paused" } else if truthy(&ad, "running") { "On" } else { "Starting" } } else { "Off" };
    let mut nums = Row::new();
    for (val, lbl) in [
        (format!("{} %", f(&today, "percent").unwrap_or(0.0)), "of lookups blocked"),
        (thousands(f(&today, "blocked").unwrap_or(0.0)), "ads and trackers stopped"),
        (format!("{} ms", f(&today, "ms").unwrap_or(0.0)), "average answer"),
    ] {
        nums = nums.push(column![txb(if on { val } else { "—".into() }, 52.0 * k, rgb(TEXT)), tx(lbl, 15.0 * k, rgb(MUTED))].width(Length::FillPortion(1)));
    }
    let mut top: Vec<E> = vec![];
    for x in arr(&v, "top") {
        top.push(row![container(tx(s(&x, "name"), 18.0 * k, rgb(TEXT))).width(Length::Fill).clip(true), tx(thousands(f(&x, "count").unwrap_or(0.0)), 17.0 * k, rgb(MUTED))].into());
    }
    if top.is_empty() {
        top.push(tx(if on { "Nothing blocked yet today." } else { "Off." }, 16.0 * k, rgb(MUTED)).into());
    }
    let mut devs: Vec<E> = vec![];
    for (i, d) in ads_devices(t).iter().enumerate() {
        let st = s(d, "state");
        let kind = if st.starts_with("Paused") || st.starts_with("Set your") { "warn" } else { "ok" };
        devs.push(switch(k, tx(s(d, "name"), 18.0 * k, rgb(TEXT)).into(), "", &st, kind, truthy(d, "on"), a.fsel == i + 1 && !a.typing, Msg::FeatRow(i + 1)));
    }
    let mut rightc = Column::new().spacing(20.0 * k).width(Length::FillPortion(10)).push(panel(k, "Your devices", devs)).push(panel(k, "Never stops the internet", vec![
        txw("If this PC can't be reached, each device goes back to normal lookups by itself within seconds. A site that breaks: allow it from here or your phone.", 16.0 * k, rgb(MUTED)).into(),
    ]));
    if a.typing {
        let input = text_input("example.com", &a.site)
            .id(crate::SITE_ID)
            .on_input(Msg::Site)
            .on_submit(Msg::FeatKey("enter".into()))
            .size(19.0 * k)
            .padding(Padding { top: 12.0 * k, right: 14.0 * k, bottom: 12.0 * k, left: 14.0 * k })
            .style(|_t: &Theme, _s| text_input::Style {
                background: iced::Background::Color(rgb(0x14100c)),
                border: iced::Border { color: rgb(ACC), width: 2.0, radius: 10.0.into() },
                icon: rgb(MUTED),
                placeholder: rgb(0x7a6a58),
                value: rgb(TEXT),
                selection: alpha(rgb(ACC), 0.4),
            });
        rightc = rightc.push(panel(k, "Allow a site", vec![input.into(), tx("A or Enter allows it · B cancels", 14.0 * k, rgb(MUTED)).into()]));
    }
    let ylabel = if a.fsel == 0 { if paused { "Block again" } else { "Pause for 1 hour" } } else { "Pause this device for 1 hour" };
    column![
        head(k, "shield", "Ad blocker", "AdGuard Home · your home Wi-Fi and your Hearth devices over Tailscale · off until you turn it on", chip, if on && !paused { rgb(OK) } else { rgb(MUTED) }, a.fsel == 0 && !a.typing),
        row![column![panel(k, "Today", vec![nums.into()]), panel(k, "Most blocked", top)].spacing(20.0 * k).width(Length::FillPortion(12)), rightc].spacing(20.0 * k),
        vfill(),
        keys(k, &[("A", "Turn on or off".into(), Msg::FeatKey("enter".into())), ("X", "Allow a site…".into(), Msg::FeatKey("x".into())), ("Y", ylabel.into(), Msg::FeatKey("y".into())), ("☰", "Settings".into(), Msg::FeatKey("menu".into()))], "Back to the dashboard"),
    ]
    .spacing(20.0 * k)
    .padding(32.0 * k)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

// ================================================================ quiet hours & rules
pub fn hhmm(m: i64) -> String {
    let m = ((m % 1440) + 1440) % 1440;
    format!("{:02}:{:02}", m / 60, m % 60)
}
pub fn whens(a: &App) -> Vec<(&'static str, String)> {
    let w = &a.when_vals;
    vec![
        ("jellyfin", "someone is watching Jellyfin".into()),
        ("game", "a game runs on this PC".into()),
        ("device", format!("a device is connected{}", if a.device.is_empty() { String::new() } else { format!(" ({})", a.device) })),
        ("time", format!("it's {} – {}", hhmm(w.2), hhmm(w.3))),
        ("drive", format!("the drive is over {} °C", w.0)),
        ("power", format!("power use is over {} W", w.1)),
        ("quiet", format!("Quiet hours: {} – {} · Mon–Fri", hhmm(w.2), hhmm(w.3))),
    ]
}
pub fn dos(a: &App) -> Vec<(&'static str, String)> {
    vec![
        ("pictures", "AI pictures wait".into()),
        ("voices", "voices wait".into()),
        ("downloads", "downloads at half speed".into()),
        ("backups", "backups wait".into()),
        ("fans", format!("fans capped at {} %", a.fan_pct)),
        ("phone", "tell my phone".into()),
    ]
}
pub fn rules_of(t: &Value) -> (Vec<Value>, Vec<Value>, Value) {
    let st = t.get("rules").cloned().unwrap_or(Value::Null);
    (arr(&st, "quiet"), arr(&st, "rules"), st)
}

pub fn rules_page<'a>(a: &'a App, t: &Value, k: f32) -> E<'a> {
    let (quiet, rules, st) = rules_of(t);
    let now = st.get("now").cloned().unwrap_or(Value::Null);
    let w = st.get("while").cloned().unwrap_or(serde_json::json!({"fans": 40, "powerW": 150, "jobs": true, "downloads": true}));
    let any_on = quiet.iter().any(|q| truthy(q, "on")) || rules.iter().any(|r| truthy(r, "on"));
    let quiet_now = truthy(&now, "quiet");
    let chip = if quiet_now { "Quiet now" } else if any_on { "On" } else { "Off" };
    let rows_n = quiet.len() + rules.len() + 1;
    let on_new = a.fsel == rows_n - 1;
    let mut qb: Vec<E> = vec![];
    for (i, q) in quiet.iter().enumerate() {
        let label = rich(vec![sp(s(q, "name"), 18.0 * k, rgb(TEXT), true), sp(format!("    {} – {} · {}", s(q, "from"), s(q, "to"), s(q, "daysText")), 18.0 * k, rgb(TEXT), false)]);
        qb.push(switch(k, label, "", "", "ok", truthy(q, "on"), a.fsel == i && a.step == 0, Msg::FeatRow(i)));
    }
    if quiet.is_empty() {
        qb.push(tx("None yet: ☰ New rule › Quiet hours makes one.", 16.0 * k, rgb(MUTED)).into());
    }
    let mut chips = Row::new().spacing(8.0 * k).push(tx("While quiet:", 18.0 * k, rgb(TEXT)));
    let mut cl = vec![format!("Fans ≤ {} %", s(&w, "fans")), format!("Power ≤ {} W", s(&w, "powerW"))];
    if truthy(&w, "jobs") {
        cl.push("Pictures and dubs wait".into());
    }
    if truthy(&w, "downloads") {
        cl.push("Downloads at half speed".into());
    }
    for c in cl {
        chips = chips.push(container(tx(c, 14.0 * k, rgb(OK))).padding(Padding { top: 0.0, right: 9.0 * k, bottom: 0.0, left: 9.0 * k }).center_y(Length::Fixed(28.0 * k)).style(boxed(Color::from_rgba(0.43, 0.89, 0.6, 0.14), 6.0 * k, None)));
    }
    qb.push(chips.wrap().vertical_spacing(8.0 * k).into());
    qb.push(tx("Games you play and things you start yourself always run at full speed.", 15.0 * k, rgb(MUTED)).into());
    let mut nowl = vec![];
    if quiet_now {
        nowl.push(sp("Quiet until ", 20.0 * k, rgb(TEXT), false));
        let qu = f(&now, "quietUntil").unwrap_or(0.0) as i64;
        let m = hearth_ui::time::local(qu);
        let h = if m.hour % 12 == 0 { 12 } else { m.hour % 12 };
        nowl.push(sp(format!("{}:{:02} {}", h, m.min, if m.hour < 12 { "AM" } else { "PM" }), 20.0 * k, rgb(TEXT), true));
    } else if truthy(&now, "game") {
        nowl.push(sp("A game is running: everything at full speed", 20.0 * k, rgb(TEXT), false));
    } else {
        nowl.push(sp("Not quiet", 20.0 * k, rgb(TEXT), false));
    }
    let wait = rowsv(now.get("wait").unwrap_or(&Value::Null));
    if !wait.is_empty() {
        nowl.push(sp(format!(" · waiting: {}", wait.join(", ")), 20.0 * k, rgb(TEXT), false));
    }
    if let Some(fs) = f(&now, "fanSpeed") {
        nowl.push(sp(format!(" · fans {fs} %"), 20.0 * k, rgb(TEXT), false));
    }
    // the right column: the rules, or the new-rule picker
    let mut rb: Vec<E> = vec![];
    let title = match a.step { 1 => "New rule · pick a When", 2 => "New rule · then a Do", _ => "When this, do that" };
    if a.step == 0 {
        for (i, r) in rules.iter().enumerate() {
            let label = rich(vec![sp("When  ", 18.0 * k, rgb(TEXT), false), sp(s(r, "whenText"), 18.0 * k, rgb(TEXT), true), sp("  →  ", 18.0 * k, rgb(TEXT), false), sp(s(r, "doText"), 18.0 * k, rgb(TEXT), true)]);
            rb.push(switch(k, label, "", if truthy(r, "paused") { "Paused" } else { "" }, "warn", truthy(r, "on"), a.fsel == quiet.len() + i, Msg::FeatRow(quiet.len() + i)));
        }
        rb.push(click(
            container(tx("+  New rule: pick a When, then a Do", 18.0 * k, rgb(ORANGE)))
                .padding(Padding { top: 0.0, right: 12.0 * k, bottom: 0.0, left: if on_new { 12.0 * k } else { 0.0 } })
                .center_y(Length::Fixed(46.0 * k))
                .width(Length::Fill)
                .style(boxed(Color::TRANSPARENT, 10.0 * k, if on_new { Some((rgb(ACC), 2.5 * k)) } else { None }))
                .into(),
            Msg::FeatKey("menu".into()),
        ));
    } else {
        let list = if a.step == 1 { whens(a) } else { dos(a) };
        let picked = if a.step == 1 { a.wi } else { a.di };
        let nw = list.len();
        for (i, (_, txt)) in list.into_iter().enumerate() {
            let on = picked == i;
            let pre = if a.step == 1 && i < nw - 1 { "When  " } else if a.step == 2 { "→  " } else { "" };
            rb.push(click(
                container(tx(format!("{pre}{txt}"), 18.0 * k, rgb(TEXT)))
                    .padding(Padding { top: 0.0, right: 12.0 * k, bottom: 0.0, left: 12.0 * k })
                    .center_y(Length::Fixed(44.0 * k))
                    .width(Length::Fill)
                    .style(boxed(if on { Color::from_rgba(1.0, 0.55, 0.26, 0.14) } else { Color::TRANSPARENT }, 10.0 * k, if on { Some((rgb(ACC), 2.0 * k)) } else { None }))
                    .into(),
                Msg::PickChoice(i),
            ));
        }
        rb.push(tx("A picks · Left / Right change its number · B goes back", 14.0 * k, rgb(MUTED)).into());
    }
    column![
        head(k, "moon", "Quiet hours & rules", "Your times and your rules for this server · nothing here is on until you make it", chip, if quiet_now || any_on { rgb(OK) } else { rgb(MUTED) }, false),
        row![
            column![panel(k, "Quiet hours", qb), panel(k, "Now", vec![rich(nowl)])].spacing(20.0 * k).width(Length::FillPortion(1)),
            column![
                panel(k, title, rb),
                panel(k, "What a rule can watch and do", vec![txw("When: Jellyfin watching · a game running · a device connected · a time · a drive's heat · power use.\nDo: AI pictures wait · voices wait · downloads slower · backups wait · fans capped · tell my phone.", 15.0 * k, rgb(MUTED)).into()])
            ]
            .spacing(20.0 * k)
            .width(Length::FillPortion(1))
        ]
        .spacing(20.0 * k),
        vfill(),
        keys(k, &[("A", "Turn on or off".into(), Msg::FeatKey("enter".into())), ("X", "Edit".into(), Msg::FeatKey("x".into())), ("Y", "Pause this rule".into(), Msg::FeatKey("y".into())), ("☰", "New rule".into(), Msg::FeatKey("menu".into()))], "Back"),
    ]
    .spacing(20.0 * k)
    .padding(32.0 * k)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

// ================================================================ the AI team (AiTeamDetail.qml, AiMesh.qml)
pub struct Mesh {
    pub team: Value,
    pub k: f32,
    pub compact: bool,
    pub t: f32,
    pub still: bool,
}

fn rr(x: f32, y: f32, w: f32, h: f32, r: f32) -> Path {
    Path::rounded_rectangle(Point::new(x, y), Size::new(w, h), r.into())
}
fn ctext(f: &mut Frame, s: &str, x: f32, y: f32, size: f32, c: Color, bold: bool, center: bool) {
    f.fill_text(canvas::Text {
        content: s.to_string(),
        position: Point::new(x, y),
        color: c,
        size: size.max(4.0).into(),
        font: if bold { weight(Weight::Semibold) } else { INTER },
        align_x: if center { Alignment::Center } else { Alignment::Left },
        ..canvas::Text::default()
    });
}
fn twidth(s: &str, size: f32) -> f32 {
    s.chars().count() as f32 * size * 0.56
}

impl<M> canvas::Program<M> for Mesh {
    type State = Memo;
    fn draw(&self, _st: &Memo, renderer: &Renderer, _th: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let (w, h) = (b.width, b.height);
        let k = self.k;
        let sc = if self.compact { 0.42 } else { 1.0 };
        let (c_text, c_muted, c_dim, c_line) = (rgb(0xfff4df), rgb(0xc9a372), rgb(0x6b5a44), rgb(0x4a3212));
        let (c_chat, c_pink, c_teal) = (Color::WHITE, rgb(0xff5fae), rgb(0x4fe3c1));
        let engines = arr(&self.team, "engines");
        let senders = arr(&self.team, "senders");
        let color_of = |x: &Value| if !truthy(x, "active") { c_dim } else if matches!(s(x, "kind").as_str(), "pictures" | "voice") { c_pink } else { c_chat };
        let slots = [(0.0, -1.0), (0.62, -0.66), (0.86, 0.0), (0.62, 0.66), (0.0, 1.0), (-0.62, 0.66), (-0.86, 0.0), (-0.62, -0.66), (0.32, -0.95), (-0.32, 0.95)];
        let pos = |i: usize| { let s = slots[i % slots.len()]; Point::new(w / 2.0 + s.0 * w * 0.42, h / 2.0 + s.1 * h * 0.40) };
        let ctr = Point::new(w / 2.0, h / 2.0);
        if !self.compact {
            let g = 40.0 * k;
            let st = Stroke::default().with_color(Color::from_rgba(201.0 / 255.0, 163.0 / 255.0, 114.0 / 255.0, 0.06)).with_width(1.0);
            let mut x = 0.0;
            while x < w {
                f.stroke(&Path::line(Point::new(x, 0.0), Point::new(x, h)), st.clone());
                x += g;
            }
            let mut y = 0.0;
            while y < h {
                f.stroke(&Path::line(Point::new(0.0, y), Point::new(w, y)), st.clone());
                y += g;
            }
        }
        let cap_w = (w * 0.42).min(460.0 * k * sc * if self.compact { 2.0 } else { 1.0 });
        let cap_h = 136.0 * k * sc * if self.compact { 1.6 } else { 1.0 };
        // the warm glow under the engines
        for (i, kk) in [(1.6f32, 0.035f32), (1.2, 0.035), (0.8, 0.04)].iter().enumerate() {
            let _ = i;
            let gw = cap_w * kk.0;
            let gh = cap_h * 3.2 * kk.0 / 1.6;
            f.fill(&rr(ctr.x - gw / 2.0, ctr.y - gh / 2.0, gw, gh, gh / 2.0), Color::from_rgba(1.0, 0.55, 0.2, if self.compact { kk.1 * 1.4 } else { kk.1 }));
        }
        let lw = (k * sc * 1.4).max(0.6);
        for (i, x) in senders.iter().enumerate() {
            let p = pos(i);
            let c = color_of(x);
            let active = truthy(x, "active");
            f.stroke(&Path::line(p, ctr), Stroke::default().with_color(if active { alpha(c, 0.85) } else { Color::from_rgba(201.0 / 255.0, 163.0 / 255.0, 114.0 / 255.0, 0.18) }).with_width(if active { 2.0 } else { 1.0 } * lw));
            if active {
                // a pulse running down the line to the engines (1.4 s, in-out)
                let ph = if self.still { 0.5 } else { (self.t * 1000.0 % 1400.0) / 1400.0 };
                let e = 0.5 - 0.5 * (std::f32::consts::PI * ph).cos();
                let tt = 0.82 * e;
                let d = 8.0 * k * (sc * 1.4).max(0.6);
                f.fill(&Path::circle(Point::new(p.x + (ctr.x - p.x) * tt, p.y + (ctr.y - p.y) * tt), d / 2.0), c);
            }
        }
        // the engines' capsule
        f.fill(&rr(ctr.x - cap_w / 2.0, ctr.y - cap_h / 2.0, cap_w, cap_h, cap_h / 2.0), rgb(0x1a1008));
        f.stroke(&rr(ctr.x - cap_w / 2.0, ctr.y - cap_h / 2.0, cap_w, cap_h, cap_h / 2.0), Stroke::default().with_color(c_line).with_width(1.0));
        let n = engines.len().max(1) as f32;
        let ew = 54.0 * k * sc * if self.compact { 1.4 } else { 1.0 };
        let gap = 22.0 * k * sc;
        let slot = (cap_w - gap) / n;
        let fs = if self.compact { 1.25 } else { 1.0 };
        for (i, e) in engines.iter().enumerate() {
            let cx = ctr.x - cap_w / 2.0 + gap / 2.0 + slot * (i as f32 + 0.5);
            let st = s(e, "state");
            let ec = if st == "off" { c_dim } else if s(e, "id") == "stt" { c_teal } else if s(e, "id") == "chat" { c_chat } else { c_pink };
            let pulse = if st == "working" && !self.still { 1.0 + 0.08 * (0.5 - 0.5 * (std::f32::consts::PI * (self.t * 1000.0 % 1400.0) / 700.0).cos()) } else { 1.0 };
            let r = ew / 2.0 * pulse;
            let cy = ctr.y - cap_h * 0.16;
            f.fill(&Path::circle(Point::new(cx, cy), r), alpha(ec, if st == "working" { 0.22 } else { 0.10 }));
            f.stroke(&Path::circle(Point::new(cx, cy), r), Stroke::default().with_color(ec).with_width(if st == "off" { 1.0 } else { 2.0 }));
            // its icon
            let isz = ew * 0.48;
            let (ls, cs) = draw::icon_lines(&s(e, "icon"));
            let kk = isz / 24.0;
            let ic = if st == "off" { alpha(c_text, 0.4) } else { c_text };
            for l in ls {
                if l.len() < 2 {
                    continue;
                }
                let path = Path::new(|pb| {
                    pb.move_to(Point::new(cx - isz / 2.0 + l[0].0 * kk, cy - isz / 2.0 + l[0].1 * kk));
                    for q in &l[1..] {
                        pb.line_to(Point::new(cx - isz / 2.0 + q.0 * kk, cy - isz / 2.0 + q.1 * kk));
                    }
                });
                f.stroke(&path, Stroke::default().with_color(ic).with_width((1.6 * kk).max(0.8)));
            }
            for (x, y, rr_) in cs {
                let p = Path::circle(Point::new(cx - isz / 2.0 + x * kk, cy - isz / 2.0 + y * kk), rr_ * kk);
                if rr_ < 2.0 { f.fill(&p, ic) } else { f.stroke(&p, Stroke::default().with_color(ic).with_width((1.6 * kk).max(0.8))) }
            }
            ctext(&mut f, &s(e, "name"), cx, cy + ew / 2.0 + 4.0 * k * sc, 13.0 * k * sc * fs, c_text, true, true);
            ctext(&mut f, &s(e, "status"), cx, cy + ew / 2.0 + 4.0 * k * sc + 16.0 * k * sc * fs, 11.5 * k * sc * fs, c_muted, false, true);
        }
        // the senders
        let fz = if self.compact { 1.3 } else { 1.0 };
        for (i, x) in senders.iter().enumerate() {
            let p = pos(i);
            let c = color_of(x);
            let active = truthy(x, "active");
            let name = s(x, "name");
            let cnt = s(x, "n");
            let last = s(x, "last");
            let ns = 13.0 * k * sc * fz;
            let ls = 11.0 * k * sc * fz;
            let bw = (twidth(&name, ns) + twidth(&cnt, ns) + 30.0 * k * sc).max(twidth(&last, ls) + 22.0 * k * sc);
            let bh = if last.is_empty() { 30.0 } else { 46.0 } * k * sc * fz;
            let op = if active || crate::f(x, "n").unwrap_or(0.0) > 0.0 { 1.0 } else { 0.55 };
            let path = rr(p.x - bw / 2.0, p.y - bh / 2.0, bw, bh, 8.0 * k * sc);
            f.fill(&path, alpha(if active { rgb(0x1e130a) } else { rgb(0x140c05) }, op));
            f.stroke(&path, Stroke::default().with_color(alpha(if active { c } else { Color::from_rgba(201.0 / 255.0, 163.0 / 255.0, 114.0 / 255.0, 0.22) }, op)).with_width(if active { 2.0 } else { 1.0 }));
            let x0 = p.x - bw / 2.0 + 11.0 * k * sc;
            let y0 = p.y - bh / 2.0 + 7.0 * k * sc;
            ctext(&mut f, &name, x0, y0, ns, alpha(c_text, op), true, false);
            f.fill_text(canvas::Text {
                content: cnt,
                position: Point::new(p.x + bw / 2.0 - 11.0 * k * sc, y0),
                color: alpha(if active { c } else { c_muted }, op),
                size: ns.max(4.0).into(),
                font: weight(Weight::Semibold),
                align_x: Alignment::Right,
                ..canvas::Text::default()
            });
            if !last.is_empty() {
                ctext(&mut f, &last, x0, y0 + ns * 1.3, ls, alpha(c_muted, op), false, false);
            }
        }
        if !self.compact && senders.is_empty() {
            ctext(&mut f, "Nobody has sent the AI team work today. Your devices reach it through One AI address.", w / 2.0, h - 36.0 * k, 14.0 * k, c_muted, false, true);
        }
        vec![f.into_geometry()]
    }
}

pub fn mesh<'a>(a: &App, k: f32, compact: bool) -> E<'a> {
    let team = a.d.as_ref().and_then(|d| d.get("team")).cloned().unwrap_or(Value::Null);
    draw::canvas(Mesh { team, k, compact, t: a.anim_t, still: a.still() }, Length::Fill, Length::Fill)
}

pub fn team_page<'a>(a: &'a App, t: &Value, k: f32) -> E<'a> {
    let team = a.d.as_ref().and_then(|d| d.get("team")).cloned().unwrap_or(Value::Null);
    let paused = s(t, "state") == "paused-you";
    let today = team.get("today").cloned().unwrap_or(Value::Null);
    let (c_text, c_muted, c_ok) = (rgb(0xfff4df), rgb(0xc9a372), rgb(0x6fe39a));
    let chip_c = if paused { rgb(0xff8a4c) } else if s(&team, "state") == "off" { c_muted } else { c_ok };
    let line = |a_: String, b_: String, bc: Color| -> E<'static> {
        row![container(tx(a_, 15.0 * k, c_text)).width(Length::Fill).clip(true), container(txw(b_, 15.0 * k, bc).align_x(iced::alignment::Horizontal::Right)).max_width(200.0 * k)].spacing(12.0 * k).into()
    };
    let tpanel = |title: &str, rows: Vec<E<'static>>| -> E<'static> {
        let mut c = Column::new().spacing(10.0 * k).push(txs(title.to_string(), 18.0 * k, c_text));
        for r in rows {
            c = c.push(r);
        }
        container(c).padding(20.0 * k).width(Length::Fill).style(boxed(rgb(0x140c03), 14.0 * k, Some((rgb(0x4a3212), 1.0)))).into()
    };
    let sent = arr(&today, "sent");
    let sent_t = if sent.is_empty() { "none".to_string() } else { sent.iter().map(|r| { let v = rowsv(r); format!("{} · {} ({})", v.get(1).cloned().unwrap_or_default(), v.first().cloned().unwrap_or_default(), v.get(2).cloned().unwrap_or_default()) }).collect::<Vec<_>>().join("\n") };
    let busiest = today.get("busiest").map(rowsv).filter(|v| v.len() >= 2).map(|v| format!("{} · {}", v[0], v[1])).unwrap_or_else(|| "—".into());
    let today_p = tpanel("Today", vec![
        line("Requests".into(), s(&today, "requests").pipe_or("0"), c_muted),
        line("Answered here".into(), s(&today, "here").pipe_or("0"), c_muted),
        line("Sent on to another device".into(), sent_t, c_muted),
        line("Busiest".into(), busiest, c_muted),
    ]);
    let mut eng: Vec<E<'static>> = vec![];
    for e in arr(&team, "engines") {
        let id = s(&e, "id");
        let st = s(&e, "state");
        let a_ = format!("{} · {}", s(&e, "name"), if id == "chat" { s(&e, "model").pipe_or("no model") } else { s(&e, "engine") });
        let b_ = if st == "off" { "off".into() } else if id == "chat" { if s(&e, "model").is_empty() { "ready".into() } else { "loaded".into() } } else if id == "voices" && st == "working" { "speaking".into() } else if id == "pictures" { s(&e, "status") } else { "ready".to_string() };
        let bc = if st != "off" && (b_ == "loaded" || b_ == "speaking") { c_ok } else { c_muted };
        eng.push(line(a_, b_, bc));
    }
    let play: Vec<E<'static>> = arr(&team, "play").iter().map(|r| { let v = rowsv(r); line(v.first().cloned().unwrap_or_default(), v.get(1).cloned().unwrap_or_default(), c_muted) }).collect();
    let btns = row![
        crate::view::sbutton(k, if paused { "Resume the team" } else { "Pause the team" }, "A", true, Msg::DetailAct("pause".into())),
        crate::view::sbutton(k, "Restart", "X", false, Msg::DetailAct("restart".into())),
        crate::view::sbutton(k, "Settings", "Y", false, Msg::DetailAct("settings-team".into())),
        fill(),
        click(tx("B  Back to the dashboard", 15.0 * k, c_muted).into(), Msg::Close)
    ]
    .spacing(12.0 * k)
    .align_y(iced::Center);
    let mut foot = Column::new().spacing(6.0 * k).push(btns);
    if !a.toast.is_empty() {
        foot = foot.push(tx(a.toast.clone(), 15.0 * k, rgb(ACC)));
    }
    column![
        row![
            container(icon("sparkle", rgb(ACC), 28.0 * k)).center_x(Length::Fixed(54.0 * k)).center_y(Length::Fixed(54.0 * k)).style(boxed(Color::from_rgba(1.0, 0.7, 0.28, 0.16), 12.0 * k, None)),
            column![txs("AI team", 34.0 * k, c_text), tx(format!("This PC's AI, for all your devices · One AI address: {}", s(&team, "address")), 15.0 * k, c_muted)].spacing(2.0),
            container(txs(if paused { "Paused".to_string() } else { s(&team, "chip") }, 15.0 * k, chip_c)).padding(Padding { top: 0.0, right: 10.0 * k, bottom: 0.0, left: 10.0 * k }).center_y(Length::Fixed(30.0 * k)).style(boxed(alpha(chip_c, 0.16), 8.0 * k, None)),
            fill()
        ]
        .spacing(18.0 * k)
        .align_y(iced::alignment::Vertical::Top),
        row![
            container(mesh(a, k, false)).padding(10.0 * k).width(Length::Fill).height(Length::Fill).clip(true).style(boxed(rgb(0x0c0702), 14.0 * k, Some((rgb(0x4a3212), 1.0)))),
            column![today_p, tpanel("Engines", eng), tpanel("While you play", play), vfill()].spacing(22.0 * k).width(Length::Fixed(300.0 * k)).height(Length::Fill)
        ]
        .spacing(22.0 * k)
        .height(Length::Fill),
        foot
    ]
    .spacing(18.0 * k)
    .padding(32.0 * k)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

trait PipeOr {
    fn pipe_or(self, d: &str) -> String;
}
impl PipeOr for String {
    fn pipe_or(self, d: &str) -> String {
        if self.is_empty() { d.to_string() } else { self }
    }
}

// ================================================================ Ithil (IthilDetail.qml)
pub fn brain_widget<'a>(a: &App, compact: bool) -> E<'a> {
    let it = a.d.as_ref().and_then(|d| d.get("ithil")).cloned().unwrap_or(Value::Null);
    let live = compact || a.ithil_win == "live";
    let b = brain::Brain {
        data: a.ithil_prep.clone(),
        yaw: a.yaw + if a.still() { 0.0 } else { a.anim_t * 0.18 },
        pitch: a.pitch,
        zoom: if compact { 1.05 } else { 1.0 },
        xray: a.xray && !compact,
        live,
        asleep: s(&it, "state") == "asleep",
        t: a.tpos,
        now_ms: if a.still() { 0.0 } else { a.now * 1000.0 },
        step: if compact { 3 } else { 1 },
    };
    let c: E = draw::canvas(b, Length::Fill, Length::Fill);
    if compact { c } else { mouse_area(c).on_move(Msg::BrainMove).on_exit(Msg::BrainLeave).into() }
}

pub fn ithil_page<'a>(a: &'a App, k: f32) -> E<'a> {
    let it = a.d.as_ref().and_then(|d| d.get("ithil")).cloned().unwrap_or(Value::Null);
    let today = it.get("today").cloned().unwrap_or(Value::Null);
    let (c_text, c_muted, c_ok, c_acc, c_bad) = (rgb(0xfff4df), rgb(0xc9a372), rgb(0x6fe39a), rgb(ACC), rgb(0xf0a8a8));
    let chip = |t: String, c: Color| -> E<'static> { container(txs(t, 13.0 * k, c)).padding(Padding { top: 0.0, right: 9.0 * k, bottom: 0.0, left: 9.0 * k }).center_y(Length::Fixed(26.0 * k)).style(boxed(alpha(c, 0.16), 7.0 * k, None)).into() };
    let pill = |t: &str, on: bool| -> E<'static> {
        container(tx(t.to_string(), 14.0 * k, c_text)).padding(Padding { top: 0.0, right: 13.0 * k, bottom: 0.0, left: 13.0 * k }).center_y(Length::Fixed(32.0 * k))
            .style(boxed(if on { Color::from_rgba(1.0, 0.7, 0.28, 0.2) } else { rgb(0x1c140a) }, 16.0 * k, Some((if on { c_acc } else { rgb(0x4a3212) }, 1.0)))).into()
    };
    let ipanel = |title: &str, rows: Vec<E<'static>>| -> E<'static> {
        let mut c = Column::new().spacing(8.0 * k).push(txs(title.to_uppercase(), 15.0 * k, c_muted));
        for r in rows {
            c = c.push(r);
        }
        container(c).padding(20.0 * k).width(Length::Fill).style(boxed(rgb(0x140c03), 14.0 * k, Some((rgb(0x4a3212), 1.0)))).into()
    };
    let gauge = |l: &str, v: f32| -> E<'static> {
        row![
            container(tx(l.to_string(), 15.0 * k, c_text)).width(Length::Fixed(100.0 * k)),
            hearth_ui::ui::bar(vec![(v.clamp(0.0, 1.0), rgb(0xf472b6))], rgb(0x0c0702), 8.0 * k, 4.0 * k),
            container(tx(format!("{:.2}", v).trim_start_matches('0').to_string(), 15.0 * k, c_muted)).align_right(Length::Fixed(40.0 * k))
        ].spacing(12.0 * k).align_y(iced::Center).into()
    };
    let needs = it.get("needs").cloned().unwrap_or(Value::Null);
    let nv = |kk: &str| f(&needs, kk).unwrap_or(0.0) as f32;
    let mut mood = vec![sp("Mood ", 16.0 * k, c_muted, false), sp(s(&it, "mood").pipe_or("—"), 16.0 * k, c_text, false)];
    let focus = s(&it, "focus");
    if !focus.is_empty() {
        mood.push(sp("  ·  Focused on ", 16.0 * k, c_muted, false));
        mood.push(sp(focus, 16.0 * k, c_text, false));
    }
    let talks = f(&today, "talks").unwrap_or(0.0) as i64;
    let tgrid = column![
        row![rich(vec![sp(talks.to_string(), 22.0 * k, c_text, true), sp(format!(" {} with you", if talks == 1 { "talk" } else { "talks" }), 16.0 * k, c_text, false)]), fill(), rich(vec![sp(s(&today, "memories").pipe_or("0"), 22.0 * k, c_text, true), sp(" memories kept", 16.0 * k, c_text, false)])],
        row![rich(vec![sp(s(&today, "blocks").pipe_or("0"), 22.0 * k, c_text, true), sp(" blocked by her gate", 16.0 * k, c_text, false)]), fill(), rich(vec![sp(s(&it, "sleepTime").pipe_or("—"), 22.0 * k, c_text, true), sp(" she sleeps", 16.0 * k, c_text, false)])]
    ].spacing(6.0 * k);
    let mut reach: Vec<E<'static>> = vec![];
    for r in arr(&it, "reach") {
        let via = s(&r, "via");
        let name = if !via.is_empty() && via != "Tailscale" { format!("{} ({via})", s(&r, "name")) } else { s(&r, "name") };
        let c = if truthy(&r, "never") { c_bad } else if truthy(&r, "ok") { c_ok } else { c_muted };
        reach.push(row![container(tx(name, 16.0 * k, c_text)).width(Length::Fill).clip(true), chip(s(&r, "chip"), c)].align_y(iced::Center).into());
    }
    let loaded = truthy(&it, "brainLoaded");
    let live = a.ithil_win == "live";
    let sleepy = s(&it, "state") == "asleep" || (!live && brain::sleepiness(&a.ithil_prep, a.tpos) > 0.5);
    let mut pills = Row::new().spacing(10.0 * k).push(click(pill("● Live", live), Msg::IthilWin("live".into()))).push(click(pill("Last 24 hours", a.ithil_win == "24"), Msg::IthilWin("24".into()))).push(click(pill("Her whole life", a.ithil_win == "all"), Msg::IthilWin("all".into())));
    if a.xray {
        pills = pills.push(pill("X-ray", true));
    }
    let when = if !live && a.ithil_prep.to > 0.0 {
        let ms = a.ithil_prep.from + (a.tpos as f64) / 120.0 * (a.ithil_prep.to - a.ithil_prep.from);
        let m = hearth_ui::time::local((ms / 1000.0) as i64);
        format!(" · {} {:02}:{:02}", ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][m.wday], m.hour, m.min)
    } else {
        String::new()
    };
    let progress = if live { 1.0 } else { a.tpos / 120.0 };
    let brain_box = stack![
        container(Space::new()).width(Length::Fill).height(Length::Fill).style(boxed(rgb(0x1e1820), 14.0 * k, Some((rgb(0x4a3212), 1.0)))),
        container(brain_widget(a, false)).padding(Padding { top: 0.0, right: 0.0, bottom: 70.0 * k, left: 0.0 }).width(Length::Fill).height(Length::Fill),
        container(row![pills, fill(), pill(&format!("{}{}", if sleepy { "☾ Asleep" } else { "Awake" }, if live { " · live" } else { "" }), false)]).padding(16.0 * k).width(Length::Fill),
        container(column![
            draw::canvas(brain::Strip { data: a.ithil_prep.clone() }, Length::Fill, Length::Fixed(14.0 * k)),
            hearth_ui::ui::bar(vec![(progress, c_acc)], rgb(0x3a2a19), 6.0 * k, 3.0 * k),
            tx(format!("Ⓛ Ⓡ move through time (the 4th dimension) · right stick turns her brain · Ⓥ X-ray{when}"), 14.0 * k, c_muted)
        ].spacing(6.0 * k)).padding(16.0 * k).align_bottom(Length::Fill).width(Length::Fill)
    ]
    .width(Length::FillPortion(135))
    .height(Length::Fill);
    let right = column![
        ipanel("Right now", vec![rich(mood), gauge("Energy", nv("energy")), gauge("Social", nv("social")), gauge("Curiosity", nv("curiosity")), gauge("Stability", nv("stability"))]),
        ipanel("Today", vec![tgrid.into()]),
        ipanel("Her brain on this PC", vec![
            row![tx(s(&it, "brainModel").pipe_or("—"), 16.0 * k, c_text), chip(if loaded { "Loaded".into() } else { "Not loaded".into() }, if loaded { c_ok } else { c_muted })].spacing(10.0 * k).align_y(iced::Center).into(),
            txw("Stays loaded in Server mode. While you play, it steps back and she rests; she's back when you stop. Pausing the AI team asks first, because her brain is in it.", 14.0 * k, c_muted).into(),
        ]),
        ipanel("Reach her from", reach),
        vfill()
    ]
    .spacing(18.0 * k)
    .width(Length::FillPortion(100))
    .height(Length::Fill);
    let name = s(&it, "name").pipe_or("Ithil");
    let ver = s(&it, "version");
    let head = row![
        container(icon("moon", rgb(0xc4b5fd), 30.0 * k)).center_x(Length::Fixed(56.0 * k)).center_y(Length::Fixed(56.0 * k)).style(boxed(rgb(0x22193a), 12.0 * k, None)),
        column![
            txs(name, 34.0 * k, c_text),
            rich(vec![sp(format!("Her own mind on this PC{} · reach her at ", if ver.is_empty() { String::new() } else { format!(" · Hearth AI {ver}") }), 16.0 * k, c_muted, false), sp(s(&it, "address"), 16.0 * k, c_text, true), sp(" (Tailscale)", 16.0 * k, c_muted, false)])
        ].spacing(2.0).width(Length::Fill),
        chip(s(&it, "chip"), if s(&it, "state") == "off" { c_muted } else { c_ok })
    ]
    .spacing(18.0 * k)
    .align_y(iced::Center);
    let k_ = |l: &str, t: &str, m: Msg| click(tx(format!("{l} {t}"), 16.0 * k, c_text).into(), m);
    let foot = row![
        k_("Ⓐ", "Talk to her", Msg::DetailAct("talk".into())),
        k_("Ⓧ", "Restart her app", Msg::DetailAct("ithil-restart".into())),
        k_("Ⓨ", if a.sleep_armed { "Press Y again to put her to sleep" } else { "Put her to sleep (asks twice)" }, Msg::DetailAct("ithil-sleep".into())),
        k_("☰", "Her settings", Msg::DetailAct("ithil-settings".into())),
        fill(),
        click(tx("Ⓑ Back to the dashboard", 16.0 * k, c_text).into(), Msg::Close)
    ]
    .spacing(30.0 * k);
    let mut c = column![head, row![brain_box, right].spacing(18.0 * k).height(Length::Fill), foot].spacing(18.0 * k).padding(30.0 * k).width(Length::Fill).height(Length::Fill);
    if !a.toast.is_empty() {
        c = c.push(tx(a.toast.clone(), 15.0 * k, c_acc));
    }
    c.into()
}

/// Ithil's tile, two wide: her small live brain (ServerTile.qml's IthilBrain, compact).
pub fn tile_brain<'a>(a: &App, k: f32) -> E<'a> {
    stack![
        container(Space::new()).width(Length::Fill).height(Length::Fill).style(boxed(rgb(0x1e1820), 10.0 * k, None)),
        brain_widget(a, true)
    ]
    .width(Length::FillPortion(1))
    .height(Length::Fill)
    .into()
}


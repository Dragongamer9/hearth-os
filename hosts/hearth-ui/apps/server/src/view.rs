//! What the dashboard draws (ServerDash.qml, ServerTile.qml, ServerDetail.qml, ServerBack.qml, ServerLine.qml,
//! ServerButton.qml), in Server mode's gold look (the Hearth AI Command Center's), sized like Mantel's: k is the screen
//! against 1920 × 1080.

use crate::draw::{self, a as alpha, icon, rgb};
use crate::{arr, f, s, truthy, App, Msg, Overlay, Screen, COLS};
use hearth_ui::app::INTER;
use hearth_ui::ui::weight;
use iced::font::Weight;
use iced::widget::text::{Rich, Span, Wrapping};
use iced::widget::{column, container, mouse_area, row, stack, text, Column, Row, Space};
use iced::{mouse, Color, Element, Font, Length, Padding};
use serde_json::Value;

type E<'a> = Element<'a, Msg>;

struct G;
#[allow(dead_code)]
impl G {
    fn bg() -> Color { rgb(0x030201) }
    fn panel() -> Color { rgb(0x0c0702) }
    fn panel2() -> Color { rgb(0x140c03) }
    fn line() -> Color { rgb(0x2b1c08) }
    fn line2() -> Color { rgb(0x4a3212) }
    fn text() -> Color { rgb(0xffe9c7) }
    fn hi() -> Color { rgb(0xfff4df) }
    fn muted() -> Color { rgb(0xc9a372) }
    fn faint() -> Color { rgb(0x8a6a3e) }
    fn acc() -> Color { rgb(0xffb347) }
    fn ink() -> Color { rgb(0x140c03) }
    fn ok() -> Color { rgb(0x6fe39a) }
    fn warn() -> Color { rgb(0xff8a4c) }
}

fn boxed(bg: Color, r: f32, line: Option<(Color, f32)>) -> impl Fn(&iced::Theme) -> container::Style {
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
    Span::new(v.into()).size(size).color(c).font(if bold { weight(Weight::Semibold) } else { INTER })
}
fn rich<'a>(v: Vec<Span<'a, (), Font>>) -> E<'a> {
    Rich::<'a, (), Msg>::from_iter(v).into()
}
fn gap<'a>(w: f32, h: f32) -> E<'a> {
    Space::new().width(Length::Fixed(w)).height(Length::Fixed(h)).into()
}
fn fill<'a>() -> E<'a> {
    Space::new().width(Length::Fill).into()
}
fn vfill<'a>() -> E<'a> {
    Space::new().height(Length::Fill).into()
}
fn hline<'a>(c: Color) -> E<'a> {
    container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(boxed(c, 0.0, None)).into()
}
fn dot<'a>(c: Color, d: f32) -> E<'a> {
    container(Space::new()).width(Length::Fixed(d)).height(Length::Fixed(d)).style(boxed(c, d / 2.0, None)).into()
}
fn click<'a>(e: E<'a>, m: Msg) -> E<'a> {
    mouse_area(e).on_press(m).interaction(mouse::Interaction::Pointer).into()
}
fn icon_box<'a>(name: &str, size: f32, r: f32, bg: Color, fg: Color, isz: f32) -> E<'a> {
    container(icon(name, fg, isz)).center_x(Length::Fixed(size)).center_y(Length::Fixed(size)).style(boxed(bg, r, None)).into()
}

const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const LDAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const LMONS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

fn clock(t: f64, ap: bool) -> String {
    let m = hearth_ui::time::local(t as i64);
    let h = if m.hour % 12 == 0 { 12 } else { m.hour % 12 };
    if ap { format!("{}:{:02} {}", h, m.min, if m.hour < 12 { "AM" } else { "PM" }) } else { format!("{}:{:02}", h, m.min) }
}

/// A panel with its title and the gold corner marks (GPanel).
fn panel<'a>(k: f32, title: &str, inner: E<'a>, h: Length) -> E<'a> {
    let body = container(
        column![
            txs(format!("◢ {}", title.to_uppercase()), 12.5 * k, G::muted()),
            container(inner).width(Length::Fill).height(Length::Fill).padding(Padding { top: 10.0 * k, ..Padding::ZERO })
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .padding(Padding { top: 14.0 * k, right: 18.0 * k, bottom: 14.0 * k, left: 18.0 * k })
    .width(Length::Fill)
    .height(h)
    .style(boxed(G::panel(), 16.0 * k, Some((G::line(), 1.0))));
    stack![body, draw::canvas(draw::Corners { color: G::acc(), len: 16.0 * k, w: 2.0 * k }, Length::Fill, Length::Fill)]
        .width(Length::Fill)
        .height(h)
        .into()
}

pub fn view(a: &App) -> E<'_> {
    let k = (a.size.width / 1920.0).min(a.size.height / 1080.0).max(0.55);
    let d = a.d.clone().unwrap_or(Value::Null);
    if a.screen != Screen::On {
        return idle(a, &d, k);
    }

    // ---- top line: Server mode, the PC's name, Tailscale and the web page, the date and the clock
    let tiles = a.tiles();
    let host = { let h = s(&d, "host"); if h.is_empty() { "Hearth".to_string() } else { h } };
    let ts_on = tiles.iter().any(|t| s(t, "id") == "who" && s(t, "state") == "running");
    let web_on = d.get("web").map(|w| truthy(w, "on")).unwrap_or(false);
    let pill = |name: &str, on: bool| -> E<'static> {
        container(tx(format!("{name} · {}", if on { "on" } else { "off" }), 14.0 * k, if on { G::ok() } else { G::muted() }))
            .padding(Padding { top: 0.0, right: 13.0 * k, bottom: 0.0, left: 13.0 * k })
            .center_y(Length::Fixed(30.0 * k))
            .style(boxed(Color::TRANSPARENT, 15.0 * k, Some((if on { alpha(G::ok(), 0.45) } else { G::line2() }, 1.0))))
            .into()
    };
    let lt = hearth_ui::time::local(a.now as i64);
    let top = column![
        container(
            row![
                icon_box("server", 42.0 * k, 11.0 * k, rgb(0x2b1a08), G::acc(), 22.0 * k),
                txs("◢ SERVER MODE", 14.0 * k, G::acc()),
                txs(host.to_uppercase(), 24.0 * k, G::hi()),
                fill(),
                pill("Tailscale", ts_on),
                pill("Web page", web_on),
                tx(format!("{} {} {}", DAYS[lt.wday], lt.day, MONS[lt.mon]), 17.0 * k, G::muted()),
                txs(clock(a.now, true), 23.0 * k, G::hi()),
            ]
            .spacing(16.0 * k)
            .align_y(iced::Center)
        )
        .padding(Padding { top: 0.0, right: 36.0 * k, bottom: 0.0, left: 36.0 * k })
        .center_y(Length::Fixed(78.0 * k - 1.0))
        .width(Length::Fill),
        hline(G::line())
    ];

    let body = row![left_column(a, &d, k), right_column(a, &d, k)]
        .spacing(22.0 * k)
        .padding(Padding { top: 14.0 * k, right: 36.0 * k, bottom: 12.0 * k, left: 36.0 * k })
        .height(Length::Fill);

    // ---- the buttons along the bottom
    let hint = |kk: &str, label: &str| -> E<'static> {
        row![
            container(txb(kk.to_string(), 12.0 * k, G::hi())).center_x(Length::Fixed(26.0 * k)).center_y(Length::Fixed(26.0 * k)).style(boxed(Color::TRANSPARENT, 13.0 * k, Some((G::muted(), 1.0)))),
            tx(label.to_string(), 15.0 * k, G::text())
        ]
        .spacing(8.0 * k)
        .align_y(iced::Center)
        .into()
    };
    let hints = column![
        hline(G::line()),
        container(
            row![
                hint("A", "Open"),
                hint("X", "Restart service"),
                hint("Y", "Pause or resume"),
                hint("☰", "Menu"),
                fill(),
                tx(a.toast.clone(), 15.0 * k, G::acc()),
                fill(),
                tx("Guide  Play (Game Mode)", 15.0 * k, G::muted()),
            ]
            .spacing(26.0 * k)
            .align_y(iced::Center)
        )
        .padding(Padding { top: 0.0, right: 36.0 * k, bottom: 0.0, left: 36.0 * k })
        .center_y(Length::Fixed(48.0 * k - 1.0))
        .width(Length::Fill)
        .style(boxed(Color::from_rgba(0.008, 0.004, 0.0, 0.92), 0.0, None))
    ];

    let page = column![top, body, hints].width(Length::Fill).height(Length::Fill);
    let base: E = stack![
        container(Space::new()).width(Length::Fill).height(Length::Fill).style(boxed(G::bg(), 0.0, None)),
        draw::canvas(draw::Dots { step: 24.0 * k, color: Color::from_rgba(1.0, 0.70, 0.28, 0.07) }, Length::Fill, Length::Fill),
        page
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into();

    // ---- overlays
    let scrim = || -> E<'static> { container(Space::new()).width(Length::Fill).height(Length::Fill).style(boxed(Color::from_rgba(0.012, 0.008, 0.004, 0.82), 0.0, None)).into() };
    fn centred(e: E<'_>) -> E<'_> {
        container(e).center(Length::Fill).into()
    }
    if a.remote.is_some() || a.overlay == Overlay::Switch {
        return stack![base, scrim(), centred(switch(a, &d, k))].into();
    }
    match a.overlay {
        Overlay::Detail => match a.cur() {
            Some(t) => {
                let id = s(&t, "id");
                let page: E = match id.as_str() {
                    "team" => crate::pages::team_page(a, &t, k),
                    "ithil" => crate::pages::ithil_page(a, k),
                    "cache" => crate::pages::cache_page(a, &t, k),
                    "ads" => crate::pages::ads_page(a, &t, k),
                    "rules" => crate::pages::rules_page(a, &t, k),
                    _ => return stack![base, click(scrim(), Msg::Close), centred(detail(a, &d, &t, k))].into(),
                };
                let mut boxc = Column::new().push(page);
                if matches!(id.as_str(), "cache" | "ads" | "rules") && !a.toast.is_empty() {
                    boxc = boxc.push(container(tx(a.toast.clone(), 15.0 * k, G::acc())).padding(Padding { top: 0.0, right: 32.0 * k, bottom: 16.0 * k, left: 32.0 * k }));
                }
                let card = container(boxc).width(Length::Fixed(1620.0 * k)).height(Length::Fixed(860.0 * k)).clip(true).style(boxed(G::panel(), 22.0 * k, Some((G::line(), 1.0))));
                stack![base, click(scrim(), Msg::Close), centred(card.into())].into()
            }
            None => base,
        },
        Overlay::Menu => stack![base, click(scrim(), Msg::Close), centred(menu(a, k))].into(),
        Overlay::Back => {
            let card = back_panel(a, &d, k);
            stack![base, container(card).align_right(Length::Fill).padding(Padding { top: (78.0 + 14.0) * k, right: 48.0 * k, bottom: 0.0, left: 0.0 })].into()
        }
        _ => base,
    }
}

// ---------------------------------------------------------------- the left column: this PC, the AIs, the services
fn left_column<'a>(a: &'a App, d: &Value, k: f32) -> E<'a> {
    let status = d.get("status").cloned().unwrap_or(Value::Null);
    let sc = if a.d.is_some() && s(&status, "level") != "ok" { G::warn() } else { G::ok() };
    let title = if a.back_info.is_some() && a.overlay == Overlay::Back { "Coming back".to_string() } else if a.d.is_some() { s(&status, "title") } else { "Starting…".into() };
    let mut devs: Vec<Span<()>> = vec![];
    if a.d.is_some() {
        let n = f(d, "devices").unwrap_or(0.0) as i64;
        devs.push(sp(format!("{n} {}", if n == 1 { "device" } else { "devices" }), 16.0 * k, G::muted(), true));
        devs.push(sp(" connected", 16.0 * k, G::muted(), false));
        let up = f(d, "uptime").unwrap_or(0.0);
        if up >= 86400.0 {
            devs.push(sp(" · Up ", 16.0 * k, G::muted(), false));
            devs.push(sp(format!("{} days", (up / 86400.0).floor() as i64), 16.0 * k, G::muted(), true));
        }
        devs.push(sp(" · Server mode since ", 16.0 * k, G::muted(), false));
        devs.push(sp(s(d, "sinceText"), 16.0 * k, G::muted(), true));
    }
    let facts = arr(d, "facts");
    let fact_of = |i: usize, def: &str| -> (String, String) {
        facts.get(i).and_then(Value::as_array).map(|x| (x.first().and_then(Value::as_str).unwrap_or(def).to_string(), x.get(1).and_then(Value::as_str).unwrap_or("—").to_string())).unwrap_or((def.into(), "—".into()))
    };
    let live = d.get("live").cloned().unwrap_or(Value::Null);
    let gpu_sub = if live.is_null() {
        String::new()
    } else {
        let mut x = { let g = s(d, "gpuName"); if g.is_empty() { "Graphics card".into() } else { g } };
        if let Some(b) = f(&live, "gpuBusy") { x += &format!(" · {}% busy", b.round()); }
        if let Some(t) = f(&live, "gpuTemp").filter(|t| *t > 0.0) { x += &format!(" · {} °C", t.round()); }
        x
    };
    let st_sub = f(&live, "storage").map(|v| format!("{v}% used")).unwrap_or_default();
    let cpu_sub = if live.is_null() { String::new() } else {
        format!("processor {}{}", f(&live, "cpuTemp").filter(|t| *t > 0.0).map(|t| format!("{t} °C · ")).unwrap_or_default(), f(&live, "cpu").map(|c| format!("{}% busy", c.round())).unwrap_or_default())
    };
    let mbps = |v: Option<f64>| v.map(|b| format!("{:.1}", b / 1e6)).unwrap_or_else(|| "—".into());
    let net_v = if live.is_null() { "—".into() } else { format!("↓{} ↑{} MB/s", mbps(f(&live, "rx")), mbps(f(&live, "tx"))) };
    let net_sub = if live.is_null() { String::new() } else { format!("{} online on Tailscale", s(&live, "online")) };
    let (l0, v0) = fact_of(0, "Graphics memory");
    let (l1, v1) = fact_of(1, "Storage");
    let (l2, v2) = fact_of(2, "Power");
    let fact_row = row![
        fact(k, l0, v0, gpu_sub, a.hist[0].clone(), rgb(0xe8d66e)),
        fact(k, l1, v1, st_sub, a.hist[1].clone(), G::acc()),
        fact(k, l2, v2, cpu_sub, a.hist[2].clone(), G::warn()),
        fact(k, "Network".into(), net_v, net_sub, a.hist[3].clone(), rgb(0x7fb2ff)),
    ]
    .spacing(12.0 * k)
    .height(Length::Fill);
    let this_pc = panel(
        k,
        &format!("This PC · {}", s(d, "host")),
        column![
            row![
                txs(title, 34.0 * k, sc),
                tx(if a.d.is_some() { format!("· {}", s(&status, "sub")) } else { String::new() }, 21.0 * k, G::muted()),
                fill(),
                rich(devs)
            ]
            .spacing(12.0 * k)
            .align_y(iced::Center),
            fact_row
        ]
        .spacing(10.0 * k)
        .into(),
        Length::Fixed(178.0 * k),
    );

    let mut col = Column::new().spacing(12.0 * k).width(Length::Fill).height(Length::Fill).push(this_pc);
    let cores = arr(d, "cores");
    let mut used = 178.0 * k;
    if !cores.is_empty() {
        let mut crow = Row::new().height(Length::Fill);
        for c in &cores {
            let id = s(c, "id");
            let st = s(c, "state");
            let working = st == "working";
            let name_c = if working { draw::core_color(&id) } else { G::hi() };
            crow = crow.push(
                column![
                    draw::canvas(draw::Core { kind: id.clone(), working, off: st == "off", fade: 1.0, t: a.anim_t, still: a.still() || (a.light() && !working) }, Length::Fixed(110.0 * k), Length::Fixed(100.0 * k)),
                    txs(s(c, "name").to_uppercase(), 14.0 * k, name_c),
                    tx(s(c, "status"), 13.5 * k, G::muted())
                ]
                .spacing(2.0 * k)
                .align_x(iced::Center)
                .width(Length::FillPortion(1)),
            );
        }
        let talker = cores.iter().find(|c| s(c, "mode") == "working" || s(c, "state") == "working").cloned();
        let asking: Vec<String> = arr(d, "who").iter().filter(|w| arr(w, "what").iter().any(|x| x.as_str() == Some("AI chat"))).map(|w| s(w, "name")).collect();
        let mut trow = Row::new().spacing(12.0 * k).align_y(iced::Center);
        match &talker {
            Some(t) => {
                let mut bars = Row::new().spacing(3.0 * k).align_y(iced::Center);
                for h in [0.7f32, 0.5, 0.8, 0.6, 0.9] {
                    bars = bars.push(container(Space::new()).width(Length::Fixed(4.0 * k)).height(Length::Fixed(22.0 * k * h)).style(boxed(rgb(0xe8d66e), 2.0, None)));
                }
                trow = trow.push(bars);
                let what = if s(t, "id") == "gpu" && !asking.is_empty() { format!("answering {}", asking.join(", ")) } else { s(t, "status") };
                trow = trow.push(rich(vec![sp(s(t, "name").to_uppercase(), 16.0 * k, G::hi(), true), sp(format!("  {what}"), 16.0 * k, G::hi(), false)]));
            }
            None => trow = trow.push(tx("All quiet: nobody's asking right now", 16.0 * k, G::muted())),
        }
        col = col.push(panel(
            k,
            "The AIs on this PC · the same as in your Command Center",
            column![crow, hline(G::line()), trow].spacing(6.0 * k).into(),
            Length::Fixed(214.0 * k),
        ));
        used += 214.0 * k + 12.0 * k;
    }
    col.push(services(a, k, used)).into()
}

fn fact<'a>(k: f32, label: String, value: String, sub: String, pts: Vec<f32>, line: Color) -> E<'a> {
    container(
        row![
            column![tx(label, 14.0 * k, G::muted()), txs(value, 23.0 * k, G::hi()), tx(sub, 13.0 * k, G::muted())].spacing(2.0 * k).width(Length::Fill).clip(true),
            container(draw::canvas(draw::Spark { pts, color: line, w: 2.0 * k }, Length::Fixed(120.0 * k), Length::Fixed(32.0 * k))).padding(Padding { top: 18.0 * k, ..Padding::ZERO })
        ]
        .spacing(6.0 * k),
    )
    .padding(Padding { top: 10.0 * k, right: 14.0 * k, bottom: 6.0 * k, left: 14.0 * k })
    .width(Length::FillPortion(1))
    .height(Length::Fill)
    .clip(true)
    .style(boxed(G::panel2(), 12.0 * k, Some((G::line(), 1.0))))
    .into()
}

/// The Services panel: every tile, five to a row (Ithil's two wide), as tall as in the mockups; the rows that fit,
/// moved along to keep the picked tile in sight (Mantel's Flickable follows it the same way).
fn services<'a>(a: &'a App, k: f32, used_above: f32) -> E<'a> {
    let tiles = a.tiles();
    let cells = a.cells();
    let tile_h = 262.0 * k;
    let gapk = 12.0 * k;
    let avail = a.size.height - 78.0 * k - 48.0 * k - 26.0 * k - used_above - 12.0 * k - (14.0 + 14.0 + 10.0 + 16.0) * k;
    // A row that only partly fits shows its top (the panel clips it), as Mantel's scrolling list does.
    let room = (avail + gapk) / (tile_h + gapk);
    let fit = (if room.fract() > 0.25 { room.ceil() } else { room.floor() } as usize).max(1);
    let rows_n = cells.iter().map(|c| c.0 + 1).max().unwrap_or(0);
    let sel_row = if a.sel >= 1 { cells.get(a.sel - 1).map(|c| c.0).unwrap_or(0) } else { 0 };
    let first = if sel_row + 1 > fit { sel_row + 1 - fit } else { 0 };
    let mut grid = Column::new().spacing(gapk).width(Length::Fill);
    for r in first..rows_n.min(first + fit) {
        let mut rw = Row::new().spacing(gapk).width(Length::Fill);
        let mut used = 0;
        for (i, c) in cells.iter().enumerate() {
            if c.0 != r {
                continue;
            }
            used += c.2;
            rw = rw.push(click(
                container(tile(&tiles[i], k * 0.86, a.sel == i + 1, a)).width(Length::FillPortion(c.2 as u16)).height(Length::Fixed(tile_h)).into(),
                Msg::Pick(i + 1),
            ));
        }
        if used < COLS {
            rw = rw.push(Space::new().width(Length::FillPortion((COLS - used) as u16)));
        }
        grid = grid.push(rw);
    }
    if tiles.is_empty() {
        grid = grid.push(tx(if a.d.is_some() { "No services yet" } else { "Reading what the server is doing…" }, 15.0 * k, G::muted()));
    }
    panel(k, "Services", container(grid).height(Length::Fill).clip(true).into(), Length::Fill)
}

/// One service's tile (ServerTile.qml, gold).
fn tile<'a>(t: &Value, k: f32, focused: bool, a: &'a App) -> E<'a> {
    let state = s(t, "state");
    let warn = state == "warn" || state == "stopped";
    let off = state == "off";
    let paused = state == "paused" || state == "paused-you";
    let (card, line, ctext, muted, acc, sunk) = (rgb(0x140c03), rgb(0x4a3212), rgb(0xfff4df), rgb(0xc9a372), rgb(0xffb347), rgb(0x0c0702));
    let fade = |c: Color| if off { alpha(c, 0.62) } else { c };
    let chip_c = if warn || paused { G::warn() } else if off { muted } else { G::ok() };
    let mut title_row = Row::new().spacing(8.0 * k).align_y(iced::Center).push(txs(s(t, "title"), 18.0 * k, fade(ctext)));
    if truthy(t, "new") {
        title_row = title_row.push(container(txb("NEW", 10.5 * k, acc)).padding(Padding { top: 1.0, right: 5.0 * k, bottom: 1.0, left: 5.0 * k }).style(boxed(Color::TRANSPARENT, 4.0 * k, Some((acc, 1.0)))));
    }
    let mut head = Row::new()
        .spacing(14.0 * k)
        .align_y(iced::Center)
        .push(icon_box(&s(t, "icon"), 40.0 * k, 10.0 * k, alpha(acc, 0.16), fade(acc), 22.0 * k))
        .push(column![title_row, tx(s(t, "engine"), 13.0 * k, fade(muted))].width(Length::Fill).clip(true));
    let chip = s(t, "chip");
    if !chip.is_empty() {
        head = head.push(container(txs(chip, 12.5 * k, chip_c)).padding(Padding { top: 0.0, right: 9.0 * k, bottom: 0.0, left: 9.0 * k }).center_y(Length::Fixed(26.0 * k)).style(boxed(alpha(chip_c, 0.16), 8.0 * k, None)));
    }
    let mut col = Column::new().spacing(8.0 * k).width(Length::Fill).height(Length::Fill).push(head);
    let boxes = arr(t, "boxes");
    // The AI team: its live mesh, small (AiMesh.qml compact).
    if truthy(t, "mesh") {
        col = col.push(container(crate::pages::mesh(a, k, true)).padding(4.0 * k).width(Length::Fill).height(Length::Fill).clip(true).style(boxed(sunk, 10.0 * k, None)));
    }
    // Ithil, two tiles wide: her state, then her mood, what she's focused on, her four needs.
    if let Some(it) = t.get("ithil").filter(|v| v.is_object()) {
        let mut right = Column::new().spacing(4.0 * k).width(Length::FillPortion(1));
        let mood = s(it, "mood");
        if !mood.is_empty() {
            right = right.push(rich(vec![sp("Mood ", 15.0 * k, muted, false), sp(mood, 15.0 * k, ctext, false)]));
        }
        let focus = s(it, "focus");
        if !focus.is_empty() {
            right = right.push(rich(vec![sp("Focused on ", 15.0 * k, muted, false), sp(hearth_ui::ui::fit(&focus, 80), 15.0 * k, ctext, false)]));
        }
        if let Some(n) = it.get("needs") {
            for (lbl, key) in [("Energy", "energy"), ("Social", "social"), ("Curiosity", "curiosity"), ("Stability", "stability")] {
                let v = f(n, key).unwrap_or(0.0).clamp(0.0, 1.0) as f32;
                right = right.push(row![
                    container(tx(lbl, 13.0 * k, ctext)).width(Length::Fixed(78.0 * k)),
                    hearth_ui::ui::bar(vec![(v, rgb(0xf472b6))], sunk, 7.0 * k, 4.0 * k),
                    container(tx(format!("{:.2}", v).trim_start_matches('0').to_string(), 13.0 * k, muted)).align_right(Length::Fixed(34.0 * k))
                ].spacing(10.0 * k).align_y(iced::Center));
            }
        }
        right = right.push(vfill());
        right = right.push(tx(if !truthy(it, "running") { "Her app isn't running · A to start her".to_string() } else {
            format!("{}{}", if truthy(it, "brainLoaded") { "Brain loaded" } else { "Brain not loaded" }, { let x = s(it, "answered"); if x.is_empty() { x } else { format!(" · answered you {x}") } })
        }, 13.0 * k, muted));
        let brain = crate::pages::tile_brain(a, k);
        col = col.push(row![brain, right].spacing(18.0 * k).height(Length::Fill));
    }
    let big = s(t, "big");
    if !big.is_empty() && boxes.is_empty() {
        col = col.push(container(row![txs(big, 34.0 * k, ctext), tx(s(t, "unit"), 18.0 * k, muted)].spacing(10.0 * k).align_y(iced::alignment::Vertical::Bottom)).padding(Padding { top: 8.0 * k, ..Padding::ZERO }));
    }
    let sub = s(t, "sub");
    if !sub.is_empty() {
        col = col.push(container(txw(sub, 14.0 * k, muted)).max_height(36.0 * k).clip(true));
    }
    for l in arr(t, "list") {
        let x = l.as_array().cloned().unwrap_or_default();
        let g = |i: usize| x.get(i).and_then(Value::as_str).unwrap_or("").to_string();
        col = col.push(row![dot(G::ok(), 8.0 * k), column![tx(g(0), 14.5 * k, ctext), tx(g(1), 12.5 * k, muted)]].spacing(10.0 * k).align_y(iced::Center));
    }
    if !boxes.is_empty() {
        let mut grid = Column::new().spacing(10.0 * k).height(Length::Fill);
        for pair in boxes.chunks(2) {
            let mut r = Row::new().spacing(10.0 * k).height(Length::Fill);
            for b in pair {
                let x = b.as_array().cloned().unwrap_or_default();
                let g = |i: usize| x.get(i).and_then(Value::as_str).unwrap_or("").to_string();
                let mut c = Column::new().spacing(2.0 * k).push(tx(g(0), 13.0 * k, muted)).push(txs(g(1), 24.0 * k, ctext));
                if !g(2).is_empty() {
                    c = c.push(tx(g(2), 12.5 * k, muted));
                }
                r = r.push(container(c).padding(Padding { top: 0.0, right: 8.0 * k, bottom: 0.0, left: 14.0 * k }).center_y(Length::Fill).width(Length::FillPortion(1)).clip(true).style(boxed(sunk, 10.0 * k, None)));
            }
            grid = grid.push(r);
        }
        col = col.push(container(grid).padding(Padding { top: 6.0 * k, ..Padding::ZERO }).height(Length::Fill));
    } else if !truthy(t, "mesh") && t.get("ithil").map(|v| !v.is_object()).unwrap_or(true) {
        col = col.push(vfill());
    }
    if let Some(b) = f(t, "bar") {
        col = col.push(hearth_ui::ui::bar(vec![((b / 100.0).min(1.0) as f32, acc)], sunk, 8.0 * k, 4.0 * k));
    }
    for r in arr(t, "rows") {
        let x = r.as_array().cloned().unwrap_or_default();
        let g = |i: usize| x.get(i).and_then(Value::as_str).unwrap_or("").to_string();
        col = col.push(row![container(tx(g(0), 14.0 * k, muted)).width(Length::Fill).clip(true), txs(hearth_ui::ui::fit(&g(1), 34), 14.0 * k, ctext)].spacing(8.0 * k));
    }
    container(col)
        .padding(22.0 * k)
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true)
        .style(boxed(card, 14.0 * k, Some(if focused { (acc, 3.0 * k) } else { (line, 1.0) })))
        .into()
}

// ---------------------------------------------------------------- the right column
fn right_column<'a>(a: &'a App, d: &Value, k: f32) -> E<'a> {
    let play = container(
        stack![
            container(
                column![
                    row![icon("pad", G::ink(), 34.0 * k), txb("PLAY", 36.0 * k, G::ink())].spacing(14.0 * k).align_y(iced::Center),
                    txs("Switch to Game Mode", 16.0 * k, G::ink())
                ]
                .spacing(2.0 * k)
                .align_x(iced::Center)
            )
            .center(Length::Fill),
            container(txs("or press Guide", 13.0 * k, G::ink())).align_right(Length::Fill).padding(14.0 * k)
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fixed(108.0 * k))
    .style(boxed(G::acc(), 18.0 * k, if a.sel == 0 { Some((rgb(0xffe2b0), 4.0 * k)) } else { None }));
    let mut col = Column::new().spacing(12.0 * k).width(Length::Fixed(430.0 * k)).height(Length::Fill).push(click(play.into(), Msg::Play));

    // Next: the schedule's next switch.
    let idle = d.get("idle").cloned().unwrap_or(Value::Null);
    let dim = f(&idle, "dim").unwrap_or(0.0);
    let next = d.get("next").filter(|n| n.is_object()).cloned();
    let wd = hearth_ui::time::local(a.now as i64).wday;
    let (n1, n2) = match &next {
        Some(n) => (s(n, "text"), format!("Your {} schedule{}", if wd % 6 == 0 { "weekend" } else { "weekday" }, if dim > 0.0 { format!(" · the screen dims after {dim} min") } else { String::new() })),
        None => ("No schedule".into(), if dim > 0.0 { format!("The screen dims after {dim} min with nothing pressed") } else { "Make one in Settings › System › Startup".into() }),
    };
    col = col.push(panel(k, "Next", column![txs(n1, 19.0 * k, G::hi()), tx(n2, 14.0 * k, G::muted())].spacing(4.0 * k).into(), Length::Fixed(104.0 * k)));

    // Graphics memory by app.
    let vt = f(d, "vramTotal").unwrap_or(0.0);
    let parts: Vec<(String, Color, f64)> = arr(d, "vram")
        .iter()
        .map(|v| {
            let (n, c) = match s(v, "who").as_str() {
                "ai" => ("AI chat", rgb(0xe8d66e)),
                "pictures" => ("ComfyUI", rgb(0xff5fd2)),
                "voices" => ("Voices", rgb(0xff9aa8)),
                "media" => ("Media", rgb(0x7fb2ff)),
                _ => ("Other", rgb(0xc9a372)),
            };
            (n.to_string(), c, f(v, "gb").unwrap_or(0.0))
        })
        .collect();
    if !parts.is_empty() && vt > 0.0 {
        let bar = hearth_ui::ui::bar(parts.iter().map(|p| ((p.2 / vt) as f32, p.1)).collect(), G::line(), 12.0 * k, 6.0 * k);
        let mut legend = Row::new().spacing(16.0 * k);
        for p in &parts {
            legend = legend.push(rich(vec![sp("● ", 14.0 * k, p.1, false), sp(format!("{} {} GB", p.0, p.2), 14.0 * k, G::muted(), false)]));
        }
        legend = legend.push(tx(format!("free {:.1} GB", (vt - f(d, "vramUsed").unwrap_or(0.0)).max(0.0)), 14.0 * k, G::muted()));
        col = col.push(panel(k, "Graphics memory by app", column![bar, legend.wrap().vertical_spacing(4.0 * k)].spacing(8.0 * k).into(), Length::Fixed(116.0 * k)));
    }

    // Running: every service in a word.
    let pills = arr(d, "pills");
    let mut pf = Row::new().spacing(8.0 * k);
    for p in &pills {
        let st = s(p, "state");
        let dc = if st == "running" { G::ok() } else if st == "off" { G::muted() } else { G::warn() };
        let mut r = Row::new().spacing(7.0 * k).align_y(iced::Center).push(dot(dc, 8.0 * k)).push(tx(s(p, "name"), 14.5 * k, G::text()));
        let extra = s(p, "extra");
        if !extra.is_empty() {
            r = r.push(tx(extra, 13.0 * k, G::muted()));
        }
        pf = pf.push(container(r).padding(Padding { top: 0.0, right: 12.0 * k, bottom: 0.0, left: 12.0 * k }).center_y(Length::Fixed(32.0 * k)).style(boxed(Color::TRANSPARENT, 16.0 * k, Some((alpha(dc, 0.4), 1.0)))));
    }
    let lines = ((pills.len() as f32) / 3.0).ceil().max(1.0);
    col = col.push(panel(k, "Running", pf.wrap().vertical_spacing(8.0 * k).into(), Length::Fixed((lines * 40.0 + 52.0) * k)));

    // Alerts.
    let mut alerts = arr(d, "alerts");
    if alerts.is_empty() {
        alerts.push(serde_json::json!({"level": "ok", "text": "Nothing needs a look", "sub": "", "at": ""}));
    }
    let mut ac = Column::new().spacing(10.0 * k);
    for al in alerts.iter().take(3) {
        let c = match s(al, "level").as_str() {
            "warn" => G::warn(),
            "ok" => G::ok(),
            "team" => rgb(0xff5fae),
            "ithil" => rgb(0xc4b5fd),
            _ => G::muted(),
        };
        let mut spans = vec![sp(s(al, "text"), 15.0 * k, G::text(), false)];
        let sub = s(al, "sub");
        if !sub.is_empty() {
            spans.push(sp(format!(" · {sub}"), 15.0 * k, G::muted(), false));
        }
        ac = ac.push(row![
            container(dot(c, 9.0 * k)).padding(Padding { top: 6.0 * k, ..Padding::ZERO }),
            container(rich(spans)).width(Length::Fill).max_height(40.0 * k).clip(true),
            tx(s(al, "at"), 13.5 * k, G::muted())
        ].spacing(10.0 * k));
    }
    let n = alerts.len().min(3) as f32;
    col = col.push(panel(k, "Alerts", ac.into(), Length::Fixed((n * 48.0 + 50.0) * k)));

    // Your phone: the web page's address and its QR code.
    if d.get("web").map(|w| truthy(w, "on")).unwrap_or(false) {
        let url = d.get("web").map(|w| s(w, "url")).unwrap_or_default();
        let mut r = Row::new().spacing(14.0 * k).align_y(iced::Center);
        if let Some(q) = &a.qr {
            r = r.push(draw::canvas(draw::QrCode { qr: q.clone(), fg: rgb(0xffe9c7), bg: rgb(0x0c0702) }, Length::Fixed(72.0 * k), Length::Fixed(72.0 * k)));
        }
        r = r.push(column![txs(url, 16.0 * k, G::hi()), txw(format!("{}Tailscale only · your PIN", if a.qr.is_some() { "Scan to open this dashboard on your phone · " } else { "Open it on your phone · " }), 13.5 * k, G::muted())].spacing(3.0 * k).width(Length::Fill));
        col = col.push(panel(k, "Your phone", r.into(), Length::Fixed(128.0 * k)));
    }
    container(col.push(vfill())).width(Length::Fixed(430.0 * k)).height(Length::Fill).clip(true).into()
}

// ---------------------------------------------------------------- the countdown (Play / Guide, or the web page's)
fn sline<'a>(k: f32, ic: &str, title: String, sub: String, tone: Color) -> E<'a> {
    container(
        row![
            icon_box(ic, 40.0 * k, 10.0 * k, alpha(tone, 0.15), tone, 22.0 * k),
            column![txs(title, 16.0 * k, G::text()), tx(sub, 13.5 * k, G::muted())].spacing(1.0).width(Length::Fill).clip(true)
        ]
        .spacing(14.0 * k)
        .align_y(iced::Center),
    )
    .padding(Padding { top: 0.0, right: 14.0 * k, bottom: 0.0, left: 14.0 * k })
    .center_y(Length::Fixed(64.0 * k))
    .width(Length::Fill)
    .style(boxed(G::panel2(), 12.0 * k, None))
    .into()
}

pub fn sbutton<'a>(k: f32, label: &str, key: &str, primary: bool, m: Msg) -> E<'a> {
    let fg = if primary { G::ink() } else { G::text() };
    let b = container(
        row![
            txs(label.to_string(), 17.0 * k, fg),
            container(txb(key.to_string(), 12.0 * k, fg)).center_x(Length::Fixed(24.0 * k)).center_y(Length::Fixed(24.0 * k)).style(boxed(Color::TRANSPARENT, 12.0 * k, Some((if primary { G::ink() } else { G::muted() }, 1.0))))
        ]
        .spacing(10.0 * k)
        .align_y(iced::Center),
    )
    .padding(Padding { top: 0.0, right: 20.0 * k, bottom: 0.0, left: 20.0 * k })
    .center_y(Length::Fixed(50.0 * k))
    .style(boxed(if primary { G::acc() } else { G::panel2() }, 12.0 * k, Some(if primary { (rgb(0xffd9a0), 3.0 * k) } else { (G::line2(), 1.0) })));
    click(b.into(), m)
}

fn switch<'a>(a: &App, d: &Value, k: f32) -> E<'a> {
    let rem = a.remote.clone();
    let is_remote = rem.is_some();
    let r = rem.clone().unwrap_or(Value::Null);
    let left = if is_remote { a.remote_left() } else { a.count };
    let total = if is_remote { f(&r, "total").unwrap_or(10.0) } else { 5.0 };
    let power = is_remote && s(&r, "what") == "power";
    let to = if is_remote {
        if power { if s(&r, "to") == "sleep" { "SLEEP" } else { "RESTART" } } else if s(&r, "to") == "desktop" { "DESKTOP MODE" } else { "GAME MODE" }
    } else {
        "GAME MODE"
    };
    let asked = if is_remote {
        format!("  ·  ASKED FOR {}", if s(&r, "from") == "schedule" { "BY THE SCHEDULE".to_string() } else { format!("FROM {}", { let b = s(&r, "by"); if b.is_empty() { "THE WEB PAGE".into() } else { b.to_uppercase() } }) })
    } else {
        String::new()
    };
    let head = format!("{}{}{}", if power { "" } else { "SWITCHING TO " }, to, asked);
    let big = if power { if s(&r, "to") == "sleep" { "The PC is going to sleep" } else { "The PC is restarting" } } else if to == "DESKTOP MODE" { "Opening Desktop Mode" } else { "Getting the PC ready to play" };
    let ring = stack![
        draw::canvas(draw::Ring { p: (left as f32 / total as f32).clamp(0.0, 1.0), track: G::line2(), fill: G::acc(), w: 12.0 * k }, Length::Fixed(200.0 * k), Length::Fixed(200.0 * k)),
        container(column![txs(left.to_string(), 72.0 * k, G::text()), tx("seconds", 15.0 * k, G::muted())].align_x(iced::Center)).center(Length::Fixed(200.0 * k))
    ];
    // What pauses and what keeps working (Light mode's own rules).
    let p = d.get("pauses").cloned().unwrap_or(Value::Null);
    let ps = |kk: &str| s(&p, kk);
    let on = |kk: &str| truthy(&p, kk);
    let mut pause: Vec<(&str, String, String)> = vec![];
    if on("picturesOn") && ps("pictures") == "stop" { pause.push(("image", "Pictures stop".into(), "After the current picture. The pictures waiting stay in the queue.".into())); }
    if on("aiOn") && ps("ai") == "small" { pause.push(("chat", format!("AI chat switches to {}", ps("small")), format!("On the processor, so your devices still get answers. {} loads again when you're back.", ps("big")))); }
    else if on("aiOn") && ps("ai") == "unload" { pause.push(("chat", "AI chat unloads its model".into(), "Frees graphics memory. The next question loads it again.".into())); }
    else if on("aiOn") && ps("ai") == "stop" { pause.push(("chat", "AI chat stops".into(), "Questions wait until you're back.".into())); }
    if on("voicesOn") && ps("voices") == "pause" { pause.push(("wave", "Voices pause".into(), "Kokoro starts again when you're back.".into())); }
    if on("lightMode") { pause.push(("cloud", "Backups wait".into(), "A backup that's due starts when you stop playing.".into())); }
    if on("mediaOn") && ps("media") == "pause" { pause.push(("film", "Media pauses".into(), "Jellyfin starts again when you're back.".into())); }
    let mut keep: Vec<(&str, String, String)> = vec![("folder", "Shared folders".into(), "They stay open on your devices.".into())];
    if on("mediaOn") && ps("media") != "pause" { keep.push(("film", "Media".into(), "Watching keeps going. Converting uses the processor.".into())); }
    keep.push(("tailscale", "Tailscale and the web page".into(), "You can still check on the PC from your phone.".into()));
    keep.push(("desktops", "Remote desktop".into(), "Stream your game to another device if you like.".into()));
    if on("aiOn") && ps("ai") == "keep" { keep.push(("chat", "AI chat".into(), "Keeps its model loaded on the graphics card.".into())); }
    let mut pc = Column::new().spacing(10.0 * k).width(Length::FillPortion(1)).push(txs("Pauses while you play", 16.0 * k, G::text()));
    if pause.is_empty() {
        pc = pc.push(tx("Nothing: Light mode is off or nothing big is on", 15.0 * k, G::muted()));
    }
    for (i, t, sub) in pause { pc = pc.push(sline(k, i, t, sub, G::warn())); }
    let mut kc = Column::new().spacing(10.0 * k).width(Length::FillPortion(1)).push(txs("Keeps working", 16.0 * k, G::text()));
    for (i, t, sub) in keep { kc = kc.push(sline(k, i, t, sub, G::ok())); }
    let (cancel, now) = if is_remote { (Msg::RemoteCancel, Msg::RemoteNow) } else { (Msg::CancelSwitch, Msg::SwitchNow) };
    container(
        column![
            row![ring, column![txs(head, 15.0 * k, G::acc()), txs(big, 40.0 * k, G::text()), txw("Big jobs pause so your game gets the graphics card. They come back by themselves when you return to Server mode.", 17.0 * k, G::muted())].spacing(8.0 * k).width(Length::Fill)].spacing(40.0 * k).align_y(iced::Center),
            row![pc, kc].spacing(28.0 * k),
            row![
                container(tx("The same rules as Light mode · change them in Settings › Home server › Server mode", 14.0 * k, G::muted())).width(Length::Fill).clip(true),
                sbutton(k, "Cancel", "B", false, cancel),
                sbutton(k, if power { "Now" } else { "Switch now" }, "A", true, now)
            ].spacing(14.0 * k).align_y(iced::Center)
        ]
        .spacing(26.0 * k),
    )
    .padding(40.0 * k)
    .width(Length::Fixed(1240.0 * k))
    .style(boxed(G::panel(), 22.0 * k, Some((G::line(), 1.0))))
    .into()
}

// ---------------------------------------------------------------- a tile opened (ServerDetail.qml)
fn card<'a>(k: f32, inner: Column<'a, Msg>) -> E<'a> {
    container(inner.spacing(12.0 * k)).padding(20.0 * k).width(Length::FillPortion(1)).height(Length::Fill).clip(true).style(boxed(G::panel2(), 14.0 * k, None)).into()
}
fn kv<'a>(k: f32, l: String, v: String) -> E<'a> {
    row![container(tx(l, 15.0 * k, G::muted())).width(Length::Fill).clip(true), txs(v, 15.0 * k, G::text())].spacing(8.0 * k).into()
}

fn detail<'a>(a: &App, d: &Value, t: &Value, k: f32) -> E<'a> {
    let id = s(t, "id");
    let ai = id == "ai";
    let acts: Vec<String> = arr(t, "actions").iter().filter_map(|x| x.as_str().map(String::from)).collect();
    let head = row![
        icon_box(&s(t, "icon"), 52.0 * k, 12.0 * k, alpha(G::acc(), 0.16), G::acc(), 28.0 * k),
        column![txs(s(t, "title"), 30.0 * k, G::text()), tx(format!("{}{}", s(t, "engine"), if ai { " · answers your devices over Tailscale" } else { "" }), 15.0 * k, G::muted())].spacing(2.0).width(Length::Fill),
        txs(s(t, "chip"), 15.0 * k, G::ok())
    ]
    .spacing(18.0 * k)
    .align_y(iced::Center);

    // Left: models (AI chat), Ithil, or what it's doing now.
    let mut left = Column::new();
    if ai {
        left = left.push(txs("Models", 17.0 * k, G::text()));
        let loaded: Vec<String> = arr(t, "loaded").iter().filter_map(|x| x.as_str().map(String::from)).collect();
        let models = arr(t, "models");
        for m in &models {
            let m = m.as_str().unwrap_or("").to_string();
            let on = loaded.contains(&m);
            left = left.push(row![icon(if on { "check" } else { "drive" }, if on { G::ok() } else { G::muted() }, 20.0 * k), container(tx(m, 16.0 * k, G::text())).width(Length::Fill).clip(true), tx(if on { "Loaded" } else { "Downloaded" }, 14.0 * k, if on { G::ok() } else { G::muted() })].spacing(12.0 * k).align_y(iced::Center));
        }
        if models.is_empty() {
            left = left.push(tx("No models downloaded: Settings › Home server › AI chat", 15.0 * k, G::muted()));
        }
        left = left.push(vfill());
        let p = d.get("pauses").cloned().unwrap_or(Value::Null);
        left = left.push(txw(match s(&p, "ai").as_str() {
            "small" => format!("While you play: {} answers on the processor; {} loads again when you're back. Models are never deleted.", s(&p, "small"), s(&p, "big")),
            "unload" => "While you play: the model is unloaded from the graphics card.".into(),
            "stop" => "While you play: AI chat stops.".into(),
            _ => "While you play: it keeps its model loaded.".into(),
        }, 14.0 * k, G::muted()));
    } else if let Some(it) = t.get("ithil").filter(|v| v.is_object()) {
        left = left.push(txs("Right now", 17.0 * k, G::text()));
        for (l, kk) in [("Mood", "mood"), ("Focused on", "focus"), ("State", "state")] {
            let v = s(it, kk);
            if !v.is_empty() {
                left = left.push(kv(k, l.into(), hearth_ui::ui::fit(&v, 60)));
            }
        }
        if let Some(n) = it.get("needs") {
            for (l, kk) in [("Energy", "energy"), ("Social", "social"), ("Curiosity", "curiosity"), ("Stability", "stability")] {
                left = left.push(kv(k, l.into(), format!("{:.2}", f(n, kk).unwrap_or(0.0))));
            }
        }
    } else {
        left = left.push(txs("Now", 17.0 * k, G::text()));
        let mut now_t = format!("{} {}", s(t, "big"), s(t, "unit")).trim().to_string();
        let sub = s(t, "sub");
        if !sub.is_empty() {
            now_t = format!("{now_t}\n{sub}");
        }
        left = left.push(txw(now_t, 20.0 * k, G::text()));
        for r in arr(t, "rows") {
            let x = r.as_array().cloned().unwrap_or_default();
            let g = |i: usize| x.get(i).and_then(Value::as_str).unwrap_or("").to_string();
            left = left.push(kv(k, g(0), g(1)));
        }
        for b in arr(t, "boxes") {
            let x = b.as_array().cloned().unwrap_or_default();
            let g = |i: usize| x.get(i).and_then(Value::as_str).unwrap_or("").to_string();
            left = left.push(kv(k, g(0), if g(2).is_empty() { g(1) } else { format!("{} · {}", g(1), g(2)) }));
        }
        for l in arr(t, "list") {
            let x = l.as_array().cloned().unwrap_or_default();
            let g = |i: usize| x.get(i).and_then(Value::as_str).unwrap_or("").to_string();
            left = left.push(kv(k, format!("● {}", g(0)), g(1)));
        }
        if matches!(id.as_str(), "cache" | "ads" | "rules") {
            left = left.push(vfill());
            left = left.push(txw("Its switches and lists are in Settings › Home server (Y opens it).", 14.0 * k, G::muted()));
        }
    }

    // Middle: the graphics card's memory, by service.
    let names = |w: &str| match w { "ai" => "AI chat", "pictures" => "Pictures", "voices" => "Voices", "media" => "Media", _ => "Other apps" };
    let colors = |w: &str| match w { "ai" => G::acc(), "pictures" => rgb(0x8b9cf7), "voices" => rgb(0xe58fd0), "media" => rgb(0x5bd18b), _ => G::muted() };
    let vt = f(d, "vramTotal").unwrap_or(0.0);
    let vram = arr(d, "vram");
    let mut mid = Column::new().push(txs("Graphics memory", 17.0 * k, G::text())).push(txs(if vt > 0.0 { format!("{} of {} GB used", s(d, "vramUsed"), vt.round()) } else { "Not known for this graphics card".into() }, 28.0 * k, G::text()));
    if vt > 0.0 {
        mid = mid.push(hearth_ui::ui::bar(vram.iter().map(|v| ((f(v, "gb").unwrap_or(0.0) / vt) as f32, colors(&s(v, "who")))).collect(), G::panel(), 14.0 * k, 7.0 * k));
    }
    for v in &vram {
        let w = s(v, "who");
        mid = mid.push(row![container(Space::new()).width(Length::Fixed(12.0 * k)).height(Length::Fixed(12.0 * k)).style(boxed(colors(&w), 3.0 * k, None)), container(tx(names(&w), 15.0 * k, G::text())).width(Length::Fill), tx(format!("{} GB", s(v, "gb")), 15.0 * k, G::muted())].spacing(10.0 * k).align_y(iced::Center));
    }

    // Right: who can reach it.
    let mut right = Column::new().push(txs("Who can reach it", 17.0 * k, G::text()));
    for (ic, t1, t2, on) in [("tailscale", "Your devices on Tailscale", "Only devices signed in to your Tailscale", true), ("server", "This PC", "Apps on this PC itself", true), ("globe", "The internet", "Never: Hearth's firewall only lets Tailscale in", false)] {
        right = right.push(row![icon(ic, G::muted(), 20.0 * k), column![tx(t1, 15.0 * k, G::text()), tx(t2, 13.0 * k, G::muted())].width(Length::Fill).clip(true), txs(if on { "On" } else { "Off" }, 14.0 * k, if on { G::ok() } else { G::muted() })].spacing(12.0 * k).align_y(iced::Center));
    }
    right = right.push(container(txs("Connected now", 15.0 * k, G::text())).padding(Padding { top: 10.0 * k, ..Padding::ZERO }));
    let title = s(t, "title");
    for w in arr(d, "who").iter().filter(|w| arr(w, "what").iter().any(|x| x.as_str() == Some(title.as_str()))) {
        right = right.push(tx(format!("● {}", s(w, "name")), 15.0 * k, G::text()));
    }

    // The buttons: what this one can do.
    let mut btns = Row::new().spacing(12.0 * k).align_y(iced::Center);
    match id.as_str() {
        "ithil" => {
            btns = btns.push(sbutton(k, "Talk to her", "A", true, Msg::DetailAct("talk".into())));
            btns = btns.push(sbutton(k, "Restart her app", "X", false, Msg::DetailAct("ithil-restart".into())));
            btns = btns.push(sbutton(k, if a.sleep_armed { "Press again: sleep" } else { "Sleep" }, "Y", false, Msg::DetailAct("ithil-sleep".into())));
        }
        "team" => {
            btns = btns.push(sbutton(k, if s(t, "state") == "paused-you" { "Resume" } else { "Pause" }, "A", true, Msg::DetailAct("pause".into())));
            btns = btns.push(sbutton(k, "Restart", "X", false, Msg::DetailAct("restart".into())));
            btns = btns.push(sbutton(k, "AI devices settings", "Y", false, Msg::DetailAct("settings-team".into())));
        }
        "cache" | "ads" | "rules" => btns = btns.push(sbutton(k, "Settings › Home server", "Y", true, Msg::DetailAct("settings".into()))),
        _ => {
            if ai {
                btns = btns.push(sbutton(k, "Unload model", "A", true, Msg::DetailAct("unload".into())));
            }
            if acts.iter().any(|x| x == "restart") {
                btns = btns.push(sbutton(k, "Restart", "X", false, Msg::DetailAct("restart".into())));
            }
            if acts.iter().any(|x| x == "pause") {
                let armed = a.pause_armed == id;
                btns = btns.push(sbutton(k, if s(t, "state") == "paused-you" { "Resume" } else if armed { "Press again to pause" } else { "Pause" }, "Y", false, Msg::DetailAct("pause".into())));
            }
        }
    }
    btns = btns.push(fill()).push(click(tx("B  Back to the dashboard", 15.0 * k, G::muted()).into(), Msg::Close));
    let mut foot = Column::new().spacing(6.0 * k).push(btns);
    if !a.toast.is_empty() {
        foot = foot.push(tx(a.toast.clone(), 15.0 * k, G::acc()));
    }
    container(column![head, row![card(k, left), card(k, mid), card(k, right)].spacing(16.0 * k).height(Length::Fill), foot].spacing(18.0 * k))
        .padding(32.0 * k)
        .width(Length::Fixed(1620.0 * k))
        .height(Length::Fixed(860.0 * k))
        .style(boxed(G::panel(), 22.0 * k, Some((G::line(), 1.0))))
        .into()
}

// ---------------------------------------------------------------- the menu (☰)
fn menu<'a>(a: &App, k: f32) -> E<'a> {
    let mut c = Column::new().spacing(6.0 * k);
    for (i, (ic, t, sub)) in a.menu_items().into_iter().enumerate() {
        let on = a.menu_sel == i;
        let r = container(row![icon(ic, if on { G::acc() } else { G::text() }, 24.0 * k), column![txs(t, 18.0 * k, G::text()), tx(sub, 13.5 * k, G::muted())].width(Length::Fill)].spacing(16.0 * k).align_y(iced::Center))
            .padding(Padding { top: 0.0, right: 12.0 * k, bottom: 0.0, left: 18.0 * k })
            .center_y(Length::Fixed(66.0 * k))
            .width(Length::Fill)
            .style(boxed(if on { alpha(G::acc(), 0.18) } else { Color::TRANSPARENT }, 12.0 * k, if on { Some((G::acc(), 2.0)) } else { None }));
        c = c.push(click(r.into(), Msg::MenuItem(i)));
    }
    container(c).padding(20.0 * k).width(Length::Fixed(640.0 * k)).style(boxed(G::panel(), 22.0 * k, Some((G::line(), 1.0)))).into()
}

// ---------------------------------------------------------------- Back in Server mode (ServerBack.qml)
fn back_panel<'a>(a: &App, d: &Value, k: f32) -> E<'a> {
    let info = a.back_info.clone().unwrap_or(Value::Null);
    let p = d.get("pauses").filter(|x| x.is_object()).cloned().or_else(|| info.get("pauses").cloned()).unwrap_or(Value::Null);
    let tiles = a.tiles();
    let tile = |id: &str| tiles.iter().find(|t| s(t, "id") == id).cloned();
    let played = f(&info, "played").unwrap_or(0.0) as i64;
    let (h, m) = (played / 3600, (played % 3600) / 60);
    let ptxt = if h > 0 { format!("{h} h {m} min") } else { format!("{m} min") };
    let mut items: Vec<(bool, String, String)> = vec![];
    if truthy(&p, "aiOn") {
        let t = tile("ai");
        let loaded: Vec<String> = t.as_ref().map(|t| arr(t, "loaded").iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        items.push((loaded.is_empty(), "AI chat".into(), if !loaded.is_empty() { format!("Ready · {}", loaded.join(", ")) } else if s(&p, "ai") == "small" { format!("{} loads on the next question", s(&p, "big")) } else { "Ready".into() }));
    }
    if truthy(&p, "picturesOn") {
        let t = tile("pictures");
        let run = t.as_ref().map(|t| s(t, "state") == "running").unwrap_or(false);
        items.push((!run, "Pictures".into(), if run { format!("Ready · {} waiting in the queue", t.map(|t| s(&t, "big")).unwrap_or_default()) } else { "Starting · the queue was kept".into() }));
    }
    if truthy(&p, "lightMode") {
        let r = tile("backups").map(|t| s(&t, "chip") == "Running").unwrap_or(false);
        items.push((r, "Backups".into(), if r { "A backup that waited runs now".into() } else { "Nothing waited".into() }));
    }
    if truthy(&p, "voicesOn") {
        items.push((false, "Voices".into(), "Ready".into()));
    }
    items.push((false, "Shared folders, media and remote desktop".into(), "Kept working the whole time".into()));
    let mut c = Column::new()
        .spacing(12.0 * k)
        .push(row![icon_box("server", 34.0 * k, 17.0 * k, alpha(G::ok(), 0.2), G::ok(), 20.0 * k), txs("Back in Server mode", 21.0 * k, G::text())].spacing(12.0 * k).align_y(iced::Center))
        .push(txw(format!("You played for {ptxt}. Everything that paused is coming back."), 15.0 * k, G::muted()));
    for (busy, t, sub) in items {
        let mark: E = if busy {
            draw::canvas(draw::Ring { p: 0.7, track: Color::TRANSPARENT, fill: G::acc(), w: 3.0 * k }, Length::Fixed(22.0 * k), Length::Fixed(22.0 * k))
        } else {
            icon_box("check", 22.0 * k, 11.0 * k, alpha(G::ok(), 0.2), G::ok(), 14.0 * k)
        };
        c = c.push(row![mark, column![txs(t, 15.5 * k, G::text()), tx(sub, 13.0 * k, G::muted())].width(Length::Fill).clip(true)].spacing(12.0 * k).align_y(iced::Center));
    }
    c = c.push(container(tx("B  Hide", 13.5 * k, G::muted())).align_right(Length::Fill));
    container(c).padding(22.0 * k).width(Length::Fixed(436.0 * k)).style(boxed(G::panel(), 18.0 * k, Some((G::line(), 1.0)))).into()
}

// ---------------------------------------------------------------- the idle screen (dim), then black
fn idle<'a>(a: &App, d: &Value, k: f32) -> E<'a> {
    let black = |e: E<'a>| -> E<'a> { container(e).width(Length::Fill).height(Length::Fill).style(boxed(Color::BLACK, 0.0, None)).into() };
    if a.screen == Screen::Off {
        return black(Space::new().into());
    }
    let lt = hearth_ui::time::local(a.now as i64);
    let st = d.get("status").cloned().unwrap_or(Value::Null);
    let dimc = rgb(0x5a4424);
    let mut line = Row::new().spacing(34.0 * k).align_y(iced::Center);
    if a.d.is_some() {
        line = line.push(row![dot(if s(&st, "level") != "ok" { rgb(0x6b5a2f) } else { rgb(0x2f6b47) }, 10.0 * k), tx(s(&st, "title"), 21.0 * k, dimc)].spacing(10.0 * k).align_y(iced::Center));
        line = line.push(tx(s(&st, "sub"), 21.0 * k, dimc));
        let n = f(d, "devices").unwrap_or(0.0) as i64;
        line = line.push(tx(format!("{n} {}", if n == 1 { "device" } else { "devices" }), 21.0 * k, dimc));
        for fa in arr(d, "facts").iter().filter_map(Value::as_array).filter(|x| x.get(1).and_then(Value::as_str) != Some("—")).take(2) {
            line = line.push(tx(format!("{} {}", fa.first().and_then(Value::as_str).unwrap_or(""), fa.get(1).and_then(Value::as_str).unwrap_or("")), 21.0 * k, dimc));
        }
    }
    let mut cores = Row::new().spacing(70.0 * k);
    for c in arr(d, "cores") {
        cores = cores.push(draw::canvas(draw::Core { kind: s(&c, "id"), working: s(&c, "state") == "working", off: false, fade: 0.32, t: a.anim_t, still: a.still() || a.light() }, Length::Fixed(90.0 * k), Length::Fixed(90.0 * k)));
    }
    black(
        column![
            vfill(),
            tx(clock(a.now, false), 230.0 * k, rgb(0x8a6a3e)).font(weight(Weight::Light)),
            tx(format!("{} {} {}", LDAYS[lt.wday], lt.day, LMONS[lt.mon]), 30.0 * k, dimc),
            gap(1.0, 26.0 * k),
            line,
            vfill(),
            cores,
            gap(1.0, 40.0 * k),
            tx(format!("{} · Server mode · press any button", s(d, "host")), 16.0 * k, rgb(0x3a2c18)),
            gap(1.0, 56.0 * k)
        ]
        .align_x(iced::Center)
        .width(Length::Fill)
        .into(),
    )
}

//! The Dashboard, desk view (board 03, docs/mockups/server-expansion/03-dashboard.png; checklist
//! docs/mockups/checklists/server-dashboard.md): the PC's six numbers, every service with its real health, what needs
//! a look with Fix it, earlier today, the one-tap fixes, network, power, tonight's jobs, where alerts go. Read from
//! `dash.py status` every 2 seconds while it's open. Desk · TV · Phone · Ally switch the view (tv.rs).

use super::Page;
use crate::{act, act_v, set, App, Msg, E};
use hearth_oss::w::{self, Sz, Tg};
use hearth_server_parts::draw::{self, pad, st};
use hearth_server_parts::{arr, b, f, nums, s};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, mouse_area, row, Column, Row, Space};
use iced::{mouse, Color, Length};
use serde_json::{json, Value};

pub const PAGE: Page = Page { id: "dash", title: "Dashboard", side: "dash", back: "home", engines: &["dash"], every: 2, bare: false, view, tags, key: super::no_key };

pub const BLOCKS: PageDef = PageDef {
    id: "dash",
    title: "Dashboard",
    blocks: &[
        bl("head", "Dashboard, its views and Fix everything", 0, 0, 9, 1),
        bl("kpis", "The PC: CPU, graphics card, memory, drives, temperatures", 0, 1, 9, 2),
        bl("services", "Services and their tabs", 0, 3, 9, 1),
        bl("ollama", "Ollama", 0, 4, 3, 1),
        bl("colibri", "Colibri", 3, 4, 3, 1),
        bl("jellyfin", "Jellyfin", 6, 4, 3, 1),
        bl("comfyui", "Imagine · ComfyUI", 0, 5, 3, 1),
        bl("kokoro", "Kokoro voices", 3, 5, 3, 1),
        bl("vaultwarden", "Passwords · Vaultwarden", 6, 5, 3, 1),
        bl("nextcloud", "Cloud · Nextcloud", 0, 6, 3, 1),
        bl("homeassistant", "Home · Home Assistant", 3, 6, 3, 1),
        bl("shield", "Hearth Shield · AdGuard Home", 6, 6, 3, 1),
        bl("games", "Game servers", 0, 7, 2, 1),
        bl("cache", "Game cache", 2, 7, 2, 1),
        bl("sunshine", "Remote play · Sunshine", 4, 7, 2, 1),
        bl("tailscale", "Tailscale · Hearth Link", 6, 7, 3, 1),
        bl("network", "Network · 24 h", 0, 9, 3, 2),
        bl("power", "Power · 24 h", 3, 9, 3, 2),
        bl("tonight", "Tonight", 6, 9, 3, 2),
        bl("foot", "Each card is a block", 0, 11, 9, 1),
        bl("needs", "Needs a look", 9, 0, 3, 3),
        bl("earlier", "Earlier today", 9, 3, 3, 2),
        bl("fixes", "One-tap fixes", 9, 5, 3, 3),
        bl("temps", "Temperatures · 24 h", 9, 8, 3, 2),
        bl("alerts", "Alerts go to", 9, 10, 3, 2),
    ],
};

/// The services' ids (each card is its own block).
const SVC_IDS: [&str; 13] = ["ollama", "colibri", "jellyfin", "comfyui", "kokoro", "vaultwarden", "nextcloud", "homeassistant", "shield", "games", "cache", "sunshine", "tailscale"];

fn tags(a: &App) -> crate::Tags {
    let d = a.d("dash");
    if d.is_null() || !b(d, "ok") {
        return Vec::new();
    }
    let mut v = vec![(format!("{} of {} healthy", f(d, "healthy"), f(d, "total")), Tg::Ok)];
    let n = f(d, "needsCount") as i64;
    if n > 0 {
        v.push((format!("{n} {}", if n == 1 { "needs a look" } else { "need a look" }), Tg::Warn));
    }
    v
}

pub fn click<'a>(el: impl Into<E<'a>>, m: Msg) -> E<'a> {
    mouse_area(el.into()).on_press(m).interaction(mouse::Interaction::Pointer).into()
}

fn cap<'a>(p: &Pal, s: impl Into<String>) -> E<'a> {
    tb(s.into().to_uppercase(), 11.0, p.muted).wrapping(Wrapping::None).into()
}

/// The view picker every Dashboard view has: Desk, TV, Phone, Ally.
pub fn views<'a>(p: &Pal, on: &str) -> E<'a> {
    let go = |id: &'static str| Msg::Go(id);
    w::seg(p, vec![("Desk".into(), on == "dash", Some(go("dash"))), ("TV".into(), on == "tv", Some(go("tv"))), ("Phone".into(), on == "phone", Some(go("phone"))), ("Ally".into(), false, Some(go("tv")))])
}

fn range_label(r: &str) -> &'static str {
    match r {
        "1h" => "Last hour ▾",
        "7d" => "Last 7 days ▾",
        _ => "Last 24 h ▾",
    }
}

fn kpi<'a>(p: &Pal, k: &Value) -> E<'a> {
    let under: E<'a> = if k.get("bar").is_none() {
        container(draw::spark(nums(k, "spark"), draw::hex(&s(k, "color")), !b(k, "nofill"), Length::Fill, 30.0)).padding(pad(6.0, 0.0, 0.0, 0.0)).into()
    } else {
        let c = if b(k, "green") { Some(w::green()) } else { None };
        column![container(w::meter(p, f(k, "bar") as f32, c, 0.0, 6.0)).padding(pad(10.0, 0.0, 0.0, 0.0)), t(s(k, "s"), 12.0, p.muted).wrapping(Wrapping::None)].spacing(4.0).into()
    };
    draw::kpi(p, &s(k, "k"), s(k, "v"), s(k, "of"), under)
}

fn metric<'a>(p: &Pal, label: &str, v: String) -> E<'a> {
    column![t(label.to_string(), 11.0, p.muted), tb(v, 13.0, p.text)].spacing(2.0).width(Length::FillPortion(1)).into()
}

/// One service's card (.sv): its mark, name and what it's doing, its dot; CPU, GPU mem, RAM, Disk; its foot and button.
fn service<'a>(p: &Pal, x: &Value) -> E<'a> {
    let state = s(x, "state");
    let warn = state == "warn" || state == "bad";
    let top = row![w::mark(&s(x, "mark"), &s(x, "grad"), 22.0), column![tb(s(x, "name"), 14.0, p.text).wrapping(Wrapping::None), t(s(x, "what"), 11.0, p.muted).wrapping(Wrapping::None)].spacing(1.0), Space::new().width(Length::Fill), st(&state, 10.0)].spacing(10.0).align_y(iced::Center);
    let m = row![metric(p, "CPU", s(x, "cpu")), metric(p, "GPU mem", s(x, "gpu")), metric(p, "RAM", s(x, "ram")), metric(p, "Disk", s(x, "disk"))].spacing(6.0);
    let id = s(x, "id");
    let fix = s(x, "fix");
    let btn: E<'a> = if warn && !fix.is_empty() {
        w::chip(p, "Fix it", true, Sz::Sm, Some(act_v("dash", vec!["fix".into(), fix], "Fixed")))
    } else {
        let a = arr(x, "action");
        let label = a.first().and_then(Value::as_str).unwrap_or("Restart").to_string();
        let what = a.get(1).and_then(Value::as_str).unwrap_or("restart").to_string();
        click(tb(label, 11.0, p.accent), act_v("dash", vec!["action".into(), id.clone(), what], ""))
    };
    let foot = row![t(s(x, "foot"), 11.0, if warn { w::yellow() } else { p.muted }).wrapping(Wrapping::None), Space::new().width(Length::Fill), btn].spacing(8.0).align_y(iced::Center);
    let (bg, line) = if warn { (Color::from_rgb8(0x25, 0x1b, 0x12), Color::from_rgb8(0x6a, 0x4a, 0x1e)) } else { (p.panel, p.line) };
    container(column![top, container(m).padding(pad(10.0, 0.0, 0.0, 0.0)), container(foot).padding(pad(10.0, 0.0, 0.0, 0.0))]).padding(pad(12.0, 14.0, 12.0, 14.0)).width(Length::Fill).style(ui::boxed(bg, p.r(12.0), Some((line, 1.0)))).into()
}

fn small<'a>(p: &Pal, x: &Value) -> E<'a> {
    let top = row![w::mark(&s(x, "mark"), &s(x, "grad"), 22.0), column![tb(s(x, "name"), 14.0, p.text).wrapping(Wrapping::None), t(s(x, "what"), 11.0, p.muted).wrapping(Wrapping::None)].spacing(1.0), Space::new().width(Length::Fill), st(&s(x, "state"), 10.0)].spacing(10.0).align_y(iced::Center);
    container(column![top, container(t(s(x, "foot"), 11.0, p.muted).wrapping(Wrapping::None)).padding(pad(10.0, 0.0, 0.0, 0.0))]).padding(pad(10.0, 12.0, 10.0, 12.0)).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0)))).into()
}

/// Cards in a grid, `cols` across.
pub fn grid<'a>(items: Vec<E<'a>>, cols: usize, gap: f32) -> E<'a> {
    let mut c = Column::new().spacing(gap);
    let mut it = items.into_iter().peekable();
    while it.peek().is_some() {
        let mut r = Row::new().spacing(gap);
        let mut n = 0;
        for _ in 0..cols {
            if let Some(e) = it.next() {
                r = r.push(container(e).width(Length::FillPortion(1)));
                n += 1;
            }
        }
        for _ in n..cols {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        c = c.push(r);
    }
    c.into()
}

fn panel<'a>(p: &Pal, inner: impl Into<E<'a>>) -> E<'a> {
    container(inner).padding(pad(10.0, 14.0, 10.0, 14.0)).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0)))).into()
}

fn view(a: &App) -> E<'_> {
    let p = a.p;
    let d = a.d("dash");
    if d.is_null() {
        return container(w::mu(&p, "Looking at the server…", 14.0)).padding(30.0).into();
    }
    if !b(d, "ok") {
        return container(column![w::h1(&p, "Dashboard"), t(s(d, "error"), 13.0, w::red())].spacing(10.0)).padding(30.0).into();
    }
    let range = a.ui_s("dashRange", "24h");
    let next = match range.as_str() {
        "1h" => "24h",
        "24h" => "7d",
        _ => "1h",
    };
    let fix_all = if f(d, "needsCount") > 0.0 { Some(act("dash", &["fix", "all"], "Fixed what needed it")) } else { None };
    let head = row![
        column![w::h1(&p, "Dashboard"), w::mu(&p, format!("Every service on the {} in one place · {}", s(d, "pc"), s(d, "updated")), 12.0)].spacing(4.0),
        Space::new().width(Length::Fill),
        views(&p, "dash"),
        w::chip(&p, range_label(&range), false, Sz::Md, Some(set("dashRange", json!(next)))),
        w::chip(&p, "Fix everything that needs it", true, Sz::Md, fix_all),
    ]
    .spacing(10.0)
    .align_y(iced::Center);
    let kpis: Vec<E<'_>> = arr(d, "kpis").iter().map(|k| kpi(&p, k)).collect();
    let kpis = { let mut r = Row::new().spacing(10.0); for k in kpis { r = r.push(container(k).width(Length::FillPortion(1))); } r };
    // the services, by the tab picked
    let tab = a.ui_s("dashTab", "All");
    let svcs = arr(d, "services");
    let shown: Vec<&Value> = svcs.iter().filter(|x| tab == "All" || s(x, "group") == tab).collect();
    let tabs = w::tabs(&p, ["All", "AI", "Media", "Home", "Games", "Network"].iter().map(|x| (x.to_string(), tab == *x, Some(set("dashTab", json!(x))))).collect());
    // each service's card is a block of its own (its id from SVC_IDS: the blocks above)
    let mut cards: Vec<(&'static str, E<'_>)> = Vec::new();
    let mut big_ids: Vec<&'static str> = Vec::new();
    let mut small_ids: Vec<&'static str> = Vec::new();
    for x in &shown {
        let Some(id) = SVC_IDS.iter().copied().find(|i| *i == s(x, "id")) else { continue };
        if b(x, "small") {
            small_ids.push(id);
            cards.push((id, small(&p, x)));
        } else {
            big_ids.push(id);
            cards.push((id, service(&p, x)));
        }
    }
    let services = row![cap(&p, format!("Services · {}", svcs.iter().filter(|x| b(x, "on")).count())), Space::new().width(Length::Fill), tabs].align_y(iced::Center);
    let net = panel(&p, column![row![cap(&p, "Network · 24 h"), Space::new().width(Length::Fill), t(s(d, "network.line"), 12.0, p.muted)].align_y(iced::Center), container(draw::spark(nums(d, "network.spark"), w::blue(), true, Length::Fill, 46.0)).padding(pad(6.0, 0.0, 0.0, 0.0))]);
    let pw = panel(&p, column![row![cap(&p, "Power · 24 h"), Space::new().width(Length::Fill), t(s(d, "power.line"), 12.0, p.muted)].align_y(iced::Center), container(draw::bars(&p, nums(d, "power.bars"), Length::Fill, 46.0)).padding(pad(6.0, 0.0, 0.0, 0.0))]);
    let mut ton = Column::new().spacing(6.0).push(row![cap(&p, "Tonight"), Space::new().width(Length::Fill), t(s(d, "tonightFrom"), 12.0, p.muted)].align_y(iced::Center));
    for x in arr(d, "tonight") {
        ton = ton.push(t(s(&x, "text"), 12.0, p.text));
    }
    let ton = panel(&p, ton);
    let foot = row![t("Each card is a block: drag to move, pick what it shows", 12.0, p.muted), Space::new().width(Length::Fill), t("Every service installs and updates from the Hearth Store · the ones that need the system ask for your password once", 12.0, p.muted)];
    // the right column
    let mut needs = Column::new().spacing(10.0).push(w::h2(&p, "Needs a look"));
    let nl = arr(d, "needs");
    if nl.is_empty() {
        needs = needs.push(draw::alert(&p, "ok", row![st("ok", 10.0), tb("Nothing needs a look", 12.0, p.text)].spacing(8.0).align_y(iced::Center)));
    }
    for n in nl.iter().take(3) {
        let fx = s(n, "fix");
        let fix_id = if fx.is_empty() { s(n, "id") } else { fx };
        let mut btns = Row::new().spacing(6.0);
        if !b(n, "nofix") {
            btns = btns.push(w::chip(&p, "Fix it", true, Sz::Sm, Some(act_v("dash", vec!["fix".into(), fix_id], "Fixed"))));
        }
        btns = btns.push(w::chip(&p, "Show me", false, Sz::Sm, Some(show_me(&s(n, "show")))));
        needs = needs.push(draw::alert(&p, "warn", column![row![st("warn", 10.0), tb(s(n, "title"), 12.0, p.text)].spacing(8.0).align_y(iced::Center), t(s(n, "text"), 12.0, p.muted), btns].spacing(8.0)));
    }
    let mut earlier = Column::new().spacing(8.0).push(container(cap(&p, "Earlier today")).padding(pad(10.0, 0.0, 0.0, 0.0)));
    for e in arr(d, "earlier") {
        earlier = earlier.push(row![st(&s(&e, "tone"), 10.0), container(t(s(&e, "text"), 12.0, p.text)).width(Length::Fill), t(s(&e, "when"), 12.0, p.muted)].spacing(8.0).align_y(iced::Center));
    }
    let mut fixes = Column::new().spacing(6.0).push(container(cap(&p, "One-tap fixes")).padding(pad(10.0, 0.0, 0.0, 0.0)));
    for x in arr(d, "fixes") {
        let r = container(row![t(s(&x, "label"), 13.0, p.text), Space::new().width(Length::Fill), t(s(&x, "note"), 12.0, p.muted)].align_y(iced::Center))
            .padding(pad(0.0, 14.0, 0.0, 14.0))
            .center_y(Length::Fixed(36.0))
            .width(Length::Fill)
            .style(ui::boxed(p.panel2, p.r(10.0), Some((p.line, 1.0))));
        fixes = fixes.push(click(r, act_v("dash", vec!["fix".into(), s(&x, "id")], "Done")));
    }
    let temps = column![container(cap(&p, "Temperatures · 24 h")).padding(pad(10.0, 0.0, 0.0, 0.0)), draw::spark_grid(nums(d, "temps24"), w::yellow(), Length::Fill, 80.0), t(s(d, "tempNote"), 12.0, p.muted)].spacing(6.0);
    let phone = b(d, "alerts.phone");
    let quiet = b(d, "alerts.quiet");
    let alerts = column![
        container(cap(&p, "Alerts go to")).padding(pad(10.0, 0.0, 0.0, 0.0)),
        draw::lr(t("Your phone · Hearth notifications", 12.0, p.text), w::sw(&p, phone, Some(act("dash", &["alerts", if phone { "off" } else { "on" }], "")))),
        draw::lr(t(s(d, "alerts.quietLine"), 12.0, p.text), w::sw(&p, quiet, Some(act("dash", &["set", "quiet", if quiet { "off" } else { "on" }], "")))),
    ]
    .spacing(10.0);
    let items: Vec<(&'static str, E<'_>)> = vec![
        ("head", head.into()),
        ("kpis", kpis.into()),
        ("services", services.into()),
        ("network", net),
        ("power", pw),
        ("tonight", ton),
        ("foot", foot.into()),
        ("needs", needs.into()),
        ("earlier", earlier.into()),
        ("fixes", fixes.into()),
        ("temps", temps.into()),
        ("alerts", alerts.into()),
    ];
    let mut items = items;
    items.extend(cards);
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "dash",
        items,
        move |ps| {
            let big: Vec<E<'_>> = big_ids.iter().map(|id| ps.take(id)).collect();
            let little: Vec<E<'_>> = small_ids.iter().map(|id| ps.take(id)).collect();
            let main = column![
                ps.take("head"),
                container(ps.take("kpis")).padding(pad(14.0, 0.0, 0.0, 0.0)),
                container(ps.take("services")).padding(pad(16.0, 0.0, 10.0, 0.0)),
                grid(big, 3, 10.0),
                container(grid(little, 4, 10.0)).padding(pad(10.0, 0.0, 0.0, 0.0)),
                container(row![container(ps.take("network")).width(Length::FillPortion(1)), container(ps.take("power")).width(Length::FillPortion(1)), container(ps.take("tonight")).width(Length::FillPortion(1))].spacing(10.0)).padding(pad(10.0, 0.0, 0.0, 0.0)),
                container(ps.take("foot")).padding(pad(12.0, 0.0, 0.0, 0.0)),
                crate::said(a),
            ];
            let side = column![ps.take("needs"), ps.take("earlier"), ps.take("fixes"), ps.take("temps"), ps.take("alerts")].spacing(10.0);
            row![
                container(iced::widget::scrollable(container(main).padding(pad(18.0, 22.0, 18.0, 22.0)))).width(Length::Fill).height(Length::Fill),
                ui::vline(p.line),
                container(iced::widget::scrollable(side)).padding(18.0).width(Length::Fixed(360.0)).height(Length::Fill),
            ]
            .into()
        },
        Msg::Describe,
    )
}

/// Show me: the part's own page or app.
pub fn show_me(what: &str) -> Msg {
    match what {
        "shield" => crate::open(&["hearth-shield-app"]),
        "backup" => Msg::Go("backup"),
        "vault" => Msg::Go("vault"),
        "drives" => crate::open(&["hearth-settings-app", "--page", "hs-drives"]),
        "gpu" | "" => crate::open(&["hearth-resmon-app"]),
        _ => crate::open(&["hearth-settings-app", "--page", "hs-services"]),
    }
}

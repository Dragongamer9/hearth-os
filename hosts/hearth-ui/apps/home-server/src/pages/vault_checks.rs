//! The Model vault's Checks and alerts (board 02, docs/mockups/server-expansion/02-vault-checks.png): what the weekly
//! Hugging Face check found (a model removed, one changed) with what to do, the last 12 Sundays, what counts as an
//! alert and where it's told; a new model to copy (it asks first, with its cost); apps loading from the vault first;
//! works offline; the upload queue; getting a model back from R2. All from `vault.py status`.

use super::dash::click;
use super::Page;
use crate::{act, act_v, set, App, Msg, E};
use hearth_oss::w::{self, Sz, Tg};
use hearth_server_parts::draw::{self, pad, st};
use hearth_server_parts::{arr, b, f, s, strs};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, Column, Row, Space};
use iced::{Color, Length};
use serde_json::{json, Value};

pub const PAGE: Page = Page { id: "checks", title: "Model vault › Checks and alerts", side: "vault", back: "vault", engines: &["vault"], every: 30, bare: false, view, tags, key: super::no_key };

pub const BLOCKS: PageDef = PageDef {
    id: "checks",
    title: "Checks and alerts",
    blocks: &[
        bl("head", "Checks and alerts, History, Check now", 0, 0, 12, 1),
        bl("found", "What the check found", 0, 1, 6, 5),
        bl("sundays", "The last 12 Sundays", 0, 6, 6, 2),
        bl("rules", "What counts as an alert", 0, 8, 6, 4),
        bl("ask", "A new model to copy?", 6, 1, 6, 3),
        bl("loads", "Apps load from the vault first", 6, 4, 6, 3),
        bl("queue", "Upload queue", 6, 7, 6, 3),
        bl("back", "Getting a model back from R2", 6, 10, 6, 2),
    ],
};

fn tags(a: &App) -> crate::Tags {
    let d = a.d("vault");
    let n = arr(d, "alerts").len() + arr(d, "asks").len();
    if d.is_null() {
        Vec::new()
    } else if n > 0 {
        vec![(format!("{n} {} a look", if n == 1 { "thing needs" } else { "things need" }), Tg::Warn)]
    } else {
        vec![("Nothing needs a look".into(), Tg::Ok)]
    }
}

fn cap<'a>(p: &Pal, x: impl Into<String>) -> E<'a> {
    tb(x.into().to_uppercase(), 11.0, p.muted).wrapping(Wrapping::None).into()
}

fn alert_card<'a>(p: &Pal, x: &Value) -> E<'a> {
    let id = s(x, "id");
    let removed = s(x, "kind") == "removed";
    let head = row![w::tag(p, if removed { "Removed" } else { "Changed" }, if removed { Tg::Bad } else { Tg::Warn }), tb(s(x, "title"), 16.0, p.text), Space::new().width(Length::Fill), t(s(x, "found"), 12.0, p.muted)].spacing(10.0).align_y(iced::Center);
    let mut c = Column::new().spacing(10.0).push(head);
    if removed {
        let text = s(x, "text");
        let (a_, rest) = text.split_once("Your copy is safe").unwrap_or((text.as_str(), ""));
        c = c.push(row![t(a_.to_string(), 12.0, p.text), tb("Your copy is safe", 12.0, p.text), t(rest.to_string(), 12.0, p.text)].wrap());
        let moved = strs(x, "moved");
        if !moved.is_empty() {
            c = c.push(t(format!("Found on Hugging Face: {}", moved.join(", ")), 12.0, w::green()));
        }
        c = c.push(row![
            w::chip(p, "Keep it forever", true, Sz::Sm, Some(act_v("vault", vec!["decide".into(), id.clone(), "keep-forever".into()], ""))),
            w::chip(p, "Keep a second copy in R2 Standard", false, Sz::Sm, Some(act_v("vault", vec!["decide".into(), id.clone(), "second-copy".into()], ""))),
            w::chip(p, "Find where it moved", false, Sz::Sm, Some(act_v("vault", vec!["decide".into(), id.clone(), "find".into()], ""))),
            Space::new().width(Length::Fill),
            container(t(s(x, "loads"), 12.0, p.muted)).width(Length::Fixed(240.0)),
        ]
        .spacing(8.0)
        .align_y(iced::Center));
        draw::alert(p, "bad", c).into()
    } else {
        c = c.push(t(s(x, "text"), 12.0, p.text));
        let inner = column![draw::lr(t(s(x, "yours"), 12.0, p.muted), t(s(x, "yoursFiles"), 12.0, p.text)), draw::lr(t(s(x, "online"), 12.0, p.muted), t(s(x, "onlineFiles"), 12.0, w::yellow()))].spacing(6.0);
        c = c.push(container(inner).padding(pad(10.0, 12.0, 10.0, 12.0)).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0)))));
        c = c.push(row![
            w::chip(p, "Keep both versions", true, Sz::Sm, Some(act_v("vault", vec!["decide".into(), id.clone(), "keep-both".into()], ""))),
            w::chip(p, "Keep mine only", false, Sz::Sm, Some(act_v("vault", vec!["decide".into(), id.clone(), "keep-mine".into()], ""))),
            w::chip(p, format!("Try the new one in {}", s(x, "app")), false, Sz::Sm, Some(act_v("vault", vec!["decide".into(), id.clone(), "try-new".into()], ""))),
        ]
        .spacing(8.0));
        draw::alert(p, "warn", c).into()
    }
}

fn sundays<'a>(p: &Pal, d: &Value) -> E<'a> {
    let h = arr(d, "history");
    let mut r = Row::new().spacing(6.0);
    let mut month = String::new();
    for x in &h {
        let day = s(x, "day");
        let (m, n) = day.split_once(' ').unwrap_or(("", day.as_str()));
        let label = if m != month { day.clone() } else { n.to_string() };
        month = m.to_string();
        let (bg, line, fg) = match s(x, "tone").as_str() {
            "bad" => (Color::from_rgb8(0x3d, 0x17, 0x17), Color::from_rgb8(0x6a, 0x2a, 0x2a), w::red()),
            "warn" => (Color::from_rgb8(0x3d, 0x2a, 0x14), Color::from_rgb8(0x6a, 0x4a, 0x1e), w::yellow()),
            _ => (Color::from_rgb8(0x21, 0x36, 0x1b), Color::from_rgb8(0x3b, 0x5a, 0x2e), w::green()),
        };
        r = r.push(container(t(label, 10.0, fg).wrapping(Wrapping::None)).center_x(Length::FillPortion(1)).height(Length::Fixed(40.0)).align_y(iced::alignment::Vertical::Bottom).padding(pad(0.0, 0.0, 4.0, 0.0)).style(ui::boxed(bg, 6.0, Some((line, 1.0)))));
    }
    for _ in h.len()..12 {
        r = r.push(container(Space::new()).width(Length::FillPortion(1)).height(Length::Fixed(40.0)).style(ui::boxed(p.panel2, 6.0, Some((p.line, 1.0)))));
    }
    column![cap(p, "The last 12 Sundays"), r, t(s(d, "historyNote"), 12.0, p.muted)].spacing(8.0).into()
}

fn rules<'a>(p: &Pal, d: &Value) -> E<'a> {
    let rule = |label: &str, key: &str| -> E<'a> {
        let on = b(d, &format!("alertRules.{key}"));
        draw::lr(t(label.to_string(), 12.0, p.text), w::sw(p, on, Some(act_v("vault", vec!["set".into(), format!("alert.{key}"), if on { "off".into() } else { "on".into() }], ""))))
    };
    let tell = strs(d, "tell");
    let tog = |k: &str| -> Option<Msg> {
        let mut v = tell.clone();
        if v.iter().any(|x| x == k) {
            v.retain(|x| x != k);
        } else {
            v.push(k.to_string());
        }
        Some(act_v("vault", vec!["set".into(), "tell".into(), v.join(",")], ""))
    };
    // Tell me on: each its own (more than one can be on)
    let mut seg = Row::new();
    for (k, l) in [("phone", "Phone"), ("hearth", "Hearth notifications"), ("email", "Email")] {
        let on = tell.iter().any(|x| x == k);
        let txt = if on { tb(l, 12.0, p.ink) } else { t(l, 12.0, p.muted) };
        seg = seg.push(click(container(txt.wrapping(Wrapping::None)).padding(pad(5.0, 9.0, 5.0, 9.0)).style(ui::boxed(if on { p.accent } else { Color::TRANSPARENT }, 0.0, None)), tog(k).unwrap()));
    }
    let seg = container(seg).clip(true).style(ui::boxed(Color::TRANSPARENT, p.r(8.0), Some((p.line, 1.0))));
    let c = column![
        rule("A model is removed or made private", "removed"),
        rule("Its files change", "files"),
        rule("Its licence line changes", "licence"),
        rule("It now needs a sign-in to download (gated)", "gated"),
        draw::lr(t("Tell me on", 12.0, p.text), seg),
    ]
    .spacing(12.0);
    column![cap(p, "What counts as an alert"), container(c).padding(pad(12.0, 16.0, 12.0, 16.0)).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0))))].spacing(8.0).into()
}

fn ask_card<'a>(a: &'a App, x: &Value) -> E<'a> {
    let p = a.p;
    let id = s(x, "id");
    let where_ = a.ui_s(&format!("askWhere.{id}"), "drive+ia");
    let pick = |w_: &'static str, l: &str| (l.to_string(), where_ == w_, Some(set(&format!("askWhere.{id}"), json!(w_))));
    let head = row![w::mark("＋", "or", 44.0), column![w::h2(&p, "A new model to copy?"), t(format!("{} added one to its list", s(x, "app")), 12.0, p.muted)].spacing(2.0), Space::new().width(Length::Fill), w::tag(&p, "Asks first", Tg::Acc)].spacing(10.0).align_y(iced::Center);
    let name = s(x, "name");
    let (n1, n2) = name.split_once(" · ").map(|(a_, b_)| (a_.to_string(), format!(" · {b_}"))).unwrap_or((name.clone(), String::new()));
    let adds = if where_ == "drive+std" { format!("{} (Standard)", s(x, "addsStd")) } else if where_ == "drive" { "nothing on R2 (the vault drive only)".to_string() } else { s(x, "adds") };
    let c = column![
        head,
        draw::lr(row![tb(n1, 12.0, p.text), t(n2, 12.0, p.text)], t(s(x, "licence"), 12.0, p.muted)),
        draw::lr(t("Size · version", 12.0, p.muted), t(s(x, "size"), 12.0, p.text)),
        draw::lr(t("Copy to", 12.0, p.muted), w::seg(&p, vec![pick("drive", "Vault drive only"), pick("drive+ia", "Drive + R2 Infrequent"), pick("drive+std", "Drive + R2 Standard")])),
        draw::lr(t("Adds each month", 12.0, p.muted), t(adds, 12.0, p.text)),
        row![
            w::chip(&p, "Copy it", true, Sz::Md, Some(act_v("vault", vec!["decide".into(), id.clone(), "copy".into(), where_.clone()], "Copying it in the background"))),
            w::chip(&p, "Not now", false, Sz::Md, Some(act_v("vault", vec!["decide".into(), id.clone(), "not-now".into()], ""))),
            w::ghost(&p, "Never ask for this one", Sz::Md, Some(act_v("vault", vec!["decide".into(), id.clone(), "never".into()], ""))),
        ]
        .spacing(8.0),
    ]
    .spacing(10.0);
    let bg = if p.dark { Color::from_rgb8(0x2a, 0x1d, 0x14) } else { Pal::mix(p.panel, p.accent, 0.06) };
    container(c).padding(16.0).width(Length::Fill).style(ui::boxed(bg, p.r(12.0), Some((p.accent, 1.0)))).into()
}

fn step<'a>(p: &Pal, title: &str, lines: Vec<(String, Color)>, green: bool) -> E<'a> {
    let mut c = Column::new().spacing(4.0).align_x(iced::Center).push(tb(title.to_string(), 15.0, p.text));
    for (l, col) in lines {
        c = c.push(t(l, 12.0, col));
    }
    let line = if green { Color::from_rgb8(0x3b, 0x5a, 0x2e) } else { p.line };
    container(c).padding(14.0).center_x(Length::FillPortion(1)).style(ui::boxed(p.panel2, p.r(12.0), Some((line, 1.0)))).into()
}

fn view<'a>(a: &'a App) -> E<'a> {
    let p = a.p;
    let d = a.d("vault");
    if d.is_null() {
        return container(w::mu(&p, "Looking at the vault…", 14.0)).padding(30.0).into();
    }
    if !b(d, "ok") {
        return container(t(s(d, "error"), 13.0, w::red())).padding(30.0).into();
    }
    let history = a.ui_b("checksHistory");
    let head = row![
        column![w::h1(&p, "Checks and alerts"), w::mu(&p, "Every Sunday at 3:00 AM Hearth asks Hugging Face whether each model is still there and still the same. Your copy never changes by itself.", 12.0)].spacing(4.0),
        Space::new().width(Length::Fill),
        w::chip(&p, if history { "Hide the history" } else { "History" }, false, Sz::Md, Some(set("checksHistory", json!(!history)))),
        w::chip(&p, "Check now", true, Sz::Md, Some(act("vault", &["check"], "Checked every model"))),
    ]
    .spacing(10.0)
    .align_y(iced::Center);
    let mut found = Column::new().spacing(12.0);
    for x in arr(d, "alerts") {
        found = found.push(alert_card(&p, &x));
    }
    if arr(d, "alerts").is_empty() {
        found = found.push(draw::alert(&p, "ok", row![st("ok", 10.0), t("The last check found nothing to look at: every kept model is still online, with the same files", 12.0, p.text)].spacing(8.0).align_y(iced::Center)));
    }
    if history {
        let mut hc = Column::new().spacing(4.0);
        for x in arr(d, "history").iter().rev() {
            hc = hc.push(row![st(&s(x, "tone"), 10.0), t(s(x, "day"), 12.0, p.text)].spacing(8.0).align_y(iced::Center));
        }
        found = found.push(draw::bx(&p, hc));
    }
    let mut ask = Column::new().spacing(10.0);
    for x in arr(d, "asks").iter().take(1) {
        ask = ask.push(ask_card(a, x));
    }
    if arr(d, "asks").is_empty() {
        ask = ask.push(draw::bx(&p, t(if b(d, "how.askFirst") { "No new model to copy: when one of the apps adds one, it's asked here first, with its cost" } else { "New models are copied by themselves (Ask before copying a new model is off)" }, 12.0, p.muted)));
    }
    let used = hearth_server_parts::at(d, "used");
    let used_line = if f(used, "all") > 0.0 { format!("used {} of {} times", f(used, "vault"), f(used, "all")) } else { "used every time it's there".to_string() };
    let loads = column![
        cap(&p, "Apps load from the vault first"),
        row![
            step(&p, "An app needs a model", vec![("Colibri, Ollama, Forge, Listen, Kokoro, Imagine, Ithil".into(), p.muted)], false),
            tb("›", 22.0, p.accent),
            step(&p, "1 · Vault drive", vec![("here, no internet needed".into(), p.muted), (used_line, w::green())], true),
            tb("›", 22.0, p.accent),
            step(&p, "2 · Your R2", vec![("if this PC lost it".into(), p.muted), ("checked by checksum".into(), p.muted)], false),
            tb("›", 22.0, p.accent),
            step(&p, "3 · Hugging Face", vec![("only if it's in neither".into(), p.muted), ("then copied (asks)".into(), p.muted)], false),
        ]
        .spacing(12.0)
        .align_y(iced::Center),
        draw::bx(&p, row![st(if b(d, "offline.ok") { "ok" } else { "off" }, 10.0), tb("Works offline", 12.0, p.text), Space::new().width(Length::Fill), t(s(d, "offline.line"), 12.0, p.muted), click(tb("Try it", 12.0, p.accent), act("vault", &["offline-test"], ""))].spacing(8.0).align_y(iced::Center)),
    ]
    .spacing(10.0);
    let mut q = Column::new().spacing(0.0);
    for (i, x) in arr(d, "queue").into_iter().enumerate() {
        let on = b(&x, "on");
        let pct = f(&x, "pct");
        let right = if on { format!("{pct}% · {}", s(&x, "left")) } else if pct == 0.0 && i < 2 { format!("next · {}", s(&x, "left")) } else { s(&x, "left") };
        q = q.push(draw::under(&p, row![st(if on { "ok" } else { "off" }, 10.0), container(t(s(&x, "name"), 13.0, p.text)).width(Length::Fill), w::meter(&p, (pct / 100.0) as f32, None, 220.0, 6.0), container(t(right, 13.0, p.muted)).width(Length::Fixed(150.0)).align_x(iced::alignment::Horizontal::Right)].spacing(12.0).align_y(iced::Center), false));
    }
    let paused = b(d, "paused");
    q = q.push(container(row![t(s(d, "queueLine"), 12.0, p.muted), Space::new().width(Length::Fill), w::chip(&p, if paused { "Carry on" } else { "Pause all" }, false, Sz::Sm, Some(act("vault", &["set", "paused", if paused { "off" } else { "on" }], "")))].align_y(iced::Center)).padding(pad(10.0, 0.0, 6.0, 0.0)));
    let queue = column![cap(&p, "Upload queue · slow, in the background"), container(q).padding(pad(6.0, 14.0, 6.0, 14.0)).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0))))].spacing(8.0);
    let gb = hearth_server_parts::at(d, "getback");
    let back: E<'_> = if gb.is_object() {
        draw::bx(&p, row![tb("Getting a model back from R2", 12.0, p.text), Space::new().width(Length::Fill), t(s(gb, "line"), 12.0, p.muted), w::chip(&p, "Get it back", false, Sz::Sm, Some(act_v("vault", vec!["getback".into(), s(gb, "id")], "Back in the vault")))].spacing(10.0).align_y(iced::Center)).into()
    } else {
        draw::bx(&p, row![tb("Getting a model back from R2", 12.0, p.text), Space::new().width(Length::Fill), t("Nothing kept on R2 yet", 12.0, p.muted)].spacing(10.0)).into()
    };
    let items: Vec<(&'static str, E<'_>)> = vec![("head", head.into()), ("found", found.into()), ("sundays", sundays(&p, d)), ("rules", rules(&p, d)), ("ask", ask.into()), ("loads", loads.into()), ("queue", queue.into()), ("back", back)];
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "checks",
        items,
        move |ps| {
            let left = column![ps.take("found"), container(ps.take("sundays")).padding(pad(6.0, 0.0, 0.0, 0.0)), container(ps.take("rules")).padding(pad(6.0, 0.0, 0.0, 0.0))].spacing(12.0).width(Length::FillPortion(1));
            let right = column![ps.take("ask"), container(ps.take("loads")).padding(pad(6.0, 0.0, 0.0, 0.0)), container(ps.take("queue")).padding(pad(6.0, 0.0, 0.0, 0.0)), ps.take("back")].spacing(12.0).width(Length::FillPortion(1));
            let body = column![ps.take("head"), container(row![left, right].spacing(18.0)).padding(pad(14.0, 0.0, 0.0, 0.0)), crate::said(a)];
            container(iced::widget::scrollable(container(body).padding(pad(18.0, 22.0, 18.0, 22.0)))).width(Length::Fill).height(Length::Fill).into()
        },
        Msg::Describe,
    )
}

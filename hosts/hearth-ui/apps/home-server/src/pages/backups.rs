//! Backups for every device (board 06, docs/mockups/server-expansion/06-backups.png; checklist
//! docs/mockups/checklists/server-backups.md): each device's last night, size, versions and 14 nights; the picked
//! device's plan (what's backed up, when, kept how long, locked, encrypted); restore a file or folder, the whole device,
//! or from the Recovery USB; last night step by step; space by device, the second copy on R2, the monthly restore
//! test, the Backups app on each device. From `backups.py status`.

use super::dash::click;
use super::Page;
use crate::{act, act_v, set, App, Msg, E};
use hearth_oss::w::{self, Sz, Tg};
use hearth_server_parts::draw::{self, pad, st};
use hearth_server_parts::{arr, b, f, s};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, Column, Row, Space};
use iced::{Color, Length};
use serde_json::{json, Value};

pub const PAGE: Page = Page { id: "backup", title: "Backups", side: "backup", back: "home", engines: &["backups"], every: 60, bare: false, view, tags, key: super::no_key };

pub const BLOCKS: PageDef = PageDef {
    id: "backup",
    title: "Backups",
    blocks: &[
        bl("head", "Backups, Back up all now, Add a device", 0, 0, 9, 1),
        bl("devices", "Every device and its last 14 nights", 0, 1, 9, 2),
        bl("plan", "What's backed up, when, kept how long", 0, 3, 4, 6),
        bl("restore", "Restore", 4, 3, 5, 6),
        bl("steps", "Last night, step by step", 0, 9, 9, 2),
        bl("space", "Space for backups", 9, 0, 3, 4),
        bl("second", "A second copy away from home", 9, 4, 3, 2),
        bl("phone", "Phone and Xbox", 9, 6, 3, 2),
        bl("checks", "Checks and the Backups app", 9, 8, 3, 4),
    ],
};

fn tags(a: &App) -> crate::Tags {
    let d = a.d("backups");
    if d.is_null() || !b(d, "ok") {
        return Vec::new();
    }
    vec![(s(d, "tag"), if s(d, "tagTone") == "ok" { Tg::Ok } else { Tg::Warn })]
}

fn cap<'a>(p: &Pal, x: impl Into<String>) -> E<'a> {
    container(tb(x.into().to_uppercase(), 11.0, p.muted).wrapping(Wrapping::None)).padding(pad(14.0, 0.0, 4.0, 0.0)).into()
}

/// The device's outline (.shape): a handheld wide and rounded, a laptop, a PC square; the one away in the accent.
fn shape<'a>(p: &Pal, kind: &str, away: bool) -> E<'a> {
    let (w_, h, r) = match kind {
        "handheld" => (50.0, 30.0, 10.0),
        "pc" => (36.0, 36.0, 6.0),
        _ => (44.0, 30.0, 6.0),
    };
    container(Space::new()).width(Length::Fixed(w_)).height(Length::Fixed(h)).style(ui::boxed(Color::TRANSPARENT, r, Some((if away { p.accent } else { p.muted }, 2.0)))).into()
}

fn device_card<'a>(p: &Pal, x: &Value, sel: bool) -> E<'a> {
    let away = s(x, "line").contains("away") || s(x, "line").contains("work");
    let top = row![shape(p, &s(x, "kind"), away), column![tb(s(x, "name"), 15.0, p.text), t(s(x, "line"), 12.0, p.muted)].spacing(1.0), Space::new().width(Length::Fill), st(&s(x, "state"), 10.0)].spacing(10.0).align_y(iced::Center);
    let mut strip = Row::new().spacing(4.0);
    for n in arr(x, "nights") {
        let c = match n.as_str().unwrap_or("") {
            "ok" => Color::from_rgb8(0x3b, 0x5a, 0x2e),
            "late" => Color::from_rgb8(0x6a, 0x4a, 0x1e),
            _ => p.panel2,
        };
        strip = strip.push(container(Space::new()).width(Length::FillPortion(1)).height(Length::Fixed(18.0)).style(ui::boxed(c, 3.0, None)));
    }
    let c = column![top, draw::lr(t("Last", 12.0, p.muted), t(s(x, "last"), 12.0, p.text)), draw::lr(t("Whole device", 12.0, p.muted), t(s(x, "whole"), 12.0, p.text)), strip].spacing(8.0);
    let (bg, line) = if sel { (Pal::mix(p.panel, p.accent, 0.08), (p.accent, 2.0)) } else { (p.panel, (p.line, 1.0)) };
    click(container(c).padding(14.0).width(Length::Fill).style(ui::boxed(bg, p.r(12.0), Some(line))), set("backupPick", json!(s(x, "id"))))
}

fn plan<'a>(a: &'a App, x: &Value) -> E<'a> {
    let p = a.p;
    let id = s(x, "id");
    let pl = hearth_server_parts::at(x, "plan");
    let sizes = hearth_server_parts::at(x, "sizes");
    let sw = |key: &str| -> E<'a> {
        let on = b(pl, key);
        w::sw(&p, on, Some(act_v("backups", vec!["set".into(), id.clone(), key.into(), if on { "off".into() } else { "on".into() }], "")))
    };
    let what = |label: &str, key: &str, size: String| -> E<'a> { row![container(t(label.to_string(), 12.0, p.text)).width(Length::Fill), t(size, 12.0, p.muted), container(sw(key)).padding(pad(0.0, 0.0, 0.0, 14.0))].align_y(iced::Center).into() };
    let size = |k: &str, d: &str| -> String {
        let v = s(sizes, k);
        if v.is_empty() { d.to_string() } else { v }
    };
    let at = s(pl, "at");
    let at_words = { let (h, m) = at.split_once(':').unwrap_or(("2", "00")); let h: u32 = h.parse().unwrap_or(2); format!("{}:{m} {}", if h % 12 == 0 { 12 } else { h % 12 }, if h < 12 { "AM" } else { "PM" }) };
    let next_at = { let h: u32 = at.split(':').next().and_then(|x| x.parse().ok()).unwrap_or(2); format!("{:02}:00", (h + 1) % 24) };
    let nights = f(pl, "nights") as i64;
    let weeks = s(pl, "weeks");
    let seg = |items: Vec<(&str, bool, &str, String)>| -> E<'a> { w::seg(&p, items.into_iter().map(|(l, on, k, v)| (l.to_string(), on, Some(act_v("backups", vec!["set".into(), id.clone(), k.into(), v], "")))).collect()) };
    let c = column![
        row![w::h2(&p, s(x, "name")), Space::new().width(Length::Fill), w::chip(&p, "Back up now", false, Sz::Sm, Some(act_v("backups", vec!["now".into(), id.clone()], "")))].align_y(iced::Center),
        cap(&p, "What's backed up"),
        what("Your Home (documents, pictures, settings)", "home", size("home", "")),
        what("Game saves (Steam, Hearth Saves, emulators)", "saves", size("saves", "")),
        what("Hearth's settings and apps list", "settings", size("settings", "")),
        what("The whole device (to put it all back)", "system", size("system", "")),
        what("Installed games", "games", "the Game cache has them".into()),
        cap(&p, "When"),
        draw::lr(t("Every night", 12.0, p.text), row![w::field(&p, at_words, 0.0, Some(act_v("backups", vec!["set".into(), id.clone(), "at".into(), next_at], ""))), t("plugged in, or 50% battery", 12.0, p.muted)].spacing(6.0).align_y(iced::Center)),
        draw::lr(t("Also when it comes home after a day away", 12.0, p.text), sw("away")),
        cap(&p, "Kept how long"),
        draw::lr(t("Every night for", 12.0, p.text), seg(vec![("7 days", nights == 7, "nights", "7".into()), ("30 days", nights == 30, "nights", "30".into()), ("90 days", nights == 90, "nights", "90".into())])),
        draw::lr(t("Then one a week for", 12.0, p.text), seg(vec![("3 months", weeks == "3m", "weeks", "3m".into()), ("1 year", weeks == "1y", "weeks", "1y".into()), ("Forever", weeks == "forever", "weeks", "forever".into())])),
        draw::lr(t(format!("Locked so it can't be changed or deleted from the {}", s(x, "name").replace("ROG ", "")), 12.0, p.text), sw("lock")),
        draw::lr(t("Encrypted · only you have the key", 12.0, p.text), w::tag(&p, "On", Tg::Ok)),
    ]
    .spacing(8.0);
    container(c).padding(16.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0)))).into()
}

fn restore<'a>(a: &'a App, d: &'a Value, x: &Value) -> E<'a> {
    let p = a.p;
    let id = s(x, "id");
    let name = s(x, "name");
    let mode = a.ui_s("restoreMode", "file");
    let modes = w::seg(&p, vec![("A file or folder".into(), mode == "file", Some(set("restoreMode", json!("file")))), ("The whole device".into(), mode == "device", Some(set("restoreMode", json!("device")))), ("From the Recovery USB".into(), mode == "usb", Some(set("restoreMode", json!("usb"))))]);
    let nights = arr(d, "nights");
    let pick_n = a.ui_s("restoreNight", &nights.first().map(|n| s(n, "id")).unwrap_or_default());
    let night = nights.iter().find(|n| s(n, "id") == pick_n).cloned().unwrap_or(Value::Null);
    let q = a.ui_s("restoreSearch", "");
    let find = act_v("backups", vec!["files".into(), id.clone(), pick_n.clone(), q.clone()], "");
    let mut c = Column::new().spacing(0.0).push(row![w::h2(&p, "Restore"), Space::new().width(Length::Fill), modes].align_y(iced::Center));
    let night_label = if night.is_null() { "No night yet ▾".to_string() } else { format!("{} {} ▾", s(&night, "label"), if s(&night, "label") == "Tonight" { s(&night, "at") } else { String::new() }).replace("  ", " ") };
    c = c.push(container(row![
        container(w::search(&p, "Search a night's backup", &q, |x| Msg::Text("restoreSearch".into(), x))).width(Length::Fill),
        w::field(&p, format!("{name} ▾"), 0.0, None),
        w::field(&p, night_label, 0.0, Some(find.clone())),
        w::chip(&p, "Look", false, Sz::Sm, Some(find.clone())),
    ].spacing(8.0).align_y(iced::Center)).padding(pad(12.0, 0.0, 0.0, 0.0)));
    c = c.push(container(t(format!("Pick a night · {} versions", f(d, "versions")), 12.0, p.muted)).padding(pad(10.0, 0.0, 6.0, 0.0)));
    let mut chips = Row::new().spacing(4.0);
    for n in nights.iter() {
        let on = s(n, "id") == pick_n;
        chips = chips.push(w::chip(&p, s(n, "label"), on, Sz::Sm, Some(Msg::Do(Box::new(crate::Act { engine: "backups", args: vec!["files".into(), id.clone(), s(n, "id"), q.clone()], ..Default::default() })))));
    }
    c = c.push(chips.wrap());
    // the night's files (backups.py files), the first folder that matches and what's in it
    let res = a.ui.get("last.backups.files").cloned().unwrap_or(Value::Null);
    let rows = arr(&res, "rows");
    let picked = a.ui_s("restorePath", &rows.iter().find(|r| !b(r, "dir")).map(|r| s(r, "path")).unwrap_or_default());
    let mut list = Column::new();
    for (i, r) in rows.iter().enumerate() {
        let dir = b(r, "dir");
        let ic = if dir { w::yellow() } else { w::blue() };
        let mut line = Row::new().spacing(10.0).align_y(iced::Center).push(w::swatch(ic, 14.0, 14.0, 4.0)).push(container(t(s(r, "name"), 13.0, p.text)).width(Length::Fill)).push(t(s(r, "line"), 13.0, p.muted));
        if b(r, "different") {
            line = line.push(w::tag(&p, "Different now", Tg::Warn));
        }
        let lit = dir && i == 0;
        let bg = if lit { Color::from_rgb8(0x3b, 0x2a, 0x1d) } else if s(r, "path") == picked { Pal::mix(p.panel, p.accent, 0.08) } else { Color::TRANSPARENT };
        let el = container(line).padding(pad(7.0, 10.0, 7.0, if dir { 10.0 } else { 34.0 })).width(Length::Fill).style(ui::boxed(bg, 0.0, None));
        list = list.push(click(el, set("restorePath", json!(s(r, "path")))));
        if i + 1 < rows.len() {
            list = list.push(container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(ui::boxed(Pal::mix(p.panel, p.line, 0.6), 0.0, None)));
        }
    }
    if rows.is_empty() {
        list = list.push(container(t("Type what you're looking for (a save, a folder) or pick a night: its files show here", 12.0, p.muted)).padding(10.0));
    }
    let fname = picked.rsplit('/').next().unwrap_or("").to_string();
    let restored = format!("{}/Restored from backups", hearth_ui::paths::home().display());
    if mode == "file" {
        c = c.push(container(container(list).clip(true).width(Length::Fill).style(ui::boxed(Color::TRANSPARENT, p.r(10.0), Some((p.line, 1.0))))).padding(pad(10.0, 0.0, 0.0, 0.0)));
        let night_id = if s(&res, "night").is_empty() { pick_n.clone() } else { s(&res, "night") };
        let can = !picked.is_empty() && !night_id.is_empty();
        c = c.push(container(row![
            w::chip(&p, if fname.is_empty() { "Put it back".to_string() } else { format!("Put back {fname}") }, true, Sz::Md, if can { Some(act_v("backups", vec!["restore".into(), id.clone(), night_id.clone(), picked.clone(), "back".into()], "")) } else { None }),
            w::chip(&p, "Put it beside the new one", false, Sz::Md, if can { Some(act_v("backups", vec!["restore".into(), id.clone(), night_id.clone(), picked.clone(), "beside".into()], "")) } else { None }),
            w::chip(&p, "Open", false, Sz::Md, Some(crate::open(&["hearth-files-app", restored.as_str()]))),
        ].spacing(8.0)).padding(pad(12.0, 0.0, 0.0, 0.0)));
    }
    // the whole device, and the Recovery USB
    let usb = mode == "usb";
    c = c.push(cap(&p, if usb { "From the Recovery USB" } else { "The whole device" }));
    let words: E<'a> = if usb {
        t(format!("When the {name} won't start: start it from the Recovery USB and pick \"Restore from my home server\". It finds this server over Wi-Fi or Tailscale and puts back the night you pick. That choice comes with the next Recovery USB."), 12.0, p.text).into()
    } else {
        row![t(format!("Put the {name} back the way it was on a night you pick: Hearth OS, apps, settings and files. On the {}: Settings › Updates & recovery › Restore from the server, or start it from the ", name.replace("ROG ", "")), 12.0, p.text), tb("Recovery USB", 12.0, p.text), t(" and pick \"Restore from my home server\" (it finds the server over Wi-Fi or Tailscale).", 12.0, p.text)].wrap().into()
    };
    c = c.push(draw::bx(&p, words));
    let code = a.ui.get("last.backups.code").cloned().unwrap_or(Value::Null);
    c = c.push(container(row![
        w::chip(&p, "Make a Recovery USB", false, Sz::Md, Some(crate::open(&["hearth-settings-app", "--page", "backup"]))),
        w::chip(&p, "Restore to a new device", false, Sz::Md, Some(act_v("backups", vec!["code".into(), id.clone()], ""))),
        Space::new().width(Length::Fill),
        t("about 25 min at home", 12.0, p.muted),
    ].spacing(8.0).align_y(iced::Center)).padding(pad(10.0, 0.0, 0.0, 0.0)));
    if !s(&code, "join").is_empty() {
        c = c.push(container(draw::bx(&p, column![t(s(&code, "said"), 12.0, p.text), t(s(&code, "join"), 11.0, p.muted).font(iced::Font::MONOSPACE)].spacing(6.0))).padding(pad(8.0, 0.0, 0.0, 0.0)));
    }
    container(c).padding(16.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0)))).into()
}

fn view<'a>(a: &'a App) -> E<'a> {
    let p = a.p;
    let d = a.d("backups");
    if d.is_null() {
        return container(w::mu(&p, "Looking at the backups…", 14.0)).padding(30.0).into();
    }
    if !b(d, "ok") {
        return container(column![w::h1(&p, "Backups"), t(s(d, "error"), 13.0, w::red())].spacing(10.0)).padding(30.0).into();
    }
    let adding = a.ui_b("backupAdd");
    let head = row![
        column![w::h1(&p, "Backups"), w::mu(&p, "Every device backs up to the server each night over Tailscale, wherever it is · only what changed is sent", 12.0)].spacing(4.0),
        Space::new().width(Length::Fill),
        w::chip(&p, "Back up all now", false, Sz::Md, Some(act("backups", &["now", "all"], ""))),
        w::chip(&p, "＋ Add a device", true, Sz::Md, Some(set("backupAdd", json!(!adding)))),
    ]
    .spacing(10.0)
    .align_y(iced::Center);
    let devs = arr(d, "devices");
    let pick = a.ui_s("backupPick", &s(d, "pick"));
    let sel = devs.iter().find(|x| s(x, "id") == pick).cloned().or_else(|| devs.first().cloned());
    let cards: Vec<E<'_>> = devs.iter().map(|x| device_card(&p, x, sel.as_ref().map(|y| s(y, "id")) == Some(s(x, "id")))).collect();
    let mut dv = Column::new().spacing(6.0).push(super::dash::grid(cards, 4, 12.0)).push(t(s(d, "nightsNote"), 12.0, p.muted));
    if !b(d, "on") {
        dv = dv.push(draw::alert(&p, "warn", row![t("Backups for every device are off: nothing is backed up each night until you turn them on", 12.0, p.text), Space::new().width(Length::Fill), w::chip(&p, "Turn them on", true, Sz::Sm, Some(act("backups", &["on"], "")))].spacing(10.0).align_y(iced::Center)));
    }
    if adding {
        let name = a.ui_s("newDevice", "");
        let kind = a.ui_s("newKind", "handheld");
        let kinds = w::seg(&p, [("handheld", "Handheld"), ("laptop", "Laptop"), ("pc", "PC"), ("other", "Other")].iter().map(|(k, l)| (l.to_string(), kind == *k, Some(set("newKind", json!(k))))).collect());
        let joined = a.ui.get("last.backups.add").cloned().unwrap_or(Value::Null);
        let mut ac = column![
            w::h2(&p, "Add a device"),
            row![w::input(&p, "its name, like rog-ally", &name, 13.0, |x| Msg::Text("newDevice".into(), x)).width(Length::Fixed(260.0)), kinds, w::chip(&p, "Make its code", true, Sz::Sm, Some(act_v("backups", vec!["add".into(), name.clone(), kind.clone()], "")))].spacing(10.0).align_y(iced::Center),
            t("The device gets a code (shown once) and its own key; it backs up here over Tailscale from tonight. Its key stays in this PC's password store and, on the device, in a file only the system can read.", 12.0, p.muted),
        ]
        .spacing(10.0);
        if !s(&joined, "join").is_empty() {
            ac = ac.push(draw::bx(&p, column![t(s(&joined, "said"), 12.0, p.text), t(s(&joined, "join"), 11.0, p.muted).font(iced::Font::MONOSPACE)].spacing(6.0)));
        }
        dv = dv.push(container(ac).padding(16.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.accent, 1.0)))));
    }
    let (plan_el, restore_el): (E<'_>, E<'_>) = match &sel {
        Some(x) => (plan(a, x), restore(a, d, x)),
        None => (draw::bx(&p, t("Add a device: each one backs up here every night", 12.0, p.muted)).into(), Space::new().into()),
    };
    let mut steps = Row::new().spacing(10.0);
    for x in arr(d, "steps") {
        let at = hearth_ui::time::clock(f(&x, "at") as i64);
        steps = steps.push(container(draw::bx(&p, row![tb(at.trim_end_matches(" AM").trim_end_matches(" PM").to_string(), 12.0, p.text), t(s(&x, "what"), 12.0, p.muted)].spacing(6.0))).width(Length::FillPortion(1)));
    }
    let steps = container(column![row![tb("LAST NIGHT, STEP BY STEP", 11.0, p.muted), Space::new().width(Length::Fill), t(format!("{} · ", s(d, "stepsLine")), 12.0, p.muted), click(tb("Full log", 12.0, p.accent), crate::open(&["hearth-settings-app", "--page", "hs-backups"]))].align_y(iced::Center), steps].spacing(10.0))
        .padding(pad(12.0, 16.0, 12.0, 16.0))
        .width(Length::Fill)
        .style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0))));
    // the right column
    let space = column![w::h2(&p, "Space for backups"), draw::bx(&p, column![row![st("ok", 10.0), tb(s(d, "space.name"), 12.0, p.text)].spacing(8.0).align_y(iced::Center), w::meter(&p, f(d, "space.frac") as f32, None, 0.0, 6.0), t(s(d, "space.line"), 12.0, p.muted)].spacing(8.0))].spacing(10.0);
    let mut byd = Column::new().spacing(6.0).push(cap(&p, "By device · all versions"));
    for x in arr(d, "byDevice") {
        byd = byd.push(row![container(t(s(&x, "name"), 12.0, p.text)).width(Length::Fill), draw::mini(&p, f(&x, "frac") as f32), container(t(s(&x, "size"), 12.0, p.muted)).width(Length::Fixed(64.0)).align_x(iced::alignment::Horizontal::Right)].spacing(10.0).align_y(iced::Center));
    }
    let space = column![space, byd].spacing(4.0);
    let r2 = b(d, "r2");
    let second = column![cap(&p, "A second copy away from home"), draw::lr(t("Copy backups to Cloudflare R2", 12.0, p.text), w::sw(&p, r2, Some(act("backups", &["set", "-", "r2", if r2 { "off" } else { "on" }], "")))), t(s(d, "r2Line"), 12.0, p.muted)].spacing(6.0);
    let phone = column![cap(&p, "Phone and Xbox"), t("The phone's photos and files go to Hearth Cloud. The Xbox keeps its saves in its own cloud: nothing to do here.", 12.0, p.muted)].spacing(6.0);
    let test = b(d, "restoreTest");
    let checks = column![
        cap(&p, "Checks"),
        draw::lr(t("Test a restore each month", 12.0, p.text), w::sw(&p, test, Some(act("backups", &["set", "-", "restoreTest", if test { "off" } else { "on" }], "")))),
        t(s(d, "testLine"), 12.0, p.muted),
        container(draw::bx(&p, column![t("The Backups app on each device", 12.0, p.text), t("Updates in the Hearth Store", 12.0, p.muted), container(w::chip(&p, "Install it · asks for your password once", true, Sz::Sm, Some(crate::open(&["hearth-store-app", "--page", "parts"])))).padding(pad(8.0, 0.0, 0.0, 0.0))].spacing(4.0))).padding(pad(8.0, 0.0, 0.0, 0.0)),
    ]
    .spacing(6.0);
    let items: Vec<(&'static str, E<'_>)> = vec![("head", head.into()), ("devices", dv.into()), ("plan", plan_el), ("restore", restore_el), ("steps", steps.into()), ("space", space.into()), ("second", second.into()), ("phone", phone.into()), ("checks", checks.into())];
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "backup",
        items,
        move |ps| {
            let main = column![
                ps.take("head"),
                container(ps.take("devices")).padding(pad(14.0, 0.0, 0.0, 0.0)),
                container(row![container(ps.take("plan")).width(Length::FillPortion(100)), container(ps.take("restore")).width(Length::FillPortion(115))].spacing(16.0)).padding(pad(14.0, 0.0, 0.0, 0.0)),
                container(ps.take("steps")).padding(pad(14.0, 0.0, 0.0, 0.0)),
                crate::said(a),
            ];
            let side = column![ps.take("space"), ps.take("second"), ps.take("phone"), ps.take("checks")].spacing(4.0);
            row![
                container(iced::widget::scrollable(container(main).padding(pad(18.0, 22.0, 18.0, 22.0)))).width(Length::Fill).height(Length::Fill),
                ui::vline(p.line),
                container(iced::widget::scrollable(side)).padding(18.0).width(Length::Fixed(330.0)).height(Length::Fill),
            ]
            .into()
        },
        Msg::Describe,
    )
}

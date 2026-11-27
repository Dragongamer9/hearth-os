//! The Model vault (board 01, docs/mockups/server-expansion/01-model-vault.png; checklist
//! docs/mockups/checklists/server-model-vault.md): what to keep with each set's monthly cost on Cloudflare R2, every
//! model with its version, licence, checksum, where it's kept and whether it's still online, where it's kept, the slow
//! upload, how it works, last week's check. Everything from `vault.py status`; Change what's kept opens the setup
//! (the vault drive, your R2 bucket and keys: the keys go straight into the system's password store).

use super::dash::click;
use super::Page;
use crate::{act, act_ask, act_v, set, App, Msg, E};
use hearth_oss::w::{self, Sz, Tg};
use hearth_server_parts::draw::{self, pad, st};
use hearth_server_parts::{arr, b, f, s};
use hearth_ui::blocks::{bl, PageDef};
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, Column, Row, Space};
use iced::{Color, Font, Length};
use serde_json::{json, Value};

pub const PAGE: Page = Page { id: "vault", title: "Model vault", side: "vault", back: "home", engines: &["vault"], every: 30, bare: false, view, tags, key: super::no_key };

pub const BLOCKS: PageDef = PageDef {
    id: "vault",
    title: "Model vault",
    blocks: &[
        bl("head", "Model vault and its buttons", 0, 0, 9, 1),
        bl("sets", "What to keep and what it costs", 0, 1, 9, 3),
        bl("models", "The models", 0, 4, 9, 8),
        bl("where", "Where it's kept", 9, 0, 3, 3),
        bl("upload", "Uploading slowly in the background", 9, 3, 3, 3),
        bl("how", "How it works", 9, 6, 3, 3),
        bl("last", "Last week's check", 9, 9, 3, 3),
    ],
};

pub fn tags(a: &App) -> crate::Tags {
    let d = a.d("vault");
    if d.is_null() || !b(d, "ok") {
        return Vec::new();
    }
    vec![(s(d, "health"), if s(d, "healthTone") == "ok" { Tg::Ok } else { Tg::Warn })]
}

pub const MONO: Font = Font::MONOSPACE;

fn cap<'a>(p: &Pal, x: impl Into<String>) -> E<'a> {
    tb(x.into().to_uppercase(), 11.0, p.muted).wrapping(Wrapping::None).into()
}

/// One set's card (.set): its name and tag, its size, what's in it, then its four price rows.
fn set_card<'a>(p: &Pal, x: &Value) -> E<'a> {
    let kept = b(x, "kept");
    let tag = if kept { w::tag(p, s(x, "tag"), Tg::Acc) } else { w::tag(p, s(x, "tag"), Tg::Dk) };
    let mut c = Column::new().spacing(0.0).push(row![tb(s(x, "title"), 16.0, p.text), Space::new().width(Length::Fill), tag].align_y(iced::Center));
    c = c.push(container(tb(s(x, "size"), 28.0, p.text)).padding(pad(6.0, 0.0, 2.0, 0.0)));
    c = c.push(container(t(s(x, "about"), 12.0, p.muted)).padding(pad(0.0, 0.0, 8.0, 0.0)));
    for r in arr(x, "rows") {
        let label = r.get(0).and_then(Value::as_str).unwrap_or("").to_string();
        let val = r.get(1).and_then(Value::as_str).unwrap_or("").to_string();
        let tone = r.get(2).and_then(Value::as_str).unwrap_or("");
        let (lc, vc, strong) = match tone {
            "good" => (p.text, w::green(), true),
            "acc" => (p.text, p.accent, true),
            "mu" => (p.muted, p.muted, false),
            _ => (p.text, p.text, true),
        };
        let v: E<'a> = if strong { tb(val, 12.0, vc).wrapping(Wrapping::None).into() } else { t(val, 12.0, vc).wrapping(Wrapping::None).into() };
        c = c.push(column![container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(ui::boxed(Pal::mix(p.panel, p.line, 0.6), 0.0, None)), container(row![t(label, 12.0, lc), Space::new().width(Length::Fill), v].align_y(iced::Center)).padding(pad(6.0, 0.0, 6.0, 0.0))]);
    }
    let (bg, line) = if kept { (Color::from_rgb8(0x2e, 0x22, 0x18), (p.accent, 2.0)) } else { (p.panel, (p.line, 1.0)) };
    let bg = if p.dark { bg } else { Pal::mix(p.panel, p.accent, 0.08) };
    let card = container(c).padding(pad(14.0, 16.0, 14.0, 16.0)).width(Length::Fill).style(ui::boxed(bg, p.r(12.0), Some(line)));
    click(card, act_v("vault", vec!["set".into(), "keep".into(), s(x, "id")], "Kept from now on"))
}

const COLS: [(&str, u16); 8] = [("Model", 34), ("App", 11), ("Version", 17), ("Licence", 11), ("Size", 8), ("Checksum", 16), ("Kept", 10), ("Online", 14)];

fn cell<'a>(el: impl Into<E<'a>>, w: u16) -> E<'a> {
    container(el.into()).width(Length::FillPortion(w)).clip(true).into()
}

fn table<'a>(a: &'a App, d: &'a Value) -> E<'a> {
    let p = a.p;
    let tab = a.ui_s("vaultTab", "All");
    let all = a.ui_b("vaultAll");
    let rows: Vec<Value> = arr(d, "models").into_iter().filter(|m| tab == "All" || s(m, "app") == tab || (tab == "Kokoro" && s(m, "app") == "Kokoro voices")).collect();
    let mut head = Row::new().spacing(0.0);
    for (h, w_) in COLS {
        head = head.push(cell(tb(h.to_uppercase(), 11.0, p.muted), w_));
    }
    let mut c = Column::new().push(container(head).padding(pad(8.0, 10.0, 8.0, 10.0)));
    c = c.push(container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(ui::boxed(p.line, 0.0, None)));
    let pick = a.ui_s("vaultRow", "");
    let n = if all { rows.len() } else { rows.len().min(12) };
    for m in rows.iter().take(n) {
        let tone = s(m, "tone");
        let tg = match tone.as_str() {
            "ok" => Tg::Ok,
            "bad" => Tg::Bad,
            "warn" => Tg::Warn,
            _ => Tg::Dk,
        };
        let mut name = Row::new().spacing(6.0).align_y(iced::Center).push(t(s(m, "name"), 12.5, p.text).wrapping(Wrapping::None));
        if b(m, "giant") {
            name = name.push(w::tag(&p, "Giant", Tg::Dk));
        }
        let kept_ok = b(m, "keptOk");
        let kept = s(m, "kept");
        let kept_el: E<'_> = if !kept_ok {
            t(kept, 12.5, p.muted).into()
        } else if let Some((here, part)) = kept.split_once(" · ").filter(|(_, r)| r.contains('%')) {
            row![t(format!("{here} · "), 12.5, p.text), t(part.to_string(), 12.5, p.muted)].into()
        } else {
            t(kept, 12.5, p.text).into()
        };
        let r = row![
            cell(name, COLS[0].1),
            cell(t(s(m, "app"), 12.5, p.muted), COLS[1].1),
            cell(t(s(m, "version"), 11.0, p.muted).font(MONO).wrapping(Wrapping::None), COLS[2].1),
            cell(t(s(m, "licence"), 12.5, p.muted).wrapping(Wrapping::None), COLS[3].1),
            cell(t(s(m, "size"), 12.5, p.text), COLS[4].1),
            cell(t(s(m, "check"), 11.0, p.muted).font(MONO).wrapping(Wrapping::None), COLS[5].1),
            cell(kept_el, COLS[6].1),
            cell(w::tag(&p, s(m, "online"), tg), COLS[7].1),
        ]
        .align_y(iced::Center);
        let id = s(m, "id");
        let lit = b(m, "flag") && kept_ok || pick == id;
        let bg = if lit { Pal::mix(p.panel, p.accent, 0.10) } else { Color::TRANSPARENT };
        let row_el = container(r).padding(pad(7.0, 7.0, 7.0, 7.0)).width(Length::Fill).style(ui::boxed(bg, p.r(6.0), None));
        c = c.push(click(row_el, set("vaultRow", json!(if pick == id { String::new() } else { id.clone() }))));
        c = c.push(container(Space::new()).width(Length::Fill).height(Length::Fixed(1.0)).style(ui::boxed(Pal::mix(p.panel, p.line, 0.6), 0.0, None)));
        if pick == id && kept_ok {
            let name = s(m, "name");
            c = c.push(container(row![
                t(format!("{name}: kept as its exact files"), 12.0, p.muted),
                Space::new().width(Length::Fill),
                w::chip(&p, "Get it back from R2", false, Sz::Sm, Some(act_v("vault", vec!["getback".into(), id.clone()], "Back in the vault"))),
                w::chip(&p, "Remove it from the vault", false, Sz::Sm, Some(act_ask("vault", vec!["remove".into(), id.clone()], &format!("Remove {name} from the vault? Its files on the vault drive are deleted (R2's copy stays). Asked twice: press Yes again on the next question."), ""))),
            ].spacing(8.0).align_y(iced::Center)).padding(pad(8.0, 10.0, 8.0, 10.0)));
        }
    }
    container(c).padding(pad(4.0, 6.0, 4.0, 6.0)).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0)))).into()
}

/// Change what's kept: the vault drive, R2 (account, bucket, class, the keys into the password store), the
/// background work on or off.
fn setup<'a>(a: &'a App, d: &'a Value) -> E<'a> {
    let p = a.p;
    let field = |k: &str, ph: &str, cur: String| -> E<'a> {
        let key = k.to_string();
        let v = a.ui.get(k).and_then(Value::as_str).map(str::to_string).unwrap_or(cur);
        w::input(&p, ph, &v, 13.0, move |x| Msg::Text(key.clone(), x)).width(Length::Fixed(360.0)).into()
    };
    let save = |k: &'static str, what: &'static str| -> Msg {
        let v = a.ui.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        act_v("vault", vec!["set".into(), what.into(), v], "Saved")
    };
    let keys = b(d, "r2.keys");
    let class = s(d, "r2.class");
    let on = b(d, "on");
    let c = column![
        w::h2(&p, "Change what's kept"),
        t("Pick a set above (it's kept from now on), where the vault lives, and your own Cloudflare R2 bucket. Nothing is ever deleted from the vault by itself.", 12.0, p.muted),
        w::kv(&p, "The vault drive (a folder on it)", row![field("vaultDrive", "/var/mnt/vault", s(d, "drive")), w::chip(&p, "Use it", false, Sz::Sm, Some(save("vaultDrive", "drive")))].spacing(8.0).align_y(iced::Center).into()),
        w::kv(&p, "R2 account ID", row![field("vaultAccount", "32 letters and digits (R2 › Account details)", s(d, "r2.account")), w::chip(&p, "Save", false, Sz::Sm, Some(save("vaultAccount", "r2.account")))].spacing(8.0).align_y(iced::Center).into()),
        w::kv(&p, "R2 bucket", row![field("vaultBucket", "hearth-models", s(d, "r2.bucket")), w::chip(&p, "Save", false, Sz::Sm, Some(save("vaultBucket", "r2.bucket")))].spacing(8.0).align_y(iced::Center).into()),
        w::kv(&p, "R2 class", w::seg(&p, vec![("Infrequent Access".into(), class != "standard", Some(act("vault", &["set", "r2.class", "infrequent"], ""))), ("Standard".into(), class == "standard", Some(act("vault", &["set", "r2.class", "standard"], "")))])),
        w::kv(&p, if keys { "R2 keys · in the system's password store ✓" } else { "R2 keys (an API token's access key and secret)" }, row![
            w::secret_input(&p, "Access key ID", &a.ui_s("r2access", ""), |x| Msg::Text("r2access".into(), x), Msg::SaveKey("access")).width(Length::Fixed(200.0)),
            w::chip(&p, "Keep", false, Sz::Sm, Some(Msg::SaveKey("access"))),
            w::secret_input(&p, "Secret access key", &a.ui_s("r2secret", ""), |x| Msg::Text("r2secret".into(), x), Msg::SaveKey("secret")).width(Length::Fixed(220.0)),
            w::chip(&p, "Keep", false, Sz::Sm, Some(Msg::SaveKey("secret"))),
        ].spacing(8.0).align_y(iced::Center).into()),
        w::kv(&p, "Work in the background (copy, then upload slowly; Sundays' check)", w::sw(&p, on, Some(act("vault", &[if on { "off" } else { "on" }], "")))),
        t("Your keys never go in a file, a log or a command line: the vault reads them from the password store only while it uploads.", 12.0, p.muted),
    ]
    .spacing(10.0);
    container(c).padding(16.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(12.0), Some((p.accent, 1.0)))).into()
}

fn view<'a>(a: &'a App) -> E<'a> {
    let p = a.p;
    let d = a.d("vault");
    if d.is_null() {
        return container(w::mu(&p, "Looking at the vault…", 14.0)).padding(30.0).into();
    }
    if !b(d, "ok") {
        return container(column![w::h1(&p, "Model vault"), t(s(d, "error"), 13.0, w::red())].spacing(10.0)).padding(30.0).into();
    }
    let setup_open = a.ui_b("vaultSetup");
    let head = row![
        column![w::h1(&p, "Model vault"), w::mu(&p, "An exact copy of every AI model Hearth offers, so it's still yours if it's ever taken down or changed online", 12.0)].spacing(4.0),
        Space::new().width(Length::Fill),
        w::chip(&p, "Check now", false, Sz::Md, Some(act("vault", &["check"], "Checked every model"))),
        w::chip(&p, "Get a model back", false, Sz::Md, Some(Msg::Go("checks"))),
        w::chip(&p, "Change what's kept", true, Sz::Md, Some(set("vaultSetup", json!(!setup_open)))),
    ]
    .spacing(10.0)
    .align_y(iced::Center);
    let mut sets = Row::new().spacing(12.0);
    for x in arr(d, "sets") {
        sets = sets.push(container(set_card(&p, &x)).width(Length::FillPortion(1)));
    }
    let sets = column![cap(&p, "What to keep · monthly cost on Cloudflare R2"), sets, t(s(d, "setsNote"), 12.0, p.muted)].spacing(8.0);
    let tab = a.ui_s("vaultTab", "All");
    let mut tabs_l: Vec<String> = vec!["All".into()];
    for ap in hearth_server_parts::strs(d, "apps") {
        tabs_l.push(if ap == "Kokoro voices" { "Kokoro".into() } else { ap });
    }
    let tabs = w::tabs(&p, tabs_l.iter().map(|x| (x.clone(), tab == *x, Some(set("vaultTab", json!(x))))).collect());
    let total = f(d, "total");
    let models = column![
        row![cap(&p, format!("The models · {} kept of {}", f(d, "keptCount"), total)), Space::new().width(Length::Fill), tabs].align_y(iced::Center),
        if setup_open { setup(a, d) } else { table(a, d) },
        row![t("Each model is kept as its exact files, with its version, licence and checksum written beside it", 12.0, p.muted), Space::new().width(Length::Fill), click(tb(if a.ui_b("vaultAll") { "Show fewer ›".to_string() } else { format!("Show all {total} ›") }, 12.0, p.accent), set("vaultAll", json!(!a.ui_b("vaultAll"))))].align_y(iced::Center),
    ]
    .spacing(8.0);
    // the right column
    let mut wh = Column::new().spacing(8.0).push(w::h2(&p, "Where it's kept"));
    for x in arr(d, "where") {
        wh = wh.push(draw::bx(&p, column![row![st(&s(&x, "state"), 10.0), tb(s(&x, "name"), 12.0, p.text), Space::new().width(Length::Fill), t(s(&x, "right"), 12.0, p.muted)].spacing(8.0).align_y(iced::Center), w::meter(&p, f(&x, "frac") as f32, None, 0.0, 6.0), t(s(&x, "line"), 12.0, p.muted)].spacing(8.0)));
    }
    let up = hearth_server_parts::at(d, "upload");
    let paused = b(d, "paused");
    let night = b(d, "night");
    let speed = s(d, "speed");
    let nxt = match speed.as_str() {
        "slow" => "medium",
        "medium" => "fast",
        _ => "slow",
    };
    let up_el: E<'_> = if up.is_null() {
        draw::bx(&p, t(if b(d, "r2.bucket") || !s(d, "r2.bucket").is_empty() { "Everything kept is on R2" } else { "Nothing to upload: set up R2 under Change what's kept" }, 12.0, p.muted)).into()
    } else {
        draw::bx(&p, column![
            draw::lr(t(s(up, "name"), 12.0, p.text), t(format!("{}%", f(up, "pct")), 12.0, p.muted)),
            w::meter(&p, (f(up, "pct") / 100.0) as f32, None, 0.0, 6.0),
            row![t(format!("{} · ", s(up, "togo")), 12.0, p.muted), tb(s(up, "left"), 12.0, p.text)],
            t(format!("{}, a third of your upload · pauses while you play or stream", s(up, "speed")), 12.0, p.muted),
            row![
                w::chip(&p, if paused { "Carry on" } else { "Pause" }, false, Sz::Sm, Some(act("vault", &["set", "paused", if paused { "off" } else { "on" }], ""))),
                w::chip(&p, format!("Speed: {speed} ▾"), false, Sz::Sm, Some(act("vault", &["set", "speed", nxt], ""))),
                w::chip(&p, "Only at night", night, Sz::Sm, Some(act("vault", &["set", "night", if night { "off" } else { "on" }], ""))),
            ]
            .spacing(6.0),
        ]
        .spacing(8.0))
        .into()
    };
    let upload = column![container(cap(&p, "Uploading slowly in the background")).padding(pad(12.0, 0.0, 0.0, 0.0)), up_el].spacing(8.0);
    let how_row = |label: &str, sub: &str, key: &str| -> E<'a> {
        let on = b(d, &format!("how.{key}"));
        let mut l = Column::new().push(t(label.to_string(), 12.0, p.text));
        if !sub.is_empty() {
            l = l.push(t(sub.to_string(), 11.0, p.muted));
        }
        draw::lr(l, w::sw(&p, on, Some(act_v("vault", vec!["set".into(), key.into(), if on { "off".into() } else { "on".into() }], ""))))
    };
    let how = column![
        container(cap(&p, "How it works")).padding(pad(12.0, 0.0, 0.0, 0.0)),
        how_row("Apps load models from the vault first", "so they work with no internet", "loadFirst"),
        how_row("Ask before copying a new model", "", "askFirst"),
        how_row("Check Hugging Face each week", "Sundays 3:00 AM · alert on your phone", "weekly"),
        how_row("Check every file's checksum monthly", "", "monthly"),
    ]
    .spacing(10.0);
    let last = hearth_server_parts::at(d, "last");
    let mut lb = Column::new().spacing(6.0);
    for r in arr(last, "rows") {
        let tone = r.get(0).and_then(Value::as_str).unwrap_or("ok").to_string();
        let text = r.get(1).and_then(Value::as_str).unwrap_or("").to_string();
        let see = r.get(2).and_then(Value::as_str).unwrap_or("");
        let right: E<'_> = if see.is_empty() { Space::new().into() } else { click(tb("See ›", 12.0, p.accent), Msg::Go("checks")) };
        lb = lb.push(draw::lr(row![st(&tone, 10.0), t(text, 12.0, p.text)].spacing(8.0).align_y(iced::Center), right));
    }
    if arr(last, "rows").is_empty() {
        lb = lb.push(t("No check yet: Check now asks Hugging Face about every model", 12.0, p.muted));
    }
    let lastc = column![
        container(cap(&p, if s(last, "date").is_empty() { "Last week's check".to_string() } else { format!("Last week's check · {}", s(last, "date")) })).padding(pad(12.0, 0.0, 0.0, 0.0)),
        draw::bx(&p, lb),
        container(t("Nothing is deleted from the vault by itself. Removing a model asks twice.", 12.0, p.muted)).padding(pad(6.0, 0.0, 0.0, 0.0)),
    ]
    .spacing(8.0);
    let items: Vec<(&'static str, E<'_>)> = vec![("head", head.into()), ("sets", sets.into()), ("models", models.into()), ("where", wh.into()), ("upload", upload.into()), ("how", how.into()), ("last", lastc.into())];
    hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "vault",
        items,
        move |ps| {
            let main = column![ps.take("head"), container(ps.take("sets")).padding(pad(14.0, 0.0, 0.0, 0.0)), container(ps.take("models")).padding(pad(14.0, 0.0, 0.0, 0.0)), crate::said(a)].spacing(0.0);
            let side = column![ps.take("where"), ps.take("upload"), ps.take("how"), ps.take("last")].spacing(4.0);
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

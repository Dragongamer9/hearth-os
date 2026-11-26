//! Board 2: every model Hearth Forge can use (what each makes, the memory it needs, whether it fits your graphics
//! card, its licence, installed or Get), the side with its pages and what to show, Tripo3D online with your key;
//! and the side's other pages: Made, Waiting and Settings.

use crate::forge::{self, Model};
use crate::{cap, lately_tile, ordinal, waiting, App, Msg};
use hearth_oss::w::{self, Sz, Tg};
use hearth_ui::ui::{self, t, tb};
use hearth_ui::Pal;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, mouse_area, row, scrollable, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding};
use serde_json::Value;

/// A tag as the table writes it (its own words, not in capitals).
fn ftag<'a>(p: &Pal, label: &str, k: Tg) -> Element<'a, Msg> {
    let (bg, fg, line) = match k {
        Tg::Ok => (Pal::mix(p.panel, w::green(), 0.16), w::green(), Pal::mix(p.panel, w::green(), 0.35)),
        Tg::Warn => (Pal::mix(p.panel, w::yellow(), 0.14), w::yellow(), Pal::mix(p.panel, w::yellow(), 0.32)),
        Tg::Bad => (Pal::mix(p.panel, w::red(), 0.16), w::red(), Pal::mix(p.panel, w::red(), 0.35)),
        _ => (p.panel2, p.muted, p.line),
    };
    container(tb(label.to_string(), 10.5, fg).wrapping(Wrapping::None)).padding(Padding { top: 3.0, right: 8.0, bottom: 3.0, left: 8.0 }).style(ui::boxed(bg, 6.0, Some((line, 1.0)))).into()
}

fn fit_kind(m: &Model) -> Tg {
    match m.fit.as_str() {
        "ok" => Tg::Ok,
        "warn" => Tg::Warn,
        "bad" => Tg::Bad,
        _ => Tg::Dk,
    }
}
fn fits(m: &Model) -> bool {
    matches!(m.fit.as_str(), "ok" | "warn")
}

/// The side's rows (the board's .nav: its name, its count, lit when it's the one open).
fn nav<'a>(p: &Pal, label: &str, count: String, on: bool, m: Msg) -> Element<'a, Msg> {
    let r = row![t(label.to_string(), 13.0, if on { p.text } else { p.muted }).wrapping(Wrapping::None), Space::new().width(Length::Fill), t(count, 11.0, p.muted)].align_y(iced::Center);
    let inner = container(r).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 }).center_y(Length::Fixed(36.0)).width(Length::Fill);
    mouse_area(w::lit_row(p, inner.into(), on, 9.0, 36.0)).on_press(m).interaction(mouse::Interaction::Pointer).into()
}

pub fn shown_models(a: &App) -> Vec<&Model> {
    let show = a.pref("show", "every");
    let q = a.search.trim().to_lowercase();
    let mut v: Vec<&Model> = a
        .cat
        .models
        .iter()
        .filter(|m| match show.as_str() {
            "fit" => fits(m),
            "installed" => a.installed(&m.id),
            "pbr" => m.pbr,
            "free" => m.free_for_paid,
            _ => true,
        })
        .filter(|m| q.is_empty() || format!("{} {} {} {}", m.name, m.maker, m.makes, m.licence).to_lowercase().contains(&q))
        .collect();
    match a.pref("sort", "fits").as_str() {
        "name" => v.sort_by_key(|m| m.name.to_lowercase()),
        "maker" => v.sort_by_key(|m| m.maker.to_lowercase()),
        "installed" => v.sort_by_key(|m| !a.installed(&m.id)),
        _ => {}
    }
    v
}

pub fn side(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let n = a.cat.models.len();
    let pg = a.page;
    let nav_c = column![
        nav(&p, "Make a model", String::new(), pg == "make", Msg::Page("make")),
        nav(&p, "Models", n.to_string(), pg == "models", Msg::Page("models")),
        nav(&p, "Made", a.made.iter().filter(|m| m.pointer("/done/ok").and_then(Value::as_bool) == Some(true)).count().to_string(), pg == "made", Msg::Page("made")),
        nav(&p, "Waiting", waiting(a).len().to_string(), pg == "waiting", Msg::Page("waiting")),
        nav(&p, "Settings", String::new(), pg == "settings", Msg::Page("settings")),
    ]
    .spacing(2.0);
    let show = a.pref("show", "every");
    let count = |f: &dyn Fn(&Model) -> bool| a.cat.models.iter().filter(|m| f(m)).count().to_string();
    let show_c = column![
        container(cap(&p, "Show")).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 }),
        nav(&p, "Every model", n.to_string(), pg == "models" && show == "every", Msg::Show("every")),
        nav(&p, &format!("Fit my {} GB", a.vram_gb()), count(&|m| fits(m)), pg == "models" && show == "fit", Msg::Show("fit")),
        nav(&p, "Installed", count(&|m| a.installed(&m.id)), pg == "models" && show == "installed", Msg::Show("installed")),
        nav(&p, "Real materials (PBR)", count(&|m| m.pbr), pg == "models" && show == "pbr", Msg::Show("pbr")),
        nav(&p, "Free for paid work", count(&|m| m.free_for_paid), pg == "models" && show == "free", Msg::Show("free")),
    ]
    .spacing(2.0);
    let credits = match a.tripo {
        Some(b) => format!("{} credits left · each model shows its cost before it's made", forge::commas(b)),
        None => "No key yet (Settings) · each model shows its cost before it's made".into(),
    };
    let online = column![
        container(cap(&p, "Online (your key)")).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 }),
        container(column![
            tb("Tripo3D", 12.0, p.text),
            ui::gap(1.0, 4.0),
            t(format!("tripo3d.ai with your own key, kept in the system's password store\n{credits}"), 12.0, p.muted).line_height(1.6),
            ui::gap(1.0, 8.0),
            row![t("Use it", 12.0, p.text), Space::new().width(Length::Fill), w::sw(&p, a.prefs.b("tripoOn", false), Some(Msg::TripoUse))].align_y(iced::Center),
        ])
        .padding(12.0)
        .style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0)))),
    ];
    let k = a.status.get("kept").cloned().unwrap_or(Value::Null);
    let kept_words = match (k.get("used").and_then(Value::as_u64), k.get("free").and_then(Value::as_f64)) {
        (Some(u), Some(f)) => format!("Models are kept in ~/Hearth/Forge/models ({} used · {} free)", gb(u as f64), gb(f)),
        _ => "Models are kept in ~/Hearth/Forge/models on the Nvidia PC".into(),
    };
    let kept = container(t(kept_words, 12.0, p.muted).line_height(1.6)).padding(Padding { top: 16.0, right: 0.0, bottom: 0.0, left: 0.0 });
    let body = hearth_ui::arrange::parts(
        &a.describe,
        &p,
        "mside",
        vec![("nav", nav_c.into()), ("show", show_c.into()), ("online", online.into()), ("kept", kept.into())],
        |ps| column![ps.take("nav"), ps.take("show"), ps.take("online"), ps.take("kept")].into(),
        Msg::Describe,
    );
    container(scrollable(body).height(Length::Fill)).width(Length::Fixed(250.0)).height(Length::Fill).padding(Padding { top: 18.0, right: 12.0, bottom: 18.0, left: 12.0 }).style(w::side_bg(&p)).into()
}

fn gb(b: f64) -> String {
    format!("{:.0} GB", b / 1e9)
}

/// The right-hand column for a model: INSTALLED · DEFAULT, INSTALLED, how far its Get is, or Get.
fn state_cell<'a>(a: &'a App, m: &Model) -> Element<'a, Msg> {
    let p = a.p;
    let st = a.mstate(&m.id);
    let getting = st.get("getting").cloned().unwrap_or(Value::Null);
    if getting.get("busy").and_then(Value::as_bool).unwrap_or(false) {
        return ftag(&p, &format!("{} %", getting.get("progress").and_then(Value::as_f64).unwrap_or(0.0).round()), Tg::Warn);
    }
    if a.installed(&m.id) {
        let is_default = a.model().id == m.id;
        let tag = w::tag(&p, if is_default { "Installed · default" } else { "Installed" }, Tg::Ok);
        return if is_default { tag } else { mouse_area(tag).on_press(Msg::PickModel(m.id.clone())).interaction(mouse::Interaction::Pointer).into() };
    }
    let err = getting.get("error").and_then(Value::as_str).unwrap_or("");
    let chip = container(t("Get", 11.5, if a.up == Some(true) && m.no_source.is_empty() { p.text } else { p.muted })).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 }).center_y(Length::Fixed(22.0)).style(ui::boxed(p.panel2, p.r(8.0), Some((p.line, 1.0))));
    let chip: Element<'_, Msg> = if a.up == Some(true) && m.no_source.is_empty() { mouse_area(chip).on_press(Msg::Get(m.id.clone())).interaction(mouse::Interaction::Pointer).into() } else { chip.into() };
    if err.is_empty() {
        chip
    } else {
        row![t(ui::fit(err, 40), 10.5, w::red()), chip].spacing(6.0).align_y(iced::Center).into()
    }
}

const COLS: [f32; 5] = [225.0, 300.0, 226.0, 330.0, 0.0];

pub fn table(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let cell = |e: Element<'static, Msg>, i: usize| -> Element<'static, Msg> {
        let c = container(e).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 });
        if COLS[i] > 0.0 { c.width(Length::Fixed(COLS[i])).into() } else { c.width(Length::Fill).into() }
    };
    let vram = a.vram_gb();
    let mut head = Row::new().align_y(iced::Center);
    for (i, h) in ["Model", "Makes", "Memory it needs", &format!("On your {vram} GB"), "Licence"].iter().enumerate() {
        head = head.push(cell(tb(h.to_uppercase(), 11.0, p.muted).wrapping(Wrapping::None).into(), i));
    }
    head = head.push(Space::new().width(Length::Fixed(150.0)));
    let mut c = Column::new().push(container(head).padding(Padding { top: 6.0, right: 0.0, bottom: 6.0, left: 0.0 })).push(ui::hline(p.line));
    let list = shown_models(a);
    for (gid, label) in &a.cat.groups {
        let rows: Vec<&&Model> = list.iter().filter(|m| &m.group == gid).collect();
        if rows.is_empty() {
            continue;
        }
        c = c.push(container(tb(label.to_uppercase(), 11.0, p.muted)).padding(Padding { top: 10.0, right: 10.0, bottom: 4.0, left: 10.0 })).push(ui::hline(p.line));
        for m in rows {
            let on = a.model().id == m.id;
            let name = row![tb(m.name.clone(), 12.5, p.text).wrapping(Wrapping::None), t(format!(" · {}", m.maker), 12.0, p.muted).wrapping(Wrapping::None)].align_y(iced::Center);
            let mut r = Row::new().align_y(iced::Center).height(Length::Fixed(29.0));
            r = r.push(cell(name.into(), 0));
            r = r.push(cell(t(m.makes.clone(), 12.5, p.text).into(), 1));
            r = r.push(cell(t(m.memory.clone(), 12.5, p.text).wrapping(Wrapping::None).into(), 2));
            r = r.push(cell(ftag(&p, &m.fit_words, fit_kind(m)), 3));
            r = r.push(cell(t(m.licence.clone(), 12.5, p.muted).into(), 4));
            r = r.push(container(state_cell(a, m)).width(Length::Fixed(150.0)).align_x(iced::alignment::Horizontal::Right).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 0.0 }));
            let bg = if on { Pal::mix(p.panel, p.accent, 0.10) } else { Color::TRANSPARENT };
            c = c.push(container(r).style(ui::boxed(bg, 0.0, None)).width(Length::Fill));
        }
    }
    if list.is_empty() {
        c = c.push(container(t("No model is like that.", 12.0, p.muted)).padding(12.0));
    }
    scrollable(c).height(Length::Fill).into()
}

pub fn main(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let body: Element<'_, Msg> = match a.page {
        "made" => made(a),
        "waiting" => waiting_page(a),
        "settings" => settings(a),
        _ => {
            let sort = a.pref("sort", "fits");
            let sort_words = match sort.as_str() {
                "name" => "By name",
                "maker" => "By maker",
                "installed" => "Installed first",
                _ => "Fits first",
            };
            let mut sort_c = Column::new().push(w::chip(&p, format!("Sort: {sort_words} ▾"), false, Sz::Sm, Some(Msg::Menu("sort"))));
            if a.menu == Some("sort") {
                sort_c = sort_c.push(w::menu(&p, [("fits", "Fits first"), ("name", "By name"), ("maker", "By maker"), ("installed", "Installed first")].iter().map(|(k, l)| (l.to_string(), sort == *k, Msg::Sort(k))).collect(), 160.0));
            }
            let head = row![
                w::h2(&p, "Every model Hearth Forge can use"),
                Space::new().width(Length::Fill),
                container(w::search(&p, "Find a model", &a.search, Msg::Search)).width(Length::Fixed(280.0)),
                sort_c,
                w::chip(&p, "Get the best 4 for my PC", true, Sz::Sm, (a.up == Some(true)).then_some(Msg::GetBest)),
            ]
            .spacing(10.0)
            .align_y(iced::Center);
            let mut about = column![t(
                format!("Each runs on your Nvidia PC. \"Fits\" is for your {}'s {} GB; a model too big here can run on a bigger PC of yours over Hearth Link. Each has its own licence, shown here.", if a.card().is_empty() { "RTX 5070".to_string() } else { a.card() }, a.vram_gb()),
                12.0,
                p.muted
            )];
            if !a.said.is_empty() {
                about = about.push(t(a.said.split('\u{1}').next().unwrap_or("").to_string(), 12.0, p.text));
            }
            hearth_ui::arrange::parts(
                &a.describe,
                &p,
                "mlist",
                vec![("head", head.into()), ("about", container(about.spacing(4.0)).padding(Padding { top: 6.0, right: 0.0, bottom: 4.0, left: 0.0 }).into()), ("table", table(a))],
                |ps| column![ps.take("head"), ps.take("about"), ps.take("table")].into(),
                Msg::Describe,
            )
        }
    };
    container(body).padding(Padding { top: 14.0, right: 20.0, bottom: 14.0, left: 20.0 }).width(Length::Fill).height(Length::Fill).into()
}

fn made(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(10.0).push(w::h2(&p, "Every model made")).push(t("Kept in ~/Hearth/Forge, each with its photo, words and settings. Open one to see it turn and send it on.", 12.0, p.muted));
    let all: Vec<&Value> = a.made.iter().collect();
    for chunk in all.chunks(6) {
        let mut r = Row::new().spacing(10.0);
        for it in chunk {
            let model = it.pointer("/done/model").or_else(|| it.pointer("/ask/model")).and_then(Value::as_str).unwrap_or("").to_string();
            let state = match it.pointer("/done/ok").and_then(Value::as_bool) {
                Some(true) => model,
                Some(false) => "stopped".into(),
                None => "waiting".into(),
            };
            r = r.push(container(column![lately_tile(a, it), t(state, 11.0, p.muted)].spacing(4.0)).width(Length::FillPortion(1)));
        }
        for _ in chunk.len()..6 {
            r = r.push(Space::new().width(Length::FillPortion(1)));
        }
        c = c.push(r);
    }
    if a.made.is_empty() {
        c = c.push(t("Nothing made yet.", 12.0, p.muted));
    }
    scrollable(c).height(Length::Fill).into()
}

fn waiting_page(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let mut c = Column::new().spacing(8.0).push(w::h2(&p, "Waiting"));
    if let Some(r) = a.status.get("running").filter(|r| !r.is_null()) {
        let id = r.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        let title = a.item(&id).and_then(|i| i.get("title")).and_then(Value::as_str).unwrap_or(&id).to_string();
        c = c.push(w::card2(&p, row![column![tb(title, 13.0, p.text), t(format!("{} · {} %", r.get("stage").and_then(Value::as_str).unwrap_or(""), r.get("progress").and_then(Value::as_f64).unwrap_or(0.0).round()), 12.0, p.muted)], Space::new().width(Length::Fill), w::chip(&p, "Stop", false, Sz::Sm, Some(Msg::Stop(id)))].align_y(iced::Center)));
    }
    let ws = waiting(a);
    for (i, (title, id)) in ws.iter().enumerate() {
        c = c.push(w::card2(&p, row![t(title.clone(), 13.0, p.text), Space::new().width(Length::Fill), t(ordinal(i), 12.0, p.muted), w::chip(&p, "Take it out", false, Sz::Sm, Some(Msg::Stop(id.clone())))].spacing(10.0).align_y(iced::Center)));
    }
    if ws.is_empty() && a.status.get("running").map(|r| r.is_null()).unwrap_or(true) {
        c = c.push(t("Nothing is being made.", 12.0, p.muted));
    }
    scrollable(c).height(Length::Fill).into()
}

fn key_row<'a>(a: &'a App, k: &'static str, label: &str, hint: &str) -> Element<'a, Msg> {
    let p = a.p;
    let v = a.keys.get(k).cloned().unwrap_or_default();
    column![
        t(label.to_string(), 13.0, p.text),
        t(hint.to_string(), 12.0, p.muted),
        row![w::secret_input(&p, "Paste it here", &v, move |s| Msg::KeyEdit(k, s), Msg::KeySave(k)).width(Length::Fixed(360.0)), w::chip(&p, "Keep it", true, Sz::Sm, (!v.trim().is_empty()).then_some(Msg::KeySave(k)))].spacing(8.0).align_y(iced::Center),
    ]
    .spacing(4.0)
    .into()
}

fn settings(a: &App) -> Element<'_, Msg> {
    let p = a.p;
    let l = &a.local;
    let on = l.get("on").and_then(Value::as_bool).unwrap_or(false);
    let here = l.get("nvidia").and_then(Value::as_bool).unwrap_or(false);
    let srv = a.server_edit.clone().unwrap_or_else(|| a.base.clone());
    let mut c = Column::new().spacing(14.0).push(w::h2(&p, "Settings"));
    let mut svc = column![
        tb("Where models are made", 13.0, p.text),
        t("Hearth Forge's own service on your Nvidia PC (port 8191, this PC and your Tailscale devices only). From another device, type that PC's name; a model too big for one PC is made on a bigger PC of yours the same way.", 12.0, p.muted),
        row![w::input(&p, "http://nvidia-pc:8191", &srv, 12.0, Msg::Server).on_submit(Msg::ServerDone).width(Length::Fixed(360.0)), w::chip(&p, "Use it", false, Sz::Sm, Some(Msg::ServerDone))].spacing(8.0).align_y(iced::Center),
    ]
    .spacing(6.0);
    if here {
        svc = svc.push(if on {
            row![t("On this PC: on (it starts a minute after you sign in, and waits while you play).", 12.0, p.text), w::chip(&p, "Turn it off", false, Sz::Sm, Some(Msg::TurnOff))].spacing(10.0).align_y(iced::Center)
        } else {
            row![t("On this PC: not set up.", 12.0, p.text), w::chip(&p, "Set up Hearth Forge on this PC", true, Sz::Sm, Some(Msg::SetUp))].spacing(10.0).align_y(iced::Center)
        });
    }
    c = c.push(w::card(&p, svc));
    c = c.push(w::card(&p, column![
        key_row(a, "tripo3d", "Your Tripo3D key", "From tripo3d.ai. Kept in the system's password store; Hearth Forge reads it only to ask Tripo3D, and never writes it anywhere."),
        ui::gap(1.0, 10.0),
        key_row(a, "huggingface", "Your Hugging Face key (only for models that ask for it)", "Stable Fast 3D, SPAR3D and Meta's models ask you to accept their licence on huggingface.co first; then Get uses this key. Kept in the password store."),
    ]));
    c = c.push(w::card(&p, column![
        folder(a, "godotProject", "Gravesleep's Godot project", "Its folder (with project.godot). Send to Godot puts the .glb in its assets/forge."),
        ui::gap(1.0, 10.0),
        folder(a, "o3deProject", "Noclip Protocol's O3DE project", "Its folder (with project.json). Send to O3DE puts the .fbx in its Assets/Forge."),
        ui::gap(1.0, 10.0),
        t(if l.get("blender").and_then(Value::as_bool).unwrap_or(false) { "Blender is here: saving .obj .fbx .stl .usdz, fewer faces and printing work." } else { "Saving .obj .fbx .stl .usdz, fewer faces and printing use Blender: Hearth 3D gets it for you." }, 12.0, p.muted),
    ]));
    if !a.said.is_empty() {
        c = c.push(t(a.said.split('\u{1}').next().unwrap_or("").to_string(), 12.0, p.text));
    }
    scrollable(c).height(Length::Fill).into()
}

fn folder<'a>(a: &'a App, k: &'static str, label: &str, hint: &str) -> Element<'a, Msg> {
    let p = a.p;
    let v = a.pref(k, "");
    column![t(label.to_string(), 13.0, p.text), t(hint.to_string(), 12.0, p.muted), w::input(&p, "found by itself", &v, 12.0, move |s| Msg::Folder(k, s)).width(Length::Fixed(480.0))].spacing(4.0).into()
}

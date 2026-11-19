//! Home (settings-A1-home-mouse.png): the PC card with Rename and About this PC, the Wi-Fi, Bluetooth, Updates and
//! Hearth Link tiles, Recommended settings, Storage and Changed lately with Undo. Its own short reads for the tiles.

use crate::data;
use crate::shared::{io_helper, Shared};
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, btn, hline, t, Kind, G};
use hearth_ui::Pal;
use iced::widget::{column, container, row, Column, Space};
use iced::{Color, Element, Length, Subscription, Task};

pub const PAGES: &[&str] = &["home"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Src {
    Wifi,
    Bt,
    Storage,
    Display,
    Updates,
}

#[derive(Default)]
pub struct State {
    pub wifi: Option<data::Wifi>,
    pub bt: Option<data::Bt>,
    pub storage: Vec<data::Drive>,
    pub night: bool,
    pub updates: Option<data::Updates>,
    pub start: String,
    pub rename: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Got(Src, run::Output),
    Did(&'static str, run::Output),
    Night(bool),
    StartIn(&'static str),
    Rename,
    RenameText(String),
    RenameSave,
    RenameCancel,
    Undo(i64),
    Go(&'static str),
}

pub fn read(s: Src) -> Task<Msg> {
    let cmd = match s {
        Src::Wifi => data::sh(data::WIFI),
        Src::Bt => data::sh(data::BT),
        Src::Storage => data::sh(data::STORAGE),
        Src::Display => data::sh("kreadconfig6 --file kwinrc --group NightColor --key Active 2>/dev/null"),
        Src::Updates => vec!["pkexec".into(), data::ADMIN.into(), "update-status".into()],
    };
    Task::perform(run::spawn(cmd), move |o| Msg::Got(s, o))
}

pub fn open(_sh: &mut Shared, st: &mut State, _page: &'static str) -> Task<Msg> {
    st.start = data::start_mode();
    Task::batch([read(Src::Wifi), read(Src::Bt), read(Src::Storage), read(Src::Display), read(Src::Updates)])
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Got(s, o) => match s {
            Src::Wifi => st.wifi = Some(data::wifi(&o.out)),
            Src::Bt => st.bt = Some(data::bt(&o.out)),
            Src::Storage => st.storage = data::storage(&o.out),
            Src::Display => st.night = o.out.trim() == "true",
            Src::Updates => st.updates = Some(o.json().filter(|_| o.ok).map(|v| data::updates(&v)).unwrap_or_default()),
        },
        Msg::Did(what, o) => {
            sh.result(what, &o);
            return read(Src::Display);
        }
        Msg::Night(on) => {
            st.night = on;
            return Task::perform(run::spawn(data::night_cmd(on)), |o| Msg::Did("Couldn't change Night light", o));
        }
        Msg::StartIn(mode) => {
            st.start = mode.to_string();
            let cmd = sh.conf_cmd(&data::session_conf(), "NIMBUS_START_MODE", mode);
            return Task::perform(run::spawn(cmd), |o| Msg::Did("Couldn't save Start in", o));
        }
        Msg::Rename => st.rename = Some(sh.info.host.clone()),
        Msg::RenameText(t) => st.rename = Some(data::host_chars(&t)),
        Msg::RenameSave => {
            if let Some(n) = st.rename.take().filter(|n| !n.is_empty()) {
                sh.info.host = n.clone();
                return Task::perform(run::spawn(vec!["pkexec".into(), data::ADMIN.into(), "hostname".into(), n]), |o| Msg::Did("Couldn't rename this PC", o));
            }
        }
        Msg::RenameCancel => st.rename = None,
        Msg::Undo(id) => {
            sh.reread_history = true;
            return Task::perform(run::spawn(vec!["python3".into(), io_helper(), "undo".into(), id.to_string()]), |o| Msg::Did("That change couldn't be undone (it was changed again since)", o));
        }
        Msg::Go(id) => sh.go(id),
    }
    Task::none()
}

pub fn rename_box<'a, M: Clone + 'a>(p: &Pal, n: &str, on: impl Fn(String) -> M + 'a, save: M, cancel: M) -> Element<'a, M> {
    column![
        hearth_ui::ui::ts("Rename this PC", sz(18.0), p.text),
        t("Letters, numbers and dashes. Your other devices see this name.", sz(13.0), p.muted),
        field(p, "chris-hearth", n, on).on_submit(save.clone()).width(Length::Fixed(320.0)),
        row![btn(p, "Save", Kind::Primary, false, if n.is_empty() { None } else { Some(save) }), btn(p, "Cancel", Kind::Normal, false, Some(cancel))].spacing(8.0)
    ]
    .spacing(10.0)
    .into()
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    if page != "home" {
        return None;
    }
    let sh = &a.sh;
    let st = &a.home;
    let p = sh.p;
    let i = &sh.info;
    let shot = container(Space::new()).width(Length::Fixed(220.0)).height(Length::Fixed(124.0)).style(ui::boxed(Pal::mix(p.bg2, p.accent, 0.18), p.r(8.0), Some((Color::BLACK, 6.0))));
    let booted = st.updates.as_ref().map(|u| data::shown_version(&u.booted)).unwrap_or_default();
    let mut facts = vec![if booted.is_empty() { "Hearth OS".to_string() } else { format!("Hearth OS {booted}") }, "Desktop Mode".to_string()];
    if !i.gpu.is_empty() {
        facts.push(i.gpu.clone());
    }
    if !i.cpu.is_empty() {
        facts.push(data::short_cpu(&i.cpu));
    }
    let pc: Element<Msg> = match &st.rename {
        Some(n) => card(&p, row![shot, rename_box(&p, n, Msg::RenameText, Msg::RenameSave, Msg::RenameCancel)].spacing(26.0).align_y(iced::Center)),
        None => card(&p, row![shot, column![
            big_title(&p, if i.host.is_empty() { "This PC".to_string() } else { i.host.clone() }),
            t(facts.join(" · "), sz(14.0), p.muted),
            row![pill(&p, "Rename", true, Some(Msg::Rename)), pill(&p, "About this PC", false, Some(Msg::Go("about")))].spacing(10.0)
        ].spacing(8.0)].spacing(26.0).align_y(iced::Center)),
    };
    let wifi_s = match &st.wifi {
        None => "…".to_string(),
        Some(w) if !w.wired.is_empty() => "Wired · connected".into(),
        Some(w) => match w.nets.iter().find(|n| n.active) {
            Some(n) => format!("{} · {}", n.ssid, data::signal_word(n.signal)),
            None if w.on => "Not connected".into(),
            None => "Off".into(),
        },
    };
    let bt_s = match &st.bt {
        None => "…".to_string(),
        Some(b) if !b.has => "No Bluetooth".into(),
        Some(b) if !b.on => "Off".into(),
        Some(b) => format!("On · {} connected", b.devs.iter().filter(|d| d.connected).count()),
    };
    let up_s = match &st.updates {
        None => "…".to_string(),
        Some(u) if !u.staged.is_empty() => "Restart to update".into(),
        Some(u) if !u.available.is_empty() => format!("{} available", data::shown_version(&u.available)),
        Some(u) if u.booted.is_empty() => "See what's new".into(),
        Some(u) => format!("Up to date · {}", data::shown_version(&u.booted)),
    };
    let tiles = column![
        row![tile(&p, G::Wifi, "Wi-Fi", wifi_s, Msg::Go("wifi")), tile(&p, G::Bt, "Bluetooth", bt_s, Msg::Go("bluetooth"))].spacing(16.0),
        row![tile(&p, G::Update, "Updates", up_s, Msg::Go("updates")), tile(&p, G::Link, "Hearth Link", crate::cats::link::summary(a), Msg::Go("link"))].spacing(16.0)
    ]
    .spacing(16.0)
    .width(Length::FillPortion(9));
    let top = row![container(pc).width(Length::FillPortion(13)), tiles].spacing(18.0);

    let mut rec = Column::new().spacing(10.0).push(inner_row(&p, G::Moon, "Night light", "Warmer colours from sunset to sunrise", switch(&p, st.night, Some(Msg::Night(!st.night)))));
    rec = rec.push(hline(p.line)).push(inner_row(&p, G::Power, "Start in", "What this PC opens when it turns on", seg(&p, vec![
        ("Game Mode".into(), st.start == "game", Some(Msg::StartIn("game"))),
        ("Desktop Mode".into(), st.start == "desktop", Some(Msg::StartIn("desktop"))),
    ])));
    let rec = titled_card(&p, "Recommended settings", Some(("See more", Msg::Go("quick"))), rec);
    let storage = titled_card(&p, "Storage", Some(("Manage", Msg::Go("storage"))), drive_rows(&p, &st.storage));
    let mut changed = Column::new().spacing(10.0);
    if sh.history.is_empty() {
        changed = changed.push(t("Nothing changed in Settings yet.", sz(13.0), p.muted));
    }
    for (n, c) in sh.history.iter().take(3).enumerate() {
        if n > 0 {
            changed = changed.push(hline(p.line));
        }
        changed = changed.push(
            row![
                column![t(format!("{} → {}", data::setting_name(&c.key), c.to), sz(15.0), p.text), t(format!("{} · {}", c.who, hearth_ui::time::when(c.at)), sz(13.0), p.muted)].spacing(2.0).width(Length::Fill),
                btn(&p, "Undo", Kind::Normal, false, Some(Msg::Undo(c.id)))
            ]
            .align_y(iced::Center),
        );
    }
    let changed = titled_card(&p, "Changed lately", Some(("All changes", Msg::Go("changes"))), changed);
    let lower = row![container(rec).width(Length::FillPortion(1)), column![storage, changed].spacing(18.0).width(Length::FillPortion(1))].spacing(18.0);
    Some(column![top, lower].spacing(18.0).into())
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(_a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    None
}

pub fn back(st: &mut State) -> bool {
    st.rename.take().is_some()
}

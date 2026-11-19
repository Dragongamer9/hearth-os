//! Personalization: Hearth's looks, dark or light, the accent. desktop.json through hearth_settings_io.py; then
//! hearth-plasma-looks puts that look's Global Theme on in Plasma (and a Global Theme picked in KDE shows here).

use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, t, t1, ts, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Subscription, Task};

pub const PAGES: &[&str] = &["appearance"];

#[derive(Default)]
pub struct State {}

#[derive(Debug, Clone)]
pub enum Msg {
    Did(run::Output),
    Look(String),
    Mode(&'static str),
    Accent(String),
    Go(&'static str),
}

pub fn open(_sh: &mut Shared, _st: &mut State, _page: &'static str) -> Task<Msg> {
    Task::none()
}

pub fn update(sh: &mut Shared, _st: &mut State, m: Msg) -> Task<Msg> {
    let change = match m {
        Msg::Did(o) => {
            sh.result("That setting couldn't be saved", &o);
            return Task::none();
        }
        Msg::Look(l) => serde_json::json!({ "look": l }),
        Msg::Mode(md) => serde_json::json!({ "mode": md }),
        Msg::Accent(c) => serde_json::json!({ "accent": c }),
        Msg::Go(id) => {
            sh.go(id);
            return Task::none();
        }
    };
    let cmd = sh.desktop_cmd(change);
    Task::perform(run::spawn(cmd), Msg::Did)
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    if page != "appearance" {
        return None;
    }
    let sh = &a.sh;
    let p = sh.p;
    let pal = hearth_ui::theme::palette_map();
    let cur_look = { let l = sh.d_str("look"); if l.is_empty() { "hearth".to_string() } else { l } };
    let cur_mode = sh.d_str("mode");
    let cur_accent = sh.d_str("accent").to_lowercase();
    let mut looks: Vec<(String, String, i64, serde_json::Map<String, serde_json::Value>)> = pal
        .iter()
        .filter(|(k, v)| !k.starts_with('_') && v.get("dark").map(|d| d.is_object()).unwrap_or(false))
        .map(|(k, v)| (k.clone(), v.get("label").and_then(|l| l.as_str()).unwrap_or(k).to_string(), v.get("order").and_then(|o| o.as_i64()).unwrap_or(99), v.get("dark").and_then(|d| d.as_object()).cloned().unwrap_or_default()))
        .collect();
    looks.sort_by_key(|l| l.2);
    let mut c = Column::new().spacing(10.0);
    c = c.push(t("Hearth's look for the whole desktop: KDE's Global Theme, its colours and Hearth's apps all follow it. A Global Theme you pick in KDE's System Settings shows here too.", sz(14.0), p.muted));
    c = c.push(head(&p, "Look"));
    let mut grid = Column::new().spacing(14.0);
    let mut line = Row::new().spacing(14.0);
    for (n, (id, label, _, dark)) in looks.iter().enumerate() {
        let col = |k: &str, d: &str| dark.get(k).and_then(|v| v.as_str()).and_then(hearth_ui::theme::hex).unwrap_or_else(|| hearth_ui::theme::hex(d).unwrap());
        let (bg, panel, accent, text) = (col("bg", "#111111"), col("panel2", "#222222"), col("ember", "#ff8a3d"), col("text", "#eeeeee"));
        let on = *id == cur_look;
        let preview = container(column![
            container(Space::new()).width(Length::Fixed(110.0)).height(Length::Fixed(6.0)).style(ui::boxed(text, 3.0, None)),
            container(Space::new()).width(Length::Fixed(80.0)).height(Length::Fixed(5.0)).style(ui::boxed(Pal::a(text, 0.5), 3.0, None)),
            row![container(Space::new()).width(Length::Fixed(40.0)).height(Length::Fixed(14.0)).style(ui::boxed(accent, 3.0, None)), container(Space::new()).width(Length::Fixed(40.0)).height(Length::Fixed(14.0)).style(ui::boxed(Pal::mix(panel, text, 0.1), 3.0, None))].spacing(6.0)
        ].spacing(8.0))
        .padding(16.0)
        .width(Length::Fill)
        .height(Length::Fixed(sz(96.0)))
        .style(ui::boxed(bg, p.r(8.0), Some((if on { p.accent } else { Pal::mix(bg, text, 0.15) }, if on { 3.0 } else { 1.0 }))));
        let name = if on { ts(label.clone(), sz(14.0), p.text) } else { t(label.clone(), sz(14.0), p.text) };
        let tile = mouse_area(column![preview, container(name).center_x(Length::Fill)].spacing(8.0)).on_press(Msg::Look(id.clone())).interaction(mouse::Interaction::Pointer);
        line = line.push(container(tile).width(Length::FillPortion(1)));
        if n % 4 == 3 {
            grid = grid.push(line);
            line = Row::new().spacing(14.0);
        }
    }
    let rest = looks.len() % 4;
    if rest != 0 {
        for _ in rest..4 {
            line = line.push(Space::new().width(Length::FillPortion(1)));
        }
        grid = grid.push(line);
    }
    c = c.push(card(&p, grid));
    c = c.push(head(&p, "Colours"));
    c = c.push(srow(&p, G::Moon, "Mode", "Dark or light", seg(&p, vec![("Dark".into(), cur_mode != "light", Some(Msg::Mode("dark"))), ("Light".into(), cur_mode == "light", Some(Msg::Mode("light")))])));
    let own_on = cur_accent.is_empty();
    let mut sw = Row::new().spacing(10.0).align_y(iced::Center);
    sw = sw.push(mouse_area(container(t1("The look's own", sz(13.0), if own_on { p.ink } else { p.text })).padding(Padding { top: 5.0, right: 12.0, bottom: 5.0, left: 12.0 }).style(ui::boxed(if own_on { p.accent } else { Color::TRANSPARENT }, 14.0, Some((p.line, 1.0))))).on_press(Msg::Accent(String::new())).interaction(mouse::Interaction::Pointer));
    if let Some(list) = pal.get("accents").and_then(|v| v.as_array()) {
        for acc in list {
            let Some(hex) = acc.get("color").and_then(|c| c.as_str()) else { continue };
            let Some(cc) = hearth_ui::theme::hex(hex) else { continue };
            let on = cur_accent == hex.to_lowercase();
            let s = sz(26.0);
            sw = sw.push(mouse_area(container(Space::new()).width(Length::Fixed(s)).height(Length::Fixed(s)).style(ui::boxed(cc, s / 2.0, Some((if on { p.text } else { Color::TRANSPARENT }, 2.5))))).on_press(Msg::Accent(hex.to_string())).interaction(mouse::Interaction::Pointer));
        }
    }
    c = c.push(srow(&p, G::Palette, "Accent colour", "Buttons, switches and what's picked", sw.into()));
    Some(c.into())
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(_a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    None
}

pub fn back(_st: &mut State) -> bool {
    false
}

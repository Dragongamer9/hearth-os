//! Settings › Quick (settings-B1-quick-controller.png; Mantel's SetQuick.qml): the controller look's first tab, big
//! tiles for the settings people change most. Light mode while you play (wide), Wi-Fi, Bluetooth, Brightness and
//! Volume (sliders, left and right change them), Start in (wide, Game Mode / Desktop Mode), Updates, Power, Night
//! light, Do not disturb. X (Pin to Quick) picks which tiles are here; the pick is kept in
//! ~/.local/share/nimbus/settings-quick.json (merged through hearth_settings_io.py like every settings file; missing =
//! all of them), the same file Mantel's Quick uses. Every tile is the same setting as on its own page, changed with the
//! same tools (nothing here keeps a second copy of a value). Read once when the tab opens; nothing polls.
//!
//! The same shape as a category module (cats/mod.rs), so main.rs lists it with them.

use crate::data;
use crate::nav::{self, Kind};
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, glyph, t, t1, tb, ts, G};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, slider, Column, Row, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Subscription, Task, Theme};
use serde_json::{json, Value};

pub const PAGES: &[&str] = &["quick"];

/// Every tile, in the order they're laid out, with its name and how many of the 4 columns it takes.
pub const TILES: &[(&str, &str, u16)] = &[
    ("lightmode", "Light mode while you play", 2),
    ("wifi", "Wi-Fi", 1),
    ("bt", "Bluetooth", 1),
    ("brightness", "Brightness", 1),
    ("volume", "Volume", 1),
    ("startin", "Start in", 2),
    ("updates", "Updates", 1),
    ("power", "Power", 1),
    ("night", "Night light", 1),
    ("dnd", "Do not disturb", 1),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Src {
    Wifi,
    Bt,
    Bright,
    Sound,
    Night,
    Updates,
    Power,
    Dnd,
    Light,
}

#[derive(Default)]
pub struct State {
    pub wifi: Option<data::Wifi>,
    pub bt: Option<data::Bt>,
    pub bright: Option<u32>,
    pub sound: Option<data::Sound>,
    pub night: Option<bool>,
    pub updates: Option<data::Updates>,
    pub power: Option<data::Power>,
    pub dnd: Option<bool>,
    /// The home server's "Light mode while you play" (None: no home server here, the tile isn't shown).
    pub light: Option<bool>,
    pub start: String,
    /// The tiles pinned (None: the file isn't there, all of them).
    pub pinned: Option<Vec<String>>,
    /// Pin to Quick (X) is open.
    pub picking: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Arrange this page's parts (secs.rs parts_dyn; main.rs gives it to Describe it)
    Arrange(hearth_ui::describe::DMsg),
    Got(Src, run::Output),
    Did(&'static str, Option<Src>, run::Output),
    Light,
    Night,
    Dnd,
    Bright(u32),
    BrightSet,
    Vol(f32),
    VolSet,
    StartIn(&'static str),
    Pin(&'static str),
    Picking,
    Go(&'static str),
}

impl From<hearth_ui::describe::DMsg> for Msg {
    fn from(d: hearth_ui::describe::DMsg) -> Msg {
        Msg::Arrange(d)
    }
}

/// A program with a time limit (red flag: nothing waits forever).
fn limited(secs: u32, cmd: Vec<String>) -> Vec<String> {
    let mut v = vec!["timeout".to_string(), secs.to_string()];
    v.extend(cmd);
    v
}

fn helper(name: &str, env: &str) -> String {
    std::env::var(env).unwrap_or_else(|_| run::shell_helper(name))
}

pub fn quick_file() -> std::path::PathBuf {
    hearth_ui::paths::home().join(".local/share/nimbus/settings-quick.json")
}

pub fn read(s: Src) -> Task<Msg> {
    let cmd = match s {
        Src::Wifi => limited(8, data::sh(data::WIFI)),
        Src::Bt => limited(10, data::sh(data::BT)),
        Src::Bright => limited(4, data::sh("brightnessctl -m -c backlight 2>/dev/null | head -1")),
        Src::Sound => limited(6, data::sh(data::SOUND)),
        Src::Night => limited(4, data::sh("kreadconfig6 --file kwinrc --group NightColor --key Active 2>/dev/null")),
        Src::Updates => limited(30, vec!["pkexec".into(), data::ADMIN.into(), "update-status".into()]),
        Src::Power => limited(6, data::sh(data::POWER)),
        // Hearth's own Do not disturb (hearth-desk, as Mantel's DeskState): on Plasma, KDE's own is in its tray.
        Src::Dnd => limited(6, vec!["python3".into(), run::shell_helper("hearth-desk"), "state".into()]),
        Src::Light => limited(15, vec!["python3".into(), helper("hearth-home-server", "HEARTH_HOME_SERVER"), "status".into()]),
    };
    Task::perform(run::spawn(cmd), move |o| Msg::Got(s, o))
}

/// What settings-quick.json pins (None: no file, or no list in it: every tile).
pub fn pinned_from(v: &Value) -> Option<Vec<String>> {
    v.get("tiles").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).filter(|x| TILES.iter().any(|t| t.0 == *x)).map(String::from).collect())
}

pub fn open(sh: &mut Shared, st: &mut State, _page: &'static str) -> Task<Msg> {
    st.start = data::start_mode();
    st.pinned = pinned_from(&hearth_ui::settings_file::read_json(&quick_file()));
    // `hearth-settings quick:pin` opens Pin to Quick.
    st.picking = sh.part == "pin";
    Task::batch([Src::Wifi, Src::Bt, Src::Bright, Src::Sound, Src::Night, Src::Updates, Src::Power, Src::Dnd, Src::Light].map(read))
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    let run_it = |cmd: Vec<String>, what: &'static str, again: Option<Src>| Task::perform(run::spawn(limited(20, cmd)), move |o| Msg::Did(what, again, o));
    match m {
        Msg::Arrange(_) => return Task::none(),
        Msg::Got(s, o) => match s {
            Src::Wifi => st.wifi = Some(data::wifi(&o.out)),
            Src::Bt => st.bt = Some(data::bt(&o.out)),
            Src::Bright => st.bright = o.out.trim().split(',').nth(3).and_then(|p| p.trim_end_matches('%').parse().ok()),
            Src::Sound => st.sound = Some(data::sound(&o.out)).filter(|s| s.ok),
            Src::Night => st.night = Some(o.out.trim() == "true"),
            Src::Updates => st.updates = Some(o.json().filter(|_| o.ok).map(|v| data::updates(&v)).unwrap_or_default()),
            Src::Power => st.power = Some(data::power(&o.out)),
            Src::Dnd => st.dnd = o.json().and_then(|v| v.get("dnd").and_then(Value::as_bool)),
            Src::Light => st.light = o.json().and_then(|v| v.get("settings").and_then(|s| s.get("lightMode")).map(|x| x.as_bool() != Some(false))),
        },
        Msg::Did(what, again, o) => {
            sh.result(what, &o);
            if let Some(s) = again {
                return read(s);
            }
        }
        Msg::Light => {
            let on = !st.light.unwrap_or(true);
            st.light = Some(on);
            return run_it(vec!["python3".into(), helper("hearth-home-server", "HEARTH_HOME_SERVER"), "set".into(), "lightMode".into(), if on { "on" } else { "off" }.into()], "Couldn't change Light mode while you play", Some(Src::Light));
        }
        Msg::Night => {
            let on = !st.night.unwrap_or(false);
            st.night = Some(on);
            return run_it(data::night_cmd(on), "Couldn't change Night light", Some(Src::Night));
        }
        Msg::Dnd => {
            let on = !st.dnd.unwrap_or(false);
            st.dnd = Some(on);
            return run_it(vec!["python3".into(), run::shell_helper("hearth-desk"), "set".into(), "dnd".into(), if on { "on" } else { "off" }.into()], "Couldn't change Do not disturb", Some(Src::Dnd));
        }
        Msg::Bright(v) => st.bright = Some(v.max(1)),
        Msg::BrightSet => {
            if let Some(b) = st.bright {
                return run_it(vec!["brightnessctl".into(), "-q".into(), "-c".into(), "backlight".into(), "set".into(), format!("{}%", b.max(1))], "Couldn't change the brightness", None);
            }
        }
        Msg::Vol(v) => {
            if let Some(s) = st.sound.as_mut() {
                s.vol = v.clamp(0.0, 1.0);
            }
        }
        Msg::VolSet => {
            if let Some(s) = &st.sound {
                return run_it(data::sh(&format!("wpctl set-mute @DEFAULT_AUDIO_SINK@ 0; wpctl set-volume @DEFAULT_AUDIO_SINK@ {:.2}", s.vol)), "Couldn't change the volume", None);
            }
        }
        Msg::StartIn(mode) => {
            st.start = mode.to_string();
            let cmd = sh.conf_cmd(&data::session_conf(), "NIMBUS_START_MODE", mode);
            return run_it(cmd, "Couldn't save Start in", None);
        }
        Msg::Pin(id) => {
            let all: Vec<String> = TILES.iter().map(|t| t.0.to_string()).collect();
            let now = st.pinned.clone().unwrap_or_else(|| all.clone());
            let next: Vec<String> = if now.iter().any(|x| x == id) { now.into_iter().filter(|x| x != id).collect() } else { all.into_iter().filter(|x| x == id || now.contains(x)).collect() };
            st.pinned = Some(next.clone());
            let cmd = sh.json_cmd(&quick_file(), json!({ "tiles": next }));
            return run_it(cmd, "Couldn't save what's on Quick", None);
        }
        Msg::Picking => st.picking = !st.picking,
        Msg::Go(id) => sh.go(id),
    }
    Task::none()
}

/// X on the Quick tab: Pin to Quick (open, or done).
pub fn x_label(st: &State) -> &'static str {
    if st.picking { "Done pinning" } else { "Pin to Quick" }
}

fn chip<'a>(p: &Pal, on: bool) -> Element<'a, Msg> {
    let c = if on { p.accent } else { p.muted };
    container(t1(if on { "On" } else { "Off" }, 13.0, c)).padding(Padding { top: 2.0, right: 10.0, bottom: 2.0, left: 10.0 }).style(ui::boxed(Color::TRANSPARENT, 12.0, Some((c, 1.2)))).into()
}

fn knob_switch<'a>(p: &Pal, on: bool) -> Element<'a, Msg> {
    let p = *p;
    let k = 20.0;
    let knob = container(Space::new()).width(Length::Fixed(k)).height(Length::Fixed(k)).style(ui::boxed(if on { p.ink } else { Pal::mix(p.text, p.muted, 0.3) }, k / 2.0, None));
    let inner: Element<'a, Msg> = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    container(inner).padding(4.0).width(Length::Fixed(52.0)).height(Length::Fixed(28.0)).style(ui::boxed(if on { p.accent } else { Pal::mix(p.panel2, p.text, 0.12) }, 14.0, None)).into()
}

fn tile_slider<'a>(p: &Pal, v: f32, on: impl Fn(f32) -> Msg + 'a, release: Msg) -> Element<'a, Msg> {
    let p = *p;
    slider(0.0..=1.0, v.clamp(0.0, 1.0), on)
        .step(0.01)
        .on_release(release)
        .width(Length::Fill)
        .height(18.0)
        .style(move |_: &Theme, _| slider::Style {
            rail: slider::Rail { backgrounds: (Background::Color(p.accent), Background::Color(Pal::mix(p.panel2, p.text, 0.12))), width: 6.0, border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 3.0.into() } },
            handle: slider::Handle { shape: slider::HandleShape::Circle { radius: 9.0 }, background: Background::Color(p.text), border_width: 0.0, border_color: Color::TRANSPARENT },
        })
        .into()
}

/// One big tile: a place for the controller's focus (A does `a`; a slider or choices drawn for it give left and
/// right), lifted a little and outlined when it's the focused one (B1).
fn big_tile<'a>(p: &Pal, title: &str, sub: &str, kind: Kind, a: Option<Msg>, inner: Element<'a, Msg>, h: f32) -> Element<'a, Msg> {
    nav::control(kind, a.iter().cloned().map(|m| Box::new(m) as nav::Act).collect(), Vec::new(), Vec::new(), String::new(), String::new());
    let r = nav::item(title, sub, true);
    let c = container(inner).id(r.key).padding(Padding { top: 16.0, right: 20.0, bottom: 16.0, left: 20.0 }).width(Length::Fill).height(Length::Fixed(h)).style(row_style(*p, p.panel, p.r(14.0), r.lit));
    let lift = if r.lit { Padding { top: 0.0, right: 0.0, bottom: 3.0, left: 0.0 } } else { Padding { top: 3.0, right: 0.0, bottom: 0.0, left: 0.0 } };
    let c: Element<'a, Msg> = container(ring(p, r.lit, p.r(14.0), c)).padding(lift).into();
    match a {
        Some(m) => mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => c,
    }
}

/// A slider tile's left and right: a 5 % step, then saved (as Mantel's tile.adjusted).
fn slider_acts(v: f32, on: fn(f32) -> Msg, set: Msg) -> (Vec<nav::Act>, Vec<nav::Act>) {
    (vec![Box::new(on((v - 0.05).max(0.0))), Box::new(set.clone())], vec![Box::new(on((v + 0.05).min(1.0))), Box::new(set)])
}

fn bright_msg(v: f32) -> Msg {
    Msg::Bright((v * 100.0).round().max(1.0) as u32)
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    if page != "quick" {
        return None;
    }
    let p = a.sh.p;
    let st = &a.quick;
    let b = big();
    let h = if b { 134.0 } else { 120.0 };
    let pinned = |id: &str| st.pinned.as_ref().map(|v| v.iter().any(|x| x == id)).unwrap_or(true);
    if st.picking {
        let mut c = Column::new().spacing(10.0).push(ts("Pin to Quick: pick what's on this tab (A pins or unpins, X when you're done)", sz(17.0), p.text));
        for (id, name, _) in TILES {
            let on = pinned(id);
            c = c.push(srow(&p, G::Grid, *name, "", switch(&p, on, Some(Msg::Pin(*id)))));
        }
        return Some(c.into());
    }
    let icon = |g: G| glyph(g, p.accent, if b { 26.0 } else { 24.0 }, 2.2);
    let words = |title: &str, sub: String| -> Element<'a, Msg> { column![tb(title.to_string(), if b { 17.0 } else { 16.0 }, p.text), t1(sub, 13.0, p.muted)].spacing(2.0).into() };
    let mut tiles: Vec<(u16, &'static str, Element<'a, Msg>)> = Vec::new();
    for (id, name, span) in TILES {
        if !pinned(id) {
            continue;
        }
        let el: Option<Element<'a, Msg>> = match *id {
            "lightmode" => st.light.map(|on| {
                let inner = column![
                    row![icon(G::Pad), column![tb(*name, if b { 18.0 } else { 16.0 }, p.text), t("Big home server jobs wait until your game ends", if b { 14.0 } else { 13.0 }, p.muted)].spacing(3.0)].spacing(14.0),
                    Space::new().height(Length::Fill),
                    row![chip(&p, on), Space::new().width(Length::Fill), knob_switch(&p, on)].align_y(iced::Center)
                ];
                big_tile(&p, name, "Big home server jobs wait until your game ends", Kind::Toggle(on), Some(Msg::Light), inner.into(), h)
            }),
            "wifi" => {
                let (on, sub) = match &st.wifi {
                    None => (false, "…".to_string()),
                    Some(w) if !w.wired.is_empty() => (true, "Wired · connected".into()),
                    Some(w) if !w.on => (false, "Off".into()),
                    Some(w) => (true, w.nets.iter().find(|n| n.active).map(|n| format!("{} · {}", n.ssid, data::signal_word(n.signal))).unwrap_or_else(|| "Not connected".into())),
                };
                let inner = column![row![icon(G::Wifi), Space::new().width(Length::Fill), chip(&p, on)], Space::new().height(Length::Fill), words(name, sub.clone())];
                Some(big_tile(&p, name, &sub, Kind::Open, Some(Msg::Go("wifi")), inner.into(), h))
            }
            "bt" => {
                let (on, sub) = match &st.bt {
                    None => (false, "…".to_string()),
                    Some(x) if !x.has => (false, "No Bluetooth".into()),
                    Some(x) if !x.on => (false, "Off".into()),
                    Some(x) => {
                        let names: Vec<String> = x.devs.iter().filter(|d| d.connected).map(|d| d.name.clone()).collect();
                        (true, if names.is_empty() { "On · nothing connected".into() } else if names.len() > 2 { format!("{} +{}", names[..2].join(", "), names.len() - 2) } else { names.join(", ") })
                    }
                };
                let inner = column![row![icon(G::Bt), Space::new().width(Length::Fill), chip(&p, on)], Space::new().height(Length::Fill), words(name, sub.clone())];
                Some(big_tile(&p, name, &sub, Kind::Open, Some(Msg::Go("bluetooth")), inner.into(), h))
            }
            "brightness" => st.bright.map(|v| {
                let f = v as f32 / 100.0;
                let (l, r) = slider_acts(f, bright_msg, Msg::BrightSet);
                nav::control(Kind::Slider, Vec::new(), l, r, String::new(), format!("{v}%"));
                let inner = column![icon(G::Display), Space::new().height(Length::Fill), words(name, format!("{v}%")), tile_slider(&p, f, bright_msg, Msg::BrightSet)].spacing(4.0);
                big_tile(&p, name, &format!("{v}%"), Kind::Slider, Some(Msg::Go("display")), inner.into(), h)
            }),
            "volume" => st.sound.as_ref().map(|s| {
                let desc = s.outs.iter().find(|o| o.name == s.out).map(|o| o.desc.clone()).unwrap_or_else(|| "Sound".into());
                let sub = format!("{desc} · {}%", (s.vol * 100.0).round());
                let (l, r) = slider_acts(s.vol, Msg::Vol, Msg::VolSet);
                nav::control(Kind::Slider, Vec::new(), l, r, String::new(), format!("{}%", (s.vol * 100.0).round()));
                let inner = column![icon(G::Sound), Space::new().height(Length::Fill), words(name, sub.clone()), tile_slider(&p, s.vol, Msg::Vol, Msg::VolSet)].spacing(4.0);
                big_tile(&p, name, &sub, Kind::Slider, Some(Msg::Go("sound")), inner.into(), h)
            }),
            "startin" => {
                let choice = seg_flat(&p, vec![
                    ("Game Mode".into(), st.start == "game", Some(Msg::StartIn("game"))),
                    ("Desktop Mode".into(), st.start == "desktop", Some(Msg::StartIn("desktop"))),
                ]);
                let inner = column![
                    row![tb(*name, if b { 18.0 } else { 16.0 }, p.text), Space::new().width(Length::Fill), t("What this PC opens when it turns on", if b { 14.0 } else { 13.0 }, p.muted)].align_y(iced::Center),
                    Space::new().height(Length::Fill),
                    choice
                ];
                Some(big_tile(&p, name, "What this PC opens when it turns on", Kind::Choice, None, inner.into(), h))
            }
            "updates" => {
                let sub = match &st.updates {
                    None => "Checking…".to_string(),
                    Some(u) if !u.read || (u.booted.is_empty() && u.available.is_empty()) => "Open to check".into(),
                    Some(u) if !u.staged.is_empty() => "Restart to finish".into(),
                    Some(u) if !u.available.is_empty() => format!("Hearth {} is ready", data::shown_version(&u.available)),
                    Some(u) => format!("Up to date · Hearth OS {}", data::shown_version(&u.booted)),
                };
                let inner = column![icon(G::Update), Space::new().height(Length::Fill), words(name, sub.clone())];
                Some(big_tile(&p, name, &sub, Kind::Open, Some(Msg::Go("updates")), inner.into(), h))
            }
            "power" => {
                let mins = a.sh.d_num("sleepMinutes", 0.0) as i64;
                let sleep = if mins == 0 { "sleeps never".to_string() } else if mins >= 60 { format!("sleeps after {} h", mins / 60) } else { format!("sleeps after {mins} min") };
                let prof = st.power.as_ref().filter(|x| !x.profile.is_empty()).map(|x| data::profile_name(&x.profile)).unwrap_or("Power");
                let sub = format!("{prof} · {sleep}");
                let inner = column![icon(G::Power), Space::new().height(Length::Fill), words(name, sub.clone())];
                Some(big_tile(&p, name, &sub, Kind::Open, Some(Msg::Go("power")), inner.into(), h))
            }
            "night" => {
                let on = st.night.unwrap_or(false);
                let sub = if on { "On" } else { "Off" }.to_string();
                let inner = column![row![icon(G::Moon), Space::new().width(Length::Fill), knob_switch(&p, on)], Space::new().height(Length::Fill), words(name, sub.clone())];
                Some(big_tile(&p, name, &sub, Kind::Toggle(on), Some(Msg::Night), inner.into(), h))
            }
            _ => {
                let on = st.dnd.unwrap_or(false);
                let sub = if on { "On" } else { "Off" }.to_string();
                let inner = column![row![icon(G::Moon), Space::new().width(Length::Fill), knob_switch(&p, on)], Space::new().height(Length::Fill), words(name, sub.clone())];
                Some(big_tile(&p, name, &sub, Kind::Toggle(on), Some(Msg::Dnd), inner.into(), h))
            }
        };
        if let Some(e) = el {
            tiles.push((*span, *name, e));
        }
    }
    // Each tile a block (secs.rs parts_dyn); as it always was when nothing is customised:
    let empty = st.pinned.as_ref().map(|v| v.is_empty()).unwrap_or(false);
    let spans: Vec<u16> = tiles.iter().map(|x| x.0).collect();
    let items: Vec<(String, Element<'a, Msg>)> = tiles.into_iter().map(|(_, n, e)| (n.to_string(), e)).collect();
    Some(crate::secs::parts_dyn(a, items, move |els| tile_grid(p, b, spans.into_iter().zip(els).collect(), empty)))
}

/// Four columns; a wide tile takes two, and moves to the next line when it doesn't fit.
fn tile_grid<'a>(p: hearth_ui::Pal, b: bool, tiles: Vec<(u16, Element<'a, Msg>)>, empty: bool) -> Element<'a, Msg> {
    let mut grid = Column::new().spacing(if b { 16.0 } else { 14.0 });
    let mut line = Row::new().spacing(if b { 16.0 } else { 14.0 });
    let mut used = 0u16;
    for (span, e) in tiles {
        if used + span > 4 {
            // (A full line gets no spacer: a FillPortion(0) one takes the line's whole width.)
            if used < 4 {
                line = line.push(Space::new().width(Length::FillPortion(4 - used)));
            }
            grid = grid.push(line);
            line = Row::new().spacing(if b { 16.0 } else { 14.0 });
            used = 0;
        }
        line = line.push(container(e).width(Length::FillPortion(span)));
        used += span;
    }
    if used > 0 {
        if used < 4 {
            line = line.push(Space::new().width(Length::FillPortion(4 - used)));
        }
        grid = grid.push(line);
    }
    if empty {
        grid = grid.push(note(&p, "Nothing is pinned to Quick. X picks what's here.", false));
    }
    grid.into()
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(_a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    None
}

pub fn back(st: &mut State) -> bool {
    std::mem::take(&mut st.picking)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pick_of_tiles() {
        assert_eq!(pinned_from(&json!({})), None);
        assert_eq!(pinned_from(&json!({"tiles": ["wifi", "nope", "dnd"]})), Some(vec!["wifi".to_string(), "dnd".to_string()]));
        assert_eq!(TILES.iter().map(|t| t.2).sum::<u16>(), 12);
    }
}

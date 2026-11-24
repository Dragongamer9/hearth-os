//! Home (the approved Settings cleanup, docs/mockups/settings-cleanup/settings-01-home.png, Nov 23, 2026; it replaced
//! settings-A1's Home): "N things need you" (Hearth OS updates ready, Hearth parts to install) with Later and Open the
//! Store, four tiles (Storage, Your AI, Home server, Hearth Link), Quick (Wi-Fi, Bluetooth, Night light and Do not
//! disturb switches, Volume and Brightness sliders), You use these most (the five pages you open most, counted in a
//! small state file by hearth-settings-values, never a setting) and This PC (Name with Rename, Hearth OS, Graphics,
//! Memory, Send logs with Send). Every value is read from this PC (hearth-settings-values home). Quick (SetQuick.qml, the big tiles of settings-B1 in this look, with Pin to
//! Quick) and All changes (SetChangesPage.qml, the whole change history with Undo). Its own short reads.
//!
//! Do not disturb: on KDE Plasma (the Fedora edition) it is Plasma's own (plasmanotifyrc's DoNotDisturb Until, a year
//! ahead as Plasma's own button sets it, with --notify so Plasma follows at once); on Mantel's desktop it is
//! hearth-desk's, as Mantel's Settings has it. Light mode while you play is hearth-home-server's lightMode.

use crate::cats::hs::kit::{self, H};
use crate::data;
use crate::shared::{io_helper, Shared};
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, t, ts, Kind, G};
use crate::widgets::btn;
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, Column, Row, Space};
use iced::{mouse, Color, Element, Length, Padding, Subscription, Task};
use serde_json::{json, Value};

pub const PAGES: &[&str] = &["home", "quick", "changes"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Src {
    Wifi,
    Bt,
    Storage,
    Display,
    Updates,
    Hs,
    Link,
    Dnd,
    Bright,
    Sound,
    Power,
    History,
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
    /// hearth-home-server status (Light mode while you play), hearth-link-settings status (the Hearth Link tile).
    pub hs: Value,
    pub link: Value,
    pub dnd: bool,
    pub plasma_dnd: bool,
    pub bright: Option<u32>,
    pub sound: Option<data::Sound>,
    pub profile: String,
    pub history: Value,
    pub pinned: Option<Vec<String>>,
    pub picking: bool,
    /// hearth-settings-values home: what this PC says (left out when it can't tell).
    pub vals: serde_json::Map<String, Value>,
    /// "N things need you": Later hides it until Settings opens again.
    pub later: bool,
    pub sent: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Got(Src, run::Output),
    Did(&'static str, run::Output),
    Night(bool),
    Light(bool),
    Dnd(bool),
    StartIn(&'static str),
    Rename,
    RenameText(String),
    RenameSave,
    RenameCancel,
    Undo(i64),
    Undone(String, run::Output),
    Go(&'static str),
    Picking,
    Pin(&'static str),
    Bright(u32),
    BrightSet,
    Vol(f32),
    VolSet,
    Vals(run::Output),
    Later,
    Store,
    /// A Quick switch through hearth-settings-values set (Wi-Fi: nmcli, Bluetooth: bluetoothctl).
    Sw(&'static str, &'static str, bool),
    NightSaved(run::Output),
    SendLogs,
    Sent(run::Output),
}

fn values_cmd(args: &[&str]) -> Vec<String> {
    let mut v = vec!["python3".to_string(), run::shell_helper("hearth-settings-values")];
    v.extend(args.iter().map(|x| x.to_string()));
    v
}

fn load_vals() -> Task<Msg> {
    Task::perform(run::spawn(values_cmd(&["home"])), Msg::Vals)
}

pub const TILES: [&str; 10] = ["lightmode", "wifi", "bt", "brightness", "volume", "startin", "updates", "power", "night", "dnd"];
fn tile_name(id: &str) -> &'static str {
    match id {
        "lightmode" => "Light mode while you play",
        "wifi" => "Wi-Fi",
        "bt" => "Bluetooth",
        "brightness" => "Brightness",
        "volume" => "Volume",
        "startin" => "Start in",
        "updates" => "Updates",
        "power" => "Power",
        "night" => "Night light",
        _ => "Do not disturb",
    }
}

fn quick_json() -> std::path::PathBuf {
    hearth_ui::paths::home().join(".local/share/nimbus/settings-quick.json")
}

const DND_READ: &str = "if echo \"$XDG_CURRENT_DESKTOP\" | grep -qi kde && command -v kreadconfig6 >/dev/null; then echo \"plasma $(kreadconfig6 --file plasmanotifyrc --group DoNotDisturb --key Until 2>/dev/null)\"; else echo desk; python3 \"$0\" state; fi";

/// Plasma's DoNotDisturb Until ("2027,11,18,21,4,0"): on while it's still ahead.
pub fn plasma_until_ahead(v: &str, now: hearth_ui::time::Tm) -> bool {
    let p: Vec<i64> = v.split(',').map(|x| x.trim().split('.').next().unwrap_or("").parse().unwrap_or(-1)).collect();
    if p.len() < 5 || p.iter().take(5).any(|x| *x < 0) {
        return false;
    }
    let then = (p[0], p[1], p[2], p[3], p[4]);
    let n = (now.year as i64, now.mon as i64 + 1, now.day as i64, now.hour as i64, now.min as i64);
    then > n
}

fn dnd_cmd(plasma: bool, on: bool) -> Vec<String> {
    if plasma {
        data::sh(if on {
            "kwriteconfig6 --file plasmanotifyrc --group DoNotDisturb --key Until \"$(date -d '+1 year' '+%Y,%-m,%-d,%-H,%-M,0')\" --notify"
        } else {
            "kwriteconfig6 --file plasmanotifyrc --group DoNotDisturb --key Until --delete --notify"
        })
    } else {
        vec!["python3".into(), run::shell_helper("hearth-desk"), "set".into(), "dnd".into(), if on { "on" } else { "off" }.into()]
    }
}

pub fn read(s: Src) -> Task<Msg> {
    let job = |h: H, a: &[&str]| Task::perform(kit::spawn(h, kit::sv(a), None, 30), move |o| Msg::Got(s, o));
    let cmd = match s {
        Src::Wifi => data::sh(data::WIFI),
        Src::Bt => data::sh(data::BT),
        Src::Storage => data::sh(data::STORAGE),
        Src::Display => data::sh("kreadconfig6 --file kwinrc --group NightColor --key Active 2>/dev/null"),
        Src::Updates => return job(H::Admin, &["update-status"]),
        Src::Hs => return job(H::Hs, &["status"]),
        Src::Link => return job(H::Link, &["status"]),
        Src::Dnd => vec!["sh".into(), "-c".into(), DND_READ.into(), run::shell_helper("hearth-desk")],
        Src::Bright => data::sh("brightnessctl -m -c backlight 2>/dev/null | head -1"),
        Src::Sound => data::sh(data::SOUND),
        Src::Power => data::sh(data::POWER),
        Src::History => vec!["python3".into(), io_helper(), "history".into(), "200".into()],
    };
    Task::perform(run::spawn(cmd), move |o| Msg::Got(s, o))
}

pub fn open(_sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    st.start = data::start_mode();
    st.picking = false;
    match page {
        "changes" => read(Src::History),
        "quick" => {
            if st.pinned.is_none() {
                let v = hearth_ui::settings_file::read_json(&quick_json());
                st.pinned = v.get("tiles").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).filter(|x| TILES.contains(x)).map(String::from).collect());
            }
            Task::batch([Src::Wifi, Src::Bt, Src::Display, Src::Updates, Src::Hs, Src::Dnd, Src::Bright, Src::Sound, Src::Power].map(read))
        }
        _ => Task::batch([Src::Dnd, Src::Bright, Src::Sound].map(read).into_iter().chain([load_vals()])),
    }
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Got(s, o) => match s {
            Src::Wifi => st.wifi = Some(data::wifi(&o.out)),
            Src::Bt => st.bt = Some(data::bt(&o.out)),
            Src::Storage => st.storage = data::storage(&o.out),
            Src::Display => st.night = o.out.trim() == "true",
            Src::Updates => st.updates = Some(o.json().filter(|_| o.ok).map(|v| data::updates(&v)).unwrap_or_default()),
            Src::Hs => st.hs = kit::answer(&o),
            Src::Link => st.link = kit::answer(&o),
            Src::Dnd => {
                let first = o.out.lines().next().unwrap_or("").to_string();
                st.plasma_dnd = first.starts_with("plasma");
                st.dnd = if st.plasma_dnd {
                    plasma_until_ahead(first.trim_start_matches("plasma").trim(), hearth_ui::time::local(hearth_ui::time::now()))
                } else {
                    o.json().map(|v| kit::b(&v, "dnd")).unwrap_or(false)
                };
            }
            Src::Bright => {
                let f: Vec<&str> = o.out.trim().split(',').collect();
                st.bright = f.get(3).and_then(|x| x.trim_end_matches('%').parse().ok());
            }
            Src::Sound => st.sound = Some(data::sound(&o.out)),
            Src::Power => st.profile = data::power(&o.out).profile,
            Src::History => st.history = o.json().unwrap_or(Value::Array(Vec::new())),
        },
        Msg::Did(what, o) => {
            sh.result(what, &o);
            return Task::batch([read(Src::Hs), read(Src::Dnd), read(Src::History), load_vals()]);
        }
        // Night light is desktop.json's nightLight (hearth_settings_io, with the change history), put on the screens by
        // hearth-screens night, as Settings › Display does.
        Msg::Night(on) => {
            st.night = on;
            let cmd = sh.desktop_cmd(json!({ "nightLight": on }));
            return Task::perform(run::spawn(cmd), Msg::NightSaved);
        }
        Msg::NightSaved(o) => {
            if !o.ok {
                sh.result("Couldn't change Night light", &o);
                return Task::none();
            }
            return Task::perform(run::spawn(vec!["python3".into(), run::shell_helper("hearth-screens"), "night".into()]), |o| Msg::Did("Couldn't change Night light", o));
        }
        Msg::Vals(o) => {
            if let Some(Value::Object(v)) = o.json() {
                st.vals = v;
            }
        }
        Msg::Later => st.later = true,
        Msg::Store => run::detached(&["hearth-store-app"]),
        Msg::Sw(val, key, on) => {
            st.vals.insert(val.to_string(), Value::String(if on { "on" } else { "off" }.into()));
            return Task::perform(run::spawn(values_cmd(&["set", key, if on { "on" } else { "off" }])), |o| {
                let ok = o.json().and_then(|v| v.get("ok").and_then(Value::as_bool)).unwrap_or(false);
                Msg::Did("Couldn't change it", run::Output { ok: o.ok && ok, ..o })
            });
        }
        Msg::SendLogs => {
            st.sent = "Sending…".into();
            return Task::perform(run::spawn(vec!["nimbus".into(), "--send-logs".into(), "Sent from Settings › Home".into()]), Msg::Sent);
        }
        Msg::Sent(o) => {
            st.sent = if o.ok { "Sent".into() } else { String::new() };
            sh.result("Couldn't send the logs", &o);
        }
        Msg::Light(on) => {
            return Task::perform(kit::spawn(H::Hs, kit::sv(&["set", "lightMode", if on { "on" } else { "off" }]), None, 30), |o| Msg::Did("Couldn't change Light mode", o));
        }
        Msg::Dnd(on) => {
            st.dnd = on;
            return Task::perform(run::spawn(dnd_cmd(st.plasma_dnd, on)), |o| Msg::Did("Couldn't change Do not disturb", o));
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
            let name = kit::arr(&st.history, "")
                .iter()
                .find(|c| kit::n(c, "id") as i64 == id)
                .map(|c| data::setting_name(&kit::s(c, "key")))
                .or_else(|| sh.history.iter().find(|c| c.id == id).map(|c| data::setting_name(&c.key)))
                .unwrap_or_else(|| "It".into());
            return Task::perform(run::spawn(vec!["python3".into(), io_helper(), "undo".into(), id.to_string()]), move |o| Msg::Undone(name.clone(), o));
        }
        Msg::Undone(name, o) => {
            let r = o.out.trim();
            if r == "ok" {
                sh.note = format!("{name} is back to what it was");
            } else if r == "changed" {
                sh.error = format!("{name} was changed again since: nothing undone");
            } else {
                sh.error = "That change isn't in the history any more".into();
            }
            sh.reread_history = true;
            return read(Src::History);
        }
        Msg::Go(id) => sh.go(id),
        Msg::Picking => st.picking = !st.picking,
        Msg::Pin(id) => {
            let cur = st.pinned.clone().unwrap_or_else(|| TILES.iter().map(|x| x.to_string()).collect());
            let next: Vec<String> = if cur.iter().any(|x| x == id) { cur.into_iter().filter(|x| x != id).collect() } else { TILES.iter().filter(|x| **x == id || cur.iter().any(|c| c == *x)).map(|x| x.to_string()).collect() };
            st.pinned = Some(next.clone());
            // settings-quick.json, merged (atomic, .bak) by hearth_settings_io.py: only the pick of tiles.
            let cmd = vec!["python3".into(), io_helper(), "merge".into(), quick_json().to_string_lossy().to_string(), json!({ "tiles": next }).to_string()];
            return Task::perform(run::spawn(cmd), |o| Msg::Did("Couldn't save which tiles are on Quick", o));
        }
        Msg::Bright(v) => st.bright = Some(v.max(1)),
        Msg::BrightSet => {
            if let Some(v) = st.bright {
                return Task::perform(run::spawn(vec!["brightnessctl".into(), "-q".into(), "-c".into(), "backlight".into(), "set".into(), format!("{v}%")]), |o| Msg::Did("Couldn't change the brightness", o));
            }
        }
        Msg::Vol(v) => {
            if let Some(s) = st.sound.as_mut() {
                s.vol = v;
            }
        }
        Msg::VolSet => {
            if let Some(s) = &st.sound {
                return Task::perform(run::spawn(vec!["wpctl".into(), "set-volume".into(), "@DEFAULT_AUDIO_SINK@".into(), format!("{:.2}", s.vol)]), |o| Msg::Did("Couldn't change the volume", o));
            }
        }
    }
    Task::none()
}

pub fn rename_box<'a, M: Clone + Send + 'static>(p: &Pal, n: &str, on: impl Fn(String) -> M + 'a, save: M, cancel: M) -> Element<'a, M> {
    column![
        hearth_ui::ui::ts("Rename this PC", sz(18.0), p.text),
        t("Letters, numbers and dashes. Your other devices see this name.", sz(13.0), p.muted),
        field(p, "chris-hearth", n, on).on_submit(save.clone()).width(Length::Fixed(320.0)),
        row![btn(p, "Save", Kind::Primary, false, if n.is_empty() { None } else { Some(save) }), btn(p, "Cancel", Kind::Normal, false, Some(cancel))].spacing(8.0)
    ]
    .spacing(10.0)
    .into()
}

fn wifi_text(st: &State) -> String {
    match &st.wifi {
        None => "…".to_string(),
        Some(w) if !w.wired.is_empty() => "Wired · connected".into(),
        Some(w) => match w.nets.iter().find(|n| n.active) {
            Some(n) => format!("{} · {}", n.ssid, data::signal_word(n.signal)),
            None if w.on => "Not connected".into(),
            None => "Off".into(),
        },
    }
}

fn light_mode(st: &State) -> Option<bool> {
    let cfg = kit::at(&st.hs, "settings");
    cfg.is_object().then(|| kit::nf(cfg, "lightMode"))
}

/// Start in's two choices; with Game Mode off on this PC (System › Startup), Desktop Mode and the way to turn it on.
fn start_in_items(sh: &Shared, st: &State) -> Vec<(String, bool, Option<Msg>)> {
    if crate::cats::system::game_modes_off(sh) {
        return vec![("Desktop Mode".into(), true, None), ("Game Mode: off".into(), false, Some(Msg::Go("startup")))];
    }
    vec![("Game Mode".into(), st.start == "game", Some(Msg::StartIn("game"))), ("Desktop Mode".into(), st.start == "desktop", Some(Msg::StartIn("desktop")))]
}

fn v<'a>(st: &'a State, k: &str) -> String {
    crate::cats::sections::s(&st.vals, k)
}

/// Night light's line: when it comes on (Settings › Display's times).
fn night_line(sh: &Shared) -> String {
    match sh.d_str("nightMode").as_str() {
        "always" => "on all the time".into(),
        "times" => format!("from {}", if sh.d_str("nightFrom").is_empty() { "20:00".to_string() } else { sh.d_str("nightFrom") }),
        _ => "on at sunset".into(),
    }
}

fn home_view<'a>(a: &'a App) -> Element<'a, Msg> {
    let sh = &a.sh;
    let st = &a.home;
    let p = sh.p;
    let mut out = Column::new().spacing(18.0);
    // N things need you (yellow dot): what hearth-settings-values found waiting; hidden when nothing is, or after Later.
    let needs: Vec<String> = st.vals.get("needs").and_then(Value::as_array).map(|x| x.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_default();
    if !needs.is_empty() && !st.later {
        let n = needs.len();
        out = out.push(sec_banner(
            &p,
            sec_dot(p.warn),
            format!("{n} {} need{} you", if n == 1 { "thing" } else { "things" }, if n == 1 { "s" } else { "" }),
            needs.join(" · "),
            vec![btn(&p, "Later", Kind::Normal, false, Some(Msg::Later)), btn(&p, "Open the Store", Kind::Primary, false, Some(Msg::Store))],
        ));
    }
    // Four tiles.
    let small = |a: String, b: String| [a, b].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
    let tiles = row![
        container(stile(&p, "Storage", v(st, "storage_free"), small(v(st, "storage_total"), v(st, "games_size")), Some(Msg::Go("storage")))).width(Length::FillPortion(1)),
        container(stile(&p, "Your AI", v(st, "ai_state"), v(st, "ai_model"), Some(Msg::Go("sec-ai")))).width(Length::FillPortion(1)),
        container(stile(&p, "Home server", v(st, "server_count"), v(st, "server_names"), Some(Msg::Go("sec-hs")))).width(Length::FillPortion(1)),
        container(stile(&p, "Hearth Link", v(st, "link_count"), v(st, "link_names"), Some(Msg::Go("sec-link")))).width(Length::FillPortion(1)),
    ]
    .spacing(18.0);
    out = out.push(tiles);

    // Quick: the switches and sliders right here.
    let wifi_on = v(st, "wifi_on") == "on";
    let bt_on = v(st, "bt_on") == "on";
    let night = sh.d_bool("nightLight", false);
    let mut quick: Vec<Element<Msg>> = Vec::new();
    if st.vals.contains_key("wifi_on") {
        quick.push(vrow(&p, "Wi-Fi", &v(st, "wifi_name"), switch(&p, wifi_on, Some(Msg::Sw("wifi_on", "wifi", !wifi_on))), None, false));
    }
    if st.vals.contains_key("bt_on") {
        quick.push(vrow(&p, "Bluetooth", &if bt_on { v(st, "bt_connected") } else { String::new() }, switch(&p, bt_on, Some(Msg::Sw("bt_on", "bt", !bt_on))), None, false));
    }
    quick.push(vrow(&p, "Night light", &night_line(sh), switch(&p, night, Some(Msg::Night(!night))), None, false));
    quick.push(vrow(&p, "Do not disturb", "", switch(&p, st.dnd, Some(Msg::Dnd(!st.dnd))), None, false));
    if let Some(s) = &st.sound {
        let vol = s.vol.min(1.0);
        let sl: Element<Msg> = container(iced::widget::slider(0.0..=1.0f32, vol, Msg::Vol).step(0.01f32).on_release(Msg::VolSet)).width(Length::Fixed(190.0)).into();
        quick.push(vrow(&p, "Volume", "", sl, None, st.bright.is_none()));
    }
    if let Some(b) = st.bright {
        let sl: Element<Msg> = container(iced::widget::slider(1..=100u32, b, Msg::Bright).on_release(Msg::BrightSet)).width(Length::Fixed(190.0)).into();
        quick.push(vrow(&p, "Brightness", "", sl, None, true));
    }

    // You use these most: the five pages opened most here, each with its value.
    let mut most: Vec<Element<Msg>> = Vec::new();
    let ids: Vec<String> = st.vals.get("most_used").and_then(Value::as_array).map(|x| x.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_default();
    let shown: Vec<(&'static str, &'static str, String)> = ids
        .iter()
        .filter_map(|id| crate::registry::page(id).map(|pg| (pg.id, crate::cats::sections::row_of(pg.id, &st.vals).unwrap_or((pg.label, String::new())))))
        .map(|(id, (label, val))| (id, label, val))
        .take(5)
        .collect();
    let n = shown.len();
    for (i, (id, label, val)) in shown.into_iter().enumerate() {
        most.push(vrow(&p, label, &val, value_chev(&p, ""), Some(Msg::Go(id)), i + 1 == n));
    }
    if most.is_empty() {
        most.push(container(t("The pages you open most show here, with their values.", sz(13.0), p.muted)).padding(6.0).into());
    }

    // This PC.
    let host = if sh.info.host.is_empty() { v(st, "host") } else { sh.info.host.clone() };
    let mut pc: Vec<Element<Msg>> = Vec::new();
    match &st.rename {
        Some(n) => pc.push(container(rename_box(&p, n, Msg::RenameText, Msg::RenameSave, Msg::RenameCancel)).padding(6.0).into()),
        None => pc.push(vrow(&p, "Name", &host, btn(&p, "Rename", Kind::Normal, false, Some(Msg::Rename)), None, false)),
    }
    let os = small(v(st, "hearth_short"), v(st, "base"));
    pc.push(vrow(&p, "Hearth OS", &os, value_chev(&p, ""), Some(Msg::Go("about")), false));
    if !v(st, "graphics").is_empty() {
        pc.push(vrow(&p, "Graphics", &v(st, "graphics"), value_chev(&p, ""), Some(Msg::Go("about")), false));
    }
    if !v(st, "memory_use").is_empty() {
        pc.push(vrow(&p, "Memory", &v(st, "memory_use"), value_chev(&p, ""), Some(Msg::Go("memory")), false));
    }
    let send_line = if st.sent.is_empty() { "to Dragongamer9 in one press".to_string() } else { st.sent.clone() };
    pc.push(vrow(&p, "Send logs", &send_line, btn(&p, "Send", Kind::Normal, false, if st.sent == "Sending…" { None } else { Some(Msg::SendLogs) }), None, true));

    let cards = row![
        container(scard(&p, "Quick", quick)).width(Length::FillPortion(1)),
        container(scard(&p, "You use these most", most)).width(Length::FillPortion(1)),
        container(scard(&p, "This PC", pc)).width(Length::FillPortion(1)),
    ]
    .spacing(18.0);
    out.push(cards).into()
}

/// settings-changes.js ago(): "just now", "2 min ago", "3 hours ago", "yesterday", "Nov 12".
pub fn ago_words(at: i64) -> String {
    let now = hearth_ui::time::now();
    let d = (now - at).max(0);
    if d < 60 {
        return "just now".into();
    }
    if d < 3600 {
        return format!("{} min ago", d / 60);
    }
    let a = hearth_ui::time::local(at);
    let n = hearth_ui::time::local(now);
    let y = hearth_ui::time::local(now - 86_400);
    if a.year == n.year && a.mon == n.mon && a.day == n.day {
        return format!("{} {}", d / 3600, if d < 7200 { "hour ago" } else { "hours ago" });
    }
    if a.year == y.year && a.mon == y.mon && a.day == y.day {
        return "yesterday".into();
    }
    hearth_ui::time::day(at)
}

/// settings-changes.js value(): a value in words.
fn value_words(key: &str, v: &Value) -> String {
    match v {
        Value::Null => if matches!(key, "wallpaper" | "accent" | "subTheme" | "barStyle" | "dockMode" | "deskClock" | "panelGlass") { "The look's own".into() } else if matches!(key, "appMenu" | "menuStyle" | "barLayout" | "barEdge" | "dockFloat") { "Default".into() } else { "Off".into() },
        Value::String(s) if s.is_empty() => value_words(key, &Value::Null),
        Value::Bool(b) => if *b { "On".into() } else { "Off".into() },
        Value::Number(n) if key == "screenOffMinutes" || key == "sleepMinutes" => {
            let m = n.as_f64().unwrap_or(0.0) as i64;
            if m == 0 { "Never".into() } else if m >= 60 && m % 60 == 0 { format!("{} {}", m / 60, if m == 60 { "hour" } else { "hours" }) } else { format!("{m} min") }
        }
        Value::Number(n) if key == "pointerSpeed" => format!("{}%", (n.as_f64().unwrap_or(0.0) * 100.0).round()),
        Value::Number(n) if key == "nightTemp" => format!("{} K", n),
        Value::String(s) if key == "wallpaper" => {
            if let Some((_, rest)) = s.split_once(':').filter(|_| s.starts_with("builtin:") || s.starts_with("anim:")) {
                let r = rest.replace(['-', '_'], " ");
                let mut c = r.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
            } else {
                s.rsplit('/').next().unwrap_or("").to_string()
            }
        }
        Value::String(s) => {
            let mut c = s.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        }
        other => kit::vs(other),
    }
}

/// settings-changes.js NAMES, with the keys data.rs doesn't name yet.
fn name_of(key: &str) -> String {
    match key {
        "lightMode" => "Light mode while you play",
        "countMe" => "Count me in the player count",
        "deskClock" => "Desktop clock",
        "panelGlass" => "Panel transparency",
        "sunsetLight" => "Light from",
        "sunsetDark" => "Dark from",
        "barLayout" => "Bar layout",
        "barEdge" => "Main bar's edge",
        "dockFloat" => "Floating dock",
        "shareStatus" => "Share this device's status",
        "menuStyle" => "Right-click menu",
        "wallMatch" => "Match sub theme",
        "nightTemp" => "Night light warmth",
        "nightMode" => "Night light on",
        "nightFrom" => "Night light from",
        "nightTo" => "Night light until",
        "nightPlace" => "Night light's place",
        "nightFade" => "Night light's fade",
        "tiles" => "Quick's tiles",
        other => return data::setting_name(other),
    }
    .to_string()
}

/// The changes worth showing (settings-changes.js HIDDEN: keys that change with another one).
fn shown_changes(h: &Value) -> Vec<&Value> {
    let hidden = ["accentFrom", "wallSwatches", "menuTiles", "nightLat", "nightLon"];
    kit::arr(h, "").iter().filter(|c| c.is_object() && !hidden.contains(&kit::s(c, "key").as_str())).collect()
}

fn changes_view<'a>(a: &'a App) -> Element<'a, Msg> {
    let p = a.sh.p;
    let items = shown_changes(&a.home.history);
    let mut c = Column::new().spacing(6.0).push(kit::words(&p, if items.is_empty() { "Nothing changed yet. What you change in Settings shows here, with Undo." } else { "What was changed in Settings, newest first. Undo puts the old value back." }));
    for ch in items {
        let key = kit::s(ch, "key");
        let title = format!("{} → {}", name_of(&key), value_words(&key, kit::at(ch, "to")));
        let who = if kit::s(ch, "who").is_empty() { "you".to_string() } else { kit::s(ch, "who") };
        let sub = format!("{who} · {}  (was {})", ago_words(kit::n(ch, "at") as i64), value_words(&key, kit::at(ch, "from")));
        c = c.push(kit::r("", title).sub(sub).control(btn(&p, "Undo", Kind::Normal, false, Some(Msg::Undo(kit::n(ch, "id") as i64)))).el(&p));
    }
    c.into()
}

// ------------------------------------------------------------------ Quick
fn shown(st: &State, id: &str) -> bool {
    let pinned = st.pinned.as_ref().map(|v| v.iter().any(|x| x == id)).unwrap_or(true);
    pinned && (id != "lightmode" || light_mode(st).is_some()) && (id != "brightness" || st.bright.is_some())
}

fn big_switch<'a>(p: &Pal, on: bool) -> Element<'a, Msg> {
    let p = *p;
    let knob = container(Space::new()).width(Length::Fixed(22.0)).height(Length::Fixed(22.0)).style(ui::boxed(if on { p.ink } else { p.muted }, 11.0, None));
    let inner: Element<Msg> = if on { row![Space::new().width(Length::Fill), knob].into() } else { row![knob, Space::new().width(Length::Fill)].into() };
    container(inner).padding(4.0).width(Length::Fixed(54.0)).height(Length::Fixed(30.0)).style(ui::boxed(if on { p.accent } else { p.panel3 }, 15.0, None)).into()
}

fn onoff_chip<'a>(p: &Pal, on: bool) -> Element<'a, Msg> {
    let c = if on { p.accent } else { p.muted };
    container(t(if on { "On" } else { "Off" }, 13.0, c)).padding(Padding { top: 2.0, right: 10.0, bottom: 2.0, left: 10.0 }).style(ui::boxed(Color::TRANSPARENT, 12.0, Some((c, 1.2)))).into()
}

/// One big tile (settings-B1): its icon, name and state; a switch, an On/Off badge or a slider.
fn qtile<'a>(p: &Pal, ic: kit::I, title: &str, sub: String, corner: Option<Element<'a, Msg>>, bottom: Option<Element<'a, Msg>>, wide: bool, on: Option<Msg>) -> Element<'a, Msg> {
    let p = *p;
    let mut head = Row::new().align_y(iced::alignment::Vertical::Top).push(kit::icon(ic, p.accent, 44.0));
    let mut col = Column::new();
    if wide {
        // settings-B1's wide tile: the name beside the icon, On / Off under it on the left, the switch on the right.
        head = head.push(container(column![ts(title.to_string(), 21.0, p.text), t(sub.clone(), 15.0, p.muted)].spacing(4.0)).padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 18.0 }).width(Length::Fill));
        col = col.push(head).push(Space::new().height(Length::Fill));
        let mut foot = Row::new().align_y(iced::Center);
        if let Some(b) = bottom {
            foot = foot.push(b);
        }
        foot = foot.push(Space::new().width(Length::Fill));
        if let Some(c) = corner {
            foot = foot.push(c);
        }
        col = col.push(foot);
    } else {
        head = head.push(Space::new().width(Length::Fill));
        if let Some(c) = corner {
            head = head.push(c);
        }
        col = col.push(head).push(Space::new().height(Length::Fill)).push(column![ts(title.to_string(), 21.0, p.text), t(sub, 15.0, p.muted)].spacing(4.0));
        if let Some(b) = bottom {
            col = col.push(container(b).padding(Padding { top: 10.0, right: 0.0, bottom: 0.0, left: 0.0 }));
        }
    }
    let c = container(col).padding(24.0).width(Length::FillPortion(if wide { 2 } else { 1 })).height(Length::Fixed(176.0)).style(ui::boxed(p.panel, p.r(14.0), None));
    match on {
        Some(m) => mouse_area(c).on_press(m).interaction(mouse::Interaction::Pointer).into(),
        None => c.into(),
    }
}

fn quick_view<'a>(a: &'a App) -> Element<'a, Msg> {
    let p = a.sh.p;
    let st = &a.home;
    let mut out = Column::new().spacing(18.0);
    out = out.push(row![kit::words(&p, "Big tiles for the settings you change most. Pin to Quick picks which are here."), pill(&p, if st.picking { "Done" } else { "Pin to Quick" }, true, Some(Msg::Picking))].align_y(iced::Center));
    if st.picking {
        let mut c = Column::new().spacing(6.0).push(ts("Pin to Quick: pick what's on this page", 17.0, p.text));
        for id in TILES {
            let on = st.pinned.as_ref().map(|v| v.iter().any(|x| x == id)).unwrap_or(true);
            c = c.push(kit::r("", tile_name(id)).switch(&p, on, Some(Msg::Pin(id))).el(&p));
        }
        return out.push(c).into();
    }
    let mut tiles: Vec<(u16, Element<Msg>)> = Vec::new();
    if shown(st, "lightmode") {
        let on = light_mode(st).unwrap_or(false);
        tiles.push((2, qtile(&p, kit::I::G(G::Pad), "Light mode while you play", "Big home server jobs wait until your game ends".into(), Some(big_switch(&p, on)), Some(onoff_chip(&p, on)), true, Some(Msg::Light(!on)))));
    }
    if shown(st, "wifi") {
        let on = st.wifi.as_ref().map(|w| w.on || !w.wired.is_empty()).unwrap_or(false);
        let sub = if on { wifi_text(st) } else { "Off".into() };
        tiles.push((1, qtile(&p, kit::I::G(G::Wifi), "Wi-Fi", sub, Some(onoff_chip(&p, on)), None, false, Some(Msg::Go("wifi")))));
    }
    if shown(st, "bt") {
        let (on, sub) = match &st.bt {
            None => (false, "…".to_string()),
            Some(b) if !b.on => (false, "Off".into()),
            Some(b) => {
                let names: Vec<String> = b.devs.iter().filter(|d| d.connected).map(|d| d.name.clone()).collect();
                (true, if names.is_empty() { "On · nothing connected".into() } else { format!("{}{}", names.iter().take(2).cloned().collect::<Vec<_>>().join(", "), if names.len() > 2 { format!(" +{}", names.len() - 2) } else { String::new() }) })
            }
        };
        tiles.push((1, qtile(&p, kit::I::Headphones, "Bluetooth", sub, Some(onoff_chip(&p, on)), None, false, Some(Msg::Go("bluetooth")))));
    }
    if shown(st, "brightness") {
        let v = st.bright.unwrap_or(0);
        tiles.push((1, qtile(&p, kit::I::Sun, "Brightness", format!("{v}%"), None, Some(iced::widget::slider(1..=100u32, v, Msg::Bright).on_release(Msg::BrightSet).into()), false, None)));
    }
    if shown(st, "volume") {
        let (v, name) = match &st.sound {
            Some(s) => (s.vol.min(1.0), s.outs.iter().find(|x| x.name == s.out).map(|x| x.desc.clone()).unwrap_or_else(|| "Sound".into())),
            None => (0.0, "Sound".into()),
        };
        tiles.push((1, qtile(&p, kit::I::G(G::Sound), "Volume", format!("{name} · {}%", (v * 100.0).round()), None, Some(iced::widget::slider(0.0..=1.0f32, v, Msg::Vol).step(0.01f32).on_release(Msg::VolSet).into()), false, None)));
    }
    if shown(st, "startin") {
        let c = container(column![
            row![ts("Start in", 20.0, p.text), Space::new().width(Length::Fill), t("What this PC opens when it turns on", 14.0, p.muted)],
            Space::new().height(Length::Fill),
            seg(&p, start_in_items(&a.sh, st))
        ])
        .padding(24.0)
        .width(Length::FillPortion(2))
        .height(Length::Fixed(176.0))
        .style(ui::boxed(p.panel, p.r(14.0), None));
        tiles.push((2, c.into()));
    }
    if shown(st, "updates") {
        let sub = match &st.updates {
            None => "Checking…".to_string(),
            Some(u) if !u.error.is_empty() => "Open to check".into(),
            Some(u) if !u.staged.is_empty() => "Restart to finish".into(),
            Some(u) if !u.available.is_empty() => format!("Hearth {} is ready", data::shown_version(&u.available)),
            Some(u) => format!("Up to date · Hearth OS {}", data::shown_version(&u.booted)),
        };
        tiles.push((1, qtile(&p, kit::I::G(G::Update), "Updates", sub, None, None, false, Some(Msg::Go("updates")))));
    }
    if shown(st, "power") {
        let sleep = a.sh.d_num("sleepMinutes", 0.0) as i64;
        let sleep_t = if sleep == 0 { "sleeps never".to_string() } else if sleep >= 60 { format!("sleeps after {} h", sleep / 60) } else { format!("sleeps after {sleep} min") };
        let prof = if st.profile.is_empty() { "Power".to_string() } else { data::profile_name(&st.profile).to_string() };
        tiles.push((1, qtile(&p, kit::I::Batt, "Power", format!("{prof} · {sleep_t}"), None, None, false, Some(Msg::Go("power")))));
    }
    if shown(st, "night") {
        tiles.push((1, qtile(&p, kit::I::G(G::Moon), "Night light", if st.night { "On".into() } else { "Off".into() }, Some(big_switch(&p, st.night)), None, false, Some(Msg::Night(!st.night)))));
    }
    if shown(st, "dnd") {
        tiles.push((1, qtile(&p, kit::I::BellOff, "Do not disturb", if st.dnd { "On".into() } else { "Off".into() }, Some(big_switch(&p, st.dnd)), None, false, Some(Msg::Dnd(!st.dnd)))));
    }
    // Four columns; a wide tile takes two.
    let mut grid = Column::new().spacing(20.0);
    let mut rowv: Vec<Element<Msg>> = Vec::new();
    let mut used = 0u16;
    for (w, e) in tiles {
        if used + w > 4 {
            let mut r = Row::new().spacing(20.0);
            for x in rowv.drain(..) {
                r = r.push(x);
            }
            if used < 4 {
                r = r.push(Space::new().width(Length::FillPortion(4 - used)));
            }
            grid = grid.push(r);
            used = 0;
        }
        rowv.push(e);
        used += w;
    }
    if !rowv.is_empty() {
        let mut r = Row::new().spacing(20.0);
        for x in rowv.drain(..) {
            r = r.push(x);
        }
        if used < 4 {
            r = r.push(Space::new().width(Length::FillPortion(4 - used)));
        }
        grid = grid.push(r);
    }
    out.push(grid).into()
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    match page {
        "home" => Some(home_view(a)),
        "quick" => Some(quick_view(a)),
        "changes" => Some(changes_view(a)),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plasma_dnd_and_words() {
        let now = hearth_ui::time::Tm { year: 2026, mon: 10, day: 18, hour: 21, min: 0, sec: 0, wday: 3 };
        assert!(plasma_until_ahead("2027,11,18,21,0,0", now));
        assert!(!plasma_until_ahead("2026,11,18,20,0,0", now));
        assert!(!plasma_until_ahead("", now));
        assert_eq!(value_words("sleepMinutes", &json!(0)), "Never");
        assert_eq!(value_words("nightLight", &json!(true)), "On");
        assert_eq!(value_words("wallpaper", &json!("builtin:ember-glow")), "Ember glow");
    }
}

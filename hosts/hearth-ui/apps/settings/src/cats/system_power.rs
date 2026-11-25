//! System › Power & battery and its pages (Mantel's SetPower.qml, SetPowerProfiles.qml, SetPowerAwake.qml,
//! SetBattery.qml; Info Center › Energy is the battery page too). The same helpers as Mantel: hearth-power (power
//! mode and power limit), hearth-hwset (the battery's charge limit, the power mode plugged in and on battery:
//! /etc/hearth/hardware.json), hearth-powerset (the lid, apps keeping it awake, hibernate, the battery's health and
//! history) and hearth-admin lid (logind). The screen-off and sleep times are desktop.json's (screenOffMinutes,
//! sleepMinutes, screenOffBattery, sleepBattery) and the power button buttons.json's (press, hold).
//!
//! On KDE Plasma it's PowerDevil that turns the screen off, puts the PC to sleep, answers the power button and the
//! lid, so each of those is also written to PowerDevil's own settings (powerdevilrc's [AC] and [Battery] profiles,
//! kwriteconfig6) and PowerDevil is asked to read them again (reparseConfiguration). What the page shows on Plasma is
//! PowerDevil's, the one that acts.

use super::*;
use iced::widget::{column, row};
use hearth_ui::ui::{self, glyph, ts, Kind};
use crate::widgets::btn;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, Path, Stroke};
use iced::widget::container;
use iced::{mouse, Color, Point, Rectangle, Renderer, Theme};
use serde_json::json;

/// What PowerDevil has (one read: NAME=value lines).
pub fn kde_read() -> String {
    let mut s = String::new();
    for (pre, prof) in [("ac", "AC"), ("bat", "Battery")] {
        s += &kr(&format!("{pre}_offon"), "powerdevilrc", &[prof, "Display"], "TurnOffDisplayWhenIdle");
        s += &kr(&format!("{pre}_off"), "powerdevilrc", &[prof, "Display"], "TurnOffDisplayIdleTimeoutSec");
        s += &kr(&format!("{pre}_sleepact"), "powerdevilrc", &[prof, "SuspendAndShutdown"], "AutoSuspendAction");
        s += &kr(&format!("{pre}_sleep"), "powerdevilrc", &[prof, "SuspendAndShutdown"], "AutoSuspendIdleTimeoutSec");
        s += &kr(&format!("{pre}_lid"), "powerdevilrc", &[prof, "SuspendAndShutdown"], "LidAction");
        s += &kr(&format!("{pre}_button"), "powerdevilrc", &[prof, "SuspendAndShutdown"], "PowerButtonAction");
        s += &kr(&format!("{pre}_dock"), "powerdevilrc", &[prof, "SuspendAndShutdown"], "InhibitLidActionWhenExternalMonitorPresent");
        s += &kr(&format!("{pre}_prof"), "powerdevilrc", &[prof, "Performance"], "PowerProfile");
    }
    s
}

fn reparse() -> Vec<String> {
    dbus("org.kde.Solid.PowerManagement", "/org/kde/Solid/PowerManagement", "org.kde.Solid.PowerManagement.reparseConfiguration", &[])
}

fn kd(st: &State, k: &str) -> String {
    vs(st.j("k-power"), k)
}

/// The minutes in use: PowerDevil's on Plasma (when it has its own), else Hearth's (desktop.json).
fn screen_off(a: &App, battery: bool) -> i64 {
    let st = &a.system;
    let pre = if battery { "bat" } else { "ac" };
    if plasma() && st.has("k-power") && !kd(st, &format!("{pre}_off")).is_empty() {
        if kd(st, &format!("{pre}_offon")) == "false" {
            return 0;
        }
        return (vf(st.j("k-power"), &format!("{pre}_off"), 0.0) / 60.0).round() as i64;
    }
    a.sh.d_num(if battery { "screenOffBattery" } else { "screenOffMinutes" }, if battery { -1.0 } else { 10.0 }) as i64
}
fn sleep_after(a: &App, battery: bool) -> i64 {
    let st = &a.system;
    let pre = if battery { "bat" } else { "ac" };
    if plasma() && st.has("k-power") && !kd(st, &format!("{pre}_sleep")).is_empty() {
        if kd(st, &format!("{pre}_sleepact")) == "0" {
            return 0;
        }
        return (vf(st.j("k-power"), &format!("{pre}_sleep"), 0.0) / 60.0).round() as i64;
    }
    a.sh.d_num(if battery { "sleepBattery" } else { "sleepMinutes" }, if battery { -1.0 } else { 30.0 }) as i64
}

/// Saving a time: Hearth's key, and on Plasma PowerDevil's for that profile (on battery "As plugged in" copies the
/// plugged-in time into the battery profile).
fn set_time(a: &App, key: &'static str, m: i64) -> Msg {
    let battery = key.ends_with("Battery");
    let off = key.starts_with("screenOff");
    let mut ex = Ex::none().re("k-power").fail("Couldn't save that");
    if plasma() {
        let prof = if battery { "Battery" } else { "AC" };
        let real = if m < 0 { if off { screen_off(a, false) } else { sleep_after(a, false) } } else { m };
        if off {
            ex = ex.and(kw("powerdevilrc", &[prof, "Display"], "TurnOffDisplayWhenIdle", if real == 0 { "false" } else { "true" }));
            if real > 0 {
                ex = ex.and(kw("powerdevilrc", &[prof, "Display"], "TurnOffDisplayIdleTimeoutSec", &(real * 60).to_string()));
            }
        } else {
            ex = ex.and(kw("powerdevilrc", &[prof, "SuspendAndShutdown"], "AutoSuspendAction", if real == 0 { "0" } else { "1" }));
            if real > 0 {
                ex = ex.and(kw("powerdevilrc", &[prof, "SuspendAndShutdown"], "AutoSuspendIdleTimeoutSec", &(real * 60).to_string()));
            }
        }
        ex = ex.and(reparse());
    }
    let mut v = serde_json::Map::new();
    v.insert(key.to_string(), json!(m));
    Msg::Desk(Value::Object(v), ex)
}

fn button_action(v: &str) -> &'static str {
    // PowerDevil's PowerButtonAction: nothing 0, sleep 1, shut down 8, the leave screen 16.
    match v {
        "nothing" => "0",
        "shutdown" => "8",
        "menu" => "16",
        _ => "1",
    }
}
fn lid_action(v: &str) -> &'static str {
    match v {
        "ignore" => "0",
        "hibernate" => "2",
        "poweroff" => "8",
        "lock" => "32",
        _ => "1",
    }
}

pub fn history(st: &State) -> Task<Msg> {
    let Some(b) = va(st.j("pset"), "batteries").first() else { return Task::none() };
    let path = vs(b, "path");
    let hours = (st.num.get("hours").copied().unwrap_or(24.0) as i64).to_string();
    Task::batch([
        read_cmd("hist-charge", py_s("hearth-powerset", vec!["history".into(), path.clone(), "charge".into(), hours.clone()])),
        read_cmd("hist-rate", py_s("hearth-powerset", vec!["history".into(), path, "rate".into(), hours])),
    ])
}

pub fn slider_save(_sh: &mut Shared, st: &mut State, k: &'static str) -> Task<Msg> {
    if k == "tdp" {
        let w = st.num.remove("tdp").unwrap_or(0.0).round() as i64;
        return exec(Ex::new(py_s("hearth-power", vec!["tdp".into(), w.to_string()])).store("power").fail("Couldn't change the power limit"));
    }
    Task::none()
}

pub fn view<'a>(a: &'a App, page: &'static str) -> El<'a> {
    match page {
        "power" => power(a),
        "power-profiles" => profiles(a),
        "power-awake" => awake(a),
        _ => battery(a),
    }
}

const PROFILE_NAMES: [(&str, &str); 3] = [("power-saver", "Battery saver"), ("balanced", "Balanced"), ("performance", "Performance")];
const OFF_CHOICES: [i64; 6] = [0, 2, 5, 10, 15, 30];
const SLEEP_CHOICES: [i64; 6] = [0, 10, 15, 30, 60, 120];

fn bat_line(b: &Value) -> String {
    let pctv = vf(b, "percentage", -1.0);
    if pctv < 0.0 {
        return String::new();
    }
    let state = vs(b, "state");
    format!("{}% · {}", pctv.round(), match state.as_str() {
        "charging" => "charging".to_string(),
        "fully-charged" | "full" => "full".to_string(),
        "discharging" => "on battery".to_string(),
        s if s.is_empty() => "on battery".to_string(),
        s => s.replace('-', " "),
    })
}

fn power(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let mut c = crate::widgets::Secs::on(a, 0).spacing(10.0);
    let pw = st.j("power");
    let pset = st.j("pset");
    let bats = va(pset, "batteries");
    let hwb = va(st.j("hw"), "batteries").first().cloned().unwrap_or(Value::Null);
    let profiles: Vec<String> = if vb(pw, "mode.supported") { va(pw, "mode.available").iter().map(|x| x.as_str().unwrap_or("").to_string()).collect() } else { vec![] };
    let active = vs(pw, "mode.active");
    c = c.push(head(&p, "Battery & performance"));
    if let Some(b) = bats.first() {
        c = c.push(srow(&p, G::Power, "Battery", bat_line(b), bar_of(&p, vf(b, "percentage", 0.0), 100.0)));
    }
    if vb(&hwb, "canLimit") {
        let lim = vf(&hwb, "limit", 100.0) as i64;
        let now = if lim >= 95 { 100 } else if lim <= 70 { 60 } else { 80 };
        let health = if present(&hwb, "health") { format!("Battery health {}%{}. ", vf(&hwb, "health", 0.0).round(), if present(&hwb, "cycles") { format!(" after {} charges", vf(&hwb, "cycles", 0.0)) } else { String::new() }) } else { String::new() };
        let set = |v: i64| Msg::Exec(Ex::new(adm("hearth-hwset", vec!["set".into(), "battery.limit".into(), v.to_string()])).re("hw").fail("Couldn't set the charge limit"));
        c = c.push(srow(&p, G::Power, "Stop charging at", format!("{health}A limit keeps the battery healthy longer if it's plugged in a lot"), choice(&p, vec![("60%".into(), now == 60, set(60)), ("80%".into(), now == 80, set(80)), ("100% (no limit)".into(), now == 100, set(100))])));
    }
    if profiles.len() > 1 {
        let opts: Vec<(String, bool, Msg)> = PROFILE_NAMES
            .iter()
            .filter(|x| profiles.iter().any(|y| y == x.0))
            .map(|(id, l)| (l.to_string(), active == *id, Msg::Exec(Ex::new(py("hearth-power", &["mode", *id])).store("power").fail("Couldn't change the power mode"))))
            .collect();
        c = c.push(srow(&p, G::Power, "Power mode", "Battery saver lasts longer; Performance is faster and warmer", choice(&p, opts)));
    } else if st.read("power") {
        c = c.push(info_row(&p, G::Power, "Power mode", "This PC doesn't offer power modes"));
    }
    if vb(pw, "tdp.supported") {
        let lo = vf(pw, "tdp.min", 5.0) as f32;
        let hi = (vf(pw, "tdp.max", 30.0) as f32).max(lo + 1.0);
        let w = st.num.get("tdp").copied().unwrap_or(vf(pw, "tdp.watts", lo as f64) as f32);
        c = c.push(srow(&p, G::Power, "Power limit (TDP)", format!("{} W", w.round()), slider_f32(w.clamp(lo, hi), lo..=hi, 1.0, |v| Msg::Num("tdp", v), Msg::NumDone("tdp"))));
    }
    c = c.push(head(&p, "More power settings"));
    c = c.push(link_row(&p, G::Power, "Plugged in, battery & lid", "Different times and power mode on battery, and what closing the lid does", Msg::Go("power-profiles")));
    let awake = va(pset, "inhibitors").iter().filter(|i| vs(i, "mode") == "block").count();
    c = c.push(link_row(&p, G::Moon, "Apps keeping it awake", if awake > 0 { format!("{awake} {} keeping the screen on or the PC awake", if awake == 1 { "app is" } else { "apps are" }) } else { "Nothing is keeping it awake right now".to_string() }, Msg::Go("power-awake")));
    c = c.push(link_row(&p, G::Power, "Battery health & history", "Charge cycles, how much it holds, graphs of charge and power, your devices' batteries", Msg::Go("battery")));

    c = c.push(head(&p, "When you're away"));
    let can_off = plasma() || st.raw("canoff").trim() == "yes";
    let off = screen_off(a, false);
    let offs: Vec<(String, bool, Option<Msg>)> = OFF_CHOICES.iter().map(|m| (minutes_word(*m), off == *m, if can_off { Some(set_time(a, "screenOffMinutes", *m)) } else { None })).collect();
    c = c.push(srow(&p, G::Display, "Turn the screen off after", if can_off { "Videos and games that are playing keep it on" } else { "Needs kscreen-doctor, which isn't installed" }, seg(&p, offs)));
    let sl = sleep_after(a, false);
    c = c.push(srow(&p, G::Moon, "Go to sleep after", "Downloads stop while it sleeps. Videos and games that are playing keep it awake", choice(&p, SLEEP_CHOICES.iter().map(|m| (minutes_word(*m), sl == *m, set_time(a, "sleepMinutes", *m))).collect())));

    c = c.push(head(&p, "Power button"));
    let b = st.file("buttons.json");
    let press = { let x = vs(b, "press"); if x.is_empty() { "sleep".to_string() } else { x } };
    let hold = { let x = vs(b, "hold"); if x.is_empty() { "menu".to_string() } else { x } };
    let btn_set = |key: &'static str, v: &'static str| -> Msg {
        let mut ex = Ex::none().re("k-power");
        // PowerDevil answers the power button on Plasma: a quick press is its PowerButtonAction.
        if plasma() && key == "press" {
            ex = ex.and(kw("powerdevilrc", &["AC", "SuspendAndShutdown"], "PowerButtonAction", button_action(v))).and(kw("powerdevilrc", &["Battery", "SuspendAndShutdown"], "PowerButtonAction", button_action(v))).and(reparse());
        }
        let mut o = serde_json::Map::new();
        o.insert(key.to_string(), json!(v));
        Msg::Json("buttons.json", Value::Object(o), ex)
    };
    let choices = [("sleep", "Sleep"), ("menu", "Power menu"), ("shutdown", "Shut down"), ("nothing", "Nothing")];
    c = c.push(srow(&p, G::Power, "Press", "A quick press", choice(&p, choices.iter().map(|(v, l)| (l.to_string(), press == *v, btn_set("press", *v))).collect())));
    c = c.push(srow(&p, G::Power, "Hold", if plasma() { "Held down for a second (in Hearth's Game Mode and Mantel; Plasma has only a press)" } else { "Held down for a second" }, choice(&p, choices.iter().map(|(v, l)| (l.to_string(), hold == *v, btn_set("hold", *v))).collect())));
    c = c.push(line(&p, "Holding it for several seconds always turns the device off, whatever is set here."));

    c = c.push(head(&p, "Turn off or switch"));
    c = c.push(link_row(&p, G::Pad, "Game Mode", "The Game Mode you picked in Gaming (Steam's or Hearth's) on the full screen", Msg::App(sv(&["nimbus-mode", "game"]))));
    c = c.push(act_row(&p, G::Moon, "Sleep", "", "Sleep", Some(Msg::App(sv(&["systemctl", "suspend"])))));
    let hib = at(pset, "hibernate");
    let hyb = at(pset, "hybrid");
    let proven = vb(hib, "proven");
    if vb(hib, "can") {
        c = c.push(act_row(&p, G::Moon, "Hibernate", if proven { "Saves everything to the drive and turns off; turn on to carry on where you were" } else { "Not tried on this PC yet: save your work first" }, "Hibernate", Some(if proven { Msg::App(sv(&["systemctl", "hibernate"])) } else { Msg::Text("askhib", "hibernate".into()) })));
        if vb(hyb, "can") {
            c = c.push(act_row(&p, G::Moon, "Hybrid sleep", if proven { "Sleeps, and saves everything to the drive too in case the power goes" } else { "Not tried on this PC yet: save your work first" }, "Hybrid sleep", Some(if proven { Msg::App(sv(&["systemctl", "hybrid-sleep"])) } else { Msg::Text("askhib", "hybrid-sleep".into()) })));
        } else if present(hyb, "can") {
            c = c.push(info_row(&p, G::Moon, "Hybrid sleep isn't available", vs(hyb, "why")));
        }
    } else if present(hib, "can") {
        c = c.push(info_row(&p, G::Moon, "Hibernate isn't available", vs(hib, "why")));
    }
    let ask = st.txt("askhib");
    if !ask.is_empty() {
        let hy = ask == "hybrid-sleep";
        c = c.push(card(&p, column![
            ts(if hy { "Hybrid sleep for the first time?" } else { "Hibernate for the first time?" }, sz(15.0), p.warn),
            t("Hearth hasn't seen this PC come back from hibernating yet. Save your work: if it doesn't come back, it starts fresh and anything not saved is lost. Once it has worked, Hearth remembers.", sz(13.0), p.muted),
            row![btn(&p, if hy { "Hybrid sleep now" } else { "Hibernate now" }, Kind::Primary, false, Some(Msg::App(sv(&["systemctl", if hy { "hybrid-sleep" } else { "hibernate" }])))), btn(&p, "Cancel", Kind::Normal, false, Some(Msg::Text("askhib", String::new())))].spacing(8.0)
        ].spacing(8.0)));
    }
    c = c.push(act_row(&p, G::Update, "Restart", "", "Restart", Some(Msg::App(sv(&["systemctl", "reboot"])))));
    c = c.push(srow(&p, G::Power, "Shut down", "", btn(&p, "Shut down", Kind::Danger, false, Some(Msg::App(sv(&["systemctl", "poweroff"]))))));
    if !vs(pw, "error").is_empty() {
        c = c.push(note(&p, vs(pw, "error"), true));
    }
    c.into()
}

/// One of the two columns: plugged in, on battery.
fn column2<'a>(p: &Pal, g: G, title: &'a str, sub: &'a str, body: Column<'a, Msg>) -> El<'a> {
    let p = *p;
    container(column![
        row![
            container(glyph(g, p.accent, sz(18.0), 1.8)).center_x(Length::Fixed(34.0)).center_y(Length::Fixed(34.0)).style(ui::boxed(Pal::a(p.accent, 0.16), 17.0, None)),
            column![ts(title, sz(16.0), p.text), t1(sub, sz(12.0), p.muted)].spacing(0.0)
        ]
        .spacing(10.0)
        .align_y(iced::Center),
        body.spacing(12.0)
    ]
    .spacing(14.0))
    .padding(18.0)
    .width(Length::Fill)
    .style(ui::boxed(p.panel, p.r(12.0), Some((p.line, 1.0))))
    .into()
}

fn small_choice<'a>(p: &Pal, title: &'a str, items: Vec<(String, bool, Msg)>) -> El<'a> {
    column![t1(title, sz(14.0), p.text), choice(p, items)].spacing(6.0).into()
}

/// Plugged in, battery & lid: side by side what happens plugged in and on battery (screen off, sleep, power mode),
/// and what closing the lid does in each case.
fn profiles(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let pset = st.j("pset");
    let has_bat = !va(pset, "batteries").is_empty();
    let on_bat = va(pset, "batteries").first().map(|b| vs(b, "state") == "discharging").unwrap_or(false);
    let hw = st.j("hw");
    let saved = |k: &str| -> String { let v = kg(at(hw, "saved"), k); if v.is_empty() { "off".into() } else { v } };
    let mode_set = |key: &'static str, v: &'static str| -> Msg {
        let mut ex = Ex::new(adm("hearth-hwset", vec!["set".into(), key.into(), v.into()])).re("hw").fail("Couldn't save the power mode");
        if plasma() {
            let prof = if key == "power.plugged" { "AC" } else { "Battery" };
            ex = ex.and(if v == "off" { kdel("powerdevilrc", &[prof, "Performance"], "PowerProfile") } else { kw("powerdevilrc", &[prof, "Performance"], "PowerProfile", v) }).and(reparse()).re("k-power");
        }
        Msg::Exec(ex)
    };
    let modes = [("off", "Don't change"), ("performance", "Performance"), ("balanced", "Balanced"), ("power-saver", "Battery saver")];
    let mut c = crate::widgets::Secs::on(a, 0).spacing(10.0);
    let off_ac = screen_off(a, false);
    let sl_ac = sleep_after(a, false);
    let mut plug = Column::new();
    plug = plug.push(small_choice(&p, "Screen off after", OFF_CHOICES.iter().map(|m| (minutes_word(*m), off_ac == *m, set_time(a, "screenOffMinutes", *m))).collect()));
    plug = plug.push(small_choice(&p, "Sleep after", SLEEP_CHOICES.iter().map(|m| (minutes_word(*m), sl_ac == *m, set_time(a, "sleepMinutes", *m))).collect()));
    if has_bat && st.has("hw") {
        let cur = saved("power.plugged");
        plug = plug.push(small_choice(&p, "Power mode", modes.iter().map(|(v, l)| (l.to_string(), cur == *v, mode_set("power.plugged", *v))).collect()));
    }
    // One above the other (side by side the choices don't fit the window's width).
    let mut cols = Column::new().spacing(12.0).push(column2(&p, G::Power, "Plugged in", if on_bat { "" } else { "In use now" }, plug));
    if has_bat {
        let off_b = screen_off(a, true);
        let sl_b = sleep_after(a, true);
        let mut bat = Column::new();
        let with_as = |list: &[i64]| -> Vec<i64> { std::iter::once(-1).chain(list.iter().copied()).collect() };
        // "As plugged in" while Hearth's own says so and the battery's time (PowerDevil's on Plasma) is the same.
        let as_off = a.sh.d_num("screenOffBattery", -1.0) as i64 == -1 && off_b == off_ac;
        let as_sl = a.sh.d_num("sleepBattery", -1.0) as i64 == -1 && sl_b == sl_ac;
        let off_b = if as_off { -1 } else { off_b };
        let sl_b = if as_sl { -1 } else { sl_b };
        bat = bat.push(small_choice(&p, "Screen off after", with_as(&OFF_CHOICES).iter().map(|m| (minutes_word(*m), off_b == *m, set_time(a, "screenOffBattery", *m))).collect()));
        bat = bat.push(small_choice(&p, "Sleep after", with_as(&SLEEP_CHOICES).iter().map(|m| (minutes_word(*m), sl_b == *m, set_time(a, "sleepBattery", *m))).collect()));
        if st.has("hw") {
            let cur = saved("power.unplugged");
            bat = bat.push(small_choice(&p, "Power mode", modes.iter().map(|(v, l)| (l.to_string(), cur == *v, mode_set("power.unplugged", *v))).collect()));
        }
        cols = cols.push(column2(&p, G::Power, "On battery", if on_bat { "In use now" } else { "" }, bat));
    }
    c = c.push(cols);
    if !has_bat && st.read("pset") {
        c = c.push(info_row(&p, G::Info, "This PC has no battery", "The times above are the ones it uses"));
    }
    let lid = at(pset, "lid");
    if st.has("pset") && (vb(lid, "present") || has_bat) {
        c = c.push(head(&p, "When the lid is closed"));
        let can_hib = vb(pset, "hibernate.can");
        let mut opts: Vec<(String, String)> = vec![("suspend".into(), "Sleep".into())];
        if can_hib {
            opts.push(("hibernate".into(), "Hibernate".into()));
        }
        for (v, l) in [("lock", "Lock the screen"), ("poweroff", "Shut down"), ("ignore", "Nothing")] {
            opts.push((v.into(), l.into()));
        }
        let cur = |k: &str, d: &str| -> String { let v = vs(lid, k); if v.is_empty() { d.into() } else { v } };
        let (b, ac, dk) = (cur("battery", "suspend"), cur("ac", "suspend"), cur("docked", "ignore"));
        let set = move |which: &'static str, v: String| -> Msg {
            let (mut nb, mut na, mut nd) = (b.clone(), ac.clone(), dk.clone());
            match which {
                "battery" => nb = v,
                "ac" => na = v,
                _ => nd = v,
            }
            let mut ex = Ex::new(adm("hearth-admin", vec!["lid".into(), nb.clone(), na.clone(), nd.clone()])).ok("Saved.").re("pset").fail("Couldn't save what the lid does");
            // PowerDevil handles the lid while Plasma runs: its own LidAction, and "with another screen" as its
            // "do nothing when an external monitor is connected".
            if plasma() {
                ex = ex
                    .and(kw("powerdevilrc", &["Battery", "SuspendAndShutdown"], "LidAction", lid_action(&nb)))
                    .and(kw("powerdevilrc", &["AC", "SuspendAndShutdown"], "LidAction", lid_action(&na)))
                    .and(kw("powerdevilrc", &["AC", "SuspendAndShutdown"], "InhibitLidActionWhenExternalMonitorPresent", if nd == "ignore" { "true" } else { "false" }))
                    .and(kw("powerdevilrc", &["Battery", "SuspendAndShutdown"], "InhibitLidActionWhenExternalMonitorPresent", if nd == "ignore" { "true" } else { "false" }))
                    .and(reparse())
                    .re("k-power");
            }
            Msg::Exec(ex)
        };
        let s1 = set.clone();
        let s2 = set.clone();
        c = c.push(chooser(a, "lidbat", G::Power, "On battery", "", opts.clone(), &vs(lid, "battery"), move |v| s1("battery", v)));
        c = c.push(chooser(a, "lidac", G::Power, "Plugged in", "", opts.clone(), &vs(lid, "ac"), move |v| s2("ac", v)));
        c = c.push(chooser(a, "liddock", G::Display, "With another screen attached", "Docked, or an external monitor plugged in", opts, &cur("docked", "ignore"), move |v| set("docked", v)));
        let by = va(lid, "setBy");
        if !by.is_empty() {
            c = c.push(line(&p, format!("Also set in {} (Hearth's choice is read after it)", by.iter().map(|x| x.as_str().unwrap_or("")).collect::<Vec<_>>().join(", "))));
        }
    } else if let Some(w) = waiting(a, "pset", "the lid and battery") {
        c = c.push(w);
    }
    c.into()
}

/// Apps keeping it awake: logind's inhibitors (hearth-powerset), read again every few seconds while open.
fn awake(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let list = va(st.j("pset"), "inhibitors");
    let words = |i: &Value| -> String {
        let what: Vec<String> = va(i, "what").iter().map(|x| x.as_str().unwrap_or("").to_string()).collect();
        let delay = vs(i, "mode") == "delay";
        let mut w = Vec::new();
        if what.iter().any(|x| x == "idle") {
            w.push("keeps the screen on");
        }
        if what.iter().any(|x| x == "sleep") {
            w.push(if delay { "delays sleep a moment" } else { "keeps the PC awake" });
        }
        if what.iter().any(|x| x == "shutdown") {
            w.push(if delay { "delays shutting down" } else { "holds off shutting down" });
        }
        w.join(", ")
    };
    let blocking: Vec<&Value> = list.iter().filter(|i| vs(i, "mode") == "block").collect();
    let delaying: Vec<&Value> = list.iter().filter(|i| vs(i, "mode") != "block").collect();
    let mut c = crate::widgets::Secs::on(a, 0).spacing(10.0);
    if let Some(w) = waiting(a, "pset", "what's keeping it awake") {
        c = c.push(w);
    }
    c = c.push(head(&p, if blocking.is_empty() { "Nothing is keeping it awake" } else { "Keeping it awake now" }));
    if blocking.is_empty() && st.has("pset") {
        c = c.push(info_row(&p, G::Check, "The PC can sleep and the screen can turn off on time", "Videos, games and downloads that need it awake show up here while they do"));
    }
    for i in &blocking {
        let who = vs(i, "who");
        let app = vs(i, "app");
        let why = vs(i, "why");
        let title = if !app.is_empty() && app != who { format!("{who} ({app})") } else { who };
        let g = if va(i, "what").iter().any(|x| x == "idle") { G::Display } else { G::Moon };
        c = c.push(srow(&p, g, title, format!("{}{}", if why.is_empty() { String::new() } else { format!("{why} · ") }, words(i)), ui::chip("Blocking", Pal::a(p.warn, 0.2), p.warn)));
    }
    if !delaying.is_empty() {
        c = c.push(head(&p, "Asking for a moment first"));
        for i in &delaying {
            let why = vs(i, "why");
            c = c.push(info_row(&p, G::Clock, vs(i, "who"), format!("{}{}", if why.is_empty() { String::new() } else { format!("{why} · ") }, words(i))));
        }
    }
    c = c.push(head(&p, "Sleep anyway"));
    c = c.push(act_row(&p, G::Moon, "Sleep now", "Puts the PC to sleep even while apps ask to keep it awake", "Sleep", Some(Msg::App(sv(&["systemctl", "suspend", "--check-inhibitors=no"])))));
    c.into()
}

/// The battery's health as a ring.
struct Ring {
    frac: f32,
    color: Color,
    track: Color,
}

impl<M> canvas::Program<M> for Ring {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, b: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        let cx = b.width / 2.0;
        let cy = b.height / 2.0;
        let r = cx.min(cy) - 7.0;
        let arc = |from: f32, to: f32| {
            Path::new(|pb| {
                let n = 64;
                for i in 0..=n {
                    let a = from + (to - from) * i as f32 / n as f32;
                    let pt = Point::new(cx + r * a.cos(), cy + r * a.sin());
                    if i == 0 {
                        pb.move_to(pt);
                    } else {
                        pb.line_to(pt);
                    }
                }
            })
        };
        let tau = std::f32::consts::PI * 2.0;
        f.stroke(&arc(0.0, tau), Stroke::default().with_color(self.track).with_width(10.0));
        if self.frac > 0.0 {
            let start = -std::f32::consts::FRAC_PI_2;
            f.stroke(&arc(start, start + tau * self.frac.min(1.0)), Stroke::default().with_color(self.color).with_width(10.0).with_line_cap(LineCap::Round));
        }
        vec![f.into_geometry()]
    }
}

/// Battery health & history (Info Center › Energy too): health, cycles, the charge and power graphs from UPower's
/// history, and your other devices' batteries (UPower: mice, keyboards, controllers, headphones).
fn battery(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let mut c = crate::widgets::Secs::on(a, 0).spacing(10.0);
    if let Some(w) = waiting(a, "pset", "the battery") {
        c = c.push(w);
    }
    let bat = va(st.j("pset"), "batteries").first();
    if let Some(b) = bat {
        let h = vf(b, "health", 0.0) as f32;
        let col = if h >= 80.0 { p.accent } else if h >= 60.0 { p.warn } else { p.danger };
        let ring = iced::widget::stack![
            iced::widget::canvas(Ring { frac: h / 100.0, color: col, track: Pal::a(p.text, 0.1) }).width(Length::Fixed(104.0)).height(Length::Fixed(104.0)),
            container(column![ts(if h > 0.0 { format!("{}%", h.round()) } else { "—".into() }, sz(24.0), p.text), t1("health", sz(11.0), p.muted)].align_x(iced::Center)).center_x(Length::Fixed(104.0)).center_y(Length::Fixed(104.0))
        ];
        let fact = |k: &str, v: String| -> El<'static> { column![t1(k.to_string(), sz(12.0), p.muted), t1(v, sz(15.0), p.text)].spacing(1.0).width(Length::Fill).into() };
        let wh = |k: &str| if vf(b, k, 0.0) > 0.0 { format!("{:.1} Wh", vf(b, k, 0.0)) } else { "—".into() };
        let ident: Vec<String> = ["vendor", "model", "technology"].iter().map(|k| vs(b, k)).filter(|x| !x.is_empty()).collect();
        let grid = column![
            row![fact("Charge cycles", if present(b, "cycles") { vs(b, "cycles") } else { "Not reported".into() }), fact("Holds now", wh("energyFull")), fact("Held when new", wh("energyDesign"))].spacing(30.0),
            row![fact("Charge", bat_line(b)), fact("Using now", if vf(b, "energyRate", 0.0) > 0.0 { format!("{:.1} W", vf(b, "energyRate", 0.0)) } else { "—".into() }), fact("Battery", if ident.is_empty() { vs(b, "name") } else { ident.join(" · ") })].spacing(30.0)
        ]
        .spacing(10.0);
        c = c.push(card(&p, row![ring, grid].spacing(26.0).align_y(iced::Center)));
        let hours = st.num.get("hours").copied().unwrap_or(24.0) as i64;
        c = c.push(srow(&p, G::Clock, "Show the last", "", choice(&p, [(6, "6 hours"), (24, "Day"), (168, "Week")].iter().map(|(h, l)| (l.to_string(), hours == *h, Msg::Num("hours", *h as f32))).collect())));
        for (key, title, unit, top) in [("hist-charge", "Charge", "%", 100.0f32), ("hist-rate", "Power used", "W", 10.0f32)] {
            let pts = va(st.j(key), "points");
            let vals: Vec<f32> = pts.iter().map(|x| vf(x, "1", 0.0) as f32).collect();
            let max = vals.iter().copied().fold(top, f32::max);
            let since = if hours >= 168 { "A week ago" } else if hours >= 24 { "A day ago" } else { "6 hours ago" };
            let body: El = if vals.len() < 2 {
                t("No history yet: UPower keeps it while the PC runs", sz(12.0), p.muted).into()
            } else {
                ui::graph(ui::Graph { values: vals, dashed: vec![], max, min: 0.0, color: p.accent, grid: Pal::a(p.text, 0.08), fill: true, width: 2.0, centred: false, gridlines: vec![max * 0.25, max * 0.5, max * 0.75] }, Length::Fill, Length::Fixed(150.0))
            };
            c = c.push(card(&p, column![
                row![ts(title, sz(15.0), p.text), iced::widget::Space::new().width(Length::Fill), t1(format!("{} {unit} at the top", max.round()), sz(11.0), p.muted)],
                body,
                row![t1(since, sz(11.0), p.muted), iced::widget::Space::new().width(Length::Fill), t1("Now", sz(11.0), p.muted)]
            ].spacing(8.0)));
        }
    } else if st.has("pset") {
        c = c.push(info_row(&p, G::Info, "This PC has no battery", "Batteries of your devices are below"));
    }
    c = c.push(head(&p, "Your devices' batteries"));
    let devs = upower_devices(st.raw("upower"));
    if devs.is_empty() {
        c = c.push(info_row(&p, G::Bt, "No other batteries", "Wireless mice, keyboards, controllers and headphones show their charge here"));
    }
    for (kind, name, pct) in devs {
        let g = if kind.starts_with("keyboard") || kind.starts_with("mouse") {
            G::Grid
        } else if kind.starts_with("gaming") {
            G::Pad
        } else if kind.starts_with("head") {
            G::Sound
        } else {
            G::Bt
        };
        let ctl: El = if pct < 15.0 { row![ui::chip("Low", Pal::a(p.warn, 0.2), p.warn), t1(format!("{}%", pct.round()), sz(13.0), p.text)].spacing(8.0).align_y(iced::Center).into() } else { t1(format!("{}%", pct.round()), sz(13.0), p.text).into() };
        c = c.push(srow(&p, g, name, "", ctl));
    }
    c.into()
}

/// `upower --dump`: the batteries of devices that aren't this PC's (kind, name, percent).
pub fn upower_devices(text: &str) -> Vec<(String, String, f64)> {
    let mut out = Vec::new();
    for block in text.split("Device: ").skip(1) {
        let path = block.lines().next().unwrap_or("").trim();
        let kind = path.rsplit('/').next().unwrap_or("").split('_').take_while(|w| !w.chars().any(|c| c.is_ascii_digit()) && w.len() > 1).collect::<Vec<_>>().join("_");
        if kind.is_empty() || ["battery", "line_power", "DisplayDevice", "ups"].iter().any(|k| kind.starts_with(k)) {
            continue;
        }
        let field = |k: &str| block.lines().find_map(|l| l.trim().strip_prefix(&format!("{k}:")).map(|v| v.trim().to_string())).unwrap_or_default();
        let pct: f64 = field("percentage").trim_end_matches('%').trim().parse().unwrap_or(-1.0);
        if pct <= 0.0 {
            continue;
        }
        let model = field("model");
        out.push((kind, if model.is_empty() { "Device".to_string() } else { model }, pct));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upower_dump() {
        let t = "Device: /org/freedesktop/UPower/devices/battery_BAT0\n  native-path: BAT0\n  battery\n    percentage: 80%\nDevice: /org/freedesktop/UPower/devices/mouse_hidpp_battery_0\n  model: MX Master 3\n  mouse\n    percentage: 12%\nDevice: /org/freedesktop/UPower/devices/DisplayDevice\n    percentage: 80%\n";
        let d = upower_devices(t);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].0, "mouse_hidpp_battery");
        assert_eq!(d[0].1, "MX Master 3");
        assert_eq!(lid_action("lock"), "32");
        assert_eq!(button_action("menu"), "16");
    }
}

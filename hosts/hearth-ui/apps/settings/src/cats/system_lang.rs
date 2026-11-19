//! System › Date & time, Region & language and its pages, Clipboard and Search (Mantel's SetDateTime.qml,
//! SetRegion.qml, SetLanguages.qml, SetSpellcheck.qml, SetLanguagePacks.qml, SetInputMethods.qml, SetClipboard.qml,
//! SetSearch.qml), with the same helpers: hearth-system (time, time zones), hearth-admin (time zone, automatic time,
//! the clock by hand, the system's language: localectl), hearth-region (languages in order, spell check, fcitx5),
//! hearth-langpacks, hearth-clipboard; desktop.json's clock24 and clipboardHistory, bar.json's clock panel and
//! clipboard.json's selection settings.
//!
//! On KDE Plasma: the 24-hour clock and the world clocks are also set on Plasma's own clock (its digital clock
//! widgets, through plasmashell's scripting), the clipboard is Klipper's (klipperrc), and Search is Plasma's: the
//! file index is Baloo's (balooctl6, baloofilerc) and the search plugins KRunner's (krunnerrc [Plugins]), since
//! those are what Plasma's menu and Alt+Space use. Elsewhere it's Hearth's own index (hearth-search).

use super::*;
use hearth_ui::ui::{btn, ts, Kind};
use iced::widget::Row;
use serde_json::json;

pub fn view<'a>(a: &'a App, page: &'static str) -> El<'a> {
    match page {
        "datetime" => datetime(a),
        "region" => region(a),
        "languages" => languages(a),
        "spellcheck" => spellcheck(a),
        "language-packs" => langpacks(a),
        "input-methods" => input_methods(a),
        "clipboard" => clipboard(a),
        _ => search(a),
    }
}

// ------------------------------------------------------------------ Date & time

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// "2026-10-19T19:40:27-07:00" as the clock shows it: "7:40 PM · Monday, October 19, 2026".
pub fn now_text(iso: &str, h24: bool) -> String {
    let n = |a: usize, b: usize| iso.get(a..b).and_then(|s| s.parse::<i64>().ok());
    let (Some(y), Some(mo), Some(d), Some(h), Some(mi)) = (n(0, 4), n(5, 7), n(8, 10), n(11, 13), n(14, 16)) else { return String::new() };
    let time = if h24 { format!("{h:02}:{mi:02}") } else { format!("{}:{mi:02} {}", (h + 11) % 12 + 1, if h < 12 { "AM" } else { "PM" }) };
    // Sakamoto's weekday.
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let yy = if mo < 3 { y - 1 } else { y };
    let wd = (yy + yy / 4 - yy / 100 + yy / 400 + t[(mo - 1).clamp(0, 11) as usize] + d).rem_euclid(7);
    format!("{time} · {}, {} {d}, {y}", DAYS[wd as usize], MONTHS[(mo - 1).clamp(0, 11) as usize])
}

fn plasma_clock(script: &str) -> Vec<String> {
    dbus("org.kde.plasmashell", "/PlasmaShell", "org.kde.PlasmaShell.evaluateScript", &[&format!("string:{script}")])
}

fn clock24_cmds(on: bool) -> Vec<Vec<String>> {
    if !plasma() {
        return vec![];
    }
    // Plasma's digital clock: use24hFormat 0 = 12-hour, 2 = 24-hour (1 is the language's own).
    vec![plasma_clock(&format!("panels().forEach(function(p){{p.widgets('org.kde.plasma.digitalclock').forEach(function(w){{w.currentConfigGroup=['Appearance'];w.writeConfig('use24hFormat',{});}});}});", if on { 2 } else { 0 }))]
}

fn zones_cmds(zones: &[String]) -> Vec<Vec<String>> {
    if !plasma() {
        return vec![];
    }
    let ok = |z: &String| z.chars().all(|c| c.is_ascii_alphanumeric() || "/_+-".contains(c));
    let list: Vec<String> = std::iter::once("'Local'".to_string()).chain(zones.iter().filter(|z| ok(z)).map(|z| format!("'{z}'"))).collect();
    vec![plasma_clock(&format!("panels().forEach(function(p){{p.widgets('org.kde.plasma.digitalclock').forEach(function(w){{w.currentConfigGroup=['Appearance'];w.writeConfig('selectedTimeZones',[{}]);}});}});", list.join(",")))]
}

fn datetime(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let tm = st.j("time");
    let kc = vs(st.j("k-clock"), "clock");
    let h24 = if plasma() && (kc == "0" || kc == "2") { kc == "2" } else { a.sh.d_bool("clock24", false) };
    let mut c = Column::new().spacing(10.0);
    if let Some(w) = waiting(a, "time", "the clock") {
        c = c.push(w);
    }
    let zone = vs(tm, "timezone");
    let ntp = vb(tm, "ntp");
    let admin_ok = vb(tm, "admin");
    c = c.push(head(&p, "Clock"));
    let sub = if st.has("time") { format!("{}{}", zone.replace('_', " "), if ntp { if vb(tm, "synced") { " · set from the internet" } else { " · waiting for the internet" } } else { " · set by hand" }) } else { "Checking…".into() };
    let now = now_text(&vs(tm, "now"), h24);
    c = c.push(info_row(&p, G::Clock, if now.is_empty() { "…".to_string() } else { now }, sub));
    let set24 = |on: bool| Msg::Desk(json!({"clock24": on}), Ex::none().and_all(clock24_cmds(on)).re("k-clock"));
    c = c.push(srow(&p, G::Clock, "Clock", "How the taskbar shows the time", choice(&p, vec![("12-hour (2:05 PM)".into(), !h24, set24(false)), ("24-hour (14:05)".into(), h24, set24(true))])));
    if vb(tm, "available") {
        let can = vb(tm, "canNtp") || !present(tm, "canNtp");
        c = c.push(srow(&p, G::Update, "Set the time automatically", if can { "Uses the internet to keep the clock right" } else { "This system has no time service; set the clock by hand" }, sw(&p, ntp, Msg::Exec(Ex::new(adm("hearth-admin", vec!["ntp".into(), if ntp { "off".into() } else { "on".into() }])).re("time").ok(if ntp { "Automatic time is off." } else { "The clock will be set from the internet." }).fail("Couldn't change that")))));
    }
    if vb(tm, "available") && !ntp {
        c = c.push(head(&p, "Set the clock"));
        let d = st.txt("date").trim().to_string();
        let t0 = st.txt("clock").trim().to_string();
        let t2 = if t0.len() == 4 && t0.as_bytes()[1] == b':' { format!("0{t0}") } else { t0.clone() };
        let date_ok = d.len() == 10 && d.chars().enumerate().all(|(i, ch)| if i == 4 || i == 7 { ch == '-' } else { ch.is_ascii_digit() });
        let time_ok = t2.len() == 5 && t2.chars().enumerate().all(|(i, ch)| if i == 2 { ch == ':' } else { ch.is_ascii_digit() });
        let go = if date_ok && time_ok { Some(Msg::Seq(vec![Msg::Exec(Ex::new(adm("hearth-admin", vec!["time-set".into(), format!("{d} {t2}")])).re("time").ok("Clock set.").fail("Couldn't set the clock")), Msg::Text("date", String::new()), Msg::Text("clock", String::new())])) } else { None };
        c = c.push(srow(&p, G::Clock, "Set the clock", "Date as year-month-day, time in 24-hour form", row![
            field(&p, "Date, like 2026-10-20", &d, |s| Msg::Text("date", s)).width(Length::Fixed(sz(190.0))),
            field(&p, "Time, like 14:05", &t0, |s| Msg::Text("clock", s)).width(Length::Fixed(sz(140.0))),
            btn(&p, "Set the clock", Kind::Primary, false, go)
        ].spacing(8.0).align_y(iced::Center).into()));
        if !d.is_empty() && !date_ok {
            c = c.push(note(&p, "Type the date like 2026-10-20.", true));
        } else if !t0.is_empty() && !time_ok {
            c = c.push(note(&p, "Type the time like 14:05.", true));
        }
    }
    // ---- time zone
    c = c.push(head(&p, "Time zone"));
    let zones = va(st.j("zones"), "zones");
    if !admin_ok && st.has("time") {
        c = c.push(line(&p, "The time zone and automatic time are set here on Hearth, the OS."));
    } else {
        let q = st.txt("tz").trim().to_lowercase();
        // The same words as before: "Find a time zone".
        c = c.push(srow(&p, G::Clock, "Time zone", if zone.is_empty() { "…".to_string() } else { zone.replace('_', " ") }, field(&p, "Find a time zone", st.txt("tz"), |s| Msg::Text("tz", s)).width(Length::Fixed(sz(300.0))).into()));
        let set_zone = |z: &Value| -> Msg {
            let id = vs(z, "zone");
            Msg::Seq(vec![Msg::Text("tz", String::new()), Msg::Text("tzregion", String::new()), Msg::Exec(Ex::new(adm("hearth-admin", vec!["timezone".into(), id])).re("time").ok(format!("Time zone set to {}.", vs(z, "city"))).fail("Couldn't change the time zone"))])
        };
        if !q.is_empty() {
            let mut n = 0;
            for z in zones.iter().filter(|z| format!("{} {} {} utc{}", vs(z, "city"), vs(z, "region"), vs(z, "zone"), vs(z, "offset")).to_lowercase().contains(&q)).take(40) {
                n += 1;
                c = c.push(value_row(&p, G::Wifi, vs(z, "city"), format!("{} · UTC{}", vs(z, "region"), vs(z, "offset")), if vs(z, "zone") == zone { "In use" } else { "" }, true).into_with(set_zone(z)));
            }
            if n == 0 {
                c = c.push(line(&p, format!("No time zone matches \"{}\"", st.txt("tz"))));
            }
        } else {
            let region = st.txt("tzregion").to_string();
            if region.is_empty() {
                let mut seen: Vec<(String, usize)> = Vec::new();
                for z in zones {
                    let r = vs(z, "region");
                    match seen.iter_mut().find(|x| x.0 == r) {
                        Some(x) => x.1 += 1,
                        None => seen.push((r, 1)),
                    }
                }
                let here = zone.split('/').next().unwrap_or("").to_string();
                for (r, n) in seen {
                    let in_use = r.replace(' ', "_") == here;
                    c = c.push(link_row(&p, G::Wifi, r.clone(), if in_use { "In use".to_string() } else { format!("{n} places") }, Msg::Text("tzregion", r)));
                }
            } else {
                c = c.push(link_row(&p, G::Arrow, "All regions", region.clone(), Msg::Text("tzregion", String::new())));
                for z in zones.iter().filter(|z| vs(z, "region") == region) {
                    c = c.push(value_row(&p, G::Clock, vs(z, "city"), format!("UTC{}", vs(z, "offset")), if vs(z, "zone") == zone { "In use" } else { "" }, true).into_with(set_zone(z)));
                }
            }
        }
    }
    // ---- the clock's panel (bar.json: another calendar, holidays, world clocks)
    c = c.push(head(&p, "The clock's panel"));
    let bar = st.file("bar.json");
    let cals = [("", "None"), ("islamic", "Islamic (Hijri)"), ("hebrew", "Hebrew"), ("persian", "Persian (Solar Hijri)"), ("julian", "Julian"), ("indian", "Indian national (Saka)")];
    let hols = [("", "None"), ("us", "United States"), ("gb", "United Kingdom (England)"), ("ie", "Ireland"), ("ca", "Canada"), ("au", "Australia"), ("de", "Germany"), ("fr", "France"), ("nl", "Netherlands"), ("es", "Spain"), ("it", "Italy"), ("islamic", "Islamic days"), ("jewish", "Jewish days"), ("persian", "Persian days")];
    let pl_note = if plasma() { " (Hearth's clock panel; Plasma's own clock has its calendars in its settings)" } else { "" };
    c = c.push(chooser(a, "cal", G::Clock, "Another calendar", format!("Its date under each day of the month{pl_note}"), cals.iter().map(|(i, l)| (i.to_string(), l.to_string())).collect(), &vs(bar, "clockAlt"), |v| Msg::Json("bar.json", json!({"clockAlt": v}), Ex::none())));
    c = c.push(chooser(a, "hol", G::Clock, "Holidays", format!("Marked on the month, the next one under it{pl_note}"), hols.iter().map(|(i, l)| (i.to_string(), l.to_string())).collect(), &vs(bar, "clockHolidays"), |v| Msg::Json("bar.json", json!({"clockHolidays": v}), Ex::none())));
    let world: Vec<String> = va(bar, "clockZones").iter().filter_map(|z| z.as_str().map(String::from)).collect();
    for z in &world {
        let rest: Vec<String> = world.iter().filter(|x| *x != z).cloned().collect();
        let cmds = zones_cmds(&rest);
        c = c.push(act_row(&p, G::Wifi, z.rsplit('/').next().unwrap_or(z).replace('_', " "), z.clone(), "Remove", Some(Msg::Json("bar.json", json!({"clockZones": rest}), Ex::none().and_all(cmds)))));
    }
    if world.len() < 6 {
        c = c.push(srow(&p, G::Plus, "Add a world clock", if plasma() { "Shown in Plasma's clock too" } else { "" }, field(&p, "Type a city (Tokyo, New York…)", st.txt("wz"), |s| Msg::Text("wz", s)).width(Length::Fixed(sz(300.0))).into()));
        let wq = st.txt("wz").trim().to_lowercase();
        if wq.chars().count() >= 2 {
            for z in zones.iter().filter(|z| format!("{} {} {}", vs(z, "city"), vs(z, "region"), vs(z, "zone")).to_lowercase().contains(&wq) && !world.contains(&vs(z, "zone"))).take(6) {
                let mut l = world.clone();
                l.push(vs(z, "zone"));
                let cmds = zones_cmds(&l);
                c = c.push(pick_row(&p, format!("{} · {} · UTC{}", vs(z, "city"), vs(z, "region"), vs(z, "offset")), Msg::Seq(vec![Msg::Text("wz", String::new()), Msg::Json("bar.json", json!({"clockZones": l}), Ex::none().and_all(cmds))])));
            }
        }
    }
    c.into()
}

/// A row that sends `m` when pressed (for rows built with a value on the right).
trait Press<'a> {
    fn into_with(self, m: Msg) -> El<'a>;
}
impl<'a> Press<'a> for El<'a> {
    fn into_with(self, m: Msg) -> El<'a> {
        iced::widget::mouse_area(self).on_press(m).interaction(iced::mouse::Interaction::Pointer).into()
    }
}

// ------------------------------------------------------------------ Region & language

fn locale_opts(st: &State) -> Vec<(String, String)> {
    va(st.j("region"), "locales").iter().map(|l| (vs(l, "id"), vs(l, "name"))).collect()
}
fn locale_name(st: &State, id: &str) -> String {
    va(st.j("region"), "locales").iter().find(|l| vs(l, "id") == id).map(|l| vs(l, "name")).unwrap_or_else(|| id.to_string())
}

/// Every key sent each time (LANG plus the formats chosen), so a change never drops another one.
fn region_save(st: &State, changes: &[(&str, String)], ok: String) -> Msg {
    let r = st.j("region");
    let mut v: Vec<(String, String)> = vec![("LANG".into(), vs(r, "lang"))];
    for k in ["LC_NUMERIC", "LC_TIME", "LC_MONETARY", "LC_MEASUREMENT", "LC_PAPER"] {
        v.push((k.into(), vs(r, &format!("formats.{k}"))));
    }
    for (k, val) in changes {
        if let Some(x) = v.iter_mut().find(|x| x.0 == *k) {
            x.1 = val.clone();
        }
    }
    let lang = v[0].1.clone();
    let mut args = vec!["locale".to_string()];
    for (k, val) in &v {
        if !val.is_empty() && (k == "LANG" || *val != lang) {
            args.push(format!("{k}={val}"));
        }
    }
    Msg::Seq(vec![Msg::Flag("signout", true), Msg::Exec(Ex::new(adm("hearth-admin", args)).re("region").ok(ok).fail("Couldn't change the language"))])
}

fn region(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let r = st.j("region");
    let mut c = Column::new().spacing(10.0);
    if let Some(w) = waiting(a, "region", "the language and formats") {
        c = c.push(w);
        if !st.has("region") {
            return c.into();
        }
    }
    let lang = vs(r, "lang");
    let opts = locale_opts(st);
    c = c.push(head(&p, "Language"));
    c = c.push(chooser(a, "lang", G::Wifi, "Language", "Menus, apps and the login screen. Apps that don't have it stay in English", opts.clone(), &lang, move |v| {
        let name = v.clone();
        Msg::Text("pending-lang", name)
    }));
    // Picking a language: saved with the others (region_save needs the read), so it goes through the page.
    if !st.txt("pending-lang").is_empty() {
        let v = st.txt("pending-lang").to_string();
        let name = locale_name(st, &v);
        c = c.push(card(&p, row![
            column![ts(format!("Use {name}?"), sz(15.0), p.text), t("Menus, apps and the login screen, from the next sign-in.", sz(13.0), p.muted)].spacing(2.0).width(Length::Fill),
            btn(&p, "Use it", Kind::Primary, false, Some(Msg::Seq(vec![Msg::Text("pending-lang", String::new()), region_save(st, &[("LANG", v.clone())], format!("Language set to {name}. Sign out and in to see it everywhere."))]))),
            btn(&p, "Cancel", Kind::Normal, false, Some(Msg::Text("pending-lang", String::new())))
        ].spacing(10.0).align_y(iced::Center)));
    }
    c = c.push(head(&p, "Formats"));
    let fmt = { let f = vs(r, "formats.LC_TIME"); if !f.is_empty() { f } else { let f = vs(r, "formats.LC_NUMERIC"); if f.is_empty() { lang.clone() } else { f } } };
    let all_fmt = |v: String| -> Vec<(&'static str, String)> { ["LC_NUMERIC", "LC_TIME", "LC_MONETARY", "LC_MEASUREMENT", "LC_PAPER"].iter().map(|k| (*k, v.clone())).collect() };
    let st2: &State = st;
    c = c.push(chooser(a, "fmt", G::Wifi, "Region", "How dates, numbers and money are written", opts.clone(), &fmt, move |v| region_save(st2, &all_fmt(v.clone()), format!("Formats of {}.", locale_name(st2, &v)))));
    let lk = at(r, "look");
    let mut bits: Vec<String> = ["number", "money"].iter().map(|k| vs(lk, k)).filter(|x| !x.is_empty()).collect();
    if present(lk, "metric") {
        bits.push(if vb(lk, "metric") { "Metric".into() } else { "US units".into() });
    }
    if !vs(lk, "paper").is_empty() {
        bits.push(format!("{} paper", vs(lk, "paper")));
    }
    c = c.push(info_row(&p, G::Info, if present(r, "look") { format!("{} · {}", vs(lk, "date"), vs(lk, "time")) } else { "…".to_string() }, bits.join(" · ")));
    let more = st.on("fmtmore");
    c = c.push(act_row(&p, G::Gear, "Change one of them", "Numbers, dates, money, measurements or paper from another region", if more { "Close" } else { "Change" }, Some(Msg::Flag("fmtmore", !more))));
    if more {
        for (k, title) in [("LC_NUMERIC", "Numbers"), ("LC_TIME", "Dates and times"), ("LC_MONETARY", "Money"), ("LC_MEASUREMENT", "Measurement units"), ("LC_PAPER", "Paper size")] {
            let cur = { let f = vs(r, &format!("formats.{k}")); if f.is_empty() { lang.clone() } else { f } };
            c = c.push(chooser(a, k, G::Gear, title, "", opts.clone(), &cur, move |v| region_save(st2, &[(k, v)], "Saved.".into())));
        }
    }
    c = c.push(link_row(&p, G::Wifi, "Languages in order", "More than one language: apps try them one after another", Msg::Go("languages")));
    c = c.push(link_row(&p, G::Check, "Spell check", "Dictionary, checking as you type, words to leave alone", Msg::Go("spellcheck")));
    c = c.push(link_row(&p, G::Grid, "Chinese, Japanese & Korean typing", "Input methods (fcitx5), off unless you turn them on", Msg::Go("input-methods")));
    c = c.push(link_row(&p, G::Clock, "Date & time", "Time zone, 24-hour clock, setting the time automatically", Msg::Go("datetime")));
    if st.on("signout") {
        c = c.push(act_row(&p, G::User, "Sign out now", "Apps pick up the new language and formats when you sign in again", "Sign out", Some(Msg::App(sv(&["nimbus-mode", "logout"])))));
    }
    c.into()
}

fn languages(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let list: Vec<String> = va(st.j("languages"), "languages").iter().filter_map(|x| x.as_str().map(String::from)).collect();
    let id_of = |l: &str| l.split('.').next().unwrap_or(l).to_string();
    let locales: Vec<(String, String)> = va(st.j("region"), "locales").iter().filter(|l| !vb(l, "missing") && vs(l, "id") != "C.UTF-8").map(|l| (id_of(&vs(l, "id")), vs(l, "name"))).collect();
    let name_of = |id: &str| locales.iter().find(|x| x.0 == id).map(|x| x.1.clone()).unwrap_or_else(|| id.to_string());
    let save = |l: Vec<String>| Msg::Seq(vec![Msg::Flag("langchanged", true), Msg::Exec(Ex::new(py("hearth-region", &["languages-set"])).input(json!({"languages": l}).to_string()).store("languages").ok("Saved. Apps use it after you sign out and in.").fail("Couldn't save the languages"))]);
    let mut c = Column::new().spacing(10.0);
    c = c.push(head(&p, if list.is_empty() { "Only the system's language" } else { "Apps try these in order" }));
    if list.is_empty() {
        c = c.push(info_row(&p, G::Wifi, "Apps use the language from Region & language", "Add languages below so an app without your first one uses the next, before falling back to English"));
    }
    for (i, l) in list.iter().enumerate() {
        let mv = |d: i64| -> Option<Msg> {
            let j = i as i64 + d;
            if j < 0 || j >= list.len() as i64 {
                return None;
            }
            let mut n = list.clone();
            n.swap(i, j as usize);
            Some(save(n))
        };
        let rm: Vec<String> = list.iter().enumerate().filter(|(k, _)| *k != i).map(|(_, x)| x.clone()).collect();
        let ctl: El = row![pill(&p, "Up", false, mv(-1)), pill(&p, "Down", false, mv(1)), pill(&p, "Remove", true, Some(save(rm)))].spacing(6.0).into();
        c = c.push(srow(&p, G::Wifi, format!("{}. {}", i + 1, name_of(l)), if i == 0 { "First choice" } else { "If an app hasn't got the ones above" }, ctl));
    }
    c = c.push(head(&p, "Add a language"));
    let left: Vec<(String, String)> = locales.iter().filter(|x| !list.contains(&x.0)).cloned().collect();
    let l2 = list.clone();
    c = c.push(chooser(a, "addlang", G::Plus, "Language", "Added at the end of the list", left, "", move |v| {
        let mut n = l2.clone();
        n.push(v);
        Msg::Seq(vec![Msg::Open(String::new()), Msg::Flag("langchanged", true), Msg::Exec(Ex::new(py("hearth-region", &["languages-set"])).input(json!({"languages": n}).to_string()).store("languages").ok("Saved. Apps use it after you sign out and in.").fail("Couldn't save the languages"))])
    }));
    if st.on("langchanged") {
        c = c.push(act_row(&p, G::User, "Sign out now", "Apps pick up the new order when you sign in again", "Sign out", Some(Msg::App(sv(&["nimbus-mode", "logout"])))));
    }
    c = c.push(link_row(&p, G::Update, "Translations & dictionaries", "Download what your languages need", Msg::Go("language-packs")));
    c.into()
}

fn spellcheck(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let sp = st.j("spell");
    let s = at(sp, "settings");
    let mut c = Column::new().spacing(10.0);
    if let Some(w) = waiting(a, "spell", "spell check") {
        c = c.push(w);
        if !st.has("spell") {
            return c.into();
        }
    }
    let dicts: Vec<(String, String)> = va(sp, "dictionaries").iter().map(|d| (vs(d, "id"), vs(d, "name"))).collect();
    if dicts.is_empty() {
        c = c.push(card(&p, column![ts("No dictionaries on this system", sz(15.0), p.text), t("Spell checking needs a Hunspell dictionary for your language", sz(13.0), p.muted)].spacing(4.0)));
    }
    let set = |ch: Value| Msg::Exec(Ex::new(py("hearth-region", &["spell-set"])).input(ch.to_string()).store("spell").fail("Couldn't save spell check"));
    c = c.push(head(&p, "Checking"));
    let cur = { let d = vs(s, "defaultLanguage"); if d.is_empty() { dicts.first().map(|x| x.0.clone()).unwrap_or_default() } else { d } };
    c = c.push(chooser(a, "dict", G::Wifi, "Dictionary", "The language words are checked in", dicts, &cur, move |v| Msg::Exec(Ex::new(py("hearth-region", &["spell-set"])).input(json!({"defaultLanguage": v}).to_string()).store("spell"))));
    for (k, title, sub) in [
        ("backgroundCheckerEnabled", "Check as you type", "Underlines words that look wrong in apps' text boxes"),
        ("checkerEnabledByDefault", "On in every new text box", "Off: apps turn it on themselves where it helps"),
        ("autodetect", "Guess the language", "Checks a sentence in French with the French dictionary, if you have it"),
        ("skipUppercase", "Skip words in capitals", "Like NASA or HTML"),
        ("skipRunTogether", "Skip run-together words", "Like setupfile"),
    ] {
        let on = vb(s, k);
        let mut o = serde_json::Map::new();
        o.insert(k.into(), json!(!on));
        c = c.push(srow(&p, G::Check, title, sub, sw(&p, on, set(Value::Object(o)))));
    }
    c = c.push(head(&p, "Words to leave alone"));
    let words: Vec<String> = va(s, "ignore").iter().filter_map(|x| x.as_str().map(String::from)).collect();
    let mut r = Row::new().spacing(6.0);
    for w in &words {
        let rest: Vec<String> = words.iter().filter(|x| *x != w).cloned().collect();
        r = r.push(pill(&p, format!("{w}  ✕"), false, Some(set(json!({"ignore": rest})))));
    }
    if !words.is_empty() {
        c = c.push(card(&p, r.wrap()));
    }
    let typed = st.txt("word").trim().to_string();
    let mut add = words.clone();
    add.push(typed.clone());
    c = c.push(field(&p, "Add a word, like Hearth", st.txt("word"), |s| Msg::Text("word", s)).on_submit(if typed.is_empty() { Msg::Text("word", String::new()) } else { Msg::Seq(vec![Msg::Text("word", String::new()), set(json!({"ignore": add}))]) }).width(Length::Fixed(sz(320.0))));
    c.into()
}

fn langpacks(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let lp = st.j("langpacks");
    let mut c = Column::new().spacing(10.0);
    if let Some(w) = waiting(a, "langpacks", "your languages") {
        c = c.push(w);
        if !st.has("langpacks") {
            return c.into();
        }
    }
    let flat = vb(lp, "flatpak");
    let langs = va(lp, "languages");
    c = c.push(head(&p, "Your languages"));
    if langs.is_empty() {
        c = c.push(link_row(&p, G::Wifi, "Choose your languages first", "Languages in order", Msg::Go("languages")));
    }
    for l in langs {
        let id = vs(l, "id");
        let apps = vf(l, "apps", 0.0) as i64;
        let dict = vs(l, "dict");
        let fl = vb(l, "flatpak");
        let ready = dict != "missing" && (!flat || fl);
        let sub = [if apps > 0 { format!("{apps} system apps speak it") } else { "The system's apps: English for now".into() }, if flat { if fl { "Flatpak apps: yes".into() } else { "Flatpak apps: not yet".into() } } else { String::new() }, if dict == "missing" { "No dictionary".into() } else { "Dictionary".into() }].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" · ");
        let key = format!("lp:{id}");
        let open = st.open == key;
        c = c.push(srow(&p, G::Wifi, vs(l, "name"), sub, row![hearth_ui::ui::chip(if ready { "Ready" } else { "Missing parts" }, Pal::a(if ready { p.good } else { p.warn }, 0.18), if ready { p.good } else { p.warn }), pill(&p, if open { "Close" } else { "More" }, true, Some(Msg::Open(key.clone())))].spacing(10.0).align_y(iced::Center).into()));
        if open {
            let mut inner = Column::new().spacing(10.0);
            inner = inner.push(inner_row(&p, G::Grid, "The system's apps", if apps > 0 { format!("{apps} apps came with this language in Hearth OS") } else { "None came with it; they show the next language in your list".into() }, iced::widget::Space::new().width(Length::Fixed(0.0)).into()));
            if flat && fl {
                inner = inner.push(inner_row(&p, G::Close, "Flatpak apps: translations downloaded", "Stop getting this language's translations for Flatpak apps", pill(&p, "Stop", true, Some(Msg::Exec(Ex::new(py_s("hearth-langpacks", vec!["flatpak-off".into(), id.clone()])).store("langpacks").ok("Flatpak apps won't get it any more."))))));
            }
            if dict != "missing" {
                inner = inner.push(inner_row(&p, G::Check, if dict == "system" { "Spell check: came with Hearth OS" } else { "Spell check: downloaded" }, "Choose it on Spell check", iced::widget::Space::new().width(Length::Fixed(0.0)).into()));
            }
            if dict == "downloaded" {
                inner = inner.push(inner_row(&p, G::Close, "Remove the downloaded dictionary", "", pill(&p, "Remove", true, Some(Msg::Exec(Ex::new(py_s("hearth-langpacks", vec!["remove-dict".into(), id.clone()])).store("langpacks").ok("Removed."))))));
            }
            c = c.push(card(&p, inner));
        }
    }
    if !langs.is_empty() {
        c = c.push(head(&p, "Download"));
        let missing: Vec<&Value> = langs.iter().filter(|l| vs(l, "dict") == "missing" || (flat && !vb(l, "flatpak"))).collect();
        let busy = !st.busy.is_empty();
        let ids: Vec<String> = missing.iter().map(|l| vs(l, "id")).collect();
        let names: Vec<String> = missing.iter().map(|l| vs(l, "name")).collect();
        c = c.push(act_row(&p, G::Update, if busy { "Downloading…" } else if missing.is_empty() { "Everything's here" } else { "Download what's missing" }, if missing.is_empty() { "Every language you chose has its translations and dictionary".to_string() } else { format!("{}: Flatpak apps' translations and spell check dictionaries", names.join(", ")) }, "Download", if missing.is_empty() || busy { None } else { Some(Msg::Exec(Ex::new(py("hearth-langpacks", &["get"])).input(json!({"languages": ids}).to_string()).store("langpacks").ok("Downloaded. Apps use it the next time they start.").fail("Couldn't download them").secs(1800))) }));
        c = c.push(link_row(&p, G::Wifi, "Languages in order", "Add or reorder your languages", Msg::Go("languages")));
    }
    c.into()
}

fn input_methods(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let im = st.j("im");
    let kwin = vb(im, "kwin") || plasma();
    let mut c = Column::new().spacing(10.0);
    if !kwin {
        c = c.push(card(&p, column![ts("Works with KWin", sz(15.0), p.text), t("fcitx5 types into apps through KWin (KDE Plasma's window manager).", sz(13.0), p.muted)].spacing(4.0)));
    }
    if let Some(w) = waiting(a, "im", "the input methods") {
        c = c.push(w);
    }
    c = c.push(head(&p, "Input methods (fcitx5)"));
    if st.has("im") && !vb(im, "installed") {
        let busy = !st.busy.is_empty();
        c = c.push(act_row(&p, G::Update, if busy { "Downloading…" } else { "Get Chinese, Japanese and Korean typing" }, "fcitx5 from Flathub, with Mozc (Japanese), Pinyin (Chinese) and Hangul (Korean). About 150 MB, for you only", "Get it", if busy { None } else { Some(Msg::Exec(Ex::new(py("hearth-region", &["im-install"])).store("im").ok("Installed. Turn it on below.").fail("Couldn't get it").secs(1800))) }));
    }
    if vb(im, "installed") {
        let on = vb(im, "on");
        c = c.push(srow(&p, G::Grid, "Type with fcitx5", "Switch languages with Ctrl+Space. Used from the next sign-in", switch(&p, on, if kwin || on { Some(Msg::Exec(Ex::new(py("hearth-region", &["im-set", if on { "off" } else { "on" }])).store("im").ok(if on { "Off from the next sign-in: Hearth's on-screen keyboard is back." } else { "On from the next sign-in. Hearth's on-screen keyboard is off while fcitx5 is on." }))) } else { None })));
        if on {
            c = c.push(card(&p, column![ts("Hearth's on-screen keyboard is off", sz(15.0), p.warn), t("KWin takes one input method at a time. Turn fcitx5 off here to have the on-screen keyboard back", sz(13.0), p.muted)].spacing(4.0)));
        }
    }
    c = c.push(head(&p, "What you get"));
    let panel = |g: &'static str, title: &'static str, sub: &'static str| -> El<'static> {
        card(&p, column![t1(g, sz(30.0), p.accent), ts(title, sz(14.0), p.text), t(sub, sz(12.0), p.muted)].spacing(4.0))
    };
    c = c.push(row![panel("日本語", "Japanese", "Mozc: kana and kanji"), panel("中文", "Chinese", "Pinyin, Shuangpin, Wubi"), panel("한국어", "Korean", "Hangul")].spacing(10.0));
    c.into()
}

// ------------------------------------------------------------------ Clipboard

fn clipboard(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let k = st.j("k-clip");
    let pl = plasma();
    let avail = pl || vb(st.j("clip"), "available");
    let cb = st.file("clipboard.json");
    let hist = if pl && !vs(k, "max").is_empty() { vf(k, "max", 20.0) > 1.0 } else if pl { true } else { a.sh.d_bool("clipboardHistory", false) };
    let sel = if pl && !vs(k, "ignoresel").is_empty() { vs(k, "ignoresel") == "false" } else { vb(cb, "selection") };
    let sync = if pl && !vs(k, "sync").is_empty() { vs(k, "sync") == "true" } else { vb(cb, "sync") };
    let mut c = Column::new().spacing(10.0);
    let hist_kde = |on: bool| -> Vec<Vec<String>> { if pl { vec![kw("klipperrc", &["General"], "MaxClipItems", if on { "25" } else { "1" }), kw("klipperrc", &["General"], "KeepClipboardContents", if on { "true" } else { "false" })] } else { vec![] } };
    let sub = if !st.read("clip") && !pl { "Checking…" } else if !avail { "Needs wl-clipboard, which isn't installed" } else { "Keep the last 25 things you copy, and the ones you pin, to paste again with Super+V. Kept on this device only." };
    c = c.push(srow(&p, G::List, "Clipboard history", sub, sw(&p, hist, Msg::Desk(json!({"clipboardHistory": !hist}), Ex::none().and_all(hist_kde(!hist)).re("k-clip").ok(if hist { "Clipboard history is off. What's already kept stays until you clear it." } else { "Clipboard history is on." })))));
    if avail {
        let kde1 = |key: &str, v: &str| -> Vec<Vec<String>> { if pl { vec![kw("klipperrc", &["General"], key, v)] } else { vec![] } };
        c = c.push(srow(&p, G::List, "Selection history", if hist { "Also keep what you highlight (middle-click pastes it), marked Selection" } else { "Turn on clipboard history first" }, switch(&p, sel, if hist { Some(Msg::Json("clipboard.json", json!({"selection": !sel}), Ex::none().and_all(kde1("IgnoreSelection", if sel { "true" } else { "false" })).re("k-clip").ok(if sel { "Highlighted text isn't kept any more." } else { "What you highlight is kept too." }))) } else { None })));
        c = c.push(srow(&p, G::Update, "Keep the clipboard and selection in step", if hist { "Text you copy can be middle-clicked, and text you highlight can be pasted with Ctrl+V" } else { "Turn on clipboard history first" }, switch(&p, sync, if hist { Some(Msg::Json("clipboard.json", json!({"sync": !sync}), Ex::none().and_all(kde1("SyncClipboards", if sync { "false" } else { "true" })).re("k-clip").ok(if sync { "The clipboard and the selection are separate again." } else { "The clipboard and the selection are kept in step." }))) } else { None })));
        if pl {
            c = c.push(link_row(&p, G::List, "Open clipboard history", "Super+V", Msg::Exec(Ex::new(dbus("org.kde.klipper", "/klipper", "org.kde.klipper.klipper.showKlipperPopupMenu", &[])).fail("Couldn't open the clipboard history"))));
        }
        if !st.on("clipclear") {
            c = c.push(act_row(&p, G::Close, "Clear clipboard history…", "Pinned items stay", "Clear…", Some(Msg::Flag("clipclear", true))));
        } else {
            let mut ex = Ex::new(py("hearth-clipboard", &["clear"])).ok("Clipboard history cleared.").fail("Couldn't clear it");
            if pl {
                ex = ex.and(dbus("org.kde.klipper", "/klipper", "org.kde.klipper.klipper.clearClipboardHistory", &[]));
            }
            c = c.push(card(&p, row![t("Clear the clipboard history? Pinned items stay.", sz(14.0), p.text).width(Length::Fill), btn(&p, "Yes, clear it", Kind::Danger, false, Some(Msg::Seq(vec![Msg::Flag("clipclear", false), Msg::Exec(ex)]))), btn(&p, "Cancel", Kind::Normal, false, Some(Msg::Flag("clipclear", false)))].spacing(8.0).align_y(iced::Center)));
        }
    }
    c = c.push(line(&p, "Passwords copied from password managers that mark them as secret are never kept or put on the selection. Super+. opens the emoji picker."));
    c.into()
}

// ------------------------------------------------------------------ Search

/// KRunner's plugins Plasma has (krunnerrc [Plugins] <id>Enabled), in Hearth's words.
const RUNNERS: [(&str, &str, &str); 16] = [
    ("calculator", "Calculator", "Sums as you type: 2+2, =sqrt(2), 15% of 80"),
    ("unitconverter", "Unit and currency conversion", "10 km to mi, 72 f in c, 20 usd to eur"),
    ("krunner_webshortcuts", "Web shortcuts", "gg:words (Google), wp: (Wikipedia), yt: (YouTube), a web address"),
    ("krunner_shell", "Commands", "Type a command to run it, in the terminal or on its own"),
    ("krunner_recentdocuments", "Recent documents", "Files you opened lately"),
    ("windows", "Open windows", "Switch to a window by its title"),
    ("krunner_bookmarksrunner", "Bookmarks", "Your browser's bookmarks"),
    ("krunner_killrunner", "Kill an app", "kill firefox: closes your own program"),
    ("org.kde.datetime", "World clock", "time tokyo"),
    ("krunner_spellcheck", "Spell check", "spell recieve"),
    ("krunner_charrunner", "Special characters", "Type a character's code to get it"),
    ("krunner_powerdevil", "Power", "sleep, hibernate, screen brightness"),
    ("krunner_sessions", "Session", "lock, log out, restart, shut down"),
    ("krunner_appstream", "App store results", "Apps on Flathub you can install"),
    ("krunner_services", "Apps", "Apps on this PC by name"),
    ("baloosearch", "Files", "Your files by name and what's in them (the file index)"),
];

pub fn kde_search_read() -> String {
    let mut s = String::new();
    s += &kr("indexing", "baloofilerc", &["Basic Settings"], "Indexing-Enabled");
    s += &kr("basic", "baloofilerc", &["General"], "only basic indexing");
    s += &kr("folders", "baloofilerc", &["General"], "folders");
    s += &kr("exclude", "baloofilerc", &["General"], "exclude folders");
    for (id, _, _) in RUNNERS {
        s += &kr(&format!("run.{id}"), "krunnerrc", &["Plugins"], &format!("{id}Enabled"));
    }
    s
}

fn kw_path(group: &str, key: &str, value: &str) -> Vec<String> {
    sv(&["kwriteconfig6", "--notify", "--file", "baloofilerc", "--group", group, "--key", key, "--type", "path", value])
}

fn search(a: &App) -> El<'_> {
    if plasma() {
        return search_plasma(a);
    }
    search_hearth(a)
}

fn baloo_line(text: &str) -> String {
    let field = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(k).map(|v| v.trim().to_string())).unwrap_or_default();
    let state = field("Indexer state:");
    let files = field("Total files indexed:");
    let size = field("Current size of index is");
    let mut parts = Vec::new();
    if !files.is_empty() {
        parts.push(format!("{files} files and folders"));
    }
    if !size.is_empty() {
        parts.push(size);
    }
    if !state.is_empty() {
        parts.push(state.to_lowercase());
    }
    if parts.is_empty() {
        if text.to_lowercase().contains("disabled") { "Off: Plasma's menu and Dolphin look through your folders when you search (slower)".into() } else { "Plasma's file index (Baloo)".into() }
    } else {
        parts.join(" · ")
    }
}

fn search_plasma(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let k = st.j("k-search");
    let on = vs(k, "indexing") != "false" && !st.raw("baloo").to_lowercase().contains("disabled");
    let contents = vs(k, "basic") != "true";
    let mut c = Column::new().spacing(10.0);
    c = c.push(head(&p, "File index"));
    c = c.push(srow(&p, G::Search, "Index my files", baloo_line(st.raw("baloo")), sw(&p, on, Msg::Exec(Ex::new(sv(&["balooctl6", if on { "disable" } else { "enable" }])).re("baloo").re("k-search").ok(if on { "The index is off. Searching still works, more slowly." } else { "The index is on. It's made in the background at the lowest priority." }).fail("Couldn't change the file index")))));
    if on {
        c = c.push(srow(&p, G::List, "Search inside documents", "Text files, OpenDocument, Word, Excel, PowerPoint and PDFs: what's in them, not only their names. Kept on this device only.", sw(&p, contents, Msg::Exec(Ex::new(kw("baloofilerc", &["General"], "only basic indexing", if contents { "true" } else { "false" })).and(sv(&["balooctl6", "check"])).re("k-search").ok(if contents { "Documents' text is taken out of the index on the next pass." } else { "Documents' text is added on the next pass." })))));
        c = c.push(act_row(&p, G::Update, "Rebuild the index", "Start again from nothing (it's made again in the background)", "Rebuild", Some(Msg::Exec(Ex::new(sv(&["balooctl6", "purge"])).and(sv(&["balooctl6", "enable"])).re("baloo").ok("Rebuilding the index…").fail("Couldn't rebuild the index")))));
        let split = |s: String| -> Vec<String> { s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect() };
        let folders = { let f = split(vs(k, "folders")); if f.is_empty() { vec![format!("{}/", hearth_ui::paths::home().to_string_lossy())] } else { f } };
        let exclude = split(vs(k, "exclude"));
        c = c.push(head(&p, "Folders it covers"));
        for (i, f) in folders.iter().enumerate() {
            let rest: Vec<String> = folders.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, x)| x.clone()).collect();
            c = c.push(act_row(&p, G::Folder, f.clone(), "", "Remove", if folders.len() > 1 { Some(Msg::Exec(Ex::new(kw_path("General", "folders", &rest.join(","))).re("k-search"))) } else { None }));
        }
        let typed = st.txt("addfolder").trim().to_string();
        let mut add = folders.clone();
        add.push(typed.clone());
        c = c.push(srow(&p, G::Plus, "Add a folder", "Like ~/Projects or /run/media/you/Data, then Enter", field(&p, "A folder", st.txt("addfolder"), |s| Msg::Text("addfolder", s)).on_submit(if typed.is_empty() { Msg::Text("addfolder", String::new()) } else { Msg::Seq(vec![Msg::Text("addfolder", String::new()), Msg::Exec(Ex::new(kw_path("General", "folders", &add.join(","))).re("k-search").ok("Added. It's indexed on the next pass."))]) }).width(Length::Fixed(sz(300.0))).into()));
        c = c.push(head(&p, "Left out"));
        c = c.push(info_row(&p, G::Shield, "Always left out", "Hidden folders (your settings and keyrings), caches and build folders"));
        for (i, f) in exclude.iter().enumerate() {
            let rest: Vec<String> = exclude.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, x)| x.clone()).collect();
            c = c.push(act_row(&p, G::Folder, f.clone(), "", "Remove", Some(Msg::Exec(Ex::new(kw_path("General", "exclude folders", &rest.join(","))).re("k-search")))));
        }
        let typed = st.txt("addexcl").trim().to_string();
        let mut add = exclude.clone();
        add.push(typed.clone());
        c = c.push(srow(&p, G::Plus, "Leave out a folder", "Like ~/Private, then Enter", field(&p, "A folder to leave out", st.txt("addexcl"), |s| Msg::Text("addexcl", s)).on_submit(if typed.is_empty() { Msg::Text("addexcl", String::new()) } else { Msg::Seq(vec![Msg::Text("addexcl", String::new()), Msg::Exec(Ex::new(kw_path("General", "exclude folders", &add.join(","))).re("k-search").ok("Left out. What was indexed from it goes on the next pass."))]) }).width(Length::Fixed(sz(300.0))).into()));
    }
    c = c.push(head(&p, "In the app menu's search and Alt+Space (KRunner)"));
    for (id, title, sub) in RUNNERS {
        let on = vs(k, &format!("run.{id}")) != "false";
        c = c.push(srow(&p, G::Search, title, sub, sw(&p, on, Msg::Exec(Ex::new(kw("krunnerrc", &["Plugins"], &format!("{id}Enabled"), if on { "false" } else { "true" })).re("k-search")))));
    }
    c = c.push(line(&p, "Alt+Space or Alt+F2 opens KDE's search bar (KRunner) at the top of the screen, with everything above."));
    c.into()
}

const HEARTH_PLUGINS: [(&str, &str, &str); 21] = [
    ("calc", "Calculator", "Sums as you type: 2+2, =sqrt(2), 15% of 80"),
    ("units", "Unit conversion", "10 km to mi, 72 f in c, 500 MB to GB"),
    ("currency", "Currency conversion", "20 usd to eur (today's rates when online, the built-in ones offline)"),
    ("web", "Web shortcuts", "gg:words (Google), wp: (Wikipedia), yt: (YouTube), ddg:, gh:, fh: (Flathub), a web address"),
    ("commands", "Commands", "Type > and a command to run it, in the terminal or on its own"),
    ("emoji", "Emoji", "heart, thumbs up, or :fire for emoji only; Enter copies it"),
    ("recent", "Recent documents", "Files you opened lately"),
    ("windows", "Open windows", "Switch to a window by its title"),
    ("browser", "Bookmarks and browser history", "Firefox's and Chromium's, read from a copy (the browser is never touched)"),
    ("desktops", "Virtual desktops and activities", "desktop 2, or an activity's name: Enter switches to it"),
    ("kill", "Kill an app", "kill firefox: closes your own program after asking"),
    ("worldclock", "World clock", "time in tokyo, 3pm london to new york"),
    ("spell", "Spell check and dictionary", "spell recieve, define hearth (hunspell; meanings on Wiktionary)"),
    ("chars", "Special characters", "degree, euro, em dash, e acute, u+00e9; Enter copies it"),
    ("colours", "Colours", "#ff8800, rgb(255, 136, 0), hsl(32, 100%, 50%): a swatch, and each form to copy"),
    ("power", "Power and session", "sleep, lock, restart, shut down, log out (asks first when it closes your apps)"),
    ("media", "Media control", "play, pause, next, previous: for what's playing"),
    ("store", "App store results", "install krita: apps on Flathub (only looked up when you type install or store)"),
    ("contacts", "Contacts", "People in Hearth Contacts by name, e-mail or phone; Enter copies the e-mail"),
    ("events", "Calendar events", "What's coming up in Hearth Calendar; today, tomorrow, this week"),
    ("notes", "Notes", "Hearth Notes by title and by what's in them"),
];

/// Hearth's own index and search plugins (hearth-search), where Plasma isn't the desktop.
fn search_hearth(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let s = st.j("hsearch");
    let pr = at(s, "prefs");
    let mut c = Column::new().spacing(10.0);
    if let Some(w) = waiting(a, "hsearch", "the file index") {
        c = c.push(w);
        if !st.has("hsearch") {
            return c.into();
        }
    }
    let set = |ch: Value, ok: &str| Msg::Exec(Ex::new(py("hearth-search", &["prefs-set"])).input(ch.to_string()).re("hsearch").ok(ok).fail("Couldn't save that"));
    let on = vb(pr, "indexing");
    let line_ = if !vb(s, "fts5") && present(s, "fts5") { "This system's SQLite has no full-text search, so files are found by looking through folders instead".to_string() } else if vs(s, "paused") == "off" { "Off: the menus and Files look through your folders when you search (slower)".into() } else if vb(s, "running") && !vb(s, "complete") { format!("Making the index… {} found so far", vf(s, "files", 0.0)) } else { format!("{} files and folders · {}", vf(s, "files", 0.0), hearth_ui::ui::bytes(vf(s, "bytes", 0.0))) };
    c = c.push(head(&p, "File index"));
    c = c.push(srow(&p, G::Search, "Index my files", line_, sw(&p, on, set(json!({"indexing": !on}), if on { "The index is off. Searching still works, more slowly." } else { "The index is on. It's made in the background at the lowest priority." }))));
    if on {
        let ct = vb(pr, "contents");
        c = c.push(srow(&p, G::List, "Search inside documents", format!("Text files, OpenDocument, Word, Excel, PowerPoint{}: what's in them, not only their names. Kept on this device only.", if vb(s, "pdf") { " and PDFs" } else { "" }), sw(&p, ct, set(json!({"contents": !ct}), ""))));
        let ob = vb(pr, "onBattery");
        c = c.push(srow(&p, G::Power, "Keep indexing on battery", "Off: the index waits until you plug in", sw(&p, ob, set(json!({"onBattery": !ob}), ""))));
        c = c.push(act_row(&p, G::Update, "Rebuild the index", "Start again from nothing (it's made again in the background)", "Rebuild", Some(Msg::Exec(Ex::new(py("hearth-search", &["rebuild"])).re("hsearch").ok("Rebuilding the index…")))));
        let folders: Vec<String> = va(pr, "folders").iter().filter_map(|x| x.as_str().map(String::from)).collect();
        c = c.push(head(&p, "Folders it covers"));
        for (i, f) in folders.iter().enumerate() {
            let rest: Vec<String> = folders.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, x)| x.clone()).collect();
            c = c.push(act_row(&p, G::Folder, if f == "~" { "Home".to_string() } else { f.clone() }, "", "Remove", if folders.len() > 1 { Some(set(json!({"folders": rest}), "")) } else { None }));
        }
        let exclude: Vec<String> = va(pr, "exclude").iter().filter_map(|x| x.as_str().map(String::from)).collect();
        c = c.push(head(&p, "Left out"));
        for (i, f) in exclude.iter().enumerate() {
            let rest: Vec<String> = exclude.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, x)| x.clone()).collect();
            c = c.push(act_row(&p, G::Folder, f.clone(), "", "Remove", Some(set(json!({"exclude": rest}), ""))));
        }
    }
    c = c.push(head(&p, "In the app menus' search"));
    let off = at(pr, "pluginsOff");
    for (id, title, sub) in HEARTH_PLUGINS {
        if !va(s, "plugins").iter().any(|x| x.as_str() == Some(id)) {
            continue;
        }
        let on = !vb(off, id);
        let mut o = off.as_object().cloned().unwrap_or_default();
        if on {
            o.insert(id.into(), json!(true));
        } else {
            o.remove(id);
        }
        c = c.push(srow(&p, G::Search, title, sub, sw(&p, on, set(json!({"pluginsOff": o}), ""))));
    }
    let cur = vb(pr, "currencyOnline");
    c = c.push(srow(&p, G::Wifi, "Today's currency rates", "Once a day, from the European Central Bank, when you're online. Off: the built-in rates only (the answer says how old they are).", sw(&p, cur, set(json!({"currencyOnline": !cur}), ""))));
    let tts = at(pr, "typeToSearch").as_bool().unwrap_or(true);
    c = c.push(head(&p, "Search bar (Alt+Space)"));
    c = c.push(srow(&p, G::Grid, "Type on the desktop to search", "With the desktop in front, typing opens the search bar with what you typed", sw(&p, tts, set(json!({"typeToSearch": !tts}), ""))));
    c.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_words() {
        assert_eq!(now_text("2026-10-19T19:40:27-07:00", false), "7:40 PM · Monday, October 19, 2026");
        assert_eq!(now_text("2026-11-18T07:05:00+00:00", true), "07:05 · Wednesday, November 18, 2026");
        assert!(baloo_line("Indexer state: Idle\nTotal files indexed: 1,234\nCurrent size of index is 3.10 MiB\n").contains("1,234 files"));
    }
}

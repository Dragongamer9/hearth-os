//! System › Display (the approved mockup settings-A2-display-mouse.png; Mantel's SetDisplay.qml): your screens drawn
//! as they're placed (1, 2…), the picked one lit, with its name, mode and graphics card, Identify, Detect and Arrange;
//! Brightness & colour (brightness, auto-brightness, external screens', Night light with how warm and when, HDR);
//! Scale & layout (scale, resolution with Other…, refresh rate, rotation, X11 apps' scaling); HDR and colour on the
//! desktop, the picture signal, games and smoothness, several screens, screen power saving, Related and Advanced.
//!
//! Everything goes through hearth-screens, as Mantel on KWin does (on KDE Plasma it uses kscreen-doctor and KWin's
//! D-Bus: docs/SCREENS.md): a change that could leave a screen dark waits for "Keep these settings" (X on the
//! controller) and goes back by itself after 15 s; kept settings go to screens.json through hearth_settings_io.
//! Night light is desktop.json's (nightLight, nightTemp, nightMode…), put on KWin's Night Light by `hearth-screens
//! night`. Game Mode's HDR is hearth-gamemode-display's; screen power saving hearth-screen-psr's (Intel laptops).

use super::*;
use iced::widget::{column, row};
use hearth_ui::ui::{self, tb, ts, Kind};
use crate::widgets::btn;
use iced::widget::{container, mouse_area, Row};
use serde_json::json;

fn outs(st: &State) -> &[Value] {
    va(st.j("screens"), "outputs")
}
fn kwin(st: &State) -> bool {
    vs(st.j("screens"), "wm") == "kwin"
}
const NEEDS_KWIN: &str = "Needs KWin (KDE Plasma's window manager)";

/// The screen being set: the picked one, else the first that's on.
pub fn cur(st: &State) -> Option<&Value> {
    let o = outs(st);
    o.iter().find(|x| vs(x, "name") == st.picked).or_else(|| o.iter().find(|x| vb(x, "enabled"))).or_else(|| o.first())
}
fn mode_now(o: &Value) -> Option<&Value> {
    va(o, "modes").iter().find(|m| vb(m, "current"))
}
fn name_of(o: &Value) -> String {
    for k in ["model", "description", "name"] {
        let s = vs(o, k);
        if !s.is_empty() {
            return s;
        }
    }
    "Screen".into()
}
/// The resolutions it can show, biggest first, each once.
fn sizes(o: &Value) -> Vec<(i64, i64)> {
    let mut v: Vec<(i64, i64)> = Vec::new();
    for m in va(o, "modes") {
        let s = (vf(m, "width", 0.0) as i64, vf(m, "height", 0.0) as i64);
        if !v.contains(&s) {
            v.push(s);
        }
    }
    v.sort_by(|a, b| (b.0 * b.1).cmp(&(a.0 * a.1)));
    v
}
/// The refresh rates at its current resolution, highest first (whole Hz, each once).
fn rates(o: &Value) -> Vec<i64> {
    let Some(c) = mode_now(o) else { return vec![] };
    let (w, h) = (vf(c, "width", 0.0), vf(c, "height", 0.0));
    let mut v: Vec<i64> = va(o, "modes").iter().filter(|m| vf(m, "width", 0.0) == w && vf(m, "height", 0.0) == h).map(|m| vf(m, "refresh", 0.0).round() as i64).collect();
    v.sort_by(|a, b| b.cmp(a));
    v.dedup();
    v
}
/// The size it takes on the desktop (its mode over its scale, turned for a rotated screen).
fn logical(o: &Value) -> (f64, f64) {
    let m = mode_now(o).or_else(|| va(o, "modes").first());
    let (w, h) = m.map(|m| (vf(m, "width", 1920.0), vf(m, "height", 1080.0))).unwrap_or((1920.0, 1080.0));
    let turned = ["90", "270", "left", "right"].iter().any(|r| vs(o, "transform").contains(r) || vs(o, "rotation") == *r);
    let sc = vf(o, "scale", 1.0).max(0.25);
    if turned { (h / sc, w / sc) } else { (w / sc, h / sc) }
}
fn pos(o: &Value) -> (f64, f64) {
    (vf(o, "position.x", 0.0), vf(o, "position.y", 0.0))
}

/// What a screen shows now, in hearth-screens' words (to go back to it): SetDisplay.qml's entryOf.
fn entry_of(o: &Value, kwin: bool) -> Value {
    let mut e = json!({"enabled": vb(o, "enabled"), "scale": vf(o, "scale", 1.0), "pos": {"x": pos(o).0, "y": pos(o).1}, "rotation": if vs(o, "rotation").is_empty() { "normal".to_string() } else { vs(o, "rotation") }});
    if let Some(m) = mode_now(o) {
        e["mode"] = json!({"w": vf(m, "width", 0.0), "h": vf(m, "height", 0.0), "r": vf(m, "refresh", 0.0)});
    }
    if vb(o, "hdr.capable") {
        e["hdr"] = json!(vb(o, "hdr.on"));
    }
    if vb(o, "vrr.capable") && kwin {
        e["vrr"] = json!(vs(o, "vrr.policy"));
    }
    if vb(o, "wcg.capable") {
        e["wcg"] = json!(vb(o, "wcg.on"));
    }
    if vb(o, "overscan.capable") {
        e["overscan"] = at(o, "overscan.value").clone();
    }
    if vb(o, "rgbRange.capable") {
        e["rgbRange"] = at(o, "rgbRange.value").clone();
    }
    if vb(o, "edr.capable") {
        e["edr"] = at(o, "edr.policy").clone();
    }
    if vb(o, "hdrCal.capable") {
        e["hdrCal"] = json!({"peak": at(o, "hdrCal.peakSet"), "avg": at(o, "hdrCal.avgSet"), "min": at(o, "hdrCal.minSet")});
    }
    e
}

/// hearth-screens answered: the picked screen still there (else the first that's on).
pub fn took(st: &mut State) {
    let name = cur(st).map(|o| vs(o, "name")).unwrap_or_default();
    if st.picked.is_empty() || !outs(st).iter().any(|o| vs(o, "name") == st.picked) {
        st.picked = name;
    }
}

fn screens(args: &[&str]) -> Vec<String> {
    py("hearth-screens", args)
}

/// Change screens now and wait for Keep: a second change while one waits goes back to what was there before both.
pub fn try_change(_sh: &mut Shared, st: &mut State, changes: Value) -> Task<Msg> {
    let k = kwin(st);
    let names: Vec<String> = changes.as_object().map(|m| m.keys().cloned().collect()).unwrap_or_default();
    let mut prev = st.keep.as_ref().map(|k| k.prev.clone()).unwrap_or_else(|| json!({"outputs": {}}));
    for n in &names {
        if at(&prev, &format!("outputs.{n}")).is_null() {
            if let Some(o) = outs(st).iter().find(|o| vs(o, "name") == *n) {
                prev["outputs"][n.as_str()] = entry_of(o, k);
            }
        }
    }
    let all: Vec<String> = prev["outputs"].as_object().map(|m| m.keys().cloned().collect()).unwrap_or_default();
    st.keep = Some(Keep { prev, names: all, left: 15 });
    let arg = json!({"outputs": changes}).to_string();
    exec(Ex::new(screens(&["apply", &arg])).store("screens").fail("That display setting didn't take").secs(40))
}

/// Changes that can't leave a screen dark: put on and kept at once.
pub fn set_now(st: &mut State, changes: Value, label: String) -> Task<Msg> {
    let Some(name) = cur(st).map(|o| vs(o, "name")) else { return Task::none() };
    let mut c = serde_json::Map::new();
    c.insert(name, changes);
    let arg = json!({"outputs": c}).to_string();
    exec(Ex::new(screens(&["set", &arg])).store("screens").ok(label).fail("Couldn't change that"))
}

fn option(o: Value, label: &str) -> Msg {
    Msg::Exec(Ex::new(screens(&["option", &o.to_string()])).store("screens").ok(label).fail("Couldn't change that"))
}

/// Keep: what the waiting screens show now goes into screens.json, through hearth-screens keep.
pub fn keep(st: &mut State) -> Task<Msg> {
    let Some(k) = st.keep.take() else { return Task::none() };
    let names = json!(k.names).to_string();
    exec(Ex::new(screens(&["keep", &names])).store("screens").ok("Display settings kept").fail("Couldn't save the display settings"))
}

/// Go back (the button, or by itself after 15 s: then the page says so, so nothing changes without you knowing).
pub fn revert(sh: &mut Shared, st: &mut State, by_itself: bool) -> Task<Msg> {
    let Some(k) = st.keep.take() else { return Task::none() };
    if by_itself {
        sh.note = "Went back to the display settings you had: the new ones weren't kept within 15 seconds.".into();
    }
    let arg = k.prev.to_string();
    exec(Ex::new(screens(&["apply", &arg])).store("screens").fail("Couldn't put the display back").secs(40))
}

pub fn slider_save(sh: &mut Shared, st: &mut State, k: &'static str) -> Task<Msg> {
    let v = st.num.get(k).copied().unwrap_or(0.0);
    match k {
        "bright" => exec(Ex::new(sv(&["brightnessctl", "-q", "-c", "backlight", "set", &format!("{}%", (v.round() as i64).max(1))])).re("bright").fail("Couldn't change the brightness")),
        "autooff" => {
            st.num.remove(k);
            exec(Ex::new(screens(&["option", &json!({"autoBrightnessOffset": v.round() as i64}).to_string()])).store("screens"))
        }
        "sdr" => {
            st.num.remove(k);
            set_now(st, json!({"sdrBrightness": v.round() as i64}), String::new())
        }
        "gamut" => {
            st.num.remove(k);
            set_now(st, json!({"sdrGamut": v.round() as i64}), String::new())
        }
        "overscan" => {
            st.num.remove(k);
            let Some(name) = cur(st).map(|o| vs(o, "name")) else { return Task::none() };
            let mut c = serde_json::Map::new();
            c.insert(name, json!({"overscan": v.round() as i64}));
            try_change(sh, st, Value::Object(c))
        }
        "barrier" => {
            st.num.remove(k);
            exec(Ex::new(screens(&["option", &json!({"barrier": {"px": v.round() as i64}}).to_string()])).store("screens"))
        }
        "nighttemp" => {
            let t = ((v / 100.0).round() * 100.0) as i64;
            update(sh, st, Msg::Desk(json!({"nightTemp": t}), Ex::new(screens(&["night"])).fail("Couldn't change Night light")))
        }
        "ext" => {
            let who = st.txt("numof").to_string();
            let Some(val) = st.numof.get(&who).copied() else { return Task::none() };
            let via_ddc = ext_list(st).iter().find(|e| e.0 == who).map(|e| e.3).unwrap_or(true);
            exec(Ex::new(screens(&[if via_ddc { "ddc-set" } else { "brightness" }, &who, &format!("{}", val.round() as i64)])).fail("Couldn't change that screen's brightness"))
        }
        _ => Task::none(),
    }
}

/// External screens' brightness: KWin's own control on a newer KWin, else DDC/CI (name, label, value, via DDC).
fn ext_list(st: &State) -> Vec<(String, String, f64, bool)> {
    let mut r: Vec<(String, String, f64, bool)> = Vec::new();
    let internal = |n: &str| n.starts_with("eDP") || n.starts_with("LVDS") || n.starts_with("DSI");
    for o in outs(st) {
        if vb(o, "enabled") && vb(o, "kwinBrightness.capable") && !internal(&vs(o, "name")) {
            r.push((vs(o, "name"), name_of(o), vf(o, "kwinBrightness.value", 0.0), false));
        }
    }
    for d in va(st.j("ddc"), "screens") {
        let n = vs(d, "name");
        if r.iter().any(|x| x.0 == n) {
            continue;
        }
        let label = outs(st).iter().find(|o| vs(o, "name") == n).map(name_of).unwrap_or_else(|| n.clone());
        let max = vf(d, "max", 100.0).max(1.0);
        r.push((n, label, (vf(d, "value", 0.0) * 100.0 / max).round(), true));
    }
    r
}

fn hhmm12(s: &str) -> String {
    let mut it = s.split(':');
    let h: i64 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let m = it.next().unwrap_or("00");
    format!("{}:{m} {}", (h + 11) % 12 + 1, if h < 12 { "AM" } else { "PM" })
}

fn gpu_short(g: &str) -> String {
    g.replace("GeForce ", "").replace("NVIDIA ", "").trim().to_string()
}

pub fn view(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let sh = &a.sh;
    let mut c = crate::widgets::Secs::on(a, 0).spacing(10.0);
    let Some(o) = cur(st) else {
        c = c.push(waiting(a, "screens", "your displays").unwrap_or_else(|| note(&p, "No display was found. Display settings work in Desktop Mode.", false)));
        return c.into();
    };
    let k = kwin(st);
    let name = vs(o, "name");
    let on: Vec<&Value> = outs(st).iter().filter(|x| vb(x, "enabled")).collect();
    let others: Vec<&Value> = on.iter().copied().filter(|x| vs(x, "name") != name).collect();
    let is_main = if k { vf(o, "priority", 0.0) as i64 == 1 } else { on.first().map(|f| vs(f, "name") == name).unwrap_or(true) };
    let gm = st.j("gm");
    let m = mode_now(o);

    // ---- your screens, drawn as they're placed (mockup settings-A2)
    let mut placed: Vec<&Value> = on.clone();
    placed.sort_by(|x, y| pos(x).0.partial_cmp(&pos(y).0).unwrap_or(std::cmp::Ordering::Equal));
    let biggest = placed.iter().map(|x| logical(x).0).fold(1.0f64, f64::max);
    let mut boxes = Row::new().spacing(14.0).align_y(iced::Center);
    for x in &placed {
        let n = on.iter().position(|y| vs(y, "name") == vs(x, "name")).map(|i| i + 1).unwrap_or(1);
        let lit = vs(x, "name") == name;
        let (lw, lh) = logical(x);
        let w = (240.0 * lw / biggest).clamp(90.0, 240.0) as f32 * if lit { 1.0 } else { 0.66 };
        let h = (w as f64 * lh / lw.max(1.0)).clamp(50.0, 150.0) as f32;
        boxes = boxes.push(
            mouse_area(
                container(tb(format!("{n}"), if lit { 34.0 } else { 26.0 }, p.text))
                    .center_x(Length::Fixed(w))
                    .center_y(Length::Fixed(h))
                    .style(ui::boxed(if lit { Pal::mix(p.panel2, p.accent, 0.08) } else { p.panel2 }, p.r(8.0), Some((if lit { p.accent } else { p.line }, 2.0)))),
            )
            .on_press(Msg::Pick(vs(x, "name")))
            .interaction(iced::mouse::Interaction::Pointer),
        );
    }
    if placed.is_empty() {
        boxes = boxes.push(container(tb("1", 26.0, p.muted)).center_x(Length::Fixed(156.0)).center_y(Length::Fixed(88.0)).style(ui::boxed(p.panel2, p.r(8.0), Some((p.line, 2.0)))));
    }
    let hdr_word = if vb(gm, "hdr") { "HDR on in Game Mode" } else { "HDR off" };
    let mut facts: Vec<String> = Vec::new();
    if let Some(m) = m {
        facts.push(format!("{} × {}", vf(m, "width", 0.0), vf(m, "height", 0.0)));
        facts.push(format!("{} Hz", vf(m, "refresh", 0.0).round()));
        facts.push(hdr_word.into());
    } else if !vb(o, "enabled") {
        facts.push("Off".into());
    }
    if !sh.info.gpu.is_empty() {
        facts.push(format!("connected to the {}", gpu_short(&sh.info.gpu)));
    }
    let main_word = if on.len() < 2 || is_main { " · main display" } else { "" };
    // Identify: on KDE Plasma, KDE's own screen numbers (kscreen's OSD); elsewhere Hearth's own, as Mantel's (each
    // screen's number in its top-left corner for 3 s: identify.rs, this app started again, never waited for).
    let identify = if plasma() {
        Some(Msg::Exec(Ex::new(dbus("org.kde.kscreen.osdService", "/org/kde/kscreen/osdService", "org.kde.kscreen.osdService.showOutputIdentifier", &[])).fail("Couldn't show the screens' numbers")))
    } else {
        // Started in the background through sh, so it's nobody's to wait for (no process left behind).
        let mut cmd = vec!["sh".to_string(), "-c".into(), "\"$0\" \"$@\" >/dev/null 2>&1 &".into()];
        cmd.extend(crate::identify::command(&on.iter().map(|x| vs(x, "name")).collect::<Vec<_>>()));
        Some(Msg::Exec(Ex::new(cmd).secs(10).fail("Couldn't show the screens' numbers")))
    };
    let detect = Msg::Exec(Ex::new(screens(&["info"])).store("screens").ok(format!("Looked for screens again: {} {}", outs(st).len(), if outs(st).len() == 1 { "screen" } else { "screens" })));
    let mut pills = Row::new().spacing(8.0).push(pill(&p, "Identify", true, identify)).push(pill(&p, "Detect", false, Some(detect)));
    if !others.is_empty() {
        pills = pills.push(pill(&p, "Arrange", false, Some(Msg::Flag("arrange", !st.on("arrange")))));
    }
    let mut info = column![ts(format!("{}{main_word}", name_of(o)), sz(18.0), p.text), t(facts.join(" · "), sz(13.0), p.muted), pills].spacing(8.0);
    if st.on("arrange") && !others.is_empty() {
        info = info.push(arrange(a, o, others[0]));
    }
    c = c.push(card(&p, row![boxes, info.width(Length::Fill)].spacing(26.0).align_y(iced::Center)));
    if let Some(kp) = &st.keep {
        c = c.push(card(&p, row![
            column![ts("Keep these display settings?", sz(15.0), p.text), t(format!("Going back to the ones you had in {} s.", kp.left), sz(13.0), p.muted)].spacing(2.0).width(Length::Fill),
            btn(&p, "Keep these settings", Kind::Primary, false, Some(Msg::Keep)),
            btn(&p, "Go back", Kind::Normal, false, Some(Msg::Revert))
        ].spacing(10.0).align_y(iced::Center)));
    }
    let problem = vs(st.j("screens"), "problem");
    if !problem.is_empty() {
        c = c.push(note(&p, problem, true));
    }
    if outs(st).len() > 1 {
        let opts: Vec<(String, bool, Msg)> = outs(st).iter().take(4).map(|x| (format!("{}{}", name_of(x).split(' ').take(3).collect::<Vec<_>>().join(" "), if vb(x, "enabled") { "" } else { " (off)" }), vs(x, "name") == name, Msg::Pick(vs(x, "name")))).collect();
        c = c.push(srow(&p, G::Display, "Screen", "The settings below are for this one", choice(&p, opts)));
    }

    // ---- Brightness & colour
    c = c.push(head(&p, "Brightness & colour"));
    if let Some(b) = st.num.get("bright").map(|v| *v as u32).or_else(|| st.raw("bright").trim().split(',').nth(3).and_then(|x| x.trim_end_matches('%').parse().ok())) {
        c = c.push(srow(&p, G::Display, "Brightness", format!("{b}% · only for screens that let Hearth change it"), slider_u32(b.clamp(1, 100), 1..=100, |v| Msg::Num("bright", v as f32), Msg::NumDone("bright"))));
    }
    let sc = st.j("screens");
    if vb(sc, "autoBrightness.capable") {
        let auto_on = vb(sc, "autoBrightness.on");
        c = c.push(srow(&p, G::Display, "Auto-brightness", "The screen follows the light in the room", sw(&p, auto_on, option(json!({"autoBrightness": !auto_on}), if auto_on { "Auto-brightness off" } else { "Auto-brightness on" }))));
        if auto_on {
            let off = st.num.get("autooff").copied().unwrap_or(vf(sc, "autoBrightness.offset", 0.0) as f32);
            let word = if off.round() == 0.0 { "As picked".to_string() } else { format!("{}{}%", if off > 0.0 { "+" } else { "" }, off.round()) };
            c = c.push(srow(&p, G::Display, "How bright", format!("{word} · darker or brighter than Auto-brightness picks for the room"), slider_f32(off, -40.0..=40.0, 5.0, |v| Msg::Num("autooff", v), Msg::NumDone("autooff"))));
            c = c.push(line(&p, if vb(sc, "autoBrightness.paused") { "Auto-brightness is paused: you set the brightness yourself. It follows the room again when the light changes." } else { "Setting the brightness yourself pauses Auto-brightness until the light in the room changes." }));
        }
    }
    let ext = ext_list(st);
    for (n, label, val, ddc) in &ext {
        let v = st.numof.get(n).copied().unwrap_or(*val as f32);
        let who = n.clone();
        c = c.push(srow(&p, G::Display, format!("Brightness · {label}"), if *ddc { "Through the screen's own controls (DDC/CI); the screen keeps it" } else { "Set by KWin" }, slider_f32(v, 0.0..=100.0, 1.0, move |x| Msg::NumOf(who.clone(), x), Msg::NumDone("ext"))));
    }
    if ext.is_empty() && st.has("ddc") && !vb(st.j("ddc"), "available") && outs(st).iter().any(|x| !["eDP", "LVDS", "DSI"].iter().any(|i| vs(x, "name").starts_with(i))) {
        c = c.push(line(&p, format!("External screens' brightness: {}", vs(st.j("ddc"), "why"))));
    }
    let night = sh.d_bool("nightLight", false);
    let mode = { let m = sh.d_str("nightMode"); if m.is_empty() { "times".to_string() } else { m } };
    let from = { let f = sh.d_str("nightFrom"); if f.is_empty() { "19:30".to_string() } else { f } };
    let to = { let f = sh.d_str("nightTo"); if f.is_empty() { "07:00".to_string() } else { f } };
    let place = sh.d_str("nightPlace");
    let night_sub = match mode.as_str() {
        "always" => "Warmer colours all the time".to_string(),
        "location" if !place.is_empty() => format!("Warmer colours from sunset to sunrise in {place}"),
        "location" => "From sunset to sunrise: pick your town below".to_string(),
        _ => format!("Warmer colours from {} to {}", hhmm12(&from), hhmm12(&to)),
    };
    let night_msg = |v: Value| Msg::Desk(v, Ex::new(screens(&["night"])).fail("Couldn't change Night light"));
    c = c.push(srow(&p, G::Moon, "Night light", night_sub, sw(&p, night, night_msg(json!({"nightLight": !night})))));
    let hdr_ok = vb(gm, "screen.hdr") && vb(gm, "has.hdr");
    let hdr_sub = if !st.has("gm") {
        "Brighter, more vivid picture in games and videos that have it".to_string()
    } else if !vb(gm, "screen.hdr") {
        "This screen doesn't have HDR".into()
    } else if !vb(gm, "has.hdr") {
        "This PC's gamescope can't do HDR".into()
    } else {
        format!("Brighter, more vivid picture in games and videos that have it{}", if vb(gm, "hdr") { " · in Game Mode" } else { "" })
    };
    let hdr_now = vb(gm, "hdr");
    c = c.push(srow(&p, G::Display, "HDR", hdr_sub, switch(&p, hdr_now, if hdr_ok { Some(Msg::Exec(Ex::new(py("hearth-gamemode-display", &["set", &json!({"hdr": !hdr_now}).to_string()])).store("gm").ok(format!("{} · from the next time Game Mode starts", if hdr_now { "HDR off" } else { "HDR on" })))) } else { None })));
    // The TV fix (settings-windows-11 display.png; the owner, Nov 27): Mantel puts the kept settings back on a screen
    // that comes back; on unless turned off here (screens.json reconnectRestore, hearth-screens option).
    if st.has("screens") {
        let back = sc.get("reconnectRestore").and_then(Value::as_bool).unwrap_or(true);
        c = c.push(srow(&p, G::Update, "Put my screen settings back when the TV reconnects", "", sw(&p, back, option(json!({"reconnectRestore": !back}), if back { "Screen settings: not put back when a screen reconnects" } else { "Screen settings: put back when a screen reconnects" }))));
    }
    if night {
        let temp = st.num.get("nighttemp").copied().unwrap_or(sh.d_num("nightTemp", 3700.0) as f32);
        // Further right is warmer: the slider runs from 6500 K (left) down to 2000 K (right).
        c = c.push(srow(&p, G::Moon, "How warm", format!("{} K · further right is warmer", (temp / 100.0).round() * 100.0), slider_f32(8500.0 - temp, 2000.0..=6500.0, 100.0, |v| Msg::Num("nighttemp", 8500.0 - v), Msg::NumDone("nighttemp"))));
        c = c.push(srow(&p, G::Clock, "When", "", choice(&p, vec![
            ("Your times".into(), mode == "times", night_msg(json!({"nightMode": "times"}))),
            ("Sunset to sunrise".into(), mode == "location", night_msg(json!({"nightMode": "location"}))),
            ("All the time".into(), mode == "always", night_msg(json!({"nightMode": "always"}))),
        ])));
        if mode == "location" {
            let q = st.txt("place").to_string();
            let search = Msg::Exec(Ex::new(py("hearth-widgets", &["places", q.trim()])).store("places").fail("Couldn't look that up"));
            c = c.push(srow(&p, G::Search, "Your town", if place.is_empty() { "Type your town or city, then Enter".to_string() } else { format!("{place} · type another town, then Enter") }, field(&p, "Your town or city", &q, |s| Msg::Text("place", s)).on_submit(search).width(Length::Fixed(sz(280.0))).into()));
            let found = va(st.j("places"), "places");
            if !found.is_empty() {
                for pl in found.iter().take(4) {
                    let lbl = if vs(pl, "label").is_empty() { vs(pl, "name") } else { vs(pl, "label") };
                    let nm = if vs(pl, "name").is_empty() { lbl.clone() } else { vs(pl, "name") };
                    c = c.push(pick_row(&p, lbl, Msg::Desk(json!({"nightLat": vf(pl, "lat", 0.0), "nightLon": vf(pl, "lon", 0.0), "nightPlace": nm, "nightMode": "location"}), Ex::new(screens(&["night"])).ok("Night light: sunset to sunrise there"))));
                }
            } else if st.has("places") {
                c = c.push(line(&p, if vs(st.j("places"), "error").is_empty() { "No place called that".to_string() } else { vs(st.j("places"), "error") }));
            }
        }
        if mode != "always" {
            let fade = sh.d_num("nightFade", 30.0) as i64;
            c = c.push(srow(&p, G::Clock, "Fade", "How long the colours take to change", choice(&p, [5, 15, 30, 60].iter().map(|v| (format!("{v} min"), fade == *v, night_msg(json!({"nightFade": v})))).collect())));
        }
        if mode == "times" {
            c = c.push(srow(&p, G::Moon, "Starts at", "Night light comes on", choice(&p, ["18:00", "19:00", "19:30", "20:00", "21:00", "22:00"].iter().map(|v| (v.to_string(), from == *v, night_msg(json!({"nightFrom": v})))).collect())));
            c = c.push(srow(&p, G::Display, "Ends at", "Back to normal colours", choice(&p, ["05:00", "06:00", "07:00", "08:00", "09:00"].iter().map(|v| (v.to_string(), to == *v, night_msg(json!({"nightTo": v})))).collect())));
        }
    }

    // ---- Scale & layout (the mockup's)
    c = c.push(head(&p, "Scale & layout"));
    if vb(o, "enabled") {
        let now = (vf(o, "scale", 1.0) * 100.0).round() / 100.0;
        // Four steps as the mockup shows them (100% to 175%; on a big sharp screen 150% to 250%), and the one in use.
        let native_w = sizes(o).first().map(|x| x.0).unwrap_or(0);
        let mut steps: Vec<f64> = if native_w >= 3000 { vec![1.5, 1.75, 2.0, 2.5] } else { vec![1.0, 1.25, 1.5, 1.75] };
        if !steps.iter().any(|s| (s - now).abs() < 0.001) {
            steps.push(now);
            steps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        }
        let mut sc_ch = serde_json::Map::new();
        let opts: Vec<(String, bool, Msg)> = steps
            .iter()
            .map(|s| {
                sc_ch.clear();
                sc_ch.insert(name.clone(), json!({"scale": s}));
                (format!("{}%", (s * 100.0).round()), (s - now).abs() < 0.001, Msg::Try(Value::Object(sc_ch.clone())))
            })
            .collect();
        c = c.push(srow(&p, G::Grid, "Scale", "Text, apps and the bar", choice(&p, opts)));
    }
    let all = sizes(o);
    if !all.is_empty() {
        let top: Vec<(i64, i64)> = all.iter().take(2).copied().collect();
        let now_s = m.map(|m| (vf(m, "width", 0.0) as i64, vf(m, "height", 0.0) as i64));
        let mode_msg = |w: i64, h: i64| -> Msg {
            let best = va(o, "modes").iter().filter(|x| vf(x, "width", 0.0) as i64 == w && vf(x, "height", 0.0) as i64 == h).map(|x| vf(x, "refresh", 0.0)).fold(0.0f64, f64::max);
            let mut ch = serde_json::Map::new();
            ch.insert(name.clone(), json!({"mode": {"w": w, "h": h, "r": best}}));
            Msg::Try(Value::Object(ch))
        };
        let mut opts: Vec<(String, bool, Msg)> = top.iter().map(|(w, h)| (format!("{w} × {h}"), now_s == Some((*w, *h)) && !st.on("otherres"), mode_msg(*w, *h))).collect();
        if let Some(n) = now_s {
            if !top.contains(&n) {
                opts.push((format!("{} × {}", n.0, n.1), !st.on("otherres"), mode_msg(n.0, n.1)));
            }
        }
        if all.len() > opts.len() {
            opts.push(("Other…".into(), st.on("otherres"), Msg::Flag("otherres", !st.on("otherres"))));
        }
        c = c.push(srow(&p, G::Display, "Resolution", format!("Recommended: {} × {}", all[0].0, all[0].1), choice(&p, opts)));
        if st.on("otherres") {
            for (i, (w, h)) in all.iter().take(10).enumerate() {
                let in_use = now_s == Some((*w, *h));
                c = c.push(pick_row(&p, format!("{w} × {h}{}{}", if i == 0 { "  (native)" } else { "" }, if in_use { "  · In use" } else { "" }), mode_msg(*w, *h)));
            }
        }
    }
    let rs = rates(o);
    if rs.len() > 1 {
        let vrr_line = if vb(o, "vrr.capable") && k {
            format!("Variable refresh (FreeSync / G-Sync) is {}", if vs(o, "vrr.policy") == "never" { "off" } else { "on" })
        } else {
            "Higher is smoother; lower saves battery".to_string()
        };
        let now_r = m.map(|m| vf(m, "refresh", 0.0).round() as i64).unwrap_or(0);
        let opts: Vec<(String, bool, Msg)> = rs
            .iter()
            .take(4)
            .map(|r| {
                let (w, h) = m.map(|m| (vf(m, "width", 0.0), vf(m, "height", 0.0))).unwrap_or((0.0, 0.0));
                let exact = va(o, "modes").iter().find(|x| vf(x, "width", 0.0) == w && vf(x, "height", 0.0) == h && vf(x, "refresh", 0.0).round() as i64 == *r).map(|x| vf(x, "refresh", 0.0)).unwrap_or(*r as f64);
                let mut ch = serde_json::Map::new();
                ch.insert(name.clone(), json!({"mode": {"w": w, "h": h, "r": exact}}));
                (format!("{r} Hz"), *r == now_r, Msg::Try(Value::Object(ch)))
            })
            .collect();
        c = c.push(srow(&p, G::Update, "Refresh rate", vrr_line, choice(&p, opts)));
    }
    if vb(o, "enabled") {
        let rot = if vs(o, "rotation").is_empty() { "normal".to_string() } else { vs(o, "rotation") };
        let opts: Vec<(String, bool, Msg)> = [("normal", "Normal"), ("left", "Left"), ("right", "Right"), ("inverted", "Upside down")]
            .iter()
            .map(|(v, l)| {
                let mut ch = serde_json::Map::new();
                ch.insert(name.clone(), json!({"rotation": v}));
                (l.to_string(), rot == *v, Msg::Try(Value::Object(ch)))
            })
            .collect();
        c = c.push(srow(&p, G::Update, "Rotation", "Turn the picture for a screen that's on its side or upside down", choice(&p, opts)));
    }
    // KWin's only (the owner, Nov 28: every button has to work; KWin is retired, Mantel runs on labwc): shown on KWin alone
    if k {
    let x11_self = vb(sc, "x11Scale.self");
    let x11_sub = if !k { NEEDS_KWIN.to_string() } else if x11_self { "Apps draw themselves at your screen's scale: sharp, though a few older ones look small".into() } else { "KWin enlarges them: the right size everywhere, a little blurry on a scaled screen".into() };
    c = c.push(srow(&p, G::Grid, "Scaling X11 apps", x11_sub, if k {
        choice(&p, vec![
            ("Apps scale themselves".into(), x11_self, option(json!({"x11Scale": true}), "X11 apps scale themselves")),
            ("KWin scales them".into(), !x11_self, option(json!({"x11Scale": false}), "KWin scales X11 apps")),
        ])
    } else {
        seg(&p, vec![("Apps scale themselves".into(), x11_self, None), ("KWin scales them".into(), !x11_self, None)])
    }));
    }

    // KWin's only (the owner, Nov 28: every button has to work; KWin is retired, Mantel runs on labwc): shown on KWin alone
    if k {
    // ---- HDR & colour on the desktop (KWin)
    c = c.push(head(&p, "HDR & colour on the desktop"));
    let desk_hdr = vb(o, "hdr.capable");
    let desk_hdr_on = desk_hdr && vb(o, "hdr.on");
    let hdr_change = |on: bool| {
        let mut ch = serde_json::Map::new();
        ch.insert(name.clone(), json!({"hdr": on}));
        Msg::Try(Value::Object(ch))
    };
    c = c.push(srow(&p, G::Display, "HDR on the desktop", if !k { format!("{NEEDS_KWIN} (Game Mode's HDR is the switch above)") } else if !desk_hdr { "This screen doesn't have HDR".into() } else { "Brighter highlights in HDR videos, games and pictures; the desktop itself keeps its colours".into() }, switch(&p, desk_hdr_on, if k && desk_hdr { Some(hdr_change(!desk_hdr_on)) } else { None })));
    if desk_hdr_on {
        let sdr = st.num.get("sdr").copied().unwrap_or(vf(o, "sdrBrightness", 200.0) as f32);
        c = c.push(srow(&p, G::Display, "Brightness of everything else", format!("{} nits · apps, the desktop and videos that aren't HDR, while HDR is on", sdr.round()), slider_f32(sdr, 80.0..=500.0, 10.0, |v| Msg::Num("sdr", v), Msg::NumDone("sdr"))));
        if vb(o, "sdrGamut.capable") {
            let g = st.num.get("gamut").copied().unwrap_or(vf(o, "sdrGamut.value", 0.0) as f32);
            c = c.push(srow(&p, G::Palette, "Colour intensity", format!("{}% · ordinary apps and videos while HDR is on: left is accurate, right more vivid", g.round()), slider_f32(g, 0.0..=100.0, 1.0, |v| Msg::Num("gamut", v), Msg::NumDone("gamut"))));
        }
        if k && vb(o, "hdrCal.capable") {
            let peak = vf(o, "hdrCal.peakSet", 0.0);
            let shown = if peak > 0.0 { peak } else { vf(o, "hdrCal.peak", 0.0) };
            // Hearth's own calibration (cats/kde.rs, as Mantel's HdrCalibrate.qml): no KDE window.
            c = c.push(link_row(&p, G::Display, "Calibrate HDR", format!("Brightest {} nits{} · three steps with test pictures", shown.round(), if peak > 0.0 { " (calibrated)" } else { " (what the screen says)" }), Msg::Go("kde-hdr")));
        }
    }
    if k && vb(o, "edr.capable") {
        let pol = vs(o, "edr.policy");
        let ch = |v: &str| {
            let mut m = serde_json::Map::new();
            m.insert(name.clone(), json!({"edr": v}));
            Msg::Try(Value::Object(m))
        };
        c = c.push(srow(&p, G::Display, "Extended Dynamic Range", "HDR videos and games brighter on this screen by turning its backlight up for them", choice(&p, vec![("Off".into(), pol != "always", ch("never")), ("On".into(), pol == "always", ch("always"))])));
    }
    let wcg_ok = k && vb(o, "wcg.capable");
    let wcg_on = wcg_ok && vb(o, "wcg.on");
    let wcg_msg = {
        let mut m = serde_json::Map::new();
        m.insert(name.clone(), json!({"wcg": !wcg_on}));
        Msg::Try(Value::Object(m))
    };
    c = c.push(srow(&p, G::Palette, "Wide colour", if !k { NEEDS_KWIN.to_string() } else if !wcg_ok { "This screen doesn't have wide colour".into() } else { "Richer colours on screens that show more than sRGB".into() }, switch(&p, wcg_on, if wcg_ok { Some(wcg_msg) } else { None })));
    let icc_ok = k && vb(o, "icc.capable");
    let icc_now = if !icc_ok { String::new() } else if vs(o, "icc.source") == "EDID" { "EDID".into() } else if vs(o, "icc.source") == "ICC" { vs(o, "icc.path") } else { String::new() };
    let mut icc_opts: Vec<(String, String)> = vec![("".into(), "None".into()), ("EDID".into(), "The screen's own".into())];
    for f in va(sc, "iccProfiles").iter().take(4) {
        icc_opts.push((vs(f, "path"), vs(f, "label")));
    }
    if icc_ok {
        c = c.push(chooser(a, "icc", G::Palette, "Colour profile", "For a calibrated screen: an ICC file in ~/.local/share/icc, or the screen's own", icc_opts, &icc_now, |v| {
            let label = if v.is_empty() { "none".to_string() } else if v == "EDID" { "the screen's own".into() } else { v.rsplit('/').next().unwrap_or("").to_string() };
            Msg::SetNow(json!({"icc": v}), format!("Colour profile: {label}"))
        }));
    } else {
        c = c.push(value_row(&p, G::Palette, "Colour profile", if !k { NEEDS_KWIN } else { "KWin can't use a colour profile for this screen" }, "None", false));
    }
    if k && vb(o, "colorPower.capable") {
        let v = vs(o, "colorPower.value");
        c = c.push(srow(&p, G::Palette, "Colour accuracy", "Accurate colours cost a little more power and speed", choice(&p, vec![
            ("Save power".into(), v != "accuracy", Msg::SetNow(json!({"colorPower": "efficiency"}), "Colour accuracy: save power".into())),
            ("Accurate colours".into(), v == "accuracy", Msg::SetNow(json!({"colorPower": "accuracy"}), "Colour accuracy: accurate colours".into())),
        ])));
    }
    }

    // ---- Picture signal (TVs)
    let over_ok = k && vb(o, "overscan.capable");
    let rgb_ok = k && vb(o, "rgbRange.capable");
    if over_ok || rgb_ok {
        c = c.push(head(&p, "Picture signal"));
        if over_ok {
            let ov = st.num.get("overscan").copied().unwrap_or(vf(o, "overscan.value", 0.0) as f32);
            c = c.push(srow(&p, G::Display, "Overscan", format!("{}% · for a TV that cuts off the picture's edges: shrinks the picture to fit", ov.round()), slider_f32(ov, 0.0..=100.0, 1.0, |v| Msg::Num("overscan", v), Msg::NumDone("overscan"))));
        }
        if rgb_ok {
            let v = vs(o, "rgbRange.value");
            let ch = |x: &str| {
                let mut m = serde_json::Map::new();
                m.insert(name.clone(), json!({"rgbRange": x}));
                Msg::Try(Value::Object(m))
            };
            c = c.push(srow(&p, G::Palette, "RGB range", "Washed-out blacks or crushed shadows on a TV? Try Full or Limited", choice(&p, vec![
                ("Automatic".into(), v != "full" && v != "limited", ch("automatic")),
                ("Full".into(), v == "full", ch("full")),
                ("Limited".into(), v == "limited", ch("limited")),
            ])));
        }
    }

    // KWin's only (the owner, Nov 28: every button has to work; KWin is retired, Mantel runs on labwc): shown on KWin alone
    if k {
    // ---- Games and smoothness
    c = c.push(head(&p, "Games and smoothness"));
    let vrr_ok = k && vb(o, "vrr.capable");
    let pol = if vrr_ok { vs(o, "vrr.policy") } else { "never".into() };
    let vrr_ch = |x: &str| {
        let mut m = serde_json::Map::new();
        m.insert(name.clone(), json!({"vrr": x}));
        Msg::Try(Value::Object(m))
    };
    let vrr_sub = if !k { NEEDS_KWIN.to_string() } else if !vrr_ok { "This screen can't follow the game's frame rate".into() } else { "The screen follows the game's frame rate, so there's no tearing or stutter. Automatic: in full-screen games".into() };
    let vrr_items: Vec<(String, bool, Option<Msg>)> = [("never", "Never"), ("automatic", "Automatic"), ("always", "Always")].iter().map(|(v, l)| (l.to_string(), pol == *v, if vrr_ok { Some(vrr_ch(v)) } else { None })).collect();
    c = c.push(srow(&p, G::Pad, "Variable refresh rate (VRR / FreeSync)", vrr_sub, seg(&p, vrr_items)));
    let tear = vb(sc, "tearing.on");
    let tear_ok = k && vb(sc, "tearing.capable");
    c = c.push(srow(&p, G::Pad, "Allow tearing in full-screen games", if !k { NEEDS_KWIN } else { "Less delay in games that ask for it, with a line where frames meet; the desktop never tears" }, switch(&p, tear, if tear_ok { Some(Msg::Exec(Ex::new(screens(&["set", &json!({"tearing": !tear}).to_string()])).store("screens").ok(if tear { "No tearing in games" } else { "Tearing allowed in full-screen games" }))) } else { None })));
    }

    // ---- Several screens
    if outs(st).len() > 1 {
        c = c.push(head(&p, "Several screens"));
        if let Some(other) = others.first() {
            if vb(o, "enabled") {
                c = c.push(arrange(a, o, other));
            }
        }
        if vb(o, "enabled") {
            if k {
            c = c.push(srow(&p, G::Display, "Make this the main screen", if !k { NEEDS_KWIN.to_string() } else if is_main { "This is the main screen: the panel, notifications and new windows are here".into() } else { "Moves the panel, notifications and new windows here".into() }, switch(&p, is_main, if k && !is_main { Some(Msg::SetNow(json!({"priority": 1}), format!("Main screen: {}", name_of(o)))) } else { None })));
            }
        }
        let use_on = vb(o, "enabled");
        let use_msg = if !use_on || !others.is_empty() {
            let mut m = serde_json::Map::new();
            m.insert(name.clone(), json!({"enabled": !use_on}));
            Some(Msg::Try(Value::Object(m)))
        } else {
            None
        };
        c = c.push(srow(&p, G::Display, "Use this screen", if use_on { if others.is_empty() { "On · it's the only screen that's on, so it stays on" } else { "On" } } else { "Off: windows and the panel move to the other screens" }, switch(&p, use_on, use_msg)));
        let bar = st.num.get("barrier").copied().unwrap_or(vf(sc, "barrier.px", 100.0) as f32);
        if k {
        c = c.push(srow(&p, G::Grid, "Sticky edge between screens", if !k { NEEDS_KWIN.to_string() } else { format!("{} px · how far you push before the pointer goes across: 0 goes straight over", bar.round()) }, if k { slider_f32(bar, 0.0..=500.0, 10.0, |v| Msg::Num("barrier", v), Msg::NumDone("barrier")) } else { t1("", 13.0, p.muted).into() }));
        }
        let corner = vb(sc, "barrier.corner");
        if k {
        c = c.push(srow(&p, G::Grid, "Sticky corners", if !k { NEEDS_KWIN } else { "The pointer doesn't slip into the next screen at a corner" }, switch(&p, corner, if k { Some(option(json!({"barrier": {"corner": !corner}}), if corner { "Sticky corners off" } else { "Sticky corners on" })) } else { None })));
        }
        let dm = vs(sc, "desktops.mode");
        if k {
        c = c.push(srow(&p, G::Grid, "Virtual desktops", if !k { NEEDS_KWIN } else if dm == "main" { "Switching desktops changes only the main screen; the others stay as they are" } else { "Switching desktops changes every screen together" }, seg(&p, vec![
            ("Every screen".into(), dm != "main", if k { Some(option(json!({"desktopsMainOnly": false}), "Desktops on every screen")) } else { None }),
            ("Main screen only".into(), dm == "main", if k { Some(option(json!({"desktopsMainOnly": true}), "Desktops on the main screen only")) } else { None }),
            ("Each its own".into(), false, None),
        ])));
        }
        // Neither labwc nor KWin can mirror: a row that can never be used isn't shown (the owner, Nov 28).
        if vb(sc, "mirror.capable") {
        c = c.push(value_row(&p, G::Grid, "Show the same thing on both screens (mirror)", vs(sc, "mirror.why"), "Not available", false));
        }
    }

    // ---- Screen power saving (Intel laptops' Panel Self Refresh)
    let psr = st.j("psr");
    if vb(psr, "supported") {
        c = c.push(head(&p, "Screen power saving"));
        let ch = vs(psr, "choice");
        let set = |v: &str, said: &str| Msg::Exec(Ex::new(adm("hearth-screen-psr", sv(&["set", v]))).store("psr").ok(said).fail("Couldn't change screen power saving"));
        c = c.push(srow(&p, G::Display, "Screen power saving", vs(psr, "line"), choice(&p, vec![
            ("Auto".into(), ch != "on" && ch != "off", set("auto", "Screen power saving: Auto")),
            ("On".into(), ch == "on", set("on", "Screen power saving is on")),
            ("Off".into(), ch == "off", set("off", "Screen power saving is off")),
        ])));
        c = c.push(line(&p, "Lets the built-in screen keep its picture by itself while nothing moves, so the graphics chip can rest. On some Intel laptops it makes the screen freeze or lag; Auto turns it off there."));
    }

    // ---- Related (the mockup's)
    c = c.push(head(&p, "Related"));
    c = c.push(link_row(&p, G::Pad, "Game Mode's resolution and frame limit", "Gaming › Performance", Msg::Go("gameperf")));

    // ---- Advanced: how the Hearth app draws on Nvidia (the same file as the Hearth app's own Settings › Display)
    let nv = st.raw("nvidia").trim().to_string();
    if st.read_ok.get("nvidia").copied().unwrap_or(false) && !nv.is_empty() {
        c = c.push(head(&p, "Advanced"));
        let pick = |v: &'static str| {
            let script = if v == "auto" { "rm -f \"${XDG_CONFIG_HOME:-$HOME/.config}/nimbus/nvidia-drawing\"".to_string() } else { format!("d=\"${{XDG_CONFIG_HOME:-$HOME/.config}}/nimbus\"; mkdir -p \"$d\" && printf %s {v} > \"$d/nvidia-drawing.tmp\" && mv -f \"$d/nvidia-drawing.tmp\" \"$d/nvidia-drawing\"") };
            Msg::Exec(Ex::new(shc(&script)).re("nvidia"))
        };
        c = c.push(srow(&p, G::Display, "Graphics card (Nvidia): how the Hearth app draws", "Fast is smooth on newer drivers. Compatible is slower but works everywhere: pick it if first-time setup or the Hearth app's window shows a black screen. Steam Game Mode isn't affected.", choice(&p, vec![
            ("Automatic".into(), nv == "auto", pick("auto")),
            ("Fast".into(), nv == "fast", pick("fast")),
            ("Compatible".into(), nv == "compatible", pick("compatible")),
        ])));
    }
    c.into()
}

/// "Where this screen is": next to the other one, so the pointer goes across where they meet.
fn arrange<'a>(a: &'a App, o: &Value, other: &Value) -> El<'a> {
    let p = a.sh.p;
    let (px, py_) = pos(o);
    let (qx, qy) = pos(other);
    let (aw, ah) = logical(other);
    let (mw, mh) = logical(o);
    let side = if px >= qx + aw { "right" } else if px + mw <= qx { "left" } else if py_ + mh <= qy { "above" } else if py_ >= qy + ah { "below" } else { "" };
    let name = vs(o, "name");
    let oname = vs(other, "name");
    let place = |w: &str| -> Msg {
        let (x, y) = match w {
            "right" => (qx + aw, qy),
            "left" => (qx - mw, qy),
            "above" => (qx, qy - mh),
            _ => (qx, qy + ah),
        };
        let mut ch = serde_json::Map::new();
        ch.insert(name.clone(), json!({"pos": {"x": x.max(0.0), "y": y.max(0.0)}}));
        if x < 0.0 || y < 0.0 {
            ch.insert(oname.clone(), json!({"pos": {"x": (-x).max(0.0), "y": (-y).max(0.0)}}));
        }
        Msg::Try(Value::Object(ch))
    };
    srow(&p, G::Grid, "Where this screen is", format!("Next to {}, so the pointer goes across where they meet", name_of(other)), choice(&p, ["left", "right", "above", "below"].iter().map(|w| (cap(w), side == *w, place(w))).collect()))
}

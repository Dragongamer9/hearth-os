//! Accessibility (SetAccessibility.qml): the magnifier, colour filters, the screen reader, KWin's typing and mouse
//! help (sticky, bounce, slow and toggle keys, mouse keys, the visual bell, kaccessrc), dwell click, text size, high
//! contrast, a large pointer and reduce motion. Saved by hearth-access in ~/.config/nimbus/accessibility.json (which
//! Game Mode shares), KWin's own extras through hearth-kwin-settings (pers/kwin.rs), as Mantel does.
//! Fedora edition: KDE Plasma runs on KWin, so hearth-access works in its KWin way there (HEARTH_WM=kwin:
//! kit::helper_env) and KWin's rows are live.

use crate::cats::pers::kit::{self, iact, irow, irow_if, said, slide, small, Ic};
use crate::cats::pers::kwin::{self, KMsg, Kw};
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{t1, G};
use iced::{Element, Subscription, Task};
use serde_json::{json, Map, Value};

pub const PAGES: &[&str] = &["accessibility"];

type E<'a> = Element<'a, Msg>;

#[derive(Default)]
pub struct State {
    skip: usize,
    st: Option<Value>,
    pending: Map<String, Value>,
    flushing: bool,
    saving: bool,
    msg: String,
    bad: bool,
    kw: Kw,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Go(&'static str),
    Status(run::Output),
    Change(Value, bool),
    Flush,
    Saved(run::Output),
    Zoom,
    Did(run::Output),
    K(KMsg),
    ReduceMotion(bool),
    Emoji,
    /// Settings' own look: "auto", "always", "never" (look.rs).
    Look(&'static str),
}

pub fn open(sh: &mut Shared, st: &mut State, _page: &'static str) -> Task<Msg> {
    st.msg.clear();
    st.skip = kit::skip_of(&std::mem::take(&mut sh.part));
    Task::batch([Task::perform(kit::run_in(kit::job("hearth-access", &["status"]), String::new()), Msg::Status), st.kw.open("accessibility").map(Msg::K)])
}

fn s(st: &State) -> Value {
    st.st.as_ref().and_then(|x| x.get("settings")).cloned().unwrap_or(json!({}))
}
fn av(st: &State) -> Value {
    st.st.as_ref().and_then(|x| x.get("available")).cloned().unwrap_or(json!({}))
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Go(id) => sh.go(id),
        Msg::Status(o) => match kit::answer(&o) {
            Ok(v) if v.is_object() => st.st = Some(v),
            Ok(_) => {}
            Err(e) => {
                st.msg = e;
                st.bad = true;
            }
        },
        // Shown at once, saved (and put on) by hearth-access half a second later, or now.
        Msg::Change(o, now) => {
            if let Some(x) = st.st.as_mut() {
                if !x["settings"].is_object() {
                    x["settings"] = json!({});
                }
                if let (Some(sv), Some(om)) = (x["settings"].as_object_mut(), o.as_object()) {
                    for (k, v) in om {
                        sv.insert(k.clone(), v.clone());
                    }
                }
            }
            if let Some(om) = o.as_object() {
                for (k, v) in om {
                    st.pending.insert(k.clone(), v.clone());
                }
            }
            if now {
                return update(sh, st, Msg::Flush);
            }
            if !st.flushing {
                st.flushing = true;
                return Task::perform(run::after(500), |_| Msg::Flush);
            }
        }
        Msg::Flush => {
            st.flushing = false;
            let o = std::mem::take(&mut st.pending);
            if o.is_empty() {
                return Task::none();
            }
            st.saving = true;
            let j = Value::Object(o).to_string();
            return Task::perform(kit::run_in(kit::job("hearth-access", &["set", &j]), String::new()), Msg::Saved);
        }
        Msg::Saved(o) => {
            st.saving = false;
            match kit::answer(&o) {
                Ok(r) => {
                    let first = r.get("problems").and_then(Value::as_object).and_then(|p| p.values().next().and_then(Value::as_str).map(String::from));
                    if r.is_object() {
                        st.st = Some(r);
                    }
                    match first {
                        Some(e) => {
                            st.msg = e;
                            st.bad = true;
                        }
                        None => st.msg.clear(),
                    }
                }
                Err(e) => {
                    st.msg = e;
                    st.bad = true;
                }
            }
        }
        Msg::Zoom => return Task::perform(kit::run_in(kit::job("hearth-access", &["magnifier", "in"]), String::new()), Msg::Did),
        Msg::Did(o) => {
            if let Err(e) = kit::answer(&o) {
                st.msg = e;
                st.bad = true;
            }
        }
        Msg::K(k) => {
            if let KMsg::Go(id) = k {
                sh.go(id);
                return Task::none();
            }
            return st.kw.update(k).map(Msg::K);
        }
        Msg::ReduceMotion(on) => {
            let cmd = sh.desktop_cmd(json!({"reduceMotion": on}));
            return Task::perform(run::spawn(cmd), Msg::Did);
        }
        Msg::Emoji => run::detached(&["hearth-emoji"]),
        Msg::Look(v) => match crate::look::save(v) {
            Ok(()) => {
                crate::look::set_current(v);
                crate::nav::set_fixed(!crate::look::auto());
                if let Some(b) = crate::look::fixed(v) {
                    sh.big = b;
                    crate::nav::set_pad_look(b);
                    crate::nav::set_shown(b);
                }
            }
            Err(e) => {
                st.msg = e;
                st.bad = true;
            }
        },
    }
    Task::none()
}

const FILTERS: &[(&str, &str, &str)] = &[
    ("grayscale", "Greyscale", "Shows everything in shades of grey"),
    ("invert", "Inverted colours", "Light becomes dark and colours flip"),
    ("invert-brightness", "Inverted brightness", "Light becomes dark; colours stay much the same"),
    ("protanopia", "Red-green (protanopia)", "Moves reds where they're easier to tell apart"),
    ("deuteranopia", "Green-red (deuteranopia)", "Moves greens where they're easier to tell apart"),
    ("tritanopia", "Blue-yellow (tritanopia)", "Moves blues and yellows where they're easier to tell apart"),
];

fn ch(o: Value, now: bool) -> Msg {
    Msg::Change(o, now)
}
fn k(file: &'static str, group: &str, key: &str, v: Value) -> Msg {
    Msg::K(kwin::set(file, group, key, v))
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<E<'a>> {
    if page != "accessibility" {
        return None;
    }
    let p = a.sh.p;
    let st = &a.a11y;
    let kw = &st.kw;
    let sv = s(st);
    let av = av(st);
    let on_kwin = kit::b_of(&av, "kwin");
    let live = kw.live();
    let note = if live { String::new() } else { ". Needs KWin (KDE Plasma's window manager)".to_string() };
    let ver = kw.ver();
    let needs = |v: u32| -> String { if live && ver > 0 && ver < v { format!(". Needs KWin {}.{} (this is {})", v / 100, v % 100, kw.version()) } else { note.clone() } };
    let can = |v: u32| live && (ver == 0 || ver >= v);
    let mut c = kit::Col::new(st.skip);

    // ------------------------------------------------ magnifier
    c = c.push(head(&p, if st.saving { "Magnifier · saving…" } else { "Magnifier" }));
    if st.st.is_some() && !on_kwin && !kit::b_of(&av, "labwcMagnifier") {
        c = c.push(kit::said(&p, &format!("This labwc ({}) has no magnifier; it came in labwc 0.7.3.", { let v = kit::s_of(&av, "labwc"); if v.is_empty() { "unknown version".to_string() } else { v } }), true));
    }
    c = c.push(iact(&p, Ic::G(G::Search), "Zoom in now", if on_kwin { "Super+= zooms in, Super+- zooms out, Super+0 goes back to normal (KWin's zoom)" } else { "Super+= zooms in, Super+- zooms out (and off), Super+Esc turns it off or back on. The view follows the pointer" }, false, Some(Msg::Zoom)));
    let size = { let x = kit::s_of(&sv, "magnifierSize"); if x.is_empty() { "full".to_string() } else { x } };
    c = c.push(irow(&p, Ic::G(G::Search), "What it magnifies", "", seg(&p, vec![("The whole screen".into(), size == "full", Some(ch(json!({"magnifierSize": "full"}), false))), ("A box around the pointer".into(), size == "lens", Some(ch(json!({"magnifierSize": "lens"}), false)))])));
    let mz = kit::n_of(&sv, "magnifierZoom", 2.0) as f32;
    let (lo, hi) = (1.25f32, 8.0f32);
    c = c.push(irow(&p, Ic::G(G::Search), "Zoom when Super+Esc turns it on", "", slide(&p, (mz - lo) / (hi - lo), 0.0..=1.0, 0.25 / (hi - lo), format!("{}×", trim_num(mz)), move |v| ch(json!({"magnifierZoom": ((lo + v * (hi - lo)) * 4.0).round() / 4.0}), false))));
    let mt = kw.n("kwinrc", "Effect-zoom", "MouseTracking", 0.0) as i64;
    c = c.push(irow_if(&p, live, Ic::Mouse, "The zoomed view follows the pointer", format!("How the view moves when the pointer nears its edge{note}"), seg(&p, [("Smoothly", 0i64), ("Pointer in the middle", 1), ("Pushed at the edge", 2), ("Stays put", 3)].iter().map(|(l, v)| (l.to_string(), mt == *v, if live { Some(k("kwinrc", "Effect-zoom", "MouseTracking", json!(v))) } else { None })).collect())));
    for (key, title, sub) in [("EnableFocusTracking", "Follow the keyboard focus", "The view moves to the window or button you move to with the keyboard"), ("EnableTextCaretTracking", "Follow the text cursor", "The view keeps up with what you type")] {
        let on = kw.b("kwinrc", "Effect-zoom", key, false);
        c = c.push(irow_if(&p, live, Ic::Kbd, title, format!("{sub}{note}"), switch(&p, on, if live { Some(k("kwinrc", "Effect-zoom", key, json!(!on))) } else { None })));
    }
    let mp = kw.n("kwinrc", "Effect-zoom", "MousePointer", 0.0) as i64;
    c = c.push(irow_if(&p, live, Ic::Mouse, "The pointer while zoomed", note.trim_start_matches(". ").to_string(), seg(&p, [("Zoomed too", 0i64), ("Normal size", 1), ("Hidden", 2)].iter().map(|(l, v)| (l.to_string(), mp == *v, if live { Some(k("kwinrc", "Effect-zoom", "MousePointer", json!(v))) } else { None })).collect())));
    let step = kit::n_of(&sv, "magnifierStep", 0.5);
    c = c.push(irow(&p, Ic::G(G::Search), "Each Super+= or Super+- step", "", seg(&p, [("Small", 0.25f64), ("Medium", 0.5), ("Big", 1.0)].iter().map(|(l, v)| (l.to_string(), (step - v).abs() < 0.01, Some(ch(json!({"magnifierStep": v}), false)))).collect())));

    // ------------------------------------------------ colour filters
    c = c.push(head(&p, "Colour filters"));
    let filters = { let f = kit::strs(&av, "filters"); if f.is_empty() { vec!["none".to_string(), "grayscale".into(), "invert".into(), "invert-brightness".into(), "protanopia".into(), "deuteranopia".into(), "tritanopia".into()] } else { f } };
    let desk = { let f = kit::strs(&av, "desktopFilters"); if f.is_empty() && av.get("desktopFilters").is_none() { vec!["none".to_string(), "invert".into()] } else { f } };
    let cur = kit::s_of(&sv, "filter");
    for f in filters.iter().filter(|f| *f != "none") {
        let (name, sub) = FILTERS.iter().find(|x| x.0 == f).map(|x| (x.1.to_string(), x.2.to_string())).unwrap_or((f.clone(), String::new()));
        let here = desk.contains(f);
        let extra = if here { String::new() } else if on_kwin { if f == "grayscale" { ". Needs KWin 6.6 in Desktop Mode".into() } else { ". KWin has no effect for it: Game Mode only".into() } } else if f != "invert-brightness" { ". Not in Desktop Mode on labwc: Mantel on KWin shows it".into() } else { ". Not in Desktop Mode yet: its window manager can't show it".into() };
        let on = cur == *f;
        c = c.push(irow(&p, Ic::G(G::Palette), name, format!("{sub}{extra}"), switch(&p, on, Some(ch(json!({"filter": if on { "none".to_string() } else { f.clone() }}), true)))));
    }
    if on_kwin && ["protanopia", "deuteranopia", "tritanopia", "grayscale"].contains(&cur.as_str()) {
        let fs = sv.get("filterStrength").and_then(Value::as_f64).unwrap_or(1.0) as f32;
        c = c.push(irow(&p, Ic::G(G::Palette), "How strong", "", slide(&p, fs, 0.0..=1.0, 0.05, format!("{}%", (fs * 100.0).round()), |v| ch(json!({"filterStrength": ((v * 20.0).round() / 20.0).max(0.1)}), false))));
    }
    let iw = kit::b_of(&sv, "invertWindow");
    c = c.push(irow_if(&p, live, Ic::G(G::Palette), "Invert one window", format!("Super+Ctrl+U inverts the colours of the window you're in; press it again to turn them back{note}"), switch(&p, iw, if live { Some(ch(json!({"invertWindow": !iw}), true)) } else { None })));
    c = c.push(small(&p, if on_kwin { "Super+Ctrl+I also inverts the colours (KWin's own shortcut)." } else { "In Desktop Mode colours are inverted with the screen's colour ramp, so Night light has to be off for it." }));

    // ------------------------------------------------ screen reader
    c = c.push(head(&p, "Screen reader"));
    let sr = kit::b_of(&sv, "screenReader");
    c = c.push(irow(&p, Ic::G(G::Sound), "Screen reader (Orca)", if st.st.is_none() { "Checking…" } else if !kit::b_of(&av, "orca") { "Orca isn't installed" } else { "Reads out what's on the screen and what you type. Super+Alt+S turns it on or off" }, switch(&p, sr, Some(ch(json!({"screenReader": !sr}), true)))));

    // ------------------------------------------------ typing and the mouse (KWin's kaccessrc)
    c = c.push(head(&p, if kw.saving > 0 { "Typing and the mouse · saving…" } else { "Typing and the mouse" }));
    let kb = |key: &str, d: bool| kw.b("kaccessrc", "Keyboard", key, d);
    let ksw = |p: &hearth_ui::Pal, i: Ic, title: &str, sub: String, group: &str, key: &str, d: bool, usable: bool| -> E<'a> {
        let on = kw.b("kaccessrc", group, key, d);
        irow_if(p, usable, i, title, sub, switch(p, on, if usable { Some(k("kaccessrc", group, key, json!(!on))) } else { None }))
    };
    c = c.push(ksw(&p, Ic::Kbd, "Sticky keys", format!("Press Shift, Ctrl, Alt or Super, then the other key: no need to hold them together{note}"), "Keyboard", "StickyKeys", false, live));
    if kb("StickyKeys", false) {
        c = c.push(kit::indent(ksw(&p, Ic::Lock, "Press twice to lock", "A key pressed twice stays down until pressed again".into(), "Keyboard", "StickyKeysLatch", true, live)));
        c = c.push(kit::indent(ksw(&p, Ic::Kbd, "Turn off when two keys are held together", String::new(), "Keyboard", "StickyKeysAutoOff", false, live)));
    }
    c = c.push(ksw(&p, Ic::G(G::Sound), "A sound for modifier keys", format!("The alert sound plays when Shift, Ctrl, Alt or Super is held, locked or let go with sticky keys{note}"), "Keyboard", "StickyKeysBeep", true, live));
    c = c.push(ksw(&p, Ic::Kbd, "Toggle keys", format!("A sound when Caps Lock, Num Lock or Scroll Lock turns on or off{note}"), "Keyboard", "ToggleKeysBeep", false, live));
    c = c.push(ksw(&p, Ic::Kbd, "Bounce keys", format!("A key pressed again straight away is ignored (for shaky hands){note}"), "Keyboard", "BounceKeys", false, live));
    if kb("BounceKeys", false) {
        let d = kw.n("kaccessrc", "Keyboard", "BounceKeysDelay", 500.0) / 2000.0;
        c = c.push(kit::indent(kwin_slider(&p, live, Ic::G(G::Clock), "Ignore it for", d as f32, format!("{} ms", (d * 2000.0).round()), |v| k("kaccessrc", "Keyboard", "BounceKeysDelay", json!(((v * 2000.0).round() as i64).max(50))))));
    }
    c = c.push(ksw(&p, Ic::Kbd, "Slow keys", format!("A key counts only when held a moment{}", needs(606)), "Keyboard", "SlowKeys", false, can(606)));
    if kb("SlowKeys", false) {
        let d = kw.n("kaccessrc", "Keyboard", "SlowKeysDelay", 500.0) / 2000.0;
        c = c.push(kit::indent(kwin_slider(&p, can(606), Ic::G(G::Clock), "Hold for", d as f32, format!("{} ms", (d * 2000.0).round()), |v| k("kaccessrc", "Keyboard", "SlowKeysDelay", json!(((v * 2000.0).round() as i64).max(50))))));
    }
    c = c.push(ksw(&p, Ic::Mouse, "Mouse keys", format!("Move the pointer with the number keys (5 clicks){}", needs(605)), "Mouse", "MouseKeys", false, can(605)));
    if kw.b("kaccessrc", "Mouse", "MouseKeys", false) {
        let d = kw.n("kaccessrc", "Mouse", "MKMaxSpeed", 30.0) / 100.0;
        c = c.push(kit::indent(kwin_slider(&p, can(605), Ic::Mouse, "Pointer speed with the keys", d as f32, format!("{}", (d * 100.0).round()), |v| k("kaccessrc", "Mouse", "MKMaxSpeed", json!(((v * 100.0).round() as i64).max(5))))));
    }
    c = c.push(ksw(&p, Ic::Bell, "Flash the screen for the alert sound", format!("When an app beeps, the screen flashes too (the visual bell){note}"), "Bell", "VisibleBell", false, live));
    if kw.b("kaccessrc", "Bell", "VisibleBell", false) {
        let inv = kw.b("kaccessrc", "Bell", "VisibleBellInvert", false);
        c = c.push(kit::indent(irow_if(&p, live, Ic::Bell, "Flash", "", seg(&p, vec![("In red".into(), !inv, if live { Some(k("kaccessrc", "Bell", "VisibleBellInvert", json!(false))) } else { None }), ("Invert the screen".into(), inv, if live { Some(k("kaccessrc", "Bell", "VisibleBellInvert", json!(true))) } else { None })]))));
    }
    c = c.push(ksw(&p, Ic::G(G::Sound), "Play the alert sound", note.trim_start_matches(". ").to_string(), "Bell", "SystemBell", true, live));

    // ------------------------------------------------ dwell click
    c = c.push(head(&p, "Dwell click"));
    let dwell_here = live && (st.st.is_none() || !on_kwin || av.get("dwell").and_then(Value::as_bool) != Some(false));
    let dw = kit::b_of(&sv, "dwellClick");
    let dsub = format!("{}{note}", if on_kwin && av.get("dwell").and_then(Value::as_bool) == Some(false) { "Needs python3-evdev for its virtual mouse" } else { "Rest the pointer and a ring fills, then it clicks. Move away or press Esc to stop it" });
    c = c.push(irow_if(&p, dwell_here, Ic::Mouse, "Click by resting the pointer", dsub, switch(&p, dw, if dwell_here { Some(ch(json!({"dwellClick": !dw}), true)) } else { None })));
    if dw {
        let dd = kit::n_of(&sv, "dwellDelay", 1000.0) as f32;
        let (lo, hi) = (300f32, 3000f32);
        let ctl: E<'a> = if dwell_here { slide(&p, (dd - lo) / (hi - lo), 0.0..=1.0, 100.0 / (hi - lo), format!("{:.1} s", dd / 1000.0), move |v| ch(json!({"dwellDelay": (((lo + v * (hi - lo)) / 100.0).round() * 100.0) as i64}), false)) } else { t1(format!("{:.1} s", dd / 1000.0), sz(13.0), p.muted).into() };
        c = c.push(irow_if(&p, dwell_here, Ic::G(G::Clock), "Rest for", "", ctl));
        let ct = { let x = kit::s_of(&sv, "dwellClickType"); if x.is_empty() { "left".to_string() } else { x } };
        c = c.push(irow_if(&p, dwell_here, Ic::Mouse, "Click", "", seg(&p, [("Left", "left"), ("Double", "double"), ("Right", "right"), ("Middle", "middle")].iter().map(|(l, v)| (l.to_string(), ct == *v, if dwell_here { Some(ch(json!({"dwellClickType": v}), false)) } else { None })).collect())));
        let tol = kit::n_of(&sv, "dwellTolerance", 10.0) as i64;
        c = c.push(irow_if(&p, dwell_here, Ic::Mouse, "How still the pointer has to be", "For a shaky hand, let it wander a little further", seg(&p, [("Very still", 5i64), ("Still", 10), ("Loose", 20)].iter().map(|(l, v)| (l.to_string(), tol == *v, if dwell_here { Some(ch(json!({"dwellTolerance": v}), false)) } else { None })).collect())));
    }

    // ------------------------------------------------ text, contrast, pointer
    c = c.push(head(&p, "Text and pointer"));
    let ts_ = kit::n_of(&sv, "textScale", 1.0);
    c = c.push(irow(&p, Ic::Text, "Text size", "For apps and window titles. Apps open now use it after you sign out and in again", seg(&p, [("Default", 1.0f64), ("Larger", 1.15), ("Large", 1.3), ("Largest", 1.5)].iter().map(|(l, v)| (l.to_string(), (ts_ - v).abs() < 0.01, Some(ch(json!({"textScale": v}), true)))).collect())));
    let hc = kit::b_of(&sv, "highContrast");
    c = c.push(irow(&p, Ic::Eye, "High contrast", "Apps with GTK use their high-contrast look (after signing out and in)", switch(&p, hc, Some(ch(json!({"highContrast": !hc}), true)))));
    let lp = kit::b_of(&sv, "largePointer");
    c = c.push(irow(&p, Ic::Mouse, "Large pointer", "A pointer twice the size (after signing out and in)", switch(&p, lp, Some(ch(json!({"largePointer": !lp}), true)))));

    c = c.push(head(&p, "Motion"));
    let rm = a.sh.d_bool("reduceMotion", false);
    c = c.push(irow(&p, Ic::Motion, "Reduce motion", "Still wallpapers and fewer animations (animated ones also hold still on battery and behind fullscreen games)", switch(&p, rm, Some(Msg::ReduceMotion(!rm)))));
    c = c.push(head(&p, "More"));
    // (Controller look in Settings: removed in 1.18, the owner, Nov 23; Settings always has its normal look.)
    c = c.push(iact(&p, Ic::Kbd, "Emoji and symbols", "Super+. opens the picker, in any app", false, Some(Msg::Emoji)));
    if !st.msg.is_empty() {
        c = c.push(said(&p, &st.msg, st.bad));
    }
    if !kw.msg.is_empty() && live {
        c = c.push(said(&p, &kw.msg, kw.msg_bad));
    }
    Some(c.into())
}

fn kwin_slider<'a>(p: &hearth_ui::Pal, on: bool, i: Ic, title: &str, v: f32, value: String, to: impl Fn(f32) -> Msg + 'a) -> E<'a> {
    let ctl: E<'a> = if on { slide(p, v.clamp(0.0, 1.0), 0.0..=1.0, 0.025, value, to) } else { t1(value, sz(13.0), p.muted).into() };
    irow_if(p, on, i, title, "", ctl)
}
fn trim_num(v: f32) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(_a: &'a App) -> Option<(E<'a>, Option<Msg>)> {
    None
}

pub fn back(_st: &mut State) -> bool {
    false
}

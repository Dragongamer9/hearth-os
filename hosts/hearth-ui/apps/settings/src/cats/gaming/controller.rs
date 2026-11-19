//! Gaming › Controller (Mantel's SetController.qml): the controller as a mouse and its pointer speed, button remapping,
//! the controller type games see and the Nintendo layout (through InputPlumber or Handheld Daemon: hearth-controller,
//! the same settings as Hearth's Game Mode), Handheld Daemon's own settings, and the help. On the Fedora edition the
//! controller mouse in KDE Plasma is hearth-pad started by hearth-pad-plasma: the switch stops or starts it for this
//! sign-in (as Mantel's does), and it's on again at the next sign-in.

use crate::cats::apps::kit::{self, *};
use crate::cats::apps::kit::button;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub ctl: Value,
    pub err: String,
    pub busy: bool,
    pub editing: String,
    pub kind: String,
    pub pad_on: Option<bool>,
    pub speed: f32,
    pub help: bool,
    pub say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Go(&'static str),
    Got(Res),
    Run(Vec<String>),
    Edit(String, String),
    Done,
    Pad(Res),
    PadSet(bool),
    PadDone(Res),
    Speed(f32),
    SpeedSet,
    SpeedSaved(Res),
    Help,
    Keyboard,
    KbdDone(Res),
}

/// The pattern for hearth-pad's process (the [d] keeps pgrep from finding the timeout running it).
const PAD: &str = "[d]esktop/shell/hearth-pad";

fn ctl(args: Vec<String>) -> Vec<String> {
    pyv("hearth-controller", args, 60)
}
fn pad_check() -> Task<M> {
    read(prog(vec!["pgrep".into(), "-u".into(), unsafe_uid(), "-f".into(), PAD.into()], 5), M::Pad)
}
fn unsafe_uid() -> String {
    // SAFETY: getuid never fails and touches nothing.
    unsafe { libc::getuid() }.to_string()
}

pub fn open(sh: &mut Shared, st: &mut St) -> Task<M> {
    st.editing.clear();
    st.kind.clear();
    st.say.clear();
    st.speed = sh.d_num("pointerSpeed", 1.0) as f32;
    Task::batch([read(ctl(vec!["status".into()]), M::Got), pad_check()])
}

pub fn back(st: &mut St) -> bool {
    if !st.kind.is_empty() {
        st.kind.clear();
        true
    } else if !st.editing.is_empty() {
        st.editing.clear();
        true
    } else {
        false
    }
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Go(id) => sh.go(id),
        M::Got(r) => {
            st.busy = false;
            if !r.v.is_null() {
                st.err = if gb(&r.v, "available") { String::new() } else { gs(&r.v, "error") };
                st.ctl = r.v;
            } else {
                st.err = r.err;
            }
        }
        M::Run(args) => {
            st.busy = true;
            sh.reread_history = true;
            return read(ctl(args), M::Got);
        }
        M::Edit(id, kind) => {
            st.editing = id;
            st.kind = kind;
        }
        M::Done => {
            st.editing.clear();
            st.kind.clear();
        }
        M::Pad(r) => st.pad_on = Some(r.ok && !r.out.trim().is_empty()),
        M::PadSet(on) => {
            let cmd = if on { prog(vec!["sh".into(), "-c".into(), "/usr/libexec/hearth/hearth-pad-plasma >/dev/null 2>&1 &".into()], 10) } else { prog(vec!["pkill".into(), "-u".into(), unsafe_uid(), "-f".into(), PAD.into()], 10) };
            st.say.ok(if on { "Controller mouse on" } else { "Controller mouse off" });
            return read(cmd, M::PadDone);
        }
        M::PadDone(_) => return Task::perform(hearth_ui::run::after(700), |_| ()).then(|_| pad_check()),
        M::Speed(v) => st.speed = (v * 10.0).round() / 10.0,
        M::SpeedSet => {
            let cmd = sh.desktop_cmd(serde_json::json!({"pointerSpeed": st.speed}));
            return read(prog(cmd, 30), M::SpeedSaved);
        }
        M::SpeedSaved(r) => {
            if !r.ok {
                st.say.bad(format!("Couldn't save the pointer speed: {}", r.err));
            }
        }
        M::Help => st.help = !st.help,
        M::Keyboard => return read(prog(kit::strs(&["busctl", "--user", "call", "org.kde.KWin", "/VirtualKeyboard", "org.kde.kwin.VirtualKeyboard", "forceActivate"]), 10), M::KbdDone),
        M::KbdDone(r) => {
            if !r.ok {
                st.say.bad("The on-screen keyboard couldn't open here (KDE Plasma's virtual keyboard isn't on).");
            }
        }
    }
    Task::none()
}

fn label<'a>(c: &'a Value, id: &str) -> String {
    ga(c, "targets").iter().find(|x| gs(x, "id") == id).map(|x| gs(x, "label")).unwrap_or_else(|| id.to_string())
}
fn mapped(c: &Value, id: &str) -> String {
    gs(go(go(c, "config"), "map"), id)
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let c = &st.ctl;
    let avail = gb(c, "available");
    let mut v: Vec<E<M>> = Vec::new();
    opt(&mut v, say_view(&p, &st.say));
    if !st.editing.is_empty() {
        let back = if st.kind.is_empty() { M::Done } else { M::Edit(st.editing.clone(), String::new()) };
        v.push(r(G::Arrow, "Back", "").on(back).view(&p));
        if st.editing == "@type" {
            v.push(head(&p, "Controller type games see"));
            let cur = gs(go(c, "config"), "target");
            for t in ga(c, "types") {
                let id = gs(t, "id");
                v.push(r(G::Pad, gs(t, "label"), gs(t, "sub")).value(if id == cur { "In use" } else { "" }).on(M::Run(vec!["type".into(), id])).view(&p));
            }
            return page(v);
        }
        let Some(b) = ga(c, "buttons").iter().find(|b| gs(b, "id") == st.editing) else { return page(v) };
        let bl = gs(b, "label");
        v.push(head(&p, match st.kind.as_str() {
            "key" => format!("{bl} types"),
            "button" => format!("{bl} acts as"),
            _ => bl.clone(),
        }));
        if st.kind.is_empty() {
            let id = st.editing.clone();
            v.push(r(G::Update, format!("Default ({bl})"), "").on(M::Run(vec!["set".into(), id.clone(), String::new()])).view(&p));
            v.push(r(G::Pad, "Another controller button…", "").chev().on(M::Edit(id.clone(), "button".into())).view(&p));
            v.push(r(G::Grid, "A keyboard key…", "").chev().on(M::Edit(id.clone(), "key".into())).view(&p));
            v.push(r(G::Close, "Nothing (turn it off)", "").on(M::Run(vec!["set".into(), id, "none".into()])).view(&p));
        } else {
            let cur = mapped(c, &st.editing);
            for t in ga(c, "targets").iter().filter(|t| gs(t, "kind") == st.kind && gs(t, "id") != st.editing) {
                let id = gs(t, "id");
                v.push(r(if st.kind == "key" { G::Grid } else { G::Pad }, gs(t, "label"), "").value(if cur == id { "In use" } else { "" }).on(M::Run(vec!["set".into(), st.editing.clone(), id])).view(&p));
            }
        }
        return page(v);
    }

    // ---- the main list
    let mut mouse: Vec<E<M>> = Vec::new();
    let pad_on = st.pad_on.unwrap_or(false);
    mouse.push(sw(&p, G::Pad, "Controller mouse", "Right stick moves the pointer, R2 clicks. Turns off by itself while a game is full screen.", pad_on, st.pad_on.map(|o| M::PadSet(!o))));
    mouse.push(srow(&p, G::Pad, "Pointer speed", format!("{:.1}× · Hearth's desktop setting; KDE Plasma's controller mouse doesn't use it yet", st.speed), slider_f32(st.speed, 0.4..=2.0, 0.1, M::Speed, M::SpeedSet)));
    v.push(group(&p, "Controller mouse", mouse));

    let mut map: Vec<E<M>> = Vec::new();
    let line = if c.is_null() {
        if st.err.is_empty() { "Loading…".to_string() } else { st.err.clone() }
    } else if !avail {
        format!("Remapping works on Hearth, the OS, with the built-in controller (InputPlumber){}", if st.err.is_empty() { ".".to_string() } else { format!(": {}", st.err) })
    } else {
        let devs: Vec<String> = ga(c, "devices").iter().map(|d| gs(d, "name")).collect();
        format!("Changes apply everywhere, in games too{}", if devs.is_empty() { ".".to_string() } else { format!(" ({}).", devs.join(", ")) })
    };
    map.push(small(&p, line));
    if avail {
        let cur = gs(go(c, "config"), "target");
        let tl = ga(c, "types").iter().find(|x| gs(x, "id") == cur).map(|x| gs(x, "label")).unwrap_or_else(|| "Automatic".into());
        map.push(r(G::Pad, "Controller type games see", "Some games show button prompts to match").value(tl).chev().on(M::Edit("@type".into(), String::new())).view(&p));
        for b in ga(c, "buttons") {
            let id = gs(b, "id");
            let m = mapped(c, &id);
            let val = if m.is_empty() { "Default".to_string() } else if m == "none" { "Off".into() } else { label(c, &m) };
            map.push(r(G::Pad, gs(b, "label"), "").value(val).chev().on(M::Edit(id, String::new())).view(&p));
        }
        map.push(r(G::Pad, "Nintendo layout", "Swaps A with B, and X with Y").on(M::Run(vec!["preset".into(), "nintendo".into()])).view(&p));
        map.push(r(G::Update, "Reset all buttons", "Back to how the controller came").on(M::Run(vec!["reset".into()])).view(&p));
    }
    v.push(group(&p, if st.busy { "Button mapping · applying…" } else { "Button mapping" }, map));
    if gs(c, "backend") == "hhd" {
        v.push(group(&p, "Handheld Daemon", vec![r(G::Gear, "Handheld Daemon settings", "Everything for the built-in controller, power limit, fan and lights: Settings › Handheld").chev().on(M::Go("handheld")).view(&p)]));
    }
    let mut help = vec![r(G::Info, "Controller buttons", "What each button does in Desktop Mode").chev().on(M::Help).view(&p)];
    if st.help {
        let rows = [
            ("Right stick", "Move the pointer (press it in: slower, precise)"),
            ("A / RT", "Click (hold to drag)"),
            ("X / LT", "Right-click"),
            ("Left stick", "Scroll"),
            ("B", "Back (Esc)"),
            ("D-pad", "Arrow keys"),
            ("L3", "Middle-click"),
        ];
        help.push(inside(rows.iter().map(|(k, d)| fact(&p, *k, *d)).collect()));
    }
    help.push(r(G::Grid, "On-screen keyboard", "Y on the controller, or here").on(M::Keyboard).view(&p));
    v.push(group(&p, "Help", help));
    page(v)
}

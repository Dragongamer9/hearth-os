//! Gaming › Family and friends (the owner, Nov 20, 2026; hearth-link's moonlight.rs, docs/HEARTH-LINK.md "Family and
//! friends with Moonlight"). Two halves:
//!
//! - **This PC**: "Let family and friends connect (Moonlight)", off until the person turns it on, and turning it on
//!   first says plainly what it allows (a dialog: see and control the screen, hear the sound). Then who can connect
//!   (each added with a one-time code or its QR code, and removable), per person Ask every time / Always allow, Whole
//!   desktop / Games only and a time limit, who's connected now with Stop sharing now.
//! - **Their PCs**: the PCs that let this one connect (online or not, asked when the page opens), Connect (Moonlight
//!   through Hearth Link) and Remove, and Add with a code.
//!
//! Everything goes through the Hearth Link service (`hearth-link call moonlight.*`): opening the page only reads. The
//! page reads again by itself only while a code is showing (every 2 s, to see the person added) or someone is
//! connected (every 20 s, for the time left).

use crate::cats::apps::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{self, t, tb, G};
use iced::widget::{column, container, qr_code, row, Column};
use iced::{Length, Task};
use serde_json::{json, Value};

#[derive(Default)]
pub struct St {
    pub st: Value,
    pub friends: Value,
    pub err: String,
    pub say: Say,
    pub busy: String,
    /// The "what turning this on allows" dialog is open.
    pub confirm: bool,
    /// The person whose choices are open.
    pub open: String,
    pub code: String,
    pub qr: Option<qr_code::Data>,
    pub qr_for: String,
    pub loaded: bool,
}

#[derive(Debug, Clone)]
pub enum M {
    Got(Res),
    Friends(Res),
    Tick,
    Toggle,
    Confirm(bool),
    Call(&'static str, Value, String),
    Done(String, Res),
    Open(String),
    Code(String),
    Join,
    Connect(String, String),
}

/// `hearth-link call CMD ARGS` (CI's pictures read hearth-link-call-CMD.json instead).
fn link(cmd: &str, args: Value, secs: u32) -> Vec<String> {
    let mut v = vec!["hearth-link".to_string(), "call".into(), cmd.to_string()];
    if !args.is_null() {
        v.push(args.to_string());
    }
    prog(v, secs)
}

fn load() -> Task<M> {
    read(link("moonlight.share.status", Value::Null, 20), M::Got)
}
fn load_friends(check: bool) -> Task<M> {
    read(link("moonlight.friends", if check { json!({"check": true}) } else { Value::Null }, 30), M::Friends)
}

fn err_of(r: &Res) -> String {
    let e = gs(&r.v, "error");
    if e.is_empty() { r.err.clone() } else { e }
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    if hearth_ui::app::arg("--show").as_deref() == Some("family-confirm") {
        st.confirm = true;
    }
    Task::batch([load(), load_friends(true)])
}

/// Reads again by itself only while something on it changes on its own.
pub fn live(st: &St) -> Option<u64> {
    if !go(&st.st, "invite").is_null() {
        Some(2)
    } else if !go(&st.st, "session").is_null() || !go(&st.friends, "connected").is_null() {
        Some(20)
    } else {
        None
    }
}

pub fn back(st: &mut St) -> bool {
    if st.confirm {
        st.confirm = false;
        return true;
    }
    false
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    let _ = sh;
    match m {
        M::Got(r) => {
            st.loaded = true;
            if r.ok {
                st.st = r.v;
                st.err.clear();
                if st.open.is_empty() {
                    st.open = ga(&st.st, "people").first().map(|p| gs(p, "id")).unwrap_or_default();
                }
                let net = gs(go(&st.st, "invite"), "netCode");
                if net != st.qr_for {
                    st.qr = if net.is_empty() { None } else { qr_code::Data::new(net.as_bytes()).ok() };
                    st.qr_for = net;
                }
            } else {
                st.err = err_of(&r);
            }
        }
        M::Friends(r) => {
            if r.ok {
                st.friends = r.v
            }
        }
        M::Tick => {
            if st.busy.is_empty() {
                return Task::batch([load(), if go(&st.st, "invite").is_null() { Task::none() } else { load_friends(false) }]);
            }
        }
        M::Toggle => {
            if gb(&st.st, "on") {
                return update(sh, st, M::Call("moonlight.share.set", json!({"on": false}), "Off. Nobody can connect, and anyone connected was disconnected.".into()));
            }
            st.confirm = true;
        }
        M::Confirm(yes) => {
            st.confirm = false;
            if yes {
                return update(sh, st, M::Call("moonlight.share.set", json!({"on": true}), "On. Add the people who may connect.".into()));
            }
        }
        M::Call(cmd, args, said) => {
            st.busy = cmd.to_string();
            st.say.clear();
            return read(link(cmd, args, 150), move |r| M::Done(said.clone(), r));
        }
        M::Done(said, r) => {
            st.busy.clear();
            if r.ok {
                if !said.is_empty() {
                    st.say.ok(said);
                }
                let note = gs(&r.v, "note");
                if !note.is_empty() {
                    st.say.bad(note);
                }
            } else {
                st.say.bad(err_of(&r));
            }
            return Task::batch([load(), load_friends(true)]);
        }
        M::Open(id) => st.open = if st.open == id { String::new() } else { id },
        M::Code(c) => st.code = c,
        M::Join => {
            let code = st.code.trim().to_string();
            if code.is_empty() {
                st.say.bad("Type the code their PC shows");
                return Task::none();
            }
            st.code.clear();
            // Six digits: the PC on this network showing them (hearth-link finds it); a short code: anywhere.
            let args = json!({"code": code});
            return update(sh, st, M::Call("moonlight.join", args, "Added. Their PC shows up here once it says you may connect.".into()));
        }
        M::Connect(id, name) => {
            return update(sh, st, M::Call("moonlight.connect", json!({"id": id}), format!("Connected to {name}: Moonlight is opening.")));
        }
    }
    Task::none()
}

fn what_word(w: &str) -> &'static str {
    if w == "games" { "Games only" } else { "Whole desktop" }
}
fn limit_word(m: u64) -> String {
    match m {
        0 => "No time limit".into(),
        60 => "1 hour".into(),
        120 => "2 hours".into(),
        180 => "3 hours".into(),
        m => format!("{m} minutes"),
    }
}
fn left(until: f64) -> String {
    if until <= 0.0 {
        return String::new();
    }
    let m = ((until - hearth_ui::time::now() as f64 * 1000.0) / 60_000.0).ceil().max(0.0) as i64;
    if m >= 60 { format!(" · {} h {} min left", m / 60, m % 60) } else { format!(" · {m} min left") }
}

/// The big code card while "Add a person" shows one: 6 digits for the same network, the short code and its QR code
/// for anywhere.
fn code_card<'a>(p: &hearth_ui::Pal, st: &'a St) -> E<'a, M> {
    let inv = go(&st.st, "invite");
    let lan = gs(inv, "code");
    let net = gs(inv, "netCode");
    let mut left_col = Column::new().spacing(10.0);
    left_col = left_col.push(tb("Add a person", sz(17.0), p.text));
    left_col = left_col.push(t("On their PC: Family and friends › Add with a code, then type this. It works once.", sz(13.0), p.muted));
    if !lan.is_empty() {
        let pretty = format!("{} {}", &lan[..3.min(lan.len())], &lan[3.min(lan.len())..]);
        left_col = left_col.push(t("On the same network", sz(12.0), p.muted));
        left_col = left_col.push(tb(pretty, sz(34.0), p.flame));
    }
    if !net.is_empty() {
        left_col = left_col.push(t("Anywhere (read it out, or send the QR code)", sz(12.0), p.muted));
        left_col = left_col.push(tb(net.clone(), sz(28.0), p.text));
    } else if gb(inv, "netError") {
        left_col = left_col.push(t("Only on the same network right now: the code for anywhere couldn't be made", sz(12.0), p.danger));
    }
    left_col = left_col.push(t("Waiting for them… (10 minutes)", sz(12.0), p.muted));
    left_col = left_col.push(kit::buttons(vec![kit::button(p, "Stop", false, Some(M::Call("moonlight.share.invite.close", Value::Null, String::new())))]));
    let mut r = row![left_col.width(Length::Fill)].spacing(20.0).align_y(iced::Center);
    if let Some(d) = &st.qr {
        r = r.push(container(qr_code(d).cell_size(5.0).style(|_t: &iced::Theme| qr_code::Style { cell: iced::Color::BLACK, background: iced::Color::WHITE }))
            .padding(12.0)
            .style(ui::boxed(iced::Color::WHITE, p.r(10.0), None)));
    }
    container(r).padding(20.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), Some((p.flame, 1.0)))).into()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let s = &st.st;
    let on = gb(s, "on");
    let mut v: Vec<E<M>> = Vec::new();
    if !st.loaded {
        v.push(words(&p, "Loading…", false));
    }
    if !st.err.is_empty() {
        v.push(words(&p, if st.err.contains("isn't running") { "Hearth Link isn't running, so this can't be changed right now.".to_string() } else { st.err.clone() }, true));
    }
    // ---- This PC
    v.push(head(&p, "This PC"));
    let sub = if st.busy == "moonlight.share.set" {
        "Working on it…".to_string()
    } else if on {
        "On: the people below can see and control this PC's screen and hear its sound with Moonlight, when you let them".to_string()
    } else {
        "Off. When it's on, the people you add can see and control this PC's screen and hear its sound with Moonlight".to_string()
    };
    v.push(sw(&p, G::Users, "Let family and friends connect (Moonlight)", sub, on, Some(M::Toggle)));
    if s.get("readable") == Some(&Value::Bool(false)) {
        v.push(words(&p, "Your Family and friends settings couldn't be read, so nothing can be changed until that's fixed.", true));
    }
    let sess = go(s, "session");
    if !sess.is_null() {
        let who = gs(sess, "who");
        let line = format!("{}{}", what_word(&gs(sess, "what")), left(gn(sess, "until")));
        v.push(r(G::Display, format!("{who} is connected now"), if gb(sess, "connected") { line } else { format!("Allowed, Moonlight is starting · {line}") }).value("Stop sharing now").danger().on(M::Call("moonlight.stop", Value::Null, "Stopped. They were disconnected.".into())).view(&p));
    } else if gb(s, "asking") {
        v.push(words(&p, "Someone is asking to connect: answer on the question on screen.", false));
    }
    if on {
        let people = ga(s, "people");
        v.push(head(&p, "Who can connect"));
        if people.is_empty() {
            v.push(words(&p, "Nobody yet. Add a person: they type a one-time code on their PC.", false));
        }
        for pe in people {
            let id = gs(pe, "id");
            let name = gs(pe, "name");
            let always = gb(pe, "always");
            let what = gs(pe, "what");
            let lim = gn(pe, "limitMinutes") as u64;
            let sub = format!("{} · {} · {} · {}", if gb(pe, "online") { "Online" } else { "Offline" }, if always { "Always allowed" } else { "Asks every time" }, what_word(&what), limit_word(lim));
            let open = st.open == id;
            v.push(expander(&p, G::User, name.clone(), sub, None, String::new(), open, M::Open(id.clone())));
            if open {
                let set = |k: &str, val: Value| Some(M::Call("moonlight.share.person", json!({"id": id, k: val}), String::new()));
                let mut items: Vec<E<M>> = Vec::new();
                items.push(choice(&p, G::Shield, format!("When {name} connects"), "Asking shows Allow / Deny on this PC's screen", vec![
                    ("Ask every time".into(), !always, set("always", json!(false))),
                    ("Always allow".into(), always, set("always", json!(true))),
                ]));
                items.push(choice(&p, G::Display, "What they get", "Games only starts them in Steam's Big Picture", vec![
                    ("Whole desktop".into(), what != "games", set("what", json!("desktop"))),
                    ("Games only".into(), what == "games", set("what", json!("games"))),
                ]));
                items.push(choice(&p, G::Clock, "Time limit", "The session ends by itself", [0u64, 30, 60, 120, 180].iter().map(|m| (match m { 0 => "None".to_string(), 30 => "30 min".into(), m => format!("{} h", m / 60) }, lim == *m, set("limitMinutes", json!(m)))).collect()));
                items.push(r(G::Close, format!("Remove {name}"), "They can't connect anymore (and Moonlight on their PC is unpaired from this one)").danger().on(M::Call("moonlight.share.remove", json!({"id": id}), format!("Removed {name}."))).view(&p));
                v.push(inside(items));
            }
        }
        if go(s, "invite").is_null() {
            v.push(r(G::Plus, "Add a person", "They get a one-time code to type on their PC (or a QR code to send them). No password").on(M::Call("moonlight.share.invite", Value::Null, String::new())).view(&p));
        } else {
            v.push(code_card(&p, st));
        }
        v.push(small(&p, "No router ports are opened. Moonlight's picture goes through Hearth Link's own connection (direct, or through its relay), encrypted end to end. While someone is connected, a notice with Stop sharing now stays on screen."));
    }
    opt(&mut v, say_view(&p, &st.say));

    // ---- Their PCs
    v.push(head(&p, "Connect to family and friends"));
    let f = &st.friends;
    let list = ga(f, "friends");
    let conn = go(f, "connected");
    if !conn.is_null() {
        v.push(r(G::Display, format!("Connected to {}", gs(conn, "name")), format!("Moonlight is open{}{}", if gb(conn, "relay") { " · through the relay" } else { "" }, left(gn(conn, "until")))).value("Disconnect").on(M::Call("moonlight.disconnect", Value::Null, "Disconnected.".into())).view(&p));
    }
    if list.is_empty() {
        v.push(words(&p, "No PCs yet. When someone adds you on their PC, type the code they give you below.", false));
    }
    for fr in list {
        let id = gs(fr, "id");
        let name = gs(fr, "name");
        let online = gb(fr, "online");
        let sharing = fr.get("sharing").and_then(Value::as_bool);
        let sub = if !online {
            "Offline".to_string()
        } else if sharing == Some(false) {
            "Online · not letting anyone connect right now".to_string()
        } else {
            format!("Online · {}{}", what_word(&gs(fr, "what")), if fr.get("ask").and_then(Value::as_bool) == Some(false) { "" } else { " · they're asked first" })
        };
        let can = online && sharing != Some(false) && conn.is_null() && st.busy.is_empty();
        let row_el = row![
            container(kit::r::<M>(G::Display, name.clone(), sub).view(&p)).width(Length::Fill),
            kit::button(&p, if st.busy == "moonlight.connect" { "Connecting…" } else { "Connect" }, true, if can { Some(M::Connect(id.clone(), name.clone())) } else { None }),
            kit::button(&p, "Remove", false, Some(M::Call("moonlight.forget", json!({"id": id}), format!("{name} is off your list. They can add you again with a code.")))),
        ]
        .spacing(10.0)
        .align_y(iced::Center);
        v.push(row_el.into());
    }
    if !gb(f, "moonlight") && !f.is_null() {
        v.push(r(G::Down, "Moonlight isn't installed", "It shows their PC's screen here: install it in Remote play").chev().on(M::Call("moonlight.friends", json!({"check": true}), String::new())).view(&p));
    }
    v.push(field_on(&p, "Add with a code: type the code their PC shows, then Enter", &st.code, M::Code, M::Join));
    page(v)
}

pub fn dialog<'a>(sh: &'a Shared, st: &'a St) -> Option<(E<'a, M>, Option<M>)> {
    if !st.confirm {
        return None;
    }
    let p = sh.p;
    let point = |g: G, s: &'static str| -> E<'a, M> {
        row![hearth_ui::ui::glyph(g, p.flame, 18.0, 1.8), t(s, sz(15.0), p.text)].spacing(12.0).align_y(iced::Center).into()
    };
    let body = column![
        tb("Let family and friends connect?", sz(20.0), p.text),
        t("With this on, the people you add can use Moonlight to:", sz(14.0), p.muted),
        point(G::Display, "See this PC's screen, whatever is on it"),
        point(G::Mouse, "Control it with their mouse, keyboard or controller"),
        point(G::Sound, "Hear its sound"),
        t("Only people you add with a one-time code can connect. Each time, this PC asks you first (unless you choose Always allow for someone), and while they're connected a notice with Stop sharing now stays on screen. Turning this off ends every session at once. No router ports are opened.", sz(13.0), p.muted),
        kit::buttons(vec![kit::button(&p, "Turn on", true, Some(M::Confirm(true))), kit::button(&p, "Cancel", false, Some(M::Confirm(false)))]),
    ]
    .spacing(14.0);
    Some((ui::dialog_box(&p, 620.0, body), Some(M::Confirm(false))))
}

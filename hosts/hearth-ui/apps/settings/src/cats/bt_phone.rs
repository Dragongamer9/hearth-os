//! Bluetooth & devices › Phone and Messages (Mantel's SetPhone, PhoneRemote and SetPhoneMessages, docs/PHONE-LINK.md):
//! your phone linked to this PC with the free KDE Connect app (Android or iPhone), through Hearth Connect's own service
//! (KDE Connect's protocol). Phone link on or off; pairing (with the key both screens show); your phones with battery
//! and signal; Ring, Ping, clipboard, send files, take a photo, browse the phone's files; send text or a link, what the
//! phone shared; the phone's music; its notifications with Reply; calls (Mute ringer); this PC's keyboard and mouse on
//! the phone; this PC's notifications on the phone and which go; what each phone may do here; Run commands; and its
//! text messages. Through hearth-phone, which only asks the service: every choice is the service's, saved safely by it.
//! The page reads again every few seconds while it's open (pairing requests and calls come in by themselves).

use crate::cats::net::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::run;
use hearth_ui::ui::{self, t, Kind, G};
use crate::widgets::btn;
use iced::keyboard::{key::Named, Event as KEvent, Key};
use iced::widget::canvas::{self, Action, Frame, Geometry};
use iced::widget::{column, container, row, Column, Row, Space};
use iced::{mouse, Length, Point, Rectangle, Renderer, Subscription, Task, Theme};
use serde_json::{json, Value};
use std::io::Write;

const H: &str = "hearth-phone";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum F {
    Send,
    CmdName,
    CmdLine,
    Ip,
    Reply,
    NewTo,
    NewBody,
}

#[derive(Default)]
pub struct S {
    pub st: Option<Value>,
    pub sel: String,
    pub say: Say,
    pub busy: bool,
    pub live: Live,
    pub f: [String; 7],
    pub open: String,
    pub replies: std::collections::HashMap<String, String>,
    pub typing: bool,
    pub said: String,
    sock: Option<std::os::unix::net::UnixStream>,
    seq: u64,
    // messages
    pub threads: Vec<Value>,
    pub messages: Vec<Value>,
    pub thread: i64,
}

#[derive(Debug, Clone)]
pub enum M {
    State(Res),
    Call(&'static str, Value, String),
    Called(String, Res),
    On(bool),
    Turned(bool, Res),
    Sel(String),
    Field(F, String),
    SendText,
    AddCmd,
    AddIp,
    SendFiles,
    Picked(String, run::Output),
    OpenPath(String),
    Open(String),
    ReplyText(String, String),
    Reply(String, String),
    Tick,
    Go(&'static str),
    // the touchpad and keyboard on the phone
    RMove(f32, f32),
    RClick(&'static str),
    RScroll(f32),
    RKey(&'static str),
    Typing,
    Key(KEvent),
    // messages
    Threads(Res),
    Thread(Res),
    OpenThread(i64),
    SmsSend,
    NewSend,
}

impl S {
    fn g(&self, f: F) -> &str {
        &self.f[f as usize]
    }
    fn state(&self) -> &Value {
        self.st.as_ref().map(|v| kit::at(v, "state")).unwrap_or(kit::null())
    }
    fn on(&self) -> bool {
        self.st.as_ref().map(|v| kit::bo(v, "running")).unwrap_or(false)
    }
    fn devices(&self) -> &[Value] {
        kit::ar(self.state(), "devices")
    }
    fn phones(&self) -> Vec<&Value> {
        self.devices().iter().filter(|d| kit::bo(d, "paired")).collect()
    }
    /// The phone shown: the one picked, else one that's connected, else the first.
    fn phone(&self) -> Option<&Value> {
        let ph = self.phones();
        ph.iter().find(|d| kit::st(d, "id") == self.sel).or_else(|| ph.iter().find(|d| kit::bo(d, "reachable"))).or_else(|| ph.first()).copied()
    }
    fn phone_id(&self) -> String {
        self.phone().map(|d| kit::st(d, "id")).unwrap_or_default()
    }
    fn linked(&self) -> bool {
        self.phone().map(|d| kit::bo(d, "reachable")).unwrap_or(false)
    }
    /// Sends one line to Hearth Connect's socket (the touchpad and keys on the phone), as PhoneRemote.qml does.
    fn send(&mut self, cmd: &str, mut args: Value) {
        let id = self.phone_id();
        if id.is_empty() {
            return;
        }
        args["id"] = Value::String(id);
        self.seq += 1;
        let line = format!("{}\n", json!({ "id": self.seq, "cmd": cmd, "args": args }));
        for _ in 0..2 {
            if self.sock.is_none() {
                let path = std::env::var("HEARTH_CONNECT_SOCKET").ok().filter(|p| !p.is_empty()).unwrap_or_else(|| format!("{}/hearth/connect.sock", std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())));
                match std::os::unix::net::UnixStream::connect(&path) {
                    Ok(s) => {
                        let _ = s.set_write_timeout(Some(std::time::Duration::from_millis(200)));
                        self.sock = Some(s);
                    }
                    Err(e) => {
                        self.said = format!("Hearth Connect didn't answer ({e})");
                        return;
                    }
                }
            }
            if let Some(s) = self.sock.as_mut() {
                if s.write_all(line.as_bytes()).is_ok() {
                    self.said.clear();
                    return;
                }
            }
            self.sock = None;
        }
    }
}

fn state() -> Task<M> {
    Task::perform(kit::job(H, kit::a(&["state"]), String::new()), M::State)
}
fn call(cmd: &'static str, args: Value, ok: String) -> Task<M> {
    Task::perform(kit::job(H, vec!["call".into(), cmd.into()], args.to_string()), move |r| M::Called(ok.clone(), r))
}

pub fn open(_sh: &mut Shared, s: &mut S, page: &'static str, part: &str) -> Task<M> {
    s.say.clear();
    s.typing = false;
    if page == "phone-messages" {
        s.thread = part.parse().unwrap_or(-1);
        s.messages.clear();
        s.live.start("phone-messages");
        return state();
    }
    s.live.start("phone");
    state()
}

fn messages_read(s: &S) -> Task<M> {
    let id = s.phone_id();
    if id.is_empty() {
        return Task::none();
    }
    let mut t = vec![Task::perform(kit::job(H, kit::a(&["call", "sms_threads"]), json!({ "id": id }).to_string()), M::Threads)];
    if s.thread >= 0 {
        t.push(Task::perform(kit::job(H, kit::a(&["call", "sms_thread"]), json!({ "id": id, "thread": s.thread }).to_string()), M::Thread));
    }
    Task::batch(t)
}

pub fn update(sh: &mut Shared, s: &mut S, m: M) -> Task<M> {
    match m {
        M::State(r) => {
            let first = s.st.is_none();
            s.st = Some(match r {
                Res::Ok(v) => v,
                Res::Err(e) => json!({ "error": e }),
            });
            if s.live.page == "phone-messages" {
                let id = s.phone_id();
                let t = messages_read(s);
                if first && !id.is_empty() {
                    return Task::batch([t, call("sms_refresh", json!({ "id": id }), String::new())]);
                }
                return t;
            }
        }
        M::Call(cmd, args, ok) => {
            s.busy = true;
            return call(cmd, args, ok);
        }
        M::Called(ok, r) => {
            s.busy = false;
            s.say.res(&r, &ok);
            return state();
        }
        M::On(on) => return Task::perform(kit::job(H, vec![if on { "on" } else { "off" }.into()], String::new()), move |r| M::Turned(on, r)),
        M::Turned(on, r) => {
            match r {
                Res::Ok(v) => {
                    s.st = Some(v);
                    s.say.ok(if on { "Phone link is on" } else { "Phone link is off" });
                }
                Res::Err(e) => {
                    s.say.bad(e);
                    return state();
                }
            }
        }
        M::Sel(id) => s.sel = id,
        M::Field(f, t) => s.f[f as usize] = t,
        M::SendText => {
            let t = s.g(F::Send).trim().to_string();
            if !s.linked() {
                s.say.bad("The phone isn't connected right now");
                return Task::none();
            }
            if !t.is_empty() {
                s.f[F::Send as usize].clear();
                let name = s.phone().map(|d| kit::st(d, "name")).unwrap_or_default();
                return call("send", json!({ "id": s.phone_id(), "text": t }), format!("Sent to {name}"));
            }
        }
        M::AddCmd => {
            let (n, c) = (s.g(F::CmdName).trim().to_string(), s.g(F::CmdLine).trim().to_string());
            if n.is_empty() || c.is_empty() {
                s.say.bad("Type a name and a command");
                return Task::none();
            }
            s.f[F::CmdName as usize].clear();
            s.f[F::CmdLine as usize].clear();
            return call("command_add", json!({ "name": n, "command": c }), format!("Added {n}"));
        }
        M::AddIp => {
            let ip = s.g(F::Ip).trim().to_string();
            if !ip.is_empty() {
                s.f[F::Ip as usize].clear();
                return call("add_ip", json!({ "ip": ip }), format!("Looking for it at {ip}"));
            }
        }
        M::SendFiles => {
            let Some(ph) = s.phone() else { return Task::none() };
            let id = kit::st(ph, "id");
            let title = format!("Send to {}", kit::st(ph, "name"));
            let home = std::env::var("HOME").unwrap_or_default();
            let (q, hq) = (crate::data::q(&title), crate::data::q(&home));
            let cmd = crate::data::sh(&format!("if command -v kdialog >/dev/null; then kdialog --title {q} --multiple --separate-output --getopenfilename {hq}; elif command -v zenity >/dev/null; then zenity --file-selection --multiple --separator='\n' --title={q} --filename={hq}/; else exit 3; fi"));
            return Task::perform(kit::exec(cmd, String::new(), 900), move |o| M::Picked(id.clone(), o));
        }
        M::Picked(id, o) => {
            if o.code == 3 {
                s.say.bad("No file picker installed");
                return Task::none();
            }
            let paths: Vec<String> = o.out.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
            if !paths.is_empty() {
                let n = paths.len();
                return call("share", json!({ "id": id, "paths": paths }), if n == 1 { "Sending 1 file…".into() } else { format!("Sending {n} files…") });
            }
        }
        M::OpenPath(p) => run::detached(&["xdg-open", p.as_str()]),
        M::Open(k) => s.open = if s.open == k { String::new() } else { k },
        M::ReplyText(k, t) => {
            s.replies.insert(k, t);
        }
        M::Reply(k, reply_id) => {
            let t = s.replies.remove(&k).unwrap_or_default();
            if !t.trim().is_empty() {
                return call("reply", json!({ "id": s.phone_id(), "replyId": reply_id, "message": t.trim() }), "Reply sent".into());
            }
        }
        M::Tick => {
            if s.live.still(5) && !s.busy {
                return state();
            }
        }
        M::Go(id) => sh.go(id),
        M::RMove(dx, dy) => s.send("remote_mouse", json!({ "dx": (dx * 10.0).round() / 10.0, "dy": (dy * 10.0).round() / 10.0 })),
        M::RClick(k) => s.send("remote_mouse", json!({ "click": k })),
        M::RScroll(d) => s.send("remote_mouse", json!({ "scroll": d })),
        M::RKey(k) => s.send("remote_key", json!({ "special": k })),
        M::Typing => s.typing = !s.typing,
        M::Key(e) => {
            if !s.typing {
                return Task::none();
            }
            if let KEvent::KeyPressed { key, modifiers, text, .. } = e {
                if key == Key::Named(Named::Escape) {
                    s.typing = false;
                    return Task::none();
                }
                let (shift, ctrl, alt) = (modifiers.shift(), modifiers.control(), modifiers.alt());
                let special = match &key {
                    Key::Named(n) => match n {
                        Named::Backspace => "Backspace",
                        Named::Tab => "Tab",
                        Named::ArrowLeft => "Left",
                        Named::ArrowUp => "Up",
                        Named::ArrowRight => "Right",
                        Named::ArrowDown => "Down",
                        Named::PageUp => "PageUp",
                        Named::PageDown => "PageDown",
                        Named::Home => "Home",
                        Named::End => "End",
                        Named::Enter => "Enter",
                        Named::Delete => "Delete",
                        Named::F1 => "F1",
                        Named::F2 => "F2",
                        Named::F3 => "F3",
                        Named::F4 => "F4",
                        Named::F5 => "F5",
                        Named::F6 => "F6",
                        Named::F7 => "F7",
                        Named::F8 => "F8",
                        Named::F9 => "F9",
                        Named::F10 => "F10",
                        Named::F11 => "F11",
                        Named::F12 => "F12",
                        _ => "",
                    },
                    _ => "",
                };
                if !special.is_empty() {
                    s.send("remote_key", json!({ "special": special, "shift": shift, "ctrl": ctrl, "alt": alt }));
                } else if let Some(tx) = text.as_ref().map(|t| t.to_string()).filter(|t| t.chars().count() == 1 && t.chars().all(|c| c as u32 >= 32)) {
                    s.send("remote_key", json!({ "key": tx, "shift": false, "ctrl": ctrl, "alt": alt }));
                } else if ctrl {
                    if let Key::Character(c) = &key {
                        let c = c.to_lowercase();
                        if c.chars().count() == 1 && c.chars().all(|x| x.is_ascii_lowercase()) {
                            s.send("remote_key", json!({ "key": c, "shift": shift, "ctrl": ctrl, "alt": alt }));
                        }
                    }
                }
            }
        }
        M::Threads(r) => {
            if let Res::Ok(v) = r {
                s.threads = kit::ar(&v, "list").to_vec();
            }
        }
        M::Thread(r) => {
            if let Res::Ok(v) = r {
                s.messages = kit::ar(&v, "list").to_vec();
            }
        }
        M::OpenThread(t) => {
            s.thread = t;
            s.messages.clear();
            return messages_read(s);
        }
        M::SmsSend => {
            let t = s.g(F::Reply).trim().to_string();
            let to = s.threads.iter().find(|x| kit::nu(x, "thread").map(|n| n as i64) == Some(s.thread)).map(|x| kit::at(x, "addresses").clone()).unwrap_or(Value::Null);
            if !t.is_empty() {
                s.f[F::Reply as usize].clear();
                return call("sms_send", json!({ "id": s.phone_id(), "thread": s.thread, "to": to, "text": t }), "Sent".into());
            }
        }
        M::NewSend => {
            let (to, t) = (s.g(F::NewTo).trim().to_string(), s.g(F::NewBody).trim().to_string());
            if to.is_empty() || t.is_empty() {
                s.say.bad("Type a number and a message");
                return Task::none();
            }
            s.f[F::NewBody as usize].clear();
            return call("sms_send", json!({ "id": s.phone_id(), "to": to, "text": t }), format!("Sent to {to}"));
        }
    }
    Task::none()
}

/// The touchpad for the phone: drag to move, click to tap, right-click for a long press, the wheel scrolls.
pub struct Pad {
    pub ready: bool,
}

#[derive(Default)]
pub struct PadState {
    last: Option<Point>,
    pressed: Option<mouse::Button>,
    moved: bool,
    clicked: Option<std::time::Instant>,
}

impl canvas::Program<M> for Pad {
    type State = PadState;
    fn update(&self, st: &mut PadState, event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<Action<M>> {
        if !self.ready {
            return None;
        }
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(b)) if cursor.is_over(bounds) => {
                st.pressed = Some(*b);
                st.last = cursor.position();
                st.moved = false;
                Some(Action::request_redraw().and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let last = st.last?;
                st.pressed?;
                let (dx, dy) = (position.x - last.x, position.y - last.y);
                if dx.abs() + dy.abs() > 0.5 {
                    st.last = Some(*position);
                    if dx.abs() + dy.abs() > 2.0 {
                        st.moved = true;
                    }
                    return Some(Action::publish(M::RMove(dx * 1.6, dy * 1.6)));
                }
                None
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(b)) => {
                let was = st.pressed.take()?;
                if st.moved || was != *b {
                    return Some(Action::request_redraw());
                }
                let kind = match b {
                    mouse::Button::Right => "right",
                    mouse::Button::Middle => "middle",
                    _ => {
                        let dbl = st.clicked.map(|c| c.elapsed().as_millis() < 350).unwrap_or(false);
                        st.clicked = if dbl { None } else { Some(std::time::Instant::now()) };
                        if dbl { "double" } else { "single" }
                    }
                };
                Some(Action::publish(M::RClick(kind)))
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y * 5.0,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 24.0,
                };
                Some(Action::publish(M::RScroll(-y)).and_capture())
            }
            _ => None,
        }
    }
    fn draw(&self, _s: &PadState, renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        vec![Frame::new(renderer, bounds.size()).into_geometry()]
    }
    fn mouse_interaction(&self, _s: &PadState, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if self.ready && cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

fn battery(d: &Value) -> String {
    let c = kit::nu(d, "battery.charge").unwrap_or(-1.0);
    if c < 0.0 {
        return String::new();
    }
    format!("{}%{}", c as i64, if kit::bo(d, "battery.charging") { " charging" } else { "" })
}
fn signal(d: &Value) -> String {
    let Some(sg) = kit::at(d, "plugins.signal").as_object() else { return String::new() };
    let Some(x) = sg.values().next() else { return String::new() };
    let n = kit::nu(x, "signalStrength").map(|n| n.clamp(0.0, 4.0) as usize);
    format!("{}{}", kit::st(x, "networkType"), n.map(|n| format!(" {}", "▂▄▆█".chars().take(n).collect::<String>())).unwrap_or_default())
}
fn phone_line(d: &Value) -> String {
    if !kit::bo(d, "reachable") {
        return "Not connected · it connects by itself when it's on the same Wi-Fi".into();
    }
    let b = battery(d);
    kit::dots(&["Connected".into(), if b.is_empty() { String::new() } else { format!("battery {b}") }, signal(d)])
}
fn when_ms(ms: f64) -> String {
    if ms <= 0.0 {
        return String::new();
    }
    let ts = (ms / 1000.0) as i64;
    let (t, n) = (hearth_ui::time::local(ts), hearth_ui::time::local(hearth_ui::time::now()));
    if t.year == n.year && t.mon == n.mon && t.day == n.day {
        hearth_ui::time::clock(ts)
    } else {
        hearth_ui::time::day(ts)
    }
}

pub fn view<'a>(sh: &'a Shared, s: &'a S, page: &'static str) -> E<'a, M> {
    s.live.seen();
    if page == "phone-messages" {
        return messages(sh, s);
    }
    let p = sh.p;
    let stt = s.state();
    let on = s.on();
    let mut c = Column::new().spacing(10.0);
    c = c.push(words(&p, "Get your phone's notifications and messages here, share files and the clipboard both ways, use the phone as a touchpad or a presentation remote, and more. Install the free KDE Connect app on your Android phone or iPhone, and have both on the same Wi-Fi."));
    let Some(st) = &s.st else { return c.push(words(&p, "Asking Hearth Connect…")).into() };
    let e = kit::st(st, "error");
    if !e.is_empty() {
        c = c.push(note(&p, e, true));
    }
    if kit::at(st, "installed").as_bool() == Some(false) {
        c = c.push(banner(&p, G::Info, "Hearth Connect isn't installed", "It comes with Hearth OS. Updating Hearth puts it back.", true, None));
    }
    let pc_name = kit::st(stt, "name");
    if kit::bo(st, "installed") {
        c = c.push(srow(&p, G::Pad, "Phone link", if on { format!("Phones on your Wi-Fi find this PC as {pc_name}") } else { "Off: this PC doesn't listen for phones".into() }, switch(&p, on, Some(M::On(!on)))));
    }
    c = c.push(said(&p, &s.say));
    if !on {
        return c.into();
    }
    // ---- someone asking to pair, and calls
    for d in s.devices().iter().filter(|d| kit::st(d, "pair.state") == "asking") {
        let (id, name) = (kit::st(d, "id"), kit::st(d, "name"));
        c = c.push(banner(&p, G::Shield, format!("{name} wants to pair"), format!("Check that the phone shows {}, then accept. Only accept phones you know.", kit::st(d, "pair.key")), false, Some(("Accept".into(), M::Call("accept", json!({ "id": id }), format!("Paired with {name}"))))));
        c = c.push(press(&p, G::Close, "Decline", format!("Don't pair with {name}"), Some(M::Call("reject", json!({ "id": id }), "Declined".into()))));
    }
    let phone = s.phone();
    let opts = phone.map(|d| kit::at(d, "options").clone()).unwrap_or(Value::Null);
    let opt = |k: &str| kit::bo(&opts, k);
    if let Some(ph) = phone {
        let call_v = kit::at(ph, "plugins.call");
        if call_v.is_object() {
            let who = kit::st(call_v, "who");
            let title = if kit::st(call_v, "state") == "ringing" { format!("{who} is calling") } else { format!("In a call with {who}") };
            c = c.push(banner(&p, G::Pad, title, format!("On {}{}", kit::st(ph, "name"), if opt("pauseOnCall") { " · what was playing here is paused until the call ends" } else { "" }), false, Some(("Mute ringer".into(), M::Call("mute_call", json!({ "id": kit::st(ph, "id") }), "Ringer muted".into())))));
        }
    }
    // ---- your phones
    c = c.push(head(&p, "Your phones"));
    let phones = s.phones();
    for d in &phones {
        let id = kit::st(d, "id");
        let reach = kit::bo(d, "reachable");
        let shown = phones.len() > 1 && phone.map(|x| kit::st(x, "id") == id).unwrap_or(false);
        let mut rr = Row::new().spacing(10.0).align_y(iced::Center);
        if shown {
            rr = rr.push(t("Shown below", sz(13.0), p.muted));
        }
        let right: E<'a, M> = rr.push(chipk(&p, if reach { "Connected" } else { "Away" }, if reach { "ok" } else { "off" })).into();
        c = c.push(xrow(&p, G::Pad, kit::st(d, "name"), phone_line(d), Some(right), false, Some(M::Sel(id))));
    }
    if phones.is_empty() {
        c = c.push(words(&p, format!("No phone yet. Open KDE Connect on your phone, pick {} and tap Pair. It shows up here to accept.", if pc_name.is_empty() { "this PC".to_string() } else { pc_name.clone() })));
    }
    // ---- the phone shown
    if let Some(ph) = phone {
        let id = kit::st(ph, "id");
        let name = kit::st(ph, "name");
        let linked = kit::bo(ph, "reachable");
        let plug = kit::at(ph, "plugins");
        let sftp = kit::at(plug, "sftp");
        let mounted = kit::st(sftp, "mounted");
        c = c.push(head(&p, name.clone()));
        let en = |m: M| if linked { Some(m) } else { None };
        c = c.push(buttons(&p, vec![
            ("Ring".into(), Kind::Normal, en(M::Call("ring", json!({ "id": id }), format!("{name} is ringing")))),
            ("Ping".into(), Kind::Normal, en(M::Call("ping", json!({ "id": id }), format!("Pinged {name}")))),
            ("Send clipboard".into(), Kind::Normal, en(M::Call("clipboard", json!({ "id": id }), "Clipboard sent".into()))),
            ("Send files…".into(), Kind::Normal, en(M::SendFiles)),
            ("Take a photo".into(), Kind::Normal, en(M::Call("photo", json!({ "id": id }), format!("Take the photo on {name}; it comes to Downloads")))),
            (if mounted.is_empty() { "Browse its files".into() } else { "Open its files".into() }, Kind::Normal, en(if mounted.is_empty() { M::Call("browse", json!({ "id": id }), format!("Asking {name} to share its files… allow it on the phone if it asks")) } else { M::OpenPath(mounted.clone()) })),
        ]));
        c = c.push(input(&p, &format!("Send text or a link to {name} (Enter sends)"), s.g(F::Send), false, |t| M::Field(F::Send, t), Some(M::SendText)));
        // What the phone shared: nothing opens by itself; Copy, or Open for a web link.
        for x in kit::ar(stt, "texts").iter().filter(|x| kit::st(x, "device") == id).take(5) {
            let url = kit::st(x, "url");
            let at = kit::nu(x, "at").unwrap_or(0.0);
            let key = format!("shared-{at}");
            let mut b = Column::new().spacing(6.0);
            if !url.is_empty() {
                b = b.push(press(&p, G::Arrow, "Open", "In your browser", Some(M::Call("open_shared", json!({ "at": at }), "Opened".into()))));
            }
            b = b.push(press(&p, G::Folder, "Copy", "", Some(M::Call("copy_shared", json!({ "at": at }), "Copied".into()))));
            c = c.push(expander(&p, if url.is_empty() { G::List } else { G::Link }, if url.is_empty() { kit::st(x, "text") } else { url.clone() }, format!("{} from {} · {}", if url.is_empty() { "Text" } else { "A link" }, kit::st(x, "from"), hearth_ui::time::clock((at / 1000.0) as i64)), "Shared", None, s.open == key, M::Open(key.clone()), if s.open == key { Some(b.into()) } else { None }));
        }
        if !mounted.is_empty() || kit::bo(sftp, "waiting") || !kit::st(sftp, "error").is_empty() {
            let folders: Vec<String> = kit::ar(sftp, "folders").iter().map(|f| kit::st(f, "name")).collect();
            c = c.push(chip_row(&p, G::Folder, if !mounted.is_empty() { "The phone's files are open" } else if kit::bo(sftp, "waiting") { "Opening the phone's files…" } else { "Couldn't open the phone's files" }, if !mounted.is_empty() { format!("{} · select to open in Files", folders.join(", ")) } else { { let e = kit::st(sftp, "error"); if e.is_empty() { "Allow it on the phone if it asks".to_string() } else { e } } }, if mounted.is_empty() { "" } else { "Open" }, "ok", if mounted.is_empty() { None } else { Some(M::OpenPath(mounted.clone())) }));
            if !mounted.is_empty() {
                c = c.push(press(&p, G::Close, "Close the phone's files", "", Some(M::Call("unmount", json!({ "id": id }), "Closed".into()))));
            }
        }
        // The phone's music: what plays on it, controlled from here.
        for pl in kit::strs(ph, "players").into_iter().take(2) {
            let now = kit::at(ph, "playing").get(&pl).cloned().unwrap_or(Value::Null);
            let playing = kit::bo(&now, "isPlaying");
            let title = { let t1 = kit::st(&now, "title"); if !t1.is_empty() { t1 } else { let n = kit::st(&now, "nowPlaying"); if n.is_empty() { pl.clone() } else { n } } };
            c = c.push(chip_row(&p, G::Sound, title, kit::dots(&[kit::st(&now, "artist"), format!("{pl} on {name}")]), if playing { "Playing" } else { "Paused" }, if playing { "ok" } else { "off" }, None));
            c = c.push(indent(buttons(&p, vec![
                ("Previous".into(), Kind::Normal, Some(M::Call("media", json!({ "id": id, "player": pl, "action": "Previous" }), String::new()))),
                (if playing { "Pause".into() } else { "Play".into() }, Kind::Primary, Some(M::Call("media", json!({ "id": id, "player": pl, "action": "PlayPause" }), String::new()))),
                ("Next".into(), Kind::Normal, Some(M::Call("media", json!({ "id": id, "player": pl, "action": "Next" }), String::new()))),
            ])));
        }
        let threads = kit::nu(plug, "sms.threads").unwrap_or(0.0) as i64;
        c = c.push(go_row(&p, G::List, "Messages", if threads > 0 { format!("{threads} conversations · read them and send a text from here") } else { "Read your text messages and send one from here (Android)".into() }, M::Go("phone-messages")));

        // ---- its notifications, with Reply
        if kit::at(&opts, "notifications").as_bool() != Some(false) {
            let ns = kit::ar(ph, "notifications");
            c = c.push(head(&p, format!("Notifications from {name}")));
            if ns.is_empty() {
                c = c.push(words(&p, if linked { "None right now. On the phone, KDE Connect asks to read notifications the first time: allow it." } else { "They show here while the phone is connected." }));
            } else {
                c = c.push(press(&p, G::Moon, format!("{} on {name}", ns.len()), "Open one to reply, use its buttons or dismiss it (on the phone too)", Some(M::Call("refresh", json!({}), String::new()))));
            }
            for n in ns.iter().take(8) {
                let nid = kit::st(n, "id");
                let key = format!("n-{nid}");
                let reply_id = kit::st(n, "replyId");
                let open = s.open == key;
                let body = if open {
                    let mut b = Column::new().spacing(6.0);
                    if !reply_id.is_empty() {
                        let who = { let t1 = kit::st(n, "title"); if t1.is_empty() { kit::st(n, "appName") } else { t1 } };
                        let k2 = key.clone();
                        b = b.push(input(&p, &format!("Reply to {who}"), s.replies.get(&key).map(|x| x.as_str()).unwrap_or(""), false, move |t| M::ReplyText(k2.clone(), t), Some(M::Reply(key.clone(), reply_id.clone()))));
                    }
                    for a in kit::strs(n, "actions") {
                        b = b.push(press(&p, G::Arrow, a.clone(), "", Some(M::Call("action", json!({ "id": id, "notification": nid, "action": a }), a.clone()))));
                    }
                    if kit::not_false(n, "dismissable") {
                        b = b.push(press(&p, G::Close, "Dismiss", "On the phone too", Some(M::Call("dismiss", json!({ "id": id, "notification": nid }), String::new()))));
                    }
                    Some(b.into())
                } else {
                    None
                };
                c = c.push(expander(&p, G::Moon, kit::dots(&[kit::st(n, "appName"), kit::st(n, "title")]).replacen(" · ", ": ", 1), kit::st(n, "text"), if reply_id.is_empty() { "" } else { "Reply" }, None, open, M::Open(key.clone()), body));
            }
        }

        // ---- this PC on the phone: its keyboard and mouse, its notifications
        c = c.push(head(&p, format!("Use this PC on {name}")));
        c = c.push(remote(&p, s, ph));
        let send_n = opt("sendNotifications");
        c = c.push(srow(&p, G::Moon, format!("Send this PC's notifications to {name}"), if send_n { "What apps here show in their pop-ups shows on the phone too; dismiss it there and it goes here" } else { "Off: this PC's notifications stay here" }, switch(&p, send_n, Some(M::Call("option", json!({ "id": id, "key": "sendNotifications", "on": !send_n }), if send_n { "They stay here".into() } else { "This PC's notifications go to the phone".into() })))));
        if send_n {
            let nout = kit::at(stt, "notifyOut");
            let set = |ch: Value| M::Call("notify_out_set", json!({ "prefs": ch }), "Saved".into());
            let body = kit::not_false(nout, "body");
            c = c.push(srow(&p, G::List, "Include what they say", "Off: only the app and the title go", switch(&p, body, Some(set(json!({ "body": !body }))))));
            let po = kit::bo(nout, "persistentOnly");
            c = c.push(srow(&p, G::Check, "Only ones that stay", "Leaves out pop-ups that go away by themselves", switch(&p, po, Some(set(json!({ "persistentOnly": !po }))))));
            let urg = kit::nu(nout, "urgency").unwrap_or(0.0) as i64;
            c = c.push(choice(&p, G::Info, "Which ones", "By how urgent the app says it is", [(0, "All"), (1, "Normal and urgent"), (2, "Urgent only")].iter().map(|&(v, l)| (l.to_string(), urg == v, Some(set(json!({ "urgency": v }))))).collect()));
            let blocked = kit::strs(nout, "blocked");
            let mut apps = kit::strs(stt, "notifyApps");
            for b in &blocked {
                if !apps.contains(b) {
                    apps.push(b.clone());
                }
            }
            for a in apps {
                let goes = !blocked.contains(&a);
                let nb: Vec<String> = if goes { blocked.iter().cloned().chain([a.clone()]).collect() } else { blocked.iter().filter(|x| **x != a).cloned().collect() };
                c = c.push(srow(&p, G::Grid, a, "Its notifications go to the phone", switch(&p, goes, Some(set(json!({ "blocked": nb }))))));
            }
        }

        // ---- what the phone may do
        c = c.push(head(&p, format!("What {name} can do")));
        let folder = kit::st(stt, "folder");
        let rows: Vec<(&str, G, String, String)> = vec![
            ("notifications", G::Moon, "Show its notifications here".into(), "With Reply for messages".into()),
            ("clipboard", G::Folder, "Share the clipboard".into(), "Copy on one, paste on the other".into()),
            ("input", G::Grid, "Touchpad, keyboard and presentation remote".into(), "The phone moves the pointer and types here".into()),
            ("pauseOnCall", G::Stop, "Pause media during calls".into(), "Plays again when the call ends".into()),
            ("commands", G::Rocket, "Run commands".into(), "Only the commands below".into()),
            ("volume", G::Sound, "Control this PC's volume".into(), "Its volume slider and mute".into()),
            ("lock", G::Shield, "Lock this PC".into(), "From the phone's Lock button. Unlocking always needs your password here".into()),
            ("muteOnCall", G::Sound, "Mute this PC's sound during calls".into(), "On again when the call ends".into()),
            ("pauseTalkingOnly", G::Pad, "Only once I answer".into(), "Pause and mute when you answer, not while it rings".into()),
            ("stayAwake", G::Power, "Keep this PC awake while it's connected".into(), format!("No screen-off or sleep while {name} is linked")),
            ("autoAccept", G::Folder, "Save its files without asking".into(), format!("Into {}", if folder.is_empty() { "Downloads".to_string() } else { folder.clone() })),
        ];
        for (k, g, title, sub) in rows {
            let v = opt(k);
            c = c.push(srow(&p, g, title.clone(), sub, switch(&p, v, Some(M::Call("option", json!({ "id": id, "key": k, "on": !v }), format!("{title}: {}", if v { "off" } else { "on" }))))));
        }
    }

    // ---- run commands
    c = c.push(head(&p, "Run commands"));
    c = c.push(words(&p, "Commands your phone can start with a tap (KDE Connect › Run command). Only these: the phone can't send its own."));
    for cm in kit::ar(stt, "commands") {
        let key = format!("cmd-{}", kit::st(cm, "key"));
        let name = kit::st(cm, "name");
        let body = if s.open == key { Some(danger_row(&p, G::Close, format!("Remove {name}"), "", Some(M::Call("command_remove", json!({ "key": kit::st(cm, "key") }), "Removed".into())))) } else { None };
        c = c.push(expander(&p, G::Rocket, name, kit::st(cm, "command"), "", None, s.open == key, M::Open(key.clone()), body));
    }
    c = c.push(input(&p, "New command's name (Lock the screen)", s.g(F::CmdName), false, |t| M::Field(F::CmdName, t), None));
    c = c.push(input(&p, "Its command (loginctl lock-session)", s.g(F::CmdLine), false, |t| M::Field(F::CmdLine, t), Some(M::AddCmd)));
    c = c.push(container(btn(&p, "Add command", Kind::Normal, false, Some(M::AddCmd))));

    // ---- pair a new phone
    c = c.push(head(&p, "Pair a new phone"));
    for d in s.devices().iter().filter(|d| !kit::bo(d, "paired") && kit::bo(d, "reachable") && kit::st(d, "pair.state") != "asking") {
        let name = kit::st(d, "name");
        let sub = if kit::st(d, "pair.state") == "requested" { format!("Waiting for the phone… check it shows {}", kit::st(d, "pair.key")) } else { "Select to pair".into() };
        c = c.push(press(&p, G::Pad, name.clone(), sub, Some(M::Call("pair", json!({ "id": kit::st(d, "id") }), format!("Asked {name}: accept on the phone")))));
    }
    c = c.push(info_row(&p, G::Info, "Phone not listed?", "Open KDE Connect on the phone (same Wi-Fi), or add this PC there by its address. Some networks (guest Wi-Fi) keep devices apart."));
    c = c.push(input(&p, "Or type the phone's address (192.168.1.23)", s.g(F::Ip), false, |t| M::Field(F::Ip, t), Some(M::AddIp)));
    let folder = kit::st(stt, "folder");
    if !folder.is_empty() {
        c = c.push(go_row(&p, G::Folder, "Files from your phone", folder.clone(), M::OpenPath(folder.clone())));
    }
    if let Some(ph) = phone {
        c = c.push(danger_row(&p, G::Close, format!("Forget {}", kit::st(ph, "name")), "Unpairs it. Pair again from the phone to use it here", Some(M::Call("forget", json!({ "id": kit::st(ph, "id") }), "Forgot it".into()))));
    }
    c.into()
}

/// This PC's keyboard and mouse on the phone (PhoneRemote.qml): a touchpad and typing, through Hearth Connect's socket.
fn remote<'a>(p: &hearth_ui::Pal, s: &'a S, ph: &Value) -> E<'a, M> {
    let p = *p;
    let reach = kit::bo(ph, "reachable");
    let ri = kit::bo(ph, "remoteInput");
    let kb = kit::bo(ph, "keyboard");
    let ready = reach && ri;
    let name = kit::st(ph, "name");
    let line = if !reach {
        "Works while the phone is connected".to_string()
    } else if !ri {
        "On the phone: KDE Connect › this PC › turn on Remote keyboard and Mouse receiver".into()
    } else if !s.said.is_empty() {
        s.said.clone()
    } else if kb {
        "The phone's KDE Connect keyboard is on: what you type goes into the field it has open".into()
    } else {
        "Pick \"KDE Connect Remote Keyboard\" as the keyboard on the phone to type there".into()
    };
    let mut top = Row::new().spacing(12.0).align_y(iced::Center).push(hearth_ui::ui::glyph(G::Grid, p.text, 22.0, 1.8)).push(column![hearth_ui::ui::tm(format!("This PC's keyboard and mouse on {name}"), sz(15.0), p.text), t(line, sz(13.0), if s.said.is_empty() { p.muted } else { p.danger })].spacing(2.0).width(Length::Fill));
    if ready {
        top = top.push(chipk(&p, if kb { "Keyboard ready" } else { "Pointer only" }, if kb { "ok" } else { "off" }));
    }
    let label = if ready { format!("Touchpad for {name}\nDrag to move · click to tap · right-click for a long press · wheel to scroll") } else { "Touchpad".into() };
    let surface = container(t(label, sz(13.0), p.muted).align_x(iced::Center)).center_x(Length::Fill).center_y(Length::Fixed(200.0)).style(ui::boxed(Pal::mix(p.panel, p.bg, 0.5), p.r(10.0), Some((Pal::a(p.text, 0.08), 1.0))));
    let pad = iced::widget::stack![surface, canvas::Canvas::new(Pad { ready }).width(Length::Fill).height(Length::Fixed(200.0))];
    let en = |m: M| if ready { Some(m) } else { None };
    let keys = buttons(&p, vec![
        (if s.typing { "Typing on the phone… (Esc stops)".into() } else { "Type on the phone".into() }, if s.typing { Kind::Primary } else { Kind::Normal }, en(M::Typing)),
        ("Esc".into(), Kind::Normal, en(M::RKey("Escape"))),
        ("Enter".into(), Kind::Normal, en(M::RKey("Enter"))),
        ("Back space".into(), Kind::Normal, en(M::RKey("Backspace"))),
    ]);
    let body = column![top, pad, keys].spacing(10.0);
    container(body).padding(16.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), Some((if s.typing { p.accent } else { Pal::a(p.text, 0.05) }, if s.typing { 2.0 } else { 1.0 })))).into()
}

use hearth_ui::Pal;

fn messages<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let mut c = Column::new().spacing(10.0);
    let Some(ph) = s.phone() else {
        return c.push(words(&p, if s.st.is_none() { "Asking Hearth Connect…" } else { "Pair your phone first (Settings › Phone)." })).push(said(&p, &s.say)).into();
    };
    c = c.push(said(&p, &s.say));
    let th = s.threads.iter().find(|x| kit::nu(x, "thread").map(|n| n as i64) == Some(s.thread));
    let names = |x: &Value| { let n = kit::strs(x, "names"); if n.is_empty() { kit::strs(x, "addresses").join(", ") } else { n.join(", ") } };
    if let Some(t0) = th {
        // ---- one conversation
        c = c.push(head(&p, names(t0)));
        c = c.push(press(&p, G::Arrow, "All conversations", "", Some(M::OpenThread(-1))));
        let n = s.messages.len();
        for m in s.messages.iter().skip(n.saturating_sub(30)) {
            let sent = kit::bo(m, "sent");
            let (bg, fg) = if sent { (p.accent, p.ink) } else { (p.panel, p.text) };
            let bubble = container(column![t(kit::st(m, "body"), sz(14.0), fg), t(when_ms(kit::nu(m, "date").unwrap_or(0.0)), sz(11.0), Pal::a(if sent { p.ink } else { p.muted }, 0.8))].spacing(2.0))
                .padding(iced::Padding { top: 10.0, right: 14.0, bottom: 10.0, left: 14.0 })
                .max_width(560.0)
                .style(ui::boxed(bg, p.r(14.0), if sent { None } else { Some((Pal::a(p.text, 0.05), 1.0)) }));
            c = c.push(if sent { row![Space::new().width(Length::Fill), bubble] } else { row![bubble, Space::new().width(Length::Fill)] });
        }
        c = c.push(input(&p, "Text message", s.g(F::Reply), false, |t| M::Field(F::Reply, t), Some(M::SmsSend)));
        return c.into();
    }
    // ---- every conversation
    c = c.push(head(&p, format!("Conversations on {}", kit::st(ph, "name"))));
    if s.threads.is_empty() {
        c = c.push(words(&p, if kit::bo(ph, "reachable") { "Loading… On the phone, KDE Connect asks to read your messages the first time: allow it." } else { "They show here while the phone is connected." }));
    }
    for x in s.threads.iter().take(40) {
        let unread = kit::nu(x, "unread").unwrap_or(0.0) as i64;
        let mut right = Row::new().spacing(10.0).align_y(iced::Center);
        if unread > 0 {
            right = right.push(chipk(&p, format!("{unread} new"), "accent"));
        }
        right = right.push(t(when_ms(kit::nu(x, "date").unwrap_or(0.0)), sz(13.0), p.muted));
        let tid = kit::nu(x, "thread").unwrap_or(-1.0) as i64;
        c = c.push(xrow(&p, G::List, names(x), format!("{}{}", if kit::bo(x, "sent") { "You: " } else { "" }, kit::st(x, "last")), Some(right.into()), false, Some(M::OpenThread(tid))));
    }
    c = c.push(head(&p, "New message"));
    c = c.push(input(&p, "To: phone number", s.g(F::NewTo), false, |t| M::Field(F::NewTo, t), None));
    c = c.push(input(&p, "Message", s.g(F::NewBody), false, |t| M::Field(F::NewBody, t), Some(M::NewSend)));
    c.into()
}

pub fn subscription(s: &S) -> Subscription<M> {
    let mut v = Vec::new();
    if s.live.running("phone") || s.live.running("phone-messages") {
        v.push(iced::time::every(std::time::Duration::from_secs(5)).map(|_| M::Tick));
    }
    if s.typing {
        v.push(iced::keyboard::listen().map(M::Key));
    }
    Subscription::batch(v)
}

pub fn back(s: &mut S) -> bool {
    if s.typing {
        s.typing = false;
        return true;
    }
    if s.live.page == "phone-messages" && s.thread >= 0 && s.live.running("phone-messages") {
        s.thread = -1;
        s.messages.clear();
        return true;
    }
    false
}

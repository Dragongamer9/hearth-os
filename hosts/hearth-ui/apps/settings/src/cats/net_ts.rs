//! Network & internet › Tailscale (Mantel's SetHomeTailscale, docs/HOME-SERVER.md): a private network for your devices
//! only. Sign in with a QR code or a link, this PC's name and address, your devices with their status and ping (select
//! one to send it a file, Taildrop), "Only reachable through Tailscale" for the shares, start at boot, the exit node
//! (off), Taildrop and waking this PC from away. All through Hearth Home server's own helper (hearth-home-server),
//! which saves its settings safely; sign-in links and keys are never written to a log.

use super::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{t, G};
use iced::widget::{column, container, image, Column};
use iced::{Length, Subscription, Task};
use serde_json::Value;

#[derive(Default)]
pub struct S {
    pub st: Option<Value>,
    pub login: Option<Value>,
    pub qr: Option<image::Handle>,
    pub pings: Value,
    pub say: Say,
    pub busy: bool,
    pub live: Live,
}

#[derive(Debug, Clone)]
pub enum M {
    Status(Res),
    Done(String, Res),
    Login(Res),
    Pings(Res),
    Start,
    SignIn,
    SignOut,
    Set(&'static str, bool, String),
    Send(String),
    Sent(Res),
    Tick,
    PingTick,
}

const H: &str = "hearth-home-server";

fn status() -> Task<M> {
    Task::perform(kit::job(H, kit::a(&["status"]), String::new()), M::Status)
}
fn ping() -> Task<M> {
    Task::perform(kit::job(H, kit::a(&["ts-ping"]), String::new()), M::Pings)
}

impl S {
    fn ts(&self) -> &Value {
        self.st.as_ref().map(|v| kit::at(v, "tailscale")).unwrap_or(kit::null())
    }
    fn connected(&self) -> bool {
        kit::st(self.ts(), "state") == "Running"
    }
}

pub fn open(_sh: &mut Shared, s: &mut S, _part: &str) -> Task<M> {
    s.say.clear();
    s.live.start("hs-tailscale");
    status()
}

pub fn update(_sh: &mut Shared, s: &mut S, m: M) -> Task<M> {
    match m {
        M::Status(r) => {
            let was = s.connected();
            match r {
                Res::Ok(v) => s.st = Some(v),
                Res::Err(e) => s.st = Some(serde_json::json!({ "error": e })),
            }
            if s.connected() {
                s.login = None;
                s.qr = None;
                if !was {
                    return ping();
                }
            }
        }
        M::Done(ok, r) => {
            s.busy = false;
            s.say.res(&r, &ok);
            // A change's notes from the system (the firewall, the service), as Mantel shows them.
            if let Res::Ok(v) = &r {
                let notes = kit::strs(v, "applied.notes");
                if !ok.is_empty() && !notes.is_empty() {
                    s.say.ok(format!("{ok} · {}", notes.join(" · ")));
                }
            }
            return status();
        }
        M::Login(r) => {
            s.busy = false;
            match r {
                Res::Ok(v) => {
                    if kit::st(&v, "authUrl").is_empty() && kit::st(&v, "state") != "Running" {
                        s.say.bad("Tailscale didn't give a sign-in link yet: try again");
                    }
                    let qr = kit::st(&v, "qr");
                    s.qr = if qr.is_empty() { None } else { std::fs::read(&qr).ok().map(image::Handle::from_bytes) };
                    s.login = Some(v);
                }
                Res::Err(e) => s.say.bad(e),
            }
            return status();
        }
        M::Pings(r) => {
            if let Res::Ok(v) = r {
                s.pings = kit::at(&v, "pings").clone();
            }
        }
        M::Start => {
            s.busy = true;
            return Task::perform(kit::job(H, kit::a(&["ts-start"]), String::new()), |r| M::Done("Tailscale is on".into(), r));
        }
        M::SignIn => {
            s.busy = true;
            s.say.clear();
            return Task::perform(kit::job(H, kit::a(&["ts-login"]), String::new()), M::Login);
        }
        M::SignOut => return Task::perform(kit::job(H, kit::a(&["ts-logout"]), String::new()), |r| M::Done("Signed out".into(), r)),
        M::Set(k, on, ok) => {
            if let Some(v) = s.st.as_mut() {
                if let Some(c) = v.get_mut("settings").and_then(|c| c.as_object_mut()) {
                    c.insert(k.to_string(), Value::Bool(on));
                }
            }
            return Task::perform(kit::job(H, kit::a(&["set", k, if on { "on" } else { "off" }]), String::new()), move |r| M::Done(ok.clone(), r));
        }
        M::Send(name) => return Task::perform(kit::job_for(H, vec!["ts-send".into(), name], String::new(), 600), M::Sent),
        M::Sent(r) => match r {
            Res::Ok(v) => {
                if !kit::bo(&v, "cancelled") {
                    s.say.ok(format!("Sent {} to {}", kit::st(&v, "sent"), kit::st(&v, "to")));
                }
            }
            Res::Err(e) => s.say.bad(e),
        },
        M::Tick => {
            let every = if s.login.is_some() { 4 } else { 15 };
            if s.live.still(every) && !s.busy {
                return status();
            }
        }
        M::PingTick => {
            if s.live.still(30) && s.connected() {
                return ping();
            }
        }
    }
    Task::none()
}

pub fn view<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    s.live.seen();
    let ts = s.ts();
    let cfg = s.st.as_ref().map(|v| kit::at(v, "settings")).filter(|c| c.is_object());
    let connected = s.connected();
    let installed = kit::at(ts, "installed").as_bool();
    let running = kit::at(ts, "running").as_bool();
    let mut c = Column::new().spacing(10.0);
    c = c.push(words(&p, "A private network for your devices only. Reach this PC from work or anywhere, with nothing opened to the internet."));
    if let Some(v) = &s.st {
        let e = kit::st(v, "error");
        if !e.is_empty() {
            c = c.push(note(&p, e, true));
        }
    }
    if s.st.is_some() && installed == Some(false) {
        c = c.push(container(t("Tailscale isn't on this PC yet: it comes with the Hearth OS update that brings Home server. Your choices below are kept.", sz(13.0), p.flame)).padding(4.0));
    }
    c = c.push(head(&p, if connected { "Signed in" } else { "Sign in" }));
    if installed == Some(true) && running == Some(false) {
        c = c.push(press(&p, G::Power, "Turn on Tailscale", "Starts it now. Whether it starts with the PC is the switch below.", Some(M::Start)));
    }
    if installed == Some(true) && running != Some(false) && !connected {
        let waiting = s.login.is_some();
        c = c.push(press(
            &p,
            G::Shield,
            if waiting { "Waiting for you to sign in…" } else { "Sign in to Tailscale" },
            if waiting { "Scan the code with your phone, or open the link on any device. Then this page updates by itself." } else { "With Google, Microsoft, GitHub or Apple. Free for personal use." },
            if s.busy { None } else { Some(M::SignIn) },
        ));
    }
    if let Some(l) = &s.login {
        let url = kit::st(l, "authUrl");
        if !url.is_empty() && !connected {
            let mut r = iced::widget::Row::new().spacing(18.0).align_y(iced::Center);
            if let Some(h) = &s.qr {
                r = r.push(container(image(h.clone()).width(Length::Fixed(200.0)).height(Length::Fixed(200.0))).padding(10.0).style(hearth_ui::ui::boxed(iced::Color::WHITE, p.r(12.0), None)));
            }
            let host = s.st.as_ref().map(|v| kit::st(v, "host")).filter(|h| !h.is_empty()).unwrap_or_else(|| "this PC".into());
            r = r.push(column![
                t("1. Scan with your phone, or open:", sz(14.0), p.text),
                t(url, sz(13.0), p.flame).font(iced::Font::MONOSPACE),
                t("2. Sign in with Google, Microsoft, GitHub or Apple", sz(14.0), p.text),
                t(format!("3. This PC shows up as {host}"), sz(14.0), p.text),
            ].spacing(6.0).width(Length::Fill));
            c = c.push(r);
        }
    }
    if connected {
        let login = kit::st(ts, "login");
        let tailnet = kit::st(ts, "tailnet");
        c = c.push(chip_row(&p, G::User, format!("Signed in{}", if login.is_empty() { String::new() } else { format!(" as {login}") }), if tailnet.is_empty() { String::new() } else { format!("Your tailnet: {tailnet}") }, "Connected", "ok", None));
        let name = { let d = kit::st(ts, "dns"); if d.is_empty() { kit::st(ts, "name") } else { d } };
        let ips = kit::strs(ts, "ips");
        c = c.push(val_row(&p, G::Display, "This PC's name", "", format!("{name}{}", ips.first().map(|i| format!(" · {i}")).unwrap_or_default()), None));
        c = c.push(press(&p, G::Arrow, "Sign out", "This PC leaves your tailnet until you sign in again", Some(M::SignOut)));
    }
    if let Some(cf) = cfg {
        c = c.push(head(&p, "Settings"));
        let on = |k: &str| kit::bo(cf, k);
        c = c.push(srow(&p, G::Shield, "Only reachable through Tailscale", "Your shares can't be opened from your home network, only from your devices on Tailscale (at home or away). The firewall lets sharing in only through Tailscale.", switch(&p, on("tsOnly"), Some(M::Set("tsOnly", !on("tsOnly"), if on("tsOnly") { "Your home network can open your shares again".into() } else { "Only through Tailscale now".into() })))));
        c = c.push(srow(&p, G::Power, "Start Tailscale when this PC starts", "Off: Tailscale runs only after you turn it on here. Either way, starting the PC never waits for it.", switch(&p, on("tsBoot"), Some(M::Set("tsBoot", !on("tsBoot"), String::new())))));
        c = c.push(srow(&p, G::Link, "Let other devices use this PC's internet (exit node)", "Off unless you turn it on. Your devices can send their internet through this PC (you also allow it in Tailscale's admin page).", switch(&p, on("exitNode"), Some(M::Set("exitNode", !on("exitNode"), String::new())))));
    }
    if connected {
        c = c.push(head(&p, "Your devices"));
        let peers = kit::ar(ts, "peers");
        for d in peers {
            let name = kit::st(d, "name");
            let os = kit::st(d, "os");
            let online = kit::bo(d, "online");
            let pg = kit::at(&s.pings, &name);
            let value = if !online {
                "offline".to_string()
            } else if pg.is_object() {
                format!("{} ms{}", kit::st(pg, "ms"), if kit::st(pg, "via") == "relay" { " (relay)" } else { "" })
            } else {
                "online".into()
            };
            let lower = os.to_lowercase();
            let g = if lower.contains("android") || lower.contains("ios") { G::Pad } else { G::Display };
            let sub = kit::dots(&[os.clone(), kit::strs(d, "ips").first().cloned().unwrap_or_default(), if online { "Select to send it a file (Taildrop)".into() } else { String::new() }]);
            c = c.push(val_row(&p, g, name.clone(), sub, value, if online { Some(M::Send(name.clone())) } else { None }));
        }
        if peers.is_empty() {
            c = c.push(words(&p, "No other devices yet. Install Tailscale on your phone or laptop and sign in with the same account."));
        }
    }
    if let Some(cf) = cfg {
        c = c.push(head(&p, "Files and waking"));
        let on = |k: &str| kit::bo(cf, k);
        c = c.push(srow(&p, G::Folder, "Receive files (Taildrop)", "Files your devices send to this PC land in Downloads › Taildrop. A name that's taken gets a new one.", switch(&p, on("taildropReceive"), Some(M::Set("taildropReceive", !on("taildropReceive"), String::new())))));
        let wired: Vec<String> = s.st.as_ref().map(|v| kit::ar(v, "wired").iter().map(|w| format!("{} {}", kit::st(w, "iface"), kit::st(w, "mac"))).collect()).unwrap_or_default();
        let sub = format!("Its network card wakes the PC from sleep (Wake-on-LAN). From away, another of your devices at home sends the wake-up{}", if wired.is_empty() { " · needs a network cable".to_string() } else { format!(" · {}", wired.join(", ")) });
        c = c.push(srow(&p, G::Moon, "Wake this PC from away", sub, switch(&p, on("wakeFromAway"), Some(M::Set("wakeFromAway", !on("wakeFromAway"), if on("wakeFromAway") { "Off".into() } else { "Ready to wake from the network".into() })))));
    }
    if let Some(v) = &s.st {
        if kit::at(v, "applied.ok").as_bool() == Some(false) {
            c = c.push(note(&p, format!("Saved, but the system couldn't be changed to match: {}", kit::st(v, "applied.error")), true));
        }
    }
    c = c.push(said(&p, &s.say));
    c.into()
}

pub fn subscription(s: &S) -> Subscription<M> {
    if !s.live.running("hs-tailscale") {
        return Subscription::none();
    }
    let every = if s.login.is_some() { 4 } else { 15 };
    let mut v = vec![iced::time::every(std::time::Duration::from_secs(every)).map(|_| M::Tick)];
    if s.connected() {
        v.push(iced::time::every(std::time::Duration::from_secs(30)).map(|_| M::PingTick));
    }
    Subscription::batch(v)
}

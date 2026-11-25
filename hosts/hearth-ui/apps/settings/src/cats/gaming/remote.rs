//! Gaming › Remote play (Mantel's SetRemote.qml): "Use this PC from anywhere" (hearth-remote-desktop: low-latency
//! remote desktop through Tailscale, pairing with Moonlight's PIN, who's connected, staying awake), streaming this
//! device's games to a TV, phone, tablet or another PC with Moonlight (Sunshine, through Hearth's core remote.*), and
//! playing another PC's games here (Moonlight from Flathub). While "Use this PC from anywhere" is on, the page reads
//! who's connected every 5 s, as Mantel's does (only while the page is open).

use crate::cats::apps::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::Task;
use serde_json::{json, Value};

#[derive(Default)]
pub struct St {
    pub st: Value,
    pub err: String,
    pub msg: String,
    pub busy: String,
    pub pairing: bool,
    pub login: bool,
    pub pin: String,
    pub dev: String,
    pub user: String,
    pub pass: String,
    pub rd: Value,
    pub rd_busy: String,
    pub rd_say: Say,
    pub rd_pairing: bool,
    pub rd_pin: String,
    pub rd_name: String,
    pub rd_check: Value,
}

#[derive(Debug, Clone)]
pub enum M {
    Go(&'static str),
    Got(Result<Value, String>),
    Rd(Res),
    Tick,
    RdRun(Vec<String>, String, &'static str),
    RdDone(String, Res),
    RdPairing,
    RdPin(String),
    RdName(String),
    RdPair,
    RdCheck,
    RdChecked(Res),
    Act(&'static str, Value, &'static str, String),
    Acted(String, Result<Value, String>),
    Login,
    User(String),
    Pass(String),
    SendLogin,
    Pairing,
    Pin(String),
    Dev(String),
    Pair,
    Moonlight,
}

fn load() -> Task<M> {
    core("remote.status", json!({}), 30, M::Got)
}
fn rd_load() -> Task<M> {
    read(py("hearth-remote-desktop", &["status"], 30), M::Rd)
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.msg.clear();
    st.rd_say.clear();
    Task::batch([load(), rd_load()])
}

pub fn rd_on(st: &St) -> bool {
    gb(&st.rd, "on")
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Go(id) => sh.go(id),
        M::Got(r) => match r {
            Ok(v) => {
                st.st = v;
                st.err.clear();
            }
            Err(e) => st.err = e,
        },
        M::Rd(r) => st.rd = if r.ok { r.v } else { json!({"error": r.err}) },
        M::Tick => {
            if st.rd_busy.is_empty() {
                return rd_load();
            }
        }
        M::RdRun(args, said, label) => {
            st.rd_busy = label.to_string();
            sh.reread_history = true;
            let a: Vec<&str> = args.iter().map(String::as_str).collect();
            return read(py("hearth-remote-desktop", &a, 120), move |r| M::RdDone(said.clone(), r));
        }
        M::RdDone(said, r) => {
            st.rd_busy.clear();
            if r.ok {
                let note = gs(&r.v, "note");
                if note.is_empty() {
                    st.rd_say.ok(said)
                } else {
                    st.rd_say.bad(format!("{said} {note}"))
                }
            } else {
                st.rd_say.bad(r.err);
            }
            return Task::batch([rd_load(), load()]);
        }
        M::RdPairing => st.rd_pairing = !st.rd_pairing,
        M::RdPin(t) => st.rd_pin = t,
        M::RdName(t) => st.rd_name = t,
        M::RdPair => {
            let pin: String = st.rd_pin.chars().filter(|c| c.is_ascii_digit()).collect();
            if pin.len() != 4 {
                st.rd_say.bad("The PIN is the 4 numbers Moonlight shows");
                return Task::none();
            }
            let n = if st.rd_name.trim().is_empty() { "Moonlight".to_string() } else { st.rd_name.trim().to_string() };
            st.rd_pin.clear();
            st.rd_name.clear();
            st.rd_pairing = false;
            return update(sh, st, M::RdRun(vec!["pin".into(), pin, n.clone()], format!("Paired {n}. In Moonlight, pick Desktop."), "pin"));
        }
        M::RdCheck => {
            st.rd_busy = "check".into();
            return read(py("hearth-remote-desktop", &["check"], 60), M::RdChecked);
        }
        M::RdChecked(r) => {
            st.rd_busy.clear();
            if r.ok {
                st.rd_check = r.v
            } else {
                st.rd_say.bad(r.err)
            }
        }
        M::Act(method, args, k, said) => {
            st.busy = k.to_string();
            st.msg.clear();
            let secs = if method == "moonlight.install" { 1800 } else { 120 };
            return core(method, args, secs, move |r| M::Acted(said.clone(), r));
        }
        M::Acted(said, r) => {
            st.busy.clear();
            match r {
                Ok(_) => st.msg = said,
                Err(e) => st.msg = e,
            }
            return load();
        }
        M::Login => st.login = !st.login,
        M::User(t) => st.user = t,
        M::Pass(t) => st.pass = t,
        M::SendLogin => {
            let args = json!({"user": st.user, "pass": st.pass});
            st.pass.clear();
            st.login = false;
            return update(sh, st, M::Act("remote.login", args, "login", "Thanks. Now pair a device.".into()));
        }
        M::Pairing => st.pairing = !st.pairing,
        M::Pin(t) => st.pin = t,
        M::Dev(t) => st.dev = t,
        M::Pair => {
            let pin: String = st.pin.chars().filter(|c| c.is_ascii_digit()).collect();
            if pin.len() != 4 {
                st.msg = "The PIN is the 4 numbers Moonlight shows".into();
                return Task::none();
            }
            let n = if st.dev.trim().is_empty() { "Moonlight".to_string() } else { st.dev.trim().to_string() };
            st.pin.clear();
            st.dev.clear();
            st.pairing = false;
            return update(sh, st, M::Act("remote.pair", json!({"pin": pin, "name": n}), "pair", format!("Paired {n}. Pick a game in Moonlight.")));
        }
        M::Moonlight => {
            if gb(&st.st, "moonlight") {
                kit::open(&["flatpak", "run", "com.moonlight_stream.Moonlight"]);
                st.msg = "Opening Moonlight…".into();
            } else if st.busy != "moonlight" {
                return update(sh, st, M::Act("moonlight.install", json!({}), "moonlight", "Moonlight is installed".into()));
            }
        }
    }
    Task::none()
}

fn since(t: f64) -> String {
    if t <= 0.0 {
        return String::new();
    }
    let m = ((hearth_ui::time::now() as f64 - t) / 60.0).round().max(0.0) as i64;
    if m < 1 { "just now".into() } else if m < 60 { format!("{m} min") } else { format!("{} h {} min", m / 60, m % 60) }
}

fn stream_line(s: &Value) -> String {
    let mut parts = Vec::new();
    if gb(s, "width") && gb(s, "height") {
        parts.push(format!("{}×{}{}", gs(s, "width"), gs(s, "height"), if gb(s, "fps") { format!(" at {} fps", gs(s, "fps")) } else { String::new() }));
    }
    if gb(s, "mbps") {
        parts.push(format!("{} Mbps", gs(s, "mbps")));
    }
    let path = gs(s, "path");
    if path != "unknown" && !path.is_empty() {
        parts.push(match path.as_str() {
            "direct" => "direct".to_string(),
            "relay" => format!("through a relay{}", if gb(s, "relay") { format!(" ({})", gs(s, "relay")) } else { String::new() }),
            _ => "home network".into(),
        });
    }
    if gb(s, "since") {
        parts.push(format!("for {}", since(gn(s, "since"))));
    }
    parts.join(" · ")
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let s = &st.st;
    let rd = &st.rd;
    let on = rd_on(st);
    let mut v = crate::secs::SecVec::new();
    if s.is_null() && st.err.is_empty() {
        v.push(words(&p, "Loading…", false));
    }
    if !st.err.is_empty() {
        v.push(words(&p, st.err.clone(), true));
    }
    // ---- Use this PC from anywhere
    if !rd.is_null() && !gb(rd, "error") {
        v.push(head(&p, "Use this PC from anywhere"));
        if !gb(rd, "installed") {
            v.push(words(&p, "This uses Sunshine, which comes with Hearth OS on PCs. It isn't on this system.", false));
        } else {
            let sub = if st.rd_busy == "on" {
                "Working on it…".to_string()
            } else if on {
                if gb(rd, "running") {
                    format!("On: this PC's screen in Moonlight on your laptop, work PC or phone, {}", if gs(rd, "reach") == "home" { "through Tailscale and your home network" } else { "only through Tailscale" })
                } else {
                    "On, but Sunshine isn't running: pick Start again below".into()
                }
            } else {
                "Your desktop on another computer or phone with the free Moonlight app, fast and through Tailscale only. Off until you turn it on".into()
            };
            v.push(sw(&p, G::Link, "Use this PC from anywhere", sub, on, Some(M::RdRun(vec!["set".into(), "on".into(), if on { "off" } else { "on" }.into()], if on { "Off. Sunshine is back as it was.".into() } else { "On. Now add this PC in Moonlight.".into() }, "on"))));
        }
        if on {
            if !gb(rd, "running") {
                v.push(r(G::Update, "Start again", "Sunshine was stopped (Remote play's switch below, or it couldn't start). Start it with Hearth's settings again").on(M::RdRun(vec!["start".into()], "Started again.".into(), "start")).view(&p));
            }
            let ts = go(rd, "tailscale");
            if !ts.is_null() && !gb(ts, "running") {
                v.push(r(G::Link, "Tailscale isn't connected", "Away from home, Moonlight finds this PC through Tailscale. Sign in on this PC and on the other device").chev().on(M::Go("hs-tailscale")).view(&p));
            }
            let a = go(rd, "address");
            let addr = match (gs(a, "name"), gs(a, "ip")) {
                (n, i) if !n.is_empty() && !i.is_empty() => format!("{n} or {i}"),
                (n, i) => if n.is_empty() { i } else { n },
            };
            if !addr.is_empty() {
                v.push(r(G::Grid, format!("In Moonlight, add {addr}"), "On the other device: sign in to Tailscale with the same account, open Moonlight, pick Add PC (+) and type that. Then pick Desktop").view(&p));
            }
            if gb(rd, "running") && !gb(rd, "needsLogin") {
                v.push(r(G::Pad, "Type Moonlight's PIN", if st.rd_busy == "pin" { "Pairing…" } else { "When Moonlight shows 4 numbers, type them here" }).on(M::RdPairing).view(&p));
            }
            if st.rd_pairing {
                v.push(field(&p, "The 4 numbers Moonlight shows", &st.rd_pin, M::RdPin).into());
                v.push(field_on(&p, "Name for that device (like Work laptop), then Enter", &st.rd_name, M::RdName, M::RdPair));
            }
            if gb(rd, "needsLogin") {
                v.push(r(G::Shield, "Sunshine has its own login", "Type it once under Sunshine's username and password below, then pair here").view(&p));
            }
            for c in ga(rd, "clients") {
                let n = if gs(c, "name").is_empty() { "Device".to_string() } else { gs(c, "name") };
                v.push(r(G::Display, n.clone(), "Paired: can use this PC from anywhere").value("Remove").on(M::RdRun(vec!["unpair".into(), gs(c, "uuid")], format!("Removed {n}. It can pair again any time."), "unpair")).view(&p));
            }
            let sm = go(rd, "stream");
            let streaming = gb(sm, "on");
            if streaming {
                let who = [gs(sm, "who"), gs(sm, "device")].into_iter().find(|x| !x.is_empty()).unwrap_or_else(|| "A device".into());
                v.push(r(G::Display, format!("{who} is using this PC now"), format!("{}{}", stream_line(sm), if gb(rd, "awake") { " · staying awake" } else { "" })).view(&p));
                if gb(sm, "tip") {
                    v.push(words(&p, format!("{}.", gs(sm, "tip")), true));
                }
                if gb(sm, "hint") {
                    v.push(small(&p, gs(sm, "hint")));
                }
                if gb(sm, "peerIp") {
                    let c = &st.rd_check;
                    let sub = if st.rd_busy == "check" {
                        "Checking…".to_string()
                    } else if !c.is_null() {
                        format!("{}{} ms. {}", if gs(c, "path") == "direct" { "Direct, ".to_string() } else { format!("Through a relay{}, ", if gb(c, "relay") { format!(" ({})", gs(c, "relay")) } else { String::new() }) }, gs(c, "ms"), gs(c, "hint"))
                    } else {
                        "One ping through Tailscale: direct or relay, and how long it takes".into()
                    };
                    v.push(r(G::Search, "Check the connection", sub).on(M::RdCheck).view(&p));
                }
            } else {
                for peer in ga(rd, "peers").iter().take(6) {
                    let path = gs(peer, "path");
                    let sub = match path.as_str() {
                        "direct" => "Online · direct connection".to_string(),
                        "relay" => format!("Online · through a relay{}: it will feel laggier", if gb(peer, "relay") { format!(" ({})", gs(peer, "relay")) } else { String::new() }),
                        _ => "Online · not connected to this PC right now".into(),
                    };
                    v.push(r(G::Grid, gs(peer, "name"), sub).view(&p));
                }
            }
            let reach = if gs(rd, "reach").is_empty() { "tailscale".to_string() } else { gs(rd, "reach") };
            v.push(choice(&p, G::Link, "Reachable", "Never from the internet. Tailscale only keeps this PC's streaming closed to your home network too", vec![
                ("Tailscale only".into(), reach == "tailscale", Some(M::RdRun(vec!["set".into(), "reach".into(), "tailscale".into()], "Only through Tailscale now.".into(), "reach"))),
                ("Tailscale and home network".into(), reach == "home", Some(M::RdRun(vec!["set".into(), "reach".into(), "home".into()], "Your home network can reach it too.".into(), "reach"))),
            ]));
            let ka = gb(rd, "keepAwake");
            v.push(sw(&p, G::Moon, "Stay awake while connected", "This PC doesn't sleep or go idle while a device streams, and can again 2 minutes after it disconnects", ka, Some(M::RdRun(vec!["set".into(), "keepAwake".into(), if ka { "off" } else { "on" }.into()], if ka { "It can sleep as usual.".into() } else { "Stays awake while connected.".into() }, "awake"))));
            v.push(r(G::Power, "Wake this PC from away", "If it's asleep, another device at home (a phone or a Hearth device on Tailscale) can wake it: Home server › Alerts & power").chev().on(M::Go("hs-alerts")).view(&p));
            v.push(small(&p, format!("{}Moonlight's Desktop shows this PC's screen as it is: in Desktop Mode your desktop, in Game Mode Steam (and a game you start there). Work computers may not allow installing Moonlight or remote access: ask before you do.", if gb(rd, "gameMode") { "Game Mode is running: whoever is connected sees and controls Steam's Game Mode, and the stream goes on. " } else { "" })));
        }
        opt(&mut v, say_view(&p, &st.rd_say));
    }

    // ---- Stream from this device
    if !s.is_null() {
        v.push(head(&p, "Stream from this device"));
        if !gb(s, "installed") {
            v.push(words(&p, "Remote play uses Sunshine, which comes with Hearth OS. It isn't on this system.", false));
        } else {
            let son = gb(s, "on");
            let sub = if st.busy == "on" { "Working on it…" } else if on { "Sunshine is also on for Use this PC from anywhere (turning this off stops it until you pick Start again)" } else { "Play your games on a TV, phone, tablet or another PC with the free Moonlight app" };
            v.push(sw(&p, G::Display, "Remote play", sub, son, Some(M::Act("remote.setOn", json!({"on": !son}), "on", if son { "Remote play is off".into() } else { "Remote play is on. Pair a device to start.".into() }))));
            if son && gb(s, "needsLogin") {
                v.push(r(G::Shield, "Sunshine's username and password", "Sunshine was set up before with its own login. Type it once so Hearth can pair devices.").on(M::Login).view(&p));
            }
            if st.login {
                v.push(field(&p, "Sunshine's username, then Enter", &st.user, M::User).into());
                v.push(field(&p, "Sunshine's password, then Enter", &st.pass, M::Pass).secure(true).on_submit(M::SendLogin).into());
            }
            if son && !on {
                let addrs: Vec<String> = ga(s, "addresses").iter().filter_map(|x| x.as_str().map(String::from)).collect();
                let sub = if st.busy == "pair" { "Pairing…".to_string() } else { format!("In Moonlight, pick this device (or add it by address{}). It shows 4 numbers: type them here.", if addrs.is_empty() { String::new() } else { format!(": {}", addrs.join(", ")) }) };
                v.push(r(G::Pad, "Pair a device", sub).on(M::Pairing).view(&p));
            }
            if st.pairing {
                v.push(field(&p, "The 4 numbers Moonlight shows", &st.pin, M::Pin).into());
                v.push(field_on(&p, "Name for that device (Moonlight), then Enter", &st.dev, M::Dev, M::Pair));
            }
            let clients = ga(s, "clients");
            if son && !on && !clients.is_empty() {
                v.push(head(&p, "Paired devices"));
                for c in clients {
                    let n = if gs(c, "name").is_empty() { "Device".to_string() } else { gs(c, "name") };
                    v.push(r(G::Display, n.clone(), "Can stream from this device").value("Remove").on(M::Act("remote.unpair", json!({"uuid": gs(c, "uuid")}), "", format!("Removed {n}. It can pair again any time."))).view(&p));
                }
            }
            if son {
                v.push(small(&p, "Works best on the same home network, with this device plugged in. Streaming uses this device's screen, so whatever plays here plays there."));
            }
        }
        v.push(head(&p, "Play another PC's games here"));
        let ml = gb(s, "moonlight");
        let sub = if st.busy == "moonlight" { "Installing Moonlight… this takes a few minutes" } else if ml { "Stream games from a gaming PC that runs Sunshine or NVIDIA GameStream" } else { "Install it to stream games from a gaming PC (it needs Sunshine on that PC)" };
        v.push(r(G::Grid, "Moonlight", sub).value(if ml { "Open" } else { "Install" }).on(M::Moonlight).view(&p));
    }
    if !st.msg.is_empty() {
        v.push(words(&p, st.msg.clone(), false));
    }
    page(v)
}

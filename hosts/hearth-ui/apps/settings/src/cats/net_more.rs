//! Network & internet › Mobile hotspot and Proxy (Mantel's SetHotspot and SetProxy), both through hearth-network.
//! Hotspot: share this PC's internet over Wi-Fi (NetworkManager's shared mode, connection "Hearth Hotspot"): on and
//! off, its name, password and band; the password goes to NetworkManager on the helper's stdin and is read back only
//! when you press Show. It never starts by itself. Proxy: none, automatic (a setup script) or by hand, saved in
//! ~/.config/nimbus/proxy.json through hearth_settings_io only when you press Save.

use super::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::widget::Column;
use iced::{Subscription, Task};
use serde_json::{json, Value};

const H: &str = "hearth-network";

#[derive(Default)]
pub struct S {
    // hotspot
    pub hs: Option<Value>,
    pub shown_pw: String,
    pub band: String,
    pub ssid: String,
    pub pass: String,
    pub hsay: Say,
    pub hwhat: &'static str,
    pub live: Live,
    // proxy
    pub px: Option<Value>,
    pub mode: String,
    pub url: String,
    pub http: String,
    pub https: String,
    pub socks: String,
    pub ignore: String,
    pub psay: Say,
    pub pwhat: &'static str,
}

#[derive(Debug, Clone)]
pub enum M {
    Hotspot(Res),
    HsOn(bool),
    HsDone(&'static str, Res),
    ShowPw,
    Pw(Res),
    Ssid(String),
    Pass(String),
    Band(&'static str),
    Save,
    Tick,
    Proxy(Res),
    Mode(&'static str),
    Url(String),
    Http(String),
    Https(String),
    Socks(String),
    Ignore(String),
    PxSave,
    PxSaved(Res),
}

impl S {
    fn made(&self) -> bool {
        self.hs.as_ref().map(|v| !kit::st(v, "uuid").is_empty()).unwrap_or(false)
    }
    fn editing(&self) -> bool {
        !self.ssid.is_empty() || !self.pass.is_empty()
    }
    fn px_changed(&self) -> bool {
        match &self.px {
            Some(v) => self.mode != kit::st(v, "mode") || [&self.url, &self.http, &self.https, &self.socks, &self.ignore].iter().any(|f| !f.is_empty()),
            None => false,
        }
    }
}

fn hotspot() -> Task<M> {
    Task::perform(kit::job(H, kit::a(&["hotspot"]), String::new()), M::Hotspot)
}

pub fn open(_sh: &mut Shared, s: &mut S, page: &'static str, _part: &str) -> Task<M> {
    if page == "hotspot" {
        s.shown_pw.clear();
        s.ssid.clear();
        s.pass.clear();
        s.hsay.clear();
        s.live.start("hotspot");
        return hotspot();
    }
    s.live.stop();
    s.psay.clear();
    for f in [&mut s.url, &mut s.http, &mut s.https, &mut s.socks, &mut s.ignore] {
        f.clear();
    }
    Task::perform(kit::job(H, kit::a(&["proxy"]), String::new()), M::Proxy)
}

fn turn_on(s: &mut S, ssid: String, pw: String) -> Task<M> {
    s.hsay.clear();
    s.hwhat = "on";
    let req = json!({ "ssid": ssid, "password": pw, "band": s.band });
    Task::perform(kit::job(H, kit::a(&["hotspot-on"]), req.to_string()), |r| M::HsDone("on", r))
}

pub fn update(_sh: &mut Shared, s: &mut S, m: M) -> Task<M> {
    match m {
        M::Hotspot(r) => match r {
            Res::Ok(v) => {
                s.band = kit::st(&v, "band");
                s.hs = Some(v);
            }
            Res::Err(e) => s.hsay.bad(e),
        },
        M::HsOn(on) => {
            if on {
                let ssid = s.hs.as_ref().map(|v| kit::st(v, "ssid")).unwrap_or_default();
                return turn_on(s, ssid, String::new());
            }
            s.hwhat = "off";
            return Task::perform(kit::job(H, kit::a(&["hotspot-off"]), String::new()), |r| M::HsDone("off", r));
        }
        M::HsDone(what, r) => {
            s.hwhat = "";
            match r {
                Res::Ok(v) => {
                    if what == "on" {
                        s.shown_pw.clear();
                        s.hsay.ok(format!("Hotspot on: join \"{}\" from your other devices.", kit::st(&v, "ssid")));
                    } else {
                        s.hsay.ok("Hotspot off.");
                    }
                    s.band = kit::st(&v, "band");
                    s.hs = Some(v);
                }
                Res::Err(e) => s.hsay.bad(e),
            }
        }
        M::ShowPw => {
            if !s.shown_pw.is_empty() {
                s.shown_pw.clear();
                return Task::none();
            }
            s.hwhat = "pw";
            return Task::perform(kit::job(H, kit::a(&["hotspot-password"]), String::new()), M::Pw);
        }
        M::Pw(r) => {
            s.hwhat = "";
            match r {
                Res::Ok(v) => s.shown_pw = kit::st(&v, "password"),
                Res::Err(e) => s.hsay.bad(e),
            }
        }
        M::Ssid(t) => s.ssid = t,
        M::Pass(t) => s.pass = t,
        M::Band(b) => s.band = b.to_string(),
        M::Save => {
            let saved = s.hs.as_ref().map(|v| kit::st(v, "ssid")).unwrap_or_default();
            let n = if s.ssid.trim().is_empty() { if s.made() { saved } else { String::new() } } else { s.ssid.trim().to_string() };
            if n.is_empty() {
                s.hsay.bad("Type a name for the hotspot.");
                return Task::none();
            }
            if !s.made() && s.pass.chars().count() < 8 {
                s.hsay.bad("The password needs 8 letters or more.");
                return Task::none();
            }
            let pw = std::mem::take(&mut s.pass);
            s.ssid.clear();
            return turn_on(s, n, pw);
        }
        M::Tick => {
            if s.live.still(15) && s.hwhat.is_empty() && !s.editing() {
                return hotspot();
            }
        }
        M::Proxy(r) => match r {
            Res::Ok(v) => {
                s.mode = kit::st(&v, "mode");
                s.px = Some(v);
            }
            Res::Err(e) => s.psay.bad(e),
        },
        M::Mode(m) => s.mode = m.to_string(),
        M::Url(t) => s.url = t,
        M::Http(t) => s.http = t,
        M::Https(t) => s.https = t,
        M::Socks(t) => s.socks = t,
        M::Ignore(t) => s.ignore = t,
        M::PxSave => {
            let Some(saved) = s.px.clone() else { return Task::none() };
            if !s.px_changed() {
                return Task::none();
            }
            s.psay.clear();
            s.pwhat = "save";
            let keep = |f: &str, k: &str| if f.trim().is_empty() { kit::st(&saved, k) } else { f.trim().to_string() };
            let mut req = json!({ "mode": s.mode });
            if s.mode == "auto" {
                req["url"] = Value::String(keep(&s.url, "url"));
            }
            if s.mode == "manual" {
                req["http"] = Value::String(keep(&s.http, "http"));
                req["https"] = Value::String(keep(&s.https, "https"));
                req["socks"] = Value::String(keep(&s.socks, "socks"));
                req["ignore"] = Value::String(keep(&s.ignore, "ignore"));
            }
            return Task::perform(kit::job(H, kit::a(&["proxy-set"]), req.to_string()), M::PxSaved);
        }
        M::PxSaved(r) => {
            s.pwhat = "";
            match r {
                Res::Ok(v) => {
                    s.mode = kit::st(&v, "mode");
                    for f in [&mut s.url, &mut s.http, &mut s.https, &mut s.socks, &mut s.ignore] {
                        f.clear();
                    }
                    s.psay.ok(if s.mode == "none" { "No proxy." } else { "Proxy saved. Apps opened from now on use it; restart open ones." });
                    s.px = Some(v);
                }
                Res::Err(e) => s.psay.bad(e),
            }
        }
    }
    Task::none()
}

pub fn view<'a>(sh: &'a Shared, s: &'a S, page: &'static str) -> E<'a, M> {
    if page == "proxy" {
        return proxy(sh, s);
    }
    let p = sh.p;
    s.live.seen();
    let mut c = crate::secs::Plain::new(0).spacing(10.0).push(head(&p, "Mobile hotspot"));
    let made = s.made();
    let (sub, can, on, ssid) = match &s.hs {
        None => ("Checking…".to_string(), false, false, String::new()),
        Some(v) => {
            let ssid = kit::st(v, "ssid");
            let on = kit::bo(v, "on");
            let can = kit::bo(v, "canHotspot");
            let sub = if s.hwhat == "on" {
                "Starting…".to_string()
            } else if kit::st(v, "device").is_empty() {
                "This PC has no Wi-Fi".into()
            } else if !can {
                "This PC's Wi-Fi can't be a hotspot".into()
            } else if on {
                format!("On as \"{ssid}\". Wi-Fi to other networks is off while it's on")
            } else if made {
                format!("Off · \"{ssid}\"")
            } else {
                "Set a name and password below first".into()
            };
            (sub, can, on, ssid)
        }
    };
    c = c.push(switch_row(&p, G::Wifi, "Share my internet", sub, on, can && (made || on), M::HsOn(!on)));
    if made {
        c = c.push(press(
            &p,
            G::Shield,
            if s.shown_pw.is_empty() { "Show the password".to_string() } else { s.shown_pw.clone() },
            if s.shown_pw.is_empty() { "To type it on your phone".to_string() } else { format!("The password for \"{ssid}\"") },
            Some(M::ShowPw),
        ));
    }
    if can {
        c = c.push(head(&p, if made { "Name and password" } else { "Set up the hotspot" }));
        let host = if sh.info.host.is_empty() { "Hearth".to_string() } else { sh.info.host.clone() };
        c = c.push(input(&p, &if made { format!("Name: {ssid}") } else { format!("Network name, like {host} hotspot") }, &s.ssid, false, M::Ssid, None));
        c = c.push(input(&p, if made { "New password (leave empty to keep it)" } else { "Password (8 letters or more)" }, &s.pass, true, M::Pass, Some(M::Save)));
        c = c.push(choice(&p, G::Wifi, "Band", "5 GHz is faster; 2.4 GHz reaches further and works with older devices", [("", "Any"), ("bg", "2.4 GHz"), ("a", "5 GHz")].iter().map(|&(v, l)| (l.to_string(), s.band == v, Some(M::Band(v)))).collect()));
        c = c.push(press(&p, G::Check, if s.hwhat == "on" { "Starting…" } else if made { "Save and turn on" } else { "Turn on the hotspot" }, "", Some(M::Save)));
    }
    c.push(said(&p, &s.hsay)).into()
}

fn proxy<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let mut c = crate::secs::Plain::new(0).spacing(10.0).push(head(&p, "Proxy"));
    let sub = match &s.px {
        None => "Checking…".to_string(),
        Some(v) => match kit::st(v, "mode").as_str() {
            "none" | "" => "Not now: apps go straight to the internet".into(),
            "auto" => format!("Automatic, from {}", kit::st(v, "url")),
            _ => {
                let h = kit::st(v, "http");
                format!("By hand{}", if h.is_empty() { String::new() } else { format!(" · {h}") })
            }
        },
    };
    c = c.push(choice(&p, G::Server, "Use a proxy", sub, [("none", "No"), ("auto", "Automatic"), ("manual", "By hand")].iter().map(|&(v, l)| (l.to_string(), s.mode == v, Some(M::Mode(v)))).collect()));
    let saved = |k: &str| s.px.as_ref().map(|v| kit::st(v, k)).unwrap_or_default();
    if s.mode == "auto" {
        c = c.push(head(&p, "Automatic setup"));
        let u = saved("url");
        c = c.push(input(&p, &if u.is_empty() { "Setup script address, like http://wpad.example/proxy.pac".to_string() } else { format!("Setup script: {u}") }, &s.url, false, M::Url, None));
    }
    if s.mode == "manual" {
        c = c.push(head(&p, "Proxies"));
        let ph = |label: &str, k: &str, eg: &str| { let v = saved(k); if v.is_empty() { format!("{label}{eg}") } else { format!("{label}: {v}") } };
        c = c.push(input(&p, &ph("Web (HTTP)", "http", ", like proxy.example.com:3128"), &s.http, false, M::Http, None));
        c = c.push(input(&p, &ph("Secure web (HTTPS)", "https", ", often the same"), &s.https, false, M::Https, None));
        c = c.push(input(&p, &ph("SOCKS", "socks", ", like localhost:1080 (optional)"), &s.socks, false, M::Socks, None));
        let ig = saved("ignore");
        c = c.push(input(&p, &format!("Skip the proxy for: {}", if ig.is_empty() && s.px.is_none() { "localhost".to_string() } else { ig }), &s.ignore, false, M::Ignore, None));
    }
    let changed = s.px_changed();
    c = c.push(Column::new().push(press(&p, G::Check, if s.pwhat == "save" { "Saving…" } else { "Save" }, if changed { "For web browsers, apps and the terminal" } else { "Nothing changed" }, if changed { Some(M::PxSave) } else { None })));
    c.push(said(&p, &s.psay)).into()
}

pub fn subscription(s: &S) -> Subscription<M> {
    if s.live.running("hotspot") && !s.editing() {
        iced::time::every(std::time::Duration::from_secs(15)).map(|_| M::Tick)
    } else {
        Subscription::none()
    }
}

//! Network & internet › Connection details (Mantel's SetNetDetails): every saved network (Wi-Fi, wired, mobile
//! broadband, VPN), and for the one you pick: what it has now (address, gateway, DNS), DNS servers of your own, metered,
//! MAC address (random or the real one), auto-connect, IPv4 by hand, IPv6 and the packet size (MTU); enterprise sign-in
//! (802.1X: PEAP, TTLS, TLS, PWD) for Wi-Fi and wired, mobile broadband (APN, SIM PIN, roaming), a QR code to share a
//! Wi-Fi network, Duplicate, and adding a wired, mobile or enterprise Wi-Fi connection. All NetworkManager's
//! (hearth-network), changed only when you press, one setting at a time; passwords go to NetworkManager on the
//! helper's stdin and NetworkManager keeps them.

use super::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{btn, t, tb, Kind, G};
use iced::widget::canvas::{self, Frame, Geometry};
use iced::widget::{column, container, row, Column};
use iced::{mouse, Color, Length, Point, Rectangle, Renderer, Size, Subscription, Task, Theme};
use serde_json::{json, Value};

const H: &str = "hearth-network";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum F {
    WiredName,
    MobName,
    Apn,
    MobUser,
    MobPw,
    Pin,
    EntSsid,
    EapId,
    EapAnon,
    EapCa,
    EapDomain,
    EapCert,
    EapKey,
    KeyPw,
    EapPw,
    NApn,
    NUser,
    NPw,
    NPin,
    Dns,
    Addr4,
    Gw4,
    Mtu,
}
const NF: usize = 23;

#[derive(Default)]
pub struct S {
    pub st: Option<Value>,
    pub uuid: String,
    pub d: Option<Value>,
    pub say: Say,
    pub manual: bool,
    pub adding: String,
    pub qr: Option<Value>,
    pub show_pw: bool,
    pub edit_eap: bool,
    pub want: String,
    pub what: &'static str,
    pub f: [String; NF],
    pub wired_dev: String,
    pub wired_open: bool,
    pub roam_new: bool,
    pub eap_kind: String,
    pub phase2: String,
    pub eap_ask: bool,
    pub mac_open: bool,
}

#[derive(Debug, Clone)]
pub enum M {
    Status(Res),
    Pick(String),
    Details(Res),
    Field(F, String),
    Adding(&'static str),
    WiredOpen,
    WiredDev(String),
    AddWired,
    AddMobile,
    RoamNew,
    Added(Res, &'static str),
    EapKind(&'static str),
    Phase2(&'static str),
    EapAsk,
    EapSave,
    EapOff,
    EapSaved(Res, bool),
    EditEap,
    Qr,
    QrGot(Res),
    ShowPw,
    HideQr,
    MobSave,
    MobRoam(bool),
    Change(Value, String),
    Changed(Res, String),
    MacOpen,
    Mac(&'static str),
    Ipv4(&'static str),
    UseAddr,
    UseDns,
    UseMtu,
    Dup,
    Dupped(Res),
}

impl S {
    fn g(&self, f: F) -> &str {
        &self.f[f as usize]
    }
    fn take(&mut self, f: F) -> String {
        std::mem::take(&mut self.f[f as usize])
    }
    fn conns(&self) -> Vec<&Value> {
        self.st.as_ref().map(|v| kit::ar(v, "connections").iter().filter(|c| matches!(kit::st(c, "kind").as_str(), "wifi" | "ethernet" | "vpn" | "mobile")).collect()).unwrap_or_default()
    }
    fn dkind(&self) -> String {
        self.d.as_ref().map(|d| kit::st(d, "kind")).unwrap_or_default()
    }
    fn dname(&self) -> String {
        self.d.as_ref().map(|d| kit::st(d, "name")).unwrap_or_default()
    }
}

fn status() -> Task<M> {
    Task::perform(kit::job(H, kit::a(&["status"]), String::new()), M::Status)
}

pub fn open(_sh: &mut Shared, s: &mut S, part: &str) -> Task<M> {
    s.say.clear();
    s.qr = None;
    s.show_pw = false;
    s.edit_eap = false;
    s.adding.clear();
    s.want.clear();
    match part {
        "add-wired" | "add-mobile" | "add-enterprise" => {
            s.adding = part[4..].to_string();
            if s.adding == "enterprise" {
                s.eap_kind = "peap".into();
                s.phase2 = "mschapv2".into();
                s.eap_ask = false;
            }
        }
        "qr" | "mobile" | "wired" => s.want = part.to_string(),
        _ => {}
    }
    status()
}

fn pick(s: &mut S, u: String) -> Task<M> {
    s.uuid = u.clone();
    s.manual = false;
    s.d = None;
    s.qr = None;
    s.show_pw = false;
    s.edit_eap = false;
    s.mac_open = false;
    s.say.clear();
    Task::perform(kit::job(H, vec!["details".into(), u], String::new()), M::Details)
}

fn change(s: &mut S, req: Value, ok: String) -> Task<M> {
    s.say.clear();
    s.what = "set";
    Task::perform(kit::job(H, vec!["details-set".into(), s.uuid.clone()], req.to_string()), move |r| M::Changed(r, ok.clone()))
}

fn eap_req(s: &S) -> Value {
    json!({
        "eap": s.eap_kind, "identity": s.g(F::EapId).trim(), "anonymous": s.g(F::EapAnon).trim(), "caCert": s.g(F::EapCa).trim(),
        "domain": s.g(F::EapDomain).trim(), "phase2": s.phase2, "password": s.g(F::EapPw), "askEachTime": s.eap_ask,
        "clientCert": s.g(F::EapCert).trim(), "privateKey": s.g(F::EapKey).trim(), "keyPassword": s.g(F::KeyPw),
    })
}

pub fn update(_sh: &mut Shared, s: &mut S, m: M) -> Task<M> {
    match m {
        M::Status(r) => match r {
            Res::Ok(v) => {
                let first = if s.uuid.is_empty() {
                    let cs = kit::ar(&v, "connections");
                    let want = match s.want.as_str() {
                        "mobile" => cs.iter().find(|c| kit::st(c, "kind") == "mobile"),
                        "wired" => cs.iter().find(|c| kit::st(c, "kind") == "ethernet"),
                        _ => None,
                    };
                    want.or_else(|| cs.iter().find(|c| kit::bo(c, "active") && kit::st(c, "kind") != "vpn")).map(|c| kit::st(c, "uuid"))
                } else {
                    None
                };
                s.st = Some(v);
                if let Some(u) = first {
                    return pick(s, u);
                }
            }
            Res::Err(e) => s.say.bad(e),
        },
        M::Pick(u) => return pick(s, u),
        M::Details(r) => match r {
            Res::Ok(v) => {
                let kind = kit::st(&v, "kind");
                s.d = Some(v);
                if s.want == "qr" && kind == "wifi" {
                    s.want.clear();
                    return update(_sh, s, M::Qr);
                }
                if s.want == "mobile" && kind == "mobile" {
                    s.want.clear();
                }
            }
            Res::Err(e) => s.say.bad(e),
        },
        M::Field(f, t) => s.f[f as usize] = t,
        M::Adding(k) => {
            s.adding = if s.adding == k { String::new() } else { k.to_string() };
            if k == "enterprise" {
                s.edit_eap = false;
                s.eap_kind = "peap".into();
                s.phase2 = "mschapv2".into();
                s.eap_ask = false;
            }
        }
        M::WiredOpen => s.wired_open = !s.wired_open,
        M::WiredDev(d) => {
            s.wired_dev = d;
            s.wired_open = false;
        }
        M::AddWired => {
            s.what = "add";
            let req = json!({ "name": s.g(F::WiredName).trim(), "device": s.wired_dev });
            return Task::perform(kit::job(H, kit::a(&["wired-add"]), req.to_string()), |r| M::Added(r, "wired"));
        }
        M::AddMobile => {
            s.what = "add";
            let req = json!({ "name": s.g(F::MobName).trim(), "apn": s.g(F::Apn).trim(), "user": s.g(F::MobUser).trim(), "password": s.g(F::MobPw), "pin": s.g(F::Pin), "roaming": s.roam_new });
            s.take(F::MobPw);
            s.take(F::Pin);
            return Task::perform(kit::job(H, kit::a(&["mobile-add"]), req.to_string()), |r| M::Added(r, "mobile"));
        }
        M::RoamNew => s.roam_new = !s.roam_new,
        M::Added(r, kind) => {
            s.what = "";
            match r {
                Res::Ok(v) => {
                    s.adding.clear();
                    let name = kit::st(&v, "name");
                    s.say.ok(if kind == "enterprise" { format!("{name} is added. It connects when it's in reach.") } else { format!("{name} is added.") });
                    let u = kit::st(&v, "uuid");
                    let t = pick(s, u);
                    s.say.ok(if kind == "enterprise" { format!("{name} is added. It connects when it's in reach.") } else { format!("{name} is added.") });
                    return Task::batch([status(), t]);
                }
                Res::Err(e) => s.say.bad(e),
            }
        }
        M::EapKind(k) => s.eap_kind = k.to_string(),
        M::Phase2(k) => s.phase2 = k.to_string(),
        M::EapAsk => s.eap_ask = !s.eap_ask,
        M::EapSave => {
            let mut req = eap_req(s);
            s.what = "eap";
            s.take(F::EapPw);
            s.take(F::KeyPw);
            if s.adding == "enterprise" {
                req["ssid"] = Value::String(s.g(F::EntSsid).trim().to_string());
                return Task::perform(kit::job(H, kit::a(&["wifi-enterprise-add"]), req.to_string()), |r| M::Added(r, "enterprise"));
            }
            return Task::perform(kit::job(H, vec!["enterprise-set".into(), s.uuid.clone()], req.to_string()), |r| M::EapSaved(r, false));
        }
        M::EapOff => {
            s.what = "eap";
            return Task::perform(kit::job(H, vec!["enterprise-set".into(), s.uuid.clone()], json!({ "off": true }).to_string()), |r| M::EapSaved(r, true));
        }
        M::EapSaved(r, off) => {
            s.what = "";
            match r {
                Res::Ok(v) => {
                    s.edit_eap = false;
                    s.say.ok(if off { format!("802.1X is off for {}.", kit::st(&v, "name")) } else { "Sign-in saved.".into() });
                    s.d = Some(v);
                }
                Res::Err(e) => s.say.bad(e),
            }
        }
        M::EditEap => {
            s.edit_eap = !s.edit_eap;
            s.adding.clear();
            if let Some(d) = &s.d {
                let e = kit::at(d, "enterprise");
                s.eap_kind = if e.is_object() { kit::st(e, "eap") } else { "peap".into() };
                let p2 = kit::st(e, "phase2");
                s.phase2 = if p2.is_empty() { "mschapv2".into() } else { p2 };
                s.eap_ask = kit::bo(e, "asks");
            }
        }
        M::Qr => {
            s.say.clear();
            s.what = "qr";
            return Task::perform(kit::job(H, vec!["wifi-qr".into(), s.uuid.clone()], String::new()), M::QrGot);
        }
        M::QrGot(r) => {
            s.what = "";
            match r {
                Res::Ok(v) => s.qr = Some(v),
                Res::Err(e) => s.say.bad(e),
            }
        }
        M::ShowPw => s.show_pw = !s.show_pw,
        M::HideQr => {
            s.qr = None;
            s.show_pw = false;
        }
        M::MobSave => {
            let mut req = json!({});
            for (f, k) in [(F::NApn, "apn"), (F::NUser, "user")] {
                if !s.g(f).trim().is_empty() {
                    req[k] = Value::String(s.g(f).trim().to_string());
                }
            }
            for (f, k) in [(F::NPw, "password"), (F::NPin, "pin")] {
                if !s.g(f).is_empty() {
                    req[k] = Value::String(s.g(f).to_string());
                }
            }
            for f in [F::NApn, F::NUser, F::NPw, F::NPin] {
                s.take(f);
            }
            s.what = "set";
            return Task::perform(kit::job(H, vec!["mobile-set".into(), s.uuid.clone()], req.to_string()), |r| M::Changed(r, "Saved.".into()));
        }
        M::MobRoam(on) => {
            return Task::perform(kit::job(H, vec!["mobile-set".into(), s.uuid.clone()], json!({ "roaming": on }).to_string()), move |r| M::Changed(r, if on { "It's used when roaming too.".into() } else { "Home network only.".into() }));
        }
        M::Change(req, ok) => return change(s, req, ok),
        M::Changed(r, ok) => {
            s.what = "";
            match r {
                Res::Ok(v) => {
                    s.d = Some(v);
                    s.say.ok(ok);
                }
                Res::Err(e) => s.say.bad(e),
            }
        }
        M::MacOpen => s.mac_open = !s.mac_open,
        M::Mac(v) => {
            s.mac_open = false;
            if v != "fixed" {
                return change(s, json!({ "mac": v }), "Saved. It's used the next time you connect.".into());
            }
        }
        M::Ipv4(v) => {
            if v == "auto" {
                return change(s, json!({ "ipv4": { "method": "auto" } }), "Automatic address.".into());
            }
            s.manual = true;
        }
        M::UseAddr => {
            s.manual = false;
            let req = json!({ "ipv4": { "method": "manual", "address": s.g(F::Addr4).trim(), "gateway": s.g(F::Gw4).trim() } });
            return change(s, req, "Address saved.".into());
        }
        M::UseDns => {
            let list: Vec<String> = s.g(F::Dns).split(|c: char| c == ',' || c.is_whitespace()).filter(|x| !x.is_empty()).map(String::from).collect();
            s.take(F::Dns);
            return change(s, json!({ "dns": list }), "DNS saved.".into());
        }
        M::UseMtu => {
            let n: u64 = s.g(F::Mtu).trim().parse().unwrap_or(0);
            s.take(F::Mtu);
            return change(s, json!({ "mtu": n }), "Saved. It's used the next time it connects.".into());
        }
        M::Dup => {
            s.what = "dup";
            return Task::perform(kit::job(H, vec!["duplicate".into(), s.uuid.clone()], String::new()), M::Dupped);
        }
        M::Dupped(r) => {
            s.what = "";
            match r {
                Res::Ok(v) => {
                    let name = kit::st(&v, "name");
                    let t = pick(s, kit::st(&v, "uuid"));
                    s.say.ok(format!("{name} is the copy."));
                    return Task::batch([status(), t]);
                }
                Res::Err(e) => s.say.bad(e),
            }
        }
    }
    Task::none()
}

fn kind_glyph(k: &str) -> G {
    match k {
        "wifi" => G::Wifi,
        "ethernet" => G::Link,
        "vpn" => G::Shield,
        _ => G::Server,
    }
}
fn kind_word(k: &str) -> &'static str {
    match k {
        "wifi" => "Wi-Fi",
        "ethernet" => "Wired",
        "vpn" => "VPN",
        _ => "Mobile broadband",
    }
}

/// A QR code from hearth-network's rows of "0" / "1" (qrencode), drawn once.
pub struct Qr {
    pub rows: Vec<Vec<bool>>,
}

impl<Msg> canvas::Program<Msg> for Qr {
    type State = ();
    fn draw(&self, _s: &(), renderer: &Renderer, _t: &Theme, bounds: Rectangle, _c: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        f.fill_rectangle(Point::ORIGIN, bounds.size(), Color::WHITE);
        let n = self.rows.len().max(1) as f32;
        let cell = (bounds.width.min(bounds.height) / n).floor().max(1.0);
        let off = ((bounds.width - cell * n) / 2.0).floor();
        let ink = Color::from_rgb8(0x11, 0x11, 0x11);
        for (y, r) in self.rows.iter().enumerate() {
            for (x, on) in r.iter().enumerate() {
                if *on {
                    f.fill_rectangle(Point::new(off + x as f32 * cell, off + y as f32 * cell), Size::new(cell, cell), ink);
                }
            }
        }
        vec![f.into_geometry()]
    }
}

pub fn view<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let mut c = Column::new().spacing(10.0).push(head(&p, "Saved networks"));
    for cn in s.conns() {
        let k = kit::st(cn, "kind");
        let u = kit::st(cn, "uuid");
        c = c.push(val_row(&p, kind_glyph(&k), kit::st(cn, "name"), format!("{}{}", kind_word(&k), if kit::bo(cn, "active") { " · Connected" } else { "" }), if s.uuid == u { "Showing" } else { "" }, Some(M::Pick(u.clone()))));
    }
    // ---- Add a network: a wired profile of your own, mobile broadband, an enterprise Wi-Fi network.
    c = c.push(head(&p, "Add a network"));
    let adding = |k: &str| if s.adding == k { "Adding" } else { "" };
    c = c.push(xrow(&p, G::Link, "Wired connection", "A second profile for a cable: another address, DNS or 802.1X sign-in", Some(row![t(adding("wired"), sz(13.0), p.muted), hearth_ui::ui::glyph(G::Chevron, p.muted, 14.0, 2.0)].spacing(10.0).align_y(iced::Center).into()), false, Some(M::Adding("wired"))));
    if s.adding == "wired" {
        let devs: Vec<String> = s.st.as_ref().map(|v| kit::ar(v, "devices").iter().filter(|d| kit::st(d, "type") == "ethernet").map(|d| kit::st(d, "device")).collect()).unwrap_or_default();
        let mut opts = vec![("Any wired port".to_string(), s.wired_dev.is_empty(), M::WiredDev(String::new()))];
        for d in devs {
            opts.push((d.clone(), s.wired_dev == d, M::WiredDev(d)));
        }
        c = c.push(indent(column![
            input(&p, "A name, like Office cable", s.g(F::WiredName), false, |t| M::Field(F::WiredName, t), None),
            dropdown(&p, G::Link, "For", "", if s.wired_dev.is_empty() { "Any wired port".to_string() } else { s.wired_dev.clone() }, s.wired_open, M::WiredOpen, opts),
            press(&p, G::Plus, if s.what == "add" { "Adding…" } else { "Add this wired connection" }, "It connects only when you pick it", Some(M::AddWired)),
        ].spacing(8.0)));
    }
    let has_modem = s.st.as_ref().map(|v| kit::bo(v, "hasModem")).unwrap_or(true);
    c = c.push(xrow(&p, G::Server, "Mobile broadband", if s.st.is_some() && !has_modem { "No modem found right now: you can still set it up for when one is plugged in" } else { "A SIM in this PC or a USB modem: its APN and PIN" }, Some(row![t(adding("mobile"), sz(13.0), p.muted), hearth_ui::ui::glyph(G::Chevron, p.muted, 14.0, 2.0)].spacing(10.0).align_y(iced::Center).into()), false, Some(M::Adding("mobile"))));
    if s.adding == "mobile" {
        c = c.push(indent(column![
            input(&p, "A name, like My phone plan", s.g(F::MobName), false, |t| M::Field(F::MobName, t), None),
            input(&p, "APN, from your mobile provider (like internet)", s.g(F::Apn), false, |t| M::Field(F::Apn, t), None),
            input(&p, "User name (most providers have none)", s.g(F::MobUser), false, |t| M::Field(F::MobUser, t), None),
            input(&p, "Password (most providers have none)", s.g(F::MobPw), true, |t| M::Field(F::MobPw, t), None),
            input(&p, "SIM PIN (if your SIM asks for one)", s.g(F::Pin), true, |t| M::Field(F::Pin, t), None),
            srow(&p, G::Link, "Use it when roaming", "Abroad or on another network: it can cost more", switch(&p, s.roam_new, Some(M::RoamNew))),
            press(&p, G::Plus, if s.what == "add" { "Adding…" } else { "Add mobile broadband" }, "", Some(M::AddMobile)),
        ].spacing(8.0)));
    }
    c = c.push(xrow(&p, G::Wifi, "Enterprise Wi-Fi (802.1X)", "Work, school or eduroam: you sign in with your own account", Some(row![t(adding("enterprise"), sz(13.0), p.muted), hearth_ui::ui::glyph(G::Chevron, p.muted, 14.0, 2.0)].spacing(10.0).align_y(iced::Center).into()), false, Some(M::Adding("enterprise"))));
    if s.adding == "enterprise" {
        c = c.push(indent(input(&p, "The network's name (SSID), like eduroam", s.g(F::EntSsid), false, |t| M::Field(F::EntSsid, t), None)));
    }

    // The 802.1X form: for a new enterprise Wi-Fi network, or the picked Wi-Fi / wired connection.
    let for_new = s.adding == "enterprise";
    if for_new || (s.edit_eap && s.d.is_some()) {
        let ent = s.d.as_ref().map(|d| kit::at(d, "enterprise").clone()).unwrap_or(Value::Null);
        c = c.push(head(&p, if for_new { "Sign-in for the new network".to_string() } else { format!("Enterprise sign-in (802.1X) for {}", s.dname()) }));
        let k = s.eap_kind.as_str();
        c = c.push(choice(&p, G::Shield, "How it signs in", "Ask your IT people if you're not sure: PEAP is the usual one (eduroam too)", [("peap", "PEAP"), ("ttls", "TTLS"), ("tls", "Certificate (TLS)"), ("pwd", "PWD")].iter().map(|&(v, l)| (l.to_string(), k == v, Some(M::EapKind(v)))).collect()));
        let idph = if !for_new && ent.is_object() { format!("User name: {}", kit::st(&ent, "identity")) } else { "Your user name (identity), like you@school.edu".to_string() };
        c = c.push(input(&p, &idph, s.g(F::EapId), false, |t| M::Field(F::EapId, t), None));
        if k != "tls" {
            c = c.push(input(&p, "Anonymous identity (optional), like anonymous@school.edu", s.g(F::EapAnon), false, |t| M::Field(F::EapAnon, t), None));
        }
        if k == "peap" || k == "ttls" {
            c = c.push(choice(&p, G::Shield, "Inner sign-in", "", [("mschapv2", "MSCHAPv2"), ("pap", "PAP"), ("gtc", "GTC")].iter().map(|&(v, l)| (l.to_string(), s.phase2 == v, Some(M::Phase2(v)))).collect()));
        }
        c = c.push(input(&p, "CA certificate file (optional but safer), like ~/Downloads/ca.pem", s.g(F::EapCa), false, |t| M::Field(F::EapCa, t), None));
        c = c.push(input(&p, "Server's domain (optional, checks it's really your network), like radius.school.edu", s.g(F::EapDomain), false, |t| M::Field(F::EapDomain, t), None));
        if k == "tls" {
            c = c.push(input(&p, "Your certificate file, like ~/Documents/me.crt", s.g(F::EapCert), false, |t| M::Field(F::EapCert, t), None));
            c = c.push(input(&p, "Its private key file, like ~/Documents/me.key", s.g(F::EapKey), false, |t| M::Field(F::EapKey, t), None));
            c = c.push(input(&p, "The private key's password (if it has one)", s.g(F::KeyPw), true, |t| M::Field(F::KeyPw, t), None));
        } else {
            if !s.eap_ask {
                c = c.push(input(&p, "Password (NetworkManager keeps it, not Hearth)", s.g(F::EapPw), true, |t| M::Field(F::EapPw, t), None));
            }
            c = c.push(srow(&p, G::Shield, "Ask for the password each time", "Off: NetworkManager keeps it, so it connects by itself", switch(&p, s.eap_ask, Some(M::EapAsk))));
        }
        c = c.push(press(&p, G::Check, if s.what == "eap" { "Saving…" } else if for_new { "Add this network" } else { "Save the sign-in" }, "", Some(M::EapSave)));
        if !for_new && s.dkind() == "ethernet" && ent.is_object() {
            c = c.push(danger_row(&p, G::Close, "Turn 802.1X off for this cable", "", Some(M::EapOff)));
        }
    }

    let Some(d) = &s.d else {
        return c.push(said(&p, &s.say)).into();
    };
    let name = kit::st(d, "name");
    let kind = kit::st(d, "kind");
    // ---- what it has now
    c = c.push(head(&p, format!("{name} now")));
    let addr = kit::st(d, "now.address");
    let dns_now = kit::strs(d, "now.dns");
    let mut parts = vec![format!("Gateway {}", { let g = kit::st(d, "now.gateway"); if g.is_empty() { "–".to_string() } else { g } }), format!("DNS {}", if dns_now.is_empty() { "–".to_string() } else { dns_now.join(", ") })];
    let a6 = kit::st(d, "now.address6");
    if !a6.is_empty() {
        parts.push(format!("IPv6 {a6}"));
    }
    c = c.push(info_row(&p, G::Info, if addr.is_empty() { "Not connected".to_string() } else { addr }, parts.join(" · ")));

    // ---- share this Wi-Fi network: a QR code a phone's camera joins from, made only when you press.
    if kind == "wifi" && kit::st(d, "security") != "enterprise" {
        c = c.push(head(&p, format!("Share {name}")));
        match &s.qr {
            None => {
                let has_qr = s.st.as_ref().map(|v| kit::bo(v, "qr")).unwrap_or(true);
                c = c.push(press(&p, G::Grid, if s.what == "qr" { "Making the code…" } else { "Show a QR code" }, if !has_qr { "Needs qrencode, which isn't on this system" } else { "Point a phone's camera at it to join this network. It shows the password" }, Some(M::Qr)));
            }
            Some(q) => {
                let rows: Vec<Vec<bool>> = kit::strs(q, "rows").iter().map(|r| r.chars().map(|ch| ch == '1').collect()).collect();
                let pw = kit::st(q, "password");
                let sec = kit::st(q, "security");
                let code = container(canvas::Canvas::new(Qr { rows }).width(Length::Fixed(224.0)).height(Length::Fixed(224.0))).padding(18.0).style(hearth_ui::ui::boxed(Color::WHITE, 16.0, None));
                let mut info = Column::new().spacing(8.0).push(tb(kit::st(q, "ssid"), sz(22.0), p.text)).push(t(if sec == "nopass" { "Open network: no password".to_string() } else { format!("{sec} · scan with the phone's camera") }, sz(13.0), p.muted));
                if !pw.is_empty() {
                    info = info.push(t(if s.show_pw { format!("Password: {pw}") } else { format!("Password: {}", "•".repeat(10)) }, sz(15.0), p.text));
                }
                let mut b = iced::widget::Row::new().spacing(10.0);
                if !pw.is_empty() {
                    b = b.push(btn(&p, if s.show_pw { "Hide the password" } else { "Show the password" }, Kind::Normal, false, Some(M::ShowPw)));
                }
                b = b.push(btn(&p, "Hide the code", Kind::Normal, false, Some(M::HideQr)));
                c = c.push(card(&p, row![code, info.push(b).width(Length::Fill)].spacing(22.0).align_y(iced::Center)));
            }
        }
    }

    // ---- mobile broadband: APN, user name, password, SIM PIN, roaming.
    if kind == "mobile" {
        let mob = kit::at(d, "mobile");
        c = c.push(head(&p, "Mobile broadband"));
        let apn = kit::st(mob, "apn");
        let user = kit::st(mob, "user");
        c = c.push(info_row(&p, G::Server, format!("APN: {}", if apn.is_empty() { "automatic".to_string() } else { apn }), format!("{}{}", if user.is_empty() { String::new() } else { format!("User {user} · ") }, if kit::bo(mob, "roaming") { "Used when roaming" } else { "Home network only" })));
        c = c.push(input(&p, "New APN, like internet", s.g(F::NApn), false, |t| M::Field(F::NApn, t), None));
        c = c.push(input(&p, "User name (most providers have none)", s.g(F::NUser), false, |t| M::Field(F::NUser, t), None));
        c = c.push(input(&p, "Password (NetworkManager keeps it)", s.g(F::NPw), true, |t| M::Field(F::NPw, t), None));
        c = c.push(input(&p, "SIM PIN (4 to 8 numbers)", s.g(F::NPin), true, |t| M::Field(F::NPin, t), None));
        c = c.push(press(&p, G::Check, if s.what == "set" { "Saving…" } else { "Save" }, "Only what you typed changes", Some(M::MobSave)));
        let roam = kit::bo(mob, "roaming");
        c = c.push(srow(&p, G::Link, "Use it when roaming", "Abroad or on another network: it can cost more", switch(&p, roam, Some(M::MobRoam(!roam)))));
    }

    // ---- its settings
    c = c.push(head(&p, format!("Settings for {name}")));
    let auto = kit::bo(d, "autoconnect");
    c = c.push(srow(&p, G::Link, "Connect automatically", "When it's in reach", switch(&p, auto, Some(M::Change(json!({ "autoconnect": !auto }), if auto { "Connects only when you pick it.".into() } else { "Connects by itself when in reach.".into() })))));
    if kind != "vpn" {
        let me = kit::st(d, "metered");
        let cur = if me == "yes" || me == "no" { me } else { "unknown".to_string() };
        c = c.push(choice(&p, G::Pulse, "Metered connection", "Updates and big downloads wait on a metered network (phone hotspots)", [("unknown", "Let Hearth guess"), ("yes", "Metered"), ("no", "Not metered")].iter().map(|&(v, l)| (l.to_string(), cur == v, Some(M::Change(json!({ "metered": v }), "Saved.".into())))).collect()));
    }
    let mac = kit::st(d, "mac");
    if !mac.is_empty() {
        let mut opts: Vec<(&'static str, String)> = vec![("default", "Hearth's default".into()), ("permanent", "The real one".into()), ("stable", "Random, the same for this network".into()), ("random", "Random each time".into()), ("preserve", "Keep what the device has".into())];
        if mac == "fixed" {
            opts.push(("fixed", format!("Typed in: {}", kit::st(d, "macFixed"))));
        }
        let cur = opts.iter().find(|o| o.0 == mac).map(|o| o.1.clone()).unwrap_or_else(|| mac.clone());
        c = c.push(dropdown(&p, G::Shield, "MAC address", "What this PC tells the network it is. Random is more private", cur, s.mac_open, M::MacOpen, opts.into_iter().map(|(v, l)| (l, v == mac, M::Mac(v))).collect()));
    }
    let auto_dns = kit::bo(d, "autoDns");
    c = c.push(srow(&p, G::Link, "Automatic DNS", "The DNS servers the network hands out. Off: only yours below", switch(&p, auto_dns, Some(M::Change(json!({ "autoDns": !auto_dns }), "Saved.".into())))));
    let mine = kit::strs(d, "dns");
    c = c.push(input(&p, &if mine.is_empty() { "Your own DNS servers, like 1.1.1.1, 9.9.9.9".to_string() } else { format!("Your DNS: {}", mine.join(", ")) }, s.g(F::Dns), false, |t| M::Field(F::Dns, t), Some(M::UseDns)));
    c = c.push(press(&p, G::Check, "Use these DNS servers", if mine.is_empty() { "Split them with commas" } else { "Empty: back to none of your own" }, Some(M::UseDns)));
    if kind != "vpn" {
        let manual = kit::st(d, "ipv4.method") == "manual";
        let gw = kit::st(d, "ipv4.gateway");
        let sub = if manual { format!("{}{}", kit::st(d, "ipv4.address"), if gw.is_empty() { String::new() } else { format!(" via {gw}") }) } else { "Handed out by the router".into() };
        c = c.push(choice(&p, G::Gear, "IP address (IPv4)", sub, vec![("Automatic".into(), !manual && !s.manual, Some(M::Ipv4("auto"))), ("By hand".into(), manual || s.manual, Some(M::Ipv4("manual")))]));
        if s.manual {
            c = c.push(input(&p, "Address, like 192.168.1.20/24", s.g(F::Addr4), false, |t| M::Field(F::Addr4, t), None));
            c = c.push(input(&p, "Gateway (your router), like 192.168.1.1", s.g(F::Gw4), false, |t| M::Field(F::Gw4, t), Some(M::UseAddr)));
            c = c.push(press(&p, G::Check, "Use this address", "", Some(M::UseAddr)));
        }
        let v6 = kit::st(d, "ipv6");
        let cur6 = if v6 == "disabled" || v6 == "ignore" { v6 } else { "auto".to_string() };
        c = c.push(choice(&p, G::Link, "IPv6", "", [("auto", "On"), ("ignore", "Leave it"), ("disabled", "Off")].iter().map(|&(v, l)| (l.to_string(), cur6 == v, Some(M::Change(json!({ "ipv6": v }), "Saved.".into())))).collect()));
    }
    if kind == "wifi" || kind == "ethernet" {
        let mtu = kit::nu(d, "mtu").unwrap_or(0.0) as u64;
        c = c.push(input(&p, &if mtu > 0 { format!("Packet size (MTU): {mtu}. Type a new one, or 0 for automatic") } else { "Packet size (MTU): automatic. Type one, like 1492, only if your provider says so".to_string() }, s.g(F::Mtu), false, |t| M::Field(F::Mtu, t), Some(M::UseMtu)));
        if !s.g(F::Mtu).trim().is_empty() {
            c = c.push(press(&p, G::Check, "Use this packet size", "", Some(M::UseMtu)));
        }
    }
    if kind == "ethernet" || (kind == "wifi" && kit::st(d, "security") == "enterprise") {
        let e = kit::at(d, "enterprise");
        let on = e.is_object();
        c = c.push(go_row(&p, G::Shield, if on { format!("Enterprise sign-in (802.1X): {}", kit::st(e, "eap").to_uppercase()) } else { "Enterprise sign-in (802.1X)".into() }, if on { format!("Signs in as {}. Change it", kit::st(e, "identity")) } else { "Off. For networks at work or school that ask for your account".into() }, M::EditEap));
    }
    c = c.push(press(&p, G::Plus, if s.what == "dup" { "Copying…" } else { "Duplicate this connection" }, "A copy to change (it connects only when you pick it)", Some(M::Dup)));
    c.push(said(&p, &s.say)).into()
}

pub fn subscription(_s: &S) -> Subscription<M> {
    Subscription::none()
}

pub fn back(s: &mut S) -> bool {
    if s.qr.is_some() {
        s.qr = None;
        s.show_pw = false;
        return true;
    }
    if s.mac_open || s.wired_open {
        s.mac_open = false;
        s.wired_open = false;
        return true;
    }
    false
}

//! Network & internet › VPN (Mantel's SetVpn): your VPNs (kept by NetworkManager), connect and disconnect, add one from
//! its file (.conf or .ovpn, the file your VPN provider gives you), remove one, or set one up by hand: IPsec,
//! L2TP/IPsec, PPTP, OpenConnect, SSTP and Fortinet, each only when NetworkManager's plugin for it is installed (the
//! others say which plugin they need). Passwords go to NetworkManager on hearth-network's stdin and NetworkManager
//! keeps them: Hearth never does. Nothing here changes by itself.

use super::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::G;
use iced::widget::{column, Column};
use iced::{Subscription, Task};
use serde_json::{json, Value};

const H: &str = "hearth-network";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum F {
    Name,
    Server,
    User,
    Pass,
    Psk,
    Group,
    Domain,
    AcctUser,
    AcctPass,
    Path,
}

#[derive(Default)]
pub struct S {
    pub st: Option<Value>,
    pub picked: String,
    pub asking: String,
    pub confirm: String,
    pub adding: String,
    pub proto: String,
    pub proto_open: bool,
    pub what: &'static str,
    pub f: [String; 10],
    pub say: Say,
    pub live: Live,
}

#[derive(Debug, Clone)]
pub enum M {
    Status(Res),
    Pick(String),
    Up(String),
    Upped(String, String, Res),
    Down(String),
    Confirm(String),
    Remove(String),
    Done(String, Res),
    Field(F, String),
    Account(String),
    PickFile,
    Picked(hearth_ui::run::Output),
    AddPath,
    Imported(Res),
    Adding(String),
    ProtoOpen,
    Proto(&'static str),
    AddByHand,
    Added(Res),
    Go(&'static str),
    Tick,
}

fn idx(f: F) -> usize {
    f as usize
}

impl S {
    fn get(&self, f: F) -> &str {
        &self.f[idx(f)]
    }
    fn vpns(&self) -> &[Value] {
        self.st.as_ref().map(|v| kit::ar(v, "vpns")).unwrap_or(&[])
    }
    fn types(&self) -> &[Value] {
        self.st.as_ref().map(|v| kit::ar(v, "vpnTypes")).unwrap_or(&[])
    }
    fn add_type(&self) -> Option<&Value> {
        self.types().iter().find(|t| kit::st(t, "id") == self.adding)
    }
}

fn status() -> Task<M> {
    Task::perform(kit::job(H, kit::a(&["status"]), String::new()), M::Status)
}

pub const PROTOS: &[(&str, &str)] = &[("anyconnect", "Cisco AnyConnect"), ("gp", "GlobalProtect"), ("pulse", "Pulse / Ivanti"), ("nc", "Juniper"), ("f5", "F5"), ("fortinet", "Fortinet")];

pub fn open(_sh: &mut Shared, s: &mut S, part: &str) -> Task<M> {
    s.say.clear();
    s.picked.clear();
    s.confirm.clear();
    s.asking.clear();
    s.adding = part.strip_prefix("add-").unwrap_or("").to_string();
    if s.proto.is_empty() {
        s.proto = "anyconnect".into();
    }
    s.live.start("vpn");
    status()
}

pub fn update(sh: &mut Shared, s: &mut S, m: M) -> Task<M> {
    match m {
        M::Status(r) => match r {
            Res::Ok(v) => s.st = Some(v),
            Res::Err(e) => s.say.bad(e),
        },
        M::Pick(u) => {
            s.picked = if s.picked == u { String::new() } else { u };
            s.confirm.clear();
        }
        M::Up(u) => return up(s, u, None),
        M::Account(u) => {
            let user = s.get(F::AcctUser).to_string();
            let pw = std::mem::take(&mut s.f[idx(F::AcctPass)]);
            return up(s, u, Some((user, pw)));
        }
        M::Upped(u, name, r) => {
            s.what = "";
            match r {
                Res::Ok(_) => {
                    s.asking.clear();
                    s.say.ok(format!("Connected to {name}."));
                }
                Res::Err(e) => {
                    s.say.bad(e);
                    let wg = s.vpns().iter().find(|v| kit::st(v, "uuid") == u).map(|v| kit::st(v, "vpnType") == "WireGuard").unwrap_or(false);
                    if !wg {
                        s.asking = u;
                    }
                }
            }
            return status();
        }
        M::Down(u) => {
            s.what = "down";
            return Task::perform(kit::job(H, vec!["vpn-down".into(), u], String::new()), |r| M::Done("Disconnected.".into(), r));
        }
        M::Confirm(u) => s.confirm = u,
        M::Remove(u) => {
            s.what = "remove";
            s.picked.clear();
            s.confirm.clear();
            return Task::perform(kit::job(H, vec!["vpn-remove".into(), u], String::new()), |r| M::Done("VPN removed.".into(), r));
        }
        M::Done(ok, r) => {
            s.what = "";
            s.say.res(&r, &ok);
            return status();
        }
        M::Field(f, t) => s.f[idx(f)] = t,
        M::PickFile => {
            let home = std::env::var("HOME").unwrap_or_default();
            return Task::perform(kit::exec(kit::pick_cmd("Add a VPN", false, &format!("{home}/Downloads"), "VPN files (*.conf *.ovpn)"), String::new(), 600), M::Picked);
        }
        M::Picked(o) => {
            if o.code == 3 {
                s.say.bad("No file picker here: type the file's place below instead.");
            } else if let Some(f) = o.out.lines().next().map(|l| l.trim().to_string()).filter(|f| !f.is_empty()) {
                return import(s, f);
            }
        }
        M::AddPath => {
            let home = std::env::var("HOME").unwrap_or_default();
            let mut f = s.get(F::Path).trim().to_string();
            if let Some(rest) = f.strip_prefix("~/") {
                f = format!("{home}/{rest}");
            }
            if !f.is_empty() {
                s.f[idx(F::Path)].clear();
                return import(s, f);
            }
        }
        M::Imported(r) => {
            s.what = "";
            match r {
                Res::Ok(v) => s.say.ok(format!("{} is added. Connect it below.", kit::st(&v, "name"))),
                Res::Err(e) => s.say.bad(e),
            }
            return status();
        }
        M::Adding(id) => {
            s.adding = if s.adding == id { String::new() } else { id };
            s.say.clear();
        }
        M::ProtoOpen => s.proto_open = !s.proto_open,
        M::Proto(p) => {
            s.proto = p.to_string();
            s.proto_open = false;
        }
        M::AddByHand => {
            s.say.clear();
            s.what = "add";
            let g = |f: F| s.get(f).trim().to_string();
            let req = json!({
                "type": s.adding, "name": g(F::Name), "gateway": g(F::Server), "user": g(F::User),
                "password": s.get(F::Pass), "psk": s.get(F::Psk), "group": g(F::Group), "domain": g(F::Domain),
                "realm": g(F::Domain), "protocol": s.proto,
            });
            return Task::perform(kit::job(H, kit::a(&["vpn-add"]), req.to_string()), M::Added);
        }
        M::Added(r) => {
            s.what = "";
            match r {
                Res::Ok(v) => {
                    let w = kit::st(&v, "warning");
                    s.say.ok(format!("{} is added{}", kit::st(&v, "name"), if w.is_empty() { ". Connect it above.".to_string() } else { format!(". {w}") }));
                    s.adding.clear();
                    s.f[idx(F::Pass)].clear();
                    s.f[idx(F::Psk)].clear();
                }
                Res::Err(e) => s.say.bad(e),
            }
            return status();
        }
        M::Go(id) => sh.go(id),
        M::Tick => {
            if s.live.still(10) && s.what.is_empty() && s.asking.is_empty() {
                return status();
            }
        }
    }
    Task::none()
}

fn up(s: &mut S, u: String, acct: Option<(String, String)>) -> Task<M> {
    s.say.clear();
    s.what = "up";
    let name = s.vpns().iter().find(|v| kit::st(v, "uuid") == u).map(|v| kit::st(v, "name")).unwrap_or_default();
    let input = acct.map(|(user, pw)| json!({ "user": user, "password": pw }).to_string()).unwrap_or_default();
    Task::perform(kit::job(H, vec!["vpn-up".into(), u.clone()], input), move |r| M::Upped(u.clone(), name.clone(), r))
}

fn import(s: &mut S, f: String) -> Task<M> {
    s.say.clear();
    s.what = "import";
    Task::perform(kit::job(H, vec!["vpn-import".into(), f], String::new()), M::Imported)
}

pub fn view<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    s.live.seen();
    let mut c = Column::new().spacing(10.0).push(head(&p, "Your VPNs"));
    for v in s.vpns() {
        let uuid = kit::st(v, "uuid");
        let name = kit::st(v, "name");
        let typ = kit::st(v, "vpnType");
        let active = kit::bo(v, "active");
        let dev = kit::st(v, "device");
        let sub = format!("{typ}{}", if active { format!(" · Connected{}", if dev.is_empty() { String::new() } else { format!(" ({dev})") }) } else { " · Not connected".into() });
        c = c.push(chip_row(&p, G::Shield, name.clone(), sub, if active { "On" } else { "" }, "ok", Some(M::Pick(uuid.clone()))));
        if s.picked == uuid {
            let mut x = Column::new().spacing(8.0);
            if !active {
                x = x.push(press(&p, G::Link, if s.what == "up" { "Connecting…" } else { "Connect" }, if typ != "WireGuard" { "Asks for your VPN account if it needs one" } else { "" }, Some(M::Up(uuid.clone()))));
            } else {
                x = x.push(press(&p, G::Close, "Disconnect", "", Some(M::Down(uuid.clone()))));
            }
            x = x.push(go_row(&p, G::Gear, "Details", "DNS, auto-connect and addresses: in Connection details", M::Go("net-details")));
            if s.confirm != uuid {
                x = x.push(danger_row(&p, G::Close, "Remove this VPN…", "", Some(M::Confirm(uuid.clone()))));
            } else {
                x = x.push(danger_row(&p, G::Close, format!("Yes, remove {name}"), "", Some(M::Remove(uuid.clone()))));
            }
            // OpenVPN's account, when it asks: straight to NetworkManager, never saved by Hearth.
            if s.asking == uuid {
                x = x.push(input(&p, "VPN user name (if it has one)", s.get(F::AcctUser), false, |t| M::Field(F::AcctUser, t), None));
                x = x.push(input(&p, "VPN password", s.get(F::AcctPass), true, |t| M::Field(F::AcctPass, t), Some(M::Account(uuid.clone()))));
                x = x.push(press(&p, G::Shield, "Connect with this account", "Handed to NetworkManager only; Hearth doesn't keep it", Some(M::Account(uuid.clone()))));
            }
            c = c.push(indent(x));
        }
    }
    if s.st.is_some() && s.vpns().is_empty() {
        c = c.push(words(&p, "No VPNs yet. Add one from the file your VPN gives you (WireGuard .conf or OpenVPN .ovpn)."));
    }
    let openvpn = s.st.as_ref().map(|v| kit::bo(v, "plugins.openvpn")).unwrap_or(false);
    c = c.push(head(&p, "Add a VPN"));
    c = c.push(press(&p, G::Plus, if s.what == "import" { "Adding…" } else { "Add from a file…" }, format!("WireGuard (.conf){}, from your VPN provider or router", if openvpn { " or OpenVPN (.ovpn)" } else { "" }), Some(M::PickFile)));
    c = c.push(input(&p, "Or the file's place, like ~/Downloads/home.conf", s.get(F::Path), false, |t| M::Field(F::Path, t), Some(M::AddPath)));
    c = c.push(press(&p, G::Folder, "Add this file", "", Some(M::AddPath)));
    if s.st.as_ref().map(|v| kit::has(v, "plugins")).unwrap_or(false) && !openvpn {
        c = c.push(info_row(&p, G::Info, "OpenVPN isn't on this system", "WireGuard works. OpenVPN needs NetworkManager's OpenVPN plugin in the image"));
    }
    // A VPN typed in by hand: every kind is listed; the ones whose plugin isn't installed say so and can't be picked.
    c = c.push(head(&p, "Set up a VPN by hand"));
    for ty in s.types() {
        let id = kit::st(ty, "id");
        let label = kit::st(ty, "label");
        if kit::bo(ty, "installed") {
            let on = s.adding == id;
            c = c.push(val_row(&p, G::Shield, label, if on { "Fill in the details below" } else { "Server, user name and password from your work or VPN provider" }, if on { "Setting up" } else { "" }, Some(M::Adding(id))));
        } else {
            c = c.push(info_row(&p, G::Info, label, format!("Not on this system: needs NetworkManager's {} plugin", kit::st(ty, "package"))));
        }
    }
    if let Some(ty) = s.add_type().filter(|t| kit::bo(t, "installed")) {
        let kind = s.adding.as_str();
        let plugin = kit::st(ty, "plugin");
        let oc = kind == "openconnect";
        let mut x = Column::new().spacing(8.0);
        x = x.push(input(&p, "A name for it, like Work VPN", s.get(F::Name), false, |t| M::Field(F::Name, t), None));
        x = x.push(input(&p, if oc { "Server, like vpn.example.com or https://vpn.example.com/group" } else { "Server (gateway), like vpn.example.com" }, s.get(F::Server), false, |t| M::Field(F::Server, t), None));
        if oc {
            let cur = PROTOS.iter().find(|x| x.0 == s.proto).map(|x| x.1).unwrap_or("Cisco AnyConnect");
            x = x.push(dropdown(&p, G::Link, "Kind of server", "", cur, s.proto_open, M::ProtoOpen, PROTOS.iter().map(|&(v, l)| (l.to_string(), v == s.proto, M::Proto(v))).collect()));
        } else {
            x = x.push(input(&p, if kind == "ipsec" { "User name (leave empty for a pre-shared key only)" } else { "User name" }, s.get(F::User), false, |t| M::Field(F::User, t), None));
            x = x.push(input(&p, "Password (NetworkManager keeps it, not Hearth)", s.get(F::Pass), true, |t| M::Field(F::Pass, t), None));
        }
        if kind == "ipsec" || kind == "l2tp" {
            x = x.push(input(&p, if kind == "l2tp" { "IPsec pre-shared key (if your VPN uses one)" } else { "Pre-shared key (if your VPN uses one)" }, s.get(F::Psk), true, |t| M::Field(F::Psk, t), None));
        }
        if oc || plugin == "libreswan" {
            x = x.push(input(&p, if oc { "Group (if your server has several; optional)" } else { "Group name (optional)" }, s.get(F::Group), false, |t| M::Field(F::Group, t), None));
        }
        if kind == "pptp" || kind == "fortisslvpn" {
            x = x.push(input(&p, if kind == "pptp" { "Windows domain (optional)" } else { "Realm (optional)" }, s.get(F::Domain), false, |t| M::Field(F::Domain, t), None));
        }
        if oc {
            x = x.push(info_row(&p, G::Info, "Your user name and password are asked when you connect", "OpenConnect signs in on the server's own sign-in page each time"));
        }
        x = x.push(press(&p, G::Plus, if s.what == "add" { "Adding…" } else { "Add this VPN" }, "It connects only when you press Connect", Some(M::AddByHand)));
        x = x.push(said(&p, &s.say));
        c = c.push(indent(x));
    } else {
        c = c.push(said(&p, &s.say));
    }
    column![c].into()
}

pub fn subscription(s: &S) -> Subscription<M> {
    if s.live.running("vpn") && s.asking.is_empty() {
        iced::time::every(std::time::Duration::from_secs(10)).map(|_| M::Tick)
    } else {
        Subscription::none()
    }
}

pub fn back(s: &mut S) -> bool {
    if !s.asking.is_empty() {
        s.asking.clear();
        return true;
    }
    if !s.confirm.is_empty() {
        s.confirm.clear();
        return true;
    }
    false
}

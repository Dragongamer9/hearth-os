//! Bluetooth › Your Bluetooth devices and Receive files & this PC's name (Mantel's SetBtDevices and SetBtReceive).
//! Devices: each paired one with its battery, Trust (connects by itself without asking), Block (never connects, even if
//! it tries), and Send a file for phones and computers (OBEX through BlueZ: hearth-bt). BlueZ remembers trust and
//! blocks itself; Hearth keeps no file. Receive: the name phones and other computers see (BlueZ's own,
//! hearth-bt-receive rename; Put back gives BlueZ's usual one), Receive files (off by default; each file is asked
//! about), where they go, taking files from trusted devices without asking (Hearth's desktop.json, through
//! hearth_settings_io), and what came lately. Opening either page only reads.

use crate::cats::net::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::run;
use hearth_ui::ui::G;
use iced::widget::Column;
use iced::{Subscription, Task};
use serde_json::{json, Value};

#[derive(Default)]
pub struct S {
    pub list: Option<Vec<Value>>,
    pub open: String,
    pub sending: String,
    pub say: Say,
    pub live: Live,
    // receive
    pub name: Option<Value>,
    pub new_name: String,
    pub items: Vec<Value>,
    pub rsay: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Devices(Res),
    Open(String),
    Trust(String, bool),
    Block(String, bool),
    Done(Res),
    Send(String),
    Picked(String, run::Output),
    Sent(String, Res),
    Tick,
    Go(&'static str),
    Name(Res),
    NewName(String),
    Rename,
    Renamed(Res, &'static str),
    Reset,
    Receive(bool),
    Trusted(bool),
    Folder,
    FolderPicked(run::Output),
    Downloads,
    Saved(run::Output),
    OpenFile(String),
}

fn devices() -> Task<M> {
    Task::perform(kit::job("hearth-bt", kit::a(&["devices"]), String::new()), M::Devices)
}

fn received() -> Vec<Value> {
    let home = std::env::var("HOME").unwrap_or_default();
    let v = hearth_ui::settings_file::read_json(&std::path::PathBuf::from(format!("{home}/.local/state/nimbus/bt-received.json")));
    kit::ar(&v, "items").to_vec()
}

pub fn open(_sh: &mut Shared, s: &mut S, page: &'static str, part: &str) -> Task<M> {
    if page == "bluetooth-receive" {
        s.rsay.clear();
        s.items = received();
        s.live.stop();
        return Task::perform(kit::job("hearth-bt-receive", kit::a(&["name"]), String::new()), M::Name);
    }
    s.say.clear();
    s.open = part.to_string();
    s.live.start("bluetooth-devices");
    devices()
}

pub fn update(sh: &mut Shared, s: &mut S, m: M) -> Task<M> {
    match m {
        M::Devices(r) => match r {
            Res::Ok(v) => {
                let l = kit::ar(&v, "devices").to_vec();
                if s.open == "first" {
                    s.open = l.first().map(|d| kit::st(d, "mac")).unwrap_or_default();
                }
                s.list = Some(l);
            }
            Res::Err(e) => {
                s.list = Some(Vec::new());
                s.say.bad(e);
            }
        },
        M::Open(mac) => s.open = if s.open == mac { String::new() } else { mac },
        M::Trust(mac, on) => return Task::perform(kit::job("hearth-bt", vec![if on { "trust" } else { "untrust" }.into(), mac], String::new()), M::Done),
        M::Block(mac, on) => return Task::perform(kit::job("hearth-bt", vec![if on { "block" } else { "unblock" }.into(), mac], String::new()), M::Done),
        M::Done(r) => {
            if let Res::Err(e) = r {
                s.say.bad(e);
            }
            return devices();
        }
        M::Send(mac) => {
            if !s.sending.is_empty() {
                return Task::none();
            }
            s.sending = mac.clone();
            let home = std::env::var("HOME").unwrap_or_default();
            return Task::perform(kit::exec(kit::pick_cmd("Send a file", false, &home, ""), String::new(), 600), move |o| M::Picked(mac.clone(), o));
        }
        M::Picked(mac, o) => {
            if o.code == 3 {
                s.say.bad("No file picker installed");
                s.sending.clear();
                return Task::none();
            }
            let f = o.out.lines().next().unwrap_or("").trim().to_string();
            if f.is_empty() {
                s.sending.clear();
                return Task::none();
            }
            s.say.clear();
            return Task::perform(kit::job_for("hearth-bt", vec!["send".into(), mac, f.clone()], String::new(), 1800), move |r| M::Sent(f.clone(), r));
        }
        M::Sent(f, r) => {
            s.sending.clear();
            let name = f.rsplit('/').next().unwrap_or(&f).to_string();
            s.say.res(&r, &format!("{name} sent."));
        }
        M::Tick => {
            if s.live.still(8) && s.sending.is_empty() {
                return devices();
            }
        }
        M::Go(id) => sh.go(id),
        M::Name(r) => match r {
            Res::Ok(v) => s.name = Some(v),
            Res::Err(e) => s.rsay.bad(e),
        },
        M::NewName(t) => s.new_name = t,
        M::Rename => {
            let n = s.new_name.trim().to_string();
            if n.is_empty() {
                return Task::none();
            }
            s.rsay.clear();
            return Task::perform(kit::job("hearth-bt-receive", vec!["rename".into(), n], String::new()), |r| M::Renamed(r, "Renamed. Devices see the new name the next time they look."));
        }
        M::Renamed(r, ok) => match r {
            Res::Ok(v) => {
                s.name = Some(v);
                s.new_name.clear();
                s.rsay.ok(ok);
            }
            Res::Err(e) => s.rsay.bad(e),
        },
        M::Reset => return Task::perform(kit::job("hearth-bt-receive", kit::a(&["reset-name"]), String::new()), |r| M::Renamed(r, "The usual name is back.")),
        M::Receive(on) => return save(sh, json!({ "btReceive": on })),
        M::Trusted(on) => return save(sh, json!({ "btReceiveTrusted": on })),
        M::Folder => {
            let home = std::env::var("HOME").unwrap_or_default();
            return Task::perform(kit::exec(kit::pick_cmd("Save received files in", true, &home, ""), String::new(), 600), M::FolderPicked);
        }
        M::FolderPicked(o) => {
            if o.code == 3 {
                s.rsay.bad("No folder picker installed");
                return Task::none();
            }
            let f = o.out.lines().next().unwrap_or("").trim().to_string();
            if !f.is_empty() {
                return save(sh, json!({ "btReceiveFolder": f }));
            }
        }
        M::Downloads => return save(sh, json!({ "btReceiveFolder": "" })),
        M::Saved(o) => sh.result("Couldn't save that", &o),
        M::OpenFile(path) => run::detached(&["xdg-open", path.as_str()]),
    }
    Task::none()
}

/// Hearth's own settings (desktop.json) change only through hearth_settings_io: merged, atomic, with a .bak.
fn save(sh: &mut Shared, v: Value) -> Task<M> {
    let cmd = sh.desktop_cmd(v);
    Task::perform(run::spawn(cmd), M::Saved)
}

fn icon_for(i: &str) -> G {
    if i.contains("phone") {
        G::Pad
    } else if i.contains("audio") || i.contains("head") {
        G::Sound
    } else if i.contains("input-gaming") {
        G::Pad
    } else if i.contains("input-mouse") || i.contains("input-keyboard") {
        G::Grid
    } else if i.contains("computer") {
        G::Display
    } else {
        G::Bt
    }
}

pub fn view<'a>(sh: &'a Shared, s: &'a S, page: &'static str) -> E<'a, M> {
    if page == "bluetooth-receive" {
        return receive(sh, s);
    }
    let p = sh.p;
    s.live.seen();
    let list = s.list.as_deref().unwrap_or(&[]);
    let mut c = crate::secs::Plain::new(0).spacing(10.0);
    if s.list.is_none() {
        c = c.push(words(&p, "Reading your devices…"));
    }
    c = c.push(head(&p, if list.is_empty() { "No paired devices" } else { "Paired" }));
    if s.list.is_some() && list.is_empty() {
        c = c.push(go_row(&p, G::Bt, "Pair a device first", "Bluetooth › Add a device", M::Go("bluetooth")));
    }
    for d in list {
        let mac = kit::st(d, "mac");
        let trusted = kit::bo(d, "trusted");
        let blocked = kit::bo(d, "blocked");
        let sub = format!("{}{}{}", if kit::bo(d, "connected") { "Connected" } else { "Not connected" }, if trusted { " · trusted" } else { "" }, if blocked { " · blocked" } else { "" });
        let batt = kit::nu(d, "battery").map(|b| format!("{}%", b.round() as i64)).unwrap_or_default();
        let open = s.open == mac;
        let body = if open {
            let mut b = Column::new().spacing(8.0);
            b = b.push(srow(&p, G::Check, "Trust", "Connects by itself when it's near, without asking", switch(&p, trusted, Some(M::Trust(mac.clone(), !trusted)))));
            b = b.push(srow(&p, G::Shield, "Block", "Never connects, even when it tries", switch(&p, blocked, Some(M::Block(mac.clone(), !blocked)))));
            if kit::bo(d, "canSend") {
                let busy = s.sending == mac;
                b = b.push(press(&p, G::Arrow, if busy { "Sending…" } else { "Send a file…" }, "The other device asks whether to take it", if s.sending.is_empty() { Some(M::Send(mac.clone())) } else { None }));
                if busy {
                    b = b.push(level_bar(&p, 0.35, p.accent));
                }
            }
            Some(b.into())
        } else {
            None
        };
        c = c.push(expander(&p, icon_for(&kit::st(d, "icon")), kit::st(d, "name"), sub, &batt, Some((if blocked { "Blocked" } else { "" }, "bad")), open, M::Open(mac.clone()), body));
    }
    c = c.push(go_row(&p, G::Arrow, "Receive files & this PC's name", "Files from phones, and the name others see", M::Go("bluetooth-receive")));
    c.push(said(&p, &s.say)).into()
}

fn receive<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let mut c = crate::secs::Plain::new(0).spacing(10.0);
    if let Some(b) = &s.name {
        if !kit::bo(b, "present") {
            c = c.push(banner(&p, G::Bt, "No Bluetooth on this PC", "Plug in a Bluetooth adapter, or turn it on in the PC's firmware setup", false, None));
        }
    }
    c = c.push(head(&p, "This PC"));
    let (title, sub, chip, kind) = match &s.name {
        None => ("Reading…".to_string(), "What phones and other computers see".to_string(), "", "off"),
        Some(b) => {
            let n = kit::st(b, "name");
            let addr = kit::st(b, "address");
            let present = kit::bo(b, "present");
            let on = kit::bo(b, "powered");
            (if n.is_empty() { "No name".into() } else { n }, if addr.is_empty() { "What phones and other computers see".into() } else { format!("What phones and other computers see · {addr}") }, if present { if on { "On" } else { "Off" } } else { "" }, if on { "ok" } else { "warn" })
        }
    };
    c = c.push(chip_row(&p, G::Display, title, sub, chip, kind, None));
    c = c.push(input(&p, "A new name, like Living room PC", &s.new_name, false, M::NewName, Some(M::Rename)));
    if let Some(b) = &s.name {
        let sys = kit::st(b, "systemName");
        if !sys.is_empty() && sys != kit::st(b, "name") {
            c = c.push(press(&p, G::Restore, "Put back the usual name", sys, Some(M::Reset)));
        }
    }
    c = c.push(head(&p, "Receiving files"));
    let on = sh.d_bool("btReceive", false);
    c = c.push(srow(&p, G::Arrow, "Receive files", "Phones and computers you've paired can send you files. You're asked about each one", switch(&p, on, Some(M::Receive(!on)))));
    let folder = sh.d_str("btReceiveFolder");
    c = c.push(go_row(&p, G::Folder, "Save them in", if folder.is_empty() { "~/Downloads".to_string() } else { kit::tilde(&folder) }, M::Folder));
    if !folder.is_empty() {
        c = c.push(press(&p, G::Restore, "Back to Downloads", "", Some(M::Downloads)));
    }
    let tr = sh.d_bool("btReceiveTrusted", false);
    c = c.push(srow(&p, G::Check, "Take files from trusted devices without asking", "Devices you trust in Your Bluetooth devices. Others are still asked about", switch(&p, tr, Some(M::Trusted(!tr)))));
    c = c.push(head(&p, if s.items.is_empty() { "Nothing received yet" } else { "Received lately" }));
    for it in s.items.iter().take(10) {
        let name = kit::st(it, "name");
        let lower = name.to_lowercase();
        let g = if [".jpg", ".jpeg", ".png", ".heic", ".webp", ".gif", ".mp4", ".mov", ".mkv", ".webm"].iter().any(|e| lower.ends_with(e)) { G::Palette } else { G::Folder };
        let at = kit::nu(it, "at").unwrap_or(0.0);
        let when = if at > 0.0 { hearth_ui::time::when((at / 1000.0) as i64) } else { String::new() };
        c = c.push(go_row(&p, g, name, kit::dots(&[kit::st(it, "from"), when]), M::OpenFile(kit::st(it, "path"))));
    }
    c.push(said(&p, &s.rsay)).into()
}

pub fn subscription(s: &S) -> Subscription<M> {
    if s.live.running("bluetooth-devices") && s.sending.is_empty() {
        iced::time::every(std::time::Duration::from_secs(8)).map(|_| M::Tick)
    } else {
        Subscription::none()
    }
}

pub fn back(s: &mut S) -> bool {
    if !s.open.is_empty() {
        s.open.clear();
        return true;
    }
    false
}

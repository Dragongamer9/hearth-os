//! Bluetooth & devices › USB devices & drives and Thunderbolt (Mantel's SetUsbDevices and SetThunderbolt). USB: what
//! happens when a drive or memory card is plugged in (Ask, Open in Files, Nothing) and the notice for a USB device
//! plugged in for the first time (Hearth's desktop.json, through hearth_settings_io, only when you pick), the USB
//! devices plugged in now with their kind and speed, and where USBGuard runs, Allow / Block for each (hearth-usbdev).
//! Thunderbolt: docks, drives and screens on Thunderbolt or USB4 (bolt, through hearth-thunderbolt): which are plugged
//! in, allowing one once or always (bolt remembers it), or forgetting it. Nothing is allowed unless you press.

use crate::cats::net::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::run;
use hearth_ui::ui::{btn, Kind, G};
use iced::widget::{Column, Row};
use iced::{Subscription, Task};
use serde_json::{json, Value};

#[derive(Default)]
pub struct S {
    pub live: Live,
    pub usb: Option<Value>,
    pub usay: Say,
    pub tb: Option<Value>,
    pub picked: String,
    pub tsay: Say,
    pub busy: bool,
}

#[derive(Debug, Clone)]
pub enum M {
    Usb(Res),
    Storage(&'static str),
    Notify(bool),
    Wrote(run::Output),
    Guard(Vec<String>),
    Guarded(Res),
    Tb(Res),
    Pick(String),
    Allow(String, &'static str),
    Forget(String),
    TbDone(&'static str, Res),
    Tick,
}

fn usb() -> Task<M> {
    Task::perform(kit::job("hearth-usbdev", kit::a(&["list"]), String::new()), M::Usb)
}
fn tb() -> Task<M> {
    Task::perform(kit::job("hearth-thunderbolt", kit::a(&["status"]), String::new()), M::Tb)
}

pub fn open(_sh: &mut Shared, s: &mut S, page: &'static str, _part: &str) -> Task<M> {
    s.live.start(page);
    if page == "thunderbolt" {
        s.tsay.clear();
        s.picked.clear();
        return tb();
    }
    s.usay.clear();
    usb()
}

pub fn update(sh: &mut Shared, s: &mut S, m: M) -> Task<M> {
    match m {
        M::Usb(r) => match r {
            Res::Ok(v) => s.usb = Some(v),
            Res::Err(e) => s.usay.bad(e),
        },
        M::Storage(v) => {
            // `removable` is one setting holding {storage: …}: the rest of it is kept.
            let mut o = sh.desktop.get("removable").and_then(|x| x.as_object()).cloned().unwrap_or_default();
            o.insert("storage".into(), json!(v));
            let cmd = sh.desktop_cmd(json!({ "removable": Value::Object(o) }));
            return Task::perform(run::spawn(cmd), M::Wrote);
        }
        M::Notify(on) => {
            let cmd = sh.desktop_cmd(json!({ "usbNotify": on }));
            return Task::perform(run::spawn(cmd), M::Wrote);
        }
        M::Wrote(o) => sh.result("Couldn't save that", &o),
        M::Guard(args) => return Task::perform(kit::job("hearth-usbdev", args, String::new()), M::Guarded),
        M::Guarded(r) => {
            if let Res::Err(e) = r {
                s.usay.bad(e);
            }
            return usb();
        }
        M::Tb(r) => match r {
            Res::Ok(v) => s.tb = Some(v),
            Res::Err(e) => s.tsay.bad(e),
        },
        M::Pick(u) => s.picked = if s.picked == u { String::new() } else { u },
        M::Allow(u, how) => {
            s.busy = true;
            return Task::perform(kit::job("hearth-thunderbolt", vec!["allow".into(), u, how.into()], String::new()), move |r| M::TbDone(if how == "once" { "Allowed." } else { "Always allowed." }, r));
        }
        M::Forget(u) => {
            s.busy = true;
            return Task::perform(kit::job("hearth-thunderbolt", vec!["forget".into(), u], String::new()), |r| M::TbDone("Forgotten.", r));
        }
        M::TbDone(ok, r) => {
            s.busy = false;
            match r {
                Res::Ok(v) => {
                    if ok == "Forgotten." {
                        s.picked.clear();
                    }
                    s.tsay.ok(ok);
                    s.tb = Some(v);
                }
                Res::Err(e) => s.tsay.bad(e),
            }
        }
        M::Tick => {
            if s.live.still(5) && !s.busy {
                return if s.live.page == "thunderbolt" { tb() } else { usb() };
            }
        }
    }
    Task::none()
}

fn icon_for(k: &str) -> G {
    match k {
        "Keyboard" | "Mouse" => G::Grid,
        "Storage" => G::Drive,
        "Webcam" | "Camera or phone" => G::Display,
        "Sound" | "Sound and video" => G::Sound,
        "Controller or input" => G::Pad,
        "Printer" => G::Grid,
        k if k.contains("Bluetooth") => G::Bt,
        _ => G::Link,
    }
}

pub fn view<'a>(sh: &'a Shared, s: &'a S, page: &'static str) -> E<'a, M> {
    s.live.seen();
    if page == "thunderbolt" {
        return thunderbolt(sh, s);
    }
    let p = sh.p;
    let mut c = Column::new().spacing(10.0);
    c = c.push(head(&p, "When a USB drive or memory card is plugged in"));
    let storage = sh.desktop.get("removable").and_then(|r| r.get("storage")).and_then(|x| x.as_str()).unwrap_or("ask").to_string();
    c = c.push(choice(&p, G::Drive, "Then", "", [("ask", "Ask me"), ("open", "Open it in Files"), ("nothing", "Nothing")].iter().map(|&(v, l)| (l.to_string(), storage == v, Some(M::Storage(v)))).collect()));
    c = c.push(words(&p, "Either way it shows in Files' sidebar, and Safely remove is there too."));
    c = c.push(head(&p, "New USB devices"));
    let notify = sh.d_bool("usbNotify", true);
    c = c.push(srow(&p, G::Moon, "Tell me about a USB device plugged in for the first time", "Its name and what kind it is, once per device", switch(&p, notify, Some(M::Notify(!notify)))));
    if s.usb.as_ref().map(|v| kit::bo(v, "usbguard.running")).unwrap_or(false) {
        c = c.push(info_row(&p, G::Shield, "USBGuard is on", "A device it blocks asks you first: Allow once or Allow always below"));
    }
    let devs = s.usb.as_ref().map(|v| kit::ar(v, "devices")).unwrap_or(&[]);
    c = c.push(head(&p, if s.usb.is_none() { "Looking…" } else if devs.is_empty() { "Nothing plugged in" } else { "Plugged in now" }));
    for d in devs {
        let kind = kit::st(d, "kind");
        let guard = kit::at(d, "guard");
        let has_guard = guard.is_object();
        let allowed = kit::st(guard, "target") == "allow";
        let (chip, ck) = if has_guard { (if allowed { "Allowed" } else { "Blocked" }, if allowed { "ok" } else { "bad" }) } else if !kit::not_false(d, "authorized") { ("Blocked", "bad") } else { ("", "ok") };
        let ids = format!("{}:{}", kit::st(d, "vendor"), kit::st(d, "product"));
        c = c.push(chip_row(&p, icon_for(&kind), kit::st(d, "name"), kit::dots(&[kind.clone(), kit::st(d, "maker"), kit::st(d, "speed"), ids]), chip, ck, None));
        if has_guard {
            let id = kit::st(guard, "id");
            let mut b = Row::new().spacing(8.0);
            if !allowed {
                b = b.push(btn(&p, "Allow once", Kind::Normal, false, Some(M::Guard(vec!["allow".into(), id.clone()]))));
                b = b.push(btn(&p, "Allow always", Kind::Primary, false, Some(M::Guard(vec!["allow".into(), id.clone(), "always".into()]))));
            } else if kind != "Keyboard" && kind != "Mouse" {
                b = b.push(btn(&p, "Block", Kind::Normal, false, Some(M::Guard(vec!["block".into(), id.clone()]))));
            }
            c = c.push(iced::widget::container(b).padding(iced::Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 4.0 }));
        }
    }
    c.push(said(&p, &s.usay)).into()
}

fn state_text(d: &Value) -> String {
    let stored = kit::bo(d, "stored");
    if !kit::bo(d, "connected") {
        format!("Not plugged in{}", if stored { " · remembered" } else { "" })
    } else if kit::bo(d, "authorized") {
        format!("Connected{}", if stored { " · always allowed" } else { " · allowed this time" })
    } else {
        "Plugged in, waiting: allow it to use it".into()
    }
}
fn security_text(s: &str) -> &'static str {
    match s {
        "none" | "dponly" => "Every device connects by itself (set in your PC's firmware setup)",
        "user" | "secure" => "New devices wait until you allow them",
        "usbonly" => "Only USB on these ports (set in your PC's firmware setup)",
        _ => "",
    }
}

fn thunderbolt<'a>(sh: &'a Shared, s: &'a S) -> E<'a, M> {
    let p = sh.p;
    let mut c = Column::new().spacing(10.0).push(head(&p, "Thunderbolt and USB4"));
    let Some(st) = &s.tb else { return c.push(words(&p, "Asking bolt…")).push(said(&p, &s.tsay)).into() };
    let avail = kit::bo(st, "available");
    if !avail {
        c = c.push(info_row(&p, G::Info, "No Thunderbolt here", "This PC has no Thunderbolt or USB4 ports, or bolt isn't running"));
    }
    let sec = security_text(&kit::st(st, "security"));
    if avail && !sec.is_empty() {
        c = c.push(info_row(&p, G::Shield, "Security", sec));
    }
    let devs = kit::ar(st, "devices");
    for d in devs {
        let uuid = kit::st(d, "uuid");
        let (conn, auth, stored) = (kit::bo(d, "connected"), kit::bo(d, "authorized"), kit::bo(d, "stored"));
        let vendor = kit::st(d, "vendor");
        let gen = kit::st(d, "generation");
        let (chip, kind) = if conn && !auth { ("Waiting", "warn") } else if auth { ("On", "ok") } else { ("", "ok") };
        c = c.push(chip_row(&p, G::Link, format!("{}{}", if vendor.is_empty() { String::new() } else { format!("{vendor} ") }, kit::st(d, "name")), format!("{}{}", state_text(d), if gen.is_empty() { String::new() } else { format!(" · {gen}") }), chip, kind, Some(M::Pick(uuid.clone()))));
        if s.picked == uuid {
            let mut x = Column::new().spacing(8.0);
            if conn && !auth {
                x = x.push(press(&p, G::Check, "Allow this time", "Until it's unplugged", Some(M::Allow(uuid.clone(), "once"))));
            }
            if !stored {
                x = x.push(press(&p, G::Shield, "Always allow it", "It connects by itself from now on", Some(M::Allow(uuid.clone(), "always"))));
            } else {
                x = x.push(danger_row(&p, G::Close, "Forget it", "It waits for you again next time", Some(M::Forget(uuid.clone()))));
            }
            c = c.push(indent(x));
        }
    }
    if avail && devs.is_empty() {
        c = c.push(words(&p, "Nothing plugged in or remembered."));
    }
    c.push(said(&p, &s.tsay)).into()
}

pub fn subscription(s: &S) -> Subscription<M> {
    if (s.live.running("usb-devices") || s.live.running("thunderbolt")) && !s.busy {
        iced::time::every(std::time::Duration::from_secs(5)).map(|_| M::Tick)
    } else {
        Subscription::none()
    }
}

pub fn back(s: &mut S) -> bool {
    if !s.picked.is_empty() {
        s.picked.clear();
        return true;
    }
    false
}

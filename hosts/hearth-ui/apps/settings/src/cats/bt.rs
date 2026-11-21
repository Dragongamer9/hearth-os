//! Bluetooth & devices (Mantel's SetBluetooth, SetBtDevices, SetBtReceive, SetKeyboard, SetMouse, SetMouseButtons,
//! SetPhone, SetPhoneMessages, SetKwinTablet, SetPrinters, SetPrintQueue, SetScanners, SetScanPdf, SetInkLevels,
//! SetUsbDevices and SetThunderbolt, in the approved Settings' look). See cats/mod.rs for the shape. The category's own
//! page lists them (frame.rs); Bluetooth is drawn here, the others in their own files (bt_*.rs). Each uses the same
//! helper as Mantel's page (bluetoothctl, hearth-bt, hearth-system, hearth-kwin-settings, hearth-phone, hearth-printq…),
//! so both desktops share every setting; on KDE Plasma the keyboard and mouse go to KWin's own files
//! (kxkbrc / kcminputrc) through hearth-kwin-settings, as with Mantel on KWin.

#[path = "bt_kw.rs"]
pub mod kw;
#[path = "bt_devs.rs"]
pub mod devs;
#[path = "bt_kbd.rs"]
pub mod kbd;
#[path = "bt_mouse.rs"]
pub mod mouse;
#[path = "bt_phone.rs"]
pub mod phone;
#[path = "bt_tablet.rs"]
pub mod tablet;
#[path = "bt_print.rs"]
pub mod print;
#[path = "bt_usb.rs"]
pub mod usb;

use crate::cats::net::kit::{self, *};
use crate::data;
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::G;
use iced::widget::Column;
use iced::{Element, Subscription, Task};

pub const PAGES: &[&str] = &[
    "bluetooth", "bluetooth-devices", "bluetooth-receive", "keyboard", "mouse", "mouse-buttons", "phone", "phone-messages", "kwin-tablet", "printers", "print-queue", "scanners", "scan-pdf", "ink-levels", "usb-devices", "thunderbolt",
];

#[derive(Default)]
pub struct State {
    pub bt: data::Bt,
    pub read: bool,
    pub scanning: bool,
    pub found: Vec<data::BtDev>,
    pub busy: String,
    pub say: Say,
    pub live: Live,
    pub devs: devs::S,
    pub kbd: kbd::S,
    pub mouse: mouse::S,
    pub phone: phone::S,
    pub tablet: tablet::S,
    pub print: print::S,
    pub usb: usb::S,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Got(run::Output),
    Did(String, run::Output),
    On(bool),
    Scan,
    Scanned(run::Output),
    Toggle(String, String, bool),
    Pair(String, String),
    Forget(String, String),
    Tick,
    Go(&'static str),
    Devs(devs::M),
    Kbd(kbd::M),
    Mouse(mouse::M),
    Phone(phone::M),
    Tablet(tablet::M),
    Print(print::M),
    Usb(usb::M),
}

fn read() -> Task<Msg> {
    Task::perform(exec(data::sh(data::BT), String::new(), 25), Msg::Got)
}
/// Every bluetoothctl call has a timeout: without one, "pair" waits for ever when the device never answers, and with
/// the Bluetooth service stopped every call (even "show") waits for it to appear.
fn act(script: String, ok: String) -> Task<Msg> {
    Task::perform(exec(data::sh(&script), String::new(), 90), move |o| Msg::Did(ok.clone(), o))
}

pub fn open(sh: &mut Shared, st: &mut State, page: &'static str) -> Task<Msg> {
    let part = std::mem::take(&mut sh.part);
    match page {
        "bluetooth-devices" | "bluetooth-receive" => return devs::open(sh, &mut st.devs, page, &part).map(Msg::Devs),
        "keyboard" => return kbd::open(sh, &mut st.kbd, &part).map(Msg::Kbd),
        "mouse" | "mouse-buttons" => return mouse::open(sh, &mut st.mouse, page, &part).map(Msg::Mouse),
        "phone" | "phone-messages" => return phone::open(sh, &mut st.phone, page, &part).map(Msg::Phone),
        "kwin-tablet" => return tablet::open(sh, &mut st.tablet, &part).map(Msg::Tablet),
        "printers" | "print-queue" | "scanners" | "scan-pdf" | "ink-levels" => return print::open(sh, &mut st.print, page, &part).map(Msg::Print),
        "usb-devices" | "thunderbolt" => return usb::open(sh, &mut st.usb, page, &part).map(Msg::Usb),
        _ => {}
    }
    st.say.clear();
    st.live.start("bluetooth");
    read()
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Got(o) => {
            st.bt = data::bt(&o.out);
            st.read = true;
        }
        Msg::Did(ok, o) => {
            st.busy.clear();
            if o.ok {
                st.say.ok(ok);
            } else {
                let why = o.why();
                st.say.bad(if why.is_empty() { "That didn't work".to_string() } else { why });
            }
            return read();
        }
        Msg::On(on) => {
            st.bt.on = on;
            return act(format!("bluetoothctl --timeout 10 power {}", if on { "on" } else { "off" }), if on { "Bluetooth on".into() } else { "Bluetooth off".into() });
        }
        Msg::Scan => {
            if st.scanning {
                return Task::none();
            }
            st.scanning = true;
            st.found.clear();
            return Task::perform(exec(data::sh(&format!("bluetoothctl --timeout 12 scan on >/dev/null 2>&1; {}", data::BT)), String::new(), 40), Msg::Scanned);
        }
        Msg::Scanned(o) => {
            st.scanning = false;
            let b = data::bt(&o.out);
            st.found = b.devs.iter().filter(|d| !d.paired).cloned().collect();
            st.bt = b;
        }
        Msg::Toggle(mac, name, connected) => {
            st.busy = mac.clone();
            let m = data::q(&mac);
            return act(format!("bluetoothctl --timeout 20 {} {m}", if connected { "disconnect" } else { "connect" }), format!("{name}{}", if connected { " disconnected" } else { " connected" }));
        }
        Msg::Pair(mac, name) => {
            st.busy = mac.clone();
            let m = data::q(&mac);
            return act(format!("bluetoothctl --timeout 40 pair {m} && bluetoothctl --timeout 10 trust {m} && bluetoothctl --timeout 20 connect {m}"), format!("{name} is paired"));
        }
        Msg::Forget(mac, name) => {
            st.found.retain(|d| d.mac != mac);
            return act(format!("bluetoothctl --timeout 10 remove {}", data::q(&mac)), format!("{name} forgotten"));
        }
        Msg::Tick => {
            if st.live.still(8) && st.busy.is_empty() && !st.scanning {
                return read();
            }
        }
        Msg::Go(id) => sh.go(id),
        Msg::Devs(x) => return devs::update(sh, &mut st.devs, x).map(Msg::Devs),
        Msg::Kbd(x) => return kbd::update(sh, &mut st.kbd, x).map(Msg::Kbd),
        Msg::Mouse(x) => return mouse::update(sh, &mut st.mouse, x).map(Msg::Mouse),
        Msg::Phone(x) => return phone::update(sh, &mut st.phone, x).map(Msg::Phone),
        Msg::Tablet(x) => return tablet::update(sh, &mut st.tablet, x).map(Msg::Tablet),
        Msg::Print(x) => return print::update(sh, &mut st.print, x).map(Msg::Print),
        Msg::Usb(x) => return usb::update(sh, &mut st.usb, x).map(Msg::Usb),
    }
    Task::none()
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    let st = &a.bt;
    let sh = &a.sh;
    Some(match page {
        "bluetooth" => bluetooth(a),
        "bluetooth-devices" | "bluetooth-receive" => devs::view(sh, &st.devs, page).map(Msg::Devs),
        "keyboard" => kbd::view(sh, &st.kbd).map(Msg::Kbd),
        "mouse" | "mouse-buttons" => mouse::view(sh, &st.mouse, page).map(Msg::Mouse),
        "phone" | "phone-messages" => phone::view(sh, &st.phone, page).map(Msg::Phone),
        "kwin-tablet" => tablet::view(sh, &st.tablet).map(Msg::Tablet),
        "printers" | "print-queue" | "scanners" | "scan-pdf" | "ink-levels" => print::view(sh, &st.print, page).map(Msg::Print),
        "usb-devices" | "thunderbolt" => usb::view(sh, &st.usb, page).map(Msg::Usb),
        _ => return None,
    })
}

fn bluetooth(a: &App) -> Element<'_, Msg> {
    let p = a.sh.p;
    let st = &a.bt;
    st.live.seen();
    let b = &st.bt;
    if !st.read {
        return note(&p, "Looking for Bluetooth…", false);
    }
    if !b.has {
        return Column::new().spacing(10.0).push(note(&p, "This PC has no Bluetooth, or it's switched off in its firmware.", false)).push(go_row(&p, G::Bt, "Receive files & this PC's name", "Files from phones over Bluetooth, and the name others see", Msg::Go("bluetooth-receive"))).into();
    }
    let paired: Vec<&data::BtDev> = b.devs.iter().filter(|d| d.paired).collect();
    let connected: Vec<String> = paired.iter().filter(|d| d.connected).map(|d| d.name.clone()).collect();
    let sub = if !b.on { "Off".to_string() } else if connected.is_empty() { "On".into() } else { format!("{} connected", connected.join(", ")) };
    let mut c = Column::new().spacing(10.0);
    c = c.push(srow(&p, G::Bt, "Bluetooth", sub, switch(&p, b.on, Some(Msg::On(!b.on)))));
    c = c.push(said(&p, &st.say));
    if b.on {
        c = c.push(head(&p, "Your devices"));
        for d in &paired {
            let sub = if st.busy == d.mac { "Working…" } else if d.connected { "Connected · select to disconnect" } else { "Not connected · select to connect" };
            c = c.push(val_row(&p, G::Pad, d.name.clone(), sub, if d.connected { "Connected" } else { "" }, Some(Msg::Toggle(d.mac.clone(), d.name.clone(), d.connected))));
        }
        if !paired.is_empty() {
            c = c.push(go_row(&p, G::Gear, "Trust, block, batteries and sending files", "Each paired device's settings", Msg::Go("bluetooth-devices")));
        } else {
            c = c.push(words(&p, "No devices yet. Put a controller or headphones in pairing mode, then look for devices."));
        }
        c = c.push(head(&p, if st.scanning { "New devices · looking…" } else { "New devices" }));
        c = c.push(press(&p, G::Bt, if st.scanning { "Looking for devices…" } else { "Look for devices" }, "Put the device in pairing mode first", if st.scanning { None } else { Some(Msg::Scan) }));
        for d in &st.found {
            c = c.push(press(&p, G::Dots, d.name.clone(), if st.busy == d.mac { "Pairing…" } else { "Select to pair" }, Some(Msg::Pair(d.mac.clone(), d.name.clone()))));
        }
        if !paired.is_empty() {
            c = c.push(head(&p, "Manage"));
            for d in &paired {
                c = c.push(danger_row(&p, G::Close, format!("Forget {}", d.name), "", Some(Msg::Forget(d.mac.clone(), d.name.clone()))));
            }
        }
    }
    c = c.push(go_row(&p, G::Bt, "Receive files & this PC's name", "Files from phones over Bluetooth, and the name others see", Msg::Go("bluetooth-receive")));
    c.into()
}

pub fn subscription(st: &State) -> Subscription<Msg> {
    let mut s = Vec::new();
    if st.live.running("bluetooth") {
        s.push(iced::time::every(std::time::Duration::from_secs(8)).map(|_| Msg::Tick));
    }
    s.push(devs::subscription(&st.devs).map(Msg::Devs));
    s.push(kbd::subscription(&st.kbd).map(Msg::Kbd));
    s.push(mouse::subscription(&st.mouse).map(Msg::Mouse));
    s.push(phone::subscription(&st.phone).map(Msg::Phone));
    s.push(print::subscription(&st.print).map(Msg::Print));
    s.push(usb::subscription(&st.usb).map(Msg::Usb));
    s.push(tablet::subscription(&st.tablet).map(Msg::Tablet));
    Subscription::batch(s)
}

pub fn dialog<'a>(a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    kit::set_current(a.page);
    let (d, o) = tablet::dialog(&a.sh, &a.bt.tablet)?;
    Some((d.map(Msg::Tablet), o.map(Msg::Tablet)))
}

pub fn back(st: &mut State) -> bool {
    // Only the page showing closes something of its own (each keeps its state while you're elsewhere).
    (kit::showing(&["kwin-tablet"]) && tablet::back(&mut st.tablet))
        || (kit::showing(&["mouse", "mouse-buttons"]) && mouse::back(&mut st.mouse))
        || (kit::showing(&["phone", "phone-messages"]) && phone::back(&mut st.phone))
        || (kit::showing(&["keyboard"]) && kbd::back(&mut st.kbd))
        || (kit::showing(&["printers", "print-queue", "scanners", "scan-pdf", "ink-levels"]) && print::back(&mut st.print))
        || (kit::showing(&["usb-devices", "thunderbolt"]) && usb::back(&mut st.usb))
        || (kit::showing(&["bluetooth-devices"]) && devs::back(&mut st.devs))
}

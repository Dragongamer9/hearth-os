//! Bluetooth & devices: Bluetooth (BlueZ's bluetoothctl). See cats/mod.rs for the shape.

use crate::data;
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{btn, t, Kind, G};
use iced::widget::{container, row, Column, Space};
use iced::{Element, Length, Padding, Subscription, Task};

pub const PAGES: &[&str] = &["bluetooth"];

#[derive(Default)]
pub struct State {
    pub bt: data::Bt,
    pub read: bool,
    pub scanning: bool,
    pub busy: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Got(run::Output),
    Did(&'static str, run::Output),
    On(bool),
    Scan,
    Scanned(run::Output),
    Connect(String, bool),
    Pair(String),
    Forget(String),
    Go(&'static str),
}

fn read() -> Task<Msg> {
    Task::perform(run::spawn(data::sh(data::BT)), Msg::Got)
}
fn act(cmd: Vec<String>, what: &'static str) -> Task<Msg> {
    Task::perform(run::spawn(cmd), move |o| Msg::Did(what, o))
}

pub fn open(_sh: &mut Shared, _st: &mut State, _page: &'static str) -> Task<Msg> {
    read()
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Got(o) => {
            st.bt = data::bt(&o.out);
            st.read = true;
        }
        Msg::Did(what, o) => {
            st.busy = false;
            sh.result(what, &o);
            return read();
        }
        Msg::On(on) => return act(data::sh(&format!("bluetoothctl power {}", if on { "on" } else { "off" })), "Bluetooth couldn't be turned on or off"),
        Msg::Scan => {
            st.scanning = true;
            return Task::perform(run::spawn(data::sh("timeout 12 bluetoothctl --timeout 10 scan on >/dev/null 2>&1; true")), Msg::Scanned);
        }
        Msg::Scanned(_) => {
            st.scanning = false;
            return read();
        }
        Msg::Connect(mac, on) => {
            st.busy = true;
            return act(vec!["timeout".into(), "20".into(), "bluetoothctl".into(), if on { "connect".into() } else { "disconnect".into() }, mac], "Couldn't do that");
        }
        Msg::Pair(mac) => {
            st.busy = true;
            let m = data::q(&mac);
            return act(data::sh(&format!("timeout 25 bluetoothctl pair {m} && bluetoothctl trust {m} >/dev/null && timeout 20 bluetoothctl connect {m}")), "Couldn't pair (is the device in pairing mode?)");
        }
        Msg::Forget(mac) => return act(vec!["bluetoothctl".into(), "remove".into(), mac], "Couldn't forget that device"),
        Msg::Go(id) => sh.go(id),
    }
    Task::none()
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    if page != "bluetooth" {
        return None;
    }
    let p = a.sh.p;
    let st = &a.bt;
    let b = &st.bt;
    if !st.read {
        return Some(note(&p, "Looking for Bluetooth…", false));
    }
    if !b.has {
        return Some(note(&p, "This PC has no Bluetooth, or it's switched off in its firmware.", false));
    }
    let mut c = Column::new().spacing(10.0);
    c = c.push(srow(&p, G::Bt, "Bluetooth", if b.on { "Headphones, controllers and other wireless devices" } else { "Off" }, switch(&p, b.on, Some(Msg::On(!b.on)))));
    if b.on {
        c = c.push(head(&p, "Your devices"));
        let paired: Vec<_> = b.devs.iter().filter(|d| d.paired).collect();
        if paired.is_empty() {
            c = c.push(note(&p, "None paired yet.", false));
        }
        for d in paired {
            let ctl: Element<Msg> = row![
                btn(&p, if d.connected { "Disconnect" } else { "Connect" }, if d.connected { Kind::Normal } else { Kind::Primary }, false, if st.busy { None } else { Some(Msg::Connect(d.mac.clone(), !d.connected)) }),
                btn(&p, "Forget", Kind::Normal, false, Some(Msg::Forget(d.mac.clone())))
            ]
            .spacing(8.0)
            .into();
            c = c.push(srow(&p, G::Bt, d.name.clone(), if d.connected { "Connected" } else { "Not connected" }, ctl));
        }
        c = c.push(row![head(&p, "Add a device"), Space::new().width(Length::Fill), container(btn(&p, if st.scanning { "Looking…" } else { "Look for devices" }, Kind::Normal, false, if st.scanning { None } else { Some(Msg::Scan) })).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 })].align_y(iced::Center));
        c = c.push(t("Put the device in pairing mode, then look for it here.", sz(13.0), p.muted));
        for d in b.devs.iter().filter(|d| !d.paired) {
            c = c.push(srow(&p, G::Bt, d.name.clone(), d.mac.clone(), btn(&p, if st.busy { "Pairing…" } else { "Pair" }, Kind::Normal, false, if st.busy { None } else { Some(Msg::Pair(d.mac.clone())) })));
        }
    }
    Some(c.into())
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(_a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    None
}

pub fn back(_st: &mut State) -> bool {
    false
}

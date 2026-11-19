//! Network & internet: Wi-Fi (NetworkManager's nmcli). See cats/mod.rs for the shape.

use crate::data;
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, btn, t, ts, Kind, G};
use iced::widget::{column, container, row, Column, Space};
use iced::{Element, Length, Padding, Subscription, Task};

pub const PAGES: &[&str] = &["wifi"];

#[derive(Default)]
pub struct State {
    pub wifi: data::Wifi,
    pub read: bool,
    pub busy: bool,
    pub join: Option<(String, String)>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Got(run::Output),
    Did(&'static str, run::Output),
    On(bool),
    Scan,
    Pick(String),
    Pw(String),
    Join,
    Cancel,
    Disconnect(String),
    Forget(String),
    Go(&'static str),
}

fn read() -> Task<Msg> {
    Task::perform(run::spawn(data::sh(data::WIFI)), Msg::Got)
}
fn act(cmd: Vec<String>, what: &'static str) -> Task<Msg> {
    Task::perform(run::spawn(cmd), move |o| Msg::Did(what, o))
}

pub fn open(_sh: &mut Shared, st: &mut State, _page: &'static str) -> Task<Msg> {
    st.join = None;
    read()
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Got(o) => {
            st.wifi = data::wifi(&o.out);
            st.read = true;
        }
        Msg::Did(what, o) => {
            st.busy = false;
            sh.result(what, &o);
            return read();
        }
        Msg::On(on) => return act(data::sh(&format!("nmcli radio wifi {}", if on { "on" } else { "off" })), "Wi-Fi couldn't be turned on or off"),
        Msg::Scan => return act(data::sh("nmcli dev wifi rescan 2>/dev/null; sleep 3"), "Couldn't look for networks"),
        Msg::Pick(ssid) => {
            if let Some(n) = st.wifi.nets.iter().find(|n| n.ssid == ssid).cloned() {
                if n.active {
                    return Task::none();
                }
                if n.known || !n.secure {
                    st.busy = true;
                    let cmd = if n.known { vec!["nmcli".into(), "con".into(), "up".into(), "id".into(), n.ssid] } else { vec!["nmcli".into(), "dev".into(), "wifi".into(), "connect".into(), n.ssid] };
                    return act(cmd, "Couldn't connect");
                }
                st.join = Some((ssid, String::new()));
            }
        }
        Msg::Pw(pw) => {
            if let Some(j) = st.join.as_mut() {
                j.1 = pw;
            }
        }
        Msg::Join => {
            if let Some((ssid, pw)) = st.join.take() {
                st.busy = true;
                return act(vec!["nmcli".into(), "dev".into(), "wifi".into(), "connect".into(), ssid, "password".into(), pw], "Couldn't connect (is the password right?)");
            }
        }
        Msg::Cancel => st.join = None,
        Msg::Disconnect(ssid) => return act(vec!["nmcli".into(), "con".into(), "down".into(), "id".into(), ssid], "Couldn't disconnect"),
        Msg::Forget(ssid) => return act(vec!["nmcli".into(), "con".into(), "delete".into(), "id".into(), ssid], "Couldn't forget that network"),
        Msg::Go(id) => sh.go(id),
    }
    Task::none()
}

pub fn view<'a>(a: &'a App, page: &'static str) -> Option<Element<'a, Msg>> {
    if page != "wifi" {
        return None;
    }
    let p = a.sh.p;
    let st = &a.net;
    let w = &st.wifi;
    if !st.read {
        return Some(note(&p, "Looking at your network…", false));
    }
    let mut c = Column::new().spacing(10.0);
    if !w.wired.is_empty() {
        c = c.push(srow(&p, G::Link, "Ethernet", format!("Connected · {}", w.wired), Space::new().into()));
    }
    if !w.has {
        return Some(c.push(note(&p, "This PC has no Wi-Fi.", false)).into());
    }
    c = c.push(srow(&p, G::Wifi, "Wi-Fi", if w.on { "Find and connect to wireless networks" } else { "Off" }, switch(&p, w.on, Some(Msg::On(!w.on)))));
    if w.on {
        c = c.push(row![head(&p, "Networks"), Space::new().width(Length::Fill), container(btn(&p, if st.busy { "Connecting…" } else { "Look again" }, Kind::Normal, false, Some(Msg::Scan))).padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 })].align_y(iced::Center));
        for n in &w.nets {
            let sub = format!("{}{}{}", if n.active { "Connected · " } else { "" }, data::signal_word(n.signal), if n.secure { " · secured" } else { " · open" });
            let ctl: Element<Msg> = if n.active {
                row![btn(&p, "Disconnect", Kind::Normal, false, Some(Msg::Disconnect(n.ssid.clone()))), btn(&p, "Forget", Kind::Normal, false, Some(Msg::Forget(n.ssid.clone())))].spacing(8.0).into()
            } else if n.known {
                row![btn(&p, "Connect", Kind::Primary, false, Some(Msg::Pick(n.ssid.clone()))), btn(&p, "Forget", Kind::Normal, false, Some(Msg::Forget(n.ssid.clone())))].spacing(8.0).into()
            } else {
                btn(&p, "Connect", Kind::Normal, false, Some(Msg::Pick(n.ssid.clone())))
            };
            c = c.push(srow(&p, G::Wifi, n.ssid.clone(), sub, ctl));
        }
        if w.nets.is_empty() {
            c = c.push(note(&p, "No networks found. Look again in a moment.", false));
        }
    }
    Some(c.into())
}

pub fn subscription(_st: &State) -> Subscription<Msg> {
    Subscription::none()
}

pub fn dialog<'a>(a: &'a App) -> Option<(Element<'a, Msg>, Option<Msg>)> {
    let (ssid, pw) = a.net.join.as_ref()?;
    let p = a.sh.p;
    let d = ui::dialog_box(&p, 440.0, column![
        ts(format!("Connect to {ssid}"), 17.0, p.text),
        t("The network's password:", 13.0, p.muted),
        field(&p, "Password", pw, Msg::Pw).secure(true).on_submit(Msg::Join),
        container(row![btn(&p, "Connect", Kind::Primary, true, if pw.len() >= 8 { Some(Msg::Join) } else { None }), btn(&p, "Cancel", Kind::Normal, false, Some(Msg::Cancel))].spacing(8.0)).align_right(Length::Fill)
    ].spacing(12.0));
    Some((d, Some(Msg::Cancel)))
}

pub fn back(st: &mut State) -> bool {
    st.join.take().is_some()
}

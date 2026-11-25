//! Gaming › Steam Game Mode and Decky (Mantel's SetSteam.qml): Hearth OS's Game Mode (Steam's own gamepad screens,
//! like SteamOS), Steam Game Mode beside the desktop (hearth-side-by-side, a test), Decky Loader and Hearth Sync (the
//! Decky plugin that puts your non-Steam games in Steam's library; built into the Fedora edition), and why Steam Game
//! Mode didn't start last time. Through Hearth's core (steamMode.status / steamMode.decky), as on Debian. Hearth
//! never signs in to Steam or touches Steam accounts.

use crate::cats::apps::kit::{self, *};
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{t, G};
use iced::Task;
use serde_json::{json, Value};

#[derive(Default)]
pub struct St {
    pub st: Value,
    pub err: String,
    pub msg: String,
    pub busy: bool,
    pub confirm: bool,
    pub side: Value,
}

#[derive(Debug, Clone)]
pub enum M {
    Got(Result<Value, String>),
    Switch,
    Decky,
    Install,
    Decided(&'static str, Result<Value, String>),
    Side(Res),
    SideSet(bool),
    SideDone(bool, Res),
}

fn load() -> Task<M> {
    core("steamMode.status", json!({}), 30, M::Got)
}
fn side(args: &[&str]) -> Vec<String> {
    let mut v = vec![libexec("hearth-side-by-side")];
    v.extend(strs(args));
    prog(v, 60)
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.msg.clear();
    st.confirm = false;
    Task::batch([load(), read(side(&["status"]), M::Side)])
}

fn decide(st: &mut St, action: &'static str) -> Task<M> {
    st.busy = true;
    st.confirm = false;
    st.msg = if action == "install" { "Installing Decky Loader and Hearth Sync… it asks for an administrator's password".into() } else { "Removing Decky Loader…".into() };
    core("steamMode.decky", json!({"action": action}), 1200, move |r| M::Decided(action, r))
}

pub fn update(_sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Got(r) => match r {
            Ok(v) => {
                st.st = v;
                st.err.clear();
            }
            Err(e) => st.err = e,
        },
        M::Switch => kit::open(&["nimbus-mode", "steam"]),
        M::Decky => {
            if st.busy || st.st.is_null() {
                return Task::none();
            }
            let d = go(&st.st, "decky");
            if gb(d, "installed") && gs(d, "via") == "hearth" {
                return decide(st, "remove");
            } else if !gb(d, "installed") {
                st.confirm = !st.confirm;
            }
        }
        M::Install => return decide(st, "install"),
        M::Decided(action, r) => {
            st.busy = false;
            match r {
                Ok(v) => {
                    st.st = v;
                    st.msg = if action == "install" { "Decky Loader and Hearth Sync are installed. Switch to Steam Game Mode to use them.".into() } else { "Decky Loader removed".into() };
                }
                Err(e) => {
                    st.msg = e;
                    return load();
                }
            }
        }
        M::Side(r) => {
            if !r.v.is_null() {
                st.side = r.v;
            }
        }
        M::SideSet(on) => return read(side(&[if on { "on" } else { "off" }]), move |r| M::SideDone(on, r)),
        M::SideDone(on, r) => {
            st.msg = if !r.ok { "That didn't work".into() } else if on { "On: Steam Game Mode now opens beside the desktop".into() } else { "Off: switching is as before".into() };
            return read(side(&["status"]), M::Side);
        }
    }
    Task::none()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let s = &st.st;
    let mut v = crate::secs::SecVec::new();
    v.push(r(G::Pad, "Switch to Steam Game Mode", "Steam's own gamepad screens, like SteamOS: where the device starts. Back here: Steam's Power › Switch to Desktop.").chev().on(M::Switch).view(&p));
    if gb(s, "lastFailure") {
        v.push(t(format!("Steam Game Mode didn't start last time, so Desktop Mode opened instead: {}. Switch to Steam Game Mode tries again.", gs(s, "lastFailure")), sz(12.0), p.danger).into());
    }
    if s.get("available").and_then(Value::as_bool) == Some(false) {
        v.push(small(&p, format!("Steam Game Mode can't run here: it needs Steam{}.", if s.get("gamescope").and_then(Value::as_bool) == Some(false) { " and gamescope" } else { "" })));
    }
    let on = gb(&st.side, "on");
    v.push(sw(&p, G::Grid, "Keep Steam Game Mode running beside the desktop (test)", "Switching only shows the other one: a second or two, and Steam stays open and signed in. It keeps about 1–1.5 GB of memory for Steam. If Steam Game Mode doesn't come up this way, the usual switch happens instead. Still being tried on the Nvidia PC and the Ally.", on, Some(M::SideSet(!on))));
    v.push(head(&p, "Your other games in Steam"));
    let d = go(s, "decky");
    let sub = if st.busy {
        "Working on it…".to_string()
    } else if s.is_null() {
        if st.err.is_empty() { "Loading…".into() } else { st.err.clone() }
    } else if gb(d, "installed") {
        format!(
            "{}{}. Decky runs only while Steam Game Mode does.",
            if gs(d, "via") == "decky" { "Decky Loader (its own install)" } else { "Installed" },
            if gb(d, "plugin") { " · Hearth Sync in place" } else if gb(d, "shipped") { " · Hearth Sync not added yet" } else { "" }
        )
    } else {
        "Adds your non-Steam games (Epic, GOG, emulators and more) to Steam's library with covers. Needs an administrator's password; turns on Steam's remote debugging.".into()
    };
    let val = if s.is_null() || st.busy { "" } else if gb(d, "installed") && gs(d, "via") == "hearth" { "Remove" } else if gb(d, "installed") { "" } else { "Install" };
    v.push(r(G::Store, "Decky Loader and Hearth Sync", sub).value(val).on(M::Decky).view(&p));
    if st.confirm {
        v.push(r(G::Check, "Install Decky Loader", "Downloads Decky Loader from its GitHub releases and adds Hearth Sync. Decky runs as root while Steam Game Mode runs, and Steam's remote debugging is turned on (any program on this device can then control Steam).").on(M::Install).view(&p));
    }
    if !st.msg.is_empty() {
        v.push(words(&p, st.msg.clone(), false));
    }
    page(v)
}

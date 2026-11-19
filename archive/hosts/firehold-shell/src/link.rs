//! A surface's line to the hub (proto.rs): one thread reads the hub's lines and hands them to iced as a stream, so
//! the surface sleeps until the hub has something new; `send` writes a command.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde_json::{json, Value};

use crate::model::Game;
use crate::proto::{self, State};
use crate::sys::App;

static SURFACE: OnceLock<String> = OnceLock::new();
static OUTPUT: OnceLock<String> = OnceLock::new();
static WRITER: Mutex<Option<UnixStream>> = Mutex::new(None);

pub fn set_surface(name: &str) {
    let _ = SURFACE.set(name.to_string());
}

/// The screen this surface is on (`firehold-shell couch --output HDMI-A-1`, one surface a screen when there are two
/// or more): its name is `couch@HDMI-A-1` to the hub. None given: the screen firehold-comp picks.
pub fn set_output(out: &str) {
    let _ = OUTPUT.set(out.to_string());
}
pub fn output() -> String {
    OUTPUT.get().cloned().unwrap_or_default()
}
/// The surface's name as the hub knows it.
pub fn surface_name() -> String {
    let s = SURFACE.get().cloned().unwrap_or_default();
    let o = output();
    if o.is_empty() {
        s
    } else {
        format!("{s}@{o}")
    }
}
/// Where a layer surface goes: its own screen, or the one firehold-comp picks.
pub fn start_mode() -> iced_layershell::settings::StartMode {
    let o = output();
    if o.is_empty() {
        iced_layershell::settings::StartMode::Active
    } else {
        iced_layershell::settings::StartMode::TargetScreen(o)
    }
}

#[derive(Debug, Clone)]
pub enum HubMsg {
    State(Box<State>),
    Library(Vec<Game>),
    Apps(Vec<App>),
    Nav(String),
    Cmd(String, String),
    Quit,
    /// The look picked in Settings changed (style.rs read it again): the surface draws itself again.
    Look,
    /// The hub went away: the surface ends (the session decides what comes next).
    Gone,
}

fn parse(v: Value) -> Option<HubMsg> {
    let t = v.get("t").and_then(|t| t.as_str())?.to_string();
    match t.as_str() {
        "state" => serde_json::from_value::<State>(v).ok().map(|s| HubMsg::State(Box::new(s))),
        "library" => serde_json::from_value::<Vec<Game>>(v.get("games")?.clone()).ok().map(HubMsg::Library),
        "apps" => serde_json::from_value::<Vec<App>>(v.get("apps")?.clone()).ok().map(HubMsg::Apps),
        "nav" => Some(HubMsg::Nav(v.get("key")?.as_str()?.to_string())),
        "cmd" => Some(HubMsg::Cmd(v.get("cmd")?.as_str()?.to_string(), v.get("arg").and_then(|a| a.as_str()).unwrap_or("").to_string())),
        "quit" => Some(HubMsg::Quit),
        _ => None,
    }
}

/// The hub's messages, as a stream (iced's Subscription::run).
pub fn stream() -> futures::channel::mpsc::UnboundedReceiver<HubMsg> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    // The look (hearth-ui's watch: inotify on desktop.json and palettes.json, asleep until one is written).
    let look_tx = tx.clone();
    std::thread::spawn(move || {
        let mut changes = hearth_ui::theme::watch();
        while futures::executor::block_on(futures::StreamExt::next(&mut changes)).is_some() {
            if crate::ui::style::reload() && look_tx.unbounded_send(HubMsg::Look).is_err() {
                return;
            }
        }
    });
    std::thread::spawn(move || {
        let surface = surface_name();
        let Some(path) = proto::socket_path() else {
            let _ = tx.unbounded_send(HubMsg::Gone);
            return;
        };
        let mut tries = 0;
        let s = loop {
            match UnixStream::connect(&path) {
                Ok(s) => break s,
                Err(_) if tries < 40 => {
                    tries += 1;
                    std::thread::sleep(Duration::from_millis(250));
                }
                Err(e) => {
                    eprintln!("firehold-shell {surface}: no hub at {}: {e}", path.display());
                    let _ = tx.unbounded_send(HubMsg::Gone);
                    return;
                }
            }
        };
        if let Ok(mut w) = s.try_clone() {
            let _ = w.set_write_timeout(Some(Duration::from_millis(500)));
            let _ = w.write_all(format!("{}\n", json!({"op": "hello", "surface": surface})).as_bytes());
            if let Ok(mut g) = WRITER.lock() {
                *g = Some(w);
            }
        }
        for line in BufReader::new(s).lines() {
            let Ok(line) = line else { break };
            let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
            if v.get("t").and_then(|t| t.as_str()) == Some("reply") {
                continue;
            }
            if let Some(m) = parse(v) {
                if tx.unbounded_send(m).is_err() {
                    return;
                }
            }
        }
        let _ = tx.unbounded_send(HubMsg::Gone);
    });
    rx
}

/// A command to the hub (proto.rs); dropped when the hub isn't there.
pub fn send(v: Value) {
    if let Ok(mut g) = WRITER.lock() {
        if let Some(w) = g.as_mut() {
            if w.write_all(format!("{v}\n").as_bytes()).is_err() {
                *g = None;
            }
        }
    }
}

pub fn op(name: &str) {
    send(json!({"op": name}));
}

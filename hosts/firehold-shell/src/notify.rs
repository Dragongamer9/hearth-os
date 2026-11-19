//! Firehold's own notifications (it's its own desktop, so it's the notification service while it's open): the
//! freedesktop one, `org.freedesktop.Notifications` on the session bus, the name every Linux app sends to (Mantel's
//! Quickshell and Plasma's plasmashell are the same thing in their desktops). What comes in goes to the hub, which shows
//! it as a card at the top right for a few seconds and keeps it in Quick Settings; buttons and closing go back to the
//! app as the standard signals. When another program already has the name (another desktop's), Firehold doesn't take
//! it: that one keeps showing them.
//!
//! The bus connection waits in the kernel between messages (zbus's own thread): nothing here runs at rest.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Mutex;

use zbus::zvariant::{OwnedValue, Value};

use crate::hub::Ev;
use crate::proto::Note;

pub const PATH: &str = "/org/freedesktop/Notifications";
pub const IFACE: &str = "org.freedesktop.Notifications";

struct Server {
    tx: Mutex<Sender<Ev>>,
    next: AtomicU32,
}

impl Server {
    fn send(&self, ev: Ev) {
        if let Ok(tx) = self.tx.lock() {
            let _ = tx.send(ev);
        }
    }
}

/// Plain text from a body that may have the spec's little markup (<b>, <i>, <a>, &amp;...).
pub fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for ch in s.chars() {
        match ch {
            '<' => tag = true,
            '>' if tag => tag = false,
            c if !tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").trim().to_string()
}

/// The app's buttons, from the spec's flat list [key, label, key, label...].
pub fn pairs(actions: &[String]) -> Vec<(String, String)> {
    actions.chunks(2).filter(|c| c.len() == 2 && !c[0].is_empty()).map(|c| (c[0].clone(), plain(&c[1]))).take(3).collect()
}

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Server {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        _app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id > 0 { replaces_id } else { self.next.fetch_add(1, Ordering::Relaxed) };
        let urgent = matches!(hints.get("urgency").map(|v| &**v), Some(Value::U8(2)));
        let note = Note {
            id,
            app: plain(&app_name).chars().take(60).collect(),
            title: plain(&summary).chars().take(120).collect(),
            body: plain(&body).chars().take(400).collect(),
            actions: pairs(&actions),
            at: crate::sys::clock().0,
            urgent,
        };
        self.send(Ev::Note(note, expire_timeout));
        id
    }

    fn close_notification(&self, id: u32) {
        self.send(Ev::NoteClose(id));
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec!["body".into(), "actions".into(), "persistence".into()]
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        ("Firehold".into(), "Hearth".into(), "test".into(), "1.2".into())
    }
}

/// Takes the name and serves it (on its own thread); the connection goes to the hub, which keeps it (and sends the
/// signals on it). Nothing when there's no session bus or the name is taken.
pub(crate) fn start(tx: Sender<Ev>) {
    std::thread::spawn(move || {
        let server = Server { tx: Mutex::new(tx.clone()), next: AtomicU32::new(1) };
        let conn = zbus::blocking::connection::Builder::session()
            .and_then(|b| b.name(IFACE))
            .and_then(|b| b.serve_at(PATH, server))
            .and_then(|b| b.build());
        match conn {
            Ok(c) => {
                let _ = tx.send(Ev::Bus(c));
            }
            Err(e) => eprintln!("firehold-shell hub: no notifications here ({e}); another program shows them"),
        }
    });
}

/// A signal back to the app: ActionInvoked (id, key) or NotificationClosed (id, why: 1 expired, 2 you closed it,
/// 3 the app closed it).
pub fn action(bus: &zbus::blocking::Connection, id: u32, key: &str) {
    let _ = bus.emit_signal(None::<&str>, PATH, IFACE, "ActionInvoked", &(id, key));
}
pub fn closed(bus: &zbus::blocking::Connection, id: u32, why: u32) {
    let _ = bus.emit_signal(None::<&str>, PATH, IFACE, "NotificationClosed", &(id, why));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_and_buttons() {
        assert_eq!(plain("<b>Saved</b> &amp; done"), "Saved & done");
        assert_eq!(plain("2 < 3"), "2");
        let p = pairs(&["default".into(), "Open".into(), "reply".into(), "<i>Reply</i>".into(), "odd".into()]);
        assert_eq!(p, vec![("default".to_string(), "Open".to_string()), ("reply".to_string(), "Reply".to_string())]);
    }
}

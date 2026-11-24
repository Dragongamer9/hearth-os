//! The windows and the desktops, from labwc itself: wlr-foreign-toplevel (every window, its app id, title and whether
//! it's in front or minimized) and ext-workspace (labwc's desktops, the one you're on). One thread per surface reads
//! them and sends a snapshot to iced when something changed; nothing runs while nothing changes. Clicks go back the
//! same way (activate, minimize, close a window; switch desktop).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{event_created_child, Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols::ext::workspace::v1::client::{ext_workspace_group_handle_v1, ext_workspace_handle_v1, ext_workspace_manager_v1};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{zwlr_foreign_toplevel_handle_v1, zwlr_foreign_toplevel_manager_v1};

use ext_workspace_handle_v1::ExtWorkspaceHandleV1 as WsHandle;
use ext_workspace_manager_v1::ExtWorkspaceManagerV1 as WsManager;
use zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1 as TlHandle;
use zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1 as TlManager;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Win {
    pub id: u32,
    pub app_id: String,
    pub title: String,
    pub active: bool,
    pub minimized: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Desk {
    pub id: u32,
    pub name: String,
    pub active: bool,
}

/// What the bar and the dock draw from.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    pub wins: Vec<Win>,
    pub desks: Vec<Desk>,
    /// labwc gave us the windows (false: an older labwc or another compositor: the dock shows pins only).
    pub has_windows: bool,
    pub has_desks: bool,
}

struct Shared {
    conn: Connection,
    seat: Option<wl_seat::WlSeat>,
    ws_mgr: Option<WsManager>,
    wins: Mutex<HashMap<u32, TlHandle>>,
    desks: Mutex<HashMap<u32, WsHandle>>,
}

static SHARED: OnceLock<Shared> = OnceLock::new();
static SENDERS: Mutex<Vec<futures::channel::mpsc::UnboundedSender<Snapshot>>> = Mutex::new(Vec::new());

#[derive(Default)]
struct St {
    wins: Vec<(TlHandle, Win, bool)>,
    desks: Vec<(WsHandle, Desk)>,
    has_windows: bool,
    has_desks: bool,
    dirty: bool,
}

impl St {
    fn snapshot(&self) -> Snapshot {
        let mut desks: Vec<Desk> = self.desks.iter().map(|(_, d)| d.clone()).collect();
        // labwc sends no coordinates: by name, numbers in order ("2" before "10"), as the QML Mantel does.
        desks.sort_by(|a, b| natural(&a.name).cmp(&natural(&b.name)));
        Snapshot {
            wins: self.wins.iter().filter(|(_, _, done)| *done).map(|(_, w, _)| w.clone()).collect(),
            desks,
            has_windows: self.has_windows,
            has_desks: self.has_desks,
        }
    }
}

/// "Desktop 10" after "Desktop 2".
fn natural(s: &str) -> (String, u64) {
    let digits: String = s.chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
    let head = s[..s.len() - digits.len()].to_lowercase();
    (head, digits.parse().unwrap_or(0))
}

/// A stream of snapshots (the thread starts with the first one asked for).
pub fn stream() -> futures::channel::mpsc::UnboundedReceiver<Snapshot> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    if let Ok(mut s) = SENDERS.lock() {
        s.push(tx);
    }
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_ok() {
        let _ = std::thread::Builder::new().name("mantel-wm".into()).spawn(|| {
            if let Err(e) = run_loop() {
                eprintln!("hearth-mantel: the window list isn't there ({e}); the dock shows your pins only");
                send(Snapshot::default());
            }
        });
    }
    rx
}

fn send(s: Snapshot) {
    if let Ok(mut v) = SENDERS.lock() {
        v.retain(|tx| tx.unbounded_send(s.clone()).is_ok());
    }
}

fn run_loop() -> Result<(), String> {
    let conn = Connection::connect_to_env().map_err(|e| e.to_string())?;
    let (globals, mut queue) = registry_queue_init::<St>(&conn).map_err(|e| e.to_string())?;
    let qh = queue.handle();
    let seat: Option<wl_seat::WlSeat> = globals.bind(&qh, 1..=7, ()).ok();
    let tl: Option<TlManager> = globals.bind(&qh, 1..=3, ()).ok();
    let ws: Option<WsManager> = globals.bind(&qh, 1..=1, ()).ok();
    let mut st = St { has_windows: tl.is_some(), has_desks: ws.is_some(), dirty: true, ..St::default() };
    let _ = SHARED.set(Shared { conn: conn.clone(), seat, ws_mgr: ws, wins: Mutex::new(HashMap::new()), desks: Mutex::new(HashMap::new()) });
    loop {
        queue.blocking_dispatch(&mut st).map_err(|e| e.to_string())?;
        if st.dirty {
            st.dirty = false;
            if let Some(sh) = SHARED.get() {
                if let Ok(mut m) = sh.wins.lock() {
                    *m = st.wins.iter().map(|(h, w, _)| (w.id, h.clone())).collect();
                }
                if let Ok(mut m) = sh.desks.lock() {
                    *m = st.desks.iter().map(|(h, d)| (d.id, h.clone())).collect();
                }
            }
            send(st.snapshot());
        }
    }
}

// ---- what labwc says

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for St {
    fn event(_: &mut Self, _: &wl_registry::WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<wl_seat::WlSeat, ()> for St {
    fn event(_: &mut Self, _: &wl_seat::WlSeat, _: wl_seat::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<TlManager, ()> for St {
    fn event(st: &mut Self, _: &TlManager, ev: zwlr_foreign_toplevel_manager_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } = ev {
            let id = toplevel.id().protocol_id();
            st.wins.push((toplevel, Win { id, ..Win::default() }, false));
        }
    }
    event_created_child!(St, TlManager, [zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (TlHandle, ())]);
}

impl Dispatch<TlHandle, ()> for St {
    fn event(st: &mut Self, h: &TlHandle, ev: zwlr_foreign_toplevel_handle_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        use zwlr_foreign_toplevel_handle_v1::Event as E;
        let Some(i) = st.wins.iter().position(|(x, _, _)| x == h) else { return };
        match ev {
            E::Title { title } => st.wins[i].1.title = title,
            E::AppId { app_id } => st.wins[i].1.app_id = app_id,
            E::State { state } => {
                let s: Vec<u32> = state.chunks_exact(4).map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]])).collect();
                st.wins[i].1.minimized = s.contains(&1);
                st.wins[i].1.active = s.contains(&2);
            }
            E::Done => {
                st.wins[i].2 = true;
                st.dirty = true;
            }
            E::Closed => {
                st.wins.remove(i);
                h.destroy();
                st.dirty = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<WsManager, ()> for St {
    fn event(st: &mut Self, _: &WsManager, ev: ext_workspace_manager_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        use ext_workspace_manager_v1::Event as E;
        match ev {
            E::Workspace { workspace } => {
                let id = workspace.id().protocol_id();
                st.desks.push((workspace, Desk { id, ..Desk::default() }));
            }
            E::Done => st.dirty = true,
            _ => {}
        }
    }
    event_created_child!(St, WsManager, [
        ext_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => (ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, ()),
        ext_workspace_manager_v1::EVT_WORKSPACE_OPCODE => (WsHandle, ()),
    ]);
}

impl Dispatch<ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, ()> for St {
    fn event(_: &mut Self, _: &ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1, _: ext_workspace_group_handle_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WsHandle, ()> for St {
    fn event(st: &mut Self, h: &WsHandle, ev: ext_workspace_handle_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        use ext_workspace_handle_v1::Event as E;
        let Some(i) = st.desks.iter().position(|(x, _)| x == h) else { return };
        match ev {
            E::Name { name } => st.desks[i].1.name = name,
            E::State { state } => {
                st.desks[i].1.active = match state {
                    WEnum::Value(s) => s.contains(ext_workspace_handle_v1::State::Active),
                    WEnum::Unknown(v) => v & 1 == 1,
                }
            }
            E::Removed => {
                st.desks.remove(i);
                h.destroy();
                st.dirty = true;
            }
            _ => {}
        }
    }
}

// ---- what the bar and the dock ask

fn with_win(id: u32, f: impl FnOnce(&TlHandle, &Shared)) {
    if let Some(sh) = SHARED.get() {
        let h = sh.wins.lock().ok().and_then(|m| m.get(&id).cloned());
        if let Some(h) = h {
            f(&h, sh);
            let _ = sh.conn.flush();
        }
    }
}

/// Brings a window to the front (and back from minimized).
pub fn activate(id: u32) {
    with_win(id, |h, sh| {
        if let Some(seat) = &sh.seat {
            h.unset_minimized();
            h.activate(seat);
        }
    });
}

pub fn minimize(id: u32) {
    with_win(id, |h, _| h.set_minimized());
}

pub fn close(id: u32) {
    with_win(id, |h, _| h.close());
}

/// Switches to a desktop.
pub fn go_desk(id: u32) {
    if let Some(sh) = SHARED.get() {
        let h = sh.desks.lock().ok().and_then(|m| m.get(&id).cloned());
        if let (Some(h), Some(m)) = (h, &sh.ws_mgr) {
            h.activate();
            m.commit();
            let _ = sh.conn.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::natural;

    #[test]
    fn desktops_sort_like_numbers() {
        let mut v = vec!["10", "2", "1"];
        v.sort_by(|a, b| natural(a).cmp(&natural(b)));
        assert_eq!(v, ["1", "2", "10"]);
    }
}

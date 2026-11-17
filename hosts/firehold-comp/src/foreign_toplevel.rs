//! wlr-foreign-toplevel-management (v3): the open windows for Mantel's taskbar and dock, exactly as labwc offers
//! them, so Quickshell's ToplevelManager (Bar.qml, Dock.qml, Islands.qml...) works unchanged: title, app id, the
//! screen it's on, maximized / minimized / activated / fullscreen, and the taskbar's requests back (activate, close,
//! minimize, maximize, fullscreen).
//!
//! The compositor calls `refresh` with what every window is now (wm.rs `toplevel_infos`) after each round of client
//! requests; this sends only what changed, then "done", to every taskbar that's listening.

use std::collections::HashMap;

use smithay::{
    output::Output,
    reexports::{
        wayland_protocols_wlr::foreign_toplevel::v1::server::{
            zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
            zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
        },
        wayland_server::{
            backend::ClientId, protocol::wl_output::WlOutput, Client, DataInit, Dispatch, DisplayHandle,
            GlobalDispatch, New, Resource,
        },
    },
};

use crate::state::{Backend, HearthState};

/// What a taskbar is told about one window.
#[derive(Debug, Clone, PartialEq)]
pub struct ToplevelInfo {
    pub id: u64,
    pub title: String,
    pub app_id: String,
    pub maximized: bool,
    pub minimized: bool,
    pub activated: bool,
    pub fullscreen: bool,
    pub output: Option<Output>,
}

impl ToplevelInfo {
    fn states(&self) -> Vec<u8> {
        use zwlr_foreign_toplevel_handle_v1::State;
        let mut s: Vec<u32> = Vec::new();
        if self.maximized {
            s.push(State::Maximized as u32);
        }
        if self.minimized {
            s.push(State::Minimized as u32);
        }
        if self.activated {
            s.push(State::Activated as u32);
        }
        if self.fullscreen {
            s.push(State::Fullscreen as u32);
        }
        s.iter().flat_map(|v| v.to_ne_bytes()).collect()
    }
}

/// A request from a taskbar, for the window with this id.
#[derive(Debug, Clone)]
pub enum ToplevelRequest {
    Activate(u64),
    Close(u64),
    SetMaximized(u64, bool),
    SetMinimized(u64, bool),
    SetFullscreen(u64, bool, Option<WlOutput>),
}

struct Known {
    info: ToplevelInfo,
    handles: Vec<ZwlrForeignToplevelHandleV1>,
}

#[derive(Default)]
pub struct ForeignToplevelState {
    managers: Vec<ZwlrForeignToplevelManagerV1>,
    known: HashMap<u64, Known>,
    /// In the order windows appeared (taskbars list them that way).
    order: Vec<u64>,
    pub requests: Vec<ToplevelRequest>,
}

impl std::fmt::Debug for ForeignToplevelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForeignToplevelState")
            .field("managers", &self.managers.len())
            .field("windows", &self.order.len())
            .finish()
    }
}

impl ForeignToplevelState {
    pub fn new<B: Backend + 'static>(dh: &DisplayHandle) -> Self {
        dh.create_global::<HearthState<B>, ZwlrForeignToplevelManagerV1, _>(3, ());
        ForeignToplevelState::default()
    }

    pub fn listeners(&self) -> usize {
        self.managers.len()
    }

    fn announce<B: Backend + 'static>(
        dh: &DisplayHandle,
        manager: &ZwlrForeignToplevelManagerV1,
        info: &ToplevelInfo,
    ) -> Option<ZwlrForeignToplevelHandleV1> {
        let client = manager.client()?;
        let handle = client
            .create_resource::<ZwlrForeignToplevelHandleV1, u64, HearthState<B>>(dh, manager.version(), info.id)
            .ok()?;
        manager.toplevel(&handle);
        handle.title(info.title.clone());
        handle.app_id(info.app_id.clone());
        if let Some(o) = &info.output {
            for wl in o.client_outputs(&client) {
                handle.output_enter(&wl);
            }
        }
        handle.state(info.states());
        if handle.version() >= 3 {
            handle.parent(None);
        }
        handle.done();
        Some(handle)
    }

    /// Tells every taskbar what changed since the last time.
    pub fn refresh<B: Backend + 'static>(&mut self, dh: &DisplayHandle, now: Vec<ToplevelInfo>) {
        let ids: Vec<u64> = now.iter().map(|i| i.id).collect();
        // Gone: closed.
        let gone: Vec<u64> = self.order.iter().copied().filter(|id| !ids.contains(id)).collect();
        for id in gone {
            if let Some(k) = self.known.remove(&id) {
                for h in &k.handles {
                    if h.is_alive() {
                        h.closed();
                    }
                }
            }
            self.order.retain(|x| *x != id);
        }
        for info in now {
            if !self.known.contains_key(&info.id) {
                let handles = self
                    .managers
                    .iter()
                    .filter_map(|m| Self::announce::<B>(dh, m, &info))
                    .collect();
                self.order.push(info.id);
                self.known.insert(info.id, Known { info, handles });
                continue;
            }
            let Some(k) = self.known.get_mut(&info.id) else { continue };
            if k.info == info {
                continue;
            }
            k.handles.retain(|h| h.is_alive());
            for h in &k.handles {
                let Some(client) = h.client() else { continue };
                if k.info.title != info.title {
                    h.title(info.title.clone());
                }
                if k.info.app_id != info.app_id {
                    h.app_id(info.app_id.clone());
                }
                if k.info.output != info.output {
                    if let Some(o) = &k.info.output {
                        for wl in o.client_outputs(&client) {
                            h.output_leave(&wl);
                        }
                    }
                    if let Some(o) = &info.output {
                        for wl in o.client_outputs(&client) {
                            h.output_enter(&wl);
                        }
                    }
                }
                h.state(info.states());
                h.done();
            }
            k.info = info;
        }
    }
}

impl<B: Backend + 'static> GlobalDispatch<ZwlrForeignToplevelManagerV1, ()> for HearthState<B> {
    fn bind(
        state: &mut Self,
        dh: &DisplayHandle,
        _client: &Client,
        resource: New<ZwlrForeignToplevelManagerV1>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        let manager = data_init.init(resource, ());
        let ft = &mut state.foreign_toplevel_state;
        for id in ft.order.clone() {
            if let Some(k) = ft.known.get_mut(&id) {
                if let Some(h) = ForeignToplevelState::announce::<B>(dh, &manager, &k.info) {
                    k.handles.push(h);
                }
            }
        }
        ft.managers.push(manager);
    }
}

impl<B: Backend + 'static> Dispatch<ZwlrForeignToplevelManagerV1, ()> for HearthState<B> {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &ZwlrForeignToplevelManagerV1,
        request: zwlr_foreign_toplevel_manager_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Request::Stop = request {
            state.foreign_toplevel_state.managers.retain(|m| m != resource);
            resource.finished();
        }
    }

    fn destroyed(state: &mut Self, _client: ClientId, resource: &ZwlrForeignToplevelManagerV1, _data: &()) {
        state.foreign_toplevel_state.managers.retain(|m| m != resource);
    }
}

impl<B: Backend + 'static> Dispatch<ZwlrForeignToplevelHandleV1, u64> for HearthState<B> {
    fn request(
        state: &mut Self,
        _client: &Client,
        _resource: &ZwlrForeignToplevelHandleV1,
        request: zwlr_foreign_toplevel_handle_v1::Request,
        id: &u64,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        use zwlr_foreign_toplevel_handle_v1::Request;
        let id = *id;
        let r = match request {
            Request::SetMaximized => ToplevelRequest::SetMaximized(id, true),
            Request::UnsetMaximized => ToplevelRequest::SetMaximized(id, false),
            Request::SetMinimized => ToplevelRequest::SetMinimized(id, true),
            Request::UnsetMinimized => ToplevelRequest::SetMinimized(id, false),
            Request::Activate { .. } => ToplevelRequest::Activate(id),
            Request::Close => ToplevelRequest::Close(id),
            Request::SetFullscreen { output } => ToplevelRequest::SetFullscreen(id, true, output),
            Request::UnsetFullscreen => ToplevelRequest::SetFullscreen(id, false, None),
            // Where the taskbar button is (for minimize animations): nothing to do yet.
            Request::SetRectangle { .. } | Request::Destroy => return,
            _ => return,
        };
        // Done after this round of requests (wm.rs), outside the protocol's dispatch.
        state.foreign_toplevel_state.requests.push(r);
    }

    fn destroyed(state: &mut Self, _client: ClientId, resource: &ZwlrForeignToplevelHandleV1, id: &u64) {
        if let Some(k) = state.foreign_toplevel_state.known.get_mut(id) {
            k.handles.retain(|h| h != resource);
        }
    }
}

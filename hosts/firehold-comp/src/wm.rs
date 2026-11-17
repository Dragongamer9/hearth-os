//! Hearth's window management on top of Anvil's: minimizing (Mantel's taskbar and Show desktop), activating, closing,
//! snapping to the screen's edges (dragging a window there, or Super+Left/Right/Up), Alt/Super+Tab, Mantel's
//! keyboard shortcuts from rc.xml, the taskbar list (foreign_toplevel.rs), the autostart, and a small status file
//! for tests and Hearth ($XDG_RUNTIME_DIR/firehold.json: the screens, Mantel's layer surfaces and the
//! windows). After every round of client requests `hearth_tick` runs.

use std::{
    cell::RefCell,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use smithay::{
    desktop::{layer_map_for_output, space::SpaceElement, WindowSurface},
    output::Output,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::protocol::wl_output::WlOutput,
    },
    utils::{IsAlive, Logical, Point, Rectangle, Size, SERIAL_COUNTER},
    wayland::{compositor::with_states, shell::xdg::XdgToplevelSurfaceData},
};
use tracing::{info, warn};

use crate::{
    focus::KeyboardFocusTarget,
    foreign_toplevel::{ToplevelInfo, ToplevelRequest},
    keybinds::{Action, Edge},
    shell::{ssd::HEADER_BAR_HEIGHT, WindowElement},
    state::{Backend, HearthState},
};

/// Each window's number for the taskbar (in its user data).
struct ToplevelId(u64);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Where a snapped or maximized window was before, to go back to.
#[derive(Default)]
pub struct Restore(pub RefCell<Option<Rectangle<i32, Logical>>>);

/// How close to a screen's edge a dragged window snaps (logical pixels).
pub const SNAP_EDGE: f64 = 6.0;

#[derive(Debug, Clone)]
pub struct Minimized {
    pub window: WindowElement,
    pub location: Point<i32, Logical>,
    pub output: Option<Output>,
}

pub fn window_id(w: &WindowElement) -> u64 {
    w.user_data()
        .insert_if_missing(|| ToplevelId(NEXT_ID.fetch_add(1, Ordering::Relaxed)));
    w.user_data().get::<ToplevelId>().map(|i| i.0).unwrap_or(0)
}

pub fn title_and_app_id(w: &WindowElement) -> (String, String) {
    match w.0.underlying_surface() {
        WindowSurface::Wayland(t) => with_states(t.wl_surface(), |states| {
            states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .map(|d| {
                    let d = d.lock().unwrap();
                    (d.title.clone().unwrap_or_default(), d.app_id.clone().unwrap_or_default())
                })
                .unwrap_or_default()
        }),
        #[cfg(feature = "xwayland")]
        WindowSurface::X11(x) => (x.title(), x.class()),
    }
}

fn is_maximized(w: &WindowElement) -> bool {
    match w.0.underlying_surface() {
        WindowSurface::Wayland(t) => t.current_state().states.contains(xdg_toplevel::State::Maximized),
        #[cfg(feature = "xwayland")]
        WindowSurface::X11(x) => x.is_maximized(),
    }
}

fn is_fullscreen(w: &WindowElement) -> bool {
    match w.0.underlying_surface() {
        WindowSurface::Wayland(t) => t.current_state().states.contains(xdg_toplevel::State::Fullscreen),
        #[cfg(feature = "xwayland")]
        WindowSurface::X11(x) => x.is_fullscreen(),
    }
}

/// An X11 window that isn't a real window (menus, tooltips: override-redirect) never goes on the taskbar.
fn is_taskbar_window(w: &WindowElement) -> bool {
    match w.0.underlying_surface() {
        WindowSurface::Wayland(_) => true,
        #[cfg(feature = "xwayland")]
        WindowSurface::X11(x) => !x.is_override_redirect(),
    }
}

impl<B: Backend + 'static> HearthState<B> {
    /// The part of a screen windows get: without Mantel's bar and dock (their exclusive zones).
    pub fn work_area(&self, output: &Output) -> Rectangle<i32, Logical> {
        let geo = self.space.output_geometry(output).unwrap_or_default();
        let zone = layer_map_for_output(output).non_exclusive_zone();
        Rectangle::new(geo.loc + zone.loc, zone.size)
    }

    fn output_for(&self, w: &WindowElement) -> Option<Output> {
        self.space
            .outputs_for_element(w)
            .first()
            .cloned()
            .or_else(|| self.space.outputs().next().cloned())
    }

    pub fn focused_window(&self) -> Option<WindowElement> {
        let focus = self.seat.get_keyboard()?.current_focus()?;
        match focus {
            KeyboardFocusTarget::Window(win) => self.space.elements().find(|e| e.0 == win).cloned(),
            _ => None,
        }
    }

    fn find_window(&self, id: u64) -> Option<WindowElement> {
        self.space
            .elements()
            .find(|w| window_id(w) == id)
            .cloned()
            .or_else(|| self.minimized.iter().find(|m| window_id(&m.window) == id).map(|m| m.window.clone()))
    }

    /// Every window as Mantel's taskbar sees it, in the order they opened.
    pub fn toplevel_infos(&self) -> Vec<ToplevelInfo> {
        let focused = self.focused_window();
        let mut out: Vec<ToplevelInfo> = self
            .space
            .elements()
            .filter(|w| w.alive() && is_taskbar_window(w))
            .map(|w| {
                let (title, app_id) = title_and_app_id(w);
                ToplevelInfo {
                    id: window_id(w),
                    title,
                    app_id,
                    maximized: is_maximized(w),
                    minimized: false,
                    activated: focused.as_ref() == Some(w),
                    fullscreen: is_fullscreen(w),
                    output: self.space.outputs_for_element(w).first().cloned(),
                }
            })
            .collect();
        out.extend(self.minimized.iter().filter(|m| m.window.alive()).map(|m| {
            let (title, app_id) = title_and_app_id(&m.window);
            ToplevelInfo {
                id: window_id(&m.window),
                title,
                app_id,
                maximized: is_maximized(&m.window),
                minimized: true,
                activated: false,
                fullscreen: false,
                output: m.output.clone(),
            }
        }));
        out.sort_by_key(|i| i.id);
        out
    }

    /// A new window gets the keyboard once it shows something (its first picture).
    pub fn focus_new_window(&mut self, surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface) {
        struct FocusedOnce;
        let Some(w) = self.window_for_surface(surface) else { return };
        if w.user_data().get::<FocusedOnce>().is_some() || SpaceElement::bbox(&w).size.w <= 0 {
            return;
        }
        w.user_data().insert_if_missing(|| FocusedOnce);
        self.activate_window(&w);
    }

    /// Raises a window and gives it the keyboard (bringing it back first when it's minimized).
    pub fn activate_window(&mut self, w: &WindowElement) {
        if let Some(pos) = self.minimized.iter().position(|m| &m.window == w) {
            let m = self.minimized.remove(pos);
            m.window.0.on_commit();
            self.space.map_element(m.window.clone(), m.location, true);
        }
        self.space.raise_element(w, true);
        #[cfg(feature = "xwayland")]
        if let Some(x) = w.0.x11_surface() {
            if let Some(xwm) = self.xwm.as_mut() {
                let _ = xwm.raise_window(x);
            }
        }
        if let Some(k) = self.seat.get_keyboard() {
            k.set_focus(self, Some(KeyboardFocusTarget::from(w.clone())), SERIAL_COUNTER.next_serial());
        }
    }

    pub fn minimize_window(&mut self, w: &WindowElement) {
        let Some(location) = self.space.element_location(w) else { return };
        let output = self.output_for(w);
        let was_focused = self.focused_window().as_ref() == Some(w);
        self.space.unmap_elem(w);
        w.set_activate(false);
        self.minimized.push(Minimized { window: w.clone(), location, output });
        if was_focused {
            self.focus_top_window();
        }
    }

    /// The keyboard to the topmost window left (or nobody).
    pub fn focus_top_window(&mut self) {
        let top = self.space.elements().rev().find(|w| is_taskbar_window(w)).cloned();
        match top {
            Some(t) => self.activate_window(&t),
            None => {
                if let Some(k) = self.seat.get_keyboard() {
                    k.set_focus(self, None, SERIAL_COUNTER.next_serial());
                }
            }
        }
    }

    pub fn close_window(&mut self, w: &WindowElement) {
        match w.0.underlying_surface() {
            WindowSurface::Wayland(t) => t.send_close(),
            #[cfg(feature = "xwayland")]
            WindowSurface::X11(x) => {
                let _ = x.close();
            }
        }
    }

    pub fn set_maximized(&mut self, w: &WindowElement, on: bool) {
        match w.0.underlying_surface() {
            WindowSurface::Wayland(t) => {
                use smithay::wayland::shell::xdg::XdgShellHandler;
                if on {
                    self.maximize_request(t.clone())
                } else {
                    self.unmaximize_request(t.clone())
                }
            }
            #[cfg(feature = "xwayland")]
            WindowSurface::X11(x) => {
                if on {
                    self.maximize_request_x11(x)
                } else if let Some(id) = self.xwm.as_ref().map(|x| x.id()) {
                    use smithay::xwayland::XwmHandler;
                    self.unmaximize_request(id, x.clone())
                }
            }
        }
    }

    pub fn set_fullscreen(&mut self, w: &WindowElement, on: bool, output: Option<WlOutput>) {
        match w.0.underlying_surface() {
            WindowSurface::Wayland(t) => {
                use smithay::wayland::shell::xdg::XdgShellHandler;
                if on {
                    self.fullscreen_request(t.clone(), output)
                } else {
                    self.unfullscreen_request(t.clone())
                }
            }
            #[cfg(feature = "xwayland")]
            WindowSurface::X11(x) => {
                if let Some(id) = self.xwm.as_ref().map(|x| x.id()) {
                    use smithay::xwayland::XwmHandler;
                    if on {
                        self.fullscreen_request(id, x.clone())
                    } else {
                        self.unfullscreen_request(id, x.clone())
                    }
                }
            }
        }
    }

    /// Puts a window on half the screen (left, right) or all of it (top), without the bar and dock. Its size before
    /// is kept, and it goes back to that when it's dragged away (`unsnap`).
    pub fn snap_window(&mut self, w: &WindowElement, edge: Edge, output: Option<Output>) {
        let Some(output) = output.or_else(|| self.output_for(w)) else { return };
        if edge == Edge::Top {
            self.space.raise_element(w, true);
            self.set_maximized(w, true);
            return;
        }
        let area = self.work_area(&output);
        let half = area.size.w / 2;
        let rect = match edge {
            Edge::Left => Rectangle::new(area.loc, Size::from((half, area.size.h))),
            _ => Rectangle::new(
                Point::from((area.loc.x + half, area.loc.y)),
                Size::from((area.size.w - half, area.size.h)),
            ),
        };
        if let Some(cur) = self.space.element_geometry(w) {
            w.user_data().insert_if_missing(Restore::default);
            let r = w.user_data().get::<Restore>().unwrap();
            if r.0.borrow().is_none() {
                *r.0.borrow_mut() = Some(cur);
            }
        }
        let header = if w.decoration_state().is_ssd { HEADER_BAR_HEIGHT } else { 0 };
        let client = Size::from((rect.size.w, (rect.size.h - header).max(1)));
        match w.0.underlying_surface() {
            WindowSurface::Wayland(t) => {
                t.with_pending_state(|s| {
                    s.states.unset(xdg_toplevel::State::Maximized);
                    s.states.unset(xdg_toplevel::State::TiledLeft);
                    s.states.unset(xdg_toplevel::State::TiledRight);
                    s.states.set(xdg_toplevel::State::TiledTop);
                    s.states.set(xdg_toplevel::State::TiledBottom);
                    s.states.set(if edge == Edge::Left {
                        xdg_toplevel::State::TiledRight
                    } else {
                        xdg_toplevel::State::TiledLeft
                    });
                    s.size = Some(client);
                });
                if t.is_initial_configure_sent() {
                    t.send_pending_configure();
                }
            }
            #[cfg(feature = "xwayland")]
            WindowSurface::X11(x) => {
                let _ = x.configure(Rectangle::new(rect.loc + Point::from((0, header)), client));
            }
        }
        self.space.map_element(w.clone(), rect.loc, true);
    }

    /// A snapped window dragged away gets its size from before back. True when it was snapped.
    pub fn unsnap(&mut self, w: &WindowElement) -> bool {
        let Some(prev) = w.user_data().get::<Restore>().and_then(|r| r.0.borrow_mut().take()) else {
            return false;
        };
        let header = if w.decoration_state().is_ssd { HEADER_BAR_HEIGHT } else { 0 };
        let client = Size::from((prev.size.w, (prev.size.h - header).max(1)));
        match w.0.underlying_surface() {
            WindowSurface::Wayland(t) => {
                t.with_pending_state(|s| {
                    s.states.unset(xdg_toplevel::State::TiledLeft);
                    s.states.unset(xdg_toplevel::State::TiledRight);
                    s.states.unset(xdg_toplevel::State::TiledTop);
                    s.states.unset(xdg_toplevel::State::TiledBottom);
                    s.size = Some(client);
                });
                if t.is_initial_configure_sent() {
                    t.send_pending_configure();
                }
            }
            #[cfg(feature = "xwayland")]
            WindowSurface::X11(x) => {
                let loc = self.space.element_location(w).unwrap_or_default();
                let _ = x.configure(Rectangle::new(loc + Point::from((0, header)), client));
            }
        }
        true
    }

    /// Where a window dragged to `pointer` would snap, if anywhere (the screen's left, right or top edge).
    pub fn snap_edge_at(&self, pointer: Point<f64, Logical>) -> Option<(Edge, Output)> {
        let output = self.space.output_under(pointer).next()?.clone();
        let geo = self.space.output_geometry(&output)?.to_f64();
        let edge = if pointer.x <= geo.loc.x + SNAP_EDGE {
            Edge::Left
        } else if pointer.x >= geo.loc.x + geo.size.w - 1.0 - SNAP_EDGE {
            Edge::Right
        } else if pointer.y <= geo.loc.y + SNAP_EDGE {
            Edge::Top
        } else {
            return None;
        };
        Some((edge, output))
    }

    /// Alt+Tab / Super+Tab: the next window (or the one before), minimized ones too.
    pub fn cycle_windows(&mut self, forward: bool) {
        let mut all: Vec<WindowElement> = self.space.elements().filter(|w| is_taskbar_window(w)).cloned().collect();
        all.extend(self.minimized.iter().map(|m| m.window.clone()));
        if all.is_empty() {
            return;
        }
        // Topmost first: the next one forward is the one under the top.
        let mapped = self.space.elements().filter(|w| is_taskbar_window(w)).count();
        all[..mapped].reverse();
        let next = if forward { all.get(1).or(all.first()) } else { all.last() };
        if let Some(n) = next.cloned() {
            self.activate_window(&n);
        }
    }

    pub fn run_action(&mut self, action: &Action) {
        match action {
            Action::Execute(cmd) => spawn_shell(cmd),
            Action::Close => {
                if let Some(w) = self.focused_window() {
                    self.close_window(&w)
                }
            }
            Action::NextWindow => self.cycle_windows(true),
            Action::PreviousWindow => self.cycle_windows(false),
            Action::Snap(edge) => {
                if let Some(w) = self.focused_window() {
                    self.unsnap(&w);
                    self.snap_window(&w, *edge, None)
                }
            }
            Action::Maximize => {
                if let Some(w) = self.focused_window() {
                    self.set_maximized(&w, true)
                }
            }
            Action::UnMaximize => {
                if let Some(w) = self.focused_window() {
                    if !self.unsnap(&w) {
                        self.set_maximized(&w, false)
                    }
                }
            }
            Action::ToggleMaximize => {
                if let Some(w) = self.focused_window() {
                    let on = !is_maximized(&w);
                    self.set_maximized(&w, on)
                }
            }
            Action::Iconify => {
                if let Some(w) = self.focused_window() {
                    self.minimize_window(&w)
                }
            }
            Action::ToggleFullscreen => {
                if let Some(w) = self.focused_window() {
                    let on = !is_fullscreen(&w);
                    self.set_fullscreen(&w, on, None)
                }
            }
            // The backend's (udev.rs: the session switches consoles).
            Action::VtSwitch(_) => {}
            Action::LeaveForKwin => {
                if self.lock.locked {
                    // Never a way around the lock screen.
                    return;
                }
                info!("Ctrl+Alt+Backspace: leaving Firehold for KWin");
                if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
                    let _ = std::fs::write(std::path::Path::new(&dir).join("firehold-leave"), b"kwin\n");
                }
                self.running.store(false, Ordering::SeqCst);
            }
        }
    }

    fn handle_toplevel_requests(&mut self) {
        let requests = std::mem::take(&mut self.foreign_toplevel_state.requests);
        if self.lock.locked {
            // Nothing is raised, closed or focused behind the lock screen.
            return;
        }
        for r in requests {
            let id = match &r {
                ToplevelRequest::Activate(id)
                | ToplevelRequest::Close(id)
                | ToplevelRequest::SetMaximized(id, _)
                | ToplevelRequest::SetMinimized(id, _)
                | ToplevelRequest::SetFullscreen(id, _, _) => *id,
            };
            let Some(w) = self.find_window(id) else { continue };
            match r {
                ToplevelRequest::Activate(_) => self.activate_window(&w),
                ToplevelRequest::Close(_) => self.close_window(&w),
                ToplevelRequest::SetMaximized(_, on) => {
                    if self.minimized.iter().any(|m| m.window == w) {
                        self.activate_window(&w);
                    }
                    self.set_maximized(&w, on)
                }
                ToplevelRequest::SetMinimized(_, true) => self.minimize_window(&w),
                ToplevelRequest::SetMinimized(_, false) => self.activate_window(&w),
                ToplevelRequest::SetFullscreen(_, on, output) => self.set_fullscreen(&w, on, output),
            }
        }
    }

    /// Starts the autostart (once): when XWayland is ready, or without it after a few seconds.
    pub fn start_autostart(&mut self, why: &str) {
        let Some(cmd) = self.autostart.take() else { return };
        if cmd.is_empty() {
            return;
        }
        let mut c = Command::new(&cmd[0]);
        c.args(&cmd[1..]).stdin(Stdio::null());
        #[cfg(feature = "xwayland")]
        if self.xdisplay.is_none() {
            c.env_remove("DISPLAY");
        }
        match c.spawn() {
            Ok(mut child) => {
                info!(cmd = ?cmd, why, "autostart started");
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            Err(e) => warn!(cmd = ?cmd, err = %e, "autostart couldn't start"),
        }
    }

    /// After every round of client requests (winit.rs and udev.rs call it).
    pub fn hearth_tick(&mut self) {
        if self.stop.load(Ordering::SeqCst) {
            info!("asked to end (signal): ending cleanly");
            self.running.store(false, Ordering::SeqCst);
        }
        self.minimized.retain(|m| m.window.alive());
        self.lock_tick();
        self.output_management_tick();
        self.fullscreen_sweep();
        self.game_tick();
        self.handle_toplevel_requests();
        let infos = self.toplevel_infos();
        let dh = self.display_handle.clone();
        self.foreign_toplevel_state.refresh::<B>(&dh, infos);
        if self.autostart.is_some() && self.started_at.elapsed().as_secs() >= 6 {
            self.start_autostart("XWayland wasn't ready after 6 s");
        }
        if self.last_status.0.elapsed().as_millis() >= 500 {
            self.write_status();
            if crate::theme::reload() {
                for w in self.space.elements() {
                    w.decoration_state().header_bar.theme_changed();
                }
            }
        }
    }

    /// The status file at once (the lock's changes shouldn't wait for the next half second).
    pub fn write_status_now(&mut self) {
        self.write_status();
    }

    /// What has the keyboard, for the status file (tests check nothing behind the lock screen has it).
    fn keyboard_focus_name(&self) -> String {
        let Some(f) = self.seat.get_keyboard().and_then(|k| k.current_focus()) else { return "none".into() };
        match f {
            KeyboardFocusTarget::Window(w) => {
                let app = self
                    .space
                    .elements()
                    .find(|e| e.0 == w)
                    .map(|e| title_and_app_id(e).1)
                    .unwrap_or_default();
                format!("window:{app}")
            }
            KeyboardFocusTarget::LayerSurface(l) => format!("layer:{}", l.namespace()),
            KeyboardFocusTarget::Popup(_) => "popup".into(),
            KeyboardFocusTarget::Lock(_) => "lock".into(),
        }
    }

    /// $XDG_RUNTIME_DIR/firehold.json, when it changed: for the tests, and for Hearth to see what's up.
    fn write_status(&mut self) {
        self.last_status.0 = std::time::Instant::now();
        let outputs: Vec<serde_json::Value> = self
            .space
            .outputs()
            .map(|o| {
                let geo = self.space.output_geometry(o).unwrap_or_default();
                let area = self.work_area(o);
                let map = layer_map_for_output(o);
                let layers: Vec<serde_json::Value> = map
                    .layers()
                    .map(|l| {
                        let g = map.layer_geometry(l).unwrap_or_default();
                        serde_json::json!({
                            "namespace": l.namespace(),
                            "layer": format!("{:?}", l.layer()),
                            "x": g.loc.x, "y": g.loc.y, "w": g.size.w, "h": g.size.h,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "name": o.name(),
                    "x": geo.loc.x, "y": geo.loc.y, "w": geo.size.w, "h": geo.size.h,
                    "scale": o.current_scale().fractional_scale(),
                    "make": o.physical_properties().make,
                    "model": o.physical_properties().model,
                    "modes": o.modes().len(),
                    "refresh": o.current_mode().map(|m| m.refresh).unwrap_or(0),
                    "workArea": [area.loc.x, area.loc.y, area.size.w, area.size.h],
                    "layers": layers,
                })
            })
            .collect();
        let windows: Vec<serde_json::Value> = self
            .toplevel_infos()
            .into_iter()
            .map(|i| {
                let g = self
                    .find_window(i.id)
                    .and_then(|w| self.space.element_geometry(&w))
                    .unwrap_or_default();
                let ssd = self.find_window(i.id).map(|w| w.decoration_state().is_ssd).unwrap_or(false);
                serde_json::json!({
                    "id": i.id, "title": i.title, "appId": i.app_id, "activated": i.activated,
                    "minimized": i.minimized, "maximized": i.maximized, "fullscreen": i.fullscreen,
                    "x": g.loc.x, "y": g.loc.y, "w": g.size.w, "h": g.size.h, "frame": ssd,
                })
            })
            .collect();
        let v = serde_json::json!({
            "pid": std::process::id(),
            "socket": self.socket_name,
            "xdisplay": self.x_display_name(),
            "outputs": outputs,
            "windows": windows,
            "taskbars": self.foreign_toplevel_state.listeners(),
            "keyboardFocus": self.keyboard_focus_name(),
            "captures": { "wlr": self.captures.taken_wlr, "ext": self.captures.taken_ext },
            "gammaHolders": self.gamma_state.holders(),
            "game": self.game_status()["game"].clone(),
            "lock": {
                "locked": self.lock.locked,
                "client": self.lock.locked && self.lock.client_alive() && !self.lock.client_gone,
                "prompt": self.lock.prompt_active(),
                "surfaces": self.lock.surfaces.iter().filter(|(_, s)| s.alive()).count(),
                "message": self.lock.prompt.message,
                "typed": self.lock.prompt.typed.chars().count(),
                "busy": self.lock.prompt.busy,
            },
        });
        let text = v.to_string();
        if text == self.last_status.1 {
            return;
        }
        let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") else { return };
        let path = std::path::Path::new(&dir).join("firehold.json");
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, &text).and_then(|_| std::fs::rename(&tmp, &path)).is_ok() {
            self.last_status.1 = text;
        }
    }

    fn x_display_name(&self) -> Option<String> {
        #[cfg(feature = "xwayland")]
        {
            self.xdisplay.map(|d| format!(":{d}"))
        }
        #[cfg(not(feature = "xwayland"))]
        {
            None
        }
    }

    /// Ending: the status file goes (nothing of the person's is touched).
    pub fn cleanup(&mut self) {
        self.game_cleanup();
        if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
            let _ = std::fs::remove_file(std::path::Path::new(&dir).join("firehold.json"));
        }
    }
}

/// A shortcut's command, as labwc runs it: through sh, with the session's environment.
pub fn spawn_shell(cmd: &str) {
    match Command::new("sh").arg("-c").arg(cmd).stdin(Stdio::null()).spawn() {
        Ok(mut child) => {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => warn!(cmd, err = %e, "couldn't start a shortcut's command"),
    }
}

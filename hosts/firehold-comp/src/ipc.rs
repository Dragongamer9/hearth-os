//! Mantel's window IPC: what KWin's hearthwindows script and hearth-kwin-bridge give Mantel on KWin, from Firehold's
//! own compositor. `$XDG_RUNTIME_DIR/firehold-wm-<wayland socket>.sock`, one JSON object a line both ways (a bare
//! word is `{"op": word}`). Mantel's side is `hearth-firehold-bridge` (WinList.qml runs it under HEARTH_WM=firehold).
//!
//! In (`op`):
//!   watch {input?}                     this connection gets `{"t": "windows", ...}` whenever anything in it changes;
//!                                      with `"input": true` also `{"t": "input", "kind": "mouse"|"keys"}` when a real
//!                                      mouse, touchpad or keyboard is used (at most once a second per kind: Firehold's
//!                                      Rust shell switches to the Desk on it; firehold-shell, docs/FIREHOLD-RUST.md)
//!   windows                            the list once
//!   activate|close {id}                minimize|maximize|fullscreen {id, on}
//!   snap {id, zone}                    left-half, right-half, maximize
//!   desktop {id, n, follow?}           a window to desktop n (1-based)
//!   goDesktop {n}  addDesktop {name?}  removeDesktop {n}  renameDesktop {n, name}
//!   shot {id, path, max?}              the window's picture as a PNG at `path` (in $XDG_RUNTIME_DIR), at most `max`
//!                                      wide; answered with `{"t": "shot", "id", "ok", "path", "w", "h"}` after the
//!                                      next frame
//!   frames {style, colours, names}     Firehold's own title bars (frames.rs) while this connection lasts
//!   key {keys: ["ctrl", "c"]} | {text} keys pressed (in order, let go in reverse) or text typed into the window that
//!                                      has the keyboard, as if typed on it (US layout; Firehold's on-screen keyboard
//!                                      and the Desk bar's File / Edit / View menus, firehold-shell)
//! Out: `{"t": "reply", "op", "ok", "error"?}` for each command, the list, and the pictures.
//!
//! The list: `{"t": "windows", "windows": [{id, appId, title, activated, minimized, maximized, fullscreen, keepAbove,
//! desktops, allDesktops, output, pid, frame: {x, y, w, h}, ssd, stack}], "active": id, "desktops": [{name, active}],
//! "stack": [ids, top first], "frames": "firehold"|"hearth", "outputs": [{name, model, x, y, w, h, mm: [w, h]}]}`
//! (ids are strings, as KWin's are).
//!
//! Nothing here can stop Firehold from starting: a socket that can't be made is logged and skipped, and nothing is
//! done while the screen is locked (but reading the list).

use std::{
    collections::HashMap,
    io::{ErrorKind, Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    time::Duration,
};

use serde_json::{json, Value};
use smithay::{
    desktop::space::SpaceElement,
    reexports::{
        calloop::{generic::Generic, Interest, Mode, PostAction},
        wayland_server::Resource,
    },
    utils::IsAlive,
};
use tracing::{info, warn};

use crate::{
    keybinds::Edge,
    shell::WindowElement,
    state::{Backend, HearthState},
    wm::{desktop_of, title_and_app_id, window_id},
};

#[derive(Debug)]
struct Watcher {
    id: u64,
    stream: UnixStream,
    inbuf: Vec<u8>,
    dead: bool,
    watching: bool,
    inputs: bool,
    sent: String,
}

#[derive(Debug, Default)]
pub struct IpcState {
    tried: bool,
    listener: Option<UnixListener>,
    pub socket_path: Option<PathBuf>,
    watchers: Vec<Watcher>,
    next: u64,
    last_input: Option<(&'static str, std::time::Instant)>,
    input_pending: Option<&'static str>,
}

impl IpcState {
    /// A real device was used (input_handler.rs): noted for the watchers that asked, at most once a second per kind.
    /// Only a field or two is touched here; it's sent in the next `ipc_tick`.
    pub fn note_input(&mut self, kind: &'static str) {
        if !self.watchers.iter().any(|w| w.inputs) {
            return;
        }
        let now = std::time::Instant::now();
        let due = match self.last_input {
            Some((k, at)) => k != kind || now.duration_since(at) >= Duration::from_secs(1),
            None => true,
        };
        if due {
            self.last_input = Some((kind, now));
            self.input_pending = Some(kind);
        }
    }
}

/// What kind of device an input event came from, for `note_input` (None: not one that says which posture).
pub fn input_kind<B: smithay::backend::input::InputBackend>(event: &smithay::backend::input::InputEvent<B>) -> Option<&'static str> {
    use smithay::backend::input::InputEvent;
    match event {
        InputEvent::Keyboard { .. } => Some("keys"),
        InputEvent::PointerMotion { .. } | InputEvent::PointerMotionAbsolute { .. } | InputEvent::PointerButton { .. } | InputEvent::PointerAxis { .. } => Some("mouse"),
        _ => None,
    }
}

/// A command line: JSON, or a bare word.
pub fn parse_line(line: &str) -> Result<Value, String> {
    let t = line.trim();
    if t.starts_with('{') {
        let v: Value = serde_json::from_str(t).map_err(|e| format!("not JSON: {e}"))?;
        if v.get("op").and_then(|o| o.as_str()).is_none() {
            return Err("no \"op\"".into());
        }
        Ok(v)
    } else if t.is_empty() {
        Err("empty".into())
    } else {
        Ok(json!({ "op": t.split_whitespace().next().unwrap_or("") }))
    }
}

/// A window id as Mantel sends it (a string, as KWin's are, or a number).
pub fn id_of(v: &Value) -> Option<u64> {
    match v.get("id")? {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.as_u64(),
        _ => None,
    }
}

/// Where a window picture may be written: a .png file under $XDG_RUNTIME_DIR, no "..".
pub fn shot_path_ok(path: &str, runtime: &Path) -> Option<PathBuf> {
    let p = Path::new(path);
    if !p.is_absolute() || p.extension().and_then(|e| e.to_str()) != Some("png") {
        return None;
    }
    if p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return None;
    }
    p.starts_with(runtime).then(|| p.to_path_buf())
}

/// A key's Linux code (evdev) by its name: letters, digits, the named keys and the modifiers.
pub fn key_code(name: &str) -> Option<u32> {
    let n = name.to_ascii_lowercase();
    if n.chars().count() == 1 {
        return char_key(n.chars().next()?).map(|(c, _)| c);
    }
    Some(match n.as_str() {
        "ctrl" | "control" => 29,
        "shift" => 42,
        "alt" => 56,
        "super" | "meta" => 125,
        "enter" | "return" => 28,
        "backspace" | "bksp" => 14,
        "tab" => 15,
        "esc" | "escape" => 1,
        "space" => 57,
        "left" => 105,
        "right" => 106,
        "up" => 103,
        "down" => 108,
        "home" => 102,
        "end" => 107,
        "delete" | "del" => 111,
        "pageup" => 104,
        "pagedown" => 109,
        "f11" => 87,
        "f5" => 63,
        "plus" | "equal" => 13,
        "minus" => 12,
        _ => return None,
    })
}

/// A character on a US keyboard: its key and whether Shift is held for it. Anything else isn't typed.
pub fn char_key(ch: char) -> Option<(u32, bool)> {
    const ROW1: &str = "qwertyuiop";
    const ROW2: &str = "asdfghjkl";
    const ROW3: &str = "zxcvbnm";
    let lower = ch.to_ascii_lowercase();
    let shift = ch.is_ascii_uppercase();
    if let Some(i) = ROW1.find(lower) {
        return Some((16 + i as u32, shift));
    }
    if let Some(i) = ROW2.find(lower) {
        return Some((30 + i as u32, shift));
    }
    if let Some(i) = ROW3.find(lower) {
        return Some((44 + i as u32, shift));
    }
    let plain = "1234567890-=[];'`\\,./";
    let shifted = "!@#$%^&*()_+{}:\"~|<>?";
    let codes = [2u32, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 26, 27, 39, 40, 41, 43, 51, 52, 53];
    if let Some(i) = plain.chars().position(|c| c == ch) {
        return Some((codes[i], false));
    }
    if let Some(i) = shifted.chars().position(|c| c == ch) {
        return Some((codes[i], true));
    }
    match ch {
        ' ' => Some((57, false)),
        '\n' => Some((28, false)),
        '\t' => Some((15, false)),
        _ => None,
    }
}

/// What `key` presses and lets go, in order: (code, pressed). Keys: each pressed in turn, let go in reverse (a
/// shortcut); text: each character pressed and let go (with Shift around it where it needs one). At most 400 steps.
pub fn key_steps(keys: &[String], text: &str) -> Result<Vec<(u32, bool)>, String> {
    let mut v = Vec::new();
    if !keys.is_empty() {
        let mut codes = Vec::new();
        for k in keys.iter().take(6) {
            codes.push(key_code(k).ok_or_else(|| format!("no key called {k}"))?);
        }
        v.extend(codes.iter().map(|c| (*c, true)));
        v.extend(codes.iter().rev().map(|c| (*c, false)));
    }
    for ch in text.chars().take(200) {
        let Some((code, shift)) = char_key(ch) else { continue };
        if shift {
            v.push((42, true));
        }
        v.push((code, true));
        v.push((code, false));
        if shift {
            v.push((42, false));
        }
    }
    if v.is_empty() {
        return Err("nothing to type".into());
    }
    v.truncate(400);
    Ok(v)
}

fn on(v: &Value) -> bool {
    v.get("on").and_then(|o| o.as_bool()).unwrap_or(true)
}

fn num(v: &Value, k: &str) -> Option<usize> {
    v.get(k).and_then(|n| n.as_u64()).map(|n| n as usize)
}

impl<B: Backend + 'static> HearthState<B> {
    fn ipc_start(&mut self) {
        let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") else { return };
        let name = self.socket_name.clone().unwrap_or_else(|| "wayland".into());
        let path = Path::new(&dir).join(format!("firehold-wm-{name}.sock"));
        let _ = std::fs::remove_file(&path);
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                warn!(path = %path.display(), err = %e, "Firehold windows: no IPC socket (Mantel's window list stays on the taskbar protocol)");
                return;
            }
        };
        let _ = listener.set_nonblocking(true);
        let Ok(watched) = listener.try_clone() else { return };
        let res = self.handle.insert_source(Generic::new(watched, Interest::READ, Mode::Level), |_, _, state| {
            state.ipc_accept();
            Ok(PostAction::Continue)
        });
        if let Err(e) = res {
            warn!(err = %e, "Firehold windows: the IPC socket couldn't be watched");
            let _ = std::fs::remove_file(&path);
            return;
        }
        info!(path = %path.display(), "Firehold windows: IPC socket ready");
        self.ipc.listener = Some(listener);
        self.ipc.socket_path = Some(path);
    }

    fn ipc_accept(&mut self) {
        for _ in 0..8 {
            let stream = match self.ipc.listener.as_ref().map(|l| l.accept()) {
                Some(Ok((s, _))) => s,
                _ => return,
            };
            let _ = stream.set_nonblocking(true);
            let Ok(watched) = stream.try_clone() else { continue };
            self.ipc.next += 1;
            let id = self.ipc.next;
            let res = self.handle.insert_source(Generic::new(watched, Interest::READ, Mode::Level), move |_, _, state| {
                Ok(if state.ipc_read(id) { PostAction::Continue } else { PostAction::Remove })
            });
            if res.is_err() {
                continue;
            }
            self.ipc.watchers.push(Watcher { id, stream, inbuf: Vec::new(), dead: false, watching: false, inputs: false, sent: String::new() });
        }
    }

    /// Reads what a connection sent and answers each line. False when it's gone.
    fn ipc_read(&mut self, id: u64) -> bool {
        let mut lines = Vec::new();
        {
            let Some(w) = self.ipc.watchers.iter_mut().find(|w| w.id == id) else { return false };
            let mut buf = [0u8; 8192];
            loop {
                match (&w.stream).read(&mut buf) {
                    Ok(0) => {
                        w.dead = true;
                        break;
                    }
                    Ok(n) => w.inbuf.extend_from_slice(&buf[..n]),
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(_) => {
                        w.dead = true;
                        break;
                    }
                }
                if w.inbuf.len() > 1 << 20 {
                    w.dead = true;
                    break;
                }
            }
            while let Some(pos) = w.inbuf.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = w.inbuf.drain(..=pos).collect();
                lines.push(String::from_utf8_lossy(&line).to_string());
            }
        }
        for l in lines {
            let reply = match parse_line(&l) {
                Ok(v) => self.ipc_command(id, &v),
                Err(e) => json!({ "t": "reply", "ok": false, "error": e }),
            };
            self.ipc_send(id, &reply);
        }
        let dead = self.ipc.watchers.iter().find(|w| w.id == id).is_none_or(|w| w.dead);
        if dead {
            self.ipc_drop(id);
        }
        !dead
    }

    fn ipc_drop(&mut self, id: u64) {
        self.ipc.watchers.retain(|w| w.id != id);
        if crate::frames::owner_gone(id) {
            info!("Firehold's title bars: off (Mantel's connection ended)");
        }
    }

    fn ipc_send(&mut self, id: u64, v: &Value) {
        let Some(w) = self.ipc.watchers.iter_mut().find(|w| w.id == id) else { return };
        if w.dead {
            return;
        }
        let line = format!("{v}\n");
        // A short wait at most: Mantel reads all the time; one that doesn't is dropped (it connects again).
        let _ = w.stream.set_nonblocking(false);
        let _ = w.stream.set_write_timeout(Some(Duration::from_millis(100)));
        if (&w.stream).write_all(line.as_bytes()).is_err() {
            w.dead = true;
            let _ = w.stream.shutdown(std::net::Shutdown::Both);
        }
        let _ = w.stream.set_nonblocking(true);
    }

    fn ipc_command(&mut self, from: u64, v: &Value) -> Value {
        let op = v.get("op").and_then(|o| o.as_str()).unwrap_or("").to_string();
        let ok = |extra: Value| {
            let mut r = json!({ "t": "reply", "op": op, "ok": true });
            if let (Some(r), Some(e)) = (r.as_object_mut(), extra.as_object()) {
                for (k, x) in e {
                    r.insert(k.clone(), x.clone());
                }
            }
            r
        };
        let err = |e: &str| json!({ "t": "reply", "op": op, "ok": false, "error": e });
        match op.as_str() {
            "watch" => {
                let inputs = v.get("input").and_then(|i| i.as_bool()).unwrap_or(false);
                if let Some(w) = self.ipc.watchers.iter_mut().find(|w| w.id == from) {
                    w.watching = true;
                    w.inputs = inputs;
                    w.sent.clear();
                }
                return ok(json!({ "version": 1, "features": ["windows", "desktops", "shot", "frames", "snap", "input"] }));
            }
            "windows" => return self.ipc_list(),
            "status" => {
                return ok(json!({ "socket": self.ipc.socket_path.as_ref().map(|p| p.display().to_string()),
                                  "connections": self.ipc.watchers.len(), "desktops": self.desktops_json(),
                                  "frames": crate::frames::with(|l| if l.firehold { "firehold" } else { "hearth" }),
                                  "pictures": self.captures.taken_windows }))
            }
            "frames" => {
                return match crate::frames::apply(v, from) {
                    Ok(changed) => {
                        if changed {
                            info!(look = %v.get("style").and_then(|s| s.as_str()).unwrap_or("firehold"), "Firehold's title bars changed");
                        }
                        ok(json!({ "changed": changed }))
                    }
                    Err(e) => err(&e),
                }
            }
            _ => {}
        }
        if self.lock.locked {
            return err("the screen is locked");
        }
        let window = id_of(v).and_then(|i| self.window_by_id(i));
        let need = |w: &Option<WindowElement>| w.clone().ok_or("no such window");
        let r: Result<Value, &str> = (|| match op.as_str() {
            "activate" => {
                self.activate_window(&need(&window)?);
                Ok(json!({}))
            }
            "close" => {
                self.close_window(&need(&window)?);
                Ok(json!({}))
            }
            "minimize" => {
                let w = need(&window)?;
                if on(v) {
                    self.minimize_window(&w)
                } else {
                    self.activate_window(&w)
                }
                Ok(json!({}))
            }
            "maximize" => {
                let w = need(&window)?;
                if self.minimized.iter().any(|m| m.window == w) {
                    self.activate_window(&w);
                }
                self.set_maximized(&w, on(v));
                Ok(json!({}))
            }
            "fullscreen" => {
                self.set_fullscreen(&need(&window)?, on(v), None);
                Ok(json!({}))
            }
            "snap" => {
                let w = need(&window)?;
                match v.get("zone").and_then(|z| z.as_str()).unwrap_or("") {
                    "left-half" | "left" => {
                        self.unsnap(&w);
                        self.snap_window(&w, Edge::Left, None)
                    }
                    "right-half" | "right" => {
                        self.unsnap(&w);
                        self.snap_window(&w, Edge::Right, None)
                    }
                    "maximize" | "top" | "full" => self.set_maximized(&w, true),
                    _ => return Err("that zone needs KWin (left-half, right-half and maximize work here)"),
                }
                Ok(json!({}))
            }
            "desktop" => {
                let n = num(v, "n").ok_or("no desktop number")?;
                let follow = v.get("follow").and_then(|f| f.as_bool()).unwrap_or(false);
                if self.move_to_desktop(&need(&window)?, n, follow) {
                    Ok(json!({}))
                } else {
                    Err("no such desktop")
                }
            }
            "goDesktop" => {
                if self.go_desktop(num(v, "n").ok_or("no desktop number")?) {
                    Ok(json!({}))
                } else {
                    Err("no such desktop")
                }
            }
            "addDesktop" => {
                let name = v.get("name").and_then(|n| n.as_str()).map(String::from);
                Ok(json!({ "n": self.add_desktop(name) }))
            }
            "removeDesktop" => {
                if self.remove_desktop(num(v, "n").ok_or("no desktop number")?) {
                    Ok(json!({}))
                } else {
                    Err("that desktop can't be removed (the last one stays)")
                }
            }
            "renameDesktop" => {
                let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("");
                if self.rename_desktop(num(v, "n").ok_or("no desktop number")?, name) {
                    Ok(json!({}))
                } else {
                    Err("no such desktop, or no name")
                }
            }
            "key" => {
                let keys: Vec<String> = v.get("keys").and_then(|k| k.as_array()).map(|a| a.iter().filter_map(|k| k.as_str().map(String::from)).collect()).unwrap_or_default();
                let text = v.get("text").and_then(|t| t.as_str()).unwrap_or("");
                let steps = key_steps(&keys, text).map_err(|_| "no keys or text to type")?;
                self.ipc_type(&steps);
                Ok(json!({ "steps": steps.len() }))
            }
            "shot" => {
                let w = need(&window)?;
                let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).ok_or("no runtime folder")?;
                let path = v
                    .get("path")
                    .and_then(|p| p.as_str())
                    .and_then(|p| shot_path_ok(p, &runtime))
                    .ok_or("the picture goes to a .png file in $XDG_RUNTIME_DIR")?;
                let max = num(v, "max").unwrap_or(640).clamp(16, 4096) as i32;
                self.captures.window_shots.push(crate::screencopy::WindowShot { window: w.clone(), id: window_id(&w), path, max, watcher: from });
                Ok(json!({ "queued": true }))
            }
            _ => Err("unknown op"),
        })();
        match r {
            Ok(extra) => ok(extra),
            Err(e) => err(e),
        }
    }

    /// The window list, as Mantel's WinList takes it.
    pub fn ipc_list(&self) -> Value {
        let focused = self.focused_window().map(|w| window_id(&w));
        // Bottom first in the space: top first here.
        let mapped: Vec<WindowElement> = self.space.elements().cloned().collect();
        let mut stack_rank: HashMap<u64, usize> = HashMap::new();
        for (i, w) in mapped.iter().rev().enumerate() {
            stack_rank.insert(window_id(w), i);
        }
        let dh = &self.display_handle;
        let infos = self.toplevel_infos();
        let windows: Vec<Value> = infos
            .iter()
            .map(|i| {
                let w = self.window_by_id(i.id);
                let frame = w.as_ref().and_then(|w| {
                    let geo = SpaceElement::geometry(w);
                    let loc = self
                        .space
                        .element_location(w)
                        .or_else(|| self.minimized.iter().chain(self.elsewhere.iter()).find(|m| &m.window == w).map(|m| m.location))?;
                    Some(json!({ "x": loc.x, "y": loc.y, "w": geo.size.w, "h": geo.size.h }))
                });
                let pid = w
                    .as_ref()
                    .and_then(|w| w.wl_surface().map(|s| s.into_owned()))
                    .and_then(|s| dh.get_client(s.id()).ok())
                    .and_then(|c| c.get_credentials(dh).ok())
                    .map(|c| c.pid)
                    .unwrap_or(0);
                let desk = w.as_ref().and_then(desktop_of).unwrap_or(self.workspaces.active + 1);
                json!({
                    "id": i.id.to_string(), "appId": i.app_id, "title": i.title, "activated": i.activated,
                    "minimized": i.minimized, "maximized": i.maximized, "fullscreen": i.fullscreen, "keepAbove": false,
                    "desktops": [desk], "allDesktops": false,
                    "output": i.output.as_ref().map(|o| o.name()).unwrap_or_default(), "pid": pid,
                    "frame": frame.unwrap_or(json!({})), "ssd": w.as_ref().map(|w| w.decoration_state().is_ssd).unwrap_or(false),
                    "stack": stack_rank.get(&i.id),
                })
            })
            .collect();
        let mut stack: Vec<(usize, u64)> = stack_rank.iter().map(|(id, r)| (*r, *id)).collect();
        stack.sort();
        json!({
            "t": "windows",
            "windows": windows,
            "active": focused.map(|f| f.to_string()).unwrap_or_default(),
            "desktops": self.desktops_json(),
            "stack": stack.into_iter().filter(|(_, id)| infos.iter().any(|i| i.id == *id)).map(|(_, id)| id.to_string()).collect::<Vec<_>>(),
            "frames": crate::frames::with(|l| if l.firehold { "firehold" } else { "hearth" }),
            "outputs": self.space.outputs().map(|o| {
                let g = self.space.output_geometry(o).unwrap_or_default();
                let p = o.physical_properties();
                json!({ "name": o.name(), "model": p.model, "x": g.loc.x, "y": g.loc.y, "w": g.size.w, "h": g.size.h, "mm": [p.size.w, p.size.h] })
            }).collect::<Vec<_>>(),
        })
    }

    /// Typed into whatever has the keyboard, as a real keyboard would (`key`): the compositor's own shortcuts are
    /// skipped (it's typing, not a command to Firehold).
    fn ipc_type(&mut self, steps: &[(u32, bool)]) {
        use smithay::{backend::input::KeyState, input::keyboard::{FilterResult, Keycode}, utils::SERIAL_COUNTER};
        let Some(keyboard) = self.seat.get_keyboard() else { return };
        for (code, down) in steps {
            let time = Duration::from(self.clock.now()).as_millis() as u32;
            let state = if *down { KeyState::Pressed } else { KeyState::Released };
            keyboard.input::<(), _>(self, Keycode::new(code + 8), state, SERIAL_COUNTER.next_serial(), time, |_, _, _| FilterResult::Forward);
        }
    }

    /// After every round (wm.rs's hearth_tick): starts the socket once, sends the list where it changed, and the
    /// window pictures that are done.
    pub fn ipc_tick(&mut self) {
        if !self.ipc.tried {
            self.ipc.tried = true;
            self.ipc_start();
        }
        for (to, reply) in std::mem::take(&mut self.captures.shots_done) {
            self.ipc_send(to, &reply);
        }
        if let Some(kind) = self.ipc.input_pending.take() {
            let to: Vec<u64> = self.ipc.watchers.iter().filter(|w| w.inputs && !w.dead).map(|w| w.id).collect();
            let v = json!({ "t": "input", "kind": kind });
            for id in to {
                self.ipc_send(id, &v);
            }
        }
        if self.ipc.watchers.iter().any(|w| w.watching && !w.dead) {
            let text = self.ipc_list().to_string();
            let due: Vec<u64> = self.ipc.watchers.iter().filter(|w| w.watching && !w.dead && w.sent != text).map(|w| w.id).collect();
            for id in due {
                let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
                self.ipc_send(id, &v);
                if let Some(w) = self.ipc.watchers.iter_mut().find(|w| w.id == id) {
                    w.sent = text.clone();
                }
            }
        }
        let dead: Vec<u64> = self.ipc.watchers.iter().filter(|w| w.dead).map(|w| w.id).collect();
        for id in dead {
            self.ipc_drop(id);
        }
        // Pictures of windows that went meanwhile: answered, not left waiting.
        self.captures.window_shots.retain(|s| s.window.alive());
    }

    pub fn ipc_cleanup(&mut self) {
        if let Some(p) = self.ipc.socket_path.take() {
            let _ = std::fs::remove_file(p);
        }
    }
}

/// `firehold-comp ctl LINE...`: one command to the running compositor's IPC (WAYLAND_DISPLAY's), its answer printed
/// (for tests and people). A `shot` waits for the picture's answer too.
pub fn ctl(args: &[String]) -> i32 {
    let line = args.join(" ");
    let (Some(dir), Some(name)) = (std::env::var_os("XDG_RUNTIME_DIR"), std::env::var("WAYLAND_DISPLAY").ok()) else {
        eprintln!("firehold-comp ctl: XDG_RUNTIME_DIR and WAYLAND_DISPLAY are needed");
        return 2;
    };
    let path = Path::new(&dir).join(format!("firehold-wm-{name}.sock"));
    let mut s = match UnixStream::connect(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("firehold-comp ctl: {}: {e}", path.display());
            return 1;
        }
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    if s.write_all(format!("{line}\n").as_bytes()).is_err() {
        return 1;
    }
    let wants_shot = parse_line(&line).ok().and_then(|v| v.get("op").and_then(|o| o.as_str()).map(|o| o == "shot")).unwrap_or(false);
    let mut buf = Vec::new();
    let mut chunk = [0u8; 65536];
    let mut lines_wanted = if wants_shot { 2 } else { 1 };
    loop {
        match s.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
        while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let l: Vec<u8> = buf.drain(..=pos).collect();
            let text = String::from_utf8_lossy(&l).trim().to_string();
            println!("{text}");
            if wants_shot && text.contains("\"ok\":false") && text.contains("\"t\":\"reply\"") {
                return 1;
            }
            lines_wanted -= 1;
            if lines_wanted == 0 {
                return if text.contains("\"ok\":false") { 1 } else { 0 };
            }
        }
    }
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn keys_and_text_to_type() {
        assert_eq!(key_steps(&["ctrl".into(), "c".into()], "").unwrap(), vec![(29, true), (46, true), (46, false), (29, false)]);
        assert_eq!(key_steps(&[], "Hi!").unwrap(), vec![(42, true), (35, true), (35, false), (42, false), (23, true), (23, false), (42, true), (2, true), (2, false), (42, false)]);
        assert_eq!(char_key('/'), Some((53, false)));
        assert_eq!(char_key('?'), Some((53, true)));
        assert_eq!(char_key('"'), Some((40, true)));
        assert_eq!(char_key('\\'), Some((43, false)));
        assert_eq!(char_key('é'), None);
        assert!(key_steps(&["nope".into()], "").is_err());
        assert!(key_steps(&[], "é").is_err());
        assert_eq!(key_code("f11"), Some(87));
    }

    use super::*;

    #[test]
    fn lines_and_ids() {
        assert_eq!(parse_line("watch\n").unwrap()["op"], "watch");
        assert_eq!(parse_line("{\"op\":\"goDesktop\",\"n\":2}").unwrap()["n"], 2);
        assert!(parse_line("{\"n\":2}").is_err());
        assert!(parse_line("{nope").is_err());
        assert!(parse_line("  ").is_err());
        assert_eq!(id_of(&json!({"id": "12"})), Some(12));
        assert_eq!(id_of(&json!({"id": 7})), Some(7));
        assert_eq!(id_of(&json!({"id": "{uuid}"})), None);
    }

    #[test]
    fn input_kinds_are_told_once_a_second_and_only_when_asked() {
        let mut s = IpcState::default();
        s.note_input("mouse");
        assert_eq!(s.input_pending, None, "nobody asked");
        let (a, _b) = UnixStream::pair().unwrap();
        s.watchers.push(Watcher { id: 1, stream: a, inbuf: Vec::new(), dead: false, watching: true, inputs: true, sent: String::new() });
        s.note_input("mouse");
        assert_eq!(s.input_pending.take(), Some("mouse"));
        s.note_input("mouse");
        assert_eq!(s.input_pending, None, "the same kind within a second");
        s.note_input("keys");
        assert_eq!(s.input_pending, Some("keys"), "another kind at once");
    }

    #[test]
    fn pictures_only_go_to_the_runtime_folder() {
        let run = Path::new("/run/user/1000");
        assert!(shot_path_ok("/run/user/1000/hearth-firehold/pictures/12.png", run).is_some());
        assert!(shot_path_ok("/run/user/1000/../../etc/x.png", run).is_none());
        assert!(shot_path_ok("/home/me/x.png", run).is_none());
        assert!(shot_path_ok("/run/user/1000/x.txt", run).is_none());
        assert!(shot_path_ok("relative.png", run).is_none());
    }
}

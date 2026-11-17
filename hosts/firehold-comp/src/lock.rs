//! The session lock (ext-session-lock-v1): Mantel's lock screen (Lock.qml, Super+L, "Lock" in the power menu, lock
//! when waking) covers every screen, and nothing else is drawn or gets a key, a click or a touch until it unlocks.
//!
//! The two rules (docs/FIREHOLD.md "The lock screen"):
//! - **Never unlocks by itself.** Only the lock client's own "unlock" or the right password in the built-in prompt
//!   below ends it. When the lock client dies (Mantel crashed, was killed), the screens stay covered.
//! - **Never locks anyone out.** Then the compositor shows its own small prompt ("type your password and press
//!   Enter"), checked by the system's own password checker (unix_chkpwd, the helper PAM's pam_unix uses), and it
//!   asks Mantel to lock again (the Super+L command from rc.xml) so its own lock screen takes over when it's back.
//!   A lock client that's alive but never shows anything gets the prompt too, after a few seconds.

use std::{
    io::Write,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

use smithay::{
    backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
    output::Output,
    reexports::{
        wayland_protocols::ext::session_lock::v1::server::ext_session_lock_v1::ExtSessionLockV1,
        wayland_server::{
            protocol::{wl_output::WlOutput, wl_surface::WlSurface},
            Resource,
        },
    },
    utils::{Rectangle, Transform, SERIAL_COUNTER},
    wayland::{
        session_lock::{LockSurface, SessionLockHandler, SessionLockManagerState, SessionLocker},
    },
};
use tracing::{info, warn};

use crate::{
    focus::KeyboardFocusTarget,
    shell::ssd::{draw_text_line, Canvas},
    state::{Backend, HearthState},
    theme,
};

/// How long a lock client that's alive may show nothing before the built-in prompt appears anyway.
const SILENT_CLIENT: Duration = Duration::from_secs(8);
/// After a wrong password, the next try waits this long (as the login screen does).
const RETRY_DELAY: Duration = Duration::from_secs(2);

/// What a screen shows while it's locked (render.rs).
#[derive(Debug, Clone)]
pub enum LockView {
    /// The lock client's surface for this screen.
    Surface(WlSurface),
    /// The built-in prompt, drawn for this screen's size.
    Prompt(MemoryRenderBuffer),
    /// Nothing yet (the lock client's picture is on its way): Mantel's background colour.
    Cover,
}

#[derive(Debug, Default)]
pub struct Prompt {
    pub typed: String,
    pub message: String,
    pub busy: bool,
    pub wait_until: Option<Instant>,
    result: Option<mpsc::Receiver<Result<bool, String>>>,
    /// What the pictures were drawn from (redrawn when it changes), per screen size.
    drawn: Vec<((i32, i32), String, MemoryRenderBuffer)>,
}

#[derive(Debug)]
pub struct LockState {
    pub manager: SessionLockManagerState,
    /// From the first lock request until the lock client unlocks (or the right password in the prompt).
    pub locked: bool,
    /// The lock that was granted: its client's "unlock" is the only one that counts.
    lock_obj: Option<ExtSessionLockV1>,
    /// Waiting for every screen to show the locked picture before telling the client it's locked.
    pending: Option<SessionLocker>,
    rendered: Vec<String>,
    pub since: Option<Instant>,
    pub surfaces: Vec<(Output, LockSurface)>,
    /// The lock client went away while locked: the prompt is up.
    pub client_gone: bool,
    gone_at: Option<Instant>,
    last_relock: Option<Instant>,
    unlock_check: bool,
    pub prompt: Prompt,
    /// How long after the lock client went away the compositor asks Mantel to lock again (FIREHOLD_RELOCK_AFTER, s).
    relock_after: Duration,
}

impl LockState {
    pub fn new<B: Backend + 'static>(dh: &smithay::reexports::wayland_server::DisplayHandle) -> LockState {
        // Only ordinary clients may lock (not sandboxed ones: a Flatpak can't lock the screen).
        let manager = SessionLockManagerState::new::<HearthState<B>, _>(dh, |client| {
            client
                .get_data::<crate::ClientState>()
                .map_or(true, |c| c.security_context.is_none())
        });
        let relock_after = std::env::var("FIREHOLD_RELOCK_AFTER")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(3);
        LockState {
            manager,
            locked: false,
            lock_obj: None,
            pending: None,
            rendered: Vec::new(),
            since: None,
            surfaces: Vec::new(),
            client_gone: false,
            gone_at: None,
            last_relock: None,
            unlock_check: false,
            prompt: Prompt::default(),
            relock_after: Duration::from_secs(relock_after),
        }
    }

    pub fn client_alive(&self) -> bool {
        self.lock_obj.as_ref().is_some_and(|l| l.is_alive())
    }

    fn surface_for(&self, output: &Output) -> Option<&LockSurface> {
        self.surfaces
            .iter()
            .find(|(o, s)| o == output && s.alive())
            .map(|(_, s)| s)
    }

    /// A lock surface for this screen that has shown a picture.
    fn showing(&self, output: &Output) -> Option<WlSurface> {
        let s = self.surface_for(output)?;
        let has_buffer = smithay::backend::renderer::utils::with_renderer_surface_state(s.wl_surface(), |st| {
            st.buffer().is_some()
        })
        .unwrap_or(false);
        has_buffer.then(|| s.wl_surface().clone())
    }

    /// The prompt is what takes the keyboard: no live lock surface anywhere, or the client never showed anything.
    pub fn prompt_active(&self) -> bool {
        if !self.locked {
            return false;
        }
        if self.client_gone || !self.client_alive() {
            return true;
        }
        let silent = self.since.is_some_and(|t| t.elapsed() >= SILENT_CLIENT);
        silent && !self.surfaces.iter().any(|(o, _)| self.showing(o).is_some())
    }

    /// What this screen shows now (None: it isn't locked).
    pub fn view(&mut self, output: &Output) -> Option<LockView> {
        if !self.locked {
            return None;
        }
        if let Some(s) = self.showing(output) {
            if !self.client_gone {
                return Some(LockView::Surface(s));
            }
        }
        if !self.prompt_active() {
            return Some(LockView::Cover);
        }
        let mode = output.current_mode()?;
        let size = output.current_transform().transform_size(mode.size);
        let scale = output.current_scale().integer_scale().max(1);
        Some(LockView::Prompt(self.prompt.picture((size.w, size.h), scale)))
    }
}

impl Prompt {
    fn key(&self) -> String {
        format!("{}|{}|{}", self.typed.chars().count(), self.message, self.busy)
    }

    fn picture(&mut self, size: (i32, i32), scale: i32) -> MemoryRenderBuffer {
        let key = self.key();
        if let Some((_, _, b)) = self.drawn.iter().find(|(s, k, _)| *s == size && *k == key) {
            return b.clone();
        }
        let px = draw_prompt(size.0, size.1, scale, self.typed.chars().count(), &self.message, self.busy);
        let buf = MemoryRenderBuffer::from_slice(
            &px,
            Fourcc::Argb8888,
            size,
            scale,
            Transform::Normal,
            Some(vec![Rectangle::from_size(size.into())]),
        );
        self.drawn.retain(|(s, _, _)| *s != size);
        self.drawn.push((size, key, buf.clone()));
        buf
    }
}

/// The built-in prompt: Mantel's lock look kept simple (its colours, a title, a line saying what to do, a field
/// with a dot per letter typed, and what happened last).
pub fn draw_prompt(w: i32, h: i32, scale: i32, typed: usize, message: &str, busy: bool) -> Vec<u8> {
    let t = theme::current();
    let s = scale.max(1) as f32;
    let mut cv = Canvas::new(w, h, t.bg);
    // A warm glow along the bottom, like Lock.qml's ember glow.
    let glow_top = (h as f32 * 0.55) as i32;
    for y in glow_top..h {
        let a = ((y - glow_top) as f32 / (h - glow_top).max(1) as f32).powf(1.6) * 0.22;
        for x in 0..w {
            cv.blend(x, y, t.accent, a);
        }
    }
    let cx = w as f32 / 2.0;
    let mid = h as f32 / 2.0;
    draw_text_line(&mut cv, "Locked", 64.0 * s, cx, mid - 90.0 * s, t.text);
    draw_text_line(
        &mut cv,
        "The lock screen stopped, so Hearth is keeping your screen locked.",
        20.0 * s,
        cx,
        mid - 40.0 * s,
        t.muted,
    );
    draw_text_line(&mut cv, "Type your password and press Enter.", 20.0 * s, cx, mid - 10.0 * s, t.muted);
    // The field.
    let fw = (520.0 * s).min(w as f32 - 40.0 * s);
    let fh = 56.0 * s;
    let fx = cx - fw / 2.0;
    let fy = mid + 20.0 * s;
    cv.round_rect(fx - s, fy - s, fw + 2.0 * s, fh + 2.0 * s, 14.0 * s, t.line);
    cv.round_rect(fx, fy, fw, fh, 13.0 * s, t.title_active);
    let dots = typed.min(40);
    if dots > 0 {
        let gap = 22.0 * s;
        let start = cx - gap * (dots as f32 - 1.0) / 2.0;
        for i in 0..dots {
            let x = start + gap * i as f32;
            cv.round_rect(x - 6.0 * s, fy + fh / 2.0 - 6.0 * s, 12.0 * s, 12.0 * s, 6.0 * s, t.text);
        }
    } else {
        draw_text_line(&mut cv, "Password", 18.0 * s, cx, fy + fh / 2.0 + 6.0 * s, t.muted);
    }
    let note = if busy { "Checking…" } else { message };
    if !note.is_empty() {
        let c = if busy { t.muted } else { t.danger };
        draw_text_line(&mut cv, note, 18.0 * s, cx, fy + fh + 40.0 * s, c);
    }
    cv.px
}

/// The person this session belongs to (whose password unlocks it).
fn user_name() -> Option<String> {
    for k in ["USER", "LOGNAME"] {
        if let Ok(v) = std::env::var(k) {
            if !v.is_empty() && !v.contains('/') {
                return Some(v);
            }
        }
    }
    let out = Command::new("id").arg("-un").output().ok()?;
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!v.is_empty()).then_some(v)
}

/// The system's own password checker (pam_unix's helper): exactly what the login screen checks.
pub fn chkpwd_path() -> Option<&'static str> {
    [
        "/usr/sbin/unix_chkpwd",
        "/sbin/unix_chkpwd",
        "/usr/bin/unix_chkpwd",
        "/usr/libexec/unix_chkpwd",
    ]
    .into_iter()
    .find(|p| std::path::Path::new(p).exists())
}

/// Checks a password in a thread (it can take a moment); the answer comes back through the channel.
fn check_password(password: String) -> mpsc::Receiver<Result<bool, String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let r = (|| {
            let user = user_name().ok_or("can't tell whose session this is")?;
            let bin = chkpwd_path().ok_or("the system's password checker (unix_chkpwd) isn't installed")?;
            let mut child = Command::new(bin)
                .args([user.as_str(), "nullok"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("couldn't run the password checker: {e}"))?;
            if let Some(mut stdin) = child.stdin.take() {
                let mut bytes = password.into_bytes();
                bytes.push(0);
                let _ = stdin.write_all(&bytes);
                bytes.iter_mut().for_each(|b| *b = 0);
            }
            let status = child.wait().map_err(|e| format!("the password checker failed: {e}"))?;
            Ok(status.success())
        })();
        let _ = tx.send(r);
    });
    rx
}

impl<B: Backend + 'static> SessionLockHandler for HearthState<B> {
    fn lock_state(&mut self) -> &mut SessionLockManagerState {
        &mut self.lock.manager
    }

    fn lock(&mut self, confirmation: SessionLocker) {
        if self.lock.locked && self.lock.client_alive() && !self.lock.client_gone {
            // Already locked by a lock screen that's alive: a second one is turned down ("finished").
            info!("lock: a second lock screen asked to lock; turned down (already locked)");
            return;
        }
        let takeover = self.lock.locked;
        self.lock.lock_obj = Some(confirmation.ext_session_lock().clone());
        self.lock.surfaces.retain(|(_, s)| s.alive());
        self.lock.client_gone = false;
        self.lock.gone_at = None;
        self.lock.unlock_check = false;
        self.lock.prompt = Prompt::default();
        if takeover {
            // The screens are covered already: the new lock screen takes over at once.
            info!("lock: a lock screen took over the lock");
            confirmation.lock();
        } else {
            info!("lock: locking");
            self.lock.locked = true;
            self.lock.since = Some(Instant::now());
            self.lock.rendered.clear();
            self.lock.pending = Some(confirmation);
            self.lock_began();
        }
        self.enforce_lock_focus();
    }

    fn unlock(&mut self) {
        // Smithay calls this for any unlock_and_destroy, even one from a lock it turned down: it only counts when
        // the lock that was granted is the one that's gone (checked once this round of requests is done).
        self.lock.unlock_check = true;
        self.handle.insert_idle(|state| state.finish_unlock_request());
    }

    fn new_surface(&mut self, surface: LockSurface, output: WlOutput) {
        let Some(output) = Output::from_resource(&output) else { return };
        if let Some(mode) = output.current_mode() {
            let size = output
                .current_transform()
                .transform_size(mode.size)
                .to_f64()
                .to_logical(output.current_scale().fractional_scale())
                .to_i32_round::<i32>();
            surface.with_pending_state(|s| s.size = Some((size.w.max(1) as u32, size.h.max(1) as u32).into()));
        }
        self.lock.surfaces.retain(|(o, s)| s.alive() && o != &output);
        self.lock.surfaces.push((output, surface));
        self.enforce_lock_focus();
    }
}
smithay::delegate_session_lock!(@<B: Backend + 'static> HearthState<B>);

impl<B: Backend + 'static> HearthState<B> {
    /// Locking started: every grab ends (a menu's keyboard grab would get the password), nothing keeps the keyboard.
    fn lock_began(&mut self) {
        let serial = SERIAL_COUNTER.next_serial();
        let time = self.clock.now().as_millis();
        let pointer = self.pointer.clone();
        if pointer.is_grabbed() {
            pointer.unset_grab(self, serial, time);
        }
        if let Some(k) = self.seat.get_keyboard() {
            if k.is_grabbed() {
                k.unset_grab(self);
            }
        }
        if let Some(t) = self.seat.get_touch() {
            if t.is_grabbed() {
                t.unset_grab(self);
            }
        }
        self.dnd_icon = None;
        self.write_status_now();
    }

    fn finish_unlock_request(&mut self) {
        if !self.lock.unlock_check {
            return;
        }
        self.lock.unlock_check = false;
        let granted_gone = self.lock.lock_obj.as_ref().is_some_and(|l| !l.is_alive());
        if self.lock.locked && granted_gone {
            info!("lock: unlocked by the lock screen");
            self.unlock_now();
        } else if self.lock.locked {
            warn!("lock: an unlock from a lock that wasn't granted was ignored");
        }
    }

    pub fn unlock_now(&mut self) {
        if let Some(l) = self.lock.lock_obj.take() {
            if l.is_alive() {
                l.finished();
            }
        }
        self.lock.pending = None;
        self.lock.locked = false;
        self.lock.since = None;
        self.lock.surfaces.clear();
        self.lock.client_gone = false;
        self.lock.gone_at = None;
        self.lock.prompt = Prompt::default();
        let outputs: Vec<Output> = self.space.outputs().cloned().collect();
        for o in &outputs {
            self.backend_data.reset_buffers(o);
        }
        self.focus_top_window();
        self.write_status_now();
    }

    /// While locked the keyboard is the lock screen's (the one under the pointer, or any), or nobody's.
    pub fn enforce_lock_focus(&mut self) {
        if !self.lock.locked {
            return;
        }
        let under = self.pointer.current_location();
        let on = self.space.output_under(under).next().cloned();
        let target = on
            .as_ref()
            .and_then(|o| self.lock.surface_for(o))
            .or_else(|| self.lock.surfaces.iter().find(|(_, s)| s.alive()).map(|(_, s)| s))
            .filter(|_| !self.lock.client_gone)
            .map(|s| KeyboardFocusTarget::Lock(s.wl_surface().clone()));
        let Some(k) = self.seat.get_keyboard() else { return };
        if k.current_focus() != target {
            k.set_focus(self, target, SERIAL_COUNTER.next_serial());
        }
    }

    /// The lock surface under a point (pointer and touch go only there while locked).
    pub fn lock_surface_under(
        &self,
        pos: smithay::utils::Point<f64, smithay::utils::Logical>,
    ) -> Option<(crate::focus::PointerFocusTarget, smithay::utils::Point<f64, smithay::utils::Logical>)> {
        if self.lock.client_gone {
            return None;
        }
        let output = self.space.output_under(pos).next()?;
        let geo = self.space.output_geometry(output)?;
        let s = self.lock.surface_for(output)?;
        Some((crate::focus::PointerFocusTarget::from(s.wl_surface()), geo.loc.to_f64()))
    }

    /// A screen showed its locked picture (the backends call this after drawing a locked frame).
    pub fn lock_frame_drawn(&mut self, output: &Output) {
        if self.lock.pending.is_none() {
            return;
        }
        let name = output.name();
        if !self.lock.rendered.contains(&name) {
            self.lock.rendered.push(name);
        }
        let all = self.space.outputs().all(|o| self.lock.rendered.contains(&o.name()));
        let late = self.lock.since.is_some_and(|t| t.elapsed() >= Duration::from_secs(2));
        if all || late {
            if let Some(p) = self.lock.pending.take() {
                info!(all, "lock: the screens are covered; the lock screen is told it's locked");
                p.lock();
            }
        }
    }

    /// The built-in prompt's keys (only while it's up). True when the key was the prompt's.
    pub fn lock_prompt_key(&mut self, sym: smithay::input::keyboard::Keysym, pressed: bool) -> bool {
        use smithay::input::keyboard::Keysym;
        if !self.lock.prompt_active() {
            return false;
        }
        if !pressed {
            return true;
        }
        let p = &mut self.lock.prompt;
        if p.busy || p.wait_until.is_some_and(|t| Instant::now() < t) {
            return true;
        }
        match sym {
            Keysym::Return | Keysym::KP_Enter => {
                let pw = std::mem::take(&mut p.typed);
                p.busy = true;
                p.message.clear();
                p.result = Some(check_password(pw));
            }
            Keysym::BackSpace => {
                p.typed.pop();
            }
            Keysym::Escape => p.typed.clear(),
            _ => {
                let u = xkbcommon::xkb::keysym_to_utf32(sym);
                if let Some(c) = char::from_u32(u).filter(|c| *c != '\0' && !c.is_control()) {
                    if p.typed.len() < 512 {
                        p.typed.push(c);
                    }
                }
            }
        }
        true
    }

    /// Every tick while locked: the client still there? the password's answer? Mantel asked to lock again?
    pub fn lock_tick(&mut self) {
        if !self.lock.locked {
            return;
        }
        if !self.lock.unlock_check && !self.lock.client_gone && !self.lock.client_alive() {
            warn!("lock: the lock screen went away while locked; the screens stay locked (built-in prompt)");
            self.lock.client_gone = true;
            self.lock.gone_at = Some(Instant::now());
            self.lock.surfaces.retain(|(_, s)| s.alive());
            // A lock that was never confirmed can't be now: the screens are covered either way.
            self.lock.pending = None;
            self.write_status_now();
        }
        let answer = self.lock.prompt.result.as_ref().and_then(|rx| rx.try_recv().ok());
        {
            if let Some(r) = answer {
                self.lock.prompt.result = None;
                self.lock.prompt.busy = false;
                match r {
                    Ok(true) => {
                        info!("lock: the right password in the built-in prompt; unlocking");
                        self.unlock_now();
                        return;
                    }
                    Ok(false) => {
                        info!("lock: a wrong password in the built-in prompt");
                        self.lock.prompt.message = "That password isn't right".into();
                        self.lock.prompt.wait_until = Some(Instant::now() + RETRY_DELAY);
                    }
                    Err(e) => {
                        warn!("lock: the password couldn't be checked: {e}");
                        self.lock.prompt.message = format!("Couldn't check the password ({e})");
                    }
                }
            }
        }
        // Mantel back (its supervisor restarts it): ask it to lock again, so its lock screen takes over.
        if self.lock.client_gone {
            let due = self.lock.gone_at.is_some_and(|t| t.elapsed() >= self.lock.relock_after);
            let again = self.lock.last_relock.is_none_or(|t| t.elapsed() >= Duration::from_secs(5));
            if due && again && self.lock.relock_after.as_secs() < 100_000 {
                self.lock.last_relock = Some(Instant::now());
                if let Some(cmd) = self.keymap.lock_command() {
                    info!("lock: asking Mantel to lock again");
                    crate::wm::spawn_shell(&cmd);
                }
            }
        }
        self.enforce_lock_focus();
    }

    /// Frame callbacks for the lock screens (so their clocks keep ticking).
    pub fn lock_send_frames(&self, output: &Output, time: Duration) {
        if !self.lock.locked {
            return;
        }
        if let Some(s) = self.lock.surface_for(output) {
            smithay::desktop::utils::send_frames_surface_tree(
                s.wl_surface(),
                output,
                time,
                Some(Duration::from_secs(1)),
                |_, _| Some(output.clone()),
            );
        }
    }

    /// A screen's size changed: its lock surface is told its new size.
    pub fn lock_output_changed(&mut self, output: &Output) {
        let Some(mode) = output.current_mode() else { return };
        let size = output
            .current_transform()
            .transform_size(mode.size)
            .to_f64()
            .to_logical(output.current_scale().fractional_scale())
            .to_i32_round::<i32>();
        for (o, s) in &self.lock.surfaces {
            if o == output && s.alive() {
                s.with_pending_state(|st| st.size = Some((size.w.max(1) as u32, size.h.max(1) as u32).into()));
                s.send_configure();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prompt_is_drawn_and_not_one_colour() {
        let px = draw_prompt(800, 600, 1, 5, "That password isn't right", false);
        assert_eq!(px.len(), 800 * 600 * 4);
        let first = &px[0..4];
        assert!(px.chunks_exact(4).any(|p| p != first));
        assert!(px.chunks_exact(4).all(|p| p[3] == 255));
    }

    #[test]
    fn a_password_is_never_checked_without_the_checker() {
        // No checker at a made-up path means "couldn't check", never "right".
        assert!(chkpwd_path().is_none_or(|p| std::path::Path::new(p).exists()));
    }
}

//! Firehold's game features, the Guide's rows (docs/FIREHOLD.md, "Games: the Guide's rows"):
//!
//! * **Frame limit**: the fullscreen game's frame callbacks (and its FIFO barrier) are paced by a timer of their own at
//!   the limit, like gamescope's framerate limit. Every other client keeps the screen's pace.
//! * **Resolution scaling** (Quality / Balanced / Performance): a Wayland game is told a smaller fullscreen size and
//!   its client gets a matching scale, so Smithay maps its buffers, input and outputs between the two sizes; the
//!   compositor draws it full screen with a linear filter and a contrast-adaptive sharpening pass (render.rs,
//!   FSR 1 RCAS-style), falling back to plain linear when the shader can't be made.
//! * **Direct to the display**: udev.rs lets the game's own buffer go straight to the primary plane (any format the
//!   plane takes) while it's fullscreen, notes whether it did, and turns VRR (adaptive sync) on for that screen when
//!   the screen can without a modeset. Anything that fails falls back to normal composition, and says so in the log.
//!
//! The choices are per game (by app id) in `~/.config/hearth/firehold-games.json`, which only the helper
//! (`hearth-firehold-game set`, through hearth_settings_io: atomic, .bak, merge) writes. The compositor only reads it
//! (at start, when it changes, and on `reload`), and keeps what it had if the file can't be read.
//!
//! The Guide talks to it through `$XDG_RUNTIME_DIR/firehold-game-<wayland socket>.sock`: one line in, one JSON line out
//! (`status`, `reload`). Nothing here can stop Firehold from starting: a socket that can't be made is logged and
//! skipped.

use std::{
    cell::Cell,
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

use serde_json::{json, Value};
use smithay::{
    desktop::WindowSurface,
    output::Output,
    reexports::{
        calloop::{
            generic::Generic,
            timer::{TimeoutAction, Timer},
            Interest, Mode, PostAction, RegistrationToken,
        },
        wayland_server::{backend::ClientId, Client, Resource},
    },
    utils::{IsAlive, Logical, Size},
    wayland::{compositor::CompositorHandler, fifo::FifoBarrierCachedState},
};
use tracing::{info, warn};

use crate::{
    shell::{FullscreenSurface, WindowElement},
    state::{Backend, HearthState},
};

// ---------------------------------------------------------------------------------------------- the choices

/// The Guide's "Frame limit" row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameLimit {
    #[default]
    Off,
    Fps(u32),
    /// The screen's own rate (the game waits for the screen, which it does anyway with vsync).
    Screen,
}

impl FrameLimit {
    pub fn parse(v: &Value) -> Option<FrameLimit> {
        match v {
            Value::Number(n) => Self::from_number(n.as_f64()?),
            Value::String(s) => match s.trim() {
                "off" | "Off" | "" => Some(FrameLimit::Off),
                "screen" | "Screen" => Some(FrameLimit::Screen),
                t => Self::from_number(t.parse::<f64>().ok()?),
            },
            Value::Null => Some(FrameLimit::Off),
            _ => None,
        }
    }

    fn from_number(n: f64) -> Option<FrameLimit> {
        if n == 0.0 {
            Some(FrameLimit::Off)
        } else if (10.0..=500.0).contains(&n) && n.fract() == 0.0 {
            Some(FrameLimit::Fps(n as u32))
        } else {
            None
        }
    }

    pub fn to_value(self) -> Value {
        match self {
            FrameLimit::Off => json!("off"),
            FrameLimit::Fps(n) => json!(n),
            FrameLimit::Screen => json!("screen"),
        }
    }

    /// The time between two frames the game is allowed, or None when nothing needs pacing: no limit, the screen's
    /// rate, or a limit at or above a real screen's rate (its vblanks already pace the game; a timer next to them
    /// would only beat against them). `screen_paces` is false nested and headless, where clients decide how fast we draw.
    pub fn interval(self, refresh_mhz: i32, screen_paces: bool) -> Option<Duration> {
        match self {
            FrameLimit::Fps(n) if n > 0 => {
                let screen = refresh_mhz as f64 / 1000.0;
                if screen_paces && screen > 0.0 && n as f64 >= screen - 0.5 {
                    None
                } else {
                    Some(Duration::from_secs_f64(1.0 / n as f64))
                }
            }
            _ => None,
        }
    }
}

/// The Guide's "Resolution scaling" row (FSR 1's ratios).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scaling {
    #[default]
    Off,
    Quality,
    Balanced,
    Performance,
}

impl Scaling {
    pub fn parse(v: &Value) -> Option<Scaling> {
        match v {
            Value::Null => Some(Scaling::Off),
            Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                "off" | "native" | "" => Some(Scaling::Off),
                "quality" => Some(Scaling::Quality),
                "balanced" => Some(Scaling::Balanced),
                "performance" => Some(Scaling::Performance),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Scaling::Off => "off",
            Scaling::Quality => "quality",
            Scaling::Balanced => "balanced",
            Scaling::Performance => "performance",
        }
    }

    /// How many times smaller the game draws on each side.
    pub fn divisor(self) -> f64 {
        match self {
            Scaling::Off => 1.0,
            Scaling::Quality => 1.5,
            Scaling::Balanced => 1.7,
            Scaling::Performance => 2.0,
        }
    }

    /// The size the game is told to draw at for a screen of `out` (even numbers, the screen's shape), or None for Off.
    pub fn render_size(self, out: Size<i32, Logical>) -> Option<Size<i32, Logical>> {
        if self == Scaling::Off || out.w < 64 || out.h < 64 {
            return None;
        }
        let even = |v: f64| (((v / 2.0).round() as i32) * 2).max(2);
        let w = even(out.w as f64 / self.divisor());
        let h = even(out.h as f64 * w as f64 / out.w as f64);
        Some(Size::from((w, h)))
    }
}

/// One game's choices. Defaults (no line for the game, or a key missing) are never written anywhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameSettings {
    pub frame_limit: FrameLimit,
    pub scaling: Scaling,
    /// The sharpening pass after upscaling.
    pub sharpen: bool,
    /// Direct to the display: the game's buffer on the primary plane when it can be.
    pub direct: bool,
    /// VRR / FreeSync while it's fullscreen, on screens that have it.
    pub vrr: bool,
    /// The Guide's "Allow tearing": the game doesn't wait for the screen. Its frame callbacks (and FIFO barrier) come
    /// as soon as it has handed in a frame, so it draws as fast as it can and the newest frame is shown at each
    /// refresh (the lowest delay Smithay 0.7 allows: it can't flip mid-scan yet, so there's no visible tear line).
    /// A frame limit, when one is set, wins.
    pub tearing: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings { frame_limit: FrameLimit::Off, scaling: Scaling::Off, sharpen: true, direct: true, vrr: true, tearing: false }
    }
}

impl GameSettings {
    /// One game's object; keys that are missing or not understood keep their defaults (and are named in `bad`).
    pub fn from_value(v: &Value, bad: &mut Vec<String>) -> GameSettings {
        let mut s = GameSettings::default();
        let Some(o) = v.as_object() else {
            bad.push("not an object".into());
            return s;
        };
        if let Some(x) = o.get("frameLimit") {
            match FrameLimit::parse(x) {
                Some(f) => s.frame_limit = f,
                None => bad.push(format!("frameLimit {x}")),
            }
        }
        if let Some(x) = o.get("scaling") {
            match Scaling::parse(x) {
                Some(f) => s.scaling = f,
                None => bad.push(format!("scaling {x}")),
            }
        }
        for (key, slot) in [("sharpen", &mut s.sharpen), ("direct", &mut s.direct), ("vrr", &mut s.vrr), ("tearing", &mut s.tearing)] {
            if let Some(x) = o.get(key) {
                match x.as_bool() {
                    Some(b) => *slot = b,
                    None => bad.push(format!("{key} {x}")),
                }
            }
        }
        s
    }
}

/// The settings file's games, or why it couldn't be read. A missing file is no games (all defaults).
pub fn parse_games(text: &str) -> Result<(HashMap<String, GameSettings>, Vec<String>), String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let o = v.as_object().ok_or("not a JSON object")?;
    let mut games = HashMap::new();
    let mut bad = Vec::new();
    if let Some(g) = o.get("games") {
        let g = g.as_object().ok_or("\"games\" isn't an object")?;
        for (app, s) in g {
            let mut b = Vec::new();
            games.insert(app.clone(), GameSettings::from_value(s, &mut b));
            bad.extend(b.into_iter().map(|x| format!("{app}: {x}")));
        }
    }
    Ok((games, bad))
}

// ---------------------------------------------------------------------------------------------- the decisions

/// What the screen's planes may do this frame (udev.rs turns it into Smithay's FrameFlags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScanoutChoice {
    /// As always (Smithay's defaults: a buffer of the swapchain's format may go on the primary plane).
    #[default]
    Normal,
    /// A fullscreen game with Direct on: its buffer may go on the primary plane in any format the plane takes.
    PreferGame,
    /// Direct turned off for this game: always composited (only the pointer keeps its plane).
    Never,
}

/// Direct scanout for a screen: the game must be fullscreen on it, Direct on, and nothing that needs drawing by the
/// GPU in between (the sharpening pass). The planes' own test (DRM's TEST_ONLY commit) still decides; when it says
/// no, Smithay composites as usual.
pub fn scanout_choice(game_fullscreen: bool, s: &GameSettings, sharpen_active: bool, covered: bool) -> ScanoutChoice {
    if !game_fullscreen {
        ScanoutChoice::Normal
    } else if !s.direct {
        ScanoutChoice::Never
    } else if sharpen_active || covered {
        // The sharpening pass, or something of Mantel's over the game (the Guide, the Couch): drawn by the GPU.
        ScanoutChoice::Normal
    } else {
        ScanoutChoice::PreferGame
    }
}

/// Which layer-shell surfaces are drawn over a fullscreen game (and get the pointer there): every Overlay surface
/// (the Guide, notices, the performance overlay) and Top surfaces that don't reserve room on the screen (the
/// Couch, Everything open). Panels that do (Mantel's bar and dock) stay under the game, as on labwc and KWin.
pub fn drawn_over_fullscreen(
    layer: smithay::wayland::shell::wlr_layer::Layer,
    zone: smithay::wayland::shell::wlr_layer::ExclusiveZone,
) -> bool {
    use smithay::wayland::shell::wlr_layer::{ExclusiveZone, Layer};
    match layer {
        Layer::Overlay => true,
        Layer::Top => !matches!(zone, ExclusiveZone::Exclusive(n) if n > 0),
        _ => false,
    }
}

/// Whether a layer surface has something to show (Quickshell unmaps a hidden window by taking its buffer away).
pub fn layer_mapped(l: &smithay::desktop::LayerSurface) -> bool {
    smithay::backend::renderer::utils::with_renderer_surface_state(l.wl_surface(), |s| s.buffer().is_some())
        .unwrap_or(false)
}

/// The layer surface over a fullscreen game under a point (screen-local), topmost first.
pub fn layer_over_fullscreen(
    map: &smithay::desktop::LayerMap,
    pos: smithay::utils::Point<f64, Logical>,
) -> Option<smithay::desktop::LayerSurface> {
    use smithay::wayland::shell::wlr_layer::Layer;
    map.layer_under(Layer::Overlay, pos)
        .or_else(|| map.layer_under(Layer::Top, pos).filter(|l| drawn_over_fullscreen(l.layer(), l.cached_state().exclusive_zone)))
        .cloned()
}

/// What a screen says about adaptive sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VrrCap {
    #[default]
    Unknown,
    No,
    /// It has it, but switching needs a modeset (a black flash): Firehold doesn't do that by itself.
    NeedsModeset,
    Yes,
}

impl VrrCap {
    pub fn name(self) -> &'static str {
        match self {
            VrrCap::Unknown => "unknown",
            VrrCap::No => "no",
            VrrCap::NeedsModeset => "needs-modeset",
            VrrCap::Yes => "yes",
        }
    }
}

/// VRR on for a screen: a fullscreen game on it with VRR on, and a screen that can switch without a modeset.
pub fn vrr_wanted(game_fullscreen: bool, s: &GameSettings, cap: VrrCap) -> bool {
    game_fullscreen && s.vrr && cap == VrrCap::Yes
}

/// What render.rs and udev.rs read for each screen, and what they found (kept in the output's user data).
#[derive(Debug, Default)]
pub struct GameOutput {
    pub game: Cell<bool>,
    pub scanout: Cell<ScanoutChoice>,
    pub vrr: Cell<bool>,
    /// Some(strength) while the game on it is upscaled with sharpening.
    pub sharpen: Cell<Option<f32>>,
    /// Whether the last frame put the game's own buffer on the primary plane (None: not known on this backend).
    pub direct: Cell<Option<bool>>,
    pub vrr_cap: Cell<VrrCap>,
    pub vrr_active: Cell<bool>,
}

pub fn output_state(o: &Output) -> &GameOutput {
    o.user_data().insert_if_missing(GameOutput::default);
    o.user_data().get::<GameOutput>().unwrap()
}

/// The sharpening strength for a screen's fullscreen game, when it's upscaled with sharpening.
pub fn sharpen_for(o: &Output) -> Option<f32> {
    o.user_data().get::<GameOutput>().and_then(|g| g.sharpen.get())
}

/// After a frame on a real screen: whether the game's buffer went straight to the display. Logged when it changes.
pub fn note_scanout(o: &Output, direct: bool) {
    let g = output_state(o);
    if g.game.get() && g.direct.get() != Some(direct) {
        if direct {
            info!(output = %o.name(), "Firehold game: drawn straight to the display (direct scanout on the primary plane)");
        } else {
            info!(output = %o.name(), choice = ?g.scanout.get(), "Firehold game: composited (no direct scanout this frame)");
        }
    }
    g.direct.set(if g.game.get() { Some(direct) } else { None });
}

// ---------------------------------------------------------------------------------------------- the state

#[derive(Debug, Clone)]
pub struct Active {
    pub window: WindowElement,
    pub output: Output,
    pub app_id: String,
    pub title: String,
    pub settings: GameSettings,
    pub interval: Option<Duration>,
    pub output_size: Size<i32, Logical>,
    /// The size the game was told to draw at (client coordinates), when it's scaled.
    pub render_size: Option<Size<i32, Logical>>,
    pub scaling_note: Option<String>,
    pub x11: bool,
    pub since: Instant,
}

#[derive(Debug)]
pub struct GameState {
    pub games: HashMap<String, GameSettings>,
    pub file: Option<PathBuf>,
    file_stamp: Option<SystemTime>,
    pub file_error: Option<String>,
    last_file_check: Instant,
    pub active: Option<Active>,
    limiter: Option<RegistrationToken>,
    pub paced: u64,
    /// Frames the game was let draw without waiting (Allow tearing).
    pub unpaced: u64,
    paced_window: (Instant, u64, f64),
    /// Every frame the game in front handed in (its window's commits), paced or not: the overlay's fps without a
    /// frame limit. Counted in shell/mod.rs's commit; the rate is worked out only when the status is asked for.
    pub frames: u64,
    frame_rate: std::cell::Cell<(Instant, u64, f64)>,
    last_enforce: Instant,
    dirty: bool,
    /// The Overlay surface given the keyboard over the game (game_overlays).
    overlay_focus: Option<smithay::desktop::LayerSurface>,
    control_tried: bool,
    listener: Option<UnixListener>,
    pub socket_path: Option<PathBuf>,
}

pub fn settings_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FIREHOLD_GAMES") {
        return Some(p.into());
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".config")))?;
    Some(base.join("hearth").join("firehold-games.json"))
}

impl GameState {
    pub fn new() -> GameState {
        let mut g = GameState {
            games: HashMap::new(),
            file: settings_path(),
            file_stamp: None,
            file_error: None,
            last_file_check: Instant::now(),
            active: None,
            limiter: None,
            paced: 0,
            unpaced: 0,
            paced_window: (Instant::now(), 0, 0.0),
            frames: 0,
            frame_rate: std::cell::Cell::new((Instant::now(), 0, 0.0)),
            last_enforce: Instant::now(),
            dirty: false,
            control_tried: false,
            overlay_focus: None,
            listener: None,
            socket_path: None,
        };
        g.read_file(true);
        g
    }

    /// Reads the per-game file when it changed (or always with `force`). A file that can't be read leaves the games
    /// as they were: the helper repairs it from its .bak; the compositor never writes it.
    fn read_file(&mut self, force: bool) -> bool {
        let Some(path) = self.file.clone() else { return false };
        let stamp = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if !force && stamp == self.file_stamp {
            return false;
        }
        self.file_stamp = stamp;
        if stamp.is_none() {
            let had = !self.games.is_empty();
            self.games.clear();
            self.file_error = None;
            return had;
        }
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| parse_games(&t)) {
            Ok((games, bad)) => {
                if !bad.is_empty() {
                    warn!(file = %path.display(), ?bad, "Firehold games: some choices weren't understood (their defaults are used)");
                }
                info!(file = %path.display(), games = games.len(), "Firehold games: per-game choices read");
                let changed = games != self.games;
                self.games = games;
                self.file_error = None;
                changed
            }
            Err(e) => {
                warn!(file = %path.display(), err = %e, "Firehold games: the per-game file can't be read; keeping what was read before");
                self.file_error = Some(e);
                false
            }
        }
    }

    pub fn limited_window(&self) -> Option<&WindowElement> {
        self.active.as_ref().filter(|a| a.interval.is_some() && self.limiter.is_some()).map(|a| &a.window)
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

/// Still fullscreen (what it was last told, so a window that was just made fullscreen counts).
fn still_fullscreen(w: &WindowElement) -> bool {
    if !w.alive() {
        return false;
    }
    match w.0.underlying_surface() {
        WindowSurface::Wayland(t) => t.with_pending_state(|s| {
            s.states.contains(smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Fullscreen)
        }),
        #[cfg(feature = "xwayland")]
        WindowSurface::X11(x) => x.is_fullscreen(),
    }
}

fn is_x11(w: &WindowElement) -> bool {
    match w.0.underlying_surface() {
        WindowSurface::Wayland(_) => false,
        #[cfg(feature = "xwayland")]
        WindowSurface::X11(_) => true,
    }
}

fn client_of(w: &WindowElement, dh: &smithay::reexports::wayland_server::DisplayHandle) -> Option<Client> {
    let s = w.wl_surface()?;
    dh.get_client(s.id()).ok()
}

impl<B: Backend> HearthState<B> {
    /// Counted from shell/mod.rs's commit (whose impl has no 'static): a frame of the game in front.
    pub fn game_frame(&mut self, window: &WindowElement) {
        if self.game.active.as_ref().is_some_and(|a| &a.window == window) {
            self.game.frames += 1;
        }
    }
}

impl<B: Backend + 'static> HearthState<B> {
    /// The control socket (called once from init; a failure only means the Guide can't change anything).
    pub fn game_start_control(&mut self) {
        let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") else { return };
        let name = self.socket_name.clone().unwrap_or_else(|| "wayland".into());
        let path = Path::new(&dir).join(format!("firehold-game-{name}.sock"));
        // Our own Wayland socket's name is ours alone (Smithay holds its lock), so a file here is a leftover.
        let _ = std::fs::remove_file(&path);
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                warn!(path = %path.display(), err = %e, "Firehold games: no control socket (the Guide's rows can't reach the compositor)");
                return;
            }
        };
        let _ = listener.set_nonblocking(true);
        let watched = match listener.try_clone() {
            Ok(l) => l,
            Err(e) => {
                warn!(err = %e, "Firehold games: no control socket");
                return;
            }
        };
        let res = self.handle.insert_source(Generic::new(watched, Interest::READ, Mode::Level), |_, _, state| {
            state.game_accept();
            Ok(PostAction::Continue)
        });
        if let Err(e) = res {
            warn!(err = %e, "Firehold games: the control socket couldn't be watched");
            let _ = std::fs::remove_file(&path);
            return;
        }
        info!(path = %path.display(), "Firehold games: control socket ready");
        self.game.listener = Some(listener);
        self.game.socket_path = Some(path);
    }

    fn game_accept(&mut self) {
        for _ in 0..8 {
            let stream = match self.game.listener.as_ref().map(|l| l.accept()) {
                Some(Ok((s, _))) => s,
                _ => return,
            };
            self.game_serve(stream);
        }
    }

    fn game_serve(&mut self, stream: UnixStream) {
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(200)));
        let mut line = String::new();
        let mut reader = BufReader::new(&stream);
        if reader.read_line(&mut line).is_err() {
            return;
        }
        let cmd = line.trim();
        let reply = match cmd {
            "status" | "" => self.game_status(),
            "reload" => {
                self.game.read_file(true);
                self.game.dirty = true;
                self.game_tick();
                self.game_status()
            }
            other => json!({ "ok": false, "error": format!("unknown command {other:?} (status, reload)") }),
        };
        let mut s = &stream;
        let _ = s.write_all(format!("{reply}\n").as_bytes());
    }

    /// After every round of client requests (hearth_tick): finds the fullscreen game and applies its choices when
    /// it, its screen or its choices changed.
    pub fn game_tick(&mut self) {
        if !self.game.control_tried {
            self.game.control_tried = true;
            self.game_start_control();
        }
        if self.game.last_file_check.elapsed() >= Duration::from_secs(2) {
            self.game.last_file_check = Instant::now();
            if self.game.read_file(false) {
                self.game.dirty = true;
            }
        }
        let found = self.space.outputs().find_map(|o| {
            o.user_data()
                .get::<FullscreenSurface>()
                .and_then(|f| f.get())
                .map(|w| (w, o.clone()))
        });
        let size_of = |s: &Self, o: &Output| s.space.output_geometry(o).map(|g| g.size).unwrap_or_default();
        let same = match (&self.game.active, &found) {
            (Some(a), Some((w, o))) => &a.window == w && &a.output == o && a.output_size == size_of(self, o),
            (None, None) => true,
            _ => false,
        };
        if !same || self.game.dirty {
            self.game.dirty = false;
            self.game_apply(found);
        } else {
            self.game_enforce_size();
        }
        self.game_overlays();
        self.game_unpaced_tick();
    }

    /// Whether the game runs without waiting for the screen right now (Allow tearing, no frame limit).
    pub fn game_unpaced(&self) -> bool {
        self.game.active.as_ref().is_some_and(|a| a.settings.tearing && a.interval.is_none() && a.window.alive())
    }

    /// Allow tearing: the game's frame callbacks and FIFO barrier as soon as it has handed in a frame (every round
    /// of client requests), not at the screen's next refresh.
    fn game_unpaced_tick(&mut self) {
        if !self.game_unpaced() {
            return;
        }
        let Some(a) = self.game.active.as_ref() else { return };
        let (window, output) = (a.window.clone(), a.output.clone());
        let time: Duration = self.clock.now().into();
        #[allow(clippy::mutable_key_type)]
        let mut clients: HashMap<ClientId, Client> = HashMap::new();
        let mut fresh = false;
        window.with_surfaces(|surface, states| {
            if let Some(b) = states.cached_state.get::<FifoBarrierCachedState>().current().barrier.take() {
                b.signal();
                if let Some(c) = surface.client() {
                    clients.insert(c.id(), c);
                }
            }
            if !states.cached_state.get::<smithay::wayland::compositor::SurfaceAttributes>().current().frame_callbacks.is_empty() {
                fresh = true;
            }
        });
        if fresh {
            window.send_frame(&output, time, Some(Duration::ZERO), |_, _| None);
            self.game.unpaced += 1;
        }
        let dh = self.display_handle.clone();
        for client in clients.into_values() {
            self.client_compositor_state(&client).blocker_cleared(self, &dh);
        }
    }

    /// Mantel's surfaces over the game (the Guide, the Couch): direct scanout waits while one is shown, and an
    /// Overlay surface that takes the keyboard gets it when it opens (the Guide is opened with the pad, not a
    /// click); the game gets it back when it closes.
    fn game_overlays(&mut self) {
        use smithay::wayland::shell::wlr_layer::{KeyboardInteractivity, Layer};
        if self.game.active.is_none() {
            self.game.overlay_focus = None;
            return;
        }
        let a = self.game.active.as_ref().unwrap();
        let (window, output, settings) = (a.window.clone(), a.output.clone(), a.settings);
        let (covered, wants_keys) = {
            let map = smithay::desktop::layer_map_for_output(&output);
            let covered = map
                .layers()
                .any(|l| drawn_over_fullscreen(l.layer(), l.cached_state().exclusive_zone) && layer_mapped(l));
            let wants = map
                .layers()
                .rev()
                .find(|l| {
                    l.layer() == Layer::Overlay
                        && l.cached_state().keyboard_interactivity != KeyboardInteractivity::None
                        && layer_mapped(l)
                })
                .cloned();
            (covered, wants)
        };
        let g = output_state(&output);
        let choice = scanout_choice(true, &settings, g.sharpen.get().is_some(), covered);
        if g.scanout.get() != choice {
            g.scanout.set(choice);
        }
        if self.lock.locked {
            return;
        }
        let Some(keyboard) = self.seat.get_keyboard() else { return };
        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
        match wants_keys {
            Some(l) if self.game.overlay_focus.as_ref() != Some(&l) => {
                let on_layer = matches!(keyboard.current_focus(), Some(crate::focus::KeyboardFocusTarget::LayerSurface(_)));
                if !on_layer {
                    keyboard.set_focus(self, Some(l.clone().into()), serial);
                }
                self.game.overlay_focus = Some(l);
            }
            Some(_) => {}
            None => {
                if self.game.overlay_focus.take().is_some() {
                    let gone = match keyboard.current_focus() {
                        Some(crate::focus::KeyboardFocusTarget::LayerSurface(l)) => !layer_mapped(&l),
                        None => true,
                        _ => false,
                    };
                    if gone && window.alive() {
                        keyboard.set_focus(self, Some(window.into()), serial);
                    }
                }
            }
        }
    }

    /// A fullscreen window that's gone or no longer fullscreen (a client that left fullscreen without a screen of
    /// its own, or ended) stops covering the screen: the normal stack (wallpaper, bar, dock, windows) is drawn again.
    pub fn fullscreen_sweep(&mut self) {
        let outputs: Vec<Output> = self.space.outputs().cloned().collect();
        for o in outputs {
            let Some(fs) = o.user_data().get::<FullscreenSurface>() else { continue };
            let stale = fs.peek().is_some_and(|w| !still_fullscreen(&w));
            if stale {
                fs.clear();
                self.backend_data.reset_buffers(&o);
                info!(output = %o.name(), "Firehold: the fullscreen window left; the desktop is drawn again");
            }
        }
    }

    /// The scaled game keeps its smaller size even when something else (its own fullscreen request, a new
    /// configure) asked for the screen's size.
    fn game_enforce_size(&mut self) {
        if self.game.last_enforce.elapsed() < Duration::from_millis(250) {
            return;
        }
        self.game.last_enforce = Instant::now();
        let Some(a) = self.game.active.as_ref() else { return };
        let Some(rs) = a.render_size else { return };
        if let WindowSurface::Wayland(t) = a.window.0.underlying_surface() {
            if t.with_pending_state(|s| s.size) != Some(rs) {
                t.with_pending_state(|s| s.size = Some(rs));
                t.send_pending_configure();
            }
        }
    }

    /// The size a scaled game is told when it asks for full screen again (shell/xdg.rs).
    pub fn game_configure_size(
        &self,
        surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) -> Option<Size<i32, Logical>> {
        let a = self.game.active.as_ref()?;
        let rs = a.render_size?;
        (a.window.wl_surface().as_deref() == Some(surface)).then_some(rs)
    }

    fn game_undo(&mut self, old: &Active) {
        for o in self.space.outputs() {
            let g = output_state(o);
            g.game.set(false);
            g.scanout.set(ScanoutChoice::Normal);
            g.vrr.set(false);
            g.sharpen.set(None);
            g.direct.set(None);
        }
        if old.render_size.is_some() && old.window.alive() {
            if let Some(client) = client_of(&old.window, &self.display_handle) {
                self.client_compositor_state(&client).set_client_scale(1.0);
            }
            if let WindowSurface::Wayland(t) = old.window.0.underlying_surface() {
                let full = t.with_pending_state(|s| s.states.contains(
                    smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Fullscreen,
                ));
                if full {
                    let size = self.space.output_geometry(&old.output).map(|g| g.size);
                    t.with_pending_state(|s| s.size = size);
                }
                t.send_pending_configure();
            }
            info!(app = %old.app_id, "Firehold game: back to the screen's own size");
        }
    }

    fn game_stop_limiter(&mut self) {
        if let Some(token) = self.game.limiter.take() {
            self.handle.remove(token);
        }
    }

    fn game_apply(&mut self, found: Option<(WindowElement, Output)>) {
        self.game_stop_limiter();
        if let Some(old) = self.game.active.take() {
            self.game_undo(&old);
        } else {
            for o in self.space.outputs() {
                let g = output_state(o);
                g.game.set(false);
                g.scanout.set(ScanoutChoice::Normal);
                g.vrr.set(false);
                g.sharpen.set(None);
            }
        }
        let Some((window, output)) = found else {
            info!("Firehold game: no fullscreen game");
            return;
        };
        let (title, app_id) = crate::wm::title_and_app_id(&window);
        let settings = self.game.games.get(&app_id).copied().unwrap_or_default();
        let refresh = output.current_mode().map(|m| m.refresh).unwrap_or(60_000);
        let interval = settings.frame_limit.interval(refresh, B::SCREEN_PACES);
        let output_size = self.space.output_geometry(&output).map(|g| g.size).unwrap_or_default();
        let x11 = is_x11(&window);

        // Resolution scaling: the game's client draws smaller; Smithay maps sizes and input with the client's scale.
        let mut render_size = None;
        let mut scaling_note = None;
        if settings.scaling != Scaling::Off {
            if x11 {
                scaling_note = Some("X11 games aren't scaled yet: this one draws at the screen's size".to_string());
            } else if let Some(rs) = settings.scaling.render_size(output_size) {
                match (client_of(&window, &self.display_handle), window.0.underlying_surface()) {
                    (Some(client), WindowSurface::Wayland(t)) => {
                        let s = rs.w as f64 / output_size.w as f64;
                        self.client_compositor_state(&client).set_client_scale(s);
                        t.with_pending_state(|st| st.size = Some(rs));
                        t.send_pending_configure();
                        render_size = Some(rs);
                    }
                    _ => scaling_note = Some("the game's window has gone".into()),
                }
            }
        }
        let sharpen_active = render_size.is_some() && settings.sharpen;
        let g = output_state(&output);
        g.game.set(true);
        g.scanout.set(scanout_choice(true, &settings, sharpen_active, false));
        g.vrr.set(settings.vrr);
        g.sharpen.set(sharpen_active.then_some(0.5));
        g.direct.set(None);

        info!(
            app = %app_id,
            output = %output.name(),
            limit = ?settings.frame_limit,
            paced = ?interval,
            scaling = settings.scaling.name(),
            render = ?render_size,
            direct = settings.direct,
            vrr = settings.vrr,
            tearing = settings.tearing,
            "Firehold game: fullscreen game's choices applied"
        );
        if let Some(n) = &scaling_note {
            warn!(app = %app_id, "Firehold game: {n}");
        }
        self.game.active = Some(Active {
            window,
            output,
            app_id,
            title,
            settings,
            interval,
            output_size,
            render_size,
            scaling_note,
            x11,
            since: Instant::now(),
        });
        self.game.paced = 0;
        self.game.paced_window = (Instant::now(), 0, 0.0);
        self.game.frames = 0;
        self.game.frame_rate.set((Instant::now(), 0, 0.0));
        if let Some(iv) = interval {
            self.game_start_limiter(iv);
        }
    }

    fn game_start_limiter(&mut self, interval: Duration) {
        let res = self.handle.insert_source(Timer::from_duration(interval), move |deadline, _, state| {
            if !state.game_limiter_tick() {
                state.game.limiter = None;
                return TimeoutAction::Drop;
            }
            let next = deadline + interval;
            let now = Instant::now();
            // Late (a long frame elsewhere): start again from now rather than sending a burst to catch up.
            TimeoutAction::ToInstant(if next + interval < now { now } else { next })
        });
        match res {
            Ok(t) => self.game.limiter = Some(t),
            Err(e) => warn!(err = %e, "Firehold game: the frame limit's timer couldn't start (the game runs unlimited)"),
        }
    }

    /// One paced frame: the game's frame callbacks and its FIFO barrier, nothing else.
    fn game_limiter_tick(&mut self) -> bool {
        let Some(a) = self.game.active.as_ref() else { return false };
        if a.interval.is_none() || !a.window.alive() {
            return false;
        }
        let window = a.window.clone();
        let output = a.output.clone();
        let time: Duration = self.clock.now().into();
        #[allow(clippy::mutable_key_type)]
        let mut clients: HashMap<ClientId, Client> = HashMap::new();
        window.with_surfaces(|surface, states| {
            let barrier = states.cached_state.get::<FifoBarrierCachedState>().current().barrier.take();
            if let Some(b) = barrier {
                b.signal();
                if let Some(c) = surface.client() {
                    clients.insert(c.id(), c);
                }
            }
        });
        window.send_frame(&output, time, Some(Duration::ZERO), |_, _| None);
        self.game.paced += 1;
        let (start, n0, _) = self.game.paced_window;
        let el = start.elapsed().as_secs_f64();
        if el >= 1.0 {
            self.game.paced_window = (Instant::now(), self.game.paced, (self.game.paced - n0) as f64 / el);
        }
        let dh = self.display_handle.clone();
        for client in clients.into_values() {
            self.client_compositor_state(&client).blocker_cleared(self, &dh);
        }
        true
    }

    /// The game's frames a second since the status was last asked for (the overlay asks once a second); nothing is
    /// timed while nobody asks.
    fn game_fps(&self) -> f64 {
        let (t, n0, last) = self.game.frame_rate.get();
        let el = t.elapsed().as_secs_f64();
        if el < 0.5 {
            return last;
        }
        let n = self.game.frames.saturating_sub(n0);
        // Asked again after a long while: that's an average over too long a time to call it the frame rate now.
        let f = if el > 5.0 { 0.0 } else { n as f64 / el };
        self.game.frame_rate.set((Instant::now(), self.game.frames, f));
        f
    }

    /// What the Guide shows (and the tests read): the game, its choices, and what actually happened.
    pub fn game_status(&self) -> Value {
        let fps = self.game_fps();
        let game = self.game.active.as_ref().map(|a| {
            let g = output_state(&a.output);
            let scale = a.output.current_scale().fractional_scale();
            let phys = |s: Size<i32, Logical>| json!([(s.w as f64 * scale).round() as i32, (s.h as f64 * scale).round() as i32]);
            let out_phys = a.output.current_mode().map(|m| json!([m.size.w, m.size.h])).unwrap_or(phys(a.output_size));
            json!({
                "appId": a.app_id,
                "title": a.title,
                "x11": a.x11,
                "output": a.output.name(),
                "outputSize": out_phys.clone(),
                "refresh": a.output.current_mode().map(|m| m.refresh as f64 / 1000.0).unwrap_or(0.0),
                "seconds": a.since.elapsed().as_secs(),
                "frameLimit": a.settings.frame_limit.to_value(),
                "frameLimitActive": a.interval.is_some() && self.game.limiter.is_some(),
                "pacedFps": (self.game.paced_window.2 * 10.0).round() / 10.0,
                "fps": (fps * 10.0).round() / 10.0,
                "frames": self.game.frames,
                "scaling": a.settings.scaling.name(),
                "renderSize": a.render_size.map(phys).unwrap_or(out_phys.clone()),
                "configureSize": a.render_size.map(|s| json!([s.w, s.h])),
                "scalingNote": a.scaling_note,
                "sharpen": a.settings.sharpen,
                "sharpenActive": g.sharpen.get().is_some(),
                "direct": a.settings.direct,
                "scanout": match g.direct.get() { Some(true) => "direct", Some(false) => "composited", None => "unknown" },
                "scanoutChoice": format!("{:?}", g.scanout.get()),
                "vrr": a.settings.vrr,
                "vrrActive": g.vrr_active.get(),
                "vrrScreen": if B::NESTED { "nested" } else { g.vrr_cap.get().name() },
                "tearing": a.settings.tearing,
                "tearingActive": a.settings.tearing && a.interval.is_none(),
                "tearingAsked": a.window.wl_surface().is_some_and(|s| crate::tearing::wants_tearing(&s)),
                "unpacedFrames": self.game.unpaced,
            })
        });
        json!({
            "ok": true,
            "backend": B::KIND,
            "game": game,
            "settingsFile": self.game.file.as_ref().map(|p| p.display().to_string()),
            "settingsError": self.game.file_error,
            "socket": self.game.socket_path.as_ref().map(|p| p.display().to_string()),
        })
    }

    pub fn game_cleanup(&mut self) {
        if let Some(p) = self.game.socket_path.take() {
            let _ = std::fs::remove_file(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_limit_values() {
        assert_eq!(FrameLimit::parse(&json!(60)), Some(FrameLimit::Fps(60)));
        assert_eq!(FrameLimit::parse(&json!("40")), Some(FrameLimit::Fps(40)));
        assert_eq!(FrameLimit::parse(&json!("off")), Some(FrameLimit::Off));
        assert_eq!(FrameLimit::parse(&json!(0)), Some(FrameLimit::Off));
        assert_eq!(FrameLimit::parse(&json!("screen")), Some(FrameLimit::Screen));
        assert_eq!(FrameLimit::parse(&json!(3)), None);
        assert_eq!(FrameLimit::parse(&json!(59.5)), None);
        assert_eq!(FrameLimit::parse(&json!(true)), None);
    }

    #[test]
    fn frame_limit_pacing() {
        // 30 on a 60 Hz screen: paced every 33.3 ms.
        let iv = FrameLimit::Fps(30).interval(60_000, true).unwrap();
        assert!((iv.as_secs_f64() - 1.0 / 30.0).abs() < 1e-9);
        // 60 or more on a 60 Hz screen: the screen paces it already.
        assert_eq!(FrameLimit::Fps(60).interval(60_000, true), None);
        assert_eq!(FrameLimit::Fps(120).interval(60_000, true), None);
        // 60 on a 120 Hz screen: paced.
        assert!(FrameLimit::Fps(60).interval(120_000, true).is_some());
        // Nested nothing paces us: 60 is a real limit.
        assert!(FrameLimit::Fps(60).interval(60_000, false).is_some());
        assert_eq!(FrameLimit::Off.interval(60_000, false), None);
        assert_eq!(FrameLimit::Screen.interval(60_000, false), None);
    }

    #[test]
    fn scaling_sizes() {
        let out = Size::from((1920, 1080));
        assert_eq!(Scaling::Quality.render_size(out), Some(Size::from((1280, 720))));
        assert_eq!(Scaling::Performance.render_size(out), Some(Size::from((960, 540))));
        let b = Scaling::Balanced.render_size(out).unwrap();
        assert_eq!(b.w % 2, 0);
        assert_eq!(b.h % 2, 0);
        assert!((b.w as f64 / b.h as f64 - 16.0 / 9.0).abs() < 0.01);
        assert_eq!(Scaling::Off.render_size(out), None);
        // A 4K screen at Quality: the "1440p" of the Guide's row.
        assert_eq!(Scaling::Quality.render_size(Size::from((3840, 2160))), Some(Size::from((2560, 1440))));
    }

    #[test]
    fn scanout_decisions() {
        let on = GameSettings::default();
        let off = GameSettings { direct: false, ..on };
        assert_eq!(scanout_choice(false, &on, false, false), ScanoutChoice::Normal);
        assert_eq!(scanout_choice(true, &on, false, false), ScanoutChoice::PreferGame);
        // The Guide or the Couch over the game: composited while it's there.
        assert_eq!(scanout_choice(true, &on, false, true), ScanoutChoice::Normal);
        // Sharpening needs the GPU: composited (as usual), never forced onto the plane.
        assert_eq!(scanout_choice(true, &on, true, false), ScanoutChoice::Normal);
        // Direct off stays off (composited) whatever is over the game.
        assert_eq!(scanout_choice(true, &off, false, true), ScanoutChoice::Never);
        assert_eq!(scanout_choice(true, &off, false, false), ScanoutChoice::Never);
        assert_eq!(scanout_choice(false, &off, false, false), ScanoutChoice::Normal);
    }

    #[test]
    fn what_is_drawn_over_a_fullscreen_game() {
        use smithay::wayland::shell::wlr_layer::{ExclusiveZone, Layer};
        // The Guide, notices, the performance overlay.
        assert!(drawn_over_fullscreen(Layer::Overlay, ExclusiveZone::Neutral));
        assert!(drawn_over_fullscreen(Layer::Overlay, ExclusiveZone::Exclusive(40)));
        // The Couch and Everything open (no room reserved).
        assert!(drawn_over_fullscreen(Layer::Top, ExclusiveZone::Neutral));
        assert!(drawn_over_fullscreen(Layer::Top, ExclusiveZone::DontCare));
        assert!(drawn_over_fullscreen(Layer::Top, ExclusiveZone::Exclusive(0)));
        // Mantel's bar and dock stay under the game; the wallpaper too.
        assert!(!drawn_over_fullscreen(Layer::Top, ExclusiveZone::Exclusive(48)));
        assert!(!drawn_over_fullscreen(Layer::Bottom, ExclusiveZone::Neutral));
        assert!(!drawn_over_fullscreen(Layer::Background, ExclusiveZone::DontCare));
    }

    #[test]
    fn vrr_decisions() {
        let on = GameSettings::default();
        let off = GameSettings { vrr: false, ..on };
        assert!(vrr_wanted(true, &on, VrrCap::Yes));
        assert!(!vrr_wanted(true, &on, VrrCap::NeedsModeset));
        assert!(!vrr_wanted(true, &on, VrrCap::No));
        assert!(!vrr_wanted(true, &on, VrrCap::Unknown));
        assert!(!vrr_wanted(false, &on, VrrCap::Yes));
        assert!(!vrr_wanted(true, &off, VrrCap::Yes));
    }

    #[test]
    fn settings_file() {
        let (g, bad) = parse_games(
            r#"{"games": {"ember": {"frameLimit": 40, "scaling": "balanced", "vrr": false, "direct": "x", "tearing": true}}}"#,
        )
        .unwrap();
        let e = g["ember"];
        assert_eq!(e.frame_limit, FrameLimit::Fps(40));
        assert_eq!(e.scaling, Scaling::Balanced);
        assert!(!e.vrr);
        assert!(e.tearing);
        assert!(!GameSettings::default().tearing, "tearing is off unless it's chosen");
        assert!(e.direct, "a value that isn't understood keeps the default");
        assert_eq!(bad.len(), 1);
        assert!(parse_games("{").is_err());
        assert!(parse_games("[]").is_err());
        assert!(parse_games("{}").unwrap().0.is_empty());
    }
}

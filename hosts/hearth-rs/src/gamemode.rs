//! Game Mode: what's in front (Hearth, the game, an app, Steam), Quick Access over a running game (the overlay),
//! the Guide button, the power and volume buttons. Port of the Electron host's main.js window handling.
//!
//! gamescope shows one window at a time; Hearth tells it which through root-window properties. Every change
//! goes through `run_focus`: one at a time, in order, and the newest request wins (a "soft" request, like the
//! hand-back after closing the overlay, only happens if nothing else was asked for meanwhile).
//!
//! The SteamOS way (hearth_core::appid): every game has a number, and so does Hearth. A thread puts the right number
//! on every window twice a second (`start_window_ids`: its process's HEARTH_GAME_ID, Steam's reaper, or Hearth's
//! own); a game comes to the front by asking gamescope for its number then Hearth's, and Hearth by asking for
//! Hearth's. Windows of games put aside carry their own numbers, which aren't asked for, so they can't come back by
//! accident. Single windows are pinned only when they aren't a game's (a Steam question, an app, a sign-in page).
//!
//! The overlay is a second, see-through window marked as gamescope's overlay (STEAM_OVERLAY). It stays mapped;
//! opening and closing only change how see-through it is (_NET_WM_WINDOW_OPACITY) and whether it gets input
//! (STEAM_INPUT_FOCUS). The controller is read here (evdev) and passed to it, since a window gamescope draws on
//! top doesn't reliably get the controller through the web view.
//!
//! On the Handheld Daemon editions (Anatase, Ubuntu, CachyOS, Peppermint) that window isn't made: HHD's overlay app
//! is Hearth's Quick Access (hosts/linux/src/overlay, in Chromium), HHD shows it over the game and holds the
//! controllers for it, and it talks to Hearth over hearth-core's overlay socket (overlay_link.rs). Hearth still
//! decides when it opens and closes (`hhd`): the same calls below, sent to that app instead of to a window.

use hearth_core::appid::{APPS_ID, HEARTH_ID};
use hearth_core::bridge::Bridge;
use hearth_core::gamescope as gs;
use hearth_core::procs;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::winhost::WindowHost;

pub const MAIN_TITLE: &str = "Hearth";
pub const OVERLAY_TITLE: &str = "Hearth Overlay";
const OPAQUE: u64 = 4294967295;

pub struct GameMode {
    /// Hearth's windows: Tauri's (WebKit), or the Electron window's (Chromium) through the core channel (winhost.rs).
    pub host: Arc<dyn WindowHost>,
    pub bridge: Arc<Mutex<Bridge>>,
    pub windowed: bool,
    pub guide: hearth_core::guide::Guide,
    gen: AtomicU64,
    focus_lock: Mutex<()>,
    /// One InputPlumber mode change at a time, always the one that fits now (open overlay: catch all; else pass).
    ip_lock: Mutex<()>,
    /// Opening and closing the overlay one at a time (a close during a slow open left it half open).
    overlay_lock: Mutex<()>,
    launch_gen: AtomicU64,
    pub overlay_on: AtomicBool,
    osd_on: AtomicBool,
    osd_gen: AtomicU64,
    look_closed: AtomicBool,
    before_overlay: AtomicU64,
    pub overlay_alive: AtomicU64,
    reveal_gen: AtomicU64,
    /// InputPlumber catches the Guide button and, while Quick Access is open, every press (see inputplumber.rs).
    pub ip: AtomicBool,
    launch_busy: AtomicU64,
    last_guide: AtomicU64,
    b_gen: AtomicU64,
    repeat_gen: AtomicU64,
    last_pad: Mutex<(String, u64)>,
    xids: Mutex<std::collections::HashMap<&'static str, u64>>,
    /// Each game's window when it was last in front (Quick Resume: the right one comes back, never another
    /// game's frozen picture).
    game_wins: Arc<Mutex<std::collections::HashMap<String, u64>>>,
    /// Handheld Daemon shows Quick Access over games (its overlay app is Hearth's); no overlay window of Hearth's own.
    pub hhd_overlay: AtomicBool,
    /// Windows that carry a number other than their process's: Xbox Cloud Gaming's window (one of Hearth's own)
    /// carries the cloud game's.
    win_tags: Arc<Mutex<std::collections::HashMap<u64, u32>>>,
    ids_on: AtomicBool,
}

fn now() -> u64 { hearth_core::library::now_ms() }
fn log(t: &str) { hearth_core::bridge::log(t); }
fn ev(name: &str, data: Value) -> Value { json!({"type": "event", "name": name, "data": data}) }
fn ev_pad(b: &str) -> Value { ev("pad", json!({"b": b})) }

impl GameMode {
    pub fn new(host: Arc<dyn WindowHost>, bridge: Arc<Mutex<Bridge>>, guide: hearth_core::guide::Guide, windowed: bool) -> Arc<GameMode> {
        let gm = Arc::new(GameMode { host, bridge, windowed, guide, gen: AtomicU64::new(0), focus_lock: Mutex::new(()), ip_lock: Mutex::new(()), overlay_lock: Mutex::new(()), launch_gen: AtomicU64::new(0),
            overlay_on: AtomicBool::new(false), osd_on: AtomicBool::new(false), osd_gen: AtomicU64::new(0), look_closed: AtomicBool::new(true),
            before_overlay: AtomicU64::new(0), overlay_alive: AtomicU64::new(0), reveal_gen: AtomicU64::new(0), ip: AtomicBool::new(false), launch_busy: AtomicU64::new(0), last_guide: AtomicU64::new(0), b_gen: AtomicU64::new(0),
            repeat_gen: AtomicU64::new(0), last_pad: Mutex::new((String::new(), 0)), xids: Mutex::new(Default::default()), game_wins: Default::default(),
            hhd_overlay: AtomicBool::new(false), win_tags: Default::default(), ids_on: AtomicBool::new(false) });
        let hhd = gm.inside() && hearth_core::overlay_link::hhd_hosts_overlay();
        gm.hhd_overlay.store(hhd, Ordering::SeqCst);
        gm
    }

    /// In gamescope as the Hearth session (not a window on a desktop).
    pub fn inside(&self) -> bool { gs::in_session() && !self.windowed && gs::available() }
    pub fn overlay_available(&self) -> bool { self.inside() }
    /// Handheld Daemon's overlay app is Quick Access over games (HHD editions), not Hearth's own overlay window.
    pub fn hhd(&self) -> bool { self.hhd_overlay.load(Ordering::SeqCst) }
    /// Something to show Quick Access in: HHD's overlay app, connected; or Hearth's overlay window.
    fn overlay_ready(&self) -> bool { if self.hhd() { hearth_core::overlay_link::connected() } else { self.has_overlay() } }

    fn cached_xid(&self, title: &'static str) -> Option<u64> { self.xids.lock().ok().and_then(|m| m.get(title).copied()) }
    fn xid(&self, title: &'static str) -> Option<u64> {
        if let Some(x) = self.xids.lock().unwrap().get(title) { return Some(*x); }
        let x = gs::window_named(title)?;
        self.xids.lock().unwrap().insert(title, x);
        Some(x)
    }
    /// Hearth's own window is there and shown (not hidden for a game).
    fn main_visible(&self) -> bool { self.host.exists("main") && self.host.is_visible("main") }
    fn main_focused(&self) -> bool { self.main_visible() && self.host.is_focused("main") }
    fn has_overlay(&self) -> bool { self.host.exists("overlay") }
    pub fn to_main(&self, v: Value) { self.host.emit_to("main", v); }
    fn to_overlay(&self, v: Value) { hearth_core::overlay_link::send(&v); self.host.emit_to("overlay", v); }

    // ------------------------------------------------------------------ what's in front

    /// A new choice was made: anything asked for before it is dropped.
    pub fn claim(&self) -> u64 { self.gen.fetch_add(1, Ordering::SeqCst) + 1 }

    /// Runs `f(still)` after the changes already queued, unless something newer was asked for meanwhile.
    pub fn run_focus(self: &Arc<Self>, label: &'static str, delay_ms: u64, soft: bool, f: impl FnOnce(&dyn Fn() -> bool) + Send + 'static) {
        if !self.inside() { return; }
        let n = if soft { self.gen.load(Ordering::SeqCst) } else { self.claim() };
        let me = self.clone();
        std::thread::spawn(move || {
            if delay_ms > 0 { std::thread::sleep(Duration::from_millis(delay_ms)); }
            let _g = me.focus_lock.lock().unwrap();
            let still = || me.gen.load(Ordering::SeqCst) == n;
            if !still() { log(&format!("focus: {label} dropped (something newer)")); return; }
            f(&still);
        });
    }
    pub fn focus_xid(self: &Arc<Self>, label: &'static str, win: u64, delay_ms: u64, soft: bool) {
        if win == 0 { return; }
        self.run_focus(label, delay_ms, soft, move |_| gs::focus_window(win));
    }

    /// Hearth back in front (a game ended, Guide was pressed...): Hearth's number.
    pub fn come_back(self: &Arc<Self>) {
        self.claim();
        if !self.overlay_on.load(Ordering::SeqCst) && !self.look_closed.load(Ordering::SeqCst) { self.overlay_look(false, false); }
        if self.host.exists("main") { self.host.unminimize("main"); self.host.show("main"); self.host.focus("main"); }
        let main = self.xid(MAIN_TITLE);
        self.run_focus("hearth", 0, false, move |_| {
            if let Some(x) = main { gs::tag_window(x, HEARTH_ID); }
            gs::focus_ids(&[HEARTH_ID]);
            // Without Steam mode, gamescope forgets a window's number when it's shown again, until Hearth puts it
            // back: pinned as well, until the next choice.
            if !gs::steam_mode() { if let Some(x) = main { gs::focus_window(x); } }
            log(&format!("focus: Hearth in front (id {HEARTH_ID})"));
        });
    }
    pub fn step_aside(&self) { if !self.windowed && self.host.exists("main") { self.host.hide("main"); } }

    fn running(&self) -> Option<(String, Vec<u32>)> {
        let b = self.bridge.lock().ok()?;
        let r = b.library.running.as_ref()?;
        Some((r.id.clone(), b.library.game_pids()))
    }
    fn steam_app(id: &str) -> u64 { id.strip_prefix("steam:").and_then(|s| s.parse().ok()).unwrap_or(0) }
    /// The running game and its number.
    fn running_app(&self) -> Option<(String, u32)> {
        let b = self.bridge.lock().ok()?;
        let r = b.library.running.as_ref()?;
        Some((r.id.clone(), if r.app_id != 0 { r.app_id } else { hearth_core::appid::steam_app_id(&r.id).unwrap_or(0) }))
    }

    /// The running game in front: by its number (gamescope picks its best window), or its window when none carries it.
    pub fn focus_running_game(self: &Arc<Self>, delay_ms: u64, soft: bool) {
        let Some((id, app_id)) = self.running_app() else { return };
        let bridge = self.bridge.clone();
        let game_wins = self.game_wins.clone();
        self.run_focus("game", delay_ms, soft, move |still| {
            let pids = || bridge.lock().map(|b| b.library.game_pids()).unwrap_or_default();
            // Never a game put aside (its window shows its last picture, frozen): not its processes, not its number.
            let (frozen, mut other_ids) = bridge.lock().map(|b| (b.library.frozen_pids(), b.library.frozen_ids())).unwrap_or_default();
            let mut exclude: Vec<u64> = frozen.into_iter().map(u64::from).collect();
            exclude.push(std::process::id() as u64);
            other_ids.push(HEARTH_ID);
            // A window pinned earlier that's a frozen game's (or Hearth's) would stay in front over the number: it goes.
            if let Some(cur) = gs::pinned() {
                let stale = gs::focusable_windows().iter().any(|w| w.win == cur && (exclude.contains(&w.pid)
                    || (w.appid != app_id as u64 && w.appid <= u32::MAX as u64 && other_ids.contains(&(w.appid as u32)))));
                if stale { gs::clear_pin(); }
            }
            // Windows other games had: one of them may be frozen with processes Hearth didn't match.
            let (known, others) = {
                let m = game_wins.lock().unwrap();
                (m.get(&id).copied().unwrap_or(0), m.iter().filter(|(k, _)| **k != id).map(|(_, w)| *w).collect::<Vec<u64>>())
            };
            log(&format!("focus: {id} (id {app_id}) to the front; games put aside: {:?}", other_ids.iter().filter(|n| **n != HEARTH_ID).collect::<Vec<_>>()));
            let want = gs::Want { app_id, exclude_pids: &exclude, not_wins: &others, other_ids: &other_ids, known };
            if let Some(w) = gs::focus_game(&want, &pids, still) {
                let mut m = game_wins.lock().unwrap();
                m.retain(|k, v| *v != w || *k == id);
                m.insert(id.clone(), w);
            } else if still() {
                log(&format!("focus: {id} (id {app_id}) has no window to show yet"));
            }
        });
    }

    /// Every window carries its game's number (hearth_core::appid), checked twice a second: Hearth's own windows get
    /// Hearth's, a game's get the game's (from its processes: HEARTH_GAME_ID, Steam's reaper, or for a game started
    /// outside Hearth the library's guesses), and a window gamescope forgot the number of (shown again without
    /// Steam mode) gets it back. A new window of a program Hearth opened that isn't a game (a browser, Moonlight)
    /// comes to the front, as it did before. First, gamescope is asked to follow the numbers (take_focus_control).
    pub fn start_window_ids(self: &Arc<Self>) {
        if !self.inside() || self.ids_on.swap(true, Ordering::SeqCst) { return; }
        let me = self.clone();
        std::thread::spawn(move || {
            let mine = std::process::id() as u64;
            let mut seen: std::collections::HashMap<u64, (u64, Option<u32>)> = Default::default();
            // Hearth's window carries Hearth's number before gamescope is told to follow numbers (or it would show nothing).
            for _ in 0..40 {
                me.tag_windows(&mut seen, true);
                if gs::focusable_windows().iter().any(|w| w.pid == mine && w.appid == HEARTH_ID as u64) { break; }
                std::thread::sleep(Duration::from_millis(500));
            }
            if gs::requested_ids().is_empty() { gs::focus_ids(&[HEARTH_ID]); }
            for _ in 0..3 {
                let r = gs::take_focus_control();
                log(&format!("ids: {r}"));
                if !r.starts_with("couldn't") { break; }
                std::thread::sleep(Duration::from_secs(3));
            }
            log(&format!("ids: Hearth is id {HEARTH_ID}, apps it opens {APPS_ID}; gamescope in Steam mode: {}", gs::steam_mode()));
            // Twice a second: without Steam mode a game's window that's mapped again (Wine does it switching modes)
            // has no number until Hearth puts it back (the screen stays dark for that moment).
            loop {
                std::thread::sleep(Duration::from_millis(500));
                me.tag_windows(&mut seen, false);
                gs::keep_list();
            }
        });
    }

    /// One pass of `start_window_ids`. `seen`: window → (its process, the number it should carry).
    fn tag_windows(self: &Arc<Self>, seen: &mut std::collections::HashMap<u64, (u64, Option<u32>)>, starting: bool) {
        let wins = gs::focusable_windows();
        let mine = std::process::id();
        let root = std::path::Path::new("/proc");
        let overlay = self.cached_xid(OVERLAY_TITLE);
        let explicit: std::collections::HashMap<u64, u32> = {
            let mut m = self.win_tags.lock().unwrap();
            if !wins.is_empty() { m.retain(|w, _| wins.iter().any(|x| x.win == *w)); }
            m.clone()
        };
        let mut open: Option<Option<Vec<(u32, String, Vec<u32>)>>> = None;
        let mut new_apps: Vec<u64> = Vec::new();
        for w in &wins {
            if Some(w.win) == overlay { continue; }
            let pid = w.pid;
            let prev = seen.get(&w.win).cloned().filter(|(p, _)| *p == pid);
            let want: Option<u32> = if let Some(id) = explicit.get(&w.win) {
                Some(*id)
            } else if pid == mine as u64 {
                Some(HEARTH_ID)
            } else if pid == 0 || pid >= u32::MAX as u64 {
                None
            } else if let Some((_, Some(id))) = prev {
                Some(id)
            } else if prev.is_some() {
                None   // looked at already: not a game's
            } else {
                let p = pid as u32;
                if procs::is_steam_client(&procs::comm(root, p)) { None }
                else if let Some(id) = procs::owner_of(root, p, mine) { Some(id) }
                else {
                    // A game whose processes carry no number (started outside Hearth or by an older one): the guesses.
                    let games = open.get_or_insert_with(|| self.bridge.try_lock().ok().map(|b| b.library.open_games()));
                    let Some(games) = games.as_ref() else { continue };   // the library is busy: next time
                    match games.iter().find(|(id, _, ps)| *id != 0 && ps.contains(&p)) {
                        Some((id, _, _)) => Some(*id),
                        None if procs::descends_from(root, p, mine) => { new_apps.push(w.win); Some(APPS_ID) }
                        None => None,
                    }
                }
            };
            if let Some(id) = want {
                if w.appid != id as u64 && gs::tag_window(w.win, id) && prev.map(|(_, d)| d != Some(id)).unwrap_or(true) {
                    let what = if id == HEARTH_ID { "Hearth".to_string() } else if id == APPS_ID { "an app Hearth opened".to_string() } else { format!("process {pid} {}", procs::comm(root, pid as u32)) };
                    log(&format!("ids: window {:#x} ({what}) gets id {id}", w.win));
                }
            }
            seen.insert(w.win, (pid, want));
        }
        if !wins.is_empty() { seen.retain(|k, _| wins.iter().any(|w| w.win == *k)); }
        if starting { return; }
        for win in new_apps {
            log(&format!("ids: new window {win:#x} of a program Hearth opened: in front"));
            self.focus_xid("app window", win, 0, false);
        }
    }

    /// Hearth stays in front with its "starting" screen while the game loads, and puts it in front once its
    /// window is really up. A Steam window that may need you is shown right away.
    pub fn show_game_when_ready(self: &Arc<Self>, id: String) {
        let g = self.launch_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let me = self.clone();
        std::thread::spawn(move || {
            log(&format!("launch: waiting for {id} to open its window"));
            let app_id = Self::steam_app(&id);
            let app_number = me.running_app().filter(|(rid, _)| *rid == id).map(|(_, n)| n).unwrap_or(0);
            let slow = { let (me, id) = (me.clone(), id.clone()); std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(40));
                if me.launch_gen.load(Ordering::SeqCst) == g { me.to_main(ev("game.slow", json!({"id": id, "firstRun": app_id == 0}))); }
            }) };
            drop(slow);
            let bridge = me.bridge.clone();
            let pids = || bridge.lock().map(|b| b.library.game_pids()).unwrap_or_default();
            let cancelled = || me.launch_gen.load(Ordering::SeqCst) != g || me.bridge.lock().map(|b| b.library.running_id().as_deref() != Some(id.as_str())).unwrap_or(true);
            me.launch_busy.store(g, Ordering::SeqCst);
            let r = gs::wait_for_window(app_number, &pids, &[std::process::id() as u64], Duration::from_secs(600), &cancelled);
            let _ = me.launch_busy.compare_exchange(g, 0, Ordering::SeqCst, Ordering::SeqCst);
            if cancelled() { return; }
            me.launch_gen.fetch_add(1, Ordering::SeqCst);
            me.step_aside();
            match r {
                Some(gs::Ready::Window(w)) => {
                    log(&format!("launch: {id} is ready (window {w:#x}, id {app_number}, {})", procs::describe(&pids())));
                    { let mut m = me.game_wins.lock().unwrap(); m.retain(|_, v| *v != w); m.insert(id.clone(), w); }
                    if app_number != 0 {
                        me.run_focus("game window", 0, false, move |_| { gs::front_by_id(app_number, w); log(&format!("focus: id {app_number} in front (window {w:#x})")); });
                    } else {
                        me.focus_xid("game window", w, 0, false);
                    }
                }
                Some(gs::Ready::Dialog(w, t)) => { log(&format!("launch: Steam is asking something (\"{t}\"), showing it")); me.focus_xid("Steam question", w, 0, false); me.focus_running_game(0, false); }
                None => { log(&format!("launch: {id} didn't open a window in 10 minutes; showing whatever it has")); me.focus_running_game(0, false); }
            }
            me.to_main(ev("game.ready", json!({"id": id})));
        });
    }

    /// A window that's still there and doesn't belong to a game put aside (frozen on its last picture).
    fn window_ok(&self, win: u64) -> bool {
        let Some(w) = gs::focusable_windows().into_iter().find(|w| w.win == win) else { return false };
        let (frozen, ids) = self.bridge.lock().map(|b| (b.library.frozen_pids(), b.library.frozen_ids())).unwrap_or_default();
        !frozen.contains(&(w.pid as u32)) && !(w.appid <= u32::MAX as u64 && ids.contains(&(w.appid as u32)))
    }
    /// Is this window the running game's (its number, or one of its processes)?
    fn running_games_window(&self, win: u64) -> bool {
        let Some((_, app_id)) = self.running_app() else { return false };
        let Some(w) = gs::focusable_windows().into_iter().find(|w| w.win == win) else { return false };
        (app_id != 0 && w.appid == app_id as u64) || self.bridge.lock().map(|b| b.library.game_pids().contains(&(w.pid as u32))).unwrap_or(false)
    }
    /// Xbox Cloud Gaming's window (one of Hearth's own) carries the cloud game's number, which goes in front.
    fn focus_stream(self: &Arc<Self>, delay_ms: u64, soft: bool) {
        let Some(x) = gs::window_named(&self.host.title("stream").unwrap_or_default()) else { return };
        let app_id = self.running_app().map(|(_, n)| n).unwrap_or(0);
        if app_id == 0 { self.focus_xid("stream", x, delay_ms, soft); return; }
        if let Ok(mut m) = self.win_tags.lock() { m.insert(x, app_id); }
        self.run_focus("stream", delay_ms, soft, move |_| {
            gs::tag_window(x, app_id);
            gs::front_by_id(app_id, x);
            log(&format!("focus: Xbox Cloud Gaming's window {x:#x} carries id {app_id}; in front"));
        });
    }
    /// Hearth is still showing "starting" for a game that hasn't put its window up yet.
    fn launch_waiting(&self) -> bool { let b = self.launch_busy.load(Ordering::SeqCst); b != 0 && b == self.launch_gen.load(Ordering::SeqCst) }
    /// The game in front again a little later as well: games coming back from a pause (Quick Resume) or a
    /// minimize take a moment to draw, and the first try can land before their window is ready.
    fn nudge_game(self: &Arc<Self>) {
        for ms in [1200u64, 3000] { self.focus_running_game(ms, true); }
    }

    /// Back to the game (Quick Access closed, "Back to game"): un-paused and in front.
    pub fn resume_game(self: &Arc<Self>) {
        let running = self.bridge.lock().map(|mut b| { let id = b.library.running_id(); if id.is_some() { b.library.resume(); } id }).unwrap_or(None);
        if running.is_none() { return; }
        self.launch_gen.fetch_add(1, Ordering::SeqCst);
        self.claim();
        self.step_aside();
        if self.host.exists("stream") { self.host.show("stream"); self.host.focus("stream"); self.focus_stream(0, false); return; }
        self.focus_running_game(0, false);
        self.nudge_game();
    }

    /// The library's events: put the game in front when it starts, Hearth back when it ends.
    pub fn on_library_event(self: &Arc<Self>, e: &Value) {
        let id = e["data"]["id"].as_str().unwrap_or("").to_string();
        match e["name"].as_str().unwrap_or("") {
            "game.started" => {
                if self.host.exists("stream") {
                    self.step_aside();
                    self.to_main(ev("game.ready", json!({"id": id})));
                    self.focus_stream(900, true);
                    return;
                }
                if !self.inside() { self.step_aside(); self.to_main(ev("game.ready", json!({"id": id}))); return; }
                self.show_game_when_ready(id);
            }
            "game.exited" | "game.gap" => {
                if e["name"] == "game.exited" { self.game_wins.lock().unwrap().remove(&id); }
                self.hide_overlay(); self.come_back();
            }
            "game.handoff" => if self.inside() { self.show_game_when_ready(id) } else { self.focus_running_game(0, false) },
            _ => {}
        }
    }

    /// Is Hearth's own window the one on screen?
    fn hearth_on_screen(&self) -> bool {
        if !self.main_visible() { return false; }
        if !self.inside() { return self.host.is_focused("main"); }
        match (gs::focused(), self.xid(MAIN_TITLE)) { (Some(f), Some(m)) => f == m, _ => self.host.is_focused("main") }
    }
    pub fn hearth_in_front(&self) -> bool { self.main_focused() && !self.overlay_on.load(Ordering::SeqCst) }

    /// Another Hearth window (a sign-in page) in front in gamescope.
    pub fn to_front(self: &Arc<Self>, title: &str) {
        if !self.inside() { return; }
        let me = self.clone();
        let t = title.to_string();
        std::thread::spawn(move || {
            for _ in 0..10 { if let Some(x) = gs::window_named(&t) { me.focus_xid("window", x, 0, false); return; } std::thread::sleep(Duration::from_millis(300)); }
        });
    }
    /// File pickers are separate windows gamescope wouldn't show while Hearth's is pinned in front.
    pub fn before_dialog(&self) { if self.inside() { self.claim(); gs::clear_focus(); } }

    // ------------------------------------------------------------------ the overlay

    pub fn overlay_look(&self, visible: bool, input: bool) {
        self.look_closed.store(!visible, Ordering::SeqCst);
        // HHD shows and hides its overlay app itself (the app tells it what's open).
        if self.hhd() { return; }
        if let Some(x) = self.xid(OVERLAY_TITLE) {
            let _ = gs::set_window(x, "_NET_WM_WINDOW_OPACITY", if visible { OPAQUE } else { 0 });
            let _ = gs::set_window(x, "STEAM_INPUT_FOCUS", if input { 1 } else { 0 });
        }
    }
    /// On screen, see-through, marked as gamescope's overlay, not taking input: ready to open at once.
    pub fn arm_overlay(&self) {
        if !self.has_overlay() { return; }
        self.host.show("overlay");
        // The window manager needs a moment to give it an X window.
        for _ in 0..20 { if self.xid(OVERLAY_TITLE).is_some() { break; } std::thread::sleep(Duration::from_millis(150)); }
        self.overlay_look(false, false);
        if let Some(x) = self.xid(OVERLAY_TITLE) { let _ = gs::set_window(x, "STEAM_OVERLAY", 1); }
    }

    pub fn show_overlay(self: &Arc<Self>) -> bool {
        if !self.overlay_ready() { return false; }
        let _o = self.overlay_lock.lock().unwrap();
        self.overlay_on.store(true, Ordering::SeqCst);
        self.osd_on.store(false, Ordering::SeqCst);
        self.claim();   // opening it is a choice too: a hand-back still waiting from the last close is dropped
        self.before_overlay.store(gs::focused().unwrap_or(0), Ordering::SeqCst);
        self.guide.buttons.store(true, Ordering::SeqCst);
        self.hold_game(true);
        // The page says it's alive every second while open; silence for 5 s means it froze: reload, close.
        self.overlay_alive.store(now(), Ordering::SeqCst);
        let me = self.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            if !me.overlay_on.load(Ordering::SeqCst) { return; }
            if now().saturating_sub(me.overlay_alive.load(Ordering::SeqCst)) > 5000 {
                log("overlay: page stopped answering, closing and reloading it");
                me.hide_overlay();
                if me.hhd() { hearth_core::overlay_link::send(&json!({"type": "overlay-host", "what": "reload"})); }
                else if me.has_overlay() { me.host.eval("overlay", "location.reload()"); }
                return;
            }
        });
        // HHD's overlay app: it tells HHD to show it and hold the controllers when it hears overlay.shown, and HHD
        // hands it the controller's presses.
        if self.hhd() {
            self.to_overlay(ev("padHost", json!({"connected": 1})));
            self.to_overlay(ev("overlay.shown", json!({})));
            return true;
        }
        // Not focused: taking the keyboard focus from the game makes some games minimize or stall (a black or
        // frozen screen after). Its buttons come from the controller readers; gamescope sends touch to it.
        if self.has_overlay() { self.host.show("overlay"); }
        if let Some(x) = self.xid(OVERLAY_TITLE) { let _ = gs::set_window(x, "STEAM_OVERLAY", 1); let _ = gs::set_window(x, "STEAM_INPUT_FOCUS", 1); }
        // Visible once the page says Quick Access is drawn, or after 300 ms at the latest.
        let r = self.reveal_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let me = self.clone();
        std::thread::spawn(move || { std::thread::sleep(Duration::from_millis(300)); if me.reveal_gen.load(Ordering::SeqCst) == r { me.reveal_overlay(); } });
        let pads = self.guide.connected();
        self.to_overlay(ev("padHost", json!({"connected": if pads > 0 { 1 } else { 0 }})));
        self.to_overlay(ev("overlay.shown", json!({})));
        true
    }
    /// While the overlay is open the game gets no controller input, and it keeps running: InputPlumber sends every
    /// press to Hearth instead of the game; controllers it doesn't manage are taken by Hearth for the time being.
    fn hold_game(&self, on: bool) {
        // Hearth Input (a game's controller layout) lets go of the controller before Hearth takes it, and gets it
        // back only after Hearth has let go (input_map.rs: it answers once it has).
        if on { hearth_core::input_map::overlay(true); }
        // With HHD's overlay app, HHD holds the controllers (the app asks it to while Quick Access is open).
        if !self.hhd() { self.guide.grab(on); }
        if self.ip.load(Ordering::SeqCst) {
            let _ = on; self.ip_sync();
        }
        if !on { hearth_core::input_map::overlay(false); }
    }

    /// Hearth Input: tells the mapper (desktop/shell/hearth-input) which game is running, whether it's in front and
    /// whether Quick Access is open, so that game's controller layout is on exactly while you're playing it. Guide
    /// presses it catches (it holds the controller) open Quick Access; its notes (gyro on/off, the radial menu) show
    /// over the game like the volume bar.
    pub fn start_input_map(self: &Arc<Self>) {
        if !self.inside() { return; }
        let (me, me2) = (self.clone(), self.clone());
        let last: Arc<Mutex<Option<String>>> = Arc::default();
        let get: hearth_core::input_map::GetState = Arc::new(move || {
            // Never waits for the bridge (a library scan can hold it): the last game seen is good enough.
            let id = match me.bridge.try_lock() {
                Ok(b) => { let id = b.library.running_id(); *last.lock().unwrap() = id.clone(); id }
                Err(_) => last.lock().unwrap().clone(),
            };
            let overlay = me.overlay_on.load(Ordering::SeqCst);
            let front = id.is_some() && !overlay && !me.hearth_on_screen();
            (id, front, overlay)
        });
        let on_event: hearth_core::input_map::OnEvent = Arc::new(move |e: Value| match e["t"].as_str() {
            Some("guide") => { let m = me2.clone(); std::thread::spawn(move || m.guide_pressed("guide")); }
            Some("osd") => {
                let text: String = e["text"].as_str().unwrap_or("").chars().take(200).collect();
                me2.osd(json!({"kind": "msg", "icon": "pad", "text": text, "ms": 1600}));
            }
            _ => {}
        });
        hearth_core::input_map::start_link(get, on_event);
    }

    /// Hearth OS: InputPlumber catches the Xbox / Guide button (and only it), and while Quick Access is open hands
    /// every press to Hearth. Reads its presses from D-Bus.
    pub fn start_inputplumber(self: &Arc<Self>) {
        if !self.inside() { return; }
        // Hearth OS on Anatase: Handheld Daemon instead of InputPlumber.
        if !hearth_core::inputplumber::available() { self.start_hhd(); return; }
        hearth_core::inputplumber::guide_only();
        self.ip.store(true, Ordering::SeqCst);
        self.ip_sync();
        log("inputplumber: catching the Guide button for Quick Access");
        let me = self.clone();
        std::thread::spawn(move || loop {
            use std::io::BufRead;
            let child = std::process::Command::new("gdbus").args(["monitor", "--system", "--dest", "org.shadowblip.InputPlumber"])
                .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn();
            let Ok(mut child) = child else { return };
            if let Some(out) = child.stdout.take() {
                for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
                    let Some((ev, down)) = hearth_core::inputplumber::parse_event(&line) else { continue };
                    if ev == "ui_guide" { if down { let m = me.clone(); std::thread::spawn(move || m.guide_pressed("guide")); } continue; }
                    if me.overlay_on.load(Ordering::SeqCst) { if let Some(b) = hearth_core::inputplumber::button(&ev) { me.overlay_pad(b, down); } }
                    // Caught presses with Quick Access closed (the service was left catching everything): give the
                    // controller back, and don't lose the press: Hearth's own screen gets it.
                    else if down {
                        me.ip_sync();
                        if let Some(b) = hearth_core::inputplumber::button(&ev) { if b != "guide" && me.hearth_on_screen() { me.to_main(ev_pad(b)); } }
                    }
                }
            }
            let _ = child.wait();
            std::thread::sleep(Duration::from_secs(2));
            // New controllers (paired later) get the same Guide-only catch.
            hearth_core::inputplumber::guide_only();
            me.ip_sync();
        });
        // Controllers that connect later: set them up too, every 10 seconds (cheap).
        let me2 = self.clone();
        std::thread::spawn(move || {
            let mut known = hearth_core::inputplumber::devices();
            loop {
                std::thread::sleep(Duration::from_secs(10));
                let now = hearth_core::inputplumber::devices();
                if now != known {
                    hearth_core::inputplumber::guide_only();
                    me2.ip_sync();
                    known = now;
                }
            }
        });
    }
    /// Hearth OS on Anatase (and the other HHD editions): Handheld Daemon (HHD) runs the built-in controller. Its
    /// virtual controller is an ordinary one, so the Guide button comes through the controller reader (guide.rs) and
    /// opens Quick Access. HHD can't hold the Guide press back from the game the way InputPlumber does (see
    /// docs/BASE-ANATASE.md). Where HHD's overlay app is Hearth's (hhd()), HHD shows Quick Access and holds the
    /// controllers while it's open, and its QAM button opens it too (hhd_open); without it, HHD's QAM button sends
    /// Guide + A, and the chord guard keeps that A from picking something in Quick Access. HHD may start a little
    /// after Hearth: looked for for a minute.
    fn start_hhd(self: &Arc<Self>) {
        let me = self.clone();
        std::thread::spawn(move || {
            for _ in 0..12 {
                if hearth_core::hhd::available() {
                    me.guide.chord_guard.store(true, Ordering::SeqCst);
                    let what = hearth_core::hhd::request("GET", "/api/v1/state", None).map(|st| hearth_core::hhd::describe(&st)).unwrap_or_default();
                    log(&format!("hhd: Handheld Daemon runs the controller ({what}); Guide comes from its virtual controller"));
                    return;
                }
                std::thread::sleep(Duration::from_secs(5));
            }
        });
    }
    /// Leaving: InputPlumber goes back to normal (or the Guide button would do nothing on the desktop).
    pub fn stop_inputplumber(&self) {
        let _g = self.ip_lock.lock().unwrap();
        if self.ip.swap(false, Ordering::SeqCst) { hearth_core::inputplumber::set_mode(hearth_core::inputplumber::MODE_NONE); }
    }
    /// InputPlumber's mode for what's showing now: Quick Access open catches every press; otherwise only Guide.
    pub fn ip_sync(&self) {
        if !self.ip.load(Ordering::SeqCst) { return; }
        let _g = self.ip_lock.lock().unwrap();
        let want = if self.overlay_on.load(Ordering::SeqCst) { hearth_core::inputplumber::MODE_ALWAYS } else { hearth_core::inputplumber::MODE_PASS };
        hearth_core::inputplumber::set_mode(want);
    }
    pub fn reveal_overlay(&self) {
        self.reveal_gen.fetch_add(1, Ordering::SeqCst);
        if self.overlay_on.load(Ordering::SeqCst) && self.look_closed.load(Ordering::SeqCst) { self.overlay_look(true, true); }
    }
    pub fn hide_overlay(self: &Arc<Self>) {
        if !self.hhd() && !self.has_overlay() { return; }
        let _o = self.overlay_lock.lock().unwrap();
        if !self.overlay_on.load(Ordering::SeqCst) && !self.osd_on.load(Ordering::SeqCst) && self.look_closed.load(Ordering::SeqCst) {
            // Already closed here: HHD's overlay app may think otherwise (it asked to close, or it just started).
            if self.hhd() { self.to_overlay(ev("overlay.hidden", json!({}))); }
            return;
        }
        self.overlay_on.store(false, Ordering::SeqCst);
        self.osd_on.store(false, Ordering::SeqCst);
        self.reveal_gen.fetch_add(1, Ordering::SeqCst);
        self.repeat_gen.fetch_add(1, Ordering::SeqCst);
        self.b_gen.fetch_add(1, Ordering::SeqCst);
        // Multi-seat: the controller keeps coming through Hearth (seat_pad).
        self.guide.buttons.store(hearth_core::multiplay::seat_hosted(), Ordering::SeqCst);
        self.hold_game(false);
        self.to_overlay(ev("overlay.hidden", json!({})));
        self.overlay_look(false, false);
        let me = self.clone();
        std::thread::spawn(move || { std::thread::sleep(Duration::from_millis(500)); if !me.overlay_on.load(Ordering::SeqCst) && !me.osd_on.load(Ordering::SeqCst) { me.overlay_look(false, false); } });
        // The controller back to what was on screen. Soft: anything chosen meanwhile wins.
        if self.inside() && !self.main_focused() {
            let before = self.before_overlay.load(Ordering::SeqCst);
            let launching = self.launch_waiting();
            // A game that's starting (or just started) may have swapped its splash for its real window while the
            // overlay was open: put back what's there now, not the window from before (that looked frozen).
            if launching { if let Some(x) = self.xid(MAIN_TITLE) { if before == x { self.focus_xid("back to starting screen", x, 200, true); } } }
            else if before != 0 && Some(before) != self.xid(OVERLAY_TITLE) && Some(before) != self.xid(MAIN_TITLE) && self.window_ok(before) {
                // The game's own window: by its number (its launcher may have handed over meanwhile); another app's: pinned.
                if self.running_games_window(before) { self.focus_running_game(200, true); self.nudge_game(); } else { self.focus_xid("back from the overlay", before, 200, true); }
            }
            else if self.running().is_some() { self.focus_running_game(200, true); self.nudge_game(); }
        }
    }

    /// Multi-seat: this player's controller for Hearth's page, read here (the host gives input to one window at a
    /// time, and the web view only reads the controller in that one). Only while Hearth is what this player sees
    /// (not a game, not the overlay); with repeat on held directions. Guide goes through guide_pressed as always.
    pub fn seat_pad(self: &Arc<Self>, b: &str, down: bool) {
        if !hearth_core::multiplay::seat_hosted() || self.overlay_on.load(Ordering::SeqCst) || b == "guide" { return; }
        let rg = self.repeat_gen.fetch_add(1, Ordering::SeqCst) + 1;
        if !down { return; }
        if !self.hearth_on_screen() { return; }
        {
            let mut last = self.last_pad.lock().unwrap();
            if last.0 == b && now().saturating_sub(last.1) < 40 { return; }   // the same press from two devices
            *last = (b.to_string(), now());
        }
        self.to_main(ev_pad(b));
        if ["up", "down", "left", "right"].contains(&b) {
            let (me, b) = (self.clone(), b.to_string());
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(340));
                while me.repeat_gen.load(Ordering::SeqCst) == rg && !me.overlay_on.load(Ordering::SeqCst) {
                    me.to_main(ev_pad(&b));
                    std::thread::sleep(Duration::from_millis(85));
                }
            });
        }
    }

    /// Controller presses for the overlay, with repeat on held directions.
    pub fn overlay_pad(self: &Arc<Self>, b: &str, down: bool) {
        if !self.overlay_on.load(Ordering::SeqCst) { return; }
        // Holding B for a second always closes it, even if the page has stopped answering.
        if b == "b" {
            let g = self.b_gen.fetch_add(1, Ordering::SeqCst) + 1;
            if down { let me = self.clone(); std::thread::spawn(move || { std::thread::sleep(Duration::from_secs(1)); if me.b_gen.load(Ordering::SeqCst) == g { log("overlay: B held, closing"); me.hide_overlay(); } }); }
        }
        let rg = self.repeat_gen.fetch_add(1, Ordering::SeqCst) + 1;
        if !down { return; }
        {
            let mut last = self.last_pad.lock().unwrap();
            if last.0 == b && now().saturating_sub(last.1) < 40 { return; }   // the same press from two devices
            *last = (b.to_string(), now());
        }
        if b == "guide" { return; }   // Guide closes it (the press handler)
        self.to_overlay(ev("pad", json!({"b": b})));
        if ["up", "down", "left", "right"].contains(&b) {
            let (me, b) = (self.clone(), b.to_string());
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(340));
                while me.repeat_gen.load(Ordering::SeqCst) == rg && me.overlay_on.load(Ordering::SeqCst) {
                    me.to_overlay(ev("pad", json!({"b": b})));
                    std::thread::sleep(Duration::from_millis(85));
                }
            });
        }
    }

    /// Guide (or View held, or Ctrl+Alt+Q): over anything that isn't Hearth, opens the overlay; again closes it.
    pub fn guide_pressed(self: &Arc<Self>, how: &str) {
        // The same press can come from two places (InputPlumber and the controller itself).
        let t = now();
        // A press ignored as a repeat still made InputPlumber catch everything: put its mode back.
        if t.saturating_sub(self.last_guide.swap(t, Ordering::SeqCst)) < 400 { self.ip_sync(); return; }
        if self.overlay_on.load(Ordering::SeqCst) { log(&format!("overlay: {how} pressed, closing")); self.hide_overlay(); return; }
        if self.hearth_on_screen() {
            // Hearth's own UI handles Guide itself: InputPlumber caught it, so pass it on and give the controller back.
            if self.ip.load(Ordering::SeqCst) { self.ip_sync(); self.to_main(ev("pad", json!({"b": "guide"}))); }
            return;
        }
        log(&format!("overlay: {how} pressed over another app"));
        if self.show_overlay() { return; }
        self.ip_sync();
        self.come_back();
        self.to_main(ev("pad", json!({"b": "guide"})));
    }

    /// Handheld Daemon's QAM button (its overlay app asks): Quick Access, like Guide. Over a game it opens there; with
    /// Hearth in front, Hearth's own Quick Access opens (as Guide does). Already open: the app is told again (it may
    /// have restarted). Closing goes through the page's own close (from_overlay).
    pub fn hhd_open(self: &Arc<Self>) {
        if self.overlay_on.load(Ordering::SeqCst) { self.to_overlay(ev("overlay.shown", json!({}))); return; }
        if self.hearth_on_screen() { self.to_main(ev("pad", json!({"b": "guide"}))); return; }
        log("overlay: Handheld Daemon's Quick Access button pressed over another app");
        if self.show_overlay() { return; }
        self.come_back();
        self.to_main(ev("pad", json!({"b": "guide"})));
    }
    /// HHD (re)started its overlay app: it starts closed, so Hearth's idea of it does too.
    pub fn overlay_app_connected(self: &Arc<Self>) {
        log("overlay: Handheld Daemon's overlay app connected");
        if self.overlay_on.load(Ordering::SeqCst) { self.hide_overlay(); }
    }

    /// The volume bar: in Hearth when it's in front, else over the game (the overlay, without taking input).
    pub fn osd(self: &Arc<Self>, data: Value) {
        if self.hearth_in_front() || !self.overlay_ready() || self.running().is_none() { self.to_main(ev("osd", data)); return; }
        if !self.overlay_on.load(Ordering::SeqCst) && !self.osd_on.load(Ordering::SeqCst) { self.osd_on.store(true, Ordering::SeqCst); self.overlay_look(true, false); }
        let ms = data["ms"].as_u64().unwrap_or(1800);
        self.to_overlay(ev("osd", data));
        let g = self.osd_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let me = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(ms));
            if me.osd_gen.load(Ordering::SeqCst) == g && me.osd_on.load(Ordering::SeqCst) && !me.overlay_on.load(Ordering::SeqCst) { me.osd_on.store(false, Ordering::SeqCst); me.overlay_look(false, false); }
        });
    }

    pub fn power_menu(self: &Arc<Self>) {
        if self.running().is_some() && !self.hearth_in_front() {
            if self.overlay_on.load(Ordering::SeqCst) || self.show_overlay() { self.osd_on.store(false, Ordering::SeqCst); self.to_overlay(ev("power.menu", json!({}))); return; }
            self.come_back();
        }
        self.to_main(ev("power.menu", json!({})));
    }

    /// A press from hearth-buttons (Game Mode): volume, mute, power.
    pub fn button(self: &Arc<Self>, m: &Value) {
        let max = self.bridge.lock().map(|b| b.max_volume()).unwrap_or(100);
        match m["t"].as_str().unwrap_or("") {
            "vol" => if let Ok(mut v) = hearth_core::system::audio::step(if m["d"].as_i64().unwrap_or(0) > 0 { 1 } else { -1 }, max) { v["kind"] = json!("vol"); v["max"] = json!(max); self.osd(v); },
            "mute" => if let Ok(mut v) = hearth_core::system::audio::toggle_mute() { v["kind"] = json!("vol"); v["max"] = json!(max); self.osd(v); },
            "power" => {
                let prefs = self.bridge.lock().map(|b| b.button_prefs()).unwrap_or(Value::Null);
                let what = prefs[if m["press"] == "long" { "hold" } else { "press" }].as_str().unwrap_or("sleep").to_string();
                log(&format!("power button: {} press → {what}", m["press"]));
                match what.as_str() {
                    "sleep" | "shutdown" => { let _ = hearth_core::power::action(&what, gs::in_session()); }
                    "menu" => self.power_menu(),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Messages from the overlay page.
    pub fn from_overlay(self: &Arc<Self>, m: &Value) -> bool {
        match (m["type"].as_str().unwrap_or(""), m["action"].as_str().unwrap_or("")) {
            ("overlay", "close") => self.hide_overlay(),
            ("overlay", "alive") => self.overlay_alive.store(now(), Ordering::SeqCst),
            ("overlay", "painted") => self.reveal_overlay(),
            ("overlay", "home") => { self.claim(); self.hide_overlay(); self.come_back(); }
            ("hhd", "open") => self.hhd_open(),
            ("resume", _) => { self.claim(); self.hide_overlay(); self.resume_game(); }
            ("power", _) => { self.hide_overlay(); return false; }   // handled like the main window's
            _ => return false,
        }
        true
    }

    /// Things the bridge asks of the windows. The bridge is busy answering while it asks, so anything that
    /// needs it again runs on its own thread.
    pub fn host_action(self: &Arc<Self>, what: &str, a: &Value) {
        if what != "screenshot" {
            let (me, what, a) = (self.clone(), what.to_string(), a.clone());
            std::thread::spawn(move || me.host_action_now(&what, &a));
            return;
        }
        self.host_action_now(what, a);
    }
    fn host_action_now(self: &Arc<Self>, what: &str, a: &Value) {
        match what {
            "focusApp" => {
                let (id, pid) = (a["id"].as_u64().unwrap_or(0), a["pid"].as_u64().unwrap_or(0));
                self.claim();
                self.hide_overlay();
                if pid == std::process::id() as u64 { return self.come_back(); }
                self.step_aside();
                if id == 0 && pid == 0 { let _ = hearth_core::steam::spawn(&hearth_core::paths::home(), "steam://open/main"); return self.host_action_now("showSteam", &json!({})); }
                // The running game: by its number (gamescope picks its best window); anything else: that window.
                if self.running_games_window(id) { self.resume_game(); return; }
                self.focus_xid("app", id, 0, false);
            }
            "screenshot" => {
                // HHD's overlay app: see-through for the picture, then back.
                if self.hhd() {
                    if self.overlay_on.load(Ordering::SeqCst) {
                        let opacity: u64 = if a["before"] == true { 0 } else { 1 };
                        hearth_core::overlay_link::send(&json!({"type": "overlay-host", "what": "opacity", "value": opacity}));
                        if a["before"] == true { std::thread::sleep(Duration::from_millis(450)); }
                    }
                    if a["before"] == true && self.main_visible() { self.step_aside(); std::thread::sleep(Duration::from_millis(450)); }
                    return;
                }
                if a["before"] == true {
                    if self.overlay_on.load(Ordering::SeqCst) { self.overlay_look(false, false); std::thread::sleep(Duration::from_millis(450)); }
                    if self.main_visible() { self.step_aside(); std::thread::sleep(Duration::from_millis(450)); }
                } else if self.overlay_on.load(Ordering::SeqCst) { self.overlay_look(true, true); }
            }
            // Steam's chat (or another Steam window) was asked for: Hearth steps aside once it's up.
            "showSteam" => {
                if !self.inside() { return; }
                self.claim();
                self.hide_overlay();
                let me = self.clone();
                // {wait: seconds}: Steam may still be starting (hidden) when its window is asked for.
                let tries = (a["wait"].as_u64().unwrap_or(14).min(300) * 10 / 7).max(20);
                std::thread::spawn(move || {
                    let mine = std::process::id() as u64;
                    for _ in 0..tries {
                        std::thread::sleep(Duration::from_millis(700));
                        let hit = gs::mapped_windows().into_iter().find(|w| w.pid != mine && w.w >= 300 && w.h >= 200 && matches!(std::fs::read_to_string(format!("/proc/{}/comm", w.pid)).unwrap_or_default().trim(), "steam" | "steamwebhelper"));
                        if let Some(w) = hit { me.step_aside(); me.focus_xid("steam window", w.win, 0, false); return; }
                    }
                    log("friends: Steam's window didn't show up");
                });
            }
            "closeGame" => { if let Ok(mut b) = self.bridge.lock() { b.library.close_running(); } }
            "comeBack" => { self.hide_overlay(); self.come_back(); }
            // Couch Co-op: the split screen (a window of the given process) in front, Hearth out of the way.
            "showPid" => {
                let pid = a["pid"].as_u64().unwrap_or(0) as u32;
                if pid == 0 || !self.inside() { return; }
                self.hide_overlay();
                self.step_aside();
                self.run_focus("couch co-op", 300, false, move |still| {
                    let mine = [std::process::id() as u64];
                    let want = gs::Want { app_id: 0, exclude_pids: &mine, not_wins: &[], other_ids: &[], known: 0 };
                    gs::focus_game(&want, &|| vec![pid], still);
                });
            }
            // Quick Resume: the game picked is running again; put it in front.
            "resumeGame" => { self.hide_overlay(); self.resume_game(); }
            _ => {}
        }
    }
}

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
//!
//! Quick Access over a game is always that overlay (Hearth's `?overlay=full` page), never the main window's panel:
//! also on Hearth's starting screen while a game or Steam loads (`open_from_hearth`), and while HHD is starting its
//! overlay app again (Hearth waits for it and asks HHD to start it: `open_or_wait`). Hearth's own Quick Access
//! over the game is only a last resort, logged as such, when the overlay can't be shown at all.
//!
//! While Quick Access is open over a game nothing puts the game back in front of it (`front_decision`), and Hearth's
//! own page doesn't act on the controller unless its window is what's on screen (`main_page_reads_pad`): it reads the
//! controller itself through the browser, and Chromium (the Electron window app) keeps doing that for a hidden window,
//! so a Guide press over a game also opened the main window's old panel behind the game, and the next press closed it
//! and sent "back to the game", which put the game over the overlay that had just opened.

use hearth_core::appid::{APPS_ID, HEARTH_ID};
use hearth_core::bridge::Bridge;
use hearth_core::gamescope as gs;
use hearth_core::procs;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A wait for Steam's window (showSteam) is under way: a second ask meanwhile isn't another one.
static STEAM_WINDOW_WAIT: AtomicBool = AtomicBool::new(false);
use crate::winhost::WindowHost;

pub const MAIN_TITLE: &str = "Hearth";
pub const OVERLAY_TITLE: &str = "Hearth Overlay";
const OPAQUE: u64 = 4294967295;
/// How long a Quick Access press waits for Handheld Daemon's overlay app to connect again: by itself, when HHD can't
/// be asked (it reconnects within a second when it's running); when HHD is asked to start it (after APP_ASK_AFTER_MS:
/// HHD's next pass, then Chromium starting, about a second on the ROG Ally); and when HHD has already been asked
/// during this time away and the app still isn't back (app_known_dead: waiting the whole time again on every press
/// was the "takes forever to open").
const APP_WAIT_MS: u64 = 3000;
const APP_ASK_AFTER_MS: u64 = 300;
const APP_RESTART_WAIT_MS: u64 = 6000;
const APP_DEAD_WAIT_MS: u64 = 2000;
/// After ending HHD's overlay app left from Desktop Mode: HHD notices it ended (it checks every half second), then is
/// asked to start it in gamescope.
const APP_AFTER_END_MS: u64 = 700;

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
    /// When the overlay last opened: a Guide tap (its release) right after it opened during that same press (Handheld
    /// Daemon's Guide + B, the Quick Access button) doesn't close it again.
    shown_at: AtomicU64,
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
    /// Quick Access was opened over Hearth's own starting screen (a game or Steam loading): it closes when the game
    /// comes up, and closing it leaves Hearth in front.
    over_launch: AtomicBool,
    /// A press waiting for HHD's overlay app to connect (its number; 0: none), and the last number given out.
    waiting: AtomicU64,
    wait_gen: AtomicU64,
    /// When a press that waited opened Quick Access in the app that just connected (it isn't closed as stale).
    opened_after_wait: AtomicU64,
    /// When Hearth last asked HHD to start the app on its own (not for a press): the open HHD sends it along is ignored.
    keeper_poked_at: AtomicU64,
    /// While HHD's overlay app is away: how many times HHD was asked to start it, and when first (app_known_dead).
    /// Both 0 again once it connects.
    app_asks: AtomicU64,
    app_first_ask: AtomicU64,
    /// Hearth set its own window aside for something else (a game, an app, a Steam window): its page isn't on screen.
    main_aside: AtomicBool,
    /// What Hearth's page was last told about reading the controller (hostScreen): 0 nothing yet, 1 it may, 2 it may not.
    pads_told: AtomicU64,
}

/// Who asks for something to come to the front.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontAsk {
    /// The running game (Hearth's own choices: it came up, the hand-back after Quick Access, the nudges after it).
    Game,
    /// Hearth's own page said "back to the game" (its panel closed, B on the starting screen).
    MainWindowResume,
    /// Hearth's own screen (a game ended, "Back to Hearth", the last resort).
    Hearth,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontAnswer {
    Go,
    /// Not now: Quick Access is open over the game, and the game would be drawn over it (or HHD's overlay hidden). The
    /// game comes back in front when Quick Access closes (hide_overlay hands it back).
    KeepQuickAccess,
}
/// Whether a request to put something in front goes ahead while Quick Access may be open over the game.
pub fn front_decision(ask: FrontAsk, quick_access_open: bool) -> FrontAnswer {
    match ask {
        FrontAsk::Hearth => FrontAnswer::Go,
        FrontAsk::Game | FrontAsk::MainWindowResume if quick_access_open => FrontAnswer::KeepQuickAccess,
        _ => FrontAnswer::Go,
    }
}
/// HHD has been asked to start its overlay app during this time away, and it still isn't back well after it should
/// be: a press then waits only briefly (APP_DEAD_WAIT_MS) before Hearth's own Quick Access, instead of the whole time.
pub fn app_known_dead(asks: u64, since_first_ask_ms: u64) -> bool { asks > 0 && since_first_ask_ms >= APP_RESTART_WAIT_MS }
/// How a Quick Access press waits for HHD's overlay app: (when HHD is asked to start it, how long in all before the
/// last resort), in ms from the press. `ended_desktop_app`: Hearth just ended HHD's overlay app left from Desktop Mode,
/// so HHD will start it in gamescope now (worth the normal wait even if it was given up on before).
pub fn app_wait_plan(known_dead: bool, ended_desktop_app: bool) -> (u64, u64) {
    if ended_desktop_app { return (APP_AFTER_END_MS, APP_AFTER_END_MS + APP_RESTART_WAIT_MS); }
    if known_dead { return (0, APP_DEAD_WAIT_MS); }
    (APP_ASK_AFTER_MS, APP_RESTART_WAIT_MS)
}
/// Whether Hearth's own page may act on the controller it reads itself (the browser's gamepad API): only while its
/// window is what's on screen and Quick Access (the overlay) isn't open over it.
pub fn main_page_reads_pad(main_aside: bool, quick_access_open: bool) -> bool { !main_aside && !quick_access_open }
/// Hearth set its window aside, but it's what's on screen now (the window in front of it closed by itself): it isn't
/// aside any more. Only ever gives the controller back, never takes it from Hearth's own screen.
pub fn still_aside(main_aside: bool, hearth_on_screen: bool) -> bool { main_aside && !hearth_on_screen }
/// Labels of focus changes that put Hearth's own window in front (everything else sets it aside).
fn focus_is_hearth(label: &str) -> bool { matches!(label, "hearth" | "back to starting screen") }

fn now() -> u64 { hearth_core::library::now_ms() }
fn log(t: &str) { hearth_core::bridge::log(t); }
fn ev(name: &str, data: Value) -> Value { json!({"type": "event", "name": name, "data": data}) }
fn ev_pad(b: &str) -> Value { ev("pad", json!({"b": b})) }

impl GameMode {
    pub fn new(host: Arc<dyn WindowHost>, bridge: Arc<Mutex<Bridge>>, guide: hearth_core::guide::Guide, windowed: bool) -> Arc<GameMode> {
        let gm = Arc::new(GameMode { host, bridge, windowed, guide, gen: AtomicU64::new(0), focus_lock: Mutex::new(()), ip_lock: Mutex::new(()), overlay_lock: Mutex::new(()), launch_gen: AtomicU64::new(0),
            overlay_on: AtomicBool::new(false), osd_on: AtomicBool::new(false), osd_gen: AtomicU64::new(0), look_closed: AtomicBool::new(true),
            before_overlay: AtomicU64::new(0), overlay_alive: AtomicU64::new(0), reveal_gen: AtomicU64::new(0), ip: AtomicBool::new(false), launch_busy: AtomicU64::new(0), last_guide: AtomicU64::new(0), shown_at: AtomicU64::new(0), b_gen: AtomicU64::new(0),
            repeat_gen: AtomicU64::new(0), last_pad: Mutex::new((String::new(), 0)), xids: Mutex::new(Default::default()), game_wins: Default::default(),
            hhd_overlay: AtomicBool::new(false), win_tags: Default::default(), ids_on: AtomicBool::new(false),
            over_launch: AtomicBool::new(false), waiting: AtomicU64::new(0), wait_gen: AtomicU64::new(0), opened_after_wait: AtomicU64::new(0),
            keeper_poked_at: AtomicU64::new(0), app_asks: AtomicU64::new(0), app_first_ask: AtomicU64::new(0), main_aside: AtomicBool::new(false), pads_told: AtomicU64::new(0) });
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
            if !focus_is_hearth(label) { me.set_aside(true); }
            f(&still);
        });
    }
    /// Hearth's window set aside (or back on screen), and its page told whether it may act on the controller.
    fn set_aside(&self, aside: bool) {
        self.main_aside.store(aside, Ordering::SeqCst);
        self.sync_main_pads();
    }
    /// Tells Hearth's page whether it may act on the controller it reads itself (when that changes).
    pub fn sync_main_pads(&self) {
        if !self.inside() { return; }
        let on = main_page_reads_pad(self.main_aside.load(Ordering::SeqCst), self.overlay_on.load(Ordering::SeqCst));
        let code = if on { 1 } else { 2 };
        if self.pads_told.swap(code, Ordering::SeqCst) != code { self.to_main(ev("hostScreen", json!({"pads": on}))); }
    }
    /// Hearth's page (re)loaded: it's told again.
    pub fn main_ready(&self) { self.pads_told.store(0, Ordering::SeqCst); self.sync_main_pads(); }
    pub fn focus_xid(self: &Arc<Self>, label: &'static str, win: u64, delay_ms: u64, soft: bool) {
        if win == 0 { return; }
        self.run_focus(label, delay_ms, soft, move |_| gs::focus_window(win));
    }

    /// Hearth back in front (a game ended, Guide was pressed...): Hearth's number.
    pub fn come_back(self: &Arc<Self>) {
        self.claim();
        self.set_aside(false);
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
    pub fn step_aside(&self) { if !self.windowed && self.host.exists("main") { self.host.hide("main"); self.set_aside(true); } }

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
        let gm = self.clone();
        self.run_focus("game", delay_ms, soft, move |still| {
            // Quick Access open over it: the game stays behind (hide_overlay hands it back when it closes).
            if front_decision(FrontAsk::Game, gm.overlay_on.load(Ordering::SeqCst)) == FrontAnswer::KeepQuickAccess {
                log(&format!("focus: {id} stays behind Quick Access (open); it comes back in front when Quick Access closes"));
                return;
            }
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
                // Hearth's window set aside but on screen again (what was in front of it closed by itself): its page
                // gets the controller back.
                if me.main_aside.load(Ordering::SeqCst) && !still_aside(true, me.hearth_on_screen()) { me.set_aside(false); }
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
            // Quick Access opened over the starting screen closes as the game comes up (as it did in Hearth's window).
            if me.over_launch.load(Ordering::SeqCst) && me.overlay_on.load(Ordering::SeqCst) { log("overlay: the game came up; closing Quick Access opened while it started"); me.hide_overlay(); }
            me.step_aside();
            match r {
                Some(gs::Ready::Window(w)) => {
                    log(&format!("launch: {id} is ready (window {w:#x}, id {app_number}, {})", procs::describe(&pids())));
                    { let mut m = me.game_wins.lock().unwrap(); m.retain(|_, v| *v != w); m.insert(id.clone(), w); }
                    if front_decision(FrontAsk::Game, me.overlay_on.load(Ordering::SeqCst)) == FrontAnswer::KeepQuickAccess {
                        log(&format!("focus: {id} stays behind Quick Access (open); it comes in front when Quick Access closes"));
                    } else if app_number != 0 {
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

    /// Hearth's own page asked to go back to the game (its panel closed, B on the starting screen). Not while Quick
    /// Access is open over the game: that's the overlay's to decide (it was the hidden page acting on a Guide press
    /// meant for the overlay).
    pub fn resume_from_main(self: &Arc<Self>) {
        if front_decision(FrontAsk::MainWindowResume, self.overlay_on.load(Ordering::SeqCst)) == FrontAnswer::KeepQuickAccess {
            log("focus: Hearth's window asked to go back to the game while Quick Access is open over it; ignored");
            return;
        }
        self.resume_game();
    }
    /// Quick Access closes for something else (an app switch, Steam, Back to Hearth...): said in the log, so a close
    /// nobody asked for can be traced.
    fn close_for(self: &Arc<Self>, why: &str) {
        if self.overlay_on.load(Ordering::SeqCst) { log(&format!("overlay: closing Quick Access ({why})")); }
        self.hide_overlay();
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
        self.shown_at.store(now(), Ordering::SeqCst);
        self.osd_on.store(false, Ordering::SeqCst);
        self.sync_main_pads();
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
    fn hold_game(self: &Arc<Self>, on: bool) {
        // Hearth Input (a game's controller layout) lets go of the controller before Hearth takes it, and gets it
        // back only after Hearth has let go (input_map.rs: it answers once it has).
        if on { hearth_core::input_map::overlay(true); }
        // With HHD's overlay app, HHD holds the controllers (the app asks it to while Quick Access is open), and
        // Steam has to keep running under it (steam_while_open).
        if !self.hhd() { self.guide.grab(on); } else { self.steam_while_open(on); }
        if self.ip.load(Ordering::SeqCst) {
            let _ = on; self.ip_sync();
        }
        if !on { hearth_core::input_map::overlay(false); }
    }

    /// Quick Access over a game on the HHD editions: HHD stops Steam (SIGSTOP) while it holds the controllers, and a
    /// Steam game froze with it ("Steam games still freezing when opening the in-game overlay"; see
    /// hearth_core::steam_thaw). Steam is let go again as soon as HHD stops it, for as long as Quick Access is open
    /// (HHD stops it again on every new hold, e.g. after a screenshot's peek), and once more after it closes (HHD's
    /// own SIGCONT never comes if its overlay thread ends meanwhile). Steam's virtual controllers are held by Hearth
    /// meanwhile, so the game still gets none of the presses meant for Quick Access; they're let go a moment after
    /// it closes (the B that closed it isn't passed on, as HHD's own 0.25 s wait does).
    fn steam_while_open(self: &Arc<Self>, on: bool) {
        if on {
            self.guide.hold_steam_pads(true);
            let me = self.clone();
            let opened = self.shown_at.load(Ordering::SeqCst);
            std::thread::spawn(move || {
                let root = std::path::Path::new("/proc");
                let uid = hearth_core::steam_thaw::uid();
                let mut steam: Vec<u32> = Vec::new();
                let mut told = false;
                let mut tick = 0u64;
                while me.overlay_on.load(Ordering::SeqCst) && me.shown_at.load(Ordering::SeqCst) == opened {
                    // Steam's processes looked up once a second; whether they're stopped every 50 ms at first (HHD
                    // stops Steam right as it takes the controllers), then every 250 ms.
                    if tick % 20 == 0 { steam = hearth_core::steam_thaw::steam_processes(root, uid); }
                    let stopped = hearth_core::steam_thaw::stopped(root, &steam);
                    if !stopped.is_empty() {
                        let gone = hearth_core::steam_thaw::let_go(&stopped);
                        if !told || gone.len() != stopped.len() {
                            told = true;
                            log(&format!("overlay: Handheld Daemon stopped Steam while Quick Access is open (process {stopped:?}); a Steam game freezes with it, so Steam was let go again ({} of {}); Steam's virtual controllers are held instead", gone.len(), stopped.len()));
                        }
                    }
                    tick += 1;
                    std::thread::sleep(Duration::from_millis(if tick < 60 { 50 } else { 250 }));
                }
            });
        } else {
            let me = self.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(300));
                if !me.overlay_on.load(Ordering::SeqCst) { me.guide.hold_steam_pads(false); }
                // HHD lets Steam go itself 0.25 s after it gives the controllers back; if it didn't (its overlay thread
                // ended, or it stopped Steam again), Hearth does.
                for wait in [400u64, 1500] {
                    std::thread::sleep(Duration::from_millis(wait));
                    if me.overlay_on.load(Ordering::SeqCst) { return; }
                    me.let_steam_run("after Quick Access closed");
                }
            });
        }
    }
    /// Steam stopped (by HHD) with Quick Access closed: let go, and said in the log.
    fn let_steam_run(&self, when: &str) {
        let gone = hearth_core::steam_thaw::let_steam_run();
        if !gone.is_empty() { log(&format!("overlay: Steam was still stopped {when} (process {gone:?}; Handheld Daemon stops it while Quick Access holds the controllers); let go again")); }
        if self.guide.steam_pads_held() && !self.overlay_on.load(Ordering::SeqCst) { self.guide.hold_steam_pads(false); }
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
            self.over_launch.store(false, Ordering::SeqCst);
            return;
        }
        self.overlay_on.store(false, Ordering::SeqCst);
        self.sync_main_pads();
        let from_launch = self.over_launch.swap(false, Ordering::SeqCst);
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
        // The controller back to what was on screen. Soft: anything chosen meanwhile wins. Opened over Hearth's
        // starting screen: Hearth was on screen, and stays (the game isn't up yet).
        if self.inside() && !from_launch && !self.main_focused() {
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
        if how == "guide-tap" && self.overlay_on.load(Ordering::SeqCst) && t.saturating_sub(self.shown_at.load(Ordering::SeqCst)) < 1000 {
            log("overlay: Guide let go just after Quick Access opened (Guide + B, the Quick Access button); leaving it open");
            return;
        }
        if t.saturating_sub(self.last_guide.swap(t, Ordering::SeqCst)) < 400 { self.ip_sync(); return; }
        if self.overlay_on.load(Ordering::SeqCst) { log(&format!("overlay: {how} pressed, closing")); self.hide_overlay(); return; }
        // Pressed again while waiting for HHD's overlay app: you changed your mind.
        if self.waiting.swap(0, Ordering::SeqCst) != 0 { log(&format!("overlay: {how} pressed again while waiting for Handheld Daemon's overlay app; not opening")); self.ip_sync(); return; }
        if self.hearth_on_screen() {
            // Hearth's own UI handles Guide itself: InputPlumber caught it, so pass it on and give the controller back.
            if self.ip.load(Ordering::SeqCst) { self.ip_sync(); self.to_main(ev("pad", json!({"b": "guide"}))); }
            return;
        }
        // A short press over a game (Settings › Controls › Quick glance): a thin bar with the time, battery, FPS and
        // the latest message for a few seconds, drawn like the volume bar (the overlay on screen without taking
        // input, so the game keeps its controller and isn't paused). A hold (or anything but a game) opens Quick Access.
        if how == "guide-tap" && self.running().is_some() && self.overlay_ready() {
            log("overlay: Guide tapped over a game: the glance");
            self.ip_sync();
            self.osd(json!({"kind": "glance", "ms": 3000}));
            return;
        }
        log(&format!("overlay: {how} pressed over another app"));
        self.open_or_wait(&format!("{how} pressed"), |me| {
            me.ip_sync();
            me.come_back();
            me.to_main(ev("pad", json!({"b": "guide"})));
        });
    }

    /// Quick Access over what's on screen. With Handheld Daemon's overlay app not connected (HHD stopped it and is
    /// starting it again, or Hearth just restarted), the press waits for it and HHD is asked to start it
    /// (app_wait_plan: up to APP_RESTART_WAIT_MS; only APP_DEAD_WAIT_MS when HHD was already asked and it never came
    /// back), after ending the one HHD may still run for Desktop Mode: the main window's Quick Access, which isn't the
    /// overlay and doesn't behave like it over a game, is not shown for a moment's disconnection. `fallback` runs only
    /// when the overlay really couldn't be shown, and that's logged loudly; it opens Hearth's own Quick Access, and
    /// closing that goes back to the game (the page sends "resume" when its Quick Access closes with a game running:
    /// ui/index.html resumeIfGame, resume_from_main).
    fn open_or_wait(self: &Arc<Self>, what: &str, fallback: impl FnOnce(&Arc<GameMode>) + Send + 'static) {
        if self.show_overlay() { return; }
        if !self.hhd() {
            log(&format!("overlay: LAST RESORT ({what}): Hearth's overlay window isn't there; showing Hearth's own Quick Access instead"));
            fallback(self);
            return;
        }
        self.ip_sync();
        let g = self.wait_gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.waiting.store(g, Ordering::SeqCst);
        log(&format!("overlay: {what}, but Handheld Daemon's overlay app isn't connected; waiting for it"));
        let me = self.clone();
        let what = what.to_string();
        std::thread::spawn(move || {
            let start = now();
            // HHD sends every press to the overlay app it started for Desktop Mode for as long as that one runs: it
            // has to end before HHD starts the app in gamescope.
            let ended = me.end_desktop_app("Quick Access was asked for");
            let dead = app_known_dead(me.app_asks.load(Ordering::SeqCst), start.saturating_sub(me.app_first_ask.load(Ordering::SeqCst)));
            let (ask_at, wait) = app_wait_plan(dead, ended);
            if dead && !ended {
                log(&format!("overlay: Handheld Daemon was already asked {} time(s) to start its overlay app and it hasn't come back; asking once more, waiting {} s at most", me.app_asks.load(Ordering::SeqCst), wait / 1000));
            }
            let mut limit = wait.min(APP_WAIT_MS.max(ask_at));
            let mut asked = false;
            let mut hhd_answered = false;
            let mut seen = 0u64;
            loop {
                if me.waiting.load(Ordering::SeqCst) != g { return; }   // pressed again, or a newer press waits
                // Opened meanwhile (HHD's own press reached Hearth through the app once it was back).
                if me.overlay_on.load(Ordering::SeqCst) { let _ = me.waiting.compare_exchange(g, 0, Ordering::SeqCst, Ordering::SeqCst); return; }
                let t = now().saturating_sub(start);
                if hearth_core::overlay_link::connected() {
                    // A moment for the app's own start (it begins closed, and Hearth's idea of it does too).
                    if seen == 0 { seen = now(); }
                    if now().saturating_sub(seen) >= 300 {
                        if me.waiting.compare_exchange(g, 0, Ordering::SeqCst, Ordering::SeqCst).is_err() { return; }
                        me.opened_after_wait.store(now(), Ordering::SeqCst);
                        if me.show_overlay() { log(&format!("overlay: Handheld Daemon's overlay app is back after {t} ms; Quick Access open ({what})")); return; }
                        break;
                    }
                } else { seen = 0; }
                if !asked && t >= ask_at {
                    asked = true;
                    me.note_app_ask();
                    match hearth_core::hhd::start_overlay_app() {
                        Ok(()) => { hhd_answered = true; limit = wait; log("overlay: asked Handheld Daemon to start its overlay app"); }
                        Err(e) => log(&format!("overlay: couldn't ask Handheld Daemon to start its overlay app: {e}")),
                    }
                }
                if t >= limit {
                    if me.waiting.compare_exchange(g, 0, Ordering::SeqCst, Ordering::SeqCst).is_err() { return; }
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            let secs = now().saturating_sub(start) as f64 / 1000.0;
            let why = if !hhd_answered { "Handheld Daemon couldn't be asked to start it".to_string() }
                else if !hearth_core::overlay_link::desktop_mode_overlays().is_empty() { "HHD's overlay app is still running for Desktop Mode and didn't end".to_string() }
                else { "HHD was asked to start it and it didn't connect".to_string() };
            log(&format!("overlay: LAST RESORT ({what}): Handheld Daemon's overlay app didn't come back in {secs:.1} s ({why}); showing Hearth's own Quick Access instead, which goes back to the game when it closes. Why it isn't running is in HHD's log (journalctl -u 'hhd*')"));
            fallback(&me);
        });
    }

    /// HHD was asked to start its overlay app (by a press or the keeper): counted for app_known_dead.
    fn note_app_ask(&self) {
        if self.app_asks.fetch_add(1, Ordering::SeqCst) == 0 { self.app_first_ask.store(now(), Ordering::SeqCst); }
    }
    /// Ends HHD's overlay app left running for Desktop Mode (overlay_link::end_desktop_mode_overlays), logged with
    /// why; answers whether there was one. The app ends itself when it sees Game Mode; this is for when it doesn't.
    fn end_desktop_app(&self, why: &str) -> bool {
        let pids = hearth_core::overlay_link::end_desktop_mode_overlays();
        if pids.is_empty() { return false; }
        log(&format!("overlay: {why}, but Handheld Daemon's overlay app is still running for Desktop Mode (process {pids:?}), so HHD sends Quick Access there and not over the game; ended it so HHD starts it in gamescope"));
        // A fresh start in gamescope: earlier requests that went to the Desktop Mode one don't count against it.
        self.app_asks.store(0, Ordering::SeqCst);
        true
    }

    /// Hearth's own screen asked for Quick Access (Guide or the Quick Access button on the starting screen, while a
    /// game or Steam is loading behind it): the overlay, the same Quick Access as over the game, not the main
    /// window's panel. The page opens its own only if Hearth answers overlay.unavailable (no overlay here at all).
    pub fn open_from_hearth(self: &Arc<Self>) {
        if !self.overlay_available() { self.to_main(ev("overlay.unavailable", json!({}))); return; }
        if self.overlay_on.load(Ordering::SeqCst) || self.waiting.load(Ordering::SeqCst) != 0 { return; }
        log("overlay: Quick Access asked for on Hearth's starting screen (a game or Steam is loading)");
        self.over_launch.store(true, Ordering::SeqCst);
        self.open_or_wait("Quick Access on the starting screen", |me| {
            me.over_launch.store(false, Ordering::SeqCst);
            me.to_main(ev("overlay.unavailable", json!({})));
        });
    }

    /// Handheld Daemon's overlay app away for a while (HHD stopped it after an error in its window handling and only
    /// starts it again on its next settings pass): HHD is asked to start it now, so Quick Access is ready before the
    /// next press. At most every 30 s and 3 times in 10 minutes, so an app that can't start isn't asked for forever.
    /// Away because HHD still runs the one it started for Desktop Mode (back from Desktop Mode, or Game Mode
    /// restarted): HHD keeps sending everything to that one (hhd/plugins/overlay/base.py OverlayService: `started`
    /// stays set until it ends) and none of the requests above could help, so it's ended first (end_desktop_app).
    pub fn start_overlay_keeper(self: &Arc<Self>) {
        let me = self.clone();
        std::thread::spawn(move || {
            let mut away_since = 0u64;
            let mut asked: Vec<u64> = Vec::new();
            let mut told_dead = false;
            let mut tick = 0u64;
            loop {
                if !me.hhd() { return; }
                if hearth_core::overlay_link::connected() {
                    away_since = 0; told_dead = false;
                    me.app_asks.store(0, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                }
                let t = now();
                // Game Mode (re)started with HHD's overlay app still running for the Desktop Mode before it (HHD sends
                // it every press until it ends): ended right away, at start and every 2 s while away, so HHD starts
                // the app in gamescope on its own (within a few seconds) and the first press opens at once.
                // HHD is asked right after (once it has seen that app end) instead of waiting for its own next pass;
                // the open it sends along is ignored (hhd_open).
                if tick % 2 == 0 && me.waiting.load(Ordering::SeqCst) == 0 && me.end_desktop_app("Game Mode is running") {
                    away_since = t; asked.clear(); told_dead = false;
                    std::thread::sleep(Duration::from_millis(APP_AFTER_END_MS));
                    if !hearth_core::overlay_link::connected() && me.waiting.load(Ordering::SeqCst) == 0 {
                        let t = now();
                        asked.push(t);
                        me.keeper_poked_at.store(t, Ordering::SeqCst);
                        me.note_app_ask();
                        match hearth_core::hhd::start_overlay_app() {
                            Ok(()) => log("overlay: asked Handheld Daemon to start its overlay app in gamescope"),
                            Err(e) => log(&format!("overlay: couldn't ask Handheld Daemon to start its overlay app: {e}")),
                        }
                    }
                }
                tick += 1;
                if away_since == 0 {
                    away_since = t;
                    // HHD's overlay thread ended (the app crashed, or gamescope did): if Steam was stopped for Quick
                    // Access then, HHD never lets it go, and Steam and its game stay frozen.
                    me.let_steam_run("when Handheld Daemon's overlay app went away");
                }
                std::thread::sleep(Duration::from_secs(1));
                let t = now();
                if t.saturating_sub(away_since) < 6000 || me.waiting.load(Ordering::SeqCst) != 0 { continue; }
                asked.retain(|a| t.saturating_sub(*a) < 600_000);
                if asked.len() >= 3 || asked.last().map(|a| t.saturating_sub(*a) < 30_000).unwrap_or(false) {
                    if asked.len() >= 3 && !told_dead {
                        told_dead = true;
                        log("overlay: Handheld Daemon was asked 3 times to start its overlay app and it hasn't connected; not asking again for 10 minutes (a Quick Access press still asks). HHD's log says why (journalctl -u 'hhd*')");
                    }
                    continue;
                }
                asked.push(t);
                me.keeper_poked_at.store(t, Ordering::SeqCst);
                me.note_app_ask();
                match hearth_core::hhd::start_overlay_app() {
                    Ok(()) => log(&format!("overlay: Handheld Daemon's overlay app has been away for {} s; asked HHD to start it again", t.saturating_sub(away_since) / 1000)),
                    Err(e) => log(&format!("overlay: Handheld Daemon's overlay app is away and HHD couldn't be asked to start it: {e}")),
                }
            }
        });
    }

    /// Handheld Daemon's QAM button (its overlay app asks): Quick Access, like Guide. Over a game it opens there; with
    /// Hearth in front, Hearth's own Quick Access opens (as Guide does). Already open: the app is told again (it may
    /// have restarted). Closing goes through the page's own close (from_overlay).
    /// `late`: pressed while the app wasn't connected, passed on now that it is. One that came with Hearth's own request
    /// to HHD to start the app (start_overlay_keeper: HHD sends an open along) isn't a press, and is ignored.
    pub fn hhd_open(self: &Arc<Self>, late: bool) {
        if late && now().saturating_sub(self.keeper_poked_at.load(Ordering::SeqCst)) < 20_000 && self.waiting.load(Ordering::SeqCst) == 0 {
            log("overlay: Handheld Daemon's overlay app started (Hearth asked for it); not opening");
            return;
        }
        if self.overlay_on.load(Ordering::SeqCst) { self.to_overlay(ev("overlay.shown", json!({}))); return; }
        if self.hearth_on_screen() { self.to_main(ev("pad", json!({"b": "guide"}))); return; }
        log("overlay: Handheld Daemon's Quick Access button pressed over another app");
        self.open_or_wait("Handheld Daemon's Quick Access button pressed", |me| {
            me.come_back();
            me.to_main(ev("pad", json!({"b": "guide"})));
        });
    }
    /// HHD (re)started its overlay app: it starts closed, so Hearth's idea of it does too (unless a press that was
    /// waiting for it has just opened Quick Access in it).
    pub fn overlay_app_connected(self: &Arc<Self>) {
        log("overlay: Handheld Daemon's overlay app connected");
        if self.waiting.load(Ordering::SeqCst) != 0 || now().saturating_sub(self.opened_after_wait.load(Ordering::SeqCst)) < 3000 { return; }
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
                    // Real sleep through logind (like SteamOS); a game running stays as it is through it. A refusal
                    // (something holding sleep off) is said on screen and in the log instead of nothing happening.
                    "sleep" => if let Err(e) = hearth_core::power::sleep_now("power button") { self.to_main(json!({"type": "toast", "text": e})); },
                    // Quick Resume games are kept through turning off when the PC can (the power menu's Shut down too).
                    "shutdown" => {
                        let keep = self.bridge.lock().map(|b| b.library.games_to_keep()).unwrap_or(false);
                        let verb = hearth_core::power::shutdown_verb(keep);
                        log(&format!("power button: turning off ({verb})"));
                        let _ = hearth_core::power::action(verb, gs::in_session());
                    }
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
            ("overlay", "close") => self.close_for("its page closed it: B, Guide or the panel emptied"),
            ("overlay", "alive") => self.overlay_alive.store(now(), Ordering::SeqCst),
            ("overlay", "painted") => self.reveal_overlay(),
            ("overlay", "home") => {
                self.claim(); self.close_for("Back to Hearth"); self.come_back();
                // A page of Hearth's to open there (Quick Access over a game: Game settings, Edit this layout, Screenshots).
                if m["open"].is_object() { self.to_main(ev("open", m["open"].clone())); }
            }
            ("hhd", "open") => self.hhd_open(m["late"] == true),
            // The page (re)loaded: HHD's overlay app loads it each time it connects, maybe after Quick Access was
            // opened (a press that waited for the app): it's told again, and gets its 5 s to answer from now.
            ("ready", _) => {
                if self.hhd() && self.overlay_on.load(Ordering::SeqCst) {
                    self.overlay_alive.store(now(), Ordering::SeqCst);
                    self.to_overlay(ev("padHost", json!({"connected": 1})));
                    self.to_overlay(ev("overlay.shown", json!({})));
                }
                return false;
            }
            ("resume", _) => { self.claim(); self.close_for("Back to game"); self.resume_game(); }
            ("power", _) => { self.close_for("power menu"); return false; }   // handled like the main window's
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
                self.close_for("switching apps");
                if pid == std::process::id() as u64 { return self.come_back(); }
                self.step_aside();
                if id == 0 && pid == 0 { let _ = hearth_core::steam::spawn(&hearth_core::paths::home(), "steam://open/main"); return self.host_action_now("showSteam", &json!({"why": "Switch app › Steam"})); }
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
                // Only ever on something you asked for (who: a["why"]), and one at a time.
                let why = a["why"].as_str().unwrap_or("not said").to_string();
                if STEAM_WINDOW_WAIT.swap(true, Ordering::SeqCst) { log(&format!("friends: Steam's window asked for again ({why}) while the last ask still waits; not asking twice")); return; }
                log(&format!("friends: Steam's window asked for ({why})"));
                self.claim();
                self.close_for("a Steam window");
                let me = self.clone();
                // {wait: seconds}: Steam may still be starting (hidden) when its window is asked for.
                let tries = (a["wait"].as_u64().unwrap_or(14).min(300) * 10 / 7).max(20);
                std::thread::spawn(move || {
                    let mine = std::process::id() as u64;
                    for _ in 0..tries {
                        std::thread::sleep(Duration::from_millis(700));
                        let hit = gs::mapped_windows().into_iter().find(|w| w.pid != mine && w.w >= 300 && w.h >= 200 && matches!(std::fs::read_to_string(format!("/proc/{}/comm", w.pid)).unwrap_or_default().trim(), "steam" | "steamwebhelper"));
                        if let Some(w) = hit { STEAM_WINDOW_WAIT.store(false, Ordering::SeqCst); me.step_aside(); me.focus_xid("steam window", w.win, 0, false); return; }
                    }
                    STEAM_WINDOW_WAIT.store(false, Ordering::SeqCst);
                    log("friends: Steam's window didn't show up");
                });
            }
            "closeGame" => { if let Ok(mut b) = self.bridge.lock() { b.library.close_running(); } }
            "comeBack" => { self.close_for("Hearth back in front"); self.come_back(); }
            // Couch Co-op: the split screen (a window of the given process) in front, Hearth out of the way.
            "showPid" => {
                let pid = a["pid"].as_u64().unwrap_or(0) as u32;
                if pid == 0 || !self.inside() { return; }
                self.close_for("Couch Co-op");
                self.step_aside();
                self.run_focus("couch co-op", 300, false, move |still| {
                    let mine = [std::process::id() as u64];
                    let want = gs::Want { app_id: 0, exclude_pids: &mine, not_wins: &[], other_ids: &[], known: 0 };
                    gs::focus_game(&want, &|| vec![pid], still);
                });
            }
            // Quick Resume: the game picked is running again; put it in front.
            "resumeGame" => { self.close_for("a game resumed"); self.resume_game(); }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_waits_less_for_an_overlay_app_known_to_be_gone() {
        // Not asked yet this time away, or asked only a moment ago: the normal wait (HHD asked almost at once).
        assert!(!app_known_dead(0, 0));
        assert!(!app_known_dead(0, 3_600_000));
        assert!(!app_known_dead(2, APP_RESTART_WAIT_MS - 1));
        // Asked, and well past the time it takes to start: known to be gone.
        assert!(app_known_dead(1, APP_RESTART_WAIT_MS));
        assert!(app_known_dead(3, 10_206_000));
        let (ask, wait) = app_wait_plan(false, false);
        assert_eq!((ask, wait), (APP_ASK_AFTER_MS, APP_RESTART_WAIT_MS));
        assert!(wait < 9000, "the 9 s wait per press was the slow open");
        let (ask, wait) = app_wait_plan(true, false);
        assert_eq!(ask, 0);
        assert!(wait <= 2000);
        // HHD's overlay app left from Desktop Mode was just ended: HHD starts it in gamescope now, worth waiting for,
        // and asked only once HHD has seen it end.
        for dead in [false, true] {
            let (ask, wait) = app_wait_plan(dead, true);
            assert!(ask >= 500 && ask < 1000);
            assert!(wait >= ask + APP_RESTART_WAIT_MS);
        }
    }

    #[test]
    fn the_report_from_the_ally_waits_less() {
        // 2026-10-24: the app had been away for 10206 s with the keeper asking every 10 minutes; every Guide press
        // still waited 9 s before Hearth's own Quick Access covered the game.
        let first_ask = 0u64;
        let press_at = 10_206_000u64;
        let (_, wait) = app_wait_plan(app_known_dead(30, press_at - first_ask), false);
        assert!(wait <= 2000);
    }

    #[test]
    fn the_game_stays_behind_open_quick_access() {
        // Quick Access open over the game: neither Hearth's own choices (the nudges, the game coming up) nor its page's
        // "back to the game" put the game over it.
        assert_eq!(front_decision(FrontAsk::Game, true), FrontAnswer::KeepQuickAccess);
        assert_eq!(front_decision(FrontAsk::MainWindowResume, true), FrontAnswer::KeepQuickAccess);
        // Closed: the game comes to the front as before.
        assert_eq!(front_decision(FrontAsk::Game, false), FrontAnswer::Go);
        assert_eq!(front_decision(FrontAsk::MainWindowResume, false), FrontAnswer::Go);
        // Hearth's own screen always may (a game ended, Back to Hearth, the last resort).
        assert_eq!(front_decision(FrontAsk::Hearth, true), FrontAnswer::Go);
        assert_eq!(front_decision(FrontAsk::Hearth, false), FrontAnswer::Go);
    }

    #[test]
    fn hearths_page_reads_the_controller_only_on_screen() {
        assert!(main_page_reads_pad(false, false));             // Hearth's screen in front
        assert!(!main_page_reads_pad(true, false));             // a game in front (Hearth's window hidden)
        assert!(!main_page_reads_pad(false, true));             // Quick Access open over the starting screen
        assert!(!main_page_reads_pad(true, true));              // Quick Access open over a game
    }

    #[test]
    fn hearth_on_screen_again_gets_the_controller_back() {
        assert!(still_aside(true, false));
        assert!(!still_aside(true, true));                      // what was in front of it closed by itself
        assert!(!still_aside(false, false));                    // never takes the controller from Hearth's screen
        assert!(!still_aside(false, true));
    }

    #[test]
    fn only_hearths_own_focus_changes_keep_its_page_on_screen() {
        for l in ["hearth", "back to starting screen"] { assert!(focus_is_hearth(l), "{l}"); }
        for l in ["game", "game window", "app window", "app", "window", "back from the overlay", "stream", "steam window", "Steam question", "couch co-op"] {
            assert!(!focus_is_hearth(l), "{l}");
        }
    }

    /// The owner's log (Hearth 0.6.0, Electron window, Handheld Daemon's overlay app; Guide over a GOG game): each press
    /// opened Quick Access, the hidden main page toggled its own panel on the same press, and on every other press it
    /// sent "back to the game", which put the game over the overlay at once ("focus: ... to the front" 8 ms after the
    /// press) and again 1.2 s and 3 s later. Replayed through the decisions: the overlay keeps the front until it closes.
    #[test]
    fn owners_guide_presses_replayed() {
        let mut aside = true;           // the game is up: Hearth's window was set aside
        let mut open = false;
        let mut game_put_over_quick_access = 0;
        let mut main_page_acted = 0;
        for press in 0..4 {
            // Guide: Hearth opens the overlay (show_overlay).
            open = true;
            // The main page reads the same press through the browser: it may act only if told it may.
            if main_page_reads_pad(aside, open) { main_page_acted += 1; }
            // What the old page did on every other press: "back to the game"; and Hearth's nudges after it.
            if press % 2 == 1 && front_decision(FrontAsk::MainWindowResume, open) == FrontAnswer::Go { game_put_over_quick_access += 1; }
            for _nudge in [1200, 3000] { if front_decision(FrontAsk::Game, open) == FrontAnswer::Go { game_put_over_quick_access += 1; } }
            // B in Quick Access: it closes (hide_overlay), and the game is handed back.
            open = false;
            assert_eq!(front_decision(FrontAsk::Game, open), FrontAnswer::Go);
            aside = still_aside(aside, false);
        }
        assert_eq!(main_page_acted, 0, "Hearth's hidden page acted on Guide meant for the overlay");
        assert_eq!(game_put_over_quick_access, 0, "the game was put in front of open Quick Access");
        // Back to Hearth (come_back): its page reads the controller again.
        assert!(main_page_reads_pad(false, open));
    }
}

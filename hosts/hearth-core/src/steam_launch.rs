//! Steam games in Game Mode: fast to start and never stuck behind something you can't see (docs/STEAM.md).
//!
//! - Steam kept ready: with "Keep Steam ready in the background" (Settings › Library, on by default) Hearth starts the
//!   Steam client hidden (-silent) a little after Game Mode comes up, when Steam has been used on this device before,
//!   and doesn't close it when idle. Play then hands the game to a Steam that's already running instead of waiting
//!   for a cold start (often a minute or more with the Flatpak, longer when Steam updates itself).
//! - The starting screen says what's going on: "Starting Steam…", "Steam is updating…", "Steam is asking
//!   something…", "Starting the game…" (`watch`, the page's "steam.launch" event).
//! - Steam's windows while a Steam game starts or runs (a cloud-save conflict, an EULA, an error, an install step)
//!   come in front (gamemode.rs `tag_windows`), and the controller mouse turns on by itself over them and over a
//!   game's own launcher window (Bethesda's, for one): `pointer_wanted`.
//!
//! Low priority: Steam isn't started with `nice`, since every game Steam starts later inherits its priority.

use serde_json::json;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Start Steam hidden when Game Mode comes up? Only in Game Mode, with the setting on (or the older "only when
/// needed" turned off with "run in the background" on), Steam here and used before, and not already running.
pub fn warm_start(keep_ready: bool, on_demand: bool, background: bool, game_mode: bool, installed: bool, used_before: bool, running: bool) -> bool {
    if running || !installed { return false; }
    if keep_ready { return game_mode && used_before; }
    !on_demand && background
}

/// Is Valve's client kept ready (pure, for the tests)? The setting, except while Hearth's own Steam client (Open) is
/// in use and signed in: then Valve's client starts only when a game needs it (Steam DRM, VAC, Steamworks) and the
/// Open engine carries on once it has closed. Not signed in to Open, Classic or Lightweight: the setting as it is.
pub fn keep_ready_effective(setting: bool, mode: crate::steam_engine::Mode, open_signed_in: bool) -> bool {
    setting && !(mode == crate::steam_engine::Mode::Open && open_signed_in)
}

/// May Hearth close an idle Steam it started? Not while it's kept ready.
pub fn may_close_idle(on_demand: bool, keep_ready: bool) -> bool { on_demand && !keep_ready }

/// Has Steam been used on this device (a Steam folder with an account in it)?
pub fn used_before(home: &Path) -> bool {
    crate::steam::steam_path(home).map(|s| crate::steam::login_users(&s).map(|u| !u.is_empty()).unwrap_or(false)).unwrap_or(false)
}

/// What the starting screen says while a Steam game starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stage { Starting, Updating, Asking, Game }
impl Stage {
    pub fn name(self) -> &'static str { match self { Stage::Starting => "starting", Stage::Updating => "updating", Stage::Asking => "asking", Stage::Game => "game" } }
    pub fn text(self) -> &'static str {
        match self {
            Stage::Starting => "Starting Steam…",
            Stage::Updating => "Steam is updating…",
            Stage::Asking => "Steam is asking something. It's in front: use the right stick and A",
            Stage::Game => "Starting the game…",
        }
    }
}

/// The stage from what Hearth sees: the game's processes up wins, then a Steam question in front, then Steam
/// updating itself, then Steam running (it has the game now) or not yet.
pub fn stage(steam_running: bool, updating: bool, asking: bool, game_up: bool) -> Stage {
    if game_up { Stage::Game }
    else if asking { Stage::Asking }
    else if updating { Stage::Updating }
    else if steam_running { Stage::Game }
    else { Stage::Starting }
}

/// Which new window counts as part of a Steam game's launch (brought in front over Hearth's starting screen).
#[derive(Debug, Clone, Default)]
pub struct NewWin { pub steam_client: bool, pub title: String, pub w: u32, pub h: u32, pub owner: Option<u32>, pub game: u32 }

/// A Steam window that may need you (not Steam's main, friends, "launching" or update windows) and is big enough to
/// be a real dialog, or a window of the game's own processes (its launcher).
pub fn counts_as_launch(w: &NewWin) -> bool {
    if w.game != 0 && w.owner == Some(w.game) { return true; }
    w.steam_client && w.w >= 240 && w.h >= 120 && crate::gamescope::steam_question(&w.title)
}

/// What's in front in gamescope, for the controller mouse.
#[derive(Debug, Clone, Default)]
pub struct Front {
    /// Hearth's own window or its Quick Access.
    pub hearth: bool,
    /// A window of Steam's own processes (its dialogs).
    pub steam_client: bool,
    /// A window of the running (or starting) game's processes, or carrying its number.
    pub game_window: bool,
    pub w: u32, pub h: u32,
    /// The screen (Hearth's own window, which is full screen).
    pub screen_w: u32, pub screen_h: u32,
}

/// Should the controller mouse be on by itself? Over a Steam window (a question, an install step), and over a
/// game's window that's clearly smaller than the screen (a launcher with a Play button); never over Hearth or a
/// game filling the screen (it gets the controller to itself).
pub fn pointer_wanted(f: &Front) -> bool {
    if f.hearth { return false; }
    if f.steam_client { return true; }
    if f.game_window && f.screen_w > 0 && f.screen_h > 0 && f.w > 0 && f.h > 0 {
        return (f.w as u64) * 10 < (f.screen_w as u64) * 9 && (f.h as u64) * 10 < (f.screen_h as u64) * 9;
    }
    false
}

/// A launch window shown smaller than Hearth asks a game's window to be (640x360): still the game's once it has
/// stayed up a few seconds (a small launcher would otherwise wait behind the starting screen for ten minutes).
pub fn small_window_ready(w: u32, h: u32, up: Duration) -> bool { w >= 320 && h >= 200 && up >= Duration::from_secs(4) }

/// The launch being watched (a newer Play replaces it).
static GEN: AtomicU64 = AtomicU64::new(0);
static ASKING: AtomicU64 = AtomicU64::new(0);

fn notify(id: &str, s: Stage) {
    crate::bridge::log(&format!("steam launch: {id}: {}", s.text()));
    crate::stores::notify(json!({"type": "event", "name": "steam.launch", "data": {"id": id, "stage": s.name(), "text": s.text()}}));
}

/// Gamemode saw a Steam question come in front during a launch: the starting screen says so.
pub fn note_asking() { ASKING.store(crate::library::now_ms(), Ordering::SeqCst); }

fn reaper_for(app: u32) -> bool {
    let Ok(dir) = std::fs::read_dir("/proc") else { return false };
    dir.flatten().any(|e| {
        let p = e.path();
        if std::fs::read_to_string(p.join("comm")).map(|c| c.trim() != "reaper").unwrap_or(true) { return false; }
        let args: Vec<String> = std::fs::read(p.join("cmdline")).unwrap_or_default().split(|c| *c == 0).filter(|a| !a.is_empty()).map(|a| String::from_utf8_lossy(a).into_owned()).collect();
        crate::procs::reaper_app("reaper", &args) == Some(app)
    })
}

fn recently_written(f: &Path, ms: u64) -> bool {
    std::fs::metadata(f).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).map(|e| e.as_millis() as u64 <= ms).unwrap_or(false)
}

/// Watches a Steam game's start (once a second, up to 5 minutes, until its processes are up or another game is
/// played) and tells the page each new stage.
pub fn watch(home: &Path, id: &str, app: u32) {
    let g = GEN.fetch_add(1, Ordering::SeqCst) + 1;
    ASKING.store(0, Ordering::SeqCst);
    let (home, id) = (home.to_path_buf(), id.to_string());
    std::thread::spawn(move || {
        let started = crate::library::now_ms();
        let mut last: Option<Stage> = None;
        for _ in 0..300 {
            if GEN.load(Ordering::SeqCst) != g { return; }
            let game_up = app != 0 && reaper_for(app);
            let running = crate::steam::client_running();
            let updating = crate::steam::steam_path(&home).map(|s| recently_written(&s.join("logs/bootstrap_log.txt"), 6000)).unwrap_or(false) && !game_up;
            let asking = ASKING.load(Ordering::SeqCst) >= started && crate::library::now_ms().saturating_sub(ASKING.load(Ordering::SeqCst)) < 60_000;
            let s = stage(running, updating, asking, game_up);
            if last != Some(s) { notify(&id, s); last = Some(s); }
            if game_up { return; }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

/// Stops watching (the game came up, or the launch was left).
pub fn stop_watch() { GEN.fetch_add(1, Ordering::SeqCst); }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_is_kept_ready_in_game_mode_when_used_before() {
        // keep_ready, on_demand, background, game_mode, installed, used_before, running
        assert!(warm_start(true, true, true, true, true, true, false));
        assert!(!warm_start(true, true, true, false, true, true, false), "not in Desktop Mode");
        assert!(!warm_start(true, true, true, true, true, false, false), "never used here: not started");
        assert!(!warm_start(true, true, true, true, false, true, false), "not installed");
        assert!(!warm_start(true, true, true, true, true, true, true), "already running");
        // Setting off: the older settings decide, as before.
        assert!(!warm_start(false, true, true, true, true, true, false));
        assert!(warm_start(false, false, true, true, true, true, false));
        assert!(!warm_start(false, false, false, true, true, true, false));
    }

    #[test]
    fn keep_ready_steps_back_for_a_signed_in_open_engine() {
        use crate::steam_engine::Mode::*;
        assert!(!keep_ready_effective(true, Open, true), "Open and signed in: Valve's client only when a game needs it");
        assert!(keep_ready_effective(true, Open, false), "not signed in to Open: behaves like Classic");
        assert!(keep_ready_effective(true, Classic, true));
        assert!(keep_ready_effective(true, Lightweight, true));
        assert!(!keep_ready_effective(false, Classic, false), "the setting off stays off");
        // And then Hearth closes an idle Valve's client again ("only when needed", on by default).
        assert!(may_close_idle(true, keep_ready_effective(true, Open, true)));
        assert!(!may_close_idle(true, keep_ready_effective(true, Open, false)));
    }

    #[test]
    fn a_steam_kept_ready_is_never_closed_when_idle() {
        assert!(!may_close_idle(true, true));
        assert!(may_close_idle(true, false));
        assert!(!may_close_idle(false, false));
    }

    #[test]
    fn launch_stages() {
        assert_eq!(stage(false, false, false, false), Stage::Starting);
        assert_eq!(stage(true, true, false, false), Stage::Updating);
        assert_eq!(stage(true, false, true, false), Stage::Asking);
        assert_eq!(stage(true, true, true, false), Stage::Asking);
        assert_eq!(stage(true, false, false, false), Stage::Game);
        assert_eq!(stage(true, false, true, true), Stage::Game);
        assert_eq!(Stage::Starting.text(), "Starting Steam…");
        assert_eq!(Stage::Updating.text(), "Steam is updating…");
        assert!(Stage::Asking.text().starts_with("Steam is asking something"));
        assert_eq!(Stage::Game.text(), "Starting the game…");
    }

    #[test]
    fn which_windows_count_as_the_launch() {
        let q = |t: &str, w, h| NewWin { steam_client: true, title: t.into(), w, h, owner: None, game: 377160 };
        assert!(counts_as_launch(&q("Cloud Sync Conflict", 700, 400)));
        assert!(counts_as_launch(&q("Fallout 4 - End User License Agreement", 800, 600)));
        assert!(counts_as_launch(&q("Steam - Error", 400, 200)));
        assert!(!counts_as_launch(&q("Steam", 1280, 800)), "Steam's main window stays out of the way");
        assert!(!counts_as_launch(&q("Launching Fallout 4", 500, 200)));
        assert!(!counts_as_launch(&q("Steam - Updating", 500, 300)));
        assert!(!counts_as_launch(&q("Cloud Sync Conflict", 100, 60)), "too small to be a dialog");
        // The game's own processes (its launcher), whatever the title.
        assert!(counts_as_launch(&NewWin { steam_client: false, title: "Fallout 4 Launcher".into(), w: 900, h: 500, owner: Some(377160), game: 377160 }));
        // Another program's window isn't the launch.
        assert!(!counts_as_launch(&NewWin { steam_client: false, title: "Moonlight".into(), w: 900, h: 500, owner: None, game: 377160 }));
        assert!(!counts_as_launch(&NewWin { steam_client: false, title: "Other".into(), w: 900, h: 500, owner: Some(5), game: 377160 }));
    }

    #[test]
    fn controller_mouse_over_steam_windows_and_launchers() {
        let screen = Front { screen_w: 1920, screen_h: 1080, ..Default::default() };
        assert!(!pointer_wanted(&Front { hearth: true, ..screen.clone() }), "Hearth has the controller");
        assert!(pointer_wanted(&Front { steam_client: true, w: 700, h: 400, ..screen.clone() }), "a Steam question");
        assert!(pointer_wanted(&Front { game_window: true, w: 1024, h: 600, ..screen.clone() }), "a launcher window");
        assert!(!pointer_wanted(&Front { game_window: true, w: 1920, h: 1080, ..screen.clone() }), "the game itself, full screen");
        assert!(!pointer_wanted(&Front { game_window: true, w: 1920, h: 1050, ..screen.clone() }), "nearly full screen is the game");
        assert!(!pointer_wanted(&Front { w: 800, h: 600, ..screen.clone() }), "someone else's window");
        assert!(!pointer_wanted(&Front { game_window: true, w: 800, h: 600, ..Default::default() }), "screen size unknown: leave it");
    }

    #[test]
    fn small_launchers_still_come_up() {
        assert!(small_window_ready(600, 350, Duration::from_secs(5)));
        assert!(!small_window_ready(600, 350, Duration::from_secs(2)));
        assert!(!small_window_ready(200, 100, Duration::from_secs(10)));
    }
}

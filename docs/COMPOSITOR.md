# Hearth's own compositor

The owner, Nov 16, 2026 (10:15 PM): **2.0 has a desktop that is 100% Hearth's own, window manager included.** Today
Mantel (the bar, the dock, Quick Access, notifications: `hosts/linux/desktop/shell`) runs on KWin, with labwc as the
fallback. For 2.0 Hearth writes its own Wayland compositor, in Rust, on the Smithay library, the way System76 wrote
COSMIC's (cosmic-comp) and the niri project wrote niri. KWin and labwc stay installed as fallbacks until Hearth's
compositor has proven itself on real devices.

The code: `hosts/hearth-compositor`. It's a **preview**: nothing starts it unless you turn it on, and if it doesn't
come up, Mantel opens on KWin (then labwc) exactly as today.

## What a compositor is, in one paragraph

On Wayland the compositor is the program that owns the screen, the keyboard, the mouse and the touchscreen. Every app
hands it pictures of its windows; the compositor decides where they go, draws them on the screen with their frames,
and sends each key press and click to the right app. KWin and labwc are compositors. Mantel isn't one: it's a set of
special windows (the bar, the dock, the wallpaper) that asks the compositor for its place through "protocols", small
agreed-on languages. Replacing KWin means Hearth's compositor has to speak every protocol Mantel and Hearth's tools
use, as well as KWin and labwc do.

## Why Smithay

- **It's the parts, not a finished desktop.** Smithay gives the hard, boring pieces (talking to the graphics card and
  screens through DRM/KMS, input through libinput, the session through libseat, XWayland, buffer handling, most
  standard protocols) and leaves every decision about how the desktop looks and behaves to us. That is what "100%
  Hearth's own" needs.
- **It's proven.** cosmic-comp (COSMIC, shipped by System76 on Pop!\_OS) and niri are both Smithay compositors in daily
  use on real hardware, Nvidia included.
- **Rust.** The same language as Hearth's core (`hosts/hearth-core`), the Game Mode host and Hearth's apps, so the same
  people and tests can work on it, and whole classes of crashes (use-after-free, buffer overruns) can't happen.
- **The alternative, wlroots (C, what labwc is built on),** would mean a C codebase next to everything else in Rust.
  Writing a compositor from nothing would be years of work.

The version is pinned exactly (`smithay = "=0.7.0"`, the release on crates.io when this started) and moved forward on
purpose, never by accident. The compositor started from Smithay's own example compositor, Anvil (MIT licence, kept in
`LICENSE-SMITHAY-ANVIL.txt`), which already does the plumbing right; Hearth's parts are added on top.

## What it must do to replace KWin for Mantel

Everything Mantel and Hearth use today on labwc or KWin, and where each stands. "Phase 1" means it's in the code now.

| What | Who uses it | Status |
|---|---|---|
| xdg-shell (normal windows and their menus) | every app | phase 1 |
| wlr-layer-shell (bar, dock, wallpaper, panels, cards) | Mantel (`PanelWindow`, `WlrLayershell` everywhere) | phase 1 |
| Hearth's own window frames (server-side decorations, xdg-decoration + KDE's) | every app that doesn't draw its own | phase 1: title bar in Mantel's colours (`~/.config/hearth/theme.json`), title, minimize / maximize / close, double-click to maximize |
| wlr-foreign-toplevel-management | Mantel's taskbar and dock (`ToplevelManager`), Show desktop, Super+1..9 | phase 1: titles, app ids, screens, maximized / minimized / activated / fullscreen, and activate / close / minimize / maximize / fullscreen back |
| Moving, resizing, stacking, focus, minimizing | everyone | phase 1 (resizing by the window's own edges; Hearth frames have no resize borders yet: stage 2) |
| Snapping to the screen's edges | everyone | phase 1: drag to the left / right edge (half), top (maximize), Super+Left/Right/Up |
| Mantel's keyboard shortcuts | Super (launcher), Super+L, Super+E, Print... | phase 1: read from the same labwc `rc.xml` (one list for both) |
| wl_seat keyboard and pointer, key repeat, keyboard layouts | everyone | phase 1 (layouts from Settings › Keyboard through the session's `XKB_DEFAULT_*`) |
| Clipboard: data-device, primary selection, wlr-data-control | everyone; Mantel's clipboard history, wl-copy / wl-paste | phase 1 |
| XWayland (X11 apps, many games, Steam) | Steam, older apps | phase 1 |
| virtual-keyboard + input-method + text-input | Hearth's on-screen keyboard, hearth-type, wtype | phase 1 (offered; the OSK path needs checking on a device) |
| pointer-constraints + relative-pointer | games (mouse look) | phase 1 (relative motion on real screens; nested runs don't have it) |
| fractional-scale, viewporter, presentation-time | handhelds' scaling, video players, smooth frames | phase 1 |
| xdg-activation | apps raising themselves ("open in the running app") | phase 1 |
| ext-idle-notify + idle-inhibit | Mantel's screen-off and sleep timers (`IdleMonitor`), videos and games keeping it awake | phase 1 |
| DRM lease | VR headsets | phase 1 (from Anvil, real screens only; needs a headset to check) |
| touch, tablets, touchpad gestures | handhelds, tablets | phase 1 basics (touch and pens from Anvil); Mantel's edge swipes: stage 2 |
| ext-session-lock | Mantel's lock screen (`Lock.qml`, Super+L) | **stage 2** (first thing: until then, don't lock on the preview) |
| wlr-screencopy + ext-image-copy-capture | grim (Print screen, Capture), wf-recorder (Hearth Recorder) | stage 2 |
| wlr-output-management | wlr-randr, Settings › Display (resolution, refresh, scale, rotation) | stage 2 |
| gamma control, or Hearth's own night light and colour filters | Night light (wlsunset), hearth-gamma (inverted colours) | stage 2 (ideally built in: no second program fighting for the screen) |
| screencast for the portal (PipeWire) | screen sharing in Discord, browsers, OBS | stage 3 (xdg-desktop-portal-wlr works once screencopy is there; a Hearth portal later) |
| tearing-control | games that want no vsync | stage 3 |
| VRR (FreeSync / G-Sync) | games | stage 3 |
| multi-GPU, Nvidia | gaming laptops, the owner's RTX 5070 PC | from Anvil (it renders on one GPU and copies to the others); needs proving on Nvidia: stage 3 |
| HDR and colour management | HDR screens | stage 4 (Smithay's support is still young) |
| multi-seat (hearth-seats: a second player's own screen, keyboard and pad) | Couch co-op | stage 4 (stays on labwc until then) |
| workspaces / desktops | Super+Ctrl+Left/Right, Mantel's switcher | stage 3 |
| window previews and Alt+Tab with pictures | Mantel's switcher | stage 3 (Alt/Super+Tab cycles windows in phase 1) |

## The stages

Sizes are rough: S is a session or two, M about a week of sessions, L several weeks.

1. **MVP, phase 1 (this branch, M).** Runs on real screens (DRM/KMS + libinput through libseat) and in a window for
   tests (winit). Windows with Hearth's frames, focus, moving, minimizing, maximizing, edge snapping, Mantel's bar,
   dock and wallpaper, Mantel's taskbar unchanged, Mantel's shortcuts, clipboard, XWayland, idle timers, a clean end on
   SIGTERM. Reads Hearth's settings, never writes them. Opt-in only, with the safe start.
2. **Parity with labwc (L).** Session lock (Mantel's lock screen), screencopy (Print screen, Capture, Recorder),
   output management (Settings › Display), night light and colour filters built in, resize borders on Hearth's frames,
   Mantel's edge swipes, the cursor theme and size from Settings, put into the image (built in the image's own build,
   next to the other Rust parts) and tried on the owner's devices (ROG Ally, the RTX 5070 PC, the Galaxy Tab A9 is
   Android so it isn't one).
3. **Parity with KWin (L).** Workspaces, window previews for Alt+Tab and the taskbar, screen sharing through the
   portal, tearing control and VRR for games, Nvidia proven, the KWin bridge's features done Hearth's way
   (`hearth-kwin-bridge` today), rules per app, the on-screen keyboard through input-method on every app.
4. **Beyond (ongoing).** HDR and colour management, multi-seat (so Couch co-op leaves labwc), Hearth's own effects
   (Mantel's frosted glass under windows, animations), Game Mode hand-offs without a black screen, and dropping KWin
   and labwc from the default ladder once the owner says it's proven.

## The safe start (why this can't leave a device unable to start)

- **Off by default.** The session only tries it when `HEARTH_DESKTOP_WM=hearth` is in `~/.config/nimbus/session.conf`,
  which only Settings › System › Your desktop › **Hearth compositor (preview)** writes, when you turn it on. Turning it
  off goes back to what you had picked before (`HEARTH_DESKTOP_WM_BEFORE`). When the compositor isn't on the device,
  the switch says so and stays off.
- **One rung above KWin.** nimbus-session's ladder is Hearth's compositor → KWin → labwc. Hearth's compositor gets the
  same checks KWin gets: not installed, ends within 15 seconds, or Mantel doesn't say it's up within 45 seconds
  (`$XDG_RUNTIME_DIR/hearth-shell-up`) → Mantel opens on KWin for the rest of that sign-in (and on labwc if KWin
  fails too), and a card says why, once. Twice in a row → KWin until the preview is turned on again. A switch you
  asked for is never counted. Safe graphics, a second player's seat and Server mode never use it.
- **What it's told and what it reads.** The same environment labwc gets (its folder's environment file, so Mantel
  and the portals take their labwc ways, which the compositor's protocols are built to match), `HEARTH_WM=hearth`,
  `HEARTH_DESKTOP_PID` (its own process id), labwc's `rc.xml` for the shortcuts and key repeat, `theme.json` for the
  colours. It writes only `$XDG_RUNTIME_DIR/hearth-compositor.json` (what's on the screens, for tests and Hearth) and
  its log, `~/.local/share/nimbus/hearth-compositor.log`.
- **Ending.** nimbus-mode ends it like KWin (a signal to `HEARTH_DESKTOP_PID`, forced after 4 seconds); it ends cleanly
  on SIGTERM, SIGINT and SIGHUP.

## How it's tested (`.github/workflows/hearth-compositor.yml`)

Building and testing happen only in GitHub Actions.

- **Build:** `cargo build --release` and `cargo test` in Fedora (where Quickshell's package is); compiler errors come
  out as annotations with the file and line.
- **Session:** `hosts/linux/test/test_hearth_compositor_session.py` runs nimbus-session for real with stand-ins: never
  by default, the preview runs it with Mantel's environment, missing / crashing / Mantel never up all open KWin (and
  labwc when KWin fails too), twice in a row pauses it, Settings keeps and restores the choice before.
- **Nested:** `.github/scripts/hearth-compositor-test.sh` runs the compositor in a 1920x1080 window (its winit backend)
  on a virtual X screen with software graphics. nimbus-session starts it with the preview on, so the real Desktop
  Mode comes up on it: the autostart, Mantel, a test window (foot). It checks the protocols offered, Mantel's bar and
  dock (layer shell) and their room on the screen, the window on the taskbar (foreign toplevel, also through
  `wlrctl`), minimizing and bringing it back, dragging it to the left edge (snapping, with `xdotool`), Settings ›
  Your desktop, a clean end on SIGTERM and through nimbus-mode, and no panics in the log.
- **Pictures:** one after each step (`comp-1-empty` … `comp-6-your-desktop`), each checked not to be one flat colour
  (the first must be exactly Mantel's background), posted as `SHOT` notices.
- **Next:** a headless backend (rendering into memory, no window) so tests don't need a virtual X screen, Smithay's
  protocol test suite (WLCS), and runs on the owner's devices once it's in the image.

## Where things are

- `hosts/hearth-compositor/src`: `main.rs` and `options.rs` (the command line), `winit.rs` (nested) and `udev.rs` (real
  screens), `state.rs` (the protocols), `shell/` (windows, layer surfaces, XWayland, Hearth's frames in `ssd.rs`),
  `wm.rs` (minimize, snap, focus, the status file), `foreign_toplevel.rs` (Mantel's taskbar), `keybinds.rs` (rc.xml),
  `theme.rs` (Mantel's colours), `input_handler.rs`, `render.rs`, `drawing.rs`.
- Session: `hosts/linux/session/nimbus-session` (`run_hearth_comp`, `hearth_comp_ok`), `nimbus-mode`,
  `hosts/linux/desktop/hearth-compositor/autostart` (runs Desktop Mode's one autostart).
- Settings: `hosts/linux/desktop/shell/hearth-your-desktop` and `SetYourDesktop.qml`; the card: `DesktopNotice.qml`.
- No version number is in the crate: the owner picks every one (Cargo reads a missing one as 0.0.0). When it goes into
  a release, its `Cargo.toml` joins the release's list of files to bump.

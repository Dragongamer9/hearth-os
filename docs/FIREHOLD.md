# Firehold: Hearth's own desktop

The owner, Nov 16, 2026 (10:15 PM): **2.0 has a desktop that is 100% Hearth's own, window manager included.** He first
named it Albireo (10:52 PM), then renamed it **Firehold** (Nov 17): everything a person sees is called
Firehold now, and the compositor itself is **firehold-comp** (binary and crate, the way COSMIC's is cosmic-comp).
Firehold has two postures, **Couch** (gold: the controller and the TV, big tiles, everything full screen) and **Desk** (blue: mouse and
keyboard, windows, Mantel's bar and dock). One session switches between them, so there is **no separate Game Mode**
any more, and games run natively on Firehold itself instead of inside gamescope. The owner's drafts of both postures:
hearth-workspace `hearth/mockups/2.0-desktop` (Couch Home, Desk, a game with the Guide, Library, Settings › Couch and
Desk).

The compositor is firehold-comp: Hearth's own Wayland compositor, in Rust, on the
Smithay library, the way System76 wrote COSMIC's (cosmic-comp) and the niri project wrote niri. Today Mantel (the bar,
the dock, Quick Access, notifications: `hosts/linux/desktop/shell`) runs on KWin, with labwc as the fallback; KWin and
labwc stay installed as fallbacks until Firehold has proven itself on real devices.

The code: `hosts/firehold-comp`. It's for 2.0 and still being built: nothing starts it by default, and if it
doesn't come up, Mantel opens on KWin (then labwc) exactly as today.

## What a compositor is, in one paragraph

On Wayland the compositor is the program that owns the screen, the keyboard, the mouse and the touchscreen. Every app
hands it pictures of its windows; the compositor decides where they go, draws them on the screen with their frames,
and sends each key press and click to the right app. KWin and labwc are compositors. Mantel isn't one: it's a set of
special windows (the bar, the dock, the wallpaper) that asks the compositor for its place through "protocols", small
agreed-on languages. Replacing KWin means Firehold has to speak every protocol Mantel and Hearth's tools
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
| Moving, resizing, stacking, focus, minimizing | everyone | phase 1; **phase 2**: resize borders around Hearth's frames (6 px just outside the frame, corners resize both ways, the arrow pointer over them) |
| Snapping to the screen's edges | everyone | phase 1: drag to the left / right edge (half), top (maximize), Super+Left/Right/Up |
| Mantel's keyboard shortcuts | Super (launcher), Super+L, Super+E, Print... | phase 1: read from the same labwc `rc.xml` (one list for both) |
| wl_seat keyboard and pointer, key repeat, keyboard layouts | everyone | phase 1 (layouts from Settings › Keyboard through the session's `XKB_DEFAULT_*`) |
| Clipboard: data-device, primary selection, wlr-data-control, ext-data-control | everyone; Mantel's clipboard history, wl-copy / wl-paste | phase 1; ext-data-control **phase 2** |
| XWayland (X11 apps, many games, Steam) | Steam, older apps | phase 1 |
| virtual-keyboard + input-method + text-input | Hearth's on-screen keyboard, hearth-type, wtype | phase 1 (offered; the OSK path needs checking on a device) |
| pointer-constraints + relative-pointer | games (mouse look) | phase 1 (relative motion on real screens; nested runs don't have it) |
| fractional-scale, viewporter, presentation-time | handhelds' scaling, video players, smooth frames | phase 1 |
| xdg-activation | apps raising themselves ("open in the running app") | phase 1 |
| ext-idle-notify + idle-inhibit | Mantel's screen-off and sleep timers (`IdleMonitor`), videos and games keeping it awake | phase 1 |
| DRM lease | VR headsets | phase 1 (from Anvil, real screens only; needs a headset to check) |
| touch, tablets, touchpad gestures | handhelds, tablets | phase 1 basics (touch and pens from Anvil); Mantel's edge swipes: stage 2 |
| ext-session-lock | Mantel's lock screen (`Lock.qml`, Super+L) | **phase 2**: see "The lock screen" below |
| wlr-screencopy + ext-image-copy-capture | grim (Print screen, Capture), wf-recorder (Hearth Recorder) | **phase 2**: both, shared memory (XRGB8888), recorders get a frame only when the screen changed; a picture while locked shows the lock screen; the pointer isn't in pictures yet |
| wlr-output-management | wlr-randr, Settings › Display (resolution, refresh, scale, rotation) | **phase 2** (v4): modes, position, rotation, scale; make, model and serial from the screen's EDID; nothing saved by the compositor (Settings saves display.sh); turning a screen off is refused for now |
| gamma control, or Hearth's own night light and colour filters | Night light (wlsunset), hearth-gamma (inverted colours) | **phase 2**: wlr-gamma-control on real screens (the card's colour ramp; one app per screen, as on labwc; back to normal when it lets go); nested there's no ramp (apps are told "failed"). Hearth's own built-in filters: stage 3 |
| screencast for the portal (PipeWire) | screen sharing in Discord, browsers, OBS | stage 3: the path is there (xdg-desktop-portal-wlr's screencast uses wlr-screencopy with damage, now offered); the portal's config for Firehold and dmabuf copies come with it |
| tearing-control | games that want no vsync | **phase 3** (1.15): offered and the hint kept per surface; the Guide's **Allow tearing** (per game, off by default) lets the fullscreen game draw without waiting for the screen (its frame callbacks and FIFO barrier as soon as it hands in a frame; the newest frame is shown at each refresh). A real mid-scan flip waits for a Smithay that can flip asynchronously (0.7 can't) |
| VRR (FreeSync / G-Sync) | games | **game rows**: on for a screen while a fullscreen game is on it (per game, on by default), only on screens that switch without a modeset; see "Games: the Guide's rows" |
| Frame limit, resolution scaling, direct scanout | the Guide over a game (mockup 3) | **game rows**: see "Games: the Guide's rows" |
| multi-GPU, Nvidia | gaming laptops, the owner's RTX 5070 PC | from Anvil (it renders on one GPU and copies to the others); needs proving on Nvidia: stage 3 |
| HDR and colour management | HDR screens | stage 4 (Smithay's support is still young) |
| multi-seat (hearth-seats: a second player's own screen, keyboard and pad) | Couch co-op | stage 4 (stays on labwc until then) |
| workspaces / desktops | Super+Ctrl+Left/Right, Mantel's desktop buttons, the pager, the search's desktops plugin | **phase 3** (1.15): rc.xml's `<desktops>` (names and number), GoToDesktop / SendToDesktop keys, and through Mantel's window IPC: go, move a window, add, remove, rename. Windows on the other desktops step aside (like minimized ones) and stay on the taskbar; activating one goes to its desktop |
| window previews and Alt+Tab with pictures | Everything open's cards, the dock | **phase 3** (1.15): a picture of one window (its frame included, nothing over it), as a PNG in `$XDG_RUNTIME_DIR`, through the window IPC's `shot` (Everything open takes them every 3 s while it's up; the dock's and the taskbar's hover label shows the app's windows' pictures, WinPreviewTip.qml) |
| Mantel's window list with ids, frames, stacking and desktops | WinList (what KWin's hearthwindows script gives on KWin) | **phase 3** (1.15): the window IPC below |
| Firehold's own title bars (mockup 2) | the Desk | **phase 3** (1.15): drawn by the compositor itself while Mantel's Desk is up (`frames.rs`, `shell/ssd.rs`), named by Mantel |

## Mantel's window IPC (1.15)

`$XDG_RUNTIME_DIR/firehold-wm-<wayland socket>.sock` (`ipc.rs`): one JSON object a line both ways. Mantel's side is
`desktop/shell/hearth-firehold-bridge serve --stdio`, which WinList.qml runs under `HEARTH_WM=firehold` exactly as it
runs hearth-kwin-bridge on KWin, so the taskbar, the dock, the pager, the desktop buttons, Everything open and the rest
work the same on both. Until the bridge talks (or if it stops) WinList keeps the taskbar protocol (wlr-foreign-toplevel),
as before. `firehold-comp ctl COMMAND` sends one command by hand (tests).

- `watch`: the window list whenever it changes: `{"t": "windows", "windows": [{id, appId, title, activated, minimized,
  maximized, fullscreen, desktops, output, pid, frame: {x, y, w, h}, ssd, stack}], "active", "desktops": [{name,
  active}], "stack": [ids, top first], "frames"}`.
- `activate`, `close`, `minimize`, `maximize`, `fullscreen`, `snap` (left-half, right-half, maximize), `desktop` (a
  window to desktop n), `goDesktop`, `addDesktop`, `removeDesktop`, `renameDesktop`.
- `shot {id, path, max}`: the window's picture, answered `{"t": "shot", ...}` after the next frame. Only into
  `$XDG_RUNTIME_DIR`, only `.png`, never while the screen is locked.
- `frames {style: "firehold", names, colours}`: Firehold's own title bars (FireholdFramesComp.qml sends it while the Desk
  is up). They go back to Hearth's plain frames when Mantel says so or its connection ends; nothing is saved.

Nothing of it can stop Firehold from starting: a socket that can't be made is logged and skipped. Screen scales are
snapped to the 1/120ths apps are told in (wlr-randr's 2.1 arrives as 2.09765625; without it every app was stretched
by a hair and looked soft).

## The lock screen (phase 2)

Mantel's `Lock.qml` locks through ext-session-lock-v1 exactly as on KWin. While it's locked Firehold draws only the
lock screen (and the pointer) on every screen, sends keys, clicks and touches only to it, ends every grab (a menu's
keyboard grab would get the password), ignores the taskbar's requests, and screen pictures show the lock screen.
Only Ctrl+Alt+F1..F12 (switching consoles) still works.

- **Never unlocks by itself.** Only the lock screen's own unlock, or the right password in the built-in prompt, ends
  it. Smithay passes on any client's unlock, even from a lock it turned down; Firehold counts it only when the lock it
  granted is the one that's gone.
- **Never locks anyone out.** If the lock screen dies (Mantel crashed or was killed), the screens stay covered and
  Firehold shows its own small prompt ("type your password and press Enter"), checked by the system's own password
  checker (`unix_chkpwd`, the helper pam_unix uses; two seconds' wait after a wrong one). A few seconds later
  (`FIREHOLD_RELOCK_AFTER`, 3 s) it asks Mantel, which its autostart brings back, to lock again with Super+L's command
  from rc.xml, and Mantel's lock screen takes over. A lock screen that's alive but shows nothing for 8 seconds gets
  the prompt too.
- Tested in `firehold-test.sh` (steps 7 and 8): locked by Mantel, Mantel killed (still locked, the prompt, a wrong
  password stays locked), Mantel back and taking over, killed again, the right password unlocks.

## Games: the Guide's rows

The Guide over a game (mockup `docs/mockups/firehold/3-in-game.png`) has three rows that only Firehold's own
compositor can do. They are built in `hosts/firehold-comp/src/game.rs` (with `render.rs` and `udev.rs`), for "the
game": the window that is fullscreen on a screen.

- **Frame limit** (Off, 30, 40, 60, 90, 120, the screen's rate). The game's frame callbacks, and its FIFO barrier
  (Vulkan's FIFO through wp-fifo), stop coming with the screen's frames and come from a timer of their own at the limit,
  the way gamescope limits a game. Nothing else waits: Mantel, the Guide and every other window keep the screen's pace.
  A limit at or above a real screen's rate needs no timer (the screen paces the game already); "the screen's rate" is
  the same as vsync.
- **Resolution scaling** (Off, Quality 1/1.5, Balanced 1/1.7, Performance 1/2 on each side, FSR 1's ratios). The game
  is told a smaller fullscreen size (1920 x 1080 at Quality: 1280 x 720) and its client gets the matching scale, so
  Smithay maps its buffers, the pointer and touch, and the screens it's told about between the two sizes. The
  compositor draws it full screen with a linear filter and a contrast-adaptive sharpening pass (FSR 1's RCAS / AMD's
  CAS style: sharpens less where the contrast is already high, so edges don't ring), a GLES shader made once per GPU.
  If that shader can't be made, plain linear is used and the log says so. The Guide's label is the size it draws at
  in the screen's pixels ("Quality · 1440p" on a 4K screen). X11 games (through XWayland, which all X11 apps share)
  aren't scaled yet: the row says so (`scalingNote`) and the game draws at the screen's size.
- **Direct to the display** (on by default). While the game is fullscreen with nothing over it, its own buffer may go
  straight onto the screen's primary plane in any format the plane takes (Smithay's `DrmCompositor`, with
  `ALLOW_PRIMARY_PLANE_SCANOUT_ANY`); the kernel's test commit decides, and when it says no the frame is composited as
  always. While it's sharpening, the game is always composited (the shader needs the GPU). Off for a game: always
  composited. Whether the last frame went straight to the display is in the status (`scanout`) and the log.
  `FIREHOLD_DISABLE_DIRECT_SCANOUT` still turns all of it off. **VRR (adaptive sync)** goes on for that screen while
  the game is fullscreen (per game, on by default), only when the screen can switch without a modeset (no black
  flash); it goes off again when the game leaves. A failure is logged and the screen keeps its fixed rate.

**The interface the Guide uses** (FireholdGuide.qml on `feature/firehold-shell` calls the helper):

| Name | What |
|---|---|
| `hearth-firehold-game status` (`hosts/linux/desktop/shell`) | one JSON line: `{"ok", "running", "backend", "game": {"appId", "title", "x11", "output", "outputSize", "refresh", "frameLimit" (`"off"`, a number or `"screen"`), "frameLimitLabel", "frameLimitActive", "pacedFps", "scaling", "scalingLabel" (`"Quality · 1440p"`), "renderSize", "configureSize", "scalingNote", "sharpen", "sharpenActive", "direct", "scanout" (`"direct"`, `"composited"`, `"unknown"`), "scanoutChoice", "vrr", "vrrActive", "vrrScreen" (`"yes"`, `"no"`, `"needs-modeset"`, `"unknown"`, `"nested"`)} or null}`. Writes nothing (opening the Guide changes nothing) |
| `hearth-firehold-game set KEY VALUE [APP]` | `frameLimit off\|30\|40\|60\|90\|120\|screen`, `scaling off\|quality\|balanced\|performance`, `direct on\|off`, `vrr on\|off`, `sharpen on\|off`, for the fullscreen game (or the app id APP); prints the new status, or `error: ...` and exits 1 |
| `hearth-firehold-game step KEY up\|down [APP]` | the Guide's ‹ › (the next or previous value, round); prints the new status |
| `$XDG_RUNTIME_DIR/firehold-game-<WAYLAND_DISPLAY>.sock` | the compositor's control socket: one line in (`status`, `reload`), one JSON line out. Only the helper needs it |
| `~/.config/hearth/firehold-games.json` | `{"games": {"<app id>": {"frameLimit", "scaling", "direct", "vrr", "sharpen"}}}`. Written only by the helper, through `hearth_settings_io` (atomic, `.bak`, merged under the lock); a key that's missing is its default and is never written. The compositor reads it at start, when it changes and on `reload`, and never writes it; a file it can't read leaves it with what it had (the helper puts it back from `.bak` the next time it saves) |
| `$XDG_RUNTIME_DIR/firehold.json`'s `game` | the same status, for tests |

**Safety.** Nothing here can stop Firehold from starting: a control socket that can't be made, a timer that can't
start, a shader that can't compile, a plane that refuses the game's buffer and a screen that won't switch VRR are each
logged (`Firehold game: ...` in `firehold.log`) and the game is drawn the normal way. The session ladder is untouched.

**Tests** (`.github/workflows/firehold.yml`, job `game`; `.github/scripts/firehold-game-test.sh`, nested and headless):
a stand-in game in plain Python (`firehold-game-client.py`: fullscreen, shared memory, a 200-pixel white square)
chosen for through the helper exactly as the Guide does. It measures the frame callbacks with no limit, at 30 (27–33
a second) and at 60; checks the size the game is told at Quality (1280 x 720) and Performance (960 x 540) and that
grim's picture of the screen has the square upscaled (300 and 400 pixels); that `status` writes nothing, that the
file is written with its `.bak`, that a damaged file is left alone by the compositor; Direct's choice on and off; a
clean end. Pictures: `firehold-gn-1-native`, `-2-quality`, `-3-performance` (nested) and `firehold-gh-2-quality`
(headless). Direct scanout and VRR need a real screen: `cargo test` checks their decisions (`game.rs`), and on a
device the log says which way each frame went. `hosts/linux/test/test_firehold_game.py` checks the helper.

## The stages

Phase 1 is Desk only, on Mantel as it is today. Couch (Firehold's own full-screen tiles, the Guide over games, games
started natively without gamescope) comes once Desk can stand in for KWin; then Steam Game Mode and Hearth Game Mode
become fallbacks the same way KWin and labwc are.

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
4. **Couch, and beyond (ongoing).** The Couch posture from the owner's mockups (no separate Game Mode; games run
   natively with the Guide over them), HDR and colour management, multi-seat (so Couch co-op leaves labwc), Hearth's own effects
   (Mantel's frosted glass under windows, animations), Game Mode hand-offs without a black screen, and dropping KWin
   and labwc from the default ladder once the owner says it's proven.

## The safe start (why this can't leave a device unable to start)

- **Off by default.** The session only tries it when `HEARTH_DESKTOP_WM=firehold` is in
  `~/.config/nimbus/session.conf` and it's enabled on that device; otherwise any Firehold setting is ignored and
  Mantel opens on KWin as always. When the compositor isn't on the device, it's never tried.
- **The old name keeps working.** A choice made while it was called Albireo (`HEARTH_DESKTOP_WM=albireo`) is read
  as Firehold and rewritten as `firehold` (atomically, keeping a .bak); the safe start's count (`desktop-fails`) and
  its notices carry over the same way. Nothing is reset.
- **A way out, always.** Ctrl+Alt+Backspace ends Firehold and opens Mantel on KWin for the rest of that sign-in (not
  counted as a failure; never while the screen is locked).
- **One rung above KWin.** nimbus-session's ladder is Firehold → KWin → labwc. Firehold gets the
  same checks KWin gets: not installed, ends within 15 seconds, or Mantel doesn't say it's up within 45 seconds
  (`$XDG_RUNTIME_DIR/hearth-shell-up`) → Mantel opens on KWin for the rest of that sign-in (and on labwc if KWin
  fails too), and a card says why, once. Twice in a row → KWin until it's turned on again (the card says so, with one button to turn it back on). A switch you
  asked for is never counted. Safe graphics, a second player's seat and Server mode never use it.
- **What it's told and what it reads.** The same environment labwc gets (its folder's environment file, so Mantel
  and the portals take their labwc ways, which the compositor's protocols are built to match), `HEARTH_WM=firehold`,
  `HEARTH_DESKTOP_PID` (its own process id), labwc's `rc.xml` for the shortcuts and key repeat, `theme.json` for the
  colours. It writes only `$XDG_RUNTIME_DIR/firehold.json` (what's on the screens, the lock, screen pictures
  taken, the game, for tests and Hearth), its log, `~/.local/share/nimbus/firehold.log`, and the game rows' control socket
  (`$XDG_RUNTIME_DIR/firehold-game-<socket>.sock`, removed when it ends). It only reads `~/.config/hearth/firehold-games.json`. Screen changes from wlr-randr last
  until it ends; nothing of yours is saved by it.
- **Ending.** nimbus-mode ends it like KWin (a signal to `HEARTH_DESKTOP_PID`, forced after 4 seconds); it ends cleanly
  on SIGTERM, SIGINT and SIGHUP.

## How it's tested (`.github/workflows/firehold.yml`)

Building and testing happen only in GitHub Actions.

- **Build:** `cargo build --release` and `cargo test` in Fedora (where Quickshell's package is); compiler errors come
  out as annotations with the file and line.
- **Session:** `hosts/linux/test/test_firehold_session.py` runs nimbus-session for real with stand-ins: never
  by default, turned on it runs with Mantel's environment, missing / crashing / Mantel never up all open KWin (and
  labwc when KWin fails too), twice in a row pauses it, Settings keeps and restores the choice before.
- **Nested:** `.github/scripts/firehold-test.sh` runs the compositor in a 1920x1080 window (its winit backend)
  on a virtual X screen with software graphics. nimbus-session starts it with it turned on, so the real Desktop
  Mode comes up on it: the autostart, Mantel, a test window (foot). It checks the protocols offered, Mantel's bar and
  dock (layer shell) and their room on the screen, the window on the taskbar (foreign toplevel, also through
  `wlrctl`), minimizing and bringing it back, dragging it to the left edge (snapping, with `xdotool`), Settings ›
  Your desktop, a clean end on SIGTERM and through nimbus-mode, and no panics in the log.
- **Phase 2 steps** in the same run: resizing by the frame's right border (5b), wl-copy / wl-paste and wtype (6b), the
  lock screen (7, 8, 9), grim's picture against the screen's and a region, wf-recorder recording (10), wlr-randr's
  list, a scale change and back, a refused mode, and Settings › Display (11).
- **Pictures:** one after each step (`firehold-1-empty` … `firehold-11-display`), each checked not to be one flat colour
  (the first must be exactly Mantel's background), posted as `SHOT` notices.
- **Headless:** `.github/scripts/firehold-headless-test.sh` runs the compositor with `--headless` (no screen and no
  window: drawn into memory with Mesa's software GL), started by nimbus-session with Mantel on it, and takes its
  picture with grim (`firehold-h-mantel`): no virtual X screen at all.
- **The image:** `os-bases/peppermint/builder.sh` builds it next to the other Rust parts into
  `/usr/libexec/hearth/firehold-comp`; base-peppermint.yml checks it's there with all its libraries, that nothing turns it
  on, and that `unix_chkpwd` is there for the lock prompt.
- **Next:** Smithay's protocol test suite (WLCS), and runs on the owner's devices.

## Where things are

- `hosts/firehold-comp/src`: `main.rs` and `options.rs` (the command line), `winit.rs` (nested), `udev.rs` (real
  screens) and `headless.rs` (no screen), `state.rs` (the protocols), `lock.rs` (the lock screen and its prompt),
  `screencopy.rs` (screen pictures), `output_management.rs` (wlr-randr), `gamma.rs` (colour ramps), `edid.rs`
  (screens' names), `tearing.rs`, `shell/` (windows, layer surfaces, XWayland, Hearth's frames in `ssd.rs`),
  `wm.rs` (minimize, snap, focus, the status file), `game.rs` (the Guide's game rows), `foreign_toplevel.rs` (Mantel's taskbar), `keybinds.rs` (rc.xml),
  `theme.rs` (Mantel's colours), `input_handler.rs`, `render.rs`, `drawing.rs`.
- Session: `hosts/linux/session/nimbus-session` (`run_firehold`, `firehold_ok`), `nimbus-mode`,
  `hosts/linux/desktop/firehold/autostart` (runs Desktop Mode's one autostart).
- Settings: `hosts/linux/desktop/shell/hearth-your-desktop` and `SetYourDesktop.qml`; the card: `DesktopNotice.qml`.
- No version number is in the crate: the owner picks every one (Cargo reads a missing one as 0.0.0). When it goes into
  a release, its `Cargo.toml` joins the release's list of files to bump.

## Firehold on Plasma (a test, Nov 18, 2026)

The owner (Nov 18): Hearth moves to Fedora (Anatase) once he has tested it, and Firehold is built two ways so he can
try both. This is the first way: **Firehold as a KDE Plasma interface on KDE's own KWin 6.8**, the way KDE builds Plasma
Bigscreen and Plasma Mobile. Nothing of KWin is patched and nothing comes back from `archive/kwin`; only public,
supported pieces: plasmashell shell packages, Plasma's QML (its task model, Kirigami, Plasma's data sources, KDE's
LayerShellQt), Plasma's scripting interface, and one KWin script through KWin's script API.

Code: `hosts/linux/desktop/firehold-plasma` (installed into the Anatase image by `install.sh` from `build.sh`).
Checklist: `docs/mockups/checklists/firehold-plasma.md`. CI: `.github/workflows/firehold-plasma.yml` (pictures on the
branch `ci/firehold-plasma-shots-<branch>`, and ten minutes at rest next to plain Plasma).

- **Couch** is the shell package `org.hearthos.firehold`; its desktop is the containment `org.hearthos.firehold.couch`
  (Home, Library, Apps, Store, Everything open, Settings › Couch and Desk, and the Guide in a layer over the game).
  Games come from `desktop/shell/hearth-firehold` and the rules from `desktop/shell/firehold.js`, the same as Mantel's
  Firehold; windows from Plasma's own task model.
- **Desk** is Plasma's own desktop: the shell package `org.hearthos.firehold.desk` is Plasma's `org.kde.plasma.desktop`
  copied when the image is built, with Firehold's first layout (a top bar with the app's name, its menus, the date and
  clock, the posture pill and Wi-Fi · battery, and a dock). It keeps its own layout file, so plain Plasma's panels are
  never touched.
- **Switching** restarts only plasmashell with the other shell package (KDE's own user service when it runs, else
  `plasmashell --replace`): KWin, the session and every app and game stay open. The Guide button (or Meta+Ctrl+G) on the
  Desk goes to the Couch; on the Couch it opens the Guide, twice Everything open. With Automatic, the mouse or keyboard
  goes to the Desk. The KWin script (loaded for the session over D-Bus, never from kwinrc) puts a game full screen on
  the Couch and back in its window on the Desk.
- **The controller**: `hearth-firehold-plasma session` reads controllers (never grabs them) and, only while the Couch
  or the Guide is in front, sends the D-pad, A, B, X, Y, LB/RB and LT/RT as keys through its own "Hearth Firehold keys"
  device. The Guide button always works.
- **Opt-in and safe to fail**: "Firehold (Plasma, test)" in Plasma's menu (and Settings › System in the Couch) picks it;
  it's saved as `HEARTH_PLASMA_DESKTOP=firehold` in `~/.config/nimbus/session.conf` (that line only, through
  `hearth_settings_io`; no line is plain KDE Plasma) and nothing changes it by itself. If the Couch or the Desk doesn't
  say it's up within 45 s, plain KDE Plasma starts in its place for that session, with everything still open, and a
  notification says so; the choice stays. nimbus-session clears Plasma's shell variable before every Plasma start, so
  plain Plasma never gets Firehold's shell, and Game Mode's own fallbacks are unchanged.
- **Settings** are Firehold's `~/.config/nimbus/firehold.json`, shared with Mantel's Firehold, written a key at a time
  and only when changed. Opening a page writes nothing.
- **Performance**: no QML Timer and no animation anywhere (a test checks); the clock is Plasma's time source once a
  minute; window pictures stream only while Everything open is on screen; the helper sleeps in `select` (it wakes every
  10 s to look for new controllers).

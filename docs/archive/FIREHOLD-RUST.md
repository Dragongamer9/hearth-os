# Firehold (Rust, test)

The owner, Nov 18, 2026 (2:34 AM): Hearth moves from Debian to Fedora (the Anatase edition), and becomes the main
edition only after he's tested it. Firehold is built two ways so he can try both: **(B) completely in Rust, the way
COSMIC is** (this page), and (A) a Plasma interface on KWin (another branch). This is (B), a prototype: nothing on the
roadmap is ticked for it.

Firehold is one desktop with three postures in one session, the same open apps in each, and switching never restarts
anything: **Couch** (the controller, big tiles, full-screen apps, the Guide button), **Desk** (mouse and keyboard,
windows, the bar and the dock) and **Away** (the home server keeps running). The approved mockups are
`docs/mockups/firehold/1-6`; what of them this build has is in `docs/mockups/checklists/firehold-rust.md`.

## What it is

- **The compositor**: `hosts/firehold-comp` (Smithay), the same one the QML Firehold uses. Its window IPC
  (`src/ipc.rs`) gives the shell the window list, actions, desktops, each window's picture and Firehold's own title
  bars. New for this build: `watch` with `"input": true` also says when a real mouse or keyboard was used (`{"t":
  "input", "kind": "mouse"|"keys"}`, at most once a second per kind), so Automatic can switch to the Desk.
  Also new here (they help the QML Firehold too): the game in front's frames are counted whether or not they're
  paced (`fps` in the game socket's status, for the overlay without a frame limit); a Wayland window that's full
  screen has no title bar (a game the Couch put there showed Firehold's bar over its top, as X11 windows never did);
  and a window placed before it had a size is placed again, in the middle when it's big, on its first frame
  (Settings hung off the screen's edge).
- **The shell**: `hosts/firehold-shell`, one Rust program in a few processes, no QML and no Quickshell:
  - `firehold-shell hub`: started by firehold-comp. It starts the surfaces, keeps one connection to firehold-comp,
    reads the controller (gilrs, read only: nothing is grabbed), decides the posture and tells each surface what to
    show (`src/hub.rs`, its socket in `src/proto.rs`).
  - `couch` (Home, Library, Apps, Store, Everything open), `bar` and `dock` (the Desk), `guide` (the Guide over a
    game), `perf` (the performance overlay), `notice` ("Switched to Desk"): each one iced program on one
    wlr-layer-shell surface (`src/ui/`). `settings` (Settings › Couch and Desk, and System) is an ordinary window.
  - `firehold-shell ctl LINE` sends the hub one command (tests); `chosen` and `choose` are the Desktop Mode choice.

### Why iced (with iced_layershell), not libcosmic

Both are System76's world: libcosmic is COSMIC's toolkit built on System76's fork of iced. For this prototype plain
**iced 0.14 from crates.io with iced_layershell 0.19** works best:

- **Headless in CI.** iced draws with wgpu where there's a GPU and falls back to tiny-skia (software, into Wayland
  shared memory) where there isn't; `ICED_BACKEND=tiny-skia` forces it. CI has no GPU, and firehold-comp's headless
  backend takes shared-memory buffers, so every screen can be pictured there (the CI pictures are tiny-skia's).
- **Fedora 45.** Everything comes from crates.io at fixed versions and builds with Fedora's own Rust in the Anatase
  base image (base-anatase.yml's "firehold" job). libcosmic isn't on crates.io: it's a git dependency that pins its own
  iced fork, wgpu and cosmic-text from git, and pulls COSMIC's config and portal crates, so builds are larger and break
  when its main branch moves.
- **Layer shell.** iced_layershell gives each iced program a wlr-layer-shell surface with its layer, anchors, size,
  margin and keyboard changed at run time (the Couch moves between the overlay and the background layer without
  being made again). That's the part libcosmic adds for COSMIC's panel; firehold-comp already speaks
  wlr-layer-shell for Mantel.
- **Look.** Firehold's look isn't COSMIC's, so libcosmic's widgets and theme would be overridden everywhere anyway.

**One look with Hearth's pure-Rust apps.** The colours are `hosts/hearth-ui`'s (`src/theme.rs`, the crate Task
Manager, System Restore and Reliability history use, on the same iced 0.14): the look picked in Settings
(`~/.local/share/nimbus/desktop.json` and palettes.json, as Mantel and Plasma's looks have it), followed at once while
Firehold is open (hearth-ui's inotify watch, asleep until one of the two files is written). With no look picked it's
Hearth's own warm look, the one the mockups are drawn in (`set_default_look("hearth")`; the other apps keep
Steam-style). Only a game's own colours and what's written over them (the hero, the tiles, GAME and WINDOW) stay
fixed. `firehold-shell/src/ui/style.rs` is only the names for hearth-ui's palette.

If COSMIC's own pieces are wanted later (its applets, its settings daemon), the shell can move to libcosmic: the
surfaces are small and talk to the hub only through `link.rs`.

### Nothing from cosmic-comp yet

firehold-comp already has what this build needs (multi-monitor output management, screencopy, the DRM backend from
Smithay's Anvil). Nothing has been taken from cosmic-comp so far; anything that is will keep System76's GPL-3.0
copyright notice in the file and be listed here.

## Opt-in and safe to fail (red flags 2 and 9)

- **Never the default.** KDE Plasma stays Desktop Mode on Anatase. "Desktop Mode: Firehold (Rust, test)" in Plasma's
  menu (`hearth-desktop-choice firehold`) saves the pick, and Settings › System inside Firehold has the same choice
  and "Back to Game Mode".
- **Beside Firehold on Plasma** (the other test, `hearth-firehold-plasma`, its choice in session.conf): the two never
  fight. Picking either Firehold, or KDE Plasma, sets the other one to plain Plasma, each through its own helper and
  only on your pick (hearth-desktop-choice, `hearth-firehold-plasma pick`, and Settings › System here, which offers
  all three when Firehold on Plasma is in the image). The pick is `~/.config/nimbus/firehold-rust.json` (`{"desktop": "firehold"}`), written
  only by `firehold-shell choose` through `settings_file.rs` (atomic, .bak, merged, only when it changes); no file is
  Plasma. Nothing else writes it, and a failure never changes it.
- **Falls back to Plasma.** nimbus-session's `run_firehold_rust` (called from `run_plasma`) starts
  `firehold-comp -- firehold-shell hub`. The hub writes `$XDG_RUNTIME_DIR/firehold-shell-up` once the Couch (and the
  Desk's bar and dock) said hello. If that doesn't come within 45 s, firehold-comp ends early, or either ends by
  itself with no mode asked for, KDE Plasma opens for this sign-in and says why once. The hub's process is watched
  (every 5 s, from the session script): a shell that died ends the compositor too, so the session goes on. A surface
  that crashes is started again by the hub (at most 5 times a minute).
- **Game Mode always comes up**: Back to Game Mode is `nimbus-mode game` (HEARTH_DESKTOP_PID is firehold-comp's), and
  nothing here touches Game Mode, booting or the boot menu.
- **Settings**: Couch and Desk is `~/.config/nimbus/firehold.json` with the QML Firehold's own keys (posture, startIn,
  guide, games, bigPicture, overlay, perScreen), one key merged at a time through the hub with `settings_file.rs`;
  opening Settings writes nothing. The Guide's game rows are firehold-comp's `~/.config/hearth/firehold-games.json`
  (per game, merged). The last posture (for "Start in: where you left it") is state, in
  `~/.local/state/hearth/firehold-rust-posture`.
- **Apps and Steam**: everything runs as on any Wayland desktop, X11 apps and Steam through firehold-comp's XWayland.
  Games run on firehold-comp itself (no gamescope); Steam Game Mode stays the way to play inside gamescope.

## Performance (red flag, Nov 17)

- Nothing redraws without a change: every surface sleeps until the hub sends something or the person touches it. The
  hub waits on sockets, the controller (the kernel's own wait in gilrs) and its children. The only timers: the clock
  (once a minute), the overlay's line (once a second, only while a game is in front and the overlay is on) and the
  windows' pictures (every 3 s, only while Everything open is on screen).
- The look: one inotify watch per surface (hearth-ui's), asleep until desktop.json or palettes.json is written (a
  folder that isn't there yet is looked for once a minute). The Guide's Record tile reads Hearth Recorder's state file
  when the Guide opens or a tile is pressed, never on a timer. The overlay's fps comes from a counter firehold-comp
  keeps on the game's frames, worked out only when the overlay asks (once a second, only while it's on).
- No animations. On the Desk the Couch is only the wallpaper (the background layer, the gradient, no keys), and the
  bar and the dock aren't running in Couch.
- CI checks it: firehold-rust.yml's "idle" runs Firehold and plain KDE Plasma for ten minutes each at rest on the same
  kind of runner, and "idle-compare" fails if the shell or the compositor use more of a core than Plasma's own (or 1%,
  whichever is more), grow more than 16 MB (or a tenth), or a shell process restarts. The Anatase image's smoothness
  step measures it in the image too (warn-only there).

## Keys (firehold-comp reads `/usr/share/hearth/firehold-rust/rc.xml`)

Super: Home on the Couch · Super+G: the Guide · Super+Tab: Everything open · Super+I: Settings · Super+D: the Desk ·
Alt+Tab, Alt+F4, F11 (full screen), Super+arrows (snap, maximize, minimize) · Print: a screenshot to
Pictures/Screenshots · the volume and brightness keys. The keyboard's arrows, Enter (A), Escape (B), Page Up/Down
(LB/RB) and Home/End (LT/RT) work on the Couch and the Guide like the controller.

## Trying it (once an image with it is published, which needs the owner's go-ahead)

1. Switch to the Anatase test (`test-1.15`) and go to Desktop Mode (KDE Plasma).
2. Open Plasma's menu, search "Firehold", pick **Desktop Mode: Firehold (Rust, test)** and say **Yes** to switch now.
3. Firehold opens on the Desk (Start in: Desk); press the controller's Guide button or Super for the Couch, move the
   mouse to go back to the Desk. Settings › System › KDE Plasma (or **Desktop Mode: KDE Plasma** in Plasma's menu)
   goes back; Back to Game Mode is there too.
4. If it doesn't start, KDE Plasma opens by itself with a note; the log is `~/.local/share/nimbus/firehold-rust.log`
   (Send logs takes it).

## The Guide's tiles

- **Record** is Hearth Recorder's (`hosts/hearth-recorder`, in the image): with its instant replay on, the tile says
  "Last 30 s too" (its own seconds) and A saves them (`hearth-recorder --save-replay`); otherwise A starts a recording
  and A again stops it (`--toggle`). Firehold never turns instant replay on by itself.
- **Friends** opens Steam's own friends list (`steam://open/friends`). How many are online is Steam's account data,
  which Hearth never reads, so the tile says "In Steam" rather than a count.

## Firehold's own parts (it's its own desktop)

Plasma's and Mantel's panels aren't there inside Firehold, so it has its own (no mockups of their own: ported from
Mantel, drawn in Firehold's look):

- **Quick Settings** (`firehold-shell quick`, Mantel's QuickAccess.qml): the Desk bar's Wi-Fi and battery open it. You
  and the battery, Settings / Lock (when swaylock is there) / Power (Sleep, Restart, Shut down); Return to Game Mode;
  Wi-Fi, Bluetooth, Do not disturb (Mantel's own switch, `hearth-desk set dnd`, so it's the same in every desktop), the
  on-screen keyboard, Away and the Couch; volume and brightness; the power profile; the notifications that came, with
  Clear all. It reads what it shows when it opens (nmcli, bluetoothctl, wpctl, powerprofilesctl, each with a time
  limit), never on a timer. Night light isn't there yet (Mantel's is wlsunset on labwc; firehold-comp's colour ramp
  works only on real screens).
- **Notifications** (`notify.rs`): Firehold is `org.freedesktop.Notifications` while it's open (only when no other
  program has the name). Cards at the top right for 6 seconds (or as long as the app asks; urgent ones stay), with the
  app's buttons and ×; the last 30 in Quick Settings. Do not disturb keeps them off the screen (urgent ones still show).
- **The on-screen keyboard** (`firehold-shell osk`, Hearth's keyboard from docs/OSK.md: the number row, QWERTY, ⇧,
  &123, space, the caret keys, ⌨ to hide): docked at the bottom, windows make room. It never takes the keyboard; what
  it types goes to the window that has it through firehold-comp's new `key` command (as if typed on a US keyboard; a
  letter with an accent isn't typed yet). The Guide's Keyboard tile and Quick Settings open it. Controller: D-pad, A
  types, X ⌫, Y space, LB / RB the caret, LT shift, RT symbols, ☰ Enter, B hides it.
- **The Desk bar's menus** (`firehold-shell menu`): File, Edit and View open under their titles with the commands every
  app has (New window, Open, Save, Print, Close; Undo, Redo, Cut, Copy, Paste, Select all, Find; Full screen, Maximize,
  Minimize, the halves, Zoom, Reload), each sent to the app in front as its own shortcut or done to its window by
  firehold-comp. The posture pill opens Couch / Desk / Away. An app's *own* menu (KDE's global menu over D-Bus) isn't
  read: firehold-comp doesn't speak KDE's appmenu protocol yet.
- **Away** (`firehold-shell away`, the third posture): from the pill's menu or Quick Settings. Every screen shows the
  time and what this PC does for your other devices (`hearth-home-server quick`, once a minute while it's up: shared
  folders, backups, the AI services, Use this PC from anywhere, anything that needs a look), with "Let them run now"
  for jobs paused for a game (the home server's own `resume on`, only when you press it). Nothing is closed; any
  button, the mouse or a key brings back what you were in (the first 3 seconds' mouse and keys don't count).
- **Per screen** (mockup 6's row): with two screens or more, the Couch, the bar and the dock are one a screen
  (`couch@HDMI-A-1`), each in its screen's posture from `perScreen` in firehold.json. Settings › Couch and Desk › Per
  screen opens a row a screen (Same as the others → Couch → Desk). A screen with its own posture keeps it when the
  mouse or the controller switches the others; games and windows go full screen or back by the posture of the screen
  they're on. CI runs it on two made-up screens (firehold-comp's new `--outputs HEADLESS-1,HDMI-A-1`).
- **The hero's chapter** (mockup 1's "Chapter 4"): no store says which chapter you're on, so the game says it itself in
  Hearth's progress file, `~/.local/share/hearth/progress/<game>.json` = `{"chapter": "Chapter 4"}`, named by Hearth's
  id or the game's title as a file name (`steam-1000.json`, `ember-trails.json`). Hearth's own games (Godot: Quad Mech,
  Gravesleep) can write it; read only, shown when it's there.

## Not done yet

See the checklist's "Still open" list: Friends' online count is Steam's account data (Hearth never reads Steam
accounts), the game drawn straight to the display needs a real screen to prove (firehold-comp does it on udev), and
an app's own global menu (KDE's appmenu protocol) isn't read; the bar's menus have the standard commands instead.

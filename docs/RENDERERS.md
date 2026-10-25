# Three ways to draw Hearth

Hearth on Linux is one program, `nimbus` (hosts/hearth-rs, on hearth-core). Everything it does (the library,
stores, Beacon, multi-seat, Game Mode, logs) is that program's. Only the drawing of its screens can be done three ways:

| Renderer | What draws | Where | Status |
|---|---|---|---|
| **WebKit** | the system's WebKitGTK, through Tauri, in the `nimbus` process; the web page ui/index.html | the usual one: AMD and Intel graphics | complete |
| **Electron** (Chromium) | Hearth's Electron window app (`/usr/lib/hearth-electron`, hosts/electron); the same web page | Nvidia graphics | complete |
| **Qt** | Hearth's Qt Quick window app (`/usr/lib/hearth-qml`, hosts/hearth-qml); native QML screens | only when chosen | **Experimental** (phase 1) |

Why Electron: on Nvidia, WebKit's fast drawing path (DMA-BUF/GBM) fails on RTX 50 cards (WebKit bug 303811) and its
compatible path copies every frame through the processor, which makes Hearth sluggish. Chromium's graphics work
well there.

Why Qt: the same Qt 6 that draws Desktop Mode (Mantel, hosts/linux/desktop/shell) can draw Game Mode without a web
engine at all: less memory, no web engine's GPU quirks, and the same look and on-screen keyboard as Mantel. It's
being built screen by screen; until it has everything, the web renderers stay the default.

## How the window apps work

`nimbus` decides at start (`hearth_core::renderer::pick_at_start`).

- **Electron**: `nimbus` starts the Electron app in its place (exec, same arguments, `HEARTH_CORE_BIN` naming itself).
- **Qt**: `nimbus` starts the Qt app as its child and waits for it (`run_qt` in main.rs), with `HEARTH_CORE_BIN` and
  `HEARTH_QT_PARENT`; the Qt app leaves with `nimbus` (PR_SET_PDEATHSIG). Waiting is what lets a crash be seen and
  counted (below) without the session's watchdog counting it as Hearth crashing: after a crash in the session `nimbus`
  leaves with 75 ("start again").

Either window app then starts `nimbus --core-stdio`: the whole Hearth, without a window, talking to the window app
over its standard input and output, one JSON object a line (hosts/hearth-rs/src/winhost.rs describes every message).
The window app shows the screens, passes their messages (the same calls and events as docs/bridge.md) and does what the
core asks of its windows: show, hide, focus, the overlay, sign-in pages, Xbox Cloud Gaming, file pickers. Window titles
are the same in all three (`Hearth`, `Hearth Overlay`), so Game Mode finds them in gamescope the same way. Nothing is
duplicated: a fix in the core is a fix for all three. The Qt app sets `HEARTH_RENDERER_RUNNING=qt` for its core, so
`host.info` (`renderer`) and Settings say Qt draws.

The session and its watchdog don't change: they start `nimbus` as always; an exit code of 75 still means "start
again", 0 still means "leave".

## Choosing

- **Automatic** (the default): Electron when there's an Nvidia graphics card (a display-class PCI device from vendor
  0x10de) and the Electron app is installed; WebKit otherwise. Qt is never picked by itself.
- Settings › Display › Renderer: Automatic, Electron, WebKit or Qt (Experimental), in the web page and in the Qt
  renderer's own Settings (`~/.config/nimbus/renderer`: `webkit`, `chromium` or `qt`; no file is Automatic). Hearth
  restarts to change it. The choices that aren't installed aren't offered.
- `HEARTH_RENDERER=webkit|chromium|qt|auto` wins over the setting (for testing).
- The Qt renderer's **Open in the web renderer** (on every "not here yet" card and in its Settings) writes
  `~/.config/nimbus/renderer-once` (`web`: automatic, but not Qt) and restarts: the next start only uses the web page;
  the setting stays Qt.

## When a renderer fails

- **Electron** that doesn't bring Hearth's screen up twice within 10 minutes (each start is recorded in
  `~/.config/nimbus/renderer-chromium-starts` until the page says it's ready): WebKit draws instead until it works again
  or the setting is changed.
- **Qt** that doesn't bring Hearth's screen up once (`renderer-qt-starts`, cleared when the Qt page says it's ready), or
  that crashes 3 times within 5 minutes (`renderer-qt-crashes`; a crash is the Qt app ending with anything but 0 or
  75, seen by `nimbus` waiting for it), is given up: the setting goes back to Automatic, and the next screen (whichever
  renderer draws it) shows a notice saying why (`renderer-note`, `take_note`). Hearth's log says so too.

Hearth's log (bridge.log) says which renderer it used (`renderer: …`), and why when it changed by itself.

## The Qt renderer (Experimental)

hosts/hearth-qml: a small C++ program (Qt 6.5 or newer: QtQuick, QtQuick.Shapes; SDL2 for controllers) and its QML
screens. It's built by CI inside each image's own base (`.github/scripts/hearth-qml-build.sh`, run by base-anatase.yml
and base-peppermint.yml), so it links that image's Qt (Fedora 45's on Anatase, Debian 13's on Peppermint).

Why C++ and plain Qt: Qt 6's development packages, SDL2 and CMake are in both distributions, the program is one small
binary on the Qt both images already have for Mantel (no Python runtime and PySide6 version to keep in step, no extra
Rust/C++ bridge like cxx-qt), and the QML side holds everything that's drawn. The C++ side only starts the core and
passes lines (`CoreLink`), reads controllers (`Pad`, SDL2's game controller API, as the web page's button names), and
logs (`Host`). All of Hearth's data and logic stays in the core; App.qml is the page side of the channel.

### What it has (phase 1)

- **Home**, the default layout: the last game's picture, Recent games (the newest big), Favorites, Recently added.
- **Library**: the grid with tabs (All, Installed, Favorites, each store, Hidden; LB / RB), A–Z or recently played,
  search; covers load as they come into view.
- **A game's page**: Play / Resume / Stream / Install / the download's progress, Exit game, Favorite, Uninstall, Cancel
  download; last played, play time, source, size; About and its location. The starting screen while it loads (Show it
  now, Quick Access).
- **Downloads**: the current download big (Pause, Resume, Cancel; Manage in Steam for Steam's), the speed, Up next,
  Recently installed.
- **Quick Access** (Guide, View, or Q): Now playing (Back to game, Exit game, its controller layout), Performance (CPU,
  GPU, memory, battery; frame-rate limit, power limit, power mode, the on-screen overlay) and Quick settings (volume,
  brightness, mute). Over a game it's the "Hearth Overlay" window on editions without Handheld Daemon; on Anatase and
  Peppermint Handheld Daemon's overlay app keeps showing the web Quick Access (from the Electron app's folder, or
  `HEARTH_UI_DIR`).
- **Power**: sleep, restart, shut down, sign out, switch to desktop.
- **Settings**: Display › Renderer, Open in the web renderer; System › About, **Send logs to the developer** (the usual
  report, which now has a Renderer section with the Qt renderer's own log), Restart Hearth; Everything else (the way to
  the web renderer).
- **Search** with Hearth's on-screen keyboard: the same keyboard as the web page's and Mantel's (hearth-osk.js and
  Mantel's OskBoard.qml, copied; docs/OSK.md), with its settings and learned words from the core (`osk.get`,
  `osk.learn`); suggestions from your games' titles. A physical keyboard types into the field.
- The controller everywhere (D-pad, left stick, A B X Y, LB / RB, ☰ menu, Guide), Hearth's focus ring, held directions
  repeat; the keyboard (arrows, Enter, Esc, Page Up / Down, `/` search, `m` menu, `q` Quick Access) and the mouse
  or touch too. Hearth's colors (your profile's accent and background theme), its icons (made from the web page's:
  `tools/gen-icons.py`) and the game art; headings in Anybody (from Hearth's website, SIL Open Font License) when CI
  could make its static cut.
- A "Qt renderer · Experimental" mark in the top bar, and a "Not in the Qt renderer yet" card on Home.

### What it doesn't have yet

Every one of these says so where you'd look for it (a card, a page, or a sheet with **Open in the web renderer**):
the Store, Friends, Screenshots, Co-op and Multi-seat, Hearth Link, the other Home and Library layouts, game settings
(Proton, launch options, mods, saves, achievements, ProtonDB), adding a game yourself, most of Settings (Wi-Fi,
Bluetooth, sound devices, controllers and Handheld Daemon, keyboard settings, profiles and accounts, updates,
accessibility...), notifications, the profile picker and lock screen, first-start setup, Quick Resume. What the core
asks for that needs a web page (a store's sign-in window, Xbox Cloud Gaming, a store's own pages, file pickers) is
answered "closed" at once, so the core never waits, and a sheet offers the web renderer.

### Logs

The Qt app's lines go to its standard error (the session keeps it in session.log) and to
`~/.local/share/nimbus/hearth-qml.log`; the screens also log through the core into bridge.log (`qt: …`): page
changes, Play, installs, renderer changes, the controllers SDL sees. Settings › System › Send logs to the developer
sends them all.

### CI

- hearth-qml.yml builds it in both bases, runs `hearth-qml --self-test` offscreen (QT_QPA_PLATFORM=offscreen, the
  software scene graph) against `hosts/hearth-qml/test/fake-core.py` (a stand-in core with sample games and the
  website's game art): every screen is opened, checked, and pictured; any QML warning fails it. It runs again in a fresh
  container with only the packages the images install. The pictures go to the branch `ci/qml-shots-<branch>-<base>`
  (a notice names it). It also checks that the copied keyboard files match their originals and Icons.js is current.
- base-anatase.yml and base-peppermint.yml build it into the image, and check it starts there (the same self-test,
  against the fake core and, reported only, against the image's own core).

## Roadmap for the Qt renderer

- **Phase 2**: notifications and the notification panel, the profile picker and lock screen, Quick Resume, Wi-Fi,
  Bluetooth and sound in Quick Access and Settings, controller and Handheld Daemon settings, adding games, game settings
  (Proton, launch options), achievements and ProtonDB on the game page, the other Library filters.
- **Phase 3**: the Store (its own pages through a QtWebEngine window, or the core's store API with native pages),
  sign-ins (a QtWebEngine window, or handing them to Hearth Browser), Friends and chats, Screenshots, Co-op and
  Multi-seat, Hearth Link.
- **Phase 4**: the other Home layouts (Xbox- and PS5-inspired) and Library layouts, first-start setup, the installer
  screens, the rest of Settings; Quick Access over games for Handheld Daemon's overlay app drawn by Qt.
- **Then**: when it has everything and holds up on the ROG Ally and on Nvidia, consider it for Automatic.

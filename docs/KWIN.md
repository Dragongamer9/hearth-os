# Mantel on KWin: every KDE Plasma 6 feature, Hearth's own way (1.14)

**What this file is.** The owner decided (Nov 16, 2026) that Desktop Mode's shell, Mantel, moves from labwc to
**KWin** (KDE's window manager and compositor), and that **every feature KDE Plasma 6 has that Mantel doesn't is
written from scratch**: Hearth's own code and Settings pages, not KDE's apps or background services. This is the
master checklist for that, so nothing is left out. It was made from KDE's own pages (Plasma 6.0 to 6.8 release
announcements, the System Settings module list, KWin's effects, KDE Connect's plugins, KRunner's runners, Dolphin,
Spectacle, Discover, KDE Wallet, Vaults, Activities, KRdp) and checked against Mantel's code at 81a2a1e.

**How to read it.**

- `- [x] feature — where Hearth has it (file)`: Hearth already does this fully.
- `- [ ] feature — Hearth has: …`: Hearth does part of it (what, and where). The line after the dash says what's left.
- `- [ ] feature`: Hearth doesn't have it yet.
- Every open item has one line on **how Hearth will do it** and a rough **size**: **S** (a day or less), **M** (a few
  days), **L** (a week or two), **XL** (several weeks).
- **An item is ticked only when it's built and tested.** When only CI has proven it, it's ticked with
  "(needs testing on a device)" after it, the same as docs/ROADMAP.md.
- "From scratch" means Hearth's own pages, panels and helpers. Like Plasma itself, they sit on the standard Linux
  services every desktop uses (KWin, PipeWire, NetworkManager, BlueZ, CUPS, systemd, UPower, fwupd, bolt,
  AccountsService, Polkit); those aren't KDE's and aren't rewritten.
- Settings red flag (CLAUDE.md rule 9) applies to every page here: KWin's own config (`kwinrc`, `kglobalshortcutsrc`,
  `kwinoutputconfig.json`, window rules) is written only through Hearth's settings helpers (merge-on-write, atomic,
  `.bak`, never defaults over a file that didn't parse), and nothing changes by itself.
- A KDE feature Hearth leaves out is never dropped silently: it's in **Not for Hearth?** at the end, for the owner to
  decide.

Counts at 81a2a1e (have · partial · missing): Stage 1 0 · 26 · 1, Stage 2 0 · 24 · 51, Stage 3 38 · 61 · 63.
Stage 1 has nothing ticked on purpose: Mantel does all of it today, but on labwc, and it counts only once it works on KWin.

---

## Stage 1: the KWin base

Desktop Mode starts KWin instead of labwc, Mantel runs on it, and everything Mantel does today keeps working. Most of
Mantel is ordinary QML and moves as it is; the parts below lean on labwc-only (wlroots) Wayland protocols that KWin
doesn't offer, so each needs a KWin way.

### Session and safety

- [ ] KWin session for Desktop Mode — Hearth has: the labwc session (`hosts/linux/desktop/labwc/autostart`, `environment`, nimbus-session). How: `kwin_wayland` started by nimbus-session with Hearth's environment, Mantel's Quickshell started by it and restarted like today (M)
- [ ] Falls back to labwc if KWin won't start, and to Game Mode if neither does — Hearth has: Quickshell keeps-closing check and Game Mode fallback (`labwc/autostart` `give_up`). How: the session tries KWin, counts quick failures, then starts today's labwc session with a notice; labwc stays installed (red flag 2) (M)
- [ ] Mantel's bar, dock, panels and desktop as layer-shell surfaces on KWin — Hearth has: the same on labwc (`shell.qml`, `Islands.qml`, `Dock.qml`). How: KWin speaks wlr-layer-shell; check exclusive zones, keyboard focus on panels and the frosted blur (`FrostBlur.qml`) on KWin (S)
- [ ] Xwayland apps and games — Hearth has: labwc's Xwayland. How: KWin's own Xwayland (on by default); Stage 2 has its options (S)
- [ ] Settings › Your desktop says KWin, and Safe start still works — Hearth has: `SetYourDesktop.qml` (Safe start). How: a line for "Mantel on KWin / Mantel on labwc (safe)" and Safe start picks labwc (S)

### Windows

- [ ] Window list for the taskbar, dock, centre pill, Task Manager, Discord and Snap layouts — Hearth has: Quickshell's `ToplevelManager` (wlr-foreign-toplevel) in `Bar.qml`, `Dock.qml`, `TaskStrip.qml`, `Islands.qml`, `Discord.qml`, `SnapLayouts.qml`, `TaskManagerView.qml`, `Pad.qml`, `Wallpaper.qml`. KWin doesn't offer wlr-foreign-toplevel. How: a small Hearth bridge (a Quickshell plugin or helper) on KWin's window-management protocol, or a KWin script that reports windows over D-Bus, giving Mantel the same list, app ids, icons, activate/minimize/close (L)
- [ ] Window actions from the taskbar (minimize, maximize, close, move to desktop, keep above, on all desktops, fullscreen) — Hearth has: labwc actions in the dock and taskbar menus (`Dock.qml`, `TaskStrip.qml`). How: through the same bridge (M)
- [ ] Window previews on hover in the dock/taskbar (Plasma shows live thumbnails) — How: KWin's window thumbnails through the bridge (M)
- [ ] Title bars (window decorations) in Hearth's look — Hearth has: labwc theme (`labwc/themerc-override`, `hosts/linux/desktop/theme`). How: a Hearth decoration for KWin (an Aurorae SVG theme drawn from Theme.qml's colours, kept in step with the accent and light/dark) (M)
- [ ] Title bar right-click menu with Hearth's items (Snap layouts…, Move to desktop, Always on top, Close) — Hearth has: labwc `menu.xml`. How: KWin's window menu plus a KWin script adding Hearth's items, or Hearth's own menu (PopMenu.qml) opened by a KWin shortcut (M)
- [ ] Alt+Tab window switcher in Hearth's look — Hearth has: labwc's thumbnail switcher in Hearth colours (`rc.xml` windowSwitcher). How: a Hearth task-switcher QML layout for KWin's TabBox (S)
- [ ] Snapping: drag to edges and corners, Super+arrows, halves and quarters — Hearth has: labwc snapping and regions (`rc.xml`). How: KWin's quick tiling with Hearth's shortcuts (S)
- [ ] Snap layouts (Super+Z, Maximize right-click) — Hearth has: `SnapLayouts.qml`, `snap-layouts.js` (labwc regions, wtype). How: the zones as KWin custom tiles, placed through a KWin script/D-Bus; KWin can tell when the pointer rests on Maximize, so hover can open it like Windows 11 (M)
- [ ] Virtual desktops: the bar's switcher, Super+Ctrl+arrows/1–4, move window to desktop — Hearth has: labwc's 4 desktops, `DeskSwitcher.qml`, `rc.xml`. How: KWin's VirtualDesktopManager over D-Bus; add/remove/rename in Stage 2 (M)
- [ ] Show desktop (Super+D) and minimize all (Super+M) — Hearth has: `hearth-desk`, the dock's strip. How: KWin's show-desktop over D-Bus (S)

### Shortcuts

- [ ] All of Mantel's ~90 shortcuts on KWin (Super, Super+Space/V/E/I/A/N/S/R/X/L/Z/., Super+1–9, Ctrl+Alt+T, Ctrl+Shift+Esc, Print keys, media/volume/brightness keys, magnifier keys…) — Hearth has: labwc `rc.xml` keybinds. How: registered with KWin's built-in global shortcuts, calling Mantel's IPC (`quickshell ipc …`) and `open.sh` as today (M)
- [ ] Super alone opens the launcher — Hearth has: labwc `Super_L`. How: KWin's "Meta key" modifier-only shortcut pointed at Mantel (S)

### Screens

- [ ] Resolution, refresh rate, scale, where each screen is, turn a screen on/off, keep-or-go-back — Hearth has: `SetDisplay.qml` with `wlr-randr`, `display.sh` at start (`labwc/autostart`), auto-scale by DPI. KWin doesn't take wlr-randr. How: Hearth's own helper on KWin's output-management protocol (or `kscreen-doctor`/`kscreenctl` while it's written), saving through KWin's own output config; Hearth's page stays (L)
- [ ] Rotation and orientation, primary screen, mirroring — Hearth has: rotation handled in size maths only (`SetDisplay.qml`). How: same helper, new rows (M)
- [ ] Screen off when idle, screen-off for Server mode — Hearth has: `wlopm` in `IdlePower.qml`, `hearth-server-mode`, `ServerDash.qml`. How: KWin's DPMS (its own protocol / D-Bus) and ext-idle-notify (S)
- [ ] Brightness keys and slider — Hearth has: `shell.qml`, `QuickAccess.qml` (backlight). How: unchanged for laptops; external screens over DDC/CI in Stage 2 (S)

### Lock, typing, screenshots

- [ ] Lock screen — Hearth has: `Lock.qml` (Quickshell `WlSessionLock`, ext-session-lock, PAM). How: check KWin accepts ext-session-lock from Quickshell; if it doesn't, Hearth's lock screen as a QML theme for KWin's locker or Hearth's own locker the owner picks (M)
- [ ] Typing for the on-screen keyboard, clipboard paste, emoji picker, snap layouts and the controller — Hearth has: `wtype` (virtual-keyboard protocol) in `Keyboard.qml`, `hearth-clipboard`, `hearth-pad`, `Pad.qml`, `SnapLayouts.qml`, `TaskStrip.qml`. KWin only lets an input method type. How: Hearth's OSK registered as KWin's input method (also gives text fields' own "show keyboard"), and a uinput typer for the rest (L)
- [ ] Screenshots (Print, Shift+Print, Super+Shift+S, Ctrl+Print to clipboard) — Hearth has: `grim` in `open.sh` (wlr-screencopy) with Spectacle as the fallback. KWin doesn't offer wlr-screencopy. How: Hearth's helper on KWin's ScreenShot2 D-Bus (full, area, window, to file and clipboard, notification) (M)
- [ ] Screen sharing for browsers, Discord, OBS (the ScreenCast portal) — Hearth has: xdg-desktop-portal-wlr (`os-bases/peppermint/build.sh`), which needs wlroots. How: Hearth's own portal backend for ScreenCast/RemoteDesktop on KWin's screencast protocol + PipeWire, with Hearth's picker (screen, window, area) (L)
- [ ] The controller in Desktop Mode (pointer, View+D-pad snap, View+B close, LB/RB windows) — Hearth has: `hearth-pad`, `Pad.qml` (labwc actions, wtype). How: the same through KWin's D-Bus and the window bridge (M)
- [ ] Night light, colour filters and magnifier keep working on day one — Hearth has: `wlsunset` (`QuickAccess.qml`), `hearth-gamma`, labwc's magnifier (`hearth-access`); none work on KWin. How: switched to KWin's own Night Light, colour filters and Zoom (pages in Stage 2), reading today's settings files so nothing changes for the owner (M)

Stage 1: have 0 · partial 26 · missing 1.

---

## Stage 2: what KWin itself can do, with Hearth Settings pages

KWin already has these. Hearth writes the pages (in the new Settings layout, both layouts A and B) and the shortcuts,
and writes KWin's config safely. "Page" below means a Hearth Settings page or section.

### Display and colour

- [ ] HDR in Desktop Mode (per screen) — Hearth has: Game Mode's HDR switch on `SetDisplay.qml` / `SetGamePerf.qml` (gamescope). How: a per-screen HDR switch on KWin's output settings, SDR brightness, colour intensity (S)
- [ ] HDR calibration (max/peak brightness wizard, like Plasma 6.4) — How: Hearth's own full-screen test pattern page writing KWin's HDR values (M)
- [ ] Extended Dynamic Range (HDR content on SDR laptop panels) — How: switch in Display (S)
- [ ] Variable refresh rate (FreeSync/G-Sync: never, always, automatic for full-screen games) — How: choice per screen (S)
- [ ] Colour profiles: ICC profile per screen, built-in (EDID) profile, colour accuracy choice — How: rows on Display, file picker for .icc (M)
- [ ] Colour depth limit (10/8/6-bit) and RGB range (full/limited) — How: advanced rows on Display (S)
- [ ] Fractional scaling that snaps to the pixel grid, per screen, and "legacy apps scale themselves / are scaled" for X11 apps — Hearth has: Scale row and auto-scale (`SetDisplay.qml`, `labwc/autostart`). How: KWin's per-screen scale plus the Xwayland scaling choice (S)
- [ ] Night light: sunset to sunrise by location, custom times, always on, temperature, fade length, Quick settings tile — Hearth has: wlsunset at a fixed 3700 K 19:30–07:00 with a Quick settings toggle (`QuickAccess.qml`, `SetDisplay.qml`). How: KWin's Night Light over D-Bus with a full page (times, place, warmth, preview) (M)
- [ ] Reduce red tint at low brightness (AMD laptops, Plasma 6.7) — How: switch on Display when the hardware supports it (S)
- [ ] Automatic brightness from the light sensor — How: KWin/ambient sensor (iio-sensor-proxy) switch on Display (S)
- [ ] Brightness for external screens (DDC/CI) — How: Hearth's helper with ddcutil, a slider per screen (M)
- [ ] Overscan, "allow tearing in full-screen games" (immediate presentation), and the screen's "Wide colour gamut" — How: advanced rows on Display and Gaming › Performance (S)
- [ ] Virtual desktops only on the main screen, or per screen (Plasma 6.6/6.7) — How: choice on the virtual desktops page (S)
- [ ] Screen edge barrier and corner barrier between screens (sticky edge) — How: slider on Display › several screens (S)
- [ ] Mirror / extend / only one screen quick switcher (Super+P) — How: Hearth's own OSD picker (M)

### Desktop effects (each a switch, with its settings, on a Hearth "Animations & effects" page)

- [ ] Animation speed (one slider for all) and Reduced motion respected — Hearth has: Reduce motion (`SetAccessibility.qml`) for Mantel. How: KWin's animation factor follows Reduce motion; slider on the page (S)
- [ ] Window open/close: Fade, Glide, Scale, Fall Apart — How: choice of one (S)
- [ ] Minimize: Magic Lamp, Squash — How: choice (S)
- [ ] Maximize animation, geometry-change animation, morphing popups, sliding popups — How: switches (S)
- [ ] Desktop switch: Slide, Fade, Cube animation — How: choice (S)
- [ ] Desktop Cube (the 3D cube of desktops) — How: switch + shortcut (S)
- [ ] Wobbly windows — How: switch + stiffness (S)
- [ ] Blur and background contrast behind Hearth's panels and translucent apps — Hearth has: its own frosted look (`Frost.qml`, `FrostBlur.qml`). How: let Mantel's panels ask KWin for blur (KWin's blur protocol) and a strength slider (M)
- [ ] Translucency (windows see-through while moving, inactive, menus) — How: switches + amounts (S)
- [ ] Dim inactive windows, dim behind password prompts — How: switches (S)
- [ ] Rounded corners and outlines for windows (Plasma 6.5 Breeze bottom corners, 6.8 window-rule outlines) — How: from Hearth's decoration and a rule for apps that draw their own (Steam) (M)
- [ ] Peek at desktop / Show desktop animation, login and logout fades — How: switches (S)
- [ ] Mouse click animation, mouse mark (draw on screen), track mouse (find the pointer), touch points — How: switches + shortcuts (S)
- [ ] Shake cursor to find it (pointer grows) — How: switch + size (S)
- [ ] Hide the pointer while typing or after a time — How: switch + delay (S)
- [ ] Snap helper / window aperture / thumbnail aside / show FPS / show paint (debug) — How: switches under "More" (S)
- [ ] Startup feedback (busy pointer or bouncing icon while an app opens) — How: choice (S)
- [ ] Visual bell (screen flashes instead of a beep) — How: Accessibility switch (S)

### Overview, switcher, desktops

- [ ] Overview (Super+W): all windows and desktops, search, drag windows between desktops — Hearth has: the launcher and the dock only. How: KWin's Overview effect, opened from Mantel (a bar button, a gesture) and shown in Hearth's colours; switchable to Hearth's own Overview later (M)
- [ ] Window view / Present windows: current desktop, all desktops, current app (Super+Ctrl+W etc.) — How: shortcuts on the page (S)
- [ ] Desktop grid (in Overview) — How: as above (S)
- [ ] Window switcher styles: Thumbnail grid, Large icons, Small icons, Text only, Thumbnails, Compact, Cover switch, Flip switch — Hearth has: one thumbnail style (labwc). How: a picker with Hearth's own QML layouts for each, plus settings (current desktop only, minimized windows, show desktop entry, order) (M)
- [ ] Second switcher (Alt+`, same app's windows) — How: second TabBox setting (S)
- [ ] Virtual desktops: add, remove, rename, rows, wrap around, switch animation, OSD with the desktop's name — Hearth has: 4 fixed numbered desktops (`rc.xml`, `DeskSwitcher.qml`). How: a Virtual desktops page on KWin's D-Bus (M)
- [ ] Different tile layouts per desktop — How: with the tiling editor below (S)

### Window management

- [ ] Tiling editor (Super+T: draw your own tile layout, drag windows into tiles with Shift) — Hearth has: six fixed snap layouts (`SnapLayouts.qml`). How: Hearth's own editor page that writes KWin's custom tiles, and Snap layouts reads them too (L)
- [ ] Window behaviour: focus follows mouse / click to focus / focus under mouse, raise on hover, click raises, focus stealing prevention — How: Window behaviour page (S)
- [ ] Title bar actions: double-click (maximize, shade, minimize…), wheel, middle-click; window actions with Super+drag (move), Super+right-drag (resize) — How: same page (S)
- [ ] Window placement for new windows (centred, smart, under the mouse, maximized) — How: choice (S)
- [ ] Snap zones: window edges stick to screen and other windows, gap sizes — How: sliders (S)
- [ ] Window rules (per app: size and place, desktop, keep above/below, no title bar, opacity, never minimise, open fullscreen, shadows/outline/corners, with "detect window") — Hearth has: nothing for users (`TaskManager.qml` mentions only labwc's own rule). How: Hearth's own Window rules page writing KWin's rule file, with a "Detect window" button and common presets (L)
- [ ] Window menu "More actions": keep above/below, on all desktops, shade, no border, fullscreen, special window settings — How: KWin's menu plus Hearth's (Stage 1) (S)
- [ ] Kill a frozen window (Ctrl+Alt+Esc, the skull pointer) and "not responding" dialog in Hearth's look — Hearth has: Task Manager's End task (`TaskManager.qml`). How: shortcut + KWin's kill helper (S)
- [ ] Session restore: reopen apps after restart (Plasma 6.1 on Wayland) — How: switch (on log out, restart; "start with an empty session") (M)
- [ ] KWin scripts shipped by Plasma: Minimize all, Desktop change OSD, video wall, sync skip-switcher with taskbar — How: Hearth's own as needed (S)
- [ ] Exclude a window from screen sharing (title bar menu, Plasma 6.6) — How: an item in Hearth's title bar menu (S)
- [ ] Picture-in-picture windows (Plasma 6.5 experimental protocol) — How: comes with KWin; check Hearth Player and browsers use it (S)

### Screen edges, gestures, input

- [ ] Hot corners / screen edges (each corner or edge: Overview, Show desktop, Lock, launcher, none), how long to push, re-activation delay, maximize by dragging to the top, quick tile by dragging to the side — Hearth has: three touch edge swipes (`EdgeSwipes.qml`: Quick settings, keyboard, launcher). How: a Screen edges page on KWin's edges, calling Mantel for Hearth's actions (M)
- [ ] Touchpad gestures: 3-finger swipe (switch desktops), 4-finger swipe up/down (Overview / Grid), 3-finger pinch to zoom (Plasma 6.4) — Hearth has: none for touchpads. How: KWin's gestures, listed and switchable on Mouse & touchpad (S)
- [ ] Touchscreen edge swipes and gestures (swipe from edges, Overview) — Hearth has: `EdgeSwipes.qml`. How: keep Hearth's, add KWin's for Overview/desktops (S)
- [ ] Touchpad: disable when a mouse is plugged in, middle-click emulation, scroll speed, edge scrolling, click method (areas vs clickfinger), disable while typing — Hearth has: speed, acceleration, natural scrolling, tap to click, scroll method, ignore while typing (`SetMouse.qml`). How: the missing rows through KWin's input config (S)
- [ ] Mouse: acceleration profile, scroll speed, middle-click paste, per-device settings — Hearth has: one set for all (`SetMouse.qml`). How: per-device list (S)
- [ ] Mouse buttons: re-map extra buttons (back/forward) to keys — How: KWin's button rebinding on Mouse page (M)
- [ ] Keyboard: Num Lock at start, layouts per window, compose key, typing delay — Hearth has: layouts, switch key, repeat (`SetKeyboard.qml`). How: rows through KWin's keyboard config (S)
- [ ] Touchscreen: which screen it belongs to, turn it off — How: KWin's touchscreen settings (S)
- [ ] Drawing tablets: which screen, area, aspect, left-handed, pressure curve and range, pen buttons, pad buttons, dials and touch rings, relative mode, calibration and test (Plasma 6.1–6.5) — Hearth has: nothing. How: Hearth's own Drawing tablet page on KWin's tablet settings, with a test canvas (L)
- [ ] Game controllers as regular mouse/keyboard input (Plasma 6.6 option) — Hearth has: its own controller mouse (`hearth-pad`). How: keep Hearth's; leave KWin's off (S)
- [ ] Sync the mouse and stylus pointers (Plasma 6.7) — How: switch on Drawing tablet (S)

### Accessibility (KWin's parts)

- [ ] Zoom (whole screen) with Mantel's keys, follow pointer / text cursor / keep pointer centred, pixel grid at high zoom — Hearth has: labwc's magnifier with Super+=/−/Esc and its settings (`SetAccessibility.qml`, `hearth-access`). How: KWin's Zoom, same keys and page, plus the follow modes (S)
- [ ] Magnifier (a lens around the pointer) — How: second choice on the same page (S)
- [ ] Colour filters in Desktop Mode: greyscale, protanopia, deuteranopia, tritanopia, intensity — Hearth has: these in Game Mode only; Desktop Mode does invert only (`SetAccessibility.qml`, `hearth-access`, `hearth-gamma`). How: KWin's colour-blindness correction effect, from the same setting (S)
- [ ] Invert colours (whole screen, one window) — Hearth has: whole screen via gamma (`hearth-gamma`). How: KWin's invert effect (+ Super+Ctrl+U per window) (S)
- [ ] Sticky keys, slow keys, bounce keys, toggle keys sound, mouse keys (move the pointer with the number pad) — Hearth has: nothing. How: KWin's accessibility keyboard options on Accessibility (M)
- [ ] Dwell click (click when the pointer stops, Plasma 6.8) — How: switch + delay (S)
- [ ] Shake cursor and Track mouse on Accessibility too — How: links to the effects (S)
- [ ] Activation sounds / visual bell — How: see effects (S)

### Xwayland and gaming

- [ ] Xwayland: let X11 apps read keys typed in other apps (for push-to-talk in Discord etc.: never / modifiers / all), X11 apps scale themselves — How: two rows under Gaming or Keyboard (S)
- [ ] Block compositing / "allow tearing" for full-screen games — How: switch on Gaming › Performance (S)

### Notifications and OSDs KWin shows

- [ ] Caps Lock / Num Lock and keyboard layout OSD — Hearth has: volume and brightness OSD (`VolumeOsd.qml`). How: Hearth OSDs for lock keys and layout (S)
- [ ] Microphone mute OSD and "app tried to use a muted microphone" warning (6.4) — How: OSD + notification (S)

Stage 2: have 0 · partial 24 · missing 51.

---

## Stage 3: written from scratch

The Plasma pieces that aren't KWin: KDE's widgets, apps and services. Each one is Hearth's own (Mantel QML, a Hearth
app under `hosts/`, or a Python/Rust helper).

### Panels and widgets

- [x] Floating panels, auto-hide, dodge windows, panel on any edge — `Islands.qml`, `Dock.qml`, `BarLayoutPicker.qml`, `SetBarDock.qml`
- [x] App launcher with search, favourites (pins), recent, categories, power actions — `Launcher.qml`, `MenuStyles.qml` (several menu styles)
- [x] Taskbar with pinned apps, grouping, jump-list menu, middle-click new window, drag to reorder, Super+1–9 — `Dock.qml`, `TaskStrip.qml`
- [x] System tray (SNI icons, menus, scroll, tooltips) — `Islands.qml`, `Bar.qml`
- [x] Clock with calendar, notification history and dashboard — `ClockPanel.qml`
- [x] Media player controls in the bar (MPRIS) — `MediaPill.qml`, `MediaCard.qml`, `MenuNowPlaying.qml`
- [x] Volume, Wi-Fi, Bluetooth, battery, brightness, power profile in Quick settings — `QuickAccess.qml`
- [x] Desktop clock and weather on the wallpaper — `DeskClock.qml`, Settings › Personalization
- [x] Show desktop — `Dock.qml`, `hearth-desk`
- [ ] Edit mode: add, move, resize and remove widgets on the desktop and in the bar (Plasma's widget explorer) — Hearth has: fixed bar layouts and a few desktop widgets. How: Mantel's own widget host: a widget catalogue, drag onto the desktop or bar, sizes, per-widget settings saved in desk.json through the settings helpers (XL)
- [ ] More panels (several bars, on several screens, clone a panel) — Hearth has: one bar + dock. How: after edit mode, a bar per screen/edge (L)
- [ ] Widgets Plasma ships: Sticky notes on the desktop — Hearth has: Hearth Notes app (`hosts/hearth-time`). How: a desktop note widget over the same notes (M)
- [ ] Widget: system monitor sensors (CPU, memory, network, disk graphs on desktop/bar) — Hearth has: Resource Monitor, Task Manager, dashboard system box (`ResourceMonitor.qml`, `ClockPanel.qml`). How: sensor widgets fed by hearth-resmon (M)
- [ ] Widget: analog/binary/fuzzy clocks, world clocks with time-zone offsets — Hearth has: digital desktop clock. How: clock widget styles + extra zones in the clock panel (S)
- [ ] Widget: alternative calendars (Islamic, Chinese/Vietnamese lunar, Hebrew, Bikram Sambat, astronomical) and holidays in the calendar — How: calendar plugins in the clock panel (M)
- [ ] Widget: weather report (forecast, feels like, sources) — Hearth has: weather in Quick settings, dashboard, Hearth Weather (`hosts/hearth-time`). How: a desktop/bar widget (S)
- [ ] Widget: folder view (a folder's files on the desktop as a widget) — Hearth has: Desktop folder icons (`Desktop.qml`). How: folder widget reusing Desktop.qml (M)
- [ ] Widget: picture frame / slideshow, web page, timer, calculator, colour picker, dictionary, disk quota, keyboard indicator, comic strip, fifteen puzzle — How: one Mantel widget each, smallest first (M in all)
- [ ] Widget: window list (all windows, by desktop), pager with window outlines, activity pager — Hearth has: `DeskSwitcher.qml` pills. How: pager widget on the window bridge (M)
- [ ] Widget: printers queue in the tray (job badge, queue) — Hearth has: Settings › Printers. How: tray item with jobs (S)
- [ ] Widget: Disks & devices (mount/unmount/eject new drives, check and repair) — Hearth has: Hearth Disks app, Files' drives. How: tray item on plug-in (M)
- [ ] Widget: camera/microphone/screen-recording "in use" indicator — How: tray icons from PipeWire's active streams (S)
- [ ] Widget: background apps in the tray (Plasma 6.7) — How: list of apps running without windows (portal Background) (S)
- [ ] Widget: quick light/dark switch (6.7) — Hearth has: Mode in Personalization. How: a Quick settings tile (S)

### Search (KRunner-style)

- [x] Search apps, Settings pages and single settings, games — `Launcher.qml`, `menu-search.js`, `settings-index.js`
- [x] Run box (Super+R) — `WinTools.qml`, `RunView.qml`, `run-map.js`
- [ ] KRunner-style search bar (Alt+Space) with plugins and an order you choose, help with "?" — Hearth has: the launcher's search. How: plugins in the launcher's search with a Search page to turn each on/off and order them (L for the framework)
- [ ] Plugin: calculator (`=` and plain sums), unit conversion, currency (ECB rates) — How: Hearth's own parser (Hearth Calculator's code), rates cached daily (M)
- [ ] Plugin: files and folders by name and content — How: with the file indexer below (M)
- [ ] Plugin: recent files and places, bookmarks (browser), browser history — How: recent-files list + Firefox/Chrome bookmark readers (M)
- [ ] Plugin: open windows, virtual desktops, activities by name — How: on the window bridge (S)
- [ ] Plugin: run a command, terminal command, kill an app by name — Hearth has: Run box. How: search plugins (S)
- [ ] Plugin: web search keywords (gg:, wp:, yt:, custom) — How: plugin + list on the Search page (S)
- [ ] Plugin: date/time in other cities, time-zone conversion — How: plugin (S)
- [ ] Plugin: spell check, dictionary, special characters (U+ codes), emoji, colours (hex/CSS preview) — Hearth has: emoji picker (`hosts/hearth-emoji`). How: plugins (S each)
- [ ] Plugin: power and session (sleep, lock, log out, restart, switch user), media control, Discover/store results for apps not installed — How: plugins (S)
- [ ] Plugin: Hearth contacts, calendar events, notes — How: plugins on Hearth Contacts/Time data (M)
- [ ] Type on the desktop to search (or to select a file by name, 6.7) — How: Desktop.qml key handler (S)

### Notifications

- [x] Notification popups, history, Do Not Disturb, mute an app, Clear all — `Notifications.qml`, `ClockPanel.qml`, `SetNotifications.qml`
- [ ] History kept across restarts (Plasma keeps it) — Hearth has: this session only (`DeskState.history`). How: saved list in the data folder, capped (S)
- [ ] Per-app notification settings (show popups, in history, sound, badge on the dock, urgent only) — Hearth has: mute an app. How: an app list on Notifications (M)
- [ ] Automatic Do Not Disturb in full-screen apps/games and while screen sharing, with missed count after — Hearth has: manual DND (urgent still shows). How: rules on Notifications (S)
- [ ] Do Not Disturb on a schedule, and its shortcut — How: rows (S)
- [ ] Progress of jobs and file transfers in notifications (with speed graph) — Hearth has: Hearth Files' own progress. How: org.kde.JobViewServer-compatible job list in Mantel (M)
- [ ] Reply from a notification (inline reply), actions kept in history — How: in Notifications.qml (S)
- [ ] Badges and progress on dock icons (unity launcher API) — How: Dock.qml (S)
- [ ] Notification sounds and a sound theme for system events — Hearth has: none. How: Hearth's own sound set + player for events (login, plug-in, error, notification) with a Sounds page (M)

### Clipboard

- [x] Clipboard history (Super+V), pictures, pinned items, Clear all keeping pinned, secrets skipped — `Clipboard.qml`, `hearth-clipboard`, `SetClipboard.qml`
- [ ] Clipboard actions (a copied URL offers "open in…", QR code of an item, edit an item) — How: menu items in Clipboard.qml (S)
- [ ] Keep the selection (middle-click paste) history separately, and sync selection with clipboard — How: options on Clipboard (S)

### Phone link (KDE Connect phone app)

- [x] Pair with the KDE Connect Android app, battery, ping, find my phone, clipboard both ways, media control, send/receive files, phone notifications with reply and actions, remote input (phone as touchpad/keyboard) — `hosts/hearth-connect` (packet types `kdeconnect.pair`, `.battery`, `.ping`, `.findmyphone`, `.clipboard`, `.mpris`, `.share`, `.notification`, `.mousepad`) (needs testing on a device per its roadmap entry)
- [ ] SMS/MMS: read conversations and send texts from the PC — How: `kdeconnect.sms.*` in hearth-connect + a Messages window (L)
- [ ] Phone calls: incoming call notification, mute/pause media during calls — How: `kdeconnect.telephony` (S)
- [ ] Contacts sync from the phone (vCards) into Hearth Contacts — How: `kdeconnect.contacts` (S)
- [ ] Browse the phone's files in Hearth Files (SFTP) — How: `kdeconnect.sftp` + a Files place (M)
- [ ] Run commands (PC commands the phone can trigger) — How: `kdeconnect.runcommand` + a list in Settings (S)
- [ ] Phone controls PC volume, lock, presentation remote (slides, laser pointer) — How: `kdeconnect.systemvolume`, `.lock`, `.presenter` (M)
- [ ] PC sends its notifications to the phone; PC as a remote keyboard for the phone — How: `kdeconnect.notification` outgoing, `.mousepad` outgoing (M)
- [ ] Phone's signal/connectivity report, stop screensaver while connected, share text and URLs — How: `kdeconnect.connectivity_report`, inhibit, `.share` text/url (S)
- [ ] Phone as a second screen (virtual monitor) and over Bluetooth — How: later; `kdeconnect.virtualmonitor`, BT backend (L)
- [ ] Phone tray widget and Settings page (devices, plugins on/off per device, rename) — Hearth has: Hearth Connect's own window. How: tray item + page (M)

### Remote desktop and screen recording

- [x] Use this PC from anywhere (Moonlight through Tailscale, Sunshine) — `hearth-remote-desktop`, `SetRemote.qml`, docs/REMOTE-DESKTOP.md
- [ ] RDP server (connect with any RDP client, system accounts, clipboard both ways, exclusive mode with a virtual screen; Plasma's KRdp) — Hearth has: Sunshine/Moonlight only. How: Hearth's RDP server on the remote-desktop portal (Stage 1) + FreeRDP's server library, a page with users and passwords (XL)
- [x] Screen recording (whole screen, area, game), instant replay, sound recorder — `hosts/hearth-recorder`
- [ ] Record a single window, record popups, record system sound and the microphone together, choose format/quality — Hearth has: screen/area/game with microphone (`hosts/hearth-recorder`). How: window source on the Stage 1 ScreenCast backend, audio mix (M)
- [ ] Screenshot tool like Spectacle: area, window, active window, delay, include pointer, annotate (arrows, blur, text, numbers), copy/save/share, text recognition (OCR), scrolling capture — Hearth has: grim shortcuts + Hearth Paint for marking up (`open.sh`, `hosts/hearth-paint`), and Spectacle itself as "Hearth Capture" (KDE's app, docs/KDE-APPS.md). How: Hearth's own capture overlay in Mantel on KWin's ScreenShot2, annotation from Hearth Paint, OCR with Tesseract (L)

### Files (Dolphin's features)

- [x] Icons/compact/details views, tabs, places, Trash, previews, filter as you type, search, hidden files, compress/extract, batch rename, Open in Terminal, network shares, drag and drop, undo, details pane, properties — `FileManager.qml`, `hearth-files`, `FileProperties.qml`
- [ ] Split view (two folders side by side, F3) — How: second pane in FileManager.qml (M)
- [ ] Terminal panel inside Files (F4) — Hearth has: Open in Terminal. How: an embedded Hearth Terminal pane (M)
- [ ] Tags, ratings and comments on files, search by them — How: xattrs (`user.xdg.tags`) + a Tags place (M)
- [ ] Group by (type, date, size) — Hearth has: sort. How: grouped views (S)
- [ ] Git status badges in folders — How: `git status --porcelain` per folder (S)
- [ ] Act as administrator (edit system files with a password prompt) — How: hearth-admin backed actions (M)
- [ ] Checksums in Properties (MD5/SHA), permissions and owner tab — Hearth has: helper computes sha (`hearth-files`). How: Properties tabs (S)
- [ ] Mount an ISO, phones over MTP, cameras (PTP) — How: udisks loop mount, gio mtp (M)
- [ ] Service menus (right-click actions apps add) — How: read `.desktop` ServiceMenus/Nautilus-style actions (M)

### File search and indexing

- [ ] File indexing (names and contents, like Baloo) with folders to skip, battery/idle limits — Hearth has: nothing system-wide (Files searches by walking folders; Photos indexes pictures). How: Hearth's own indexer (Rust: inotify + a SQLite FTS index, text extractors for PDF/office/media tags), a File search page (XL)
- [ ] Recent files tracking (and "forget recent files", privacy) — How: recently-used.xbel + Privacy page (S)

### Passwords, wallet, vaults

- [ ] Password store (KDE Wallet): Secret Service for apps, unlocked at login (PAM), a manager page to view/edit/remove, several wallets — Hearth has: Passwords & Keys viewer on GNOME Keyring/KWallet (`hosts/hearth-tools/src/secrets.rs`). How: Hearth's own Secret Service daemon (org.freedesktop.secrets) with an encrypted store, PAM unlock, and Passwords & Keys as its manager (XL)
- [ ] SSH and GPG key passphrases asked in Hearth's look (ksshaskpass) — How: Hearth askpass helper + pinentry (S)
- [ ] Vaults: encrypted folders (gocryptfs/CryFS), lock/unlock in the tray, offline-only vaults, auto-lock — Hearth has: Briefcase with gocryptfs locking (`hearth-briefcase`, `Briefcase.qml`). How: Vaults as their own tray item and Files place on the same code, "lock when idle", "only when offline" (M)
- [ ] Password prompts for admin actions (the Polkit agent) in Hearth's look — Hearth has: KDE's/GNOME's agent (`hearth_tasks_base.py` lists `polkit-kde-authentication-agent-1`). How: Hearth's own Polkit agent in Mantel (dims the screen, fingerprint too) (M)

### Activities

- [ ] Activities: separate sets of windows, wallpaper, widgets and favourites (Work, Gaming…), switch with Super+Q, apps/files remembered per activity, privacy per activity — Hearth has: nothing. How: Hearth's own activity service (a list in desk.json, windows tagged through a KWin script, the dock/launcher/wallpaper/recent files following the activity), a switcher panel (XL)

### App store and updates

- [x] App store (Flathub, Debian packages) — Bazaar + `SetApps.qml`, `DebInstaller.qml`, docs/STORE.md
- [x] System updates with the previous version kept — `SetUpdates.qml`, bootc, docs/UPDATES.md
- [ ] Discover's extras: update notifications with "install from the notification", app reviews and ratings, sandbox permission changes on update, verified/third-party label, flatpak+https links, install fonts, drivers — Hearth has: Bazaar's own. How: Hearth's update notifier + store pages as needed (L)
- [ ] App permissions page (Flatpak permissions per app: files, devices, network, background, portals) — How: Hearth's page on `flatpak override` (M)
- [ ] Firmware updates (fwupd: BIOS, devices, docks), with a notification — Hearth has: `fwupd` named only in `hearth-tasks`. How: Hearth's page on fwupd's D-Bus (list, update, history) (M)
- [ ] Firmware security (HSI level, Secure Boot, TPM, IOMMU) in About — How: fwupd's security attributes on About (S)
- [ ] Shut down or restart after installing updates — How: row in Updates (S)
- [ ] Backups of your files (Plasma 6.8 ships Kup) — Hearth has: Hearth Link backups and saves, home-server backups (`SetHomeBackups.qml`, `SetLink.qml`). How: a "Back up this PC's folders to a drive" page on the same backup code (M)

### Network

- [x] Wi-Fi networks, hidden networks, forget, properties, hotspot — `SetWifi.qml`, `DeviceSettings.qml`
- [x] Tailscale (reach your devices) — `SetHomeTailscale.qml`
- [ ] Wired, mobile broadband (SIM, APN), and editing any connection (IPv4/IPv6, DNS, static address, metered, MAC randomisation, 802.1X/enterprise Wi-Fi) — Hearth has: Wi-Fi connect/properties. How: Hearth's own connection editor on NetworkManager (L)
- [ ] VPN: WireGuard, OpenVPN, IPsec/StrongSwan (EAP-TTLS), L2TP, PPTP, OpenConnect, SSTP, Fortinet; import a .ovpn/.conf — How: VPN page on NetworkManager's VPN plugins, with import (L)
- [ ] Wi-Fi QR codes: show one with the password, connect by scanning one (with Hearth Camera) — How: QR on Wi-Fi properties; scanner in Hearth Camera (S)
- [ ] Proxy settings (system-wide, PAC) — How: page writing the portal/gsettings/env proxy (S)
- [ ] Firewall page (on/off, rules, per app) — Hearth has: firewall rules used internally (`hearth-remote-desktop`, `SetHomeSafety.qml`). How: Hearth's page on firewalld/nftables (M)
- [ ] Duplicate a connection, network speed in the tray — How: rows (S)
- [ ] Windows shares: browse and mount SMB shares, share a folder — Hearth has: Files' SMB, Hearth Link sharing (`hearth_netshares.py`, `SetLink.qml`). How: "Share this folder" in Files' Properties (S)

### Online accounts

- [ ] Online accounts (Google, Nextcloud/ownCloud, Microsoft, IMAP, CalDAV/CardDAV) giving calendars, contacts, files and mail to apps — Hearth has: CardDAV in Hearth Contacts (`hosts/hearth-contacts`). How: Hearth's own accounts service (OAuth through the browser, tokens in Hearth's password store), used by Contacts, Calendar, Files (Nextcloud WebDAV) (XL)

### Accounts, sign-in, login

- [x] Users: add, remove, account type, picture, password — `SetUsers.qml`
- [x] Sign in automatically, lock after sleep — `SetSignin.qml`
- [ ] Fingerprint sign-in and fingerprint+password together (fprintd), enrol fingers — How: Hearth's enrol page on fprintd, PAM for login/lock/polkit (M)
- [ ] Face / picture cropping for the avatar (6.2) — How: crop box on Users (S)
- [ ] Login screen settings (theme, background, auto-login user, Hibernate/Sleep on it) — Hearth has: SDDM (KDE's) with Hearth's settings (`SetSignin.qml`, `SetYourDesktop.qml`). How: Hearth's own login screen (a greetd greeter in QML with Mantel's look) and its page; SDDM kept as fallback until it's proven (XL)
- [ ] First-run account setup after install (Plasma Setup, 6.6) — Hearth has: Game Mode's first-time setup (docs/HEARTH-GAME-MODE.md). How: already Hearth's; check it covers making the account on pre-installed PCs (S)
- [ ] Switch user (a second person signed in at once) and "log out" choices (also "next restart goes to the boot menu") — How: power menu items, logind seats (M)

### Region, language, time

- [x] Date & time: time zone, automatic time, 12/24-hour — `SetDateTime.qml`
- [x] Keyboard layouts — `SetKeyboard.qml`
- [ ] Language of the system (with download of translations), several languages in order — How: Region & language page writing locale (M)
- [ ] Formats: numbers, currency, dates, measurement units, paper size, first day of the week — Hearth has: the calendar follows the locale's first weekday (`ClockPanel.qml`). How: formats page (LC_* overrides) (M)
- [ ] Spell check settings (default language, auto-detect, ignore words) for apps — Hearth has: OSK word suggestions (`osk/`). How: a page writing hunspell/enchant settings (S)
- [ ] Input methods for Chinese/Japanese/Korean (fcitx5 in Plasma's virtual keyboard setting) — How: Hearth's page to pick and set up fcitx5, KWin input method (M)

### Look

- [x] Dark / light / automatic by time of day, accent colour (presets, from wallpaper, custom) — `SetAppearance.qml`, `Theme.qml`
- [x] Wallpaper: pictures, moving wallpapers, your own picture — `Wallpaper.qml`, `SetAppearance.qml`
- [x] Boot splash choice — `SetBoot.qml`, `SetBootFrames.qml` (Plymouth)
- [x] Fonts viewer, install and remove fonts — `hosts/hearth-fonts`
- [ ] Wallpaper per screen, slideshow (folder, interval, order), picture of the day, wallpaper that changes with light/dark — Hearth has: one wallpaper for all screens; light/dark for looks. How: rows on Personalization (M)
- [ ] Save your own look as a theme (global theme), light/dark/twilight versions — Hearth has: eight looks and sub themes (`SetAppearance.qml`, `subthemes.js`, `MantelLookExport.qml`). How: "Save as my look" (S)
- [ ] System fonts: UI font, monospace, small, title bar, sizes, anti-aliasing, hinting, sub-pixel, force DPI — Hearth has: text size in Accessibility, the Look's font. How: a Fonts page (fontconfig + Qt/GTK settings) (M)
- [ ] Cursor theme and size (also follows into apps), cursor preview — Hearth has: Large pointer switch (`SetAccessibility.qml`). How: cursor page with Hearth's cursor set and others installed (S)
- [ ] Icon theme choice — Hearth has: Hearth's own icon set only. How: page listing installed icon themes (S)
- [ ] App style and colours for Qt and GTK apps (window frame colour intensity, GTK theme sync) — Hearth has: `hosts/hearth-apptheme`, qt6ct Hearth palette, `theme/apply.sh`. How: page rows for frame colour intensity and "apps follow Hearth" (S)
- [ ] Splash at sign-in (Plasma's KSplash) — Hearth has: boot splash and startup animation. How: a short Mantel sign-in fade (S)
- [ ] Keyboard RGB lights follow the accent (6.1) — Hearth has: handheld lights through HHD (`SetHandheld.qml`). How: OpenRGB-style helper for common keyboards (M)

### Sound

- [x] Volume, mute, over-amplification, output and input devices, per-app volume mixer — `SetSound.qml`, `Mixer.qml`
- [ ] Device profiles (stereo/surround/HDMI, headset modes), balance per channel, speaker test — How: rows on Sound via PipeWire (M)
- [ ] Microphone test (hear yourself / level, 6.7), push-to-talk mute shortcut — How: Sound page (S)
- [ ] Move one app's sound to another device (in the mixer) and per-app input — How: Mixer.qml (S)
- [ ] Scroll on an app's dock icon to change its volume (6.6) — How: Dock.qml (S)
- [ ] Warning when "raise maximum volume" stays on — How: notice (S)

### Power

- [x] Power profiles, screen-off and sleep timers, power button, low battery actions — `SetPower.qml`, `IdlePower.qml`, `BatteryWatch.qml`
- [ ] Different settings on AC, on battery, on low battery; lid close action; dim screen before off — How: profiles on Power (M)
- [ ] Battery charge limit (stop at 80 %) — How: sysfs `charge_control_end_threshold` through hearth-admin (S)
- [ ] Apps blocking sleep, shown and overridable (6.2), Hibernate and hybrid sleep — How: list on Power from logind inhibitors (S)
- [ ] Battery health and cycle count, energy graphs (Info Center's Energy) — Hearth has: Resource Monitor. How: UPower history on Power (M)
- [ ] Batteries of Bluetooth devices, mice and controllers in the tray — Hearth has: Bluetooth list. How: UPower devices in Quick settings (S)

### Devices and hardware

- [x] Bluetooth: on/off, pair, connect — `SetBluetooth.qml`
- [x] Printers: add (network, USB, address), default, test page, remove — `SetPrinters.qml`
- [x] Game controllers: test and calibrate — Controller Lab in `hosts/hearth-gaming`, `SetController.qml`
- [ ] Bluetooth: send/receive files (OBEX), device battery, rename, trust/block — How: rows on Bluetooth (M)
- [ ] Printers: print queue window, jobs badge, ink/toner low notification, Windows shared printers, warning on unavailable printer — How: Hearth's queue window and notices (M)
- [ ] Scanners: scan to PDF/picture, several pages — Hearth has: Printers & Scanner in `hosts/hearth-tools`. How: check it covers multi-page PDF and feeder scanners, add what's missing (S)
- [ ] Thunderbolt devices: allow/block, always allow (bolt) — How: page on boltd's D-Bus (S)
- [ ] Removable storage: what to do on plug-in (open Files, import photos, nothing), automount — How: page + udisks watcher (S)
- [ ] USB device permission prompts (6.6) — How: Hearth prompt on the USB portal (M)
- [ ] Touchscreen calibration — How: on Display/Touchscreen (S)

### Startup, session, defaults

- [x] Default apps per kind — `SetDefaults.qml`
- [ ] Autostart page: apps and scripts at sign-in, toggle without deleting (6.8), add your own — Hearth has: XDG autostart is honoured (Task Manager's startup list, `hearth_tasks_procs.py`). How: an Autostart page (S)
- [ ] Background services page (what runs for the desktop, on/off) — Hearth has: Task Manager › Services. How: link from Settings (S)
- [ ] File associations editor (every file type → app, beyond the kinds) — Hearth has: kinds only (`SetDefaults.qml`). How: an advanced list (M)
- [ ] Desktop session: "on login, restore last session / start empty", confirm logout, default leave option — How: rows on Startup (S)
- [ ] Menu editor (edit launcher entries, icons, hide apps, 6.4 KMenuEdit) — How: "Edit app…" in the launcher's menu writing local .desktop files (M)

### Info and system

- [x] About this PC and hardware details — `SetAbout.qml`, `ResourceMonitor.qml`, `DeviceManagerView.qml`
- [x] Task manager / system monitor (processes, GPU, history) — `TaskManager.qml`, `hearth-tasks`
- [x] Disk manager and SMART health — `hosts/hearth-disks`, `SetHomeDrives.qml`
- [ ] Info Center pages Hearth lacks: OpenGL/Vulkan/Wayland details, kernel modules, interrupts, PCI/USB trees, Samba status, sensors — Hearth has: many in Device Manager and Resource Monitor. How: a "Technical details" section (M)
- [ ] Out-of-memory notification with tips (6.3) — Hearth has: Memory settings (`SetMemory.qml`). How: notice when systemd-oomd/earlyoom kills (S)
- [ ] Welcome app on first sign-in and after updates (Plasma Welcome) — Hearth has: Welcome & Tips (`hosts/hearth-tools`), Live USB welcome. How: open it once per new version (S)

### Hearth-specific extras KDE has that Hearth's own apps may want

- [ ] Emoji picker extras: mixed skin tones, sizes (6.7/6.8) — Hearth has: `hosts/hearth-emoji` with skin tones. How: options (S)
- [ ] On-screen keyboard press-and-hold for special characters (6.7) — Hearth has: accents in Hearth's OSK (docs/OSK.md). How: check press-and-hold opens them on touch, add it if not (S)

Stage 3: have 38 · partial 61 · missing 63.

---

## Not for Hearth? (the owner decides)

KDE features that look like they don't fit Hearth, and why. Nothing here is dropped until the owner says so.

- **X11 session**: Plasma 6.8 retired it; Hearth is Wayland-only (Xwayland for old apps).
- **KDE Store downloads (Get New…: widgets, KWin effects and scripts, themes, wallpapers)**: they're KDE-format add-ons that wouldn't fit Mantel; Hearth could have its own add-on catalogue later.
- **Global menu (app menus in the top bar)**: few apps still export menus on Wayland; costs bar space.
- **KDE User Feedback / telemetry and the yearly donation request**: Hearth has its own anonymous player count.
- **Plasma Mobile, Plasma Bigscreen**: Hearth has Hearth for Android and Hearth TV.
- **Union / Breeze / Oxygen / Air app styles and Aurorae theme choice**: Hearth's look is its own; one Hearth style plus the looks.
- **Audio CDs, CDDB, KIO "Browser identification", cookies, cache (Konqueror-era network pages)**: no CD drives on target hardware; browsers handle the rest.
- **Konsole profiles and Kate sessions widgets/search plugins**: Hearth Terminal and Hearth Text are Hearth's own; their sessions could be plugins later.
- **Comic strip, Fifteen puzzle, Dictionary, Disk quota widgets**: listed in Stage 3's widget line; likely not wanted.
- **Remote desktop client (KRDC)**: Moonlight is already in; an RDP/VNC client could be a Hearth app later.
- **KDE Wallet's several named wallets and its separate KWalletManager app**: Hearth's password store would keep one store per user, unlocked with the sign-in password.
- **Activities' per-activity power settings and the Activity pager**: only if Activities is built.
- **Keyboard LED colour sync**: depends on rare keyboard support; listed in Stage 3 in case.
- **Plasma Vault's CryFS/EncFS backends**: Hearth uses gocryptfs only (EncFS is insecure, CryFS is slower).
- **Plasma's desktop scripting (JavaScript to set up panels)**: Hearth's settings files do this.
- **KDE's own login manager (Plasma Login Manager, 6.6) and SDDM themes**: replaced by Hearth's own login screen (Stage 3).
- **Info Center's X-Server page, Window Manager page, and System Settings' "Compositor" backend choice (OpenGL/QPainter)**: KWin-on-Wayland only.

## Notes for whoever builds this

- KWin lacks wlroots' protocols Mantel uses today: **wlr-foreign-toplevel** (window list), **wlr-screencopy** (grim),
  **wlr-gamma-control** (wlsunset, hearth-gamma), **wlr-output-management** (wlr-randr), **wlr-output-power**
  (wlopm) and the **virtual-keyboard** protocol for apps (wtype). Stage 1's items replace each one; labwc keeps them
  for the fallback, so both paths stay working.
- The ScreenCast portal (xdg-desktop-portal-wlr) won't work on KWin; without Hearth's own backend, screen sharing in
  browsers and Discord breaks on day one. That's in Stage 1 for that reason.
- Every new page follows the new Settings layout (docs/mockups/checklists/settings-new.md) and, where the owner
  approves a mockup, red flag 11 (built completely, checklist in docs/mockups/checklists).

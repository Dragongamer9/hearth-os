# Every KDE Plasma feature in Mantel on KWin: where 1.14 stands

The master checklist (feature/kde-feature-list's KWIN.md, made at 81a2a1e) checked item by item against
**feature/kwin-1.14 at 00ced87** (Nov 16, 2026). The owner's rule: everything KDE Plasma 6 has that Mantel lacks is
written for Hearth from scratch.

**How to read it.**

- `- [x] part — file: "proof"`: built, with a test or CI picture that shows it. "(needs testing on a device)" when only
  CI has proven it, as in docs/ROADMAP.md.
- `- [ ] part — partly: …`: some of it is built (what, and where); the line says what's left.
- `- [ ] part`: not built on feature/kwin-1.14.
- Sizes for what's left: **S** (a day or less), **M** (a few days), **L** (a week or two), **XL** (several weeks).
- **Not merged yet** (so not counted here): `feature/kwin-pages` (Settings pages for KWin's own settings: effects,
  virtual desktops, tiling, screen edges, window rules, drawing tablet and touchscreen, games and X11 apps, KWin's
  parts of Mouse & touchpad and Accessibility, and `hearth-kwin-settings`) and `feature/search-store` (KRunner-style
  search, a file index, Passwords like KDE Wallet, Hearth's own App store, Files' terminal panel and split view). Items
  they would cover say so.

Counts (done · partly · missing): Stage 1 19 · 6 · 3 (one line added by this audit), Stage 2 3 · 16 · 56, Stage 3 58 · 24 · 80.

---

## Stage 1: the KWin base

### Session and safety

- [x] KWin session for Desktop Mode — hosts/linux/session/nimbus-session: "run_kwin … exec "$1" --xwayland $3 $HEARTH_KWIN_ARGS "$2"", hosts/linux/desktop/kwin/autostart; test_kwin_session.py, kwin-test.yml (needs testing on a device)
- [x] Falls back to labwc if KWin won't start, and to Game Mode if neither does — nimbus-session `desktop_wm` / `run_classic` (missing, ends within 15 s, Mantel not up in 45 s, twice → labwc until turned back on); test_kwin_session.py: "test_kwin_dying_fast_opens_labwc_counts_and_twice_stays", "test_mantel_never_up_ends_kwin_and_opens_labwc" (needs testing on a device)
- [ ] Mantel's bar, dock, panels and desktop as layer-shell surfaces on KWin — partly: they show with their exclusive zones under KWin (kwin-windows.yml pictures "kwin-halves", "kwin-desktop-menu"); keyboard focus on each panel (launcher, Quick Access, Settings) under KWin isn't checked anywhere. Left: a KWin check that types into each panel (S)
- [x] Xwayland apps and games — nimbus-session: "--xwayland"; kwin-windows.yml "KWin's X server for the X11 window" (36d3f1a) (needs testing on a device)
- [x] Settings › Your desktop says KWin, and Safe start still works — SetYourDesktop.qml: "KWin didn't start this time, so labwc", "Safe start"; hearth-your-desktop; test_kwin_session.py "test_wm_writes_only_its_line_with_a_backup", "test_safe_graphics_is_labwc" (needs testing on a device)
- [ ] **(found in this audit)** Keyboard layouts, key repeat and pointer settings applied under KWin — labwc/autostart:35 runs `hearth-kwin-settings pointer-apply` / `input-watch` only if the file is there, and it isn't on feature/kwin-1.14 (it's on unmerged feature/kwin-pages); docs/DESKTOPS.md lists it under "Not yet under KWin". Left: merge it, or SetMouse/SetKeyboard do nothing under KWin (M)

### Windows

- [x] Window list for the taskbar, dock, centre pill, Task Manager, Discord and Snap layouts — WinList.qml, hearth-kwin-bridge, kwin/windows/hearth-windows.js; Dock/TaskStrip/Bar/Islands/TaskManagerView/Discord/SnapLayouts/Pad: "WinList.toplevels"; test_kwin_windows.py "test_script_against_pretend_kwin", "test_no_direct_toplevel_manager"; kwin-windows.yml end to end (needs testing on a device)
- [x] Window actions from the taskbar (minimize, maximize, close, move to desktop, keep above, on all desktops, fullscreen) — hearth-windows.js `run()`: "case \"keepAbove\"", "case \"allDesktops\"", "case \"fullscreen\"", "case \"desktop\""; Dock.qml: "if (WinList.kwin) { WinList.moveToDesktop(win, n); return; }"; test_kwin_windows.py "test_window_actions_are_known" (needs testing on a device)
- [ ] Window previews on hover in the dock/taskbar. Left: KWin's window thumbnails through the bridge, or a ScreenCast stream per window (M)
- [x] Title bars in Hearth's look — theme/hearth_aurorae.py, kwin/windows/xdg/kwinrc: "theme=__aurorae__svg__hearth"; test_kwin_windows.py "test_aurorae_files", "test_labwcs_colours"; kwin-windows.yml pictures "kwin-frame-active/inactive/maximized/hearth-gold/csd" (needs testing on a device)
- [x] Title bar right-click menu with Hearth's items — hearth-windows.js: "registerUserActionsMenu … \"Snap layouts…\"" plus each zone; KWin's own menu has Move to desktop, Keep above, Close; kwin-windows.yml picture "kwin-window-menu" (needs testing on a device)
- [ ] Alt+Tab window switcher in Hearth's look — Hearth has: KWin's default switcher (no Hearth TabBox layout anywhere in kwin/). Left: a Hearth task-switcher QML layout for KWin's TabBox, picked in Hearth's kwinrc defaults (S)
- [x] Snapping: drag to edges and corners, Super+arrows, halves and quarters — kwin/xdg/kwinrc: "ElectricBorderTiling=true"; shortcuts.json "snap-left … snap-down" (win: left/right/up/down); test_kwin_windows.py "test_zones_match_snap_layouts_and_labwc"; picture "kwin-halves" (needs testing on a device)
- [ ] Snap layouts (Super+Z, Maximize right-click) — partly: Super+Z and the title bar menu's Snap place windows through WinList.snap (SnapLayouts.qml: "if (WinList.kwin) { WinList.snap(win, q.id); return; }", picture "kwin-snap-layouts"). Left: right-click (or rest the pointer) on Maximize opening Snap layouts under KWin (Aurorae's button does KWin's own thing) (S)
- [x] Virtual desktops: the bar's switcher, Super+Ctrl+arrows/1–4, move window to desktop — kwin/windows/xdg/kwinrc "[Desktops] Number=4"; DeskSwitcher.qml: "if (WinList.kwin) return WinList.desktops…"; shortcuts.json "desktop-1…4", "window-to-desktop-1…4"; test_kwin_windows.py "test_same_keys_and_commands" (needs testing on a device)
- [x] Show desktop (Super+D) and minimize all (Super+M) — shortcuts.json "show-desktop": "call taskbar showDesktop"; kdeKeys frees KDE's Super+D; test_kwin_windows.py "test_kde_keys_table" (needs testing on a device)

### Shortcuts

- [x] All of Mantel's shortcuts on KWin — kwin/windows/shortcuts.json (66 Hearth keys, registered by hearth-windows.js `registerShortcuts`; KDE's clashing keys freed once, hearth-kwin-bridge `fix_kde_keys`); test_kwin_windows.py "test_every_labwc_key_is_covered", "test_frees_defaults_only_and_gives_hearth_its_keys"; real key presses in kwin-windows.yml (39c6fcc) (needs testing on a device)
- [x] Super alone opens the launcher — kwin/windows/xdg/kwinrc: "[ModifierOnlyShortcuts] Meta=org.hearth.Windows,…,Launcher"; test_kwin_windows.py "test_extra_ids_and_launcher" (CI only warns: KWin makes no call in the runner, 03c1c2d) (needs testing on a device)

### Screens

- [x] Resolution, refresh rate, scale, position, on/off, keep-or-go-back — hearth-screens (kscreen-doctor under KWin), SetDisplay.qml, docs/SCREENS.md; test_hearth_screens.py (35 tests); desktop-test.yml `kwin-screens` job with the real kscreen-doctor (needs testing on a device)
- [ ] Rotation and orientation, primary screen, mirroring — partly: rotation under KWin (hearth-screens, test_hearth_screens.py "test_kwin_command"); mirroring shown greyed ("not possible in KWin 6.3", docs/SCREENS.md); the primary screen is read (`priority`, hearth-screens:219) but can't be picked. Left: a "Main screen" row (kscreen-doctor `output.N.priority.1`) (S)
- [x] Screen off when idle, screen-off for Server mode — hearth-screen-power: KWin's DPMS through "kscreen-doctor --dpms"; IdlePower.qml; kwin-screens job "screen off near the end" (needs testing on a device)
- [x] Brightness keys and slider — shortcuts.json "brightness-up", "brightness-down" (the same commands as labwc's) (needs testing on a device)

### Lock, typing, screenshots

- [x] Lock screen — kwin-lock/contents/lockscreen/LockScreen.qml, kwin-lock/install.sh, Lock.qml ("hands locking to KWin's locker"), hearth-lockcheck, nimbus-session `kwin_lock_ready` (no lockout: `--no-lockscreen` when it isn't there); kwin-input.yml `lock-debian` locks for real and unlocks through PAM (needs testing on a device)
- [x] Typing for the on-screen keyboard, clipboard paste, emoji picker, snap layouts and the controller — hearth-type, kwin-helpers/hearth-kwin-im.c, kwin/xdg/kwinrc "InputMethod[$e]=/usr/share/applications/hearth-kwin-im.desktop"; test_kwin_input.py "test_keyboard_types_through_hearth_type_on_kwin", "test_paste_and_magnifier"; kwin-input.yml types into a GTK field (needs testing on a device)
- [x] Screenshots (Print, Shift+Print, Super+Shift+S, Ctrl+Print) — os/system_files/usr/bin/hearth-screenshot ("under KWin it runs hearth-screens shot"), hearth-screens `grab`, shortcuts.json "screenshot", "screenshot-region", "screenshot-clipboard"; test_hearth_screens.py "test_screenshot_trust_for_kwin"; kwin-screens job picture with KWin's permission check on (needs testing on a device)
- [ ] Screen sharing for browsers, Discord, OBS (the ScreenCast portal) — partly: hearth_screencast.py, cast-picker.qml (Screens and Windows), hearth-portals.conf; kwin-input.yml gets a PipeWire stream with real frames through xdg-desktop-portal. Left: the picker's "area" choice (a part of a screen); PickColor for the Screenshot portal (S)
- [ ] The controller in Desktop Mode (pointer, View+D-pad snap, View+B close, LB/RB windows) — partly: hearth-pad types KWin's (same) keys through its own uinput keyboard and Pad.qml follows WinList ("readonly property var active: WinList.activeToplevel"); the lock screen part is tested (test_hearth_pad.py). Left: a KWin check that View+D-pad, View+B and LB/RB do the right thing on KWin (S)
- [ ] Night light, colour filters and magnifier keep working on day one — partly: Night light goes to KWin's Night Light (hearth-screens; test_hearth_screens.py "test_night_light_follows_the_compositor"); the magnifier keys reach KWin's Zoom (hearth-access:481). Left: colour filters and invert under KWin: labwc/autostart:34 runs `hearth-access apply kwin`, which hearth-access:471 doesn't accept (only `game` / `desktop`), so nothing happens; KWin's filters are on unmerged feature/kwin-pages (S)

---

## Stage 2: what KWin itself can do, with Hearth Settings pages

Most of this section is what unmerged **feature/kwin-pages** builds; on feature/kwin-1.14 only the Display parts
(feature/kwin-screens) are in.

### Display and colour

- [ ] HDR in Desktop Mode (per screen) — partly: HDR switch, SDR brightness and wide colour per screen under KWin (hearth-screens, SetDisplay.qml; test_hearth_screens.py "test_kwin_keep_saves_hdr_vrr_and_profile"). Left: colour intensity (S)
- [ ] HDR calibration (max/peak brightness wizard) (M)
- [ ] Extended Dynamic Range (HDR content on SDR laptop panels) (S)
- [x] Variable refresh rate (Never / Automatic / Always) — hearth-screens, SetDisplay.qml, docs/SCREENS.md "VRR / FreeSync"; test_hearth_screens.py "test_kwin_keep_saves_hdr_vrr_and_profile" (needs testing on a device)
- [ ] Colour profiles — partly: none / the screen's own / an ICC file (hearth-screens, SetDisplay.qml). Left: the colour accuracy choice (S)
- [ ] Colour depth limit and RGB range (S)
- [ ] Fractional scaling per screen and the X11 apps' scaling choice — partly: per-screen scale under KWin (hearth-screens). Left: "X11 apps scale themselves / are scaled" (kwin-pages has it) (S)
- [ ] Night light: by location, custom times, always on, warmth, fade, Quick settings tile — partly: on/off, warmth, evenings or always, from/until, the Quick settings toggle (SetDisplay.qml "nightFrom", hearth-screens `[NightColor]`). Left: sunset to sunrise by location, fade length (S)
- [ ] Reduce red tint at low brightness (AMD laptops) (S)
- [ ] Automatic brightness from the light sensor (S)
- [ ] Brightness for external screens (DDC/CI) (M)
- [ ] Overscan, allow tearing, wide colour gamut — partly: tearing (kwinrc `[Compositing] AllowTearing`) and wide colour (hearth-screens). Left: overscan (S)
- [ ] Virtual desktops only on the main screen, or per screen (S)
- [ ] Screen edge barrier and corner barrier between screens (S)
- [ ] Mirror / extend / only one screen quick switcher (Super+P) (M)

### Desktop effects (all on unmerged feature/kwin-pages)

- [ ] Animation speed (one slider) and Reduced motion respected — partly: Reduce motion for Mantel only (SetAccessibility.qml) (S)
- [ ] Window open/close: Fade, Glide, Scale, Fall Apart (S)
- [ ] Minimize: Magic Lamp, Squash (S)
- [ ] Maximize, geometry-change, morphing and sliding popups (S)
- [ ] Desktop switch: Slide, Fade, Cube (S)
- [ ] Desktop Cube (S)
- [ ] Wobbly windows (S)
- [ ] Blur and background contrast behind Hearth's panels — partly: Mantel's own frost (FrostBlur.qml, works the same on KWin). Left: KWin's blur for translucent apps and a strength slider (M)
- [ ] Translucency (moving, inactive, menus) (S)
- [ ] Dim inactive windows, dim behind password prompts (S)
- [ ] Rounded corners and outlines for windows — partly: rounded top corners in Hearth's Aurorae frames (hearth_aurorae.py). Left: bottom corners, outlines, apps that draw their own (Steam) (M)
- [ ] Peek at desktop / Show desktop animation, login and logout fades (S)
- [ ] Mouse click animation, mouse mark, track mouse, touch points (S)
- [ ] Shake cursor to find it (S)
- [ ] Hide the pointer while typing or after a time (S)
- [ ] Snap helper / window aperture / thumbnail aside / show FPS / show paint (S)
- [ ] Startup feedback (busy pointer or bouncing icon) (S)
- [ ] Visual bell (S)

### Overview, switcher, desktops

- [ ] Overview (Super+W): all windows and desktops, search, drag windows between desktops (M)
- [ ] Window view / Present windows (S)
- [ ] Desktop grid (S)
- [ ] Window switcher styles (Thumbnail grid, Large icons, … Cover switch, Flip switch) and their settings (M)
- [ ] Second switcher (Alt+`, same app's windows) (S)
- [ ] Virtual desktops: add, remove, rename, rows, wrap around, OSD — Hearth has: 4 fixed, named desktops (kwin/windows/xdg/kwinrc "[Desktops]") (M)
- [ ] Different tile layouts per desktop (S)

### Window management

- [ ] Tiling editor (Super+T, Shift-drag into tiles) — Hearth has: Snap layouts' fixed zones (SnapLayouts.qml, shortcuts.json "extraZones") (L)
- [ ] Window behaviour: focus policy, raise on hover, click raises, focus stealing prevention — Hearth has: ClickToFocus fixed (kwin/xdg/kwinrc) (S)
- [ ] Title bar actions and Super+drag — partly: double-click maximizes (kwin/windows/xdg/kwinrc "TitlebarDoubleClickCommand=Maximize"). Left: wheel, middle-click, Super+drag move/resize choices (S)
- [ ] Window placement for new windows (S)
- [ ] Snap zones: edges stick, gap sizes — partly: Snap layouts' gap (shortcuts.json "gap": 6). Left: the sliders (S)
- [ ] Window rules page (per app, "Detect window", presets) — partly: one built-in rule (shortcuts.json "rules": Task Manager kept on top; hearth-windows.js `applyRules`). Left: the users' page and KWin's rule file (L)
- [ ] Window menu "More actions" — partly: KWin's own window menu (keep above, all desktops, fullscreen) with Hearth's Snap. Left: shade, no border and special window settings in Hearth's look (S)
- [ ] Kill a frozen window (Ctrl+Alt+Esc) and "not responding" in Hearth's look (S)
- [x] Session restore: reopen apps after restart — SessionRestore.qml, SetStartup.qml "Reopen my apps from last time" (off by default, WinList); test_system_settings_114.py "test_off_by_default_and_only_at_sign_in" (needs testing on a device)
- [ ] Plasma's KWin scripts (Minimize all, Desktop change OSD, video wall, sync skip-switcher) — Hearth has: Minimize all (show-desktop-2) (S)
- [ ] Exclude a window from screen sharing (title bar menu) (S)
- [ ] Picture-in-picture windows: check Hearth Player and browsers use it under KWin (S)

### Screen edges, gestures, input

- [ ] Hot corners / screen edges page — Hearth has: every KWin edge set to None (kwin/xdg/kwinrc "[ElectricBorders]"), maximize/tile by dragging on (M)
- [ ] Touchpad gestures (3/4-finger swipes, pinch) listed and switchable (S)
- [ ] Touchscreen edge swipes and gestures — partly: Hearth's own (EdgeSwipes.qml); KWin's touch edges off (kwin/xdg/kwinrc "[TouchEdges]"). Left: KWin's for Overview/desktops (S)
- [ ] Touchpad: the missing rows (disable with a mouse, middle-click emulation, scroll speed, edge scrolling, click method) — Hearth has: SetMouse.qml's rows, which aren't applied under KWin yet (see Stage 1's audit line) (S)
- [ ] Mouse: acceleration profile, scroll speed, middle-click paste, per-device (S)
- [ ] Mouse buttons: re-map extra buttons (M)
- [ ] Keyboard: Num Lock at start, layouts per window, compose key, typing delay (S)
- [ ] Touchscreen: which screen, turn it off (S)
- [ ] Drawing tablets (L)
- [ ] Game controllers as regular mouse/keyboard input left off — partly: Hearth's controller mouse (hearth-pad). Left: KWin's option explicitly off in Hearth's kwinrc defaults (S)
- [ ] Sync the mouse and stylus pointers (S)

### Accessibility (KWin's parts)

- [ ] Zoom with Mantel's keys, follow modes, pixel grid — partly: Super+= / − / 0 reach KWin's Zoom (hearth-access:481, shortcuts.json "kwinOwn"; test_kwin_input.py "test_paste_and_magnifier"). Left: follow pointer / text cursor / centred and the page (S)
- [ ] Magnifier (a lens around the pointer) (S)
- [ ] Colour filters in Desktop Mode (greyscale, protanopia, deuteranopia, tritanopia, intensity) (S)
- [ ] Invert colours (whole screen, one window) — Hearth has: labwc's gamma invert; nothing under KWin (Stage 1's line) (S)
- [ ] Sticky, slow, bounce and toggle keys, mouse keys (M)
- [ ] Dwell click (S)
- [ ] Shake cursor and Track mouse on Accessibility (S)
- [ ] Activation sounds / visual bell (S)

### Xwayland and gaming

- [ ] Xwayland: X11 apps reading keys typed elsewhere (push-to-talk), X11 apps' scaling — partly: "XwaylandEavesdrops=None" as Hearth's default (kwin/xdg/kwinrc). Left: the rows (S)
- [x] Allow tearing for full-screen games — SetDisplay.qml, hearth-screens (kwinrc `[Compositing] AllowTearing`, `tearing` in screens.json); test_hearth_screens.py; kwin-screens job "tearing last" (needs testing on a device)

### Notifications and OSDs KWin shows

- [ ] Caps Lock / Num Lock and keyboard layout OSD (S)
- [ ] Microphone mute OSD and "app tried to use a muted microphone" (S)

---

## Stage 3: written from scratch

### Panels and widgets

- [x] Floating panels, auto-hide, dodge windows, panel on any edge — `Islands.qml`, `Dock.qml`, `BarLayoutPicker.qml`, `SetBarDock.qml`
- [x] App launcher with search, favourites, recent, categories, power actions — `Launcher.qml`, `MenuStyles.qml`
- [x] Taskbar with pinned apps, grouping, jump lists, middle-click, reorder, Super+1–9 — `Dock.qml`, `TaskStrip.qml`
- [x] System tray — `Islands.qml`, `Bar.qml`
- [x] Clock with calendar, notification history and dashboard — `ClockPanel.qml`
- [x] Media player controls in the bar — `MediaPill.qml`, `MediaCard.qml`, `MenuNowPlaying.qml`
- [x] Quick settings (volume, Wi-Fi, Bluetooth, battery, brightness, power profile) — `QuickAccess.qml`
- [x] Desktop clock and weather on the wallpaper — `DeskClock.qml`
- [x] Show desktop — `Dock.qml`, `hearth-desk`
- [ ] Edit mode: add, move, resize and remove widgets on the desktop and in the bar — partly: the desktop part is built (Widgets.qml, WidgetGallery.qml, WidgetArrange.qml, WidgetStore.qml: gallery, drag, resize, remove, Arrange with the controller; widgets-test.yml pictures; test_desktop_widgets.py), and five widgets can go "in the bar" (BarWidgets.qml). Left: moving and ordering the bar's own items; docs/ROADMAP.md's widgets line is still unticked (M)
- [ ] More panels (several bars, on several screens, clone a panel) (L)
- [x] Sticky notes on the desktop — WidgetNotes.qml; widgets-test.yml (needs testing on a device)
- [x] System monitor sensors on the desktop and in the bar — WidgetMonitor.qml ("hearth-tasks sample"), BarWidgets.qml; widgets-test.yml (needs testing on a device)
- [ ] Analog/binary/fuzzy clocks, world clocks — partly: analog or digital with your cities (WidgetClock.qml "cities"). Left: binary and fuzzy styles, extra zones in the clock panel (S)
- [ ] Alternative calendars and holidays in the calendar (M)
- [x] Weather report widget (forecast, °C/°F, place by name) — WidgetWeather.qml, hearth-widgets (Open-Meteo); widgets-test.yml (needs testing on a device)
- [x] Folder view widget — WidgetFolder.qml; widgets-test.yml (needs testing on a device)
- [ ] Picture frame, web page, timer, calculator, colour picker, dictionary, disk quota, keyboard indicator, comic strip, fifteen puzzle — partly: photo frame (WidgetPhotos.qml), timer and stopwatch (WidgetTimer.qml), plus news, app tiles, quick notes, battery, media. Left: web page, calculator, colour picker, keyboard indicator (dictionary, disk quota, comic, fifteen puzzle are in "Not for Hearth?") (M)
- [ ] Window list widget, pager with window outlines, activity pager (M)
- [ ] Printers queue in the tray (S)
- [ ] Disks & devices in the tray (M)
- [ ] Camera / microphone / screen-recording "in use" indicator — partly: a dot under the bar while recording or while someone is connected by VNC (CaptureIndicator.qml, Recorder.qml). Left: camera and microphone in use (PipeWire's streams) (S)
- [ ] Background apps in the tray (S)
- [ ] Quick light/dark tile in Quick settings (S)

### Search (KRunner-style) — the framework and plugins are on unmerged feature/search-store

- [x] Search apps, Settings pages and single settings, games — `Launcher.qml`, `menu-search.js`, `settings-index.js`
- [x] Run box (Super+R) — `WinTools.qml`, `RunView.qml`, `run-map.js`
- [ ] KRunner-style search bar (Alt+Space) with plugins in an order you choose, "?" help (L)
- [ ] Plugin: calculator, unit conversion, currency (M)
- [ ] Plugin: files and folders by name and content (M)
- [ ] Plugin: recent files and places, bookmarks, browser history (M)
- [ ] Plugin: open windows, virtual desktops, activities by name (S)
- [ ] Plugin: run a command, terminal command, kill an app (S)
- [ ] Plugin: web search keywords (S)
- [ ] Plugin: time in other cities, time-zone conversion (S)
- [ ] Plugin: spell check, dictionary, special characters, emoji, colours (S each)
- [ ] Plugin: power and session, media control, store results (S)
- [ ] Plugin: Hearth contacts, calendar events, notes (M)
- [ ] Type on the desktop to search (S)

### Notifications

- [x] Popups, history, Do Not Disturb, mute an app, Clear all — `Notifications.qml`, `ClockPanel.qml`, `SetNotifications.qml`
- [ ] History kept across restarts — Hearth has: this session only (DeskState.qml "the notification history for this session") (S)
- [x] Per-app notification settings (allowed, pop-ups, sound, lock screen, priority) — hearth-notify, NotifyRules.qml, SetNotifications.qml; test_system_settings_114.py "test_only_changes_are_kept_and_merged", "test_the_server_follows_the_rules" (badges on the dock: see below; the lock screen part on labwc's lock only, docs/SYSTEM-SETTINGS.md) (needs testing on a device)
- [ ] Automatic Do Not Disturb in full-screen apps and while screen sharing, with a missed count (S)
- [ ] Do Not Disturb on a schedule, and its shortcut (S)
- [ ] Progress of jobs and file transfers (JobViewServer) (M)
- [ ] Reply from a notification, actions kept in history — partly: the Reply box for any notification that takes an inline reply (NotifReply.qml; phone-link.yml pictures). Left: actions kept in the history (S)
- [ ] Badges and progress on dock icons (Unity launcher API) (S)
- [ ] Notification sounds and a sound theme for system events — partly: a per-app sound switch and one notification sound in the image (hearth-notify "sound", 88ffaf3). Left: a sound set for system events and a Sounds page (M)

### Clipboard

- [x] Clipboard history, pictures, pinned, Clear all keeping pinned, secrets skipped — `Clipboard.qml`, `hearth-clipboard`, `SetClipboard.qml`
- [ ] Clipboard actions (open a URL in…, QR code, edit an item) (S)
- [ ] Selection (middle-click) history and sync with the clipboard (S)

### Phone link (KDE Connect)

- [x] Pairing, battery, ping, find my phone, clipboard, media, files, notifications with reply, remote input — `hosts/hearth-connect` (needs testing on a device)
- [x] SMS/MMS: read conversations and send texts — hosts/hearth-connect/src/plugins/sms.rs, SetPhoneMessages.qml; hosts/hearth-connect/tests/test_phone_link.py against a pretend phone; phone-link.yml pictures (needs testing on a device)
- [x] Phone calls: who's calling, mute ringer, media paused — plugins/telephony.rs; test_phone_link.py (needs testing on a device)
- [x] Contacts from the phone into Hearth Contacts — plugins/contacts.rs; test_phone_link.py (needs testing on a device)
- [x] Browse the phone's files (SFTP) — plugins/sftp.rs (sshfs); test_phone_link.py (needs testing on a device)
- [x] Run commands — plugins/runcommand.rs, SetPhone.qml; test_phone_link.py (needs testing on a device)
- [ ] Phone controls PC volume, lock, presentation remote — partly: volume (plugins/systemvolume.rs) and the presentation remote (plugins/presenter.rs), tested in test_phone_link.py. Left: `kdeconnect.lock` (lock this PC from the phone) (S)
- [ ] PC sends its notifications to the phone; PC as a remote keyboard for the phone (`kdeconnect.notification` and `.mousepad` outgoing) (M)
- [ ] Signal report, stop the screensaver while connected, share text and URLs — partly: signal (`kdeconnect.connectivity_report`) and the phone's camera (`kdeconnect.photo`). Left: inhibit while connected, text/URL shares checked (S)
- [ ] Phone as a second screen, Bluetooth link (L)
- [x] Phone tray item and Settings page — PhoneStatus.qml, SetPhone.qml, Quick Access tile; phone-link.yml pictures (needs testing on a device)

### Remote desktop and screen recording

- [x] Use this PC from anywhere (Moonlight through Tailscale, Sunshine) — `hearth-remote-desktop`, `SetRemote.qml`
- [ ] Remote desktop server (the owner dropped RDP for VNC) — partly: hearth-vnc (VeNCrypt TLS only, Tailscale/home network, password in the keyring, lock-out after five tries, view only, Disconnect), SetRemoteDesktop.qml, KWin control through Hearth's RemoteDesktop portal ("Allow control?", hearth-kwin-cast input); test_remote_record.py, remote-record.yml pictures. Left: docs/ROADMAP.md's "Remote desktop and recording" line is still unticked; touch input isn't passed on (docs/KWIN-INPUT.md "no touch yet") (S)
- [x] Screen recording, instant replay, sound recorder — `hosts/hearth-recorder`
- [ ] Record a window, system sound and the microphone together, choose format and quality — partly: hearth-record and Recorder.qml (a screen or a window through the portal, this PC's sound and the microphone, 30/60 fps, GPU encoder with x264 fallback); test_remote_record.py. Left: a format choice (MP4 only) and recording popups (S)
- [ ] Screenshot tool like Spectacle — partly: Hearth Capture (Capture.qml, CaptureEditor.qml, hearth-capture-helper: all / screen / window / area, delay, pointer, arrows, boxes, pen, text, blur, pixelate, numbers, crop, copy/save, OCR with tesseract, QR codes); test_hearth_capture.py, capture-polkit-test.yml pictures. Left: scrolling capture, share (S)

### Files (Dolphin's features)

- [x] Views, tabs, places, Trash, previews, filter, search, hidden files, archives, batch rename, Open in Terminal, network shares, undo, details, properties — `FileManager.qml`, `hearth-files`, `FileProperties.qml`
- [ ] Split view (F3) — on unmerged feature/search-store (M)
- [ ] Terminal panel (F4) — on unmerged feature/search-store (M)
- [ ] Tags, ratings and comments (M)
- [ ] Group by (S)
- [ ] Git status badges (S)
- [ ] Act as administrator (M)
- [ ] Checksums, permissions and owner tabs in Properties (S)
- [ ] Mount an ISO, MTP phones, PTP cameras (M)
- [ ] Service menus (M)

### File search and indexing

- [ ] File indexing (names and contents) with folders to skip and battery limits — on unmerged feature/search-store (XL)
- [ ] Recent files tracking and "forget recent files" (S)

### Passwords, wallet, vaults

- [ ] Password store (Secret Service, unlocked at login, a manager page) — on unmerged feature/search-store ("Passwords like KDE Wallet") (XL)
- [ ] SSH and GPG passphrases asked in Hearth's look (S)
- [ ] Vaults: encrypted folders in the tray, auto-lock, offline-only — Hearth has: Briefcase (`hearth-briefcase`) (M)
- [ ] Password prompts for admin actions (Polkit agent) in Hearth's look — partly: Mantel's own agent (PolkitPrompt.qml, polkit-actions.js, hearth-polkit-fallback with the classic agent as fallback); test_hearth_polkit_mantel.py, capture-polkit-test.yml pictures. Left: fingerprint (with fprintd below) (S)

### Activities

- [ ] Activities — partly: Activities.qml, ActivitySwitcher.qml, ActivityChip.qml, activities.js (windows, wallpaper, widgets and pinned apps per activity, switcher in the bar, Quick Access and the controller, Meta+Ctrl+Q / Meta+Ctrl+Shift+Q, Move to activity in the window menus); test_desktop_widgets.py, widgets-test.yml. Left: recent files and privacy per activity; Super+Q is still Close, not the switcher (shortcuts.json "close" Meta+Q); the ROADMAP line is unticked (M)

### App store and updates

- [x] App store (Flathub, Debian packages) — Bazaar + `SetApps.qml`, `DebInstaller.qml`
- [x] System updates with the previous version kept — `SetUpdates.qml`, bootc
- [ ] Discover's extras (update notifications, reviews, permission changes, verified label, links, fonts, drivers) — Hearth's own App store is on unmerged feature/search-store (L)
- [x] App permissions page (Flatpak overrides) — SetAppPermissions.qml, hearth-flatpak-perms (flatpak override --user, Reset); test_settings_more_114.py "test_permissions_from_metadata_and_your_overrides"; picture ss-app-permissions (needs testing on a device)
- [ ] Firmware updates (fwupd) with a notification — partly: SetFirmware.qml, hearth-firmware, hearth-sysset-admin (parts install after a password; system firmware shown, never installed: red flag 2, docs/SYSTEM-SETTINGS.md); test_system_settings_114.py "test_one_rule_for_system_firmware"; the notification (FirmwareWatch.qml, once a day, each version once; switch on SetFirmware.qml). Left: the history (S)
- [x] Firmware security (Secure Boot, TPM, HSI level) — SetFirmware.qml, hearth-firmware; test_system_settings_114.py "test_security_level", "test_secure_boot_words" (needs testing on a device)
- [x] Shut down or restart after installing updates — SetUpdates.qml "Shut down when it's done" and "Shut down and finish updating" (a minute to cancel); picture ss-updates-shutdown (needs testing on a device)
- [x] Backups of your files to a drive — reused: Home server › Backups & sync (SetHomeBackups.qml: to a drive, another Hearth device or the cloud), linked from Updates & recovery (settings-pages.js LINKS "updates"); feature/pc-backups (UrBackup for the house's PCs) is already in feature/kwin-1.14 (needs testing on a device)

### Network

- [x] Wi-Fi networks, hidden, forget, properties, hotspot — `SetWifi.qml`, `DeviceSettings.qml`, SetHotspot.qml
- [x] Tailscale — `SetHomeTailscale.qml`
- [ ] Wired, mobile broadband, editing any connection — partly: Connection details (DNS, metered, random MAC, IP by hand, IPv6, auto-connect: SetNetDetails.qml, hearth-network; test_system_settings_114.py "test_details_args_are_checked"). Left: wired and mobile broadband (SIM, APN), 802.1X/enterprise Wi-Fi (L)
- [ ] VPN — partly: WireGuard and OpenVPN from their file, connect, OpenVPN account asked when connecting (SetVpn.qml, hearth-network; test_system_settings_114.py "test_vpn_password_goes_on_stdin_never_argv"). Left: IPsec/StrongSwan, L2TP, PPTP, OpenConnect, SSTP, Fortinet (M)
- [ ] Wi-Fi QR codes (show one, scan one) (S)
- [x] Proxy settings (none, automatic, by hand) — SetProxy.qml, hearth-network; test_system_settings_114.py "test_proxy_saved_with_settings_io_and_handed_on" (needs testing on a device)
- [ ] Firewall page (M)
- [ ] Duplicate a connection, network speed in the tray (S)
- [ ] "Share this folder" (SMB) in Files' Properties (S)

### Online accounts

- [x] Online accounts (Nextcloud, CalDAV/CardDAV, WebDAV, IMAP, Google, Microsoft) for calendars, contacts and files — SetAccounts.qml, hearth_accounts.py, hosts/hearth-contacts/src/online.rs, hosts/hearth-time/src/accounts.rs, docs/ONLINE-ACCOUNTS.md; test_online_accounts.py, test_account_calendars.py, online-accounts.yml (Google/Microsoft need the owner's client ids; no Hearth app reads mail yet) (needs testing on a device)

### Accounts, sign-in, login

- [x] Users: add, remove, account type, picture, password — `SetUsers.qml` (account type: test_system_settings_114.py "test_user_type_plan")
- [x] Sign in automatically, lock after sleep — `SetSignin.qml`
- [ ] Fingerprint sign-in and enrolling fingers (fprintd) (M)
- [ ] Cropping the account picture (S)
- [ ] Login screen settings and Hearth's own login screen — partly: the greetd greeter (os/system_files/usr/share/hearth/greeter, hearth-login-screen, hearth-greetd.service with SDDM coming back by itself), Sign-in options switch with Preview (LoginScreenSwitch.qml), Login screen page with a background (SetLoginScreen.qml); test_hearth_login_screen.py, login-greetd-test.yml. Left: it's off by default and SDDM is still the display manager until it's proven on a device (S)
- [ ] First-run account setup on pre-installed PCs: check Game Mode's first-time setup covers it (S)
- [ ] Switch user and the extra log out choices (M)

### Region, language, time

- [x] Date & time — `SetDateTime.qml`
- [x] Keyboard layouts — `SetKeyboard.qml` (labwc; see Stage 1's audit line for KWin)
- [ ] Language of the system, several in order — partly: language (SetRegion.qml, hearth-region, 50 languages in the image; test_system_settings_114.py "test_set_locale_runs_localectl_with_a_list"); several in order (SetLanguages.qml, hearth-region "languages-set": LANGUAGE in environment.d and plasma-localerc; test_settings_more_114.py "test_languages_in_order"; picture ss-languages). Left: downloading translations (S)
- [x] Formats: numbers, currency, dates, units, paper size — SetRegion.qml, hearth-region "FORMAT_KEYS = ('LC_NUMERIC', 'LC_TIME', 'LC_MONETARY', 'LC_MEASUREMENT', 'LC_PAPER')"; test_system_settings_114.py "test_localectl_parse" (needs testing on a device)
- [x] Spell check settings for apps — SetSpellcheck.qml, hearth-region "spell-set" (Sonnet.conf [General], only its keys); test_settings_more_114.py "test_spell_keeps_other_lines"; picture ss-spellcheck (needs testing on a device)
- [x] Input methods for Chinese/Japanese/Korean (fcitx5) — SetInputMethods.qml, hearth-region "im", "im-set", "im-install" (optional: from Flathub when asked; on makes it KWin's input method in your kwinrc only, off gives the on-screen keyboard back); test_settings_more_114.py "test_fcitx_on_and_off_only_touch_its_key"; picture ss-input-methods (needs testing on a device)

### Look

- [x] Dark / light / automatic, accent colour — `SetAppearance.qml`, `Theme.qml`
- [x] Wallpaper: pictures, moving, your own — `Wallpaper.qml`, `SetAppearance.qml`
- [x] Boot splash choice — `SetBoot.qml`, `SetBootFrames.qml`
- [x] Fonts viewer, install and remove — `hosts/hearth-fonts`
- [ ] Wallpaper per screen, slideshow, picture of the day, light/dark wallpaper — partly: a wallpaper per activity (Activities.qml, `Theme.activityWall`); per screen and a slideshow (SetWallpaperMore.qml, WallShow.qml, Wallpaper.qml "WallShow.forScreen"; picture ss-wallpaper-more). Left: picture of the day, light/dark wallpaper (S)
- [ ] Save your own look as a theme (S)
- [ ] System fonts — partly: UI font and size, fixed-width font, smoothing, hinting, font DPI (SetFonts.qml, hearth-lookset; test_system_settings_114.py "test_set_saves_merges_and_hands_on_only_to_hearths_files"); small and window title fonts (hearth-lookset smallFont/titleFont: kdeglobals smallestReadableFont and [WM] activeFont, GNOME's titlebar-font; test_settings_more_114.py "test_small_and_title_fonts"; picture ss-fonts-small). Left: sub-pixel order (S)
- [x] Cursor theme and size — SetFonts.qml, hearth-lookset; test_system_settings_114.py "test_theme_lists" (needs testing on a device)
- [x] Icon theme choice — SetFonts.qml, hearth-lookset; test_system_settings_114.py "test_theme_lists" (needs testing on a device)
- [ ] App style: window frame colour intensity, "apps follow Hearth" — partly: frame colour intensity (SetLookMore.qml, desktop.json frameTint, hearth_aurorae.py "tint_colors" / "tint_themerc" for KWin and labwc; test_settings_more_114.py "test_frame_tint"; picture ss-look-more). Left: "apps follow Hearth" (S)
- [x] Splash at sign-in — SigninSplash.qml (flame, logo or plain; clicks go through it, gone after 2.5 s, once per sign-in), SetLookMore.qml; test_settings_more_114.py "test_watchers_are_loaded_by_file_name_and_cant_hold_the_desktop_up"; pictures ss-look-more, ss-signin-splash (needs testing on a device)
- [ ] Keyboard RGB lights follow the accent (M)

### Sound

- [x] Volume, mute, over-amplification, devices, per-app mixer — `SetSound.qml`, `Mixer.qml`
- [x] Device profiles, balance, speaker test — SetSoundSpeakers.qml, hearth-sound "balance", "speaker-test"; test_settings_more_114.py "test_balance_keeps_the_loudest_level_and_leaves_centre_alone", "test_speaker_tone_is_only_in_its_channel"; picture ss-sound-speakers (needs testing on a device)
- [x] Microphone test and push-to-talk mute shortcut — SetPushToTalk.qml, PushToTalk.qml, hearth-ptt (KWin's global shortcuts, pressed/released; the microphone's mute put back when it stops); test_settings_more_114.py "test_keys_and_only_under_kwin"; picture ss-push-to-talk (needs testing on a device)
- [ ] Move one app's sound to another device, per-app input — partly: each app's output (SetSoundApps.qml "Plays on", Mixer.qml "outOf" / "sendTo"; picture ss-sound-apps). Left: per-app input (S)
- [x] Scroll on an app's dock icon to change its volume — Dock.qml "wheelVolume", Mixer.qml "stepFor", Settings.dockScrollVolume (switch on SetSoundApps.qml); test_settings_more_114.py "test_settings_keys_saved_one_by_one" (needs testing on a device)
- [x] Warning when "raise maximum volume" stays on — SetSound.qml: "Louder than 100% can hurt" asked before it turns on, and "The volume is over 100%" while it is (needs testing on a device)

### Power

- [x] Power profiles, timers, power button, low battery — `SetPower.qml`, `IdlePower.qml`, `BatteryWatch.qml`
- [ ] On AC / on battery / low battery settings, lid action, dim before off (M)
- [x] Battery charge limit — SetPower.qml through hearth-hwset (/etc/hearth/hardware.json); test_system_settings_114.py "test_charge_limit_through_hwset" (needs testing on a device)
- [ ] Apps blocking sleep, Hibernate and hybrid sleep — partly: SetPowerAwake.qml (logind's inhibitors, hearth-powerset "parse_inhibitors"), Hibernate in SetPower.qml only where hearth-powerset "hibernate_check" says it can (kernel, Secure Boot lockdown, swap on a drive big enough, logind), asked first until it has come back once, booting never changed; test_settings_more_114.py "test_hibernate_is_offered_only_when_everything_allows_it", "test_inhibitors_skip_system_ones"; pictures ss-power-awake, ss-power. Left: hybrid sleep (S)
- [x] Battery health, cycle count, energy graphs — SetBattery.qml (health ring, cycles, charge and power graphs from UPower's GetHistory), hearth-powerset "history"; test_settings_more_114.py "test_battery_history_and_upower"; picture ss-battery (needs testing on a device)
- [x] Batteries of Bluetooth devices, mice and controllers in Quick settings — QuickAccess.qml "devBatts", SetBattery.qml "Your devices' batteries", SetBtDevices.qml (needs testing on a device)

### Devices and hardware

- [x] Bluetooth: on/off, pair, connect — `SetBluetooth.qml`
- [x] Printers — `SetPrinters.qml`
- [x] Game controllers: test and calibrate — `hosts/hearth-gaming`, `SetController.qml`
- [ ] Bluetooth: send/receive files, battery, rename, trust/block — partly: sending files (OBEX through obexd), battery, trust and block (SetBtDevices.qml, hearth-bt); test_settings_more_114.py "test_bluetooth_info"; picture ss-bluetooth-devices. Left: receiving files, rename (S)
- [ ] Printers: queue window, ink notices, Windows shared printers — partly: the print queue (SetPrintQueue.qml: hold, go on, cancel, cancel all mine, printed lately; hearth-printq); test_settings_more_114.py "test_print_queue_and_scanners"; picture ss-print-queue. Left: ink notices, Windows shared printers (S)
- [ ] Scanners: multi-page PDF and feeders, checked — partly: scanners checked (SetScanners.qml: the scanners SANE finds and a test scan; hearth-printq "scanners", "scan-test"); picture ss-scanners. Left: multi-page PDF and feeders (Hearth Tools' scanner) (S)
- [x] Thunderbolt devices: allow once or always, forget — SetThunderbolt.qml, hearth-thunderbolt; test_system_settings_114.py "test_allow_and_forget", "test_no_bolt" (needs testing on a device)
- [x] Removable storage: what happens on plug-in — SetUsbDevices.qml (Ask / Open in Files / Nothing: Settings.removable), DeviceWatch.qml "drive" (udev, hearth-files mount); pictures ss-usb-devices, ss-drive-plugged (needs testing on a device)
- [x] USB device permission prompts — DeviceWatch.qml "usb" (a notice for a new device; where USBGuard blocked it, Allow), SetUsbDevices.qml, hearth-usbdev (never blocks by itself); test_settings_more_114.py "test_usb_devices_from_sysfs"; picture ss-usb-devices (needs testing on a device)
- [x] Touchscreen calibration — KwinCalibrate.qml, SetKwinTablet.qml (from feature/kwin-pages, merged): four targets, KWin's calibrationMatrix; under KWin only (needs testing on a device)

### Startup, session, defaults

- [x] Default apps per kind — `SetDefaults.qml`
- [x] Autostart page — SetAutostart.qml, hearth-autostart (add an app or a command, remove yours, turn on/off; a system one never deleted); test_settings_more_114.py "test_a_command_survives_the_round_trip", "test_a_system_one_is_never_deleted"; picture ss-autostart (needs testing on a device)
- [x] Background services page — SetServices.qml, hearth-services; test_system_settings_114.py "test_a_service_for_everyone_is_masked_for_you" (needs testing on a device)
- [x] File associations editor — SetFileTypes.qml, hearth-mime (every kind, its endings, the app it opens in, your own endings in mime/packages/hearth-user.xml); test_settings_more_114.py "test_types_set_and_reset", "test_own_endings_file"; picture ss-file-types (needs testing on a device)
- [ ] Desktop session rows — partly: "Reopen my apps from last time" (SetStartup.qml). Left: confirm logout, default leave option (S)
- [x] Menu editor ("Edit app…") — SetMenuEditor.qml, hearth-menu (your copy of the entry, X-Hearth-Edited, Put back), "Edit app…" in the app menu's right-click (shell.qml); test_settings_more_114.py "test_save_makes_your_copy_and_reset_puts_it_back"; picture ss-menu-editor (needs testing on a device)

### Info and system

- [x] About this PC — `SetAbout.qml`, `ResourceMonitor.qml`, `DeviceManagerView.qml`
- [x] Task manager — `TaskManager.qml`, `hearth-tasks`
- [x] Disk manager and SMART — `hosts/hearth-disks`, `SetHomeDrives.qml`
- [ ] Info Center's technical pages (OpenGL/Vulkan/Wayland, modules, interrupts, PCI/USB, Samba, sensors) — partly: processor, graphics card, memory, Vulkan and OpenGL, PCI with drivers, USB, energy (SetInfoCenter.qml, hearth-info); test_settings_more_114.py "test_info_center_parsers"; pictures ss-info-*. Left: kernel modules, interrupts, Samba, sensors (S)
- [x] Out-of-memory notification with tips — OomWatch.qml (the kernel's oom_kill count, a notice with tips and Memory settings; memory pressure tip at most every 6 h; Hearth's guard tells its own); picture ss-oom-notice (needs testing on a device)
- [x] Welcome app once per new version — Welcome.qml, welcome.json ("1.14"), seen kept in ~/.local/state/nimbus/welcome.json; picture ss-welcome (needs testing on a device)

### Hearth-specific extras

- [ ] Emoji picker: mixed skin tones, sizes (S)
- [ ] On-screen keyboard press-and-hold for special characters, checked on touch (S)

---

## Not for Hearth? (the owner decides)

The list in feature/kde-feature-list's KWIN.md stands as it is, plus:

- **RDP server (KRdp)**: dropped by the owner (Nov 16, 2026) for the VNC server (hearth-vnc).

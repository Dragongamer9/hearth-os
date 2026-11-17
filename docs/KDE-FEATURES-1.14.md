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
- [x] **(review, Nov 17)** No lockout or setting change from KDE Plasma's leftovers — kwin/xdg/kscreenlockerrc: "Autolock[$i]=false", kwin/xdg/kwinrc: "InputMethod[$ei]=", kwin/windows/xdg/kwinrc: "Meta[$i]="; hearth-your-desktop `kwin-overrides` (the one-time "Some of your KDE settings aren't used here" card); nimbus-session `kwin_lock_ready`: "hearth-lockcheck --as-kwin", kept per image in kwin-lock-proven; hearth-login-screen: "GOOD_SECS"; hearth-screens `kwinrc_set`: "io.update_text(KWINRC"; SetKeyboard.qml: "Bring back KDE's own shortcuts"; hearth-network / hearth-lookset `plain_write`; test_kwin_session.py "test_the_lock_screen_must_really_load_before_kwins_locker_is_on", "test_your_kde_settings_that_hearth_overrides_are_told_once"; kwin-input.yml lock-debian (KWin 6.3: "KWin didn't lock by itself after 80 s idle"); kwin-test.yml pictures "kwin-7-kde-settings-card", "kwin-8-keyboard-kde-keys" (needs testing on a device)
- [x] Xwayland apps and games — nimbus-session: "--xwayland"; kwin-windows.yml "KWin's X server for the X11 window" (36d3f1a) (needs testing on a device)
- [x] Settings › Your desktop says KWin, and Safe start still works — SetYourDesktop.qml: "KWin didn't start this time, so labwc", "Safe start"; hearth-your-desktop; test_kwin_session.py "test_wm_writes_only_its_line_with_a_backup", "test_safe_graphics_is_labwc" (needs testing on a device)
- [ ] **(found in this audit)** Keyboard layouts, key repeat and pointer settings applied under KWin — labwc/autostart:35 runs `hearth-kwin-settings pointer-apply` / `input-watch` only if the file is there, and it isn't on feature/kwin-1.14 (it's on unmerged feature/kwin-pages); docs/DESKTOPS.md lists it under "Not yet under KWin". Left: merge it, or SetMouse/SetKeyboard do nothing under KWin (M)

### Windows

- [x] Window list for the taskbar, dock, centre pill, Task Manager, Discord and Snap layouts — WinList.qml, hearth-kwin-bridge, kwin/windows/hearth-windows.js; Dock/TaskStrip/Bar/Islands/TaskManagerView/Discord/SnapLayouts/Pad: "WinList.toplevels"; test_kwin_windows.py "test_script_against_pretend_kwin", "test_no_direct_toplevel_manager"; kwin-windows.yml end to end (needs testing on a device)
- [x] Window actions from the taskbar (minimize, maximize, close, move to desktop, keep above, on all desktops, fullscreen) — hearth-windows.js `run()`: "case \"keepAbove\"", "case \"allDesktops\"", "case \"fullscreen\"", "case \"desktop\""; Dock.qml: "if (WinList.kwin) { WinList.moveToDesktop(win, n); return; }"; test_kwin_windows.py "test_window_actions_are_known" (needs testing on a device)
- [x] Window previews on hover in the dock/taskbar — kwin/windows/hearth-previews.qml (KWin declarative script, loaded by hearth-kwin-bridge): "WindowThumbnail"; Dock.qml / TaskStrip.qml: "WinList.preview(btn.previewWins"; kwin-windows.yml: "the previews card shows over the desktop", picture "kwin-preview-zoom" (needs testing on a device)
- [x] Title bars in Hearth's look — theme/hearth_aurorae.py, kwin/windows/xdg/kwinrc: "theme=__aurorae__svg__hearth"; test_kwin_windows.py "test_aurorae_files", "test_labwcs_colours"; kwin-windows.yml pictures "kwin-frame-active/inactive/maximized/hearth-gold/csd" (needs testing on a device)
- [x] Title bar right-click menu with Hearth's items — hearth-windows.js: "registerUserActionsMenu … \"Snap layouts…\"" plus each zone; KWin's own menu has Move to desktop, Keep above, Close; kwin-windows.yml picture "kwin-window-menu" (needs testing on a device)
- [x] Alt+Tab window switcher in Hearth's look — os/system_files/usr/share/kwin/tabbox/org.hearth.switcher (and kwin-wayland/tabbox): "KWin.TabBoxSwitcher"; kwin/windows/xdg/kwinrc: "LayoutName=org.hearth.switcher"; kwin-windows.yml: "Alt+Tab switches to the other window", picture "kwin-alt-tab" (needs testing on a device)
- [x] Snapping: drag to edges and corners, Super+arrows, halves and quarters — kwin/xdg/kwinrc: "ElectricBorderTiling=true"; shortcuts.json "snap-left … snap-down" (win: left/right/up/down); test_kwin_windows.py "test_zones_match_snap_layouts_and_labwc"; picture "kwin-halves" (needs testing on a device)
- [x] Snap layouts (Super+Z, Maximize) — Super+Z and the title bar menu as before; resting the pointer on Maximize: hearth-windows.js "onCursor" / "HoverMax", hearth-kwin-bridge "HOVER_WAIT", SnapLayouts.qml "hoverOpen"; kwin-windows.yml: "resting on Maximize opens Snap layouts under the button", picture "kwin-snap-maximize-zoom". (Hover, not right-click: KWin doesn't tell scripts about clicks on its buttons) (needs testing on a device)
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
- [x] Rounded corners and outlines for windows (Hearth's frames) — hearth_aurorae.py: all four corners rounded (`bottom_radius`, as far as the window's own square corner fits inside the frame), 4 px sides and bottom ("SIDE = 4"), a light 1 px outline all round (`outline_color`); shadow, maximized and live look change kept; test_kwin_windows.py "test_rounded_bottom_corners_and_outline"; kwin-windows.yml: "the frame's bottom corners are rounded", "the frame has a light outline round it", pictures "kwin-frame-shadow-light-corner", "kwin-frame-shadow-dark-corner" (needs testing on a device)
- [ ] Rounded corners for apps that draw their own frames (Steam, GTK header bars) — needs a KWin effect that clips windows' corners; KWin 6.3 has none built in (M)
- [ ] Peek at desktop / Show desktop animation, login and logout fades (S)
- [ ] Mouse click animation, mouse mark, track mouse, touch points (S)
- [ ] Shake cursor to find it (S)
- [ ] Hide the pointer while typing or after a time (S)
- [ ] Snap helper / window aperture / thumbnail aside / show FPS / show paint (S)
- [ ] Startup feedback (busy pointer or bouncing icon) (S)
- [ ] Visual bell (S)

### Overview, switcher, desktops

- [x] Overview (Super+W): all windows and desktops, search, drag windows between desktops — KWin's own, in Hearth's KDE colour scheme, opened from Mantel: Dock.qml / TaskStrip.qml Task view button "WinList.effect(\"overview\")", shortcuts.json "overview" (Meta+W, KDE's freed), hearth-pad View + Y; Milou in the image (build.sh "optional milou"); kwin-windows.yml: "Super+W opens the Overview", picture "kwin-overview" (needs testing on a device)
- [x] Window view / Present windows — KWin's, from the Task view button's menu and Ctrl+F10 (shortcuts.json "present-windows"); kwin-windows.yml picture "kwin-present-windows" (needs testing on a device)
- [x] Desktop grid — KWin's, from the Task view button's menu and Ctrl+F8 (shortcuts.json "desktop-grid"); kwin-windows.yml picture "kwin-desktop-grid" (needs testing on a device)
- [ ] Window switcher styles (Thumbnail grid, Large icons, … Cover switch, Flip switch) and their settings (M)
- [ ] Second switcher (Alt+`, same app's windows) (S)
- [ ] Virtual desktops: add, remove, rename, rows, wrap around, OSD — Hearth has: 4 fixed, named desktops (kwin/windows/xdg/kwinrc "[Desktops]") (M)
- [ ] Different tile layouts per desktop — needs newer KWin: KWin's tiles are one layout per screen in 6.3 (Debian 13); a layout per virtual desktop came with Plasma 6.4 (KDE bug 466031). Snap layouts' zones (SnapLayouts.qml) are the same on every desktop until then (S)

### Window management

- [ ] Tiling editor (Super+T, Shift-drag into tiles) — Hearth has: Snap layouts' fixed zones (SnapLayouts.qml, shortcuts.json "extraZones") (L)
- [ ] Window behaviour: focus policy, raise on hover, click raises, focus stealing prevention — Hearth has: ClickToFocus fixed (kwin/xdg/kwinrc) (S)
- [ ] Title bar actions and Super+drag — partly: double-click maximizes (kwin/windows/xdg/kwinrc "TitlebarDoubleClickCommand=Maximize"). Left: wheel, middle-click, Super+drag move/resize choices (S)
- [ ] Window placement for new windows (S)
- [ ] Snap zones: edges stick, gap sizes — partly: Snap layouts' gap (shortcuts.json "gap": 6). Left: the sliders (S)
- [ ] Window rules page (per app, "Detect window", presets) — partly: one built-in rule (shortcuts.json "rules": Task Manager kept on top; hearth-windows.js `applyRules`). Left: the users' page and KWin's rule file (L)
- [ ] Window menu "More actions" — partly: KWin's own window menu (keep above, all desktops, fullscreen) with Hearth's Snap. Left: shade, no border and special window settings in Hearth's look (S)
- [x] Kill a frozen window (Ctrl+Alt+Esc) in Hearth's look — ForceClose.qml ("Click the window that isn't responding", "Force close"), title bar menu "Force close (not responding)…", hearth-kwin-bridge "queryWindowInfo" / "KILL_WAIT" / "killable"; kwin-windows.yml: "Force close ends a frozen window (asked, then made to)", pictures "kwin-force-close-pick", "kwin-force-close". Left: KWin's own automatic "not responding" notice stays KWin's (needs testing on a device)
- [x] Session restore: reopen apps after restart — SessionRestore.qml, SetStartup.qml "Reopen my apps from last time" (off by default, WinList); test_system_settings_114.py "test_off_by_default_and_only_at_sign_in" (needs testing on a device)
- [ ] Plasma's KWin scripts (Minimize all, Desktop change OSD, video wall, sync skip-switcher) — Hearth has: Minimize all (show-desktop-2) (S)
- [x] Exclude a window from screen sharing (title bar menu) — hearth-windows.js "Hide from screen sharing" (and KWin's excludeFromCapture where it has it); hearth_screencast.py "hidden_windows"; cast-picker.qml "hidden from sharing and not listed"; kwin-windows.yml: "the screen-sharing picker leaves the hidden window out", pictures "kwin-window-menu-extras", "kwin-cast-hidden" (needs testing on a device)
- [x] Picture-in-picture windows stay on top under KWin — shortcuts.json "rules": "picture-in-picture" (Firefox's "Picture-in-Picture", Chromium's "Picture in picture", by title and app: Firefox, Firefox ESR, LibreWolf, Chromium, Chrome, Brave, Edge, Vivaldi), hearth-windows.js `ruleMatches`; kwin-windows-mock.js "picture-in-picture windows stay on top", test_kwin_windows.py "test_picture_in_picture_rule"; kwin-windows.yml: "Firefox's picture-in-picture window stays on top", "the picture-in-picture window is still on top with another window in front", picture "kwin-pip-on-top". Hearth Player has no picture-in-picture (WebKitGTK has none) (needs testing on a device)

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

- [x] Caps Lock / Num Lock and keyboard layout OSD — SystemOsd.qml "Caps Lock on", hearth-notifyd (the keyboards' LEDs; KWin's org.kde.keyboard "layoutChanged"; labwc: Settings › Keyboard's list, switched from the bar's layout item); test_notify_widgets_114.py, notify-widgets.yml pictures "08-osd-caps", "09-osd-layout", "kwin-03-layout-osd" (KWin switching itself) (needs testing on a device)
- [x] Microphone mute OSD and "app tried to use a muted microphone" — SystemOsd.qml "Microphone muted", "The microphone is muted" (NotifyHub.micApps: PipeWire's Stream/Input/Audio while the default mic is muted); notify-widgets.yml pictures "10-osd-mic", "11-osd-mic-wanted" (needs testing on a device)

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
- [x] Edit mode: add, move, resize and remove widgets on the desktop and in the bar — Widgets.qml, WidgetGallery.qml, WidgetArrange.qml, WidgetStore.qml, BarWidgets.qml; 1.14 round 2: the bar's own items moved, ordered and hidden (BarArrange.qml, BarLayout.qml, widget-tools.js "barLayout"; right-click the bar or Settings › Bar and dock › Arrange the bar); test_notify_widgets_114.py "test_bar_items_every_one_once_flame_kept_new_ones_in_place"; notify-widgets.yml pictures "19-bar-arranged", "20-arrange-items" (needs testing on a device)
- [x] More panels (several bars, on several screens, clone a panel) — ExtraPanels.qml, BarArrange.qml "Clone the main bar", BarLayout.qml "p:<id>"; test_notify_widgets_114.py "test_panels"; notify-widgets.yml pictures "21-arrange-panels", "22-panels", "kwin-05-panel" (needs testing on a device)
- [x] Sticky notes on the desktop — WidgetNotes.qml; widgets-test.yml (needs testing on a device)
- [x] System monitor sensors on the desktop and in the bar — WidgetMonitor.qml ("hearth-tasks sample"), BarWidgets.qml; widgets-test.yml (needs testing on a device)
- [x] Analog/binary/fuzzy clocks, world clocks — WidgetClock.qml ("binary", "fuzzy", cities), world clocks in the clock's panel (ClockPanel.qml "zoneTime", Settings › Date & time); test_notify_widgets_114.py "test_clocks"; notify-widgets.yml pictures "w-clock", "w-clock-2", "23-clock-panel-calendars" (needs testing on a device)
- [x] Alternative calendars and holidays in the calendar — calendars.js (Islamic, Hebrew, Persian, Julian, Indian; ten countries' holidays and Islamic / Jewish / Persian days), WidgetCalendar.qml, ClockPanel.qml, SetDateTime.qml "The clock's panel"; test_notify_widgets_114.py "test_known_dates", "test_holidays"; notify-widgets.yml pictures "w-calendar", "23-clock-panel-calendars" (needs testing on a device)
- [x] Weather report widget (forecast, °C/°F, place by name) — WidgetWeather.qml, hearth-widgets (Open-Meteo); widgets-test.yml (needs testing on a device)
- [x] Folder view widget — WidgetFolder.qml; widgets-test.yml (needs testing on a device)
- [x] Picture frame, web page, timer, calculator, colour picker, keyboard indicator — WidgetPhotos.qml, WidgetTimer.qml, and 1.14 round 2: WidgetWeb.qml (drawn by a browser with no window in a profile of its own, hearth-widgets "webshot"), WidgetCalc.qml (widget-tools.js "calc", never eval), WidgetColour.qml (KWin's picker or slurp + grim), WidgetKeys.qml; test_notify_widgets_114.py; notify-widgets.yml pictures "w-web", "w-calc", "w-colour", "w-keys" (dictionary, disk quota, comic, fifteen puzzle are in "Not for Hearth?") (needs testing on a device)
- [x] Window list widget, pager with window outlines — WidgetWindowlist.qml, WidgetPager.qml (outlines from KWin's frames; labwc doesn't say which desktop a window is on, so there it shows the one you're on); notify-widgets.yml pictures "w-windowlist", "w-pager" (activity pager: the activity switcher, ActivitySwitcher.qml) (needs testing on a device)
- [x] Printers queue in the tray — Islands.qml "cPrinters" (hearth-notifyd parse_lpstat, Cancel per job); test_notify_widgets_114.py "test_tray_readers"; notify-widgets.yml picture "02b-tray" (needs testing on a device)
- [x] Disks & devices in the tray — Islands.qml "cDisks" (hearth-notifyd parse_lsblk; Open, Safely remove, Eject through udisksctl); test_notify_widgets_114.py "test_tray_readers"; notify-widgets.yml picture "02b-tray" (needs testing on a device)
- [x] Camera / microphone / screen-recording "in use" indicator — CaptureIndicator.qml (recording, VNC), and 1.14 round 2: Islands.qml "cPrivacy" (the camera: hearth-notifyd camera_users, /dev/video* open; the microphone: PipeWire's recording streams); notify-widgets.yml picture "02b-tray" (needs testing on a device)
- [x] Background apps in the tray — Islands.qml "cBackground" (the portal's org.freedesktop.background.Monitor, else flatpak ps; Quit); notify-widgets.yml picture "02b-tray" (needs testing on a device)
- [x] Quick light/dark tile in Quick settings — QuickAccess.qml "Tile { navId: theme" (Auto off says so); test_notify_widgets_114.py "test_light_dark_tile"; notify-widgets.yml picture "14-quick-access" (needs testing on a device)

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
- [x] History kept across restarts — DeskState.qml (~/.local/state/nimbus/notifications.json, one key per notification, SettingsWriter; Settings › Notifications › Keep the list after a restart); test_notify_widgets_114.py "test_history_kept_atomically_and_buttons_kept"; notify-widgets.yml "restored after it" check, picture "24-history-after-restart" (needs testing on a device)
- [x] Per-app notification settings (allowed, pop-ups, sound, lock screen, priority) — hearth-notify, NotifyRules.qml, SetNotifications.qml; test_system_settings_114.py "test_only_changes_are_kept_and_merged", "test_the_server_follows_the_rules" (badges on the dock: see below; the lock screen part on labwc's lock only, docs/SYSTEM-SETTINGS.md) (needs testing on a device)
- [x] Automatic Do Not Disturb in full-screen apps and while screen sharing, with a missed count — NotifyHub.qml "autoReason", Notifications.qml "onQuietEnded"; test_notify_widgets_114.py "test_automatic_quiet_never_touches_your_switch"; notify-widgets.yml "a notification popped up in full screen" check, pictures "04-quiet-fullscreen", "05-quiet-ended" (needs testing on a device)
- [x] Do Not Disturb on a schedule, and its shortcut — NotifyHub.qml "inSchedule", SetNotifications.qml "On a schedule", Super+Alt+N (labwc rc.xml "W-A-n", shortcuts.json "dnd"); test_notify_widgets_114.py "test_dnd_shortcut_on_both"; notify-widgets.yml picture "12-settings-notifications" (needs testing on a device)
- [x] Progress of jobs and file transfers (JobViewServer) — hearth-notifyd (org.kde.JobViewServer V1 and V2, only when nobody else has it), Islands.qml "cJobs", ClockPanel.qml (Pause, Cancel), a notification when done; a notification's "value" hint as a bar; test_notify_widgets_114.py "test_jobs_v1_and_v3"; notify-widgets.yml pictures "03-clock-panel-jobs", "kwin-02-clock-panel-jobs" (needs testing on a device)
- [x] Reply from a notification, actions kept in history — NotifReply.qml; DeskState.qml "invoke" (each notification stays alive while it's in the list; after a restart, Open <app>), ClockPanel.qml; test_notify_widgets_114.py; notify-widgets.yml picture "07-history-with-buttons" (needs testing on a device)
- [x] Badges and progress on dock icons (Unity launcher API) — hearth-notifyd "com.canonical.Unity.LauncherEntry", Dock.qml "DockBadge" (a running job's progress too); notify-widgets.yml pictures "02c-dock-badge", "kwin-01-popup-job-badge" (needs testing on a device)
- [x] Notification sounds and a sound theme for system events — Sounds.qml, SetSounds.qml (Settings › Sound › Sounds), desktop/sounds/hearth (Hearth's own, tools/make-hearth-sounds.py, GPL-3.0; the freedesktop set played if installed: desktop/sounds/LICENSES.md); test_notify_widgets_114.py "test_every_event_has_hearths_own_sound_and_the_licences_are_written_down"; notify-widgets.yml picture "13-settings-sounds" (needs testing on a device)

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
- [x] Phone controls PC volume, lock, presentation remote — hosts/hearth-connect/src/plugins/lock.rs (never unlocks), systemvolume.rs, presenter.rs; test_phone_link.py: "test_20a_lock_this_pc_from_the_phone"; phone-link.yml round2 pictures (needs testing on a device)
- [x] PC sends its notifications to the phone; PC as a remote keyboard for the phone — plugins/sendnotify.rs (session bus watch, choices in settings.json "notifyOut"), plugins/remote.rs, PhoneRemote.qml; test_phone_link.py: "test_20b_this_pcs_notifications_on_the_phone", "test_20c_this_pc_types_and_points_on_the_phone" (needs testing on a device)
- [x] Signal report, stop the screensaver while connected, share text and URLs — daemon.rs set_awake (logind idle:sleep inhibitor), plugins/mod.rs share_notice (Copy / Open), "send"; test_phone_link.py: "test_20d_calls_mute_and_staying_awake", "test_20e_text_and_links_both_ways" (needs testing on a device)
- [ ] Phone as a second screen, Bluetooth link (L) — later (the owner's round 2 list leaves them out)
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
- [ ] Tags, ratings and comments (M) — partly: tags (user.xdg.tags, Tags › menu, a Tags list in the left menu, find by tag; hearth_files_extras.py, FilesExtras.qml; test_files_extras.py: "test_tags_set_list_find_and_in_listings"; files-extras.yml pictures). Left: ratings and comments (S)
- [x] Group by — FileManager.qml (type, date, size, tag); files-extras.yml "files-x1-tags-grouped" (needs testing on a device)
- [x] Git status badges — hearth_files_extras.py git_status, FilesMarks.qml; test_files_extras.py: "test_git_status_badges"; files-extras.yml "files-x4-git" (needs testing on a device)
- [x] Act as administrator — FilesExtras.qml, os/system_files/usr/libexec/nimbus/hearth-files-admin (never /boot, /efi, the image, fstab, kernel/boot settings), 49-hearth-admin.rules AUTH_ADMIN_KEEP; test_files_extras.py: "test_never_what_starts_the_pc"; files-extras.yml "files-x6-admin" (needs testing on a device)
- [x] Checksums, permissions and owner tabs in Properties — FileProperties.qml (SHA-512, compare a pasted one, group, whole folder, owner as administrator); test_files_extras.py: "test_chmod_tree_and_groups"; files-extras.yml "files-x7-checksums", "files-x8-permissions" (needs testing on a device)
- [x] Mount an ISO, MTP phones, PTP cameras — hearth_files_extras.py iso_mount (udisks, read only), portable (gvfs); test_files_extras.py: "test_iso_mount_and_close_through_udisks", "test_phones_and_cameras_from_gio"; files-extras.yml "files-x5-iso-menu", "files-x1-tags-grouped" (needs testing on a device)
- [x] Service menus — hearth_files_extras.py service_menus / service_run (kio/servicemenus, kservices5/ServiceMenus); test_files_extras.py: "test_service_menus_kde_style"; files-extras.yml "files-x2-menu" (needs testing on a device)

### File search and indexing

- [ ] File indexing (names and contents) with folders to skip and battery limits — on unmerged feature/search-store (XL)
- [x] Recent files tracking and "forget recent files" — SetRecentFiles.qml, hearth_files_extras.py recent_*; test_files_extras.py: "test_recent_files_privacy"; files-extras.yml "files-x10-recent-privacy" (needs testing on a device)

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
- [ ] App permissions page (Flatpak overrides) (M)
- [ ] Firmware updates (fwupd) with a notification — partly: SetFirmware.qml, hearth-firmware, hearth-sysset-admin (parts install after a password; system firmware shown, never installed: red flag 2, docs/SYSTEM-SETTINGS.md); test_system_settings_114.py "test_one_rule_for_system_firmware". Left: a notification when an update is there, the history (S)
- [x] Firmware security (Secure Boot, TPM, HSI level) — SetFirmware.qml, hearth-firmware; test_system_settings_114.py "test_security_level", "test_secure_boot_words" (needs testing on a device)
- [ ] Shut down or restart after installing updates (S)
- [ ] Backups of your files to a drive (feature/pc-backups exists, not merged here) (M)

### Network

- [x] Wi-Fi networks, hidden, forget, properties, hotspot — `SetWifi.qml`, `DeviceSettings.qml`, SetHotspot.qml
- [x] Tailscale — `SetHomeTailscale.qml`
- [ ] Wired, mobile broadband, editing any connection — partly: Connection details (DNS, metered, random MAC, IP by hand, IPv6, auto-connect: SetNetDetails.qml, hearth-network; test_system_settings_114.py "test_details_args_are_checked"). Left: wired and mobile broadband (SIM, APN), 802.1X/enterprise Wi-Fi (L)
- [ ] VPN — partly: WireGuard and OpenVPN from their file, connect, OpenVPN account asked when connecting (SetVpn.qml, hearth-network; test_system_settings_114.py "test_vpn_password_goes_on_stdin_never_argv"). Left: IPsec/StrongSwan, L2TP, PPTP, OpenConnect, SSTP, Fortinet (M)
- [ ] Wi-Fi QR codes (show one, scan one) (S)
- [x] Proxy settings (none, automatic, by hand) — SetProxy.qml, hearth-network; test_system_settings_114.py "test_proxy_saved_with_settings_io_and_handed_on" (needs testing on a device)
- [ ] Firewall page (M)
- [ ] Duplicate a connection, network speed in the tray (S)
- [x] "Share this folder" (SMB) in Files' Properties — FileProperties.qml Sharing tab through hearth-home-server; test_files_extras.py: "test_share_this_folder_through_home_server"; files-extras.yml "files-x9-sharing" (needs testing on a device)

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
- [ ] Language of the system, several in order — partly: language (SetRegion.qml, hearth-region, 50 languages in the image; test_system_settings_114.py "test_set_locale_runs_localectl_with_a_list"). Left: several languages in order (LANGUAGE), downloading translations (S)
- [x] Formats: numbers, currency, dates, units, paper size — SetRegion.qml, hearth-region "FORMAT_KEYS = ('LC_NUMERIC', 'LC_TIME', 'LC_MONETARY', 'LC_MEASUREMENT', 'LC_PAPER')"; test_system_settings_114.py "test_localectl_parse" (needs testing on a device)
- [ ] Spell check settings for apps (S)
- [ ] Input methods for Chinese/Japanese/Korean (fcitx5) (M)

### Look

- [x] Dark / light / automatic, accent colour — `SetAppearance.qml`, `Theme.qml`
- [x] Wallpaper: pictures, moving, your own — `Wallpaper.qml`, `SetAppearance.qml`
- [x] Boot splash choice — `SetBoot.qml`, `SetBootFrames.qml`
- [x] Fonts viewer, install and remove — `hosts/hearth-fonts`
- [ ] Wallpaper per screen, slideshow, picture of the day, light/dark wallpaper — partly: a wallpaper per activity (Activities.qml, `Theme.activityWall`). Left: per screen, slideshow, picture of the day (M)
- [ ] Save your own look as a theme (S)
- [ ] System fonts — partly: UI font and size, fixed-width font, smoothing, hinting, font DPI (SetFonts.qml, hearth-lookset; test_system_settings_114.py "test_set_saves_merges_and_hands_on_only_to_hearths_files"). Left: small and title bar fonts, sub-pixel order (S)
- [x] Cursor theme and size — SetFonts.qml, hearth-lookset; test_system_settings_114.py "test_theme_lists" (needs testing on a device)
- [x] Icon theme choice — SetFonts.qml, hearth-lookset; test_system_settings_114.py "test_theme_lists" (needs testing on a device)
- [ ] App style: window frame colour intensity, "apps follow Hearth" (S)
- [ ] Splash at sign-in (S)
- [ ] Keyboard RGB lights follow the accent (M)

### Sound

- [x] Volume, mute, over-amplification, devices, per-app mixer — `SetSound.qml`, `Mixer.qml`
- [ ] Device profiles, balance, speaker test — partly: sound card profiles (SetSound.qml, hearth-sound; test_system_settings_114.py "test_profile_checked_and_set_with_a_list"). Left: balance per channel, speaker test (S)
- [ ] Microphone test and push-to-talk mute shortcut — partly: the microphone test (hearth-sound, test_system_settings_114.py "test_levels"). Left: push-to-talk (S)
- [ ] Move one app's sound to another device, per-app input (S)
- [ ] Scroll on an app's dock icon to change its volume (S)
- [ ] Warning when "raise maximum volume" stays on (S)

### Power

- [x] Power profiles, timers, power button, low battery — `SetPower.qml`, `IdlePower.qml`, `BatteryWatch.qml`
- [ ] On AC / on battery / low battery settings, lid action, dim before off (M)
- [x] Battery charge limit — SetPower.qml through hearth-hwset (/etc/hearth/hardware.json); test_system_settings_114.py "test_charge_limit_through_hwset" (needs testing on a device)
- [ ] Apps blocking sleep, Hibernate and hybrid sleep (S)
- [ ] Battery health, cycle count, energy graphs — partly: health shown with the charge limit (SetPower.qml). Left: cycle count and UPower's history graphs (M)
- [ ] Batteries of Bluetooth devices, mice and controllers in Quick settings (S)

### Devices and hardware

- [x] Bluetooth: on/off, pair, connect — `SetBluetooth.qml`
- [x] Printers — `SetPrinters.qml`
- [x] Game controllers: test and calibrate — `hosts/hearth-gaming`, `SetController.qml`
- [ ] Bluetooth: send/receive files, battery, rename, trust/block (M)
- [ ] Printers: queue window, ink notices, Windows shared printers (M)
- [ ] Scanners: multi-page PDF and feeders, checked (S)
- [x] Thunderbolt devices: allow once or always, forget — SetThunderbolt.qml, hearth-thunderbolt; test_system_settings_114.py "test_allow_and_forget", "test_no_bolt" (needs testing on a device)
- [ ] Removable storage: what happens on plug-in (S)
- [ ] USB device permission prompts (M)
- [ ] Touchscreen calibration (S)

### Startup, session, defaults

- [x] Default apps per kind — `SetDefaults.qml`
- [ ] Autostart page (S)
- [x] Background services page — SetServices.qml, hearth-services; test_system_settings_114.py "test_a_service_for_everyone_is_masked_for_you" (needs testing on a device)
- [ ] File associations editor (M)
- [ ] Desktop session rows — partly: "Reopen my apps from last time" (SetStartup.qml). Left: confirm logout, default leave option (S)
- [ ] Menu editor ("Edit app…") (M)

### Info and system

- [x] About this PC — `SetAbout.qml`, `ResourceMonitor.qml`, `DeviceManagerView.qml`
- [x] Task manager — `TaskManager.qml`, `hearth-tasks`
- [x] Disk manager and SMART — `hosts/hearth-disks`, `SetHomeDrives.qml`
- [ ] Info Center's technical pages (OpenGL/Vulkan/Wayland, modules, interrupts, PCI/USB, Samba, sensors) (M)
- [ ] Out-of-memory notification with tips (S)
- [ ] Welcome app once per new version (S)

### Hearth-specific extras

- [ ] Emoji picker: mixed skin tones, sizes (S)
- [ ] On-screen keyboard press-and-hold for special characters, checked on touch (S)

---

## Not for Hearth? (the owner decides)

The list in feature/kde-feature-list's KWIN.md stands as it is, plus:

- **RDP server (KRdp)**: dropped by the owner (Nov 16, 2026) for the VNC server (hearth-vnc).

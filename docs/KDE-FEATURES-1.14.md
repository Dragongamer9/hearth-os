# Every KDE Plasma feature in Mantel on KWin: where 1.14 stands

The master checklist (feature/kde-feature-list's KWIN.md, made at 81a2a1e) checked item by item against
**feature/kwin-1.14 at 00ced87** (Nov 16, 2026), recounted on feature/kwin-1.14 on Nov 17. The owner's rule: everything KDE Plasma 6 has that Mantel lacks is
written for Hearth from scratch.

**How to read it.**

- `- [x] part — file: "proof"`: built, with a test or CI picture that shows it. "(needs testing on a device)" when only
  CI has proven it, as in docs/ROADMAP.md.
- `- [ ] part — partly: …`: some of it is built (what, and where); the line says what's left.
- `- [ ] part`: not built on feature/kwin-1.14.
- Sizes for what's left: **S** (a day or less), **M** (a few days), **L** (a week or two), **XL** (several weeks).
- `feature/kwin-pages` (Settings pages for KWin's own settings) and `feature/search-store` (search, the file index,
  Passwords, the App store, Files' terminal panel and split view) are merged into feature/kwin-1.14 now; their lines
  are recounted from the integration branch.

Counts (done · partly · missing), recounted on feature/kwin-1.14: Stage 1 28 · 1 · 0, Stage 2 49 · 7 · 19, Stage 3 96 · 16 · 50.

---

## Stage 1: the KWin base

### Session and safety

- [x] KWin session for Desktop Mode — hosts/linux/session/nimbus-session: "run_kwin … exec "$1" --xwayland $3 $HEARTH_KWIN_ARGS "$2"", hosts/linux/desktop/kwin/autostart; test_kwin_session.py, kwin-test.yml (needs testing on a device)
- [x] Falls back to labwc if KWin won't start, and to Game Mode if neither does — nimbus-session `desktop_wm` / `run_classic` (missing, ends within 15 s, Mantel not up in 45 s, twice → labwc until turned back on); test_kwin_session.py: "test_kwin_dying_fast_opens_labwc_counts_and_twice_stays", "test_mantel_never_up_ends_kwin_and_opens_labwc" (needs testing on a device)
- [x] Mantel's bar, dock, panels and desktop as layer-shell surfaces on KWin — they show with their exclusive zones under KWin (kwin-windows.yml pictures "kwin-halves", "kwin-desktop-menu"); keyboard focus on each panel checked with a real keyboard: tools/e2e-kwin-windows.py "typing reaches the app menu", "the arrow keys move along Quick Access", "typing reaches Hearth Settings' search" (shell.qml IpcHandler "focus") (needs testing on a device)
- [x] **(review, Nov 17)** No lockout or setting change from KDE Plasma's leftovers — kwin/xdg/kscreenlockerrc: "Autolock[$i]=false", kwin/xdg/kwinrc: "InputMethod[$ei]=", kwin/windows/xdg/kwinrc: "Meta[$i]="; hearth-your-desktop `kwin-overrides` (the one-time "Some of your KDE settings aren't used here" card); nimbus-session `kwin_lock_ready`: "hearth-lockcheck --as-kwin", kept per image in kwin-lock-proven; hearth-login-screen: "GOOD_SECS"; hearth-screens `kwinrc_set`: "io.update_text(KWINRC"; SetKeyboard.qml: "Bring back KDE's own shortcuts"; hearth-network / hearth-lookset `plain_write`; test_kwin_session.py "test_the_lock_screen_must_really_load_before_kwins_locker_is_on", "test_your_kde_settings_that_hearth_overrides_are_told_once"; kwin-input.yml lock-debian (KWin 6.3: "KWin didn't lock by itself after 80 s idle"); kwin-test.yml pictures "kwin-7-kde-settings-card", "kwin-8-keyboard-kde-keys" (needs testing on a device)
- [x] Xwayland apps and games — nimbus-session: "--xwayland"; kwin-windows.yml "KWin's X server for the X11 window" (36d3f1a) (needs testing on a device)
- [x] Settings › Your desktop says KWin, and Safe start still works — SetYourDesktop.qml: "KWin didn't start this time, so labwc", "Safe start"; hearth-your-desktop; test_kwin_session.py "test_wm_writes_only_its_line_with_a_backup", "test_safe_graphics_is_labwc" (needs testing on a device)
- [ ] **(found in this audit)** Keyboard layouts, key repeat and pointer settings applied under KWin — partly: the pointer settings are (hearth-kwin-settings pointer-apply / input-watch, from feature/kwin-pages). Left: keyboard layouts and key repeat (feature/w3-keyboard) (M)

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
- [x] Rotation and orientation, primary screen, mirroring — rotation (hearth-screens, test_hearth_screens.py "test_kwin_command"); the main screen: SetDisplay.qml "Make this the main screen", hearth-screens `priority` ("output.HDMI-A-1.priority.1", every screen's kept), test_hearth_screens.py "test_kwin_commands_for_the_new_keys", "test_keep_and_restore_the_new_keys"; mirroring can't be done by KWin 6.3, shown greyed with why (SetDisplay.qml, Super+P's "Duplicate") (needs testing on a device)
- [x] Screen off when idle, screen-off for Server mode — hearth-screen-power: KWin's DPMS through "kscreen-doctor --dpms"; IdlePower.qml; kwin-screens job "screen off near the end" (needs testing on a device)
- [x] Brightness keys and slider — shortcuts.json "brightness-up", "brightness-down" (the same commands as labwc's) (needs testing on a device)

### Lock, typing, screenshots

- [x] Lock screen — kwin-lock/contents/lockscreen/LockScreen.qml, kwin-lock/install.sh, Lock.qml ("hands locking to KWin's locker"), hearth-lockcheck, nimbus-session `kwin_lock_ready` (no lockout: `--no-lockscreen` when it isn't there); kwin-input.yml `lock-debian` locks for real and unlocks through PAM (needs testing on a device)
- [x] Typing for the on-screen keyboard, clipboard paste, emoji picker, snap layouts and the controller — hearth-type, kwin-helpers/hearth-kwin-im.c, kwin/xdg/kwinrc "InputMethod[$e]=/usr/share/applications/hearth-kwin-im.desktop"; test_kwin_input.py "test_keyboard_types_through_hearth_type_on_kwin", "test_paste_and_magnifier"; kwin-input.yml types into a GTK field (needs testing on a device)
- [x] Screenshots (Print, Shift+Print, Super+Shift+S, Ctrl+Print) — os/system_files/usr/bin/hearth-screenshot ("under KWin it runs hearth-screens shot"), hearth-screens `grab`, shortcuts.json "screenshot", "screenshot-region", "screenshot-clipboard"; test_hearth_screens.py "test_screenshot_trust_for_kwin"; kwin-screens job picture with KWin's permission check on (needs testing on a device)
- [x] Screen sharing for browsers, Discord, OBS (the ScreenCast portal) — hearth_screencast.py, cast-picker.qml (Screens, Windows and Area: `{ id: "area", label: "Area", on: root.hasArea }`, KWin's region stream through hearth-kwin-cast "stream region"), PickColor (hearth_screencast.py "self.pick_color(conn, handle, inv)", hearth-kwin-cast "pick": KWin's /ColorPicker), hearth-portals.conf; test_kwin_input.py "test_area_in_desktop_pixels", "test_pick_color"; kwin-input.yml's PipeWire stream; desktop-test.yml kwin-screens picture "kwin-12-area-picker" (needs testing on a device)
- [x] The controller in Desktop Mode (pointer, View+D-pad snap, View+B close, LB/RB windows) — hearth-pad types KWin's (same) keys through its own uinput keyboard and Pad.qml follows WinList; checked on KWin with a uinput controller: tools/e2e-kwin-windows.py "controller: View + D-pad left snaps the window to the left half", "controller: View + B closes the window in front", "controller: RB switches to the other window", "controller: LB switches back"; the lock screen part in test_hearth_pad.py (needs testing on a device)
- [x] Night light, colour filters and magnifier keep working on day one — hearth-access: `apply kwin` (apply_filter_kwin, the magnifier and the look under KWin) (needs testing on a device)

---

## Stage 2: what KWin itself can do, with Hearth Settings pages

Most of this section is what **feature/kwin-pages** builds (merged), with the Display parts from feature/kwin-screens.

### Display and colour

- [x] HDR in Desktop Mode (per screen) — HDR switch, SDR brightness, wide colour and colour intensity (SetDisplay.qml "Colour intensity", hearth-screens `sdrGamut`: "output.DP-1.sdrGamut.70"); test_hearth_screens.py "test_kwin_keep_saves_hdr_vrr_and_profile", "test_kwin_reads_the_new_fields"; picture "kwin-7-hdr-colour-intensity" (needs testing on a device)
- [x] HDR calibration (max/peak brightness wizard) — HdrCalibrate.qml ("Brightest spot", "Brightest across the screen", "Darkest"; Finish goes through Keep: "page.countdown = 15"), hearth-screens `hdrCal` ("output.DP-1.maxBrightnessOverride.800"); test_hearth_screens.py "test_kwin_commands_for_the_new_keys"; picture "kwin-10-hdr-calibrate" (needs testing on a device)
- [ ] Extended Dynamic Range (HDR content on SDR laptop panels) — partly: SetDisplay.qml "Extended Dynamic Range" (Keep needed), hearth-screens `edr` ("edrPolicy."), shown only when kscreen-doctor reports it (test_hearth_screens.py "test_kwin_reads_the_new_fields"). Left: KWin 6.3 (Debian 13) has no EDR, so the row is hidden there; check the kscreen-doctor wording on a newer KWin (S)
- [x] Variable refresh rate (Never / Automatic / Always) — hearth-screens, SetDisplay.qml, docs/SCREENS.md "VRR / FreeSync"; test_hearth_screens.py "test_kwin_keep_saves_hdr_vrr_and_profile" (needs testing on a device)
- [x] Colour profiles — none / the screen's own / an ICC file, and the colour accuracy choice (SetDisplay.qml "Colour accuracy", hearth-screens `colorPower`: "output.DP-1.colorPowerTradeoff.preferEfficiency"); test_hearth_screens.py "test_kwin_commands_for_the_new_keys" (needs testing on a device)
- [ ] Colour depth limit and RGB range — partly: RGB range (SetDisplay.qml "RGB range", Keep needed; hearth-screens "rgbrange."; picture "kwin-8-picture-signal"). Left: colour depth: KWin 6.3's output management (kscreen-doctor) can't set bits per colour, so it's hidden (hearth-screens `depth` "capable": False) (S)
- [x] Fractional scaling per screen and the X11 apps' scaling choice — per-screen scale (hearth-screens); SetDisplay.qml "Scaling X11 apps" (kdeglobals [KScreen] XwaylandClientsScale through hearth-kwin-settings set, the same key as SetKwinGames.qml); test_hearth_screens.py "test_kde_files_change_only_their_keys" (needs testing on a device)
- [x] Night light: by location, custom times, always on, warmth, fade, Quick settings tile — SetDisplay.qml "Sunset to sunrise" (your town by name: hearth-widgets places), "Fade"; Settings.qml nightPlace/nightLat/nightLon/nightFade; KWin `Mode=Location` (hearth-screens "LatitudeFixed"), wlsunset "-l" / "-d"; test_hearth_screens.py "test_night_by_location_and_fade"; kwin-screens job "Night light by location didn't reach kwinrc"; picture "kwin-6-brightness-and-night" (needs testing on a device)
- [ ] Reduce red tint at low brightness (AMD laptops) — not in KWin 6.3 (Plasma 6.7): hidden (hearth-screens `redTint` "capable": False) until a KWin that has it (S)
- [ ] Automatic brightness from the light sensor (S)
- [x] Brightness for external screens (DDC/CI) — SetDisplay.qml "Brightness · " per screen; hearth-screens `ddc` / `ddc-set` (ddcutil, guarded: missing ddcutil or i2c-dev says why; nothing saved, the screen keeps it) or KWin's own control on a newer KWin (`brightness`); test_hearth_screens.py "test_ddc"; picture "kwin-6-brightness-and-night" (needs testing on a device)
- [x] Overscan, allow tearing, wide colour gamut — SetDisplay.qml "Overscan" (Keep needed; hearth-screens "overscan."), tearing (kwinrc `[Compositing] AllowTearing`), wide colour (hearth-screens); test_hearth_screens.py "test_kwin_commands_for_the_new_keys"; picture "kwin-8-picture-signal" (needs testing on a device)
- [ ] Virtual desktops only on the main screen, or per screen — partly: "Main screen only" (SetDisplay.qml "Virtual desktops", hearth-main-desktops.js: the other screens' windows on every desktop, put back when off; screens.json `desktopsMainOnly`, loaded again at start); test_hearth_screens.py "test_main_desktops_script". Left: each screen its own desktops needs a newer KWin than 6.3 (shown greyed) (S)
- [x] Screen edge barrier and corner barrier between screens — SetDisplay.qml "Sticky edge between screens", "Sticky corners" (kwinrc [EdgeBarrier] through hearth-kwin-settings set, the same keys as SetKwinEdges.qml); test_hearth_screens.py "test_kde_files_change_only_their_keys"; picture "kwin-9-several-screens" (needs testing on a device)
- [x] Mirror / extend / only one screen quick switcher (Super+P) — ScreenSwitcher.qml ("This screen only", "Duplicate" with why, "Extend", "Second screen only"; Keep or back in 15 s), hearth-screens `plan`, shortcuts.json "screens-switch" Meta+P, labwc rc.xml W-p; test_hearth_screens.py "test_super_p_plans"; picture "kwin-11-super-p" (needs testing on a device)

### Desktop effects (feature/kwin-pages, merged)

- [x] Animation speed (one slider) and Reduced motion respected — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [x] Window open/close: Fade, Glide, Scale, Fall Apart — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [x] Minimize: Magic Lamp, Squash — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [x] Maximize, geometry-change, morphing and sliding popups — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [x] Desktop switch: Slide, Fade, Cube — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [ ] Desktop Cube (S)
- [x] Wobbly windows — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [x] Blur and background contrast behind Hearth's panels — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [x] Translucency (moving, inactive, menus) — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [x] Dim inactive windows, dim behind password prompts — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [x] Rounded corners and outlines for windows (Hearth's frames) — hearth_aurorae.py: all four corners rounded (`bottom_radius`, as far as the window's own square corner fits inside the frame), 4 px sides and bottom ("SIDE = 4"), a light 1 px outline all round (`outline_color`); shadow, maximized and live look change kept; test_kwin_windows.py "test_rounded_bottom_corners_and_outline"; kwin-windows.yml: "the frame's bottom corners are rounded", "the frame has a light outline round it", pictures "kwin-frame-shadow-light-corner", "kwin-frame-shadow-dark-corner" (needs testing on a device)
- [ ] Rounded corners for apps that draw their own frames (Steam, GTK header bars) — needs a KWin effect that clips windows' corners; KWin 6.3 has none built in (M)
- [x] Peek at desktop / Show desktop animation, login and logout fades — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [x] Mouse click animation, mouse mark, track mouse, touch points — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [x] Shake cursor to find it — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [ ] Hide the pointer while typing or after a time (S)
- [ ] Snap helper / window aperture / thumbnail aside / show FPS / show paint (S)
- [x] Startup feedback (busy pointer or bouncing icon) — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [x] Visual bell — SetAccessibility.qml (kaccessrc Bell), hearth-kwin-settings (needs testing on a device)

### Overview, switcher, desktops

- [x] Overview (Super+W): all windows and desktops, search, drag windows between desktops — KWin's own, in Hearth's KDE colour scheme, opened from Mantel: Dock.qml / TaskStrip.qml Task view button "WinList.effect(\"overview\")", shortcuts.json "overview" (Meta+W, KDE's freed), hearth-pad View + Y; Milou in the image (build.sh "optional milou"); kwin-windows.yml: "Super+W opens the Overview", picture "kwin-overview" (needs testing on a device)
- [x] Window view / Present windows — KWin's, from the Task view button's menu and Ctrl+F10 (shortcuts.json "present-windows"); kwin-windows.yml picture "kwin-present-windows" (needs testing on a device)
- [x] Desktop grid — KWin's, from the Task view button's menu and Ctrl+F8 (shortcuts.json "desktop-grid"); kwin-windows.yml picture "kwin-desktop-grid" (needs testing on a device)
- [x] Window switcher styles (Thumbnail grid, Large icons, … Cover switch, Flip switch) and their settings — SetKwinEffects.qml, hearth-kwin-settings (needs testing on a device)
- [ ] Second switcher (Alt+`, same app's windows) (S)
- [x] Virtual desktops: add, remove, rename, rows, wrap around, OSD — SetKwinDesktops.qml (needs testing on a device)
- [ ] Different tile layouts per desktop — needs newer KWin: KWin's tiles are one layout per screen in 6.3 (Debian 13); a layout per virtual desktop came with Plasma 6.4 (KDE bug 466031). Snap layouts' zones (SnapLayouts.qml) are the same on every desktop until then (S)

### Window management

- [x] Tiling editor (Super+T, Shift-drag into tiles) — SetKwinTiling.qml (needs testing on a device)
- [ ] Window behaviour: focus policy, raise on hover, click raises, focus stealing prevention — Hearth has: ClickToFocus fixed (kwin/xdg/kwinrc) (S)
- [ ] Title bar actions and Super+drag — partly: double-click maximizes (kwin/windows/xdg/kwinrc "TitlebarDoubleClickCommand=Maximize"). Left: wheel, middle-click, Super+drag move/resize choices (S)
- [ ] Window placement for new windows (S)
- [x] Snap zones: edges stick, gap sizes — SetKwinTiling.qml (needs testing on a device)
- [x] Window rules page (per app, "Detect window", presets) — SetKwinRules.qml (needs testing on a device)
- [ ] Window menu "More actions" — partly: KWin's own window menu (keep above, all desktops, fullscreen) with Hearth's Snap. Left: shade, no border and special window settings in Hearth's look (S)
- [x] Kill a frozen window (Ctrl+Alt+Esc) in Hearth's look — ForceClose.qml ("Click the window that isn't responding", "Force close"), title bar menu "Force close (not responding)…", hearth-kwin-bridge "queryWindowInfo" / "KILL_WAIT" / "killable"; kwin-windows.yml: "Force close ends a frozen window (asked, then made to)", pictures "kwin-force-close-pick", "kwin-force-close". Left: KWin's own automatic "not responding" notice stays KWin's (needs testing on a device)
- [x] Session restore: reopen apps after restart — SessionRestore.qml, SetStartup.qml "Reopen my apps from last time" (off by default, WinList); test_system_settings_114.py "test_off_by_default_and_only_at_sign_in" (needs testing on a device)
- [ ] Plasma's KWin scripts (Minimize all, Desktop change OSD, video wall, sync skip-switcher) — Hearth has: Minimize all (show-desktop-2) (S)
- [x] Exclude a window from screen sharing (title bar menu) — hearth-windows.js "Hide from screen sharing" (and KWin's excludeFromCapture where it has it); hearth_screencast.py "hidden_windows"; cast-picker.qml "hidden from sharing and not listed"; kwin-windows.yml: "the screen-sharing picker leaves the hidden window out", pictures "kwin-window-menu-extras", "kwin-cast-hidden" (needs testing on a device)
- [x] Picture-in-picture windows stay on top under KWin — shortcuts.json "rules": "picture-in-picture" (Firefox's "Picture-in-Picture", Chromium's "Picture in picture", by title and app: Firefox, Firefox ESR, LibreWolf, Chromium, Chrome, Brave, Edge, Vivaldi), hearth-windows.js `ruleMatches`; kwin-windows-mock.js "picture-in-picture windows stay on top", test_kwin_windows.py "test_picture_in_picture_rule"; kwin-windows.yml: "Firefox's picture-in-picture window stays on top", "the picture-in-picture window is still on top with another window in front", picture "kwin-pip-on-top". Hearth Player has no picture-in-picture (WebKitGTK has none) (needs testing on a device)

### Screen edges, gestures, input

- [x] Hot corners / screen edges page — SetKwinEdges.qml (needs testing on a device)
- [x] Touchpad gestures (3/4-finger swipes, pinch) listed and switchable — SetMouse.qml lists KWin's gestures (switching them isn't possible on KWin 6.3) (needs testing on a device)
- [x] Touchscreen edge swipes and gestures — SetKwinEdges.qml (needs testing on a device)
- [x] Touchpad: the missing rows (disable with a mouse, middle-click emulation, scroll speed, edge scrolling, click method) — hearth-kwin-settings pointer-apply, SetMouse.qml (needs testing on a device)
- [ ] Mouse: acceleration profile, scroll speed, middle-click paste, per-device (S)
- [ ] Mouse buttons: re-map extra buttons (M)
- [ ] Keyboard: Num Lock at start, layouts per window, compose key, typing delay (S)
- [x] Touchscreen: which screen, turn it off — SetKwinTablet.qml, KwinCalibrate.qml (needs testing on a device)
- [x] Drawing tablets — SetKwinTablet.qml, KwinCalibrate.qml (needs testing on a device)
- [ ] Game controllers as regular mouse/keyboard input left off — partly: Hearth's controller mouse (hearth-pad). Left: KWin's option explicitly off in Hearth's kwinrc defaults (S)
- [ ] Sync the mouse and stylus pointers (S)

### Accessibility (KWin's parts)

- [x] Zoom with Mantel's keys, follow modes, pixel grid — SetAccessibility.qml, hearth-access (needs testing on a device)
- [x] Magnifier (a lens around the pointer) — SetAccessibility.qml, hearth-access (needs testing on a device)
- [x] Colour filters in Desktop Mode (greyscale, protanopia, deuteranopia, tritanopia, intensity) — SetAccessibility.qml, hearth-access (needs testing on a device)
- [ ] Invert colours (whole screen, one window) — Hearth has: labwc's gamma invert; nothing under KWin (Stage 1's line) (S)
- [ ] Sticky, slow, bounce and toggle keys, mouse keys (M)
- [ ] Dwell click (S)
- [x] Shake cursor and Track mouse on Accessibility — SetAccessibility.qml, hearth-access (needs testing on a device)
- [ ] Activation sounds / visual bell (S)

### Xwayland and gaming

- [x] Xwayland: X11 apps reading keys typed elsewhere (push-to-talk), X11 apps' scaling — SetKwinGames.qml (needs testing on a device)
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

### Search (KRunner-style) — the framework and plugins are on feature/search-store (merged)

- [x] Search apps, Settings pages and single settings, games — `Launcher.qml`, `menu-search.js`, `settings-index.js`
- [x] Run box (Super+R) — `WinTools.qml`, `RunView.qml`, `run-map.js`
- [ ] KRunner-style search bar (Alt+Space) with plugins in an order you choose, "?" help (L)
- [x] Plugin: calculator, unit conversion, currency — search/calc.js, search/units.js, search/currency.js (needs testing on a device)
- [x] Plugin: files and folders by name and content — hearth-search (SQLite FTS5 index) (needs testing on a device)
- [x] Plugin: recent files and places, bookmarks, browser history — search/recent.js, search/browser.js (Firefox and Chromium-based, read from a copy: hearth-search-sources "read_copy"); test_search_plugins_114.py "test_browser_reads_a_copy_even_while_the_browser_holds_its_lock"; search-plugins.yml picture "01-bookmarks" (needs testing on a device)
- [x] Plugin: open windows, virtual desktops, activities by name — search/windows.js, search/desktops.js (KWin desktops, Activities.qml); test_search_plugins_114.py "test_desktops_and_activities"; search-plugins.yml picture "02-activities" (needs testing on a device)
- [x] Plugin: run a command, terminal command, kill an app — search/commands.js, search/kill.js ("kill <name>", asks first: SearchSources.qml "dialog.confirm"; never Hearth's own programs); test_search_plugins_114.py "test_kill_asks_first_and_never_hearths_own", "test_procs_and_kill"; search-plugins.yml pictures "03-kill", "04-kill-asks-first" (needs testing on a device)
- [x] Plugin: web search keywords — search/web.js (needs testing on a device)
- [x] Plugin: time in other cities, time-zone conversion — search/worldclock.js ("time in tokyo", "3pm london to tokyo"; summer time from hearth-search-sources zones); test_search_plugins_114.py "test_world_clock", "test_zones_follow_summer_time"; search-plugins.yml picture "05-world-clock" (needs testing on a device)
- [x] Plugin: spell check, dictionary, special characters, emoji, colours — search/spell.js (hunspell; "define" opens Wiktionary), search/chars.js, search/emoji.js, search/colours.js (swatch in Launcher.qml / MenuSearch.qml); test_search_plugins_114.py "test_spell_and_dictionary", "test_special_characters", "test_colours"; search-plugins.yml pictures "06-spell", "07-characters", "08-colour" (needs testing on a device)
- [x] Plugin: power and session, media control, store results — search/power.js (Restart, Shut down, Log out ask first), search/media.js (MPRIS), search/store.js ("install krita"); test_search_plugins_114.py "test_power_asks_first_where_it_closes_apps", "test_media", "test_store"; search-plugins.yml pictures "09-restart", "10-restart-asks-first", "11-media", "12-store" (needs testing on a device)
- [x] Plugin: Hearth contacts, calendar events, notes — search/contacts.js, search/events.js, search/notes.js (hearth-search-sources pim: Hearth Contacts' .vcf, hearth-time --events, ~/Documents/Notes); test_search_plugins_114.py "test_contacts_events_notes", "test_pim"; search-plugins.yml pictures "13-contacts", "14-calendar", "15-notes" (needs testing on a device)
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
- [ ] Phone as a second screen, Bluetooth link (L) — later, the owner's call
- [x] Phone tray item and Settings page — PhoneStatus.qml, SetPhone.qml, Quick Access tile; phone-link.yml pictures (needs testing on a device)

### Remote desktop and screen recording

- [x] Use this PC from anywhere (Moonlight through Tailscale, Sunshine) — `hearth-remote-desktop`, `SetRemote.qml`
- [x] Remote desktop server (the owner dropped RDP for VNC) — hearth-vnc (VeNCrypt TLS only, Tailscale/home network, password in the keyring, lock-out after five tries, view only, Disconnect), SetRemoteDesktop.qml, KWin control through Hearth's RemoteDesktop portal ("Allow control?", hearth-kwin-cast input) with keyboard, pointer and touch (hearth_screencast.py "KEYBOARD | POINTER | TOUCHSCREEN", hearth-kwin-cast "td" / "tm" / "tu" / "tf"); test_remote_record.py, test_kwin_input.py "test_touch_lines", remote-record.yml pictures; docs/ROADMAP.md's "Remote desktop and recording" line ticked (needs testing on a device)
- [x] Screen recording, instant replay, sound recorder — `hosts/hearth-recorder`
- [ ] Record a window, system sound and the microphone together, choose format and quality — partly: hearth-record and Recorder.qml (a screen or a window through the portal, this PC's sound and the microphone, 30/60 fps, GPU encoder with x264 fallback); test_remote_record.py. Left: a format choice (MP4 only) and recording popups (S)
- [ ] Screenshot tool like Spectacle — partly: Hearth Capture (Capture.qml, CaptureEditor.qml, hearth-capture-helper: all / screen / window / area, delay, pointer, arrows, boxes, pen, text, blur, pixelate, numbers, crop, copy/save, OCR with tesseract, QR codes); test_hearth_capture.py, capture-polkit-test.yml pictures. Left: scrolling capture, share (S)

### Files (Dolphin's features)

- [x] Views, tabs, places, Trash, previews, filter, search, hidden files, archives, batch rename, Open in Terminal, network shares, undo, details, properties — `FileManager.qml`, `hearth-files`, `FileProperties.qml`
- [x] Split view (F3) — FilesSplitPane.qml (needs testing on a device)
- [x] Terminal panel (F4) — FilesTerminal.qml (needs testing on a device)
- [ ] Tags, ratings and comments (M) — partly: tags (user.xdg.tags, Tags › menu, a Tags list in the left menu, find by tag; hearth_files_extras.py, FilesExtras.qml; test_files_extras.py: "test_tags_set_list_find_and_in_listings"; files-extras.yml pictures). Left: ratings and comments (S) — later, the owner's call
- [x] Group by — FileManager.qml (type, date, size, tag); files-extras.yml "files-x1-tags-grouped" (needs testing on a device)
- [x] Git status badges — hearth_files_extras.py git_status, FilesMarks.qml; test_files_extras.py: "test_git_status_badges"; files-extras.yml "files-x4-git" (needs testing on a device)
- [x] Act as administrator — FilesExtras.qml, os/system_files/usr/libexec/nimbus/hearth-files-admin (never /boot, /efi, the image, fstab, kernel/boot settings), its own polkit action fun.hearthos.files-admin (auth_admin_keep for it alone), every name made in a copy or move checked; test_files_extras.py: "test_never_what_starts_the_pc"; files-extras.yml "files-x6-admin" (needs testing on a device)
- [x] Checksums, permissions and owner tabs in Properties — FileProperties.qml (SHA-512, compare a pasted one, group, whole folder, owner as administrator); test_files_extras.py: "test_chmod_tree_and_groups"; files-extras.yml "files-x7-checksums", "files-x8-permissions" (needs testing on a device)
- [x] Mount an ISO, MTP phones, PTP cameras — hearth_files_extras.py iso_mount (udisks, read only), portable (gvfs); test_files_extras.py: "test_iso_mount_and_close_through_udisks", "test_phones_and_cameras_from_gio"; files-extras.yml "files-x5-iso-menu", "files-x1-tags-grouped" (needs testing on a device)
- [x] Service menus — hearth_files_extras.py service_menus / service_run (kio/servicemenus, kservices5/ServiceMenus); test_files_extras.py: "test_service_menus_kde_style"; files-extras.yml "files-x2-menu" (needs testing on a device)

### File search and indexing

- [x] File indexing (names and contents) with folders to skip and battery limits — hearth-search, SetSearch.qml (needs testing on a device)
- [x] Recent files tracking and "forget recent files" — SetRecentFiles.qml, hearth_files_extras.py recent_*; test_files_extras.py: "test_recent_files_privacy"; files-extras.yml "files-x10-recent-privacy" (needs testing on a device)

### Passwords, wallet, vaults

- [x] Password store (Secret Service, unlocked at login, a manager page) — SetPasswords.qml (needs testing on a device)
- [ ] SSH and GPG passphrases asked in Hearth's look (S)
- [ ] Vaults: encrypted folders in the tray, auto-lock, offline-only — Hearth has: Briefcase (`hearth-briefcase`) (M)
- [ ] Password prompts for admin actions (Polkit agent) in Hearth's look — partly: Mantel's own agent (PolkitPrompt.qml, polkit-actions.js, hearth-polkit-fallback with the classic agent as fallback); test_hearth_polkit_mantel.py, capture-polkit-test.yml pictures. Left: fingerprint (with fprintd below) (S)

### Activities

- [ ] Activities — partly: Activities.qml, ActivitySwitcher.qml, ActivityChip.qml, activities.js (windows, wallpaper, widgets and pinned apps per activity, switcher in the bar, Quick Access and the controller, Meta+Ctrl+Q / Meta+Ctrl+Shift+Q, Move to activity in the window menus); test_desktop_widgets.py, widgets-test.yml. Left: recent files and privacy per activity; Super+Q is still Close, not the switcher (shortcuts.json "close" Meta+Q); the ROADMAP line is unticked (M)

### App store and updates

- [x] App store (Flathub, Debian packages) — Bazaar + `SetApps.qml`, `DebInstaller.qml`
- [x] System updates with the previous version kept — `SetUpdates.qml`, bootc
- [ ] Discover's extras (update notifications, reviews, permission changes, verified label, links, fonts, drivers) — Hearth's own App store is on feature/search-store (merged) (L)
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
- [x] Touchscreen calibration — SetKwinTablet.qml, KwinCalibrate.qml (needs testing on a device)

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

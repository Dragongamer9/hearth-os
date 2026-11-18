# Firehold on Plasma (a test): mockups 1–6 built on KDE Plasma and KWin

Mockups: firehold/1-couch-home.png, firehold/2-desk.png, firehold/3-in-game.png, firehold/4-switcher.png, firehold/5-library.png, firehold/6-settings-couch-desk.png
Release: next

The same approved mockups as firehold.md, built the second way the owner asked for (Nov 18, 2026): the Couch as a
Plasma shell package (hosts/linux/desktop/firehold-plasma), the Desk as Plasma's own desktop in Firehold's layout, on
KDE's own KWin through its public interfaces (docs/FIREHOLD.md "Firehold on Plasma"). A prototype on branch
feature/firehold-plasma: no roadmap item is ticked for it. Proofs point at its files; open parts say why.

## 1. Couch Home (1-couch-home.png)

- [x] Top bar: the Firehold logo tile (flame) at the left — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Logo.qml: "strokeColor: \"#2a1206\""
- [x] Top bar tabs: Home (selected, underlined), Library, Apps, Store — hosts/linux/desktop/firehold-plasma/couch/contents/ui/TopBar.qml: "[[\"home\", \"Home\"], [\"library\", \"Library\"], [\"apps\", \"Apps\"], [\"store\", \"Store\"]]"
- [x] Top bar: the posture pill with its dot: "Couch · switches to Desk when you touch the mouse" — hosts/linux/desktop/firehold-plasma/couch/contents/ui/TopBar.qml: "Couch · switches to Desk when you touch the mouse"
- [x] Top bar: Wi-Fi status — hosts/linux/desktop/firehold-plasma/hearth-firehold-plasma: "out['net'] = 'Wi-Fi' if t == 'wifi' else 'Wired'"
- [x] Top bar: battery percentage (78%) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/TopBar.qml: "text: bar.couch.sys.battery + \"%\""
- [x] Top bar: the clock (10:22 PM) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/TopBar.qml: "Qt.formatTime(bar.couch.now, \"h:mm AP\")"
- [x] Top bar: the account avatar (initial, round) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/TopBar.qml: "text: bar.couch.sys.initial"
- [x] Hero card: the last game played, in a wide rounded card with its colours as a gradient — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "readonly property var game: couch.continueGame"
- [x] Hero: "CONTINUE · 41 MIN AGO" line (accent, capitals, how long ago) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "\"CONTINUE · \" + F.ago("
- [x] Hero: the game's name, large (Ember Trails) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "font.pixelSize: 64 * home.k"
- [x] Hero line: the chapter (when the game's entry has one) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "home.game.chapter || \"\""
- [x] Hero line: "Saved on this PC and in the cloud" — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "\"Saved on this PC and in the cloud\""
- [x] Hero line: "runs natively, no Game Mode" — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "\"runs natively, no Game Mode\""
- [x] Hero: Resume button (accent, selected) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "home.running ? \"Resume\" : \"Play\""
- [x] Hero: "Open in a window" button — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "\"Open in a window\""
- [x] Hero: the game's art on the right (the dashed "Game art" area) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "text: \"Game art\""
- [x] Recent row: title "Recent" — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "text: \"Recent\""
- [x] Recent row: seven tiles with the game's art, its store on the tile and its name under it — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "F.recent(couch.library, 7,"
- [x] "Open on this PC" row with "· the same windows you see on the Desk" — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "\" · the same windows you see on the Desk\""
- [x] Open windows as cards: icon, name and what's in it (Plasma's own task model) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "couch.openList.filter(function (o) { return o.kind === \"window\"; })"
- [x] Hint bar: A Play — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "key: \"A\"; label: \"Play\""
- [x] Hint bar: X Options — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "key: \"X\"; label: \"Options\""
- [x] Hint bar: Y Search — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "key: \"Y\"; label: \"Search\""
- [x] Hint bar: ≡ Guide: Quick Access — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "label: \"Guide: Quick Access\""
- [x] Hint bar: LB RB Tabs — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "key: \"LB RB\"; label: \"Tabs\""
- [x] The warm dark background (gradient) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/main.qml: "fh.tab === \"home\" && fh.page === \"tabs\" ? \"#2a1409\""

## 2. Desk (2-desk.png)

- [x] Top bar: app tile at the left with the focused app's name — hosts/linux/desktop/firehold-plasma/app/contents/ui/main.qml: "text: root.appName || \"Firehold\""
- [x] Top bar: the focused app's menus (File, Edit, View): KDE's global menu widget — hosts/linux/desktop/firehold-plasma/desk/layout.js: "top.addWidget(\"org.kde.plasma.appmenu\");"
- [x] Top bar: date and clock in the middle (Mon Nov 16 · 10:22 PM) — hosts/linux/desktop/firehold-plasma/desk/layout.js: "clock.writeConfig(\"customDateFormat\", \"ddd MMM d\");"
- [x] Top bar: the posture pill "Desk" with its blue dot — hosts/linux/desktop/firehold-plasma/posture/contents/ui/main.qml: "Text { text: \"Desk\""
- [x] Top bar: Wi-Fi · battery (Wi-Fi · 78%) — hosts/linux/desktop/firehold-plasma/posture/contents/ui/main.qml: "text: [root.net, root.batteryText].filter(Boolean).join(\" · \")"
- [ ] A window with its title (Hearth Files — Pictures) and three round window buttons, the close one in the accent colour
- [ ] Hearth Files' picture grid inside it (two rows of five)
- [ ] The game in a window with the accent border (the same game resumed on the Couch)
- [ ] The game window's title line: name · paused · 60 fps limit · VRR
- [ ] The game window's hint chip "Full screen: F11 or ≡"
- [x] The game window keeps the game's picture (games go back in their windows on the Desk) — hosts/linux/desktop/firehold-plasma/kwin/contents/code/main.js: "} else if ((made[key(w)] || isGame(w)) && w.fullScreen && games === \"follow\") {"
- [x] Notification card "Switched to Desk": "You moved the mouse. Everything stayed open. Press the Guide button to go back to the Couch." — hosts/linux/desktop/firehold-plasma/hearth-firehold-plasma: "' Everything stayed open. Press the Guide button to go back to the Couch.'"
- [x] Dock: Home (H, accent) — hosts/linux/desktop/firehold-plasma/desk/layout.js: "\"applications:org.hearthos.firehold.home.desktop\""
- [x] Dock: Hearth Files (F) with its running dot — hosts/linux/desktop/firehold-plasma/desk/layout.js: "\"applications:hearth-files.desktop\""
- [x] Dock: Firefox (W) with its running dot — hosts/linux/desktop/firehold-plasma/desk/layout.js: "\"preferred://browser\""
- [x] Dock: the game (▶) with its running dot (running apps follow the pinned ones) — hosts/linux/desktop/firehold-plasma/desk/layout.js: "var tasks = dock.addWidget(\"org.kde.plasma.icontasks\");"
- [x] Dock: Discord (D) with its running dot (running apps follow the pinned ones) — hosts/linux/desktop/firehold-plasma/desk/layout.js: "var tasks = dock.addWidget(\"org.kde.plasma.icontasks\");"
- [x] Dock: Steam (S) with its running dot — hosts/linux/desktop/firehold-plasma/desk/layout.js: "\"applications:steam.desktop\""
- [x] Dock: Library (L) — hosts/linux/desktop/firehold-plasma/desk/layout.js: "\"applications:org.hearthos.firehold.library.desktop\""
- [x] Dock: Settings (⚙) — hosts/linux/desktop/firehold-plasma/desk/layout.js: "\"applications:systemsettings.desktop\""
- [ ] Dock glyph tiles (H, F, W, ▶, D, S, L, ⚙ letters on coloured squares) instead of app icons

## 3. A game with the Guide (3-in-game.png)

- [ ] The game full screen, drawn straight to the display (no gamescope)
- [ ] Performance overlay at the top left: fps · frame time · GPU % · temperature · watts (KWin's Show FPS effect only shows fps)
- [x] Guide panel on the right, over the game — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GuideLayer.qml: "LayerShell.Window.layer: LayerShell.Window.LayerOverlay"
- [x] Guide: the game's art, name (Ember Trails) and "Playing 1 h 12 min · native, no Game Mode" — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Guide.qml: "\" · native, no Game Mode\""
- [x] Guide tile: Screenshot · Saved to Pictures (selected) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Guide.qml: "[\"Screenshot\", \"Saved to Pictures\"]"
- [ ] Guide tile: Record · Last 30 s too
- [ ] Guide tile: Friends · 2 online
- [x] Guide tile: Keyboard · On-screen — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Guide.qml: "[\"Keyboard\", \"On-screen\"]"
- [x] Guide tile: Volume · 64% — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Guide.qml: "[\"Volume\", sys.volume >= 0 ? sys.volume + \"%\""
- [x] Guide tile: Brightness · 80% — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Guide.qml: "[\"Brightness\", sys.brightness >= 0 ? sys.brightness + \"%\""
- [ ] Guide row: Frame limit · ‹ 60 › (KWin has no frame limit; the row says "Not on Plasma")
- [ ] Guide row: Resolution scaling · ‹ Quality · 1440p › (KWin doesn't upscale games; the row says "Not on Plasma")
- [x] Guide row: VRR / FreeSync · Smooth when it dips below 60 · ‹ On › (green): KDE's own adaptive sync setting — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Guide.qml: "[\"VRR / FreeSync\", \"Smooth when it dips below 60\", sys.vrr || \"—\", sys.vrr === \"On\"]"
- [x] Guide row: Allow tearing · Lowest delay for fast games · ‹ Off ›: KWin's own AllowTearing — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Guide.qml: "[\"Allow tearing\", \"Lowest delay for fast games\""
- [x] Guide row: Power · Handhelds and laptops · ‹ Balanced · 18 W › — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Guide.qml: "[\"Power\", \"Handhelds and laptops\", powerName + (sys.watts >= 0 ? \" · \" + sys.watts + \" W\" : \"\")"
- [x] Guide row: Performance overlay · Top-left corner · ‹ Basic › — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Guide.qml: "[\"Performance overlay\", \"Top-left corner\""
- [x] Guide button: Put it in a window on the Desk — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Guide.qml: "[\"Put it in a window on the Desk\", false]"
- [x] Guide button: Home (the game keeps running) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Guide.qml: "[\"Home (the game keeps running)\", false]"
- [x] Guide button: Quit game (red outline) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Guide.qml: "[\"Quit game\", true]"

## 4. Everything open (4-switcher.png)

- [x] Title "Open now" — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Switcher.qml: "text: \"Open now\""
- [x] "Games and windows are one list. Pick one to jump to it, in Couch or Desk." — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Switcher.qml: "Games and windows are one list. Pick one to jump to it, in Couch or Desk."
- [x] A grid of live pictures of every game and window (three a row): KWin's screencast through PipeWire, as Plasma's task manager does — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Thumb.qml: "TaskManager.ScreencastingRequest {"
- [x] Each card's tag: GAME or WINDOW — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Switcher.qml: "card.game ? \"GAME\" : \"WINDOW\""
- [x] The selected card with the accent border and glow — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Switcher.qml: "Qt.rgba(1, 138 / 255, 61 / 255, 0.22)"
- [x] Each card's name and what it's doing (firehold.js's openItems, as Mantel's Firehold) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/main.qml: "F.openItems(wins, library, function (w) { return w.appName; })"
- [x] Hint bar: A Go to it — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Switcher.qml: "label: \"Go to it\""
- [x] Hint bar: X Close — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Switcher.qml: "label: \"Close\""
- [x] Hint bar: Y Window ⇄ full screen — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Switcher.qml: "label: \"Window ⇄ full screen\""
- [x] Hint bar: B Back — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Switcher.qml: "label: \"Back\""

## 5. Library (5-library.png)

- [x] Top bar: logo tile and tabs Home, Library (selected, underlined), Apps, Store — hosts/linux/desktop/firehold-plasma/couch/contents/ui/TopBar.qml: "visible: bar.couch.tab !== \"library\""
- [x] Search box: "Search 412 games" (the real count) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Library.qml: "\"Search \" + (lib.couch.counts.all || lib.couch.library.length) + \" games\""
- [x] Filter chips with counts: All, Installed, Steam, GOG, Epic, Windows apps, Emulators, Your games — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Library.qml: "readonly property var chips: F.chips(couch.counts)"
- [x] Tall game tiles with their art, six a row — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Library.qml: "readonly property int cols: 6"
- [x] Each tile's store tag (Steam, GOG, Epic, Emulator, Windows app, Your game) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Library.qml: "text: cell.modelData.store || \"\""
- [x] Each tile's state: Playing, Installed, Update ready, Not installed, Dev build, Cloud saves — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Library.qml: "text: F.status(cell.modelData, lib.couch.playing)"
- [x] Each game's name under its tile — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Library.qml: "text: cell.modelData.title"
- [x] The selected tile with the accent border — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Library.qml: "border.color: lib.couch.pal.accent"
- [x] Hint bar: A Play — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Library.qml: "text: \"A Play\""
- [x] Hint bar: X Game options — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Library.qml: "text: \"X Game options\""
- [x] Hint bar: Y Search — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Library.qml: "text: \"Y Search\""
- [x] Hint bar: LT RT Filters — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Library.qml: "text: \"LT RT Filters\""

## 6. Settings › Couch and Desk (6-settings-couch-desk.png)

- [x] Settings window with the list on the left: System, Couch and Desk (selected), Display, Gaming, Controllers, Network, Sound, Accounts, Updates (the others open KDE's own pages) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/CouchSettings.qml: "[\"system\", \"System\"], [\"couch\", \"Couch and Desk\"], [\"kcm_kscreen\", \"Display\"]"
- [x] Title "Couch and Desk" — hosts/linux/desktop/firehold-plasma/couch/contents/ui/CouchSettings.qml: "st.section === \"system\" ? \"System\" : \"Couch and Desk\""
- [x] "One desktop, two ways to sit. Couch is for the controller and the TV; Desk is for the mouse and keyboard. Switching never closes anything." — hosts/linux/desktop/firehold-plasma/couch/contents/ui/CouchSettings.qml: "One desktop, two ways to sit. Couch is for the controller and the TV; Desk is for the mouse and keyboard. Switching never closes anything."
- [x] Posture card: Automatic · Controller → Couch, mouse or keyboard → Desk (selected, half and half picture) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/CouchSettings.qml: "[\"auto\", \"Automatic\", \"Controller → Couch, mouse or keyboard → Desk\"]"
- [x] Posture card: Always Couch · A console: tiles, full screen, Guide button — hosts/linux/desktop/firehold-plasma/couch/contents/ui/CouchSettings.qml: "[\"couch\", \"Always Couch\", \"A console: tiles, full screen, Guide button\"]"
- [x] Posture card: Always Desk · A PC: windows, bar and dock — hosts/linux/desktop/firehold-plasma/couch/contents/ui/CouchSettings.qml: "[\"desk\", \"Always Desk\", \"A PC: windows, bar and dock\"]"
- [ ] Row: Per screen · The TV stays Couch while the monitor is Desk · TV: Couch · Monitor: Desk (one Plasma shell runs on every screen: the row says so)
- [x] Row: Start in · What you see when the PC turns on · Couch — hosts/linux/desktop/firehold-plasma/couch/contents/ui/CouchSettings.qml: "[\"Start in\", \"What you see when the PC turns on\", startNames[couch.startIn]]"
- [x] Row: Guide button · Press once: Quick Access. Twice: everything open · Quick Access — hosts/linux/desktop/firehold-plasma/couch/contents/ui/CouchSettings.qml: "[\"Guide button\", \"Press once: Quick Access. Twice: everything open\""
- [x] Row: Games · Open full screen on the Couch, in a window on the Desk · Follow the posture — hosts/linux/desktop/firehold-plasma/couch/contents/ui/CouchSettings.qml: "[\"Games\", \"Open full screen on the Couch, in a window on the Desk\", gameNames[couch.gamesOpen]]"
- [x] Row: Big Picture · Steam's own TV mode isn't needed; Steam is an app like the others · Off — hosts/linux/desktop/firehold-plasma/couch/contents/ui/CouchSettings.qml: "[\"Big Picture\", \"Steam's own TV mode isn't needed; Steam is an app like the others\", couch.bigPicture ? \"On\" : \"Off\"]"

## Still open on Plasma, and why

- Desk windows in Firehold's own look (left-aligned title, three round buttons, the game's accent border, its title line and
  the "Full screen: F11 or ≡" chip): KWin draws window frames through a decoration theme (Breeze, Aurorae); a Firehold
  decoration theme would do it without patching KWin, not built yet. Hearth Files' own grid is the app's own.
- The dock's letter tiles: Plasma's icon task manager shows app icons.
- The game drawn straight to the display, Frame limit, Resolution scaling, the full performance overlay: KWin has no
  frame limiter or game upscaler; direct scanout of a full-screen game is KWin's own choice. The overlay row turns on
  KWin's Show FPS effect.
- Record "Last 30 s too" (no replay buffer: Spectacle records the screen), Friends "2 online" (needs Steam's friends data).
- Per screen: one plasmashell shell package runs on every screen, so the posture is the same on all of them.

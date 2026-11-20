# Firehold desktop, Couch and Desk (mockups 1–6 approved Nov 16, 2026)

Mockups: firehold/1-couch-home.png, firehold/2-desk.png, firehold/3-in-game.png, firehold/4-switcher.png, firehold/5-library.png, firehold/6-settings-couch-desk.png
Release: 2.0

Where the pictures say Hearth (Hearth Files aside, an app's own name) the desktop is called Firehold. The sample games
(Ember Trails, Skyforge, Night Harbor...) stand for the person's real games and their real art.

## 1. Couch Home (1-couch-home.png)

- [x] Top bar: the Firehold logo tile (flame) at the left — hosts/linux/desktop/shell/FireholdCouch.qml: "c.strokeStyle = \"#2a1206\""
- [x] Top bar tabs: Home (selected, underlined), Library, Apps, Store — hosts/linux/desktop/shell/FireholdCouch.qml: "[[\"home\", \"Home\"], [\"library\", \"Library\"], [\"apps\", \"Apps\"], [\"store\", \"Store\"]]"
- [x] Top bar: the posture pill with its dot: "Couch · switches to Desk when you touch the mouse" — hosts/linux/desktop/shell/FireholdCouch.qml: "Couch · switches to Desk when you touch the mouse"
- [x] Top bar: Wi-Fi status — hosts/linux/desktop/shell/FireholdCouch.qml: "NetBtWatch.netKind === \"wifi\" ? \"Wi-Fi\""
- [x] Top bar: battery percentage (78%) — hosts/linux/desktop/shell/FireholdCouch.qml: "Math.round(batt.percentage * 100) + \"%\""
- [x] Top bar: the clock (10:22 PM) — hosts/linux/desktop/shell/FireholdCouch.qml: "\"h:mm AP\""
- [x] Top bar: the account avatar (initial, round) — hosts/linux/desktop/shell/FireholdCouch.qml: "text: cw.initial"
- [x] Hero card: the last game played, in a wide rounded card with its colours as a gradient — hosts/linux/desktop/shell/FireholdCouch.qml: "readonly property var game: cw.fh.continueGame"
- [x] Hero: "CONTINUE · 41 MIN AGO" line (accent, capitals, how long ago) — hosts/linux/desktop/shell/FireholdCouch.qml: "\"CONTINUE · \" + F.ago(hero.game.lastPlayed"
- [x] Hero: the game's name, large (Ember Trails) — hosts/linux/desktop/shell/FireholdCouch.qml: "font.pixelSize: 64 * cw.k"
- [ ] Hero line: the chapter
- [x] Hero line: "Saved on this PC and in the cloud" (Steam Cloud's own folder for the game has its saves) — hosts/linux/desktop/shell/FireholdCouch.qml: "hero.game.cloudSaves ? \"Saved on this PC and in the cloud\""
- [x] Hero line: "runs natively, no Game Mode" — hosts/linux/desktop/shell/FireholdCouch.qml: "\"runs natively, no Game Mode\"].filter(Boolean).join(\" · \")"
- [x] Hero: Resume button (accent, selected) — hosts/linux/desktop/shell/FireholdCouch.qml: "? \"Resume\" : \"Play\""
- [x] Hero: "Open in a window" button — hosts/linux/desktop/shell/FireholdCouch.qml: "text: \"Open in a window\""
- [x] Hero: the game's art on the right (the dashed "Game art" area) — hosts/linux/desktop/shell/FireholdCouch.qml: "text: \"Game art\""
- [x] Recent row: title "Recent" — hosts/linux/desktop/shell/FireholdCouch.qml: "text: \"Recent\""
- [x] Recent row: seven tiles with the game's art, its store on the tile (Steam, GOG, Emulator, Epic, Steam, Windows app, Your game) and its name under it (Skyforge, Night Harbor, Pixel Rally, Hollow Deep, Tiny Farm, Old Office, Quad Mech) — hosts/linux/desktop/shell/FireholdCouch.qml: "F.recent(fh.library, 7,"
- [x] "Open on this PC" row with "· the same windows you see on the Desk" — hosts/linux/desktop/shell/FireholdCouch.qml: "\" · the same windows you see on the Desk\""
- [x] Open windows as cards: icon, name and what's in it (Hearth Files · Pictures › Trips, Firefox · 3 tabs, Discord · 2 new messages, Steam · Downloading 1 update) — hosts/linux/desktop/shell/FireholdCouch.qml: "fh.openList.filter(o => o.kind === \"window\")"
- [x] Hint bar: A Play — hosts/linux/desktop/shell/FireholdCouch.qml: "Hint { key: \"A\"; label: \"Play\" }"
- [x] Hint bar: X Options — hosts/linux/desktop/shell/FireholdCouch.qml: "Hint { key: \"X\"; label: \"Options\" }"
- [x] Hint bar: Y Search — hosts/linux/desktop/shell/FireholdCouch.qml: "Hint { key: \"Y\"; label: \"Search\" }"
- [x] Hint bar: ≡ Guide: Quick Access — hosts/linux/desktop/shell/FireholdCouch.qml: "\"Guide: Quick Access\""
- [x] Hint bar: LB RB Tabs — hosts/linux/desktop/shell/FireholdCouch.qml: "Hint { key: \"LB RB\"; label: \"Tabs\" }"
- [x] The warm dark background (gradient) — hosts/linux/desktop/shell/FireholdCouch.qml: "c0: cw.page === \"switcher\" ? \"#2c160a\" : cw.tab === \"home\" ? \"#4a2410\""

## 2. Desk (2-desk.png)

- [x] Top bar: app tile at the left with the focused app's name (Hearth Files) — hosts/linux/desktop/shell/FireholdDesk.qml: "text: dk.frontName"
- [x] Top bar: the focused app's menus (File, Edit, View) — hosts/linux/desktop/shell/FireholdDesk.qml: "model: [\"File\", \"Edit\", \"View\"]"
- [x] Top bar: date and clock in the middle (Mon Nov 16 · 10:22 PM) — hosts/linux/desktop/shell/FireholdDesk.qml: "Qt.formatDate(clock.date, \"ddd MMM d\") + \" · \""
- [x] Top bar: the posture pill "Desk" with its blue dot — hosts/linux/desktop/shell/FireholdDesk.qml: "text: pill.lm ? pill.lm.modeName(pill.lm.me.mode) : \"Desk\""
- [x] Top bar: Wi-Fi · battery (Wi-Fi · 78%) — hosts/linux/desktop/shell/FireholdDesk.qml: ".filter(Boolean).join(\" · \")"
- [x] A window with its title (Hearth Files — Pictures) and three round window buttons, the close one in the accent colour — hosts/linux/desktop/shell/FireholdFrames.qml: "color: modelData === \"close\" ? F.C.accent : F.C.card2"
- [x] Hearth Files' picture grid inside it (two rows of five): Hearth Files itself, a window like any other on the Desk with Firehold's title bar — hosts/linux/desktop/shell/firehold.js: "if (mf) return { name: \"Hearth Files\", sub: mf[1] };"
- [x] The game in a window with the accent border (the same game resumed on the Couch) — hosts/linux/desktop/shell/FireholdFrames.qml: "color: \"transparent\"; border.width: 2; border.color: F.C.accent"
- [x] The game window's title line: name · paused · 60 fps limit · VRR — hosts/linux/desktop/shell/FireholdFrames.qml: "\" · \" + F.gameLine(\"\", one.win.minimized, fr.fh.frameLimit, fr.vrr)"
- [x] The game window's hint chip "Full screen: F11 or ≡" — hosts/linux/desktop/shell/FireholdFrames.qml: "\"Full screen: F11 or ≡\""
- [x] The game window keeps the game's picture ("The same game you resumed on the Couch, now in a window") — hosts/linux/desktop/shell/FireholdFrames.qml: "mask: Region { id: hits }"
- [x] On Firehold's own compositor the title bars are drawn by firehold-comp itself in this look (left-aligned "App — place", three 14 px round buttons 10 apart, close in the accent, rounded top corners), named by Mantel — hosts/firehold-comp/src/shell/ssd.rs: "fn draw_firehold_bar("
- [x] On Firehold's own compositor: the game window's accent border all round, its title line and the chip "Full screen: F11 or ≡" (a click on it makes it full screen) — hosts/firehold-comp/src/shell/ssd.rs: "Some(Button::Fullscreen) => state.handle.insert_idle(move |data| data.set_fullscreen(&w, true, None))"
- [x] Mantel names each window for the compositor's bars ("Hearth Files — Pictures", a game's name and line) — hosts/linux/desktop/shell/FireholdFramesComp.qml: "op: \"frames\", style: \"firehold\""
- [x] Notification card "Switched to Desk": "You moved the mouse. Everything stayed open. Press the Guide button to go back to the Couch." — hosts/linux/desktop/shell/Firehold.qml: "Everything stayed open. Press the Guide button to go back to the Couch."
- [x] Dock: Home (H, accent) — hosts/linux/desktop/shell/FireholdDesk.qml: "glyph: \"H\"; tint: F.C.accent"
- [x] Dock: Hearth Files (F) with its running dot — hosts/linux/desktop/shell/FireholdDesk.qml: "readonly property var running"
- [x] Dock: Firefox (W) with its running dot — hosts/linux/desktop/shell/FireholdDesk.qml: "readonly property var running"
- [x] Dock: the game (▶) with its running dot — hosts/linux/desktop/shell/FireholdDesk.qml: "glyph: modelData.game ? \"▶\""
- [x] Dock: Discord (D) with its running dot — hosts/linux/desktop/shell/FireholdDesk.qml: "readonly property var running"
- [x] Dock: Steam (S) with its running dot — hosts/linux/desktop/shell/FireholdDesk.qml: "readonly property var running"
- [x] Dock: Library (L) — hosts/linux/desktop/shell/FireholdDesk.qml: "glyph: \"L\""
- [x] Dock: Settings (⚙) — hosts/linux/desktop/shell/FireholdDesk.qml: "glyph: \"⚙\""

## 3. A game with the Guide (3-in-game.png)

- [ ] The game full screen, drawn straight to the display (no gamescope)
- [x] Performance overlay at the top left: fps · frame time · GPU % · temperature · watts — hosts/linux/desktop/shell/FireholdPerf.qml: "p.push(Math.round(w) + \" W\")"
- [x] Guide panel on the right, over the game — hosts/linux/desktop/shell/FireholdGuide.qml: "WlrLayershell.namespace: \"hearth-firehold-guide\""
- [x] Guide: the game's art, name (Ember Trails) and "Playing 1 h 12 min · native, no Game Mode" — hosts/linux/desktop/shell/FireholdGuide.qml: "\" · native, no Game Mode\""
- [x] Guide tile: Screenshot · Saved to Pictures (selected) — hosts/linux/desktop/shell/FireholdGuide.qml: "[\"Screenshot\", \"Saved to Pictures\"]"
- [ ] Guide tile: Record · Last 30 s too
- [ ] Guide tile: Friends · 2 online
- [x] Guide tile: Keyboard · On-screen — hosts/linux/desktop/shell/FireholdGuide.qml: "[\"Keyboard\", \"On-screen\"]"
- [x] Guide tile: Volume · 64% — hosts/linux/desktop/shell/FireholdGuide.qml: "[\"Volume\", vol >= 0 ? vol + \"%\""
- [x] Guide tile: Brightness · 80% — hosts/linux/desktop/shell/FireholdGuide.qml: "[\"Brightness\", brightness >= 0 ? brightness + \"%\""
- [x] Guide row: Frame limit · Steadier, cooler, longer battery · ‹ 60 › (live from firehold-comp: firehold.yml pictures firehold-13-guide, firehold-c3-guide show ‹ 60 ›) — hosts/linux/desktop/shell/FireholdGuide.qml: "[\"Frame limit\", \"Steadier, cooler, longer battery\", comp ? fh.frameLimit : \"Not yet\""
- [x] Guide row: Resolution scaling · Firehold upscales the game itself · ‹ Quality · 1440p › (live: ‹ Quality · 720p › on the tests' 1080p screen, 1440p on a 4K one; firehold-13-guide, firehold-c3-guide) — hosts/linux/desktop/shell/FireholdGuide.qml: "[\"Resolution scaling\", \"Firehold upscales the game itself\", comp ? scaling : \"Not yet\""
- [x] Guide row: VRR / FreeSync · Smooth when it dips below 60 · ‹ On › (green) — hosts/linux/desktop/shell/FireholdGuide.qml: "[\"VRR / FreeSync\", \"Smooth when it dips below 60\""
- [x] Guide row: Allow tearing · Lowest delay for fast games · ‹ Off › — hosts/linux/desktop/shell/FireholdGuide.qml: "[\"Allow tearing\", \"Lowest delay for fast games\""
- [x] Guide row: Power · Handhelds and laptops · ‹ Balanced · 18 W › — hosts/linux/desktop/shell/FireholdGuide.qml: "[\"Power\", \"Handhelds and laptops\""
- [x] Guide row: Performance overlay · Top-left corner · ‹ Basic › — hosts/linux/desktop/shell/FireholdGuide.qml: "[\"Performance overlay\", \"Top-left corner\""
- [x] Guide button: Put it in a window on the Desk — hosts/linux/desktop/shell/FireholdGuide.qml: "\"Put it in a window on the Desk\""
- [x] Guide button: Home (the game keeps running) — hosts/linux/desktop/shell/FireholdGuide.qml: "\"Home (the game keeps running)\""
- [x] Guide button: Quit game (red outline) — hosts/linux/desktop/shell/FireholdGuide.qml: "[\"Quit game\", true]"

## 4. Everything open (4-switcher.png)

- [x] Title "Open now" — hosts/linux/desktop/shell/FireholdCouch.qml: "text: \"Open now\""
- [x] "Games and windows are one list. Pick one to jump to it, in Couch or Desk." — hosts/linux/desktop/shell/FireholdCouch.qml: "Games and windows are one list. Pick one to jump to it, in Couch or Desk."
- [x] A grid of live pictures of every game and window (three a row) — hosts/linux/desktop/shell/FireholdCouch.qml: "source: cw.fh.pictureOf(sw.modelData.win); live: true"
- [x] Each card's tag: GAME or WINDOW — hosts/linux/desktop/shell/FireholdCouch.qml: "sw.game ? \"GAME\" : \"WINDOW\""
- [x] The selected card with the accent border and glow — hosts/linux/desktop/shell/FireholdCouch.qml: "Qt.rgba(1, 138 / 255, 61 / 255, 0.22)"
- [x] Each card's name and what it's doing (Ember Trails · game · paused, Hearth Files · Pictures › Trips, Firefox · 3 tabs, Discord · 2 new messages, Pixel Rally · emulator · suspended, Steam · downloading) — hosts/linux/desktop/shell/firehold.js: "(src === \"emulator\" ? \"emulator\" : \"game\") + \" · \" + state"
- [x] Hint bar: A Go to it — hosts/linux/desktop/shell/FireholdCouch.qml: "label: \"Go to it\""
- [x] Hint bar: X Close — hosts/linux/desktop/shell/FireholdCouch.qml: "label: \"Close\""
- [x] Hint bar: Y Window ⇄ full screen — hosts/linux/desktop/shell/FireholdCouch.qml: "label: \"Window ⇄ full screen\""
- [x] Hint bar: B Back — hosts/linux/desktop/shell/FireholdCouch.qml: "label: \"Back\""

## 5. Library (5-library.png)

- [x] Top bar: logo tile and tabs Home, Library (selected, underlined), Apps, Store — hosts/linux/desktop/shell/FireholdCouch.qml: "visible: cw.tab !== \"library\""
- [x] Search box: "Search 412 games" (the real count) — hosts/linux/desktop/shell/FireholdCouch.qml: "\"Search \" + (cw.fh.counts.all || cw.fh.library.length) + \" games\""
- [x] Filter chips with counts: All 412 (selected), Installed 96, Steam 288, GOG 41, Epic 57, Windows apps 9, Emulators 14, Your games 3 — hosts/linux/desktop/shell/firehold.js: "[\"winapp\", \"Windows apps\"], [\"emu\", \"Emulators\"], [\"yours\", \"Your games\"]"
- [x] Tall game tiles with their art, six a row — hosts/linux/desktop/shell/FireholdCouch.qml: "readonly property int cols: 6"
- [x] Each tile's store tag (Steam, GOG, Epic, Emulator, Windows app, Your game) — hosts/linux/desktop/shell/FireholdCouch.qml: "text: lt.modelData.store"
- [x] Each tile's state: Playing, Installed, Update ready, Not installed, Dev build, Cloud saves — hosts/linux/desktop/shell/firehold.js: "return \"Cloud saves\";"
- [x] Each game's name under its tile — hosts/linux/desktop/shell/FireholdCouch.qml: "text: lt.modelData.title"
- [x] The selected tile with the accent border — hosts/linux/desktop/shell/FireholdCouch.qml: "border.color: lt.ring ? cw.pal.accent"
- [x] Hint bar: A Play — hosts/linux/desktop/shell/FireholdCouch.qml: "T { text: \"A Play\" }"
- [x] Hint bar: X Game options — hosts/linux/desktop/shell/FireholdCouch.qml: "T { text: \"X Game options\" }"
- [x] Hint bar: Y Search — hosts/linux/desktop/shell/FireholdCouch.qml: "T { text: \"Y Search\" }"
- [x] Hint bar: LT RT Filters — hosts/linux/desktop/shell/FireholdCouch.qml: "T { text: \"LT RT Filters\" }"

## 6. Settings › Couch and Desk (6-settings-couch-desk.png)

- [x] Settings window with the list on the left: System, Couch and Desk (selected), Display, Gaming, Controllers, Network, Sound, Accounts, Updates — hosts/linux/desktop/shell/FireholdSettings.qml: "[\"\", \"Couch and Desk\"]"
- [x] Title "Couch and Desk" — hosts/linux/desktop/shell/FireholdSettings.qml: "text: \"Couch and Desk\""
- [x] "One desktop, two ways to sit. Couch is for the controller and the TV; Desk is for the mouse and keyboard. Switching never closes anything." — hosts/linux/desktop/shell/FireholdSettings.qml: "One desktop, two ways to sit."
- [x] Posture card: Automatic · Controller → Couch, mouse or keyboard → Desk (selected, half and half picture) — hosts/linux/desktop/shell/FireholdSettings.qml: "[\"auto\", \"Automatic\", \"Controller → Couch, mouse or keyboard → Desk\"]"
- [x] Posture card: Always Couch · A console: tiles, full screen, Guide button — hosts/linux/desktop/shell/FireholdSettings.qml: "[\"couch\", \"Always Couch\", \"A console: tiles, full screen, Guide button\"]"
- [x] Posture card: Always Desk · A PC: windows, bar and dock — hosts/linux/desktop/shell/FireholdSettings.qml: "[\"desk\", \"Always Desk\", \"A PC: windows, bar and dock\"]"
- [x] Row: Per screen · The TV stays Couch while the monitor is Desk · TV: Couch · Monitor: Desk — hosts/linux/desktop/shell/FireholdSettings.qml: "\"The TV stays Couch while the monitor is Desk\""
- [x] Row: Start in · What you see when the PC turns on · Couch — hosts/linux/desktop/shell/FireholdSettings.qml: "\"What you see when the PC turns on\""
- [x] Row: Guide button · Press once: Quick Access. Twice: everything open · Quick Access — hosts/linux/desktop/shell/FireholdSettings.qml: "\"Press once: Quick Access. Twice: everything open\""
- [x] Row: Games · Open full screen on the Couch, in a window on the Desk · Follow the posture — hosts/linux/desktop/shell/FireholdSettings.qml: "\"Open full screen on the Couch, in a window on the Desk\""
- [x] Row: Big Picture · Steam's own TV mode isn't needed; Steam is an app like the others · Off — hosts/linux/desktop/shell/FireholdSettings.qml: "\"Steam's own TV mode isn't needed; Steam is an app like the others\""

## Still open on feature/firehold-shell, and why

- Hero line, chapter: no store or game data Hearth reads says which chapter you're on; the line shows it when a game's entry has `chapter`, and otherwise starts with where it's saved.
- The game full screen drawn straight to the display, Frame limit, Resolution scaling: need Firehold's compositor (feature/firehold-game, not on this branch yet). The Guide's rows already read `hearth-firehold-game status` and change them with `hearth-firehold-game step frameLimit|scaling up|down` (shown as its frameLimitLabel / scalingLabel, "60", "Quality · 1440p"); until the helper and the compositor are there they say "Not yet".
- Record "Last 30 s too": hearth-record has no replay buffer; firehold-comp's (feature/firehold-game) is coming.
- Friends "2 online": needs Steam's friends data (Discord gives Mantel no online count).

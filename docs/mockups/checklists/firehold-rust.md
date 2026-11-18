# Firehold (Rust, test): Couch and Desk all in Rust (mockups 1–6, approved Nov 16, 2026)

Mockups: firehold/1-couch-home.png, firehold/2-desk.png, firehold/3-in-game.png, firehold/4-switcher.png, firehold/5-library.png, firehold/6-settings-couch-desk.png
Release: next

The same six mockups as `firehold.md` (the QML Firehold), built a second way: `hosts/firehold-shell` (iced on
wlr-layer-shell) on `hosts/firehold-comp`, for the owner to try on the Anatase test (docs/FIREHOLD-RUST.md). A
prototype: no roadmap item is ticked for it. CI pictures: firehold-rust.yml's `firehold-r1` … `firehold-r6`.

## 1. Couch Home (1-couch-home.png)

- [x] Top bar: the Firehold logo tile (flame) at the left — hosts/firehold-shell/src/ui/couch.rs: "canvas(Flame(c_ink()))"
- [x] Top bar tabs: Home (selected, underlined), Library, Apps, Store — hosts/firehold-shell/src/ui/couch.rs: "[(\"home\", \"Home\"), (\"library\", \"Library\"), (\"apps\", \"Apps\"), (\"store\", \"Store\")]"
- [x] Top bar: the posture pill with its dot: "Couch · switches to Desk when you touch the mouse" — hosts/firehold-shell/src/ui/couch.rs: "Couch · switches to Desk when you touch the mouse"
- [x] Top bar: Wi-Fi status — hosts/firehold-shell/src/sys.rs: "return \"Wi-Fi\".into();"
- [x] Top bar: battery percentage (78%) — hosts/firehold-shell/src/sys.rs: "return format!(\"{}%\", c.round() as i64);"
- [x] Top bar: the clock (10:22 PM) — hosts/firehold-shell/src/sys.rs: "now.format(\"%-I:%M %p\")"
- [x] Top bar: the account avatar (initial, round) — hosts/firehold-shell/src/ui/couch.rs: "tb(s.st.initial.clone(), 20.0 * k, c_text())"
- [x] Hero card: the last game played, in a wide rounded card with its colours as a gradient — hosts/firehold-shell/src/ui/couch.rs: ".style(boxed(hero_bg(), 24.0 * k, None))"
- [x] Hero: "CONTINUE · 41 MIN AGO" line (accent, capitals, how long ago) — hosts/firehold-shell/src/ui/couch.rs: "format!(\"CONTINUE · {}\", ago.to_uppercase())"
- [x] Hero: the game's name, large (Ember Trails) — hosts/firehold-shell/src/ui/couch.rs: "tb(g.title.clone(), 64.0 * k, ART_TEXT)"
- [ ] Hero line: the chapter (shown when a game's entry has one; no store data Hearth reads says which chapter you're on)
- [x] Hero line: "Saved on this PC and in the cloud" (Steam Cloud's own folder for the game, or Hearth Saves) — hosts/firehold-shell/src/model.rs: "Saved on this PC and in the cloud"
- [x] Hero line: "runs natively, no Game Mode" — hosts/firehold-shell/src/model.rs: "runs natively, no Game Mode"
- [x] Hero: Resume button (accent, selected) — hosts/firehold-shell/src/ui/couch.rs: "{ \"Resume\" } else { \"Play\" }"
- [x] Hero: "Open in a window" button — hosts/firehold-shell/src/ui/couch.rs: "button_box(\"Open in a window\""
- [x] Hero: the game's art on the right (the dashed "Game art" area) — hosts/firehold-shell/src/ui/couch.rs: "t(\"Game art\", 18.0 * k"
- [x] Recent row: title "Recent" — hosts/firehold-shell/src/ui/couch.rs: "ts(\"Recent\", 22.0 * k, c_text())"
- [x] Recent row: seven tiles with the game's art, its store on the tile and its name under it — hosts/firehold-shell/src/ui/couch.rs: "model::recent(&self.lib, self.hero(), 7)"
- [x] "Open on this PC" row with "· the same windows you see on the Desk" — hosts/firehold-shell/src/ui/couch.rs: "\" · the same windows you see on the Desk\""
- [x] Open windows as cards: icon, name and what's in it — hosts/firehold-shell/src/ui/couch.rs: "words.push(t(w.sub.clone(), 14.0 * k, c_muted()))"
- [x] Hint bar: A Play — hosts/firehold-shell/src/ui/couch.rs: "hint(\"A\", if s.tab == \"home\" { \"Play\" }"
- [x] Hint bar: X Options — hosts/firehold-shell/src/ui/couch.rs: "hint(\"X\", \"Options\", k)"
- [x] Hint bar: Y Search — hosts/firehold-shell/src/ui/couch.rs: "hint(\"Y\", \"Search\", k)"
- [x] Hint bar: ≡ Guide: Quick Access — hosts/firehold-shell/src/ui/couch.rs: "\"Guide: Quick Access\""
- [x] Hint bar: LB RB Tabs — hosts/firehold-shell/src/ui/couch.rs: "hint(\"LB RB\", \"Tabs\", k)"
- [x] The warm dark background (gradient) — hosts/firehold-shell/src/ui/style.rs: "pub fn page_bg(glow: Color) -> Background"

## 2. Desk (2-desk.png)

- [x] Top bar: app tile at the left with the focused app's name (Hearth Files) — hosts/firehold-shell/src/ui/desk.rs: "tb(front, 15.0, c_text())"
- [x] Top bar: the focused app's menus shown (File, Edit, View) — hosts/firehold-shell/src/ui/desk.rs: "for m in [\"File\", \"Edit\", \"View\"]"
- [ ] Top bar: those menus open the app's own menus (they're labels: firehold-comp speaks no global-menu protocol yet)
- [x] Top bar: date and clock in the middle (Mon Nov 16 · 10:22 PM) — hosts/firehold-shell/src/ui/desk.rs: "format!(\"{} · {}\", st.date, st.clock)"
- [x] Top bar: the posture pill "Desk" with its blue dot — hosts/firehold-shell/src/ui/desk.rs: "ts(\"Desk\", 14.0, c_text())"
- [x] Top bar: Wi-Fi · battery (Wi-Fi · 78%) — hosts/firehold-shell/src/ui/desk.rs: "filter(|x| !x.is_empty()).collect::<Vec<_>>().join(\" · \")"
- [x] A window with its title (Hearth Files — Pictures) and three round window buttons, the close one in the accent colour, drawn by firehold-comp — hosts/firehold-comp/src/shell/ssd.rs: "fn draw_firehold_bar("
- [x] Each window named for firehold-comp's bars by the Rust shell ("Hearth Files — Pictures", a game's name and line) — hosts/firehold-shell/src/hub.rs: "json!({\"op\": \"frames\", \"style\": \"firehold\", \"names\": names})"
- [x] Hearth Files' picture grid inside it: Hearth Files itself, a window like any other on the Desk — hosts/firehold-shell/src/model.rs: "(\"org.hearth.files\", \"Hearth Files\")"
- [x] The game in a window with the accent border (the same game resumed on the Couch) — hosts/firehold-shell/src/hub.rs: "\"game\": w.game, \"line\": line"
- [x] The game window's title line: name · paused · 60 fps limit · VRR — hosts/firehold-shell/src/hub.rs: "parts.push(format!(\"{fl} fps limit\"));"
- [x] The game window's hint chip "Full screen: F11 or ≡" (a click makes it full screen) — hosts/firehold-comp/src/frames.rs: "pub const CHIP_TEXT: &str = \"Full screen: F11 or ≡\";"
- [x] The game window keeps the game's picture (the game itself, moved out of full screen, never restarted) — hosts/firehold-shell/src/hub.rs: "Windows the Couch made full screen go back to windows"
- [x] Notification card "Switched to Desk": "You moved the mouse. Everything stayed open. Press the Guide button to go back to the Couch." — hosts/firehold-shell/src/hub.rs: "Everything stayed open. Press the Guide button to go back to the Couch."
- [x] Dock: Home (H, accent) — hosts/firehold-shell/src/ui/desk.rs: "vec![(\"home\".to_string(), \"H\".to_string(), c_accent(), false)]"
- [x] Dock: Hearth Files (F) with its running dot — hosts/firehold-shell/src/model.rs: "if name == \"Hearth Files\" {"
- [x] Dock: Firefox (W) with its running dot — hosts/firehold-shell/src/model.rs: "if name == \"Firefox\" {"
- [x] Dock: the game (▶) with its running dot — hosts/firehold-shell/src/model.rs: "return \"▶\".into();"
- [x] Dock: Discord (D) with its running dot — hosts/firehold-shell/src/ui/desk.rs: "\"Discord\" => c(0x2c, 0x2a, 0x3e)"
- [x] Dock: Steam (S) with its running dot — hosts/firehold-shell/src/ui/desk.rs: "\"Steam\" => c(0x1f, 0x2a, 0x36)"
- [x] Dock: Library (L) — hosts/firehold-shell/src/ui/desk.rs: "v.push((\"library\".into(), \"L\".into()"
- [x] Dock: Settings (⚙) — hosts/firehold-shell/src/ui/desk.rs: "v.push((\"settings\".into(), \"⚙\".into()"

## 3. A game with the Guide (3-in-game.png)

- [x] The game full screen on Firehold itself (no gamescope; the Couch puts it there) — hosts/firehold-shell/src/hub.rs: "let full = if w.game { self.st.cfg.games == \"fullscreen\""
- [ ] The game drawn straight to the display (firehold-comp's direct scanout: only on a real screen, not provable in CI)
- [x] Performance overlay at the top left: GPU % · temperature · watts, and fps · frame time while firehold-comp paces the game — hosts/firehold-shell/src/hub.rs: "p.push(format!(\"{:.1} ms\", 1000.0 / f));"
- [x] Performance overlay: fps and frame time when no frame limit is set (firehold-comp counts every frame of the game in front; CI checks the line without a limit, firehold-r3b) — hosts/firehold-comp/src/game.rs: "\"fps\": (fps * 10.0).round() / 10.0,"
- [x] Guide panel on the right, over the game — hosts/firehold-shell/src/ui/guide.rs: "anchor: Anchor::Top | Anchor::Bottom | Anchor::Right,"
- [x] Guide: the game's art, name (Ember Trails) and "Playing 1 h 12 min · native, no Game Mode" — hosts/firehold-shell/src/ui/guide.rs: "format!(\"{} · native, no Game Mode\", g.playing)"
- [x] Guide tile: Screenshot · Saved to Pictures (selected) — hosts/firehold-shell/src/ui/guide.rs: "(\"Screenshot\", if g.screenshot.is_empty() { \"Saved to Pictures\".into() }"
- [x] Guide tile: Record · Last 30 s too (Hearth Recorder's instant replay; A saves the last 30 s, or starts and stops a recording when replay is off; needs testing on a device: CI's recorder is a stand-in) — hosts/firehold-shell/src/hub.rs: "(k, secs) if k == \"replay\" => format!(\"Last {secs} s too\"),"
- [ ] Guide tile: Friends · 2 online (the tile opens Steam's own friends list; how many are online is Steam's account data, which Hearth doesn't read)
- [x] Guide tile: Keyboard · On-screen — hosts/firehold-shell/src/ui/guide.rs: "(\"Keyboard\", \"On-screen\".into())"
- [x] Guide tile: Volume · 64% — hosts/firehold-shell/src/ui/guide.rs: "(\"Volume\", pct(g.volume))"
- [x] Guide tile: Brightness · 80% — hosts/firehold-shell/src/ui/guide.rs: "(\"Brightness\", pct(g.brightness))"
- [x] Guide row: Frame limit · Steadier, cooler, longer battery · ‹ 60 › (live from firehold-comp) — hosts/firehold-shell/src/ui/guide.rs: "(\"Frame limit\", \"Steadier, cooler, longer battery\""
- [x] Guide row: Resolution scaling · Firehold upscales the game itself · ‹ Quality · 1440p › (live) — hosts/firehold-shell/src/ui/guide.rs: "(\"Resolution scaling\""
- [x] Guide row: VRR / FreeSync · Smooth when it dips below 60 · ‹ On › (green) — hosts/firehold-shell/src/ui/guide.rs: "(\"VRR / FreeSync\", \"Smooth when it dips below 60\""
- [x] Guide row: Allow tearing · Lowest delay for fast games · ‹ Off › — hosts/firehold-shell/src/ui/guide.rs: "(\"Allow tearing\", \"Lowest delay for fast games\""
- [x] Guide row: Power · Handhelds and laptops · ‹ Balanced · 18 W › — hosts/firehold-shell/src/ui/guide.rs: "(\"Power\", \"Handhelds and laptops\""
- [x] Guide row: Performance overlay · Top-left corner · ‹ Basic › — hosts/firehold-shell/src/ui/guide.rs: "(\"Performance overlay\", \"Top-left corner\""
- [x] Guide button: Put it in a window on the Desk — hosts/firehold-shell/src/ui/guide.rs: "(\"Put it in a window on the Desk\", false)"
- [x] Guide button: Home (the game keeps running) — hosts/firehold-shell/src/ui/guide.rs: "(\"Home (the game keeps running)\", false)"
- [x] Guide button: Quit game (red outline) — hosts/firehold-shell/src/ui/guide.rs: "(\"Quit game\", true)"

## 4. Everything open (4-switcher.png)

- [x] Title "Open now" — hosts/firehold-shell/src/ui/couch.rs: "tb(\"Open now\", 40.0 * k, c_text())"
- [x] "Games and windows are one list. Pick one to jump to it, in Couch or Desk." — hosts/firehold-shell/src/ui/couch.rs: "Games and windows are one list. Pick one to jump to it, in Couch or Desk."
- [x] A grid of live pictures of every game and window (three a row; firehold-comp's window pictures every 3 s while it's up) — hosts/firehold-shell/src/ui/couch.rs: "image(image::Handle::from_path(&w.pic))"
- [x] Each card's tag: GAME or WINDOW — hosts/firehold-shell/src/ui/couch.rs: "text(if w.game { \"GAME\" } else { \"WINDOW\" })"
- [x] The selected card with the accent border and glow — hosts/firehold-shell/src/ui/style.rs: "border: Border { color: accent, width: 3.0, radius: radius.into() },"
- [x] Each card's name and what it's doing (Ember Trails · game · paused, Hearth Files · Pictures › Trips...) — hosts/firehold-shell/src/model.rs: "win.sub = format!(\"{} · {state}\", if g.source == \"emu\" { \"emulator\" } else { \"game\" });"
- [x] Hint bar: A Go to it — hosts/firehold-shell/src/ui/couch.rs: "hint(\"A\", \"Go to it\", k)"
- [x] Hint bar: X Close — hosts/firehold-shell/src/ui/couch.rs: "hint(\"X\", \"Close\", k)"
- [x] Hint bar: Y Window ⇄ full screen — hosts/firehold-shell/src/ui/couch.rs: "hint(\"Y\", \"Window ⇄ full screen\", k)"
- [x] Hint bar: B Back — hosts/firehold-shell/src/ui/couch.rs: "hint(\"B\", \"Back\", k)"

## 5. Library (5-library.png)

- [x] Top bar: logo tile and tabs Home, Library (selected, underlined), Apps, Store — hosts/firehold-shell/src/ui/couch.rs: "top_bar(s, k, s.tab == \"library\")"
- [x] Search box: "Search 412 games" (the real count) — hosts/firehold-shell/src/ui/couch.rs: "format!(\"Search {count} games\")"
- [x] Filter chips with counts: All, Installed, Steam, GOG, Epic, Windows apps, Emulators, Your games — hosts/firehold-shell/src/model.rs: "(\"winapp\", \"Windows apps\"), (\"emu\", \"Emulators\"), (\"yours\", \"Your games\")"
- [x] Tall game tiles with their art, six a row — hosts/firehold-shell/src/ui/couch.rs: "let cols = 6usize;"
- [x] Each tile's store tag (Steam, GOG, Epic, Emulator, Windows app, Your game) — hosts/firehold-shell/src/ui/couch.rs: "text(g.store()).size(13.0 * k)"
- [x] Each tile's state: Playing, Installed, Update ready, Not installed, Dev build, Cloud saves — hosts/firehold-shell/src/model.rs: "\"Cloud saves\""
- [x] Each game's name under its tile — hosts/firehold-shell/src/ui/couch.rs: "ts(g.title.clone(), 18.0 * k, c_text())].spacing(16.0 * k)"
- [x] The selected tile with the accent border — hosts/firehold-shell/src/ui/couch.rs: "Box::new(boxed(tile_bg(&g.id), 14.0 * k, Some((c_accent(), 3.0))))"
- [x] Hint bar: A Play, X Game options, Y Search, LT RT Filters — hosts/firehold-shell/src/ui/couch.rs: "[\"A Play\", \"X Game options\", \"Y Search\", \"LT RT Filters\"]"

## 6. Settings › Couch and Desk (6-settings-couch-desk.png)

- [x] Settings window with the list on the left: System, Couch and Desk (selected), Display, Gaming, Controllers, Network, Sound, Accounts, Updates — hosts/firehold-shell/src/ui/settings.rs: "[\"System\", \"Couch and Desk\", \"Display\", \"Gaming\", \"Controllers\", \"Network\", \"Sound\", \"Accounts\", \"Updates\"]"
- [x] Title "Couch and Desk" — hosts/firehold-shell/src/ui/settings.rs: "tb(\"Couch and Desk\", 36.0, c_text())"
- [x] "One desktop, two ways to sit. Couch is for the controller and the TV; Desk is for the mouse and keyboard. Switching never closes anything." — hosts/firehold-shell/src/ui/settings.rs: "One desktop, two ways to sit. Couch is for the controller and the TV; Desk is for the mouse and keyboard. Switching never closes anything."
- [x] Posture card: Automatic · Controller → Couch, mouse or keyboard → Desk (selected, half and half picture) — hosts/firehold-shell/src/ui/settings.rs: "posture_card(\"auto\", \"Automatic\", \"Controller → Couch, mouse or keyboard → Desk\""
- [x] Posture card: Always Couch · A console: tiles, full screen, Guide button — hosts/firehold-shell/src/ui/settings.rs: "posture_card(\"couch\", \"Always Couch\", \"A console: tiles, full screen, Guide button\""
- [x] Posture card: Always Desk · A PC: windows, bar and dock — hosts/firehold-shell/src/ui/settings.rs: "posture_card(\"desk\", \"Always Desk\", \"A PC: windows, bar and dock\""
- [x] Row: Per screen · The TV stays Couch while the monitor is Desk · TV: Couch · Monitor: Desk (shown from what's saved) — hosts/firehold-shell/src/ui/settings.rs: "setting_row(\"Per screen\", \"The TV stays Couch while the monitor is Desk\""
- [ ] Per screen, applied: each screen in its own posture (the Rust shell has one posture for every screen so far, and no picker for it)
- [x] Row: Start in · What you see when the PC turns on · Couch — hosts/firehold-shell/src/ui/settings.rs: "setting_row(\"Start in\", \"What you see when the PC turns on\""
- [x] Row: Guide button · Press once: Quick Access. Twice: everything open · Quick Access — hosts/firehold-shell/src/ui/settings.rs: "setting_row(\"Guide button\", \"Press once: Quick Access. Twice: everything open\""
- [x] Row: Games · Open full screen on the Couch, in a window on the Desk · Follow the posture — hosts/firehold-shell/src/ui/settings.rs: "setting_row(\"Games\", \"Open full screen on the Couch, in a window on the Desk\""
- [x] Row: Big Picture · Steam's own TV mode isn't needed; Steam is an app like the others · Off — hosts/firehold-shell/src/ui/settings.rs: "setting_row(\"Big Picture\", \"Steam's own TV mode isn't needed; Steam is an app like the others\""

## Still open, and why

- The chapter: no store or game data Hearth reads says which chapter you're on (shown when a game's entry has one).
- The desk bar's menus: firehold-comp has no global-menu protocol yet, so File, Edit and View are labels.
- Straight to the display: firehold-comp's direct scanout and VRR work only on a real screen (udev), not in CI.
- Friends "2 online": how many friends are online is Steam's account data; Hearth never reads Steam accounts (the tile opens Steam's own friends list instead).
- Per screen: one posture for every screen; the row shows what Firehold.json says.
- Away (the home server keeps running) isn't in these mockups and has no screen here yet.

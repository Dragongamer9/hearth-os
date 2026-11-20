# Firehold on labwc, fleshed out (the same 12 mockups, approved Nov 19, 2026; for labwc on Nov 20, 2026)

Mockups: firehold-plasma/01-couch-home.png, firehold-plasma/02-couch-home-rows.png, firehold-plasma/03-couch-game-options.png, firehold-plasma/04-modes-per-screen.png, firehold-plasma/05-settings-modes.png, firehold-plasma/06-away.png, firehold-plasma/07-quick-settings-desk.png, firehold-plasma/08-quick-settings-couch.png, firehold-plasma/09-notification-cards.png, firehold-plasma/10-notification-history.png, firehold-plasma/11-desk-bar-app-menus.png, firehold-plasma/12-desk-bar-standard-menus.png
Release: 1.16

Firehold on labwc (Fedora edition, Settings › Your desktop › "Firehold on labwc"; docs/LABWC-FEDORA.md). The owner
decided on Nov 20 (03:07) that it gets all twelve of Firehold on Plasma's approved mockups, built exactly as shown.
The Couch's pages are the very files Firehold on Plasma draws (desktop/firehold-plasma/couch/contents/ui, reached from
Mantel's shell through desktop/shell/fhcouch); labwc's side is desktop/shell/FhCouchHost.qml and FhpCouch.qml, the
helper's labwc paths are in hearth-firehold-plasma (LABWC). A part is ticked here only once it's pictured on labwc
(labwc-fedora.yml's "Firehold on labwc, the approved screens": lw-NN pictures next to mockup-NN, on ci/labwc-fedora).

## 1. Couch home (01-couch-home.png)

- [x] Top bar: Firehold flame tile, tabs Home (selected, underlined) / Library / Apps / Media / Store — hosts/linux/desktop/firehold-plasma/couch/contents/ui/TopBar.qml: "[[\"home\", \"Home\"], [\"library\", \"Library\"], [\"apps\", \"Apps\"], [\"media\", \"Media\"], [\"store\", \"Store\"]]"
- [x] Top bar right: downloads pill ("2 downloads" with a spinner), controller batteries pill, mode pill ("Couch · on the TV"), Wi-Fi, clock, account letter or picture — hosts/linux/desktop/firehold-plasma/couch/contents/ui/TopBar.qml: "text: \"Couch · on the \" + (bar.couch.screenIsTv ? \"TV\" : bar.couch.screenWord)"
- [x] Hero card: "CONTINUE · 41 MIN AGO ON THIS PC", the game's series line and big title, its art behind — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "\" ON THIS PC\""
- [x] Hero card: the chapter line ("Angmar campaign · Chapter 5 of 8: …") where the game or Beacon says it; nothing for games that don't — hosts/linux/desktop/firehold-plasma/couch/contents/ui/fhp.js: "parts.push(\"Chapter \" + p.chapter + (p.of ? \" of \" + p.of : \"\") + (p.name ? \": \" + p.name : \"\"));"
- [x] Hero card: the details line (mod, launcher, where it's saved, runs natively) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/fhp.js: "parts.push(\"runs natively, no Game Mode\");"
- [x] Hero card buttons: Continue (focused, accent ring), Open in a window, Game page, ⋯ — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "\"Open in a window\", \"Game page\", \"⋯\"]"
- [x] Hero card side panel: campaign progress (percent, bar, hours played) and "Also on the ROG Ally · saves in step" — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "\"Also on the \" + home.cards.alsoOn + \" · saves in step\""
- [x] Continue playing row: five tiles with store badge, title, who/when line, progress bar on the tile; "RB for more" — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "note: home.contRow.length >= 5 ? \"RB for more\" : \"\""
- [x] Recently added row: tiles with NEW / DEV badges, store badge, title and line; "from every store" — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "title: \"Recently added\"; note: \"from every store\""
- [x] Downloads and updates card: each download with art, name, store, size done, time left, percent and bar; "2 running · 1 waiting"; Hearth OS update line — hosts/linux/desktop/firehold-plasma/couch/contents/ui/HomeDownloads.qml: "\"Hearth OS update ready · goes in when you restart\""
- [x] Controllers card: each controller and headset with its battery; a low one in warning colour ("charge soon") — hosts/linux/desktop/firehold-plasma/couch/contents/ui/fhp.js: "(c.low ? \" charge soon\" : \"\")"
- [x] Playing on the TV card: app, title and episode, art, pause and skip buttons, time and bar — hosts/linux/desktop/firehold-plasma/couch/contents/ui/MediaCard.qml: "H.clock(mc.cp.position) + \" / \" + H.clock(mc.cp.length)"
- [x] Button hints: A Continue, X Options, Y Search, ☰ Quick Settings, View Everything open, LB RB Tabs — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "key: \"⧉\"; label: \"Everything open\""

## 2. Couch home, further down (02-couch-home-rows.png)

- [x] Watch and listen row of media apps — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "title: \"Watch and listen\""
- [x] A launcher row (Beacon) with the controller focus on a tile and its "update ready" state — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "title: \"Beacon · Battle for Middle-earth and more\""
- [x] Steam row — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "\" installed\"; note: \"sorted by last played\""
- [x] "From your Hearth" row: your own devices from Hearth Link (Dad's PC playing a game with a Help him button, the Ally, the laptop); never Steam friends — hosts/linux/desktop/firehold-plasma/couch/contents/ui/fhp.js: "\"Help him\""
- [x] TV-style notification (a controller's battery is low) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "\"Plug it in soon · hold ≡ to see all notifications\""
- [x] The focus ring follows the controller between rows and tiles; the page scrolls to keep it in view — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "readonly property real scrollTo:"

## 3. Game options (03-couch-game-options.png)

- [x] Opens with X on a tile, over the dimmed Couch home, controller-focused — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Home.qml: "if (kk === Qt.Key_X) { const g = rowGame(row, col); if (g) couch.openOptions(g); return true; }"
- [x] Play on the TV · Couch / on the monitor · Desk — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "playOn: \"Play on\""
- [x] Open in a window — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "window: \"Open in a window\""
- [x] Controller layout — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "layout: \"Controller layout\""
- [x] Pin to Home — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "\"Keeps it at the front of Continue playing\""
- [x] Saves — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "saves: \"Saves\""
- [x] Who's playing (Chris / Dad): a profile chosen for each game — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "couch.set(\"players\", m);"
- [x] Game page — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "\"Saves, screenshots, add-ons, Proton version, files\""
- [x] Uninstall (asks first) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "\"Its saves stay on Hearth Storage. A: Uninstall · B: Keep it\""
- [x] Button hints for the menu — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "key: \"◀ ▶\"; label: \"Change\""

## 4. Modes for each screen (04-modes-per-screen.png)

- [x] Opens from the Desk bar's mode pill — hosts/linux/desktop/shell/FireholdDesk.qml: "pill.lm.popupOpen = !pill.lm.popupOpen"
- [x] Both screens drawn as placed, each named (TCL TV, LG monitor), with its current mode — hosts/linux/desktop/firehold-plasma/couch/contents/ui/ModesCard.qml: "return (qa ? qa.virtualX : 0) - (qb ? qb.virtualX : 0);"
- [x] Couch / Desk / Away choice for each screen — hosts/linux/desktop/firehold-plasma/couch/contents/ui/ModesCard.qml: "options: [\"Couch\", \"Desk\", \"Away\"]"
- [x] Buttons: Both Couch, Both Desk, Away now — hosts/linux/desktop/firehold-plasma/couch/contents/ui/ModesCard.qml: "\"Away now (both)\""
- [x] Switches: Automatic, Away after 20 minutes — hosts/linux/desktop/firehold-plasma/couch/contents/ui/ModesCard.qml: "\" min with nobody at \""
- [x] Key and button hints — hosts/linux/desktop/firehold-plasma/couch/contents/ui/ModesCard.qml: "\"Keys: Super+Shift+C Couch · Super+Shift+D Desk · Super+Shift+A Away · Guide button: Couch on the \""

## 5. Settings › Couch, Desk and Away (05-settings-modes.png)

- [ ] In the Settings look, its own page
- [ ] For each screen: the mode it starts in, what's on its Couch home, sound in Couch
- [ ] Switching by itself: a controller puts the TV in Couch, the mouse puts its own screen in Desk, Away after a time, what keeps running while Away, games follow the mode
- [ ] The line "Switching modes is never a settings change" (switching is live state, never saved by itself: red flag 9)

## 6. Away (06-away.png)

- [x] Big clock and date — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Away.qml: "text: Qt.formatTime(aw.couch.now, \"h:mm AP\")"
- [x] Cards for what keeps working: a download, Hearth Link serving another device, the home server — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Away.qml: "kind: \"Home server\""
- [x] Notifications shown only as a count while Away — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Away.qml: "lines: [\"Hidden here so nobody else reads them\"]"
- [x] Power and screen-off notes — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Away.qml: "text: [\"Fans quiet: Power saver until you're back\","
- [x] How to come back (any button, key or the mouse) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/Away.qml: "text: \"Back to the Couch\""

## 7. Quick Settings on the Desk (07-quick-settings-desk.png)

- [x] Opens from the bar; you, the device and how long it's been up; Settings, Lock, Power — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "text: \"Lock\"; focused: fq.isFocused(bLock)"
- [x] Tiles: Wi-Fi (network and strength), Bluetooth (connected count), Do not disturb (with its schedule), Keyboard, Night light (until when), Screenshot — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "wifi.strength && !couch ? \" · \" + wifi.strength"
- [x] Modes for each screen in Quick Settings — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "\"each screen on its own\""
- [x] Volume with the output picker (Headset / TV (HDMI) / Speakers) and percent — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "visible: fq.sinkList.length > 1"
- [x] Brightness for the monitor; "TV: set on the TV's remote" — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "text: \"TV: set on the TV's remote\""
- [x] Power profile: Power saver / Balanced / Performance — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "\"power-saver\": \"Power saver\", \"balanced\": \"Balanced\", \"performance\": \"Performance\""
- [x] Batteries of the PC's devices (controllers, headset, mouse), a low one in warning colour — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "parent.parent.modelData.low ? F.warn : F.text"
- [x] Return to Game Mode: desktop apps close first and it asks about unsaved work; Go — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "\"Steam Game Mode · desktop apps close first, it asks about unsaved work\""
- [x] Notifications summary line with Open all — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "text: \"Open all ›\""

## 8. Quick Settings with the controller (08-quick-settings-couch.png)

- [x] LB / RB tabs and the focus ring — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "text: \"LB RB\""
- [x] Power row with the device's own steps (Silent 10 W / Balanced 18 W / Turbo 25 W on the Ally) — hosts/linux/desktop/plasma-hearth/hearth-quick: "'balanced': 'Balanced · 18 W'"
- [x] "Game Mode is off on this device · turn it on in Settings › System › Startup" when Game Mode is switched off — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "\"Turn it on in Settings › System › Startup\""
- [x] Button hints — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "t: \"Toggle\" }, { k: \"◀▶\", c: \"#d9d2cc\", t: \"Adjust\""

## 9. Notification cards (09-notification-cards.png)

- [x] Cards at the top right, each with its app, time, text and actions — hosts/linux/desktop/plasma-hearth/package/FireholdCards.qml: "readonly property var acts: n ? Array.from(n.actions || [])"
- [x] An urgent card stays until answered (Dad's PC asks for help, Connect with Remote desktop) — hosts/linux/desktop/plasma-hearth/package/FireholdCards.qml: "text: \"Stays until you answer\""
- [x] Game ready card: Play / Play on the TV — hosts/linux/desktop/plasma-hearth/package/FireholdCards.qml: "visible: card.acts.length > 0"
- [x] A card with a picture (a recorded clip) — hosts/linux/desktop/shell/DeskState.qml: "n.hints[\"image-path\"]"
- [x] A message card with a reply box in the card — hosts/linux/desktop/plasma-hearth/package/FireholdCards.qml: "visible: !!card.n && card.n.hasInlineReply === true"
- [x] "2 more" when there are more — hosts/linux/desktop/plasma-hearth/package/FireholdCards.qml: " more</b> · \""

## 10. Notification history (10-notification-history.png)

- [x] Opens from the clock; a calendar — hosts/linux/desktop/plasma-hearth/package/FireholdHistory.qml: "the time, the calendar, Do not disturb"
- [x] Do not disturb schedule (11 PM–7 AM, in full-screen games) and its exceptions (Hearth Link family messages always through) — hosts/linux/desktop/plasma-hearth/package/FireholdHistory.qml: "text: \"Always lets through: \""
- [x] Filter chips — hosts/linux/desktop/plasma-hearth/package/FireholdHistory.qml: "{ id: \"games\", t: \"Games\" }"
- [x] "While you played: N held back" — hosts/linux/desktop/plasma-hearth/package/FireholdHistory.qml: "// While you played"
- [x] The list grouped Today / Yesterday, an action on each — hosts/linux/desktop/plasma-hearth/package/fh.js: "return \"Yesterday\";"
- [x] Mute an app; kept for 7 days — hosts/linux/desktop/plasma-hearth/package/FireholdHistory.qml: "mute this app for 1 hour, or for good · Kept for 7 days"

## 11. The Desk bar with the app's own menus (11-desk-bar-app-menus.png)

- [x] The app in front with its own menus (KDE's global menu), File open — hosts/linux/desktop/shell/hearth-globalmenu: "BUS_NAME = 'com.canonical.AppMenu.Registrar'"
- [x] Left to right: app, menus, clock, mode pill, downloads, Hearth Link, controllers, Quick Settings, notifications — hosts/linux/desktop/shell/FhpDeskBar.qml: "TrayButton { anchors.verticalCenter: parent.verticalCenter; glyph: \"bell\""
- [x] The first-run tip naming each part of the bar — hosts/linux/desktop/firehold-plasma/couch/contents/ui/BarTipCard.qml: "\"THE BAR, LEFT TO RIGHT\""

## 12. The Desk bar's standard menus (12-desk-bar-standard-menus.png)

- [x] Standard menus for an app with none of its own (the app menu, Window, Help) — hosts/linux/desktop/shell/FhpDeskBar.qml: "[[\"app\", bar.appName], [\"window\", \"Window\"], [\"help\", \"Help\"]]"
- [x] Window › Move to screen (each screen with its mode) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/deskmenus.js: "here ? \"here now\" : s.mode === \"couch\" ? \"goes full screen\""
- [x] Help: search every command (Alt+Space) — hosts/linux/desktop/shell/FhpDeskBar.qml: "else if (a === \"search\") ipc([\"searchbar\", \"key\"]);"
- [x] The "controller in your hand" hint — hosts/linux/desktop/shell/Firehold.qml: "\"A controller is in your hand\""

## Build status

- 72 of 76 parts pictured on labwc and ticked (labwc-fedora.yml run 44748680972, branch ci/labwc-fedora
  run-44748680972/mockups: lw-NN next to mockup-NN; copies in hearth-workspace hearth/assets/1.16-firehold-labwc-mockups).
  The Desk bar's own menus come from Hearth's menu source (hearth-globalmenu); the stand-in apps are a Qt Widgets
  "Hearth Files" with the mockup's menus and a "Trains 2010" window with none.
- Pictures are drawn without a graphics card (CI): the art is square-cornered there (Qt's software drawing can't run the
  rounding effect); on a PC with a graphics card it's rounded as drawn.
- Open: 5, Settings › Couch, Desk and Away (Hearth's Rust Settings app on labwc): pictured in the image run (the
  container has no Rust apps), ticked from those pictures.

# Firehold on Plasma, fleshed out (mockups 01–12 approved Nov 19, 2026)

Mockups: firehold-plasma/01-couch-home.png, firehold-plasma/02-couch-home-rows.png, firehold-plasma/03-couch-game-options.png, firehold-plasma/04-modes-per-screen.png, firehold-plasma/05-settings-modes.png, firehold-plasma/06-away.png, firehold-plasma/07-quick-settings-desk.png, firehold-plasma/08-quick-settings-couch.png, firehold-plasma/09-notification-cards.png, firehold-plasma/10-notification-history.png, firehold-plasma/11-desk-bar-app-menus.png, firehold-plasma/12-desk-bar-standard-menus.png
Release: 1.16

Firehold running on KDE Plasma (hearth-firehold-plasma) on the Fedora edition. The owner approved all twelve on Nov 19
("they all look good add them to the 1.16 update") with the defaults as drawn. The sample games, people and devices
stand for the person's own. Sources: hearth-workspace hearth/mockups/firehold-plasma-more (firehold.html, make.py).
Hearth never reads Steam account data: downloads come from Steam's files on the PC, and there's no Steam friends count.

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
- [ ] Play on the TV · Couch / on the monitor · Desk
- [x] Open in a window — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "window: \"Open in a window\""
- [x] Controller layout — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "layout: \"Controller layout\""
- [x] Pin to Home — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "\"Keeps it at the front of Continue playing\""
- [x] Saves — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "saves: \"Saves\""
- [x] Who's playing (Chris / Dad): a profile chosen for each game — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "couch.set(\"players\", m);"
- [x] Game page — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "\"Saves, screenshots, add-ons, Proton version, files\""
- [x] Uninstall (asks first) — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "\"Its saves stay on Hearth Storage. A: Uninstall · B: Keep it\""
- [x] Button hints for the menu — hosts/linux/desktop/firehold-plasma/couch/contents/ui/GameOptions.qml: "key: \"◀ ▶\"; label: \"Change\""

## 4. Modes for each screen (04-modes-per-screen.png)

- [ ] Opens from the Desk bar's mode pill
- [ ] Both screens drawn as placed, each named (TCL TV, LG monitor), with its current mode
- [ ] Couch / Desk / Away choice for each screen
- [ ] Buttons: Both Couch, Both Desk, Away now
- [ ] Switches: Automatic, Away after 20 minutes
- [ ] Key and button hints

## 5. Settings › Couch, Desk and Away (05-settings-modes.png)

- [ ] In the Settings look, its own page
- [ ] For each screen: the mode it starts in, what's on its Couch home, sound in Couch
- [ ] Switching by itself: a controller puts the TV in Couch, the mouse puts its own screen in Desk, Away after a time, what keeps running while Away, games follow the mode
- [ ] The line "Switching modes is never a settings change" (switching is live state, never saved by itself: red flag 9)

## 6. Away (06-away.png)

- [ ] Big clock and date
- [ ] Cards for what keeps working: a download, Hearth Link serving another device, the home server
- [ ] Notifications shown only as a count while Away
- [ ] Power and screen-off notes
- [ ] How to come back (any button, key or the mouse)

## 7. Quick Settings on the Desk (07-quick-settings-desk.png)

- [ ] Opens from the bar; you, the device and how long it's been up; Settings, Lock, Power
- [ ] Tiles: Wi-Fi (network and strength), Bluetooth (connected count), Do not disturb (with its schedule), Keyboard, Night light (until when), Screenshot
- [ ] Modes for each screen in Quick Settings
- [ ] Volume with the output picker (Headset / TV (HDMI) / Speakers) and percent
- [ ] Brightness for the monitor; "TV: set on the TV's remote"
- [ ] Power profile: Power saver / Balanced / Performance
- [ ] Batteries of the PC's devices (controllers, headset, mouse), a low one in warning colour
- [ ] Return to Game Mode: desktop apps close first and it asks about unsaved work; Go
- [ ] Notifications summary line with Open all

## 8. Quick Settings with the controller (08-quick-settings-couch.png)

- [ ] LB / RB tabs and the focus ring
- [ ] Power row with the device's own steps (Silent 10 W / Balanced 18 W / Turbo 25 W on the Ally)
- [ ] "Game Mode is off on this device · turn it on in Settings › System › Startup" when Game Mode is switched off
- [ ] Button hints

## 9. Notification cards (09-notification-cards.png)

- [ ] Cards at the top right, each with its app, time, text and actions
- [ ] An urgent card stays until answered (Dad's PC asks for help, Connect with Remote desktop)
- [ ] Game ready card: Play / Play on the TV
- [ ] A card with a picture (a recorded clip)
- [ ] A message card with a reply box in the card
- [ ] "2 more" when there are more

## 10. Notification history (10-notification-history.png)

- [ ] Opens from the clock; a calendar
- [ ] Do not disturb schedule (11 PM–7 AM, in full-screen games) and its exceptions (Hearth Link family messages always through)
- [ ] Filter chips
- [ ] "While you played: N held back"
- [ ] The list grouped Today / Yesterday, an action on each
- [ ] Mute an app; kept for 7 days

## 11. The Desk bar with the app's own menus (11-desk-bar-app-menus.png)

- [ ] The app in front with its own menus (KDE's global menu), File open
- [ ] Left to right: app, menus, clock, mode pill, downloads, Hearth Link, controllers, Quick Settings, notifications
- [ ] The first-run tip naming each part of the bar

## 12. The Desk bar's standard menus (12-desk-bar-standard-menus.png)

- [ ] Standard menus for an app with none of its own (the app menu, Window, Help)
- [ ] Window › Move to screen (each screen with its mode)
- [ ] Help: search every command (Alt+Space)
- [ ] The "controller in your hand" hint

## Build status (branch feature/firehold-plasma-1.16; not ticked until each part is pictured and compared)

- Done, pictured (run 44524216417, fp-01/02/03): Couch home rows, cards, hero, Game options (sections 1-3). Fixes after
  those pictures are pushed but not pictured yet.
- Built, waiting for pictures (runs 44527587999 Plasma, 44527592179 Rust apps): Couch, Desk and Away per screen
  (live state in $XDG_RUNTIME_DIR/hearth-firehold-plasma/modes.json, the Couch as a layer over the Desk, Modes popup
  from the mode pill, Super+Shift+C/D/A, Identify), Away, Settings › System › Couch, Desk and Away (cats/couch.rs).
- Backend only: Quick Settings (`hearth-firehold-plasma qs` / `qs-do`); its UI and the notification cards/history are
  the shared plasmoids from feature/plasma-hearth-menu (style "firehold").
- Next: open the stage 2 pictures and fix; Play on with two screens; hook the bar to the shared Quick Settings and
  notifications; the Desk bar (sections 11-12: bar parts, first-run tip, standard menus, Move to screen).

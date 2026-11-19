# Firehold on Plasma, fleshed out (mockups 01–12 approved Nov 19, 2026)

Mockups: firehold-plasma/01-couch-home.png, firehold-plasma/02-couch-home-rows.png, firehold-plasma/03-couch-game-options.png, firehold-plasma/04-modes-per-screen.png, firehold-plasma/05-settings-modes.png, firehold-plasma/06-away.png, firehold-plasma/07-quick-settings-desk.png, firehold-plasma/08-quick-settings-couch.png, firehold-plasma/09-notification-cards.png, firehold-plasma/10-notification-history.png, firehold-plasma/11-desk-bar-app-menus.png, firehold-plasma/12-desk-bar-standard-menus.png
Release: 1.16

Firehold running on KDE Plasma (hearth-firehold-plasma) on the Fedora edition. The owner approved all twelve on Nov 19
("they all look good add them to the 1.16 update") with the defaults as drawn. The sample games, people and devices
stand for the person's own. Sources: hearth-workspace hearth/mockups/firehold-plasma-more (firehold.html, make.py).
Hearth never reads Steam account data: downloads come from Steam's files on the PC, and there's no Steam friends count.

## 1. Couch home (01-couch-home.png)

- [ ] Top bar: Firehold flame tile, tabs Home (selected, underlined) / Library / Apps / Media / Store
- [ ] Top bar right: downloads pill ("2 downloads" with a spinner), controller batteries pill, mode pill ("Couch · on the TV"), Wi-Fi, clock, account letter or picture
- [ ] Hero card: "CONTINUE · 41 MIN AGO ON THIS PC", the game's series line and big title, its art behind
- [ ] Hero card: the chapter line ("Angmar campaign · Chapter 5 of 8: …") where the game or Beacon says it; nothing for games that don't
- [ ] Hero card: the details line (mod, launcher, where it's saved, runs natively)
- [ ] Hero card buttons: Continue (focused, accent ring), Open in a window, Game page, ⋯
- [ ] Hero card side panel: campaign progress (percent, bar, hours played) and "Also on the ROG Ally · saves in step"
- [ ] Continue playing row: five tiles with store badge, title, who/when line, progress bar on the tile; "RB for more"
- [ ] Recently added row: tiles with NEW / DEV badges, store badge, title and line; "from every store"
- [ ] Downloads and updates card: each download with art, name, store, size done, time left, percent and bar; "2 running · 1 waiting"; Hearth OS update line
- [ ] Controllers card: each controller and headset with its battery; a low one in warning colour ("charge soon")
- [ ] Playing on the TV card: app, title and episode, art, pause and skip buttons, time and bar
- [ ] Button hints: A Continue, X Options, Y Search, ☰ Quick Settings, View Everything open, LB RB Tabs

## 2. Couch home, further down (02-couch-home-rows.png)

- [ ] Watch and listen row of media apps
- [ ] A launcher row (Beacon) with the controller focus on a tile and its "update ready" state
- [ ] Steam row
- [ ] "From your Hearth" row: your own devices from Hearth Link (Dad's PC playing a game with a Help him button, the Ally, the laptop); never Steam friends
- [ ] TV-style notification (a controller's battery is low)
- [ ] The focus ring follows the controller between rows and tiles; the page scrolls to keep it in view

## 3. Game options (03-couch-game-options.png)

- [ ] Opens with X on a tile, over the dimmed Couch home, controller-focused
- [ ] Play on the TV · Couch / on the monitor · Desk
- [ ] Open in a window
- [ ] Controller layout
- [ ] Pin to Home
- [ ] Saves
- [ ] Who's playing (Chris / Dad): a profile chosen for each game
- [ ] Game page
- [ ] Uninstall (asks first)
- [ ] Button hints for the menu

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

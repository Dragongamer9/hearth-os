# Build anything: every desktop your own (approved Nov 22, 2026)

The owner, Nov 22: "I even want to make Hearth Game mode Fully modular as well", "i want the imagination to be endless,
with little to NO resrticitons of what people can do", "If someone wants to turn thier desktop into an Xbox series x or
into a Ps5 interface ect they can do it", "Hearth Powers it all but were no 2 desktops or can have to be similar, it all
cators to what the user wants", "on a fresh install a full screen popup comes up with 2 options Hearth OS Mantel and the
Other be Build your own Desktop", then "looks good". Builds on desktop-layouts.md (the shared core, Describe it).
Sources: hearth-workspace `hearth/mockups/build-anything`.

Mockups: build-anything/anything-1-no-two-alike.png, build-anything/anything-2-console-dashboard.png, build-anything/anything-3-icon-row-home.png, build-anything/anything-4-block-editor.png, build-anything/anything-5-ai-made-block.png, build-anything/anything-6-game-mode-blocks.png, build-anything/anything-7-first-start.png
Release: 1.18

## 1. No two desktops are the same
- [x] Any desktop made from blocks, each person's own, on the same Hearth (apps, games, updates, settings) — hosts/linux/desktop/shell/Screen.qml: "the desktop as one screen of blocks"
- [x] Ready-made starting points: console dashboard, icon-row home, starship cockpit, cabin by the fire, paper notebook, 1995 computer — hosts/linux/desktop/shell/ScreenLayout.qml: "{ id: \"retro\", name: \"1995 computer\""
- [x] Each made by saying it, sketching it or building it block by block; the words that made it kept with it — hosts/linux/desktop/shell/ScreenEditor.qml: "[\"describe-screen\", asked, JSON.stringify(draft)].concat(sketch ? [sketch] : [])"
- [x] Only Hearth's own icons, sounds and fonts in them: no other company's logos, names or sounds — hosts/linux/desktop/shell/Screen.qml: "// A block's sound (Hearth's own: desktop/sounds/hearth/stereo/block-*.wav)"

## 2. The desktop as a console dashboard
- [x] Whole-screen layouts (not only bars, docks and panels), with windows, Alt+Tab and Files working over them — hosts/linux/desktop/shell/Screen.qml: "under your windows (windows open over"
- [x] Profile corner (picture, name, online, friends playing); search, settings, battery and time corner — hosts/linux/desktop/shell/Screen.qml: "\" friend playing\" : \" friends playing\""
- [x] Hero tile of the last game that grows on focus with a white outline, with when and where you were — hosts/linux/desktop/shell/Screen.qml: "text: scr.whenWhere(hi.g)"
- [x] Rows: Jump back in (games), Your apps and things (Games, Store, Files, Browser, Music, Friends, downloads) — hosts/linux/desktop/shell/screens/console.json: "\"title\": \"Your apps and things\""
- [x] The Guide: a side menu on the Home button (Home, Quick settings, Friends and chat, Capture and share, Your windows, Game Mode, Power) — hosts/linux/desktop/shell/Screen.qml: "{ t: \"Capture and share\", a: \"capture\" }"

## 3. The desktop as an icon-row home
- [x] Tabs across the top (Games, Media, Desktop: Desktop shows your windows and files) — hosts/linux/desktop/shell/Screen.qml: "text: modelData === \"music\" ? \"Media\" : modelData === \"windows\" ? \"Desktop\""
- [x] An icon row, the picked one bigger with its outline; the library at the end — hosts/linux/desktop/shell/Screen.qml: "// iconrow: your games as rounded icons; the picked one bigger with its outline"
- [x] The picked game's art filling the screen, its name big, Play and ⋯ — hosts/linux/desktop/shell/Screen.qml: "// play: the picked game's name big, Play and ⋯."
- [x] Activity cards under it (best lap, played this week, next up) — hosts/linux/desktop/shell/Screen.qml: "s: \"these two weeks\""

## 4. The block editor
- [x] Blocks to drag onto the screen: Hero tile, Tile row, Icon row, Tabs, Guide menu, Clock, Profile, Battery, Bar, Dock, Media, Friends, Notes, Weather, Windows, Your own… — hosts/linux/desktop/shell/ScreenEditor.qml: "{ b: \"own\", t: \"Your own…\", g: \"sparkle\", shows: [] }"
- [x] "Describe a block that isn't here, and your AI makes it"; Made by people — hosts/linux/desktop/shell/ScreenEditor.qml: "text: \"MADE BY PEOPLE\""
- [x] Free placement on a grid, any size, with handles; the picked block outlined — hosts/linux/desktop/shell/ScreenEditor.qml: "// its handles: top-left and bottom-right (size it)"
- [x] The picked block: Shows; Look (colours, Corners, See-through, Font, On focus, Sound, Animation); Place (grid, above or under windows); When pressed — hosts/linux/desktop/shell/ScreenEditor.qml: "Head { text: \"WHEN PRESSED\" }"
- [x] Undo, Try it full screen, Share it, Keep this; Describe it under the screen — hosts/linux/desktop/shell/ScreenEditor.qml: "{ t: \"Try it full screen\", a: \"full\" }"

## 5. A block your AI makes
- [x] New blocks written by your AI when Hearth hasn't got one, shown live as NEW — hosts/linux/desktop/shell/ScreenEditor.qml: "\"MADE FOR YOU BY YOUR AI · NEW\""
- [x] Each runs as its own small program: stopped by itself if it uses too much processor or memory; can't touch starting up, Hearth's system or your settings; off any time in Settings › Your desktop › Your blocks — hosts/hearth-layout/src/made.rs: "libc::setrlimit(libc::RLIMIT_CPU, &cpu);"
- [x] It asks before it reaches anything (an address, files, the internet): Allow, Just this once, No — hosts/linux/desktop/shell/OwnBlock.qml: "{ t: \"Just this once\", a: \"once\" }"
- [x] What you asked; Keep this, Change it…, Try again, Undo, Share it with Hearth… — hosts/linux/desktop/shell/ScreenEditor.qml: "text: \"Share it with Hearth…\""

## 6. Hearth Game Mode from the same blocks
- [x] Game Mode's Home, Library and Quick Access made of the same blocks as the desktop, EDITING in the bar — ui/gm-blocks.js: "<span class=\"gmb-tag\">EDITING</span>"
- [x] Edited with the controller: A pick up / put down, X its look, Y Describe it, B done, LB/RB between Home, Library and Quick Access, the right stick to resize, the left stick to move — ui/gm-blocks.js: "case 'rup': case 'rdown': gmbResize("
- [x] Describe it on the on-screen keyboard or by voice (Say it), Undo, Make it — ui/gm-blocks.js: "call('gmblocks.listen'"
- [x] A + to add a block; the 15 Home layouts as ready-made starting points — ui/gm-blocks.js: "'Start from: ' + l.name"

## 7. The first start after a fresh install
- [x] Full screen, once: "How do you want your desktop?", changeable later in Settings › Your desktop — hosts/linux/desktop/shell/FirstStart.qml: "text: \"How do you want your desktop?\""
- [x] Hearth OS Mantel (Ready now; the bar, the dock, Quick Access, Game Mode): Use Hearth OS Mantel — hosts/linux/desktop/shell/FirstStart.qml: "text: \"Use Hearth OS Mantel\""
- [x] Build your own desktop: the ready-made ones and Start from nothing; Build my own, Describe it — hosts/linux/desktop/shell/FirstStart.qml: "fs.editor.describeNew()"
- [x] Mouse, keyboard, touch or controller (A choose, ◀ ▶ move) — hosts/linux/desktop/shell/FirstStart.qml: "text: \"Mouse, keyboard, touch or controller\""

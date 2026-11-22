# Mantel in Rust (test)

The owner, Nov 22, 2026: "Could we Remake Mantel in rust, trying to make everything out of rust so it runns smother
and faster", then "ok have a Rust Mantel as a Test version in the Hearth Store so it is opt in for now, Start working
on it".

**What it is:** Mantel (Hearth Classic) drawn in Rust with iced on wlr-layer-shell (`hosts/mantel-rs`, the program
`hearth-mantel`), the way the archived Rust Firehold drew its bar and dock. It looks like the QML Mantel: the same
palettes (`theme/palettes.json`), the same settings files (`desktop.json`, `desk.json`, read only), the same icons and
wallpapers (from the QML Mantel's folder).

**Opt in only.** It's a test part in the Hearth Store (`mantel-rust`). It starts only when the Store's copy is
installed and `hearth-part` says it may be used: signed, made for this Hearth OS, not failed. labwc's autostart runs
`hearth-mantel session` in place of Quickshell then. If that closes within 20 s it's marked failed and the QML Mantel
starts, the same rule as a failed Store copy of Mantel (red flag 2). Removing it in the Store brings the QML Mantel
back at the next sign-in. The QML Mantel stays the default and the safe start the whole time.

**Exactly like Mantel** (the owner, Nov 22: "that looks rough, Needs to look 100% like mantel"): every size, gap,
colour, corner and font is taken from the QML Mantel's own files (Islands.qml, Dock.qml, Launcher.qml, Wallpaper.qml,
Theme.qml, Frame.qml), and its icons are drawn from the same icon-data.js. CI starts both Mantels with the same
settings on the same screen and puts their pictures side by side (QML above, Rust below) with how far apart they are:
the branch `ci/mantel-rust-shots`, `pair-*.jpg`. A piece is done only when its pairs match.

**One program** (Nov 22): the wallpaper, the bar, the dock and the app menu are surfaces of one `hearth-mantel
session`, as the QML Mantel is one Quickshell: one graphics context instead of four (the first test, a program per
surface, used about 500 MB with software drawing). `hearth-mantel menu` opens or closes the running shell's menu.

## The plan, piece by piece

Each piece looks like the QML Mantel's (pictured side by side in CI), passes the smoothness check at rest, and uses
less memory than the QML part it stands in for before the next one starts.

1. [ ] The bar (Islands.qml: the flame, the desktops, the window in front, the clock pill, the status island, Game
   Mode), the dock (Dock.qml: the flame, pins and open apps, dots, Show desktop), the wallpaper (Hearth's fireside,
   Slate Tide, pictures), the Hearth app menu (Launcher.qml: Apps and Games, search, ALL APPS and the System apps
   folder, the power row, the controller hints), as one program: in code; done when CI's pairs match
2. [ ] The Hearth Store's test entry ("Get test versions" on: Get / Remove, marked TEST); a quick update of it needs
   the owner's version number (Actions › Quick update for an app › mantel-rust)
3. [ ] Notifications (Mantel is the notification server) and the notification centre on the clock pill
4. [ ] Quick Access (Wi-Fi, Bluetooth, sound, brightness, night light) and the status menus
5. [ ] The full app menu: Apps and Games tabs, Settings pages in search, the power row, every menu style
6. [ ] The lock screen, password prompts (polkit) and the on-screen keyboard
7. [ ] The desktop (icons, right-click menu), every look's bar and dock styles, upright screens, several screens
8. [ ] The controller everywhere (Mantel's pad navigation)
9. [ ] Switch the default, only when every piece is done and the owner has tried it on his devices

## Not in the test yet (it says so)

- Notifications, Quick Access, the lock screen and the on-screen keyboard (pieces 3, 4 and 6): password prompts use
  the classic agent (hearth-polkit-fallback starts it when Mantel's isn't there).
- The moving wallpapers are drawn at rest (as the QML Mantel holds them with Reduce motion or without a GPU).
- Apps' tray icons, the clock pill's music and notifications, the Gaming look's cut corners, Console's shelf.
- labwc's Super key still asks Quickshell for the menu (the flame opens it; `hearth-mantel menu` too).
- Right-click menus on the dock and the bar.

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

## The plan, piece by piece

Each piece looks like the QML Mantel's (pictured side by side in CI), passes the smoothness check at rest, and uses
less memory than the QML part it stands in for before the next one starts.

1. [x] The bar (islands: the flame, the desktops, the clock pill, Wi-Fi / volume / battery, Game Mode), the dock
   (pins and open apps, dots, click to open or bring to the front), the wallpaper, a first app menu (search and grid),
   the session that keeps them up (built and checked in CI: mantel-rust.yml)
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
- The animated wallpapers show as their still pictures; Neon, Zen, Glass and Synthwave as the look's colours.
- Right-click menus on the dock and the bar.

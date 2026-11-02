# Moving out of Hearth's old Game Mode (toward 1.1)

Hearth OS is Debian + Mantel + Steam Game Mode. Hearth's own full-screen Game Mode (the Hearth app: `ui/index.html` on
`hosts/hearth-core`, drawn by the WebKit, Electron or Qt host) is archived. This page lists everything people used in
it and where each thing lives now.

**Where the old Game Mode still runs** (and nowhere else people can see it):

- **First-time setup** after installing (nimbus-session's `setup` mode), until the 1.1 installer takes it over.
- **The last-resort fallback** in nimbus-session, when nothing else can start.
- **`HEARTH_LEGACY_GAME_MODE=1`** (the developer switch): it's back as "Game Mode", and Mantel's launcher lists the
  Hearth app again.
- **Interim windows** (below): a few pages that have no app of their own yet open in the Hearth app's window at that
  one page (`nimbus --windowed --page <page>`). That's a normal window, not Game Mode.

Hidden: the "Hearth" menu entry (`nimbus.desktop`, now `NoDisplay=true`, and Mantel's launcher leaves it out),
Settings rows that switched to it or said "set it in Game Mode", the docs and README wording that offered it. Every
Game Mode button (taskbar, power menu, Quick Access, Super+G) opens Steam Game Mode.

## How the moved pages talk to Hearth

Mantel's new pages make the same calls the old Settings made, so they behave the same:

- `nimbus --settings-call <method> <json>` (`hosts/hearth-core/src/settings_cli.rs`, used through `HearthCall.qml`):
  one of Hearth's own bridge calls, from an allow-list (Remote play, Steam Game Mode, Storage, backups), answered as
  one line of JSON. Long jobs (moving a game, adding up every game's size) aren't on the list.
- `nimbus --windowed --page <page>` (`bridge.rs` `start_page`, `ui/index.html` `openStartPage`): the interim windows.
- The helpers the pages already used: `hearth-admin` (updates, going back a version), `hearth-power`, `hearth-system`,
  `hearth-access`, `hearth-link-settings`, `hearth-signin open <store>` (store sign-ins).

## Inventory

Status: **Mantel** = already in Mantel's Settings · **Moved** = moved into Mantel's Settings now · **App** = in an app
of its own · **Interim** = opens in the Hearth app's window at that page for now (needs its own app) · **Obsolete** =
only meant something in the old Game Mode, dropped.

### Settings › You

| Old page / row | Status | Now |
|---|---|---|
| Profile: name, picture | Mantel | Settings › Users (your account's name and picture) |
| Profile: status line | App | Friends (Matrix presence) |
| Profile: accent colour, background | Mantel | Settings › Appearance (accent, wallpaper) |
| Profiles: switch, add, ask at start | Obsolete | One account per person (Settings › Users), each with their own Steam account |
| Parental controls (PIN, limits) | Obsolete | Steam Families in Steam; Linux accounts in Settings › Users |
| Appearance: look, Made for, menu style, Quick Access style, themes, home layouts, sound pack, interface sounds | Obsolete | Old Game Mode's own look |
| Appearance: text size | Mantel | Settings › Accessibility › Text size |
| Appearance: desktop wallpaper | Mantel | Settings › Appearance |
| Library look (layouts) | Obsolete | Steam's library |
| Accessibility: zoom, colour filters, screen reader, high contrast, text size | Mantel | Settings › Accessibility |
| Accessibility: Game Mode zoom (gamescope) | Obsolete | Steam's own magnifier (Steam › Settings › Accessibility); row removed |
| Sign-in: automatically, password after sleep, start in, password, device name | Mantel | Settings › Sign-in |

### Settings › Connections

| Old page / row | Status | Now |
|---|---|---|
| Wi-Fi, airplane mode | Mantel | Settings › Wi-Fi |
| Bluetooth | Mantel | Settings › Bluetooth |
| Nearby Share, Hearth Connect | App | Nearby Share, Hearth Connect (their own apps) |
| Remote play (Sunshine on/off, Sunshine login, pair with PIN, paired devices, Moonlight) | **Moved** | Settings › Remote play (`SetRemote.qml`) |
| Hearth Link (this device, pairing, devices, copies, sync settings, relay) | Mantel | Settings › Hearth Link |
| Hearth Link › Get a game from another Hearth | Interim | `--page library` (Get from another Hearth); also in Settings › Games & stores |
| Hearth Storage: role, servers & devices, shared folders, backups & saves, away from home, SMB on/off | Mantel | Settings › Hearth Link (Hearth Storage sections) |
| Hearth Storage: SMB password, game archive browsing, media, downloads & updates | Interim | `--page storage` ("Hearth Storage's extras" row) |
| Friends & privacy | App | Friends (Mantel's Friends panel) |

### Settings › Display & sound

| Old page / row | Status | Now |
|---|---|---|
| Resolution, refresh rate, scale, brightness | Mantel | Settings › Display |
| Nvidia drawing (fast / compatible) | Mantel | Settings › Display › Nvidia, renamed "How the Hearth app draws" (first-time setup, interim windows) |
| Renderer (WebKit / Electron / Qt) | Obsolete | Picked automatically; only first-time setup draws with it |
| Volume, mute, over-amplification, output, microphone | Mantel | Settings › Sound |

### Settings › Controls & power

| Old page / row | Status | Now |
|---|---|---|
| Power mode, power limit (TDP), battery | Mantel | Settings › Power |
| Power button press / hold, sleep timers | Mantel | Settings › Power |
| Controllers: button test | App | Controller Lab |
| Controllers: controller mouse, pointer speed, Nintendo layout, controller type | Mantel | Settings › Controller |
| Controllers: button prompts, swap A/B and X/Y in Hearth's screens | Obsolete | Old Game Mode's own screens |
| Handheld Daemon | Interim | `--page hhd` from Settings › Controller (its own app is on feature/handheld-osk) |
| In-game overlay (Guide opens it, hold View, quick glance) | Obsolete | Steam's Quick Access and overlay in Steam Game Mode |
| Hearth Input layouts (per game) | Interim | Game setup in the library (`--page library`); needs its own app |
| Keyboard (on-screen keyboard layouts, typing, look, learned words) | Mantel | Settings › Keyboard (themes: feature/handheld-osk) |

### Settings › Games

| Old page / row | Status | Now |
|---|---|---|
| Game stores & accounts (Epic, GOG, Xbox sign-in) | **Moved** | Settings › Games & stores › Store accounts (`hearth-signin open <store>`; the sign-in itself still runs in the Hearth app's window) |
| Library sources (Steam, Epic, GOG, emulators; keep Steam ready), add a game manually, rescan | Interim | `--page sources` from Settings › Games & stores |
| Storage: drives, games by size, clean up, low space warning | **Moved** | Settings › Storage (`SetStorage.qml`) |
| Storage: move a game, add up sizes again | Interim | `--page diskspace` from Settings › Storage |
| Import (games and files from other drives) | Interim | `--page import` from Settings › Games & stores |
| Multi-seat | Obsolete | Dropped for now (`HEARTH_LEGACY_MULTIPLAYER=1`) |
| Couch Co-op | Obsolete | Dropped for now (`HEARTH_LEGACY_MULTIPLAYER=1`) |

### Settings › System

| Old page / row | Status | Now |
|---|---|---|
| Updates: version, look for updates, download, restart | Mantel | Settings › Updates |
| Updates: go back to the previous version, stay on this one | **Moved** | Settings › Updates › If something broke (`hearth-admin rollback`) |
| Updates: writable system (Peppermint) | Mantel | Settings › Updates |
| Device: name, storage | Mantel | Settings › Sign-in (name), Settings › About (storage) |
| Device: game art kept offline | Obsolete | Old library's art cache (Hearth Sync uses it by itself) |
| Device: benchmark | App | Benchmark (Hearth Tools); also in Settings › Games & stores |
| Web browser (Hearth Browser, Aloha) | Mantel | Settings › Web browser |
| Claude | Mantel | Settings › Claude |
| Steam Game Mode: switch, Decky Loader and Hearth Sync, last failure | **Moved** | Settings › Steam Game Mode (`SetSteam.qml`) |
| Default apps | Mantel | Settings › Default apps |
| Date and time, 24-hour clock | Mantel | Settings › Date & time |
| Count me in the player count | Obsolete (follow-up) | The old app sent the ping; Mantel needs its own counter ping to keep it |
| Backup: back up Hearth | **Moved** | Settings › About › Backup |
| Backup: restore from a backup | Interim | `--page system` (Hearth restarts after) |
| Help & logs: save / send logs | **Moved** | Settings › About › Send logs to the developer (`nimbus --send-logs`), and the Logs app |
| Help & logs: send a report when stuck, log controller input, restart Hearth, reset appearance | Obsolete | About the old Game Mode app itself |
| Desktop Mode page (switch to the desktop, recovery tips) | Obsolete | Steam's Power › Switch to Desktop |
| Install Hearth OS (live USB) | Mantel | Settings › Install Hearth (1.1's installer replaces it) |
| About: version, device, support, credits | Mantel | Settings › About |
| Linux (Android only) | Obsolete | Hearth for Android is archived |
| Integrations, theme files (hearth-play.js) | Obsolete | Old Game Mode's own look and Discord status |

### Screens that weren't Settings

| Old screen | Status | Now |
|---|---|---|
| Home, Library, game pages | App | Steam's library (Hearth Sync adds your other games) and Mantel's launcher › Games |
| Epic and GOG installs, Downloads | Interim | `--page downloads` / `--page library`; needs its own app (or Heroic / Minigalaxy) |
| Store (Steam, GOG, Epic, Xbox, Battle.net) | Interim | `--page shop`; Steam's store in Steam |
| Recomps | Interim | `--page recomps` |
| Friends (chats, requests, invites, activity) | App | Friends |
| Mods (Hearth Mods, Nexus, Wabbajack) | App | Mods |
| Saves | App | Saves |
| Controller test | App | Controller Lab |
| Theater | App | Theater |
| Screenshots | App | Photos |
| Benchmark | App | Benchmark (Hearth Tools) |
| Hearth Link screen | Mantel | Settings › Hearth Link |
| Quick Access, power menu, dock | Obsolete | Steam's Quick Access in Game Mode; Mantel's Quick Access and power menu |
| News (blog) | Obsolete | The website |
| First-time setup | Kept | Runs once after installing, until the 1.1 installer |

## Needs its own app (the interim list)

Library sources and Import · Epic/GOG installs and Downloads · Store and Recomps · Storage's moves · Hearth Storage's
extras · restoring a backup · store sign-ins (the sign-in window) · Hearth Input layouts · Handheld Daemon (in progress).

## Follow-ups

- The old Hearth Decky plugin's quick links (`hearth-app:` Steam shortcuts such as "Hearth Settings") stay on devices
  that had it; Hearth Sync leaves them alone. Removing them belongs in Hearth Sync.
- The player count ping (Count me in) only came from the old app.

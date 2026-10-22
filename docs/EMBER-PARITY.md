# Ember and Hearth OS: Game Mode parity

The shared UI (`ui/index.html`) runs on both hosts. This page compares what each host answers, feature by feature:
**Hearth OS** (the Rust host, `hosts/hearth-core`), **Ember before** this round, and **Ember now**.
Ember's Mantel desktop (taskbar, Hearth Settings pages, Apps, desktop icons) is tracked separately in
[MANTEL-PARITY.md](MANTEL-PARITY.md).

How the audit was done: every `call('…')` in `ui/index.html` (plus the ones made through `dlCall`, `imCall` and
`pinCall`), checked against hearth-core's `bridge.rs` match arms, against Ember's `SystemBridge.cs` (with
`GameModeBridge.cs` and Mantel's `DeskBridge.cs`), and against `HOSTINFO.features` on each side.

**Key:** *same*: already equal. *done*: added in this round. *differs*: Windows does it its own way, or can't (the reason is given).

## Features Ember reports

| `HOSTINFO.features` | Hearth OS | Ember before | Ember now |
|---|---|---|---|
| quickResume, friends, profiles | yes | yes | same |
| desktopMode, mantel | — | yes | same (Windows only) |
| restartApp, sendLogs, exportLogs | yes | no | done |
| screenshots | yes | no | done |
| gameSettings, hearthInput, pause | (Linux shows these without a flag) | no | done: the UI now shows them on Ember when the flag is there |
| collections | (no flag) | no | done (flag for information only) |
| overamp, powerButton | yes | no | differs: Linux-only settings (PipeWire over-amplification, logind power key) |
| remap | with InputPlumber/HHD | no | differs: Linux controller daemons; Windows has Steam Input / the vendor's tool |
| coop, seats, extraSeat | Beta Experimental | no | differs: not possible on Windows without third-party tools (see below) |
| xodus, nvidiaDrawing | Linux | no | differs: Linux only (Xbox PC games run natively on Windows; WebKitGTK setting) |

## Game Mode and the app

| Feature | Hearth OS | Ember before | Ember now |
|---|---|---|---|
| Library: Steam, Epic, GOG, Xbox, your own games | yes | yes | same |
| Collections (Library tabs of your own) | `library.collection.*`, `cols` on each game | no (the calls failed, `cols` missing) | **done**: same calls and fields, kept in `library.json` |
| Library layouts, compact settings, themes | UI only | UI only | same (nothing host-side) |
| Hide, favorite, add/remove your own game | yes | yes | same |
| Delete game | Epic/GOG by Hearth; Steam through `steam://uninstall` | Epic/GOG only; Steam games showed Delete but it failed | **done**: Steam games go to Steam's own uninstall (answers `{steam: true}` like Hearth) |
| Downloads: pause, resume, start now | yes | Cancel only (the other buttons failed) | **done**: `game.pauseInstall/resumeInstall/installFirst` in Ember's queue (legendary and gogdl carry on from what they got) |
| Downloads: Manage in Steam | `steam.downloads` | missing | **done** |
| Store sign-ins (Epic, GOG, Xbox, Steam) | yes (in Desktop Mode from Game Mode) | yes (own window) | same (`accounts.signInDesktop` is Linux Game Mode only) |
| GOG cloud saves | yes, per-game switch | yes, always | **done**: the per-game switch (Game settings › Saves) |
| Epic cloud saves | yes (`legendary sync-saves`) | no | **done**: down before playing, up after, for games Ember installed; legendary finds the save folder on Windows |
| Xbox Cloud Gaming tile | yes (Linux streams Game Pass) | no | differs: on Windows, Game Pass PC games install with the Xbox app and show under Xbox; a cloud tile is left for later |
| Emulators (RetroArch) | yes | no | differs: Linux only for now (`emu.*` and the Emulators source are behind `LINUX` in the UI) |
| Beacon (BFME and friends) | `beacon status --json` etc. | no | **done (hook)**: lists and starts Beacon's games through `beacon.exe` when the Windows Beacon is installed (PATH or `%LOCALAPPDATA%\Programs\Beacon`), same contract as `beacon.rs`; Beacon's window as `beacon-app.exe`. Nothing shows until a Windows Beacon exists |
| Move games / Import games | `import.*`, `takeover.*` | no | differs: Linux only (bringing games over from Windows/other Linux systems); the Import tab is hidden on Ember |
| Game details: ProtonDB, achievements | `game.info` | no | differs: ProtonDB is Linux-only; the UI only asks on Linux |
| Game settings page | Proton, launch options, MangoHud, GameMode, FSR, frame-rate cap, power limit, controller layout, cloud saves | no (menu item hidden on Windows) | **done**: kind `windows`: controller layout, power limit (AMD), cloud saves, Delete. Proton/MangoHud/FSR/frame-rate cap don't apply on Windows (games run natively; no system frame limiter) |
| Quick Resume (several games put aside, bring back, close) | `SIGSTOP`/`SIGCONT`, up to 5, memory guard | yes (`NtSuspendProcess`, up to 5, memory guard, list kept across restarts) | **done (improved)**: refuses games with anti-cheat (Easy Anti-Cheat, BattlEye, EA anti-cheat and others, found by their processes) and games Windows won't let it freeze, with the reason, instead of half-freezing them; the running game is remembered across Ember restarts too (`running-game.json`), like Hearth's `restore_quick` |
| Pause (Quick Access › Now playing) | `game.pause/unpause` | no (button hidden) | **done**: freezes the game in place; it carries on by itself when its window is back in front |
| Quick Resume through Shut down and Restart | hibernation status + setup; Shut down hibernates and turns off, Restart hibernates and restarts (`hearth-admin hibernate-reboot`) | status only | partly: Shut down hibernates with games open when hibernation is on (setup needs `powercfg /hibernate on` as administrator; Ember explains). Windows can't hibernate and then restart, so Restart closes games and says so |
| Hearth Input (per-game controller layouts) | hearth-input (evdev/uinput) | no | **done**: C# port of the engine, same tables, templates (incl. Strategy (RTS)), checks and editor; XInput in, `SendInput` (scan codes) out. See *Hearth Input on Windows* |
| Per-game power limit | HHD or hearth-power | no | **done**: RyzenAdj on AMD handhelds/laptops, per-game limit applied when the game comes to the front, the usual one back after. See *Power limit on Windows* |
| Quick Access › Performance | CPU/GPU/RAM, frame-rate limit, overlay, power mode, TDP, HHD | CPU/GPU/RAM/battery only | **done**: power mode chips and the TDP chips (where supported); frame-rate limit and on-screen overlay stay off (no gamescope/mangoapp on Windows) |
| In-game overlay | gamescope layer | own topmost window | same idea, different way |
| Screenshots (take, browse, delete) | yes | take only | **done**: Library › Screenshots lists `Pictures\Screenshots\Ember`, served from `nimbus-art.local/shot/…` |
| Friends: Message, Join, Steam profile | `friends.action` | list only (View Steam profile failed) | **done**: same `steam://` links |
| Bug reports: Save logs / Send logs to the developer | `logs.save`, `logs.send`, `logs.sendToDeveloper` | no | **done**: same "Hearth logs" text format (header line, `===== About this device =====` JSON first), personal details removed, posted to the same service with `X-Hearth-Kind: windows` and the "What happened?" note |
| Restart the app | `app.restart` | no | **done** (games keep running; Ember picks the running game up again) |
| Steam in the background | `-silent`, Steam's own windows filtered (`steam_question`) | `-silent`, hid only the "Steam" window, and brought any other big Steam window in front | **done**: same rule as Hearth: "Steam", "Friends List" hidden; Launching/Preparing/Updating/Big Picture left alone (never pulled over the game); real questions (cloud conflict, sign-in, Install) brought in front once |
| Couch Co-op | `hearth-coop` (Beta Experimental) | no | differs: **not available on Ember**. Running several copies of a game, each with its own controller and part of the screen, needs Nucleus Co-op-style injection and input isolation on Windows. The feature flag stays off, so the UI hides it |
| Multi-seat | Beta Experimental | no | differs: Windows client editions can't host several local desktops |
| Remote play (Sunshine/Moonlight) | yes | no | differs: Linux setup only; on Windows, Steam Remote Play / Sunshine's own installer |
| OS install, OS updates, system migration, grow the drive | Linux | — | differs: Hearth OS only. Ember has Switch to Hearth OS instead |
| Backup and restore | yes | no | not done this round (UI hides it on Ember; Ember's data is `%LOCALAPPDATA%\Nimbus`) |
| Web browser (Aloha, Hearth Browser) | Linux | — | differs: Linux only |

## Hearth Input on Windows

`hosts/windows/Nimbus.Host/Input/HearthInput.cs`, fed every 10 ms by `ControllerBridge`.

- **Same data.** `input-layouts.json` is built into Ember as a resource, so the keys, controls, limits and templates
  are the same file Hearth uses. Layouts are checked with the same rules and messages as `input_map.rs` and
  hearth-input's `validate_layout`. Each game's choice is a file in `%LOCALAPPDATA%\Nimbus\input\` named like Hearth's.
  Beacon's strategy games start with Strategy (RTS), LOTR: Conquest with Off.
- **Same engine.** Taps, tap-and-hold, turbo, the second set (hold), the radial menu, stick-as-keys with hysteresis,
  stick-as-mouse/scroll curves, trigger thresholds and "buttons already down are ignored until let go" are ported
  line for line from hearth-input.
- **When it runs.** Only while the running game's window is in front (its process is in the game's folders), or while
  a layout is tried from the editor. It lets go, releasing every key, when the overlay or Ember comes in front, when
  the game leaves the front, and for 1.5 s after Guide (Quick Access opening).
- **Output.** `SendInput` with scan codes (the key's place on the keyboard, like Linux's `KEY_*`), so DirectInput and
  raw-input games see them; mouse moves are relative, the wheel takes partial notches. Nothing but key releases is
  sent while one of Ember's own windows is in front, except to the editor's Try it page, which catches the keys itself.
- **Limits.** XInput has no back paddles (L4/R4/L5/R5) and no motion sensors: those controls and gyro aiming do nothing
  (`input.status` reports no gyro). Windows can't take the controller away from the game: a game that reads
  controllers sees both, so layouts are for games without controller support (every game starts Off). HidHide (a
  driver) can hide a controller from games; Ember detects it (`input.status.hidhide`) and explains, but doesn't set it
  up (it needs administrator rights and per-device choices, and couldn't be tested here).

## Power limit on Windows

`hosts/windows/Nimbus.Host/Services/TdpService.cs`.

- **How.** RyzenAdj 0.17.0 (`--stapm-limit`, `--fast-limit`, `--slow-limit`, all the same, 5–30 W, as hearth-admin does).
  Downloaded from its GitHub release the first time, checked against a pinned SHA-256 (the zip and each file in it), like
  legendary and gogdl.
- **Administrator rights.** RyzenAdj's WinRing0 driver needs them. Unless Ember runs as administrator, each change
  goes through Windows' permission prompt: `Nimbus.exe --admin tdp FILE WATTS FOLDER` (the same `--admin` route as Switch
  to Hearth OS) copies the files to `%ProgramData%\EmberRyzenAdj` (owned by Administrators, read-only for users), checks
  every SHA-256 again, and runs it.
- **Per game.** Game settings › Power limit keeps a limit for the game. It goes on when the game starts or comes back
  from Quick Resume, and the limit Ember set before comes back when it closes or is put aside (if Ember never set one,
  nothing is put back). Picking "Usual (no change)" there forgets it.
- **When it steps aside.** Not AMD, or a desktop Ryzen (RyzenAdj knows laptop and handheld chips), or **Handheld
  Companion** running: it owns the limit and has no documented API to ask it, so Ember says "set it there".
- **Known Windows problem.** Windows 11's vulnerable-driver blocklist (on with Memory integrity) blocks WinRing0 on many
  PCs; RyzenAdj then can't initialize and Ember shows that reason.

## Bug reports

`hosts/windows/Nimbus.Host/Services/BugReport.cs`. The file starts `Hearth logs · <date> · Ember <version>`, then
`===== About this device =====` (Settings › About's JSON plus `version`, `host: ember`, `kind: windows`), Ember's
settings, Quick Resume, controllers and Hearth Input, power, other programs (Steam, Beacon, HidHide, Handheld
Companion, WebView2), the controller input lines, recent errors, `bridge.log` (newest 1500 lines, polling noise left
out), `bridge.log.old`, the last update's log, memory and processes, and Windows' Application log errors. Tokens,
e-mail addresses, Steam ids, Wi-Fi names, your user name and user folder are taken out (the same rules as `logs.rs`).
Saved as `%USERPROFILE%\Hearth-logs-<date>.txt` (and copied to any removable drive); Send posts it to
`https://hearth-counter.game305-cl.workers.dev/v1/logs` with `X-Hearth-Version`, `X-Hearth-Kind: windows` and the
percent-encoded `X-Hearth-Note`, and shows the code. `bridge.log` now rolls over at 4 MB to `bridge.log.old`.

## Beacon on Windows (the hook)

`hosts/windows/Nimbus.Host/Library/Beacon.cs`, the same contract as `hosts/hearth-core/src/beacon.rs` and
[BEACON.md](BEACON.md): `beacon.exe status --json`, `beacon.exe mods <game>`, `beacon.exe select-mod <game> <id|none>`,
`beacon.exe launch <game> [--mod=<id>|--vanilla] --wait`, and Beacon's window `beacon-app.exe --fullscreen`. Game and mod ids
are checked the same way, paths must be absolute Windows paths. `beacon.exe` is looked for in absolute `PATH` folders,
`%LOCALAPPDATA%\Programs\Beacon` (and its `bin`), and `%ProgramFiles%\Beacon`. **For the Windows Beacon:** Windows file
names ignore case, so the window can't be `Beacon.exe` next to a `beacon.exe` tool: ship the window as `beacon-app.exe`,
or the tool in `bin\`. A running game is tracked by the game folder from `status` (`path`).

## What's left

- Xbox Cloud Gaming tile on Ember (a WebView2 window at xbox.com/play, like Hearth's stream window).
- Backup and restore of `%LOCALAPPDATA%\Nimbus`.
- Setting up HidHide automatically.
- Couch Co-op and Multi-seat: not feasible without third-party tools.
- Emulators (RetroArch for Windows) would need its own scanner and download.

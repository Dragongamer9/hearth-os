# Ember (Windows 11 shell): developer guide

Ember is Hearth's library and desktop as a Windows 11 shell that replaces Explorer. This is the technical guide: running it, the bridge, the system settings, the library, store accounts, the overlay, shell mode and updates. For an overview see the [README](../README.md).

## Try the prototype (no build)

Open `ui/index.html` in Edge or Chrome. Plug in an Xbox-style controller and press any button.

| Action | Controller | Keyboard |
|---|---|---|
| Move | D-pad / left stick | Arrow keys |
| Select | A | Enter / Space |
| Back | B | Esc / Backspace |
| Main menu | Start (☰) | M |
| Quick Access | View (⧉) | Q |
| Search + on-screen keyboard | Y | / |
| Switch tabs (Library, Quick Access) | LB / RB | [ / ] |
| Favorite (game page) | X | — |

Touch and mouse work too. Sliders in Quick Access adjust with left/right while focused.

## Run inside the host (Windows)

Needs the .NET 8 SDK. The WebView2 runtime ships with Windows 11.

```powershell
cd hosts\windows\Nimbus.Host
dotnet run -- --windowed      # dev: normal window
dotnet run                    # fullscreen, like the real shell
```

The UI detects the host via `window.chrome.webview` (on Linux, `window.nimbusHost`) and sends messages to it. The full contract is in [docs/bridge.md](bridge.md).

| Message | Host does today |
|---|---|
| `launch` (steam) | `steam://rungameid/<appId>` |
| `launch` (manual) | starts the .exe and reports `gameExited` |
| `power` | sleep / restart / shut down / start `explorer.exe` |
| `exitGame` | closes a tracked manual game |
| `call` | system settings API, see below |
| `setting`, `install`, `ready` | stubbed; wired in milestone 2 |

**Escape hatch:** `Ctrl+Alt+Shift+E` always starts File Explorer while Ember runs. `Ctrl+Shift+Esc` (Task Manager) is always available from Windows.

See **Using Ember as your shell** below.

## System settings

Settings › Wi-Fi, Bluetooth, Display, Audio, Battery & power, plus the Quick Access toggles, are real in the host.
In the browser they run against a built-in fake Windows (try a wrong password: anything under 8 characters, or `wrongpass`).

| Area | Methods | Windows API |
|---|---|---|
| Radios | `radios.set`, `airplane.set` | `Windows.Devices.Radios` (airplane mode is emulated: radios off, restored after) |
| Wi-Fi | `wifi.scan`, `wifi.connect`, `wifi.disconnect`, `wifi.forget` | `Windows.Devices.WiFi`; forget uses `netsh wlan delete profile` |
| Bluetooth | `bt.paired`, `bt.scanStart/Stop`, `bt.pair`, `bt.confirmPin`, `bt.unpair` | `Windows.Devices.Enumeration` pairing (Classic + LE) |
| Audio | `audio.get`, `audio.setVolume`, `audio.setMute`, `audio.setDefault` | Core Audio COM; default device via `IPolicyConfig` (undocumented, stable) |
| Display | `display.get`, `display.setBrightness` | WMI `WmiMonitorBrightness` (built-in screens only) |
| Power | `power.get`, `power.setMode`, battery in `sys.status` | `SystemInformation.PowerStatus`, power-mode overlay in `powrprof.dll` |

The host also pushes `status` events when radios, network or battery change.

**Known Windows requirements and limits**
- Wi-Fi scanning needs *Settings › Privacy & security › Location › Let desktop apps access your location* (Windows 11 24H2+). Ember shows that message if access is denied.
- Bluetooth devices can't be force-connected or disconnected through public APIs; Ember pairs and removes them, and paired devices auto-connect when switched on. PIN-match pairing asks you to confirm in the UI.
- External-monitor brightness (DDC/CI) isn't implemented yet.
- Power mode only applies while the Balanced power plan is active.

## Game library

The host scans installed games at startup and on *Settings › Library sources › Rescan library*:

| Store | Where it looks | Launches with | Art |
|---|---|---|---|
| Steam | `libraryfolders.vdf` → `appmanifest_*.acf`; play time from the signed-in user's `localconfig.vdf` | `steam://rungameid/<id>` | Custom grid images, then Steam's `appcache/librarycache` |
| Epic | `ProgramData\Epic\EpicGamesLauncher\Data\Manifests\*.item` | `com.epicgames.launcher://apps/...?action=launch` | Generated |
| GOG | `HKLM\SOFTWARE\GOG.com\Games` (32-bit view) | The game's exe | Generated |
| Xbox / Game Pass | Store packages with a `MicrosoftGame.config` | `AppListEntry.LaunchAsync()` | `ShellVisuals` images from the config |
| Manual | Added with the file picker | The .exe / shortcut | Generated |

Ember's own data (favorites, hidden games, manual games, play stats, enabled stores) is in `%LOCALAPPDATA%\Nimbus\library.json`.
Art is served to the UI from `https://nimbus-art.local/<id>/<cover|hero|logo|wide>`.

**Running games:** after launch, Ember watches for any process whose exe is inside the game's folder. That is how it knows the game started (even through Steam/Epic) and when it closed. *Stop* asks the game to close, then force-closes it after ~6 s. When the game exits, Ember records play time and comes back to the front. Verified on a real PC with a Steam game: started detected after Steam's hand-off, closed in ~1 s.

`tools/TestGame` + `scripts/windows/build-testgame.cmd` build a tiny stand-in game for testing on PCs without any installed games.

## Owned games, installs and Steam in the background

- **Owned Steam games** come from Steam's binary caches: `appcache/packageinfo.vdf` (packages this account has licenses for, minus package 0) joined with `appcache/appinfo.vdf` (name, type, OS, release state, art paths). Only released Windows games are listed. No Web API key or sign-in needed. Parser: `Library/SteamCache.cs` (appinfo v27–v29, packageinfo v27–v28).
- **Art for uninstalled games** uses the exact CDN paths Steam records (`store_item_assets/steam/apps/<id>/<hash>/library_capsule.jpg` etc.). If an image fails, the UI tries the next one, then a gradient.
- **Install** runs `steam://install/<id>`. Steam always shows its small Install dialog; that confirmation can't be skipped.
- **Downloads**: state and progress come from each `appmanifest_*.acf` (`StateFlags`, `BytesDownloaded/BytesToDownload`). Steam only writes those at checkpoints, so between checkpoints Ember estimates progress from the PC's received network bytes and shows the speed. The UI polls `library.downloads` every 3 s while something is installing, reloads the library when a download finishes, and shows "ready to play".
- **Steam in the background** (*Settings › Library sources*, on by default): Ember starts `steam.exe -silent` at startup and before Steam launches/installs, and hides Steam's main window (title exactly "Steam") for a short while afterwards. Other Steam windows, like the Install dialog, are left alone, and the tray icon still opens Steam.

## In-game overlay

Press the **Xbox button** (or **Ctrl+Alt+Q**) during a game: a Quick Access panel slides in from the right over the game, with quick settings, live performance (CPU, GPU, video memory, RAM, battery), Now playing (play time, screenshot, controller mouse, Back to Ember, Exit game) and notifications. B, the Xbox button or clicking the game closes it. In the Ember UI itself the Xbox button opens Quick Access.

- It's an ordinary always-on-top window: nothing is injected into games, so anti-cheat has nothing to object to. It shows over borderless/windowed games and the "fullscreen optimized" mode most games use on Windows 10/11; very old exclusive-fullscreen games may minimize when it opens.
- Windows Game Bar also opens on the Xbox button by default. **Settings › Controls › Xbox button opens only Ember** turns that off (`HKCU\Software\Microsoft\GameBar\UseNexusForGameBarEnabled`). **Hold View to open the overlay** is an optional backup.
- Screenshots go to `Pictures\Screenshots\Nimbus`.
- The Xbox button is read with XInput's long-standing undocumented `XInputGetStateEx` (ordinal 100).

## Store accounts: Epic, GOG and Xbox (Library › Stores)

Open **Library › Stores & accounts** (X on a controller, F on a keyboard) to sign in. Each store's own sign-in page opens in a full-screen window; you type your password there, and Ember only picks up the one-time code the store hands back. In that window the controller mouse turns on by itself, Y opens an on-screen keyboard and B cancels.

| Store | What you get | How |
|---|---|---|
| **Epic Games** | Your whole Epic library; install, launch and uninstall **without the Epic Games Launcher** | [legendary](https://github.com/legendary-gl/legendary) 0.21.1 (open source, GPLv3) |
| **GOG** | Your whole GOG library with covers; install, launch and uninstall **without GOG Galaxy** | GOG's web API for sign-in and the list, [gogdl](https://github.com/Heroic-Games-Launcher/heroic-gogdl) 1.3.0 (from Heroic, GPLv3) for downloads |
| **Xbox** | Gamertag and gamerpic; the PC games you've played on Xbox, with Xbox art | Microsoft account → Xbox Live sign-in. Install opens the Microsoft Store, since Microsoft only lets the Store / Xbox app install these games |
| **Steam** | Already built in | Signs in through Steam itself ("Open Steam") |

- legendary and gogdl are separate programs, downloaded from their GitHub releases the first time you sign in, checked against a pinned SHA-256, and kept in `%LOCALAPPDATA%\Nimbus\tools`. They are not part of Nimbus.
- These are unofficial integrations (the same approach Heroic and Playnite use). Stores can change their APIs.
- Sign-ins are stored per Windows user: Epic in `%LOCALAPPDATA%\Nimbus\legendary`, GOG in `%LOCALAPPDATA%\Nimbus\gog`, Xbox's token encrypted with Windows DPAPI in `%LOCALAPPDATA%\Nimbus\accounts`.
- Epic and GOG games install to `%USERPROFILE%\Games\Epic` and `\GOG` by default (change it on the Stores page). Downloads run one at a time and show in **Downloads**, where you can cancel them; starting again resumes. Epic games' one-time prerequisites (e.g. Visual C++ runtimes) run after the install, like the Epic launcher does.
- Game lists refresh in the background every 12 hours, or with **Refresh list**.

## Profile

**Settings › Profile**: your name, a status line, a picture (initials, 12 built-in pictures, your Xbox/Steam/GOG picture once signed in, or any image file), an accent color, and a background theme (Midnight, Slate, Plum, Forest, Ember, Black, or your own picture). The picture shows in the top bar and the menu.

Debug helpers: `bridge.log` next to `Nimbus.exe` logs every bridge call and UI image failures; **Ctrl+Shift+D** logs the focused game's art state. `scripts/windows/rebuild-and-run.cmd` builds and relaunches in one click.

## Install on any PC

Open PowerShell and run:

```powershell
irm https://raw.githubusercontent.com/Dragongamer9/hearth-os/main/tools/install-nimbus.ps1 | iex
```

or double-click `scripts/windows/install-nimbus.cmd`. It installs the .NET 8 Desktop Runtime if missing, downloads the latest release, checks its SHA-256, installs it to `%LOCALAPPDATA%\Nimbus\app`, saves the undo script, and makes Ember your shell (add `-NoShell` to skip that). `scripts/windows/install-shell.cmd` does the same from a local build, for development.

## Using Ember as your shell

1. Run **`scripts/windows/install-shell.cmd`**. It publishes a Release build to `%LOCALAPPDATA%\Nimbus\app`, copies the undo script to `%USERPROFILE%\nimbus-restore.cmd`, and sets
   `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Winlogon\Shell` = `"...\Nimbus.exe" --shell`.
   This affects **your account only**; other accounts and Safe Mode keep Explorer. If the build fails, nothing is changed.
2. Sign out and back in. Windows starts Ember instead of the taskbar and desktop.

**Getting back to Explorer**
- In Ember: *Settings › Shell › Return to the Windows desktop* (now and at every sign-in).
- Any time: **Ctrl+Alt+Shift+E** opens a File Explorer window.
- From a black screen: **Ctrl+Shift+Esc** → *Run new task* → `%USERPROFILE%\nimbus-restore.cmd`.
- Safe Mode always uses Explorer.

**Safety built in**
- With `--shell`, Ember runs as a small watchdog plus the real UI. If the UI crashes it is restarted; 3 crashes within 2 minutes → Explorer is started instead.
- If WebView2 can't start, Ember starts Explorer rather than leaving a black screen.
- Alt+F4 doesn't close Ember in shell mode. *Power › Sign out / Restart / Shut down* work as usual.

**Not handled yet in shell mode:** apps in your Startup folder and Run keys (Explorer normally starts them), the system tray, and notifications from other apps.

## Updates

Ember updates itself from GitHub Releases of **Dragongamer9/hearth-os**.

- *Settings › Updates* shows the version, checks GitHub (also automatically at startup and every 6 hours), downloads the new build, verifies its SHA-256, and *Restart Ember to update* swaps it in.
- The swap happens after Ember exits (`%LOCALAPPDATA%\Nimbus\updates\apply-update.cmd`): `app` → `app.old`, new build → `app`, then Ember restarts (as the shell if it was the shell).
- If the new version crashes 3 times within 2 minutes, the watchdog swaps `app.old` back automatically. After a minute of running fine, the update counts as good.
- Only the installed copy (`%LOCALAPPDATA%\Nimbus\app`) updates; a development build from this folder doesn't.

**Publishing an update:** raise `<Version>` in `hosts/windows/Nimbus.Host/Nimbus.Host.csproj` (e.g. 0.3.0 → 0.3.1) and push to `main`. The *Build and release* workflow (`.github/workflows/release.yml`) builds on Windows and Linux, packages `Nimbus-win-x64.zip` and `Nimbus-linux-x86_64.AppImage` (each with a `.sha256`), creates tag `v0.3.1` and publishes the release; the commit message becomes the release notes. Every Ember PC then sees it. Pushes that don't change the version are only built, as a compile check.

## Controls

Ember works with a controller, a keyboard, or a mouse/touch, and switches its on-screen prompts to whatever you used last (*Settings › Controls* can pin them). Prompts in the bottom bar are clickable, and the top bar has Back, Search, Quick Access and Notifications buttons.

| Action | Controller | Keyboard |
|---|---|---|
| Move | D-pad / left stick | Arrows or W A S D |
| Select | A | Enter / Space |
| Back | B | Esc / Backspace |
| Main menu | Start (☰) | M |
| Quick Access | View (⧉) | Q |
| Search | Y | / |
| Switch tabs | LB / RB | Tab / Shift+Tab, [ ] |
| Favorite | X | F |
| Home | | Home |
| Controls page | | F1 |

# Nimbus Shell

A controller-first replacement shell for Windows 11, modeled on the SteamOS gamepad UI.
Milestone 1: clickable UI prototype + the native host it runs in.

```
Nimbus/
  ui/index.html              Whole UI (HTML/CSS/JS, mock library). Runs in any browser.
  host/Nimbus.Host/          .NET 8 WinForms + WebView2 host (fullscreen, bridge, escape hatch)
    SystemBridge.cs          Request/response API the settings pages call
    Services/                Wi-Fi, Bluetooth, radios, audio, brightness, power
  tools/restore-explorer.ps1 Puts explorer.exe back if the shell ever breaks
```

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
cd host\Nimbus.Host
dotnet run -- --windowed      # dev: normal window
dotnet run                    # fullscreen, like the real shell
```

The UI detects the host via `window.chrome.webview` and sends messages to it:

| Message | Host does today |
|---|---|
| `launch` (steam) | `steam://rungameid/<appId>` |
| `launch` (manual) | starts the .exe and reports `gameExited` |
| `power` | sleep / restart / shut down / start `explorer.exe` |
| `exitGame` | closes a tracked manual game |
| `call` | system settings API, see below |
| `setting`, `install`, `ready` | stubbed; wired in milestone 2 |

**Escape hatch:** `Ctrl+Alt+Shift+E` always starts File Explorer while Nimbus runs. `Ctrl+Shift+Esc` (Task Manager) is always available from Windows.

See **Using Nimbus as your shell** below.

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
- Wi-Fi scanning needs *Settings › Privacy & security › Location › Let desktop apps access your location* (Windows 11 24H2+). Nimbus shows that message if access is denied.
- Bluetooth devices can't be force-connected or disconnected through public APIs; Nimbus pairs and removes them, and paired devices auto-connect when switched on. PIN-match pairing asks you to confirm in the UI.
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

Nimbus's own data (favorites, hidden games, manual games, play stats, enabled stores) is in `%LOCALAPPDATA%\Nimbus\library.json`.
Art is served to the UI from `https://nimbus-art.local/<id>/<cover|hero|logo|wide>`.

**Running games:** after launch, Nimbus watches for any process whose exe is inside the game's folder. That is how it knows the game started (even through Steam/Epic) and when it closed. *Stop* asks the game to close, then force-closes it after ~6 s. When the game exits, Nimbus records play time and comes back to the front. Verified on a real PC with a Steam game: started detected after Steam's hand-off, closed in ~1 s.

`tools/TestGame` + `build-testgame.cmd` build a tiny stand-in game for testing on PCs without any installed games.

## Owned games, installs and Steam in the background

- **Owned Steam games** come from Steam's binary caches: `appcache/packageinfo.vdf` (packages this account has licenses for, minus package 0) joined with `appcache/appinfo.vdf` (name, type, OS, release state, art paths). Only released Windows games are listed. No Web API key or sign-in needed. Parser: `Library/SteamCache.cs` (appinfo v27–v29, packageinfo v27–v28).
- **Art for uninstalled games** uses the exact CDN paths Steam records (`store_item_assets/steam/apps/<id>/<hash>/library_capsule.jpg` etc.). If an image fails, the UI tries the next one, then a gradient.
- **Install** runs `steam://install/<id>`. Steam always shows its small Install dialog; that confirmation can't be skipped.
- **Downloads**: state and progress come from each `appmanifest_*.acf` (`StateFlags`, `BytesDownloaded/BytesToDownload`). Steam only writes those at checkpoints, so between checkpoints Nimbus estimates progress from the PC's received network bytes and shows the speed. The UI polls `library.downloads` every 3 s while something is installing, reloads the library when a download finishes, and shows "ready to play".
- **Steam in the background** (*Settings › Library sources*, on by default): Nimbus starts `steam.exe -silent` at startup and before Steam launches/installs, and hides Steam's main window (title exactly "Steam") for a short while afterwards. Other Steam windows, like the Install dialog, are left alone, and the tray icon still opens Steam.

Debug helpers: `bridge.log` next to `Nimbus.exe` logs every bridge call and UI image failures; **Ctrl+Shift+D** logs the focused game's art state. `rebuild-and-run.cmd` builds and relaunches in one click.

## Install on any PC

Open PowerShell and run:

```powershell
irm https://raw.githubusercontent.com/Dragongamer9/nimbus/main/tools/install-nimbus.ps1 | iex
```

or double-click `install-nimbus.cmd` in this folder. It installs the .NET 8 Desktop Runtime if missing, downloads the latest release, checks its SHA-256, installs it to `%LOCALAPPDATA%\Nimbus\app`, saves the undo script, and makes Nimbus your shell (add `-NoShell` to skip that). `install-shell.cmd` does the same from a local build, for development.

## Using Nimbus as your shell

1. Run **`install-shell.cmd`**. It publishes a Release build to `%LOCALAPPDATA%\Nimbus\app`, copies the undo script to `%USERPROFILE%\nimbus-restore.cmd`, and sets
   `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Winlogon\Shell` = `"...\Nimbus.exe" --shell`.
   This affects **your account only**; other accounts and Safe Mode keep Explorer. If the build fails, nothing is changed.
2. Sign out and back in. Windows starts Nimbus instead of the taskbar and desktop.

**Getting back to Explorer**
- In Nimbus: *Settings › Shell › Return to the Windows desktop* (now and at every sign-in).
- Any time: **Ctrl+Alt+Shift+E** opens a File Explorer window.
- From a black screen: **Ctrl+Shift+Esc** → *Run new task* → `%USERPROFILE%\nimbus-restore.cmd`.
- Safe Mode always uses Explorer.

**Safety built in**
- With `--shell`, Nimbus runs as a small watchdog plus the real UI. If the UI crashes it is restarted; 3 crashes within 2 minutes → Explorer is started instead.
- If WebView2 can't start, Nimbus starts Explorer rather than leaving a black screen.
- Alt+F4 doesn't close Nimbus in shell mode. *Power › Sign out / Restart / Shut down* work as usual.

**Not handled yet in shell mode:** apps in your Startup folder and Run keys (Explorer normally starts them), the system tray, and notifications from other apps.

## Updates

Nimbus updates itself from GitHub Releases of **Dragongamer9/nimbus**.

- *Settings › Updates* shows the version, checks GitHub (also automatically at startup and every 6 hours), downloads the new build, verifies its SHA-256, and *Restart Nimbus to update* swaps it in.
- The swap happens after Nimbus exits (`%LOCALAPPDATA%\Nimbus\updates\apply-update.cmd`): `app` → `app.old`, new build → `app`, then Nimbus restarts (as the shell if it was the shell).
- If the new version crashes 3 times within 2 minutes, the watchdog swaps `app.old` back automatically. After a minute of running fine, the update counts as good.
- Only the installed copy (`%LOCALAPPDATA%\Nimbus\app`) updates; a development build from this folder doesn't.

**Publishing an update:** raise `<Version>` in `host/Nimbus.Host/Nimbus.Host.csproj` (e.g. 0.3.0 → 0.3.1) and push to `main`. The *Build and release* workflow (`.github/workflows/release.yml`) builds on Windows, packages `Nimbus-win-x64.zip` + `.sha256`, creates tag `v0.3.1` and publishes the release; the commit message becomes the release notes. Every Nimbus PC then sees it. Pushes that don't change the version are only built, as a compile check.

## Controls

Nimbus works with a controller, a keyboard, or a mouse/touch, and switches its on-screen prompts to whatever you used last (*Settings › Controls* can pin them). Prompts in the bottom bar are clickable, and the top bar has Back, Search, Quick Access and Notifications buttons.

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

## License

No license: all rights reserved. The source is public to read, but it may not be copied, modified or redistributed without permission.

## Next milestones

1. ~~Real library~~ ✅ done (incl. owned Steam games, installs, download progress). Still to do: art for Epic/GOG/manual games (e.g. SteamGridDB); owned-but-uninstalled games for Epic/GOG/Xbox (needs their sign-in APIs).
2. ~~Shell mode~~ ✅ first version. Still to do: run Startup/Run-key apps, a tray replacement, other apps' notifications.
3. **More system controls:** external-monitor brightness (DDC/CI), resolution/refresh switching, TDP via vendor tools, volume-change notifications.
4. **Guide button overlay:** global gamepad hook so the Quick Access panel opens over a running game.

# Hearth (Hearth OS for Linux)

Hearth is the Linux side of Hearth OS: the same UI (`../../ui`), hosted in Electron and run inside **gamescope**, the compositor SteamOS uses
for Gaming Mode. Hearth takes Steam's place in that setup: you pick "Hearth" on the login screen and get a
fullscreen, controller-first library of your Steam, Epic, GOG and manual games, plus Xbox Cloud Gaming.

> Status: first version. The scanners, game tracking, bridge and session watchdog have tests (`npm test`);
> the Electron window itself still needs trying on real hardware.

```
hosts/linux/
  src/main.js            Electron app: fullscreen window, nimbus:// and nimbus-art:// schemes, hides itself while a game runs
  src/preload.js         window.nimbusHost: the same postMessage/addEventListener channel WebView2 gives the UI on Windows
  src/bridge.js          The bridge contract (../../shared/bridge.md)
  src/system.js          Wi-Fi (nmcli), Bluetooth (bluetoothctl), audio (wpctl), brightness (brightnessctl), power
  src/library/           Steam, Heroic (Epic + GOG), Xbox Cloud Gaming, manual games; /proc game tracking
  src/stream.js          Xbox Cloud Gaming window
  src/guide.js           Guide button over games (reads /dev/input directly)
  src/updates.js         Self-update of the AppImage from GitHub Releases
  session/               Login-screen session, gamescope launcher and crash watchdog
  test/                  node --test, no Electron needed
```

## Try it

Needs Node 22.

```sh
cd hosts/linux
npm install
npm start                 # windowed, on your normal desktop
npm test                  # scanner, tracking and bridge tests
npm run dist              # builds the AppImage in dist/
```

F12 opens the developer tools when running from source.

## The Hearth session

On Hearth, the OS (`../../os`), all of this is already installed. On another distro:

*Settings › Shell › Add the Hearth session* (asks for your password once) installs:

- `/usr/share/wayland-sessions/nimbus.desktop`: the "Hearth" entry on the login screen
- `/usr/local/bin/nimbus-session`: starts `gamescope -f -- nimbus-watchdog <Hearth>`
- `/usr/local/lib/nimbus/nimbus-watchdog`: restarts Hearth if it crashes, restarts it after an update,
  rolls back an update that keeps crashing, and ends the session (back to the login screen) rather than leave a black screen

Without the UI: `sudo session/install-session.sh` (or `--remove`), and put the path of the AppImage in `~/.config/nimbus/app-path`.
Extra gamescope options go in `~/.config/nimbus/session.conf`, for example `NIMBUS_GAMESCOPE_ARGS="-W 1280 -H 800 --adaptive-sync"`.

Needs: `gamescope`, and for the settings pages NetworkManager (`nmcli`), BlueZ (`bluetoothctl`), PipeWire (`wpctl`),
`brightnessctl` and `power-profiles-daemon`. Anything missing just shows as unavailable.

**Getting out:** *Power › Switch to desktop* ends the session; pick your usual desktop on the login screen.
If Hearth is stuck: Ctrl+Alt+F3, sign in, `loginctl terminate-user $USER`.

## Desktop Mode

*Power › Switch to desktop* (or Settings › Shell) switches the Hearth session to Desktop Mode without logging out:
normal windows on the **labwc** window manager, with Hearth's own taskbar, app launcher, Quick Access, notifications,
on-screen keyboard and wallpaper, made with **Quickshell** (`desktop/shell/`). The **Game Mode** button on the
taskbar, Super+G, or Game Mode in Quick Access or the Hearth menu goes back.

**With a controller** (`desktop/shell/hearth-pad`), the whole desktop works without a mouse or keyboard:

| Control | Does |
|---|---|
| Right stick (press: precise) | Move the pointer |
| Left stick | Scroll |
| A / RT | Click (hold to drag) |
| X / LT | Right-click |
| B | Back (Esc) |
| Y | On-screen keyboard (D-pad picks keys, A types, X ⌫, LB shift, ☰ Enter, B closes) |
| D-pad | Arrow keys |
| LB / RB | Switch windows |
| Menu ☰ | Apps |
| Guide | Hearth menu: point either stick at Apps, Keyboard, Quick Access, Close window, Game Mode, Screenshot, Controller mouse on/off, Controls |
| View + D-pad / View + B | Snap left/right, maximize, minimize / close the window |
| View | Show the controls |

It pauses while a game is fullscreen (Guide still opens the menu) and can be turned off in Quick Access.
Needs `python3-evdev` and access to `/dev/uinput` (Hearth, the OS, has both; elsewhere Steam's udev rules usually
give it). If Steam is running with its own desktop controller layout, turn one of them off.

**Touch:** swipe in from the right edge for Quick Access, the left edge for the keyboard, the bottom edge for apps.
The taskbar moves down the left side on an upright screen, and small sharp screens (handhelds) start at 125–150%
scale until you pick one in Settings › Display.

**Look:** warm, firelit colors, a wallpaper with a slowly breathing fire glow and drifting embers, and
**Night light** in Quick Access (warmer screen from sunset to sunrise, with `wlsunset`).

| Key | Does |
|---|---|
| Super | App launcher (type to search, Enter to open) |
| Super+G | Game Mode |
| Super+Return / Super+E | Terminal / file manager (whichever is installed) |
| Super+Q | Close window |
| Super+Arrows | Maximize, minimize, snap left/right |
| Alt+Tab | Switch windows |

Needs `labwc` and `quickshell` (Fedora: COPR `errornointernet/quickshell`; Arch: `quickshell`). Your wallpaper:
`~/.config/nimbus/wallpaper.jpg`. Your own startup apps: an executable `~/.config/nimbus/autostart`.
Shell log: `$XDG_RUNTIME_DIR/nimbus-shell.log`.

## Display

Settings › Display changes resolution, refresh rate and scale. A new mode is kept only if you confirm it
within 15 seconds; otherwise Hearth switches back, so a mode your screen can't show fixes itself.

| Where Hearth runs | Uses | Scale changes |
|---|---|---|
| Game Mode (gamescope) | `-W -H -r` gamescope options in `~/.config/nimbus/session.conf`; Game Mode restarts to apply | The Hearth interface |
| Desktop Mode (labwc) and other wlroots desktops | `wlr-randr`, saved to `~/.config/nimbus/display.sh` | The whole screen |
| KDE Plasma | `kscreen-doctor` | The whole screen |
| X11 desktops | `xrandr` | The Hearth interface |

## Games

| Source | Where it looks | Launches with | Running detection |
|---|---|---|---|
| Steam (native or Flatpak) | `libraryfolders.vdf` → `appmanifest_*.acf`; play time from `localconfig.vdf`; owned-but-not-installed games from Steam's `appinfo.vdf` / `packageinfo.vdf` | `steam steam://rungameid/<id>` | Steam's `reaper` process with `AppId=<id>`, plus its children |
| Epic | Your Epic account (Library › Stores), through **legendary**; games Heroic installed are found too | Proton via **umu-run**, one Wine prefix per game | Processes in the install folder (Wine paths included) |
| GOG | Your GOG account, GOG's web API plus **gogdl** for downloads; Heroic installs found too | Proton via umu-run | Same |
| Xbox | Your Xbox profile (gamertag, gamerpic, games you've played); the games themselves stream (below) | Xbox Cloud Gaming window | The window |
| Manual | File picker | Programs and AppImages directly; `.desktop` files by their `Exec`; Windows `.exe` with umu-run, or `wine` | Its process and folder |

Epic and GOG games install to `~/Games/Epic` and `~/Games/GOG` (change it on the Stores page), with download
progress, cancel and uninstall, the same as on Windows. legendary, gogdl and umu-launcher are downloaded from
their GitHub releases the first time they're needed and checked against pinned SHA-256 hashes; an `umu-run`
already installed on the system is used instead. Wine prefixes: `~/.local/share/nimbus/prefixes/<store>-<id>`.

When a game starts, Hearth hides its window so the game is in front, and comes back when the game ends.
The **Guide** button during a game brings Hearth back with Quick Access open: Performance (CPU, GPU and video
memory on AMD, RAM, battery), Take a screenshot (Pictures/Screenshots/Hearth), Back to game, Exit game.

Store sign-in windows (Epic, GOG, Microsoft) work with a controller: the sticks move a pointer, A clicks,
Y opens an on-screen keyboard, B cancels. If the Guide button doesn't come through on your controller,
Settings › Controls lists the devices being watched and can open Quick Access on View held for a second.
Not on Linux yet: the controller mouse for the whole desktop (Steam Input's desktop mode does the job meanwhile).

## Xbox Cloud Gaming

Game Pass titles that can stream show up under Library › Xbox with a **Stream** button. They aren't
"installed", so they stay out of Home until you've played or favorited one; the **Xbox Cloud Gaming** tile on
Home opens the whole catalog. Streaming needs Game Pass Ultimate; sign in once with your Xbox account (the
sign-in is kept in its own browser profile). Leave a stream with **View + Menu held for 1.5 s**, the Guide
button, or Ctrl+Shift+Q.

## Data

`~/.local/share/nimbus/`: `library.json` (favorites, hidden, manual games, play time, sources), `bridge.log`,
`session.log`, `xcloud-catalog.json`.

# Couch Co-op (experimental)

Couch Co-op splits one screen between 2 to 4 players. Each player has their own controller and runs their own copy
of the game, so it works even for games that have no split-screen of their own. This is the same idea as
PartyDeck on Linux and Nucleus Co-op on Windows. The copies then join each other the way they would at a LAN
party: over the network, inside your own PC.

The backend is `/usr/libexec/nimbus/hearth-coop`. It is one Python 3 script with no dependencies outside the
standard library. Hearth's Game Mode UI calls it through the Rust bridge. Tests are in
`hosts/linux/test/test_hearth_coop.py`.

## What a game needs

- **LAN play, or Steam multiplayer.** The copies have to find each other over the local network. Games with
  LAN or direct-IP play work as they are. Steam games that use Steam's matchmaking work through the Goldberg
  Steam emulator (see below).
- **No anti-cheat.** EasyAntiCheat, BattlEye, EA Javelin, Vanguard and similar systems refuse to run several
  copies at once, and they refuse the emulator. Hearth detects them in the game folder and says plainly that the
  game can't be played in Couch Co-op.
- **No Steam DRM wrapper (SteamStub).** A game whose exe is wrapped in Steam's protection only starts through
  Steam. Hearth detects the wrapper (a `.bind` section in the exe) and refuses the game.
- **No online-only services.** Epic Online Services and GOG Galaxy multiplayer won't connect the copies.

## How it works

```
outer gamescope (Game Mode)       shows one fullscreen window:
└── labwc (nested, -C $XDG_RUNTIME_DIR/hearth-coop)
    ├── autostart → hearth-coop _container    starts the players, waits, cleans up
    ├── bwrap (player 1) → gamescope -W w -H h → [umu-run] game     window "hearth-coop-p1" at rect 1
    ├── bwrap (player 2) → gamescope ...                             window "hearth-coop-p2" at rect 2
    └── ...
```

1. **Container.** In Game Mode, gamescope shows a single window. So Couch Co-op starts a nested **labwc**
   (the same compositor as Desktop Mode) as that window, using the X11 backend. Its output is resized to the
   screen with `wlr-randr --custom-mode`. The config is generated per session, with no decorations, no gaps and
   no focus-follows-mouse. **Super+Esc** quits the whole session.
2. **One gamescope per player.** Each copy runs inside its own nested `gamescope --backend sdl -W w -H h -w w -h h`,
   so the game renders at exactly that player's part of the screen. The game never sees the real screen size.
3. **Placing the windows.** labwc's window rules place a window when it opens. Each player's gamescope gets its
   own app id, `hearth-coop-p<seat>`, set through `SDL_APP_ID`, `SDL_VIDEO_WAYLAND_WMCLASS` and
   `SDL_VIDEO_X11_WMCLASS`, with `SDL_VIDEODRIVER=wayland`. A rule for each id runs `MoveTo` and `ResizeTo`.
   There is also a fallback for gamescope builds that ignore the app id. The players start one at a time, a few
   seconds apart (`--stagger`). Before each start, a `gamescope*` rule is rewritten to that player's rectangle and
   labwc is told to reload (SIGHUP). Rules only apply when a window opens, so windows that are already placed
   stay where they are. The variables that steer the nested gamescope are removed again for the game itself
   (`env -u ...`), so the game opens inside gamescope rather than directly on labwc.
4. **Keeping controllers apart.** Each copy runs in **bubblewrap**:
   - The whole host filesystem is visible, but `/dev` is a fresh minimal one. Only the GPU, sound, ntsync and
     dma-buf devices come back in, plus **only that player's controller**: its `eventN`, its `jsN`, the
     controller's other evdev nodes (DualSense motion and touchpad), and its `hidraw` node.
   - `/run/udev` is an empty tmpfs, so libudev has nothing to report about the other devices.
   - `SDL_JOYSTICK_DISABLE_UDEV=1` is SDL's own switch (SDL2 and SDL3) for scanning `/dev/input` instead of
     asking udev. SDL already falls back like this in sandboxes. Proton's winebus uses SDL too, so Windows
     games get the same behaviour.
   - `--unshare-pid --die-with-parent`: when a copy is stopped, everything it started stops with it, including
     Wine servers and pressure-vessel.
   - Keyboard and mouse go to one player: seat 1 by default (`--keyboard-seat N`, 0 for nobody), or the seat
     given the `kbm` pad. That seat also gets the keyboard and mouse evdev nodes, and its window opens last so
     labwc gives it keyboard focus. Every other player's input comes from their own controller through evdev,
     so window focus doesn't matter to them.
5. **Separate data per player** (under `~/.local/share/nimbus/coop/players/<seat>/`):
   - `home/` is the player's own `HOME` and `XDG_*_HOME`. Shared tools are symlinked back to the real home:
     Steam, umu, Proton, Vulkan layers, MangoHud, fonts and icons.
   - `prefixes/<game>/` is the player's own Wine prefix for Windows games, which holds their saves and settings.
   - `games/<game>/` is the player's view of the game folder:
     - With **fuse-overlayfs** (the preferred option), the real install is the read-only lower layer. Anything
       the game writes goes to `upper/`, and the game runs from `merged/`.
     - Without fuse-overlayfs, the view is a **symlink copy** (real folders, symlinked files). New files land in
       the view. Existing files point at the original, which is read-only inside the sandbox, so a game that
       rewrites its own files can fail. For native games, the exe itself is copied, because the game finds its
       files through `/proc/self/exe`.
   - The real install is always mounted read-only inside the sandbox. Steam's `userdata/` and `config/` are too.
6. **Windows games** run with `umu-run` (Bazzite's, or Hearth's own copy in `~/.local/share/nimbus/tools`).
   They use the newest GE-Proton or UMU-Proton found (otherwise Valve's Proton), with `UMU_RUNTIME_UPDATE=0` so
   that four copies don't all try to update the runtime at once. Steam games get `GAMEID=umu-<appid>` for
   protonfixes.
7. **Steam games** get **gbe_fork** (Goldberg). In each player's view only, every `steam_api.dll`,
   `steam_api64.dll` and `libsteam_api.so` is replaced with the emulator's build for the same architecture
   (read from the PE or ELF header). A `steam_settings/` folder is written next to each one:
   - `steam_appid.txt`
   - `configs.user.ini`: `account_name` = the player's name, and `account_steamid` = 76561197960265728 + 1000000 + seat,
     which is stable per seat so saves stay with the seat
   - `configs.main.ini`: LAN mode, `listen_port=47584`
   - `custom_broadcasts.txt` = 127.0.0.1
   - the older `force_*.txt` files, for older builds

   The emulator is downloaded once, from a pinned gbe_fork release, into `~/.local/share/nimbus/coop/gbe/<tag>/`.
   The Windows build is a `.7z`, so it is unpacked with `7z`, `7za`, `7zz` or `bsdtar`. When `GBE_SHA256` is
   filled in, the download is checked against it.
8. **Session.** `$XDG_RUNTIME_DIR/hearth-coop.json` holds the plan and the process ids. The outer `start` only
   writes it before labwc starts. After that, only the container writes it. When every player has quit, the
   container stops what's left, unmounts the overlays, closes labwc and deletes the file. `stop` does the same
   from outside: SIGTERM to each copy's process group, SIGKILL after 5 s, then the container and labwc are
   stopped and the overlays unmounted. Per-player data (saves, the upper layers) is kept.

## Controllers and InputPlumber

`pads` reads `/proc/bus/input/devices` and `/sys/class/input`. A gamepad is a device with absolute axes and a
gamepad or joystick button, or with a `js` handler. Motion sensors and touchpads are left out. The InputPlumber
heuristic works like this:

- **InputPlumber's virtual controllers count as pads.** A gamepad is treated as InputPlumber's virtual
  controller (kind `inputplumber`) when all of these are true: it is under `/devices/virtual/`, it has no
  `Phys`, InputPlumber is running (an `inputplumber` process), and it is named like one of InputPlumber's
  targets ("Microsoft X-Box 360 pad", Xbox, DualSense, Steam Deck, "InputPlumber ..."). This is the device the
  game should get.
- **Physical pads InputPlumber manages are hidden.** InputPlumber hides the physical controllers it manages: it
  removes the node from `/dev/input`, makes it unreadable, or moves it to `/dev/inputplumber/sources`. While
  InputPlumber runs, such a physical pad is listed under `hidden` with `managedBy: inputplumber` and is never
  handed to a game, because its input already reaches the virtual pad. On a handheld, the built-in controller
  therefore shows up as the InputPlumber virtual pad.
- **Other physical pads are used as they are** (kind `physical`). Bluetooth LE pads sit under
  `/devices/virtual/misc/uhid` but have a `Phys`, so they count as physical. Other virtual pads, such as Steam
  Input's, are kind `virtual`.

`wait-button` supports a "Player 2, press A" screen. It listens on every usable pad and reports the first one
where a face or trigger button goes down.

## Limits

- **Performance.** Every player runs a full copy of the game, plus a nested gamescope each. On handhelds (Steam
  Deck, Ally, Legion Go), 2 players is realistic for light games and 3 to 4 only for very light ones. RAM is
  usually the limit. Each copy renders at its own part of the screen only.
- **The first start is slow.** Each player gets a fresh Wine prefix on the first start of each game.
- **Native Steam games** run outside Steam's runtime. Some need libraries that only Steam's runtime provides.
- **Steam games must be owned and installed through your Steam.** Couch Co-op only uses games that are installed
  in your Steam library.
- **Games that rewrite files in their own folder** need fuse-overlayfs.
- **Games that ignore `$HOME`** and look up your real home folder share one save folder between all players.
- **Mouse clicks** in another player's window move keyboard focus there.
- Untested on real hardware so far: see the bottom of this file.

## Legal

The Goldberg emulator (gbe_fork) is used only so that **your own copies of games you own** can talk to each
other over a local network inside your PC. It replaces Steam's network layer for those copies and nothing else.
It never unlocks content, never touches your Steam installation or account, and never modifies the real game
files. It changes only the player's own copy-on-write view. Couch Co-op refuses games with anti-cheat and with
Steam's DRM wrapper, and doesn't try to get around them.

## Command line

```
hearth-coop check | pads | wait-button [--timeout S] [--exclude id,...] | games | profiles
hearth-coop profile-set --seat N --name NAME
hearth-coop gbe-fetch [--kind linux|windows]
hearth-coop start --game ID --players N --pads id1,id2[,...] [--layout auto|vertical|horizontal|grid]
                  [--width W --height H] [--keyboard-seat N] [--exe PATH] [--stagger S] [--dry-run]
hearth-coop status | stop
```

- Output is `progress: N text` lines, then one JSON line.
- Errors are `error: <plain sentence>` and exit with status 1.
- `vertical` = side by side, `horizontal` = one above the other.
- `auto` for 2 players means side by side when the screen is 16:10 or wider, otherwise stacked. 3 or 4 players
  always use a 2x2 grid; with 3, one corner stays empty.
- `--pads` takes ids from `pads`. The id `kbm` gives a player the keyboard and mouse instead of a controller.
- `--dry-run` prints the whole plan (commands and environment) without starting anything.

## Troubleshooting

- labwc log: `$XDG_RUNTIME_DIR/hearth-coop/labwc.log`
- Each player's last run: `~/.local/share/nimbus/coop/players/<seat>/last-run.log`
- `hearth-coop start ... --dry-run` shows the exact bwrap, gamescope and game commands for each player.
- Super+Esc quits the session.

## Not yet tested on real hardware

- Whether Bazzite's gamescope honours the SDL app id variables with `--backend sdl`. If it doesn't, the
  staggered fallback rule does the placing.
- labwc's `MoveTo`, `ResizeTo` and `SetDecorations` in window rules (needs labwc 0.7.2 or newer), and
  `wlr-randr --custom-mode` on the X11 backend inside gamescope.
- Several gbe_fork copies on the same port of one PC finding each other through broadcast or 127.0.0.1.
- pressure-vessel (umu) inside bubblewrap with the filtered `/dev`.
- The gbe_fork release tag and asset names (`GBE_TAG`, `GBE_ASSETS`), which need checking against the releases
  page before shipping.

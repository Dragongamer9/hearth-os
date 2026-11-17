# Screens in Desktop Mode (1.14)

Mantel runs on labwc or, from 1.14, on KWin. Everything about screens goes through one helper,
`hosts/linux/desktop/shell/hearth-screens`, so Settings and the other helpers don't care which one is running.

| | labwc | KWin (`HEARTH_WM=kwin`) |
|---|---|---|
| Resolution, refresh rate, scale, position, on/off | wlr-randr | kscreen-doctor (`libkscreen-bin`) |
| Rotation | wlr-randr `--transform` | kscreen-doctor |
| HDR on the desktop, its SDR brightness, wide colour | "Needs Mantel on KWin" | kscreen-doctor |
| VRR / FreeSync (Never / Automatic / Always) | "Needs Mantel on KWin" | kscreen-doctor |
| Colour profile (none, the screen's own, an ICC file) | "Needs Mantel on KWin" | kscreen-doctor |
| Allow tearing in full-screen games | "Needs Mantel on KWin" | kwinrc `[Compositing] AllowTearing` |
| Mirroring | not possible (shown, greyed out) | not possible in KWin 6.3 (shown, greyed out) |
| Night light (on/off, how warm, evenings or all the time, from/until) | wlsunset (the shell starts it) | KWin's Night Light (kwinrc `[NightColor]`) |
| Screen off when idle | wlopm (`hearth-screen-power`) | KWin's DPMS: `kscreen-doctor --dpms` (`hearth-screen-power`) |
| Screenshots | grim | KWin's `org.kde.KWin.ScreenShot2` (`hearth-screenshot`) |

## What's saved, and where

- **Screens**: `~/.config/nimbus/screens.json` is the one source of truth, written only when you press Keep (or for
  changes that need no Keep, like SDR brightness or a colour profile), through `hearth_settings_io` (atomic, `.bak`,
  merged per screen, a damaged file restored from `.bak` and never replaced with defaults). Reading the screens never
  writes anything.
- `~/.config/nimbus/display.sh` (what labwc's autostart has always run) is kept in step with it: the same lines as
  before, other screens' lines kept. So nothing set on labwc is lost on KWin, and going back to labwc keeps everything.
  If something else (the Hearth app) writes a newer `display.sh`, its lines count.
- Settings only KWin has (HDR, VRR, colour profile) stay in `screens.json` when you change something on labwc.
- **Night light** is Settings' own (`desktop.json`: `nightLight`, `nightTemp`, `nightMode`, `nightFrom`, `nightTo`);
  under KWin the shell copies it into KWin's Night Light whenever it changes.
- **Tearing** is in `screens.json` (`tearing`) and copied into kwinrc.

## Keep or go back

A change that could leave a screen dark (resolution, refresh rate, scale, position, on/off, rotation, HDR, wide
colour, VRR) waits 15 seconds for Keep. Without it the screen goes back by itself and the page says so.

## At Desktop Mode's start

The session runs `hearth-screens restore` under KWin. It puts back only what differs from what KWin already shows.
A saved mode the screen doesn't list now is left out (and you're told, once). It never turns every screen off. A
small sharp screen with no saved scale gets 125% or 150%, the same rule labwc's autostart uses. Then it sets KWin's
Night Light and tearing.

## 1.14's KDE rows (Mantel on KWin)

| Row on Settings › Display | Where it lives | Keep needed? |
|---|---|---|
| Make this the main screen | KWin's priority (`kscreen-doctor output.N.priority.1`), every screen's kept in screens.json | no |
| Colour intensity (HDR on) | KWin's SDR gamut wideness (`sdrGamut.0-100`), screens.json `sdrGamut` | no |
| Colour accuracy | KWin's colour power trade-off (`colorPowerTradeoff.preferEfficiency/preferAccuracy`), screens.json `colorPower` | no |
| Calibrate HDR (HDR on, a screen KWin knows the brightness of) | `HdrCalibrate.qml`: three test pictures; KWin's `maxBrightnessOverride`, `maxAverageBrightnessOverride`, `minBrightnessOverride`; screens.json `hdrCal {peak, avg, min}` | yes (Finish) |
| Extended Dynamic Range (newer KWin only; hidden on 6.3) | `edrPolicy`, screens.json `edr` | yes |
| Overscan, RGB range (screens that have them) | `overscan.0-100`, `rgbrange.automatic/full/limited`, screens.json | yes |
| Scaling X11 apps | kdeglobals `[KScreen] XwaylandClientsScale` (the same key as Gaming › Full-screen games & X11 apps) | no |
| Sticky edge between screens, sticky corners | kwinrc `[EdgeBarrier] EdgeBarrier`, `CornerBarrier` (also on Screen edges) | no |
| Virtual desktops: every screen / main screen only | screens.json `desktopsMainOnly`; `hearth-main-desktops.js` (a KWin script that keeps the other screens' windows on every desktop and puts back only those when turned off). "Each its own" needs a newer KWin | no |
| Brightness of an external screen | DDC/CI through `ddcutil` (`hearth-screens ddc`, `ddc-set`), or KWin's own control where a newer KWin has it; nothing saved (the screen keeps it) | no |
| Auto-brightness (a light sensor only) | screens.json `autoBrightness`; the shell runs `hearth-screens autobright` while it's on | no |
| Night light: sunset to sunrise, fade | desktop.json `nightMode: "location"`, `nightPlace`, `nightLat`, `nightLon` (your town, looked up by name), `nightFade` (minutes); KWin's `Mode=Location` with the fixed place (never its network lookup), wlsunset's `-l/-L` and `-d` | no |

Hidden because KWin 6.3 (Debian 13) has no way to set them: colour depth (bits per colour), reduce red tint, each
screen with its own desktops, mirroring.

**Super+P** (`ScreenSwitcher.qml`, both compositors): This screen only, Duplicate (says why it can't), Extend, Second
screen only. `hearth-screens plan MODE` works out the changes; the switcher applies them and waits 15 s for Keep, going
back by itself otherwise.

Preview (CI pictures only): `HEARTH_SCREENS_FIXTURE=<folder>` with `kscreen.json`, `kscreen.txt` and `ddc.json`
(`hosts/linux/test/fixtures/screens-preview`): the screens come from there and nothing is sent to KWin.

## Screenshots under KWin

KWin answers `org.kde.KWin.ScreenShot2` only for a program whose `.desktop` file lists that interface in
`X-KDE-DBUS-Restricted-Interfaces`. KWin finds that file by the program's executable. A Python script's executable
is Python itself, so trusting it would trust every Python program. Instead `hearth-screens` asks the small C helper
`/usr/libexec/hearth/hearth-kwin-cast` (`shot screen NAME | active-window | active-screen | workspace | area X Y W H
[CURSOR 0|1 [FULL 0|1]]`, full resolution unless FULL is 0), which `hearth-kwin-cast.desktop` trusts. The helper
answers only when its parent is `hearth-portal` or `hearth-screens`, root's files in a folder only root can change,
and `hearth-screens` gets only screenshots and the list of screens from it (no streams, remote input or colour
picking). `/usr/bin/hearth-screenshot` takes grim's arguments (`[-g "X,Y WxH"] [-o SCREEN] [-c] FILE|-`): it is grim
under labwc, and under KWin it runs `hearth-screens shot`. (Before this, 1.14's development builds trusted a copy of
Python, `hearth-shot-python`, so any Python script run with it could take screenshots; the image removes it.) `open.sh` (Print Screen, Quick Access, the
controller's screenshot), Hearth Paint, the Hearth app and hearth-core all use it.

## For other parts of Hearth

- `hearth-screens get` gives the screens in `wlr-randr --json`'s shape (with more fields) on both: use it instead of
  wlr-randr.
- `hearth-screen-power off|on|check` instead of wlopm.
- `hearth-screenshot` instead of grim.
- Capture: `$NIMBUS_SHARE/desktop/shell/hearth-screens grab MODE [--screen N] [--geometry X,Y,W,H] [--pointer] --out FILE`.
  MODE is `screen` (`--screen`: a name, or 1 for the first screen that's on; without it the active screen), `all`,
  `window` (KWin's active window; under labwc only with `--geometry`, since labwc can't say which window is active) or
  `area` (`--geometry`). Exit 3: no window to take. Under KWin the picture comes through `hearth-kwin-cast shot`.
- The Hearth app's Settings › Display (hearth-core `display.rs`, `hosts/linux/src/display.js`) goes through
  `hearth-screens get` / `set` in Mantel's session (`HEARTH_WM` labwc or kwin), so it never writes `display.sh` or
  `screens.json` itself there.
- Tests: `hosts/linux/test/test_hearth_screens.py`. CI: Desktop Mode test (labwc) and its `kwin-screens` job (KWin
  with the real kscreen-doctor, Night Light, DPMS, and a screenshot with KWin's permission check on).

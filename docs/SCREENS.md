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

## Screenshots under KWin

KWin answers `org.kde.KWin.ScreenShot2` only for a program whose `.desktop` file lists that interface in
`X-KDE-DBUS-Restricted-Interfaces`. KWin finds that file by the program's executable. A Python script's executable
is Python itself, so trusting it would trust every Python program. Instead the image hard-links Python as
`/usr/libexec/nimbus/hearth-shot-python`, and `/usr/share/applications/hearth-screenshot.desktop` trusts only that.
`/usr/bin/hearth-screenshot` takes grim's arguments (`[-g "X,Y WxH"] [-o SCREEN] [-c] FILE|-`): it is grim under
labwc, and under KWin it runs `hearth-screens shot` through that Python. `open.sh` (Print Screen, Quick Access, the
controller's screenshot), Hearth Paint, the Hearth app and hearth-core all use it.

## For other parts of Hearth

- `hearth-screens get` gives the screens in `wlr-randr --json`'s shape (with more fields) on both: use it instead of
  wlr-randr.
- `hearth-screen-power off|on|check` instead of wlopm.
- `hearth-screenshot` instead of grim.
- Tests: `hosts/linux/test/test_hearth_screens.py`. CI: Desktop Mode test (labwc) and its `kwin-screens` job (KWin
  with the real kscreen-doctor, Night Light, DPMS, and a screenshot with KWin's permission check on).

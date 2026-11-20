# Hearth Classic (labwc + Mantel) and Firehold on labwc on the Fedora edition (1.16)

The owner (Nov 19, 2026): "Also port over the entire labwc as well, have it as a selection. But FIX all the problems it
had. Don't have it as a fallback either." And: "also start a Firehold version that uses labwc so I can test them all.
Fedora is way smoother and faster than Debian was, so we might go back to labwc."

Branch `feature/labwc-fedora`. Status: built, checked in CI (labwc-fedora.yml, and base-anatase.yml's required step);
needs testing on a device (the ROG Ally and the laptop).

## What you get

Settings › System › Your desktop on the Fedora edition offers four desktops to try side by side:

| Choice | What it is |
|---|---|
| KDE Plasma | the default, as before |
| Firehold (Plasma, test) | Firehold's Couch and Desk as Plasma shells (feature/firehold-plasma-1.16) |
| **Mantel (Hearth Classic, labwc)** | Debian's whole Desktop Mode: labwc and the Mantel shell (bar, dock, Quick Access, notifications, Hearth Files, Settings, Task Manager, the on-screen keyboard, hearth-pad) |
| **Firehold on labwc (test)** | Mantel's own Firehold (Firehold.qml: the Couch for the controller, the Desk for the mouse), switched on for that sign-in only |

- One saved line: session.conf's `HEARTH_PLASMA_DESKTOP` (`classic`, `classicfirehold`, `firehold` or `plasma`), written
  only when you pick (hearth-labwc-desktop / hearth-firehold-plasma, through hearth_settings_io: atomic, a .bak, only
  that line). Letters only, because the other readers take `[a-z]*`. Firehold on labwc never writes firehold.json:
  `HEARTH_FIREHOLD_SESSION=1` turns it on for that sign-in (Firehold.qml `sessionPicked`).
- **Never a fallback.** nimbus-session starts labwc only when it's picked (one call, in its marked block before
  `run_plasma`). KDE Plasma or Firehold on Plasma failing stays as it was (Steam Game Mode, recovery).
- **If it doesn't come up** (labwc or Quickshell missing, labwc ending within 15 s, Mantel not up within 45 s, Mantel
  giving up: `hearth-labwc-shell-failed` from labwc/autostart): KDE Plasma for the rest of that sign-in and a notification
  says why; your choice is kept. Twice in a row: KDE Plasma until you pick it again. One try per start: never a loop.
- No login-screen entry of its own (labwc's is moved aside); Back to Game Mode, Game Mode on this PC Off and the rest
  work as in Mantel (nimbus-mode ends labwc through HEARTH_DESKTOP_PID).

## How it's built

- **Quickshell v0.3.2** (Debian's pinned release; Fedora 45 only has a 0.2.1 git snapshot), built in its own stage of
  os-bases/anatase/Containerfile from the same base image, so against the image's own Qt
  (os-bases/anatase/quickshell-build.sh). build.sh refuses one built for another Qt.
- **labwc 0.20.2** from Fedora 45, and Mantel's packages by Fedora's names (os-bases/anatase/labwc-install.sh).
  Required: labwc, Xwayland, Qt's QML, Wayland and SVG modules, jemalloc. Optional (a part shows as unavailable when
  one is missing, as on Debian): qt5compat, image formats, multimedia, qt6ct, grim, slurp, wlr-randr, wlsunset, wlopm,
  wl-clipboard, cliphist, wtype, foot, xdotool, the wlr and GTK portals, libnotify, python3-evdev / -gobject / -dbus,
  Breeze cursors and icons, Adwaita and hicolor icons, Inter and Noto fonts, GNOME Keyring, libsecret, gvfs (SMB,
  FUSE), gocryptfs, fuse3, poppler-utils, tesseract (English), zbar, mate-polkit. The build lists any that Fedora
  doesn't have (`::warning title=labwc on Fedora`).
- If Quickshell or labwc can't be put in, the choice simply isn't offered (no hearth-labwc-desktop) and the image is
  still built: KDE Plasma is unchanged.
- Not ported: Mantel's own file chooser for other apps (the portal stays GTK's / KDE's on this edition, as before).

## Problems it had, and what was done

From the owner's logs (hearth-logs, Nov 10-19: the laptop G2W9-WQTH-FK, F2RL-BJY4-SV, WYGE-6CCV-8M; the Ally
T7CS-FRKM-59), the roadmap (1.13-1.15, 1.14 pulled) and the shell's code.

1. **The shell never went quiet** (1.13: 6-8 % of a core with nothing open on the laptop; 1.14: 13-15 % on the laptop
   and 21-27 % on the Ally, all day). Causes found and fixed:
   - hearth-pad (the controller) woke 125 times a second all day and opened every input device every 1-3 s to look
     for controllers. Now it sleeps until a press while nothing is held (a quarter second at most), and devices that
     aren't controllers are remembered (by their device node) instead of being opened again.
   - hearth-buttons (power and volume keys) woke 10 times a second and reopened every device every 5 s: now asleep
     until a key, the same memory of devices, and a press that wakes the PC is still seen as one.
   - Every bar (one per screen, Bar.qml and Islands.qml, and Firehold's Desk) started nmcli every 10 s and two
     bluetoothctl runs of up to 3 s every 15 s: now NetBtWatch.qml listens (`nmcli monitor`, `gdbus monitor` on BlueZ)
     and reads once after a change; a 2-minute fallback only when a listener can't run.
   - Low battery: a shell, powerprofilesctl (Python) and brightnessctl every 30 s on every PC: now only while on battery
     (and once when you plug in or unplug).
   - Vaults: a shell and often Python every 15 s for everyone: now when the list changes, after a lock or unlock, when
     the bar's menu opens, and every 5 minutes only while you have a vault.
   - Activities checked every window every 2 s with only Main: now every 30 s, only with more than one activity.
   - The controller hints ticked every second all day: one 12-second timer started by the press.
   - The out-of-memory watch: every 60 s (was 20 s), read straight from /proc (was a shell, grep and head each time).
   - **The on-screen keyboard's glow** kept animating while the keyboard was closed (its window hidden, the board
     not), which woke the shell 60 times a second all day: the board now hides with its window. Found in CI by
     tracing the shell at rest (1,291 wake-ups in 20 s on its main thread).
   - The bar's clock pill animated its width every minute (about 20 frames a minute): not while nobody's there.
   - Firehold's Couch read nmcli every 15 s: now NetBtWatch.qml too.
   - The recording and remote-desktop state files (not there until used) were re-read every 2 s and reset the shell's
     state each time: now only when something changed.
   - **The moving wallpaper** (the default fireside) redrew the whole screen every frame behind every window: it now
     holds still while any window is open and after 30 s without input (was 3 minutes, and only behind maximized ones).
2. **Memory** (1.14: 400-670 MB on the laptop, up to 1.15 GB on the Ally): measured in CI at rest (must stay under
   350 MB and not grow); see the numbers below.
3. **Slow Hearth Files**: timed cold and warm in CI (first change on screen within 1.5 s).
4. **The laptop's screen freezes** (Intel PSR, "Atomic commit failed"): 1.14's Screen power saving covers the driver;
   the constant redraws that made it worse are gone (above).
5. **A shell that can't start** sent Debian to Game Mode; here it ends labwc and KDE Plasma opens instead, with why.

CI keeps them from coming back: `hosts/linux/test/test_mantel_rest.py` (no always-on fast timer, no polling bar, the
helpers asleep when idle, the wallpaper's rules), `test_labwc_fedora_session.py` (a choice, never a fallback, no loop,
the pick written only by a pick), and `.github/scripts/labwc-fedora-rest.sh`, which fails when a desktop is over:
the shell at rest 0.25 % of a core on average or 0.75 % in any minute, labwc 0.50 %, the shell over 320 MB or growing
more than 15 MB in 10 minutes, more than 60 frames drawn at rest, any other process over 1 %, more processes at the
end, more than 2 programs started in 2 minutes at rest, Hearth Files over 1.5 s.

## Measurements (CI, Fedora 45, a virtual 1920x1080 screen, software drawing, 10 minutes at rest)

| | Shell at rest | Busiest minute | labwc | Shell memory | Frames drawn | Programs started | Hearth Files |
|---|---|---|---|---|---|---|---|
| Before (round 1, run 44668815171) | 0.59 % | 0.66 % | 0.00 % | 279 → 269 MB | 123 | 6 in 2 min | (not timed) |
| Mantel (Hearth Classic, labwc), run 44678744824 | **0.07 %** | 0.10 % | 0.00 % | 267 → 254 MB | 16 | 0 | 0.28 s cold, 0.24 s warm |
| Firehold on labwc, run 44678744824 | **0.09 %** | 0.10 % | 0.00 % | 252 → 251 MB | 17 | 0 | |

Up in 5-6 s. labwc 0.20.2 (wlroots 0.20.2), Quickshell v0.3.2 on Qt 6.11.2. Round 1 already had the bar, pad and
battery fixes; on the owner's devices 1.14's shell used 13-27 % (with the controller helper's own 125 wake-ups a
second on top, a separate process). The pictures: hearth-os branch ci/labwc-fedora, copied to hearth-workspace
`hearth/assets/1.16-labwc-fedora-shots`.

## Firehold on labwc and the 12 approved 1.16 Firehold mockups

Firehold on labwc is Debian's Firehold as it was (Couch: Home, Library, Apps, Store, Everything open; the Guide; Couch
and Desk settings; the Desk). The 1.16 mockups (docs/mockups/firehold-plasma on feature/firehold-plasma-1.16) are being
built for Firehold on Plasma; Firehold on labwc doesn't have them yet: see the report of the run that pictured it.

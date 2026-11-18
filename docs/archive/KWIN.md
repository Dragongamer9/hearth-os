# Mantel on KWin (1.14): archived in 1.15

Hearth 1.14 moved Mantel, Hearth's desktop, from labwc to KWin. On the owner's devices it went badly: the i3-N305
laptop's first start after the update didn't come up on KWin (the safe start opened labwc), the lock screen's test
didn't finish there, the ROG Xbox Ally started at KWin's own 210% size, Hearth's apps were missing from the menu, and
KWin's Aurorae frames and Overview weren't installed. On Nov 17 the owner said: "if we're just going to abandon KWin then
stop all progress on it and Archive it, Fix the Stability of what hearth is currently running on."

So from 1.15 Desktop Mode is **Mantel on labwc (Hearth Classic)**, as in 1.13, with Firehold (the 2.0 preview, off
unless you turn it on) one step above it: Firehold → labwc. Game Mode is untouched.

## What happens on a device that was on KWin

- `session.conf`'s `HEARTH_DESKTOP_WM=kwin` (or no line, 1.14's default) opens Mantel on labwc. The line itself is
  never rewritten (red flag 9): if KWin comes back from the archive, the choice is still there.
- The first time, Mantel says so once (`desk_notice kwin archived`, DesktopNotice.qml: "Desktop Mode is back on
  labwc"). 1.14's other KWin notices (KWin didn't start, the lock screen is off, your KDE settings) aren't shown any more.
- Settings › System › Your desktop shows labwc as the window manager (no KWin choice). `hearth-your-desktop wm kwin`
  means labwc.
- Firehold's Ctrl+Alt+Backspace leaves for labwc.

## Where the code is

Everything moved keeps its old path under `archive/kwin/`:

| Moved | What it was |
|---|---|
| `hosts/linux/session/nimbus-session-kwin.sh` | nimbus-session's KWin part: `run_kwin`, its wait for Mantel (with 1.15's "still busy loading" wait), the lock guard (`kwin_lock_*`), `wm_env_back`, `settle_kwin` |
| `hosts/linux/desktop/kwin/` | KWin's session files: its autostart, Hearth's kwinrc / kscreenlockerrc defaults, the window-list script (`windows/hearth-windows.js`, `hearth-previews.qml`, `shortcuts.json`) |
| `hosts/linux/desktop/kwin-helpers/` | `hearth-kwin-im` (input method) and `hearth-kwin-cast` (screen sharing, screenshots), and the Plasma/Shell package type |
| `hosts/linux/desktop/kwin-lock/` | Hearth's lock screen for KWin's screen locker (`org.hearth.mantel-lock`) |
| `hosts/linux/desktop/shell/` | `hearth-lockcheck`, the KWin Settings pages (`SetKwinDesktops`, `SetKwinEdges`, `SetKwinEffects`, `SetKwinGames`, `SetKwinRules`, `SetKwinTablet`, `SetKwinTiling`, `SetKwinWindows`, `KwinCalibrate`), `FireholdFrames.qml` (Firehold's frames drawn over KWin's), and `hearth-your-desktop-kwin.py` (its `kwin-overrides` part) |
| `os/system_files/usr/share/` | `applications/hearth-kwin-im.desktop`, `hearth-kwin-cast.desktop`, `kwin/tabbox/org.hearth.switcher` (`kwin/scripts/hearthpad` stays: it is Hearth Plasma's, archived in place) |
| `hosts/linux/test/` | `test_kwin_session.py`, `test_kwin_input.py`, `test_kwin_keyboard.py`, `test_kwin_windows.py`, `test_kwin_settings.py`, `test_kwin_effects_extras.py`, `kwin-windows-mock.js` |
| `.github/workflows/`, `.github/scripts/` | `kwin-test.yml`, `kwin-input.yml`, `kwin-windows.yml`; `kwin-test.sh`, `kwin-ci.sh`, `kwin-debian-lock.sh` (not run: they're outside `.github/workflows`) |
| `tools/` | `e2e-kwin-windows.py` |

desktop-test.yml's `kwin-screens` and the KWin pictures' jobs run only by hand (Run workflow › "Only the KWin screens
job"); clip-capture.yml and firehold-shell.yml build the archived helpers from `archive/kwin` when they're run.

## What stayed in place, and why

These are shared with labwc, Mantel or Firehold; their KWin branches only run with `HEARTH_WM=kwin`, which nothing sets
any more:

- `WinList.qml` (its KWin bridge sits next to Firehold's), `hearth-kwin-bridge` (WinList and Settings › Keyboard name
  it), `Lock.qml`, `Dwell.qml` / `hearth-dwell`, `SnapLayouts.qml`, `ForceClose.qml`, `Pad.qml`, `hearth-firehold`
  (its KWin pictures branch), `firehold-orientation.py`.
- `KwinSettings.qml`, `KwinGate.qml` and `hearth-kwin-settings`: used by Mouse, Mouse buttons and Accessibility
  (`hearth-access` loads hearth-kwin-settings for its KWin part).
- `hearth-screens` and `hearth-screen-power` (their KWin branch uses kscreen-doctor, no longer installed),
  `SetDisplay.qml`'s KWin rows ("Needs Mantel on KWin").
- `theme/hearth_aurorae.py` (hearth-theme writes the Aurorae theme with your look), `hearth_kconfig.py`, `hearth-type`,
  `hearth-portal` and `hearth_screencast.py` (Hearth Files as the file chooser), `kwin_e2e.py` (test_files_extras uses it).
- `docs/KWIN-INPUT.md`, `docs/KWIN-SETTINGS.md`, `docs/KDE-FEATURES-1.14.md`: marked archived at the top.

## Taken out of the image

kwin-wayland (with kwin-common and Aurorae), kglobalacceld, xdg-desktop-portal-kde, milou, kwin-addons, libkscreen-bin
(kscreen-doctor), libkf6config-bin (kwriteconfig6), the KWin helpers from the builder, the lock screen package and
hearth-portals.conf. Firehold is Hearth's own compositor and needs none of them; polkit-kde-agent-1, breeze,
kde-cli-tools, plasma-integration and Kirigami stay (Mantel's password prompts and Hearth's KDE apps).

## Bringing it back

Move the folders back to their paths, put `run_kwin` and its helpers back into nimbus-session's `run_classic`, add
the packages and the image checks back (git history of `os-bases/peppermint/build.sh` and `base-peppermint.yml`
before this change), and the pages back into `settings-pages.js` (then `python3 tools/build-settings-index.py`).

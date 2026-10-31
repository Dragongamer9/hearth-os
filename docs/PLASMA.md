# Hearth Plasma

**Status: Beta Experimental, not tried on a device yet.** Usable entirely with a controller (see "With a controller"). Built into Hearth OS on Anatase and Peppermint, checked by
CI (`plasma-test.yml`, the image checks in `base-anatase.yml` / `base-peppermint.yml`).

Desktop Mode can run one of three desktops. Game Mode is the same with either.

| Desktop | What it is | |
|---|---|---|
| **Hearth Classic** | labwc + Mantel (Quickshell): today's desktop | always installed, the default and the fallback |
| **Hearth Plasma** | KDE Plasma 6 in Hearth's look, with Hearth's widgets | picked in Settings or on the login screen |
| Hearth GNOME | GNOME Shell in Hearth's look, with Hearth's extensions | picked in Settings or on the login screen (docs/GNOME.md) |

Switching keeps your files and settings: both desktops use the same home folder, the same apps and the same Hearth
settings files. Plasma's own settings (`~/.config/kdeglobals`, `plasma-org.kde.plasma.desktop-appletsrc`...) are only
read by Plasma.

## Picking a desktop

- **Game Mode › Settings › Desktop Mode › Desktop** (`ui/index.html` `deskRow`/`deskPick`, bridge `login.desktop` /
  `login.setDesktop`, `hosts/hearth-core/src/admin.rs` `desktop()` / `set_desktop()`).
- **Hearth Classic › Settings › Sign-in › Desktop** (`SetSignin.qml`, through `desktop/plasma/hearth-plasma desktop` /
  `desktop-set`). Hearth GNOME is there too (greyed out where it isn't installed; `SetChoice.qml` options can be `disabled`).
- **The login screen**: "Hearth Classic" and "Hearth Plasma" sessions next to "Hearth"
  (`hosts/linux/session/hearth-classic.desktop`, `hearth-plasma.desktop`; Hearth Plasma is hidden by its `TryExec`
  where Plasma isn't installed). Picking one runs `nimbus-session --desktop=classic|plasma`, which saves the choice,
  so automatic sign-in (always the "Hearth" session) keeps it. "Hearth GNOME" is there the same way (docs/GNOME.md).

All three write the same line: `HEARTH_DESKTOP=classic|plasma|gnome` in `~/.config/nimbus/session.conf`. It's read again
every time Desktop Mode starts, so a change made in Game Mode applies at the next switch.

## Safety: Plasma can never leave the device without a desktop

`nimbus-session` `run_desktop`:

1. Hearth Plasma only when it's picked, `startplasma-wayland`, `kwin_wayland` and `plasmashell` are installed, this
   isn't Recovery's "safe graphics this time", and it's not an extra multi-seat player.
2. `run_plasma` starts `startplasma-wayland` and waits up to `HEARTH_PLASMA_TIMEOUT` (60 s) for KWin and the Plasma
   shell to be running (zombies don't count). No desktop in time, Plasma ending before it came up, or crashing (an
   error within 30 s, without asking for another mode) → `$XDG_RUNTIME_DIR/hearth-plasma-failed` is written, what
   Plasma left is cleaned up (`settle_plasma`) and **Hearth Classic starts instead, for the rest of that sign-in**.
   Settings then says so ("Hearth Plasma didn't start this time").
3. Everything else (Game Mode, the switch, Recovery, the boot guard) is unchanged. Hearth Classic stays installed in
   every image.

`nimbus-mode game|desktop|logout` ends Plasma through its own sign-out (`org.kde.Shutdown.logout`, no questions) and
by force after 8 s if it hangs; `nimbus-mode current` says `desktop` while Plasma runs (`$XDG_RUNTIME_DIR/hearth-plasma.pid`).
Plasma's own Log Out ends the Hearth session, like Hearth Classic's.

The first time Hearth Plasma starts, Plasma settings left by another Plasma session (Anatase's own) that would hide
Hearth's layout are put aside once: `plasma-org.kde.plasma.desktop-appletsrc` and `plasmashellrc` become
`*.before-hearth`, and the `LookAndFeelPackage=` line leaves `kdeglobals` (a copy is kept as
`kdeglobals.before-hearth`). The mark is `~/.config/nimbus/plasma-set-up`.

Tests: `hosts/linux/test/test_hearth_plasma.py` runs the real session with stand-ins (crash, no desktop in time,
not installed, normal start and end, the login screen's choice, the first-start tidy-up).

## The look, part by part

Hearth's Plasma files are system-wide defaults: you can change anything in System Settings and it stays changed.
`nimbus-session` puts `/usr/share/hearth-plasma/xdg` first in `XDG_CONFIG_DIRS` for Hearth Plasma only, so a plain
Plasma session (the base's own) keeps its own look.

| Part | Hearth Classic (Mantel) | Hearth Plasma | Where |
|---|---|---|---|
| Global theme | | look-and-feel package `org.hearth.desktop` (System Settings › Global Theme › Hearth) | `os/system_files/usr/share/plasma/look-and-feel/org.hearth.desktop` |
| Colours | `Theme.qml` palette, ember accent | `Hearth.colors` colour scheme (already used for KDE apps), accent `#ff8a3d` | `hosts/linux/desktop/theme/Hearth.colors`, `hearth-plasma/xdg/kdeglobals` |
| Plasma style (panels, popups) | `Theme.panel`, `Theme.line` | desktop theme `hearth`: Breeze's shapes, Hearth's colours | `usr/share/plasma/desktoptheme/hearth` |
| Taskbar | floating rounded bar along the bottom | floating panel along the bottom, full width, same order | `contents/layouts/org.kde.plasma.desktop-layout.js` |
| Launcher | Hearth mark, Apps / Games, System apps folder, search, power | `org.hearth.launcher` (same lists, same split; Super opens it) | plasmoid + `desktop/plasma/hearth-plasma apps/games` |
| Pinned and open apps | Mantel pins (Beacon, Xodus Nexus, Bazaar) | Icons-only Task Manager with the same default pins | layout script |
| Tray, volume, network, battery, Bluetooth | Mantel's own icons | Plasma's System Tray (plasma-pa, plasma-nm, powerdevil, bluedevil) | |
| Quick Access | `QuickAccess.qml` | `org.hearth.quicksettings`: volume, brightness, Wi-Fi, power mode, Settings, screenshot, Game Mode, sleep | plasmoid + `hearth-plasma quick/quick-set` |
| Hearth Link, Hearth Storage | Settings › Hearth Link | `org.hearth.link`, `org.hearth.storage` in the tray (forward while a pairing code shows) | plasmoids + `desktop/shell/hearth-link-settings` |
| Notifications, calendar | bell + clock | Plasma's notifications, Digital Clock with the date | layout script |
| Game Mode button, Super+G | yes | `org.hearth.gamemode`, Super+G (`hearth-gamemode.desktop`, `kglobalshortcutsrc`) | |
| Show desktop strip | the very end of the bar | Peek at Desktop at the very end | layout script |
| Window frames | labwc: icon left, minimize/maximize/close, rounded, shadows | Breeze in Hearth's colours, same buttons, rounded, large shadows | `hearth-plasma/xdg/kwinrc`, `breezerc` |
| Lock screen | `Lock.qml` | Plasma's lock screen with Hearth's wallpaper and colours; not locked unless asked (like Classic) | `hearth-plasma/xdg/kscreenlockerrc` |
| Wallpaper | Fireside (moving) | Fireside (still: `/usr/share/wallpapers/Hearth-fireside`) | layout script |
| Icons, fonts, cursor | Breeze Dark + Hearth's icon set, sans 10, Breeze cursor | the same | `kdeglobals`, `kcminputrc` |
| Splash | none | none | `ksplashrc` |

Hearth's widgets reuse Hearth Classic's backends instead of new logic: `hearth-link-settings` (Hearth Link's socket),
`hearth-games`, `hearth-power`, Hearth Files' `.desktop` reading (`hearth-files`), wpctl, brightnessctl and nmcli.
They call them through Plasma's `executable` engine (`org.kde.plasma.plasma5support`).

## With a controller

Hearth Plasma stays KDE Plasma (its own launcher, panel, tray, notifications and lock screen, in Hearth's theme) and
can be used entirely with a controller, the same way as Hearth Classic. One helper does it for both desktops:
`hearth-pad --plasma` (`desktop/shell/hearth-pad`, the `plasma_*` functions), started with
`desktop/shell/osk-plasma.qml` by `/usr/share/hearth-plasma/xdg/autostart/hearth-osk.desktop` (Hearth Plasma only).
The QML host draws Mantel's own Hearth menu (`RadialMenu.qml`), controls card and messages (`Help.qml`) and on-screen
keyboard (`Keyboard.qml`) as layer-shell windows that never take the keyboard focus; everything Plasma gets from the
controller is typed or pointed through hearth-pad's uinput keyboard and mouse (KWin has no wtype).

| Controller | Hearth Classic | Hearth Plasma |
|---|---|---|
| Right stick (R3: precise) | the pointer | the same |
| A / RT, X / LT, L3 | click (hold to drag), right-click, middle-click | the same |
| Left stick | scroll | the same |
| D-pad | arrow keys | arrow keys: Plasma's launcher, tray and Quick Access popups, KRunner, Dolphin, System Settings |
| A / B / X in a menu | Enter / Esc / Menu key | the same in Plasma's popups, the taskbar and keyboard apps; RT / LT still click there |
| B | Esc | the same |
| Y, Guide + X | on-screen keyboard | the same keyboard |
| Menu ☰ | Hearth's launcher | Plasma's launcher (Hearth Launcher, `activateLauncherMenu`, as Super) |
| Guide | Hearth menu | the same menu: Apps, Keyboard, Quick Access (Hearth Quick Access, Meta+Alt+A), Close window (Alt+F4), Game Mode, Screenshot (Spectacle), Controller mouse on/off, Controls |
| LB / RB | switch windows | the same (KWin's Alt+Tab) |
| View + D-pad | snap left/right, maximize, minimize | KWin: tile left/right (Meta+←/→), maximize (Meta+PgUp), minimize (Meta+PgDn) |
| View + B | close the window | the same (Alt+F4) |
| View + A | | the taskbar gets the keys (Meta+Alt+P): the D-pad moves along it (launcher, tasks, tray, Quick Access, the clock, Game Mode), A opens |
| View + X | | Overview: every window, the D-pad picks one, A brings it up |
| View + Y | | clipboard history (Klipper, Meta+V) |
| View | the controls | the controls (Plasma's list) |
| Text fields | Hearth's own open the keyboard when reached with the controller | KRunner, and any text field KWin reports (`org.kde.kwin.VirtualKeyboard` `active`) within 8 s of using the controller; Plasma's launcher search waits for Y (the D-pad works its list) |
| A fullscreen game in front | the controller pauses (Guide still works) | the same |

**What's in front.** KWin's script `hearthpad` (`/usr/share/kwin/scripts/hearthpad`, turned on only by Hearth
Plasma's `kwinrc`) calls `org.hearth.Pad.Window(class, title, fullscreen, kind)` on the session bus whenever the
active window changes; hearth-pad serves `org.hearth.Pad` with GLib's D-Bus (python3-gi, in both images) in a thread of
its own and, once at start, runs `desktop/plasma/kwin-report.js` through KWin's scripting for the window already in
front. Plasma's popups, its taskbar, KRunner, Dolphin, System Settings and the lock screen take the controller as keys;
the desktop itself doesn't (A clicks its icons).

**Hearth's KDE-based apps** (Hearth Files, Terminal, Text, Reader, Capture, Calculator) get a mapping each, like Mantel's versions: see docs/KDE-APPS.md.

**Hearth's widgets with the D-pad.** Hearth Launcher: the search field, down into the grid, down again to Settings,
Game Mode and the power buttons (left/right along them, up back). Hearth Quick Access: the D-pad moves between the
controls (a slider keeps left/right for its value), A presses. Game Mode on the taskbar: A when the taskbar has the
keys. Hearth Quick Access has the global shortcut Meta+Alt+A (the layout script; hearth-pad gives it to older layouts
through `evaluateScript` when it has none).

**Safety.** The autostart does nothing without quickshell or the QML file; hearth-pad exits (and isn't restarted)
without python3-evdev or /dev/uinput; without python3-gi it still works, A just clicks everywhere; the KWin script's
calls simply fail when hearth-pad isn't running. Game Mode, the session's fallback to Hearth Classic and booting are
untouched.

Tests: `test_hearth_pad.py` (`PlasmaTest`: every mapping, menus taking the keys, the taskbar and Overview, the Hearth
menu's actions, fullscreen pause, the keyboard on text fields) and the controller check in `plasma-test.yml`.

## What's installed

Only Plasma's workspace (no `kde-standard` app bundle, no recommends):

- Anatase: `plasma-workspace kwin plasma-desktop` (+ when there: plasma-nm, plasma-pa, powerdevil, kscreen,
  kscreenlocker, bluedevil, polkit-kde, xdg-desktop-portal-kde, plasma-breeze, breeze-icon-theme, kde-cli-tools,
  plasma-systemsettings, kirigami, spectacle). Anatase's login screen (Plasma Login Manager) stays.
- Peppermint: `plasma-workspace kwin-wayland plasma-desktop` (+ the same optional set by Debian's names, and Dolphin
  for folders). SDDM stays the display manager.

## CI

`plasma-test.yml` (run by hand, or on pushes to main/bughunt touching these files):

- qmlformat must parse every Hearth widget; qmllint's findings are shown as warnings;
- every `metadata.json` is JSON, the layout script is valid JavaScript (`node --check`), and `kpackagetool6` /
  `plasma-apply-lookandfeel` find the look-and-feel package and all five widgets;
- `test_hearth_plasma.py`;
- KWin on a virtual 1920×1080 screen with Plasma's shell and Hearth's defaults (a new user, so Hearth's layout),
  screenshots of the desktop, the launcher, Quick Access, Hearth Link, Hearth Storage and the lock screen; errors
  from Hearth's widgets in Plasma's log fail the run. Side-by-side pictures with Hearth Classic's (from the Desktop
  Mode test's `ci/desktop-shots-*` branch) and a README go to the branch `ci/plasma-shots-<branch>`.
- the controller, end to end (`tools/e2e-plasma-pad.py`): a virtual Xbox 360 pad (uinput; the container gets the
  host's `/dev`, udev tags the devices, the outer labwc reads them with libinput and passes them to KWin) drives the
  real `hearth-pad --plasma`, the script standing in for `osk-plasma.qml`. Checked through KWin (a one-line KWin script
  calling hearth-pad's `Probe`): the right stick moves the pointer, View shows the controls, Guide + X asks for the
  keyboard, Menu opens Plasma's launcher, the D-pad and A start an app from it, the Hearth menu opens Hearth Quick
  Access and B closes it, View + B closes a window. Pictures `pad-*.png` on the same branch.

## Not done yet

- Not tried on a device: the timings (60 s to come up), Nvidia, handhelds, the controller.
- Controller: the radial menu and the keyboard are drawn by Quickshell over KWin (CI checks hearth-pad and Plasma's
  side, Hearth Classic's tests check the QML). Text fields in KDE apps open the keyboard by themselves only if KWin
  reports them without an input method configured (the CI log says what it saw); otherwise Y / Guide + X.
  Hearth Friends is a Hearth Classic panel (not in Plasma). Plasma's notification popups are reached with the pointer
  or through the tray (View + A).
- Plasma keeps its own look for its own parts (launcher popup, lock screen, notifications): by the owner's choice,
  Hearth Plasma is Plasma in Hearth's theme, not a copy of Mantel.
- Hearth Files built from Dolphin, Hearth Storage as a KIO plugin, KDE apps rebuilt as Hearth apps (ROADMAP).
- Hearth GNOME: see docs/GNOME.md.

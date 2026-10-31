# Hearth GNOME

**Status: Beta Experimental. It hasn't been tried on a device yet.** It's built into Hearth OS on Anatase and
Peppermint. CI checks it with `gnome-test.yml` and with the image checks in `base-anatase.yml` / `base-peppermint.yml`.

Hearth GNOME is the third Desktop Mode desktop, next to Hearth Classic (labwc + Mantel) and Hearth Plasma
(docs/PLASMA.md). It runs GNOME Shell, the current release on each base: GNOME 50 on Anatase (Fedora 44) and GNOME 48 on
Peppermint (Debian trixie). It has a look of its own that is still clearly Hearth and Mantel, and it all works with a
controller. Game Mode is the same whichever desktop you use.

## Picking it

GNOME is picked in the same places as Hearth Plasma, and they all write `HEARTH_DESKTOP=gnome` in
`~/.config/nimbus/session.conf`:

- **Game Mode › Settings › Desktop Mode › Desktop** (`ui/index.html` `deskPick`; `admin.rs` `desktop()` /
  `set_desktop()` / `gnome_installed()`).
- **Hearth Classic › Settings › Sign-in › Desktop** (`SetSignin.qml`, through `hearth-plasma desktop` /
  `desktop-set`).
- **The login screen**: the "Hearth GNOME" session (`hosts/linux/session/hearth-gnome.desktop`). Its `TryExec` hides it
  where GNOME isn't installed. Picking it runs `nimbus-session --desktop=gnome`, which saves the choice.

## Safety: it can never leave the device without a desktop

`nimbus-session` has GNOME functions of its own (`can_gnome`, `run_gnome`, `settle_gnome`). They work the same way as
Plasma's:

1. GNOME only runs when it's picked, `gnome-session`, `gnome-shell` and Hearth's session mode
   (`/usr/share/gnome-shell/modes/hearth.json`) are installed, Recovery hasn't asked for safe graphics this time,
   and it isn't an extra multi-seat player.
2. `run_gnome` starts `gnome-session` with `GNOME_SHELL_SESSION_MODE=hearth`, `XDG_CURRENT_DESKTOP=GNOME` and
   `HEARTH_DESKTOP=gnome`. It then waits up to `HEARTH_GNOME_TIMEOUT` (60 s) for GNOME Shell to run. If GNOME Shell
   doesn't start in time, ends before it came up, or crashes within 30 s, `$XDG_RUNTIME_DIR/hearth-gnome-failed` is
   written, `settle_gnome` cleans up GNOME's user services and environment, and **Hearth Classic starts instead for
   the rest of that sign-in**. Settings then says "Hearth GNOME didn't start this time" (`fellBackFrom`).
3. `nimbus-mode game|desktop|logout` ends GNOME through its own sign-out without asking
   (`gnome-session-quit --logout --no-prompt`, or `org.gnome.SessionManager.Logout 1`). If GNOME hangs, it's ended by
   force after 8 s. `nimbus-mode current` says `desktop` while `$XDG_RUNTIME_DIR/hearth-gnome.pid` is alive.
4. GDM isn't installed and the display manager is never changed: the build puts it back if a package changed it.
   GNOME's first-sign-in assistant and tour are kept away.
5. Every extension is guarded. A broken or missing one leaves plain GNOME, not a dead desktop. hearth-pad stops by
   itself and says why if python3-evdev or `/dev/uinput` is missing.

Game Mode, the boot path and Hearth Classic don't change. Hearth Classic stays installed in every image.

## What it looks like

All of this comes from Hearth's own GNOME Shell extensions in `os/system_files/usr/share/gnome-shell/extensions`.
Nothing is downloaded. The session mode `hearth` (`modes/hearth.json`) switches them on for Hearth GNOME only, so the
base's plain GNOME session stays plain. Mantel's palette (`Theme.qml`): firelit darks `#0f0b09` / `#1c1512` /
`#261d18`, the line colour `#3b2e26`, text `#f4ece3`, the ember accent `#ff8a3d`.

| Part | Hearth Classic (Mantel) | Hearth GNOME | Where |
|---|---|---|---|
| Shell theme | `Theme.qml` | menus, Quick Settings, calendar, notifications, overview, app grid, Alt+Tab switcher, dialogs, OSDs and sliders in Hearth's colours, with the ember accent on focus and on checked items; Noto Sans | `hearth-theme@hearthos.fun/stylesheet.css` |
| Bar | floating rounded bar along the bottom | GNOME's top bar moved to the bottom and floating: rounded, with a margin all round, and windows keep clear of it. Order: Hearth mark, Activities, pinned and open apps, … quick settings, clock with the date, Game Mode pill, show-desktop strip | `hearth-bar@hearthos.fun`, `modes/hearth.json` `panel` |
| Pinned and open apps | Mantel's pins | GNOME's favourites, seeded once with Mantel's pins (Beacon, Xodus Nexus, Bazaar) and GNOME Files. Windows of one app share a button. Click opens, raises or minimizes; middle-click opens a new window; right-click (X on the controller) shows GNOME's app menu | `hearth-bar` |
| Launcher | Hearth mark: Apps / Games, System apps folder, search, power | the Hearth mark: the same lists, split and power row (Settings, Game Mode, Lock, Sleep, Restart, Power off). The controller's Menu button opens it | `hearth-launcher@hearthos.fun` + `desktop/gnome/hearth-gnome apps/games` |
| Quick Access | `QuickAccess.qml` | GNOME's Quick Settings (volume, brightness, Wi-Fi, Bluetooth, screenshot, power), plus Hearth's: Power mode (hearth-power, so handheld modes work too), Game Mode, Hearth Settings, Keyboard, Controller mouse | `hearth-quick@`, `hearth-pad@` |
| Hearth Link, Hearth Storage | Settings › Hearth Link | Quick Settings toggles with menus. Link: pair / stop pairing, the code, paired devices, Sync now, settings; its icon shows on the bar while a code is up. Storage: use this device as Hearth Storage, servers and devices, Back up now | `hearth-link@`, `hearth-storage@` + `desktop/shell/hearth-link-settings` |
| Game Mode button, Super+G | yes | the "Game Mode" pill on the accent colour, and Super+G | `hearth-gamemode@hearthos.fun` |
| Notifications, calendar | bell + clock | GNOME's, in Hearth's colours | |
| Wallpaper | Fireside | Fireside, on the desktop and the lock screen | `hearth-theme` (first run) |
| Apps' look | `theme/apply.sh` | the same: GTK 3 / GTK 4 / libadwaita get Hearth's colours from `~/.config/gtk-4.0/gtk.css`, a dark style and the orange accent. GNOME Files, Console and Text Editor get bigger rounded rows, taller header bars and an ember focus ring. KDE-based apps keep qt6ct with Hearth's palette | `theme/gtk/gtk4.css` |
| Icons, cursor | Breeze Dark + Hearth's icon set, Breeze cursor | the same (Hearth's icons are in hicolor) | `hearth-theme` |
| Window buttons | minimize / maximize / close | minimize / maximize / close | `hearth-theme` |
| Lock | not locked unless asked | no lock when idle (GNOME's own lock is still on the power menu, Super+L) | `hearth-theme` |

The first time Hearth GNOME starts, `hearth-theme` writes Hearth's defaults into GNOME's settings, but only for keys
you haven't set yourself (`Gio.Settings.get_user_value`). It leaves the mark `~/.config/nimbus/gnome-set-up`, and
anything you change in GNOME's Settings afterwards stays.

The extensions share one module, `hosts/linux/desktop/gnome/hearth.js`, installed at
`/usr/share/nimbus/desktop/gnome/hearth.js`. It runs Hearth's helpers and reads their JSON. The Python helper
`desktop/gnome/hearth-gnome` is Hearth Plasma's `hearth-plasma` with GNOME's rules for which apps show
(`OnlyShowIn` / `NotShowIn` GNOME, `Hidden`). So the launcher, Quick Access, Link and Storage use exactly the same
backends as Mantel and Hearth Plasma's widgets: hearth-files' `.desktop` reading, hearth-games, hearth-power, wpctl,
brightnessctl, nmcli and hearth-link-settings.

## One app of each kind

GNOME uses its own apps for the kinds it has: **GNOME Files** (Nautilus), **Console** and **Text Editor**. They're
installed in both images and shown only in Hearth GNOME, because the build adds `OnlyShowIn=GNOME;` to their menu
entries. Inside Hearth GNOME, the Hearth and KDE apps of those kinds are hidden: `/usr/share/hearth-gnome/applications`
holds `Hidden=true` entries for hearth-files, hearth-files-kde, hearth-terminal, hearth-terminal-kde, hearth-text,
hearth-text-kde, Dolphin, Konsole, Kate, KWrite and foot, plus `gnome-mimeapps.list`. That list makes folders open in
GNOME Files, text in Text Editor and the terminal Console. `run_gnome` puts that folder first in `XDG_DATA_DIRS`, only
for GNOME. Hearth apps with no GNOME equivalent stay visible. Your own default apps (`~/.config/mimeapps.list`) still
come first.

| Kind | GNOME's app (desktop id) | Hidden in GNOME |
|---|---|---|
| Files | `org.gnome.Nautilus.desktop` | hearth-files, hearth-files-kde, org.kde.dolphin |
| Terminal | `org.gnome.Console.desktop` | hearth-terminal, hearth-terminal-kde, org.kde.konsole, foot, footclient, foot-server |
| Text editor | `org.gnome.TextEditor.desktop` | hearth-text, hearth-text-kde, org.kde.kate, org.kde.kwrite |

`XDG_CURRENT_DESKTOP` is `GNOME` in Hearth GNOME. When the shared app-kinds table lands (feature/app-per-desktop,
docs/APP-KINDS.md), these entries belong in it, and the folder above can then be made from it.

There's one known gap: "Show in folder" from other apps still goes to Hearth Files. hearth-portal holds
`org.freedesktop.FileManager1` for the whole Hearth session, in both modes.

## The controller

`hearth-pad@hearthos.fun` does for GNOME what `Pad.qml` does for Mantel. It starts `hearth-pad --gnome`, and hearth-pad's
GNOME mode lives in its own functions (`gnome_button`, `gnome_tick`, `gnome_command`) so that merges stay easy.
Typing and the pointer go through hearth-pad's uinput keyboard and mouse.

| Button | On the desktop | While GNOME Shell has the keyboard (overview, app grid, menus, Quick Settings, calendar, the bar) | In GNOME Files / Console / Text Editor |
|---|---|---|---|
| Right stick | pointer (R3: precise) | pointer | pointer |
| Left stick | scroll | scroll | scroll |
| A | click | Enter | open (Enter) |
| B | Esc | Esc (closes menus, gives focus back) | back (Files: Alt+Left) / Esc |
| X | right-click | Menu key (context menu) | context menu |
| Y | Hearth keyboard | Hearth keyboard | search + keyboard (Files) / keyboard |
| D-pad | arrow keys | moves GNOME Shell's focus | moves the selection |
| LB / RB | switch windows (Alt+Tab) | previous / next part (Shift+Tab / Tab) | previous / next tab |
| LT / RT | right-click / click | right-click / click | scroll up / down |
| L3 | middle-click | middle-click | click |
| Menu (☰) | Hearth launcher | Hearth launcher | Hearth launcher |
| View (tap) | Activities overview | | |
| View + D-pad | snap, maximize, restore (Super + arrows) | | |
| View + B | close the window | | |
| View + Menu | focus on the bar (then the D-pad walks it, A opens) | | |
| Guide | Hearth menu: Apps, Keyboard, Quick Settings, Close window, Game Mode, Notifications, Overview, Controller mouse | | |
| Guide + X | Hearth's on-screen keyboard (open / close) | | |

The extension tells hearth-pad what's on screen. It sends `keys {on}` when GNOME Shell's stage has the key focus or a
modal is up, `pause {on}` when a fullscreen window is in front, and `app {kind}` from the focused window's class
(`org.gnome.Nautilus` → files, `org.gnome.Console` / kgx → terminal, `org.gnome.TextEditor` → text). It checks every
250 ms.

### The on-screen keyboard

GNOME has no layer-shell, so the keyboard isn't a Quickshell window as it is under Plasma. It's drawn by the extension
(`keyboard.js`) as Shell chrome along the bottom: windows make room for it, and it never takes the keyboard focus. It's
Hearth's one keyboard: the engine is Mantel's copy of `ui/osk/hearth-osk.js`, loaded as it is, with the same settings,
learned words, word lists, themes, the Ember Wheel and split sticks (docs/OSK.md). Typing goes through hearth-pad's
uinput keyboard. Characters a US uinput keyboard can't type (accents, emoji), or everything when hearth-pad isn't
running, go through GNOME's own virtual keyboard instead. **GNOME's own on-screen keyboard is switched off** while the
extension runs (including the one that comes up by itself on touch), and it comes back when the extension is off.

## What's installed

Only the shell and what it needs. There's no GNOME app bundle, no weak dependencies or recommends, and no GDM:

- Anatase: `gnome-shell mutter gnome-session gnome-control-center`, plus `xdg-desktop-portal-gnome`,
  `gnome-settings-daemon`, `nautilus`, `gnome-console` and `gnome-text-editor` when they're available.
- Peppermint: the same names from trixie (`--no-install-recommends`). SDDM stays the display manager.

**Image size**: the build measures `/usr` + `/etc` before and after the GNOME install and prints it as
`::notice title=Hearth GNOME`. It also writes the number to `/usr/share/nimbus/hearth-gnome-size-mb`, and the image check
reports that as "Hearth GNOME size".

## CI

`gnome-test.yml` runs each base (Fedora 44 for Anatase, Debian trixie for Peppermint) in a privileged container that
has the runner's uinput (`.github/scripts/gnome-check.sh`):

1. GNOME's minimal set, installed the way the images install it, with its size noted.
2. Every extension's JavaScript must parse (`node --check`). Every `metadata.json` must name its folder and list that
   base's GNOME version. The session mode must be valid JSON. `test_hearth_gnome.py`, `test_hearth_plasma.py` and
   `test_hearth_pad.py` run.
3. A headless GNOME Shell starts in Hearth's session mode, plus the CI-only helper `hearth-ci@hearthos.fun`
   (`.github/scripts/gnome-ci`). The helper allows Eval and screenshots, and replays hearth-pad's uinput devices into
   the headless shell. Then `tools/e2e-hearth-gnome.py` checks that every Hearth extension is active without errors,
   that the bar is at the bottom, that Super+G is taken and that the launcher lists apps. It takes screenshots of the
   desktop, Quick Settings, Link, the launcher, Storage, the overview, the app grid, the calendar, the Hearth menu and
   the keyboard. Finally a virtual Xbox 360 pad tests the pointer, RT click, Menu → launcher (with the D-pad and B in
   it), Guide → Hearth menu, View → overview, Guide + X → keyboard (A types, B hides), View + Menu → bar, GNOME Files
   (open a folder, go back, open a file in Text Editor) and Console (type through the keyboard).

The pictures, side by side with Hearth Classic's (from the Desktop Mode test's `ci/desktop-shots-*` branch), and a
README go to `ci/gnome-shots-<branch>`.

`test_hearth_gnome.py` covers the session (crash, no desktop in time, not installed, a normal start and end, the
login screen's choice, Plasma unchanged), the helper, hearth-pad's GNOME mode, and the files.

## Not done yet

- It hasn't been tried on a device: the timings (60 s to come up), Nvidia, handhelds, and the controller on real
  hardware.
- "Show in folder" from other apps opens Hearth Files (see above).
- The keyboard's split shape draws as a full keyboard; its accents and Ember Wheel show in the strip above the keys
  instead of over them.
- GNOME's Super key opens GNOME's overview. Hearth's launcher is on the Hearth mark and the controller's Menu button.

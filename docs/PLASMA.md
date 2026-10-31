# Hearth Plasma

**Status: Beta Experimental, not tried on a device yet.** Built into Hearth OS on Anatase and Peppermint, checked by
CI (`plasma-test.yml`, the image checks in `base-anatase.yml` / `base-peppermint.yml`).

Desktop Mode can run one of two desktops. Game Mode is the same with either.

| Desktop | What it is | |
|---|---|---|
| **Hearth Classic** | labwc + Mantel (Quickshell): today's desktop | always installed, the default and the fallback |
| **Hearth Plasma** | KDE Plasma 6 in Hearth's look, with Hearth's widgets | picked in Settings or on the login screen |
| Hearth GNOME | | coming later (shown greyed out) |

Switching keeps your files and settings: both desktops use the same home folder, the same apps and the same Hearth
settings files. Plasma's own settings (`~/.config/kdeglobals`, `plasma-org.kde.plasma.desktop-appletsrc`...) are only
read by Plasma.

## Picking a desktop

- **Game Mode › Settings › Desktop Mode › Desktop** (`ui/index.html` `deskRow`/`deskPick`, bridge `login.desktop` /
  `login.setDesktop`, `hosts/hearth-core/src/admin.rs` `desktop()` / `set_desktop()`).
- **Hearth Classic › Settings › Sign-in › Desktop** (`SetSignin.qml`, through `desktop/plasma/hearth-plasma desktop` /
  `desktop-set`). Hearth GNOME shows there greyed out (`SetChoice.qml` options can be `disabled`).
- **The login screen**: "Hearth Classic" and "Hearth Plasma" sessions next to "Hearth"
  (`hosts/linux/session/hearth-classic.desktop`, `hearth-plasma.desktop`; Hearth Plasma is hidden by its `TryExec`
  where Plasma isn't installed). Picking one runs `nimbus-session --desktop=classic|plasma`, which saves the choice,
  so automatic sign-in (always the "Hearth" session) keeps it. Hearth's own login screen theme notes that Hearth
  GNOME is coming later.

All three write the same line: `HEARTH_DESKTOP=classic|plasma` in `~/.config/nimbus/session.conf`. It's read again
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

## Not done yet

- Not tried on a device: the timings (60 s to come up), Nvidia, handhelds, the controller.
- Controller control of the desktop (Mantel's `hearth-pad`), Mantel's on-screen keyboard and radial menu, the
  Hearth menu, clipboard history, Hearth Friends on the taskbar: Hearth Classic only for now.
- The lock screen is Plasma's layout (Hearth's colours and wallpaper), not a port of `Lock.qml`.
- Hearth Files built from Dolphin, Hearth Storage as a KIO plugin, KDE apps rebuilt as Hearth apps (ROADMAP).
- Hearth GNOME.

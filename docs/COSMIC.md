# Hearth COSMIC

**Status: Beta Experimental, not tried on a device yet.** Built into Hearth OS on **Anatase only**, checked by CI
(`cosmic-test.yml`, the image checks in `base-anatase.yml`).

Hearth COSMIC is System76's COSMIC desktop in Hearth's look, with Hearth's launcher, Quick Access, Hearth Link, Hearth
Storage, the Hearth menu, the on-screen keyboard and full controller use, as another choice for Desktop Mode next to
Hearth Classic and Hearth Plasma (docs/PLASMA.md). Game Mode is the same whichever desktop you pick.

| Edition | Hearth COSMIC | Why |
|---|---|---|
| Anatase (Fedora 45) | yes | Fedora packages COSMIC (1.10.0 in Fedora 45) |
| Peppermint (Debian trixie) | "not available on this edition" | Debian has no COSMIC packages (not in trixie, not in backports). Building all of COSMIC (about twenty Rust programs) from source in every image build would add an hour or more to each Peppermint build and its own upkeep; System76's own packages are for Pop!_OS (Ubuntu), not Debian. It can come later if Debian packages it. |

## Picking it

The same places as Hearth Plasma, writing `HEARTH_DESKTOP=cosmic` in `~/.config/nimbus/session.conf`:

- **Game Mode › Settings › Desktop Mode › Desktop** (`ui/index.html` `deskPick`; `hosts/hearth-core/src/admin.rs`
  `cosmic_installed()`).
- **Hearth Classic › Settings › Sign-in › Desktop** (`SetSignin.qml`, through `desktop/plasma/hearth-plasma desktop` /
  `desktop-set cosmic`).
- **The login screen**: "Hearth COSMIC" (`hosts/linux/session/hearth-cosmic.desktop`, hidden by its `TryExec` where
  COSMIC isn't installed), which runs `nimbus-session --desktop=cosmic` and saves the choice.

Where it isn't installed (Peppermint) it shows greyed out as "Hearth COSMIC (not available on this edition)". COSMIC's
own "COSMIC" login entry is removed: Hearth COSMIC is the way in, with Game Mode and the fallback.

## Safety: COSMIC can never leave the device without a desktop

`nimbus-session` `run_desktop`, the same rules as Hearth Plasma:

1. Hearth COSMIC only when it's picked, `start-cosmic`, `cosmic-comp`, `cosmic-panel` and Hearth's COSMIC defaults
   are installed, this isn't Recovery's "safe graphics this time", and it's not an extra multi-seat player.
2. `run_cosmic` starts `start-cosmic` and waits up to `HEARTH_COSMIC_TIMEOUT` (60 s) for COSMIC's compositor and panel.
   No desktop in time, COSMIC ending before it came up, or crashing within 30 s → `$XDG_RUNTIME_DIR/hearth-cosmic-failed`,
   what COSMIC left is cleaned up (`settle_cosmic`) and **Hearth Classic starts instead for the rest of that sign-in**.
   Settings then says "Hearth COSMIC didn't start this time".
3. `nimbus-mode game|desktop|logout` ends COSMIC (`cosmic-session`, by force after 8 s); `nimbus-mode current` says
   `desktop` while it runs (`$XDG_RUNTIME_DIR/hearth-cosmic.pid`).
4. Booting, the login screen and Game Mode are untouched. Fedora's `cosmic-session` can't be installed without COSMIC's
   login screen (cosmic-greeter, greetd) and first-run wizard: they're installed but never switched on. The image build
   checks the display manager is still Anatase's (and puts it back if a package changed it), and the wizard's autostart
   entry is removed.

Tests: `hosts/linux/test/test_hearth_cosmic.py` (crash, no desktop in time, not installed, normal start and end, the
login screen's choice, Deepin's choice falling back to Classic).

## The look

Hearth's COSMIC settings are system-wide defaults made at image build time from COSMIC's own
(`desktop/cosmic/hearth-cosmic-defaults` into `/usr/share/hearth-cosmic/data/cosmic`). `nimbus-session` puts that
folder first in `XDG_DATA_DIRS` only for Hearth COSMIC. Anything you change in COSMIC Settings goes to `~/.config/cosmic`
and wins.

| Part | Hearth Classic (Mantel) | Hearth COSMIC | Where |
|---|---|---|---|
| Colours | Theme.qml: warm firelit darks | every grey of COSMIC's dark theme becomes Hearth's dark of the same lightness | `hearth-cosmic-defaults` (`RAMP`) |
| Accent | ember `#ff8a3d` | ember: COSMIC's cyan and all its shades become ember shades; window hint ember | same |
| Bar | floating rounded bar along the bottom | one floating, rounded COSMIC panel along the bottom (14 px corners, 8 px gap), apps on the left, status and clock on the right; no top bar, no dock | `desktop/cosmic/defaults/com.system76.CosmicPanel*` |
| Pinned apps | Mantel's pins | Hearth Launcher, Hearth, COSMIC Files, Terminal, Edit, Hearth Browser, Beacon, Xodus Nexus, Bazaar, Hearth Quick Access, Hearth Link, Game Mode | `com.system76.CosmicAppList/v1/favorites` |
| Launcher | Hearth's (Super) | Hearth's own launcher, the same QML (Super, Menu on the controller, the panel's Hearth Launcher) | `desk-cosmic.qml`, `system_actions` |
| Quick Access | QuickAccess.qml | the same QuickAccess.qml, plus COSMIC's own sound, network, Bluetooth, battery applets | `desk-cosmic.qml` |
| Hearth Link, Hearth Storage | Settings › Hearth Link | one card (CosmicHub.qml) from the panel's Hearth Link entry (Hearth Storage in its menu) | `CosmicHub.qml`, `hearth-link-settings` |
| Game Mode | button, Super+G | Game Mode on the panel, in Quick Access and the Hearth menu; Super+G | `custom` shortcuts, `hearth-gamemode.desktop` |
| Wallpaper | Fireside (moving) | Fireside (still) | `com.system76.CosmicBackground/v1/all` |
| Fonts, icons | sans-serif, Breeze Dark + Hearth's icons | the same sans-serif font (fontconfig's, read at build time), Breeze Dark with Hearth's app icons | `com.system76.CosmicTk` |
| Spacing | big controller-friendly rows | COSMIC's "Spacious" density and header size (bigger rows in COSMIC Files, Terminal, Edit and Settings) | `com.system76.CosmicTk` |
| Window shortcuts | labwc's | COSMIC's own, plus Super+Ctrl+Up / Down maximize / minimize | `custom` |

### One app of each kind

COSMIC uses its own apps for the kinds it has: **COSMIC Files** (file manager), **COSMIC Terminal** and **COSMIC Edit**
(text editor), in Hearth's colours and spacing through the theme above. Hearth Files, Hearth Terminal and Hearth Text
are hidden in COSMIC (by the app-kinds table, docs/APP-KINDS.md). `XDG_CURRENT_DESKTOP` is `COSMIC` (set by
`start-cosmic`). Desktop ids: `com.system76.CosmicFiles.desktop`, `com.system76.CosmicTerm.desktop`,
`com.system76.CosmicEdit.desktop`; COSMIC's screenshot tool `com.system76.CosmicScreenshot.desktop` and COSMIC Settings
`com.system76.CosmicSettings.desktop` come with `cosmic-session` too. Not installed: COSMIC's store, player, system
monitor and wallpapers (Hearth already has those kinds).

## The companion: Hearth's pieces on COSMIC

`hosts/linux/desktop/shell/desk-cosmic.qml` (Quickshell, layer-shell windows over COSMIC; started by
`/usr/share/hearth-cosmic/xdg/autostart/hearth-cosmic.desktop`, only in Hearth COSMIC). It's Mantel's own QML:
Launcher.qml, QuickAccess.qml, RadialMenu.qml (the Hearth menu), Keyboard.qml (the on-screen keyboard), Help.qml (the
controls), plus CosmicHub.qml. `desktop/cosmic/hearth-cosmic launcher|games|quick|link|storage|menu|keyboard` opens
them (the panel's entries and Super use it), starting the companion first if it isn't running.

**Known COSMIC quirk (seen in CI, to check on a device):** COSMIC 1.10's compositor drops the companion's whole
Wayland connection whenever one of its windows closes (Qt's layer-shell teardown; Qt's text-input and fractional
scaling were ruled out). So `hearth-cosmic companion` keeps it running (started again within a second), the
controller is a separate process (`hearth-cosmic pad`: `hearth-pad --cosmic --socket`) that keeps the pointer and keys
working meanwhile, and a menu asked for in that second (Guide, Menu, the keyboard) opens as soon as the companion is
back. The companion's messages are in `~/.local/share/nimbus/cosmic-companion.log`.

Why not COSMIC applets: they're Rust programs on libcosmic, a library whose interface still changes between COSMIC
releases, and each applet is a separate 30-50 MB program. Mantel's QML is already Hearth's look and logic, and runs on
COSMIC as layer-shell windows; the panel gets Hearth's entries as pinned apps.

## The controller

`hearth-pad --cosmic` (`CosmicPad`), started by the companion. Keyboard-first in every window, through uinput:

| Control | Does |
|---|---|
| Right stick | the pointer (R3: precise) |
| Left stick, LT / RT | scroll (LT up, RT down) |
| A | Enter; a click when the pointer was moved in the last 1.2 s |
| B | back: Esc (COSMIC Files: the folder before, Alt+Left) |
| X | right-click (context menu) |
| Y | COSMIC Files / Edit: search (Ctrl+F) and the on-screen keyboard; Terminal and other windows: the keyboard |
| D-pad | arrow keys: moves the selection in lists, grids and menus |
| LB / RB | switch windows (Alt+Tab); in COSMIC Files, Terminal and Edit when COSMIC says they're in front: tabs (Ctrl+Tab) |
| View + LB / RB | the other one: tabs (Ctrl+Tab), or windows in COSMIC's apps |
| View + Y | search (Ctrl+F) and the on-screen keyboard, in any window |
| View + B | close the window (Super+Q) |
| View + D-pad | move the window left / right, maximize, minimize |
| View (tap) | the controls card |
| Menu ☰ | Hearth's launcher (apps and games) |
| Guide | the Hearth menu (apps, keyboard, Quick Access, close, Game Mode, screenshot, controller mouse, controls) |
| Guide + X | Hearth's on-screen keyboard (open / close), typing through hearth-pad's uinput keyboard |

In Hearth's own menus (launcher, Quick Access, Hearth Link card) the controller types arrows / Enter / Esc, as in
Hearth Classic. A fullscreen game would pause it.

**Which app is in front:** COSMIC 1.10 doesn't tell other programs which window has the focus (it has no
wlr-foreign-toplevel protocol, only its own), so in CI the companion sees no app id, and the per-app keys (B back in
Files, Y search, LB / RB tabs) don't switch on by themselves; every window gets the general keys, and View + LB / RB
(tabs) and View + Y (search) give the same things by hand. The per-app table is ready in `hearth-pad` (`COSMIC_APPS`)
for when COSMIC reports it. For the same reason the controller isn't paused automatically by a fullscreen game in
Hearth COSMIC (Game Mode is where games are meant to run; "Controller mouse: off" in Quick Access or the Hearth menu
pauses it by hand).

## What's installed

`hosts/linux/desktop/cosmic/fedora-packages` (shared by the image build and the test): COSMIC's workspace
(cosmic-session, -comp, -panel, -settings, -settings-daemon, -launcher, -applets, -bg, -osd, -notifications,
-workspaces, -app-library, -randr, -idle, -icon-theme, xdg-desktop-portal-cosmic), COSMIC Files, Terminal and Edit,
and what Fedora's cosmic-session can't go without (cosmic-osk, cosmic-greeter, cosmic-initial-setup: installed, unused).
No recommends.

**Size:** about 650 MB more on Anatase (CI's measurement on Fedora 45; each COSMIC program carries its own copy of
COSMIC's toolkit). The biggest parts are COSMIC Files, the applets, Settings, the first-run wizard and the greeter
(both forced by Fedora's cosmic-session), the portal, Terminal and the app library. That's over the 600 MB guideline.

## CI

`cosmic-test.yml` (pushes to main / bughunt / this branch touching these files, or by hand):

- `test_hearth_cosmic.py` (session fallback, the desktop choice, CosmicPad's keys, the defaults generator);
- `.github/scripts/cosmic-test.sh` in Fedora 45: COSMIC from `fedora-packages` and its size, Hearth's defaults
  (Super = Hearth's launcher, no cyan left in the theme), COSMIC's compositor nested in a headless labwc with its
  panel, wallpaper, settings daemon and Hearth's companion; screenshots of the desktop, launcher (apps, games), Quick
  Access, Hearth menu, Hearth Link, Hearth Storage, the keyboard, the controls, COSMIC Files, Terminal and Edit;
- controller checks (`.github/scripts/cosmic-pad-drive.py`): a virtual Xbox 360 pad drives hearth-pad --cosmic, the
  keys it types are checked step by step and passed on to COSMIC, with screenshots of each app driven by the pad;
- side by side with Hearth Classic and Mantel's Files and Text, and a README, on the branch `ci/cosmic-shots-<branch>`.

## Not done yet

- Not tried on a device: the timings, Nvidia, handhelds, the controller on real hardware.
- Peppermint (no Debian packages).
- The lock screen is COSMIC's (Hearth's colours), not a port of `Lock.qml`.
- Hearth Settings opens as the Hearth app's settings in COSMIC (Mantel's Settings app is part of Hearth Classic).

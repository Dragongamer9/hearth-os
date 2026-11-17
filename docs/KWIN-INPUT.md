# Mantel on KWin: typing, the lock screen and screen sharing (1.14)

Three things Mantel does on labwc through wlroots-only Wayland protocols that KWin doesn't have. On KWin
(`HEARTH_WM=kwin`, set by the KWin session) Hearth does them its own way; on labwc nothing changes.

| | labwc (as before) | KWin |
|---|---|---|
| Typing (on-screen keyboard, clipboard paste, emoji picker, Hearth Connect, magnifier keys) | wtype (virtual-keyboard protocol) | `hearth-type`: Hearth's input method for text fields, a uinput keyboard for keys |
| Lock screen | `Lock.qml` (ext-session-lock-v1) | KWin's own locker with Hearth's lock screen in it |
| Screen sharing and screenshots for apps | xdg-desktop-portal-wlr | Hearth's portal backend (`hearth_screencast.py`) on KWin's screencast protocol + PipeWire, Hearth's picker |

## Typing: hearth-type

`hosts/linux/desktop/shell/hearth-type` takes wtype's arguments (`-M ctrl -k v -m ctrl`, `-- text`). On labwc it simply
runs wtype. On KWin:

- **Text** goes to `hearth-kwin-im` (`hosts/linux/desktop/kwin-helpers/hearth-kwin-im.c`), Hearth's input method. KWin
  starts it itself (kwinrc `[Wayland] InputMethod=/usr/share/applications/hearth-kwin-im.desktop`) and hands it a
  context whenever a text field has the keyboard (apps with text-input v1/v2/v3: Qt, GTK, Firefox, Chromium/Electron
  with Wayland IME). Text is committed whole: any character, emoji, other scripts, whatever the keyboard layout.
  Its socket (`$XDG_RUNTIME_DIR/hearth/im.sock`, this user only) also says when a text field gets or loses the
  keyboard (`W`), for the on-screen keyboard to open by itself later.
- **Keys and shortcuts**, and text where no text field takes the input method (games, Xwayland apps, terminals
  without it), go through a uinput keyboard (python3-evdev, the same `/dev/uinput` access hearth-pad has), using
  the active layout's keys (KWin's `org.kde.keyboard`, then kxkbrc, `XKB_DEFAULT_LAYOUT`; tables in
  `osk-keymap.json`). Characters that layout can't type are left out (said on stderr).

Wired in: `Keyboard.qml` (the on-screen keyboard), `hearth-clipboard` (paste), `hearth-access` (the magnifier's
Meta+= / Meta+- / Meta+0, KWin's Zoom), `hosts/hearth-emoji` and `hosts/hearth-connect` (typing from the phone).
Still labwc key bindings typed with wtype, for the window branch to move to KWin's D-Bus: "Move to desktop" in
`Bar.qml`, `Dock.qml`, `TaskStrip.qml`, and `SnapLayouts.qml`.

## Lock screen

KWin doesn't implement ext-session-lock-v1 (wayland.app lists it as unsupported even in KWin 6.7; checked in CI:
`kwin_e2e.py`), so Quickshell's `WlSessionLock` would cover nothing there. On KWin, `Lock.qml` asks KWin's locker
(`org.freedesktop.ScreenSaver.Lock`), checks it really locked (`GetActive`), and if it didn't, says so with a critical
notification and a toast: never a silent "locked". KWin's locker is fail-safe (if its screen crashes, the screen
stays covered).

What it shows is Hearth's: `hosts/linux/desktop/kwin-lock`, installed by `kwin-lock/install.sh` as the Plasma shell
package `org.hearth.mantel-lock` (/usr/share/plasma/shells; only a lock screen). KWin's locker loads the shell package
named by `PLASMA_DEFAULT_SHELL` in KWin's environment (the KWin session sets it) if its `X-Plasma-APIVersion` is 2 or
more. It has the same clock, password field and own on-screen keyboard (touch, D-pad and A) as Lock.qml, in plain
QtQuick, and checks the password with PAM through KWin's locker (PAM service `kde`). `kscreenlockerrc` (Hearth's
KWin defaults) turns off KWin's own automatic locking and lock on resume, so Mantel's settings (Lock when waking,
idle) are the only ones deciding. Those keys are fixed (`Autolock[$i]`, KConfig's "immutable"), so a
`~/.config/kscreenlockerrc` left from KDE Plasma (1.13) can't turn them back on; so are kwinrc's
`[Wayland] InputMethod` and `[ModifierOnlyShortcuts] Meta` (Mantel's keyboard and Super key), never Night Light or
tearing (Settings › Display writes those to your kwinrc). If your own files set any of them, Hearth tells you once
which and why (hearth-your-desktop `kwin-overrides`, its `LOCKED` list; the "Some of your KDE settings aren't used
here" notice) and never changes your files.

The controller on the lock screen: `Pad.qml` tells hearth-pad `locked {on}` while it's up (`DeskState.locked`, which
on KWin follows KWin's locker). hearth-pad then holds the controller (a game behind the lock never sees a press) and
sends only arrows for the D-pad, Enter for A, Esc for B and Backspace for X; no menu, keyboard, pointer or shortcuts.

### Never a lockout

KWin's locker is fail-safe the strict way: if its lock screen can't load, it keeps everything covered by its own
emergency screen, which on a handheld without a keyboard would be a lockout. So:

- **The image build fails** without the locker or without Hearth's lock screen loading in it. On Debian
  `kwin-lock/install.sh` installs `libkscreenlocker6` (the greeter `kscreenlocker_greet`), `plasma-framework`
  (libplasma's plugins: the Plasma/Shell package type the greeter reads the lock screen as; not the Plasma desktop),
  the QtQuick modules, `/etc/pam.d/kde` when nothing brought one, and the lock screen. Then `hearth-lockcheck
  --as-kwin` runs the greeter's own test run off screen and needs the lock screen's "hearth-lock-screen: loaded" line.
  `base-peppermint.yml` checks the same.
- **Lock.qml asks `hearth-lockcheck` before every lock on KWin**, which goes by the running KWin's own environment
  (`/proc/<KWin>/environ`): KWin must have `PLASMA_DEFAULT_SHELL=org.hearth.mantel-lock`, the greeter must be
  installed, the package complete, and the greeter's test run with KWin's environment must show Hearth's lock screen
  loading (a positive sign; an error, a fallback or silence all fail). The test run is cached per change and made at
  Mantel's start. If anything fails it **doesn't lock**: a critical notification and a toast say why.
- **The session's guard** (nimbus-session `run_kwin`, red flag 2: a device must never be locked out): when Hearth's
  lock screen package or KWin's locker (`kscreenlocker_greet`) isn't there, or the greeter's test run
  (`hearth-lockcheck --as-kwin`, at most 12 s; it ends as soon as the greeter says it loaded) doesn't show Hearth's
  lock screen loading, KWin starts with `--no-lockscreen` and
  without `PLASMA_DEFAULT_SHELL`, so nothing can lock with a lock screen that can't show. You're told once, through
  Hearth's own notice ("The lock screen is off", DesktopNotice.qml, which Do not disturb can't hide; the details in
  Settings › System › Your desktop and session.log), and Lock.qml says why each time you lock. Told again if it
  breaks again after working. On real images this never trips: their build fails first (above). A pass is kept per
  image (`~/.local/state/hearth/kwin-lock-proven`: the image's version and the lock screen's and greeter's checksums),
  so the test run only happens on the first start after an update; a failure is never kept.
- **KWin's environment for the check**: right before kwin_wayland starts, run_kwin writes the exact environment KWin
  gets to `$XDG_RUNTIME_DIR/hearth/kwin.env` (`env -0`; mode 600 in a 700 folder; written whole, then moved into
  place), since KWin's file capability hides its `/proc/<pid>/environ`. A failed write doesn't stop KWin (it's
  logged; Lock.qml then won't lock). It's removed when KWin ends.
- **If the locker breaks anyway** (its "the screen locker is broken" emergency screen, on a device that passed every
  check): the screen stays covered, which is KWin's choice and the safe one. To get back in without a keyboard: hold
  the power button to restart (the next start shows Hearth's login screen or SDDM as usual); with a keyboard, Ctrl+Alt+F3
  for a text console, sign in, run `loginctl unlock-session` (for your session, `loginctl list-sessions` shows its
  number), then Ctrl+Alt+F1 (or the number your desktop is on) to go back. Over SSH or Hearth Link's terminal the same
  command works. The Guide button deliberately does **not** unlock: anything that unlocks without your password
  because the lock screen "looks broken" could be triggered on purpose (crash the greeter, get in), so a broken locker
  is only ever left through a restart or `loginctl` by someone who can sign in. Send logs afterwards: kwin.log has the
  greeter's error.
- CI (`kwin-input.yml`) checks all of this, and on Debian trixie (`lock-debian`) locks for real, types a wrong
  password with hearth-type (still locked) and the right one (unlocked: PAM).
- Left: something else asking the system to lock (`loginctl lock-session`, an app's "lock now") goes to KWin's
  locker straight, without Hearth's check. The image build's checks make that safe as long as the image is whole.

**If the screen ever is stuck locked** (KWin's emergency screen, or a lock screen that won't take the password):
from another computer `ssh` in, or on the device press Ctrl+Alt+F3 for a text console and sign in, then run
`loginctl list-sessions` to find Desktop Mode's session, then `loginctl unlock-session <ID>` (from another console a
bare `loginctl unlock-session` would unlock that console's own session, not the desktop's), or
`sudo loginctl unlock-sessions` for all of them. Restarting the device also ends it.

## Screen sharing and screenshots

`hearth-portal filechooser` (D-Bus `org.freedesktop.impl.portal.desktop.hearth`) also serves
`org.freedesktop.impl.portal.ScreenCast` and `.Screenshot` (`hearth_screencast.py`):

- An app asks to share → Hearth's picker (`cast-picker.qml`: Screens and Windows, each with a picture of what it
  shows now; arrows/D-pad, Tab, Enter/A, Esc/B) → KWin makes a PipeWire stream of the choice
  (`zkde_screencast_unstable_v1`) → xdg-desktop-portal hands it to the app. Cursor hidden / in the picture /
  metadata, "remember this" (restore data kept by the app, the same screen while it's there). The stream ends when
  the app stops or KWin ends it (window closed).
- Screenshot: the whole desktop without questions, or the picker with "interactive"; saved in Pictures/Screenshots.
  PickColor isn't offered yet.
- The backend answers only xdg-desktop-portal itself (the D-Bus sender must own `org.freedesktop.portal.Desktop`),
  so apps can't skip the picker or the Allow? question by calling it straight, and the window list only from KWin.
- `hearth-kwin-cast` works only when its parent is `hearth-portal` (a root-owned file nobody else can change). The
  risk left, for the owner to know: KWin trusts the helper by its path, and any program running as you could still
  start `hearth-portal cast shot …` itself, or read the screen in other ways a desktop can't stop (as on X11).
- Remote control (`org.freedesktop.impl.portal.RemoteDesktop`, e.g. VNC): Hearth asks "Allow … to control this
  computer?" (access-prompt.qml), optionally with the screen picker when the app also shares the screen, then the
  app's keys and pointer go to KWin through its fake input (`hearth-kwin-cast input`; keycodes, keysyms on a US layout,
  relative and absolute pointer, buttons, scrolling; no touch yet). It ends when the app closes the session.
- "Allow?" questions (xdg-desktop-portal's Access, e.g. before an app's first screenshot) are Hearth's too:
  `access-prompt.qml` (Allow / Don't allow, keyboard and D-pad, nothing allowed unless you pick Allow). On labwc
  they stay GTK's.
- KWin gives its screencast protocol and screenshot interface only to a program whose .desktop file asks for them,
  found by path, so those calls go through `hearth-kwin-cast` (`kwin-helpers/hearth-kwin-cast.c`,
  `/usr/libexec/hearth`, `hearth-kwin-cast.desktop`). The window list comes from a KWin script calling back.
- Registered in `hearth.portal` (`Interfaces=…FileChooser;…ScreenCast;…Screenshot;…Access;`) and, for the KWin session,
  `/usr/share/xdg-desktop-portal/hearth-portals.conf` (`XDG_CURRENT_DESKTOP=Hearth:KDE`; ScreenCast, Screenshot, Access and FileChooser = hearth). labwc keeps
  `nimbus-portals.conf` (wlr).

## Built and tested

The two C helpers are built by the image's builder stage (`os-bases/peppermint/builder.sh`) and in CI, never on a
PC. `.github/workflows/kwin-input.yml`: unit tests (`test_kwin_input.py`), the helpers on Debian trixie, and KWin
for real (nested in a headless labwc): typing into a GTK field, Mantel's on-screen keyboard, a PipeWire stream with
real frames, both portals through xdg-desktop-portal with the picker driven like a person would, Hearth's lock screen
in KWin's locker, and a real lock from Lock.qml. Pictures as SHOT notices.

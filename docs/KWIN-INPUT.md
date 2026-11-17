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

What it shows is Hearth's: `hosts/linux/desktop/kwin-lock` (installed as the look-and-feel package
`org.hearth.mantel-lock`, only a lock screen): the same clock, password field and own on-screen keyboard (touch,
D-pad and A) as Lock.qml, plain QtQuick, checking the password with PAM through KWin's locker. Hearth's KWin defaults
(`hosts/linux/desktop/kwin/xdg/kscreenlockerrc`) pick it and turn off KWin's own automatic locking and lock on resume,
so Mantel's settings (Lock when waking, idle) stay the only ones deciding.

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
- KWin gives its screencast protocol and screenshot interface only to a program whose .desktop file asks for them,
  found by path, so those calls go through `hearth-kwin-cast` (`kwin-helpers/hearth-kwin-cast.c`,
  `/usr/libexec/hearth`, `hearth-kwin-cast.desktop`). The window list comes from a KWin script calling back.
- Registered in `hearth.portal` (`Interfaces=…FileChooser;…ScreenCast;…Screenshot;`) and, for the KWin session,
  `/usr/share/xdg-desktop-portal/hearth-portals.conf` (`XDG_CURRENT_DESKTOP=Hearth:KDE`). labwc keeps
  `nimbus-portals.conf` (wlr).

## Built and tested

The two C helpers are built by the image's builder stage (`os-bases/peppermint/builder.sh`) and in CI, never on a
PC. `.github/workflows/kwin-input.yml`: unit tests (`test_kwin_input.py`), the helpers on Debian trixie, and KWin
for real (nested in a headless labwc): typing into a GTK field, Mantel's on-screen keyboard, a PipeWire stream with
real frames, both portals through xdg-desktop-portal with the picker driven like a person would, Hearth's lock screen
in KWin's locker, and a real lock from Lock.qml. Pictures as SHOT notices.

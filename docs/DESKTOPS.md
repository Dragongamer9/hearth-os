# Desktop Mode's desktop

Desktop Mode is **Mantel**, Hearth's own desktop (the bar, the dock, Quick Access, notifications, the controller:
`hosts/linux/desktop/shell`). Settings › System › Your desktop shows it.

## Mantel on KWin (1.14)

The owner, Nov 16, 2026: Mantel moves from labwc to **KWin**, KDE's window manager, and what KDE Plasma has that Mantel
lacks gets written for Mantel. KWin draws, moves and snaps the windows; Mantel is still everything you see around them.

- **Starting.** nimbus-session's `run_classic` starts `kwin_wayland --xwayland` (its screen locker is Mantel's lock) with Desktop Mode's one
  autostart (`desktop/kwin/autostart` runs `desktop/labwc/autostart`). KWin gets labwc's environment file (with your
  accessibility lines), `XDG_CURRENT_DESKTOP=Hearth:Nimbus:KDE` (Hearth first: its portals, `hearth-portals.conf`; Nimbus
  next, so Hearth's default apps, file chooser and app kinds work as before), `PLASMA_DEFAULT_SHELL=org.hearth.mantel-lock`
  and Hearth's KWin defaults first in `XDG_CONFIG_DIRS` (`desktop/kwin/xdg`: `kwinrc` with Hearth's on-screen keyboard
  as KWin's input method, and the input part's `kscreenlockerrc`).
  They're system defaults: your own `~/.config/kwinrc` is read on top and Hearth never writes it. KWin's messages go
  to `~/.local/share/nimbus/kwin.log` (in Send logs).
- **The safe start.** labwc stays in the image. If KWin isn't installed, ends within 15 seconds, or Mantel doesn't say
  it's up within 45 seconds (shell.qml writes `$XDG_RUNTIME_DIR/hearth-shell-up`), Mantel opens on labwc for the rest
  of that sign-in and a card says why, once. Twice in a row, Mantel stays on labwc until you turn KWin back on (the
  card's "Use KWin again", or Settings › System › Your desktop). A switch you asked for is never counted. Safe
  graphics, a second player's seat and Server mode use labwc.
- **Your choice.** Settings › System › Your desktop › Window manager: KWin (the default) or labwc, from the next start of
  Desktop Mode (session.conf's `HEARTH_DESKTOP_WM`, written only when you pick).
- **For everything Mantel starts:** `HEARTH_WM=kwin|labwc` and `HEARTH_DESKTOP_PID` (the window manager's process id,
  for both). `LABWC_PID` is only ever set under labwc, and labwc's own commands (`labwc --reconfigure`, `labwc --exit`)
  only run there. nimbus-mode ends KWin with a signal to `HEARTH_DESKTOP_PID` (forced after 4 seconds, as before).
- **Hooks for the other KWin parts:** KWin also reads `desktop/kwin/windows/xdg` (the windows part's defaults: desktops,
  Mantel's keys), Mantel starts its bridge to KWin itself (`desktop/shell/hearth-kwin-bridge`), and the autostart runs
  `desktop/shell/hearth-screens restore` (screens) under KWin when it's there;
  wlr-randr and display.sh stay labwc's. Keyboard shortcuts (Super alone, Super+Alt+R...) are the windows part's.
- **Not yet under KWin** (their own branches): keyboard layouts and pointer settings applied live, screens, the
  magnifier and Mantel's own window list and workspaces through KWin.

## Hearth's own compositor (2.0, a preview)

For 2.0 Mantel gets a window manager that is Hearth's own (the owner, Nov 16): `hosts/hearth-compositor`, on Smithay.
It's off unless you turn on Settings › System › Your desktop › Hearth compositor (preview); then the session tries it
first and falls back to KWin (then labwc) with the same safe start. The plan, what it does so far and how it's tested:
[COMPOSITOR](COMPOSITOR.md).

## KDE Plasma and GNOME: retired in 1.14

1.13 let you download KDE Plasma or GNOME and use one instead of Mantel. The owner retired both on Nov 16, 2026
("archive the KDE and GNOME choices, since we're migrating to KWin"); how it worked and how to bring it back:
[archive/DESKTOPS-KDE-GNOME](archive/DESKTOPS-KDE-GNOME.md).

- If you had KDE Plasma or GNOME picked, Desktop Mode opens Mantel and a card says once why ("KDE Plasma and GNOME
  were retired"). Settings › System › Your desktop keeps showing that notice. Your session.conf isn't changed.
- KDE's or GNOME's apps you downloaded stay installed until you remove them; their own login entries stay hidden, so
  the login screen only offers Hearth's.

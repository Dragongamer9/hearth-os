# Game Mode beside the desktop (1.14, a test)

Off unless you turn it on: **Settings › Gaming › Steam Game Mode and Decky › Keep Steam Game Mode running beside the
desktop (test)**.

With it on, going to Steam Game Mode from Desktop Mode doesn't close the desktop. Steam Game Mode runs on its own
screen (the PC's 9th text screen, tty9) next to it, and switching only shows the other one: a second or two. Steam
stays open and signed in, which keeps about 1–1.5 GB of memory for it.

## How it works

- `hearth-side-by-side` (`/usr/libexec/nimbus`): `on`/`off` (session.conf's `HEARTH_SIDE_BY_SIDE` line only, through
  `hearth_settings_io`, under the lock, with .bak), `status`, `game` and `desktop`.
- `nimbus-mode steam` (or `game` when Steam Game Mode is your Game Mode) from Mantel asks `hearth-side-by-side game`
  first. The first time it starts `hearth-side-game@<you>.service`: a second sign-in of your own account on tty9,
  opened by systemd as a kiosk session is (no password, only the session part of sign-in), running `nimbus-session`
  with `HEARTH_SIDE=1`, which runs Steam Game Mode only, with its own next-mode file. Then it shows that screen
  (`loginctl activate`).
- Steam's Power › Switch to Desktop (steamos-session-select) shows the desktop's screen again; Steam keeps running.
- When Steam ends, the side session ends and the desktop is shown.
- Polkit (`49-hearth-side.rules`) lets you start and stop only your own instance, only from the active local session.

## Never stuck

- The unit has no `[Install]`: it can't start at boot or by itself.
- If Steam Game Mode doesn't come up within 25 seconds, or ends as it starts, or its screen can't be shown,
  `hearth-side-by-side game` stops it and `nimbus-mode` does the usual switch, so Game Mode always comes up.
- Turning it off closes Steam beside the desktop and switching is as before.

Each step is written to the session log as `side by side: …`.

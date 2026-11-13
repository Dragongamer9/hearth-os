# System Restore and Reliability history

Two Windows-style tools in Desktop Mode (Mantel), built to the approved mockups for 1.10. Both open from the app
menu, from Settings › Updates & recovery ("If something broke"), and by name: `hearth-restore` and
`hearth-reliability` (for the Run box and Super+X). They work with mouse, touch, keyboard and the controller.

## System Restore

Puts Hearth and its settings back to an earlier restore point.

### What a restore point is

- **The Hearth version** that was running when it was made (the bootc deployment it can go back to).
- **A copy of your Hearth settings files**: everything in `~/.config/nimbus` that is a `.json`, `.conf` or `.sh`
  file (session.conf, desk.json, osk.json, taskmgr.json, display.sh…), `~/.config/nimbus/input/*.json`,
  `~/.config/nimbus/theme/`, `~/.config/hearth/{drive-letters.json,theme.json,theme.css,seats-audio.json}`,
  `~/.config/labwc/{environment,rc.xml}`, `~/.config/mimeapps.list` and `~/.local/share/nimbus/desktop.json`
  (the files docs/SETTINGS-PERSISTENCE.md lists).
- **Never in it**: anything that looks like a secret (oauth, token, password…), a display mode on trial, learned
  keyboard words, `library.json` (play time and your games aren't settings), a file bigger than 1 MB, a link, a
  damaged settings file. Apps you installed and your own files are never part of a point and never touched.

Points live in `~/.local/share/hearth/restore/<id>/` (`point.json` and `files/`, readable only by you). Each is
built in a hidden folder and renamed into place whole. About 10 are kept (plus the two newest "Update" ones), so
they never fill the drive; none is made when the drive has under 200 MB free.

### When points are made

| Type | When |
|---|---|
| Update | Settings › Updates' Download / Update anyway makes "Before updating to Hearth X" first (detached, so it can never stop the update); the first time a new Hearth version runs (an update from Settings or the background one) makes "Hearth X" |
| Automatic | Before the writable system is switched (Settings › Updates › Unlocked system), and "Before restoring to …" right before a restore |
| Scheduled | Once a week |
| Manual | "Make a restore point now" |

The weekly point and the new-version point come from a **user** timer (`hearth-restore-points.timer`: 5 minutes
after signing in and once a day, low priority). It's a oneshot that copies a few small files; it can't hold up
starting the PC or the session.

### What will change

The right side shows it for the picked point: the Hearth version (going back, staying, or why it can't), how many
settings changed since then ("See every changed setting" lists each one: file › key, now and then; files made since
are listed as kept), that apps stay, and that your files are not touched.

### Restore and restart

`hearth_restore.py apply` does, in order:

1. a restore point of today's settings ("Before restoring to …"), so you can come back;
2. the point's settings files back, each through `hearth_settings_io` (under the file's lock, atomic, today's copy
   in `.bak`); only files on the list whose copy is intact; files made since are left as they are;
3. only when the point's version is **exactly the one bootc keeps** (the rollback deployment), nothing waits for a
   restart and no rollback is queued: Settings › Updates' own `pkexec /usr/libexec/nimbus/hearth-admin rollback`
   (bootc rollback, an argument list). If it fails or doesn't report the rollback as queued, step 2 is undone from
   step 1 and nothing has changed;
4. a restart (`systemctl reboot`) when the version goes back.

| The point's version | What happens |
|---|---|
| the one running now | settings only, no restart needed |
| the one bootc keeps | settings, then `hearth-admin rollback`, then restart; the version you came from stays in the boot menu (and in Settings › Updates as "Go back"), so you can come back |
| already queued to start next | settings, then restart |
| kept, but an update waits for a restart | refused (bootc refuses a rollback then); "Restore settings only" is offered |
| no longer kept (bootc keeps two) | it says so; settings only |
| the system couldn't say | settings only |

Nothing in System Restore changes booting by itself: no new privileged command, no `bootc switch`, no edits to the
boot menu. The only boot change is the existing, documented `bootc rollback` through the existing helper and polkit
rule, the same as Settings › Updates › Go back. Opening System Restore writes nothing (it reads the points and asks
`hearth-admin update-status`, read-only); settings go back only after you confirm.

## Reliability history

How Hearth has been running, day by day (or week by week): a stability line from 10 (steady) to 1 (rocky) and a mark
per day for each kind of event. Click a day (or use Left / Right) to list its events, with **Details** and, for
problems, **Send logs** (Hearth's own `hearth-send-logs`, the same as Settings › Backup and help, with the event as
the note; you get a code to share).

| Row | From |
|---|---|
| App stopped working | crashes: systemd-coredump's entries in the system journal (`MESSAGE_ID=fc2e22bc…`) |
| Hearth problems | the session's fallbacks and watchdog lines (`~/.local/share/nimbus/session.log`), the reasons it leaves (`$XDG_RUNTIME_DIR/hearth-*-mode-failed`, `~/.local/state/nimbus/steam-default-off`, `hearth-gm-off`), Hearth's own services that failed (`hearth-*`, `nimbus-*`) |
| Warnings | network drops from NetworkManager, kept modest: not when the PC sleeps or you disconnect, and drops within an hour on one device are one warning |
| Updates and changes | restore points: a new Hearth version, "Before updating…", restores |

The stability line: each day starts from the day before plus 0.5 (up to 10) and drops by 1.2 per crash, 2 per Hearth
problem and 0.4 per warning (at most 4.5 a day), never below 1.

It only reads. The journal is read when the window opens, with journalctl's own filters, and only for days it
hasn't seen: each finished day's summary is kept in `~/.cache/hearth/reliability.json` (a cache, not a setting;
days older than 120 are dropped), so the next opening reads just today.

## Files

- `hosts/linux/desktop/shell/hearth_restore.py`, `Restore.qml`, `RestoreView.qml`
- `hosts/linux/desktop/shell/hearth_reliability.py`, `Reliability.qml`, `ReliabilityView.qml`
- `os/system_files/usr/bin/hearth-restore`, `hearth-reliability`; `usr/share/applications/hearth-restore.desktop`,
  `hearth-reliability.desktop`; `usr/lib/systemd/user/hearth-restore-points.{service,timer}`
- Tests: `hosts/linux/test/test_restore_reliability.py` (desktop-test.yml)

# Notifications (Mantel, 1.14)

Hearth's own notification server (`hosts/linux/desktop/shell/Notifications.qml`) with KDE Plasma's features written
for Hearth, the same on KWin and labwc. CI: `.github/workflows/notify-widgets.yml`, `test_notify_widgets_114.py`.

- **History kept across restarts**: `~/.local/state/nimbus/notifications.json`, one key per notification, through
  SettingsWriter (atomic, `.bak`; a damaged file is put back). Settings › Notifications › Keep the list after a
  restart (on). A notification stays alive while it's in the list, so **its buttons work from the history**; after a
  restart its app can't hear them, so only "Open <app>" is left.
- **Automatic Do Not Disturb** (NotifyHub.qml, `~/.config/nimbus/notify.json`): while a full-screen app is in front
  (on), while the screen is shared or recorded (a screencast, Hearth's recorder, Remote desktop; on), and on a
  schedule (off). It never changes your Do not disturb switch; the bell shows why, and when it's over a note says how
  many came in. Super+Alt+N switches Do not disturb.
- **Hearth's own warnings are never blocked** (NotifyRules.isHearthWarning: urgent ones from any Hearth app, and
  normal ones from Hearth itself or with the `hearth.warning` category): not by an app rule, muting, Do not disturb,
  automatic quiet or low priority. Hearth's apps can't be muted.
- **Job progress**: `hearth-notifyd` serves `org.kde.JobViewServer` (V1 and V2) when nobody else has it; copies and
  downloads show in the tray (a ring) and the clock's panel with Pause / Cancel, and leave a notification when done.
  A notification's `value` hint (a portal's or an app's download) is a progress bar on it.
- **Dock badges** (the Unity launcher API, `com.canonical.Unity.LauncherEntry`): a count, progress and urgent on the
  app's dock icon; a running job's progress too.
- **Sounds** (Sounds.qml, Settings › Sound › Sounds, `~/.config/nimbus/sounds.json`): Hearth's own short sounds
  (`desktop/sounds/hearth`, made by `tools/make-hearth-sounds.py`, GPL-3.0), or the freedesktop theme if installed
  (played, never copied: `desktop/sounds/LICENSES.md`), or none; each event on or off with Play.
- **OSDs** (SystemOsd.qml): Caps Lock / Num Lock (the keyboards' LEDs), the keyboard layout (KWin's
  `org.kde.keyboard`; on labwc, Settings › Keyboard's list), the microphone muted, and an app that wants a muted mic.

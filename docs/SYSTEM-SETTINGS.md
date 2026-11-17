# Hearth Settings: the system pages (1.14)

KDE System Settings' areas Hearth didn't have, written as Hearth's own pages (the owner, Nov 16: everything Plasma has,
from scratch). Each page is a `Set*.qml` in `hosts/linux/desktop/shell` with a small Python helper next to it; tests in
`hosts/linux/test/test_system_settings_114.py`; screenshots from desktop-test.yml's `system-settings-shots` job.

| Page | Where | Helper | Keeps |
|---|---|---|---|
| VPN, Mobile hotspot, Connection details | Network & internet | `hearth-network` | nothing: NetworkManager's own (secrets only on nmcli's stdin) |
| Proxy | Network & internet | `hearth-network` | `~/.config/nimbus/proxy.json` |
| Firmware & device security | Privacy & security | `hearth-firmware`, root: `hearth-sysset-admin` | nothing (fwupd's) |
| Region & language | System | `hearth-region`, root: `hearth-sysset-admin locale` | `/etc/locale.conf` (localectl) |
| Fonts, cursor & icons | Personalization | `hearth-lookset` | `~/.config/nimbus/fonts-cursor-icons.json` |
| Background services | Apps | `hearth-services` | nothing (systemd's enable/disable/mask) |
| Login screen | Accounts | `hearth-loginscreen`, root: `hearth-sysset-admin login-background` | `/var/lib/hearth/login-screen/background` |
| Thunderbolt | Bluetooth & devices | `hearth-thunderbolt` | nothing (bolt's) |
| Sound: apps, card profiles, mic test | System › Sound | `hearth-sound` | nothing (PipeWire's; the mixer's levels as before) |
| Power: charge limit | System › Power | `hearth-hwset` (Task Manager's) | `/etc/hearth/hardware.json` |
| Notifications for each app | System › Notifications | `hearth-notify`, read by `NotifyRules.qml` | `~/.config/nimbus/notify-apps.json` |
| Reopen my apps from last time | System › Startup | `SessionRestore.qml` | `restoreApps` in desktop.json (off by default); the list in `~/.local/state/hearth/open-apps.json` |

## Firmware: what Hearth installs (red flag: never leave a device unable to start)

One rule decides "system firmware", in `hearth-firmware` and `hearth-sysset-admin` alike (`is_system_firmware`): fwupd's
UEFI plugins, flashrom, or an internal part flagged needs-reboot. In 1.14 Settings **shows** a system firmware update
but **never installs** it: that firmware changes how the PC starts. The page says to back up first and use the PC
maker's tool or the firmware setup; `hearth-sysset-admin firmware-update` refuses it too. Other parts (receivers, docks,
drives) install after an administrator's password (49-hearth-admin.rules), and nothing restarts by itself.

## Notifications

An app turned off is dropped completely. Hearth's own notifications (app names starting with Hearth, Nimbus, Mantel or
Task Manager: lock warnings, a settings file put back from its .bak, pairing requests) can't be turned off
(`NotifyRules.isHearth`, and `hearth-notify` refuses it), and Hearth's own urgent ones always pop up.

**Lock screen:** "Show on the lock screen" is shown by Mantel's lock screen on labwc (`Lock.qml`). KWin's lock screen
(feature/kwin-input's `kwin-lock/LockScreen.qml`) doesn't show these notifications yet.

## Reopen my apps

Uses `WinList` (feature/kwin-windows: KWin or labwc) when the shell has it, else Quickshell's ToplevelManager. Only at
sign-in, once per session, and only if it was already on; never Steam, games or Hearth's own windows.

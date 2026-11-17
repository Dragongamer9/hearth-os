# More of KDE's System Settings in Hearth Settings (1.14)

The owner's list (Nov 16, 2026): KDE System Settings features Hearth Settings lacked, written from scratch in Hearth's
look. Every page has a picture from the Desktop Mode test (`desktop-test.yml`, the `system-settings-shots` job, `ss-*`).
Tests: `hosts/linux/test/test_settings_more_114.py`. The master list with what's done is `docs/KDE-FEATURES-1.14.md`.

## What's where

| Area | Page (settings id) | File | Helper | What it changes, only when you press |
|---|---|---|---|---|
| Sound | Speakers & balance (`sound-speakers`) | SetSoundSpeakers.qml | hearth-sound `outputs`, `balance`, `speaker-test` | PipeWire's own volumes (it remembers them) |
| Sound | Apps' sound (`sound-apps`) | SetSoundApps.qml | Mixer.qml | an app's output (WirePlumber), `dockScrollVolume` |
| Sound | Push to talk (`push-to-talk`) | SetPushToTalk.qml, PushToTalk.qml | hearth-ptt (KWin's global shortcuts) | `pttOn`, `pttKey` |
| Sound | Over-amplification asks first | SetSound.qml | — | `buttons.json` overamp (as before) |
| Power | Plugged in, battery & lid (`power-profiles`) | SetPowerProfiles.qml | hearth-powerset, hearth-admin `lid` | `screenOffBattery`, `sleepBattery`, hardware.json `power.plugged/unplugged`, `/etc/systemd/logind.conf.d/60-hearth-lid.conf` |
| Power | Apps keeping it awake (`power-awake`) | SetPowerAwake.qml | hearth-powerset | nothing |
| Power | Hibernate (on Power) | SetPower.qml | hearth-powerset `hibernate_check` | nothing (offered only where it can; booting never changed) |
| Power | Battery health & history (`battery`, `info-energy`) | SetBattery.qml | hearth-powerset `history` | nothing |
| Power | Device batteries | QuickAccess.qml, SetBattery.qml | UPower | nothing |
| Apps | Autostart (`autostart`) | SetAutostart.qml | hearth-autostart | `~/.config/autostart/*.desktop` |
| Apps | File types (`file-types`) | SetFileTypes.qml | hearth-mime | `~/.config/mimeapps.list`, `~/.local/share/mime/packages/hearth-user.xml` |
| Apps | App menu editor (`menu-editor`, "Edit app…") | SetMenuEditor.qml | hearth-menu | your copy in `~/.local/share/applications` (X-Hearth-Edited) |
| Apps | App permissions (`app-permissions`) | SetAppPermissions.qml | hearth-flatpak-perms | Flatpak's overrides for you |
| Look | Wallpaper per screen & slideshow (`wallpaper-more`) | SetWallpaperMore.qml, WallShow.qml | — | `wallScreens`, `wallShow` |
| Look | My looks (`my-looks`) | SetMyLooks.qml | hearth-lookset | `~/.config/nimbus/my-looks.json` |
| Look | Small and window title fonts (`fonts:small`) | SetFonts.qml | hearth-lookset | fonts-cursor-icons.json (smallFont, smallSize, titleFont, titleSize) |
| Look | Window frames & sign-in splash (`look-more`) | SetLookMore.qml, SigninSplash.qml | hearth-theme, hearth_aurorae.py | `frameTint`, `signinSplash` |
| Region | Languages in order (`languages`) | SetLanguages.qml | hearth-region | `~/.config/environment.d/60-hearth-language.conf`, plasma-localerc |
| Region | Spell check (`spellcheck`) | SetSpellcheck.qml | hearth-region | `~/.config/KDE/Sonnet.conf` [General] |
| Region | Chinese, Japanese & Korean typing (`input-methods`) | SetInputMethods.qml | hearth-region `im*` | `~/.config/kwinrc` [Wayland] InputMethod, environment.d; fcitx5 from Flathub only when asked |
| Updates | Shut down after updating | SetUpdates.qml | — | nothing saved (this update only) |
| Updates | Firmware notification | FirmwareWatch.qml, SetFirmware.qml | hearth-firmware | `firmwareNotify` |
| Updates | Backups | Home server › Backups & sync, linked from Updates | — | (as before) |
| Devices | Your Bluetooth devices (`bluetooth-devices`) | SetBtDevices.qml | hearth-bt | BlueZ's own trust / block |
| Devices | Print queue (`print-queue`), Scanners (`scanners`) | SetPrintQueue.qml, SetScanners.qml | hearth-printq | your print jobs |
| Devices | USB devices & drives (`usb-devices`) | SetUsbDevices.qml, DeviceWatch.qml | hearth-usbdev | `removable`, `usbNotify`; USBGuard only where it runs |
| Devices | Touchscreen calibration | KwinCalibrate.qml (feature/kwin-pages) | hearth-kwin-settings | KWin's device setting |
| Info | Info Center (`info-cpu`, `info-gpu`, `info-memory`, `info-graphics`, `info-pci`, `info-usb`) | SetInfoCenter.qml | hearth-info | nothing |
| Info | Out-of-memory notice | OomWatch.qml | — | `oomNotice` |
| Info | Welcome once per version | Welcome.qml, welcome.json | — | `welcomeAfterUpdates`; seen in `~/.local/state/nimbus/welcome.json` |

## The owner's rules

- **Nothing can stop Desktop Mode from starting** (red flag 2): the watchers (PushToTalk, FirmwareWatch, OomWatch,
  DeviceWatch, Welcome, SigninSplash) are loaded by file name, so a problem in one only loses it. The sign-in splash
  lets clicks and keys through and is gone after 2.5 s. Hibernate is offered only where it can work, and Hearth never
  changes booting for it. The lid setting reloads logind, never restarts it.
- **Settings never change by themselves** (red flag 9): new desktop.json keys are merged one at a time
  (Settings.qml), missing ones are defaults never written; opening a page writes nothing (the test checks). Files that
  aren't Hearth's (kwinrc, Sonnet.conf, mimeapps.list, plasma-localerc) get only their one key changed, every other
  line kept, through hearth_settings_io (atomic, .bak). A system autostart entry or menu entry is never touched: your
  own copy is written beside it, and Put back removes only a copy Hearth made.
- Push to talk and fcitx5 typing need Mantel on KWin; under labwc their pages say so.

## Left for later

Per-app microphone, hybrid sleep, receiving files over Bluetooth and renaming devices, ink notices and Windows shared
printers, multi-page scanning, kernel modules / interrupts / Samba / sensors in Info Center, picture of the day and a
light/dark wallpaper, sub-pixel order, "apps follow Hearth", downloading translations, firmware history.

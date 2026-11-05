# Hearth Link in Steam Game Mode

Hearth Link (`hosts/decky-hearth-link`) is a small Decky plugin for Steam Game Mode's Quick Access. It shows what
Hearth Link (`hosts/hearth-link`, docs/HEARTH-LINK.md) does on this device: your other Hearth devices, games copied
between them, Hearth Storage and pairing. Everything works with a controller. Nothing of Steam's screens is changed.

The image ships it next to Hearth Sync (docs/HEARTH-SYNC.md): it's in `/usr/share/hearth/decky-hearth-link`, and
`hearth-decky` copies it into Decky's plugins folder when Decky is installed from Settings › System and brings it up to
date when Steam Game Mode starts (its `PLUGINS` list). An image built without it simply goes without it.

## The panel

**Home server** (only on a PC that's a home server: set up, sharing or backing up; docs/HOME-SERVER.md). This PC's
name, its shares, space and people, the backups in a line (last good one, waiting for your game, didn't finish), what's
wrong (alerts), **Light mode while you play**, what Light mode holds back right now, and **Let paused jobs run now** /
**Pause them again while I play**. It asks Mantel's home server helper only `quick`, `quick-set lightMode on|off` and
`resume on|off` (main.py `home_server`, an allow-list, a 10-second timeout, as you, never a password); when the helper
isn't there or doesn't answer the section isn't shown, and its own error boundary keeps a problem in it from touching
the rest of the panel.

**Devices.** This device, then each paired device: online on this network, online over the internet, or offline with
when it was last seen. A sleeping Hearth Storage device that Hearth Link knows how to wake gets **Wake**. An online
device gets **Get a game from …**, which lists the games on it; pick one to copy it here.

**Send a game.** Pick a game installed here and an online device, then **Send**. The other device copies it from this one
(it may ask first, unless it always accepts this device); while it does, the panel shows how much has gone over. The
receiving device can pause or cancel it.

**Copies.** Games coming to this device, with progress, speed and time left, **Pause**, **Resume** and **Cancel**; a game
another device offers, with **Get it** and **No thanks**. When a copy is done:
- a Steam game comes with its appmanifest, which Steam reads when it starts: **Restart Steam to see it** (Steam's own
  restart; when Steam doesn't have that call any more, the panel says to restart Steam from its Power menu);
- any other game is in Hearth's library, and Hearth Sync adds it to Steam's library at its next check.

**Hearth Storage** (only when there is a Hearth Storage device, or this is one). The storage device (online or asleep,
free space, **Wake**), **Sync now** (your Hearth data with every paired device), download once (games kept on Hearth
Storage that this device hasn't got, with **Get …**, and their copies), saves that follow you (each game's last sync,
and saves that differ), the update cache (the Hearth OS update kept, from which device) and device health (how many
devices are on, and low battery, low space, an older Hearth OS or an old backup).

**Sync between your devices** (when a device is paired). Three switches, each this device's own:
- **Sync Decky plugins** (off until you turn it on). Your Decky plugins' settings follow you: `settings*.json` and
  `config.json` in `~/homebrew/settings/<plugin>`, one key at a time; a key that looks like a sign-in (token, password,
  secret, api key, auth, cookie, session…) or holds one is never shared and never replaced. Hearth's own plugins
  (Hearth Sync, Hearth Link, Hearth Music) are on every device already, so only Hearth Sync's `settings.json` travels
  here (Hearth Link's own switches and state stay on each device; Hearth Music's panel settings go with **Sync Hearth
  settings** below). The plugins your other devices have and this one hasn't are listed with
  **Install …**: Hearth Link copies that plugin's folder from a device that has it (`decky.fetch`, from the device that
  listed it first, then any other online one), the files are checked again and packed as a zip, and Decky Loader's own
  install is asked for it, which shows Decky's **Install …?** prompt: a plugin from another device is only installed
  when you say yes here (Decky runs plugins as root). Nothing is uninstalled because another device removed it.
- **Saves follow you**: Hearth Link's existing saves-follow setting (Hearth Storage), shown as it is and changed here
  (`storage.saves.set`, sent as `storage.set {savesFollow}` only). Steam Cloud games already sync through Steam.
- **Sync Hearth settings** (off until you turn it on). These follow you: Mantel's look, light or dark, accent colour,
  Hearth's own wallpapers (not a picture file of yours), reduce motion, 24-hour clock and night light
  (`desktop.json`); the on-screen keyboard's look and typing choices (`osk.json`, not its position); Game Mode's
  appearance and button swaps (`game-mode-ui.json`); Hearth Player's shuffle, repeat, cue and subtitles (not its
  volume); Hearth Music's panel (`~/homebrew/settings/decky-hearth-music/settings.json`: on or off, volume, delay, in the
  library too, Apple's previews and the now-playing note; never the songs you picked per game, which are file paths);
  the power button's press and hold (`buttons.json`). These stay on each device: screen modes
  (`session.conf`, `display.sh`), sound devices, volume and louder-than-100%, HHD, TDP and power, network, pointer
  speed, power timers and lock on wake.

How: each key carries the time it changed, and the newest change wins, per key (never a whole file). This device's
values go into Hearth Link's `link/prefs.json` ("local"), which Hearth Link's own sync carries as fields
`g/x/<group>/<key>` (sync.rs); what another device changed comes back in "remote" and is merged into the latest file,
that key only, under the file's lock, atomically, with `FILE.bak` (`hearth_settings_io`, docs/SETTINGS-PERSISTENCE.md;
the plugin ships a copy). With a switch off this device shares nothing for it and takes nothing. The first time a
switch is turned on, whichever is newer wins for each key: the file here (its last change) or the other device's. A
removed key is never removed elsewhere. Rounds run every 30 seconds while Steam Game Mode (Decky) runs and Hearth Link
is on; changes made in Desktop Mode go out at the next round in Game Mode. An app that keeps its settings in memory
(Hearth Player) sees a change the next time it starts.

**Pair a device.** **Show this device's code** shows the 6-digit code to type on the other device (and, with **Also
allow a device somewhere else**, an internet code). A nearby device showing its code appears as **Pair with …**: pick it
and type its code. **Enter an internet code** pairs with a device elsewhere. There's no QR code: pairing is by typing
the code, on both sides.

**When Hearth Link isn't running.** The panel says so, offers **Start Hearth Link** (your own `hearth-link.service`) and
shows the devices as this device last saw them, read only.

Settings that change or remove things stay in Settings › Hearth Link (in Hearth, or Mantel's Hearth Link page in Desktop
Mode): unpairing, turning Hearth Storage and its parts on or off (except saves that follow you, above), shared folders
and Windows sharing, removing a device from Hearth Storage.

## How it talks to Hearth Link

The backend (`main.py`) runs as you inside Decky and only talks to Hearth Link's own socket,
`$XDG_RUNTIME_DIR/hearth-link/link.sock` (its folder must be yours and closed to others, the socket yours, and the other
end is checked to be you too, the same checks as Hearth's core service socket). Only these commands are sent, each with
only its checked arguments:

| Panel | Hearth Link command |
| --- | --- |
| Devices, copies, offers, pairing state | `status` (cut short: never the device's ticket, relay, settings or addresses) |
| Get a game from … | `games {peer}`, then `get {peer, game}` |
| Send a game | `local.games`, `send {peer, game}` |
| Copies | `transfer.pause`, `transfer.resume`, `transfer.cancel {id}`, `transfer.clear`; `offer.accept`, `offer.decline {id}` |
| Wake | `storage.wake {server}` |
| Sync now | `sync.now` |
| Hearth Storage | `storage.cache.status`, `storage.cache.get {game}`, `storage.saves.status`, `storage.updates.status`, `storage.devices` |
| Pairing | `pair.open {internet}`, `pair.close`, `pair.join {code, id}` |
| Saves follow you | `storage.saves.set {follow}` → `storage.set {savesFollow}` (a real true or false only) |
| Install … (a plugin from another device) | `decky.fetch {peer, folder}` (not from the panel: the backend's `fetch_plugin`; the other device answers `decky.plugin`, decky.rs: its regular files, no links, at most 40 MB, never Hearth's own plugins) |

Device ids are 64 hex digits, copy and offer ids hex, game ids Hearth's `store:rest` ids, codes 6 digits (or an internet
code's letters and digits). Hearth's core service (`core_service.rs`) is unchanged: its `link` command stays for the
older plugin. Hearth Link's status now says whether a device is on this network (`lan`); an older Hearth Link that
doesn't just shows "Online".

## Tests

`.github/workflows/decky-hearth-link.yml`: the backend's tests (`tests/test_main.py`: the allow-list, the short status
without secrets, the read-only view, a socket that isn't yours, a stand-in Hearth Link on a real socket;
`tests/test_sync.py`: the switches off by default and nothing changing then, newest change per key, device settings
staying, sign-ins never shared, the safe writes, a plugin's files checked and packed for Decky's install), the pure
parts' unit tests (`tests/model.test.mjs`), the build and zip, pictures of every state of the real panel over stand-ins
(`tests/render-panel.mjs`, onto the `ci/hearth-link-shots` branch), the Decky helper's tests and hearth-link's unit
tests.

## Not verified yet

On a device: the panel in Steam's Quick Access (only drawn over stand-ins so far), Decky's dropdown and text field with
a controller, Steam's restart call, how soon Hearth Sync picks up a non-Steam game copied here, and `systemctl --user
start` from inside Decky, the sync switches between two real devices, and Decky's install of a plugin zip from
another device (`utilities/install_plugin` with a `file://` artifact, which Decky Loader's own code supports).

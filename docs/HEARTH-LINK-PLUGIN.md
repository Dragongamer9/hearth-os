# Hearth Link in Steam Game Mode

Hearth Link (`hosts/decky-hearth-link`) is a small Decky plugin for Steam Game Mode's Quick Access. It shows what
Hearth Link (`hosts/hearth-link`, docs/HEARTH-LINK.md) does on this device: your other Hearth devices, games copied
between them, Hearth Storage and pairing. Everything works with a controller. Nothing of Steam's screens is changed.

The image ships it next to Hearth Sync (docs/HEARTH-SYNC.md): it's in `/usr/share/hearth/decky-hearth-link`, and
`hearth-decky` copies it into Decky's plugins folder when Decky is installed from Settings › System and brings it up to
date when Steam Game Mode starts (its `PLUGINS` list). An image built without it simply goes without it.

## The panel

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

**Pair a device.** **Show this device's code** shows the 6-digit code to type on the other device (and, with **Also
allow a device somewhere else**, an internet code). A nearby device showing its code appears as **Pair with …**: pick it
and type its code. **Enter an internet code** pairs with a device elsewhere. There's no QR code: pairing is by typing
the code, on both sides.

**When Hearth Link isn't running.** The panel says so, offers **Start Hearth Link** (your own `hearth-link.service`) and
shows the devices as this device last saw them, read only.

Settings that change or remove things stay in Settings › Hearth Link (in Hearth, or Mantel's Hearth Link page in Desktop
Mode): unpairing, turning Hearth Storage and its parts on or off, shared folders and Windows sharing, removing a device
from Hearth Storage.

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

Device ids are 64 hex digits, copy and offer ids hex, game ids Hearth's `store:rest` ids, codes 6 digits (or an internet
code's letters and digits). Hearth's core service (`core_service.rs`) is unchanged: its `link` command stays for the
older plugin. Hearth Link's status now says whether a device is on this network (`lan`); an older Hearth Link that
doesn't just shows "Online".

## Tests

`.github/workflows/decky-hearth-link.yml`: the backend's tests (`tests/test_main.py`: the allow-list, the short status
without secrets, the read-only view, a socket that isn't yours, a stand-in Hearth Link on a real socket), the pure
parts' unit tests (`tests/model.test.mjs`), the build and zip, pictures of every state of the real panel over stand-ins
(`tests/render-panel.mjs`, onto the `ci/hearth-link-shots` branch), the Decky helper's tests and hearth-link's unit
tests.

## Not verified yet

On a device: the panel in Steam's Quick Access (only drawn over stand-ins so far), Decky's dropdown and text field with
a controller, Steam's restart call, how soon Hearth Sync picks up a non-Steam game copied here, and `systemctl --user
start` from inside Decky.

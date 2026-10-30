# Hearth Link

Hearth Link (`hosts/hearth-link`, the `hearth-link.service` user unit) pairs your Hearth devices, keeps Hearth's
data in sync between them and copies games from one to another. This page is about **pairing**; the crate's
`src/lib.rs` describes the rest (sync, copies, relays).

Settings › Hearth Link is the same in Game Mode (`ui/index.html`, `pageLink`) and Desktop Mode (Mantel's
`SetLink.qml` through `hearth-link-settings`). Everything works with a controller.

## On the same network: pick it, type 6 digits

1. **Device A:** Settings › Hearth Link › **Pair a device**. A shows a 6-digit code (for 5 minutes) and says in its
   network announcements (UDP broadcast, port 47741, every 2 seconds while pairing) that it's ready to pair.
2. **Device B:** Settings › Hearth Link lists the Hearth devices on the network by itself, the ones ready to pair
   first, each with its name, kind (handheld, laptop, desktop...) and edition (Anatase, Peppermint...). Pick A.
3. B asks for the code with the number pad. Type the 6 digits shown on A. Done: both say **Paired ✓**.

Nothing is typed but the 6 digits. A device that isn't ready says "select Pair a device on it first".

## Somewhere else: a 10-character code

1. **Device A:** Pair a device, then **Not on the same network?** A shows a code like `K7M-4QX-92HC` (10 minutes,
   works once).
2. **Device B:** Settings › Hearth Link › **Enter a code**, and type it. That's all: no link, no id.

Typing is forgiving: spaces, dashes and small letters don't matter, the dashes appear as you type, and the code
never has 0, O, 1, I or L in it. The last character is a check character, so a mistyped character (or two swapped)
shows "That code doesn't look right" right away, before anything goes over the network.

## What each screen says

| Device A (showing the code) | Device B (typing it) |
| --- | --- |
| Waiting for the other device… | Looking up the code… (short codes) |
| Found "Nvidia-PC": checking the code… | Found "Nvidia-PC": connecting… / Checking the code… |
| Nvidia-PC typed a wrong code (3 tries left) | Wrong code. 3 tries left (the code is asked for again) |
| Paired ✓ with Nvidia-PC | Paired ✓ with Nvidia-PC |
| Too many wrong codes, so pairing stopped | Wrong code. That was the last try: pairing stopped on … |
| The code ran out | That code expired / That code was used, stopped or ran out |

A wrong code never means starting over: B keeps the device picked and asks for the code again.

## Other ways

**Other ways to pair** keeps the long link from before (the device's id and relay, under Other ways › Show this
device's long link): type it on the other device, then the 6-digit code. Useful when neither the network list nor a
short code works (for example a lookup service that can't be reached).

## How a short code finds the other device (no Hearth server)

- `shortcode.rs`: 9 random characters from a 31-character alphabet (about 45 bits) and a check character (the
  weighted sum of all 10, weights 1 to 10, is 0 modulo 31).
- From the code, both devices derive (200 000 rounds of BLAKE3, so trying codes is slow) an Ed25519 key pair and a
  sealing key.
- `lookup.rs`: A publishes a small signed pkarr record under that key's public half on public pkarr relays: n0's
  (`dns.iroh.link`, which iroh itself uses for address lookups) and pkarr's own (`relay.pkarr.org`, which also puts it
  on the Mainline DHT). It's sealed (BLAKE3 stream and MAC) and holds A's device id, name, kind, relay and local
  addresses. A publishes it again every 2 minutes while the code shows, and replaces it with "gone" when the code is
  used, stopped or runs out. `HEARTH_LINK_PKARR` (comma-separated URLs, or `off`) replaces the relays.
- B derives the same key, asks the relays, opens the record and connects to A (directly, hole punched, or through
  the relay, as any Hearth Link connection).

## Security

- Pairing is always SPAKE2 with the code as the password, bound to both devices' keys as QUIC's TLS proved them,
  then a confirmation each way (`pair.rs`). Finding A (on the network, or through the record) grants nothing.
- **Guessing:** A takes one pairing attempt at a time and stops pairing after **5 wrong codes** (both codes count
  together); a new code is needed after that. A guesser gets 5 tries at 1 in a million (6 digits) or 1 in 26 trillion
  (short code).
- **The record:** without the code nobody can find it (its address is the derived key), read it (sealed) or replace
  it (signed with the derived key). Working the code out from a record would mean trying up to 31⁹ codes at 200 000
  BLAKE3 rounds each, inside the 10 minutes the code lives; and the code is used up the moment B pairs.
- **What's public:** a lookup service sees that some key published a small sealed record, and B's address when it
  asks. Device ids and relays aren't secrets (the long link always contained them).
- **Logs:** every step is in the journal (`journalctl --user -u hearth-link`, lines `hearth-link: pairing: ...`):
  opened, a device showing a code on the network, the short code published or withdrawn, found, connected, wrong
  code (tries left), stopped after too many, paired. Codes are never logged.

## Calls (the service's socket, `hearth-link call`)

- `pair.open {internet?: bool, seconds?: n}`: shows the 6-digit code; `internet` adds a short code (published
  first, so an error means the lookup services couldn't be reached). Answers the `pairing` status.
- `pair.close`: stops pairing (and withdraws the short code).
- `pair.join {id, code}`: a device picked on the network (or its long link) and its 6-digit code.
- `pair.join {code}`: a short code alone.
- `pair.check {code}`: checks a typed short code without the network.
- `status`: `pairing {code, until, triesLeft, netCode, netUntil}`, `pairEvent {state, name, triesLeft}` (this
  device's side), `joining {state, name}` (while `pair.join` runs), `nearby [{id, name, kind, edition, pairing}]`,
  `me {kind, edition, ticket}`, `shortCodes`.

Devices from 0.6.5 still pair with newer ones with the 6-digit code (they show no kind or edition, and can't use
short codes).

## Tests

- `cargo test` (hosts/hearth-link): the alphabet and check character (every single wrong character and swap is
  caught), sealing, the record's pieces, pairing state and expiry, SPAKE2.
- `tests/e2e.py lan` (the Hearth Link workflow): two devices in two network namespaces and a local pkarr relay:
  5 wrong codes stop pairing, a code that ran out, a short code (typo caught at once, a code nobody shows, pairing,
  the used code refused, one that ran out), the long link, then picking the device from the network list (kind and
  edition checked) with a wrong code first and the right one after; the logs checked for every step and no codes.
- `tests/e2e.py internet`: pairing across two NATs from the short code alone.
- `tools/ui-check.js` (release.yml): the Game Mode screens, the number pad, a wrong code asked again, short codes
  checked and formatted as typed. `hosts/linux/test/test_hearth_link_settings.py`: Mantel's helper.

# Hearth Storage

A spare PC running Hearth as a small home server for your other Hearth devices (`hosts/hearth-link/src/storage.rs`).
It is Hearth Link: the same pairing, the same encrypted connections, the same copy engine. Only paired devices can
use it, and nothing is opened to the internet unless you turn something on.

**On the server:** Settings › Hearth Storage › **Use this device as Hearth Storage** (off by default). Then:

- **Shared folders**: pick folders in Home or on a drive. Your devices list them (Game Mode's Hearth Storage page,
  Hearth Files' "Hearth Storage" places as `hearth://…`), copy from them and play videos and music from them. Over
  Hearth Link they're read-only; hidden files (name starting with a dot) are never listed or given out, and a link
  pointing out of the shared folder is refused.
- **Game archive**: a device moves a game here ("Move to Hearth Storage" in the game's menu). The server copies it
  with its saves (`Hearth Storage/Archive/…`); when it's all there the device checks it against its own copy and
  says it's safe to uninstall. "Copy it back" brings it back as the store's own game (Steam's appmanifest, Epic,
  GOG, Beacon), only sending what isn't there already.
- **Backups**: each device that picks this server ("Back up saves and screenshots") has them copied here every few
  hours (`Hearth Storage/Backups/<device>/Saves|Screenshots`): Steam Cloud's local copies, every game's Windows user
  folder (caches and Temp left out), Hearth's and Steam's screenshots. Only changed files are sent; files deleted on
  the device stay in the backup.
- **Away from home** (off by default): the server only answers devices on the local network. On, it also answers
  them over Hearth Link's own connection (direct, or through the relay, end-to-end encrypted). No port is opened.
- **Windows sharing (SMB)** (off by default): the shared folders for Windows PCs on this network, signed in as your
  user with a password set on that page. `hearth-storage-helper` (root, through pkexec and 49-hearth-storage.rules)
  writes Samba's config; the firewall opens TCP 445 to local addresses only (`hearth-link-firewall open smb`), again
  at every start while it's on. Needs Samba installed on the server.
- **Sleep when idle** (off by default): after N minutes with no copies, no game and the session idle (logind), it
  suspends. Turning the server on sets its wired network cards to wake from the network (ethtool, and
  NetworkManager so it lasts); the PC's firmware may need Wake on LAN turned on too.

**On the other devices:** a server that doesn't answer is sent a Wake-on-LAN packet (its cards' addresses are
remembered from the last time it answered) and waited for up to about a minute. Media plays through a link on the
device itself (`http://127.0.0.1:<port>/<random>/<name>`, one link per file, 12 hours) that Hearth Player opens; the
device fetches the bytes from the server over Hearth Link as the player asks for them.

**Saves follow you** (`saves.rs`, on by default once a device backs up to a server; Backups & saves › Saves follow
you): before a game starts, Hearth asks the server (at most 2.5 seconds, never waking it; then the game starts as
always) whether it has a newer save for that game, and if so copies it over the one here first; after the game, the
save here goes to the server. Only games whose store doesn't sync saves itself: Steam games without Steam Cloud, Epic
games without Epic's cloud saves, GOG games without GOG's cloud saves (or with Cloud saves turned off for the game),
Beacon's games. A game's save is its Windows user folder in its own prefix (caches left out). Each device remembers
the server version it last had and what its save looked like then; when both changed, both are kept and you pick
(a controller prompt before the game, or the "Pick a save" row): never silently overwritten. What's replaced is kept:
on the server the last 5 in `Hearth Storage/Saves/<game>/replaced/`, on the device the last 3 in Hearth Link's
`save-backups/<game>/`. A push copies into `incoming/` and only replaces `current/` when it's all there. A save that
couldn't reach the server (it was off) is sent later.

**Device health** (`health.rs`): every device reports to its servers every 10 minutes (`device.report`: name, kind,
edition, Hearth version, battery, free space, last backup, and hearth-boot-guard's state, only read from grubenv's
`hearth_tries` and `/var/lib/hearth/boot-fallback.json`). Servers & devices lists every paired device: online or
last seen, battery, free space, last backup, version (behind the newest of your devices?) and whether its last start
was good. When the server's asleep, a device shows what it knows itself.

**Removing a device** (a device's menu › Remove this device…, from any device or the server): Hearth Link's unpair,
on this device and on the server; Hearth Storage can also delete that device's backups and the games it archived
(`devices.usage` says how much that frees first; nothing is deleted if the server doesn't answer).

**Requests** (paired devices only, `storage.rs` lists them): `storage.info`, `storage.ls`, `storage.stat`,
`storage.read` (chunk frames, like a copy), `archive.offer|list|remove|done`, `backup.offer`, `saves.info|offer|done`,
`device.report`, `devices.list|usage|forget`. Copies use the usual manifest/hashes/chunks requests with the ids
`archive:<key>`, `share:<share>:<path>`, `backup:saves|shots`, `savedev:<game>` (a device's save, only to its save
server) and `savesrv:<key>` (the server's kept save).

**The UI's calls** (`hearth-link call storage.…`): `status`, `servers`, `set`, `candidates`, `share.add|remove|set`,
`smb.password`, `ls`, `get`, `stream`, `archive`, `archived`, `archive.remove|forget|here`, `restore`, `backup.now`,
`backups`, `wake`, `saves.before|pull|after|resolve|status`, `devices`, `device.usage`, `device.remove`. The page's
Downloads & updates section uses `storage.cache.status|set` and `storage.updates.status|set` and hides them on a
Hearth Link without them.

**Settings › Hearth Storage** is one page in sections: This device (server on/off), Servers & devices (health,
remove), Shared folders, Game archive, Backups & saves, Media, Downloads & updates, and Advanced (away from home,
Windows sharing, sleep, wake), folded. Mantel's Hearth Link page has the same sections for what it shows.

**Tests:** `cargo test` (storage.rs: shared paths, settings, the archive's layout, where copies land, Wake-on-LAN
packets, ranges, reading a range; saves.rs: the plan never overwrites both, the fingerprint, a push replacing current
and keeping the old one, stores that sync left alone; health.rs: battery, boot state only read, versions, rows, a
device's data on the server), `tests/e2e.py lan` (A as the server: shared folder, media link, a game archived
and copied back, backups, turned off), `hosts/linux/test/test_hearth_storage.py` (Mantel, Hearth Files, wiring),
`test_hearth_storage_helper.py` (Samba config, the password only on stdin), `test_link_firewall.py` (port 445).

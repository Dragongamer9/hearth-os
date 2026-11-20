# Hearth Link

Hearth Link (`hosts/hearth-link`, the `hearth-link.service` user unit) pairs your Hearth devices, keeps Hearth's
data in sync between them and copies games from one to another. This page is about **pairing**; the crate's
`src/lib.rs` describes the rest (sync, copies, relays).

Settings › Hearth Link is the same in Game Mode (`ui/index.html`, `pageLink`) and Desktop Mode (Mantel's
`SetLink.qml` through `hearth-link-settings`). Everything works with a controller.

## My device, a family member's device or a friend's device

Pairing asks first: **My device** (the default), **A family member's device** or **A friend's device**. In Game Mode
it's a choice before the code shows (or before you pick the other device); in Mantel it's **The next device you pair
is** above the list (it's only for that screen, never saved). Each choice says in one line what it shares:

- **My device:** yours, paired with all your devices at once, with your settings, library, saves and status.
- **A family member's device:** pick this on your own device (your dad's PC). You see it in Your devices and can help
  on it (wake it, remote desktop); none of your settings, saves or library go onto it.
- **A friend's device:** paired with this device only, just for copying games. Nothing else is shared.

What it ends up as, from both sides' picks (`pair.rs relation`): a **friend's** if either side picked A friend's
device; **yours** only when both picked My device; otherwise a **family member's**. The side that picked A family
member's device helps on the other (on the family member's PC you can pick My device: it only means it trusts yours;
your things still never go onto it). If both picked it, each helps on the other.

- **Your devices are paired once for all of them.** Pair a new device with any one of yours and it's paired with every
  other one of yours too, with no code on each: your devices tell each other who else is yours (below).
- **A family member's device** is shared among your devices the same way: it shows on every device of yours.
- **A friend's device** is paired with that one device only, for copying games. Your other devices never hear of it.

See "Your devices, family members' devices and friends' devices" below for what each can do.

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

## Your devices, family members' devices and friends' devices

`hosts/hearth-link/src/family.rs`. Each paired device in `peers.json` has `relation` (`mine`, `family` or `friend`),
`friend` (still written: true for a family member's device too, so a Hearth from before family devices reads one as a
friend's, the safe side; a file from before `relation` is read from `friend` as before: false, yours, for every device
paired before friends existed), `introducedBy` (the device of yours that told this one about it), and for a family
member's device `familySide` (`member`: it's the family member's device, this one helps on it; `helper`: it's a device
of someone who helps on this one; `both`), `shares` (this device's switches for it), `theirShares` (what it said it
lets this one have, shown only) and `macs` (its network cards, to wake it). peers.json is written atomically, merged
in memory and never over a file that failed to read (state.rs); opening a page writes nothing.

**Your devices share their lists** (`family.sync`, protocol 2). Right after pairing with one of yours, and every 5
minutes after that (not while a game runs), each device sends each of your devices the others it knows
(`{devices: [{id, name, pairedAt, addrs, relay}], family: [{id, name, pairedAt, addrs, relay, macs}], removed: [{id,
at}]}`) and gets theirs back. A device it doesn't know yet is added (as yours from `devices`, as a family member's
device from `family`; `introducedBy` set, "paired through B" on both screens) and reached directly from then on.
Family members' devices are in their own list so a Hearth from before them (which reads `devices` only) never takes
one for yours. When a round changes something it runs again at once, so the news reaches every device quickly; a
device that was off catches up the next time it reaches any of yours. Until a device added this way has been reached
once, its "not paired" answer (it hasn't heard of this one yet) doesn't unpair it. One of yours made a family member's
device on another of yours is one on this device too (only ever down, never back to yours).

**Removing one of your devices (or a family member's device) removes it everywhere.** Unpairing it (or Remove this
device) on any of your devices is remembered (`removed-devices.json` next to `peers.json`, atomic writes, the newest
200), passed on by `family.sync`, and each of your devices drops it and tells it it's unpaired. An older list never
adds it back: a removal wins over a `pairedAt` before it. Pairing with it again (a newer `pairedAt`) clears the removal.

**A friend's device** stays paired with the one device it paired with.

- **It can:** be seen online, send you a game copy (accepted as before), copy a game you pick for it (its list of
  games), and unpair.
- **It can't:** sync settings or the library, see or back up saves, use Hearth Storage (shared folders, the game
  archive, backups, the download cache, updates for every device), see your status (Your devices), hand off a game,
  use your controllers, start updates, use your AI, run benchmarks, reach Decky, or learn about your other devices.
  It isn't told which stores you're signed in to, and its sign-ins aren't used for hints.
- **How:** a request from a friend's device that isn't one of `FRIEND_OPS` (`hello`, `ping`, `unpair`, `games`,
  `manifest`, `hashes`, `chunks`, `offer`) is answered "That's only for your own devices" before anything runs
  (`node.rs` on_stream, `family::allowed`); archived games, shared folders and backups are refused to it even through
  a copy (`local_game`); and this device never sends it anything personal (`family::may_ask`, checked in `request`
  before anything goes out).

**A family member's device** (the owner's dad's PC, Nov 18, 2026): you can see it and help on it, and nothing of yours
goes onto it unless its own switches say so.

- **It can, on top of a friend's:** show in **Your devices** on every device of yours, with its status (`status.get`:
  its switch "Let it see this device's status" per device of yours, on by default on the family member's device; yours
  toward it is off until you turn it on for that device); be woken (**Wake it**: it says its network cards in `hello`
  to the people who help on it); **Remote desktop** to it (`family.remote`: whether Settings › Remote play › Use this PC
  from anywhere is on there and its name for Moonlight through Tailscale, docs/REMOTE-DESKTOP.md; nothing else: no
  logins, no paired devices; and `family.remote.pin`: Moonlight's 4 numbers typed on your device go to its Sunshine,
  so you don't have to be at it). Hearth Link has no messages or file sending between devices of its own beyond these
  (files go through Hearth Storage's shared folders, below).
- **Its own switches** (on the family member's device, per device of yours, in that device's menu; all off by default,
  changed only there, never from your side: `peer.set` refuses them on yours, and your side only shows what they
  chose, `theirShares`):
  - **Share my game library**: favorites, hidden games, collections and play time sync both ways with that device
    (`sync`, with only the library's fields: `sync.rs library_only`); never settings, profiles, appearance or layouts.
  - **Back up my saves**: `saves.*` with your Hearth Storage (and saves follow it there).
  - **Use Hearth Storage (shared folders, the game archive, backups)**: `storage.*`, `archive.*`, `cache.*`, `updates.*`.
  - **Back up this PC**: `backup.*` (its saves and screenshots to your Hearth Storage).
  While a switch is off, its requests are refused exactly as a friend's are; when it's on, only that kind is let
  through (`family::switch_of`), checked on both sides before anything runs (on the family member's device from its
  own switch, on yours from what it said; `local_game` checks the same for a copy of a backup or a shared folder).
  Turning one off stops it at once (the next request is refused; `family.shares` tells your devices right away).
  Away from home, your Hearth Storage answers it only with Away from home on, as for your own devices.
- **It can't, ever:** get your settings, hand off games, use your controllers, start updates on your devices, use your
  AI, reach Decky, run benchmarks, or see your sign-ins. Its own list is never listened to (`family.sync` from it is
  refused). It's told only the names of your devices that talk to it (`family.intro`: `{devices: [{id, name, relay}],
  removed}`), and it takes that only from the device of yours it paired with with a code (never from one it heard of,
  a friend's or anyone else), so your other devices can reach it.
- **How:** `FAMILY_OPS` (`family.intro`, `family.shares`, `family.relation`, `family.remote`, `family.remote.pin`,
  `status.get`) on top of `FRIEND_OPS`, plus each switch's kind; anything else is refused before anything runs.

- **Changing it:** a device's menu › **Make it a family member's device**, **Make it a friend's device** or **Make it
  one of my devices** (Game Mode, Mantel's Settings › Hearth Link, or `hearth-link-settings make-family|make-friend|
  make-mine ID`). Going down (yours › a family member's › a friend's) stops sharing with it on this device at once and
  tells it (`family.relation`, which a device only ever takes as a step down). Going up to My device needs both devices
  to agree: unpair it and pair again, picking My device on both. Removing works as before.

**Safety.** A device is only ever added on the word of one of your own devices: an already paired device you picked
My device for, over Hearth Link's end-to-end encrypted connection, which only paired devices can open. A friend's or a
family member's device is never listened to (its `family.sync` is refused) and never told about your devices, except
the names a family member's device gets from the device of yours it paired with. Nobody can add a device by showing up
and asking. A device on a Hearth OS from before this answers `family.sync` with "Unknown request": nothing is shared
with it and it stays paired as before; one from before family devices reads a family member's device as a friend's.

Logs: `hearth-link: family: B told this device about C (…): paired`, `… said … was removed: removed here too`,
`… told this device about …: it can help on this one too`, `… made … a family member's device: here too`.
Tests: `cargo test` (family.rs: a new device learns your others and never friends, a friend is never listened to,
removed everywhere and not added back, an older removal loses to pairing again, what a friend's device may ask; a
family device sees nothing personal, each of its switches opens only its kind and closes again, it's shared among your
devices and never listened to, it hears names only from the device it paired with, old peers.json files read as before
and an older Hearth reads a family device as a friend's, only the library goes to it, remote desktop says only how to
reach it; pair.rs: yours only when both say so, family from both picks; state.rs: devices from before are yours,
friends and removals are kept), `hosts/linux/test/test_hearth_link_settings.py` (the helper and both screens), and
`hearth-link.yml`'s three-device e2e (`e2e.py family`: A–B and B–C paired, A and C paired through B with no code; C
removed on A, B drops it; C again as a friend's device of A's, B never hears of it, games can be seen, sync refused;
then C as a family member's device of A's: B has it through A and C knows B only from A, C's status on A and B and
A's on C only once A turns it on, remote desktop and its PIN from A and B and never from C, Use Hearth Storage and
Share my game library each refused until C turns it on (A can't), then only that kind, the library without settings,
off again at once; removed on A, gone everywhere).

## Family and friends with Moonlight

The owner, Nov 20, 2026: "add the ability to connect to family and friends pc with moonlight have it a deal they have
to enable". Code: `hosts/hearth-link/src/moonlight.rs`; screens: Settings › Gaming › **Family and friends**
(`hosts/hearth-ui/apps/settings/src/cats/gaming/family.rs`), `hearth-family-app` (`hosts/hearth-ui/apps/family`),
Hearth Games' **Family and friends** tab, Steam Game Mode's Quick Access (Hearth Link › Family and friends) and
Firehold's Couch (Apps › Family and friends).

**On the PC being connected to** (your dad's, a friend's):

- **Let family and friends connect (Moonlight)** is off until that person turns it on. Turning it on first says plainly
  what it allows: seeing this PC's screen, controlling it with a mouse, keyboard or controller, and hearing its sound.
- **Who can connect:** only the people they add. **Add a person** shows a one-time code: 6 digits on the same network,
  and a 10-character code with its QR code for anywhere (Hearth Link's pairing, works once, no password). A PC that's
  paired already (your devices, Dad's PC) keeps whose it is; a new one is paired as a friend's device. Each person can
  be removed any time (that also unpairs their Moonlight from this PC's Sunshine).
- **Per person:** **Ask every time** (the default: "Chris wants to connect" with Allow, Deny and Always allow Chris on
  this PC's screen; nobody answering in 60 seconds is a no) or **Always allow**; **Whole desktop** or **Games only**;
  a **time limit** (none, 30 minutes, 1, 2 or 3 hours) after which the session ends by itself.
- **While someone is connected:** a notice that stays on top and in the taskbar ("Chris is connected (Moonlight)") with
  **Stop sharing now**, a notification, and the line in Settings and Game Mode's Quick Access with Stop sharing now.
  Without `hearth-family-app` (an image built without it) the question and the notice are notifications with buttons.
- **Turning it off** ends every session at once (Hearth Link closes their connection, so the picture stops) and nobody
  new can connect. Sunshine's own pairing of their Moonlight stays until they're removed.

**On the connecting PC** (yours): Family and friends lists each PC that lets this one connect, online or offline (each
is asked once when the list opens: nothing polls), with **Connect** (Moonlight opens on that PC's screen) and
**Remove**, and **Add with a code** for the code their person gives you.

**How it gets there (no router ports, no UPnP, no Tailscale needed):**

- Moonlight's connections go through Hearth Link's own connection to that PC: QUIC with TLS 1.3, each device's key
  pinned at pairing, direct on the same network or hole punched over the internet, through the relay only when
  nothing else works. Its own protocol (`hearth/link-moonlight/1`) is only answered from a paired device the PC has
  just said yes to (`moonlight.ask`), and only for that session.
- Here, Moonlight runs in a private network of its own (a user and network namespace with only `lo`): Hearth Link opens
  Sunshine's ports there (TCP 47984, 47989, 48010; UDP 47998, 47999, 48000, from that PC's own Sunshine port) at an
  address of its own per PC (127.77.x.y), and carries each TCP connection on a QUIC stream and each UDP packet in QUIC
  datagrams (split in up to 8 parts when the path's datagrams are smaller; a packet with a lost part is dropped, as a
  network would). Your own Sunshine (if this PC has one) is never in the way, and Moonlight reaches nothing else.
- There, Sunshine only takes connections from that PC itself (Hearth's nftables table, `remote-fw local`: only `lo`;
  Tailscale too when Use this PC from anywhere is on). Sunshine is set up when the switch is turned on but only
  started for a session (unless Remote play or Use this PC from anywhere keeps it running), and stopped again after.
- Moonlight still pairs with that PC's Sunshine (its certificate) the first time: Hearth makes a one-time PIN, gives it
  to Moonlight here (`moonlight pair HOST --pin`, in its environment only) and sends it over Hearth Link to that PC's
  Sunshine (`hearth-remote-desktop pin`, named `Hearth <first 8 of the device id>`). It's never shown or logged.
- **Games only** starts them in Sunshine's **Steam Big Picture** app, and Sunshine's prep command refuses **Desktop**
  for them (`hearth-remote-desktop session start` reads the session file). Sunshine shows the whole screen whatever app
  is picked, so outside Game Mode they still see what's on the screen: on a PC in Game Mode that's Steam.
- Through the relay, Moonlight is started at 5 Mbps (`--bitrate 5000`); direct, at Moonlight's own setting.

**Settings and secrets:** the switch and each person's choices are in `~/.config/nimbus/family-moonlight.json` (`on`,
`people: {deviceId: {name, always, what, limitMinutes}}`), written only when the person changes something, through
settings_file.rs (atomic, `.bak`, merged; a file that can't be read is never saved over, and Settings says so). Device
ids aren't secrets; the keys stay in Hearth Link's store (identity.key, peers.json: `moonlight` on a peer says that PC
lets this one connect). Who's connected now: `$XDG_RUNTIME_DIR/hearth-link/moonlight-session.json` while it lasts.

**Calls:** between devices (any relation; each checks the switch and the list itself): `moonlight.hello`,
`moonlight.offer`, `moonlight.revoke`, `moonlight.ask`, `moonlight.pin`, `moonlight.end`. From the UI:
`moonlight.share.status`, `moonlight.share.set {on}`, `moonlight.share.person {id, always?, what?, limitMinutes?}`,
`moonlight.share.remove {id}`, `moonlight.share.invite`, `moonlight.share.invite.close`, `moonlight.stop`,
`moonlight.friends {check?}`, `moonlight.connect {id}`, `moonlight.disconnect`, `moonlight.forget {id}`,
`moonlight.join {code, id?}` (6 digits: the PC on this network showing a code). Sunshine's side:
`hearth-remote-desktop family on|off|start|idle|paired KEY|forget KEY`.

**Tests:** `cargo test` (moonlight.rs: packets split and put back, a lost part drops only its packet, the choices read
safely and saved merged, never over a file that can't be read), `hosts/linux/test/test_hearth_remote_desktop.py`
(Sunshine kept to this PC and not started at rest, off undone, Games only refuses Desktop, the local-only rule),
`hearth-link.yml`'s e2e (`e2e.py family`: off until turned on, a person added with a code, a question answered no
then yes, Moonlight's TCP and UDP from its private network to a stand-in Sunshine with its one-time PIN, Stop sharing
now, Always allow with Games only and a time limit, turning it off ending the session, removing; `e2e.py internet`:
the same through the relay and hole punched, speeds posted), the Decky panel's tests and pictures, and the Rust apps'
pictures (`family-*`, mouse and controller looks).

**Needs the owner's say:** how far to lower Moonlight through the relay (iroh's public relays are shared and slow for
video; 5 Mbps for now, or say "direct only" and Hearth won't use the relay for Moonlight), and whether Games only
should also be refused outside Game Mode (Sunshine can't show only the games while a desktop is on screen).

## Calls (the service's socket, `hearth-link call`)

- `pair.open {internet?: bool, seconds?: n, as?: "mine"|"family"|"friend", mine?: bool}`: shows the 6-digit code;
  `internet` adds a short code (published first, so an error means the lookup services couldn't be reached); `as` is
  what this side picked (`mine: false` alone is A friend's device, as before; missing: My device, or what was picked
  when pairing opened). Answers the `pairing` status.
- `pair.close`: stops pairing (and withdraws the short code).
- `pair.join {id, code, as?, mine?}`: a device picked on the network (or its long link) and its 6-digit code. Answers
  `{id, name, friend, relation, familySide}`.
- `pair.join {code, as?, mine?}`: a short code alone.
- `peer.set {id, alwaysAccept?, friend?: true, relation?: "family"|"friend"|"mine", shares?: {status?, library?,
  saves?, storage?, backup?}}`: `friend: true` or `relation: "friend"` makes it a friend's device, `relation: "family"`
  a family member's device, `relation: "mine"` answers how to (pairing again); `shares` sets a family device's
  switches on this device (`status` on either side; the other four only on the family member's own device).
- `peer.wake {id}`: Wake-on-LAN (a family member's device's own network cards, or Hearth Storage's for yours).
- `peer.remote {id}`: that device's remote desktop: `{on, running, tailscale, host, name, ip, streaming}`.
- `peer.remote.pin {id, pin, name?}`: Moonlight's 4 numbers to that device's Sunshine.
- `pair.check {code}`: checks a typed short code without the network.
- `status`: `pairing {code, until, triesLeft, netCode, netUntil}`, `pairEvent {state, name, triesLeft, friend, relation, familySide}` (this
  device's side), `joining {state, name}` (while `pair.join` runs), `nearby [{id, name, kind, edition, pairing}]`,
  `me {kind, edition, ticket}`, `shortCodes`, and each of `peers` has `friend` (a friend's device), `relation`,
  `familySide`, `shares` (what this device lets it have: `{status, library, saves, storage, backup}`), `theirShares`,
  `canWake` and `introducedBy` (a name). `status.devices` lists your devices and family members' devices (each with
  `relation` and `canWake`).

Devices from 0.6.5 still pair with newer ones with the 6-digit code (they show no kind or edition, and can't use
short codes). A device from before My device / A friend's device doesn't say which it is: it pairs as one of yours,
unless this side picked A friend's or a family member's device. One from before family devices only says mine or not
(`mine`): picking A family member's device here sends `mine: false`, so it keeps this one as a friend's device.

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
Settings › Home server (docs/HOME-SERVER.md) builds on it for everything else: Samba shares for Windows, phones and
TVs with their own settings, drives and snapshots, and Tailscale (its root helper uses `hearth-storage-helper`'s Samba,
password and Wake-on-LAN code, and the two keep their Samba shares apart).
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
- **Windows sharing (SMB)** (off by default): the shared folders for Windows PCs on this network, so a Windows PC
  can map one as a network drive (File Explorer › This PC › Map network drive, `\\<device>\<folder>`). Each shared
  folder is in it unless you leave it out (the folder's menu › Leave it out of Windows sharing), read-only for
  Windows unless you let Windows PCs change it. Signing in: your user name and a **password made on the device**
  when you first turn it on (4 groups of 4 characters without look-alikes, about 95 bits; never a fixed one, never
  typed in). Advanced › Windows sharing sign-in shows both on the device, and makes a new password when you want
  one. It goes to Samba through the helper's stdin and is kept only in Hearth Link's folder (`smb-password`, 0600)
  to show it again; it's never logged. `hearth-storage-helper` (root, through pkexec and 49-hearth-storage.rules)
  writes Samba's config; the firewall opens TCP 445 to local addresses only (`hearth-link-firewall open smb`), again
  at every start while it's on, and closes it when it's turned off. Needs Samba installed on the server.
- **Sleep when idle** (off by default): after N minutes with no copies, no game and the session idle (logind), it
  suspends. Turning the server on sets its wired network cards to wake from the network (ethtool, and
  NetworkManager so it lasts), and again at every start while it's a server; the PC's firmware may need Wake on LAN
  turned on too.

**On the other devices:** a server that doesn't answer is sent a Wake-on-LAN packet (its cards' addresses are
remembered from the last time it answered) and waited for up to about a minute, before any Storage operation
(browsing, copies, backups, the archive, saves). A server that's asleep also gets a **Wake** row next to it in
Servers & devices (and in its menu, and under Advanced). Media plays through a link on the
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
`save-backups/<game>/`. A pull goes into a staging folder next to the save, is checked against the server's file
count and size, and is swapped in with renames; one that fails, times out or comes in short is deleted and the save
here isn't touched. A push copies into `incoming/` and only replaces `current/` when it's all there. A save that
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
`smb.password` (a new generated password: `{user, password}`), `smb.show`, `ls`, `get`, `stream`, `archive`, `archived`, `archive.remove|forget|here`, `restore`, `backup.now`,
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
`test_hearth_storage_helper.py` (Samba config, the password only on stdin), `test_link_firewall.py` (port 445);
storage.rs also tests the generated password (format, no look-alikes, kept 0600) and leaving a folder out of
Windows sharing.

## Download once for every device

Hearth Storage can fetch things once and hand them to your other devices at home-network speed
(`hosts/hearth-link/src/cache.rs`). Both caches fall back to the normal way by themselves: a server that's off,
asleep or gone never stops a device from installing, updating or starting.

### Games

**Why this way:** the server already copies games with Hearth Link's copy engine (the archive), so the cache is the
same thing kept per game instead of per device: paired devices only, encrypted, resumable, only changed files sent,
and the copy lands as the store's own game. Store accounts, sign-ins and DRM are never touched: it's the files the
store wrote, and the store still checks and finishes them. Not chosen: a lancache-style DNS-intercepting HTTP cache
(it redirects every device on the network, not just paired ones, needs a DNS change on the router or each device,
and Steam/Epic increasingly download over HTTPS it can't cache); Steam's own **Local Network Game Transfers**
(Steam › Settings › Downloads, on by default for your own devices) keeps working alongside and is left as Steam set
it: Hearth doesn't edit Steam's settings.

- **On the server:** `cache` (off by default: it takes drive space). Copies go to `Hearth Storage/Cache/<store>-<id>`
  (game files only, never saves or Proton prefixes). The oldest-used copies are removed when the drive gets within
  20 GB of full; archives and backups are never touched.
- **On the devices:** `cacheUse` (on). Every half hour a device offers one installed Steam/Epic/GOG game the server
  hasn't got, or has an older build of (Steam's buildid); the server pulls only what changed, one game at a time.
  `storage.cache.get` installs a game from the server instead of the store's download, or puts the server's newer
  Steam build over the one installed here (not while a game runs; Steam is told by its appmanifest, restart Steam to
  see it). Games the server itself has installed are offered too.
- **Requests:** `cache.list`, `cache.offer {game}`; copies use the ids `cache:<key>` (from the server) and the
  device's own game id with the store `cache` (to the server).

### Hearth OS updates

On the server this is opt-in (`updates`, off by default; devices use a server's copy with `updatesUse`, on). The
server fetches each new image of the refs its devices run (they tell it, see below) once, every 6 hours
(`hearth-update-mirror fetch`, as the server's user, into `Hearth Storage/Updates`: content-addressed, every piece
checked against its sha256, the previous image kept too). Devices then update from it:

- **The mirror entry is not in the image.** `hearth-update-mirror sync` (root; run by may-stage before every
  background update and by `hearth-admin update` before "Update now") writes
  `/etc/containers/registries.conf.d/60-hearth-storage.conf` only while this device updates from a server (Hearth
  Link's `storage.updates.mirror` says `configured`: a paired server offers updates and `updatesUse` is on) and
  removes it otherwise, or on any doubt. A device with no server has today's update path byte for byte. The entry
  makes bootc (and rpm-ostree, uupd) try a mirror for `ghcr.io/dragongamer9` first: `127.0.0.1:479`,
  `hearth-update-mirror.socket` (root, socket-activated, a port only root can take).
- **Self-healing:** a background update that fails while the entry is in place (may-stage marks the attempt,
  `ExecStopPost=hearth-update-mirror after-update` reads systemd's `$SERVICE_RESULT`) removes the entry and skips
  the mirror for 7 days (`/var/lib/hearth/update-mirror-skip.json`), so the next attempt goes straight to ghcr.io.
  "Update now" that fails with the entry in place does the same and is retried at once without it.
- With the entry but the server not answering, the mirror's `/v2/` answers 503 and bootc goes to ghcr.io itself.
  With one: **tag manifests always come straight from
  ghcr.io** (never stale, never from anyone else); manifests by digest and layers come from the server through this
  device's Hearth Link (`storage.updates.fetch`: copied into `~/.cache/hearth-link/update-blobs`, then checked
  against their sha256 by the mirror before a byte goes to bootc) and from ghcr.io when the server hasn't got them.
  A piece that doesn't check out: never sent, fetched from ghcr.io instead, and the server is skipped for 6 hours.
  bootc checks every piece itself as well.
- Hearth Link's socket also accepts root for these four commands only (`ipc.rs` ROOT_COMMANDS).

**Hold-back** (`holdBack`, on; only does anything with a server): when the server has a new image, the first device
to ask goes first; the others' background updates wait (`ExecCondition=hearth-update-mirror may-stage` on
bootc-fetch-apply-updates and uupd, next to hearth-boot-guard's may-update) until a device reports that
hearth-boot-guard marked a start of that image good: `good` writes `/var/lib/hearth/boot-good.json` (image, digest,
version) after resetting GRUB's count, and Hearth Link sends it to the server (`updates.report`). Limits, so it
can never keep a device from updating: at most 3 days from when the server first saw the image, 3 days per device
(Hearth Link) and 3 days in a row (may-stage's own `/var/lib/hearth/update-hold.json`); server unreachable: only a
hold it already gave, within those 3 days; no Hearth Link, no server, any error: no wait. "Update now" is never
held (ExecCondition is only on the background services). If the first device's start fails, hearth-boot-guard
takes it back to the previous version and nobody else gets that image until someone starts it well or 3 days pass.

- **Requests:** `updates.stat {digest}`, `updates.read {digest, offset, len}` (chunk frames), `updates.hold {ref,
  booted}`, `updates.report {ref, digest, version, goodAt}`. Server state: `updates.json` in Hearth Link's folder.

**Bridge calls** (`link.…` in hearth-core's link.rs): `storage.cache.status` (`enabled`, `keep`, `use`, the server's
`games`, and on a device the server's `available` games and newer-build `updates`), `storage.cache.set {enabled}`
(on a server: keep copies; on a device: use them; `keep`/`use` set either), `storage.cache.get {game, server?,
place?}`, `storage.cache.remove {key}`, `storage.updates.status` (`cached`: each ref's image, version, `firstDevice`,
`verifiedBy`, `holdUntil`; `using`, `booted`, this device's `hold`), `storage.updates.set {enabled, holdBack}`
(`enabled`: on a server fetching images, on a device using a server's; `fetch`/`use` set either).

**Tests:** `cargo test` (cache.rs: builds, keys, Steam libraries, digests and refs, hold-back's decision and caps,
the cache's list), `hosts/linux/test/test_hearth_update_mirror.py` (a fake ghcr.io with its token step and a fake
Hearth Link: the server's fetch, the mirror's pieces only when they check out, 503 without a server, hold-back's
cap and failing open, the mirror entry written only while a server is used and removed for 7 days after a failed
background update or "Update now" (retried straight from the registry), the boot guard's note).

# Across your devices

Settings › Hearth Link › **All your devices** (Game Mode), once a device is paired.

## Continue on this device

`hosts/hearth-link/src/handoff.rs`. A game that's running on one of your devices, or was quit there in the last 15
minutes, shows on the other devices' Home (classic and Tiles looks; a toast on the others) as **Continue here**,
when it's installed there too.

1. Hearth writes `now-playing.json` in Hearth Link's folder when a game starts (`{game, title, at}`) and adds
   `quitAt` when it ends (hearth-core link.rs). Devices ask each other `handoff.now` (at most every 15 seconds while
   Home is open; never while a game runs on the asking device).
2. **Continue here** (`handoff.take`) asks the other device (`handoff.ask`). A game still running there is never
   closed from here (that could lose progress): the other device shows "<device> wants to continue <game>" (a
   prompt in Game Mode and a desktop notification) with **Quit it here** / **Keep playing here**, and this device
   waits up to 5 minutes.
3. Once it's quit, the other device sends its save the usual way (saves follow you, to the Hearth Storage server)
   and this device waits until that's done; then the game starts here the usual way, which brings the newest save
   first (and asks if both changed). Store cloud saves (Steam Cloud and the like) need nothing more. Without a
   Hearth Storage server the card asks first, since only store cloud saves come along then.

Requests: `handoff.now`, `handoff.ask {game}`, `handoff.cancel`. UI calls: `handoff.status` (`offers`, `asked`,
`savesServer`), `handoff.take {peer, game}` (`state`: ready, unsynced, conflict or declined), `handoff.decline {peer}`.

## Update every device at once

`hosts/hearth-link/src/fleet.rs`. **Update every device** starts Hearth OS updates on all your paired Hearth devices
(and this one, unless you pick "every device but this one"), with each device's progress in the list. Each device
decides for itself, by the same rules as always, and nothing is forced:

- **Boot safety first** (hearth-boot-guard's files, only read): updates paused after a fallback (the last update
  didn't start and it went back) stop it, and it says so. While the version it's on isn't confirmed as starting well
  yet, it waits (staging another now could replace the version it would fall back to).
- **Someone's playing:** it waits until the game ends. **Battery** below 40% and not charging: it waits for power.
- **Hold-back** (Hearth Storage): it waits like background updates do (the same answer, at most 3 days, failing
  open).
- Then it runs the same update as Settings › Updates › Update now (`pkexec hearth-admin update`): download and
  stage, with its progress. **It never restarts**: the new version starts at the device's next restart, the previous
  one stays to go back to, and hearth-boot-guard's fallback and the background services' ExecConditions are
  untouched. A device that already has an update waiting for a restart says so.
- A device waits at most a day, and **Stop waiting** stops the ones that haven't started yet.

Requests: `update.start`, `update.status`, `update.cancel`. UI calls: `fleet.update {here?}`, `fleet.status`,
`fleet.cancel` (each answers `devices: [{id, name, me?, state, pct, desc, error, battery}]`; `state` is one of
starting, confirming, waitingGame, waitingBattery, heldBack, updating, staged, upToDate, restart, paused, error,
cancelled, gaveUp, unsupported, offline).

## Your devices

Settings › Hearth Link › **Your devices** (`SetDevices.qml`, also a card on Settings' Home): each of your other paired
Hearth devices with online or when it was last seen, Game Mode / Desktop Mode / Server mode, what's playing, battery
and charging, CPU and GPU use and temperature, memory, graphics memory, each drive, the Hearth OS version and how long
it's been on. Pick one for the last hour in small graphs (CPU, GPU, memory, temperatures, battery) and what you can do
with it: open its shared folders in Hearth Files (Hearth Storage), **Wake it** (Hearth Storage's Wake-on-LAN, when that
device taught this one how) and **Remote desktop** (Settings › Remote play). A device that's off shows its last numbers,
marked as last known (`~/.cache/hearth/devices-status.json`, a cache).

- **How it travels:** only over Hearth Link, so only between paired devices: a connection from anything else is refused
  before any request (`node.rs`), and `status.get` checks pairing again (`status.rs`). End-to-end encrypted, at home or
  anywhere (direct, Tailscale or the relay). No port is opened and nothing goes to the internet.
- **What's sent** (`hearth-device-status status`, protocol 1, read only from /proc, /sys and Steam's manifests for a
  game's name): `{protocol, shared, name, model, kind, version, mode, playing {title, appid}, battery {percent,
  charging, pluggedIn}, cpu {use, temp}, gpu {vendor, use, temp, vramUsed, vramTotal}, ram {used, total}, drives
  [{name, used, total}], uptime, history [{t, cpu, gpu, ram, cpuTemp, gpuTemp, battery}], shares [{name, write, reach,
  badge}], smb {home, away}, updated}`. No secrets: no accounts, passwords, keys, file or folder paths (drives are
  System, Home or their label). `shares` are its Home server's shared folders that are on (`badge`: home or drive;
  `reach`: home or tailscale) and `smb` the names other devices reach them by (`<name>.local` at home, its Tailscale
  name), only when it shares something. It reuses hearth-stats
  (CPU, memory, temperatures) and hearth-ai-share's battery, graphics card and version code; Nvidia's numbers come from
  `nvidia-smi` when it's there.
- **The last hour:** while sharing is on, Hearth Link runs `hearth-device-status sample` once a minute and it keeps the
  last hour in `$XDG_RUNTIME_DIR/hearth-device-status.json` (gone at restart; not a setting).
- **Let my other devices see this device's status** (Hearth Link's page and Your devices; Mantel's `shareStatus` in
  desktop.json): on unless you turn it off, because it only reaches your own paired devices. On is only the default
  read when the key is missing, never written. Off, or a desktop.json that can't be read: `status.get` answers only
  `{"protocol": 1, "shared": false}` and nothing is kept for the graphs.

Requests: `status.get`. UI calls: `status.devices` (`devices: [{id, name, online, lastSeen, shared, status, old?}]`;
`old`: a device on a Hearth OS before this). Tests: `cargo test` (status.rs: only paired devices get an answer, the
not-shared answer, rows), `hosts/linux/test/test_device_status.py` (the status's shape, sharing off or unreadable, the
last hour, drives, no secrets, the page), desktop-test.yml's stand-in devices (`devices_sample.py`).

## Your devices in Hearth Files

Hearth Files' sidebar has a **Your devices** section by itself as soon as something is found (`hearth_netshares.py`,
`hearth-files places`): each paired Hearth device that shares folders (from its status above), online or offline, its
shares under it (Home / Whole drive, Read only / Read & write, Through Tailscale), other computers sharing on the home
network (DNS-SD `_smb._tcp` through `avahi-browse`) and Tailscale devices that answer SMB on port 445 (a plain connect,
nothing sent), then **How to open from Windows or a phone** (the `\\host\share` and `smb://host/share` addresses, at
home and away, with Copy). The Open / Save windows are Hearth Files too, so they show the same.

- **Finding:** what was found is kept in `~/.local/state/nimbus/files-devices.json` (state, not a setting; no
  passwords: only the user name you signed in with per device). The sidebar reads it at once; a fresh look starts in
  the background when it's a minute old (one at a time, only in the desktop session).
- **One click:** mounted with gvfs (`gio mount smb://host/share`) and opened at its folder under
  `/run/user/<uid>/gvfs` (Hearth Files can change files there, as on a USB drive, when the share is read & write).
  The address is chosen by itself: the home name when it answers here, the Tailscale name away. A share that's
  **Only through Tailscale** on the sharing device always goes through Tailscale, even at home (its Samba lets only
  Tailscale addresses in, so the home name was refused as if the password were wrong). A Tailscale name this device
  can't look up (MagicDNS off here) is opened by its Tailscale IP. A whole computer (not a Hearth device) asks which of
  its shared folders to open. The image has what this needs: `gio` (libglib2.0-bin), gvfs's SMB backend
  (gvfs-backends), gvfs-fuse (the folder under `/run/user/<uid>/gvfs`) and `secret-tool` (libsecret-tools); before
  that, shares were listed but none would open. gio is given the session's D-Bus (`$XDG_RUNTIME_DIR/bus`) when it's
  started without it, so Connect automatically from the user timer reaches the same gvfs.
- **Sign in once:** the first time it asks for the user name and the sharing password (the box hides it), then
  **Remember** or **Just this time**. Remember keeps the password in the system keyring (Secret Service through
  `secret-tool`, with gvfs's network-password attributes: protocol smb, server, object = the share, user, domain), never
  in Hearth's files. The password goes to Hearth Files' helper in `HEARTH_INPUT` and to gio and secret-tool on their
  input, never on a command line.
- **When it can't**, it says why and what to do (`classify`, with gio's own words underneath): a wrong password asks
  again at once (gvfs asking for the password a second time is taken as "wrong", not answered again and again), with
  the sharing device's account name filled in and "no sharing password yet" when that device says none was set; a
  share that's off; a device that's asleep (with **Wake it** when Hearth Storage knows its Wake-on-LAN address); a
  device that's online but whose shares don't answer (firewall, or Samba not running there); Samba turning this device
  away; Tailscale not signed in here or not on there; the SMB version; this device missing gvfs or gvfs-fuse; no
  desktop session. A paired Hearth device's status says, besides its names, the account to sign in with (`smb.user`),
  whether a sharing password was set there (`smb.password`) and whether its Samba runs (`smb.on`): never the password.
- **The sharing side** (Settings › Home server › Shared folders): when a firewall runs there (ufw or firewalld: not
  in the image, but one can be installed), port 445 is opened for the home network while home-network shares are on
  (`hearth-link-firewall open smb`), and closed again when they're off unless Hearth Storage's Windows sharing still
  uses it. When setting the sharing password fails, smbpasswd's own reason is shown.
- **Connect automatically** (Hearth OS 1.7; a shared folder's menu in the sidebar, right-click, press and hold or the
  menu button; off for each until you turn it on): that share opens by itself after you sign in, in Steam Game Mode
  and Desktop Mode alike, so it's already there in Files, the Open / Save windows and games. What's kept is where to
  find it (Hearth Files' `autoConnect` in files.json, written through `hearth_settings_io`: atomic, `.bak`, merged
  with the rest), never a password: it signs in with what the keyring remembers. `hearth-files-autoconnect.timer`
  (a user timer) runs `hearth-files net-autoconnect` 30 seconds after signing in and every 3 minutes after, so a
  device that was asleep or away is picked up when it's back. A device that doesn't answer within 1.5 seconds is
  skipped quietly until the next run; one that needs signing in or no longer has the share is left alone until you
  open it yourself (noted only in `$XDG_RUNTIME_DIR`). Nothing waits for it: it isn't part of signing in, Game Mode
  or Steam starting. Turned on, it's also tried right away. Off for everything: `HEARTH_NO_AUTOCONNECT=1`.
- **Show other devices in Files** (Settings › Hearth Link › Your devices; Hearth Files' `showDevices` in files.json):
  on unless you turn it off, only read when it's missing.
- Settings › Hearth Link › Your devices › a device › **Open shared folder …** opens it in Hearth Files the same way
  (`hearth-filemanager hearthnet://…`).

Tests: `hosts/linux/test/test_netshares.py` (avahi and Tailscale parsing, home vs away, the keyring with a stand-in
secret-tool, signing in through a stand-in gio, the sidebar model, no password in files or command lines,
Connect automatically: kept merged in files.json, opened without asking, asleep skipped, signing in left alone),
`test_netshares_mount.py` (every reason a share didn't open: Tailscale only, the Tailscale IP, each error told apart,
a wrong password stopping at once, the session bus, no gvfs-fuse, picking a computer's share, the sharing device's
hints, smbpasswd's reason, the firewall, the image's packages), home-server.yml's `smb-e2e` (a real Samba with
Hearth's own config and a real gvfs: `hosts/linux/test/e2e_netshares.py`), and desktop-test.yml's Hearth Files with
stand-in devices.

## Local AI over Hearth Link

`hosts/hearth-link/src/ai.rs`, a first version. A device with a strong graphics card shares a model server you
installed on it yourself (Ollama, llama.cpp's llama-server, or anything speaking Ollama's or OpenAI's HTTP API)
with your paired devices.

- **On the device with the model** (off by default): Local AI › Share this device's AI. Its server's address must be
  on that device itself (`http://127.0.0.1:11434`, Ollama's, by default; llama-server is usually `:8080`). Nothing
  is opened on the network; paired devices reach it over Hearth Link's encrypted connection.
- **On another device:** Local AI › Use <device>'s AI. Apps on that device then use `http://127.0.0.1:11435` (or the
  free port it says) as a local Ollama or OpenAI-compatible server; only that device itself can connect to it.
- Only using models goes through (GET and POST under `/api/` and `/v1/`), never managing them (`/api/pull`, `push`,
  `delete`, `create`, `copy`, `blobs`). Requests at most 8 MB, answers 32 MB.
- Not yet: answers come whole (a streamed answer arrives at once, when the model has finished), and nothing picks
  the device with the strongest card for you (the list shows each one's models, and graphics memory where the
  driver reports it).

Requests: `ai.info`, `ai.http {method, path, ctype, body}`. UI calls: `ai.status`, `ai.set {serve, backend}`,
`ai.use {server}`, `ai.stop`.

**Share this device's AI** (docs/AI-SHARE.md) builds on this for the Hearth AI Command Center: each device says what
AI it has (chat, voices, pictures, speech to text) at `http://<its Tailscale name>:8189/v1/hearth/ai`, and the Home
server lists every device at `/v1/hearth/ai/devices`, including paired Hearth Link devices (from `status`) that aren't
sharing through Tailscale. It's opened from Settings › Hearth Link › Share this device's AI (Mantel). Hearth Link's own
share above stays as it is: it's for apps on your paired devices, over Hearth Link's encrypted connection.

## Tests

- `cargo test`: handoff.rs (what's offered, the file, the answer once quit), fleet.rs (who may update now: boot
  safety, playing, battery, hold-back; reading Update now's output; what counts as busy), ai.rs (only this device's
  own server, only model use, the HTTP pieces including chunked answers, off by default); hearth-core link.rs
  (now-playing.json).
- `tests/e2e.py lan` (`extras`): a game running on A offered on B, B waiting while it runs (A asked) and taking it
  over once quit; A updating B: B waits while a game runs on it, then runs Update now (a stand-in) and stages it,
  A left out stays as it was; A sharing a fake Ollama (streamed, chunked) used from B through 127.0.0.1 on B, model
  management refused, nothing shared before it's on or after it's off.
- `hosts/linux/test/test_hearth_storage.py` (`test_link_extras`): the Game Mode wiring and the bridge's calls.

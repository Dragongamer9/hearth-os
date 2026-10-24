# Hearth Friends (Matrix)

Friends, chats, presence and game invites for Hearth OS, on the open [Matrix](https://matrix.org) protocol. Hearth
runs no server: an account lives on matrix.org or any Matrix homeserver the player picks, and works in every other
Matrix app (Element and others) too.

| Part | Where |
| --- | --- |
| The service (account, sync, end-to-end encryption, the local socket) | `hosts/hearth-friends` (`hearth-friends serve`) |
| Its rules: friend links, game invite and Play together cards, privacy buckets, settings, token redaction | `hosts/hearth-friends/src/lib.rs` |
| The core's side: `mfriends.*` calls, events to the page, "Playing <game>", Play together on this device, steam:// links | `hosts/hearth-core/src/matrix_friends.rs`, `bridge.rs` |
| Game Mode's Friends screen, chat, Quick Access › Friends, toasts | `ui/index.html` (search `Hearth Friends on Matrix`) |
| Desktop Mode (Mantel): the bar's Friends button and panel | `hosts/linux/desktop/shell/Friends.qml` |
| Image builds: `/usr/bin/hearth-friends`, `hearth-friends.service` (user), `/usr/share/nimbus/emoji-data.js` | `base-anatase.yml`, `base-peppermint.yml`, `os-bases/*/build.sh` |
| Tests | `cargo test` in the crate; `tools/friends-e2e.py` against Synapse; `tools/ui-check.js`; `.github/workflows/friends.yml` |

## How it fits together

* `hearth-friends serve` runs per user (the session starts it; Hearth's core and `hearth-friends call|watch` start it
  when it isn't running). It listens on `$XDG_RUNTIME_DIR/hearth-friends.sock` (0600, and it checks the caller's uid).
  One JSON request per line, one answer per line; `watch` turns a connection into an event stream
  (`message`, `presence`, `rooms`, `request`, `typing`, `status`). See `src/proto.rs`.
* Game Mode registers as a `gamemode` watcher: then new messages show as Hearth's own toast and go to Quick Access ›
  Notifications, and nothing pops up on the desktop. Otherwise the service sends them to Mantel's notification server
  (org.freedesktop.Notifications).
* Friends are direct chats (Matrix `m.direct`) with one other person. A friend request is a new encrypted direct chat
  with them; accepting is joining it. Group chats are ordinary encrypted rooms.

## Security and privacy

* **Encryption.** Every chat Hearth creates is end-to-end encrypted (Megolm, via matrix-sdk); pictures are encrypted
  attachments. Cross-signing is set up at sign-in. Hearth itself keeps message text only in memory.
* **Secrets.** The Matrix session (access token) and the key that encrypts the local SQLite store (state and
  encryption keys, `~/.local/share/hearth/friends/store`, 0700) live in the system keyring through the Secret Service
  (GNOME Keyring), named per data folder. If the keyring isn't there, the player stays signed in only until the
  service stops (the screen says so). Nothing secret is written to Hearth's files.
* **Logs.** The service doesn't set up matrix-sdk's tracing, and every error that's shown or logged passes through
  `redact()` (access/refresh tokens, `Bearer`, passwords, PINs). CI greps the logs for tokens.
* **Defaults.** Messages from friends only: invites from anyone else go to Requests without a pop-up (a friend
  request, being a direct chat, is always a request). "Show what I'm playing" is on and can be turned off; presence
  and status aren't end-to-end encrypted (the homeserver sees them), and the Settings page says so. Block uses
  Matrix's ignore list (the server drops their invites and messages; their pending requests are declined). Report uses
  the Matrix report APIs (a message, the user, or the chat). Mute is per chat.
* **Steam.** Hearth never signs in to Steam or reads Steam tokens. Steam's own windows open with a fixed set of
  steam:// links (`steam_link_ok`: friends list, a game, the store, `steam://connect/<server>`).
* **Pictures** can be sent only from Hearth's screenshots folder or ~/Pictures (checked by the core).

## Game invites and joining

* **Invite to play**: an `m.room.message` (text for other Matrix apps) carrying `io.hearthos.game`: title, store,
  Hearth's id, the store's own id and, optionally, a server. The other Hearth finds the game in its library by id,
  then store + store id, then title, and offers Play/Install; a Steam game not owned offers the Steam store page.
* **Join via Steam** opens Steam's friends list (where Steam's own Join Game/Invite are). **Join server** uses
  `steam://connect/<host:port>` for Steam games, and shows the address for others.
* **Play together (Beta Experimental)** works for any game, like Remote Play Together: the host's Sunshine streams
  the game and the friend's Moonlight plays it, their controller becoming another player.
  1. A friend in a game shows as *Joinable* (Sunshine is there and "Let friends ask to join" is on).
  2. The friend asks (or answers the host's offer): a `request` with a one-time 4-digit PIN, inside the encrypted chat.
  3. The host says yes (controller only by default: Sunshine's keyboard and mouse input off) and sends `accept` with
     the addresses to reach it. The host's Hearth enters the PIN in Sunshine once the friend's Moonlight starts
     pairing (tried for 90 s), under the name `Friend <id> <device>`.
  4. The friend's Hearth runs `moonlight pair <host> --pin <PIN>` then `moonlight stream <host> Desktop`.
  5. *Remove from game* unpairs that device and closes the stream; the friend is told with `kick`.
  The way to reach the host is a `Transport` (`play_together` in `matrix_friends.rs`): the local network today;
  Hearth Link (hosts/hearth-link) plugs in through `hearth-link call play.address {}` answering
  `{"ready": bool, "addresses": [...]}`. Hearth Link links your own paired devices and has no tunnel for a friend's
  device yet, so until that call exists Play together works on the same network only.

## Accounts

Sign-in is with a user name and password (works on matrix.org, including its newer account system, and on most
servers). New accounts can be made from Hearth on servers with open sign-up or a sign-up token; servers that want a
captcha, an email or their terms accepted on their own page (matrix.org among them) get their web page instead
(app.element.io), after which the player signs in. Single sign-on (OAuth) isn't there yet.

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
| Signing in and up on the server's own page (OAuth 2.0: QR code for a phone, the browser), the recovery key | `hosts/hearth-friends/src/service/oauth.rs` |
| Tests | `cargo test` in the crate; `tools/friends-e2e.py` against Synapse; `tools/friends-oauth-e2e.py` against Synapse with the Matrix Authentication Service; `tools/ui-check.js`; `.github/workflows/friends.yml` |

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
  `redact()` (access/refresh tokens, `Bearer`, passwords, PINs, sign-in codes, web addresses' query parts). CI greps
  the logs for tokens and codes.
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

Hearth asks each server how it signs people in (`authMethods`) and shows only what it offers:

* **With your phone (QR code)**: for servers with Matrix's newer sign-in on their own page (OAuth 2.0, MSC3861 and
  its parts; matrix.org runs the Matrix Authentication Service) that offer the device authorization grant
  (RFC 8628). Hearth shows a QR code, the short address and a code; the player scans it, signs in or makes the
  account on the server's own page on the phone (its captcha, email and terms included) and allows Hearth. Hearth
  checks back every few seconds and is signed in by itself: nothing is typed with the controller. "Create an
  account with your phone" is the same page; the player picks *Create account* there.
* **In the browser on this device**: the authorization code flow with PKCE. Hearth listens on
  `http://127.0.0.1:<port>/callback` (a loopback address, RFC 8252), opens the server's page in Hearth Browser
  (Game Mode) or the default browser (Desktop Mode, through the service), and finishes when the browser comes back.
  "Create an account" asks for the server's sign-up page (`prompt=create`) when the server offers it.
* **User name and password**: servers without the newer sign-in (and matrix.org, where it still works).
* **Create an account in Hearth**: servers with open sign-up or a sign-up token. Servers that want a captcha, an
  email or their terms accepted on their own page and have no newer sign-in get their web page instead.

How the newer sign-in works in the service (`hosts/hearth-friends/src/service/oauth.rs`):

* matrix-sdk's OAuth API does the discovery (`auth_metadata`), the authorization code flow, restoring the session,
  renewing tokens and revoking them. The device grant is plain HTTP in oauth.rs (matrix-sdk keeps its own copy for
  logging in from another Matrix device, MSC4108).
* Hearth registers itself with the server at each sign-in, on each device (dynamic client registration, MSC2966):
  a public client named "Hearth", `client_uri` https://hearthos.fun/, the logo from the site, `application_type`
  native, and only the grant that sign-in uses. There is no client id or secret in the repository.
* The session (client id, user, device, access and refresh tokens) is kept in the system keyring like password
  sessions, as `{"kind": "oauth", ...}`; sessions from 0.6.5 (no kind) are password sessions and keep working.
  matrix-sdk renews the access token with the refresh token when it runs out, and the new pair replaces the old
  one in the keyring. If the server refuses the refresh token, the player is signed out with a message.
* Signing out revokes the session's tokens at the server (RFC 7009); password sessions call `/logout` as before.
* The log (`service.log` / the journal) gets each step (discovery, registration, code issued, approved, browser
  came back, token renewed, signed out) and never a token, a code, or an address with a code or state in it
  (`redact()` drops web addresses' query parts too).

After signing in, Hearth offers a **recovery key** once (Friends screen and Settings › Recovery key): "Set up a
recovery key" turns on the encrypted key backup and shows the key once (Hearth doesn't keep it), and an account that
already has one (made in Element, say) gets "Enter my recovery key" to unlock older encrypted messages. "Not now"
skips it.

### Testing it with a phone

1. Friends › Hearth Friends, signed out, server matrix.org: "With your phone" and "On this device" show.
2. "Create an account with your phone": scan the QR code with the phone's camera (or open the short address and type
   the code), pick Create account on matrix.org's page, finish it there, then allow Hearth. Hearth signs in by
   itself within a few seconds.
3. Settings › Sign out, then "Sign in with your phone" with the same account; then "Sign in in the browser".
4. The recovery key question: set one up, sign out, sign in again and use "Enter my recovery key".
5. Desktop Mode's Friends panel: the same buttons; A starts the phone sign-in.

CI (`.github/workflows/friends.yml`) runs all of it against a second Synapse that uses the Matrix Authentication
Service, both started for the run with one throwaway account (`tools/friends-oauth-e2e.py`): the phone and the
browser are played by a small form filler on MAS's own pages. It also looks, read only, at what matrix.org
advertises. No real server gets an account from CI.

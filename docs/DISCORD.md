# Discord on Hearth OS

Since 1.6, Hearth's friends live in Discord. Hearth Friends (Matrix) is archived (`docs/archive/FRIENDS.md`,
`docs/ARCHIVED.md`).

Hearth only uses what Discord officially offers: the Discord app itself, the app's local Rich Presence socket (IPC),
and `discord-<app id>://` links. Hearth **never** asks for, reads or stores a Discord token or password, never reads or
sends your messages or friends list, and never scripts, reads or injects into Discord's window or its storage.
Discord's terms forbid that kind of thing (self-bots, user tokens, unofficial client APIs), and it can get an account
banned.

## What you get

| Where | What |
| --- | --- |
| Mantel's bar (the chat button; any bar edge) | The first press asks **which Discord** (three cards, below). After that it toggles a **small Discord window** by the bar: above a bottom bar, below a top bar, beside a side bar, centred on the button, about 420×640. It's Discord's own window, so you read and reply to friends there. Press again to hide it: it's minimized, never closed, so Discord keeps running and its notifications keep coming. Its size, if you resize it, is kept for next time (`~/.local/state/nimbus/discord-window.json`: state, not a setting). If the window isn't in front, the button brings it to the front first. |
| Settings › Gaming › Discord (`SetDiscord.qml`) | Which Discord (the chooser again), **Open Discord** (full size), the **Discord application ID** (paste it, one step), **Show what I'm playing on Discord**, **Let friends ask to join**, and **Invite link** while you play. |
| Steam Game Mode: Quick Access › Hearth Link › **Discord** | **Open Discord** (full size), the two switches, the Play together PIN, **Set up Discord** (opens Hearth Settings › Discord in a window). Discord's notifications (and other apps') show as Decky toasts. |
| Hearth's own Game Mode (archived; first-time setup and the fallback) | Quick Access › Discord: Open Discord (the app, or the web window when it isn't installed; Install from Flathub) and the "Playing" switch. Its Matrix Friends parts are hidden (the core no longer offers `mfriends`). |
| `quickshell ipc … call discord toggle / open / hide / full / chooser` | The same from scripts and keybinds; older `friends toggle/open/close` calls open Discord. |

### The three Discords

The chooser shows once (the first press of the bar's button, or Settings › Discord › Which Discord). Hearth saves your
pick (`client` in `~/.config/nimbus/discord.json`) only when you pick, and installs only what you picked, only when you
press **Install**.

| Card | What Hearth does |
| --- | --- |
| **Discord** (recommended) | The official app: Flathub's `com.discordapp.Discord`, `flatpak install --user`. A `discord` from a .deb works too. |
| **Discord in a web window** (official, nothing to install) | Discord's own web app (`https://discord.com/app`) in Hearth Browser (Firefox) with its own profile (`~/.local/share/hearth/discord-web`) and the tab and address bars hidden, so it looks like an app. You sign in on Discord's own page. |
| **Dorion** (lighter, unofficial) | [Dorion](https://github.com/SpikeHD/Dorion), a small Tauri app that loads Discord's web app and uses less memory. The card says plainly that it's unofficial and that Discord's rules don't allow third-party apps, so there's a small risk to the account. Hearth installs its official GitHub release's `.deb` for your processor, **only if GitHub publishes a SHA-256 for it and the download matches it**, unpacked into `~/.local/opt/dorion` (nothing system-wide). Dorion wasn't confirmed on Flathub when this was written, so Flathub isn't used for it. |

Every client is started under XWayland (Firefox with `MOZ_ENABLE_WAYLAND=0`, Dorion with `GDK_BACKEND=x11`; Discord's
app is XWayland already), so Hearth can move and size the window (`xdotool windowmove/windowsize`, which labwc honours
for XWayland windows). Showing, hiding and maximizing go through labwc's window list (foreign toplevel), for every
client the same way.

**Controller:** the chooser takes Left/Right, A and B. Discord's window is an ordinary app window, so the controller
works there as in any app: the stick moves the pointer (controller mouse) and A clicks, and **Guide + X** opens
Mantel's on-screen keyboard for typing a reply. It's an XWayland window, so the keyboard doesn't pop up by itself when
a text box is focused: Guide + X it is. B and Esc are Discord's own there (closing its dialogs); the bar button (or
`discord hide`) hides the window.

## "Playing <game>" (Rich Presence)

While a game from Hearth's library runs (started by Hearth: Hearth's Game Mode or Mantel's Games), Hearth tells the
Discord app on this device, over its IPC socket, what you're playing: the game's clean title (no ™/®, no store
suffixes), its picture when Hearth has a web picture for it (else Hearth's own), and the time since it started.

Setup, once: make an application at [discord.com/developers/applications](https://discord.com/developers/applications)
(its name is what Discord shows, e.g. "Playing Hearth OS"; upload an image named `hearth` under Rich Presence › Art
Assets for the small icon), copy its **Application ID**, paste it in Settings › Discord and press Save. The ID isn't a
secret; Hearth reads it from, in order: `HEARTH_DISCORD_APP_ID` (testing), `appId` in `~/.config/nimbus/discord.json`
(what Settings saves), `/etc/hearth/discord.conf` (`app_id=…`, for an image-wide one), and a build-time variable. No ID:
no Rich Presence, and nothing talks to Discord's socket. Saving the ID also registers `discord-<id>://` for Hearth (a
hidden `hearth-discord-<id>.desktop`, set in `mimeapps.list` with only that line changed), which is how Discord starts
Hearth for a friend's invite.

## Ask to join, Join and Play together

With **Let friends ask to join** on (the default), the activity also carries a party and a join secret, so friends see
**Ask to join** on your game in Discord (and you can invite them from Discord's own + button in a chat). Discord only
gives the secret to friends you invited or let join.

The secret (`h1|<Hearth game id>|<address>|<PIN>|<expires>|<title>`, 128 characters at most) carries:

- the game's Hearth id and title, so the friend's Hearth opens **that game's page** (by id, else by title: Play or
  Install; a Steam game they don't have opens Steam's store page);
- for **Play together**, only while Remote play's Sunshine is on (Settings › Remote play; Hearth never turns it on for
  this): this device's address (its Tailscale address when it has one, so a friend you shared the PC with in
  Tailscale's admin page reaches it; else the local network's) and a **one-time 4-digit PIN that expires after 10
  minutes**. The PIN shows on both screens (Settings › Discord / Quick Access here, a notification there). No password,
  key or token is ever in it.

When a friend presses Join, their Discord fires `ACTIVITY_JOIN` with the secret on their Hearth: either to Hearth's
core while it's connected (it subscribes), or Discord starts Hearth through `discord-<id>://` and
`hearth-discord join --wait` connects and asks for it. Then:

1. Play together (the PIN hasn't expired and Moonlight is installed): their Hearth runs `moonlight pair <address> --pin
   <PIN>` then `moonlight stream <address> Desktop`. Your Hearth enters that PIN in Sunshine while it's valid (it tries
   every few seconds), names the device `Friend Discord <n>`, makes the friend's input **controller only** (no keyboard
   or mouse on your PC, as Play together did before), tells you, and makes a new PIN.
2. Otherwise the game's page opens (`nimbus --windowed --game-page <id>`).

**Invite link** (Settings › Discord while you play, or `hearth-discord invite`): `https://hearthos.fun/join#<secret>`
to paste into a Discord chat yourself. The secret is after the `#`, which browsers never send to the website; the page
hands it to Hearth (`hearth://join/…`, `hearth-join.desktop` in the image).

**Remove a friend from your game:** Settings › Remote play (their device is `Friend Discord <n>`).

## Notifications

Discord's own desktop notifications are what Hearth shows; Hearth reads nothing else of Discord.

- Desktop Mode: Mantel's notification server (`Notifications.qml`), as for every app.
- Steam Game Mode has no notification server, so the Hearth Link plugin runs `hearth-discord notify-bridge` while
  Hearth's core service runs (it's started next to Steam there and nowhere else): it takes desktop notifications only
  when nobody else serves them, gives the name up to anyone who asks, and quits within 3 seconds once Steam Game Mode
  ends, so Desktop Mode's own notification server always gets it back. Each one becomes a Decky toast (title and
  plain text, as Discord wrote it). Nothing is logged or kept beyond the last 20 in memory.
- Hearth's own (archived) Game Mode: Discord's notifications aren't shown there.

## What Discord doesn't allow, so Hearth doesn't do it

- Reading or sending messages, reading the friends list, or acting as your account in any way (self-bots, user tokens,
  unofficial client APIs).
- Sending a game invite to a chosen friend from Hearth: that needs the Discord Social SDK's `SendActivityInvite` with
  your Discord account linked to the app. Instead, you invite from Discord's own **+** button in a chat (Discord
  shows "Invite to join"), or paste Hearth's **invite link**.
- Custom buttons next to the join buttons: Discord shows either custom URL buttons or the join buttons, so Hearth
  uses the join buttons.

### The Discord Social SDK

Checked for 1.6: the Social SDK's terms (support-dev.discord.com, "Discord Social SDK Terms", March 2025) don't limit
it to games or rule out a launcher, but Discord reviews integrations against its requirements, can revoke access at
any time, and the SDK is a closed C++ library. Its invites (`SendActivityInvite`, receiving invites in Hearth) need the
player's Discord account linked to the app with OAuth (tokens Hearth would have to keep) and the
`GetDefaultCommunicationScopes` scope. That's more than Hearth should hold, and the owner has no approved integration,
so Hearth doesn't use it. The local IPC Rich Presence it does use (`SET_ACTIVITY` with party and secrets, `SUBSCRIBE
ACTIVITY_JOIN`) is the same mechanism Discord's own Rich Presence library has always used; Discord's RPC page says
the approval step is for RPC's other, OAuth-scoped commands.

## What's left of Hearth Friends (Matrix)

- The service (`hearth-friends`) isn't built, shipped or started any more; its user service is gone from the image.
  A leftover `default.target.wants` link from an older image points at nothing, and systemd skips it: the session
  starts the same.
- Your old Matrix data is **left exactly where it was**: the keyring entries and `~/.local/share/hearth/friends/` are
  never read, moved or deleted. The Matrix settings aren't read any more and aren't changed.
- Your own play-time profile (Friends › Activity in Hearth's Game Mode) stays; only sharing it with friends went.

## Files

| Part | Where |
| --- | --- |
| The helper (chooser's pick, installs, starting and placing the window, invites, joining, the notification bridge) | `hosts/linux/desktop/shell/hearth-discord` |
| The bar's button, the quick window and the chooser | `hosts/linux/desktop/shell/Discord.qml` (`Bar.qml`, `Islands.qml`, `QuickAccess.qml`, `shell.qml`) |
| Settings › Gaming › Discord | `hosts/linux/desktop/shell/SetDiscord.qml`, `settings-pages.js` |
| Rich Presence, join secrets, ACTIVITY_JOIN, Play together's PIN | `hosts/hearth-core/src/discord.rs`, `play_together.rs` |
| Steam Game Mode | `hosts/decky-hearth-link` (main.py `discord*`, src/index.tsx `DiscordSection`) |
| The invite page | `site/join/index.html` |
| Tests | `hosts/linux/test/test_hearth_discord.py`, `cargo test` in hearth-core (`discord::tests`), `hosts/decky-hearth-link/tests/test_main.py` (`DiscordQuickAccess`), `desktop-test.yml` (a stand-in Discord window: chooser, quick window, hide without quitting, full) |

## Needs testing on a device

Everything here passed CI only. On a device: the chooser and each client's install; the quick window's placement on
every bar edge with the real Discord, the web window and Dorion; hiding without losing notifications; Rich Presence
with a real application ID; Ask to join → Join between two Hearth devices (game page, then Play together over
Tailscale); Discord starting Hearth through `discord-<id>://` when Hearth isn't running; toasts in Steam Game Mode.

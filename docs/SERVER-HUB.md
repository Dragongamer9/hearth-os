# The Home server helper's Game servers, Game cache, Remote play, Media and PCs

Boards: docs/mockups/server-expansion/05-game-servers.png, 07-game-cache.png, 08-remote-play.png, 09-media.png,
10-more-pcs.png (approved Nov 26, 2026: "thoes look good"). Checklists: docs/mockups/checklists/server-game-servers.md,
server-game-cache.md, server-remote-play.md, server-media.md, server-more-pcs.md.

## How it ships (the owner: "All this has to be updated thru the store")

| Piece | Where | Comes with |
|---|---|---|
| The five pages (pure Rust, made of blocks) | `hosts/hearth-ui/apps/server-more` (a library: one module per page; `hearth-server-more-app` shows them on their own) | the Home server helper's Store app |
| What they do as you | `hosts/linux/desktop/shell/hearth-server-hub` (builds on `hearth-server-apps`, `hearth-remote-desktop`, `hearth-device-status`) | Mantel's Store part |
| What needs root, once, with the admin password | `hearth-home-server-helper games-setup`, `play-setup`, `media-setup` (packages through hearth-unlock, so they're kept across updates; the games' rule in /etc) | the Home server helper system part |
| The game cache | `hearth-server-dns` (LANCache and Hearth Shield's redirects) | the Home server helper system part: quick-update.yml packs it as `hearth-server-parts/server-dns.py`; `hearth-home-server-helper cache-setup` (Install it, admin password once) writes /etc/systemd/system units for the cache that run a root-owned copy of it (/var/lib/hearth/server-dns/engine, refreshed each time the helper runs it as root), so the cache never needs a Hearth OS update |
| Between your PCs | Hearth Link's `server.ask` / `server.wake` (hosts/hearth-link/src/server.rs): your own devices only; the PC asked answers with its own `hearth-server-hub link-answer` | Hearth Link's Store part |
| Hearth Server on other PCs | `hearth-server-hub agent-status` in each PC's Hearth Link status (`hearthServer`, hearth-device-status) | Mantel's Store part (on every Hearth PC) |

Hooking the pages into the Home server helper's window: its sidebar lists `hearth_server_more::SIDE`, it keeps one
`Hub`, passes `Msg` to `Hub::update`, draws `Hub::view(part, &describe, pal)` and adds `hearth_server_more::blocks()`
to its Describe pages. `Msg::D` is its own Describe's.

## What each part does (part 2, Nov 26)

- **Game servers**: Valheim's day from the world's .db (its saved time over 30-minute days); each player's device from
  Minecraft's own log (it names each player's address), the addresses on the game's port (inside its container and
  Podman's forwarding) and what the timer learned (a new player and a new address in the same 5 minutes), named by
  Tailscale and Hearth Link ("the ROG Ally", "over Tailscale (shared)"). Kick: Minecraft's and Bedrock's consoles;
  any other game by address (`games-kick`: hearth_games' `kicked` set with a 2-minute timeout, no mod needed). Shared
  with: Tailscale's sharee nodes (whose they are) and the invites you make here ("Who is it for?"). Mods: a player's
  Hearth device asks the server's mods over Hearth Link and gets the same versions from Thunderstore itself
  (`game-join`, `mods-sync`), then joins; friends without Hearth get a Thunderstore profile code for r2modman
  (`mods-code`). Minecraft Bedrock: Mojang's Bedrock Dedicated Server (itzg/minecraft-bedrock-server). Run on
  another PC if this one's busy: a whole game server moves to one of your PCs that takes jobs (its world saved,
  sent over Hearth Link, started there; copies come back hourly with nobody on; it comes home when you say or by
  itself).
- **Game cache**: Epic (builds named in their address), Battle.net (its product folders) and the Xbox app (package
  names) named; Hearth OS updates your devices got from this PC counted by Hearth Link (`updates-served.json`).
  When full, drop the least used: off means LANCache gets 6% headroom and, at 97% of your size, the stores' names
  stop going to the cache (new downloads go straight through), so nothing is ever dropped. Keep a game forever: each
  night at 3 AM every piece of that game is asked from the cache once (one byte), so nginx counts it as just used.
- **Remote play**: the devices are Sunshine's paired ones and your Hearth devices, each ready, away (Tailscale says
  it isn't on your home network; the device says 5G, LTE, Wi-Fi or wired) or off; a Hearth device's quality goes into
  its own Moonlight.conf (only those keys, a .bak kept); Play on a picked device opens Moonlight there; Hearth
  Retro's games stream through its --play; while a stream is on, `play-watch` samples the delay every 5 s and takes a
  small picture of the stream every 20 s; frames on time are Moonlight's own count from a Hearth device's last
  stream; waking from away goes through another of your devices at home (`server.wake`); sleeping after 30 minutes
  only when it can be woken again, nobody streams, plays or is on a game server and nothing of Hearth's is busy.
- **Media**: each wish becomes a job: a recording through Jellyfin's guide (a series timer), a public-domain film
  from the Internet Archive (only items whose licence says public domain), your disc when it's in, your clips
  (Media/Drop/<title>) joined; recordings get their ads cut with comskip where this Debian has it; each is named
  (Movies/Title (Year), TV/Show/Season NN) and asks when it isn't sure, then goes into Jellyfin. Music, photos and
  Space are counted from the files (`media-scan`, at most every 6 hours, in its own low-priority unit); the same photo
  twice is found by SHA-256 and by a small picture fingerprint (ffmpeg), and Look through them makes a folder of
  links (nothing moved or deleted). Added this week's posters are Jellyfin's own art.
- **PCs**: each PC's card from its Hearth Link status with Hearth Server's part (its jobs, watts); the week from the
  timer's 5-minute samples; every move listed; found on your network from Hearth Link's announcements, Sunshine's
  mDNS (_nvstream._tcp) and Tailscale; when a PC goes off, this one stands in with each whole job it can run (a game
  from its last copy, an AI engine it has too) and hands it back.

## Still open, and why

- Pre-loading games (now and overnight): only a store's own client can download a game, and Steam's needs your
  account, which Hearth never uses.
- GOG through the cache: GOG downloads only over https, which LANCache can't keep.
- Epic games whose addresses carry only numbers, and Store apps in the cache's Hearth row (https).
- Moonlight on Google TV (Android): its quality, starting a stream on it and its frames on time can't be reached
  from here (it isn't a Hearth device).
- Each game and Hearth Server as their own Store apps: they come in Mantel's Store part; a listing of their own is
  the Store list's, which only the owner publishes.

## Safe by default (red flags 2 and 9)

- Nothing starts by itself. `hearth-server-hub.timer` (every 5 minutes) is enabled only while a schedule, Stop when
  nobody's on, Pause while you play or a streaming rule is on; it has no boot ordering and its service can't fail
  (`ExecStart=-`). The game servers are hearth-server-apps' units (no `[Install]`, 4 tries then stopped).
- The games' firewall unit only loads a rule (`ExecStart=-/usr/sbin/nft -f /etc/hearth/games.nft`), isn't ordered
  before anything and can't fail the boot.
- While a game is played on this PC the timer only lowers the game servers' CPU share (CPUWeight 10, nobody is
  kicked) when you chose that; nothing heavy runs.
- Your choices: `~/.config/nimbus/server-hub.json` through hearth_settings_io (atomic, .bak, merged under a lock, only
  the page's own choices accepted, defaults never written).
- Steam: dedicated servers download with SteamCMD's anonymous login inside their images. Never a Steam account.
- Media: only your discs (copied as they are; a copy-protected disc isn't unlocked), the antenna (Jellyfin's Live TV),
  your own videos, public-domain films from the Internet Archive and a folder you drop files in. No torrents, no
  searching for copies of anything else (a test checks the helper for it).
- PCs: each PC runs whole jobs; one AI model is never split over the internet.

## Tests and pictures

`hosts/linux/test/test_hearth_server_hub.py` (home-server.yml) builds every page from the boards' sources
(`.github/scripts/hearth-apps-rust-shots.d/server-hub-fixture.py`) and checks the words; hearth-apps-rust.yml's
`server-more` shots picture the five pages from the same pretend servers.

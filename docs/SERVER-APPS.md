# Server apps and Phone notifications

Planned for Hearth OS 1.7. Settings › Home server › **Server apps** and **Phone notifications**, plus what your other
Hearth devices see of a PC in Server mode.

## Server apps (the catalog)

One-click game servers and your own versions of apps you'd otherwise pay for. Every one is **off and not installed**
until you press Install, then turn it on.

| App | What it is | Port(s) | Reach by default |
|---|---|---|---|
| Minecraft | Java Edition server (itzg/minecraft-server; Paper, Vanilla or Fabric; memory, players). Needs Mojang's EULA accepted | 25565/tcp | home network and Tailscale |
| Horizon | Minecraft: Java Edition 26.3 rewritten in Rust (github.com/horizon-rust/horizon), no Java. Its signed server build is downloaded at Install (checked against Horizon's release key: the Nightly for its own date and commit) and run read-only in Debian's small image; the game files come from your own Minecraft, prepared by Horizon's launcher (Hearth Games › Recomps), mounted read-only. Needs Mojang's EULA accepted; world name, Minecraft accounts only. Same port as Minecraft's server: one at a time | 25565/tcp | home network and Tailscale |
| Valheim | Dedicated server (lloesche/valheim-server; server and world name; a password of 5+ characters) | 2456-2457/udp | home network and Tailscale |
| Terraria | TShock server (ryshe/terraria; world name and size) | 7777/tcp | home network and Tailscale |
| Palworld | Dedicated server (thijsvanloef/palworld-server-docker; name, players, optional password) | 8211/udp, 27015/udp | home network and Tailscale |
| Vaultwarden | Password manager server for the Bitwarden apps (sign-ups on until you turn them off) | 8222/tcp | Tailscale only |
| Immich | Photo and video backup from your phone (server, Postgres and Valkey in one pod; no machine learning) | 2283/tcp | Tailscale only |
| Navidrome | Music streaming of your Music folder (read only) to any Subsonic app | 4533/tcp | Tailscale only |
| ntfy | Your own notification server for Phone notifications | 2586/tcp | Tailscale only |

How it runs, and why it can't stop the PC from starting:

- Each app is a container in **your own rootless Podman**, run by **your own user service** `hearth-app@<id>.service`
  (`systemd --user`, written by `hearth-server-apps` into `~/.config/systemd/user`). The service has no `[Install]`
  section; its timer starts it a minute after you sign in and is enabled only while the app is on. Nothing in the image,
  the session, Game Mode or Steam starts or waits for it. `ConditionPathExists=/usr/bin/podman`; four failed starts in
  15 minutes and it stays stopped (the page says so, and Phone notifications tell you).
- Images are pulled only when you press Install or Update (in a transient unit of yours at low priority), then run with
  `--pull=never`.
- **Who can reach it**: "Home network and Tailscale" listens everywhere; "Tailscale only" listens only on 127.0.0.1 and
  this PC's Tailscale address. Two apps on the same port can't be on together.
- Your worlds and data are in `~/Server apps/<name>`. Remove takes away the images; the data goes only with two
  confirmations (`podman unshare rm`, since containers own some files).
- Passwords are kept in `~/.local/state/nimbus/server-apps-secrets.json` (0600), never in settings, and handed to Podman
  in a 0600 env file in your runtime folder, never on a command line.

Your choices are in `~/.config/nimbus/server-apps.json` through `hearth_settings_io` (atomic, `.bak`, merged under a
lock; only the key you changed is written; defaults are filled in when read and never saved). Install state:
`~/.local/state/nimbus/server-apps.json`.

## Phone notifications (ntfy)

Home server alerts (what Alerts & power finds: drives failing, hot or full, backups failing or late, a mirror missing a
drive) and server apps that stopped, sent to the ntfy app on your phone. Off until you turn it on; **Make a topic**
makes a long random topic name (anyone with the name can read it). ntfy.sh by default, or your own ntfy (the catalog's,
or any address). Your own timer `hearth-server-ntfy.timer` (enabled only while it's on) checks every 5 minutes; the
same problem is sent again only after a day. Send a test is on the page.

## Your server on your other devices (Hearth Link, the Ally)

A PC that uses Server mode adds a short `server` part to the status it shares with your paired devices
(`hearth-device-status`, only when "Let my other devices see this device's status" is on): the status line, each
dashboard tile in short, alerts' text and the server apps. No addresses, paths or who's connected. It shows:

- in Desktop Mode: Settings › Hearth Link › Your devices › the PC › **Its server** (with Open its dashboard, the web page
  over Tailscale);
- in Steam Game Mode (the Ally): Quick Access › Hearth Link › **Your server: <name>** (the Hearth Link plugin).

Server mode's dashboard shows a "Server apps" pill while one is on, and `/api/summary` carries `apps`.

## Not done yet (from the server ideas list)

Night maintenance, a power history graph, a self-hosted GitHub Actions runner and one smart AI address.

## Files

- `hosts/linux/desktop/shell/hearth-server-apps`, `SetHomeApps.qml`, `SetHomePhone.qml`, `SetDevices.qml` (Its server),
  `hearth-device-status` (`server`), `hearth-server-mode` (the pill, `apps` in the summary)
- `hosts/decky-hearth-link/main.py` (`servers`), `src/index.tsx` (`YourServers`)
- Tests: `hosts/linux/test/test_hearth_server_apps.py` (Home server workflow); screenshots in desktop-test.yml
  (`shots/home-server/35-…38-…`, `shots/looks/steam-11e-settings-device-server.png`)

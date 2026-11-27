# Game servers (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-server-expansion)

Mockups: server-expansion/05-game-servers.png
Release: next

The owner, Nov 26: "Ok we're gonna expand heavily on the Server side of hearth OS", "Let's do it", then on the boards
"thoes look good". Every part installs and updates through the Hearth Store ("All this has to be updated thru the
store"), never as a Hearth OS update; anything needing system pieces installs itself with the admin password once
("Debian system files are unlocked"). Made of blocks.

## Every part of the board
- [x] Title bar: the HS mark, Home server helper and the page's name (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/main.rs: "\"Home server helper\","
- [x] Minimize, maximize and close (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/oss-kit/kit.rs: "ui::G::Minimize"
- [x] Sidebar lines for this page set with their counts (Game servers 3 on, Game cache 612 GB, Remote play 1 playing, Media 7,412, PCs 3 PCs) (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/lib.rs: "(\"games\", \"Game servers\", Part::Games)"
- [x] Updates in the Hearth Store · every part is its own Store app · never a Hearth OS update (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/main.rs: "every part is its own Store app · never a Hearth OS update"
- [x] Made of blocks · Edit layout (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/main.rs: "tb(\"Edit layout\", 12.0, p.accent)"
- [x] The rest of the Home server helper's sidebar: Search the server, Server home, Dashboard, Model vault, Backups, This server (from the Dashboard part), these five pages hooked into it (apps/home-server pages/more.rs; CI picture server-more-w1, w4) (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/home-server/src/pages/more.rs: "a.more.view($part, &a.describe, a.p).map(Msg::More)"
- [x] 3 RUNNING · 7 PLAYERS tag (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'%d running · %d players'"
- [x] Game servers and its line: One click makes a server on the Nvidia PC · friends join over Tailscale, nothing open to the internet (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/games.rs: "friends join over Tailscale, nothing open to the internet"
- [x] Invite friends (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/games.rs: "kit::chip(&p, \"Invite friends\""
- [x] ＋ New server (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/games.rs: "\"＋ New server\""
- [x] Valheim card: RUNNING, its art, 3 of 10 online, world "Ashfjord", since 11:40 PM · CPU 6% · RAM 3.1 GB, Stop, Restart, Console, outlined while open (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'since %s · CPU %d%% · RAM %.1f GB'"
- [x] Minecraft card: 2 of 8 online, Java · 1.21 (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'Java · %s'"
- [x] Palworld card: 2 of 16 online, "Ranch night" (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'palworld': '\"%s\"'"
- [x] Gravesleep card: OFF · YOUR GAME, test server · 4 players, Godot, build from your repo · starts Sat 7 PM, Start, Newest build (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'build from your repo%s'"
- [x] Noclip Protocol card: OFF · YOUR GAME, co-op test · 4 players, O3DE, dedicated server build · not made yet, Make the server build (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "dedicated server build · not made yet"
- [x] Start, Stop and Restart really start and stop the server (hearth-server-apps' units; its world backed up when it stops) (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "m.world_backup(gid, 'stopped')"
- [x] Console: its last lines (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'podman', 'logs', '--tail', '200'"
- [x] The open server: VH mark, Valheim · "Ashfjord", seed f8Kq2Lm · password on · crossplay off (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "bits.append('seed %s' % info['seed'])"
- [ ] "day 214" in the open server's line: Valheim keeps the day inside the world's .db, which isn't read yet (only the seed from its .fwl)
- [x] Players · Mods · Worlds · Settings (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/games.rs: "[\"Players\", \"Mods\", \"Worlds\", \"Settings\"]"
- [x] PLAYING NOW · 3: each player with their letter, name, how long, ADMIN for you (Steam's server query and Minecraft's list ping) (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "def a2s(port"
- [ ] Each player's device ("· the ROG Ally", "· over Tailscale (shared)"): the game servers don't say which device a player is on; Hearth only guesses "over Tailscale (shared)" from Tailscale's shared devices
- [x] Kick (Minecraft, through its console) (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'mc-send-to-console', 'kick', who"
- [ ] Kick for a Valheim player: Valheim has no remote console, so the page says to kick from the game itself
- [x] Mods · 4 on · from Thunderstore, checked: BepInEx pack, Plant Everything, Valheim Plus, Better Archery, each with its switch (Thunderstore's own API, deprecated mods refused) (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "thunderstore.io/api/experimental/package/"
- [ ] Players' games get the same mods by themselves when they join from Hearth (the line is shown; the players' side isn't built)
- [x] Invite friends · over Tailscale: Share this server with a friend's Tailscale, they only see this one game, Make an invite link (Tailscale's share page) (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'open': 'https://login.tailscale.com/admin/machines'"
- [x] The address with its port and password ••••••, Copy (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/games.rs: "Msg::Copy(s(i, \"address\")"
- [ ] Shared with: Sam · Jordan · Dad (Minecraft only): read from the page's settings, but nothing writes who a game was shared with yet (Tailscale doesn't tell)
- [x] World backups: every night at 4:00 AM and when it stops · kept 30, the three newest with their sizes and Restore (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'every night at %s and when it stops · kept %d'"
- [x] Start and stop on a schedule: the week's grid (Valheim, Minecraft, Gravesleep), a press sets or clears a day, started and stopped by the 5-minute timer (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "def in_span(days, t)"
- [x] stops when nobody's on for 30 min (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "stopped %s (nobody on for %d min)"
- [x] Players this week: the bars, Fri … Thu, most: Sat 9 PM, 9 players (the timer keeps each hour's players) (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'most: %s %s, %d players'"
- [x] Your games' servers: Gravesleep · from your Gravesleep repo, build Nov 25 · headless Godot; Quad mech game · 4-seat test, build Nov 19; Noclip Protocol · O3DE server build, Set it up (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'from your Gravesleep repo'"
- [x] Your own games run their headless build in Podman (Godot and O3DE servers) (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-apps: "exec \"./$b\" --headless"
- [x] New server · Each game is its own Store app · updates in the Hearth Store (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/games.rs: "Each game is its own Store app · updates in the Hearth Store"
- [ ] Each game as its own Store app: today the game servers come in Mantel's Store part (hearth-server-apps' catalog), not one Store app each
- [x] Minecraft · Java or Bedrock, Valheim, Palworld, Terraria · 7 Days to Die · Ark, Gravesleep, Noclip Protocol, Quad mech game (your game): each installs its server (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'Terraria · 7 Days to Die · Ark'"
- [x] 7 Days to Die and Ark dedicated servers (SteamCMD's anonymous login inside their images) (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-apps: "'docker.io/vinanrra/7dtd-server:latest'"
- [ ] Minecraft Bedrock: only Java Edition (itzg/minecraft-server) is in the catalog
- [x] First server only: Install the game server helper; It adds the system parts it needs (a service and a firewall rule for Tailscale only); Install it · asks for your password once (CI picture server-more-1; needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-home-server-helper: "def games_setup():"
- [x] Using the PC fairly: Pause servers' CPU while you play here (a small CPU share while a game runs here, nobody kicked) (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'CPUWeight=' + weight"
- [x] Most memory for all servers: 12 GB (one slice with that memory limit) (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "MemoryMax=%dG"
- [ ] Run on another PC if this one's busy (kept as a choice; moving a running game server to another PC isn't built)
- [x] See PCs to let Dad's PC host one (CI picture server-more-1; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/games.rs: "See PCs to let Dad's PC host one"
- [x] Using the servers: CPU · all servers 14%, Memory · all servers 6.2 of 12 GB, Worlds and saves 3.4 GB · in Backups (CI picture server-more-1; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'%.1f of %d GB'"

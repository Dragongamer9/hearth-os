# Remote play hub (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-server-expansion)

Mockups: server-expansion/08-remote-play.png
Release: next

The owner, Nov 26: "Ok we're gonna expand heavily on the Server side of hearth OS", "Let's do it", then on the boards
"thoes look good". Every part installs and updates through the Hearth Store ("All this has to be updated thru the
store"), never as a Hearth OS update; anything needing system pieces installs itself with the admin password once
("Debian system files are unlocked"). Made of blocks.

## Every part of the board
- [x] Title bar: the HS mark, Home server helper and the page's name (CI picture server-more-3; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/main.rs: "\"Home server helper\","
- [x] Minimize, maximize and close (CI picture server-more-3; needs testing on a device) — hosts/hearth-ui/apps/oss-kit/kit.rs: "ui::G::Minimize"
- [x] Sidebar lines for this page set with their counts (Game servers 3 on, Game cache 612 GB, Remote play 1 playing, Media 7,412, PCs 3 PCs) (CI picture server-more-3; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/lib.rs: "(\"games\", \"Game servers\", Part::Games)"
- [x] Updates in the Hearth Store · every part is its own Store app · never a Hearth OS update (CI picture server-more-3; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/main.rs: "every part is its own Store app · never a Hearth OS update"
- [x] Made of blocks · Edit layout (CI picture server-more-3; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/main.rs: "tb(\"Edit layout\", 12.0, p.accent)"
- [x] The rest of the Home server helper's sidebar: Search the server, Server home, Dashboard, Model vault, Backups, This server (from the Dashboard part), these five pages hooked into it (apps/home-server pages/more.rs; CI picture server-more-w1, w4) (CI picture server-more-3; needs testing on a device) — hosts/hearth-ui/apps/home-server/src/pages/more.rs: "a.more.view($part, &a.describe, a.p).map(Msg::More)"
- [x] 1 PLAYING · THE TV tag (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'1 playing · the %s'"
- [x] Remote play and its line: Play any game on the Nvidia PC from the TV, the phone, the Ally or the truck laptop · Sunshine on the PC, Moonlight on each device (CI picture server-more-3; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/play.rs: "Sunshine on the PC, Moonlight on each device"
- [x] Pair a device (Moonlight's 4 numbers, sent to Sunshine) (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'pin', pin, 'Moonlight'"
- [x] Wake the PC (Wake-on-LAN: a magic packet to its network card) (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "def magic_packet(mac):"
- [x] Playing now: LIVE, PLAYING NOW, Gravesleep · on the TCL 55" TV, 4K · 60 fps · HDR · 38 Mb/s · delay 9 ms · Xbox controller (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'%s · on the %s'"
- [ ] Playing now's picture of the game itself (the board's scene): drawn as a dark gradient, not the stream's own picture
- [x] Send a message (a notice on the PC's screen, which is what's streamed) (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'From the Home server'"
- [x] Stop the stream (Sunshine closes the app) (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'/api/apps/close'"
- [x] THE NVIDIA PC WHILE STREAMING: Video encoder (NVENC) 22%, GPU 71% (nvidia-smi) (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'--query-gpu=utilization.encoder,utilization.gpu'"
- [ ] Frames sent on time 99.6%: Sunshine doesn't report it; the bar shows only when a source gives it
- [x] Delay over the last 10 minutes · 7–12 ms, its line (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'Delay over the last 10 minutes · %d–%d ms'"
- [ ] The delay's samples while streaming: the page draws them, but nothing samples the delay every few seconds yet
- [x] While someone's playing: Stop Colibri and Imagine to free the GPU (started again after), Keep the game servers running, Pause the Model vault upload (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "def play_rules(s, t, did):"
- [x] Devices · quality for each: TCL 55" TV (PLAYING, Moonlight on Google TV · wired, 4K, 60, 40 Mb/s, HDR · surround on · 5.1), ROG Ally (READY, 1080p, 120, 30 Mb/s, Away from home 720p · 15 Mb/s), Phone (1080p, 60, 20 Mb/s, On 5G 720p · 8 Mb/s), Truck laptop (AWAY · 5G, 900p, 60, 10 Mb/s · lowers itself, Last played) (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'via': 'Moonlight on Google TV · wired'"
- [x] A press on Size, Frames or Bit rate changes it for that device (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "def play_device(did, key, value):"
- [ ] Each device's quality reaching that device's Moonlight (kept on the server; Moonlight on each device isn't set from here yet)
- [ ] Each device's state from the network (READY, AWAY · 5G) and the truck laptop's last game: only the one streaming is known; the rest are shown as ready
- [x] Any game on the PC · 86 with Played lately, Steam, Your games, Retro, The whole desktop (Steam's installed games, your own games, the desktop) (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "def steam_library():"
- [x] Each game's art and Play (into Sunshine's list, so every device's Moonlight shows it) (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'steam steam://rungameid/%s'"
- [ ] "on the TV ▾": starting the stream on a picked device (the device has to open it in Moonlight)
- [ ] Retro games in the library (no retro games are listed yet)
- [x] Who's playing: You · on the TV, 1 h 12 min, Gravesleep · started 12:12 AM (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'You · on the %s'"
- [x] One stream at a time on this PC. Another device asks first: "Take over from the TV?" (CI picture server-more-3; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/play.rs: "Another device asks first: \\\"Take over from the TV?\\\""
- [x] Wake the PC: Wake it from any device (Moonlight's wake button · or the phone's Home server page), Wake it from away, over Tailscale, Let it sleep after 30 min with nobody (kept) (CI picture server-more-3; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/play.rs: "Moonlight's wake button · or the phone's Home server page"
- [ ] Wake it from away over Tailscale (needs an always-on PC at home to send the packet) and Let it sleep after 30 min (kept as choices, not acted on yet)
- [x] Played this week: TV 9 h, ROG Ally 4 h, Truck laptop 2 h 10, Phone 35 min (the timer counts each stream's minutes) (CI picture server-more-3; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "key = '%s|%s' % (datetime.date.fromtimestamp(t).isoformat(), streaming)"
- [x] Sunshine on the PC · Moonlight on each device: Both from the Hearth Store… Sunshine needs a system part to capture the screen. Install it · asks for your password once (CI picture server-more-3; needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-home-server-helper: "def play_setup():"
- [x] For the Xbox itself, Hearth Console Remote Play is its own app. (CI picture server-more-3; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/play.rs: "For the Xbox itself, Hearth Console Remote Play is its own app."
- [x] Remote play · Built on Sunshine (GPL-3) and Moonlight (GPL-3) (CI picture server-more-3; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/play.rs: "Sunshine (GPL-3) and Moonlight (GPL-3)"

# Settings › Home server (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-server-expansion)

Mockups: server-expansion/11-server-home.png
Release: next

The owner, Nov 26: "Ok we're gonna expand heavily on the Server side of hearth OS", "Let's do it", then on the boards
"thoes look good". Every part installs and updates through the Hearth Store ("All this has to be updated thru the
store"), never as a Hearth OS update; anything needing system pieces installs itself with the admin password once
("Debian system files are unlocked"). Made of blocks.

## Every part of the board
- [x] Settings' window with its sections on the left, Home server lit — hosts/hearth-ui/apps/settings/src/frame.rs: "let on = i == a.cat && a.query.is_empty();"
- [x] "Made of blocks · Edit layout" at the bottom of Settings' sections — hosts/hearth-ui/apps/settings/src/frame.rs: "t1(\"Made of blocks · \", 12.0, p.muted)"
- [x] The title "Settings › Home server", Home server in the accent — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "tb(sec.0, 34.0, p.accent)"
- [x] Its line "This PC serving your home: AI, games, backups, media and more · every part is its own Hearth Store app" — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "line: \"This PC serving your home: AI, games, backups, media and more · every part is its own Hearth Store app\","
- [x] The overall card: its dot and "12 of 13 services healthy · 3 PCs · 8 devices" (real counts: healthy services from the Dashboard, PCs, Tailscale's devices) (needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-server-parts/dash.py: "'%d of %d services healthy · %d PC%s · %d device%s'"
- [x] Its line: what needs a fix · last backups 2:36 AM · 1 playing on the TV · all reached privately through Tailscale — os/system_files/usr/libexec/nimbus/hearth-server-parts/dash.py: "bits.append('all reached privately through Tailscale')"
- [x] Fix it (the first thing that needs a look, fixed) — hosts/hearth-ui/apps/settings/src/cats/hs_board.rs: "engine::spawn(\"dash\", vec![\"fix\".into(), id], String::new())"
- [x] Open the dashboard — hosts/hearth-ui/apps/server-parts/src/home.rs: "\"Open the dashboard\""
- [x] Server mode (in the accent) — hosts/hearth-ui/apps/server-parts/src/home.rs: "w::chip(p, \"Server mode\", true, Sz::Md, Some(m(HAct::ServerMode)))"
- [x] The tiles four across, from the one list of parts — hosts/hearth-ui/apps/server-parts/src/home.rs: "pub fn tiles_grid<"
- [x] Dashboard tile: DB, "every service, its health and fixes", "12 of 13 healthy", "GPU 41% · 62° · drives 37% · power 186 W", "Desk, TV, phone and Ally views" — os/system_files/usr/libexec/nimbus/hearth-server-parts/dash.py: "'Desk, TV, phone and Ally views'"
- [x] Model vault tile: MV, "your own copy of every AI model", "45 of 50 models · 1.25 TB", "here and on R2 · $12.50 a month", in red "1 removed online, your copy safe" — os/system_files/usr/libexec/nimbus/hearth-server-parts/vault.py: "'%d removed online, your copy safe' % removed"
- [x] Backups tile: BK, "every device, every night", "4 devices · all done", "Ally, laptop, truck laptop, Dad's PC", "kept 30 nights · 1.9 TB · copy on R2" — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "' · copy on R2' if c.get('r2') else ''"
- [ ] Game servers tile (GS, "3 running · 7 players", Valheim, Minecraft, Palworld, the Gravesleep test, UPDATE READY): feature/server-2's part
- [ ] Game cache tile (GC, "1.9 TB saved", 612 GB cached, the stores): feature/server-2's part
- [ ] Remote play tile (RP, "1 playing · the TV", Gravesleep 4K 60, 4 devices paired): feature/server-2's part
- [ ] Media tile (MD, "7,412 in Jellyfin", getting it, photos moved from iCloud): feature/server-2's part
- [ ] PCs tile (PC, "3 PCs · all healthy", Nvidia PC, Dad's PC, old laptop): feature/server-2's part
- [x] Each tile's graph in its own colour — hosts/hearth-ui/apps/server-parts/src/home.rs: "draw::spark(nums(v, \"spark\"), draw::hex(part.color), true, Length::Fill, 44.0)"
- [x] Each tile's "● Updates in the Store" with UP TO DATE, or UPDATE READY from the Store's own list — hosts/hearth-ui/apps/server-parts/src/home.rs: "w::tag(p, \"Update ready\", Tg::Acc)"
- [x] The picked tile lit with the accent — hosts/hearth-ui/apps/server-parts/src/home.rs: "(Pal::mix(p.panel, p.accent, 0.08), (p.accent, 2.0))"
- [x] A part not on this PC yet says so and Gets it from the Store — hosts/hearth-ui/apps/server-parts/src/home.rs: "\"Not on this PC yet: it comes from the Hearth Store\""
- [x] YOUR DEVICES · 8 and "all on Tailscale · Hearth Link shares the AI" — os/system_files/usr/libexec/nimbus/hearth-server-parts/dash.py: "'all on Tailscale · Hearth Link shares the AI'"
- [x] Each device with its dot and what it's doing (TCL 55" TV · playing, Xbox · off, Truck laptop · away) — os/system_files/usr/libexec/nimbus/hearth-server-parts/dash.py: "note = 'playing'"
- [x] "Each tile is a block: drag to reorder, hide the ones you don't use · Edit layout" (each tile its own block) — hosts/hearth-ui/apps/server-parts/src/home.rs: "\"Each tile is a block: drag to reorder, hide the ones you don't use · \""
- [x] Updates for the server, and "Every part comes from the Hearth Store and updates there, never as a Hearth OS update." — hosts/hearth-ui/apps/server-parts/src/home.rs: "\"Every part comes from the Hearth Store and updates there, never as a Hearth OS update.\""
- [x] The part with an update ready: its mark and name, READY, what's new, Update, What's new, Try the test version — hosts/hearth-ui/apps/server-parts/src/home.rs: "\"Try the test version\""
- [x] Update installs the Store's copy (a system part with the admin password) — hosts/hearth-ui/apps/server-parts/src/store.rs: "let mut a = vec![\"system-part-install\", u.id.as_str()];"
- [x] Update server apps by themselves (off unless you turn it on) — os/system_files/usr/libexec/nimbus/hearth-server-parts/dash.py: "DEFAULTS = {'phone': False, 'quiet': False, 'quietFrom': 22, 'quietTo': 6, 'onlyIdle': True, 'auto': False"
- [x] Only when nobody's playing or watching (on) — os/system_files/usr/libexec/nimbus/hearth-server-parts/dash.py: "if c.get('onlyIdle', True) and (C.playing() or (C.state('play') or {}).get('now')):"
- [x] "Parts that need the system (a service, the firewall, a TV tuner) install with your password once, from inside the app." — hosts/hearth-ui/apps/server-parts/src/home.rs: "\"Parts that need the system (a service, the firewall, a TV tuner) install with your password once, from inside the app.\""
- [x] ALSO HERE: Shared folders 4, Files & links on, People 3, Drives & undo all healthy, Safety on, Alerts & power phone alerts on, The other services Cloud, Home, Shield… (each opens its page) — hosts/hearth-ui/apps/settings/src/cats/hs_board.rs: "(\"The other services\", \"Cloud, Home, Shield…\".to_string(), \"hs-services\"),"
- [x] Every part arrives and updates through the Hearth Store: the engines ride in the Home server helper's system part — .github/workflows/quick-update.yml: "cp os/system_files/usr/libexec/nimbus/hearth-server-parts/* /tmp/part/hearth-server-parts/"

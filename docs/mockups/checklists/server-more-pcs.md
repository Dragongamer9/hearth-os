# More PCs as one server (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-server-expansion)

Mockups: server-expansion/10-more-pcs.png
Release: next

The owner, Nov 26: "Ok we're gonna expand heavily on the Server side of hearth OS", "Let's do it", then on the boards
"thoes look good". Every part installs and updates through the Hearth Store ("All this has to be updated thru the
store"), never as a Hearth OS update; anything needing system pieces installs itself with the admin password once
("Debian system files are unlocked"). Made of blocks.

## Every part of the board
- [x] Title bar: the HS mark, Home server helper and the page's name (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/main.rs: "\"Home server helper\","
- [x] Minimize, maximize and close (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/oss-kit/kit.rs: "ui::G::Minimize"
- [x] Sidebar lines for this page set with their counts (Game servers 3 on, Game cache 612 GB, Remote play 1 playing, Media 7,412, PCs 3 PCs) (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/lib.rs: "(\"games\", \"Game servers\", Part::Games)"
- [x] Updates in the Hearth Store · every part is its own Store app · never a Hearth OS update (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/main.rs: "every part is its own Store app · never a Hearth OS update"
- [x] Made of blocks · Edit layout (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/main.rs: "tb(\"Edit layout\", 12.0, p.accent)"
- [x] The rest of the Home server helper's sidebar: Search the server, Server home, Dashboard, Model vault, Backups, This server (from the Dashboard part), these five pages hooked into it (apps/home-server pages/more.rs; CI picture server-more-w1, w4) (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/home-server/src/pages/more.rs: "a.more.view($part, &a.describe, a.p).map(Msg::More)"
- [x] 3 PCS · ONE SERVER tag (CI picture server-more-5; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'%d PCs · one server'"
- [x] PCs and its line: Add another PC and Hearth spreads the work across them… You still see one server. (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/pcs.rs: "You still see one server."
- [x] How it spreads the work (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/pcs.rs: "Each kind of work goes, whole, to the PC that suits it"
- [x] ＋ Add a PC (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/pcs.rs: "\"＋ Add a PC\""
- [x] Nvidia PC card: N mark, RTX 5070 12 GB · 32 GB · 22 TB · at home, MAIN, Healthy · 62° · up 9 days, 186 W, CPU 18%, GPU 41%, Doing (Colibri · Qwen3.6 35B (Big AI), Jellyfin, Imagine, Kokoro, Home, Shield, Valheim, Palworld, Model vault · Backups (first copy)) (CI picture server-more-5; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "def pc_card(p, main=False, limits=None):"
- [x] Dad's PC card: D mark, Radeon RX 6750 XT 12 GB · 32 GB · 2 TB · at Dad's, OVER TAILSCALE, Healthy · 48° · Dad's using it, gives 40%, CPU 26%, GPU 35%, Doing (Ollama small model and Listen for the family, Minecraft (Dad plays on it), Backups, the second copy away from home, Steps back when Dad plays Trainz) (CI picture server-more-5; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "right = 'gives %d%%' % int(limits.get('dadMax') or 40)"
- [x] Old laptop card: L mark, Core i5 · 16 GB · 1 TB · on the shelf at home, PLUGGED IN, Healthy · 41° · lid shut, 14 W, CPU 9%, Battery UPS, Doing (Hearth Shield (a second one…), Passwords · a copy that's always on, Wakes the Nvidia PC when it's needed, Gravesleep test server) (CI picture server-more-5; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "tag = 'plugged in'"
- [ ] Each other PC's jobs, watts and limits from Hearth Server on that PC (the page reads them from each PC's status; Hearth Server on each PC that reports them isn't built yet)
- [x] Each PC's share: AI models, Storage · 25 TB, Game servers · 4, Backups · 2 copies, Always-on services, in each PC's colour (CI picture server-more-5; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "def share_of(cards):"
- [x] Each PC runs whole models: one model isn't split over the internet (too slow)… (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/pcs.rs: "one model isn't split over the internet (too slow)"
- [x] If a PC goes off: Nvidia PC · Dad's PC · Old laptop, and what moves (Ollama and Listen move to the Nvidia PC, Colibri waits its turn; Minecraft moves to the Nvidia PC, world saved first · 40 s; The second backup copy waits for it; Nothing is lost · moves back when it's on again, asks first) (CI picture server-more-5; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "def moves_if_off(cards, name, limits):"
- [ ] Moving the work by itself when a PC goes off (the plan is shown; nothing moves yet)
- [x] Tried Nov 23: Dad's PC turned off for 2 hours… (the last test, when there is one) (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/pcs.rs: "format!(\"Tried {}: {}\""
- [x] The last 7 days: each PC's on and off, on all week, off twice (Nov 21 storm, Nov 23), on all week · 14 W, 6 moves this week, all told to you, none lost anything · See every move (CI picture server-more-5; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'off %s' % ("
- [ ] The last 7 days from each PC's own record (nothing records it yet) and See every move's list
- [x] Add a PC: Any PC with Hearth OS, at home or over Tailscale. Install Hearth Server on it from the Hearth Store, then pick it here. Found on your network: Laptop (yours) Add, Truck laptop · away not a good server (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/pcs.rs: "Install Hearth Server on it from the Hearth Store, then pick it here."
- [ ] Found on your network from the network itself (listed from the helper; finding Hearth PCs on the network isn't built)
- [x] What each PC may do: Dad's PC gives at most 20% · 40% · 60%, Only when Dad isn't playing, Never touches Dad's files ALWAYS, Old laptop: always on (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/pcs.rs: "\"Never touches Dad's files\""
- [x] Moving things: Move work by itself when one's busy, Tell me each time something moves, Move it back when it's free Ask · Always (CI picture server-more-5; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/pcs.rs: "\"Move it back when it's free\""
- [ ] Hearth Server on each PC · its own Store app (Updates in the Hearth Store, on every PC at once or one at a time; Install it): Hearth Server as a Store app isn't made yet

# Game cache (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-server-expansion)

Mockups: server-expansion/07-game-cache.png
Release: next

The owner, Nov 26: "Ok we're gonna expand heavily on the Server side of hearth OS", "Let's do it", then on the boards
"thoes look good". Every part installs and updates through the Hearth Store ("All this has to be updated thru the
store"), never as a Hearth OS update; anything needing system pieces installs itself with the admin password once
("Debian system files are unlocked"). Made of blocks.

## Every part of the board
- [x] Title bar: the HS mark, Home server helper and the page's name (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/main.rs: "\"Home server helper\","
- [x] Minimize, maximize and close (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/oss-kit/kit.rs: "ui::G::Minimize"
- [x] Sidebar lines for this page set with their counts (Game servers 3 on, Game cache 612 GB, Remote play 1 playing, Media 7,412, PCs 3 PCs) (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/lib.rs: "(\"games\", \"Game servers\", Part::Games)"
- [x] Updates in the Hearth Store · every part is its own Store app · never a Hearth OS update (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/main.rs: "every part is its own Store app · never a Hearth OS update"
- [x] Made of blocks · Edit layout (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/main.rs: "tb(\"Edit layout\", 12.0, p.accent)"
- [x] The rest of the Home server helper's sidebar: Search the server, Server home, Dashboard, Model vault, Backups, This server (from the Dashboard part), these five pages hooked into it (apps/home-server pages/more.rs; CI picture server-more-w1, w4) (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/home-server/src/pages/more.rs: "a.more.view($part, &a.describe, a.p).map(Msg::More)"
- [x] ON · SAVED 1.9 TB OF DOWNLOADS tag (CI picture server-more-2; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'on · saved %s of downloads'"
- [x] Game cache and its line (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/cache.rs: "Every other device at home gets it from the server at home-network speed."
- [ ] Pre-load a game: pre-loading needs a store's own download (Steam's needs your account, which Hearth never uses); the button opens Keep a game forever's list instead
- [x] Settings (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/cache.rs: "kit::chip_acc(&p, \"Settings\""
- [x] SAVED FROM THE INTERNET 1.9 TB · since Sep 2 · about 41 hours of waiting (hearth-server-dns counts it since the cache started) (CI picture server-more-2; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'since %s · about %d hours of waiting'"
- [x] CACHE SIZE 612 GB of 1 TB, its bar (CI picture server-more-2; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "['Cache size', gb(used, 0)"
- [x] FROM THE CACHE THIS MONTH 84% of every game download at home (CI picture server-more-2; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'of every game download at home'"
- [x] RIGHT NOW 940 Mb/s to the ROG Ally · Baldur's Gate 3 update (CI picture server-more-2; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'to the %s · %s'"
- [x] How it works, right now: The internet (Steam, Epic, GOG, Battle.net, Xbox app), first time only · 300 Mb/s, Game cache on the Nvidia PC · 612 GB, home network · up to 2.5 Gb/s, each device and what it's getting (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/cache.rs: "\"How it works, right now\""
- [x] Each game: Game, Store, Cached, Got by, Saved, Used (Baldur's Gate 3 … Fortnite, + 34 more games), the one downloading now lit (hearth-server-dns counts each game) (CI picture server-more-2; needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-server-dns: "per = stats.setdefault('perGame', {})"
- [ ] Each game's name for every store: Steam's games are named (their depot's app); Epic, GOG and Xbox downloads don't name their game, so they show as "Epic Games downloads"
- [ ] Hearth OS updates and Store apps as a row: Hearth OS updates come through the home server already (hearth-update-mirror) but aren't counted on this page yet; Store apps download over https, which can't be cached
- [x] Use by device · this month: ROG Ally 412 GB, Laptop 188 GB, Xbox 116 GB, Truck laptop 41 GB · at home, Dad's PC away (CI picture server-more-2; needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-server-dns: "dm = stats.setdefault('devMonth', {})"
- [x] Stores it caches: Steam, Epic Games · GOG · Battle.net, Xbox app and the Xbox console, Hearth OS updates and Hearth Store apps, each CACHED (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/cache.rs: "kit::tag(&p, \"Cached\", Tag::Ok)"
- [ ] GOG cached: GOG downloads over https only, which LANCache can't keep (sending them to the cache would break them), so GOG goes straight through
- [x] Game downloads at home · November: each day's bars, from the cache 642 GB · from the internet 121 GB (CI picture server-more-2; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'name': datetime.date.fromtimestamp(now()).strftime('%B')"
- [x] Settings: Cache size 500 GB · 1 TB · 2 TB (hearth-server-dns cache-place) (CI picture server-more-2; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "if gbs not in ('500', '1000', '2000'):"
- [x] On: Games drive · 4 TB ▾ (the drives to pick from) (CI picture server-more-2; needs testing on a device) — hosts/linux/desktop/shell/hearth-server-hub: "'findmnt', '-J', '-b'"
- [ ] When full, drop the least used: LANCache always drops the least used when full; the switch is kept but turning it off doesn't change that yet
- [ ] Keep a game forever (Baldur's Gate 3, Valheim) · Pick: the picks are kept, but LANCache can't pin a game yet
- [ ] Pre-load overnight: Get updates before you play, for games installed on 2 or more devices, Between 2:00 – 6:00 AM (kept as choices; downloading them needs the store's own client)
- [x] How devices find it: Hearth Shield points the stores at the cache for every device at home, the Xbox and the TV too… (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/cache.rs: "Hearth Shield points the stores at the cache for every device at home, the Xbox and the TV too."
- [x] Game cache · its own Store app: Updates in the Hearth Store. It adds a system service and Hearth Shield's redirect. Install it · asks for your password once (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/cache.rs: "It adds a system service and Hearth Shield's redirect."
- [x] Encrypted downloads can't be cached by any store's rules; those go straight through. (CI picture server-more-2; needs testing on a device) — hosts/hearth-ui/apps/server-more/src/cache.rs: "Encrypted downloads can't be cached by any store's rules; those go straight through."

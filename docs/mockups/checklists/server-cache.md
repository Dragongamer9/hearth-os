# Server mode › Game download cache (approved Nov 15, 2026)

Mockups: next-update/server-cache.png
Release: 1.13

- [x] Its tile on the dashboard, and A opens this view — hosts/linux/desktop/shell/hearth-server-mode: "cache = {'id': 'cache', 'title': 'Game download cache'"
- [x] "Game download cache", "LANCache on this PC · on your home network (and Tailscale) · off until you turn it on", On / Off chip — hosts/linux/desktop/shell/CacheDetail.qml: "LANCache on this PC · on your home network (and Tailscale) · off until you turn it on"
- [x] Saved this week: "N GB not downloaded again", the line with how much came from the internet and how much went to your devices from here — hosts/linux/desktop/shell/CacheDetail.qml: "Your internet only gave each game once: "
- [x] ... measured from LANCache's own log (from the cache = saved, from the internet once) — os/system_files/usr/libexec/nimbus/hearth-server-dns: "def cache_tick(stats, conf, names, now=None):"
- [x] Saved per day, Mon–Sun bars — hosts/linux/desktop/shell/CacheDetail.qml: "model: det.v.bars || []"
- [x] Right now: each device downloading, the game, "from the cache" or "from the internet once", and its speed — hosts/linux/desktop/shell/CacheDetail.qml: "modelData.from + \" · \" + det.rate(modelData.rate)"
- [x] Stores: Steam, Epic Games, Battle.net, Xbox / Microsoft Store, each with its switch — os/system_files/usr/libexec/nimbus/hearth-server-dns: "'xbox': {'label': 'Xbox / Microsoft Store'"
- [x] Space: which drive, how much of how much, the bar; the oldest games leave first when it's full — hosts/linux/desktop/shell/CacheDetail.qml: "The oldest games leave the cache first when it's full."
- [x] If this PC isn't there: devices download from the internet as normal; nothing waits, nothing breaks — os/system_files/usr/libexec/nimbus/hearth-dns-client: "two misses in a row and the card's own settings come back"
- [x] Ⓐ Turn on or off, Ⓧ Empty the cache (asks twice), Ⓨ Settings, Ⓑ Back to the dashboard — hosts/linux/desktop/shell/CacheDetail.qml: "[\"X\", \"Empty the cache (asks twice)\", \"empty\"]"
- [x] Off until you turn it on — os/system_files/usr/libexec/nimbus/hearth-server-dns: "DEFAULTS = {'cache': {'on': False"

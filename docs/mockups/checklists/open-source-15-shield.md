# Hearth Shield (Pi-hole, EUPL-1.2, and AdGuard Home, GPL-3) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/15-shield.html)

Mockups: open-source-apps/15-shield.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps/shield), made of blocks (studio: Edit layout, Customize, its own
blocks window), built on AdGuard Home (Hearth's own, Server mode's hearth-server-dns) or a Pi-hole's API.

Board: Hearth Shield (Pi-hole/AdGuard Home): ads and trackers blocked for every device at home.

## Every part of the board
- [x] Title bar: the SH mark, Hearth Shield, "· ads and trackers blocked for every device at home" (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "ads and trackers blocked for every device at home"
- [x] ON · RUNS ON THE NVIDIA PC tag (Paused · n min left, Off) (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "On · runs on the {}"
- [x] Pause 5 min (and Block again) (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "\"Pause 5 min\""
- [x] Minimize, maximize and close (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/oss-kit/kit.rs: "ui::G::Minimize"
- [x] Side: Today, Devices 11, What was asked live, Block lists 6, Always allow 14, Always block 9, Family rules, Settings (each its own page) (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "item(Page::Family, \"Family rules\""
- [x] HOW IT WORKS and its words (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "as the address book for the whole house, so every device is covered, even ones you can't install apps on."
- [x] IF THE PC IS OFF: "Devices fall back to the router's normal one by themselves. Nothing stops working." (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "Devices fall back to the router's normal one by themselves. Nothing stops working."
- [x] Today and "Since midnight · updated a second ago" (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "updated a second ago"
- [x] Today / 7 days / 30 days (each day's totals kept by hearth-server-dns) (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "\"7 days\""
- [x] ASKED 48,213 · look-ups from 11 devices (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "look-ups from {} devices"
- [x] BLOCKED 11,904 · 24.7% of everything asked (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "% of everything asked"
- [x] BLOCK LISTS 1.21 M · addresses in 6 lists · updated 3:00 AM (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "addresses in {} lists"
- [x] TIME SAVED ~ 41 min · of ads not loaded (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "of ads not loaded"
- [x] Through the day: the hours' bars, allowed and blocked, with their legend and 12 AM · 6 AM · 12 PM · 6 PM · now (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "\"Through the day\""
- [x] Devices · give one its own rules from its page: DEVICE, ASKED, BLOCKED, SHARE BLOCKED, each with its mark, name, line and › (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "\"SHARE BLOCKED\""
- [x] A device's page: its numbers, its name, its own rules (block for it, don't, pause an hour) (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "fn device_page"
- [x] Blocked the most · today: what it is, who asked, how many (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "\"Blocked the most\""
- [x] BLOCK LISTS: Ads and trackers (main list), Smart TV spying, Phone app trackers, Scam and malware sites, Game launcher ads, Adult sites, each a switch (CI pictures shield-1…10; needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-server-dns: "'label': 'Smart TV spying'"
- [x] Something broke on a site? Open What was asked, find it, press Allow. It's back in seconds, and Hearth remembers it. (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "Open What was asked, find it, press Allow. It's back in seconds, and Hearth remembers it."
- [x] What was asked, live, with Allow and Block (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "fn log_page"
- [x] Always allow and Always block: the sites, add and remove (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "fn domains_page"
- [x] Family rules: adult sites, safe search (CI pictures shield-1…10; needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-server-dns: "safe-search on|off"
- [x] Settings: where it runs (this PC's AdGuard Home or a Pi-hole, its password only in the keyring) and how devices fall back (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "Its password is kept only in this PC's keyring, never in a file."
- [x] Shield off: Turn Shield on (AdGuard Home as Server mode runs it, never part of starting the PC) (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "\"Turn Shield on\""
- [x] Credit: Hearth Shield · Built on Pi-hole (EUPL-1.2) and AdGuard Home (GPL-3) (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "\"Pi-hole (EUPL-1.2) and AdGuard Home (GPL-3)\""
- [x] Made of blocks: the window (side, page, right side), Today (head, numbers, chart, devices), the right side (top, lists, tip), and every other page (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "bl(\"kpis\""
- [x] Its own blocks window: its buttons, panes and info (CI pictures shield-1…10; needs testing on a device) — hosts/hearth-ui/apps/shield/src/main.rs: "name: \"Shield\""

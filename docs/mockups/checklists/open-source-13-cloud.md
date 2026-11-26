# Hearth Cloud (Nextcloud, AGPL-3) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/13-cloud.html)

Mockups: open-source-apps/13-cloud.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps/cloud), made of blocks (studio: Edit layout, Customize, its own
blocks window), built on the open-source project the board credits: Nextcloud on the Nvidia PC (hearth-oss-server, its
own image in rootless Podman, 127.0.0.1 and Tailscale only) through WebDAV, CalDAV, CardDAV and its OCS API.

Board: Hearth Cloud (Nextcloud, AGPL-3): his own cloud on the Nvidia PC, files, calendar and contacts on his phone and laptop.
The board's top bar and wallpaper are the desktop's. Pictures: hearth-apps-rust.yml, cloud.sh (cloud-1-board …).

## Every part of the board

### The title bar
- [x] The CL letter mark and the name Hearth Cloud — hosts/hearth-ui/apps/cloud/src/main.rs: "w::mark(\"CL\", \"bl\", 22.0)"
- [x] "· your own cloud, on your Nvidia PC" — hosts/hearth-ui/apps/cloud/src/main.rs: "· your own cloud, on your Nvidia PC"
- [x] SERVER ON · NVIDIA PC (Nextcloud's own service, from hearth-oss-server) — hosts/hearth-ui/apps/cloud/src/main.rs: "w::tag(&p, \"Server on · Nvidia PC\", Tg::Ok)"
- [x] – ▢ ✕ — hosts/hearth-ui/apps/cloud/src/main.rs: "TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close }"

### The side
- [x] ⌕ Search files, events, people — hosts/hearth-ui/apps/cloud/src/main.rs: "w::search(&p, \"Search files, events, people\", &a.q, Msg::Search)"
- [x] Home, Files 48,210, Calendar "3 today", Contacts 214, Shared with others 6, Devices 4, Bin 12 (Home lit; each opens its page) — hosts/hearth-ui/apps/cloud/src/main.rs: "(\"calendar\", \"Calendar\", format!(\"{today_n} today\"))"
- [x] SPACE ON THE SERVER: its bar and "1.24 TB of 4 TB used" (WebDAV's quota) — hosts/hearth-ui/apps/cloud/src/dav.rs: "\"quota-used-bytes\" => e.used = txt.parse().unwrap_or(0),"
- [x] "Photos live in the Home server helper. Hearth Cloud keeps files, calendar and contacts." — hosts/hearth-ui/apps/cloud/src/main.rs: "Photos live in the Home server helper. Hearth Cloud keeps files, calendar and contacts."
- [x] FROM OUTSIDE HOME: "Reached over Tailscale only · not open to the internet" (it listens only on 127.0.0.1 and Tailscale) — hosts/hearth-ui/apps/cloud/src/main.rs: "\"not open to the internet\""

### Home
- [x] Home, "Everything synced · last change 1:21 AM from the laptop" — hosts/hearth-ui/apps/cloud/src/main.rs: "format!(\"Everything synced · last change {}{}\""
- [x] ＋ Upload (a WebDAV PUT, marked with the Hearth device it came from) — hosts/hearth-ui/apps/cloud/src/main.rs: "acct.send(\"PUT\", &path, bytes, \"application/octet-stream\", &[])?;"
- [x] ＋ New folder (MKCOL) — hosts/hearth-ui/apps/cloud/src/main.rs: "acct.send(\"MKCOL\""
- [x] Add a device (in the accent): what to sign in with on a phone or a Hearth device, and where — hosts/hearth-ui/apps/cloud/src/main.rs: "On a phone: Nextcloud's own app (files), DAVx⁵ (calendar and contacts)."
- [x] Recent files with All / Documents / Projects — hosts/hearth-ui/apps/cloud/src/main.rs: "(\"Projects\".into(), a.tab == \"projects\", Some(Msg::Tab(\"projects\")))"
- [x] Its table: NAME (each file's colour square), WHERE, CHANGED, FROM, ⋯ (Open, Share a link, Delete), newest first (DAV SEARCH) — hosts/hearth-ui/apps/cloud/src/dav.rs: "<d:orderby><d:order><d:prop><d:getlastmodified/></d:prop><d:descending/></d:order></d:orderby>"
- [x] FROM: the Hearth device that changed it (Hearth's own WebDAV property, written when Hearth Cloud uploads), else who did — hosts/hearth-ui/apps/cloud/src/dav.rs: "\"device\" if n.tag_name().namespace() == Some(NS) => e.from = txt,"
- [x] FROM for files changed by other programs (Nextcloud's phone app, the desktop client, a browser): Hearth's own small Nextcloud app, put in by Hearth Cloud, notes the device of every write (CI job nextcloud-device: a phone's upload read back with its device, in the real Nextcloud image; needs testing on a device) — hosts/hearth-ui/apps/cloud/nextcloud/hearthdevice/lib/Listener/Written.php: "public const NAME = '{https://hearthos.fun/ns}device';"
- [x] Today · Thu Nov 26 and ＋ Event (a CalDAV PUT) — hosts/hearth-ui/apps/cloud/src/main.rs: "format!(\"Today · {} {} {}\", DAYS[now.wday]"
- [x] The week M–S with today in the accent and the days with events underlined in blue — hosts/hearth-ui/apps/cloud/src/main.rs: "style(ui::boxed(if has { w::blue() } else { Color::TRANSPARENT }, 2.0, None))"
- [x] Today's events: All day Thanksgiving (Family · at Dad's), 2:00 PM Dinner, 10:00 PM Work, overnight shift (until 6:00 AM · repeats Mon–Fri), each with its calendar's colour line (CalDAV) — hosts/hearth-ui/apps/cloud/src/dav.rs: "\"repeats Mon–Fri\".into()"
- [x] Devices, "all up to date": Phone, Laptop · Hearth OS, ROG Ally (SYNCED), Dad's PC (SHARED) — hosts/hearth-ui/apps/cloud/src/main.rs: "w::tag(&p, if d.shared { \"Shared\" } else { \"Synced\" }, if d.shared { Tg::Bl } else { Tg::Ok })"
- [x] Contacts, "214 · on every device": each with its round letter and "Phone, email · family" (CardDAV) — hosts/hearth-ui/apps/cloud/src/main.rs: "format!(\"{} · on every device\", a.d.contacts.len())"
- [x] Shared with others: Trainz routes (with Dad · can change, Manage), Hearth OS ISO test (link · ends Dec 3, Copy link), Truck console prints (link · view only), Family calendar (with 4 people) — hosts/hearth-ui/apps/cloud/src/main.rs: "\"/ocs/v2.php/apps/files_sharing/api/v1/shares\""
- [x] Manage: can change on or off, stop sharing — hosts/hearth-ui/apps/cloud/src/main.rs: "w::chip(&p, \"Stop sharing\", false, Sz::Sm, Some(Msg::Unshare(k)))"
- [x] "Links ask for a password and end by themselves." (Nextcloud's own sharing rules, set by Hearth; Hearth's links have a password and end in a week) — hosts/linux/desktop/shell/hearth-oss-server: "shareapi_enforce_links_password"

### The credit, the blocks, the server
- [x] "Hearth Cloud · Built on Nextcloud (AGPL-3)" — hosts/hearth-ui/apps/cloud/src/main.rs: "w::credit(&p, \"Hearth Cloud\", \"Built on Nextcloud (AGPL-3)\")"
- [x] Every part a block (3 pages, 14 blocks), Edit layout in the side, its own blocks window (10 buttons, 3 panes, 4 info) — hosts/hearth-ui/apps/cloud/src/main.rs: "name: \"Cloud\","
- [x] Signing in Nextcloud's own way (Login Flow v2), its app password in the keyring — hosts/hearth-ui/apps/cloud/src/dav.rs: "/index.php/login/v2"
- [x] Nextcloud installed and turned on the Hearth way when it isn't there (its image in your Podman, its user service guarded) — os/system_files/usr/lib/systemd/user/hearth-nextcloud.service: "ConditionPathExists=%h/.local/state/nimbus/oss-server/nextcloud.ready"

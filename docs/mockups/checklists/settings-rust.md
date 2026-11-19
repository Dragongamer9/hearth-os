# Hearth Settings as its own pure-Rust app (layout A of the approved new Settings, for the Fedora edition)

Mockups: next-update/settings-A1-home-mouse.png, next-update/settings-A2-display-mouse.png
Release: next

The owner (Nov 18, 2026): Settings is its own app, not tied to the Game Mode app, in pure Rust like the new Task
Manager (hosts/hearth-ui/apps/settings). It follows the Global Theme picked in KDE, as every Hearth app must.

## The frame
- [x] left list: you (picture, name, Administrator · PC name) — hosts/hearth-ui/apps/settings/src/frame.rs: "\"Administrator\" } else { \"Standard account\" }"
- [x] Search settings, finding pages by name and words — hosts/hearth-ui/apps/settings/src/frame.rs: "field(&p, \"Search settings\", &a.query, Msg::Query)"
- [x] the 13 categories with the picked one marked — hosts/hearth-ui/apps/settings/src/registry.rs: "assert_eq!(CATEGORIES.len(), 13);"
- [x] the page's path as its title (System › Display) — hosts/hearth-ui/apps/settings/src/frame.rs: "fn crumbs(a: &App)"
- [x] button hints: A Select, B Back, Y Search, LB/RB categories, Close — hosts/hearth-ui/apps/settings/src/frame.rs: "\"Previous / next category\""
- [x] follows the Global Theme picked in KDE's System Settings, Hearth's looks or any other — hosts/hearth-ui/src/theme.rs: "fn from_plasma() -> Option<Pal>"
- [ ] layout B (the controller look) and switching to it when a controller is used
- [ ] the controller moving between rows (A, B, LB/RB work as keys only: Esc, Page Up / Page Down)

## Home (settings-A1)
- [x] the PC card: picture, name, Hearth OS version · mode · graphics card · processor, Rename and About this PC — hosts/hearth-ui/apps/settings/src/cats/home.rs: "pill(&p, \"About this PC\""
- [x] four quick tiles: Wi-Fi, Bluetooth, Updates, Hearth Link with their state — hosts/hearth-ui/apps/settings/src/cats/home.rs: "tile(&p, G::Link, \"Hearth Link\""
- [x] Recommended settings: Night light (switch), Start in (Game Mode / Desktop Mode) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"What this PC opens when it turns on\", seg(&p"
- [x] Recommended settings: Light mode while you play, Do not disturb — hosts/hearth-ui/apps/settings/src/cats/home.rs: "Hold notifications until you turn it off"
- [x] Storage: each drive with its bar and used of total, Manage — hosts/hearth-ui/apps/settings/src/cats/home.rs: "Some((\"Manage\", Msg::Go(\"storage\")))"
- [x] Changed lately: the last changes (what → to, who, when), each with Undo — hosts/hearth-ui/apps/settings/src/cats/home.rs: "btn(&p, \"Undo\""
- [x] All changes (the full list page, each with Undo and what it was) — hosts/hearth-ui/apps/settings/src/cats/home.rs: "What was changed in Settings, newest first. Undo puts the old value back."
- [x] Quick (See more): the big tiles of settings-B1 (Light mode while you play wide, Wi-Fi, Bluetooth, Brightness and Volume with sliders, Start in wide, Updates, Power, Night light, Do not disturb) and Pin to Quick — hosts/hearth-ui/apps/settings/src/cats/home.rs: "Pin to Quick: pick what's on this page"
- [x] Hearth Link's tile shows its devices (3 devices) — hosts/hearth-ui/apps/settings/src/cats/link.rs: "pub fn summary_of(v: &Value) -> String"

## A page (settings-A2, System › Display)
- [x] the displays drawn as you have them (1, 2), with the main one's name, resolution, refresh and graphics card — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"{} · main display\""
- [ ] Identify, Detect, Arrange, HDR
- [x] sections with headings (Brightness & colour, Scale & layout), each setting a row with its icon, name, line under it and its control — hosts/hearth-ui/apps/settings/src/cats/system.rs: "head(&p, \"Scale & layout\")"
- [x] Night light, Brightness, Scale, Resolution, Refresh rate — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Refresh rate\", \"How many pictures a second the display shows\""
- [x] X Keep these settings on a display change, and the old one back by itself if not kept — hosts/hearth-ui/apps/settings/src/cats/system.rs: "fn display_change("
- [ ] Related: Game Mode's resolution and frame limit

## Pages built (beyond the mockups' two)
- [x] Sound: volume, mute, speakers and microphone to use — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Recording from here\""
- [x] Power & battery: battery, power mode — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Power saver for a longer battery, Performance for games\""
- [x] Storage, Startup, Date & time (time zone, set by itself), About (with Rename) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Find a time zone\""
- [x] Your desktop: KDE Plasma, Firehold (Rust, test), Firehold (Plasma, test), Switch now — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Firehold (Plasma, test)\""
- [x] Wi-Fi: on/off, networks, connect with a password, disconnect, forget — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"The network's password:\""
- [x] Bluetooth: on/off, your devices (connect, forget), look for and pair new ones — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"Look for devices\""
- [x] Personalization: the 8 looks, dark / light, the accent (KDE's Global Theme follows) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"Accent colour\", \"Buttons, switches and what's picked\""
- [x] Updates & recovery: the status card (look for updates, download, restart), progress line by line while updating, Update anyway, Shut down when it's done (and its countdown with Cancel), Shut down and finish updating — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "Updated. Shutting down in"
- [x] Updates & recovery: a restore point before updating and before the writable system is switched, Go back (asks first) and Stay on, System Restore, Reliability history, the previous version kept — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "Yes, go back at the next restart"
- [x] Updates & recovery: Decky plugins (each plugin, New from Hearth, Also install new Hearth plugins, Check now, Update, Update plugins automatically, Use the built-in plugins, the last error) — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "Also install new Hearth plugins"
- [x] Updates & recovery: the writable system where hearth-unlock is (Debian's; none on the Fedora edition, so it doesn't show) — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "Turn my changes back on"
- [x] Backup and help: Logs, Send logs to the developer (with its code), Back up Hearth to a drive, Restore from a backup — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "Send logs to the developer"
- [x] Hearth Link: this device and Rename, Share this device's AI, the kind of device you pair (My device, A family member's device, A friend's device), Pair a device with its codes, nearby devices, a code from somewhere else, the long link — hosts/hearth-ui/apps/settings/src/cats/link.rs: "A family member's device"
- [x] Hearth Link: your devices with each one's menu (what it is, unpair), copies (pause, resume), sync, accept games, share status, relay, Hearth Storage (role, servers & devices with remove, shared folders, backups & saves, advanced) — hosts/hearth-ui/apps/settings/src/cats/link.rs: "Hearth Storage · This device"
- [x] Your devices: every device's card, this PC as the home server, share status and Show other devices in Files; one device with the last hour's graphs, its server, now, and what to do with it — hosts/hearth-ui/apps/settings/src/cats/link.rs: "Do something with it"
- [x] Home server: the landing (what needs a look, four numbers, its pages in four groups, Light mode, hidden items), Set up your home server (its five steps, the sign-in QR code) — hosts/hearth-ui/apps/settings/src/cats/hs/overview.rs: "1 thing needs a look"
- [x] Home server: Server mode (power when idle, what pauses when you play, the web page and its PIN pad and permissions), AI engine (Colibri: install, engine, models in every group, models folder, download, the plan) — hosts/hearth-ui/apps/settings/src/cats/hs/overview.rs: "Graphics card limit when idle"
- [x] Home server: Shared folders (each share's settings, whole drives, Home and the system drive asked twice, the sharing password, how to open them), Files & links, People, Family & home — hosts/hearth-ui/apps/settings/src/cats/hs/sharing.rs: "Share the system drive (read only)"
- [x] Home server: Services (media server, Files on the web, AI pictures with video, Voice Studio with its PIN and key, AI voices, AI chat), AI chat, AI devices, Server apps (each app, world backups), Phone notifications, Gaming — hosts/hearth-ui/apps/settings/src/cats/hs/services.rs: "Back up the worlds every night"
- [x] Home server: Drives & undo (health, snapshots, browse and restore, adding a drive asked twice), Backups & sync (each backup, restore, adding one, devices backing up here, phone photos, folder sync), Safety, Alerts & power (with the power history graph and UPS) — hosts/hearth-ui/apps/settings/src/cats/hs/safe.rs: "Erase {} now"
- [ ] every other page of Mantel's Settings (listed on its category as "Coming next")

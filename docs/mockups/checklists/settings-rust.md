# Hearth Settings as its own pure-Rust app (layout A of the approved new Settings, for the Fedora edition)

Mockups: next-update/settings-A1-home-mouse.png, next-update/settings-A2-display-mouse.png
Release: next

The owner (Nov 18, 2026): Settings is its own app, not tied to the Game Mode app, in pure Rust like the new Task
Manager (hosts/hearth-ui/apps/settings). It follows the Global Theme picked in KDE, as every Hearth app must.

## The frame
- [x] left list: you (picture, name, Administrator · PC name) — hosts/hearth-ui/apps/settings/src/pages.rs: "\"Administrator\" } else { \"Standard account\" }"
- [x] Search settings, finding pages by name and words — hosts/hearth-ui/apps/settings/src/pages.rs: "text_input(\"Search settings\", &a.query)"
- [x] the 13 categories with the picked one marked — hosts/hearth-ui/apps/settings/src/main.rs: "pub const CATS: [(&str, &str, &str); 13]"
- [x] the page's path as its title (System › Display) — hosts/hearth-ui/apps/settings/src/pages.rs: "fn crumbs(a: &App)"
- [x] button hints: A Select, B Back, Y Search, LB/RB categories, Close — hosts/hearth-ui/apps/settings/src/pages.rs: "\"Previous / next category\""
- [x] follows the Global Theme picked in KDE's System Settings, Hearth's looks or any other — hosts/hearth-ui/src/theme.rs: "fn from_plasma() -> Option<Pal>"
- [ ] layout B (the controller look) and switching to it when a controller is used
- [ ] the controller moving between rows (A, B, LB/RB work as keys only: Esc, Page Up / Page Down)

## Home (settings-A1)
- [x] the PC card: picture, name, Hearth OS version · mode · graphics card · processor, Rename and About this PC — hosts/hearth-ui/apps/settings/src/pages.rs: "pill(&p, \"About this PC\""
- [x] four quick tiles: Wi-Fi, Bluetooth, Updates, Hearth Link with their state — hosts/hearth-ui/apps/settings/src/pages.rs: "tile(&p, G::Link, \"Hearth Link\""
- [x] Recommended settings: Night light (switch), Start in (Game Mode / Desktop Mode) — hosts/hearth-ui/apps/settings/src/pages.rs: "\"What this PC opens when it turns on\", seg(&p"
- [ ] Recommended settings: Light mode while you play, Do not disturb
- [x] Storage: each drive with its bar and used of total, Manage — hosts/hearth-ui/apps/settings/src/pages.rs: "tm(\"Manage\", 14.0, p.accent)"
- [x] Changed lately: the last changes (what → to, who, when), each with Undo — hosts/hearth-ui/apps/settings/src/pages.rs: "btn(&p, \"Undo\""
- [ ] All changes (the full list page)
- [ ] Hearth Link's tile shows its devices

## A page (settings-A2, System › Display)
- [x] the displays drawn as you have them (1, 2), with the main one's name, resolution, refresh and graphics card — hosts/hearth-ui/apps/settings/src/pages.rs: "\"{} · main display\""
- [ ] Identify, Detect, Arrange, HDR
- [x] sections with headings (Brightness & colour, Scale & layout), each setting a row with its icon, name, line under it and its control — hosts/hearth-ui/apps/settings/src/pages.rs: "head(&p, \"Scale & layout\")"
- [x] Night light, Brightness, Scale, Resolution, Refresh rate — hosts/hearth-ui/apps/settings/src/pages.rs: "\"Refresh rate\", \"How many pictures a second the display shows\""
- [x] X Keep these settings on a display change, and the old one back by itself if not kept — hosts/hearth-ui/apps/settings/src/main.rs: "fn display_change("
- [ ] Related: Game Mode's resolution and frame limit

## Pages built (beyond the mockups' two)
- [x] Sound: volume, mute, speakers and microphone to use — hosts/hearth-ui/apps/settings/src/pages.rs: "\"Recording from here\""
- [x] Power & battery: battery, power mode — hosts/hearth-ui/apps/settings/src/pages.rs: "\"Power saver for a longer battery, Performance for games\""
- [x] Storage, Startup, Date & time (time zone, set by itself), About (with Rename) — hosts/hearth-ui/apps/settings/src/pages.rs: "\"Find a time zone\""
- [x] Your desktop: KDE Plasma, Firehold (Rust, test), Firehold (Plasma, test), Switch now — hosts/hearth-ui/apps/settings/src/pages.rs: "\"Firehold (Plasma, test)\""
- [x] Wi-Fi: on/off, networks, connect with a password, disconnect, forget — hosts/hearth-ui/apps/settings/src/pages.rs: "fn join_dialog"
- [x] Bluetooth: on/off, your devices (connect, forget), look for and pair new ones — hosts/hearth-ui/apps/settings/src/pages.rs: "\"Look for devices\""
- [x] Personalization: the 8 looks, dark / light, the accent (KDE's Global Theme follows) — hosts/hearth-ui/apps/settings/src/pages.rs: "\"Accent colour\", \"Buttons, switches and what's picked\""
- [x] Updates & recovery: check, update (a restore point first), restart, go back a version, System Restore, Reliability history — hosts/hearth-ui/apps/settings/src/pages.rs: "\"Go back a version\""
- [ ] every other page of Mantel's Settings (listed on its category as "Coming next")

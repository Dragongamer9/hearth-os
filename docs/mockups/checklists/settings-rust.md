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
- [x] Recommended settings: Night light (switch), Start in (Game Mode / Desktop Mode) — hosts/hearth-ui/apps/settings/src/cats/home.rs: "\"What this PC opens when it turns on\", seg(&p"
- [ ] Recommended settings: Light mode while you play, Do not disturb
- [x] Storage: each drive with its bar and used of total, Manage — hosts/hearth-ui/apps/settings/src/cats/home.rs: "Some((\"Manage\", Msg::Go(\"storage\")))"
- [x] Changed lately: the last changes (what → to, who, when), each with Undo — hosts/hearth-ui/apps/settings/src/cats/home.rs: "btn(&p, \"Undo\""
- [ ] All changes (the full list page)
- [ ] Hearth Link's tile shows its devices

## A page (settings-A2, System › Display)
- [x] the displays drawn as you have them (1, 2), with the main one's name, resolution, refresh and graphics card — hosts/hearth-ui/apps/settings/src/cats/system_display.rs: "\" · main display\""
- [x] Identify, Detect, Arrange, HDR — hosts/hearth-ui/apps/settings/src/cats/system_display.rs: "pill(&p, \"Identify\", true, identify)"
- [x] sections with headings (Brightness & colour, Scale & layout), each setting a row with its icon, name, line under it and its control — hosts/hearth-ui/apps/settings/src/cats/system_display.rs: "head(&p, \"Scale & layout\")"
- [x] Night light, Brightness, Scale, Resolution (with Other…), Refresh rate (with whether variable refresh is on) — hosts/hearth-ui/apps/settings/src/cats/system_display.rs: "\"Refresh rate\", vrr_line"
- [x] X Keep these settings on a display change, and the old one back by itself if not kept — hosts/hearth-ui/apps/settings/src/cats/system_display.rs: "pub fn try_change("
- [x] Related: Game Mode's resolution and frame limit — hosts/hearth-ui/apps/settings/src/cats/system_display.rs: "\"Game Mode's resolution and frame limit\", \"Gaming › Performance\", Msg::Go(\"gameperf\")"

## Pages built (beyond the mockups' two)
- [x] Sound: volume, mute, speakers and microphone to use — hosts/hearth-ui/apps/settings/src/cats/system_sound.rs: "\"Recording from here\""
- [x] Power & battery: battery, power mode — hosts/hearth-ui/apps/settings/src/cats/system_power.rs: "\"Battery saver lasts longer; Performance is faster and warmer\""
- [x] Storage, Startup, Date & time (time zone, set by itself), About (with Rename) — hosts/hearth-ui/apps/settings/src/cats/system_lang.rs: "\"Find a time zone\""
- [x] Your desktop: KDE Plasma, Firehold (Rust, test), Firehold (Plasma, test), Switch now — hosts/hearth-ui/apps/settings/src/cats/system_more.rs: "\"Firehold (Plasma, test)\""
- [x] Wi-Fi: on/off, networks, connect with a password, disconnect, forget — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"The network's password:\""
- [x] Bluetooth: on/off, your devices (connect, forget), look for and pair new ones — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"Look for devices\""
- [x] Personalization: the 8 looks, dark / light, the accent (KDE's Global Theme follows) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"Accent colour\", \"Buttons, switches and what's picked\""
- [x] Updates & recovery: check, update (a restore point first), restart, go back a version, System Restore, Reliability history — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "\"Go back a version\""
- [ ] every other page of Mantel's Settings (listed on its category as "Coming next")

## System: every page of Mantel's System category (ported from its Set*.qml, same helpers; KDE Plasma's own settings too)
Pictured in CI: settings-sys-<page> (.github/scripts/hearth-apps-rust-shots.d/settings-system.sh, from examples).
- [x] Display: auto-brightness, external screens' brightness, Night light's warmth / when / place / fade, Game Mode's HDR, HDR and colour on the desktop, picture signal, VRR and tearing, rotation, X11 apps, several screens, screen power saving, Nvidia drawing — hosts/hearth-ui/apps/settings/src/cats/system_display.rs: "\"Sticky edge between screens\""
- [x] Sound: over-amplification (asks first; Plasma's 150% too), apps playing sound, sound cards' profiles, the microphone test — hosts/hearth-ui/apps/settings/src/cats/system_sound.rs: "\"Louder than 100% can hurt\""
- [x] Speakers & balance: output, balance, each speaker around you with a test tone, all in turn — hosts/hearth-ui/apps/settings/src/cats/system_sound.rs: "\"Play each speaker in turn\""
- [x] Apps' sound: each app's volume and speakers, each app's microphone, the dock's scrolling — hosts/hearth-ui/apps/settings/src/cats/system_sound.rs: "\"Apps using a microphone\""
- [x] Push to talk: the switch, the key, how it works (KDE Plasma has none, so it says Mantel runs it) — hosts/hearth-ui/apps/settings/src/cats/system_sound.rs: "\"Push to talk needs Mantel on KWin\""
- [x] Sounds: the set (KDE's sound theme follows), each event with Play — hosts/hearth-ui/apps/settings/src/cats/system_sound.rs: "\"Where they come from\""
- [x] Notifications: Do not disturb, quiet in full screen and while sharing (Plasma's plasmanotifyrc), on a schedule, history, each app — hosts/hearth-ui/apps/settings/src/cats/system_more.rs: "\"WhenScreenSharing\", \"WhenScreensMirrored\""
- [x] Power & battery: charge limit, power limit, screen off and sleep times and the power button (PowerDevil on Plasma), Turn off or switch with Hibernate's first-time question — hosts/hearth-ui/apps/settings/src/cats/system_power.rs: "fn reparse()"
- [x] Plugged in, battery & lid: the two columns, the lid on battery / plugged in / with a screen — hosts/hearth-ui/apps/settings/src/cats/system_power.rs: "\"With another screen attached\""
- [x] Apps keeping it awake, read again every few seconds, Sleep anyway — hosts/hearth-ui/apps/settings/src/cats/system_power.rs: "\"Asking for a moment first\""
- [x] Battery health & history (and Info Center › Energy): the health ring, the facts, the charge and power graphs, your devices' batteries — hosts/hearth-ui/apps/settings/src/cats/system_power.rs: "impl<M> canvas::Program<M> for Ring"
- [x] Memory: zram, keep memory free, swappiness, the swap file, when memory runs out, Save / Try until restart / Back to recommended — hosts/hearth-ui/apps/settings/src/cats/system_more.rs: "\"Try until restart\""
- [x] Storage: drives, games by size, clean up, the low space warning (Hearth's core), df when the core can't answer — hosts/hearth-ui/apps/settings/src/cats/system_more.rs: "\"Low means less than\""
- [x] Drives & folders: Hearth (C:) with Windows names, system folders, address bar, Terminal speaks C:\\, Copy path, drive letters — hosts/hearth-ui/apps/settings/src/cats/system_more.rs: "\"Copy path gives\""
- [x] Startup: Start in (Game Mode, Desktop Mode, Server mode), the schedule, reopen apps and confirm logging out (ksmserverrc on Plasma), default leave option — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Reopen my apps from last time\""
- [x] Boot splash: each splash, Use this only after you pick it — hosts/hearth-ui/apps/settings/src/cats/system_more.rs: "\"Use this\""
- [x] Remote desktop: on/off, who's connected, how to connect, password, reachable, view only — hosts/hearth-ui/apps/settings/src/cats/system_more.rs: "\"Who can connect and what they can do\""
- [x] Date & time: the clock, 12 / 24-hour (Plasma's clock too), automatic time, the clock by hand, time zone by search or region, the clock's panel with world clocks — hosts/hearth-ui/apps/settings/src/cats/system_lang.rs: "\"Add a world clock\""
- [x] Region & language: language, formats, change one of them, sign out — hosts/hearth-ui/apps/settings/src/cats/system_lang.rs: "\"Change one of them\""
- [x] Languages in order, Spell check, Translations & dictionaries, Chinese / Japanese / Korean typing — hosts/hearth-ui/apps/settings/src/cats/system_lang.rs: "\"Download what's missing\""
- [x] Clipboard: history, selection, in step (Klipper on Plasma), open, clear — hosts/hearth-ui/apps/settings/src/cats/system_lang.rs: "\"Keep the clipboard and selection in step\""
- [x] Search: on Plasma the file index (Baloo) and KRunner's plugins; elsewhere Hearth's index — hosts/hearth-ui/apps/settings/src/cats/system_lang.rs: "fn search_plasma("
- [x] About: version and date, the facts, Rename, Resource Monitor, Info Center — hosts/hearth-ui/apps/settings/src/cats/system_about.rs: "\"Resource Monitor\""
- [x] Info Center: every part as a tab, sections, gauges, lists with Find, Copy this page — hosts/hearth-ui/apps/settings/src/cats/system_about.rs: "\"Copy this page\""
- [x] Install Hearth: says it's for the live USB (and opens the installer there) — hosts/hearth-ui/apps/settings/src/cats/system_about.rs: "\"Install Hearth OS on this PC\""
- [ ] Boot splash: each splash's picture and its played preview (hearth-ui draws no pictures yet: no image support in its iced features)
- [ ] Display: Hearth's own HDR calibration pictures (HdrCalibrate.qml needs Mantel; on Plasma the row opens KDE's own calibration)
- [ ] Display: Identify outside KDE Plasma (it uses KDE's own screen numbers, kscreen's OSD)
- [ ] Startup: editing the schedule's changes on this page (shown here; changed on Home server › Server mode)

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
- [x] Recommended settings: Light mode while you play, Do not disturb — hosts/hearth-ui/apps/settings/src/cats/home.rs: "Hold notifications until you turn it off"
- [x] Storage: each drive with its bar and used of total, Manage — hosts/hearth-ui/apps/settings/src/cats/home.rs: "Some((\"Manage\", Msg::Go(\"storage\")))"
- [x] Changed lately: the last changes (what → to, who, when), each with Undo — hosts/hearth-ui/apps/settings/src/cats/home.rs: "btn(&p, \"Undo\""
- [x] All changes (the full list page, each with Undo and what it was) — hosts/hearth-ui/apps/settings/src/cats/home.rs: "What was changed in Settings, newest first. Undo puts the old value back."
- [x] Quick (See more): the big tiles of settings-B1 (Light mode while you play wide, Wi-Fi, Bluetooth, Brightness and Volume with sliders, Start in wide, Updates, Power, Night light, Do not disturb) and Pin to Quick — hosts/hearth-ui/apps/settings/src/cats/home.rs: "Pin to Quick: pick what's on this page"
- [x] Hearth Link's tile shows its devices (3 devices) — hosts/hearth-ui/apps/settings/src/cats/link.rs: "pub fn summary_of(v: &Value) -> String"

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

## Apps (Mantel's Apps category, for the Fedora edition)
- [x] Installed apps: Hearth's Debian box (state, its packages with Remove, Install a .deb, Reset, Remove the box), Bazaar (open, its install, Fix your Flatpak folder), your Flathub apps with Remove, AppImages — hosts/hearth-ui/apps/settings/src/cats/apps/installed.rs: "\"Reset the Debian box…\""
- [x] App store: search, Discover (categories, Popular, Trending, New and updated), a category or a search — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "\"New and updated\""
- [x] App store: an app's page (name, summary, developer, rating, size, Install / Open / Remove, description, reviews, license, app id, website) — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "\"No ratings yet\""
- [x] App store: Updates (Tell me when app updates are there, Update all, each update, new permissions shown and Update and allow, Hearth OS updates, Firmware) — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "\"Update and allow\""
- [x] App store: Sources (each remote, remove yours, add a .flatpakrepo) — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "\"Add a source…\""
- [x] App store: Drivers, the Fedora way (the PC's parts and the driver in use from lspci, the firmware and graphics packages from rpm, firmware the kernel couldn't load; drivers come with Hearth OS's updates) — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "pub const FEDORA_DRIVERS"
- [x] App store: B goes back from an app to its list, and `store:updates` / `store:sources` / `store:drivers` / `store:app-<id>` open there — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "\"drivers\" => update(sh, st, M::Drivers)"
- [ ] App store: app icons and screenshots as pictures (the settings app has no image support yet: a store mark and the screenshots' count stand in)
- [x] Windows apps: the built-in Proton, Open a Windows program, each app's Open, Show log (its last start), the detailed Proton log once, Try with the next Proton, Proton, DXVK, VKD3D, Windows version, files from Windows, the fixes, In Steam, Remove, downloading a Proton after a yes — hosts/hearth-ui/apps/settings/src/cats/apps/winapps.rs: "\"Detailed Proton log for the next start\""
- [x] Default apps: every kind with its app, its choices, Reset to Hearth's default, Reset to Hearth's defaults (with Yes / Cancel) — hosts/hearth-ui/apps/settings/src/cats/apps/defaults.rs: "\"Reset to Hearth's defaults…\""
- [x] File types: find by name or ending, the groups, each kind's Opens in, Use the system's choice, remove and add endings — hosts/hearth-ui/apps/settings/src/cats/apps/filetypes.rs: "\"Add an ending, like *.myext\""
- [x] Autostart: what starts at sign-in with switches, Remove yours, Add an app (found in the menu's apps), Add a command, Background services — hosts/hearth-ui/apps/settings/src/cats/apps/autostart.rs: "\"Add this command\""
- [x] App menu editor: pick an app, its name, description, icon and command, Save, menu category, Run in a terminal, Show in the app menu, Put back how it was / Remove this entry, Add your own; `menu-editor:<app id>` — hosts/hearth-ui/apps/settings/src/cats/apps/menued.rs: "\"Save the name, description, icon and command\""
- [x] App permissions: the app picked (from Flathub), what it may reach with switches and Changed, Reset to what the app asks for; `app-permissions:<app id>` — hosts/hearth-ui/apps/settings/src/cats/apps/perms.rs: "\"Reset to what the app asks for\""
- [x] Background services: yours with switches (running or not), Hearth's locked ones with why; Check again in place of Mantel's 15 s re-reading — hosts/hearth-ui/apps/settings/src/cats/apps/services.rs: "services Hearth needs\""
- [x] Window rules: each rule (name, Use this rule, Pick the app's window, app and title matching, the 12 settings each with Yes/No and when, position, size, desktop, screen, opacity in front and behind, Delete), New rule; `kwin-rules:<window class>` with Make a rule for it — hosts/hearth-ui/apps/settings/src/cats/apps/rules.rs: "\"Allow tearing (less lag in games)\""
- [x] Web browser: Hearth Browser or Google Chrome (Must Have For Xodus Nexus), Chrome's update and remove, Open Hearth Browser, make it the default, Bigger buttons and text, Aloha Browser (install, default, update, remove) — hosts/hearth-ui/apps/settings/src/cats/apps/browser.rs: "\"Must Have For Xodus Nexus\""
- [x] Claude: Claude Desktop (state, unofficial on this edition, sandbox, Get Claude, Open, Check / Update, Remove) and Claude Code (Get, Open in the Terminal, Remove), with progress — hosts/hearth-ui/apps/settings/src/cats/apps/claude.rs: "\"Unofficial on this edition\""

## Gaming (Mantel's Gaming category, for the Fedora edition)
- [x] Gaming: the Game Mode card with Switch to Game Mode — hosts/hearth-ui/apps/settings/src/cats/gaming/landing.rs: "\"Switch to Game Mode\""
- [x] Gaming: which Game Mode (the two picture cards, In use, can't start here), Switch now, Play Steam games in Steam Game Mode, Hearth Game Mode didn't start / is paused — hosts/hearth-ui/apps/settings/src/cats/gaming/landing.rs: "\"Play Steam games in Steam Game Mode\""
- [x] Gaming: Start in (Game Mode, Desktop Mode, Server mode when it's on), Decky with its state, Startup animation with the one in use — hosts/hearth-ui/apps/settings/src/cats/gaming/landing.rs: "\"What this PC opens when it turns on or you sign in\""
- [x] Gaming: Your games, Controls & performance, Light mode while you play, and the pages Mantel reaches elsewhere (Performance, Saved games, Full-screen games, Discord) — hosts/hearth-ui/apps/settings/src/cats/gaming/landing.rs: "\"Light mode while you play\""
- [x] Saved games: Back up all saves, Restore all saves, each game (backed up or not, only in the backup) backing up or restoring just it — hosts/hearth-ui/apps/settings/src/cats/gaming/saves.rs: "\"Restore all saves\""
- [x] Performance (gameperf): Resolution, Refresh rate, Frame limit, HDR, Used the next time Game Mode starts — hosts/hearth-ui/apps/settings/src/cats/gaming/perf.rs: "\"Used the next time Game Mode starts\""
- [x] Steam Game Mode and Decky: Switch to Steam Game Mode, why it didn't start, beside the desktop (test), Decky Loader and Hearth Sync with Install (after a yes) / Remove — hosts/hearth-ui/apps/settings/src/cats/gaming/steam.rs: "\"Keep Steam Game Mode running beside the desktop (test)\""
- [x] Startup animation: Steam's own and each of Hearth's as a card (In use, its matching splash, Preview ›), the preview with Use this, Use with its boot splash, Play the video; `startup-animation:preview-<id>` — hosts/hearth-ui/apps/settings/src/cats/gaming/boot.rs: "\"Use with its boot splash\""
- [ ] Startup animation: the animations' pictures (the preview sheets' frames; a display mark stands in until the settings app can show pictures)
- [x] Games & stores: Hearth Games, Epic, GOG, Battle.net, Xbox sign-in, Library sources (emulators), Downloads, Store, Recomps, Import, from another Hearth, Mods, Saves, Controller Lab, Discord, Theater, Benchmark; never Steam's sign-in — hosts/hearth-ui/apps/settings/src/cats/gaming/games.rs: "Hearth never signs in to Steam for you.\""
- [x] Discord: which Discord (the three choices, Install), Open Discord, the application ID (Save, Remove), Show what I'm playing, Let friends ask to join, the invite link — hosts/hearth-ui/apps/settings/src/cats/gaming/discord.rs: "\"Let friends ask to join\""
- [x] Remote play: Use this PC from anywhere (switch, Start again, Tailscale, the address, Moonlight's PIN, paired devices, who's connected, Check the connection, Reachable, Stay awake, Wake from away), Remote play (switch, Sunshine's login, Pair a device, paired devices), Moonlight — hosts/hearth-ui/apps/settings/src/cats/gaming/remote.rs: "\"Check the connection\""
- [x] Controller: Controller mouse (KDE Plasma's hearth-pad, stopped or started for this sign-in), pointer speed, button mapping (type games see, each button to a button, a key or nothing, Nintendo layout, Reset), Handheld Daemon, the buttons' help, on-screen keyboard — hosts/hearth-ui/apps/settings/src/cats/gaming/controller.rs: "\"Nintendo layout\""
- [ ] Controller: the pointer speed reaching KDE Plasma's controller mouse (hearth-pad --standalone doesn't read pointerSpeed yet; it's saved in desktop.json for Mantel)
- [x] Full-screen games & X11 apps: tearing, Tearing for one game, screen edges, keys X11 apps can see, mouse buttons too, scaling X11 apps — hosts/hearth-ui/apps/settings/src/cats/gaming/kwingames.rs: "\"Keys X11 apps can see while you're in another app\""
- [x] Handheld: Handheld Daemon's state, Check again, Show advanced settings, every section with Steam's note and every kind of setting (switch, choices, a long list picked one by one, slider, action pressed twice, group, colour, text) — hosts/hearth-ui/apps/settings/src/cats/gaming/handheld.rs: "\"Press again to confirm\""

Notes on Apps and Gaming (feature/settings-apps-gaming): every page there uses Mantel's own helpers, so Debian and
the Fedora edition share their settings; CI pictures them as CI is (settings-apps-*, settings-gaming-*) and, with
answers from a real device (HEARTH_SETTINGS_FIXTURES), as on an ROG Ally (…-ally, …-device). Two pages read again by
themselves while they're live, as Mantel's do: Remote play every 5 s while "Use this PC from anywhere" is on, Handheld
every 6 s while Handheld Daemon runs (about 0.9 % of a core while open; nothing once you leave the page).
Background services no longer re-reads every 15 s (Check again instead).

## System: every page of Mantel's System category (ported from its Set*.qml, same helpers; KDE Plasma's own settings too)
Pictured in CI: settings-sys-<page> (.github/scripts/hearth-apps-rust-shots.d/settings-system.sh, from examples).
Not tried on a device yet: everything here is CI-proven only (from example answers). For the owner to decide: push to
talk does nothing on KDE Plasma (it's Mantel's), so its switch is off there; Boot splash uses the same hearth-admin
boot-splash as Debian, untried on the Fedora edition; the screen-off / sleep / lid / power button choices also write
PowerDevil's powerdevilrc on Plasma (PowerDevil is what acts there).
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

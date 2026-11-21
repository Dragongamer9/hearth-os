# Hearth Settings as its own pure-Rust app (the approved new Settings' layouts A and B, for the Fedora edition)

Mockups: next-update/settings-A1-home-mouse.png, next-update/settings-A2-display-mouse.png, next-update/settings-B1-quick-controller.png, next-update/settings-B2-display-controller.png
Release: 1.16

The owner (Nov 18, 2026): Settings is its own app, not tied to the Game Mode app, in pure Rust like the new Task
Manager (hosts/hearth-ui/apps/settings). It follows the Global Theme picked in KDE, as every Hearth app must.

## The frame
- [x] left list: you (picture, name, Administrator · PC name) — hosts/hearth-ui/apps/settings/src/frame.rs: "\"Administrator\" } else { \"Standard account\" }"
- [x] Search settings, finding pages by name and words — hosts/hearth-ui/apps/settings/src/frame.rs: "text_input(\"Search settings\", &a.query).id(nav::SEARCH)"
- [x] the 13 categories with the picked one marked — hosts/hearth-ui/apps/settings/src/registry.rs: "assert_eq!(CATEGORIES.len(), 13);"
- [x] the page's path as its title (System › Display) — hosts/hearth-ui/apps/settings/src/frame.rs: "fn crumbs(a: &App)"
- [x] button hints: A Select, B Back, Y Search, LB/RB categories, Close — hosts/hearth-ui/apps/settings/src/frame.rs: "\"Previous / next category\""
- [x] follows the Global Theme picked in KDE's System Settings, Hearth's looks or any other — hosts/hearth-ui/src/theme.rs: "fn from_plasma() -> Option<Pal>"

## Switching between the looks (settings-new.md: A with the keyboard and mouse, B with a controller)
- [x] B while a controller was used last, A while the keyboard or mouse was, switching live (needs testing on a device: CI has no controller, both looks are pictured with --layout) — hosts/hearth-ui/apps/settings/src/main.rs: "if !a.sh.big && a.fixed.is_none() && look::auto() {"
- [x] the page and the setting you were on stay picked across the switch — hosts/hearth-ui/apps/settings/src/main.rs: "The page and the focused row stay."
- [x] the controller read from /dev/input as well as its keys (hearth-pad, Steam Input), each press done once, its click held back (needs testing on a device) — hosts/hearth-ui/apps/settings/src/nav.rs: "if click && now_ms() < HOLD_CLICKS.load(Ordering::Relaxed) {"

## Moving with the controller or the arrow keys (both looks; pictured: settings-a3-display-moved, settings-b7-quick-moved)
- [x] the focus moves between every row and control (D-pad / left stick / arrows), the page scrolling to keep it in view — hosts/hearth-ui/apps/settings/src/nav.rs: "pub fn reveal<M: Send + 'static>() -> iced::Task<M> {"
- [x] A (Enter) selects: a switch turns on or off, a link opens, a button presses, a text field takes the typing — hosts/hearth-ui/apps/settings/src/main.rs: "return iced::widget::operation::focus(id);"
- [x] left and right change sliders and choices; left with nothing to change goes to the list on the left — hosts/hearth-ui/apps/settings/src/main.rs: "let (acts, _) = nav::take(if act == Left { nav::Which::Left } else { nav::Which::Right });"
- [x] B (Esc) back: a dialog or search closes, a subpage to its page (its row focused again), the page to the list — hosts/hearth-ui/apps/settings/src/main.rs: "// The row that opened this page gets the focus back."
- [x] Y (Ctrl+F) search, LB / RB (Page Up / Down) previous / next category or tab, ☰ (Ctrl+W) close — hosts/hearth-ui/apps/settings/src/main.rs: "P::Menu => Action::Close,"
- [x] the focused row lifted with the accent outline — hosts/hearth-ui/apps/settings/src/widgets.rs: "pub fn ring<'a, M: 'a>(p: &Pal, lit: bool, r: f32, el: impl Into<E<'a, M>>) -> E<'a, M> {"

## B: Quick (settings-B1; pictured: settings-b1-quick, settings-b5-pin)
- [x] header "Settings" with the LB/RB tabs: Quick, System, Network, Devices, Gaming, Look, Accounts, Server — hosts/hearth-ui/apps/settings/src/frame.rs: "for (i, (_, label, _)) in TABS.iter().enumerate() {"
- [x] big tiles: Light mode while you play (wide), Wi-Fi, Bluetooth, Brightness and Volume (with sliders), Start in (wide, Game Mode / Desktop Mode), Updates, Power, Night light, Do not disturb — hosts/hearth-ui/apps/settings/src/quick.rs: "pub const TILES: &[(&str, &str, u16)] = &["
- [x] the focused tile lifted with the accent outline — hosts/hearth-ui/apps/settings/src/quick.rs: "let lift = if r.lit {"
- [x] X Pin to Quick (pick which settings are on Quick, kept in settings-quick.json as Mantel's) — hosts/hearth-ui/apps/settings/src/quick.rs: "hearth_ui::paths::home().join(\".local/share/nimbus/settings-quick.json\")"
- [x] button hints: A Select, B Back, Y Search, X Pin to Quick, LB/RB Previous / next category, Close — hosts/hearth-ui/apps/settings/src/frame.rs: "r = r.push(hint(&p, \"X\", x, false, Some(Msg::X)));"

## B: a page (settings-B2; pictured: settings-b2-display, settings-b3-sound, settings-b4-list, settings-b6-search, settings-b8-history, settings-b9-default)
- [x] the category's pages listed on the left (the tab's pages) — hosts/hearth-ui/apps/settings/src/frame.rs: "pub fn tab_list(tab: usize) -> Vec<&'static registry::Page> {"
- [x] big rows: sliders with their value, ‹ value › choices changed with left and right, switches, › for more — hosts/hearth-ui/apps/settings/src/widgets.rs: "pub fn stepper<'a, M: Clone + 'a>("
- [x] the help panel for the focused setting: what it does, how to change it with the controller, and when it was last changed and by whom — hosts/hearth-ui/apps/settings/src/frame.rs: "format!(\"Changed by {who} · {when}\")"
- [x] X Default (puts that one setting back, asking first) — hosts/hearth-ui/apps/settings/src/main.rs: "Only this one setting changes, and Undo on Home puts it back."
- [x] Y search in the controller look (a search box over the page) — hosts/hearth-ui/apps/settings/src/frame.rs: "return ui::dialog(base, d.into(), Some(Msg::Find(false)));"

How it's built (layout B and the controller, Nov 18): every row and control drawn with widgets.rs is a place for the
focus (nav.rs); the controller is read from /dev/input by one sleeping thread (pad.rs, no polling; 0.00 % of a core at
rest in CI) as well as arriving as hearth-pad's / Steam's keys, and each press is done once (a key or click within
350 ms of the same press read from the controller is dropped). Pages' buttons are pressable with the controller when
drawn with widgets.rs `btn` (same arguments as hearth_ui's). The rows each page shows (headings, the display card,
HDR, the second display) are the page modules'. Tests: `--layout a|b`, `--focus TITLE`, `--find WORDS`, `--press "right
down a"` (presses as if from the controller), pictured by .github/scripts/hearth-apps-rust-shots.d/settings-layout-b.sh.

## Home (settings-A1)
- [x] the PC card: picture, name, Hearth OS version · mode · graphics card · processor, Rename and About this PC — hosts/hearth-ui/apps/settings/src/cats/home.rs: "pill(&p, \"About this PC\""
- [x] four quick tiles: Wi-Fi, Bluetooth, Updates, Hearth Link with their state — hosts/hearth-ui/apps/settings/src/cats/home.rs: "tile(&p, G::Link, \"Hearth Link\""
- [x] Recommended settings: Night light (switch), Start in (Game Mode / Desktop Mode; with Game Mode off on this PC, Desktop Mode and "Game Mode: off", Nov 19) — hosts/hearth-ui/apps/settings/src/cats/home.rs: "\"What this PC opens when it turns on\", seg(&p, start_in_items(sh, st))"
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
- [x] Your desktop: KDE Plasma, Firehold (Plasma, test), Switch now (Firehold (Rust, test) archived Nov 19) — hosts/hearth-ui/apps/settings/src/cats/system_more.rs: "\"Firehold (Plasma, test)\""
- [x] Personalization: the 8 looks, dark / light, the accent (KDE's Global Theme follows) — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "Msg::Look(l) => return save(sh, json!({\"look\": l, \"subTheme\": \"\"}))"
- [x] Updates & recovery: the status card (look for updates, download, restart), progress line by line while updating, Update anyway, Shut down when it's done (and its countdown with Cancel), Shut down and finish updating — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "Updated. Shutting down in"
- [x] Updates & recovery: a restore point before updating and before the writable system is switched, Go back (asks first) and Stay on, System Restore, Reliability history, the previous version kept — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "Yes, go back at the next restart"
- [x] Updates & recovery: Decky plugins (each plugin, New from Hearth, Also install new Hearth plugins, Check now, Update, Update plugins automatically, Use the built-in plugins, the last error) — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "Also install new Hearth plugins"
- [x] Updates & recovery: the writable system where hearth-unlock is (Debian's; none on the Fedora edition, so it doesn't show) — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "Turn my changes back on"
- [x] Backup and help: Logs, Send logs to the developer (with its code), Back up Hearth to a drive, Restore from a backup — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "Send logs to the developer"
- [x] Hearth Link: this device and Rename, Share this device's AI, the kind of device you pair (My device, A family member's device, A friend's device), Pair a device with its codes, nearby devices, a code from somewhere else, the long link — hosts/hearth-ui/apps/settings/src/cats/link.rs: "A family member's device"
- [x] Hearth Link: your devices with each one's menu (what it is, unpair), copies (pause, resume), sync, accept games, share status, relay, Hearth Storage (role, servers & devices with remove, shared folders, backups & saves, advanced) — hosts/hearth-ui/apps/settings/src/cats/link.rs: "Hearth Storage · This device"
- [x] Hearth Link: a device's choices as hearth-link has them (make it a family member's / a friend's / one of my devices; a family device's switches: its status, and on the family member's side library, saves, Hearth Storage and backups; what they share with you; remote desktop with Moonlight's PIN; wake it) — hosts/hearth-ui/apps/settings/src/cats/link.rs: "vec![\"share\".into(), id.into(), key.into()"
- [x] Your devices: every device's card, this PC as the home server, share status and Show other devices in Files; one device with the last hour's graphs, its server, now, and what to do with it — hosts/hearth-ui/apps/settings/src/cats/link.rs: "Do something with it"
- [x] Home server: the landing (what needs a look, four numbers, its pages in four groups, Light mode, hidden items), Set up your home server (its five steps, the sign-in QR code) — hosts/hearth-ui/apps/settings/src/cats/hs/overview.rs: "1 thing needs a look"
- [x] Home server: Server mode (power when idle, what pauses when you play, the web page and its PIN pad and permissions), AI engine (Colibri: install, engine, models in every group, models folder, download, the plan) — hosts/hearth-ui/apps/settings/src/cats/hs/overview.rs: "Graphics card limit when idle"
- [x] Home server: Shared folders (each share's settings, whole drives, Home and the system drive asked twice, the sharing password, how to open them), Files & links, People, Family & home — hosts/hearth-ui/apps/settings/src/cats/hs/sharing.rs: "Share the system drive (read only)"
- [x] Home server: Services (media server, Files on the web, AI pictures with video, Voice Studio with its PIN and key, AI voices, AI chat), AI chat, AI devices, Server apps (each app, world backups), Phone notifications, Gaming — hosts/hearth-ui/apps/settings/src/cats/hs/services.rs: "Back up the worlds every night"
- [x] Home server: Drives & undo (health, snapshots, browse and restore, adding a drive asked twice), Backups & sync (each backup, restore, adding one, devices backing up here, phone photos, folder sync), Safety, Alerts & power (with the power history graph and UPS) — hosts/hearth-ui/apps/settings/src/cats/hs/safe.rs: "Erase {} now"
- [x] every other page of Mantel's Settings (none is left "Coming next": every page of settings-pages.js and kde-settings.json is built; pictured: settings-fin-15-apps) — hosts/hearth-ui/apps/settings/src/registry.rs: "fn every_page_of_mantels_settings_is_built()"

## Apps (Mantel's Apps category, for the Fedora edition)
- [x] Installed apps: Hearth's Debian box (state, its packages with Remove, Install a .deb, Reset, Remove the box), Bazaar (open, its install, Fix your Flatpak folder), your Flathub apps with Remove, AppImages — hosts/hearth-ui/apps/settings/src/cats/apps/installed.rs: "\"Reset the Debian box…\""
- [x] App store: search, Discover (categories, Popular, Trending, New and updated), a category or a search — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "\"New and updated\""
- [x] App store: an app's page (name, summary, developer, rating, size, Install / Open / Remove, description, reviews, license, app id, website) — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "\"No ratings yet\""
- [x] App store: Updates (Tell me when app updates are there, Update all, each update, new permissions shown and Update and allow, Hearth OS updates, Firmware) — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "\"Update and allow\""
- [x] App store: Sources (each remote, remove yours, add a .flatpakrepo) — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "\"Add a source…\""
- [x] App store: Drivers, the Fedora way (the PC's parts and the driver in use from lspci, the firmware and graphics packages from rpm, firmware the kernel couldn't load; drivers come with Hearth OS's updates) — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "pub const FEDORA_DRIVERS"
- [x] App store: B goes back from an app to its list, and `store:updates` / `store:sources` / `store:drivers` / `store:app-<id>` open there — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "\"drivers\" => update(sh, st, M::Drivers)"
- [x] App store: app icons and screenshots as pictures (hearth-store pics saves Flathub's pictures once in its cache; the icon in each list and on the app's page, the screenshots side by side with ‹ › and Left / Right; pictured: settings-fin-01-store, settings-fin-02-store-app) — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "Screenshots (SetStore.qml's shotsCard)"
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
- [x] Startup animation: the animations' pictures (each card's poster frame from the preview sheet, playing while the mouse or the focus is on it; the preview playing; pictured: settings-fin-04-animation, settings-fin-05-animation-focus, settings-fin-06-animation-preview) — hosts/hearth-ui/apps/settings/src/bootpics.rs: "pub fn load_frames(s: &Sheet, g: Geo)"
- [x] Games & stores: Hearth Games, Epic, GOG, Battle.net, Xbox sign-in, Library sources (emulators), Downloads, Store, Recomps, Import, from another Hearth, Mods, Saves, Controller Lab, Discord, Theater, Benchmark; never Steam's sign-in — hosts/hearth-ui/apps/settings/src/cats/gaming/games.rs: "Hearth never signs in to Steam for you.\""
- [x] Discord: which Discord (the three choices, Install), Open Discord, the application ID (Save, Remove), Show what I'm playing, Let friends ask to join, the invite link — hosts/hearth-ui/apps/settings/src/cats/gaming/discord.rs: "\"Let friends ask to join\""
- [x] Remote play: Use this PC from anywhere (switch, Start again, Tailscale, the address, Moonlight's PIN, paired devices, who's connected, Check the connection, Reachable, Stay awake, Wake from away), Remote play (switch, Sunshine's login, Pair a device, paired devices), Moonlight — hosts/hearth-ui/apps/settings/src/cats/gaming/remote.rs: "\"Check the connection\""
- [x] Controller: Controller mouse (KDE Plasma's hearth-pad, stopped or started for this sign-in), pointer speed, button mapping (type games see, each button to a button, a key or nothing, Nintendo layout, Reset), Handheld Daemon, the buttons' help, on-screen keyboard — hosts/hearth-ui/apps/settings/src/cats/gaming/controller.rs: "\"Nintendo layout\""
- [x] Controller: the pointer speed reaching KDE Plasma's controller mouse (hearth-pad --plasma and --standalone read desktop.json's pointerSpeed at the start and within 2 s of a change; test_pad_speed_plasma.py; pictured: settings-gaming-10-controller) — hosts/linux/desktop/shell/hearth-pad: "speed_watch = SpeedWatch() if (plasma or standalone) and not cosmic else None"
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
- [x] Boot splash: each splash's picture and its played preview (SetBootCard.qml's cards, SetBootPreview.qml's Use this / Use with its startup animation / Close; pictured: settings-fin-07-boot, settings-fin-08-boot-focus, settings-fin-09-boot-preview) — hosts/hearth-ui/apps/settings/src/cats/system_more.rs: "pub fn boot_preview(a: &App)"
- [x] Display: Hearth's own HDR calibration pictures (HdrCalibrate.qml's three test pictures, built by feature/settings-kde-all as Calibrate HDR; pictured: kde-80-hdr-step1, kde-81-hdr-step3) — hosts/hearth-ui/apps/settings/src/cats/kde.rs: "for k in [1u8, 2, 3, 4, 6, 8]"
- [x] Display: Identify outside KDE Plasma (each screen's number in its top-left corner for 3 s, as SetDisplay.qml: `hearth-settings-app --identify`, layer-shell overlays; on Plasma KDE's own; pictured: settings-fin-12-identify) — hosts/hearth-ui/apps/settings/src/identify.rs: "OutputOption::OutputName(name)"
- [x] Startup: editing the schedule's changes on this page (each day's 24-hour track, each change with ← → move, A mode, X remove and its mouse buttons, Add a change; saved a moment later through hearth-server-mode schedule-set; pictured: settings-fin-10-startup-schedule, settings-fin-11-startup-schedule-b) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "pub fn sched_edit(st: &mut State, day: &'static str, i: usize, op: SchedOp)"

## Network & internet (every page, from Mantel's Set*.qml with the same helpers)
- [x] Wi-Fi: the network you're on (name, signal, band, address), Properties, Disconnect, Forget (asked once more) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "Hide properties"
- [x] Wi-Fi: the switch, networks nearby with signal and lock, connect (password on nmcli's stdin), Show all networks — hosts/hearth-ui/apps/settings/src/cats/net.rs: "Show all networks ("
- [x] Wi-Fi: Add a hidden network, Look again, the other network pages listed under it — hosts/hearth-ui/apps/settings/src/cats/net.rs: "Add a hidden network"
- [x] Tailscale: sign in (QR code and link), this PC's name, sign out, your devices with ping and Taildrop — hosts/hearth-ui/apps/settings/src/cats/net_ts.rs: "Waiting for you to sign in…"
- [x] Tailscale: Sign in works from any state: starts and enables tailscaled when it is off, makes you its operator, shows the link and a big QR code, and says which step failed — hosts/hearth-ui/apps/settings/src/cats/net_ts.rs: "ts-signin"
- [x] Tailscale: only through Tailscale, start at boot, exit node, receive files, wake from away — hosts/hearth-ui/apps/settings/src/cats/net_ts.rs: "Wake this PC from away"
- [x] VPN: your VPNs, connect (account asked when needed), disconnect, details, remove (asked once more) — hosts/hearth-ui/apps/settings/src/cats/net_vpn.rs: "Connect with this account"
- [x] VPN: add from a file (picker or its place), set up IPsec, L2TP, PPTP, OpenConnect, SSTP, Fortinet by hand — hosts/hearth-ui/apps/settings/src/cats/net_vpn.rs: "Set up a VPN by hand"
- [x] Mobile hotspot: share my internet, show the password, name, password and band — hosts/hearth-ui/apps/settings/src/cats/net_more.rs: "Share my internet"
- [x] Proxy: no, automatic (setup script) or by hand (web, secure web, SOCKS, skip list), Save — hosts/hearth-ui/apps/settings/src/cats/net_more.rs: "Skip the proxy for: "
- [x] Connection details: saved networks, what it has now, auto-connect, metered, MAC, DNS, IPv4, IPv6, MTU, duplicate — hosts/hearth-ui/apps/settings/src/cats/net_details.rs: "Duplicate this connection"
- [x] Connection details: add a wired, mobile broadband or enterprise Wi-Fi network; 802.1X sign-in — hosts/hearth-ui/apps/settings/src/cats/net_details.rs: "Enterprise sign-in (802.1X) for {}"
- [x] Connection details: share a Wi-Fi network with a QR code, mobile broadband APN, PIN and roaming — hosts/hearth-ui/apps/settings/src/cats/net_details.rs: "Show a QR code"

## Bluetooth & devices (every page, from Mantel's Set*.qml with the same helpers)
- [x] Bluetooth: on/off, your devices (connect, disconnect), look for and pair new ones, forget — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "Trust, block, batteries and sending files"
- [x] Your Bluetooth devices: battery, Trust, Block, Send a file — hosts/hearth-ui/apps/settings/src/cats/bt_devs.rs: "Never connects, even when it tries"
- [x] Receive files & this PC's name: rename, put back the usual name, receive files, where, trusted, received lately — hosts/hearth-ui/apps/settings/src/cats/bt_devs.rs: "Take files from trusted devices without asking"
- [x] Keyboard: layouts (add, order, remove), switch shortcut, remember the layout, repeat delay and speed, Num Lock, compose key (KWin's kxkbrc / kcminputrc on Plasma) — hosts/hearth-ui/apps/settings/src/cats/bt_kbd.rs: "Remember the layout"
- [x] Keyboard: the on-screen keyboard (layouts, suggestions, fast typing, its 9 looks, size, shape, position, trail, rumble, clear words) — hosts/hearth-ui/apps/settings/src/cats/bt_kbd.rs: "Clear learned words"
- [x] Keyboard: shortcuts, and bring back KDE's own shortcuts — hosts/hearth-ui/apps/settings/src/cats/bt_kbd.rs: "Bring back KDE's own shortcuts"
- [x] Mouse & touchpad: speed, acceleration, natural scrolling, left-handed, scroll speed, middle-click, middle-click paste, every touchpad setting, gestures — hosts/hearth-ui/apps/settings/src/cats/bt_mouse.rs: "Ignore the touchpad while typing"
- [x] Mouse buttons: each button's job (KWin's rebinds), press a button to add it, a key you press — hosts/hearth-ui/apps/settings/src/cats/bt_mouse.rs: "Point here and press the mouse button you want to change"
- [x] Phone: phone link, pairing requests, calls, your phones, Ring, Ping, clipboard, files, photo, send text, shared, music — hosts/hearth-ui/apps/settings/src/cats/bt_phone.rs: "wants to pair"
- [x] Phone: notifications with Reply, this PC's keyboard and mouse on the phone, its notifications on the phone, what it can do, run commands, pair a new phone — hosts/hearth-ui/apps/settings/src/cats/bt_phone.rs: "This PC's keyboard and mouse on {name}"
- [x] Messages: conversations, one conversation with Reply, a new message — hosts/hearth-ui/apps/settings/src/cats/bt_phone.rs: "Conversations on {}"
- [x] Drawing tablet & touchscreen: one pointer, screen, part of the screen and tablet, left-handed, pressure, pen buttons; touchscreen screen, turned, calibrate (four crosses), undo — hosts/hearth-ui/apps/settings/src/cats/bt_tablet.rs: "Touch the centre of each cross"
- [x] Printers: my printers (test page, default, remove), queue and scanners, add one (look, by address) — hosts/hearth-ui/apps/settings/src/cats/bt_print.rs: "Add by address"
- [x] Print queue: each printer, its jobs with Hold / Go on / Cancel, Cancel all mine, printed lately — hosts/hearth-ui/apps/settings/src/cats/bt_print.rs: "Cancel all mine"
- [x] Scanners: the scanners found and a test scan with its picture — hosts/hearth-ui/apps/settings/src/cats/bt_print.rs: "It scanned. This is the test picture."
- [x] Scan to PDF: scanner, resolution, colour, source, scan pages, order, remove, save, open, start again — hosts/hearth-ui/apps/settings/src/cats/bt_print.rs: "Scan every page in the feeder"
- [x] Ink & toner: the notice switch, each cartridge's level in its colour, low ones marked — hosts/hearth-ui/apps/settings/src/cats/bt_print.rs: "Tell me when ink or toner runs low"
- [x] USB devices & drives: when a drive is plugged in, new USB devices, USBGuard, Allow / Block — hosts/hearth-ui/apps/settings/src/cats/bt_usb.rs: "Allow always"
- [x] Thunderbolt: security, devices, allow once or always, forget — hosts/hearth-ui/apps/settings/src/cats/bt_usb.rs: "Always allow it"

The pictures of these pages: the hearth-apps-rust workflow's settings-nb-* (the pages on a pretend PC, .github/scripts/hearth-apps-rust-shots.d/settings-net-bt.sh, and what they say with nothing to ask). Only the top of a long page is in its picture.

## Personalization, Accounts, Privacy & security, Accessibility (branch feature/settings-pers-acc)
Every page of these four categories, ported from Mantel's Set*.qml with the same helpers, for KDE Plasma on the Fedora
edition. Pictures: settings-pa-* (CI, .github/scripts/hearth-apps-rust-shots.d/settings-pers-acc.sh).

- [x] Personalization: KDE's Global Theme shown (Hearth's look or one picked in KDE), Use Hearth's look, Global Themes in KDE — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "KDE's Global Theme: {shown}"
- [x] Personalization: sub themes (palettes.json "subThemes" and the black set), Match sub theme — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "pub fn subs(look: &str, mode: &str) -> Vec<Sub>"
- [x] Personalization: Dark / Light / Auto (Sunset) with its two times — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "(\"sunsetLight\", \"Light from\", \"07:00\")"
- [x] Personalization: the wallpaper (the look's own, Hearth's, Pictures › Wallpapers, Choose a picture…), put on in Plasma — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "plasma-apply-wallpaperimage"
- [x] Personalization: accent presets, the look's own, From wallpaper (KDE's on Plasma), a hue, a typed colour — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "Or type a colour, like #1a9fff, then Enter"
- [x] Personalization: keyboard lights follow the accent, desktop clock and weather (KDE's widgets), panel transparency, on-screen keyboard style — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "Keyboard lights follow the accent"
- [x] Bar and dock on Plasma (KDE's panel): placement, auto-hide, bar style, width, alignment, thickness, network speed, Arrange the bar, add / remove a panel, the app launcher, right-click menu — hosts/hearth-ui/apps/settings/src/cats/pers/desk.rs: "Network speed in the bar"
- [x] Bar and dock off Plasma (Mantel's keys): layout, edge, floating dock, auto-hide, bar style, app menu — hosts/hearth-ui/apps/settings/src/cats/pers/desk.rs: "Top bar + floating dock"
- [x] Fonts, cursor & icons: fonts and sizes, small and title fonts with their preview, smoothing, sub-pixel order, hinting, DPI, cursor, cursor size, icon theme (and KDE's own on Plasma) — hosts/hearth-ui/apps/settings/src/cats/pers/fonts.rs: "Font smoothing and DPI"
- [x] Wallpaper per screen, slideshow, picture of the day (Bing, NASA, Wikimedia), light and dark, on Plasma's desktops — hosts/hearth-ui/apps/settings/src/cats/pers/desk.rs: "org.kde.potd"
- [x] My looks: save how it looks now, use, delete — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "Save as my look"
- [x] Window frames & sign-in splash: frame accent preview and slider, apps follow Hearth, the four splashes, Show it now (KDE's splash on Plasma) — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "ksplashqml"
- [x] Window effects: every switch, choice and slider of SetKwinEffects.qml (hearth-kwin-settings, live on Plasma) — hosts/hearth-ui/apps/settings/src/cats/pers/kwin.rs: "Show paint"
- [x] Virtual desktops, Tiling (the layout drawn, presets, columns, rows, gap, quick tiling, snapping), Screen edges (the screen drawn, each corner and edge, touch edges), Window behaviour — hosts/hearth-ui/apps/settings/src/cats/pers/kwin.rs: "Where new windows open"
- [x] Accounts: the category page, your account, picture, password, other people (type, remove keeping or deleting files), add an account, finish setting up — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "Remove, but keep their files"
- [x] Accounts: your picture cut to fit and shown in its circle (as Mantel shows it), in the list of accounts and the pictures to pick from — hosts/hearth-ui/apps/settings/src/pics.rs: "Everything outside the circle see-through, its edge softened over a pixel."
- [x] Sign-in options: sign in automatically, Hearth's login screen (test), lock after sleep (KDE's lock on resume too), password, fingerprint (add, forget, where it's used), Moved — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "Unlock the lock screen with a finger"
- [x] Online accounts: your accounts with what each is used for, Open in Files, remove; Nextcloud, Google, Microsoft (browser sign-in, Cancel), CalDAV/CardDAV, WebDAV, IMAP — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "Sign in in the browser"
- [x] Login screen: the picture (Hearth's, your own, Choose a picture…, Hearth's fireside), Sign in automatically — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "Use Hearth's fireside"
- [x] Privacy & security: the category page, Player count, Firewall (on/off, incoming, Tailscale, rules, let a port in) — hosts/hearth-ui/apps/settings/src/cats/privacy.rs: "Let a port in"
- [x] Vaults, Recent files (with activities and left out), Firmware & device security, its history, Passwords — hosts/hearth-ui/apps/settings/src/cats/privacy.rs: "Hearth never shows or copies a saved password here."
- [x] Accessibility: magnifier, colour filters, invert one window, screen reader, sticky/bounce/slow/toggle/mouse keys, the bell, dwell click, text size, high contrast, large pointer, reduce motion, emoji — hosts/hearth-ui/apps/settings/src/cats/a11y.rs: "Click by resting the pointer"
- [x] Pictures of wallpapers, avatars and the login screen's background drawn as the pictures themselves (read once off the window's thread and shrunk; colour until then) — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "let face: E = match crate::pics::get(&plasma_wall(sh, f), crate::pics::WALL)"

What these four categories still need outside their own files (feature/settings-pers-acc):
* Pictures drawn as pictures (wallpapers, avatars, the login screen's background) and your picture in its circle: done
  in the 1.15 merge (feature/1.15-rust-apps, pics.rs) now that the settings app has iced's image feature.
* On KDE Plasma nothing yet changes light and dark at the Sunset times, and hearth-plasma-looks doesn't use
  frameTint, appsFollow or menuStyle: those three are saved (Mantel shares them) and their rows say what they reach.
  A light and a dark wallpaper are put on when the mode is changed here.
* Not carried over on purpose: Vaults' 10-second re-read (no timers while a page is open; it's read after each
  change), Slideshow's "Next picture now" (Plasma's scripting has no call for it: the page points to KDE's own).

## The last parts (feature/settings-rust-finish, Nov 20)
Every open part above is built and pictured in CI (Hearth apps (pure Rust), .github/scripts/hearth-apps-rust-shots.d/settings-finish.sh: settings-fin-*). Not tried on a device yet.
* The App store's pictures come from Flathub's addresses through `hearth-store pics` (saved once in ~/.cache/hearth/store/pics, at most 400 kept); offline, the store mark stands in.
* The boot pictures play only while a preview is open or the mouse or the controller's focus is on a card (at the sheets' 10 frames a second); at rest nothing runs (settings-fin-01-store and settings-fin-07-boot are measured at rest).
* Startup's change rows have small ‹ › × buttons for the mouse as well (Mantel's rows only take the keys); the controller works as Mantel's (← → move, A mode, X remove).
* Identify outside Plasma starts the app again with `--identify` (iced_layershell, as hearth-pad-host); without layer-shell (Game Mode's gamescope) it shows nothing and says so in its log.
* Shared files touched: hearth-store (`pics`), hearth-pad (pointer speed on Plasma), and hearth-apps-rust.yml (a third pictures matrix: the runs had 511 pictures, over the two matrices' 500).

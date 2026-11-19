# Hearth Settings as its own pure-Rust app (the approved new Settings' layouts A and B, for the Fedora edition)

Mockups: next-update/settings-A1-home-mouse.png, next-update/settings-A2-display-mouse.png, next-update/settings-B1-quick-controller.png, next-update/settings-B2-display-controller.png
Release: next

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
- [x] B while a controller was used last, A while the keyboard or mouse was, switching live (needs testing on a device: CI has no controller, both looks are pictured with --layout) — hosts/hearth-ui/apps/settings/src/main.rs: "if !a.sh.big && a.fixed.is_none() {"
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
- [x] Recommended settings: Night light (switch), Start in (Game Mode / Desktop Mode) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"What this PC opens when it turns on\", seg(&p"
- [ ] Recommended settings: Light mode while you play, Do not disturb
- [x] Storage: each drive with its bar and used of total, Manage — hosts/hearth-ui/apps/settings/src/cats/home.rs: "Some((\"Manage\", Msg::Go(\"storage\")))"
- [x] Changed lately: the last changes (what → to, who, when), each with Undo — hosts/hearth-ui/apps/settings/src/cats/home.rs: "btn(&p, \"Undo\""
- [ ] All changes (the full list page)
- [ ] Hearth Link's tile shows its devices

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
- [x] Updates & recovery: check, update (a restore point first), restart, go back a version, System Restore, Reliability history — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "\"Go back a version\""
- [ ] every other page of Mantel's Settings (listed on its category as "Coming next")

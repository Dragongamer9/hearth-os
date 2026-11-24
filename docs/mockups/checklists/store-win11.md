# Hearth Store in the Windows 11 Store style (approved Nov 22, 2026)

The owner, Nov 22: "Give me some mockups of the Hearth Store make it look more modern and better", "Make the Hearth
Store look like the windows 11 app store", "there needs to be a button that you can click to check for updates", then
"i like it". Built in the Hearth Store app (hosts/hearth-ui/apps/store, pure Rust); Describe it comes to it with the
desktop layouts work (docs/mockups/checklists/desktop-layouts.md, part 12). Sources: hearth-workspace
hearth/mockups/store-win11.

Mockups: store-win11/store-w11-1-home.png, store-win11/store-w11-2-apps.png, store-win11/store-w11-3-app-page.png, store-win11/store-w11-4-library-updates.png, store-win11/store-w11-5-test-version.png, store-win11/store-w11-6-settings.png
Release: next

## The frame (every page)
- [x] Title bar: the Store's icon and name, the search box in the middle ("Search Hearth apps, parts and test versions", Ctrl+K), your picture, the window buttons — hosts/hearth-ui/apps/store/src/w11.rs: "Search Hearth apps, parts and test versions"
- [x] Left rail: Home, Apps, Hearth parts, Tests (only with Get test versions on), then Library with the count of updates, and Settings; the page you're on lit with an accent line — hosts/hearth-ui/apps/store/src/w11.rs: "item(\"parts\", G::Store, \"Hearth\\nparts\", Page::BuiltIn, 0)"
- [x] The page on a lighter layer with a rounded top-left corner (Windows 11's look), in the Hearth look's colours — hosts/hearth-ui/apps/store/src/w11.rs: "top_left: z(10.0)"

## 1. Home
- [x] Home and "Checked 2 min ago · 3 updates" with Check for updates — hosts/hearth-ui/apps/store/src/w11.rs: "format!(\"Checked {}{}\""
- [x] The featured app, big (Hearth AI 0.6: its picture, NEW VERSION, its line, Update, See details), the carousel's dots — hosts/hearth-ui/apps/store/src/w11.rs: "\"NEW VERSION\""
- [x] Two side tiles (Command Center 3.15; Mantel in Rust, TEST) — hosts/hearth-ui/apps/store/src/w11.rs: "fn side_picks"
- [x] Your apps as tiles: icon, name, kind, Up to date / Update / the new version / Built in / Installed — hosts/hearth-ui/apps/store/src/w11.rs: "State::BuiltIn => chip(&p, \"Built in\", Chip::Built)"

## 2. Apps
- [x] Filter pills (All, AI, Gaming, Devices, Tools, Fun) and Sort — hosts/hearth-ui/apps/store/src/w11.rs: "\"Sort: Name\""
- [x] A collection banner (Your AI team, on your own PC) — hosts/hearth-ui/apps/store/src/w11.rs: "YOUR AI TEAM, ON YOUR OWN PC"
- [x] Every app as a card with Get / Open / Update / Built in / Coming — hosts/hearth-ui/apps/store/src/w11.rs: "State::Coming => chip(&p, \"Coming\", Chip::Coming)"
- [x] Looking for other apps? Open App store (Flathub's, in Settings) — hosts/hearth-ui/apps/store/src/w11.rs: "Looking for other apps?"

## 3. An app's page
- [x] The app's own colours and picture behind the top of the page — hosts/hearth-ui/apps/store/src/w11.rs: "fn backdrop"
- [x] Its card: big icon, name, Dragongamer9, Open (or Get / Update), Check for updates, ⋯ (Check and repair, Show its files, Release page, Uninstall) — hosts/hearth-ui/apps/store/src/main.rs: "Page::App(id) => vec![Msg::Go(Page::Repair(id.clone())), Msg::ShowFiles(id.clone()), Msg::ReleasePage(id.clone()), Msg::AskUninstall(id.clone())]"
- [x] Version, download size, channel; "Checked with the release's SHA-256 list"; "Updates itself" when it does — hosts/hearth-ui/apps/store/src/w11.rs: "Checked with the release's SHA-256 list"
- [x] Screenshots, What's new, Description, Additional information (made by, released, installs as, needs, updates, source) — hosts/hearth-ui/apps/store/src/w11.rs: "\"Additional information\""

## 4. Library
- [x] A big Check for updates and when it last checked — hosts/hearth-ui/apps/store/src/w11.rs: "format!(\"Last checked: {}\""
- [x] Updates: how many ready and updating, Pause, Update all — hosts/hearth-ui/apps/store/src/w11.rs: " · {busy} updating"
- [x] Each update: from → to, what's new in a line, its size, Update; the one downloading with its progress and ✕ — hosts/hearth-ui/apps/store/src/w11.rs: "btn(&p, \"✕\", B::Normal"
- [x] Your apps: Apps / Hearth parts / Tests, Sort by, each with when it changed, Open and ⋯ — hosts/hearth-ui/apps/store/src/w11.rs: "list.sort_by_key(|e| std::cmp::Reverse(when(e)));"
- [x] "This PC: Hearth OS <version> · Fedora edition" — hosts/hearth-ui/apps/store/src/w11.rs: "format!(\"This PC: {} ›\""

## 5. A test version's page
- [x] The TEST notice ("may change from week to week… your usual one stays one click away, and comes back by itself if this one doesn't start") — hosts/hearth-ui/apps/store/src/w11.rs: "A test version: it may change from week to week."
- [x] Get test version (Remove when it's there), version, size, for which Hearth OS, "Signed and checked before it starts" — hosts/hearth-ui/apps/store/src/w11.rs: "Signed and checked before it starts"
- [x] What's in it, Not in it yet, More test versions — hosts/hearth-ui/apps/store/src/w11.rs: "\"More test versions\""

## 6. Settings
- [x] Check for updates (when it last checked, how many ready) with Check now — hosts/hearth-ui/apps/store/src/w11.rs: "\"Check now\""
- [x] Look for updates every day; Install updates by themselves (off unless you turn it on; never while you play) — hosts/hearth-ui/apps/store/src/w11.rs: "Install updates by themselves"
- [x] Get test versions — hosts/hearth-ui/apps/store/src/w11.rs: "\"Get test versions\", \"Shows test versions"
- [x] Where apps go (Change…), The app list (signed, its number, Trusted), About Hearth Store — hosts/hearth-ui/apps/store/src/w11.rs: "\"About Hearth Store\""

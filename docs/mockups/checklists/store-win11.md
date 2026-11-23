# Hearth Store in the Windows 11 Store style (approved Nov 22, 2026)

The owner, Nov 22: "Give me some mockups of the Hearth Store make it look more modern and better", "Make the Hearth
Store look like the windows 11 app store", "there needs to be a button that you can click to check for updates", then
"i like it". Built in the Hearth Store app (hosts/hearth-ui/apps/store, pure Rust); Describe it comes to it with the
desktop layouts work (docs/mockups/checklists/desktop-layouts.md, part 12). Sources: hearth-workspace
hearth/mockups/store-win11.

Mockups: store-win11/store-w11-1-home.png, store-win11/store-w11-2-apps.png, store-win11/store-w11-3-app-page.png, store-win11/store-w11-4-library-updates.png, store-win11/store-w11-5-test-version.png, store-win11/store-w11-6-settings.png
Release: next

## The frame (every page)
- [ ] Title bar: the Store's icon and name, the search box in the middle ("Search Hearth apps, parts and test versions", Ctrl+K), your picture, the window buttons
- [ ] Left rail: Home, Apps, Hearth parts, Tests (only with Get test versions on), then Library with the count of updates, and Settings; the page you're on lit with an accent line
- [ ] The page on a lighter layer with a rounded top-left corner (Windows 11's look), in the Hearth look's colours

## 1. Home
- [ ] Home and "Checked 2 min ago · 3 updates" with Check for updates
- [ ] The featured app, big (Hearth AI 0.6: its picture, NEW VERSION, its line, Update, See details), the carousel's dots
- [ ] Two side tiles (Command Center 3.15; Mantel in Rust, TEST)
- [ ] Your apps as tiles: icon, name, kind, Up to date / Update / the new version / Built in / Installed

## 2. Apps
- [ ] Filter pills (All, AI, Gaming, Devices, Tools, Fun) and Sort
- [ ] A collection banner (Your AI team, on your own PC)
- [ ] Every app as a card with Get / Open / Update / Built in / Coming
- [ ] Looking for other apps? Open App store (Flathub's, in Settings)

## 3. An app's page
- [ ] The app's own colours and picture behind the top of the page
- [ ] Its card: big icon, name, Dragongamer9, Open (or Get / Update), Check for updates, ⋯ (Check and repair, Show its files, Release page, Uninstall)
- [ ] Version, download size, channel; "Checked with the release's SHA-256 list"; "Updates itself" when it does
- [ ] Screenshots, What's new, Description, Additional information (made by, released, installs as, needs, updates, source)

## 4. Library
- [ ] A big Check for updates and when it last checked
- [ ] Updates: how many ready and updating, Pause, Update all
- [ ] Each update: from → to, what's new in a line, its size, Update; the one downloading with its progress and ✕
- [ ] Your apps: Apps / Hearth parts / Tests, Sort by, each with when it changed, Open and ⋯
- [ ] "This PC: Hearth OS <version> · Fedora edition"

## 5. A test version's page
- [ ] The TEST notice ("may change from week to week… your usual one stays one click away, and comes back by itself if this one doesn't start")
- [ ] Get test version (Remove when it's there), version, size, for which Hearth OS, "Signed and checked before it starts"
- [ ] What's in it, Not in it yet, More test versions

## 6. Settings
- [ ] Check for updates (when it last checked, how many ready) with Check now
- [ ] Look for updates every day; Install updates by themselves (off unless you turn it on; never while you play)
- [ ] Get test versions
- [ ] Where apps go (Change…), The app list (signed, its number, Trusted), About Hearth Store

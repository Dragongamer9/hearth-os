# Proton Manager (ProtonUp-Qt, GPL-3) as Settings › Gaming › Proton and Wine (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/02-proton-manager.html)

Mockups: open-source-apps/02-proton-manager.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps), made of blocks (studio: Edit layout, Customize, its own blocks
window), built on the open-source project the board credits.

Board: Proton Manager (ProtonUp-Qt, GPL-3) as Settings › Gaming › Proton and Wine: installed and available versions, each game's pick, how Hearth picks (nothing switches by itself).

## The window
- [x] Settings' title bar (its gear mark) and side: Dragongamer9 · Nvidia PC · Hearth OS 1.18, Find a setting, the 14 pages with Gaming picked (each opens Settings there) (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "const NAV: [(&str, &str); 14]"
- [x] Made of blocks (Edit layout), with its own buttons, panes and info (CI picture oss-a-02-proton-manager-editing; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "const KIT: Kit = Kit {"
- [x] The credit: Proton Manager in Settings › Gaming · Built on ProtonUp-Qt (GPL-3) (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "\"Built on ProtonUp-Qt (GPL-3)\""
- [x] Gaming ›, Proton and Wine, PROTON MANAGER, ↻ Check for new versions (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "\"↻ Check for new versions\""
- [x] "What runs your Windows games. Hearth keeps the versions you have; nothing is added, removed or switched unless you say so." (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "nothing is added, removed or switched unless you say so."

## Installed
- [x] Installed · 6 · its size · All / Proton / Wine (CI picture oss-a-02-proton-manager; needs testing on a device; the board's 4.1 GB is its six sizes' sum in decimal, the app shows 3.7 GB) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "(\"Wine\".into(), a.filter == 2, Msg::Filter(2))"
- [x] Each version: P / W mark, name, · DEFAULT, where it's from · its size, how many games use it, Default / Make default, Remove (only when no game uses it) (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "let can_remove = n == 0 && !is_def"
- [x] Versions found where ProtonUp-Qt finds them: built into Hearth OS, Hearth's runners, Steam's compatibilitytools.d and its Protons, Lutris' wine runners, the system's Wine (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "fn scan(default: String"
- [x] Make default: Hearth's own, Steam's (config.vdf, only while Steam is closed) and Lutris' default (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/steam.rs: "pub fn set_mapping("

## Get more
- [x] Get more: Proton Hearth / Proton-GE / Wine-GE / Valve Proton / Luxtorpeda (each project's GitHub releases; Valve's through Steam) (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "Family { tab: \"Luxtorpeda\", repo: \"luxtorpeda-dev/luxtorpeda\""
- [x] Each release: date and what's new, NEW, INSTALLED, Get, older (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "format!(\"{} · older\", r.day)"
- [x] Downloading: name · downloading, its bar, "62% · 490 of 790 MB", Pause (and Resume where it stopped) (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "\"{}% · {} of {} MB\""
- [x] Checked with the project's SHA-512 and unpacked into Hearth's runners, shown to Steam and Lutris by a link (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "fn unpack(dl: Dl"

## Each game's pick
- [x] Each game's pick · "only games you changed are listed · 7 of 215" · Find a game (CI picture oss-a-02-proton-manager; needs testing on a device; the board's 214 doesn't add up with its own counts (215)) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "\"only games you changed are listed · {} of {}\""
- [x] GAME / FROM / RUNS WITH ▾ / WHY / Use default, for Steam's games (config.vdf), Game Fixer's (Lutris) and Hearth Games' (proton-worked.json) (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "container(col(&p, \"Runs with\"))"

## How Hearth picks
- [x] For new games ▾ (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "\"For new games\""
- [x] Get new versions by themselves · Tell me when a new version is out · Remove versions no game uses · Show versions in Game Mode's game menu (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "(\"Show versions in Game Mode's game menu\", \"gameMenu\", true)"
- [x] Where they live: ~/.local/share/hearth/runners (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "\"Where they live\""
- [x] "Proton Hearth 9-5 is out" · Get it · "Your games stay on 9-4 until you move them. Try it on one game first from that game's page." (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "Try it on one game first from that game's page."

## The window
- [x] Settings' title bar (its gear mark) and side: Dragongamer9 · Nvidia PC · Hearth OS 1.18, Find a setting, the 14 pages with Gaming picked (each opens Settings there) (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "const NAV: [(&str, &str); 14]"
- [x] Made of blocks (Edit layout), with its own buttons, panes and info (CI picture oss-a-02-proton-manager-editing; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "const KIT: Kit = Kit {"
- [x] The credit: Proton Manager in Settings › Gaming · Built on ProtonUp-Qt (GPL-3) (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "\"Built on ProtonUp-Qt (GPL-3)\""
- [x] Gaming ›, Proton and Wine, PROTON MANAGER, ↻ Check for new versions (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "\"↻ Check for new versions\""
- [x] "What runs your Windows games. Hearth keeps the versions you have; nothing is added, removed or switched unless you say so." (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "nothing is added, removed or switched unless you say so."

## Installed
- [x] Installed · 6 · its size · All / Proton / Wine (CI picture oss-a-02-proton-manager; needs testing on a device; the board's 4.1 GB is its six sizes' sum in decimal, the app shows 3.7 GB) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "(\"Wine\".into(), a.filter == 2, Msg::Filter(2))"
- [x] Each version: P / W mark, name, · DEFAULT, where it's from · its size, how many games use it, Default / Make default, Remove (only when no game uses it) (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "let can_remove = n == 0 && !is_def"
- [x] Versions found where ProtonUp-Qt finds them: built into Hearth OS, Hearth's runners, Steam's compatibilitytools.d and its Protons, Lutris' wine runners, the system's Wine (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "fn scan(default: String"
- [x] Make default: Hearth's own, Steam's (config.vdf, only while Steam is closed) and Lutris' default (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/steam.rs: "pub fn set_mapping("

## Get more
- [x] Get more: Proton Hearth / Proton-GE / Wine-GE / Valve Proton / Luxtorpeda (each project's GitHub releases; Valve's through Steam) (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "Family { tab: \"Luxtorpeda\", repo: \"luxtorpeda-dev/luxtorpeda\""
- [x] Each release: date and what's new, NEW, INSTALLED, Get, older (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "format!(\"{} · older\", r.day)"
- [x] Downloading: name · downloading, its bar, "62% · 490 of 790 MB", Pause (and Resume where it stopped) (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "\"{}% · {} of {} MB\""
- [x] Checked with the project's SHA-512 and unpacked into Hearth's runners, shown to Steam and Lutris by a link (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "fn unpack(dl: Dl"

## Each game's pick
- [x] Each game's pick · "only games you changed are listed · 7 of 215" · Find a game (CI picture oss-a-02-proton-manager; needs testing on a device; the board's 214 doesn't add up with its own counts (215)) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "\"only games you changed are listed · {} of {}\""
- [x] GAME / FROM / RUNS WITH ▾ / WHY / Use default, for Steam's games (config.vdf), Game Fixer's (Lutris) and Hearth Games' (proton-worked.json) (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "container(col(&p, \"Runs with\"))"

## How Hearth picks
- [x] For new games ▾ (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "\"For new games\""
- [x] Get new versions by themselves · Tell me when a new version is out · Remove versions no game uses · Show versions in Game Mode's game menu (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "(\"Show versions in Game Mode's game menu\", \"gameMenu\", true)"
- [x] Where they live: ~/.local/share/hearth/runners (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "\"Where they live\""
- [x] "Proton Hearth 9-5 is out" · Get it · "Your games stay on 9-4 until you move them. Try it on one game first from that game's page." (CI picture oss-a-02-proton-manager; needs testing on a device) — hosts/hearth-ui/apps/proton-manager/src/main.rs: "Try it on one game first from that game's page."

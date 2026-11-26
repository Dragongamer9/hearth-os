# Hearth Game Fixer (Lutris, GPL-3) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/01-game-fixer.html)

Mockups: open-source-apps/01-game-fixer.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps), made of blocks (studio: Edit layout, Customize, its own blocks
window), built on the open-source project the board credits.

Board: Hearth Game Fixer (Lutris, GPL-3): a Windows game installed with a community script (BFME II with Edain), its runner, fixes you can switch, the script's steps and the logs.

## The window
- [x] Title bar: the GF mark, Hearth Game Fixer, "· any Windows game, with the community's fixes", + Install a game, minimize / maximize / close (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "line: \"· any Windows game, with the community's fixes\".into()"
- [x] Made of blocks: the side, the page and every card can be moved, sized, hidden and customized (Edit layout), with Game Fixer's own buttons, panes and info in its blocks window (CI picture oss-a-01-game-fixer-editing, -customize; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "const KIT: Kit = Kit {"
- [x] The credit: Hearth Game Fixer · Built on Lutris (GPL-3) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "(\"Hearth Game Fixer\", \"Built on Lutris (GPL-3)\")"
- [x] Lutris checked for, and Get Lutris (Flathub, for this person only) when it isn't here (needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "const LUTRIS: Need = Need { name: \"Lutris\""

## The side
- [x] Search games and scripts (a name that isn't yours looks in the community's scripts) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"⌕ Search games and scripts\""
- [x] YOUR GAMES · 9: each game's picture, name and "runner · ready" from Lutris' own list and settings (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "format!(\"Your games · {}\", a.games.len())"
- [x] A game that needs a fix in yellow, one installing in the accent with its percent (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "format!(\"Installing · {}%\", p.round() as i64)"
- [x] The picked game lit, with the accent's bar on its left (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/oss-kit-a/src/lib.rs: "pub fn side_row"
- [x] COMMUNITY SCRIPTS: "4,812 install scripts, checked by people who play them. Pick a game and Hearth does the rest: runner, fixes, extras." (the count is lutris.net's) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "install scripts, checked by people who play them. Pick a game and Hearth does the rest: runner, fixes, extras."
- [x] Browse scripts (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Browse scripts\""

## The game's banner
- [x] Its banner with the hills (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "struct Hills;"
- [x] READY TO PLAY, SCRIPT: ROTWK + EDAIN · V3.2, 7.8 GB (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "format!(\"Script: {name} · v{ver}\")"
- [x] Battle for Middle-earth II and "Rise of the Witch-king · installed from the disc image with the community script" (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "g.note.get(\"about\")"
- [x] ▶ Play (Lutris starts it, its output kept as the last-start log) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "format!(\"lutris:rungame/{}\", g.slug)"
- [x] Open folder (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Open folder\""
- [x] Make a desktop shortcut (the Apps menu and the desktop) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "pub fn shortcut("
- [x] ⋯ (show it in Lutris, copy the folder's path, look again) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Copy the game folder's path\""

## Runner
- [x] Runner · "What runs this Windows game on Hearth" (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"What runs this Windows game on Hearth\""
- [x] Runs with ▾ (Lutris' wine versions; written to the game's own Lutris file) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "&[\"wine\", \"version\"], Some(q.as_str())"
- [x] Script asks for: Wine-GE 8-26 or newer (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Script asks for\""
- [x] Graphics: DXVK 2.4 / OpenGL / WineD3D (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "(\"OpenGL\".into(), gi == 1, Msg::Graphics(1))"
- [x] Screen: Window / Full screen / Gamescope (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "(\"Gamescope\".into(), si == 2, Msg::Screen(2))"
- [x] Resolution ▾ (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Resolution\""
- [x] Prefix: ~/Games/bfme2/prefix · 64-bit (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "c.push(k::kv(&p, \"Prefix\""
- [x] Run before the game: set-hd-mode.sh (editable) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Run before the game\""
- [x] Extra settings: WINEDLLOVERRIDES=d3d9=n · 2 more (editable) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "format!(\"{kk}={v} · {} more\", rest.len())"
- [x] Overlay in this game: Off / FPS / Full (MangoHud through Lutris) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Overlay in this game\""
- [x] OTHER RUNNERS YOU HAVE: Proton Hearth 9-4, Proton 9.0, Wine-GE 8-26, Wine 9.20, DOSBox, ScummVM (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Other runners you have\""
- [x] "Managed in Settings › Gaming › Proton and Wine" with Open (Proton Manager) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Managed in Settings › Gaming › Proton and Wine\""

## Fixes
- [x] Fixes · 6 ON · FROM THE SCRIPT · "Each one can be turned off and back on" (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Each one can be turned off and back on\""
- [x] Large address aware · "Lets the game use 4 GB, stops the late-game crash" (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "\"Lets the game use 4 GB, stops the late-game crash\""
- [x] Widescreen and HUD fix · Online play (GameRanger-style lobby) · DirectX 9 parts · Visual C++ 2005 · Disc check skip (the script's own fixes: its settings or its files, moved aside when off) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "pub fn move_fix_files"
- [x] Esync / Fsync · "Smoother with many units on screen" (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "\"Smoother with many units on screen\""
- [x] Keep the cursor inside the game · "Off: you play with two screens" (off) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "\"Off: you play with two screens\""
- [x] Each one's switch, written to the game's Lutris file one line at a time (atomic, .bak) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "pub fn apply_fix("

## Install script
- [x] Install script · "by the community · 2,314 installs · updated Nov 9" (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "format!(\"{} installs\", k::commas(n))"
- [x] Its five steps, each ticked (now / waiting while it installs) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "let now_ = !done && installing && !seen_now;"
- [x] Installing a game with a community script (lutris.net's list; Lutris asks for the disc images in its own window) (CI picture oss-a-01-game-fixer-install; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "format!(\"lutris:install/{inst}\")"

## Logs
- [x] Logs · Last start / Install (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "(\"Last start\".into(), a.log_tab == 0, Msg::LogTab(0))"
- [x] Copy (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "iced::clipboard::write(t)"
- [x] Send logs (Hearth's own, with the game and its log as the note) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"hearth-send-logs\".into()"
- [x] The log with times, warnings yellow and good lines green (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "pub fn log_lines("

## The window
- [x] Title bar: the GF mark, Hearth Game Fixer, "· any Windows game, with the community's fixes", + Install a game, minimize / maximize / close (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "line: \"· any Windows game, with the community's fixes\".into()"
- [x] Made of blocks: the side, the page and every card can be moved, sized, hidden and customized (Edit layout), with Game Fixer's own buttons, panes and info in its blocks window (CI picture oss-a-01-game-fixer-editing, -customize; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "const KIT: Kit = Kit {"
- [x] The credit: Hearth Game Fixer · Built on Lutris (GPL-3) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "(\"Hearth Game Fixer\", \"Built on Lutris (GPL-3)\")"
- [x] Lutris checked for, and Get Lutris (Flathub, for this person only) when it isn't here (needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "const LUTRIS: Need = Need { name: \"Lutris\""

## The side
- [x] Search games and scripts (a name that isn't yours looks in the community's scripts) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"⌕ Search games and scripts\""
- [x] YOUR GAMES · 9: each game's picture, name and "runner · ready" from Lutris' own list and settings (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "format!(\"Your games · {}\", a.games.len())"
- [x] A game that needs a fix in yellow, one installing in the accent with its percent (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "format!(\"Installing · {}%\", p.round() as i64)"
- [x] The picked game lit, with the accent's bar on its left (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/oss-kit-a/src/lib.rs: "pub fn side_row"
- [x] COMMUNITY SCRIPTS: "4,812 install scripts, checked by people who play them. Pick a game and Hearth does the rest: runner, fixes, extras." (the count is lutris.net's) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "install scripts, checked by people who play them. Pick a game and Hearth does the rest: runner, fixes, extras."
- [x] Browse scripts (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Browse scripts\""

## The game's banner
- [x] Its banner with the hills (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "struct Hills;"
- [x] READY TO PLAY, SCRIPT: ROTWK + EDAIN · V3.2, 7.8 GB (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "format!(\"Script: {name} · v{ver}\")"
- [x] Battle for Middle-earth II and "Rise of the Witch-king · installed from the disc image with the community script" (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "g.note.get(\"about\")"
- [x] ▶ Play (Lutris starts it, its output kept as the last-start log) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "format!(\"lutris:rungame/{}\", g.slug)"
- [x] Open folder (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Open folder\""
- [x] Make a desktop shortcut (the Apps menu and the desktop) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "pub fn shortcut("
- [x] ⋯ (show it in Lutris, copy the folder's path, look again) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Copy the game folder's path\""

## Runner
- [x] Runner · "What runs this Windows game on Hearth" (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"What runs this Windows game on Hearth\""
- [x] Runs with ▾ (Lutris' wine versions; written to the game's own Lutris file) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "&[\"wine\", \"version\"], Some(q.as_str())"
- [x] Script asks for: Wine-GE 8-26 or newer (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Script asks for\""
- [x] Graphics: DXVK 2.4 / OpenGL / WineD3D (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "(\"OpenGL\".into(), gi == 1, Msg::Graphics(1))"
- [x] Screen: Window / Full screen / Gamescope (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "(\"Gamescope\".into(), si == 2, Msg::Screen(2))"
- [x] Resolution ▾ (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Resolution\""
- [x] Prefix: ~/Games/bfme2/prefix · 64-bit (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "c.push(k::kv(&p, \"Prefix\""
- [x] Run before the game: set-hd-mode.sh (editable) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Run before the game\""
- [x] Extra settings: WINEDLLOVERRIDES=d3d9=n · 2 more (editable) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "format!(\"{kk}={v} · {} more\", rest.len())"
- [x] Overlay in this game: Off / FPS / Full (MangoHud through Lutris) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Overlay in this game\""
- [x] OTHER RUNNERS YOU HAVE: Proton Hearth 9-4, Proton 9.0, Wine-GE 8-26, Wine 9.20, DOSBox, ScummVM (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Other runners you have\""
- [x] "Managed in Settings › Gaming › Proton and Wine" with Open (Proton Manager) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Managed in Settings › Gaming › Proton and Wine\""

## Fixes
- [x] Fixes · 6 ON · FROM THE SCRIPT · "Each one can be turned off and back on" (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"Each one can be turned off and back on\""
- [x] Large address aware · "Lets the game use 4 GB, stops the late-game crash" (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "\"Lets the game use 4 GB, stops the late-game crash\""
- [x] Widescreen and HUD fix · Online play (GameRanger-style lobby) · DirectX 9 parts · Visual C++ 2005 · Disc check skip (the script's own fixes: its settings or its files, moved aside when off) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "pub fn move_fix_files"
- [x] Esync / Fsync · "Smoother with many units on screen" (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "\"Smoother with many units on screen\""
- [x] Keep the cursor inside the game · "Off: you play with two screens" (off) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "\"Off: you play with two screens\""
- [x] Each one's switch, written to the game's Lutris file one line at a time (atomic, .bak) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "pub fn apply_fix("

## Install script
- [x] Install script · "by the community · 2,314 installs · updated Nov 9" (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "format!(\"{} installs\", k::commas(n))"
- [x] Its five steps, each ticked (now / waiting while it installs) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "let now_ = !done && installing && !seen_now;"
- [x] Installing a game with a community script (lutris.net's list; Lutris asks for the disc images in its own window) (CI picture oss-a-01-game-fixer-install; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "format!(\"lutris:install/{inst}\")"

## Logs
- [x] Logs · Last start / Install (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "(\"Last start\".into(), a.log_tab == 0, Msg::LogTab(0))"
- [x] Copy (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "iced::clipboard::write(t)"
- [x] Send logs (Hearth's own, with the game and its log as the note) (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/main.rs: "\"hearth-send-logs\".into()"
- [x] The log with times, warnings yellow and good lines green (CI picture oss-a-01-game-fixer; needs testing on a device) — hosts/hearth-ui/apps/game-fixer/src/lutris.rs: "pub fn log_lines("

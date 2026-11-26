# Retro to Steam (Steam ROM Manager, GPL-3) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/05-retro-to-steam.html)

Mockups: open-source-apps/05-retro-to-steam.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps), made of blocks (studio: Edit layout, Customize, its own blocks
window), built on the open-source project the board credits.

Board: Retro to Steam (Steam ROM Manager, GPL-3): emulator games found, the art it found for each, and adding them to the Steam library.

## The window
- [x] The desktop's top bar names it "Retro to Steam" (its menu entry's name) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — os/system_files/usr/share/applications/hearth-retro-to-steam.desktop: "Name=Retro to Steam"
- [x] Its title bar: the RS mark (Hearth's own teal square, no copied icon), "Retro to Steam", "· your emulator games in the Steam library, with art", – ▢ ✕ (and Edit layout) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "· your emulator games in the Steam library, with art"

## Consoles, where the games are, the Steam user
- [x] CONSOLES · FROM HEARTH RETRO: each console Hearth Retro found, its count and emulator ("312 · Snes9x"), a switch to add it or not; the one shown lit (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "CONSOLES · FROM HEARTH RETRO"
- [x] WHERE THE GAMES ARE: the folder (~/Hearth/Retro/roms), typed in and kept as Hearth Retro's own (one setting for both) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/main.rs: "m.insert(\"romsFolder\".into(), Value::String(v));"
- [x] "Each game starts in the emulator Hearth Retro set up, with your controller and saves." (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "Each game starts in the emulator Hearth Retro set up, with your controller and saves."
- [x] STEAM USER: "The one on this PC ▾" (or each person's name from Steam's own loginusers.vdf, to pick) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "\"The one on this PC ▾\""

## The games
- [x] "Found 412 games" and "Art found for 398 · 14 need a pick · 61 already in Steam (kept as they are)", counted for real (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "Art found for {} · {} need a pick · {} already in Steam (kept as they are)"
- [x] Grid art · Wide art · Hero · Logo · Icon: which art the cards show (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/art.rs: "\"Grid art\""
- [x] ⌕ Find a game (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "\"⌕ Find a game\""
- [x] All 412 · Needs a pick 14 · New 351 · Already in Steam 61 (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "Already in Steam {}"
- [x] "Art from community art sites · only images for that game" (libretro's thumbnails; SteamGridDB with your own key: Art sources…) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "Art from community art sites · only images for that game"
- [x] Each game's card: its art (downloaded when it's shown) or its own colours with the diamond, its name on it and under it (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "struct Diamond;"
- [x] ‹ 1 of 6 ›: step through the art found for it (kept as your pick) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "\"{i} of {n}\""
- [x] ● new (or "in Steam") (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "\"● new\""
- [x] No art found · Pick one or use your own (dashed), "needs a pick · …" (… chooses your own picture) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "\"Pick one or use your own\""
- [x] The picked card lit with the accent (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "border: Border { color: p.accent, width: 3.0, radius: 15.0.into() }"

## Picked · how it will look in Steam
- [x] PICKED · HOW IT WILL LOOK IN STEAM, its name, "SNES · Snes9x · Super Metroid (USA).sfc" (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "PICKED · HOW IT WILL LOOK IN STEAM"
- [x] The hero with the logo over it (the logo's picture, or the name as Steam's title), HERO · 3 OF 9 (a press steps to the next) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "HERO · {hi} OF {hn}"
- [x] The grid art with its name, the wide art ("Wide art"), the icon (or its letters, "SM"), "Icon · Logo" and "both found" (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "\"both found\""
- [x] NAME IN STEAM (typed, kept) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "\"Name in Steam\""
- [x] COLLECTION "Retro · SNES ▾" (Retro · SNES, Retro, SNES or None: Steam's tags in shortcuts.vdf) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "\"Collection\""
- [x] Show in Game Mode's Retro shelf (the "Retro" collection) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "Show in Game Mode's Retro shelf"
- [x] Use my own image · Choose… (Hearth's file chooser; for the art kind shown) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "\"Use my own image{}\""

## Adding to Steam
- [x] "Steam has to be closed while games are added. Hearth asks first and opens it again after." with its yellow dot (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "Steam has to be closed while games are added. Hearth asks first and opens it again after."
- [x] Take out what I added (only the ones Retro to Steam added: their ids are kept) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/job.rs: "Took out {removed} games Retro to Steam had added, and their art."
- [x] Preview only (what it would write, nothing written) (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/job.rs: "Nothing has been written."
- [x] Add 351 games to Steam: asks to close Steam if it's open, closes it, downloads the art, writes shortcuts.vdf (the old one kept as .bak; a list that can't be read is never written over), copies the art into config/grid, opens Steam again (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/job.rs: "settings_file::write_bytes(&file, &bytes, vdf::valid)"
- [x] Each shortcut starts the game with hearth-retro-app --play, in the emulator Hearth Retro set up (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/job.rs: "\"--play \\\"{}\\\"\""
- [x] Never a sign-in, an account or a token: Steam's own files on this PC only (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/steam.rs: "never a sign-in, a token or an account"
- [x] "Retro to Steam · Built on Steam ROM Manager (GPL-3)" (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "Built on Steam ROM Manager (GPL-3)"

## Behind the board
- [x] Steam's binary shortcuts.vdf read and written whole (fields it doesn't know kept), Steam's shortcut ids (CRC-32), the text VDF (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/vdf.rs: "pub fn shortcut_id(exe: &str, name: &str) -> u32"
- [x] Every part a block (the title bar, consoles and Steam, the games, Picked, Add to Steam, the credit; and their own parts), moved, sized, hidden and customized in Edit layout (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "pub const BLOCKS: &[PageDef]"
- [x] Retro to Steam's own blocks window: its buttons (Add, Preview, Take out, each art kind, the filters, Art sources, Use my own image), panes and info (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/view.rs: "name: \"Retro to Steam\""
- [x] Its choices kept the safe way (settings_file), only when you change something; your SteamGridDB key readable only by you (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — hosts/hearth-ui/apps/retro-to-steam/src/main.rs: "settings_file::update_json(&prefs_path(), |m| f(m))"
- [x] In the Hearth Store's list (built in) and the quick-update list; its menu entry and icon (CI pictures oss-retro-to-steam-1…7; needs testing on a device) — os/system_files/usr/share/hearth/store/catalog.json: "\"partId\": \"hearth-retro-to-steam-app\""

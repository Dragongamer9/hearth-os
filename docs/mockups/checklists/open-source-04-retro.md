# Hearth Retro (RetroDECK/EmuDeck, GPL-3) in Game Mode (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/04-retro.html)

Mockups: open-source-apps/04-retro.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps), made of blocks (studio: Edit layout, Customize, its own blocks
window), built on the open-source project the board credits.

Board: Hearth Retro (RetroDECK/EmuDeck, GPL-3) in Game Mode: consoles as shelves, the picked game, every emulator set up in one go, the BIOS check and controllers.

## The top bar
- [x] The mark (an R in Hearth Retro's own pink rounded square, no copied icon) and "Hearth Retro" (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "mark(\"R\", 0xff9be8, 0xb04fc8"
- [x] LB · Shelves · Set up · BIOS check · Controllers · Search · RB: LB / RB switch the tabs (the controller's, and Ctrl+Page Up / Down) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "pb(&p, \"LB\", false, true)"
- [x] "1,284 games · 14 consoles", counted from the ROM folders (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "{} games · {} consoles"
- [x] The time and this PC's battery ("1:24 AM · 82%") (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "hearth_ui::time::clock(a.now)"

## The shelves
- [x] Each console a shelf: its name, how many games ("312 games"), its emulator and "see all ›" (opens that console in Search) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "{} · see all ›"
- [x] Eight tiles per shelf: the game's box art (Hearth Retro's own, Retro to Steam's, ES-DE's scraped covers) or its own colours, with its name on the dark fade (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/library.rs: "pub fn art(g: &Game)"
- [x] The picked game lit: the accent outline with the glow (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "blur_radius: 40.0"
- [x] Three shelves in view (SNES, PlayStation, Nintendo 64 on the board); the D-pad and the wheel move down the consoles; each shelf's most recently played first (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "three shelves in view"

## Picked
- [x] PICKED, the game's name, "SNES · 1995 · emulator Snes9x · last played 3 days ago" (the year from ES-DE's game list, last played from Hearth Retro's own log) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "last played {}"
- [x] SAVE STATES · 4 (its save states found), CRT LOOK, 4:3 · SMOOTH (the game's own look, set in ⋯ and given to RetroArch when it starts) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "SAVE STATES · {states}"
- [x] RETRO ACHIEVEMENTS 31/77: your progress from RetroAchievements, with your own name and web API key (⋯ › RetroAchievements…) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "RETRO ACHIEVEMENTS {won}/{of}"
- [x] ▶ Play (starts it in its emulator), Load a save (its save states by slot: RetroArch's --entryslot), ⋯ (the options) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "\"▶ Play\""

## Set up in one go
- [x] "Set up in one go" with DONE (or how many are left, and Set it all up: RetroDECK from Flathub, for you only, and Hearth Retro's one RetroArch config) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "\"Set up in one go\""
- [x] "Every emulator installed and set the same way: your controller, hotkeys, saves in ~/Hearth/Retro, synced with Hearth Link." (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "Every emulator installed and set the same way: your controller, hotkeys, saves in ~/Hearth/Retro, synced with Hearth Link."
- [x] RetroArch, Snes9x, DuckStation, PCSX2, Dolphin, PPSSPP, Flycast, melonDS, mGBA, Mupen64Plus and +4 more, each with its dot (green: here; yellow: not yet), looked for in RetroDECK, Flathub, the system and RetroArch's cores (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/emus.rs: "Emu { name: \"Mupen64Plus\""

## BIOS check
- [x] "BIOS check" and 9 OF 11 (every BIOS folder RetroDECK, EmuDeck, RetroArch and Hearth Retro use) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "\"{good} OF {all}\""
- [x] PlayStation · scph5501.bin · checked (its MD5 is a good one's) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/bios.rs: "490f666e1afb15b7362b406ed1cea246"
- [x] PlayStation 2 · not found · Where to put it (the folder, and Open the BIOS folder) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "\"Where to put it\""
- [x] Saturn · wrong file (bad check) · Why (the file's check and a good one's) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "wrong file (bad check)"

## Controllers
- [x] "Controllers" and "same buttons in every emulator" (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "\"same buttons in every emulator\""
- [x] Player 1 · Xbox Wireless Controller, "Hotkey: ≡ + button · ≡+R1 save · ≡+L1 load" (the hotkeys Set up in one go gives RetroArch) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "Hotkey: ≡ + button · ≡+R1 save · ≡+L1 load"
- [x] Player 2 · 8BitDo SN30 Pro, "Ready · battery 70%": the controllers from the kernel, each with its battery (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "Ready · battery {b}%"

## Game Mode's footer
- [x] "Hearth Retro in Game Mode · also in the desktop as a window" (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "Hearth Retro in Game Mode · also in the desktop as a window"
- [x] A Play, X Saves, Y Favourite, ≡ Options, B Back: the controller's buttons (and buttons to press) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "hint(\"A\", true, false, \"Play\""
- [x] "Hearth Retro · Built on RetroDECK and EmuDeck (GPL-3)" (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "Built on RetroDECK and EmuDeck (GPL-3)"

## Behind the board
- [x] Works with a controller in Game Mode: read from /dev/input while it's in front and no game runs (Hearth Notes' pad.rs), full screen with --game-mode or under gamescope (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/main.rs: "if a.focused && !a.running"
- [x] Each game starts in its emulator (RetroArch's core with Hearth's config, else the emulator on its own); `--play FILE` does it with no window (Retro to Steam's shortcuts) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/main.rs: "fn play_cli(file: &str) -> i32"
- [x] Every part a block: moved, sized, hidden and customized in Edit layout (the top bar, the page, the footer; the shelves and the four cards; every tab's parts) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "pub const BLOCKS: &[PageDef]"
- [x] Hearth Retro's own blocks window: its buttons (tabs, Play, Load a save, Favourite, Options, Set it all up, Read again), panes (the four cards) and info (games, consoles, the picked game, BIOS) (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/view.rs: "name: \"Hearth Retro\""
- [x] Its settings and play log written the safe way (settings_file: atomic, .bak, merged), only when you change something (CI pictures oss-retro-1…11; needs testing on a device) — hosts/hearth-ui/apps/retro/src/prefs.rs: "settings_file::update_json(&path(), |m| f(m))"
- [x] In the Hearth Store's list (built in) and the quick-update list; its menu entry and icon (CI pictures oss-retro-1…11; needs testing on a device) — os/system_files/usr/share/hearth/store/catalog.json: "\"partId\": \"hearth-retro-app\""

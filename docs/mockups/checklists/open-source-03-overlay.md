# Hearth Overlay (MangoHud + GOverlay, MIT/GPL-3) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/03-overlay.html)

Mockups: open-source-apps/03-overlay.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps), made of blocks (studio: Edit layout, Customize, its own blocks
window), built on the open-source project the board credits.

Board: Hearth Overlay (MangoHud + GOverlay, MIT/GPL-3): the overlay over a game (FPS, frame-time graph, GPU and CPU use and temps, memory) and its settings window, with Game Mode's levels.

## The window
- [x] Title bar: the HO mark, Hearth Overlay, "· settings · changes show live in the game" (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "\"· settings · changes show live in the game\""
- [x] Made of blocks (Edit layout), with its own buttons, panes and info (CI picture oss-a-03-overlay-editing; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const KIT: Kit = Kit {"
- [x] The credit: Hearth Overlay · Built on MangoHud (MIT) and GOverlay (GPL-3) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "\"Built on MangoHud (MIT) and GOverlay (GPL-3)\""
- [x] The overlay over the game (MangoHud draws it from these settings; CI shows MangoHud over vkcube) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const SHOWS: [(&str, &[&str]); 11]"
- [x] All games / Forza Horizon 5 / Elden Ring / + Game (each game's own MangoHud file), On (CI picture oss-a-03-overlay, -forza; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "(\"+ Game\".into(), false, Msg::Pop(Pop::AddGame))"

## Start from and Shows
- [x] START FROM: Just FPS, Small, Full, Battery (Ally), Stream-safe, My own… (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const START: [&str; 6]"
- [x] SHOWS: the eleven, each ticked or not, written to MangoHud's file (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "(\"Network up and down\", &[\"network\"])"

## Where and Look
- [x] WHERE: the 3 × 3 places and its name (Top left) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const PLACES: [(&str, &str); 9]"
- [ ] "Drag it in the game to place it anywhere" (MangoHud can't be dragged in a game; the words are shown, the dragging needs a MangoHud change)
- [x] LOOK: Size 14 px, See-through 62 % (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "format!(\"{} %\", a.see.round())"
- [x] Layout: Rows / One line (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "(\"One line\".into(), one, Msg::OneLine(true))"
- [x] Colours (the four rows' colours) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const COLOUR_KEYS"
- [x] Warn when hot: GPU 83 °C (the field) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "format!(\"GPU {warn} °C\")"
- [ ] Warn when hot doing something in the game (MangoHud has no temperature warning: the number is kept, nothing in the game warns yet)

## Frame rate
- [x] Limit: Off / 30 / 40 / 60 / 120 / 144 / Own… (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const LIMITS: [u32; 6]"
- [x] Smoother frame pacing (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "\"Smoother frame pacing\""
- [x] Show and hide: Shift + F12 (MangoHud's toggle_hud) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "\"toggle_hud\""
- [ ] or Controller: ≡ + R3 (shown; the controller combination needs Hearth's controller host to send MangoHud's key, not done)
- [x] Save the numbers to a file: ~/Hearth/Benchmarks (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "\"Save the numbers to a file\""

## In Game Mode and the buttons
- [x] In Game Mode · Quick menu › Performance › Overlay level 0–4 · 0 Off / 1 FPS / 2 Small / 3 Full / 4 Everything (MangoHud's presets.conf) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/conf.rs: "pub fn set_preset("
- [x] Reset this game · Copy to all games · Done (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "\"Copy to all games\""
- [x] MangoHud checked for, Get MangoHud (Flathub's layer) when it isn't here (needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const MANGOHUD: Need"

## The window
- [x] Title bar: the HO mark, Hearth Overlay, "· settings · changes show live in the game" (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "\"· settings · changes show live in the game\""
- [x] Made of blocks (Edit layout), with its own buttons, panes and info (CI picture oss-a-03-overlay-editing; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const KIT: Kit = Kit {"
- [x] The credit: Hearth Overlay · Built on MangoHud (MIT) and GOverlay (GPL-3) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "\"Built on MangoHud (MIT) and GOverlay (GPL-3)\""
- [x] The overlay over the game (MangoHud draws it from these settings; CI shows MangoHud over vkcube) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const SHOWS: [(&str, &[&str]); 11]"
- [x] All games / Forza Horizon 5 / Elden Ring / + Game (each game's own MangoHud file), On (CI picture oss-a-03-overlay, -forza; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "(\"+ Game\".into(), false, Msg::Pop(Pop::AddGame))"

## Start from and Shows
- [x] START FROM: Just FPS, Small, Full, Battery (Ally), Stream-safe, My own… (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const START: [&str; 6]"
- [x] SHOWS: the eleven, each ticked or not, written to MangoHud's file (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "(\"Network up and down\", &[\"network\"])"

## Where and Look
- [x] WHERE: the 3 × 3 places and its name (Top left) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const PLACES: [(&str, &str); 9]"
- [ ] "Drag it in the game to place it anywhere" (MangoHud can't be dragged in a game; the words are shown, the dragging needs a MangoHud change)
- [x] LOOK: Size 14 px, See-through 62 % (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "format!(\"{} %\", a.see.round())"
- [x] Layout: Rows / One line (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "(\"One line\".into(), one, Msg::OneLine(true))"
- [x] Colours (the four rows' colours) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const COLOUR_KEYS"
- [x] Warn when hot: GPU 83 °C (the field) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "format!(\"GPU {warn} °C\")"
- [ ] Warn when hot doing something in the game (MangoHud has no temperature warning: the number is kept, nothing in the game warns yet)

## Frame rate
- [x] Limit: Off / 30 / 40 / 60 / 120 / 144 / Own… (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const LIMITS: [u32; 6]"
- [x] Smoother frame pacing (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "\"Smoother frame pacing\""
- [x] Show and hide: Shift + F12 (MangoHud's toggle_hud) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "\"toggle_hud\""
- [ ] or Controller: ≡ + R3 (shown; the controller combination needs Hearth's controller host to send MangoHud's key, not done)
- [x] Save the numbers to a file: ~/Hearth/Benchmarks (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "\"Save the numbers to a file\""

## In Game Mode and the buttons
- [x] In Game Mode · Quick menu › Performance › Overlay level 0–4 · 0 Off / 1 FPS / 2 Small / 3 Full / 4 Everything (MangoHud's presets.conf) (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/conf.rs: "pub fn set_preset("
- [x] Reset this game · Copy to all games · Done (CI picture oss-a-03-overlay; needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "\"Copy to all games\""
- [x] MangoHud checked for, Get MangoHud (Flathub's layer) when it isn't here (needs testing on a device) — hosts/hearth-ui/apps/overlay/src/main.rs: "const MANGOHUD: Need"

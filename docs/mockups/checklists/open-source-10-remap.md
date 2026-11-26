# Hearth Remap (Input Remapper, GPL-3) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/10-remap.html)

Mockups: open-source-apps/10-remap.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps/remap), made of blocks (studio: Edit layout, Customize, its own
blocks window), built on Input Remapper's presets and service.

Board: Hearth Remap (Input Remapper, GPL-3): any key, mouse or controller button, per game and app, with macros.

## Every part of the board
- [x] Title bar: the RM mark, Hearth Remap, "· any key, mouse or controller button, your way" (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "any key, mouse or controller button, your way"
- [x] Pause all remaps (and Resume) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "\"Pause all remaps\""
- [x] Minimize, maximize and close (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/oss-kit/kit.rs: "ui::G::Minimize"
- [x] DEVICES: each controller, keyboard and mouse with its line (Bluetooth · player 1, USB · 104 keys, USB · 7 buttons, built in · back paddles) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/ir.rs: "format!(\"{via} · player {players}\")"
- [x] PROFILES FOR THIS CONTROLLER: Everywhere (when nothing else fits), the game's (when the game is in front), Desktop (controller as a mouse), Hearth Files (when Files is in front), Noclip Protocol (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "when nothing else fits"
- [x] ＋ Profile for a game or app (games from Steam's library, Hearth's apps, Desktop Mode) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "＋ Profile for a game or app"
- [x] The profile's name and "Turns on by itself when the game is in front, off when you leave it · from the Game Fixer's game list" (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "Turns on by itself when the game is in front, off when you leave it"
- [x] ON tag (Off, Paused) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "tag(&p, \"On\", Tag::Ok)"
- [x] Copy from… (another profile's changes) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "\"Copy from…\""
- [x] Test it (on now, whatever is in front) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "\"Test it\""
- [x] The controller drawn: body, bumpers, sticks, D-pad, Y X B A, View and ≡ (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/pad.rs: "(Ctl::Btn(308), 650.0, 178.0, \"Y\")"
- [x] Labels with lines for each change: LB → Shift (add to group), X → Macro: Build farm, A → Left click, B → Right click, Left stick → Move the camera, Right stick → Mouse pointer, D-pad → Ctrl + 1 · 2 · 3 · 4 (groups) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "(groups)"
- [x] Lit ones are changed in this profile (from Everywhere) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "fn lit(&self)"
- [x] "Press a button on the controller to pick it · lit ones are changed in this profile" (pressing the controller's button picks it) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "Press a button on the controller to pick it · lit ones are changed in this profile"
- [x] Macro · Build farm and "on X · runs once per press · 0.4 s" (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "runs once per press"
- [x] Once / While held / Turn on/off (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "\"Turn on/off\""
- [x] ● Record (records keys pressed, with the waits between) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "\"● Record\""
- [x] The steps with their numbers and arrows: Press B, Wait 50 ms, Press F, Click where the pointer is, Wait 100 ms, Press Esc (each can be picked, changed or deleted) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/ir.rs: "Click where the pointer is"
- [x] ＋ Step (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "＋ Step"
- [x] YOUR MACROS · 5 with where each is used (X · BFME II, Y · BFME II, Mouse 5 · Files, View · everywhere, Keyboard F9) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "Your macros · {}"
- [x] ＋ New macro (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "＋ New macro"
- [x] Steps can be: press, hold, let go, type words, click, move the pointer, scroll, wait (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "Steps can be: press, hold, let go, type words, click, move the pointer, scroll, wait"
- [ ] Steps can be: run a command (Input Remapper's macros can't run a program, and a Hearth helper reading the keyboard would need root: not built)
- [x] Try it (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "\"Try it\""
- [x] All changes · 8 in this profile (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "{} in this profile"
- [x] Each change: its key, →, what it does (Shift, Macro: Build farm, Left click, Right click, Arrow keys (camera), Mouse pointer, Ctrl + 1, F12 (screenshot)), Edit (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/ir.rs: "Some(\"screenshot\")"
- [x] Edit: a key or combination, Left click, Right click, Mouse pointer, Arrow keys, a macro, or back as it was (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "fn editor("
- [x] ＋ Change another button (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "＋ Change another button"
- [x] THIS PROFILE: Turns on when (BFME II is in front ▾) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "\"Turns on when\""
- [x] Stick dead zone 8 % (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "\"Stick dead zone\""
- [x] Pointer speed 60 % (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "\"Pointer speed\""
- [x] Show what changed on screen (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "\"Show what changed on screen\""
- [x] Hold ≡ for 3 s to turn remaps off (the runner reads the controller's ≡) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/run.rs: "fn watch_menu"
- [x] Profiles turn on by themselves when their game or app is in front (the runner, a guarded user service) (CI pictures remap-1…5; needs testing on a device) — os/system_files/usr/lib/systemd/user/hearth-remap.service: "ExecStart=/usr/bin/hearth-remap-app --run"
- [x] Credit: Hearth Remap · Built on Input Remapper (GPL-3) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "\"Input Remapper (GPL-3)\""
- [x] Made of blocks: devices, profiles, the profile's head, the controller, macros, all changes, this profile (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "bl(\"macro\""
- [x] Its own blocks window: its buttons, panes and info (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "name: \"Remap\""
- [x] Built on Input Remapper: its presets written in its own format (what Hearth doesn't know kept), started and stopped with input-remapper-control (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/ir.rs: "pub fn write_preset"
- [x] Input Remapper off: Turn Input Remapper on; not on the PC: said (it comes with the Hearth OS update) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/remap/src/main.rs: "\"Turn Input Remapper on\""
- [x] Its choices kept safely (atomic, a .bak, merged; opening it writes nothing) (CI pictures remap-1…5; needs testing on a device) — hosts/hearth-ui/apps/oss-kit/kit.rs: "merge_json"

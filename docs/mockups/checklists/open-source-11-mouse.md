# Hearth Mouse (Piper, GPL-2, and libratbag, MIT) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/11-mouse.html)

Mockups: open-source-apps/11-mouse.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps/mouse), made of blocks (studio: Edit layout, Customize, its own
blocks window), built on libratbag's ratbagd D-Bus API (as Piper uses).

Board: Hearth Mouse (Piper/libratbag): DPI steps, buttons, report rate, lights and the profiles kept on the mouse.

## Every part of the board
- [x] Title bar: the MO mark, Hearth Mouse, "· your gaming mouse, set up on Hearth" (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "your gaming mouse, set up on Hearth"
- [x] The mouse picker: Gaming mouse · USB receiver ▾ (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "USB receiver"
- [x] Minimize, maximize and close (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/oss-kit/kit.rs: "ui::G::Minimize"
- [x] Gaming mouse and "7 buttons · 26,000 DPI sensor · wireless · battery 76%" (battery from UPower) (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "DPI sensor"
- [x] SAVED ON THE MOUSE tag (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "\"Saved on the mouse\""
- [x] The mouse drawn: body, wheel, DPI button, side buttons, the logo and bottom strip in their lights' colours (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/pic.rs: "// the lights: the logo and the bottom strip"
- [x] Pins 1–6 and L on its buttons (the picked one lit) (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/pic.rs: "fn pin_at"
- [x] "Click a number or press the mouse's button to pick it" (its own buttons pressed over it pick them) (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/pic.rs: "(mouse::Button::Forward, _)"
- [x] Not tied to one maker (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "\"Not tied to one maker\""
- [x] How fast it moves (DPI) and "5 steps · the DPI button goes through them" (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "steps · the DPI button goes through them"
- [x] ＋ Step (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "\"＋ Step\""
- [x] The five steps: Step n (· now), its DPI, its colour (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "Step {} · now"
- [x] Step 3 slider and the 1600 DPI field (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "text_input(\"1600 DPI\""
- [x] Up and down separately (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "\"Up and down separately\""
- [x] Buttons · this profile: 1 Left button, 2 Right button, 3 Wheel press, 4 DPI button, 5 Side, front, 6 Side, back, each with its list ▾ (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/rb.rs: "\"Side, front\""
- [x] Wheel · Scroll · 3 lines (the desktop's setting) (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "Scroll · {} lines"
- [x] More: any key, a macro from Hearth Remap, a Hearth action · Hearth Remap › (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "More: any key, a macro from Hearth Remap, a Hearth action"
- [x] Reports per second and "Higher is smoother, uses a bit more battery"; 125 / 500 / 1000 / 2000 / 4000 Hz (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "Higher is smoother, uses a bit more battery"
- [ ] Lift-off distance Low / Mid / High (shown; libratbag has no setting for it yet, so it can't be changed from Linux)
- [x] Angle snapping (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "\"Angle snapping\""
- [ ] Sleep after 5 min ▾ (shown; libratbag has no setting for it yet)
- [x] Lights: Logo and Bottom strip, each with its colour and Still / Breathing / Wave (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "\"Bottom strip\""
- [x] Show the DPI step colour (the logo takes the step's colour when you pick a step) (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "\"Show the DPI step colour\""
- [x] Follow Hearth Lights (Hearth Lights' colour and effect on the mouse's lights) (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "\"Follow Hearth Lights\""
- [x] Profiles on the mouse and "5 kept on the mouse itself, so they work on any PC" (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "kept on the mouse itself, so they work on any PC"
- [x] 1 · Everyday (on), 2 · BFME II, 3 · Shooters, 4 · Hearth CAD, 5 · empty (dashed; a press makes it) (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "\"empty\".to_string()"
- [x] Switch with the game: ask me (the runner, a guarded user service) (CI pictures mouse-1…3; needs testing on a device) — os/system_files/usr/lib/systemd/user/hearth-mouse.service: "ExecStart=/usr/bin/hearth-mouse-app --run"
- [x] Credit: Hearth Mouse · Built on Piper (GPL-2) and libratbag (MIT) (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "\"Piper (GPL-2) and libratbag (MIT)\""
- [x] Made of blocks: the mouse, DPI, buttons, reports per second, lights, profiles (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "bl(\"lights\""
- [x] Its own blocks window: its buttons, panes and info (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "name: \"Mouse\""
- [x] Every change written to the mouse (ratbagd, then Commit) (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/rb.rs: "pub fn commit"
- [x] libratbag not here: said (it comes with the Hearth OS update); not answering: Start ratbagd (CI pictures mouse-1…3; needs testing on a device) — hosts/hearth-ui/apps/mouse/src/main.rs: "\"Start ratbagd\""

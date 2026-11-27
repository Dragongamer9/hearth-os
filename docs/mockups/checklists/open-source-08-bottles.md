# Hearth Bottles (Bottles, GPL-3) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/08-bottles.html)

Mockups: open-source-apps/08-bottles.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps), made of blocks (studio: Edit layout, Customize, its own blocks
window), built on the open-source project the board credits.

Board: Hearth Bottles (Bottles, GPL-3): Windows apps in their own bottles, each with its runner, programs, extras and snapshots.

## The window
- [x] Title bar: the BO mark, Hearth Bottles, "· Windows apps, each in its own bottle", + New bottle (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"· Windows apps, each in its own bottle\""
- [x] Made of blocks (Edit layout), with its own buttons, panes and info (CI picture oss-a-08-bottles-editing; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "const KIT: Kit = Kit {"
- [x] The credit: Hearth Bottles · Built on Bottles (GPL-3) (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Built on Bottles (GPL-3)\""
- [x] Bottles checked for, Get Bottles (Flathub) when it isn't here (needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "const BOTTLES: Need"

## The side
- [x] Find a bottle or app (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Find a bottle or app\""
- [x] The six bottles: their bottle, name, "Apps · Wine-GE 8-26 · 6 programs" (Bottles' own bottle.yml) (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "fn read_bottles()"
- [x] Run a .exe · "Drop a Windows program here. Hearth picks a bottle for it or makes a new one." (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "Drop a Windows program here. Hearth picks a bottle for it or makes a new one."

## The bottle
- [x] Its bottle, name, "Kind: Apps · made Oct 30 · 3.8 GB · ~/.local/share/hearth/bottles/office" (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "format!(\"Kind: {}\""
- [x] ▶ Run a program… · Open its C: drive · ⋯ (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Open its C: drive\""
- [x] READY · RUNNER WINE-GE 8-26 · WINDOWS 10 · 64-BIT · DXVK 2.4 · SNAPSHOT 2 H AGO (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "format!(\"Snapshot {}\", ago(s.2))"
- [x] PROGRAMS IN THIS BOTTLE · 6: each with its mark, name, what it is, ▶ Run, In Apps menu ✓, ⋯ (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"In Apps menu ✓\""

## The cards
- [x] Extras installed · + Add (winetricks into the bottle, recorded in Bottles' list): .NET Framework 4.8, Visual C++ 2015–2022, Core fonts, Media Foundation, each with what it's for (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Extras installed\""
- [x] Snapshots · Take one now · Go back (Hearth's own copies of the bottle) (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "fn go_back("
- [x] Running now · Stop all · Word · Story notes.docx, foobar2000 with memory and CPU, Stop (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Running now\""
- [x] Get a program · "installers checked by the community" (Bottles' programs list) · Your own .exe… (CI picture oss-a-08-bottles; needs testing on a device; installing one asks bottles-cli for Bottles' installer, untried on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"installers checked by the community\""

## This bottle's settings
- [x] Runner ▾ · Acts like ▾ (bottles-cli edit) (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "&[\"edit\", \"-b\", &b, \"--win\", w]"
- [x] Graphics DXVK / VKD3D / OpenGL (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "(\"VKD3D\".into(), gi == 1, Msg::Graphics(1))"
- [x] Sharper fonts · Fit to my screen scale (Wine's registry) · Snapshot before each install (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Snapshot before each install\""
- [x] KEPT APART: Can see my home folder · Shared folders (Wine's folders linked to yours) · Internet (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Shared folders\""
- [x] Controller and USB (CI picture oss-a-08-bottles; needs testing on a device; the switch is kept in the bottle's file, Bottles has no per-bottle device setting to apply) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Controller and USB\""
- [x] SHOWS UP IN: Apps menu · Files' Open with (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Files' Open with\""
- [x] SPACE: the bar, "3.8 GB in this bottle", "21.4 GB in all 6" (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "format!(\"{} in all {}\""
- [x] Back up the bottle · Copy it · Delete… (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Back up the bottle\""

## The window
- [x] Title bar: the BO mark, Hearth Bottles, "· Windows apps, each in its own bottle", + New bottle (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"· Windows apps, each in its own bottle\""
- [x] Made of blocks (Edit layout), with its own buttons, panes and info (CI picture oss-a-08-bottles-editing; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "const KIT: Kit = Kit {"
- [x] The credit: Hearth Bottles · Built on Bottles (GPL-3) (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Built on Bottles (GPL-3)\""
- [x] Bottles checked for, Get Bottles (Flathub) when it isn't here (needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "const BOTTLES: Need"

## The side
- [x] Find a bottle or app (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Find a bottle or app\""
- [x] The six bottles: their bottle, name, "Apps · Wine-GE 8-26 · 6 programs" (Bottles' own bottle.yml) (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "fn read_bottles()"
- [x] Run a .exe · "Drop a Windows program here. Hearth picks a bottle for it or makes a new one." (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "Drop a Windows program here. Hearth picks a bottle for it or makes a new one."

## The bottle
- [x] Its bottle, name, "Kind: Apps · made Oct 30 · 3.8 GB · ~/.local/share/hearth/bottles/office" (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "format!(\"Kind: {}\""
- [x] ▶ Run a program… · Open its C: drive · ⋯ (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Open its C: drive\""
- [x] READY · RUNNER WINE-GE 8-26 · WINDOWS 10 · 64-BIT · DXVK 2.4 · SNAPSHOT 2 H AGO (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "format!(\"Snapshot {}\", ago(s.2))"
- [x] PROGRAMS IN THIS BOTTLE · 6: each with its mark, name, what it is, ▶ Run, In Apps menu ✓, ⋯ (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"In Apps menu ✓\""

## The cards
- [x] Extras installed · + Add (winetricks into the bottle, recorded in Bottles' list): .NET Framework 4.8, Visual C++ 2015–2022, Core fonts, Media Foundation, each with what it's for (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Extras installed\""
- [x] Snapshots · Take one now · Go back (Hearth's own copies of the bottle) (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "fn go_back("
- [x] Running now · Stop all · Word · Story notes.docx, foobar2000 with memory and CPU, Stop (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Running now\""
- [x] Get a program · "installers checked by the community" (Bottles' programs list) · Your own .exe… (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"installers checked by the community\""
- [x] Installing one of the community's installers: Hearth runs Bottles' manifest itself, step by step in the bottle (extras, checked downloads, the setup, registry, DLLs, the program added), each step shown (CI pictures oss-a-08-bottles-installing, -installed; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/installer.rs: "pub fn do_step(c: &Ctx, s: &[(String, String)])"

## This bottle's settings
- [x] Runner ▾ · Acts like ▾ (bottles-cli edit) (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "&[\"edit\", \"-b\", &b, \"--win\", w]"
- [x] Graphics DXVK / VKD3D / OpenGL (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "(\"VKD3D\".into(), gi == 1, Msg::Graphics(1))"
- [x] Sharper fonts · Fit to my screen scale (Wine's registry) · Snapshot before each install (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Snapshot before each install\""
- [x] KEPT APART: Can see my home folder · Shared folders (Wine's folders linked to yours) · Internet (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Shared folders\""
- [x] Controller and USB: Wine's own bus driver settings (winebus: SDL and input controllers, hidraw USB devices) imported into the bottle's registry, on or kept apart (needs testing on a device) — hosts/hearth-ui/apps/bottles/src/installer.rs: "pub fn devices_reg(on: bool) -> String"
- [x] SHOWS UP IN: Apps menu · Files' Open with (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Files' Open with\""
- [x] SPACE: the bar, "3.8 GB in this bottle", "21.4 GB in all 6" (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "format!(\"{} in all {}\""
- [x] Back up the bottle · Copy it · Delete… (CI picture oss-a-08-bottles; needs testing on a device) — hosts/hearth-ui/apps/bottles/src/main.rs: "\"Back up the bottle\""

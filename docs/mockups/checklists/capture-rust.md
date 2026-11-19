# Capture in pure Rust (Mantel's Capture, ported for the Fedora edition)

Mockups: capture-mantel/1-take.jpg, capture-mantel/2-editor-share.jpg, capture-mantel/3-scroll-pill.jpg
Release: next

Capture has no approved mockup of its own: these three pictures are Mantel's Capture as built and shown in 1.14 (CI
pictures on KWin, Nov 17), the look this port matches part for part (Capture.qml, CaptureEditor.qml, CaptureBtn.qml,
CaptureGlyph.qml, CaptureIndicator.qml, capture-draw.js). The app is hosts/hearth-ui/apps/capture (hearth-capture-app),
for KDE Plasma on KWin, where Mantel can't run; its pictures come from hearth-apps-rust.yml
(.github/scripts/hearth-apps-rust-shots.d/capture.sh: capture-1 … capture-20). Parts that only a KWin session can
show (KWin's screenshots, Print Screen, the pills' places) are built but left open until they're proven on a device.

## 1-take: Take
- [x] Title bar with Capture's camera, and the header: the camera in an accent square, "Capture", the Take / Edit / Recent pages (Recent with its count) — hosts/hearth-ui/apps/capture/src/pages.rs: "format!(\"Recent  {}\", a.recent.len())"
- [x] What the controller does, on the right: "A choose · B close" (the editor's "Ctrl+Z undo · Ctrl+C copy · Ctrl+S save") — hosts/hearth-ui/apps/capture/src/pages.rs: "\"A choose  ·  B close\""
- [x] "What to capture": An area, A window, This screen, All screens, Scrolling, each with its picture and what it is; the chosen one in the accent — hosts/hearth-ui/apps/capture/src/pages.rs: "(\"scroll\", \"scroll\", \"Scrolling\", \"A long picture of a page that scrolls\")"
- [x] A double click on one takes it — hosts/hearth-ui/apps/capture/src/pages.rs: ".on_double_click(Msg::TakeMode(id))"
- [x] "Wait before taking it": No wait, 3 s, 5 s, 10 s (with the timer) — hosts/hearth-ui/apps/capture/src/pages.rs: "label(\"Wait before taking it\", 15.0, p.text, true)"
- [x] Scrolling's "Hearth scrolls it for me" / "I'll scroll it myself" — hosts/hearth-ui/apps/capture/src/pages.rs: "\"I'll scroll it myself\""
- [x] "Leave the pointer out" / "The pointer is in the picture" — hosts/hearth-ui/apps/capture/src/pages.rs: "\"The pointer is in the picture\""
- [x] Take a screenshot (Take in N s with a wait), with the chosen kind's picture — hosts/hearth-ui/apps/capture/src/pages.rs: "format!(\"Take in {} s\", a.delay())"
- [x] Record, dimmed when no recorder is installed (and says so when pressed) — hosts/hearth-ui/apps/capture/src/main.rs: "No screen recorder is installed (Hearth Recorder)."
- [x] What went wrong, in red, above Take — hosts/hearth-ui/apps/capture/src/pages.rs: "label(a.error.clone(), 13.0, p.danger, false)"
- [x] Recent at a glance: "Recent", See all, the last six pictures, "Screenshots you save show up here…" — hosts/hearth-ui/apps/capture/src/pages.rs: "Screenshots you save show up here. They're kept in Pictures › Screenshots."
- [x] "Print Screen saves the whole screen right away · Super+Shift+S picks an area" — hosts/hearth-ui/apps/capture/src/pages.rs: "Print Screen saves the whole screen right away · Super+Shift+S picks an area"
- [x] Your choices kept in ~/.config/nimbus/capture.json, only when you change one (opening writes nothing; CI checks) — hosts/hearth-ui/apps/capture/src/main.rs: "settings_file::merge_json(&prefs_path(), &c)"
- [x] The D-pad / arrows move along the controls, A (Enter) presses, B (Esc) closes, with the focus ring — hosts/hearth-ui/apps/capture/src/pages.rs: "Key::Named(Named::Escape) => return Task::done(Msg::Close)"

## Recent
- [x] The pictures saved lately, each with its name, when and its size; a click opens it in the editor — hosts/hearth-ui/apps/capture/src/pages.rs: "w.push_str(&format!(\"  ·  {} × {}\", it.w, it.h))"
- [x] On the one under the pointer (or the controller's): Copy, Show in folder, Take off the list — hosts/hearth-ui/apps/capture/src/pages.rs: "b(\"\").glyph(\"folder\").view(&p, Msg::RecentFolder(path.clone()))"
- [x] "A edit · X show in folder · Delete take off the list", and Enter / Delete / Menu / Esc — hosts/hearth-ui/apps/capture/src/pages.rs: "\"A edit  ·  X show in folder  ·  Delete take off the list\""
- [x] "Nothing saved yet…" when there's nothing — hosts/hearth-ui/apps/capture/src/pages.rs: "Nothing saved yet. Screenshots you save are kept in Pictures › Screenshots."

## 2-editor-share: the editor
- [x] Tools: Arrow, Box, Pen, Highlighter, Text, Blur, Pixelate, Numbered steps, Crop (the chosen one with its name) — hosts/hearth-ui/apps/capture/src/draw.rs: "(T::Step, \"steps\", \"Numbered steps\")"
- [x] Seven colours, S / M / L, Undo and Redo — hosts/hearth-ui/apps/capture/src/editor.rs: "for (k, t) in [('s', \"S\"), ('m', \"M\"), ('l', \"L\")]"
- [x] Arrows, boxes, pen and highlighter drawn on the picture as you drag — hosts/hearth-ui/apps/capture/src/editor.rs: "f.stroke(&path, st(lw * 3.5, Color { a: 0.38, ..c }, LineCap::Square))"
- [x] Text typed right where you click, with its outline — hosts/hearth-ui/apps/capture/src/editor.rs: "layers = layers.push(pin(field).x(tx.max(0.0)).y(ty.max(0.0)))"
- [x] Blur and Pixelate (nothing readable left in the saved picture) — hosts/hearth-ui/apps/capture/src/export.rs: "draw::blur(&mut region, rw, rh, (1.0 / draw::BLUR).round() as usize)"
- [x] Numbered steps, counting up — hosts/hearth-ui/apps/capture/src/editor.rs: "let n = draw::next_step(&ed.shapes);"
- [x] Crop: outside dimmed, "Drag the part to keep · W × H", All of it, Done — hosts/hearth-ui/apps/capture/src/editor.rs: "format!(\"Drag the part to keep{}\""
- [x] Save and copy (Pictures › Screenshots, and copied; "Working…" meanwhile), at full size with the markup, never over the picture taken — hosts/hearth-ui/apps/capture/src/editor.rs: "format!(\"Saved as {name} and copied\")"
- [x] Copy, Save as…, Show in folder, Share… — hosts/hearth-ui/apps/capture/src/editor.rs: "b(\"Share…\").glyph(\"share\")"
- [x] Read text and QR code (dimmed without tesseract / zbarimg), New screenshot — hosts/hearth-ui/apps/capture/src/editor.rs: "b(\"New screenshot\").glyph(\"area\")"
- [x] Share: Copy, Save, Open with (the apps for pictures, the default marked), Send to your phone (Phone link's phones; "Phone link is off…" / "No phone connected right now.") — hosts/hearth-ui/apps/capture/src/editor.rs: "\"Phone link is off (Settings › Phone).\""
- [x] Save as: where (Pictures › Screenshots and a new name), PNG, JPEG or WEBP by its ending, "There's a file with that name. Replace it?", the keyboard button, Cancel, Save / Replace — hosts/hearth-ui/apps/capture/src/editor.rs: "There's a file with that name. Replace it?"
- [x] Text in the picture (selectable), its messages, Copy the text — hosts/hearth-ui/apps/capture/src/editor.rs: "Reading text needs tesseract (tesseract-ocr), which isn't installed on this PC."
- [x] QR codes, each with Copy and Open (for a link), and their messages — hosts/hearth-ui/apps/capture/src/editor.rs: "Reading QR codes needs zbarimg (zbar-tools), which isn't installed on this PC."
- [x] What happened, for a few seconds at the top ("Saved as …", "Copied: paste it into any app"…) — hosts/hearth-ui/apps/capture/src/editor.rs: "\"Copied: paste it into any app\""
- [x] The picture's size, "Saved: name", and how a scrolling capture went — hosts/hearth-ui/apps/capture/src/editor.rs: "format!(\"\\nSaved: {}\""
- [x] Ctrl+Z, Ctrl+Y / Ctrl+Shift+Z, Ctrl+C, Ctrl+S, Ctrl+Shift+S, Esc — hosts/hearth-ui/apps/capture/src/main.rs: "} else if ctrl && ch == \"s\" && shift {"

## The area picker
- [x] A still picture of the screen, dimmed outside the box; the box in the accent with its four corners — hosts/hearth-ui/apps/capture/src/picker.rs: "let dim = Color::from_rgba8(5, 7, 10, 0x8c as f32 / 255.0);"
- [x] The size by the box ("· sizing" while sizing) — hosts/hearth-ui/apps/capture/src/picker.rs: "if a.sel_sizing { \"  ·  sizing\" }"
- [x] At the top: "Drag around what you want" (or what this kind asks), "D-pad move · X size · A take · B cancel" — hosts/hearth-ui/apps/capture/src/picker.rs: "\"D-pad move  ·  X size  ·  A take  ·  B cancel\""
- [x] Take and Cancel by the box; drag a box, drag inside it to move it, double-click to take; the D-pad moves and sizes it — hosts/hearth-ui/apps/capture/src/picker.rs: "draw::resize_sel(c, dx, dy, w, h)"
- [ ] One picker on every screen at once (Mantel's): a Wayland app can only cover the screen it's on, so here it's the screen you're on

## 3-scroll-pill: scrolling capture
- [x] The pill: "Scroll it slowly: Hearth joins the pictures" / "Scrolling and joining the pictures…", "N pictures · H pixels tall", Cancel, Done — hosts/hearth-ui/apps/capture/src/pages.rs: "format!(\"{frames} {}  ·  {tall} pixels tall\""
- [x] Mantel's scrolling capture behind it (hearth-capture-helper scroll, scroll-stop) and the note in the editor — hosts/hearth-ui/apps/capture/src/main.rs: "\"It didn't scroll, so this is one picture. \""
- [ ] The pill at the top of the screen, or the bottom when the area starts near the top, and out of the pictures (a KWin script: kwin.rs script_for; built, needs a test on a device)

## The countdown and the capture dot
- [x] "Taking it in N" with the timer; a click cancels — hosts/hearth-ui/apps/capture/src/pages.rs: "format!(\"Taking it in {}\", a.countdown.max(1))"
- [x] The capture dot: "Recording m:ss" with the blinking red dot and Stop; "<device> is connected" with Disconnect — hosts/hearth-ui/apps/capture/src/indicator.rs: "format!(\"Recording {}\", i.elapsed())"
- [ ] The countdown at the top and the dot at the top right on Plasma (the KWin script; built, needs a test on a device)

## On KDE Plasma (os-bases/anatase/build.sh "Capture (Rust)")
- [x] Capture in Plasma's menu (hearth-capture.desktop: the app, its actions; Spectacle's entries out of the menu), only when the app was built and starts — os-bases/anatase/build.sh: "# >>> Capture (Rust)"
- [ ] Screenshots from KWin's org.kde.KWin.ScreenShot2, allowed through Capture's menu entry, for Capture, Print Screen and hearth-screenshot (kwin.rs `shot`; built, needs a test on a device)
- [ ] Print Screen saves the whole screen right away, Super+Shift+S picks an area (KDE's global shortcuts; built, needs a test on a device)
- [ ] Record through hearth-record (KDE's screen-sharing portal, PipeWire, GStreamer's encoder), else Hearth Recorder (built, needs a test on a device)

## Status (Nov 18)
Built and pictured in hearth-apps-rust.yml (capture-1 … capture-20, each opened next to the pictures above): every
page, panel, the picker, the pills and the dot; a real screenshot and a real area picker through the helper (grim
under CI's sway); Save and copy checked at full size with its markup and on the recent list; Take at rest 0 % CPU,
22 MB. Open: the four device-only parts above (KWin, the keys, the pills' places, recording) and the one-picker-per-screen
part Wayland doesn't allow. Mockups: Mantel's built pages stand in for mockups until the owner approves them as such.

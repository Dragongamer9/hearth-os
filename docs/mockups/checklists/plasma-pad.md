# The controller in KDE Plasma (the Fedora edition): hearth-pad-host

Mockups: plasma-pad/1-menu.jpg, plasma-pad/2-keyboard.jpg, plasma-pad/3-controls.jpg
Release: next

The owner (Nov 18, 2026, on his ROG Ally): the controller should work in the Fedora edition's Desktop Mode like it does
on Debian. There's no drawn mockup for these: what's approved is Mantel's own (hosts/linux/desktop/shell Pad.qml,
RadialMenu.qml, Help.qml with osk-plasma.qml's list, Keyboard.qml / OskBoard.qml / OskKey.qml / OskThemes.qml,
HintStrip.qml), ported part by part. The three pictures in docs/mockups/plasma-pad are CI's pictures of this port
(hearth-apps-rust.yml, pad-1-menu, pad-2-keyboard, pad-5-controls), there for the owner to compare with Debian and
approve or change; they're not drawn mockups. Code: hosts/hearth-ui/apps/pad (binary hearth-pad-host), started by
os-bases/anatase/system_files/usr/libexec/hearth/hearth-pad-plasma (hearth-pad --standalone stays the fallback).

## Running the controller helper (Pad.qml)
- [x] hearth-pad --plasma started, read and written one JSON line at a time, started again 3 s after it ends (not after exit 2 / 3) — hosts/hearth-ui/apps/pad/src/link.rs: "if tx.unbounded_send(Ev::Ended(code)).is_err() || code == 2 || code == 3"
- [x] ready: the controller mouse setting and an open keyboard told again — hosts/hearth-ui/apps/pad/src/main.rs: "pad_send(json!({ \"cmd\": \"mouse\", \"on\": self.mouse_on }));"
- [x] KWin's hearthpad script (which window is in front) loaded for the session when the image didn't turn it on — hosts/hearth-ui/apps/pad/src/main.rs: "org.kde.kwin.Scripting.loadScript"
- [x] fullscreen games pause the controller (hearth-pad, from KWin's window in front) — hosts/linux/desktop/shell/hearth-pad: "fs = fullscreen and kind not in ('desktop', 'dock') and cls.lower() not in PLASMA_SHELL"
- [x] controller apps own the controller on Plasma too (owner {app}); Guide still opens the menu, Guide + R3 the pointer — hosts/linux/desktop/shell/hearth-pad: "self.set_front(cls, caption)"
- [x] the Hearth menu's Controller mouse over a controller app: the pointer in that app (padmouse) — hosts/hearth-ui/apps/pad/src/main.rs: "pad_send(json!({ \"cmd\": \"padmouse\" }));"
- [x] no layer-shell: exit 4, and the launcher falls back to the plain controller — os-bases/anatase/system_files/usr/libexec/hearth/hearth-pad-plasma: "if [ \"$rc\" != 4 ] && { [ \"$rc\" = 0 ] || [ \"$ran\" -ge 20 ]; }; then"
- [x] nothing runs at rest (pictured run: 0.00 % of a core, 6.9 MB) — .github/scripts/hearth-apps-rust-shots.d/pad.sh: "At rest (nothing open, no helper): nothing may run."

## The Hearth menu (RadialMenu.qml, Guide)
- [x] the screen dimmed, the soft disc with the warm glow in the middle — hosts/hearth-ui/apps/pad/src/view.rs: "The soft disc behind the items, warmed by the fire in the middle."
- [x] the Hearth mark in the middle, the lit item's name under it, "Stick to choose · A select · B close" — hosts/hearth-ui/apps/pad/src/view.rs: "Stick to choose · A select · B close"
- [x] eight items clockwise from the top: Apps, Keyboard, Quick Access, Close window, Game Mode, Screenshot, Controller mouse (on / off), Controls — hosts/hearth-ui/apps/pad/src/main.rs: "Item { action: \"screenshot\", label: \"Screenshot\", icon: G::Camera },"
- [x] the lit item in the accent, a little bigger — hosts/hearth-ui/apps/pad/src/view.rs: "let d = s * 0.17 * if lit { 1.14 } else { 1.0 };"
- [x] the mouse and touch: pointing lights an item, a click runs it, a click outside closes the menu — hosts/hearth-ui/apps/pad/src/view.rs: "on_enter(Msg::RadialHover(i as i32)).on_press(Msg::RadialPick(i as i32))"
- [x] what the items do in Plasma (osk-plasma.qml): Plasma's launcher, the keyboard, Alt+F4, Game Mode, Spectacle — hosts/hearth-ui/apps/pad/src/main.rs: "sleep 0.5; spectacle -f -b -n 2>/dev/null"
- [ ] the open / close animation (scale and fade): drawn at once, no animation
- [ ] the Hearth logo picture (images/hearth-logo.png): drawn as a flame mark instead (the apps' renderer has no image support switched on)
- [ ] Quick Access: Hearth's Quick Access widget isn't in the Fedora edition's Plasma yet; the item opens Hearth Settings there

## The controls card and messages (Help.qml)
- [x] the card: Hearth mark, "Controller controls", the line under it, Plasma's fifteen controls in two columns — hosts/hearth-ui/apps/pad/src/view.rs: "(\"View + A\", \"The taskbar (D-pad along it, A opens)\"),"
- [x] More help: Welcome & Tips (hearth-tools --app welcome --page help) — hosts/hearth-ui/apps/pad/src/main.rs: "\"--app\".into(), \"welcome\".into(), \"--page\".into(), \"help\".into()"
- [x] closes on a click and by itself after 12 s; View again closes it — hosts/hearth-ui/apps/pad/src/main.rs: "wait(12_000, Wait::Help, g)"
- [x] short messages at the top (Controller mouse on / off, hearth-pad's toast), gone after 1.8 s, clicks go through — hosts/hearth-ui/apps/pad/src/main.rs: "wait(1800, Wait::Toast, g)"

## The button hints (HintStrip.qml)
- [x] A Open, X Options, Y Keyboard, ☰ Apps, ⌂ Hearth menu, ⧉ Controls in a pill along the bottom — hosts/hearth-ui/apps/pad/src/view.rs: "(\"⌂\", p.accent, \"Hearth menu\"),"
- [x] for 12 s after the controller was used for Hearth's menus, not over the menu or the card, clicks go through — hosts/hearth-ui/apps/pad/src/main.rs: "const HINTS_FOR: Duration = Duration::from_secs(12);"

## The on-screen keyboard (Keyboard.qml, OskBoard.qml, OskKey.qml, OskThemes.qml)
- [x] Hearth's one keyboard engine (ui/osk/hearth-osk.js) as it is, so layouts, words, accents, Ember Wheel and split sticks are the same — hosts/hearth-ui/apps/pad/src/osk.rs: "__e = HearthOSK.createEngine({"
- [x] Y and Guide + X open it (hearth-pad), B or its key hides it; its buttons and sticks from hearth-pad (oskpad, oskaxes) — hosts/hearth-ui/apps/pad/src/main.rs: "__e.setHints(true); __e.handle({}, {}, Date.now());"
- [x] typing through hearth-pad's uinput keyboard (emit / key), rumble on a key — hosts/hearth-ui/apps/pad/src/main.rs: "\"emit\" => pad_send(json!({ \"cmd\": \"emit\""
- [x] docked at the bottom (or top) taking its own space so windows shrink, its height by the size setting — hosts/hearth-ui/apps/pad/src/main.rs: "exclusive_zone: Some(h as i32),"
- [x] the strip: word suggestions (the first in the accent) or what the controller does, and the layout badge — hosts/hearth-ui/apps/pad/src/view.rs: "A type · X ⌫ · Y space · LT ⇧ · LB RB move · ☰ Enter · B hide · hold A for accents · RT clicks"
- [x] the keys by the engine's widths, modifier keys, Enter and caps lock in the accent, Shift on, keys that are off faded — hosts/hearth-ui/apps/pad/src/view.rs: "} else if k.go || k.lock {"
- [x] the controller buttons on the keys (X ⌫, Y space, LT, RT...) while it's used — hosts/hearth-ui/apps/pad/src/osk.rs: "pub fn glyph(&self, k: &Key) -> String {"
- [x] the picked key ringed in the accent with its warmth — hosts/hearth-ui/apps/pad/src/view.rs: "shadow: if glow { Shadow { color: Pal::a(accent, 0.35)"
- [x] accents above the key (hold A, View), pick with the D-pad or a tap — hosts/hearth-ui/apps/pad/src/view.rs: "// accents (hold A, View, or a long touch)"
- [x] the Ember Wheel over the keys: eight coals, the tilted one bigger and lit with its four letters — hosts/hearth-ui/apps/pad/src/view.rs: "let pd = wd * 0.26 * if on { 1.45 } else { 1.0 };"
- [x] split sticks' aims (two rings) and the split shape (halves at the edges) — hosts/hearth-ui/apps/pad/src/view.rs: "Color::from_rgb8(0xff, 0xd3, 0x6b)"
- [x] its looks: Steam-style (following the Global Theme on Plasma), Hearth warm, High contrast, Frosted, Pocket, Classic dark, Ember glow, Slate, Daylight — hosts/hearth-ui/apps/pad/src/looks.rs: "pub const IDS: [&str; 9] = [\"steam\", \"warm\", \"contrast\", \"frost\", \"pocket\", \"hearth\", \"ember\", \"slate\", \"daylight\"];"
- [x] the settings read from osk.json (never written), the system's layouts from Plasma's kxkbrc — hosts/hearth-ui/apps/pad/src/osk.rs: "LayoutList="
- [x] learned words kept in osk-words.json the safe way (merged, atomic, .bak), shared with Game Mode — hosts/hearth-ui/apps/pad/src/main.rs: "hearth_ui::settings_file::merge_json(&path, &m)"
- [x] the clipboard panel's recent copies, from Plasma's Klipper — hosts/hearth-ui/apps/pad/src/main.rs: "getClipboardHistoryMenu"
- [x] taps with the mouse or touch on keys, suggestions and accents — hosts/hearth-ui/apps/pad/src/main.rs: "Msg::KeyTap(r, c) => return a.engine"
- [ ] the ember's glide and its trail of sparks, and the pulsing warmth: the ring moves at once, no animation
- [ ] a long touch on a letter for its accents (touch only; hold A and View do it with the controller)
- [ ] the password mode for Hearth's password prompt (Plasma's own prompts don't tell Hearth; the keyboard opens in text mode)
- [ ] opening by itself on a text field after the controller was used: hearth-pad does it (KWin's virtual keyboard signal), not tested on a device yet

## Still to see on a device (needs testing on the ROG Ally)
- [ ] the overlays over a real KWin session, a controller app and a fullscreen game

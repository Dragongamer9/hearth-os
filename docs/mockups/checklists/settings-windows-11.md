# Settings in the Windows 11 look again (approved Nov 28, 2026)

Mockups: settings-windows-11/home.png, settings-windows-11/system.png, settings-windows-11/display.png
Release: next

The owner, Nov 28: "can we make the settings page look more like windows 11 settings again. it was more controller
friendly", then "looks good make it and push they update to it". Ships as a Settings Store update (with Mantel's for the
TV row). Source and its pictures: hearth-workspace hearth/mockups/2026-11-28-settings-windows-11.

## Every page
- [x] your picture (or initial), name and "Administrator · PC name" at the top of the side list — hosts/hearth-ui/apps/settings/src/frame.rs: "let brand: Element<'_, Msg> = row![avatar, who]"
- [x] Find a setting with the search mark, the Y cap beside it and an accent line under it — hosts/hearth-ui/apps/settings/src/frame.rs: "text_input(\"Find a setting\", &a.query)"
- [x] every section in the side list with its icon, the open one lit with the accent mark — hosts/hearth-ui/apps/settings/src/frame.rs: "glyph(cat_glyph(id), if on { p.accent } else { p.muted }, 18.0, 1.8)"
- [x] the controller hints along the bottom: A Select, B Back, Y Find a setting, LB / RB Previous / next section, ☰ Close — hosts/hearth-ui/apps/settings/src/frame.rs: "let page = column![titlebar, body, hints(a)];"
- [x] the focused row ringed in the accent — hosts/hearth-ui/apps/settings/src/widgets.rs: "let el = ring(p, r.lit, p.r(10.0), c);"

## Home (home.png)
- [x] the title "Home" — hosts/hearth-ui/apps/settings/src/frame.rs: "return tb(\"Home\", 34.0, p.text).into();"
- [x] this PC: its picture, name, Hearth OS · graphics · processor, Rename (accent) and About this PC — hosts/hearth-ui/apps/settings/src/cats/home.rs: "let line = small(v(st, \"hearth_short\"), small(v(st, \"graphics\"), data::short_cpu(&sh.info.cpu)));"
- [x] three tiles: Wi-Fi (network · signal), Updates, Bluetooth (on · connected) — hosts/hearth-ui/apps/settings/src/cats/home.rs: "tile(&p, G::Bt, \"Bluetooth\", bt_line, Msg::Go(\"bluetooth\"))"
- [x] Recommended settings: Night light (switch), Light mode while you play (switch), Start in (a drop-down: Game Mode / Desktop Mode) — hosts/hearth-ui/apps/settings/src/cats/home.rs: "dropdown(&p, G::Power, \"Start in\", \"What this PC opens when it turns on\""

## A section (system.png)
- [x] the title is the section's own name ("System") — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "Some(tb(sec.0, 34.0, p.text).into())"
- [x] its pages in one column of big rows, each its own card: icon, name, the line under it, › — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "wrow(p, Some(crate::frame::page_glyph(go)), title, sub, right, open)"

## System › Display (display.png)
- [x] the title "System › Display" — hosts/hearth-ui/apps/settings/src/frame.rs: "let mut parts: Vec<(&'static str, Msg)> = vec![(cat.1, Msg::Cat(a.cat))];"
- [x] Brightness with its slider — hosts/hearth-ui/apps/settings/src/cats/system_display.rs: "c = c.push(srow(&p, G::Display, \"Brightness\""
- [x] Scale & layout: resolution, refresh rate, scale — hosts/hearth-ui/apps/settings/src/cats/system_display.rs: "c = c.push(head(&p, \"Scale & layout\"));"
- [x] Brightness & colour: Night light, HDR — hosts/hearth-ui/apps/settings/src/cats/system_display.rs: "c = c.push(head(&p, \"Brightness & colour\"));"
- [x] Put my screen settings back when the TV reconnects (on by default; Mantel puts them back when a screen comes back) — hosts/hearth-ui/apps/settings/src/cats/system_display.rs: "\"Put my screen settings back when the TV reconnects\""
- [x] Mantel puts the kept settings back on a screen that comes back — hosts/linux/desktop/shell/shell.qml: "command: [\"python3\", Quickshell.shellDir + \"/hearth-screens\", \"restore\"]"

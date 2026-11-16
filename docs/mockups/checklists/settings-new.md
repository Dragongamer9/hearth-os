# The new Settings: keyboard and mouse look (A), controller look (B) (approved Nov 15, 2026)

Mockups: next-update/settings-A1-home-mouse.png, next-update/settings-A2-display-mouse.png, next-update/settings-B1-quick-controller.png, next-update/settings-B2-display-controller.png
Release: 1.14

The owner's choice: Settings shows layout A while you use a keyboard and mouse, and switches to layout B by itself
when it sees a controller being used (and back when the mouse or keyboard is used). Every page works in both.

## Switching between them
- [x] A while the keyboard or mouse was used last, B while a controller was used last, switching live without losing your place — hosts/linux/desktop/shell/SettingsApp.qml: "property string want: SettingsNav.input === \"pad\" && !bFailed ? \"SettingsFrameB.qml\" : \"SettingsFrame.qml\""
- [x] the page and the setting you were on stay picked across the switch — hosts/linux/desktop/shell/SettingsApp.qml: "item.restoreFocus(keep)"

## A: Home (settings-A1)
- [x] left list: you (picture, name, Administrator · PC name), Search settings (Y), the 13 categories with the picked one marked — hosts/linux/desktop/shell/SettingsFrame.qml: "\"Administrator\" : \"Standard account\""
- [x] the PC card: picture, name, Hearth OS version · mode · graphics card · processor, Rename and About this PC — hosts/linux/desktop/shell/SetHome.qml: "page.shortGpu(page.info.gpu), page.shortCpu(page.info.cpu)"
- [x] four quick tiles: Wi-Fi, Bluetooth, Updates, Hearth Link with their state — hosts/linux/desktop/shell/SetHome.qml: "label: \"Hearth Link\"; status: page.linkText"
- [x] Recommended settings: Night light, Light mode while you play, Do not disturb (switches with On/Off), Start in (Game Mode / Desktop Mode) — hosts/linux/desktop/shell/SetHome.qml: "StartInChoice { flat: true; sub: \"What this PC opens when it turns on\" }"
- [x] Storage: each drive with its bar and used of total, Manage — hosts/linux/desktop/shell/SetHome.qml: "title: \"Storage\"; linkText: \"Manage\""
- [x] Changed lately: the last changes (what, from → to, who, when), each with Undo, and All changes — hosts/linux/desktop/shell/SetHome.qml: "title: \"Changed lately\"; linkText: \"All changes\""
- [x] button hints: A Select, B Back, Y Search, LB/RB categories, Close — hosts/linux/desktop/shell/SettingsFrame.qml: "label: \"Previous / next category\""

## A: a page (settings-A2, System › Display)
- [x] the page's path as its title (System › Display) — hosts/linux/desktop/shell/SettingsFrame.qml: "Reg.crumbs(entry.id)"
- [ ] the displays drawn as you have them (1, 2), with the main one's name, resolution, refresh, HDR and graphics card, Identify, Detect, Arrange (drawn, Identify, Detect, Arrange done: SetDisplay.qml "id: scrPic"; still open: HDR is only shown as "HDR off", there is no HDR switch yet)
- [ ] sections with headings (Brightness & colour, Scale & layout, Related), each setting a row with its icon, name, line under it and its control (switch or choices) (Brightness & colour and Scale & layout done, Resolution now a choice like the mockup; still open: the HDR row and the Related section)
- [ ] Related links to other pages (Game Mode's resolution and frame limit) (open: Mantel has no page for Game Mode's resolution and frame limit yet; it needs one first)
- [x] X Keep these settings on a display change (and the old one back by itself if not kept) — hosts/linux/desktop/shell/SetDisplay.qml: "countdown > 0 ? \"Keep these settings\" : \"\""

## B: Quick (settings-B1)
- [x] header "Settings" with the LB/RB tabs: Quick, System, Network, Devices, Gaming, Look, Accounts, Server — hosts/linux/desktop/shell/SettingsFrameB.qml: "model: Reg.TABS"
- [x] big tiles: Light mode while you play (wide), Wi-Fi, Bluetooth, Brightness and Volume (with sliders), Start in (wide, Game Mode / Desktop Mode), Updates, Power, Night light, Do not disturb — hosts/linux/desktop/shell/SetQuick.qml: "readonly property var allTiles: [\"lightmode\", \"wifi\", \"bt\", \"brightness\", \"volume\", \"startin\", \"updates\", \"power\", \"night\", \"dnd\"]"
- [x] the focused tile lifted with the accent outline — hosts/linux/desktop/shell/SetQuick.qml: "transform: Translate { y: tile.activeFocus ? -3 : 0 }"
- [x] X Pin to Quick (pick which settings are on Quick) — hosts/linux/desktop/shell/SetQuick.qml: "readonly property string xLabel: \"Pin to Quick\""

## B: a page (settings-B2, System › Display)
- [x] the category's pages listed on the left (Display, Sound, Power & battery, Storage, Keyboard, Date & time, About) — hosts/linux/desktop/shell/SettingsFrameB.qml: "model: frame.tabPages"
- [x] big rows: sliders with their value, ‹ value › choices changed with left and right, switches, › for more — hosts/linux/desktop/shell/SetChoice.qml: "id: stepper"
- [x] the help panel for the focused setting: what it does, how to change it with the controller, and when it was last changed and by whom — hosts/linux/desktop/shell/SettingsFrameB.qml: "\"Changed by \" + (frame.lastChange.who || \"you\")"
- [x] X Default (puts that one setting back, asking first) — hosts/linux/desktop/shell/SettingsFrameB.qml: "\" back to its default?\""

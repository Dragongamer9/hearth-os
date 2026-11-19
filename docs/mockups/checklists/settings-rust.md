# Hearth Settings as its own pure-Rust app (layout A of the approved new Settings, for the Fedora edition)

Mockups: next-update/settings-A1-home-mouse.png, next-update/settings-A2-display-mouse.png
Release: next

The owner (Nov 18, 2026): Settings is its own app, not tied to the Game Mode app, in pure Rust like the new Task
Manager (hosts/hearth-ui/apps/settings). It follows the Global Theme picked in KDE, as every Hearth app must.

## The frame
- [x] left list: you (picture, name, Administrator · PC name) — hosts/hearth-ui/apps/settings/src/frame.rs: "\"Administrator\" } else { \"Standard account\" }"
- [x] Search settings, finding pages by name and words — hosts/hearth-ui/apps/settings/src/frame.rs: "field(&p, \"Search settings\", &a.query, Msg::Query)"
- [x] the 13 categories with the picked one marked — hosts/hearth-ui/apps/settings/src/registry.rs: "assert_eq!(CATEGORIES.len(), 13);"
- [x] the page's path as its title (System › Display) — hosts/hearth-ui/apps/settings/src/frame.rs: "fn crumbs(a: &App)"
- [x] button hints: A Select, B Back, Y Search, LB/RB categories, Close — hosts/hearth-ui/apps/settings/src/frame.rs: "\"Previous / next category\""
- [x] follows the Global Theme picked in KDE's System Settings, Hearth's looks or any other — hosts/hearth-ui/src/theme.rs: "fn from_plasma() -> Option<Pal>"
- [ ] layout B (the controller look) and switching to it when a controller is used
- [ ] the controller moving between rows (A, B, LB/RB work as keys only: Esc, Page Up / Page Down)

## Home (settings-A1)
- [x] the PC card: picture, name, Hearth OS version · mode · graphics card · processor, Rename and About this PC — hosts/hearth-ui/apps/settings/src/cats/home.rs: "pill(&p, \"About this PC\""
- [x] four quick tiles: Wi-Fi, Bluetooth, Updates, Hearth Link with their state — hosts/hearth-ui/apps/settings/src/cats/home.rs: "tile(&p, G::Link, \"Hearth Link\""
- [x] Recommended settings: Night light (switch), Start in (Game Mode / Desktop Mode) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"What this PC opens when it turns on\", seg(&p"
- [ ] Recommended settings: Light mode while you play, Do not disturb
- [x] Storage: each drive with its bar and used of total, Manage — hosts/hearth-ui/apps/settings/src/cats/home.rs: "Some((\"Manage\", Msg::Go(\"storage\")))"
- [x] Changed lately: the last changes (what → to, who, when), each with Undo — hosts/hearth-ui/apps/settings/src/cats/home.rs: "btn(&p, \"Undo\""
- [ ] All changes (the full list page)
- [ ] Hearth Link's tile shows its devices

## A page (settings-A2, System › Display)
- [x] the displays drawn as you have them (1, 2), with the main one's name, resolution, refresh and graphics card — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"{} · main display\""
- [ ] Identify, Detect, Arrange, HDR
- [x] sections with headings (Brightness & colour, Scale & layout), each setting a row with its icon, name, line under it and its control — hosts/hearth-ui/apps/settings/src/cats/system.rs: "head(&p, \"Scale & layout\")"
- [x] Night light, Brightness, Scale, Resolution, Refresh rate — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Refresh rate\", \"How many pictures a second the display shows\""
- [x] X Keep these settings on a display change, and the old one back by itself if not kept — hosts/hearth-ui/apps/settings/src/cats/system.rs: "fn display_change("
- [ ] Related: Game Mode's resolution and frame limit

## Pages built (beyond the mockups' two)
- [x] Sound: volume, mute, speakers and microphone to use — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Recording from here\""
- [x] Power & battery: battery, power mode — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Power saver for a longer battery, Performance for games\""
- [x] Storage, Startup, Date & time (time zone, set by itself), About (with Rename) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Find a time zone\""
- [x] Your desktop: KDE Plasma, Firehold (Rust, test), Firehold (Plasma, test), Switch now — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Firehold (Plasma, test)\""
- [x] Personalization: the 8 looks, dark / light, the accent (KDE's Global Theme follows) — hosts/hearth-ui/apps/settings/src/cats/pers.rs: "\"Accent colour\", \"Buttons, switches and what's picked\""
- [x] Updates & recovery: check, update (a restore point first), restart, go back a version, System Restore, Reliability history — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "\"Go back a version\""
- [ ] every other page of Mantel's Settings (listed on its category as "Coming next")

## Network & internet (every page, from Mantel's Set*.qml with the same helpers)
- [x] Wi-Fi: the network you're on (name, signal, band, address), Properties, Disconnect, Forget (asked once more) — hosts/hearth-ui/apps/settings/src/cats/net.rs: "Hide properties"
- [x] Wi-Fi: the switch, networks nearby with signal and lock, connect (password on nmcli's stdin), Show all networks — hosts/hearth-ui/apps/settings/src/cats/net.rs: "Show all networks ("
- [x] Wi-Fi: Add a hidden network, Look again, the other network pages listed under it — hosts/hearth-ui/apps/settings/src/cats/net.rs: "Add a hidden network"
- [x] Tailscale: sign in (QR code and link), this PC's name, sign out, your devices with ping and Taildrop — hosts/hearth-ui/apps/settings/src/cats/net_ts.rs: "Waiting for you to sign in…"
- [x] Tailscale: only through Tailscale, start at boot, exit node, receive files, wake from away — hosts/hearth-ui/apps/settings/src/cats/net_ts.rs: "Wake this PC from away"
- [x] VPN: your VPNs, connect (account asked when needed), disconnect, details, remove (asked once more) — hosts/hearth-ui/apps/settings/src/cats/net_vpn.rs: "Connect with this account"
- [x] VPN: add from a file (picker or its place), set up IPsec, L2TP, PPTP, OpenConnect, SSTP, Fortinet by hand — hosts/hearth-ui/apps/settings/src/cats/net_vpn.rs: "Set up a VPN by hand"
- [x] Mobile hotspot: share my internet, show the password, name, password and band — hosts/hearth-ui/apps/settings/src/cats/net_more.rs: "Share my internet"
- [x] Proxy: no, automatic (setup script) or by hand (web, secure web, SOCKS, skip list), Save — hosts/hearth-ui/apps/settings/src/cats/net_more.rs: "Skip the proxy for: "
- [x] Connection details: saved networks, what it has now, auto-connect, metered, MAC, DNS, IPv4, IPv6, MTU, duplicate — hosts/hearth-ui/apps/settings/src/cats/net_details.rs: "Duplicate this connection"
- [x] Connection details: add a wired, mobile broadband or enterprise Wi-Fi network; 802.1X sign-in — hosts/hearth-ui/apps/settings/src/cats/net_details.rs: "Enterprise sign-in (802.1X) for {}"
- [x] Connection details: share a Wi-Fi network with a QR code, mobile broadband APN, PIN and roaming — hosts/hearth-ui/apps/settings/src/cats/net_details.rs: "Show a QR code"

## Bluetooth & devices (every page, from Mantel's Set*.qml with the same helpers)
- [x] Bluetooth: on/off, your devices (connect, disconnect), look for and pair new ones, forget — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "Trust, block, batteries and sending files"
- [x] Your Bluetooth devices: battery, Trust, Block, Send a file — hosts/hearth-ui/apps/settings/src/cats/bt_devs.rs: "Never connects, even when it tries"
- [x] Receive files & this PC's name: rename, put back the usual name, receive files, where, trusted, received lately — hosts/hearth-ui/apps/settings/src/cats/bt_devs.rs: "Take files from trusted devices without asking"
- [x] Keyboard: layouts (add, order, remove), switch shortcut, remember the layout, repeat delay and speed, Num Lock, compose key (KWin's kxkbrc / kcminputrc on Plasma) — hosts/hearth-ui/apps/settings/src/cats/bt_kbd.rs: "Remember the layout"
- [x] Keyboard: the on-screen keyboard (layouts, suggestions, fast typing, its 9 looks, size, shape, position, trail, rumble, clear words) — hosts/hearth-ui/apps/settings/src/cats/bt_kbd.rs: "Clear learned words"
- [x] Keyboard: shortcuts, and bring back KDE's own shortcuts — hosts/hearth-ui/apps/settings/src/cats/bt_kbd.rs: "Bring back KDE's own shortcuts"
- [x] Mouse & touchpad: speed, acceleration, natural scrolling, left-handed, scroll speed, middle-click, middle-click paste, every touchpad setting, gestures — hosts/hearth-ui/apps/settings/src/cats/bt_mouse.rs: "Ignore the touchpad while typing"
- [x] Mouse buttons: each button's job (KWin's rebinds), press a button to add it, a key you press — hosts/hearth-ui/apps/settings/src/cats/bt_mouse.rs: "Point here and press the mouse button you want to change"
- [x] Phone: phone link, pairing requests, calls, your phones, Ring, Ping, clipboard, files, photo, send text, shared, music — hosts/hearth-ui/apps/settings/src/cats/bt_phone.rs: "wants to pair"
- [x] Phone: notifications with Reply, this PC's keyboard and mouse on the phone, its notifications on the phone, what it can do, run commands, pair a new phone — hosts/hearth-ui/apps/settings/src/cats/bt_phone.rs: "This PC's keyboard and mouse on {name}"
- [x] Messages: conversations, one conversation with Reply, a new message — hosts/hearth-ui/apps/settings/src/cats/bt_phone.rs: "Conversations on {}"
- [x] Drawing tablet & touchscreen: one pointer, screen, part of the screen and tablet, left-handed, pressure, pen buttons; touchscreen screen, turned, calibrate (four crosses), undo — hosts/hearth-ui/apps/settings/src/cats/bt_tablet.rs: "Touch the centre of each cross"
- [x] Printers: my printers (test page, default, remove), queue and scanners, add one (look, by address) — hosts/hearth-ui/apps/settings/src/cats/bt_print.rs: "Add by address"
- [x] Print queue: each printer, its jobs with Hold / Go on / Cancel, Cancel all mine, printed lately — hosts/hearth-ui/apps/settings/src/cats/bt_print.rs: "Cancel all mine"
- [x] Scanners: the scanners found and a test scan with its picture — hosts/hearth-ui/apps/settings/src/cats/bt_print.rs: "It scanned. This is the test picture."
- [x] Scan to PDF: scanner, resolution, colour, source, scan pages, order, remove, save, open, start again — hosts/hearth-ui/apps/settings/src/cats/bt_print.rs: "Scan every page in the feeder"
- [x] Ink & toner: the notice switch, each cartridge's level in its colour, low ones marked — hosts/hearth-ui/apps/settings/src/cats/bt_print.rs: "Tell me when ink or toner runs low"
- [x] USB devices & drives: when a drive is plugged in, new USB devices, USBGuard, Allow / Block — hosts/hearth-ui/apps/settings/src/cats/bt_usb.rs: "Allow always"
- [x] Thunderbolt: security, devices, allow once or always, forget — hosts/hearth-ui/apps/settings/src/cats/bt_usb.rs: "Always allow it"

The pictures of these pages: the hearth-apps-rust workflow's settings-nb-* (the pages on a pretend PC, .github/scripts/hearth-apps-rust-shots.d/settings-net-bt.sh, and what they say with nothing to ask). Only the top of a long page is in its picture.

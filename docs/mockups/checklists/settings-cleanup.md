# Settings cleaned up, and Settings › AI (approved Nov 23, 2026)

The owner, Nov 23: "Need to clean it up, along with the Setting App make me some mockups" (after Ollama and Jellyfin
wouldn't start on Fedora and the laptop couldn't find his PC's AI), "Make me mockups on every settings page section",
then "The settings pages look good as well". Built in the Settings app (hosts/hearth-ui/apps/settings, pure Rust);
the values shown are read from the PC, never made up. Sources: hearth-workspace hearth/mockups/2026-11-23-settings-all
and hearth/mockups/2026-11-23-settings-ai. Decided with them (Nov 23): Fedora updates are named by date; nothing
updates by itself (every "by themselves" switch starts off).

Mockups: settings-cleanup/settings-01-home.png, settings-cleanup/settings-02-system.png, settings-cleanup/settings-03-network.png, settings-cleanup/settings-04-bluetooth-and-devices.png, settings-cleanup/settings-05-hearth-link.png, settings-cleanup/settings-06-personalization.png, settings-cleanup/settings-07-apps.png, settings-cleanup/settings-08-gaming.png, settings-cleanup/settings-09-ai.png, settings-cleanup/settings-10-accounts.png, settings-cleanup/settings-11-home-server.png, settings-cleanup/settings-12-privacy-and-security.png, settings-cleanup/settings-13-accessibility.png, settings-cleanup/settings-14-updates-and-recovery.png, settings-cleanup/ai-1-on-the-pc.png, settings-cleanup/ai-2-on-the-laptop.png, settings-cleanup/ai-3-when-something-is-wrong.png, settings-cleanup/ai-4-pick-a-model.png
Release: 1.18

## The frame (every page)
- [x] Left: Settings with Hearth's flame, Search settings, and the 14 sections in this order: Home, System, Network & internet, Bluetooth & devices, Hearth Link, Personalization, Apps, Gaming, AI, Accounts, Home server, Privacy & security, Accessibility, Updates & recovery; the open one lit with the accent line (replaced by the Windows 11 look, settings-windows-11.md, the owner, Nov 28) — hosts/hearth-ui/apps/settings/src/frame.rs: "Your picture and name over the search, as in Windows 11"
- [x] Page title "Settings › <section>" (Home: just "Home") with its one-line description under it (replaced by the Windows 11 look, settings-windows-11.md, the owner, Nov 28) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "Windows 11's look (settings-windows-11 system.png): the section's own name"
- [x] Each section's pages grouped in cards with a small capitals heading; every row: name, a short line, its current value read from the PC, and › — hosts/hearth-ui/apps/settings/src/widgets.rs: "pub fn scard<"
- [x] Switches and sliders right on the page where the mockup has them (they change the setting through the usual safe writers, red flag 9) — hosts/linux/desktop/shell/hearth-settings-values: "sio.merge_json(str(path), {'testVersions' if key == 'test_versions' else 'autoInstall': on}, who='you')"
- [x] The KDE-only pages gathered in one dashed "More KDE settings" row at the bottom of their section, listing them (System, Bluetooth & devices, Personalization, Apps) (shown only where KDE is installed; Hearth OS on Debian has none) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "items.push((\"More in KDE's settings\".to_string(), kde_more(&p, list, Msg::Go(go))));"

## 01. Home
- [x] "Home" with this PC's name, Hearth OS and Fedora versions, "what needs you first, then what you use most" (on Debian: Hearth OS and Debian versions) (replaced by the Windows 11 look, settings-windows-11.md, the owner, Nov 28: just "Home"; this PC's name and version on its card) — hosts/hearth-ui/apps/settings/src/frame.rs: "return tb(\"Home\", 34.0, p.text).into();"
- [x] "N things need you" banner (yellow dot) naming them (Fedora updates ready, a Hearth part to install), Later and Open the Store; hidden when nothing needs you (on Debian: a Hearth OS update ready, Hearth parts to install) — hosts/hearth-ui/apps/settings/src/cats/home.rs: "btn(&p, \"Open the Store\", Kind::Primary, false, Some(Msg::Store))"
- [x] Four tiles: Storage (free, of total, games), Your AI (answering / model), Home server (running count, names), Hearth Link (devices) (replaced by the Windows 11 look, settings-windows-11.md, the owner, Nov 28) — hosts/hearth-ui/apps/settings/src/cats/home.rs: "tile(&p, G::Wifi, \"Wi-Fi\", wifi_text(st)"
- [x] Quick: Wi-Fi and its network with a switch, Bluetooth with connected count and a switch, Night light switch, Do not disturb switch, Volume slider, Brightness slider (Brightness where the PC has a backlight) (replaced by the Windows 11 look, settings-windows-11.md, the owner, Nov 28) — hosts/hearth-ui/apps/settings/src/cats/home.rs: "\"Night light\", \"Warmer colours from sunset to sunrise\""
- [x] You use these most: the five pages you open most, each with its value (opens counted in a small state file, never a setting) — hosts/linux/desktop/shell/hearth-settings-values: "def opened(page):"
- [x] This PC: Name with Rename, Hearth OS, Graphics, Memory in use, Send logs with Send (replaced by the Windows 11 look, settings-windows-11.md, the owner, Nov 28) — hosts/hearth-ui/apps/settings/src/cats/home.rs: "btn(&p, \"About this PC\""

## 02. System
- [x] Screen and sound: Display, Sound, Notifications, Couch Desk and Away (each with its value) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"couch-desk-away\", \"Couch, Desk and Away\", \"Each screen's mode\", V::K(\"couch\"))"
- [x] Power and storage: Power & battery, Memory, Storage, Drives & folders — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"drives\", \"Drives & folders\", \"Hearth (C:), drive letters\", V::K(\"drives\"))"
- [x] Starting and the desktop: Startup, Your desktop, Boot splash (Install Hearth only on the live USB: the owner, Nov 24, "remove the annouying install hearth os in the settings") — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "(x.go != \"install\" || live)"
- [x] Language and time: Date & time, Region & language, Clipboard, Search — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"search\", \"Search\", \"File index, inside documents\", V::K(\"search\"))"
- [x] Reaching this PC: Remote desktop (VNC through Tailscale), Remote desktop (RDP) (RDP shows only where KDE is installed) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "k(\"kde-krdp\", \"Remote desktop (RDP)\""
- [x] About: About, System information (processor, graphics, sensors, USB, PCI) (System information shows only where KDE is installed) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "k(\"kde-info\", \"System information\""
- [x] More KDE settings: Desktop session, Plasma renderer, KRunner, trash, KDE background services, your folders, day and night times, colour profiles (only where KDE is installed) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "kde: Some((\"kde-session\""

## 03. Network & internet
- [x] Connected banner: network, Wi-Fi generation and speed, address, Tailscale's state and address; Details and Share Wi-Fi (QR code) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "b(1, \"Share Wi-Fi (QR code)\", false)"
- [x] Wi-Fi card: the networks (connected / saved / open, strength, security) and the Wi-Fi switch — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "Msg::Switch(\"wifi_on\", \"wifi\", !o)"
- [x] Private and away: Tailscale, VPN, Mobile hotspot, Proxy — hosts/linux/desktop/shell/hearth-settings-values: "put(hotspot='on' if hot else 'off')"
- [x] Details: Connection details, Web search keywords, Push services, Connection preferences (KDE-only rows show only where KDE is installed; Hearth OS on Debian has none) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "k(\"kde-netpref\", \"Connection preferences\""
- [x] This network: Metered switch, Private network switch, Data used this month (Data used shows when the PC can tell) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "V::S(\"private_net\", \"private_net\")"

## 04. Bluetooth & devices
- [x] Four tiles: Bluetooth (paired, connected), Controller (name, battery), Phone (name, linked, battery), Printer (count ready) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "(\"Printer\", or(g(\"printer\"), \"none\"), g(\"printers\"), \"printers\")"
- [x] Wireless: Bluetooth, Phone, Drawing tablet & touchscreen — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"kwin-tablet\", \"Drawing tablet & touchscreen\""
- [x] Typing and pointing: Keyboard, Mouse & touchpad, Logitech G15, Shortcuts (KDE-only rows show only where KDE is installed; Hearth OS on Debian has none) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "k(\"kde-shortcuts\", \"Shortcuts\""
- [x] Plugged in: Printers & scanners, USB devices & drives, Thunderbolt, Device actions (KDE-only rows show only where KDE is installed; Hearth OS on Debian has none) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "k(\"kde-device-actions\", \"Device actions\""
- [x] More KDE settings: Plasma Keyboard (only where KDE is installed) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "kde: Some((\"kde-plasmakeyboard\""

## 05. Hearth Link
- [x] Banner: N devices paired, encrypted, nothing opened to the internet; Pair a device — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "b(0, \"Pair a device\", true)"
- [x] A card per device: its letter, name, kind, battery, temperature, what it's doing, online chip, a line of what it uses; Send files, Help them, More — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "b(\"Send files\", \"link\"), b(\"Help them\", \"remote\"), b(\"More\", \"devices\")"
- [x] Share between them: Files, AI, Clipboard (switch), Game library (Clipboard: Hearth Link sends copied text to your own devices, off by default) — hosts/hearth-link/src/clipboard.rs: "\"clip.put\" => {"
- [x] Your devices: Battery temperatures and drives, Find a device, Forget a device — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"devices\", \"Forget a device\""

## 06. Personalization
- [x] Banner: the look, mode and accent with its swatch, your desktop and wallpaper; My looks, Change the look — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "b(1, \"Change the look\", true)"
- [x] Accent swatches in a row (the picked one ringed) and Light / Dark — hosts/hearth-ui/apps/settings/src/cats/sections.rs: ".on_press(Msg::Accent(name))"
- [x] Your desktop: Desktop layout, Bar and dock, Wallpaper, My looks — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"my-looks\", \"My looks\", \"Save it all and put it back\", V::K(\"my_looks\"))"
- [x] Windows: Window effects, Virtual desktops, Tiling, Screen edges, Window behaviour, Task switcher (KDE-only rows show only where KDE is installed; Hearth OS on Debian has none) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "k(\"kde-tabbox\", \"Task switcher\""
- [x] Text and frames: Fonts cursor & icons, Window frames, Sign-in splash — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"look-more\", \"Sign-in splash\""
- [x] More KDE settings: Application style, Plasma style, colours, global theme, title bars, KWin scripts, activities, font management (only where KDE is installed) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "kde: Some((\"kde-style\""

## 07. Apps
- [x] Four tiles: Installed (count by source), Windows apps, Starting with you, Updates (in the Hearth Store) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "(\"Starting with you\", g(\"autostart_line\")"
- [x] Your apps: Installed apps, Hearth Store, App store (Flathub), Windows apps — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(STORE, \"Hearth Store\", \"Hearth's apps and parts, updates\", V::K(\"parts_ready\"))"
- [x] What opens what: Default apps, File types, App menu editor, Web browser — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"browser\", \"Web browser\""
- [x] Running and allowed: Autostart, Background services, App permissions, Window rules, Claude — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"claude\", \"Claude\", \"Claude on this PC\""
- [x] More KDE settings: Music CDs (only where KDE is installed) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "kde: Some((\"kde-cddb\""

## 08. Gaming
- [x] Banner: Steam Game Mode with Hearth Sync, Decky and its plugins, games across stores; Switch to Game Mode — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "b(0, \"Switch to Game Mode\", false)"
- [x] Game Mode: Steam Game Mode and Decky, Performance, Startup animation, Full-screen games & X11 apps — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"kwin-games\", \"Full-screen games & X11 apps\""
- [x] Your games: Games & stores, Saved games, Discord — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"discord\", \"Discord\""
- [x] Playing: Controller, Remote play, Handheld, Controllers in KDE (KDE-only rows show only where KDE is installed; Hearth OS on Debian has none) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "k(\"kde-gamecontroller\", \"Controllers in KDE\""

## 09. AI (the section page; its own boards ai-1 to ai-4 below)
- [x] Banner: Your AI is answering, engine, model, shared with; Test it, Change model — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "sec_banner(p, sec_dot(p.good), \"Your AI is answering\".into()"
- [x] Engines on this PC: Ollama (state, models, switch), Colibri (state, Pick a model), Models — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "btn(p, \"Pick a model\", BtnKind::Normal, false, Some(Msg::Go(\"hs-ai\")))"
- [x] Sharing: Share this PC's AI (switch), each device using it, the Command Center address with Copy — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "vrow(p, \"Hearth AI Command Center\", &addr, btn(p, \"Copy\""
- [x] What uses it: Describe it, Ithil, AI photos, Sort my Downloads, each with its AI and ready — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "(\"Sort my Downloads\", \"Hearth Files\", \"sortdl\"),"

## 10. Accounts
- [x] Banner: your picture, name, account kind, how you sign in; Change picture — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "b(0, \"Change picture\", false)"
- [x] You and others: Your account, Other people, Family and friends — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"hs-people\", \"Family and friends\""
- [x] Signing in: Sign-in options, Screen locking, Login screen, Fingerprint — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"signin\", \"Fingerprint\""
- [x] Online accounts: each account with signed in, Add an account — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "\"Add an account\", \"Microsoft, CalDAV, IMAP...\""

## 11. Home server
- [x] Banner: N services running with their names, reached through Tailscale; Dashboard, Server mode — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "b(0, \"Dashboard\", false), b(1, \"Server mode\", false)"
- [x] Services: Services, Server apps, AI devices (to Settings › AI), Gaming — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"ai\", \"AI devices\""
- [x] Files and people: Shared folders, Files & links, People, Family & home — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"hs-family\", \"Family & home\""
- [x] Keeping it safe: Drives & undo, Backups & sync, Safety, Alerts & power, Phone notifications — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"hs-phone\", \"Phone notifications\""

## 12. Privacy & security
- [x] Four tiles: Firewall, Secure Boot (and TPM), Vaults, Player count — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "(\"Player count\", or(g(\"player_count\"), \"on\")"
- [x] Protection: Firewall, Firmware & device security, Vaults, Passwords — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"passwords\", \"Passwords\""
- [x] What Hearth remembers: Recent files, Clipboard history, Search index — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"search\", \"Search index\""
- [x] What leaves this PC: Player count (switch), Send logs (only when you press it), KDE user feedback (switch) (KDE user feedback shows only where KDE is installed) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "k(\"kde-feedback\", \"KDE user feedback\""

## 13. Accessibility
- [x] Seeing: Text size slider, Bigger cursor, High contrast, Reduce motion, Zoom (switches), Colour filters — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "container(slider_f32(text, 1.0..=2.0, 0.25, Msg::TextSize, Msg::TextSizeDone)).width(Length::Fixed(190.0))"
- [x] Hearing: Mono audio, Flash the screen for alerts (switches), Live captions — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"accessibility\", \"Live captions\""
- [x] Using: Sticky keys, Slow keys, Mouse keys (switches), On-screen keyboard, Screen reader (Orca) (switch) (on labwc, Hearth's desktop, sticky/slow/mouse keys are saved and the page says plainly they aren't on this desktop yet) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "Not on this desktop yet (saved)"

## 14. Updates & recovery
- [x] Banner: Fedora updates · <date> are ready to get, the packages, the one before stays in the boot menu, a Hearth part to install; Later, Get it now (on Debian: Hearth OS <version> is ready to get, Debian's updates come with it) — hosts/linux/desktop/shell/hearth-settings-values: "bits.append('the one before stays in the boot menu')"
- [x] Updates: Hearth OS (version, "when Dragongamer9 releases one"), Fedora updates (by date), Hearth parts (no restart), Apps (Flathub) (on Debian: Debian updates come with each Hearth OS update) — hosts/linux/desktop/shell/hearth-settings-values: "'%s · when Dragongamer9 releases one'"
- [x] How they come: Fedora updates by themselves (starts off), Hearth parts by themselves (starts off), Get test versions, App updates (Discover) (on Debian: Hearth OS updates by themselves, hearth-admin auto-update; Hearth parts by themselves and Get test versions are the Hearth Store's own settings; App updates (Discover) shows only where KDE is installed) — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(STORE, \"Hearth parts by themselves\""
- [x] Recovery and help: Previous version with Use it, Backup and restore Hearth, Send logs with Send, Recovery USB — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "r(\"backup\", \"Recovery USB\""

## AI 1. Settings › AI on the PC
- [x] "Everything AI on this PC in one place" line — hosts/hearth-ui/apps/settings/src/cats/sections.rs: "\"Everything AI on this PC in one place\""
- [x] Hero: green dot, Your AI is answering, engine, model, test time, shared with N devices; Test it, Change model — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "format!(\"shared with {shared_with} of your devices\")"
- [x] Engines on this PC with the graphics card and its memory: Ollama (running, Hearth's AI uses this, models and size, starts a minute after the PC, switch), Colibri (off, Pick a model), Models (names, search-only marked) with Manage — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "titled(p, \"Engines on this PC\", &head_right, rows)"
- [x] Sharing: Share this PC's AI with my devices (switch, over Hearth Link, nothing opened on your network), each device using it (name, what for, last asked, Stop), Hearth AI Command Center address with Copy — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "\"Over Hearth Link, encrypted · nothing opened on your network\""
- [x] What uses your AI: Describe it, Ithil, AI photos, Sort my Downloads as cards, each with its own AI picker and ready; "Hearth's AI" explained, nothing changes by itself (each pick saved in ai-uses.json by hearth-ai-check use, and honoured: Describe it by hearth-layout engine_for, Sort my Downloads by hearth-layout sort-ask, AI photos by hearth_photos_ai.py chosen_vision, Ithil reads it with hearth-ai-check for ithil) — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "list = list.push(item(which.clone(), !own, Msg::UseFor(id, \"hearth\".into())));"

## AI 2. On the laptop
- [x] Hero: Using <PC>'s AI, over Hearth Link, engine, model, test time; Test it, Use another — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "btn(p, \"Use another\", BtnKind::Normal, false, Some(Msg::Go(\"link\")))"
- [x] Where your AI comes from (first that answers): your PC (answering, using), this laptop (no engine, Install Ollama), another device (not sharing); drag to change the order; tries the next when one is away (the order saved by hearth-ai-check order; hearth-ai-check status and Describe it follow it) — hosts/hearth-ui/apps/settings/src/cats/ai.rs: ".on_press(Msg::DragStart(kind.clone())).interaction(mouse::Interaction::Grab)"
- [x] Test it: what was asked, where it went, ok with the time, blocks, checked by Hearth, nothing changed, Show it (hearth-layout describe-screen asked of the AI itself; Show it opens it in the editor, not kept) — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "A whole desktop came back in {:.1} s"
- [x] Thinking models: Qwen3 asked to answer straight away, its thinking skipped (on) — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "\"Qwen3 is asked to answer straight away; its thinking is skipped\""
- [x] What uses your AI with each one's AI and ready — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "sec_chip(p, if ready { \"ready\" } else { \"waiting\" }, ready)"

## AI 3. When something's wrong
- [x] Red banner: what couldn't start and why in plain words (SELinux), your models and settings are fine; Show what happened, Fix it — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "btn(p, \"Fix it\", BtnKind::Primary, false"
- [x] Yellow banner: a part waiting for your AI (AI photos), starts by itself, nothing to do — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "\"AI photos is waiting for your AI\".into()"
- [x] If no AI answers: Use your PC's AI (Use it), Turn on Ollama here (Install), Another address (Add) — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "opt(\"Another address\", \"Any OpenAI-style AI on your network.\".into(), \"Add\""
- [x] What Hearth checked just now: each engine and device with its state, the model for writing, search-only models; added to Send logs (Send logs includes hearth-ai-check status) — hosts/hearth-core/src/logs.rs: "section(\"AI: what Hearth checked (Settings › AI)\""
- [x] Restarting by itself: tried 5 times in 10 minutes then stopped, Try again — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "tried 5 times in 10 minutes, then stopped"

## AI 4. Pick a model
- [x] A sheet over the page: Model for Hearth's AI and what it's for; ones that can't write greyed out — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "\"What Describe it, Ithil and the rest write with. Ones that can't write are greyed out.\""
- [x] Each model: name, best here, size, fits in memory / on the graphics card, speed, thinks (skipped), uncensored (speed measured by hearth-ai-check speed from Ollama's own eval_duration; "speed not measured yet" until then) — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "about {} words a second"
- [x] Get another model…, Cancel, Use <model> — hosts/hearth-ui/apps/settings/src/cats/ai.rs: "btn(&p, \"Get another model…\""

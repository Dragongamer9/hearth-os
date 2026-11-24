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
- [ ] Left: Settings with Hearth's flame, Search settings, and the 14 sections in this order: Home, System, Network & internet, Bluetooth & devices, Hearth Link, Personalization, Apps, Gaming, AI, Accounts, Home server, Privacy & security, Accessibility, Updates & recovery; the open one lit with the accent line
- [ ] Page title "Settings › <section>" (Home: just "Home") with its one-line description under it
- [ ] Each section's pages grouped in cards with a small capitals heading; every row: name, a short line, its current value read from the PC, and ›
- [ ] Switches and sliders right on the page where the mockup has them (they change the setting through the usual safe writers, red flag 9)
- [ ] The KDE-only pages gathered in one dashed "More KDE settings" row at the bottom of their section, listing them (System, Bluetooth & devices, Personalization, Apps)

## 01. Home
- [ ] "Home" with this PC's name, Hearth OS and Fedora versions, "what needs you first, then what you use most"
- [ ] "N things need you" banner (yellow dot) naming them (Fedora updates ready, a Hearth part to install), Later and Open the Store; hidden when nothing needs you
- [ ] Four tiles: Storage (free, of total, games), Your AI (answering / model), Home server (running count, names), Hearth Link (devices)
- [ ] Quick: Wi-Fi and its network with a switch, Bluetooth with connected count and a switch, Night light switch, Do not disturb switch, Volume slider, Brightness slider
- [ ] You use these most: the five pages you open most, each with its value
- [ ] This PC: Name with Rename, Hearth OS, Graphics, Memory in use, Send logs with Send

## 02. System
- [ ] Screen and sound: Display, Sound, Notifications, Couch Desk and Away (each with its value)
- [ ] Power and storage: Power & battery, Memory, Storage, Drives & folders
- [ ] Starting and the desktop: Startup, Your desktop, Boot splash (Install Hearth only on the live USB: the owner, Nov 24, "remove the annouying install hearth os in the settings")
- [ ] Language and time: Date & time, Region & language, Clipboard, Search
- [ ] Reaching this PC: Remote desktop (VNC through Tailscale), Remote desktop (RDP)
- [ ] About: About, System information (processor, graphics, sensors, USB, PCI)
- [ ] More KDE settings: Desktop session, Plasma renderer, KRunner, trash, KDE background services, your folders, day and night times, colour profiles

## 03. Network & internet
- [ ] Connected banner: network, Wi-Fi generation and speed, address, Tailscale's state and address; Details and Share Wi-Fi (QR code)
- [ ] Wi-Fi card: the networks (connected / saved / open, strength, security) and the Wi-Fi switch
- [ ] Private and away: Tailscale, VPN, Mobile hotspot, Proxy
- [ ] Details: Connection details, Web search keywords, Push services, Connection preferences
- [ ] This network: Metered switch, Private network switch, Data used this month

## 04. Bluetooth & devices
- [ ] Four tiles: Bluetooth (paired, connected), Controller (name, battery), Phone (name, linked, battery), Printer (count ready)
- [ ] Wireless: Bluetooth, Phone, Drawing tablet & touchscreen
- [ ] Typing and pointing: Keyboard, Mouse & touchpad, Logitech G15, Shortcuts
- [ ] Plugged in: Printers & scanners, USB devices & drives, Thunderbolt, Device actions
- [ ] More KDE settings: Plasma Keyboard

## 05. Hearth Link
- [ ] Banner: N devices paired, encrypted, nothing opened to the internet; Pair a device
- [ ] A card per device: its letter, name, kind, battery, temperature, what it's doing, online chip, a line of what it uses; Send files, Help them, More
- [ ] Share between them: Files, AI, Clipboard (switch), Game library
- [ ] Your devices: Battery temperatures and drives, Find a device, Forget a device

## 06. Personalization
- [ ] Banner: the look, mode and accent with its swatch, your desktop and wallpaper; My looks, Change the look
- [ ] Accent swatches in a row (the picked one ringed) and Light / Dark
- [ ] Your desktop: Desktop layout, Bar and dock, Wallpaper, My looks
- [ ] Windows: Window effects, Virtual desktops, Tiling, Screen edges, Window behaviour, Task switcher
- [ ] Text and frames: Fonts cursor & icons, Window frames, Sign-in splash
- [ ] More KDE settings: Application style, Plasma style, colours, global theme, title bars, KWin scripts, activities, font management

## 07. Apps
- [ ] Four tiles: Installed (count by source), Windows apps, Starting with you, Updates (in the Hearth Store)
- [ ] Your apps: Installed apps, Hearth Store, App store (Flathub), Windows apps
- [ ] What opens what: Default apps, File types, App menu editor, Web browser
- [ ] Running and allowed: Autostart, Background services, App permissions, Window rules, Claude
- [ ] More KDE settings: Music CDs

## 08. Gaming
- [ ] Banner: Steam Game Mode with Hearth Sync, Decky and its plugins, games across stores; Switch to Game Mode
- [ ] Game Mode: Steam Game Mode and Decky, Performance, Startup animation, Full-screen games & X11 apps
- [ ] Your games: Games & stores, Saved games, Discord
- [ ] Playing: Controller, Remote play, Handheld, Controllers in KDE

## 09. AI (the section page; its own boards ai-1 to ai-4 below)
- [ ] Banner: Your AI is answering, engine, model, shared with; Test it, Change model
- [ ] Engines on this PC: Ollama (state, models, switch), Colibri (state, Pick a model), Models
- [ ] Sharing: Share this PC's AI (switch), each device using it, the Command Center address with Copy
- [ ] What uses it: Describe it, Ithil, AI photos, Sort my Downloads, each with its AI and ready

## 10. Accounts
- [ ] Banner: your picture, name, account kind, how you sign in; Change picture
- [ ] You and others: Your account, Other people, Family and friends
- [ ] Signing in: Sign-in options, Screen locking, Login screen, Fingerprint
- [ ] Online accounts: each account with signed in, Add an account

## 11. Home server
- [ ] Banner: N services running with their names, reached through Tailscale; Dashboard, Server mode
- [ ] Services: Services, Server apps, AI devices (to Settings › AI), Gaming
- [ ] Files and people: Shared folders, Files & links, People, Family & home
- [ ] Keeping it safe: Drives & undo, Backups & sync, Safety, Alerts & power, Phone notifications

## 12. Privacy & security
- [ ] Four tiles: Firewall, Secure Boot (and TPM), Vaults, Player count
- [ ] Protection: Firewall, Firmware & device security, Vaults, Passwords
- [ ] What Hearth remembers: Recent files, Clipboard history, Search index
- [ ] What leaves this PC: Player count (switch), Send logs (only when you press it), KDE user feedback (switch)

## 13. Accessibility
- [ ] Seeing: Text size slider, Bigger cursor, High contrast, Reduce motion, Zoom (switches), Colour filters
- [ ] Hearing: Mono audio, Flash the screen for alerts (switches), Live captions
- [ ] Using: Sticky keys, Slow keys, Mouse keys (switches), On-screen keyboard, Screen reader (Orca) (switch)

## 14. Updates & recovery
- [ ] Banner: Fedora updates · <date> are ready to get, the packages, the one before stays in the boot menu, a Hearth part to install; Later, Get it now
- [ ] Updates: Hearth OS (version, "when Dragongamer9 releases one"), Fedora updates (by date), Hearth parts (no restart), Apps (Flathub)
- [ ] How they come: Fedora updates by themselves (starts off), Hearth parts by themselves (starts off), Get test versions, App updates (Discover)
- [ ] Recovery and help: Previous version with Use it, Backup and restore Hearth, Send logs with Send, Recovery USB

## AI 1. Settings › AI on the PC
- [ ] "Everything AI on this PC in one place" line
- [ ] Hero: green dot, Your AI is answering, engine, model, test time, shared with N devices; Test it, Change model
- [ ] Engines on this PC with the graphics card and its memory: Ollama (running, Hearth's AI uses this, models and size, starts a minute after the PC, switch), Colibri (off, Pick a model), Models (names, search-only marked) with Manage
- [ ] Sharing: Share this PC's AI with my devices (switch, over Hearth Link, nothing opened on your network), each device using it (name, what for, last asked, Stop), Hearth AI Command Center address with Copy
- [ ] What uses your AI: Describe it, Ithil, AI photos, Sort my Downloads as cards, each with its own AI picker and ready; "Hearth's AI" explained, nothing changes by itself

## AI 2. On the laptop
- [ ] Hero: Using <PC>'s AI, over Hearth Link, engine, model, test time; Test it, Use another
- [ ] Where your AI comes from (first that answers): your PC (answering, using), this laptop (no engine, Install Ollama), another device (not sharing); drag to change the order; tries the next when one is away
- [ ] Test it: what was asked, where it went, ok with the time, blocks, checked by Hearth, nothing changed, Show it
- [ ] Thinking models: Qwen3 asked to answer straight away, its thinking skipped (on)
- [ ] What uses your AI with each one's AI and ready

## AI 3. When something's wrong
- [ ] Red banner: what couldn't start and why in plain words (SELinux), your models and settings are fine; Show what happened, Fix it
- [ ] Yellow banner: a part waiting for your AI (AI photos), starts by itself, nothing to do
- [ ] If no AI answers: Use your PC's AI (Use it), Turn on Ollama here (Install), Another address (Add)
- [ ] What Hearth checked just now: each engine and device with its state, the model for writing, search-only models; added to Send logs
- [ ] Restarting by itself: tried 5 times in 10 minutes then stopped, Try again

## AI 4. Pick a model
- [ ] A sheet over the page: Model for Hearth's AI and what it's for; ones that can't write greyed out
- [ ] Each model: name, best here, size, fits in memory / on the graphics card, speed, thinks (skipped), uncensored
- [ ] Get another model…, Cancel, Use <model>

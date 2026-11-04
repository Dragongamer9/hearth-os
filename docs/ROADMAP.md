# Hearth OS roadmap

Hearth OS is a free, Debian-based OS for gaming PCs and handhelds: it starts in **Steam Game Mode**, with **Hearth
Sync** putting your games from other stores into Steam's library, and **Mantel**, Hearth's own desktop, one press away.
**Ember** is Hearth on Windows 11. Versions are MAJOR.MINOR with a date (docs/UPDATES.md), and the owner picks every
number. Work lands on `bughunt` and goes out on `main` when it's tested. What's archived: docs/ARCHIVED.md.

## Next (unreleased)

- [ ] Hearth Classic round 4: the bottom bar and the in-game top bar fit on a device: Steam's narrow menu window is resized to a bar at the bottom (the top while a game runs) and put back afterwards, or, where Steam won't resize it, Hearth's bar is drawn in the main window and follows Steam's see-through menu (up and down move it, A presses Steam's entry). Diagnostics show the method and the window's size before and after. docs/HEARTH-LOOK.md (needs testing on a device)

- [ ] Hearth Classic round 3: the bottom bar and the in-game top bar now reach Steam's real menu (Steam draws it in its own window, which the look didn't reach before), are checked on Steam's real menu before they stay, and give way to Steam's menu in Hearth's colours when they don't fit; a Diagnostics section in the panel shows what was found and writes it to hearth-classic.log (Hearth's Logs app). The home, the Library's tabs and the bar look more like the old Hearth Game Mode (Recent games with the newest big, a banner from the focused game, a light picked tab, icon-only dock). docs/HEARTH-LOOK.md (needs testing on a device)

- [ ] Hearth Classic round 2: the home's places open Steam's real pages (Store, Downloads, Media, Friends no longer show empty pages; the Library at worst), the look goes further into the old Hearth Game Mode on Steam's Library, game pages, Quick Access, dialogs and menus, the Steam button's menu becomes Hearth's bottom bar (and Hearth's top bar while a game runs; switches "Hearth bottom bar menu" and "In a game: Hearth top bar"), and focusing a game on the Hearth home plays its Hearth Music theme like Steam's library does. If Steam's menu changes, Steam's own menu stays and the panel says so; docs/HEARTH-LOOK.md (needs testing on a device)

- [ ] Hearth Classic, a new Decky plugin: the old Hearth Game Mode look on top of Steam Game Mode, off until you pick it in Quick Access › Hearth Classic. Hearth's colours, corners, focus glow, top bar, side menu and wallpaper over Steam's screens, and the old Hearth home on Steam's Home (Jump back in, Hearth's places, Continue playing, Favorites, a row per store from Steam and Hearth Sync; every game opens Steam's own page). If a Steam update changes the screens it styles, it pauses by itself, puts Steam's normal look back and says so in the panel ("Hearth Classic paused: Steam changed its layout"); your choice stays saved. Arrives through the plugins channel; docs/HEARTH-LOOK.md (needs testing on a device)

- [ ] Hearth Music picks the right song or none: it only plays a game's own soundtrack (the album named after the whole game title plus "Original Soundtrack"/"OST"/"Original Score"), never a pop song, film score, sequel's or other game's soundtrack, cover or compilation that shares the name ("Control", "Prey", "Inside", "Hades II", "Hollow Knight" for Silksong), with ™/®, years and edition words left out of Steam's names; Deezer by the same rules. Automatic matches from 1.2 are checked again once; songs you chose with *Choose music…* are never touched (docs/HEARTH-MUSIC.md; needs testing on a device)

- [ ] Hearth Music: *Scan all games* checks every installed and non-Steam game again with today's matching (n of N, cancellable, your picks untouched), and *Review music for every game* lists them as No music found, Found automatically and Your picks, each with its song and a ▶ to hear it; *Fix* any of them with *Choose music…* (results heard before you choose), *No music for this game* or *Back to automatic*. Shortcuts named with a store ("Epic: …", "… (GOG)") now find their music, and a better answer plays without restarting Steam (docs/HEARTH-MUSIC.md; needs testing on a device)

## 1.2 (released 2026-11-03)

- [ ] Decky plugin updates without a system update: Hearth Sync, Hearth Link and Hearth Music update from a "plugins" release you publish with your own version number (Actions › Decky plugins release), checked (SHA-256, minimum versions) and put in place by Decky alone, the image's older ones never replacing them, the copy before kept and put back by itself if a new one doesn't load; Settings › Updates › Decky plugins (versions, Check now, Update, Use the built-in plugins, "Update plugins automatically", off until you turn it on) and a Send logs section. New Hearth plugins arrive the same way, without a system update: listed under "New from Hearth" and installed only when you tick "Also install new Hearth plugins" (off until you do), checked and load-checked like the others and removed again if one doesn't load; a plugin is only ever taken away when a release retires it. The plugins now run only on Hearth OS and show an invitation to Hearth OS anywhere else (docs/PLUGIN-UPDATES.md; needs testing on a device)

- [ ] Hearth Music finds a song for your whole library in the background the first time (and with *Scan my library*), so games start their theme right away: installed games first, then the rest and Hearth Sync's shortcuts, politely rate-limited, paused while a game runs, resumed after a restart, with "Matched 143 of 210" in its panel; new installs are matched as they appear and answers refreshed weekly. Deezer's 30-second previews fill in when Apple Music has no good match (streamed, never stored, "Preview from Deezer", *Use Deezer previews* on by default), and *Choose music…* lets you search your soundtracks, Apple Music and Deezer for a game and keep your choice, which always wins and no scan changes. Still never YouTube (docs/HEARTH-MUSIC.md; needs testing on a device)

## 1.1 (released 2026-11-02)

- [x] Install Package no longer sits on "Reading the package…" forever, and settings pages that run a helper no longer hang: Hearth 1.0's Quickshell refused how the helper's environment was set ("Cannot assign QJSValue to QVariantHash"), so the job never started. It's set the way Quickshell expects now (Install Package, Settings pages, sign-in options, Wi-Fi with a password), and a job that still can't start says so instead of waiting.
- [x] No black screen when Steam was never set up: Steam's own first-time "Steam installer" question opened in a window Steam Game Mode doesn't show, so a device that hadn't used Steam yet (an Intel laptop on 1.0) sat on a black screen. Hearth now starts Desktop Mode instead and says to open Steam there once and sign in; Game Mode works from then on.
- [x] Steadier game sound: the image now includes RealtimeKit, so PipeWire gets the sound priority it asks for (without it, busy games like Hogwarts Legacy made the sound choppy on the Nvidia PC), and games through Proton get a 1024-sample buffer floor; Send logs gains a Sound section (RealtimeKit, outputs, buffer sizes, sound messages).
- [x] Game Mode and Steam Game Mode start at a mode the screen really has: a chosen resolution or refresh rate the connected screen doesn't list (the Nvidia PC's 2560 × 1440 at 120 Hz on a TV that has it only at 60 Hz, which made gamescope abort) is skipped for that start with a notice and Try again, session.conf kept as it is; Steam Game Mode retries once at the screen's own mode instead of failing; the ROG Ally's built-in screen gets its real 48–120 Hz range for Steam when gamescope doesn't know the panel (ROG Xbox Ally); Send logs now shows each screen's EDID and its modes with refresh rates (it said "no EDID" for every screen).
- [ ] Hearth Link sync switches in Steam Game Mode's Quick Access, each off until you turn it on: Decky plugins (their settings follow you, never sign-ins; a plugin another device has installs here through Decky's own prompt), saves that follow you (Steam Cloud games already sync through Steam), and Hearth settings (look, theme, keyboard look, Hearth Music, buttons; screen modes, sound, power and network stay per device), newest change wins per key (docs/HEARTH-LINK-PLUGIN.md; needs testing on a device)
- [ ] Hearth Link in Steam Game Mode: a new Decky plugin (shipped next to Hearth Sync) shows your Hearth devices, sends and gets games between them, Hearth Storage (Sync now, download once, saves, update cache, device health) and pairs new devices from Quick Access (docs/HEARTH-LINK-PLUGIN.md; needs testing on a device)
- [x] Settings are always remembered and never change by themselves: every settings file is saved all at once with a backup copy, a damaged one comes back from that copy instead of resetting to the defaults, two programs saving at once both keep their change, and when Game Mode has to drop a screen mode that won't start it tells you and offers it again (docs/SETTINGS-PERSISTENCE.md; needs testing on a device)
- Hearth's old Game Mode is hidden everywhere (it only runs first-time setup): its settings are now in Desktop Mode's Settings (new Remote play, Games & stores, Storage and Steam Game Mode pages; Go back a version in Updates; logs and backup in About) or in their own apps (docs/SETTINGS-MOVE.md) (needs testing on a device)
- Hearth Mods' Play modded starts Skyrim and other Bethesda games through their script extender with the right Proton (it only asked Steam, which ran the plain launcher), logs every step for Send logs, and Add to Steam puts a game's Modded, Hearth Mod Organizer and Mod Organizer 2 entries in Steam with its art. Hearth Mods is also laid out again, game first: a Library of your games with their art, each game's hero (mods on, load order health, last deployed, Play modded), the mod list as a priority ladder with drag or controller moving, conflict badges and a details panel, and Modlists, Downloads, Organizer, Accounts and Settings in a side rail.
- Hearth's own apps follow the desktop's look (Settings › Appearance: Steam-style or Hearth, dark or light, the accent), live while they're open: the Tauri apps, Theater and Hearth Browser through ~/.config/hearth/theme.css / theme.json (hosts/hearth-apptheme), and KDE apps (Hearth Files, KCalc, Konsole, Kate, Okular) through kdeglobals' colour scheme when they're next opened, which also fixes KCalc staying light in Desktop Mode.
- Settings › Handheld: Hearth's own Handheld Daemon app (every HHD setting from its own settings tree, advanced ones behind a switch, notes where Steam takes over in Steam Game Mode, "isn't running" on desktops), also from Quick Access and Settings › Controller.
- On-screen keyboard looks for Mantel: Steam-style (default, follows your accent and dark/light), Hearth warm, High contrast, Frosted and Pocket, next to the first four, picked in Settings › Keyboard.
- Decky Loader built into the image (pinned v3.3.1, checksum-checked at build), started for Steam Game Mode by hearth-decky@<you>.service without a password; earlier downloaded installs move over by themselves.
- [x] New USB installer, install-only, in the Steam-style look: full screen and controller-first (touch, mouse and an on-screen keyboard too), Welcome and screen rotation, Wi-Fi (optional), choose the drive (hold A to erase), you, time zone, install, remove the USB, restart into Steam Game Mode; first-time setup is folded in, so the old Game Mode screens no longer run after installing (needs testing on a device)
- [x] Hearth GRUB theme: the boot menu (still shown as before) in a slate Steam-style look with the Hearth flame, blue highlight and clean names ("Hearth OS 1.1", "... (previous version)"); if anything of the theme is missing GRUB shows its plain menu and starts the same way (needs testing on a device)
- [x] Boot splash presets: Hearth flame on slate (default), Steam-style blue and minimal, chosen in Settings › Boot & startup without rebuilding anything; if a splash can't show, the start goes on in text (needs testing on a device)
- [x] Hearth startup animations for Steam Game Mode: four original ones (flame ignition, slate light sweep, logo reveal, rising embers) for 16:9 and 16:10, picked in Settings › Boot & startup and listed in Steam's own Startup Movie setting (needs testing on a device)
- [x] Apps with their own controller controls work reliably on the desktop: while one is in front, the desktop controls step aside (Guide still opens the Hearth menu, Guide + R3 gives you the pointer there), pop-ups and Quick Access no longer take the controller away from it, and the apps' controller reading no longer stops after a hiccup (docs/CONTROLLER-FOCUS.md) (needs testing on a device)
- Hearth Music: a Decky plugin that plays the highlighted game's theme song in Steam Game Mode (your pick, the game's soundtrack, or Apple Music's 30-second preview), fading out when you press Play (docs/HEARTH-MUSIC.md).
- Steam games from other Steam folders (Flathub Steam's, unregistered SteamLibrary folders) can be brought into Steam from Hearth Sync's panel or with `hearth-steam-import` (docs/HEARTH-SYNC.md).

## Planned

**1.1**

- [ ] A new Steam-style USB installer that only installs (the Recovery USB stays the place to repair and go back)
- [ ] Hearth startup animations, picked in Steam's Startup Movie setting or in Hearth Sync
- [ ] Boot splash presets, switched safely, with a plain text start as the fallback
- [ ] A Hearth GRUB theme; the boot menu stays visible as it is today

**Later**

- [ ] Heroic Games Launcher preinstalled
- [ ] An install helper for Epic's Linux launcher, once an official download is confirmed
- [ ] An app-update signing key, to turn on fast app updates between system updates (docs/APP-UPDATES.md)
- [ ] Hearth Mod Organizer: LOOT masterlist sorting; files for the game's own folder (script extender loaders, ENB:
      MO2's Root Builder); Nexus downloads straight into an instance, mod categories, notes and update checks; FOMOD
      scripts and BAIN wizards (docs/MODS.md)
- [ ] Ember: app pop-up notifications in full Mantel on Windows (Windows only shares them with packaged apps)

## Waiting on setup

- A cosign key for signing the OS images (`SIGNING_SECRET`); then signature checks turned on in Hearth OS
- Support page and donations: built, hidden until the owner fills in a link in ui/support-links.js and
      site/support-links.js

## 1.0 (released 2026-11-02)

Hearth OS 1.0: from now on versions are MAJOR.MINOR (1.0, 1.1 … 1.10, then 2.0), shown with their date, "Hearth OS 1.0 · November 2, 2026" (docs/UPDATES.md).

- [x] Ready for 1.0: version numbers are compared by number everywhere (1.0 is newer than 0.6.63, 1.10 newer than 1.9, "1.0" and "1.0.0" are the same), people see "1.0" for 1.0.0 (Settings, Mantel, hearthfetch, the Decky plugin, release titles and the website), and each release's date from its ROADMAP heading is shown next to it, offline on the device too (needs testing on a device)
- [x] Mantel is now Hearth's only desktop: Hearth Plasma, Hearth GNOME and Hearth COSMIC are gone (Settings no longer offers a desktop choice, and a device that had one picked simply opens Mantel), and Hearth Disks no longer offers the small online installer (the offline USB and the Recovery USB stay)
- [x] Hearth OS is now Debian + Mantel (Desktop Mode) + Steam Game Mode: the device starts in Steam's Game Mode and every Game Mode button opens it; if Steam can't start, Mantel opens with the reason and a "Try Steam Game Mode again" button (never back and forth). Hearth's own Game Mode is archived (first-time setup still runs in Hearth's app once), and multi-seat and Couch Co-op are hidden for now
- [x] Hearth Sync: a small Decky plugin that keeps your non-Steam games (Epic, GOG, Battle.net, Xbox Cloud Gaming, emulators, recomps, Beacon, games you added, plus Minigalaxy, Heroic and the Epic Games Launcher) in Steam's library with their covers, in a collection per store; Hearth OS ships it instead of the Hearth takeover plugin (removed from Decky at the next start) and now comes with Minigalaxy
- [x] Mantel's Steam-style look is the default for new installs and updated PCs (Hearth's warm look stays in Settings › Appearance); the Inter font is in the image
- [x] New website design: hearthos.fun and its docs redone for Hearth OS on Debian with Steam Game Mode, Mantel and Hearth Sync, in a deep blue-slate look with a bright blue accent (Anybody and Figtree, both served from the site), dark and light; what's coming in the next update is marked as such, and what's archived or paused (Hearth's Game Mode, Anatase, multi-seat and couch co-op) is said plainly; downloads are the offline USB and the Recovery USB (the small installer is no longer offered), and Mantel is the only desktop
- [x] Hearth OS is now simply Debian-based: the Peppermint edition is shown as "Debian" ("Hearth OS on Debian", os-release PRETTY_NAME "Hearth OS (Debian 13)") in the installer, Settings, the website and the docs, and it's the default and recommended edition; Anatase is archived in the installer and Switch edition too. The optional Xfce session is off by default. Internal names stay the same (image hearth-peppermint, edition id peppermint, VARIANT_ID), so existing installs keep updating
- [x] Peppermint is the main Hearth OS; Anatase is archived (no more Anatase builds or updates from November 2026, installs keep their last version). The website offers Peppermint only (the Peppermint offline USB and the small installer)
- [x] Steam Game Mode: the Hearth plugin updates with Hearth OS (Decky kept the copy from the day it was installed, so Steam Game Mode still showed an older Hearth plugin, without the newer pages, covers and fixes); it's replaced by the image's own each time Steam Game Mode starts when it differs
- [x] Hearth's open Steam client: when a Steam server answers "TryAnotherCM" (it can't sign anyone in), the next try goes to a different Steam server (switching between TCP and WebSocket) instead of the same kind again, and the log says which server answered

# Archive (before 1.0)

The releases before 1.0, kept as they were. They include editions and features that are archived now (Hearth's own
Game Mode, Anatase, Ubuntu, CachyOS, Bazzite, Hearth Plasma / GNOME / COSMIC, multi-seat, Couch Co-op, Hearth for
Android): see docs/ARCHIVED.md. The plans from before 1.0 are in the git history of this file.

## 0.6.63 (released)

- [x] Steam Game Mode: Steam's games start again (Steam was told to start games on gamescope's second X display, which gamescope was never asked to make, so games never opened; gamescope now makes it, as SteamOS does); the logs now include the game-launch lines of Steam's own logs

## 0.6.62 (released)

- [x] Steam Game Mode, the rest of Hearth's Game Mode (Experimental, needs testing on a device): Quick Resume (put the running game aside, resume, close; it carries on by itself when Steam brings it back) and game streaming (Sunshine on/off, pair Moonlight with its PIN) in Quick Access › Hearth, Hearth Friends (read only) and Hearth Link / Hearth Storage (Sync now, Wake) pages, Friends, Hearth Link and Couch Co-op in Steam's main menu, Hearth's keyboard waits behind its button in Hearth's windows opened from Steam, put aside now pauses Flathub Steam's games too, and the in-game page's button goes on to the game's page instead of itself; docs/STEAM-GAME-MODE.md has the table of what's ported
- [x] Steam Game Mode, multi-seat (Experimental, needs testing on a device): player 1 plays in Steam's Game Mode while the other players keep their own Hearth Game Mode (Steam's gamescope stays on player 1's card, or runs inside the multi-seat host for shared and split screens, falling back to plain Steam Game Mode); Quick Access › Hearth Players › Seats shows each player, their own account and state, starts or stops one extra player and gives a controller to a player, through Hearth's core service. Nothing changes without multi-seat. docs/STEAM-GAME-MODE.md
- [x] Steam Game Mode, couch co-op (Experimental, needs testing on a device): Quick Access › Hearth Players shows the connected controllers in Hearth's player order (players 1-4, remembered by controller), moves them up and down and rumbles one to tell which it is; a split-screen page plays a Hearth Couch Co-op game, or adds it to Steam's library, as one Steam shortcut (`hearth-coop play`: Hearth's split screen, a nested labwc in Steam's gamescope with a nested gamescope per player, each with their own controller from the player order), stopped cleanly when Steam closes it. docs/STEAM-GAME-MODE.md
- [x] Steam Game Mode: Hearth's Store, Mods, Storage and Recomps pages work inside Steam (Experimental, needs testing on a device): buy or claim on the store's own page in Steam's browser, install Epic and GOG games with progress, turn mods on and off, uninstall them or install downloaded ones, move games between drives, uninstall, clean up, use a drive for new games, install and play recomps, all through Hearth's core service and with a controller; Steam's own games stay with Steam's own pages. docs/STEAM-GAME-MODE.md
- [x] Steam Game Mode, Hearth inside Steam's menus, Quick Access and in-game overlay (Experimental, needs testing on a device): Hearth's core runs without a window next to Steam (started and stopped with Steam Game Mode, guarded, no controllers, gamescope focus, pad mouse or Steam control of its own) and answers the Hearth plugin live over a socket only you can open; Steam's main menu gets Hearth Home, Store, Library, Theater, Mods, Storage, Downloads, "Now playing" and Hearth's power options (Switch to Hearth Game Mode, Desktop Mode) in Hearth's look; Quick Access gets Hearth, Hearth Performance (power mode, Hearth's power limit read only, Steam keeps the frame limit and TDP) and Hearth Controller tabs beside Steam's (through Decky's own tab list); the game you're playing gets a Hearth section (its Hearth Input layout, its Proton for Play through Hearth, its mods in Hearth Mods, Hearth's facts), and Hearth Input layouts work in Steam Game Mode for the game in front. Each part has its own switch and turns itself off if Steam changes. docs/STEAM-GAME-MODE.md (needs testing on a device)

## 0.6.61 (released)

- [x] Fix: signing in to Hearth's Steam engine was impossible: the Lightweight engine never started (0.6.59's "Cannot marshal 'parameter #2'") and crashed when it let go. It starts again (its restart now passes its settings along too), exits cleanly, CI checks every native call of the built program, and Settings says why sign-in isn't possible and offers to start the engine again instead of showing nothing (needs testing on a device)
- [x] Hearth's own Steam client (Open) is the default: new installs and everyone who never chose an engine (once, with a short explainer in Settings › Library; turn it off any time). Until you sign in to it everything works like before; Settings and the Library offer the sign-in. While it's signed in, Valve's Steam client isn't kept running: it starts only for games that need it (Steam DRM, VAC, Steamworks) and Hearth's client carries on when it has closed. Steam Game Mode (Decky) still uses Valve's client (needs testing on a device)
- [x] Downloads page for Hearth's Steam client: pause and resume each download or all of them, move them up, down or to the top, cancel, speed and time left, kept across restarts; automatic updates for installed Steam games (Settings › Library: off, when no game is running, or always; never during a game unless always) and a download speed limit (needs testing on a device)
- [x] Steam Cloud saves with Hearth's Steam client: games Hearth starts directly get their newer cloud saves before they start and send changed ones up after they close (in the game's Proton prefix for Windows games); when saves changed both here and in the cloud, Hearth asks (keep this device's, the cloud's, or both) and never deletes anything without asking; games whose saves Hearth can't place start through Valve's Steam client, which syncs them (needs testing on a device)
- [x] Beta branches and Workshop items with Hearth's Steam client: pick a game's beta in its ⋯ menu (password-protected betas ask for their password), it switches in Downloads and is recorded like Steam does; subscribed Workshop items download to where the game and Valve's Steam client look for them, update, and Unsubscribe removes them (needs testing on a device)
- [x] Fix: moving a Peppermint device to Hearth OS on Anatase failed ("composefs: enabled at runtime, but support is not compiled in"): Peppermint's ostree is now built with composefs and SELinux support (without SELinux, Anatase's files came out unlabeled and it started to a black screen), and switching editions first drops the downloads kept from earlier pulls so the new edition is fetched whole; Peppermint's own deployments keep composefs off, so its booting is unchanged (needs testing on a device)
- [x] Settings › Storage, like SteamOS': every drive games can be on (this device, SD cards, Hearth Disks' games drives, USB drives, Steam libraries) with a bar of what fills it (games, Proton prefixes, shader caches, downloads); every installed game by size across all stores, sortable and by drive, its prefix and shader cache shown apart; one-press Move to another drive for Steam (with Steam closed: folder, prefix, shader cache and Workshop, records switched), Epic (legendary), GOG and Recomps, copied, checked byte for byte and only then deleted, with progress, Stop and recovery after a power cut (Battle.net, Xbox PC, Beacon and emulators say where to move them instead); Uninstall from the same list; Clean up (shader caches, prefixes of games that are gone, never one with saves that aren't in Steam Cloud, unused Proton versions, unfinished downloads), each confirmed; a low space warning in notifications and before installs; a Storage section in Send logs. Hidden on Ember (docs/STORAGE.md) (needs testing on a device)
- [x] Steam Game Mode, Hearth takeover (Experimental): with the Hearth Decky plugin, Steam's Game Mode turns into Hearth as far as Decky allows, with Steam's library, downloads and overlay underneath: Hearth's home screen in place of Steam's Home (Jump back in, Continue playing, Favorites, a row per store including Steam, Recently added), Hearth Store (Steam, Epic, GOG, free games, Xbox, Battle.net, Recomps), Hearth Library with a tab per store, Downloads and Storage as pages of their own, Hearth's stores as tabs in Steam's Library, ProtonDB and achievements from Hearth on game pages, Hearth's places in Steam's main menu, the power mode and Switch to Hearth Game Mode in Quick Access. One "Hearth takeover" switch (on by default) and one per part; a part Steam's update breaks turns itself off and Steam's own screen shows. docs/STEAM-GAME-MODE.md (needs testing on a device)
- [x] Fix: Steam Game Mode showed no installed Steam games when both the system Steam and Flathub's Steam are installed and Flathub's is the one in use: it now starts the Steam whose folder has the sign-in (needs testing on a device)
- [x] Fix: the "From Hearth" row on Steam's home in Steam Game Mode showed blank dark-blue cards with only a store badge. The covers were there, but Hearth's look over Steam paints every Steam button's background, which hid them. Cards now show the picture itself, always with the game's title at the bottom; a picture that doesn't load gives way to the next one (portrait cover, then Steam's own picture for Steam games, then the wide art cropped) or to a title card, never a blank card. Every kind of picture Hearth keeps works (Hearth's picture cache, webp, avif, gif, your own pictures), on Hearth's home, pages and Library tabs too, and the art given to synced Steam shortcuts is turned into PNG when Steam can't take it. Pictures that can't be shown are counted in the plugin's log, and Send logs has a Steam Game Mode (Decky plugin) section (needs testing on a device)
- [x] Fix: Hearth's games added to Steam's library didn't start in Steam Game Mode: Steam started them with its own runtime's libraries, which broke Hearth's program. Each shortcut now cleans Steam's environment first (Steam's overlay still works over the game), works with the system's Steam and Flathub's alike (decided at every launch), and shortcuts synced before are fixed by themselves when the plugin starts; every start and failure is in Hearth's log (needs testing on a device)
- [x] Update Hearth OS from Steam Game Mode: Steam's Settings › System › Check for updates and Quick Access › Hearth › Hearth OS updates both run Hearth's own update with progress; the device never restarts by itself, and the version before stays installed (needs testing on a device)
- [x] Start in Steam Game Mode (Experimental, needs testing on a device): Settings › System › Steam Game Mode › **Start in Steam Game Mode** (off by default, per device, offered only with Steam installed and signed in) starts the device in Steam's Game Mode instead of Hearth's. Ways back with only a controller: Quick Access › Hearth › Switch to Hearth Game Mode or the new **Start in Steam Game Mode** switch there, Steam's Power › Switch to Desktop, holding B or Guide while the device starts (skips it once), Hearth Recovery's Desktop Mode or Safe graphics this time; with a keyboard, `hearth.mode=game` on the kernel command line. It turns itself off (with a note in Settings) when Steam or gamescope is missing, safe graphics are on, Steam isn't signed in, Steam fails within 20 seconds, or Steam Game Mode closes by itself twice within 10 minutes. Never for extra multi-seat players or the live USB / installer. Hearth's Game Mode stays the default.
- [x] Hearth app updates without a system update: Hearth's own app and screens (the Hearth program and its Chromium window app) update by themselves in minutes, between Hearth OS system updates, from signed bundles on each release; the version built into the system is never touched and always stays as the fallback. Hearth only starts an update that's signed, made for this edition and this system's app ABI, newer than the built-in version and unchanged; two Game Mode starts with it that don't bring the screen up (a crash, a black screen, an exit) set it aside and the built-in version starts, with a note in Settings. Settings › Updates › Hearth app updates (Automatic, Ask, Off), Use the built-in version, hearth.app=builtin at boot, and a Hearth app updates section in Send logs. Publishing waits for the signing key secret (docs/APP-UPDATES.md) (needs testing on a device)

## 0.6.60 (released)

- [x] Steam games start faster: "Keep Steam ready in the background" (Settings › Library, on by default) starts Steam hidden when Game Mode comes up, if you've used it here before, and keeps it running, so Play hands the game straight to Steam instead of waiting for Steam to start (needs testing on a device)
- [x] Steam's pop-ups while a Steam game starts or runs (cloud-save conflict, EULA, errors, install steps) come in front of Hearth's starting screen instead of hiding behind it, and the controller mouse turns on by itself over them and over a game's own launcher (Bethesda's, for one): right stick moves the pointer, A clicks, View + Menu turns it off (needs testing on a device)
- [x] The starting screen says what Steam is doing ("Starting Steam…", "Steam is updating…", "Steam is asking something…", "Starting the game…") instead of disappearing after 25 seconds, and after 2½ minutes offers Keep waiting or Back to Hearth (needs testing on a device)
- [x] Fix: the keyboard over a game (Guide + X) didn't type into the game. The first Guide + X after Quick Access had been open closed itself right away, and with Handheld Daemon (Anatase) the keys went to Quick Access's window instead of the game. It now stays open, the controller drives it, letters, Enter, Backspace and the arrows reach the game (native and Proton), and B or Guide closes it and gives the controller back. Each typed batch is in the log with how it was sent (needs testing on a device)
- [x] Battle.net is set up through Faugus Launcher (preinstalled on Anatase and Peppermint): one press, Blizzard games in the Library and Play through Battle.net as before; Battle.net set up before keeps working (needs testing on a device)
- [x] Fix: the controller did nothing in Hearth Plasma, Hearth GNOME and Hearth COSMIC (Hearth Classic was fine). Plasma and COSMIC never started their controller helper (their autostart folder isn't read on a real sign-in); the session now starts it itself once the desktop is up, once, only in Hearth's desktops. In Hearth GNOME the session mode is handed to GNOME Shell's service, and if Hearth's controller extension still doesn't start, the pointer, clicks and keys work anyway and the reason is logged. Send logs now include plasma.log, gnome.log, cosmic.log and whether Quickshell is installed (needs testing on a device)
- [x] Steam Game Mode (Experimental): Steam's own gamepad screens in gamescope, switched to for one time only from the power menu or Settings › System (always back to Hearth's Game Mode afterwards, and at every start); optional Decky Loader install with the Hearth plugin: Hearth's look over Steam, a Hearth row on Steam's home, your other stores' games with Play, Sync to Steam library (they play through Hearth), Hearth Store, Mods and Theater, and Switch to Hearth Game Mode; docs/STEAM-GAME-MODE.md (needs testing on a device)

## 0.6.59 (released)

- [x] Hearth GNOME (both editions) and Hearth COSMIC (Anatase only) as new Desktop Mode desktops, in Hearth's look with Hearth's launcher, Quick Settings/Quick Access, Hearth Link, Hearth Storage, Game Mode button and on-screen keyboard; full controller control; falls back to Hearth Classic if they can't start — needs testing on a device
- [x] Hearth Deepin listed as not available (neither Fedora nor Debian ships the Deepin desktop)
- [x] One app of each kind per desktop: each desktop shows its own Files, Terminal and Text Editor; apps with no other of their kind show everywhere
- [x] Hearth Plasma: full controller control, everything Hearth Classic does — needs testing on a device
- [x] Peppermint writable by default with a lock switch in Settings; boot-safety guards (safe start without the changes, interrupted installs repaired, boot menu written from the image, core packages held) — needs testing on a device
- [x] Hearth Input overhaul: Guide + X on-screen keyboard over any game and in every desktop, Steam Input-style layouts (action sets, layers, chords, turbo, flick stick, gyro), edited with the controller — needs testing on a device
- [x] Fix: Steam games freezing when Quick Access opens over them (HHD editions)
- [x] Hearth Mods: Accounts (Nexus Mods, LoversLab, MEGA, Google), automatic MEGA downloads and a guided flow for the rest; Hearth Mod Organizer (MO2-compatible, native); mods managed and installed from Game Mode
- [x] Hearth Theater: each site's shows, movies and videos on Theater's home screen, with Search across all sites
- [x] New website design

## 0.6.58 (released)

- [x] Desktop selector: Hearth Classic, Hearth Plasma (Hearth GNOME later) in Settings and at sign-in; falls back to Hearth Classic by itself when Plasma can't start — needs testing on a device
- [x] Hearth Plasma (first version): KDE Plasma 6 in Hearth's look with Hearth Launcher, Quick Access, Hearth Link, Hearth Storage and Game Mode widgets; not yet a one-to-one match with Hearth Classic
- [x] Hearth Files built from KDE Dolphin, and Hearth Storage (hearth:/) in every KDE app's file dialogs; KDE-based Terminal, Text Editor, Reader, Calculator and Capture (Capture on Anatase only for now); the old apps kept as "(classic)"
- [x] Hearth Link: Windows (SMB) sharing per folder, wake devices over the network, Continue on this device, update every device at once, local AI shared with paired devices
- [x] Ventoy: several ISOs on one USB stick in Hearth Disks — needs testing on a real stick
- [x] Browser: move around web pages with the D-pad
- [x] Discord status and Support page built, hidden until set up
- [x] Qt renderer: more of phase 2 (notifications, Wi-Fi/Bluetooth, game settings, achievements, Library filters)

## 0.6.57 (released)

- [x] Automatic fallback: two starts in a row that never reach Hearth, and the previous version starts by itself (and stays the default); background updates pause for three days after
- [x] Hearth Recovery on the device: hold Volume Down (F8, or Select + Start) while starting; opens by itself after a failed start; controller, volume/power buttons or keyboard
- [x] Hearth Recovery USB: every action works (reinstall keeping files keeps the old versions, factory reset never erases other systems, copy files off, send logs, Desktop Mode)
- [x] Peppermint keeps the system log across restarts; Wi-Fi power saving off (ROG Ally drops); .deb installs no longer stuck on "Reading the package…"
- [x] Battle.net (one press, Blizzard games in the Library, Store section) and Xodus Nexus (preinstalled, Xbox PC tab, Stores card, pinned) — needs testing on a device
- [x] Multi-seat game sharing: player 1's GOG games on every seat, shared Epic installs, Steam Family per seat with "In use by…"
- [x] Hearth Storage: a spare Hearth PC as a home server — shared folders, game archive, backups, saves follow you, media, download once for every device, Hearth update cache with hold-back, device health, removing old devices, one new settings page
- [x] Dragonfire look (Settings › Appearance › Look): its own Game Mode Home, performance rings, friends, Updates & Recovery page
- [x] Steam engines (experimental, off by default): Lightweight (Valve's engine without Steam's window) and Hearth's own open Steam client
- [x] Hearth Benchmark: processor, memory, storage, graphics, sustained/heat, battery and network tests with history and device compare

## 0.6.56 (released)

- [x] Quick Access over Steam games no longer puts the game aside by itself (black or frozen games on the ROG Ally)
- [x] Steam is never started twice and its window is asked for once at a time (several Steam windows and a sluggish game on the Nvidia PC)
- [x] Switching to Desktop Mode closes paused / put-aside games first (they froze the system), with a safety net in the session
- [x] Settings › You › Appearance › Quick Access style: Top dock (new) or Side panel (classic)

## 0.6.55 (released)

- [x] Peppermint: `sudo hearth-unlock` unlocks the system (apt installs and changes anything, kept across restarts) while keeping updates and rollback; `sudo hearth-unlock --reapply` after an update reinstalls your packages
- [x] Hearth Recovery listed in the USB maker

## 0.6.54 (released)

- [x] Peppermint: the boot menu points each entry at the ostree boot folder that really exists (after 0.6.53 a stale path left the ROG Ally at a black screen)

## 0.6.53 (released)

- [x] Make a bootable USB (Hearth Disks): pick a Linux distro and Hearth downloads it from the project's own servers, checks its checksum and signature, writes it and reads it back; Windows 10/11 from Microsoft's page (with optional Windows 11 setup tweaks); or your own ISO/image. Only removable drives are offered; Hearth's own drive never is
- [x] The power-button press that wakes the device no longer puts it back to sleep (ROG Ally in Desktop Mode); one power-button handler per mode, presses right after waking are ignored, and Send logs shows who asked for each sleep

## 0.6.52 (released)

- [x] The Bottom Dock is Game Mode's main menu: live cards above each item (continue playing, friends online, downloads with progress, nearby devices), badges, a game dock over a running game, and search that finds settings too; the classic side menu stays under Settings › You › Appearance › Menu style
- [x] Settings redesigned: six groups (You, Connections, Display & sound, Controls & power, Games, System) with live status cards, pages as tabs (LB/RB), a new Help & logs page; every earlier setting is still there
- [x] Go back to the previous version from Settings › System › Updates (Anatase)
- [x] The in-game overlay is a top dock: Now playing, Performance (temperatures, power draw, per-game saving), Controller layout, Friends (quick replies), Capture and Quick settings; a short Guide tap shows a quick-glance bar without pausing the game
- [x] The overlay that Handheld Daemon starts gets a font setup Chromium can read (for the blank overlay on Nvidia PCs) and its own log section
- [x] Qt renderer (Experimental): Settings › Display › Renderer › Qt — Home, Library, game pages, Downloads, Quick Access, Power and Settings drawn natively with Qt Quick; falls back to Automatic if it fails
- [x] On-screen keyboard rumble always stops (ROG Ally kept rumbling)

## 0.6.51 (released)

- [x] The controller works the on-screen keyboard in Game Mode again (search, Send logs and every other prompt); the controller is now tested through the real gamepad path
- [x] Hearth Link pairing without long links: nearby devices are listed by themselves (name, kind, edition) and a 6-digit code pairs them; different networks use a short code like K7M-4QX-92HC (typos caught at once, 10 minutes, single use); wrong codes can be retried, 5 wrong codes stop pairing
- [x] Hearth Friends: sign up or sign in with a QR code on your phone, or in the browser, without retyping (matrix.org and other servers with Matrix's newer sign-in); a recovery key for encrypted history
- [x] hearth-input ends cleanly when the session stops; repeating controller errors no longer flood the log

## 0.6.5 (released)

- [x] Hearth Link: pair your Hearth devices, copy games between them fast on your network or over the internet, and sync your Hearth data
- [x] Hearth Friends on Matrix: friends, encrypted chats, "Playing X · Joinable", invite-to-play cards, join via Steam or a server address; in Game Mode, Quick Access and Desktop Mode's bar; Play together (Beta Experimental, same network)
- [x] A Store in Game Mode with Steam, GOG, Epic and Xbox sections; store sign-ins shared with the library
- [x] One new on-screen keyboard everywhere (Ember Wheel, split sticks, word suggestions, layouts, emoji, themes)
- [x] Hearth Mods: Nexus Mods downloads (Mod Manager Download links), Wabbajack modlists, links/copies/overlay installs
- [x] Bazaar is the app store on both editions; Hearth's old app store is archived
- [x] Steam owned games come from this device's Steam files when Steam's site needs a signed-in visitor; file pickers no longer freeze Hearth
- [x] Missing store sign-ins come back from the device, with private sign-in backups
- [x] Real sleep on the power button; the built-in controller no longer wakes handhelds; a Sleep section in Send logs
- [x] Couch Co-op and Multi-seat: Bluetooth LE controllers (Xbox Wireless) can be claimed, co-op players' screens start, and a failed start says why
- [x] hearthos.fun redesigned, with open-source game art in the screenshots

## 0.6.4 (released)

- [x] Photos, Documents, Calculator, Player, Write, Sheets, Slides, Terminal and Beacon no longer close as they open on Nvidia PCs
- [x] The in-game Quick Access overlay comes back right away after Game Mode restarts or a trip to Desktop Mode (handhelds with Handheld Daemon) and opens faster; Hearth's own Quick Access panel is now the same as the overlay one
- [x] Steam games show again: Hearth finds Steam in ~/.steam/debian-installation and other layouts and finds the signed-in account reliably; Library › Stores says why when it can't
- [x] Hearth Files is the default file manager and the Open/Save window in other apps (Flatpaks, Firefox, Chromium/Electron, GTK, Qt, Hearth's apps)
- [x] Hearth's apps are the defaults for everything they open; Settings › Default apps changes them, in Game Mode and Desktop Mode
- [x] No on-screen keyboard after typing on a keyboard or using a mouse; Settings › Appearance › Made for (Automatic, Mouse & keyboard, Controller)
- [x] Three keyboard & mouse layouts (Desk grid, Table, Launcher) for Home and the Library; the 24 experimental layouts fixed and each set in its own drop-down
- [x] New apps: Hearth Recorder (screen, sound, instant replay), Camera and Share (your screen in any browser on the network)
- [x] Peppermint: .deb files install through a Debian box, AppImages run and go in the menu, the app store and Bazaar work, a broken system Flatpak setup is repaired at start-up, no hostname warning, and password prompts appear in both modes
- [x] Sign-in pages open in your default browser (Hearth Browser) instead of Chrome
- [x] Games added in Beacon 0.0.18 (Faugus-style, with ProtonPlus-style compatibility tools) show under Beacon › Other Games
- [x] Settings › Display explains HDMI 2.1 / 144 Hz problems, and Send logs includes the screen's modes and the Nvidia driver's HDMI link

## 0.6.3 (released)

- [x] Send logs includes the screen's full list of modes, what it says it can do (EDID) and the Nvidia driver's HDMI link messages, to find out why a 120/144 Hz TV only shows 60 Hz at 4K
- [x] Settings › Display's 60 Hz tip adds the usual workaround: switch the screen off and on with its own power button while Hearth is showing

## 0.6.2 (released)

- [x] The new Quick Access overlay opens over games and while a game is loading on Nvidia PCs (Chromium drawing engine): the old full-window panel no longer appears, and the game isn't pulled back in front while Quick Access is open
- [x] Settings › Display explains when a TV or monitor only reports 60 Hz at your resolution, and what usually fixes it

## 0.6.1 (released)

- [x] New Hearth apps on Anatase and Peppermint, each with its own icon and opening with a controller, keyboard or mouse: Hearth Text (text and code editor; Files now opens scripts, .desktop files, configs and logs sensibly), Photos and Documents (PDF, e-books, comics), Player (video, music, podcasts and radio, with trimming and other edits saved as a copy) and Calculator, Clock (alarms that ring with the app closed, timers, stopwatch, world clock), Calendar, Notes and Weather, Office (Write, Sheets and Slides: .docx, .xlsx, .pptx and more), Paint, Emoji & symbols (Super+.), Fonts, Contacts (vCard, optional CardDAV sync) and Maps (OpenStreetMap), Nearby Share (LocalSend-compatible) and Hearth Connect (KDE Connect-compatible phone link), Controller Lab, Saves and Mods, and System tools (Disk Usage, System Info, Logs, Network Tools, Passwords & Keys, Printers & Scanner, Welcome & Tips)
- [x] Hearth's own icon set for every Hearth app, and Flatpak apps show their icons in the app menu
- [x] Settings › Accessibility: magnifier, colour filters (Game Mode), screen reader (Orca), text size, high contrast and a large pointer
- [x] Full codecs for playback and editing (H.264, H.265, AV1 and more) on both editions
- [x] Peppermint: joining Wi-Fi no longer says "insufficient privileges"

## 0.6.0 (released)

- [x] Claude on Hearth OS: Get Claude (the desktop app, from Anthropic, kept up to date) and Get Claude Code, from the app menu and Settings, on Anatase and Peppermint
- [x] Hearth Files is a full file manager: a complete Properties window (sizes counted live, dates, permissions, opens with, checksums), tabs, bookmarks, a path bar you can type in, icon/compact/detail views, drag and drop, Replace / Skip / Keep both, templates, batch rename, compress and extract (.zip, .7z, .tar, .rar and more), search, open a terminal here (Linux and Windows)
- [x] Desktop Mode works like a normal desktop: Super+D and the usual shortcuts, screenshots of the screen or an area (saved, copied, with a notification), a taskbar with pin dragging, app actions, show desktop and a notification count, desktop icons with multi-select, cut/copy/paste and drag and drop, Mouse & touchpad settings, screen-off and sleep timers, several screens, and apps set to start automatically really start; Ember's Mantel gets the same where Windows allows
- [x] App menu: the apps that came with the system are in one System apps folder; Hearth's own and yours come first
- [x] Ubuntu and CachyOS editions archived for now, to focus on Anatase and Peppermint; systems on them can switch from Settings › Updates
- [x] Ember (Windows) and the one-line installer find new versions again (they skipped everything numbered 0.3 to 0.5)

## 0.5.1 (released)

- [x] Quick Resume keeps your games through Restart and Shut down, not just sleep: one switch, the power menu says which actions keep your games, and "Shut down and close games" / "Restart and close games" really close them; Settings says plainly why if this PC can't keep them yet, with Set up right there (Linux; on Windows, Shut down keeps them)
- [x] Quick Access over games is always the new overlay: also while a game or Steam is starting, and while Handheld Daemon restarts it (no more falling back to the older Quick Access); Ember uses the same new overlay, Open apps included
- [x] Updates: an app (Flatpak) that can't be updated no longer marks the whole system update as failed; it still says "Restart to finish updating"
- [x] Remove the old system (after moving from Windows or another Linux): Hearth now gets the freed space, and another Linux's swap partition can be removed
- [x] First start after switching edition: your account always gets its groups (video, logs)
- [x] Epic and Xbox sign-in: the controller no longer clicks random things on the sign-in page (the page reacted to the controller by itself, alongside Hearth's pointer)
- [x] Anatase: paired Bluetooth controllers connect again when their pairing files had lost their security labels (fixed by itself at every start)

## 0.5.0 (released)

- [x] Steam like GOG and Epic: owned games show with Install, and Steam games install and uninstall from Hearth without Steam's windows (progress in Downloads); Steam starts only when it's needed and closes itself after 10 idle minutes; signing in is Steam's own window (Linux and Windows)
- [x] Screen modes: a refresh rate the screen won't take falls back to the screen's own mode instead of going back to the sign-in screen; a second change during the 20-second check goes back to the right mode; 4096×2160 isn't offered on TVs; Hearth and Desktop Mode keep each other's monitor settings
- [x] Installer: checks for internet before touching the drive; erasing a drive with Linux on LVM works; reserved user names refused; the room shown is for the Windows on the chosen drive
- [x] Hearth Disks: never offers to format the partitions the system runs from (LVM, RAID, encrypted); refuses if the drive was swapped after you confirmed
- [x] Hearth Terminal: a big paste no longer freezes the window; shells that end are closed
- [x] Game Mode: the game case shelf works with a controller, "Played today" stays right, parental limits apply to Search and Hidden, switches that fail go back, the lock screen clock keeps time, About names your edition
- [x] ROG Ally: plugged in at the charge limit shows as plugged in; the power limit comes back after sleep
- [x] Multi-seat: extra players keep their screen and card, can't change the whole PC's power settings, and get only the actions that work for them; "Restart Game Mode" restarts it instead of signing out
- [x] Desktop Mode: Bluetooth page doesn't hang without Bluetooth, copying to USB sticks with links works, screenshots with an apostrophe in the folder name and two in the same second are saved, the clipboard history survives odd text, Wi-Fi networks you renamed are still saved
- [x] Nvidia PCs (Chromium window): the UI size you set is used at start, a second store sign-in doesn't cancel itself, a crash restarts Hearth
- [x] Ember (Windows): rolling back an update keeps the old version, paused games unfreeze, low memory closes one game at a time, unplugging the controller doesn't leave keys held, play-time limits count right, each profile keeps its own GOG account, the taskbar comes back after a crash, "Switch to Hearth OS" leaves Windows the room it says
- [x] Safer and sturdier: store sign-in tokens and the Sunshine password are no longer visible to other users on the PC; app updates without a checksum are refused; profile and Steam account files survive a power cut; a clock set back no longer ends a limited profile's play time; damaged Steam files don't crash Hearth; Quick Access no longer stalls on Nvidia PCs
- [x] Images: Steam is built into Anatase again (it was left out and downloaded after first boot); Sunshine (Remote play, streamed players) is on Peppermint; CachyOS builds again with the newest pacman

## 0.4.0 (released)

- [x] Hearth Browser is real Firefox: Google sign-in, streaming video (DRM) and uBlock Origin work, Hearth's look, big buttons and an on-screen keyboard in Game Mode (Linux and Windows)
- [x] Hearth Terminal: Hearth's own terminal with tabs, split view, find, themes and a quick-keys bar for controllers (Linux and Windows)
- [x] Hearth Disks: drives and their health, format USB drives and SD cards, partitions, Use for games (Linux and Windows)
- [x] hearthfetch: Hearth's own system summary for the terminal (Linux and Windows)
- [x] App drawer: a Games tab with your installed games (Desktop Mode and Ember), and a Windows tab with the built-in programs (Ember)
- [x] Hearth Input (controller layouts): the controller is taken over again after Handheld Daemon or InputPlumber re-make it (it did nothing before), and Bluetooth Xbox / DirectInput pads' right stick works; its activity is in the logs
- [x] Switch edition and updates without podman: it's unpacked for that run when apt can't install it
- [x] Logs: Nvidia's numbers only on devices with Nvidia's driver running
- [x] Display: resolution and refresh rate really change. Game Mode offers only the modes the screen has, applies a new one when it restarts, and switches back by itself after 20 seconds unless you keep it (a mode the screen can't show no longer leaves it stuck); the built-in screen's refresh rate changes on the spot. Desktop Mode asks for the screen's exact rate and says when the screen refuses
- [x] Controller mouse with an Xbox controller over Bluetooth (Desktop Mode and Game Mode): the right stick moves the pointer again (it never moved)
- [x] CachyOS: Game Mode no longer crashes over and over (GTK's SVG icon loader was missing); a multi-seat host that crashes on start falls back to normal Game Mode after three tries

## 0.3.1 (released)

- [x] Switch edition and updates on Peppermint, Ubuntu and CachyOS: podman in the images (bootc needs it); systems without it get it for that run
- [x] Brightness on Peppermint and Ubuntu (accounts get write access at start-up; Game Mode falls back to logind)
- [x] Game Mode sounds keep working (the sound engine recovers when it's suspended or stalls)
- [x] Switch edition: its messages and errors go in the logs

## 0.3.0 (released)

- [x] Game Mode keyboard: the selected key lights up, like SteamOS's
- [x] Power button: does what Settings › Power button says (Sleep no longer shuts down: the system leaves the button to Hearth while it runs)
- [x] Anatase: the Hearth boot splash instead of start-up and shut-down text

## 0.2.10 (released)

- [x] Steam's Game Mode taken out: Game Mode is always Hearth's (the old choice is cleared at the next start)
- [x] Game Mode keyboard laid out like SteamOS's: docked at the bottom, a caret you can move, shift and caps, &123 symbols; Y delete, X space, LB shift, RB symbols, Menu done

## 0.2.9 (released)

- [x] Steam's Game Mode (Experimental): the Flatpak may reach Wi-Fi, power, Bluetooth and SteamOS Manager; one that didn't end cleanly falls back to Hearth's at the next start; set back to Hearth's once after 0.2.8

## 0.2.8 (released)

- [x] Game Mode: Hearth's own or Steam's own Game Mode (Big Picture), in Settings › System and Desktop Mode's Settings › Power
- [x] Installer fixes: the firmware boot entry, the account (written directly), CachyOS without SELinux; every ISO tested end to end (install, then start) before it's uploaded
- [x] Steam games: every installed game shows again with one profile

## 0.2.7 (released)

- [x] Hearth on Nvidia through Chromium: the same Hearth core with an Electron (Chromium) window on Nvidia graphics, chosen at start (WebKit's DMA-BUF drawing fails on RTX 50 cards); Settings › Display › Drawing engine; back to WebKit after two failed starts
- [x] Installer: its messages in Hearth's log, and the installer's logs in bug reports sent from the USB

## 0.2.6 (released)

- [x] Bug reports: what the desktop controller helper receives (and whether another program holds the controller); processor and Nvidia graphics load

## 0.2.5 (released)

- [x] Switch edition (Settings › Updates): from one edition to another (Anatase, Ubuntu, CachyOS, Peppermint), keeping your games
- [x] Multi-seat: no performance overlay (it crashed over and over reading the other players' processes)
- [x] Offline installers for Ubuntu, CachyOS and Peppermint (the small installer plus that edition)

## 0.2.4 (released)

- [x] Multi-seat: every player's controller works in Hearth whichever window is active (Hearth reads it itself)
- [x] Multi-seat split screen: no title bars, each player's window placed in its part even where labwc's sandbox rules don't match
- [x] Bug reports: crash dumps shortened so Hearth's log always fits; a Multi-seat section (set-up, host log, window rules)

## 0.2.3 (released)

- [x] BFME games installed by the All-in-One launcher start (they run in the launcher's Wine setup, with its registry and CD key; they sat on the splash screen before) (Beacon 0.0.8)
- [x] Install with the All-in-One launcher: Beacon opens it, waits while you install the game there, closes it and adds the game
- [x] Game Mode shows the windows Beacon's installers and launchers open
- [x] Controller mouse in Game Mode (Quick Access › Controller mouse; View + Menu turns it off)
- [x] Profiles keep their own Steam account, and each sees only its account's Steam games

## 0.2.2 (released)

- [x] Hearth for Android (Beta Experimental): Android games and emulators in Hearth's library, RetroArch ROMs, controller, home app; the APK on every release
- [x] Hearth Linux on Android, stage 1: Debian arm64 under PRoot (Settings › System › Linux), apps shown through Termux:X11 (allowed once)
- [x] Ember installs Beacon (Library › Stores › Get Beacon, and in Ember's installer)

## 0.2.1 (released)

- [x] Fix: the selection jumping back and forth when moving with the controller (a touchscreen, Handheld Daemon's virtual mouse or a still pointer took it back)
- [x] Fix: Epic game art with spaces in its address didn't download
- [x] Ember finds Beacon where Beacon's Windows installer puts it
- [x] Desktop Mode's controller helper keeps a note of what it sees (in bug reports)

## 0.2.0 (released)

- [x] Library layouts: Covers, List & details, Shelves, Compact wall, plus 12 Beta Experimental ones (Settings › You › Library)
- [x] Beacon inside Hearth: Game setup (install, locate, CD key, runner, resolution, community versions, patches, logs) and Mods pages; Beacon 0.0.6 with a Windows version
- [x] Fail-safe: a stuck Hearth frees itself, sends a report and restarts if it has to; `hearth-send-logs` from a terminal
- [x] Lighter effects when drawing the compatible way (Nvidia RTX 50); graphics memory shown right (built-in and Nvidia)
- [x] Multi-seat: streamed seats and split-screen seats
- [x] Sign in again when Epic, GOG or Xbox sign-ins expire
- [x] Couch Co-op: the Steam emulator follows its newest release when the pinned one is gone
- [x] Bug reports reach GitHub (the copy goes up as plain text; dashboard settings kept on deploy)
- [x] Ember: Game Mode catch-up (Hearth Input, power limit, bug reports, Beacon, screenshots) and Mantel matching Hearth OS's, with pinning
- [x] Offline USB: boot test before upload

## 0.1.9 (released)

- [x] Fix: a blank screen with only the pointer where the graphics driver can't give Hearth its fast drawing buffers (GBM): Hearth starts again the compatible way and remembers it; basic graphics uses it from the start

## 0.1.8 (released)

- [x] Fix: Game Mode freezing for good while a GOG or Epic game was in the download queue (the Downloads list waited on itself), every time it started

## 0.1.7 (released)

- [x] Fix: Game Mode answering "The system didn't respond in time" to everything (it waited on Beacon's program, and on Handheld Daemon when that stopped answering)

## 0.1.6 (released)

- [x] Beacon in Hearth: its games in the library with a mod picker and a Stores card, Strategy (RTS) controller layout, preinstalled in the images, pinned in Mantel; Beacon's own AppImage/.deb/.rpm releases
- [x] Quick Access › Performance uses Handheld Daemon's power settings, per game; Hearth never writes the limit itself under HHD
- [x] Fixes: Quick Resume flicker (HHD rewrote gamescope's list), games that wouldn't close (zombie launchers), Game Mode's pointer and keyboard in Desktop Mode sign-ins
- [x] More compact Settings (Display & sound in one tab); website redesign

## 0.1.5 (released)

- [x] Hearth Input (Beta Experimental): per-game controller layouts that act as a keyboard and mouse (templates, editor, tap/hold, second set, gyro, paddles, radial menu)
- [x] Quick Resume: every game gets a number (STEAM_GAME on its windows, HEARTH_GAME_ID on its processes) and gamescope is switched to Steam's focus strategy, so the picked game comes to the front
- [x] Mantel overhaul: pinned apps, snapping, Alt+Tab, 4 desktops, desktop icons, calendar and notification history, tray; Settings for date and time, keyboard, default apps, users, printers; Apps store (Flathub); clipboard history
- [x] Every Handheld Daemon setting inside Hearth; Quick Access over games through HHD's overlay (Chromium)
- [x] Store sign-ins through Desktop Mode; Steam runs hidden in the background; Quick Access while a game starts; Couch Co-op in the main menu

## 0.1.4 (released)

- [x] Hearth OS on Bazzite is archived: 0.1.3 was its last update (os.yml only runs by hand now). Releases publish Hearth OS on Anatase (ghcr.io/dragongamer9/hearth-os:latest); the Ubuntu, CachyOS and Peppermint editions get every release as :testing
- [x] Quick Access over games through Handheld Daemon's overlay (HHD editions): drawn by Chromium, the game doesn't see the controller while it's open
- [x] ISOs on Anatase: one offline live ISO for every graphics card (9.4 GB); the small installer (1.6 GB) picks the edition

## 0.1.3 (released)

- [x] Move to a new Hearth OS (Settings › Updates, on the Bazzite-based Hearth OS): choose Hearth OS on Anatase (recommended) or Ubuntu, CachyOS or Peppermint (Beta Experimental); each is offered once it's published. A clean install that keeps the games, their saves and Steam/Epic/GOG; the rest of home is put aside for 30 days
- [x] Handheld Daemon support (inert where HHD isn't installed): Guide button, power limits, controller settings
- [x] hearth-admin works on systems without rpm-ostree (bootc-only editions)

## 0.1.2 (released)

- [x] Starting up next to another system: Hearth gets its own EFI partition in its own space (the other system can't overwrite or take over its boot loader), its boot entry points there and goes first, and Hearth keeps itself first at every start (opt out: /etc/hearth/keep-boot-order)
- [x] Nvidia: Hearth draws with WebKit's fast path on drivers 560+ (Game Mode was sluggish), with Fast/Compatible in Settings › Display in both modes; Quick Access shows Nvidia GPU load; the mouse pointer shows and moves in Desktop Mode
- [x] Delete game in Game settings and the game's options (Epic and GOG by Hearth, Steam through Steam)
- [x] Move games and Make Hearth bigger (Beta Experimental): move games from Windows/SteamOS/another Linux (copied, checked byte for byte, then deleted there), then give the freed space to Hearth
- [x] Full ISOs install offline (they carry Hearth OS); the small installer downloads it; ISOs on Cloudflare R2

## 0.1.1 (released)

- [x] Couch Co-op (Beta Experimental): split-screen for 2-4 players on one screen, each with their own controller and copy of the game (they find each other like on a LAN; a local Steam stand-in for Steam games); each player presses A to join; games with anti-cheat refused with a plain message
- [x] Multi-seat (Beta Experimental): a separate Game Mode per screen or TV, each with its own account, controllers, Steam and saves; a graphics card per player, or one card driving several screens; Remove multi-seat undoes everything
- [x] Fix: Quick Resume showed another game's frozen picture; only the last game put aside came back properly
- [x] Fix: the power limit didn't change on the ROG Xbox Ally (set through ASUS's firmware attributes now, and a change that didn't take isn't reported as done)
- [x] Wi-Fi: a short list (your network, saved ones, the strongest) with Show all networks; Desktop Mode's Wi-Fi and Bluetooth pages no longer lose the selection (the controller got stuck) when their lists refresh
- [x] Fix: Xbox Cloud Gaming's Microsoft sign-in clicked links while you typed with the controller
- [x] Fix (Ember): Mantel's Hearth menu, Quick Access and launcher opened and closed at once with a controller

## 0.1.0 (released)

- [x] Fix: the profile button at the bottom of the side menu couldn't be reached with the D-pad (since 0.0.30)
- [x] Fix: one error in a button press stopped the controller until Hearth restarted (it now keeps working)
- [x] Fix: the D-pad skipped a row of buttons that sits to the side (Power mode, Controls); up/down now goes to the next row
- [x] Fix: a stick resting near its edge counted as many presses (presses at 55%, lets go below 35%)
- [x] Fix: volume and brightness sliders dropping out of change mode, or jumping back, when the status refreshed
- [x] Fix: in the carousel and other experimental layouts, a background refresh could make Play start the first game instead of the one picked
- [x] Fix: Shut down's confirmation didn't show in the in-game overlay
- [x] Fix: Quick Access performance refreshing twice a second; Friends selection moving to another person when the list changes; swapped A/B echo filter; Screenshots opening on the old picture
- [x] Fix: closing Quick Access while it was still opening could leave the game with no controller input, or input going to the hidden overlay (opening and closing now happen one at a time; the controller service's mode always matches what's on screen)
- [x] Fix: a quick second Xbox-button press left the game with no controller input
- [x] Fix: closing Quick Access now goes back to what was on screen (Steam chat, another app), not always the game; the system clock changing could close Quick Access by itself
- [x] Fix: Update now said "Done" when nothing was downloaded; now it says "already up to date", or the real problem (like another update running); it also checks the way the terminal's update does
- [x] Fix: an app that couldn't update no longer marks the whole system update as failed
- [x] Fix: Send logs lost the system journal (long output made it time out); the same affected any long command
- [x] Fix: the frame-rate limit claimed to work in Desktop Mode
- [x] Fix: leaving Desktop Mode could show a false "Desktop Mode keeps closing" error in Game Mode
- [x] Fix: turning off with games kept could be set up without really working (SELinux label on the swap file)
- [x] Appearance is shorter: experimental layouts fold away behind one row (Show/Hide), and layout cards are smaller
- [x] Fix: the downloads check could pile up into dozens of checks a second, each redrawing the screen (a likely cause of the jumping selection and the sluggishness); now one check at a time, and the screen only redraws when something changed
- [x] Logs for bug reports: controller input first, newest Hearth lines, old repeated lines left out, small enough to read; Hearth's log is kept to a few MB
- [x] Fix: after going to Desktop Mode and back, only the Xbox button worked (the controller service was left catching every button): it's reset whenever the mode changes, and Hearth uses presses that were caught instead of losing them
- [x] The Linux AppImage (for other Linux systems) is now the Rust app, under the same file name: the old Electron app updates itself into it, and after 5 minutes of running fine the new app archives the old Electron AppImage (deleted after 30 days) and clears its caches; the AppImage then updates itself from releases
- [x] Cleanup: obsolete workflows and dead code removed; no more Electron preview pre-releases
- [x] Bug finder (tools/ui-nav-check.js): presses every direction from every control on every screen and lists controls the D-pad can't reach

- [x] Hearth Browser on real Chromium (CEF): same look, bookmarks, history and settings as before (they carry over); ad and tracker blocking; the old browser stays as a fallback
- [x] Task Manager (Desktop Mode, Hearth OS and Windows Mantel): end apps that stopped responding (Force end if they don't close), CPU, memory and GPU graphs, temperatures, fans and battery, startup apps and background services; Ctrl+Shift+Esc
- [x] Files (Desktop Mode, Hearth OS and Windows Mantel): places, drives, USB and SD cards, Steam libraries, copy and move with progress, Trash, rename, new folder, Open with, controller actions menu (X)
- [x] Windows Mantel made the same as Hearth OS's Desktop Mode
- [x] Xodus Nexus: pick where Xbox games go when adding it (this device, an SD card or a second drive); "Sign in to Xodus Nexus" opens it in Desktop Mode, where the controller types; a password store runs in both modes so its sign-in is remembered

## 0.0.31 (released)

- [x] Fix: up and down didn't move the selection in 0.0.30 on some controllers (the change that ignored sticks resting off-center is undone)

## 0.0.30 (released)

- [x] Fix: Shut down in the power menu was skipped when moving down; menus now go item by item
- [x] Shut down and Restart ask first when games are open or in Quick Resume (and offer Turn off and keep games, or Sleep)
- [x] Fix: the selection jumping several places in Settings and menus (keys Steam or the controller driver make from a press, arriving after the button is up, no longer count as a second press; a stick or trigger that rests off-center no longer moves the selection by itself; slower repeat when holding a direction)
- [x] Experimental home layouts (Settings › Appearance › Experimental): 12 new layouts, marked beta, with live upvotes from everyone using Hearth

## 0.0.29 (released)

- [x] Fix: 0.0.28's app page didn't start (a piece of the Screenshots page went missing), which left Hearth and Ember blank
- [x] Required release check: Hearth's page must load and draw every screen without a script error, or nothing is published

## 0.0.28 (released)

- [x] Xbox PC games (experimental): add Xodus Nexus from Settings › Games › Library; Hearth downloads and updates it and puts it in the library
- [x] Update now works on systems with extra packages installed on top (it updates through rpm-ostree when bootc can't)
- [x] Update now: real download progress when updating through rpm-ostree (size, layers, speed), and "Done" only once the update is really downloaded (if bootc finds nothing but rpm-ostree does, it gets it)

- [x] Smoother menus and library on AMD/Intel handhelds: WebKit's fast GPU path is back on (it's only turned off on Nvidia) and pages are always drawn by the graphics card
- [x] Player counter: each device counts once (an id from the device itself, the same after updates), not again after every update
- [x] Friends: Steam, Xbox and Epic friends together, sorted into Playing, Online, Away and Offline, with a filter per platform; pick a friend to message them (Steam chat), join their server game, play the same game (installed or Xbox Cloud Gaming) or open their profile
- [x] Send logs to the developer: Settings › System sends the bug-report logs to Hearth OS's own service and shows a short code to share (kept 14 days)
- [x] Sign-in pages (Xbox, Epic, GOG): typing with the controller no longer clicks things on the page (controller buttons turned into mouse clicks and keys are dropped while you use the controller or the on-screen keyboard; touch still works)

## 0.0.27 (released)

- [x] Black screen (with the game still playing) after Quick Access or Quick Resume: games no longer minimize themselves when something else comes in front, and Hearth shows them again if they did
- [x] A game that was starting or had just started looked frozen after closing Quick Access: Hearth now puts back the game's current window
- [x] Sliders (volume, brightness) in Quick Access and Settings: press A to change one, A or B when done; moving past one no longer changes it
- [x] Quick Resume sometimes showed the last game's frozen picture: games put aside are never the one put in front
- [x] Settings › System › Save logs for a bug report: Hearth's and the system's logs in one file, personal details taken out, saved in the home folder and on a USB drive if one is plugged in
- [x] Required release check: every Hearth OS image must be able to update itself from Game Mode and Desktop Mode, or it isn't published

## 0.0.26 (released)

- [x] In-game overlay: games keep running while it is open, the game gets no controller input (InputPlumber), and only the Xbox / Guide button opens it
- [x] Power limit and power modes work on Bazzite's SteamOS Manager (custom profile), frame-rate limit works in games (gamescope's Vulkan layer)
- [x] Screenshots: gamescope takes them in Game Mode; Library › Screenshots to look through and delete them
- [x] Settings condensed: five groups (You, Connections, Device, Games, System) with their pages as tabs
- [x] Settings › Updates: Update now is back on Hearth OS (and Settings from Mantel, the OS name, controller remapping)
- [x] Ember: Quick Resume and profiles (Who's playing, own Steam account and sign-ins, parental controls)
- [x] GOG cloud saves (Hearth and Ember)
- [x] Steam friends: who's online and what they're playing (Hearth and Ember)
- [x] Hearth Browser blocks ads and trackers; Hearth Browser is back on Hearth OS (missing since 0.0.24)
- [x] Faster moving through a big library; a library speed test in CI
- [x] Installer test in CI: installs onto a disk image and boots it

## 0.0.25 (released)

- [x] Quick Resume, like Xbox: starting another game puts the one you're playing aside, frozen where you left it (up to 5); kept through Sleep and Hearth restarting, and through Shut down and Restart where the PC allows it (one switch: with games open, Shut down and Restart hibernate and come back to them; Shut down and close games really closes them)
- [x] Library is fast again in the Rust app (covers load in the background, off-screen tiles aren't drawn)
- [x] Xbox Series controllers pair over Bluetooth
- [x] Installer download shows percent, GB, speed and time left, every second
- [x] Quick Access over a game no longer moves the game underneath: Hearth takes the controller while it's open

## 0.0.24 (released)

- [x] Hearth is now a Rust app (about 11 MB instead of about 250 MB with Electron): same screens, less memory, faster start
- [x] Installer USB: boots into Hearth, finds every Linux and Windows on the PC (SteamOS too), connects a controller by itself, downloads Hearth OS while installing
- [x] Store sign-in pages work with just a controller

## 0.0.23 (released)

- [x] Profiles on Hearth: "Who's playing?", each person's own Steam account, Epic/GOG/Xbox sign-ins, favorites and play time; installed games shared
- [x] Faster Game Mode ↔ Desktop switches
- [x] Games with a launcher (press Play, the launcher closes, the game starts) come to the front
- [x] Game pages: ProtonDB rating ("On Linux: Gold") and your Steam achievements
- [x] Per-game frame-rate cap and power limit, for every game including Steam's
- [x] Performance overlay levels in Quick Access (Off, FPS, Simple, Detailed, Everything)
- [x] Pause a running game (frozen, no power used) from Quick Access
- [x] Library collections and a Show filter
- [x] Parental controls: PIN, play time a day, allowed games
- [x] Remote play: stream to Moonlight (Sunshine set up from Settings), and Moonlight to play another PC here
- [x] Backup and restore to a USB drive or this device
- [x] Emulators: ROMs folder (EmuDeck folders too) with box art, played with RetroArch
- [x] Themes and sound packs; the desktop wallpaper from Hearth's Appearance settings
- [x] Epic cloud saves for games Hearth installs
- [ ] More stores (itch.io, Amazon)

## 0.0.22

- [x] Hearth's own login screen (SDDM theme like the lock screen, controller works on it)
- [x] Hearth Browser, first version: tabs, address bar with suggestions, bookmarks, history, downloads, find, zoom, start page, settings
- [x] Ember: Hearth Settings window in Mantel; taskbar down the side on upright screens
- [x] Overlay doesn't flash black when opened over a fullscreen game
- [x] Ember's full Mantel looks and works like Hearth OS's: floating taskbar, launcher, Quick Access, power menu, Hearth menu (Xbox button), controls card, volume bar, fireside wallpaper, Hearth's controller layout

## 0.0.21 (released)

- [x] Hearth's own installer instead of Fedora's: screens in the Hearth app (controller) and Mantel (mouse and keyboard), no-password option
- [x] Sign-in choices: sign in automatically or not, ask for the password after sleep (Game Mode lock screen too)
- [x] Ember: Desktop style and wallpaper in Settings › Shell

## 0.0.20

- [x] Full Mantel for Ember: no Explorer at all, Ember hosts the tray (experimental)
- [x] Full on-screen keyboard in Mantel (Linux)
- [ ] Download the installer from inside Ember (needs the ISOs on SourceForge)
- [x] First-start setup on a fresh Hearth OS (Wi-Fi, store sign-ins, the controller, importing games)
- [x] Mantel lock screen (Super+L, power menu, on waking)
- [x] Ember as the shell starts your startup apps
- [x] External monitor brightness on Windows (DDC/CI)
- [x] Import games from Lutris
- [x] Overlay rework: stays on screen see-through and only changes visibility and input (Game Bar style), one focus owner so the screen stops jumping back, Ember closes a stuck overlay
- [x] Live player counter: Hearth and Ember report in, the count shows on the website and About page (off switch in Settings › System › Privacy)

## 0.0.19 (released)

- [x] Remove the old system: erase other systems' partitions on Hearth's drive, grow Hearth into the space
- [x] Live USB: Install next to Windows (checks, shrinks Windows, opens the installer with Use free space)
- [x] Ember: Switch to Hearth OS (make room on C:, write the installer USB, restart into it)
- [x] Mantel for Ember: Hearth's taskbar, launcher and wallpapers over Windows' desktop
- [x] Games load behind Hearth's starting screen and come forward when ready; slow Proton first starts explained
- [x] Offer to import games once on first start

## 0.0.18 (released)

- [x] Game Mode ↔ Desktop switch can't freeze
- [x] Detailed system update progress (download speed, time left, install and app steps)
- [x] Ember Desktop Mode: the Windows desktop and back (shortcut, tray, Ctrl+Alt+G, Xbox button), start-in setting
- [x] Import games and files: from Windows, SteamOS / Steam Deck and other Linux, other drives and SD cards
      (Steam, Epic, GOG, Heroic, Lutris, Proton saves, personal files; copy or move)
- [ ] Fixes from 0.0.17 testing

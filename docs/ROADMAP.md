# Hearth OS roadmap

Hearth OS is a free, Debian-based OS for gaming PCs and handhelds: it starts in **Steam Game Mode**, with **Hearth
Sync** putting your games from other stores into Steam's library, and **Mantel**, Hearth's own desktop, one press away.
**Ember** is Hearth on Windows 11. Versions are MAJOR.MINOR with a date (docs/UPDATES.md), and the owner picks every
number. Work lands on `bughunt` and goes out on `main` when it's tested. What's archived: docs/ARCHIVED.md.

## Planned for 1.12

- [ ] Task Manager, Resource Monitor, Hearth Files, C: everywhere and Recovery built completely to their approved mockups: the parts 1.11 left out, every one listed in docs/mockups/checklists/task-manager.md, resource-monitor.md, hearth-files.md, c-drive.md and recovery.md (red flag 11). Done so far: Memory's Overview (the programs' memory with compressed memory dashed, the composition bar, Swap, Memory pressure, Memory setting, Biggest users)
- [ ] Hearth Recovery USB built from the Debian edition (it's still made from the archived Anatase image, and was last uploaded on 26 October), so the Recovery USB matches what devices run — needs its own port: the Recovery build uses Fedora's live tools (dracut-live, livesys, Titanoboa), so it would be rebuilt on the Debian installer USB's live boot instead
- [ ] Hearth Games, stage two: cloud saves, DLC, Amazon (nile)
- [ ] Smaller, faster updates (instead of 2–3 GB every time): Debian's packages frozen to a set date (moved on purpose, weekly or for a security fix), repeatable builds so unchanged files stay byte for byte the same and aren't downloaded again, Hearth's own apps in small pieces of their own, each build saying how big the update is against what devices have now, and the download done quietly in the background so it's ready at the next restart. A normal update should be about 200–500 MB; the previous version always stays startable
- [ ] Bring the ARM build (feature/arm64, published as hearth-peppermint-arm64) up to date with the main Hearth OS and keep it there
- [ ] Tidy-up (nothing users see changes): split the biggest files into smaller parts (Hearth Files' FileManager.qml, Game Mode's ui/index.html, hearth-home-server and its root helper, Task Manager's view); subfolders in the desktop shell's 262-file folder (apps, settings pages, helpers); the "nimbus" names inside the code moved toward "Hearth" where it's safe (never paths an installed PC relies on); merged branches deleted (listed for the owner first); a quick check in CI for a stale settings index, roadmap items done twice, and tests that only pass on one kind of machine
- [x] The GitHub README redone: what Hearth OS is now (Debian, Steam Game Mode, Mantel, Hearth Sync), pictures, how to install and update, where things are in the repo, what's archived, and the current version
- [x] Horizon (github.com/horizon-rust/horizon, MIT): the native Rust remake of Minecraft: Java Edition 26.3, no Java needed, in Hearth Games with one-press install and updates (its Linux AppImage, x86-64 only), a Hearth Sync shortcut for Steam Game Mode, controller support on and Game Mode friendly; you sign in with your own Microsoft account in Horizon's own launcher (Hearth never handles it), and it uses your copy of the game. Also Horizon's headless server as a choice next to the Minecraft server in Settings › Game servers. Done: in Hearth Games › Recomps (the PC section), its launcher's AppImage from its `launcher@` releases, updates with the others, Hearth Sync as for every port (needs testing on a device). Still to do: Horizon's own server in Settings › Game servers
- [ ] Server mode › AI team (the owner's choice for 1.12; mockups server-dash / server-detail): a tile with a small live mesh of this PC's AI engines (AI chat, Pictures, Voices, Speech to text) and everything that sends them work around them (your devices, the web page, the pictures queue, the Command Center's AIs through One AI address), each request a pulse that lights its sender; A opens it big with today's counts, the engines and what happens while you play. The same drawing as the Command Center's Live › Mesh

## Later

- [ ] Task Manager, still to come: per-app network limits and blocking (Allow: Limit to … / Blocked) from mockup tm-5
- [ ] Resource Monitor, still to come: the Processor tab's services, libraries and per-core graphs (resmon-2); Your PC stays as it is
- [ ] "Reset this game's Windows setup" in a Windows game's Properties: rebuilds its Proton/Wine setup and keeps every save (they live in your folders once One C: is on); moving a game to another drive takes its setup with it
- [ ] Later, step 2 (with great care): real /Users and /Program Files folders on disk, your home moved to /Users/<name> with the old place kept as a hidden link; a one-time move with a backup and a way back, tested in CI and on a spare device first
- [ ] A self-hosted GitHub Actions runner on the home server (needs a registration token the owner sets up himself)
- [ ] For the owner: a signing key in the repository's secrets, so each new app installs over the last one (the old 0.2.2 needs uninstalling once: it was signed with a throwaway key)
- [ ] PRoot for newer phones with 16 KB memory pages (not needed on the Tab A9)
- [ ] Game saves backed up by Ludusavi (github.com/mtkennerly/ludusavi, MIT) on the home server: its list of where thousands of games keep their saves, Proton and Wine games included, under Hearth's save backups over Tailscale

## 1.11 (released 2026-11-13)

ISOs: the offline installer USB is on 1.11 (rebuilt and uploaded 2026-11-13, install test passed); the Recovery USB stays on its Anatase build until the Debian Recovery USB (1.12).

- [x] Hearth Link pairs once for all your devices: pairing asks My device or A friend's device. Your own devices swap their lists, so a new one is paired with every device you have, and removing one removes it everywhere; a friend's device stays paired only with the device it paired with, for copying games, and never sees the others or anything personal. A device on an older Hearth OS stays paired as now (docs/HEARTH-LINK.md, "Your devices and friends' devices") (needs testing on a device)
- [x] Server mode's new gold look, the Hearth AI Command Center's: the AIs on this PC as living cores, the same shapes, colours and talking motions as in the Command Center (AI chat's model, ComfyUI, Voices, Whisper, AI sharing), with who's being answered; everything the dashboard showed before, plus last-hour graphs on graphics memory, storage, power and network, graphics memory by app, Voices and Server apps tiles, the schedule's next switch, and a QR code to open the web page on your phone; the countdown, menu, details and idle screen in gold, with faint cores on the dimmed clock; Reduce motion keeps the cores still (needs testing on a device)
- [x] Hearth Recovery opens on a health check (the approved mockup B): it looks at the drive alone, offline and without changing anything (the drive's own self-check, free space, which versions started fine from the installed system's journal, a display setting on trial, the automatic fallback, your files) and says what's wrong, with one suggested fix front and centre (start the previous version, reset the display settings, Desktop Mode to free up space, or copy your files off a failing drive) and the other tools beside it; Y checks again; choosing a version shows how each one started lately. A portrait screen keeps the plain list. `hearth-recovery health`, tested with the Recovery screen test (needs testing on a device, and on a Recovery USB once it's rebuilt)
- [x] Fix: Voice Studio's Install stopped at once with "PyTorch didn't install … no such option: --progress-bar": pip's progress option was given before the install command, where pip doesn't accept it (needs testing on a device)
- [x] No more "… (classic)" in the app menu: the old Hearth Files, Terminal, Text Editor and Calculator keep their plain names (each desktop already shows one app of each kind), and an old "(classic)" is taken off (needs testing on a device)
- [x] A better app menu search, in every menu style: names that are or start with what you type first, then a word in the name, words in any order ("man task"), initials ("tm" → Task Manager), one typo ("calculater", "tsak"), accents and dashes ignored ("cafe", "wifi"), the program's own name, Windows' names (explorer, cmd, notepad, control panel, taskmgr, snip…), and apps you opened lately a little higher; the Best match is the top score of apps, games and Settings pages (needs testing on a device)
- [x] Apps for hardware a PC doesn't have are hidden: NVIDIA Settings only shows with an Nvidia graphics card. Worked out at every sign-in (a card put in shows it at once), nothing is uninstalled, and nothing waits on it (needs testing on a device)
- [x] The desktop shell's slow leak (1.5 GB and 100% CPU after 25 hours on 1.10, against about 440 MB): the helper calls Desktop, Files and file Properties make are always cleaned up, even when the window that asked closed meanwhile; the shell writes down its own memory and CPU every 10 minutes (in Send logs, "Desktop shell health"), and when it has grown far past normal or stays busy it refreshes itself, only while the screen is off or locked and none of its windows is open, then says so. The samples show what else still grows (needs testing on a device)
- [x] Memory and swap, on by default (the approved mockup): compressed memory (zram) sized for each PC (1.5× memory up to 8 GB, 1× up to 16 GB, half above; up to 2× with the recommended size marked), zstd or the other ways this kernel offers (lz4, lz4hc, deflate), swappiness 200 with zram, Keep memory free (Off, 2 GB by default, 25%, 50%) as the kernel's watermark, page-cluster 0, no watermark boost; your drive swap file (/var/swap/swapfile) used after zram, made or resized only when you press it, up to 64 GB and always leaving 20 GB free on C:. Task Manager › Performance › Memory › Swap & settings and Settings › System › Memory; Save, Try until restart and Back to recommended. Your test files (99-hearth-memory-test.conf, hearth-zram-test.service) keep working and their values are shown as yours; Use Hearth's settings hands them over with the same values (the files kept as .bak). Your own /etc/sysctl.d files win. Nothing waits for it at start-up (needs testing on a device)
- [x] When memory runs out (the memory guard, Off / Gentle / Strong): instead of the PC freezing, Hearth closes the one app using the most memory and swap, all of its processes, and tells you which; never one on your Never close list (the desktop, Steam, the game you're playing, the AI Command Center, and any you add), which the kernel's own last resort spares too. No MGLRU min_ttl (it closed the Command Center and Claude on the laptop) (needs testing on a device)
- [x] Memory "In use" counted the way Windows does (total − free − cache) in Task Manager, Resource Monitor and the stats overlay: it said 100% in use with gigabytes free when Keep memory free was high (needs testing on a device)
- [x] Task Manager, improved (the approved mockups): Processes with PID, Compressed (memory moved to zram or swap) and Power columns and a Hearth group; right-click End process tree, Set priority (Normal, Below normal, Low), Run on these cores, Limit CPU / memory (a cap on the app's own scope, never closed), Never close when memory runs out, Go to details; Details with Parent, Swapped, Threads and Priority, and Set priority / affinity; each Performance page opens a settings tab: CPU Power & settings (power mode, turbo boost, top speed, temperature, throttling), Disk Health & settings, Network (metered, a mobile hotspot), GPU Tuning (LACT), Battery Health & settings (health, cycles, a charge limit) — remembered in /etc/hearth/hardware.json and put back at every start (needs testing on a device)
- [x] Task Manager: Scheduled tasks (systemd timers: yours on / off, the system's read only, next and last run) and Users (who is signed in, their CPU, memory and processes) (needs testing on a device)
- [x] Task Manager › App history: processor time, disk read and written and when each app was last used, since a date, counted every 5 minutes by a small user timer (only ~/.local/state/hearth/app-history.json), with Delete usage history (needs testing on a device)
- [x] Task Manager › Performance › CPU › Game cores (off unless you turn it on): while a game runs it gets its own cores (2, 4 or 6 of 8, say) and the system's services (updates, AI, builds) run on the others; runtime-only, so a restart or turning it off shares every core again (needs testing on a device)
- [x] Resource Monitor: Disk tab (every drive's kind, file system, busy, response, free, total, how full and temperature; files the busiest programs have open, ✎ for writing), Network tab (each program's download and upload speed from its TCP connections, and each connection's ping), Graphics tab (3D, video decode and video memory per program; 3D, video decode, video memory and temperature graphs) (needs testing on a device)
- [x] Hearth Files, the rest of the approved mockups (files-6 to files-8): Extra large icons, View › Navigation pane, and the right-click's Pin to the left menu and Open in Terminal (tabs, the details pane, Copy as path and the quick icons on top were already there) (needs testing on a device)
- [x] Graphics card controls with LACT 0.10.1 (MIT) in the image: Task Manager › Performance › GPU › Tuning opens it, for fan curves, power limits and undervolting on Nvidia, AMD and Intel. Its daemon stays off until you open tuning the first time (one password prompt), so nothing about the card changes by itself (needs testing on a device)
- [x] Low battery: a warning at 10% and at 5%, and at 3% the PC is put away safely before the battery dies: hibernated where a hibernation has come back on that PC before, otherwise a clean shutdown at 2% (UPower), so no files are lost. Plugging in cancels (needs testing on a device)
- [x] A PC moved from the Fedora edition gets Hearth's own boot menu (`sudo /usr/libexec/nimbus/hearth-boot-move run --yes`, only when asked): it kept starting through Fedora's shim and GRUB, without Hearth's themed menu and ordering. A full copy of the EFI partition and the menu first, Fedora's loader kept with its own firmware entry ("Hearth OS (old boot menu)"), a menu both GRUBs read, then Debian's signed shim and GRUB; `undo` puts the copy back. Proven in a virtual PC: Fedora's loader starts, the move, then it starts through Debian's GRUB (needs testing on a device: the Nvidia PC)
- [x] Hearth Files, the Windows 11 left menu (the approved option A): a narrow 158 px menu you can drag wider or narrower (remembered); Home, Gallery, Home server (your devices' shares); your folders; This PC with Hearth (C:) open (Users, Program Files, Windows apps) and the other drives; Network; the Linux folders folded under "Linux" only with Show the Linux folders. Folders with thousands of files open much faster: each name is sorted with its keys worked out once, and the hidden view no longer builds rows (needs testing on a device)
- [x] Hearth's terminal speaks C:\: cd C:\Users, cd G:\Steam, cd.., dir (/a /s /w /b), copy, xcopy, robocopy, move, ren, del, rd (/s), md, type, cls, where, tasklist, taskkill, ipconfig, ver, start, explorer and notepad, with C:\ and the other drive letters understood; Linux commands and paths work as always; del, move and rd /s ask first; off with HEARTH_WINDOWS_COMMANDS=0; never in scripts (needs testing on a device)
- [x] Saved games (Settings › Gaming), with Ludusavi 0.31.0 (MIT) in the image: every game's saves found (Steam, Proton, Wine, Heroic, Lutris and thousands of others), backed up into your Saved Games folder and put back, all at once or one game at a time, on this PC or another after a reinstall; only when you press it (needs testing on a device)
- [x] One C: for Windows apps and Steam's Windows games: each keeps its own Windows setup, but its Documents, Desktop, Pictures, Music, Videos, Downloads and a new Saved Games folder are your real folders, and its drives get the same letters as in Hearth (Steam games at every sign-in, in the background). Only empty folders and Hearth's own links change, so saves already inside a game's setup stay exactly where they are (needs testing on a device)
- [x] A Compatibility section for Windows apps (Settings › Windows apps): Proton version, DXVK, VKD3D and the Windows version together with "Use these files from Windows" (DLL overrides like d3d9 or dinput8, the app's own copy first) and the common fixes (no esync, no fsync, more memory for old 32-bit games, Nvidia DLSS, hide an Nvidia card, old OpenGL games), all remembered per app (needs testing on a device)
- [x] Hearth (C:) everywhere, step 1 (the approved mockups cdrive-1 … 4): every app's Open and Save window (Hearth's picker, which apps from apt and Flatpak use too) shows Hearth (C:) and its Windows folder names, accepts a typed C:\\ or G:\\ path, and always hands the app the real path (a folder that's only a name in the view can't be saved into); Properties shows where a file is as C:\\Users\\… with the Linux path under it, and Copy gives the C:\\ form; Settings › Drives & folders: Show system folders, Apps you install, Terminal speaks C:\\ (on / off). The Run box already took C:\\ paths. Nothing on disk moves (needs testing on a device)
- [x] One Settings › Your devices page: your paired devices at the top (Hearth Link), and on the PC that's the home server a "This PC is the home server" section under them saying what it does (shared folders, backups, people, AI, what's paused while you play), one press to its pages. Hearth Link and the home server stay two services underneath, and every setting stays where it's saved (needs testing on a device)
- [x] Proton detector for Hearth Games: when Hearth picked the Proton (your own pick always wins), the game is watched; if it closes right away with an error, that Proton is noted and the next one is tried first next time (a notification says which), and one it ran well with is remembered for that game (needs testing on a device)
- [x] Resource Monitor's Overview rebuilt (the approved mockup resmon-1): Processor, Memory, Disk, Network and Graphics, each with its chips (use, speed, temperature, memory in use and in swap, memory pressure, the busiest drive, down / up, the graphics card) and its top programs with their PID; click a heading to fold it; tick programs to see only them in every section; the list scrolls instead of being cut off (needs testing on a device)
- [x] Resource Monitor's Memory tab (mockup resmon-3): the bar shows compressed memory too, and the numbers under it: what zram holds and its ratio, the drive swap, compressed memory's real size, committed; programs' Compressed column (needs testing on a device)
- [x] Resource Monitor's Processor tab: Priority for each program, and the selected program's open files (with their C:\ names) (needs testing on a device)
- [x] Resource Monitor's Network tab: listening ports with who can reach each (this PC only, Tailscale only, any network) and open connections, with the program when it's yours (needs testing on a device)
- [x] Optimize drives (Task Manager › Performance › Disk › Health & settings): each drive's kind, SSDs and NVMe drives TRIMmed weekly (never defragmented) with the last run shown, and Optimize now (needs testing on a device)
- [x] Optimize drives on hard drives: Optimize now checks a hard drive's ext4 for fragmentation (e4defrag -c) and defragments only above a score of 30; never Btrfs or NTFS (needs testing on a device)
- [x] Main drive almost full: a pop-up at 90% full or under 10 GB free (and again under 3 GB) says "If the drive Hearth OS starts from fills up, your PC might not start properly" with Free up space (Settings › Storage). Hearth's partition holds the system and your files together with nothing set aside, so a full one can stop the login screen and session from starting (as happened on Anatase, same layout): Hearth now keeps a 2 GB emergency reserve while there's plenty of room and frees it at start-up or from the pop-up when under 2 GB are left, and a system update stops before downloading when under 5 GB are free (needs testing on a device)
- [x] The same almost-full warning in Hearth Disks' Disk Management: a banner over the volume list, and any volume with 10% or less free shows its free space in gold (needs testing on a device)
- [x] One AI address (Settings › Home server › AI devices, off until you turn it on): apps that speak OpenAI's API use the home server's address (`http://<server>:8189/v1`) and each chat, voice or speech-to-text request goes on to the first free device in your order that has the model, skipping one that's playing, on battery or off; Tailscale only, with the share code if you set one (needs testing on a device)
- [x] Home server › Alerts & power: Night maintenance (off until you turn it on; once a night at 2:30, 3:30, 4:30 or 5:30, only when nobody uses the PC and it isn't busy: TRIM, system logs kept to 1 GB, Flatpak app updates, downloaded package files cleared; each step on its own, never anything that touches starting the PC; Last night says what it did) and Power history (off until you turn it on; processor and graphics card watts every 5 minutes, 30 days kept, the last 24 hours as a graph with the average and kWh a day) (needs testing on a device)

## 1.10 (released 2026-11-09)

- [x] Hearth OS's built-in Proton works without Steam and 100% offline: Hearth Games' stores, your own games, Steam-engine direct starts, Recomps, Battle.net and Hearth Mods start Windows games with hearth-proton-run (the built-in GE-Proton, no umu-launcher, Steam Runtime or download); a saved Proton that isn't on the PC is swapped for the built-in one for that start, with a message, and kept; checked with the network off in CI (docs/OFFLINE-PROTON.md) (needs testing on a device)
- [x] Pick your Game Mode in Settings › Gaming › Game Mode: **Steam Game Mode** (the default, exactly as before) or **Hearth Game Mode** (Hearth's own full-screen screens), as two picture cards you move through with the controller. It applies the next time Game Mode starts, with Switch now right there (needs testing on a device)
- [x] The same choice in Hearth Game Mode's Settings › System, and in Steam Game Mode's Quick Access › Hearth Sync › Game Mode (with Switch now: Steam closes properly first). Nothing of Steam's own screens is changed (needs testing on a device)
- [x] Everything that opens "Game Mode" opens the one you picked: turning on and signing in (automatic sign-in too), Mantel's Game Mode buttons and Super+G, Server mode's Guide countdown, Play and way back, Steam's own Game Mode switch. Switch to desktop from either Game Mode opens Mantel (needs testing on a device)
- [x] Hearth Game Mode works on today's image again: it starts Hearth's app in gamescope with Hearth Input, first-time setup no longer pushes it over to Steam, Steam games still play through the real Steam client, and your other stores' games play through Hearth (needs testing on a device)
- [x] If Hearth Game Mode can't start (missing pieces, Hearth crashing 3 times, closing right away), that start goes to Steam Game Mode, then Mantel, and says why: a toast in Steam Game Mode, Mantel's "didn't start" card, Settings, and Send logs. Failing twice in 10 minutes pauses it until you press Try Hearth Game Mode again. Your choice is never changed by Hearth (needs testing on a device)
- [x] Holding B or Guide while it starts still opens Desktop Mode, and nothing about booting, the boot menu or the login screen changed (needs testing on a device)
- [x] Steam games open in Steam Game Mode: in Hearth Game Mode, Play on a Steam game shows "Opening in Steam Game Mode…" and Steam Game Mode starts that game straight away; when it ends you're back in Hearth Game Mode by itself (started something else from Steam's library: it stays; Quick Access › Hearth Sync has Back to Hearth Game Mode). If Steam Game Mode can't come up, you're back in Hearth Game Mode with a note. Your Game Mode choice never changes. **Play Steam games in Steam Game Mode** in Settings turns it off (on by default; nothing saved until you flip it). Other stores' games keep playing inside Hearth Game Mode (needs testing on a device)
- [x] Hearth no longer changes the Steam engine by itself: with none picked, Steam games use Valve's Steam client, and nothing is saved until you pick an engine in Settings › Library (an older Hearth switched it to Hearth's own Steam client once when the app first started; anyone it switched keeps that and can turn it off there) (needs testing on a device)
- [x] Hearth (C:) the Windows way in Hearth Files (the approved mockups): opening Hearth (C:) shows Program Files (every installed app, one folder each: Hearth's own, Flatpaks, Hearth Games, Recomps), Program Files (x86) (Windows apps and Steam), ProgramData, Users (your folder with AppData: Roaming, Local, Local\Temp) and Windows (look only), each with where it really is (an On Linux column and In Hearth / On Linux in the details pane). Only names: nothing on your drive moves. The address bar shows and understands C:\Program Files\… (D:, E: as before), the right-click menu has Go to the real folder, Copy path, Copy Linux path, Modify, Uninstall and Properties, Properties says where the apps come from, and files can't be dropped straight into the view's folders. "Show the Linux folders too" adds the real folders. Settings › System › Drives & folders (and Files' View menu) has the switches and the drive letters; nothing is saved until you change one. Windows apps keep their own C: (needs testing on a device)
- [x] Briefcase is a folder you make: Files no longer pins ~/Briefcase in the navigation pane or makes it by itself. New › Briefcase makes one anywhere, and Add to Briefcase offers New Briefcase… (pick where) when you have none. A ~/Briefcase you already have stays where it is and keeps working, and a Favorite you added for it stays (needs testing on a device)
- [x] Hearth Disks is now laid out like Windows' Disk Management: a volume list on top and a map of every disk below (gold: Hearth runs from it, green: games, blue: a partition, black: unallocated), Hearth's drive letters (C: is Hearth), a right-click or X menu with New Simple Volume, Change drive letter, Format, Extend, Shrink, Delete, Use for games and Properties (General, Health with SMART and a full check, Volumes). Hearth's own partitions only offer Open and Properties; Format and Delete need the volume's name typed; Extend and Shrink go through udisks2 and refuse while Steam uses the drive; "Tell me if this drive starts to fail" is checked daily by a small user timer that does nothing unless you turned it on for a drive (docs/STORAGE.md "Disk Management") (needs testing on a device)
- [x] System Restore (the app menu, Settings › Updates & recovery, `hearth-restore`), to the approved mockup: restore points with date, description and type (Update, Automatic, Scheduled, Manual), "What will change" (the Hearth version, how many settings changed, apps stay, your files are never touched), See every changed setting, Make a restore point now, Restore and restart; mouse, keyboard and the controller (needs testing on a device)
- [x] Restore points: a copy of your Hearth settings files (only the listed ones; never secrets, play time or your files), made before each update from Settings, the first time a new Hearth version runs, before the writable system is switched, weekly (a user timer that can't hold up starting) and on demand; about 10 kept in ~/.local/share/hearth/restore (needs testing on a device)
- [x] Restoring: today's settings are kept as a restore point first, the settings go back through hearth_settings_io (atomic, .bak), and the version goes back only with Settings › Updates' own hearth-admin rollback when it's the one bootc keeps (the version you come from stays in the boot menu); if that fails, today's settings come back. A version no longer kept: it says so and offers the settings only (needs testing on a device)
- [x] Reliability history (the app menu, Settings › Updates & recovery, `hearth-reliability`), to the approved mockup: a stability line over 14 days or weeks, marks for App stopped working (crashes), Hearth problems (session fallbacks, failed Hearth services), Warnings (network drops, kept modest) and Updates and changes; a day's events with Details and Send logs. Reads only; the journal is read on open and finished days are cached in ~/.cache/hearth (needs testing on a device)
- [x] Device Manager: the PC's devices in Windows' groups (Audio, Bluetooth, Cameras, Controllers, Disk drives, Display adapters, Keyboards and mice, Monitors, Network adapters, Processors, USB controllers, Other devices), the driver (built into Hearth or added), firmware, maker and where it's connected, a yellow warning for no driver, missing firmware and errors from the kernel log, Send logs, Disable / Enable for Bluetooth and Wi-Fi only, Nvidia driver settings, Scan for hardware changes (needs testing on a device)
- [x] Run (Super+R): Windows commands (taskmgr, devmgmt.msc, diskmgmt.msc, control, ms-settings:, notepad, cmd, explorer, calc, msconfig, rstrui, perfmon /rel, appwiz.cpl…), %appdata% / %localappdata% / %temp% / %userprofile%, C:\ and D:\ paths with This PC's letters, apps, websites, suggestions and recent entries (state, written only after opening something) (needs testing on a device)
- [x] The Super+X menu (also a right-click on the Start button): Windows' quick links with their underlined letters; entries whose app isn't installed are hidden; no Terminal (Admin), since Hearth has none (needs testing on a device)
- [x] Volume mixer: right-click the sound icon › Volume mixer, or Settings › Sound › Volume mixer. Pick the output (speakers, headphones, TV), the system volume, and a row per app playing sound with its icon, name, where it plays, mute and a slider. Right-click an app (X on the controller) to play it on another output. Each app's level is remembered by the app's name, only when you move its slider, and put back when it plays again; opening the mixer writes nothing. Controller: Up/Down rows, Left/Right volume, A mutes (needs testing on a device)
- [x] Snap layouts: Super+Z, a right-click on a window's Maximize button, or "Snap layouts…" in its title bar menu shows six layouts (halves, two-thirds and a third, thirds, quarters, a half and two quarters, narrow-wide-narrow); pick a zone and the window snaps there. Hovering over Maximize can't open it (labwc draws the title bars and doesn't say where the pointer rests), so right-click does. Controller: D-pad between zones, A picks (needs testing on a device)
- [x] Clipboard history, restyled and extended (Super+V): text and pictures with the app they came from and when, pin and unpin, Clear all (pinned items stay), click to paste; about 25 items in ~/.local/share/hearth/clipboard. Off until you turn it on (a switch in the popup and in Settings › Clipboard); a new install starts with it off, and an existing install keeps what it had. Copies a password manager marks as secret are never kept. The old cliphist history isn't carried over (needs testing on a device)
- [x] Emoji picker (Super+.), restyled: a compact popup with Emoji, Symbols and Kaomoji tabs, search and Most used; the pick is typed into the app you were in (needs testing on a device)
- [x] Hearth for Android is out of the archive: the app is built again on every change (Actions › Android), with today's Hearth page (its looks, Settings, the keyboard and everything the page loads now goes into the app, which was missing and would have left parts blank) (needs testing on a device)
- [x] Made for the Samsung Galaxy Tab A9 (8.7", MediaTek Helio G99): landscape looks like a Steam Deck, and on a tablet Hearth now turns to portrait too, with the page's narrow layout; phones stay landscape. Other MediaTek and ARM64 phones and tablets keep working (needs testing on a device)
- [x] The device's settings inside Hearth, with the controller: all four volumes and mute, brightness, adaptive brightness, screen timeout, auto-rotate and Do Not Disturb change right in Hearth (Android asks once for "Modify system settings" or "Do Not Disturb access", and says why first; a "no" changes nothing) (needs testing on a device)
- [x] Wi-Fi in Hearth: the network's name and signal, networks nearby, and joining one with a password typed in Hearth (Android shows its own "Save network?"); Wi-Fi on/off, mobile data and NFC open Android's panel over Hearth, because Android doesn't let apps switch those (needs testing on a device)
- [x] Bluetooth in Hearth: paired devices listed, pairing a controller or headphones from Android's list over Hearth, turning Bluetooth on with Android's dialog (needs testing on a device)
- [x] Battery: level, charging and battery saver shown in Hearth; "Let Hearth run in the background" so Samsung's battery saving doesn't close Hearth or Hearth Linux (needs testing on a device)
- [x] Everything else is one press away on the exact Android page, and B brings you straight back: dark mode, text and display size, default home app, notifications, storage, date and time, language, accessibility, developer options; Uninstall… in a game's options (needs testing on a device)
- [x] Your library and settings are kept by the settings rules (saved whole, a backup copy kept, never written over when unreadable, and Hearth tells you if that ever happens); Appearance is kept by the app too, not only by the page (needs testing on a device)
- [x] Android 14/15: the "Hearth Linux is running" notification asks for permission when Linux first starts something; Hearth Linux's downloads checked (Debian and PRoot still download and match their checksums) (needs testing on a device)
- [x] Checks on every change: the unit tests, Hearth's page as the app shows it on a Tab A9 (pictures of Home, Library and Display & sound) and the app started on an Android emulator until Home is up (needs testing on a device)
- [x] Sub themes in Settings › Personalisation: Hearth, Steam-style and Gaming each get five colourways (today's colours first, then four new ones: Rose Quartz, Evergreen, Midnight, Copper Sea / Crimson, Emerald, Royal, Graphite / Toxic, Inferno, Ultraviolet, Frostbite) and the Black set of seven (Black & Red, Black & Green, Black & Yellow, Hazard, Volt, Holly, Traffic Light), each with its own wallpaper and a light mode. Only the colours and wallpaper change; each look keeps its shape. The page is now compact dropdowns (Look, Sub theme with pictures and colour dots, Mode, Match sub theme, Accent) that work with a controller. Nobody's desktop changes until they pick one, and a wallpaper you chose yourself is never replaced (needs testing on a device)
- [x] Resource Monitor in Mantel: Your PC, Overview, Processor, Memory, Disk, Network and Graphics, per-program tables with tags (Game · Proton, Steam, Hearth, Home server, System), live graphs, right-click End / Efficiency mode / Open file location / Search online, LB / RB tabs; from the app menu, Task Manager and Settings › System › About (needs testing on a device)
- [x] Your PC: the board or device drawn from what this PC really has (desktop ATX / mATX / ITX boards, laptops, ROG Ally family, Steam Deck, Legion Go), every slot, port and header with its wires (as many lines as lanes), flow on busy links, click a part for its full specs, and tips only when true (needs testing on a device)
- [x] Memory sticks' maker, part number and slot through a small read-only helper (no password for the person at the device, cached in ~/.cache/hearth) (needs testing on a device)
- [x] Quick Access › Resources in Steam Game Mode (Hearth Sync's panel) and Hearth Game Mode, with "Pause downloads and backups while I play" (this session only, no setting changed) (needs testing on a device)
- [x] Resource Monitor from Run (`resmon`) and the Super+X menu (Resource Monitor, C); `perfmon` still opens Task Manager › Performance (needs testing on a device)
- [x] Settings › Personalisation no longer looks for a missing picture (wall-neon.jpg) for Gaming's wallpaper: it shows Gaming's own picture (needs testing on a device)

## 1.9 (released 2026-11-08)

- [x] The USB installer looks for controllers again on its first screen: a Controllers card lists every controller it finds (wired and built-in ones show as Connected straight away), and a Bluetooth controller put in pairing mode pairs by itself (Pairing…, then Connected; Couldn't connect has Try again). Only controllers are paired, never keyboards or headsets. A PC without Bluetooth says to plug in a controller with a cable. It stops looking when you leave the first screen, and Let's go always works (needs testing on a device)
- [x] The USB installer, redesigned to the approved mockups: "Your PC, ready to play." with a big flame and the controllers as tiles on Welcome, Internet (optional, with Skip), the drives as big cards you browse with ◀ ▶ showing what's on each, Hold Ⓐ to erase, About you, the time zone with quick picks (the full list one press away), a last look that also lists your controllers and look, a huge percentage with a checklist that follows the real install steps, and "Hearth OS is ready.". **Pick a look** on Welcome (the Look chip, or Ⓧ): the same eight looks as Mantel and the website, coloured from Mantel's own palettes, with Dark / Light / Auto; each one recolours every screen and brings its touches (Gaming's cut corners, Synthwave's grid and scanlines, Glass's aurora, Cabin's wood, Console's bigger type). The look you pick becomes your desktop's starting look on the installed system (merged into your settings once, with a backup copy); pick nothing and Hearth OS keeps its own defaults. A broken look never stops the install (needs testing on a device)
- [x] Play with mods: a modded Steam game (Skyrim SE with a Wabbajack list like Tuxborn) starts on the regular Steam, which it can reach: Play starts that Steam hidden when it isn't running, uses its Proton, and when only the Flatpak Steam is open says to close it instead of SKSE quietly ending after 3 seconds. Send logs keeps Proton's errors instead of hundreds of trace lines (needs testing on a device)
- [x] Voice Studio, Hearth's own voice studio on the home server (Settings › Home server › Services › Voice Studio): install on demand (about 14 GB, in your Home), on/off, Open, a PIN for its page (or Server mode's), the address and an API key to copy for the Command Center, Stop it while I play, Remove (your voices stay unless you confirm twice). Only your devices on Tailscale and this PC can reach it (needs testing on a device)
- [x] Speak: any voice, 40+ languages, speed, download as MP3, WAV, FLAC or Opus (needs testing on a device)
- [x] Clone a voice from a short recording (upload, or record; on a phone the recorder opens), with a consent box; it goes into your voice library (needs testing on a device)
- [x] Design a voice from words or chips, hear it, keep it: it stays the same person every time (needs testing on a device)
- [x] Transcribe audio or video to text, SRT and VTT, with Speaker 1 / Speaker 2 labels (needs testing on a device)
- [x] Your voices: one library for every feature and the API (hear, use, default, rename, delete), plus eight ready-made voices (needs testing on a device)
- [x] Audiobooks from pasted text, .txt or .epub: chapters found by themselves, narrated in a queue that carries on after a restart or a game, with an MP3 per chapter, a zip and an .m4b with chapters (needs testing on a device)
- [x] Video dubbing: transcribed, translated by your AI service, spoken again in each speaker's own cloned voice (or one you pick), each line timed to the original, music and sounds kept (Demucs), muxed back with subtitles in both languages; queued like audiobooks (needs testing on a device)
- [x] OpenAI-compatible speech API: /v1/audio/speech, /v1/audio/transcriptions, /v1/audio/translations, /v1/audio/voices and /v1/models, with a key, for the Hearth AI Command Center and other apps (needs testing on a device)
- [x] Kokoro (AI voices) keeps working as before for anyone who uses it, and Voice Studio lists its voices too (needs testing on a device)
- [x] The web page never goes back to the PIN screen without a word: when the PIN was right but the browser didn't keep the sign-in, it says so and what to do; sign-ins, wrong PINs and unknown sign-ins go to the log for Send logs (needs testing on a device)
- [x] Hearth Storage's Windows sharing: a folder whose name has a line break, a trailing backslash or % in it isn't shared any more (such a name could add Samba settings of its own, even a share of the whole system) (needs testing on a device)
- [x] Make a bootable USB: an internal drive always asks for an administrator's password, even when another option has a line break in it (polkit rule and hearth-usb both check) (needs testing on a device)
- [x] Hearth Recovery's "Reset screen settings" writes session.conf safely (a new file swapped in, same owner and permissions), so a power cut can't leave your other session settings half-written; a file it can't write is reported instead of ending Recovery (needs testing on a device)
- [x] Ember: an update without its checksum file is refused instead of being installed unchecked (needs testing on a device)
- [x] Store sign-ins: starting a second sign-in while one was still open (Sign in pressed again, or another store from Hearth in Desktop Mode) no longer closes the new sign-in window and puts Hearth back over it a moment later (needs testing on a device)
- [x] Settings changed in another program (Task Manager's Steam in the background, Hearth Link, the core service) now show and work in Hearth's Game Mode straight away, instead of the value Hearth read when it started (needs testing on a device)
- [x] Quick Resume: closing a game put aside no longer ends another game's processes that it shares a folder or Wine prefix with (needs testing on a device)
- [x] The fail-safe: background work (Battle.net's setup, a tool downloaded for a game) no longer switches the fail-safe off for the next request, so a Hearth stuck on that request is still freed or restarted (needs testing on a device)
- [x] Settings › Display: the renderer choice, Nvidia's drawing choice and Xodus Nexus's games folder are written whole with a last good copy, so a power cut can't empty them and turn them back to Automatic by themselves (needs testing on a device)
- [x] Clock and Calendar: a power cut that damaged clock.json or calendar.json no longer lets the next save (or the background service after a missed alarm) wipe every alarm or event; they come back from a .bak (needs testing on a device)
- [x] Calendar: events added while a subscribed calendar was downloading are no longer undone by the service saving its older copy (needs testing on a device)
- [x] Calculator, Emoji, Documents, Maps, Player, Terminal, Text, Office, Contacts and Theater: a damaged settings or list file is put back from a .bak instead of the defaults being saved over it (history, saved places, podcasts, CardDAV accounts, sites...) (needs testing on a device)
- [x] Terminal, Office and Calculator: two windows no longer undo each other's settings (each saves only what it changed) (needs testing on a device)
- [x] Hearth Connect, Nearby Share and Recorder: a damaged paired-devices or settings file is put back from a .bak, so a power cut no longer means pairing every phone again or losing the recorder's choices (needs testing on a device)
- [x] First-time setup: "Sign in to your game stores" shows your stores once they're read (it said "Loading…" until something else redrew the page), and says why with Try again when they can't be read (needs testing on a device)
- [x] First-time setup: B goes to the step before (and does nothing on the first). It used to leave setup for Hearth's Home half done, and a step passed by mistake (like Wi-Fi) couldn't be reached again with a controller (needs testing on a device)
- [x] First-time setup on Hearth OS: "Look for games on other drives" no longer marks setup as done, so B comes back to setup and finishing it still hands over to Steam Game Mode (it stayed in Hearth's own Game Mode) (needs testing on a device)
- [x] Hearth's page: Library › Stores, Settings › Profiles and Settings › Import games say why with Try again when they can't be read, instead of "Loading…" for good (needs testing on a device)
- [x] Hearth's page: a look setting (layout, text size, button swaps) that couldn't be saved to its file says so, instead of going back by itself at the next start (needs testing on a device)
- [x] Quick Access (top dock): left / right move from the tab that's selected, so no tab is skipped after the pointer selected another one (needs testing on a device)
- [x] Hearth's page: long names fit. The clock no longer breaks onto two lines next to a long game title; long Bluetooth, sound device and profile names wrap inside their rows and cards instead of running over the next column; a title with no spaces wraps on the game page (needs testing on a device)
- [x] Downloads: Steam's own words for a stopped download are shown as text, never as part of the page (needs testing on a device)
- [x] Desktop Mode: saving a keyboard layout or the mouse settings, changing your password, a Startup schedule, joining a Wi-Fi network with a password and opening a shared folder with a password no longer fail. Hearth handed them the text through a way Debian's Quickshell refuses ("Unable to assign QJSValue to QVariantHash" in every log), so they got nothing; it goes on the helper's input now (needs testing on a device)
- [x] Hearth Files only saves the view settings you changed: it wrote back everything it had read when it opened, so "Show in Files" (Settings › Devices) or another Files window's folder views could switch back by themselves (needs testing on a device)
- [x] A Desktop Mode setting whose save failed is tried again instead of being dropped, and the taskbar's switches (Do Not Disturb, desktop icons, auto-arrange, unpinning) say when they couldn't be saved instead of quietly coming back at the next sign-in (needs testing on a device)
- [x] The right-click menu scrolls when it's taller than the screen (a handheld with app scaling): rows ran off the bottom and Up/Down picked ones you couldn't see (needs testing on a device)

## 1.8 (released 2026-11-08)

- [x] Task Manager, the Windows 11 way: a title bar with search (name, publisher or PID), a left rail that folds to icons (Processes, Performance, Startup apps, Details, Services, Settings at the bottom). Processes groups Apps and Background processes with Steam games, Windows apps and Part of Hearth marked, unfolds apps with several processes, shows Name, Status (Efficiency mode leaf, Suspended), CPU, Memory, Disk, Network and GPU with the totals in the column headers and heat-map shading in your accent's warm tones, and has Run new task, End task, Efficiency mode (lowest priority for your own apps, user-level only) and View, plus a right-click menu (End task, Efficiency mode, Open file location in Hearth Files, Search online, Properties). Performance has mini graphs on the left (CPU, Memory, each disk with This PC's letters when it has them, Wi-Fi / Ethernet, each graphics card with its temperature, the battery) and a big 60-second graph with the classic numbers under it. Startup apps shows the last start-up time, Enabled / Disabled with an impact estimate, and Hearth's own as Part of Hearth (Disable only changes your own copy). Details lists every process; Services lists yours (start / stop, never Hearth's own) and the system's (read only). Settings: start page, update speed (High / Normal / Low / Paused), always on top; column widths, sort and the last page are remembered in taskmgr.json, and opening it writes nothing. One light sampler serves every page while it's open. Controller: Up / Down in the rail, A or X opens a row's menu, B goes back (needs testing on a device)
- [x] **This PC in Hearth Files, the Windows 11 way** (the owner approved the mockups, 2026-11-07): a This PC page with Folders, Devices and drives (each with a drive letter that stays the same, C: for Hearth, D:, E: … for other drives, and the thin space bar) and Network locations; the navigation pane with This PC opened out; the address bar in letters (This PC › Games (D:) › …) with the Linux path in the details pane and Copy path; Installed apps like Windows Settings (Hearth, Flatpak, Windows apps, Steam games, Recomps, with size, Modify, Move to another drive, Uninstall); Windows apps see the same drive letters. Nothing moves on disk (needs testing on a device)
- [x] Desktop Mode: four looks for the right-click menu, in Settings › Personalization › Bar and dock › Right-click menu (picture cards; it changes straight away; Today stays until you pick another): **Today**; **Windows 11 style** (quick icons for Cut, Copy, Rename and Trash on top, short rows with their shortcuts, submenus open to the side); **Compact list** (the smallest); **Tiles and groups** (big tiles for the main actions, then titled groups). One menu for Hearth Files, the desktop, the bar, the dock and the app menus; all follow your look and accent, light and dark, and work with the mouse, keyboard and controller (A choose, B back, with a small hint row while the controller is in use). If a new look can't load, today's menu shows (needs testing on a device)
- [x] Hearth Games › Recomps: native PC ports of console games (Zelda 64: Recompiled, Ship of Harkinian, Unleashed Recompiled…) get their own tab next to GOG, Epic and Battle.net: browse by console, search, Installed and Updates; each port's page says which build this PC gets, its version and patch notes; Install (with a drive choice), "Needs your copy of …" with the file chooser and the copy check (and Use it anyway), progress and Cancel, Update, Uninstall (your copy of the game stays) and Play. Settings › Games & stores › Recomps opens it; installed ports still show up in Steam Game Mode (needs testing on a device)
- [x] Hearth Files: the **Briefcase**, the Windows 95–XP way, made new. A Briefcase is a folder (~/Briefcase in the navigation pane; New › Briefcase makes more) that travels to a USB stick, a shared folder or another Hearth device. Drag files and folders in, or Add to Briefcase: each one remembers its original (a folder on this PC, a USB drive found again by its ID under any name, or a shared folder from Your devices) and shows Up-to-date, Newer in Briefcase / at original, Changed in both, "Plug in STICK" / "Desk is offline" (never called an orphan just because it's away) or Orphan. Update All / Update shows every action first (Replace →, ← Replace, Skip, Keep both, "Deleted at original: keep or delete?") and you can change each row; copies are atomic, replaced versions go to the Trash on their own side (a USB stick's own Trash; a shared folder's into .briefcase-replaced), a file changed in both places is never decided for you. Split from original, Find original, Send Briefcase to… a USB drive; a Status column and badges; everything with the controller (needs testing on a device)
- [x] Hearth Files: **locked Briefcases**. Lock this Briefcase… encrypts its files inside the folder with a password you choose (gocryptfs, so a lost USB stick gives nothing away; it opens with the same password on another Hearth device), checks the encrypted copy, and only then moves the unlocked copy to the Trash; Remove lock does the reverse just as safely; Change password. Unlock / Lock in the command bar and the menu, a padlock on the icon, "Remember on this device" (the keyring, off unless you tick it). It locks by itself after the time you choose in Customize (Never, 1 minute … 4 hours or your own; 15 minutes when you add a lock; counted from the last time something in it was opened or changed, never while a file in it is open, with a "will lock in 1 minute" notice and Keep unlocked), when the PC sleeps and when the USB stick is ejected (each a switch), and when you sign out. "Locks in 12 min" in the details pane (needs testing on a device)
- [x] Hearth Files: **Briefcase auto-sync**, on (Right away) for every new Briefcase: a change is copied a few seconds after it's saved (once the file is left alone and closed), or every 5 / 15 / 30 minutes / your own, When I lock it, or Off. Only one-sided changes and new files sync by themselves; "Changed in both" and deletions always wait for you ("2 files in 'School' need you", with Open); an unplugged drive or an offline device waits quietly and syncs when it's back; a locked Briefcase waits until it's unlocked; network syncs are throttled and it pauses while a game runs. "Synced 2 min ago" / "Syncing…" in the details pane and status bar. A small user service that starts after sign-in only when a Briefcase wants it, never holding up sign-in or Game Mode (needs testing on a device)
- [x] Hearth Files: **Customize each Briefcase** (name, colour, icon: classic briefcase, satchel, suitcase, backpack or box, a badge, a note in its details pane, its own view and sort, auto-sync, auto-lock, show in the navigation pane), kept in the Briefcase itself so it looks the same on a USB stick or another device; **move, copy or swap items between Briefcases** (drag, Move to / Copy to Briefcase ›, Swap with…), keeping each item's link to its original; a locked one asks for its password first (needs testing on a device)
- [x] Windows apps: Unigine Heaven (and any installer whose name doesn't say "setup") works as an installer. Hearth 1.7.0 took Unigine_Heaven-4.0.exe for the program itself, so every start ran the installer again; installers are now also recognised by what's inside them (Inno Setup, NSIS, InstallShield, WiX), and the app becomes what the installer's Start menu shortcut starts, with its arguments and folder (Heaven's launcher needs both). An app made by 1.7.0 is repaired at its next start. Proton now starts programs the way umu-launcher does outside Steam (not through Proton's steam.exe, which wants the Steam client), .bat files run through cmd, every start and install is logged (the app's logs folder keeps the last 5 with Proton's and the program's output, exit code and time; one line in Hearth's log), Settings › Windows apps shows the last result with Show log and a "Detailed Proton log for the next start" switch, Send logs has a Windows apps section (apps, last logs, Protons, Vulkan), and an app's own icon shows in Mantel's menus (a picture file in Icon= was looked for as a Hearth icon). CI runs real 64-bit, 32-bit, .bat, installer and Direct3D 11 programs through the image's GE-Proton (needs testing on a device)
- [x] Desktop Mode: Hearth's own apps (Games, Mods, Saves, Controller Lab, Disks, Contacts, Time, Maps and the rest) are the bar's size again. Their text grew with the window's width (made big for Game Mode and TVs), so in a desktop window it was about 1.4 times the bar's text and nearly twice it maximized. On a desktop it now matches the bar (14 px, times your Settings › Display › Scale, which the bar and the apps both follow); Game Mode and apps opened from it keep the big sizes. Desktop Mode's CI test measures four apps next to the bar and screenshots them (needs testing on a device)
- [x] Server mode: the web page (port 8460) lets you type your PIN again. It told browsers to send no referrer, so they marked every sign-in as coming from nowhere and the PC refused it as "Not from this page" (1.6 and 1.7). Settings sets the PIN with a PIN pad that works with only a controller (Server mode's Settings has no on-screen keyboard), every failure says what to do (wrong length, letters, a folder that isn't yours, the page not starting, Tailscale not connected, no PIN yet), and a typo isn't counted as a wrong PIN (needs testing on a device)
- [x] Hearth Files: your devices' shared folders open again. The image had no SMB support for Hearth Files (gvfs, gvfs-fuse, gio, secret-tool), so shares were listed but none would mount; a share that's "Only through Tailscale" is now opened through Tailscale even at home (its Samba refused the home name as if the password were wrong); a Tailscale name that can't be looked up is opened by its Tailscale IP; and every failure says why and what to do (wrong password, no sharing password set there, share off, asleep, Samba not running or refusing, Tailscale off, firewall). The sharing PC opens port 445 in ufw/firewalld when one runs and shows smbpasswd's own reason. A real Samba + gvfs test runs in CI (needs testing on a device)
- [x] Mods: Play through Hearth Mod Organizer (a Wabbajack list, or your own instance) now ends when the game does. It used to wait forever for the overlay's own helper (fuse-overlayfs), so the mods stayed laid over the game and every later Play, from Game Mode too, said "The game is running through the organizer: close it first" (the owner's Tuxborn log). CI now runs the real overlay test (needs testing on a device)
- [x] Mods: a deploy from before a restart is never taken as still running (the process number may belong to another Hearth Mods after a restart) (needs testing on a device)
- [x] Wabbajack lists: their Proton folder is set up with Hearth's own umu-launcher (Hearth OS has none on the system, so the Windows parts these lists need were never installed) and with the Proton every later start uses: the GE-Proton built into Hearth OS for a new list, kept for that list; a list set up before keeps the Proton it was made with. Mod Organizer 2's fallback entry names umu and the Proton in full (needs testing on a device)
- [x] Mods: modded starts pick their Proton the way Hearth Games does (the Proton detector: what worked, Hearth's table, the built-in GE-Proton), keep a Steam game's own Proton for its Steam folder, and log Proton without its two heaviest channels (they slowed games with many SKSE plugins down) (needs testing on a device)
- [x] Wabbajack lists: taking one out of Hearth (Forget) takes its Hearth Mod Organizer instance and its Add to Steam entries out too (refused while its game runs; mods laid over by hand are taken away first), so nothing is left pointing at a deleted folder (needs testing on a device)
- [x] Desktop Mode: Alt+Tab's list no longer has the "icon" field the image's labwc rejects at every start ("bad windowSwitcher field 'icon'") (needs testing on a device)
- [x] Send logs: "System version" no longer runs bootc without root or rpm-ostree on Debian (the two errors in every report); a new "Updates and the automatic fallback" section shows the fallback's count, whether it went back, and what the fallback, ostree's finalizing and the boot menu said this start and the one before (needs testing on a device)
- [x] Automatic fallback: coming back from hibernation ("Turn off and keep games", "Restart and keep games") isn't counted as a start any more. GRUB counts the way back; two hibernations left the count at 2, so the next real start (the one applying an update) went to the previous version and made it the default: one way a downloaded update never starts (needs testing on a device)
- [x] Hearth's Game Mode on PCs without Handheld Daemon (laptops, desktops): no more asking HHD for its overlay app every few seconds; Quick Access opens Hearth's own straight away (needs testing on a device)
- [x] On-screen keyboard: no warnings for its settings files before anything is saved (real read errors still show) (needs testing on a device)
- [x] Home server AI service: one that can't run on this PC stops after 5 tries in 10 minutes instead of restarting every 10 seconds forever; starting it from Settings is a fresh try (needs testing on a device)
- [x] Mods: Repair (a Wabbajack list's Proton folder) runs on its own and the list's card shows how it goes: installing the Windows parts takes many minutes, so the call used to fail with "took too long to answer" while it went on (the owner's "Repair just errors out"). winetricks' output goes to the list's Log, and every problem says what to do (offline, umu-launcher not fetched yet, the list's folder gone) (needs testing on a device)
- [x] Game Mode's Mods screens for a Wabbajack list show its mods (Hearth Mod Organizer's list) instead of an empty one, answered from a cache that's made again when the list changes, so big lists stay quick (needs testing on a device)
- [x] Steam Game Mode: Steam's own session switch (steamos-session-select) is written to the session log with what asked, and only the desktop sessions SteamOS knows switch to Desktop Mode (before, any unknown or empty request did). Right after the PC starts, Steam waits up to 20 s for NetworkManager to bring Wi-Fi up (the laptop's Steam started offline at every start: "LogonFailure No Connection"), never longer, and Steam Game Mode always starts (needs testing on a device)
- [x] Desktop Mode: an icon that's hidden loads nothing (hundreds of "Cannot open icons/<app>-light.svg" warnings in every log, from icons kept hidden next to each app's own picture) (needs testing on a device)
- [x] Start-up: systemd-networkd's network wait (2 minutes at every start, though NetworkManager runs the network), Nvidia's persistence daemon on PCs without an Nvidia card, and Debian's system-wide speech-dispatcher no longer run (and fail) at every start; each only runs where it can (needs testing on a device)
- [x] Hearth's gaming apps (Mods, Saves, Controller Lab): a settings file that's damaged is kept aside as <name>.damaged-<time>, never written over (needs testing on a device)
- [x] Nexus Mods (free accounts): Hearth Browser hands "Mod Manager Download" (nxm://) links straight to Hearth Mods, without Firefox's "open with" question, which was hard to answer with a controller in Game Mode (needs testing on a device)
- [x] Send logs: "Hearth Input not running" outside Game Mode and InputPlumber missing on PCs without it are no longer listed as problems (needs testing on a device)

## 1.7 (released 2026-11-07)

- [x] DNS can't get stuck: /etc/resolv.conf is linked back to systemd-resolved at every start even if something replaced it, and NetworkManager hands DNS to systemd-resolved directly (the owner's laptop, after a freeze: Wi-Fi connected, every site "could not resolve", restarts didn't help) (needs testing on a device)
- [x] **Hearth Games** (stage one, docs/HEARTH-GAMES.md): Hearth's own store app for GOG, Epic and Battle.net (our own, not a Heroic fork), on the same engines Heroic uses (gogdl, legendary) through Hearth's store engine, and Faugus for Battle.net: sign in on the store's own page, your library as covers (filter by store, installed, search), install on the drive you pick, downloads with pause / carry on / cancel, Play (the same start as the game's Steam Game Mode shortcut, so it shows there through Hearth Sync), uninstall; Hearth look, controller-first; Settings › Games & stores opens it (needs testing on a device)
- [x] Hearth Games' engines in the image: gogdl and legendary in /usr/lib/hearth/tools, the same pinned releases and checksums as Hearth's (no download before the first install) (needs testing on a device)
- [x] Windows games with no Proton picked start with the GE-Proton built into Hearth OS when the image has one (offline), else UMU-Proton as before; the Proton detector plugs in at proton_pick.rs auto_for (needs testing on a device)
- [x] **Windows apps** with Proton built in, offline (docs/WINDOWS-APPS.md): GE-Proton11-3 is in the image (pinned with its SHA-256; the build stops if it can't get it), linked into Steam's system folder for compatibility tools without touching Steam's settings; a .exe or .msi opened from Files (or right-click › Open with Hearth, or the app launcher's Open with) runs in its own prefix in ~/Games/Windows apps/<name>, installers first, then their program becomes the app with a menu entry and the program's own icon; Settings › Apps › Windows apps: open, Proton, DXVK, VKD3D, Windows version, In Steam (through Hearth Sync), remove; older GE-Protons downloaded only when needed, after you say yes, with a progress bar (needs testing on a device)
- [x] **Proton detector**: picks the Proton for each Windows app and each Hearth Games game you haven't picked one for: what it last worked with on this device, then Hearth's bundled table (/usr/share/hearth/proton-rules.json, no live lookups), the built-in GE-Proton, Valve's newest Proton from Steam, then older GE-Protons; a Windows app that closes with an error within 15 seconds gets the next one offered (Try with …) and the one that works is remembered (needs testing on a device)
- [x] **Server apps catalog** (docs/SERVER-APPS.md): Settings › Home server › Server apps: one-click Minecraft, Valheim, Terraria and Palworld servers, and your own Vaultwarden (passwords), Immich (photos), Navidrome (music) and ntfy; each off and not installed until you ask, in your own rootless Podman as your own user service (never part of starting the PC or Game Mode; four failed starts and it stays stopped), home network and Tailscale or Tailscale only, its options and password, logs, update, remove (data stays unless you confirm twice) (needs testing on a device)
- [x] **Phone notifications** (ntfy): Home server alerts and server apps that stopped, on your phone; off until you turn it on, a random private topic, ntfy.sh or your own ntfy, Send a test (needs testing on a device)
- [x] **Your server on your other devices**: a PC using Server mode shows its dashboard in short in Settings › Hearth Link › Your devices (Its server, Open its dashboard) and in Steam Game Mode's Quick Access › Hearth Link (Your server, for the Ally); Server mode's dashboard shows a Server apps pill (needs testing on a device)
- [x] **Connect network drives automatically**: a "Connect automatically" switch per share in Files (its menu: right-click, press and hold or the menu button); those shares open by themselves 30 seconds after signing in and every 3 minutes after, in Steam Game Mode and Desktop Mode, with the password the keyring remembers; asleep or offline devices are skipped quietly and picked up when they're back, one that needs signing in waits until you open it; nothing in signing in, Game Mode or Steam waits for it (docs/HEARTH-LINK.md) (needs testing on a device)
- [x] **Hearth Files redesign**, laid out like Windows 11's File Explorer in Hearth's look (dark and light): tabs along the top, Back / Forward / Up / Refresh, the address bar and Search, a command bar (New, Cut, Copy, Paste, Rename, Copy path, Delete, Sort, View, Show, Undo, Redo, Details), the navigation pane (Home, Favorites, This device, Games, Drives, Hearth Storage, Your devices, Trash), a Home page with Quick access and recent files, and a details pane. New too: undo and redo (Ctrl+Z / Ctrl+Y) for rename, new, Move to Trash, copy, move and links; Large, Small and Tiles views next to Medium, List and Details; a file name extensions switch; Alt-drag makes a link; the drag shows a picture. Everything it already did stays (multi-select and rubber band, drag and drop, copy with progress and name prompts, compress, mount and eject, shares, controller, touch and on-screen keyboard) (docs/HEARTH-FILES.md) (needs testing on a device)
- [x] **AI video** in Hearth's ComfyUI: Add video in AI pictures' settings gets the GGUF loader and Wan 2.2 14B sized to the graphics card (Q4_K_M from 12 GB, Q3_K_M from 8 GB), with its text encoder and VAE, each file checked by size and SHA-256 before it's used, resuming where it stopped; picture to video only (about 25 GB) or both (about 45 GB); Remove takes away only what Hearth downloaded (docs/HOME-SERVER.md) (needs testing on a device)
- [x] The controller works on Server mode's dashboard: 1.6 switched it off as it started, so only Guide did anything (A, B, X, Y, the D-pad and ☰ work again); and the controller helper starts again if it ever closes (needs testing on a device)
- [x] Server mode's settings page can be found: a Server mode card on Settings › Home server, and the greyed-out Server mode in System › Startup opens it (1.6 only reached it through Settings' search) (needs testing on a device)
- [x] Settings › Wi-Fi no longer says "Not connected" while you are connected: with a mesh (one network name on several access points) the page could keep the wrong one, and right after start-up it could miss the link; it now also asks NetworkManager which network is up (needs testing on a device)
- [x] The website's live count works: every Hearth OS device's check-in was turned away by Cloudflare (it didn't like Python's default name for itself, "error code: 1010"), so only the Command Center was counted; the check-in now names itself Hearth OS

## 1.6 (released 2026-11-06)

- [x] Desktop Mode: Super+Up / Super+Down work again (Up maximizes, or makes a snapped half a quarter and a quarter its half again; Down does the opposite, then restores a maximized window, then minimizes). labwc refused the old keys ("action 'If' cannot be a child action"); a test now catches that (needs testing on a device)
- [x] The installer test runs again: both jobs install Hearth OS on Debian (the one next to SteamOS still asked for the archived Anatase) and the boot check refreshes apt before getting qemu (it stopped with exit code 100); failures say why on the run page
- [x] Tiles and Start screen app menus: move tiles, by dragging with the mouse, or Y (or Move in a tile's menu), the D-pad and A to drop (B puts it back); saved for you only when you drop one somewhere new. The hints say what X and Y do (needs testing on a device)
- [x] Tiles app menu: its search field sits in the bar next to the flame, as in the mockup; what you type shows there (needs testing on a device)
- [x] The app menus' Updates tile shows Steam games with an update waiting (Hearth's games list now says so; other stores don't report it) (needs testing on a device)
- [x] Settings › Network & internet › Wi-Fi: the network you're on at the top (name, signal, band, address; Properties, Disconnect, Forget), and Add a hidden network (needs testing on a device)
- [x] Settings › Hearth Link › Your devices (and a card on Settings' Home): your other Hearth devices' online or last seen, Game Mode / Desktop Mode / Server mode, what's playing, battery, CPU / GPU use and temperature, memory, graphics memory, drives, version and uptime, with the last hour in small graphs, their shared folders, Wake it and Remote desktop. Only over Hearth Link between paired devices; "Let my other devices see this device's status" turns it off (needs testing on a device)
- [x] Hearth Files shows your devices' shared folders by itself: a "Your devices" section in the sidebar (and the Open / Save windows) with each paired Hearth device's shares, other computers sharing at home, and Tailscale devices that share; one click opens one (the home name at home, the Tailscale name away), signing in once with Remember in the system keyring; clear words for a wrong password, a share that's off, a device asleep (Wake it) or Tailscale signed out; "How to open from Windows or a phone"; a "Show other devices in Files" switch (needs testing on a device)
- [x] Settings: Sound, Notifications, Power & battery, Date & time, Mouse & touchpad, Printers, Apps, Default apps, Controller and Handheld in titled groups of cards with icons, as in the mockups (nothing they do changed) (needs testing on a device)
- [x] Server mode (docs/SERVER-MODE.md): a third mode next to Steam Game Mode and Desktop Mode, its own light session (a small labwc with one full-screen dashboard: status, Play, a tile per service, alerts), off until you turn it on in Settings › Home server › Server mode; your services keep running as their own services and the dashboard shows and controls them (needs testing on a device)
- [x] Server mode always ends in Game Mode when something's wrong: off or missing files, the dashboard not appearing within a minute, closing three times right away, or failing twice in 10 minutes (then paused until you open it); with it off, Game Mode and Desktop Mode start exactly as before (needs testing on a device)
- [x] Server mode's Play button and Guide: a 5-second countdown showing what pauses and what keeps working (A now, B cancel), then Steam Game Mode; leaving Game Mode comes back to Server mode with "Back in Server mode"; Desktop Mode from the dashboard's menu (needs testing on a device)
- [x] While you play, AI chat switches to the small model (qwen3:14b on the processor) so your other devices still get answers, and the big one loads again when you're back; ComfyUI finishes its picture and keeps its queue (Light mode's own settings; models are never deleted) (needs testing on a device)
- [x] Settings › System › Startup: start in Game Mode, Desktop Mode or Server mode, and with Server mode an optional schedule (weekdays and the weekend); no schedule unless you make one, and a change never closes a game or leaves Desktop Mode (needs testing on a device)
- [x] Server mode stays awake, with lower power limits when nothing's busy (power-saver, the graphics card's limit) and the screen dimming to a clock, then black (needs testing on a device)
- [x] Server mode's web page over Tailscale (port 8460, phone and laptop layouts): status, pause and restart services, switch modes, sleep or restart; PIN protected (only its hash kept), Tailscale and this PC only, every permission switchable; a switch from the web counts down 10 seconds on the home screen (needs testing on a device)
- [x] Home server › AI chat: an "Uncensored (18+)" group behind "Show 18+ models" (off by default, so a family PC never shows them): Qwen3 30B-A3B and 14B abliterated, Dolphin 3 and Rocinante 12B (Mistral Nemo, for roleplay and writing), each name checked; "Download any model by name" (Ollama names or hf.co GGUFs, validated); the abliterated 14B can be the small model while you play (needs testing on a device)
- [x] Home server › Shared folders: share a whole data drive (also from Drives & undo), your whole Home (read only, one confirmation; read & write a second) or the system drive (read only only, two confirmations, its secrets blocked), each on your home network and Tailscale by default with a "Whole drive" / "Home" / "System drive" badge; hidden files stay blocked everywhere (needs testing on a device)
- [x] A read-only summary for your other Hearth devices (/api/summary, hearth-server-mode summary) for Hearth Link and Quick Access (needs testing on a device; a Hearth Link page for it is next)
<!-- Discord replaces Hearth Friends (feature/discord-friends) -->
- [x] Discord replaces Hearth Friends (Matrix, archived with the owner's okay, docs/ARCHIVED.md): the bar's Friends button is now **Discord**; the first press asks which Discord you want (the official app, recommended; Discord in a web window, nothing to install; or Dorion, lighter but unofficial, with its risk said plainly) and installs only what you pick when you press Install (docs/DISCORD.md) (needs testing on a device)
- [x] Discord's small window by the bar, on whichever edge the bar is: Discord's own window, so you reply to friends there; pressing the button again hides it without quitting (notifications keep coming); its size is remembered; Open full Discord makes it a normal window (needs testing on a device)
- [x] Settings › Gaming › Discord: which Discord, "Playing <game>" on Discord with a one-step setup (paste the application ID) and an on/off switch, Let friends ask to join, and an invite link to paste in a chat (needs testing on a device)
- [x] Ask to join / Join from Discord: a friend joining opens the game's page on their Hearth (Play / Install), or Play together (Sunshine to Moonlight, over Tailscale or the local network) with a one-time PIN shown on both screens that expires in 10 minutes (needs testing on a device)
- [x] Steam Game Mode: Quick Access › Hearth Link › Discord (open Discord, the switches, the PIN, setup), and Discord's notifications as toasts (needs testing on a device)
- [x] Hearth Friends' Matrix service is no longer built, shipped or started; your old Matrix data and settings are left where they were, untouched (needs testing on a device)
<!-- end Discord -->
- [x] Fix: GOG games (and other games Hearth starts with Proton, like Epic's) didn't show up in Steam Game Mode: they started, but Steam Game Mode only shows windows carrying the number Steam gave the shortcut, and umu labelled the game's windows with its own. Hearth now gives the game's windows the shortcut's number, so the game comes to the front; existing shortcuts are fixed by themselves at the next sync. A game Steam is set to run with a compatibility tool (Properties › Compatibility) can't start either: Hearth Sync's panel now names those games and says how to turn it off (your setting, left as it is). Every failed start says why in Hearth's log ("GOG Tomb Raider: launch failed because …") and Send logs shows Proton's output of the last starts and Debian Steam's own logs (needs testing on a device)

## 1.5 (released 2026-11-06)

- [x] Hearth Settings redesign (the approved mockups): opens on a new **Home** page (this PC with Rename, Wi-Fi / Bluetooth / Updates / Hearth Link at a glance, recommended settings, Bluetooth devices, storage, the home server, your look); 12 categories instead of one long list of 42 pages (System, Network & internet, Bluetooth & devices, Hearth Link, Personalization, Apps, Gaming, Accounts, Home server, Privacy & security, Accessibility, Updates & recovery); setting cards in titled groups with the control or a status chip on the right and a chevron only where something opens; expander cards (the on-screen keyboard, resolution, more accent colours, the wallpapers, each home server service's details); breadcrumb titles ("Home server › Services"); warning banners (Home server's "1 thing needs a look") (needs testing on a device)
- [x] Settings search finds single settings with where they are ("Go to sleep after · System › Power & battery"), pages, and the old page names ("Appearance" still finds Personalization); opening a result goes straight to that setting (needs testing on a device)
- [x] Settings with the controller: A select, B back up one level, Y search, LB / RB previous / next category, ☰ close, shown at the bottom, and a big focus ring; a handheld layout for 1280×800 screens (needs testing on a device)
- [x] Every old Settings page has one home (Tailscale in Network & internet with a link from Home server, Remote play in Gaming, AI chat under Home server › Services, the boot splash in System › Startup and the startup animation in Gaming, Night light in Display, Reduce motion in Accessibility, Start in in Gaming and on Home, the device name on Home, the player count in Privacy & security, backup and logs in Updates & recovery); every old page link still opens; opening any page changes no setting (checked in CI for every page) (needs testing on a device)
- [x] New installs start with the Steam-style look and the Sky accent; existing installs keep their look and accent (a default only, never written) (needs testing on a device)
- [x] If the new Settings frame can't load, Settings opens a plain list of every page instead, so it always opens (needs testing on a device)
- [x] Settings › Network & internet opens on Wi-Fi itself (as in the mockup), with Tailscale under "Reach your devices from anywhere"; Updates & recovery gets the mockup's layout: a status card ("You're up to date" with Look for updates, or the update that's ready with Download / Restart now), Decky plugins folded into one expander card, "If something broke" with Go back and a reminder that the previous version always stays startable, then Backup and help (needs testing on a device)
- [x] Remote play, Bar and dock (bar layouts and app menus), Startup (boot splash and its previews) the AI pictures and voices services and AI devices (Home server › AI devices) all sit in their new places; `hearth-settings appearance:bar` and the other old page links open the new place; CI opens every page, every category, every old name and every page part and checks that no settings file changed (needs testing on a device)
- [x] Use this PC from anywhere (docs/REMOTE-DESKTOP.md): one switch in Settings › Remote play for low-latency remote desktop to this PC from work or anywhere, with Moonlight through Tailscale (Sunshine with NVENC's fastest preset on Nvidia, VA-API elsewhere, audio, the Desktop app, never UPnP); only through Tailscale by Hearth's own firewall rule unless you choose your home network too, never the internet; off by default, nothing enabled in the image (needs testing on a device)
- [x] Use this PC from anywhere: pairing made easy ("In Moonlight, add <name> or <address>", type Moonlight's PIN in Settings, paired devices with Remove; Sunshine's login made on the PC, 0600, used only on this PC; works with newer Sunshine's pairing too) (needs testing on a device)
- [x] Use this PC from anywhere: who is connected, resolution, frame rate and bitrate, direct or through a relay (with a plain tip when your network blocks direct connections), a recommended Moonlight setting, Check the connection (one Tailscale ping); stay awake while connected; a link to waking the PC from away (needs testing on a device)
- [x] Use this PC from anywhere: Game Mode keeps working while someone is connected (the stream shows Steam's Game Mode), and Game Mode's Quick Access says who is using the PC (needs testing on a device)
- [x] Desktop Mode controller: clicks work everywhere again. RT / LT always click and right-click (also in Hearth Files, Settings, Task Manager, Open / Save dialogs and Hearth's menus, where they did nothing), A and X click once you've moved the pointer with the right stick (the D-pad gives them back to Enter / the Menu key), the pointer holds still for a moment when a click goes down so the title bar's close button and dragging a window by its title bar work, the menus' keys mode can no longer get stuck on, quick taps are never lost, and pad.log shows the clicks and the keys mode (docs/HEARTH-INPUT.md) (needs testing on a device)
- [x] Home server: AI pictures (ComfyUI) in Services and on the AI page: installed only when you ask (ComfyUI's release from GitHub with its commit checked, PyTorch's CUDA 12.8 builds for the RTX 50 series, its requirements) into ~/ComfyUI, or your own ~/ComfyUI adopted with "Take over" (your comfyui.service is switched off, never deleted, and Give back undoes it); on/off, only through Tailscale and on this PC, Open, Models folder (no model is ever downloaded), status, Update, Remove (your models stay unless you confirm twice; pictures and workflows always stay); stopped while you play so the graphics card is yours (on by default) (needs testing on a device)
- [x] Home server: AI voices (Kokoro-FastAPI) in Services and on the AI page: installed only when you ask, in rootless Podman by the exact image Hearth pins (Podman checks it), on the CPU; on/off, only through Tailscale and on this PC, the address to paste into the Command Center (Settings › Voice › Speech servers › Kokoro-FastAPI) with Copy, status, Remove; your own "kokoro" container taken over only when you say; can stop while you play (off by default). Both in the Overview's Services count and Game Mode's Quick Access line. git and python3-venv in the Debian image (needs testing on a device)
- [x] Settings › Boot & startup shows every boot splash and startup animation as a card with its picture; picking one plays it in the window (the splashes drawn from their own boot scripts, never by starting the real boot splash; the animations as a flip book, or the full video on request). Looking changes nothing: only Use this chooses. Works with a controller (needs testing on a device)
- [x] Three new startup animations for Steam Game Mode, for 16:9 and 16:10: Aurora (northern lights over a night sky), Starlight (stars join up into the Hearth arch, then the flame lights) and Ember orbit (three embers spiral into the flame and keep circling it) (needs testing on a device)
- [x] Three new boot splashes that go with them: Aurora, Starlight and Ember orbit. The default splash and the one you picked before stay as they were; each new one is checked in CI by running its script at several screen sizes with the disk password prompt, and started in a virtual PC (needs testing on a device)
- [x] Matching sets: a new splash and its startup animation chosen together in one press ("Use with its startup animation"), only when you press it (needs testing on a device)
- [x] Mantel bar layouts: besides "Top bar + floating dock" (still the default), "One main bar" on the bottom, top, left or right that holds everything (the Hearth button, your apps, desktops, the window in front, the clock, the status icons, notifications, Quick settings and Game Mode); left and right stack the icons with a compact clock, and menus open toward the screen. "Floating app menu" on or off (off puts the app menu button and your apps in the main bar, no dock). Auto-hide and the space windows can use work on every edge, with the Game Mode return fix; changes apply straight away; the classic taskbar still comes up if the new bar can't load. Settings › Appearance › Bar and dock (docs/MANTEL.md) (needs testing on a device)
- [x] Mantel app menus: pick one of five in Settings › Appearance › Bar and dock › App menu (picture cards): Hearth (today's), Centred (pinned apps in pages, Recommended, you, Settings and Power), Tiles (a rail, an A–Z list with letter jumps, resizable live tiles), Start screen (full screen, live tiles in groups that scroll sideways, All apps, full-screen search) and Classic (two columns, frequent apps with jump lists, All apps in place, your places, Shut down with an arrow, glass in Hearth's colours). Every one has Mantel's search (apps, games, Settings pages and files), controller hints and moves, and works in all eight looks. Live tiles show what's playing, the weather, your calendar, the Home server and game updates when Hearth has them, and are saved per user when you resize one. Picking Tiles or Classic asks Match the menu or Keep mine. New handheld installs start with the Start screen; existing devices keep the Hearth menu, and nothing is written until you choose (needs testing on a device)
- [x] Share this device's AI (docs/AI-SHARE.md): Settings › Home server › AI devices (also from AI and Hearth Link): Share this device's AI (off by default, per device), which services to share (Ollama chat, Kokoro voices, ComfyUI pictures, whisper speech to text, found by asking them), Share only when plugged in and Not while I'm playing (both on by default), an optional share code (only its hash kept); a read-only answer at http://<Tailscale name>:8189/v1/hearth/ai (protocol 1: device, version, playing, busy, battery, services with models and voices), only through Tailscale behind Hearth's own firewall rule (needs testing on a device)
- [x] Share this device's AI: the Home server lists every Hearth device's AI (online, playing, plugged in, models, voices, picture engine; devices seen before and paired Hearth Link devices too) at /v1/hearth/ai/devices for the Hearth AI Command Center, with the Command Center address to copy and the order it tries them in (Nvidia PC first; Move up / Move down with a controller) (needs testing on a device)
- [ ] Share this device's AI, later: Hearth Link's ai.info answering with the same document (list paired devices without Tailscale); a way to share a service that only listens on its own device

## 1.4 (released 2026-11-05)

- [x] Mantel dock: coming back from Game Mode, an auto-hide dock no longer keeps the bottom of the screen reserved, so windows can fill the whole screen again (it waits for your settings, and refits when its space changes) (needs testing on a device)

- [x] Home server, stage 1 (docs/HOME-SERVER.md): a new Home server group in Mantel's Settings with Overview (Light mode while you play, at a glance, Show hidden items), Shared folders (Samba with SMB3 and no guests: read only or read & write, home network or Tailscale only, a 30-day recycle bin, your own sharing password, how to open them; nothing shared until you share it), Drives & undo (space, temperature and SMART health, btrfs snapshots hourly/daily/weekly with Browse & restore into a new folder, adding a drive as more storage, a mirror or a backup target with two confirmations naming it; the system drive is never picked), Tailscale (QR sign-in, your devices with ping, Only reachable through Tailscale with firewall rules, start at boot, exit node, Taildrop, wake from away) and the setup steps with the controller. Builds on Hearth Storage. Samba, smartmontools, nftables and Tailscale (its repository's key checked) in the Debian image, every service off until you turn it on (needs testing on a device)
- [x] Home server, stage 2 (docs/HOME-SERVER.md): Backups & sync (restic backups of the shares you pick to a backup drive, another Hearth device over Tailscale or the cloud (Backblaze B2 or S3, through rclone), encrypted with your password; hourly, daily or weekly with keep counts, Back up now, the last run and a weekly check, Restore always into a new folder; backups from your other Hearth devices with a code each, phone photos, folders kept the same between Hearth devices with Syncthing over Tailscale; Light mode makes them wait while you play) (needs testing on a device)
- [x] Home server, stage 2: People (Samba-only accounts with no login, per-share No access / Read only / Read & write, their own folder with a size limit on btrfs, passwords only to smbpasswd, removing a person never deletes their files) (needs testing on a device)
- [x] Home server, stage 2: Alerts & power (drive health, a drive almost full, a failed or late backup, a mirror missing a drive, as notifications and optionally Taildrop to your devices; sleep when nobody needs it but never while playing, sharing or backing up, stay awake while used, Wake-on-LAN, a nightly sleep and wake schedule, spinning down idle data drives, a UPS through NUT shown once one is plugged in) (needs testing on a device)
- [x] Home server, stage 2: the Game Mode Quick Access tile (in the Hearth Link plugin: the server in short, Light mode while you play, let paused jobs run now; hidden when the helper isn't there) and the setup steps' First backup pointing at Backups & sync. restic, rclone, Syncthing, hdparm and NUT in the Debian image, every service and timer off until you turn it on (needs testing on a device)
- [x] Home server, stage 3: Services (the media server Jellyfin, also your photo library, and the AI service Ollama, each downloaded only when you ask from its maker's release with the SHA-256 checked, never in the image; Files on the web with your web password; Remote play, folder sync and the game archive in one list; each with a status, Open and home network or Tailscale only) (needs testing on a device)
- [x] Home server, stage 3: AI ("AI voices, pictures & chat"): Ollama for the Hearth AI Command Center's "Your own GPU server", only through Tailscale or on this PC (never without Hearth's firewall rule); qwen3:30b-a3b (recommended), qwen3-coder:30b-a3b, gpt-oss:20b and qwen3:14b downloaded only when picked; the address to paste; models unloaded (or the service stopped) while you play; Command Center items hidden until it's seen (on this PC, or using the AI service from one of your devices); voices a documented placeholder (needs testing on a device)
- [x] Home server, stage 3: Files & links (search every share, smart folders: recent screenshots, big files, duplicates, not changed in a year, with a clean up that only moves files to the recycle bin and Put back; changed this week; space per share; share links with an end date, password and most downloads; a drop-off folder; Tailscale only unless you choose your home network) (needs testing on a device)
- [x] Home server, stage 3: Gaming (a Games folder for your Hearth devices, Steam's own LAN transfers and the game archive, Remote play over Tailscale, what Light mode holds back) and Family & home (a Family folder for everyone, People, cameras found on your network shown only once one is found) (needs testing on a device)
- [x] Home server, stage 3: Safety (who can reach what, open ports, Tailscale only, the last backup, SMB3 and no guests; fixes that say what they change and Undo; the ransomware guard: shares turn read only when far too many files are renamed or deleted at once, you're told, and one step makes them writable again), the Overview's new tiles and links, and the AI service's state in Game Mode's Quick Access tile. Every new service and timer off until you turn it on (needs testing on a device)
- [ ] Home server, later: parental controls (play time), ad blocking for the home, a shared calendar server, recording cameras, a photo library app (Immich), game servers

## 1.3 (released 2026-11-04)

- [ ] Mantel 2026, the desktop's facelift: floating top islands (the Hearth mark, desktops, the window in front, a centre clock pill that grows for what's playing and new notifications, the status island and a big Game Mode button) and a floating dock with every taskbar behaviour; Quick settings with big tiles, pill sliders, Bluetooth, Do not disturb and battery time left; a centred launcher whose search also finds Settings pages and games; a notification centre with a dashboard (calendar, media, weather, system); Settings with search and groups; Appearance with eight looks as picture cards (Hearth, Steam-style and the new Gaming, Zen, Glass, Synthwave, Cabin and Console), Bar and dock, From wallpaper, Desktop clock and weather, Panel transparency, and Sunset (off unless you choose it); handheld tuning with controller hints. Choosing a look changes nothing else; new options are never written until you change them; if the new bar can't load, the classic taskbar comes up. docs/MANTEL.md (needs testing on a device)

- [ ] Hearth Eras, a new Decky plugin from the plugin channel ("New from Hearth"): Quick Access › Hearth Eras has a gallery of home screens and looks inspired by past console eras, Hearth's own designs: **Channels** (a rounded TV-channel grid on silver with a clock tray), **Blades** (glossy panels you swipe through sideways), **Tiles** (a big-tile dashboard with a hero banner), **Crossbar** (a category bar with lists under it over slow waves), **Pocket** (a handheld-style icon row over your wallpaper) and **Hearth Classic** (the old Hearth home and look). Each has its own Home on Steam's Home and a matching skin on Steam's Library, game pages, Quick Access and dialogs, with up to three colour presets. Extras, each off until you turn it on: era sounds (made by Hearth), quick switching (hold L1: D-pad left/right changes the era, L2/R2 its colours), loading screen style, on-screen keyboard skin, styled Library pages. Steam's look is the default; every game opens Steam's own page; with nothing to show, Steam's Home stays; if Steam changes, it pauses and says so. docs/HEARTH-ERAS.md (needs testing on a device)
- [ ] Hearth Classic is retired from the plugin channel: the next plugins release removes downloaded copies and no longer offers it; its code stays for Hearth Eras' Classic era. docs/PLUGIN-UPDATES.md

- [ ] Hearth Classic round 9: "In games and the Store: Hearth side rail" (on with Hearth Classic; takes over the top bar switch's saved choice): where Steam keeps its menu window narrow, over games and on the Store its menu becomes Hearth's side rail (the old dock's look standing up, the game's card first), the library keeps the bottom bar, and the resize is tried only once per start. (needs testing on a device)

- [ ] Hearth Classic round 8: Steam's menu window is resized with the arguments Steam expects on the device (a number as ResizeTo's and MoveTo's third argument, the screen's scale), so the real bar shows in the library, on the Store and as the top bar in games, with every attempt in the diagnostics. (needs testing on a device)

- [ ] Hearth Classic round 7: sending its diagnostics works from Decky (an "https error" on 1.2.5): the system's certificates, and the system's curl (like Hearth's own Send logs) when the direct way fails; the panel and the log say why a send failed. From the owner's ROG Ally log: Steam's menu window is resized through its browser view's own bounds (the only way that can work there), the blank side menu behind the mirror is gone (Hearth Classic's page background no longer paints Steam's menu window), and over a game the menu stays Steam's own in Hearth's colours instead of a blank strip. (needs testing on a device)

- [ ] Hearth Classic round 6: "Send diagnostics to Hearth automatically" (off until you turn it on) sends Hearth Classic's own diagnostics through Hearth's Send logs when the bar falls back, the self-check fails, the mirror is used or a resize is refused, and once at the first menu opening (once per reason per start, ten a day, personal details taken out), plus "Send Hearth Classic log now" and "Last sent"; much deeper diagnostics of Steam's windows while the menu is open, to find the blank side menu and the blank top dock. docs/HEARTH-LOOK.md (needs testing on a device)

- [ ] Hearth Classic round 5: no blank menu behind the bottom bar (Steam's empty side-menu place is hidden while the bar is up), the Store keeps Steam's menu in its own place in Hearth's colours (web pages are drawn above the bar), and a self-check after resizing makes sure the bar's entries are really visible, else Steam's window goes back (in a game: Steam's menu in Hearth's colours instead of a blank strip). docs/HEARTH-LOOK.md (needs testing on a device)

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

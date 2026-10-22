# Hearth OS roadmap

Hearth OS is one controller-first UI with two homes: **Hearth** (Linux: an installable OS built on Bazzite, plus
an app/session for other distros, with **Mantel** as its desktop) and **Ember** (the Windows 11 shell).
Releases are 0.0.N; work lands on `linux-host` and goes out when it's tested.

## Done

- Game Mode in gamescope, Mantel desktop (labwc + Quickshell) with its own settings app, switching between them
- Hearth OS images for AMD/Intel (`hearth`) and Nvidia (`hearth-nvidia`), updates that roll back, live ISOs
- Library for Steam, Epic, GOG, Xbox Cloud and your own games; downloads page; in-game overlay; art kept offline
- Power modes and power limit (TDP), controller remapping, volume/power buttons, Aloha Browser install
- Website (GitHub Pages), releases that hide older ones

## 0.0.18 (released)

- [x] Game Mode ↔ Desktop switch can't freeze
- [x] Detailed system update progress (download speed, time left, install and app steps)
- [x] Ember Desktop Mode: the Windows desktop and back (shortcut, tray, Ctrl+Alt+G, Xbox button), start-in setting
- [x] Import games and files: from Windows, SteamOS / Steam Deck and other Linux, other drives and SD cards
      (Steam, Epic, GOG, Heroic, Lutris, Proton saves, personal files; copy or move)
- [ ] Fixes from 0.0.17 testing

## 0.0.19 (released)

- [x] Remove the old system: erase other systems' partitions on Hearth's drive, grow Hearth into the space
- [x] Live USB: Install next to Windows (checks, shrinks Windows, opens the installer with Use free space)
- [x] Ember: Switch to Hearth OS (make room on C:, write the installer USB, restart into it)
- [x] Mantel for Ember: Hearth's taskbar, launcher and wallpapers over Windows' desktop
- [x] Games load behind Hearth's starting screen and come forward when ready; slow Proton first starts explained
- [x] Offer to import games once on first start

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

## 0.0.21 (released)

- [x] Hearth's own installer instead of Fedora's: screens in the Hearth app (controller) and Mantel (mouse and keyboard), no-password option
- [x] Sign-in choices: sign in automatically or not, ask for the password after sleep (Game Mode lock screen too)
- [x] Ember: Desktop style and wallpaper in Settings › Shell

## 0.0.22

- [x] Hearth's own login screen (SDDM theme like the lock screen, controller works on it)
- [x] Hearth Browser, first version: tabs, address bar with suggestions, bookmarks, history, downloads, find, zoom, start page, settings
- [x] Ember: Hearth Settings window in Mantel; taskbar down the side on upright screens
- [x] Overlay doesn't flash black when opened over a fullscreen game
- [x] Ember's full Mantel looks and works like Hearth OS's: floating taskbar, launcher, Quick Access, power menu, Hearth menu (Xbox button), controls card, volume bar, fireside wallpaper, Hearth's controller layout

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

## Archived

- Hearth for Android (and Hearth Linux on Android): archived 2026-10-21; focus is Hearth OS and Ember. Code in `hosts/android`, build by hand only.

## 0.2.6 (released)

- [x] Bug reports: what the desktop controller helper receives (and whether another program holds the controller); processor and Nvidia graphics load

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

## Later


## Not started

- App pop-up notifications in full Mantel on Windows (Windows only shares them with packaged apps)

## Waiting on setup

- SourceForge project for the ISOs (then the website's download buttons go live)
- Cosign key for signing the OS images (`SIGNING_SECRET`); then turn on signature checks in Hearth OS

## Hearth Browser, next

- Controller: move between links with the D-pad in Game Mode
- Hearth Browser on Windows (Ember)

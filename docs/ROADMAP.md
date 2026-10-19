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

- [x] Quick Resume, like Xbox: starting another game puts the one you're playing aside, frozen where you left it (up to 5); kept through Sleep and Hearth restarting, and through turning off where the PC allows it
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

## 0.0.27

- [x] Black screen (with the game still playing) after Quick Access or Quick Resume: games no longer minimize themselves when something else comes in front, and Hearth shows them again if they did
- [x] A game that was starting or had just started looked frozen after closing Quick Access: Hearth now puts back the game's current window
- [x] Sliders (volume, brightness) in Quick Access and Settings: press A to change one, A or B when done; moving past one no longer changes it
- [x] Quick Resume sometimes showed the last game's frozen picture: games put aside are never the one put in front

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

- [ ] New live ISOs (AMD/Intel and Nvidia) with Install next to Windows

## Not started

- App pop-up notifications in full Mantel on Windows (Windows only shares them with packaged apps)

## Waiting on setup

- SourceForge project for the ISOs (then the website's download buttons go live)
- Cosign key for signing the OS images (`SIGNING_SECRET`); then turn on signature checks in Hearth OS

## Hearth Browser, next

- Controller: move between links with the D-pad in Game Mode
- Hearth Browser on Windows (Ember)

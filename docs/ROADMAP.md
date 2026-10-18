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
- [x] Live player counter: Hearth and Ember report in, the count shows on the website and About page (off switch in Settings › System › Privacy)

## 0.0.25

- [ ] New live ISOs (AMD/Intel and Nvidia) with Install next to Windows (no ISOs before this)

## Not started

- App pop-up notifications in full Mantel on Windows (Windows only shares them with packaged apps)

## Waiting on setup

- SourceForge project for the ISOs (then the website's download buttons go live)
- Cosign key for signing the OS images (`SIGNING_SECRET`); then turn on signature checks in Hearth OS

## Parked

- Hearth Browser (Electron)

# Hearth Recomps

**Game Mode › Store › Recomps** is Hearth's own store for native PC versions of console games: recompilations
(like Zelda 64: Recompiled, Banjo: Recompiled or Unleashed Recompiled), decompilation ports (like Ship of Harkinian,
Starship or SpaghettiKart) and other fan ports. They run as normal PC programs, not in an emulator.

You bring your own copy of the game. Hearth installs the port from its project's own GitHub releases, checks your
copy, puts it where the port looks, keeps the port updated and puts it in your Library with Play. Hearth never
downloads, hosts or links to games, ROMs or disc images.

## Using it

1. Open **Store**, press **RB** until **Recomps**. Browse with the controller; filter by console, show only
   installed ports, ports with updates or ports with a Linux build, sort them, or search (by the port's or the game's
   name).
2. Open a port. Its page shows what it is, which build this PC gets (a Linux build, or the Windows build through
   Proton when there's no Linux one), the newest version and its **patch notes** from GitHub.
3. **Install**. If a USB drive or SD card is plugged in, Hearth asks where to install it.
4. **Needs your copy of …**: Hearth's file picker opens. Choose the ROM or disc image you made from your own game.
   Hearth checks it (see below) and sets it up. If it doesn't match, Hearth says so; **Use it anyway** is there for
   versions the project may accept but doesn't list.
5. **Play** (on its page, or in **Library › Recomps**). Playtime, favourites, collections, per-game settings
   (Proton version, launch options, controller layout) work as for any game in the library.

The **⋯** button on a port's page has the rest: choose a version (older releases too), choose a download (any file
of the release), choose the program to start, skip an update, keep a version (no updates), add it to the Desktop
Mode menu, back up saves and mods, **I have it already** (point Hearth at a folder where you installed it yourself),
refresh from GitHub, hide it, and uninstall.

## Your copy of the game

Each port says what it needs, for example "Majora's Mask (USA) ROM". There are three ways a port gets it:

- **Hearth places it** (N64Recomp ports): Hearth checks your ROM exactly the way the port does (the XXH3 hash the
  port's own code checks, with the ROM in .z64 byte order) and saves it where the port keeps it
  (`~/.config/<port>/<game>.z64`). The port then starts straight into the game without asking.
- **In its folder** (Harbour Masters ports and others that look beside their program): Hearth puts a link to your
  file in the port's folder, under the name the project documents. Nothing is copied. A .cue sheet brings its .bin
  tracks along.
- **Asked on first start** (ports with their own setup, such as the Xbox 360 recompilations that extract a disc
  image): Hearth checks your file and remembers it; choose the same file when the port asks.

Hearth checks your copy against a hash only when the project documents one (in its README or its own code: SHA-1,
MD5, SHA-256 or XXH3). Otherwise it says the copy couldn't be checked. ROMs in any N64 byte order (.z64, .v64, .n64)
are checked as .z64, which is what the projects' hashes are for.

## Installs and updates

- Installs go to `~/.local/share/nimbus/recomps/<id>/app` (or `<drive>/Hearth Recomps/<id>` on a USB drive or SD
  card). Hearth keeps its own notes beside it in `~/.local/share/nimbus/recomps/<id>/`.
- Hearth picks the download for this PC from the release's files: a Linux build for this processor (x86_64 or Arm)
  first (an AppImage over an archive, the plain build over a Steam Deck one), otherwise the Windows build, which runs
  with Proton (umu-launcher, which Hearth OS has). Source code, macOS, Android and checksum files are never picked.
- The download is checked against the size GitHub gives, and against its SHA-256 when GitHub has one.
- **Updates keep your things.** A new version is copied over the old one: files that came with the old version are
  replaced, and nothing else is touched. Saves, mods, texture packs, settings, DLC folders and your game file stay,
  even when they're inside the port's folder. Nothing in folders like `saves/`, `mods/`, `dlc/` or `config/` is ever
  replaced. Hearth looks for updates to installed ports at most every 6 hours; Store › Recomps shows **Update**.
- **Uninstall** asks first. It offers to back up your saves and mods first. A folder Hearth made is deleted; a folder
  you pointed Hearth at (**I have it already**) is only forgotten. N64Recomp ports keep their saves in `~/.config`,
  which uninstalling never touches.
- **Back up saves and mods** makes `~/.local/share/nimbus/recomps/backups/<id>-<date>.tar.gz` with your files from
  the port's folder and, for N64Recomp ports, its folder in `~/.config` (without the ROM).

## GitHub without a sign-in

Hearth asks GitHub's public REST API without signing in, so it never needs a GitHub token. GitHub allows 60
questions an hour like that, so Hearth is careful:

- Browsing the catalogue asks GitHub nothing. Only opening a port's page, installing and the update check do.
- A project's releases are kept for 6 hours. After that Hearth asks again with the answer's ETag; an unchanged answer
  doesn't count towards GitHub's limit.
- If the limit is reached, Hearth stops asking until GitHub's reset time, shows what it asked earlier, and says
  until when.

## The catalogue

`os/system_files/usr/share/hearth/recomps.json` (in the image at `/usr/share/hearth/recomps.json`, and built into
Hearth). Once a day Hearth fetches the newest one from Hearth OS's own repository (main branch), so ports added to
the catalogue arrive without a Hearth update. A newer file only replaces the old one if every entry is valid.

Every project in it was checked to exist with releases on GitHub (October 2026), and its README (and for N64Recomp
ports, its source code) was read for what it needs. Fields:

| Field | Meaning |
|---|---|
| `id`, `title`, `game`, `console`, `kind` | Hearth's name for it, the port's name, the original game, the console, and recomp / decomp / port |
| `repo` | Its GitHub project (owner/name): the only place Hearth downloads it from |
| `linux` | true: the project makes a Linux build; false: Windows only (runs with Proton); null: Hearth decides from the release's files |
| `needs`, `files` | The copy of the game you supply, and its file types |
| `setup` | `n64recomp`, `beside` (with `besideName`) or `in-app`, as above |
| `hash` | Hashes the project documents: `sha1`, `md5`, `sha256` or `xxh3` (any one matching is enough) |
| `n64` | N64Recomp ports: `programId` (its folder in ~/.config) and `gameId` (the ROM's file name there) |
| `saves`, `mods` | Where it keeps them |
| `icon` | The project's own icon in its repository (or none: Hearth draws a tile). No store-owned art. |
| `assets` | Optional patterns for a project whose release files Hearth can't tell apart by name |
| `experimental`, `featured`, `arm64` | Early ports, featured ports, ports with Arm builds too |

**Add a GitHub project** (Store › Recomps) adds any project with releases. It's marked "Added by you: not checked by
Hearth" and kept in `~/.local/share/nimbus/recomps/custom.json`.

## Everything RecompLauncher, RecompStore and GithubLauncher do, and how Hearth does it

### RecompLauncher (recomplauncher.com: a website catalogue, with a desktop app in beta)

| Feature | In Hearth |
|---|---|
| A catalogue of 200+ fan-made ports | 107 ports in Hearth's own catalogue, each one checked to exist with releases on GitHub. Projects without releases (or whose GitHub project is gone) are left out until they have them. Anything else: **Add a GitHub project**. |
| Platform per port (Windows, Linux, macOS) | Each port's page says which build this PC gets: Linux, or Windows through Proton. The **Linux builds** filter. macOS doesn't apply. |
| Categories by console, search, filters | Console filter, Installed, Updates, Linux builds, Added by you, Hidden; search by port or game name; sort by Featured, A to Z, console or recently played |
| Ranked by popularity | Not done: GitHub's star counts would take a question per project (over the 60-an-hour limit). Featured ports and installed ones come first instead. |
| Links to the developers' GitHub releases | Hearth installs from exactly those releases, and names the project on each page |
| Desktop app: automatic downloads from the repos | Install, with the right file picked for this PC |
| Desktop app: game directory setup (your own game files) | **Needs your copy of …**: the file picker, a hash check, and the file put where the port looks |
| Desktop app: update management | Update checks every 6 hours, Update in place, skip an update, keep a version, install an older version |
| Desktop app: a controller UI like Big Picture | Hearth's Game Mode: every screen works with a controller |

### RecompStore (elripalda/RecompStore: a PS5 homebrew store, GPL-3.0)

| Feature | In Hearth |
|---|---|
| Install and update ports from their GitHub releases | Yes |
| Updates keep saves, DLC and mods | Yes: an update only replaces the old version's own files |
| Set up from your own disc image | Yes, for ROMs and disc images (see "Your copy of the game") |
| Launch | Play, on the port's page and in the Library |
| Refresh the home screen so new games show | The Library updates by itself after an install |
| Delete, after a confirmation | Uninstall asks, and offers a backup first |
| Patch notes from GitHub | On each port's page |
| Self-update (checks its releases at start and every 6 hours) | Hearth Recomps is part of Hearth, so it updates with Hearth OS. Its catalogue refreshes itself daily. |
| Install to internal storage, M.2 or USB | This device, or any USB drive, SD card or second drive that's mounted |
| Custom sources, marked unchecked | **Add a GitHub project**, marked "Added by you: not checked by Hearth" |
| Games, Emulators, Apps and Tools tabs | Games only. Emulators already have their place in Hearth (Library › Emulators, Settings › Games), and Hearth OS has Bazaar for apps. |
| Send installs from a phone (QR code) | Not done: on a PS5 typing is the hard part. Hearth has an on-screen keyboard and Hearth Link; a phone install queue may come later. |
| Guided installer (.elf) | Not needed: Hearth Recomps comes with Hearth. |
| Firmware and PS5-specific parts (ShadowMountPlus, the PS5 Vulkan port) | Don't apply to a PC |

### GithubLauncher, formerly N64RecompLauncher (SirDiabo)

| Feature | In Hearth |
|---|---|
| Browse, download, track and update ports from GitHub | Yes |
| An app list that updates itself (SirDiabo/GHLAppList) | Hearth's own catalogue refreshes daily from Hearth OS's repository. GHLAppList helped find projects; every entry was checked and written for Hearth. |
| Add your own apps (any GitHub repository) | **Add a GitHub project** |
| Choose a version, skip an update, keep a preferred version | ⋯ › Choose a version, Skip, Keep this version |
| Choose between several downloads | ⋯ › Choose a download |
| Choose the program to start | ⋯ › Choose the program to start |
| Locate an existing install | ⋯ › I have it already |
| Hide apps | ⋯ › Hide it (Recomps › Hidden shows them again) |
| Sort by last played and other orders | Featured, A to Z, Console, Recently played |
| Add to Steam, desktop shortcuts | **Add to the Desktop Mode menu** makes a menu entry; Steam's "Add a non-Steam game" lists it too. Hearth doesn't write Steam's own files: in Game Mode, Hearth's Library is where the game shows. |
| Windows builds on Linux (a launch command) | The Windows build starts with Proton (umu-launcher) automatically; per-game settings choose the Proton version |
| Per-game settings | Hearth's per-game settings (launch options, Proton, controller layout) |
| Custom icons | The project's own icon where it has one, otherwise Hearth's tile with the title |
| Controller input, fullscreen | Game Mode |
| A GitHub token for the rate limit | Not done on purpose: Hearth never asks for or keeps secrets. It caches answers and respects GitHub's limit instead (see above). |
| Command line (--list, --download, --update, --run) | Not done yet: everything is in Game Mode. The bridge calls (`recomps.*`) are there for a command line later. |
| Background music, colours, grid sizes | Hearth's own look and settings |
| Linux and Steam Deck builds | Hearth OS itself |

## Credits

Hearth Recomps is written from scratch for Hearth. It copies no code or data from these projects, but owes them the
idea and the layout:

- **RecompStore** by elripalda (GPL-3.0): https://github.com/elripalda/RecompStore
- **RecompLauncher**: https://recomplauncher.com/
- **GithubLauncher** (formerly N64RecompLauncher) by SirDiabo, and its app list **GHLAppList**, which helped find
  projects for the catalogue: https://github.com/SirDiabo/GithubLauncher

Every port is made by its own project and contributors, listed below with their GitHub projects; the N64Recomp
ports build on **N64Recomp** and **N64ModernRuntime** (Mr-Wiseguy and contributors), the Harbour Masters ports on
**libultraship**, and many Xbox 360 recompilations on **XenonRecomp** / **ReXGlue**. Hearth doesn't make any of the
ports.

## The ports

"Your copy" says how the port gets it; "Checked" lists the hashes Hearth checks it against. Ports marked (early) are new or partly working.

### Nintendo 64 (37)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| 2 Ship 2 Harkinian | The Legend of Zelda: Majora's Mask | `harbourmasters/2ship2harkinian` | Linux | Majora's Mask ROM (a supported version) (In its folder) | — |
| Automobili Lamborghini: Recompiled (early) | Automobili Lamborghini | `alondero/automobililamborghini-recomp` | Linux | Automobili Lamborghini (USA) ROM (Asked on first start) | XXH3 |
| Banjo: Recompiled | Banjo-Kazooie | `BanjoRecomp/BanjoRecomp` | Linux | Banjo-Kazooie (USA, v1.0) ROM (Hearth places it) | XXH3 |
| BattleShip (early) | Super Smash Bros. | `JRickey/BattleShip` | Linux | Super Smash Bros. (USA v1.0 or Japan) ROM (Asked on first start) | — |
| Blast Corps: Recompiled | Blast Corps | `SlaveOfIDO/blastcorps-recomp` | Linux | Blast Corps (USA) ROM (Hearth places it) | XXH3 |
| Bomberman 64: Recompiled | Bomberman 64 | `RevoSucks/BM64Recomp` | Linux | Bomberman 64 (USA) ROM (Hearth places it) | XXH3 |
| Bomberman Hero: Recompiled | Bomberman Hero | `RevoSucks/BMHeroRecomp` | Linux | Bomberman Hero (USA) ROM (Hearth places it) | XXH3 |
| Castlevania: Legacy of Darkness Recompiled (early) | Castlevania: Legacy of Darkness | `fliperama86/cvlod_recomp` | Linux | Castlevania: Legacy of Darkness (USA) ROM (Asked on first start) | — |
| Chameleon Twist: Recompiled | Chameleon Twist (Japan) | `Rainchus/ChameleonTwist1-JP-Recomp` | Linux | Chameleon Twist (Japan) ROM (Asked on first start) | XXH3 |
| Diddy Kong Racing Recompiled | Diddy Kong Racing | `ThatGuyMcd/DKR-R` | Linux | Diddy Kong Racing (USA) ROM (Asked on first start) | — |
| Dinosaur Planet: Recompiled | Dinosaur Planet | `DinosaurPlanetRecomp/dino-recomp` | Linux | The Dinosaur Planet prototype ROM (Hearth places it) | XXH3 |
| DK64: Rekongpiled | Donkey Kong 64 | `Rainchus/Donkey-Kong-64-Recompiled` | Linux | Donkey Kong 64 (USA) ROM (Hearth places it) | XXH3 |
| Dr. Mario 64: Recompiled Plus | Dr. Mario 64 | `theboy181/drmario64_recomp_plus` | Linux | Dr. Mario 64 (USA) ROM (Asked on first start) | XXH3 |
| G-Diffuser | F-Zero X | `Zorkats/G-Diffuser` | Linux | F-Zero X ROM (Asked on first start) | — |
| Ghostship | Super Mario 64 | `harbourmasters/ghostship` | Linux | Super Mario 64 (USA or Japan) ROM (In its folder) | — |
| Goemon 64: Recompiled | Mystical Ninja Starring Goemon | `klorfmorf/Goemon64Recomp` | Linux | Mystical Ninja Starring Goemon (USA) ROM (Hearth places it) | XXH3 |
| Golden Balloon (early) | Diddy Kong Racing | `akratch/goldenballoon` | Linux | Diddy Kong Racing ROM (Asked on first start) | — |
| Harvest Moon 64: Recompiled | Harvest Moon 64 | `HarvestMoon64Recomp/HarvestMoon64Recomp` | Linux | Harvest Moon 64 (USA) ROM (Hearth places it) | XXH3 |
| Infinite Mario 64 Ever | Infinite Mario 64 (fan game on libsm64) | `Brawmario/infinite-mario-64-ever` | Linux | Super Mario 64 (USA) ROM (Asked on first start) | — |
| Lighthouse | Banjo-Kazooie | `HarbourMasters/Lighthouse` | Linux | Banjo-Kazooie ROM (a supported version) (In its folder) | — |
| Mega Man 64: Recompiled | Mega Man 64 | `MegaMan64Recomp/MegaMan64Recompiled` | Linux | Mega Man 64 (USA) ROM (Hearth places it) | XXH3 |
| Paper Mario ReCut (early) | Paper Mario | `SMCGames/Paper-Mario-ReCut` | Windows (Proton) | Paper Mario (USA) ROM (Asked on first start) | — |
| PaperBoat | Paper Mario | `HarbourMasters/PaperBoat` | Linux | Paper Mario ROM (a supported version) (In its folder) | — |
| Perfect Dark | Perfect Dark | `fgsfdsfgs/perfect_dark` | Linux | Perfect Dark (USA, Rev 1) ROM (In its folder) | MD5 |
| Pokémon Stadium: Recompiled (early) | Pokémon Stadium | `mstan/PokemonStadiumRecomp` | Linux | Pokémon Stadium (USA, v1.0) ROM (Asked on first start) | XXH3, MD5 |
| Quest 64: Recompiled | Quest 64 | `Rainchus/Quest64-Recomp` | Linux | Quest 64 (USA) ROM (Hearth places it) | XXH3 |
| Ship of Harkinian | The Legend of Zelda: Ocarina of Time | `harbourmasters/shipwright` | Linux | Ocarina of Time ROM (a supported version: ship.equipment checks yours) (In its folder) | — |
| sm64coopdx | Super Mario 64 (online co-op) | `coop-deluxe/sm64coopdx` | Linux | Super Mario 64 (USA) ROM (Asked on first start) | — |
| Snowboard Kids 2: Recompiled | Snowboard Kids 2 | `cdlewis/snowboardkids2-recomp` | Linux | Snowboard Kids 2 (USA) ROM (Hearth places it) | XXH3 |
| Snowboard Kids: Recompiled | Snowboard Kids | `cdlewis/snowboardkids-recomp` | Linux | Snowboard Kids (USA) ROM (Hearth places it) | XXH3 |
| Space Station Silicon Valley: Recompiled | Space Station Silicon Valley | `Cellenseres/SSSV_Recomp` | Linux | Space Station Silicon Valley (USA, v1.0) ROM (Hearth places it) | XXH3 |
| SpaghettiKart | Mario Kart 64 | `harbourmasters/spaghettikart` | Linux | Mario Kart 64 (USA) ROM (In its folder) | SHA1 |
| Starship | Star Fox 64 | `harbourmasters/starship` | Linux | Star Fox 64 (USA 1.0 or 1.1) ROM (In its folder) | — |
| Trouble Makers (early) | Mischief Makers | `ThiagoLira/trouble-makers-pc-recomp` | Linux | Mischief Makers (USA) ROM (Asked on first start) | XXH3 |
| Wave Race 64: Recompiled (early) | Wave Race 64 | `elliotttate/wave-race-64-recomp` | Picked from the release | Wave Race 64 (USA) ROM (Asked on first start) | SHA1 |
| WCW vs. nWo World Tour: Recompiled | WCW vs. nWo: World Tour | `jessetbh/WCWvsNWOWorldTourRecomp` | Linux | WCW vs. nWo: World Tour (USA) ROM (Asked on first start) | XXH3, SHA1 |
| Zelda 64: Recompiled | The Legend of Zelda: Majora's Mask | `Zelda64Recomp/Zelda64Recomp` | Linux | Majora's Mask (USA) ROM (Hearth places it) | XXH3 |

### GameCube (7)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| Animal Crossing PC Port (early) | Animal Crossing | `flyngmt/ACGC-PC-Port` | Windows (Proton) | Animal Crossing (GameCube) disc image (In its folder) | — |
| Dusklight | The Legend of Zelda: Twilight Princess | `TwilitRealm/dusklight` | Picked from the release | Twilight Princess (GameCube) disc image (Asked on first start) | — |
| Mario Party 4 (early) | Mario Party 4 | `mariopartyrd/marioparty4` | Linux | Mario Party 4 disc image (Asked on first start) | — |
| melee-pc | Super Smash Bros. Melee | `999sian/melee-pc` | Linux | Super Smash Bros. Melee (NTSC 1.02) disc image (Asked on first start) | — |
| Open Nectar (early) | Pikmin | `SSunnKing/Open-Nectar---Pikmin-Native-PC-Port` | Linux | Pikmin (GameCube) disc image (Asked on first start) | — |
| Ring Out (early) | SoulCalibur II | `jackpoison-prog/RingOut` | Linux | SoulCalibur II (GameCube) disc image (Asked on first start) | — |
| Strikers | Super Mario Strikers | `new-coke/strikers` | Linux | Super Mario Strikers disc image (Asked on first start) | — |

### Wii (3)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| Kirby Wii Recomp (ModernGekko) (early) | Kirby's Return to Dream Land | `ExpansionPak/ModernGekko` | Picked from the release | Kirby's Return to Dream Land (Wii) disc image (Asked on first start) | — |
| Project Reignition | Sonic and the Secret Rings (fan remake) | `kumapauz/project-reignition` | Picked from the release | Nothing (Asked on first start) | — |
| Wiicompiled (early) | Mario Kart Wii | `patchzyy/Wiicompiled` | Linux | Mario Kart Wii disc image (PAL, NTSC-U or NTSC-J) (Asked on first start) | — |

### Xbox 360 (17)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| Ace Combat 6 Recomp (early) | Ace Combat 6: Fires of Liberation | `sal063/AC6_recomp` | Windows (Proton) | Ace Combat 6 (Xbox 360) game files, extracted into assets/ (Asked on first start) | — |
| Condemned2Recomp | Condemned 2: Bloodshot | `psxrestore/Condemned2Recomp` | Picked from the release | Condemned 2: Bloodshot (Xbox 360) game files (Asked on first start) | — |
| Deadly Premonition Recompilation (early) | Deadly Premonition | `LittleBitUA/DPRecomp` | Linux | Deadly Premonition (Xbox 360, PAL or USA) disc image; its installer extracts it (Asked on first start) | — |
| Dragon Ball Z Budokai HD Recompiled | Dragon Ball Z: Budokai HD | `WistfulHopes/DBZ1` | Windows (Proton) | Dragon Ball Z Budokai HD (Xbox 360) game files (the folder is the program's first argument) (Asked on first start) | — |
| GoldenEye 007 Recompiled (early) | GoldenEye 007 (XBLA) | `SunJaycy/GoldenEye-Recomp` | Windows (Proton) | Game files, put in the assets folder (Asked on first start) | — |
| Hell's Gate (early) | Dante's Inferno | `florinp93/hells-gate-recomp` | Linux | Dante's Inferno (Xbox 360) disc image; its installer extracts it (Asked on first start) | — |
| Kameo: RePowered (early) | Kameo: Elements of Power | `birabittoh/KameoRePowered` | Linux | Kameo (Xbox 360) game files, extracted into assets/ (Asked on first start) | — |
| Project Gotham Racing 4 Recompiled (early) | Project Gotham Racing 4 | `beatrixzy/PGR4-Recomp` | Windows (Proton) | Project Gotham Racing 4 (Xbox 360) game files (Asked on first start) | — |
| Re-NuT | Banjo-Kazooie: Nuts & Bolts | `masterspike52/reNut` | Linux | Banjo-Kazooie: Nuts & Bolts (USA) disc image and its title update (Asked on first start) | — |
| re:Blue | Blue Dragon | `zolaware/reblue` | Linux | Blue Dragon (Xbox 360), all three disc images (Asked on first start) | — |
| ReDahm (early) | Destroy All Humans! Path of the Furon | `masterspike52/redahm` | Windows (Proton) | Path of the Furon (Xbox 360) game files (Asked on first start) | — |
| Sega Superstars Tennis ReMatch | Sega Superstars Tennis | `m1ndcrap/Sega-Superstars-Tennis-ReMatch` | Linux | Sega Superstars Tennis (Xbox 360) game files, extracted into assets/ (Asked on first start) | — |
| Skate: Recompiled | Skate 3 | `mchughalex/skate3recomp` | Linux | Skate 3 (Xbox 360) disc image (Asked on first start) | — |
| The Simpsons Game - Recompiled | The Simpsons Game | `YesterMester/TheSimpsonsGameRecomp` | Linux | The Simpsons Game (Xbox 360) disc image; its launcher's Install tab takes it (Asked on first start) | — |
| Unleashed Recompiled | Sonic Unleashed | `hedge-dev/UnleashedRecomp` | Linux | Sonic Unleashed (Xbox 360) game files and title update; its own installer takes them (Asked on first start) | — |
| Viva Piñata: Trouble in Paradise Recompiled | Viva Piñata: Trouble in Paradise | `SolarCookies/TiP-Recomp` | Linux | Viva Piñata: Trouble in Paradise (Xbox 360) disc image (Asked on first start) | — |
| WWE SmackDown vs. Raw 2007 Recompiled | WWE SmackDown vs. Raw 2007 | `HollywoodAkeem/SVR07-Recomp` | Windows (Proton) | SmackDown vs. Raw 2007 (Xbox 360) game files (Asked on first start) | — |

### PlayStation (23)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| Ape Escape Recomp (early) | Ape Escape | `mstan/ApeEscapeRecomp` | Linux | Ape Escape disc image (Asked on first start) | — |
| Bloody Roar II Recompiled (early) | Bloody Roar II | `novapowers0/BloodyRoar2Recomp` | Linux | Bloody Roar II disc image (Asked on first start) | — |
| Crash Bandicoot 2 Recomp (early) | Crash Bandicoot 2: Cortex Strikes Back | `Zumbo06/Crash2Recomp` | Windows (Proton) | Crash Bandicoot 2 disc image (Asked on first start) | — |
| Crash Bandicoot Recomp (early) | Crash Bandicoot | `Matteo842/CrashBandicoot-Launcher` | Linux | Crash Bandicoot disc image (.chd or .cue) (In its folder) | — |
| CTR Native | Crash Team Racing | `CTR-tools/ctr-native` | Linux | Crash Team Racing (USA) disc image (Asked on first start) | — |
| Einhänder Recompiled (early) | Einhänder | `strider973/Einhander-Recompiled` | Windows (Proton) | Einhänder disc image (Asked on first start) | — |
| Klonoa: Door to Phantomile Recomp (early) | Klonoa: Door to Phantomile | `TechnicallyComputers/Klonoa-Door-to-Phantomile` | Windows (Proton) | Klonoa disc image (Asked on first start) | — |
| Mega Man X5 Recomp (early) | Mega Man X5 | `mstan/MegaManX5Recomp` | Windows (Proton) | Mega Man X5 (USA) disc image (Asked on first start) | — |
| Mega Man X6 Recomp (early) | Mega Man X6 | `mstan/MegaManX6Recomp` | Windows (Proton) | Mega Man X6 (USA) disc image (Asked on first start) | — |
| R.E.L.I.V.E. | Oddworld: Abe's Oddysee and Abe's Exoddus | `AliveTeam/alive_reversing` | Linux | Your Abe's Oddysee or Abe's Exoddus game files (PC or PlayStation) (Asked on first start) | — |
| Rage Racer PC | Rage Racer | `khasinski/rage-racer-pc` | Linux | Rage Racer disc image (Asked on first start) | — |
| REDRIVER2 | Driver 2 | `OpenDriver2/REDRIVER2` | Linux | Driver 2 game files from your discs (Asked on first start) | — |
| Severed Chains | The Legend of Dragoon | `Legend-of-Dragoon-Modding/Severed-Chains` | Linux | The Legend of Dragoon disc images, in its isos folder (In its folder) | — |
| Silent Hill Decompilation Project (early) | Silent Hill | `SlickAmogus/silent-hill-decomp` | Linux | Silent Hill disc image, in its gamedata folder (In its folder) | — |
| Street Fighter Alpha 3 Recompiled | Street Fighter Alpha 3 | `TechnicallyComputers/Street-Fighter-Alpha-3-Recomp` | Windows (Proton) | Street Fighter Alpha 3 disc image (Asked on first start) | — |
| Street Fighter EX Plus Alpha Recomp (early) | Street Fighter EX Plus α | `guimaraf/plusalpha-recomp` | Windows (Proton) | Street Fighter EX Plus α (USA) disc image and a PlayStation BIOS you dumped (Asked on first start) | — |
| Street Fighter EX2 Plus Recompiled (early) | Street Fighter EX2 Plus | `strider973/Street-Fighter-EX2-Plus-Recompiled` | Windows (Proton) | Street Fighter EX2 Plus disc image (Asked on first start) | — |
| Strider 2 Recompiled (early) | Strider 2 | `strider973/STRIDER-2-Recompiled` | Linux | Strider 2 (USA, SLUS-01163) bin + cue (Asked on first start) | — |
| SymphonyRecomp | Castlevania: Symphony of the Night | `BlackLabelHQ/SymphonyRecomp` | Windows (Proton) | Symphony of the Night (USA) disc image, bin/cue or CHD (Asked on first start) | — |
| Syphon Filter PC | Syphon Filter | `Madxbio97/SF-pc-port` | Windows (Proton) | Syphon Filter disc image (Asked on first start) | — |
| Tomba! Recomp (early) | Tomba! | `mstan/TombaRecomp` | Linux | Tomba! disc image (Asked on first start) | SHA1 |
| ValkyrieRecomp (early) | Valkyrie Profile | `Ed1z19/ValkyrieRecomp` | Windows (Proton) | Valkyrie Profile disc image (Asked on first start) | — |
| WipeOut: Phantom Edition | Wipeout | `wipeout-phantom-edition/wipeout-phantom-edition` | Picked from the release | Wipeout (USA) bin + cue (Asked on first start) | — |

### PlayStation 2 (3)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| 3SX | Street Fighter III: 3rd Strike | `crowded-street/3sx` | Linux | SF33RD.AFS from your 3rd Strike (PS2) disc (Asked on first start) | — |
| 3SXW | Street Fighter III: 3rd Strike | `guimaraf/3sxw` | Linux | Street Fighter III: 3rd Strike (PS2) disc image (it extracts SF33RD.AFS) (Asked on first start) | — |
| MikuPan | Fatal Frame | `Mikompilation/MikuPan` | Linux | IMG_HD.BIN and IMG_BD.BIN from your Fatal Frame disc, in the program's folder (Asked on first start) | — |

### Nintendo DS (2)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| Metroid Prime: Hunters Recomp (early) | Metroid Prime: Hunters | `mstan/MetroidPrimeHuntersRecomp` | Linux | Metroid Prime: Hunters (USA, rev 0) ROM (In its folder) | SHA1 |
| Pokémon Platinum PC Port | Pokémon Platinum | `cybervisi0n/pokeplatinum` | Linux | Pokémon Platinum (USA, CPUE01) ROM (Asked on first start) | — |

### Nintendo 3DS (1)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| TriAevum (early) | The Legend of Zelda: Ocarina of Time 3D | `coccofresco/TriAevum` | Linux | Ocarina of Time 3D ROM (decrypted) (Asked on first start) | — |

### Game Boy (1)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| Gen1Recomp++ (early) | Pokémon Red, Blue, Yellow, Gold, Silver and Crystal | `bryanthaboi/gen1recomp` | Linux | Your Pokémon Game Boy cartridge dump (Asked on first start) | — |

### Game Boy Advance (3)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| Auto-TMC | The Legend of Zelda: The Minish Cap | `StonedModder/auto-tmc` | Linux | The Minish Cap ROM (Asked on first start) | — |
| Mario Kart: Super Circuit Recomp (early) | Mario Kart: Super Circuit | `mstan/MarioKartSuperCircuitRecomp` | Linux | Mario Kart: Super Circuit (USA) ROM (Asked on first start) | — |
| Wario Land 4 (early) | Wario Land 4 | `BioRhythmicStudios/warioland4` | Linux | Wario Land 4 (USA/Europe or Japan) ROM (Asked on first start) | MD5 |

### Super Nintendo (5)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| Mega Man X SNESRecomp | Mega Man X | `mstan/MegaManXSNESRecomp` | Linux | Mega Man X (USA, Rev 1) ROM (Asked on first start) | SHA256, SHA1 |
| Mega Man X2 Recomp (early) | Mega Man X2 | `mstan/MegaManX2Recomp` | Windows (Proton) | Mega Man X2 (USA) ROM (Asked on first start) | SHA256 |
| smw | Super Mario World | `snesrev/smw` | Picked from the release | Super Mario World (USA) ROM, named smw.sfc (In its folder) | — |
| Super Metroid Launcher | Super Metroid | `RadzPrower/Super-Metroid-Launcher` | Windows (Proton) | Super Metroid (USA) ROM (Asked on first start) | SHA256 |
| Zelda 3 Launcher | The Legend of Zelda: A Link to the Past | `RadzPrower/Zelda-3-Launcher` | Windows (Proton) | A Link to the Past (USA) ROM (Asked on first start) | SHA256 |

### NES (1)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| Super Mario Bros. Remastered | Super Mario Bros. | `JHDev2006/Super-Mario-Bros.-Remastered-Public` | Linux | Super Mario Bros. (NES) ROM (Asked on first start) | — |

### Mega Drive / Genesis (2)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| Sonic 1 Forever | Sonic the Hedgehog | `ElspethThePict/S1Forever` | Picked from the release | Sonic the Hedgehog game data (Data.rsdk from the 2013 release) (Asked on first start) | — |
| Sonic 3 A.I.R. | Sonic 3 & Knuckles | `Eukaryot/sonic3air` | Linux | The Sonic 3 & Knuckles ROM (from the Steam release) (Asked on first start) | — |

### Arcade (2)

| Port | Game | Project | Build | Your copy | Checked |
|---|---|---|---|---|---|
| CannonBall DX (early) | OutRun (arcade) | `Endprodukt/cannonball-dx` | Linux | The OutRun arcade ROM files, in its roms folder (In its folder) | — |
| Initial D Arcade Stage 3 Recompiled (early) | Initial D Arcade Stage 3 | `distilledorion-sketch/Initial-D-Arcade-Stage-3-Native-SH-4-Recompilation-WIP-` | Windows (Proton) | Your matching game files, in its 'game files' folder (Asked on first start) | — |

## Checks

- Rust tests (`hosts/hearth-core/src/recomps.rs`, rust.yml): the catalogue (every entry valid, hashes well formed,
  icons only from the projects' own repositories, no links to ROM sites), GitHub's release answer (a recorded shape
  with Linux, Arm, Windows, Flatpak and source files: the right file for each PC, the newest stable release, drafts
  left out), its headers (ETag, the rate limit), checking a copy (SHA-1, MD5, SHA-256, and XXH3 on a byte-swapped
  N64 ROM), updates keeping saves, mods and settings, finding the program to start, putting your file where the port
  looks, projects you add, and menu entries.
- `tools/ui-check.js`: Store › Recomps with the preview's data: the tab and RB, filters, search, an installed port's
  page (Play, Update, patch notes, the copy check, the ⋯ sheet), one that isn't installed (Install, Needs your copy,
  Windows build through Proton), and B back.
- Not yet tried on a device: a real install, update and uninstall.

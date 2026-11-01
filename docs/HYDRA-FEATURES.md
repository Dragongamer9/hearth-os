# Hydra Launcher's features, the Hearth way

[Hydra Launcher](https://github.com/hydralauncher/hydra) (MIT licence, Electron/React/TypeScript with Python and Rust)
is a game launcher with a catalogue, a library, achievements, cloud saves, a profile, friends and a download manager.
Hearth now has its own version of every **legitimate** Hydra feature. It's written from scratch in Hearth's own stack
(Rust in `hosts/hearth-core/src/play/`, Game Mode's page in `ui/hearth-play.js`). No Hydra code was copied. Thanks to
Hydra's authors for the ideas.

**What Hearth leaves out, on purpose.** Hearth only downloads games from legitimate sources you're entitled to. So
these Hydra pieces are **not built**:

- community "download sources" and repack JSON feeds (FitGirl, DODI and the like)
- any connection to repack, warez or file-hoster sites (Gofile, Datanodes, Vikingfile, PixelDrain…)
- debrid services (Real-Debrid, TorBox, AllDebrid) for game downloads
- reading achievement files written by cracks or Steam emulators (Goldberg, CODEX, RUNE…)
- Hydra's torrent client, which exists to download those repacks

Where these come up in the list below, the line says **not built: Hearth only downloads from legitimate sources**.

## The checklist

What Hydra has (from its README, docs, release notes up to 4.3.0 and its screens), and where Hearth stands.
**Already in Hearth**: Hearth had it before this work. **Built now**: added with this work (needs testing on a
device). **Not built**: left out on purpose, with the reason.

### Library and playing

| Hydra feature | Hearth |
|---|---|
| Add any game or program you own to the library | Already in Hearth: Library › Add a game (program, AppImage, .desktop file, Windows .exe) |
| Windows games on Linux with Wine/Proton, a prefix and Proton version per game, launch options | Already in Hearth: Proton through umu-launcher, per-game Proton, arguments, environment, MangoHud, GameMode, FSR (game settings) |
| Scan folders for games and add them | **Built now**: Library › Add a game › Find games in a folder (finds .exe files, AppImages and Linux launchers; leaves out installers, uninstallers, redistributables and games already added) |
| Custom cover art, non-Steam art, SteamGridDB assets | **Built now**: a game's page › Change pictures (cover, banner, background, logo from SteamGridDB, with **your own** SteamGridDB API key, off until you type it). Works for any game; the store's own pictures come back with one press |
| Playtime tracking for every game, last played | Already in Hearth: every game, any store (paused time doesn't count) |
| Steam playtime sync, Steam library import | Already in Hearth: Steam's own play time and your Steam games (Hearth never signs in to Steam) |
| Epic library, downloads, updates and launches (Legendary) | Already in Hearth: Epic and GOG install, update and play through Hearth, with their cloud saves |
| Launch installed games through Steam | Already in Hearth |
| Library sorting, filters, search, collections ("game grouping"), badges | Already in Hearth: Library tabs, sorting, collections, favourites, search |
| Big Picture mode, `--big-picture` flag, power options | Already in Hearth: Game Mode is Hearth's whole interface |
| Emulation ("Classics"): RetroArch, PCSX2, RPCS3, PPSSPP, Dolphin, BIOS checks | Already in Hearth: Library › Emulators (RetroArch with the right core per system, ROMs folder) |
| Desktop shortcuts, tray icon | Already in Hearth (Desktop Mode's launcher; Hearth is the session itself) |
| Discord Rich Presence | Already in Hearth (built, hidden until the owner sets the Discord application ID) |
| Auto-updates, crash reports | Already in Hearth (system updates; Send logs) |

### Catalogue and game pages

| Hydra feature | Hearth |
|---|---|
| A catalogue to browse and search, with prices and store pages | Already in Hearth: Game Mode › Store (Steam, GOG, Epic, Xbox, Battle.net) |
| Suggestions based on what you play ("suggestion algorithm") | **Built now**: Game Mode › Discover › "Because you play …": Steam's best sellers in the genres of your most played games, minus what you own |
| Discover games you haven't played | **Built now**: Discover › Not played yet (your own unstarted games), Pick up again, Popular games you haven't played, Surprise me |
| Game page: description, screenshots, genres, developer | Already in the Store's game pages; **built now** for the library's game pages too (More about it) |
| System requirements | **Built now**: a game's page › System requirements (PC and Linux, from the game's Steam store page) |
| HowLongToBeat times | **Built now, from a legal source**: a game's page › How long to beat, from IGDB's official `game_time_to_beats` API with **your own** free Twitch developer app (off until you type it). HowLongToBeat itself has no public API and its terms don't allow scripted use, so Hearth doesn't read it |
| Similar games | **Built now** as Discover's "Because you play …" shelves |
| ProtonDB / how it runs on Linux | Already in Hearth: ProtonDB rating on game pages and in the Store |
| Trailers | Not built yet (the Store shows screenshots); a later step |
| Reviews written by Hydra users, review panel | Not built: needs a central Hydra-style server; Hearth runs none (each device has its own). Store pages show each store's own rating |
| Global leaderboards, rankings, medals, streaks | Not built: same reason (needs a central server) |

### Achievements

| Hydra feature | Hearth |
|---|---|
| Steam achievements | Already in Hearth: from Steam's own files on this device; **built now**: from Steam's official Web API with **your own** key and SteamID64, for games whose achievements aren't on this device |
| Epic achievements | Not yet: Epic doesn't offer achievements to other apps (Hydra reads them with an Epic session; Hearth won't use your Epic tokens for this) |
| GOG achievements | Not yet: GOG only gives them out per game, with each game's own GOG Galaxy sign-in |
| RetroAchievements (account, realtime unlocks) | **Built now**: achievements for your emulated games from RetroAchievements' official Web API, with **your own** user name and web API key (unlock pop-ups during play: not yet) |
| Achievements from crack and Steam-emulator files (Goldberg, CODEX, RUNE…) | **Not built: Hearth only downloads from legitimate sources** and never reads crack files |
| Achievement notifications, souvenirs (a screenshot at each unlock), export | Not built yet (Hearth has screenshots and toasts; unlock detection comes first) |

### Cloud saves

| Hydra feature | Hearth |
|---|---|
| Find each game's saves (Ludusavi's manifest), back up and restore per game, several backups, custom paths | Already in Hearth: **Saves** (`hosts/hearth-gaming`, from a game's ⋯ menu: Back up saves), built on [Ludusavi's manifest](https://github.com/mtkennerly/ludusavi-manifest) (open, credited in the app) |
| Saves in the cloud, following you between devices | Already in Hearth: Hearth Storage › saves that follow you (your own Hearth server, no subscription), plus Epic and GOG's own cloud saves |
| Hydra Cloud subscription | Not built: Hearth keeps saves on your own devices (Hearth Storage), free |

### Profile and friends

| Hydra feature | Hearth |
|---|---|
| A profile that shows friends what you're playing | Already in Hearth: Hearth Friends (Matrix) shows "Playing …"; **built now**: Friends › Activity, your gaming profile (playing now, total play time, recently played, most played), and **Share my activity**, which sends titles and play time only to your friends, end-to-end encrypted in your chats with them (off until you turn it on) |
| Friends' activity | **Built now**: Friends › Activity › Friends' activity |
| Friend requests, codes, blocking, direct messages with pictures, friend notifications | Already in Hearth: Hearth Friends |
| Profile banner, badges, profile privacy | Partly: avatar and background in Settings › Profile; sharing is opt-in per item (Show what I'm playing, Share my activity). Badges need a central server: not built |
| Screenshots: hotkey, gamepad capture, lightbox | Already in Hearth: Screenshots (capture from Quick Access, gallery, send to a friend) |
| Uploading screenshots to a public profile, comments | Not built: needs a central server |

### Downloads

| Hydra feature | Hearth |
|---|---|
| A download manager: queue, pause and resume, progress, speed | **Built now**: Downloads › Other downloads: a queue (1 to 3 at once, Start first), pause and resume that carries on where it stopped (also after a restart), a speed limit, then Add to library (programs) or Unpack (archives, then pick the game) |
| Direct HTTP downloads | **Built now**: Download from a web address (http/https only, also after redirects) |
| Store downloads (Steam, Epic, GOG) | Already in Hearth: Downloads, with Steam's, Epic's and GOG's own queues |
| Free / open-source games | **Built now**: Discover and Get games › Free & open source (SuperTuxKart, OpenRA, Endless Sky, Shattered Pixel Dungeon, Unciv, Cataclysm: DDA), each from its own project's latest GitHub release |
| itch.io | **Built now**: Get games › itch.io: your itch.io library and free games, with **your own** itch.io API key (the queue only gets itch.io's short-lived download address, never the key) |
| GOG offline installers | Already: your GOG games install through Hearth. Offline installer files download on GOG's own page (the store window); then Find games in a folder or Add a game runs them with Proton |
| Archive extraction, stopping when the drive is full | **Built now**: Unpack (bsdtar, tar or unzip, never outside its folder); a full drive stops the download with "Is the drive full?" and Resume carries on |
| Community download sources, repack feeds (FitGirl, DODI…) | **Not built: Hearth only downloads from legitimate sources** |
| Repack, warez and file-hoster sites, hoster unlocking (Gofile, Datanodes, Vikingfile, PixelDrain…) | **Not built: Hearth only downloads from legitimate sources** |
| Debrid services (Real-Debrid, TorBox, AllDebrid) | **Not built: Hearth only downloads from legitimate sources** |
| Torrent client (libtorrent), seeding, network adapter binding | **Not built: Hearth only downloads from legitimate sources** (Hydra's torrents serve the repack feeds) |

### Looks and settings

| Hydra feature | Hearth |
|---|---|
| Themes (custom CSS), theme editor, theme store | Already in Hearth: built-in theme packs and accents; **built now**: Settings › Appearance › Theme files, your own CSS themes from `~/.local/share/nimbus/themes` (an example to copy; a theme can't load anything from the internet). Hydra's online theme store: not built (Hearth has no central server) |
| Languages | Hearth is in English for now |
| Integrations in settings (RetroAchievements, Steam…) | **Built now**: Settings › Games › Integrations: SteamGridDB, IGDB, Steam Web API, RetroAchievements, itch.io, each off until you type your own key |

## How it works

- **Your own keys.** Settings › Games › Integrations keeps each key in your profile's
  `~/.local/share/nimbus/accounts/integrations.json` (only you can read it; it moves with your profile). The page
  never gets a whole key back, only its last four characters. A key goes only to its own service, through curl's
  standard input (never on a command line). Hearth ships no keys and signs in to nothing for you.
- **Discover** reads Steam's public store pages for genres and best sellers (the same pages the Store uses), never
  anything about you. Your library and play time stay on the device.
- **Achievements** use only official APIs: Steam's Web API (`ISteamUserStats`) and RetroAchievements' Web API.
- **Downloads** are curl writing `<file>.part` with resume (`-C -`) and `--limit-rate`, kept in
  `~/.local/share/nimbus/downloads/direct.json`. Files go to `~/Downloads/Hearth`.
- **Activity** is an `io.hearthos.activity` event that Hearth Friends sends in each friend's direct chat, only in
  end-to-end encrypted chats and only when it changed. It carries titles and play time, nothing else (no store ids,
  folders or accounts). Friends' cards are kept in memory only.
- **Theme files** are cleaned before they reach the page: no `@import`, no web or file addresses in `url()`.

| Part | Where |
|---|---|
| The core (`play.*` calls) | `hosts/hearth-core/src/play/` (`mod.rs`, `keys.rs`, `discover.rs`, `details.rs`, `achievements.rs`, `artwork.rs`, `downloads.rs`, `sources.rs`, `finder.rs`, `activity.rs`, `themes.rs`) |
| Custom pictures in the library | `library.rs` (dto: `customArt` over the store's pictures) |
| Activity over Hearth Friends | `hosts/hearth-friends/src/lib.rs` (`activity_content`), `service.rs` (`setActivity`, `activity`) |
| Game Mode's screens | `ui/hearth-play.js` (hooked into `ui/index.html` with a few `hp…` lines) |
| Tests | `cargo test` in hearth-core (mocked answers; a local web server for pause/resume) and hearth-friends; `tools/ui-check.js` (the "hydra features" step); `tools/friends-e2e.py` (Activity between two users) |

## Credits

- [Hydra Launcher](https://github.com/hydralauncher/hydra) (MIT) for the feature set. No Hydra code is in Hearth.
- [Ludusavi](https://github.com/mtkennerly/ludusavi) and its [manifest](https://github.com/mtkennerly/ludusavi-manifest) (MIT) for where games keep their saves (Hearth's Saves app).
- [SteamGridDB](https://www.steamgriddb.com), [IGDB](https://www.igdb.com), [RetroAchievements](https://retroachievements.org) and [itch.io](https://itch.io) for their official APIs.

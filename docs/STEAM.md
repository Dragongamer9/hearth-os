# Steam in Hearth

Hearth treats Steam like its Epic and GOG integration: your Steam games are in the library (installed or not),
Install and Delete work from Hearth, downloads show in Hearth's Downloads page, and Steam itself stays out of
sight. All of it uses the **real Steam client** and only its public surfaces:

- `steam://` addresses and the client's own command-line options (`-silent`, `-shutdown`);
- Steam's own files on disk (`libraryfolders.vdf`, `appmanifest_<id>.acf`, `loginusers.vdf`, `registry.vdf`,
  `appcache/appinfo.vdf`, `logs/content_log.txt`, `logs/cloud_log.txt`), read, and for installs and uninstalls
  written the way Steam writes them;
- the public Steam Community games page of the signed-in account.

Hearth never loads `steamclient.so`, never speaks Steam's network protocol, needs no Web API key, and **never
signs in to Steam itself**. It never sees, types or stores a password or token. Signing in always happens in
Steam's own window, which you use; Hearth only brings that window in front and reads Steam's files to notice
when you're done.

The code lives in `hosts/hearth-core/src`:

| File | What it does |
| --- | --- |
| `steam.rs` | Finding Steam, its libraries, installed games (records), art, starting Steam hidden |
| `steam_owned.rs` | Owned games from the Community page, Steam's `appinfo.vdf` (read only), the cache |
| `steam_ctl.rs` | Silent installs and uninstalls, download progress, Steam only when needed, sign-in watching |
| `library.rs`, `bridge.rs` | Wiring into the library, the Downloads page and the UI's calls |

Ember (Windows) does the same in C#: `hosts/windows/Nimbus.Host/Library/SteamControl.cs` (owned games from the
Community page, install records, progress, uninstall, Steam only when needed, sign-in watching), wired into
`LibraryService.cs`, `SystemBridge.cs` and `GameModeBridge.cs`, with the same UI calls and settings
(`library.setSteamOnDemand`, `steam.libraries`, `steam.setLibrary`, `steam.showWindow`, `steam.signInCancel`).
The differences on Windows:

- Steam's folder comes from the registry (`HKCU\Software\Valve\Steam\SteamPath`), `registry.vdf`'s
  `AutoLoginUser` is the registry value of the same name, and a running Steam game is Steam's own `RunningAppID`
  registry value (there's no reaper process).
- Owned games come from Steam's local caches (`packageinfo.vdf` + `appinfo.vdf`, as before) **and** the Community
  page, merged.
- Steam's windows: its main window is kept hidden while it starts (`SteamWindows.HideMainFor`); its sign-in and
  install windows are brought in front. Ember comes back in front when the sign-in is done.
- Hearth Task Manager's "Steam starts with Ember" is the opposite of "Start Steam only when needed".
- There are no Proton prefixes to keep; everything else about uninstalling is the same.

## 1. Owned games that aren't installed

`steam_owned::refresh` asks `https://steamcommunity.com/profiles/<id64>/games?tab=all&xml=1` for the signed-in
account (the `MostRecent` user in `loginusers.vdf`). If that answer isn't a game list, it reads the games page
(`/games/?tab=all`), whose list is in `data-profile-gameslist="…rgGames…"` (or `var rgGames = […]` on the
older page). This only works while the profile's **Game details** are public. When they're private, the
library shows installed games only and Library › Stores says how to change it (Steam › Privacy › Game details).

Names, types and install folders for apps Steam already knows come from `appcache/appinfo.vdf` (versions 27, 28
and 29). Hearth seeks past the apps it doesn't need, so a big file costs little. Tools, soundtracks, videos and
DLC are left out, and so are Steam's runtimes.

The list is cached per Steam account in `~/.local/share/nimbus/cache/steam-owned-<id64>.json` and fetched again
when it's older than 12 hours, after signing in, or with Refresh list. A passing problem (no connection) keeps
the old list. Each game shows with the same art Steam uses (your grid art, Steam's library cache, then Steam's
CDN) and an Install button.

## 2. Installing without Steam's dialog

Pressing Install in Steam makes Steam write `steamapps/appmanifest_<id>.acf` with `StateFlags` `1026`
(update required, update started) and empty `InstalledDepots`, then download. Hearth writes the same record
itself (`steam_ctl::manifest_text`), with:

- `installdir` from `appinfo.vdf` (or the title without path characters);
- `LastOwner` set to the signed-in account;
- the record put in the **library you picked** (Library › Stores › Steam library for new Steam games). By
  default that's a Steam library on the drive of Hearth's install folder, or else Steam's own folder.

Steam reads its libraries when it starts, so:

- Steam not running: Hearth starts it hidden (`-silent`).
- Steam running and idle (no Steam game, no download or update, no cloud sync in the last 2 minutes): Hearth
  closes it properly (`steam -shutdown`, never killed) and starts it again hidden.
- Steam busy: the record waits. The game shows as *Waiting for Steam*, and Hearth restarts Steam once it's idle.

Hearth then watches the record. Steam rewrites it as soon as it takes the install up. If Steam deletes it, or
hasn't touched it two minutes after starting, Hearth removes its record (only if it's still byte for byte what
Hearth wrote) and **falls back to Steam's own install window** (`steam://install/<id>`), brought in front in Game
Mode. Before Steam takes it up, a waiting install can be cancelled from Hearth's Downloads page.

## 3. Download progress

The Downloads page lists Steam downloads beside Epic and GOG ones:

- state and bytes from each record's `StateFlags`, `BytesDownloaded` and `BytesToDownload` (as before);
- speed from how fast those bytes grow, or from Steam's `logs/content_log.txt` (`Current download rate: … Mbps`)
  while the log is being written, because Steam only writes its byte counts now and then;
- what Steam is doing (`AppID <id> update changed : …`), shown as Downloading, Verifying, Installing or
  Making room, and why it stopped (`state changed : … (Disk write failure)`), shown as *Stopped: …*.

Steam doesn't offer a reliable way for another program to pause or reorder its downloads, so Hearth doesn't try.
*Manage in Steam* opens Steam's downloads page in front.

## 4. Uninstalling without a dialog

Hearth asks first (its own Delete sheet). Then `steam_ctl::uninstall`:

1. refuses while Steam is working on the game (`StateFlags` downloading, updating, checking, running…), and while
   the game or any other Steam game runs;
2. if Steam is running, closes it properly first, because Steam keeps its own list of installed games and would
   write the record back. If another download is going, it refuses instead;
3. reads the record again (Steam writes records as it closes) and checks it once more;
4. sets the record aside (`.acf.hearth-removing`), so a half-done removal never looks installed. Then it
   deletes, **only inside that Steam library**: `steamapps/common/<installdir>` (one plain folder name: no `/`, no
   `..`; a link there is removed, never followed), the game's shader cache, unfinished download, temp files and
   Workshop items, and finally the record. Anything whose real parent isn't the library's `steamapps` folder is
   left alone. Proton prefixes (`compatdata`, where saves live) are kept.

Steam notices the game is gone the next time it starts.

## 5. Steam only when needed

Settings › Library › **Start Steam only when needed** is on by default:

- Hearth doesn't start Steam when it starts.
- Steam starts hidden when you play a Steam game, install one, or use Steam's features (messaging or joining
  a friend, Manage in Steam, signing in).
- Every 30 seconds Hearth checks whether it can close a Steam **it started**. It closes it only when nothing has
  used Steam for 10 minutes and Steam has been up that long, no Steam game runs (none of Hearth's, no Steam
  reaper process), no record shows a download, update or check, `content_log.txt` hasn't changed in 90 seconds,
  `cloud_log.txt` hasn't changed in 2 minutes (cloud sync), no install or sign-in is waiting, and Steam's own
  window isn't in front. Then it runs `steam -shutdown`, which lets Steam finish properly. Hearth never kills
  Steam.

With the setting off, Steam starts hidden when Hearth starts and keeps running, as before.

## 6. Signing in

Library › Stores › Sign in to Steam (also in Game Mode):

1. In Game Mode, a Hearth sheet explains what comes next: Steam's window, the QR code for the Steam app on your
   phone, or typing your name and password. *Use Desktop Mode instead* is still offered.
2. Hearth starts Steam hidden if it isn't running, opens `steam://open/main` (Steam shows its sign-in window when
   nobody is signed in) and brings Steam's window in front (gamescope focus, `showSteam` in
   `hosts/hearth-rs/src/gamemode.rs`).
3. While you sign in, the Steam card says so and offers *Show Steam's window* and *Cancel*. The Guide button
   brings you back to Hearth at any time.
4. Hearth reads Steam's files every 2 seconds (for up to 15 minutes): signed in means an account is marked
   `MostRecent` in `loginusers.vdf` **and** Steam wrote that file, or changed the account or `AutoLoginUser` in
   `registry.vdf`, after the sign-in began. Then Hearth comes back in front, says who's signed in, and fetches
   that account's games.

## Tests

`cargo test` in `hosts/hearth-core` covers: reading the games XML and page (with private profiles), the cache
rules, reading `appinfo.vdf` in versions 27, 28 and 29, `StateFlags` decoding, the install record Hearth writes
(it reads back as Steam's format), when a record counts as taken up / gone / timed out, picking the library,
reading `content_log.txt`, removing files only inside a library (links, `..`, records outside a library),
when Steam may be closed, and noticing a sign-in from Steam's files.

What needs a real Steam account and device (it can't run in CI): Steam actually taking up a Hearth-written
record, the fallback window, the Community page answers for a real profile, the sign-in window in gamescope,
and closing Steam after 10 idle minutes.

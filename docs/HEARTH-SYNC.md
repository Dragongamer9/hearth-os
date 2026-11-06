# Hearth Sync

Hearth Sync is a small Decky plugin (`hosts/decky-hearth-sync`) for Steam Game Mode. Its one job: keep the games you
have outside Steam in Steam's library, so you can find and start them from Steam like any other game.

It is what Hearth OS ships in Decky now. The older Hearth plugin (`archive/hosts/decky-hearth`, the one that took over Steam's
screens) isn't shipped for now; its code stays in the repo.

## What it adds

- Every installed game in Hearth's library that isn't a Steam game: Epic, GOG, Battle.net, Xbox (the Xbox Cloud Gaming
  tile), emulators, recomps, Beacon and games you added by hand.
- Games installed with **Minigalaxy** (GOG; Hearth OS comes with it), read from `~/.config/minigalaxy` and its install
  folder (each game's `gameinfo` or `goggame-<id>.info`). Linux games run their `start.sh`; Windows games run in the
  game's own Wine prefix, the way Minigalaxy starts them. DOSBox and ScummVM games are left to Minigalaxy.
- Games installed with **Heroic** (Epic, GOG, Amazon), started through Heroic's own `heroic://launch` link.
- Games installed with Epic's **Epic Games Launcher for Linux** (preview), started through its
  `com.epicgames.launcher://` link. Hearth OS doesn't include that launcher (it isn't open source); get it from Epic.
  This part is unverified: Epic hasn't documented the launcher's files, so Hearth Sync looks for install manifests
  shaped like the Windows launcher's under `~/.local/share/com.epicgames.eglv2/`, and only adds them when something on
  the device opens `com.epicgames.launcher://` links.

- What **Hearth Mods' Add to Steam** asked for (`~/.local/share/nimbus/mods/steam-shortcuts.json`, docs/MODS.md):
  "<Game> (Modded)", "<Game> — Hearth Mod Organizer" and "<Game> — Mod Organizer 2", each running `hearth-mods`
  (only entries that run Hearth Mods' own program are taken), with the game's pictures, in the "Modded games"
  collection. Adding one again updates its shortcut; taking it out of the file removes it.

A game installed through two launchers gets one shortcut: the newest install wins.

Each shortcut gets the game's portrait cover, wide picture, hero and logo (from Hearth's art cache and the other
launchers' pictures), and goes into a Steam collection named after its store ("Epic", "GOG", "Battle.net",
"Emulators"...), when Steam's collections can be reached.

## When it syncs

When Steam Game Mode starts, and then a quick check every 5 minutes. The check only re-reads your games when their
files changed (or every 30 minutes), and only touches Steam when something changed. Missing pictures are tried again
every 30 minutes.

## What it never touches

Only shortcuts Hearth Sync made itself (it keeps a list). Your own non-Steam shortcuts and other tools' shortcuts stay
as they are. The first time, it takes over the game shortcuts the older Hearth plugin made, so nothing appears twice.
It never changes Steam's screens, and it never signs in to anything. If Steam changes in a way it doesn't expect, it
turns itself off with a note in its panel instead of breaking Steam.

## Quick Access

Quick Access › Hearth Sync shows how many games are synced, when it last synced and any problems, with **Sync now**,
a switch per store (all on; turning one off removes that store's shortcuts) and **Remove all Hearth Sync shortcuts**.

## Games from other Steam folders

Games installed while the PC used Flathub's Steam live in `~/.var/app/com.valvesoftware.Steam/.local/share/Steam`, and
other drives can hold `SteamLibrary` folders Steam was never told about; the Steam in `~/.local/share/Steam` doesn't see
them. Hearth Sync's panel lists them under **Other Steam folders** (a game counts when its `appmanifest_<appid>.acf` and
its `steamapps/common/<installdir>` folder are both there, and Steam doesn't already have it).

**Bring them into Steam** (press twice) restarts Steam; while it's closed the games come in:

- on the same drive as one of your Steam's libraries: moved there (a rename, instant): the game's folder, its Proton
  prefix `compatdata/<appid>` (where the saves are) and its `shadercache/<appid>`, then the manifest last;
- on another drive: that folder is added to Steam's library list (`steamapps/libraryfolders.vdf`, backed up first as
  `libraryfolders.vdf.hearth-bak`); nothing is copied. Only a real library folder, never a whole Steam install.

Nothing is overwritten: a game whose folder is already in the target is skipped and named; an existing Proton prefix
at the target stays, and the old one is left where it was (said in the result). A move that fails halfway is undone.
The panel shows the counts afterwards; `~/.local/state/hearth/steam-import.log` keeps them.

From the desktop: `hearth-steam-import` (lists, then asks), `--yes`, `--json`, `--wait` (waits for Steam to close).
It refuses while Steam runs.

## Starting a game

Hearth's games start with `nimbus --play-now <id>` (Hearth's own launch logic; Steam's environment is cleaned first),
so Steam tracks the play time. Minigalaxy games start directly. Heroic and Epic Games Launcher games start through
their launcher, which returns right away, so Steam may not count their play time.

Every shortcut runs `/bin/sh` (Hearth's small wrapper), never a Windows program: Hearth starts Windows games (GOG,
Epic...) with Proton itself, through umu. So:

- **Don't set a compatibility tool on a Hearth Sync shortcut** (Properties › Compatibility › "Force the use of a
  specific Steam Play compatibility tool"): Steam then runs the wrapper through Proton or inside its runtime's
  container and the game never starts. Hearth Sync's panel names such games and says how to turn it off (it reads
  Steam's `config/config.vdf`, never changes it), and a start that ends up inside Steam's container writes
  "play-now: <id>: not started: … inside its container" to Hearth's log.
- **The game's window gets Steam's number.** Steam Game Mode only shows windows carrying the app id Steam gave the
  shortcut. A game started through umu and Proton labelled its windows with umu's own number instead, so GOG and Epic
  games ran with nothing on screen (1.3–1.5). `--play-now` now puts the shortcut's number (from `SteamGameId`) on every
  window of the game's own processes (`STEAM_GAME`) and logs "window … gets Steam's id …".
- **Why a start failed** is in Hearth's log: "GOG <game>: launch failed because …" (no game process, or it closed
  within 30 seconds, with the last error lines of Proton's output). umu's and Proton's output of each game's last start
  is kept in `~/.local/share/nimbus/launch-logs/` and Send logs shows the newest three (plus Steam's own logs from
  `~/.steam/debian-installation`, Debian's Steam).

## Logs

The plugin's log is in `~/homebrew/logs/Hearth Sync/` (counts and reasons only, never paths or addresses) and goes into
Hearth's bug reports along with whether the older Hearth plugin is still there.

# Hearth Music

Hearth Music is a small Decky plugin (`hosts/decky-hearth-music`) for Steam Game Mode. It plays the theme song of the
game you're looking at while you browse Steam's library, crossfades when you move to another game, fades out the
moment you press Play and stays silent while a game runs. It ships in the image next to Hearth Sync and goes into
Decky's plugins folder with it (Settings › System › Steam Game Mode, Decky).

## Where the song comes from

In this order:

1. **A song you picked**: in its Quick Access panel, *Choose music…* searches your soundtracks, Apple Music and
   Deezer for the game on screen (it starts with "<title> soundtrack"; type other words to search again) and keeps the
   one you choose; *Pick a song…* takes an audio file of yours; *Pick a song for <collection>…* is for every game in
   one of its Steam collections (Hearth Sync's store collections count: "Epic", "GOG"...). A game's own pick wins over
   its collection's, and a pick always wins over the automatic song; the library scan never changes it. *Use the
   automatic song* forgets the game's pick. A pick from Apple or Deezer keeps only the track's id and names, never its
   sound, and stays silent (saying why) while that service's previews are turned off.
2. **The game's own soundtrack**, when you have it: Steam soundtrack folders (`steamapps/music/<name>`, or
   `steamapps/common/<Game> Soundtrack` / `OST`) in any of your Steam libraries; the track named like a main/title
   theme, otherwise the first track. Then, conservatively, an audio file in the game's own folder whose name clearly
   says it's the main or title theme ("Main Theme", "Title Theme", "Main Menu"...) and is song-sized (300 KB-60 MB).
   Most games pack their music in archives, so this rarely finds one; it never guesses from vague names.
3. **Apple Music's official 30-second preview**, from Apple's iTunes Search API (`itunes.apple.com/search`,
   `media=music`): it searches "<title> main theme", then "<title> soundtrack" if the first wasn't clearly right, with
   Steam's ™/®, years ("Tomb Raider (2013)") and edition words ("Game of the Year Edition", "Remastered", "Deluxe
   Edition") left out. It plays nothing rather than a wrong song: a result counts only when its album is the game's
   own soundtrack, its name being the game's whole title followed only by soundtrack words ("CONTROL (Original
   Soundtrack)", "Hades: Original Score", "Vol. 2"), never another word ("Inside Out", "Hades II", "Celeste: Farewell"
   and "Hollow Knight" aren't "Inside", "Hades", "Celeste" or "Hollow Knight: Silksong") and never a film's or series'
   ("Original Motion Picture Soundtrack", "Music from the Netflix Series"); or, for titles of two words or more, when the
   track is the game's title plus a theme word ("Hollow Knight Main Theme") on an album that names the game or is a
   soundtrack. A one-word title ("Control", "Prey", "Inside") needs the game's soundtrack album. Within that album the
   main/title theme or the title track wins, then the first track. Covers, tributes, remixes, piano/lo-fi/8-bit or
   "epic" versions and compilations ("Video Game Themes", cover artists) are left out by title, album or artist, and
   the preview must come from Apple's own servers. The preview is streamed from
   Apple's address and never stored (see "How it works"). The panel and the "Now playing" note show the track, the artist,
   "Preview courtesy of Apple Music" and an *Open in Apple Music* link. Can be turned off (*Apple Music previews*).
4. **Deezer's 30-second preview**, only when Apple has no good match (not when Apple merely can't be asked right
   now): Deezer's public search (`api.deezer.com/search`, no account or key), the same two searches and the same
   rules (the game's title, a soundtrack album or theme-named track, no covers or remixes, the preview from Deezer's
   own servers, `*.dzcdn.net`). Streamed, never stored; the panel and the note say "Preview from Deezer" with an *Open
   in Deezer* link. Deezer's developer guidelines forbid storing its audio ("Local Storage/Offline Storage of audio
   data is strictly forbidden"), and its preview addresses are signed and expire, so only the track id is kept and the
   current address is asked for (`api.deezer.com/track/<id>`) when it plays, kept in memory until just before it
   expires. Can be turned off (*Use Deezer previews*, on by default; if you had turned Apple's previews off before
   Deezer came, Deezer starts off too). With both off nothing is ever sent anywhere. No good match anywhere: silence.

**No YouTube.** Hearth Music never plays, extracts or downloads anything from YouTube (or Invidious, Piped, yt-dlp and
the like): YouTube's terms don't allow audio-only background playback outside its own visible player, and Hearth OS
has a no-piracy rule. A test fails if any of it shows up in the plugin.

### The library scan

So that games start their song right away instead of waiting for a search, Hearth Music goes through your whole
library in the background the first time it runs: every game in Steam's library, plus non-Steam shortcuts (Hearth
Sync's store games among them) by the names Steam shows. Installed games first, then the rest of the library, then
the shortcuts. For each game it finds the best match in the order above (your pick is left as it is; a soundtrack you
have; Apple's preview; Deezer's) and keeps only the answer: the source, the track id, its title, artist and album,
and for Apple the preview address (Deezer's expire, so not those). It's polite: Apple at most once every few seconds
and 20 times a minute (Apple asks for about 20), Deezer once a second; when either answers 429 or 403 (or Deezer says
its quota is used up) it waits a minute, doubling up to an hour. It waits while a game runs (and after Play is
pressed), and if it's stopped (Steam restarted, the device turned off) it carries on where it was: games answered
recently are skipped. New installs are matched when they show up (the library is checked every 15 minutes), and
answers older than a week are refreshed. *Scan all games* in the panel checks every installed game and non-Steam game again with today's matching, ignoring the answers kept (your picks are never looked up or changed); it shows "Checking every game: 24 of 64 · INSIDE" and can be cancelled, keeping what it found so far. The panel shows how far it is: "Matched 143 of 210", and whether it's looking, paused while a
game runs, or waiting for the network or a service that asked to slow down. If the network is gone, the answers kept
stay exactly as they were.

The answers are kept in `~/.cache/hearth-music/lookups.json` (the scan's last count in `scan.json`), written the safe
way: all at once with the previous version kept as `lookups.json.bak`, a damaged file put back from it, and the scan
and the panel's own lookups merged under one lock so neither loses the other's. A match is used for 30 days, "no
match" for 7, a failed call is tried again after an hour; the scan refreshes them weekly. Each answer notes the
matcher that found it: when the matching rules get stricter (matcher 2, after 1.2 picked wrong songs for some games),
the scan looks every automatic answer up once more, and an old answer that fails today's rules is never played, not
even offline. Your own choices from *Choose music…* live in `picks.json`, not in this cache, and are never looked up,
re-checked or changed. Only these answers are kept; no preview's sound ever is.

## Quick Access panel

- **Now playing**: the game, the song and where it's from (and Apple's or Deezer's credit and link), or why it's silent.
- **Hearth Music** on/off, **Volume**, **Wait before playing** (0-5 s, default 1.5 s: flicking through the library
  doesn't start every song), **In the library too** (off: game pages only; on: also the highlighted tile in the
  library), **Apple Music previews**, **Use Deezer previews**, **Now playing note** (a short toast the first time a game's song plays).
- **Song for <game>**: *Choose music…* and the picks above.
- **Your library**: the scan's count, *Scan all games* (or *Cancel scan* while it runs) and *Review music for every
  game*: every installed game and non-Steam game in three groups, **No music found**, **Found automatically** and
  **Your picks**, each with its song, album and where it's from (Apple Music, Deezer, your soundtrack or your file),
  a ▶ to hear it, and *Fix*: *Choose music…* (the search box starts with the game's name without ™/®, years or
  editions; each result can be heard with ▶ before you *Choose* it), *No music for this game* (kept as your pick, so
  that game stays silent) and *Back to automatic*. Your choices are kept in `picks.json` like every pick (saved all at
  once with a `.bak`) and only you change them.
- Answers the panel heard during this session are asked again after five minutes and when a scan finishes, so what a
  scan or a fix found plays right away; while a music service asks to wait, an older answer that passes today's rules
  keeps playing.

These settings are kept in `~/homebrew/settings/decky-hearth-music/settings.json`, saved the safe way
(docs/SETTINGS-PERSISTENCE.md: only the key you changed is merged into the latest file, atomically, with a `.bak`; a
damaged file comes back from it). With Hearth Link's **Sync Hearth settings** on, they follow you to your other Hearth
devices one key at a time (docs/HEARTH-LINK-PLUGIN.md; *Use Deezer previews* isn't among the keys it carries yet, so
it stays per device); your picks (`picks.json`) and the lookup answers (`lookups.json`, a cache) stay on each device.

## How it works

The frontend reads Steam's gamepad UI the way Hearth Sync and the older Hearth plugin do, every read guarded:

- the page on screen: the main window's router location (`/library/app/<appid>` is a game page);
- the highlighted library tile: a focus listener on Steam's window; the app id comes from the focused tile's React
  props (`app.appid`, `appOverview.appid`...);
- the running game: `Router.MainRunningApp`; Play pressed: `SteamClient.Apps.RegisterForGameActionStart` (a quick
  fade out, then silent for 30 s or until the game runs);
- a game's name and collections: `appStore`, `collectionStore`.

If Steam's page and running game can't be read at all, Hearth Music turns itself off with a note in its panel.

Songs play in Steam's own browser (an HTML audio element in the plugin's page), so they go out with Steam's sound and
follow Game Mode's volume; two elements crossfade (equal power, 1.5 s).

- **Apple's and Deezer's previews are streamed**, following their terms: the audio element plays the https preview address
  directly. If Steam's browser refuses it (its page's rules, a mixed origin), the backend fetches the preview into
  memory and passes it on, through ffmpeg into Ogg Opus by pipe when needed. Nothing of it is written to disk; only
  the lookup answers (`lookups.json`) are kept. Tests check this for both, file by file.
- **Your own files** (picks, soundtracks, game files) are handed over by the backend; a format Steam's browser can't
  decode is made into Ogg Opus with ffmpeg first (kept in `~/.cache/hearth-music/ogg`). Only files the backend chose
  can be asked for (by key, never by path).

## Tests

`.github/workflows/decky-hearth-music.yml`: the backend's tests with made-up answers and no network (matching, the
order of sources, Deezer only when Apple has no match and its address asked for when it plays, the lookup cache
written atomically with a .bak, a failed call leaving it as it was, the rate limits and backing off on 429/403, the
scan's order, resuming and pausing while a game runs, picks winning and surviving a rescan, the Deezer switch on by
default and remembered, Apple's and Deezer's previews never written to disk, fake Steam folders, hearth-decky shipping
it), the pure parts' unit tests (credits, the scan's lines), the build and Decky's zip, and pictures of the real panel
(also the scan's progress and *Choose music…*) and the "Now playing" note over stand-ins (branch
`ci/hearth-music-shots`).

## Not verified on a device yet

Steam's internals aren't documented: the router path, the tile's React props and `RegisterForGameActionStart` are
read the way other Decky plugins read them, but haven't been checked on this Steam build. Where Steam keeps the whole library
for the scan (`collectionStore.allGamesCollection`, `appStore.allApps`; installed games are read from Steam's
folders either way) is read the same guarded way. Whether Steam's browser plays
Apple's and Deezer's preview addresses directly (otherwise the backend's in-memory stream, transcoded if needed) is also unchecked.

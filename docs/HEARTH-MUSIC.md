# Hearth Music

Hearth Music is a small Decky plugin (`hosts/decky-hearth-music`) for Steam Game Mode. It plays the theme song of the
game you're looking at while you browse Steam's library, crossfades when you move to another game, fades out the
moment you press Play and stays silent while a game runs. It ships in the image next to Hearth Sync and goes into
Decky's plugins folder with it (Settings › System › Steam Game Mode, Decky).

## Where the song comes from

In this order:

1. **A song you picked**: in its Quick Access panel, *Pick a song…* for the game on screen, or *Pick a song for
   <collection>…* for every game in one of its Steam collections (Hearth Sync's store collections count: "Epic",
   "GOG"...). A game's own pick wins over its collection's. *Use the automatic song* forgets the game's pick.
2. **The game's own soundtrack**, when you have it: Steam soundtrack folders (`steamapps/music/<name>`, or
   `steamapps/common/<Game> Soundtrack` / `OST`) in any of your Steam libraries; the track named like a main/title
   theme, otherwise the first track. Then, conservatively, an audio file in the game's own folder whose name clearly
   says it's the main or title theme ("Main Theme", "Title Theme", "Main Menu"...) and is song-sized (300 KB-60 MB).
   Most games pack their music in archives, so this rarely finds one; it never guesses from vague names.
3. **Apple Music's official 30-second preview**, from Apple's iTunes Search API (`itunes.apple.com/search`,
   `media=music`): it searches "<title> main theme", then "<title> soundtrack" if the first wasn't clearly right, and
   takes the result whose album or track name holds the game's title (or its short form without edition words or
   subtitle) together with a soundtrack album or a theme-named track; covers, tributes, remixes, karaoke, lo-fi and
   8-bit versions are left out, and the preview must come from Apple's own servers. The preview is streamed from
   Apple's address and never stored (see "How it works"). The panel and the "Now playing" note show the track, the artist,
   "Preview courtesy of Apple Music" and an *Open in Apple Music* link. No good match: silence.
   Can be turned off (*Apple Music previews*); then nothing is ever sent anywhere.

Hearth Music never uses YouTube or anything that downloads from it.

Apple asks for about 20 calls a minute at most: Hearth Music waits a few seconds between calls, stops for the minute
when it reaches 20, and keeps every answer in `~/.cache/hearth-music/lookups.json`: a match for 30 days, "no match"
for 7 days, a failed call for an hour. So each game is looked up once. Only these answers are kept; Apple's audio
never is.

## Quick Access panel

- **Now playing**: the game, the song and where it's from (and Apple's credit and link), or why it's silent.
- **Hearth Music** on/off, **Volume**, **Wait before playing** (0-5 s, default 1.5 s: flicking through the library
  doesn't start every song), **In the library too** (off: game pages only; on: also the highlighted tile in the
  library), **Apple Music previews**, **Now playing note** (a short toast the first time a game's song plays).
- **Song for <game>**: the picks above.

These settings are kept in `~/homebrew/settings/decky-hearth-music/settings.json`, saved the safe way
(docs/SETTINGS-PERSISTENCE.md: only the key you changed is merged into the latest file, atomically, with a `.bak`; a
damaged file comes back from it). With Hearth Link's **Sync Hearth settings** on, they follow you to your other Hearth
devices one key at a time (docs/HEARTH-LINK-PLUGIN.md); your picks (`picks.json`, file paths) stay on each device.

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

- **Apple's previews are streamed**, following Apple's terms: the audio element plays the https preview address
  directly. If Steam's browser refuses it (its page's rules, a mixed origin), the backend fetches the preview into
  memory and passes it on, through ffmpeg into Ogg Opus by pipe when needed. Nothing of it is written to disk; only
  the lookup answers (`lookups.json`) are kept.
- **Your own files** (picks, soundtracks, game files) are handed over by the backend; a format Steam's browser can't
  decode is made into Ogg Opus with ffmpeg first (kept in `~/.cache/hearth-music/ogg`). Only files the backend chose
  can be asked for (by key, never by path).

## Tests

`.github/workflows/decky-hearth-music.yml`: the backend's tests (matching, the order of sources, the lookup cache and
Apple's rate limit with made-up answers, Apple's previews never written to disk, fake Steam folders, hearth-decky shipping it), the pure parts' unit tests, the build
and Decky's zip, and pictures of the real panel and the "Now playing" note over stand-ins (branch
`ci/hearth-music-shots`).

## Not verified on a device yet

Steam's internals aren't documented: the router path, the tile's React props and `RegisterForGameActionStart` are
read the way other Decky plugins read them, but haven't been checked on this Steam build. Whether Steam's browser plays
Apple's preview addresses directly (otherwise the backend's in-memory stream, transcoded if needed) is also unchecked.

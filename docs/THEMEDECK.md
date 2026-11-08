# ThemeDeck (game music in Steam Game Mode)

**ThemeDeck** plays music on Steam's game pages in Steam Game Mode: songs you give each game (from your own music
files, or from YouTube once you turn that on in ThemeDeck), plus optional ambient music for the rest of Steam and
separate Store music, with a "Now Playing" card. It's a Decky plugin by **BrenticusMaximus**, not made by Hearth:
<https://github.com/BrenticusMaximus/ThemeDeck>. It replaced Hearth's own **Hearth Music** in November 2026, the
owner's call (Hearth Music is archived: [ARCHIVED](ARCHIVED.md), [HEARTH-MUSIC](archive/HEARTH-MUSIC.md)).

Open it from Quick Access (the **…** button) › Decky › ThemeDeck. How to use it is ThemeDeck's own README.

## What Hearth ships, and how

- **Unmodified, at a pinned release: ThemeDeck v4.0.0**, the release ZIP
  `https://github.com/BrenticusMaximus/ThemeDeck/releases/download/v4.0.0/ThemeDeck-v4.0.0.zip`, SHA-256
  `9a2bd31bf4a9daeb2a2c3ba1e3b94fab4259dac27166a2098405580e3063b9c0`.
- The image build (`os-bases/peppermint/build.sh`) downloads that ZIP, checks it against the SHA-256 (a mismatch or a
  failed download **fails the build**), and unpacks it as it is into `/usr/share/hearth/decky-third-party/ThemeDeck`.
  Hearth changes none of its files. The image check (`base-peppermint.yml`) confirms it's there with its licence.
- `hearth-decky` (Decky's service, before Decky starts in Steam Game Mode) copies it into
  `~/homebrew/plugins/ThemeDeck`, the folder Decky's store would use, so its settings are in
  `~/homebrew/settings/ThemeDeck` either way. Hearth's copy has a small `.hearth-built-in.json` beside ThemeDeck's
  files (which version Hearth put there); a Hearth OS update with a newer pinned ThemeDeck replaces only that copy.
- **A ThemeDeck you installed or updated yourself** (from Decky's store or a ZIP: no `.hearth-built-in.json`) is
  yours: Hearth never replaces or removes it.
- It's updated **with Hearth OS** (bump `THEMEDECK_VERSION` and `THEMEDECK_SHA256` together in `build.sh`), not
  through the Decky plugins release: it isn't one of Hearth's own plugins (docs/PLUGIN-UPDATES.md explains why).
- Hearth Link doesn't sync anything of ThemeDeck between your devices (its files are songs on that device) and never
  offers it to install from another device (every Hearth OS device has it).

## YouTube support is ThemeDeck's own, and off until you ask

Hearth OS **doesn't preinstall** `yt-dlp`, Deno or any YouTube tooling (the image check makes sure neither is
there). ThemeDeck sets these up itself, in its own settings folder, only when you press its own
**Install YouTube support** button; its "Use Firefox cookies for YouTube" stays off unless you turn it on. Hearth has
no YouTube code of its own. Using YouTube's audio is between you, ThemeDeck and YouTube's terms.

## Your Hearth Music songs

At the first Steam Game Mode start after the update, before Decky starts, `hearth-decky` carries your Hearth Music
picks over to ThemeDeck **once** (`music_to_themedeck`), as you, then takes Hearth Music away:

- **Carried over:** every music file of yours that you picked for a game in Hearth Music
  (`~/homebrew/settings/decky-hearth-music/picks.json`) becomes that game's song in ThemeDeck
  (`~/homebrew/settings/ThemeDeck/tracks.json`, in ThemeDeck's own format: full volume, no start skip, looping, as
  ThemeDeck sets a new song).
- **Not carried over:** Apple Music and Deezer previews you chose (they were streamed, never stored, so there's no
  file), songs picked for a collection (ThemeDeck has songs per game), "No music for this game" (a game without a song
  is silent in ThemeDeck anyway), files that are gone, and songs Hearth Music found by itself (the game's soundtrack
  folder, previews). Hearth Music's volume and other panel settings don't carry over either (ThemeDeck has its own).
- **Your ThemeDeck songs win:** a game you already gave a song in ThemeDeck keeps it; everything else in ThemeDeck's
  file (ambient and Store music, other games) stays as it is.
- **The settings rules** (docs/SETTINGS-PERSISTENCE.md, `hearth_settings_io.py`): merged under the file's lock,
  written atomically with the file before kept as `tracks.json.bak`, and nothing is written over a `tracks.json` that
  can't be read (it's tried again at the next start instead). Hearth Music's own files are only read, never changed,
  and stay where they are (`~/homebrew/settings/decky-hearth-music`, `~/.cache/hearth-music`).
- What happened is in `~/.local/state/nimbus/themedeck-from-hearth-music.json` (when, how many carried over, kept and
  skipped and why) and on Decky's service log line (`musicPicks`). A failure never stops Decky or Steam Game Mode: it
  only shows there.

## Licence and credit

ThemeDeck is free software under the **BSD-3-Clause** licence. Its `LICENSE` ships with it, in the plugin's folder
(`/usr/share/hearth/decky-third-party/ThemeDeck/LICENSE`, and in `~/homebrew/plugins/ThemeDeck` with every copy) and
as `/usr/share/doc/themedeck/copyright`:

```
BSD 3-Clause License

Copyright (c) 2025, BrenticusMaximus
Original Copyright (c) 2022-2024, Steam Deck Homebrew

All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

Hearth OS doesn't use ThemeDeck's or its author's name to promote Hearth OS; it's listed here as what it is, a
third-party plugin Hearth OS includes. Support for ThemeDeck itself is its author's (see its README).

## Tests

`hosts/linux/test/test_themedeck.py` (run with the other `hosts/linux` tests): the copy (unmodified, licence included,
yours left alone, links never followed, the update to a newer pinned version), Hearth Music retired (Hearth's copy
only, its settings kept, a download never put back), the carry-over (files yes, previews and the rest no, merged
without overwriting your ThemeDeck songs, once, `.bak`, a damaged file never written over, Hearth Music's files only
read, Decky still starting when it fails), and the pin in `build.sh` (version, SHA-256, `sha256sum -c`, no YouTube
tooling). On a device: that ThemeDeck loads and plays a carried-over song on the game's page.

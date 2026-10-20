# Beacon in Hearth

[Beacon](https://github.com/Dragongamer9/beacon) is the Linux launcher for The Lord of the Rings: The Battle for
Middle-earth games (BFME, BFME II, The Rise of the Witch-king and their mods), the fan-made Age of the Ring,
The Lord of the Rings: Conquest, and the community versions of Universe at War, Armies of Exigo, Rise of Nations:
Rise of Legends and Star Wars: Galactic Battlegrounds. Hearth OS ships it, and Hearth shows its games in the
library next to Steam, Epic, GOG and the rest.

Hearth doesn't read Beacon's files or start the games itself. It runs **Beacon's own command-line tool, `beacon`**,
and reads what it prints. Beacon stays in charge of game folders, Wine/Proton, prefixes, CD keys, community files
and mods. The code is `hosts/hearth-core/src/beacon.rs`.

Since Beacon 0.0.6, **everything Beacon does has a screen in Hearth**: a Beacon game's page › Game setup and Mods
(see "Beacon's screens in Hearth" below). Beacon's own window is only needed outside Hearth.

## What Hearth runs

Always Beacon's own executable with a list of arguments. Never a shell, and never anything Beacon's output names.

| Command | What for |
|---|---|
| `beacon status --json` | Every game Beacon knows, with its state (Beacon's `GameStatus` list) |
| `beacon mods <game>` | One game's mods (only for games that are set up) |
| `beacon select-mod <game> <mod id\|none>` | Game settings › Mod: the mod Play starts with. Beacon keeps it, so Beacon's window shows the same choice |
| `beacon launch <game> [--mod=<id> \| --vanilla] --wait` | Play (Beacon's chosen mod), or Play with… (one mod, or none, this time only) |
| `beacon-app` / `beacon-app --fullscreen` | Open Beacon. Full screen in Game Mode, a window in Desktop Mode |
| `beacon --json <command> …` | Game setup and Mods (Beacon 0.0.6+): the table under "Beacon's screens in Hearth" |

`<game>` is always one of Beacon's game ids: `bfme1 bfme2 rotwk aotr conquest uaw exigo rol swgb` (`GameId`'s
serde names). Hearth checks it against that list. Mod ids are checked before they go on a command line:
letters, digits and `. _ : -` only, up to 128 characters, not starting with `-`, `.` or `:`. Beacon's ids
(`<game>:<slug>`, like `rotwk:edain`) always pass. The mod goes in as one `--mod=<id>` argument, so it can never
be read as another option.

### Where `beacon` comes from

The first that exists and is executable:

1. `beacon` in an absolute `PATH` folder, `~/.local/bin`, `/usr/bin`, `/usr/local/bin` or `/opt/beacon/bin`. Hearth OS
   installs Beacon's release package, which puts it in `/usr/bin`.
2. Beacon's Flatpak (`io.github.dragongamer9.beacon`, Beacon's Tauri identifier), **only if the Flatpak ships the tool**
   (`files/bin/beacon`), run as `flatpak run --command=beacon --env=HEARTH_GAME_ID=<n> io.github.dragongamer9.beacon …`.
   Beacon doesn't build a Flatpak today, so this is only for later.

`beacon-app` is looked for the same way. A Flatpak runs it as `flatpak run io.github.dragongamer9.beacon`.

## What Hearth reads

### `beacon status --json`

Hearth uses these fields and ignores the rest:

| Field | Use |
|---|---|
| `id` | The library id is `beacon:<id>`. Games with an id Hearth doesn't know are left out (and logged) |
| `family` | `bfme`, `lotr` or `other`: the tile's shade |
| `title`, `short` | Names (control characters removed, length capped) |
| `path` | Shown on the game's page. Absolute paths only |
| `installed` | Set up in Beacon. Games that aren't set up show **Set up**, which opens Game setup |
| `ready`, `issues[{level, message}]` | The first `error` issue blocks Play. Hearth shows Beacon's own message; Game setup offers the fix. A `warn` issue shows as a note |
| `selected_mod` | The mod Play starts with (checked like any mod id) |
| `edition` | Shown in About (for example a community edition) |
| `steam_appid`, `via_steam` | Steam art. A game Beacon starts through Steam is also tracked by Steam's app id |

### `beacon mods <game>`

Beacon prints one mod a line: `"{star} {id:<30} {kind:<9} {path}"`, where the star is `*` for the chosen mod.
Hearth keeps a line only if all of these hold:

- it starts with `*` or a space;
- its id is safe;
- its kind is `mod` or `launcher`;
- its path is absolute.

Anything else is skipped, including the "No mods…" line and a line that only looks like a mod because of a space
in it. Hearth makes a mod's name from its id: `rotwk:age-of-the-ring` becomes "Age of the Ring". At most 200 mods
are kept per game.

The library still reads this text form (it works with every Beacon); Game setup and Mods use `beacon --json mods`,
which has the real names.

**Welcome in Beacon:** a `beacon status --json` field with each game's art (or bundled pictures) would let the
tiles show it.

## Beacon's screens in Hearth (Beacon 0.0.6+)

Beacon 0.0.6 gave every command a `--json` form (Beacon's README, "For Hearth / scripting"; its
`crates/beacon-core/src/report.rs`). Hearth always puts `--json` first (`beacon --json args rotwk -- -win`, so it can
never become one of the game's arguments). Reading commands print one JSON document; changing commands print
`{"event":"progress","pct":40,"text":"…"}` lines while they work, then one result line, `{"ok":true,"message":…}` or
`{"ok":false,"kind":"usage|failed|program","error":…}`, with exit codes 0, 2 (bad arguments), 1 (failed) and 3 (the
Windows program closed with an error).

| Page call (`bridge.rs` → `beacon::bridge_call`) | What Hearth runs | Checked first |
|---|---|---|
| `beacon.details {id}` | `status`, `mods <g>`, `catalog <g>`, `community <g>` (games that have community versions), `runners` | game id |
| `beacon.set {id, what, value}` | `set-runner`, `resolution`, `windowed`, `edition`, `virtual-desktop`, `via-steam`, `args <g> -- <text>` | runner: in `runners`' list; resolution `WxH`, 640×480 to 16384×16384, or `clear`; edition `auto` or a lowercase id; on/off/default; arguments: up to 400 characters, no control characters |
| `beacon.setKey {id, key \| clear}` | `set-key <g>` with **the key on stdin**, or `set-key <g> --clear` | letters, digits and dashes, 4–64 characters. Never on a command line, never logged |
| `beacon.locate {id}` | folder picker, then `set-path <g> <folder>` | an absolute folder that exists |
| `beacon.addMod {id, folder?, select?}` | file or folder picker, then `add-mod <g> <path> [--select]` | an absolute file/folder that exists |
| `beacon.removeMod {id, mod}` | `remove-mod <id>` | a safe mod id of that game (`<g>:…`) |
| `beacon.job {id?, job, step?, folder?}` | long jobs, below | as above |
| `beacon.log {id}` | `log <g> --lines 300` | game id |
| `beacon.openLink {url}` | Hearth Browser (`/usr/bin/hearth-browser <url>`), else `xdg-open` | `https://`, one line, no spaces or quotes |

Long jobs (`beacon.job`):

| `job` | Runs | Time limit |
|---|---|---|
| `setup` | `setup <g>`: Prepare the game | 30 min |
| `install` | file picker, then `run <g> <setup.exe>`: install a game from its disc or download | 6 h |
| `patch` | file picker, then `run <g> <exe>`: a patch or mod installer | 6 h |
| `tool` | `run <g> --tool`: AotR's own launcher (when `status` has `tool_launcher`) | 6 h |
| `scan` | `scan`: Find games | 30 min |
| `findMods` | `find-mods`: Find mods on this device | 30 min |
| `addFiles` | file or folder picker, then `add-files <g> <step> <path>`: a community version's download | 6 h |

How it runs:

- **Never on the bridge thread.** Reads and quick changes answer later from a thread (`space::answer_later`), with a
  20-second limit per Beacon call. Long jobs run on their own thread and send `beacon.progress {game, job, pct, text}`
  events, then `beacon.done {game, job, ok, message | error, found, added, installed}`. File and folder pickers open
  first, as elsewhere in Hearth (the fail-safe is told a person is choosing).
- **One change at a time** (`beacon::Busy`): each Beacon process reads its config when it starts and writes it when it
  ends, so two at once would lose one's change. A second change while one runs says "Beacon is still busy (…)".
- **Read again afterwards:** after every change Beacon's games are read again on that same thread (`beacon::refresh`)
  and handed to the library (`Library::beacon_slot`), which uses them on its next tick and tells the page
  (`library.changed`); the answer to a quick change is the game's details read again.
- **Every Beacon process has a time limit**; one that runs over is stopped. Long jobs' processes are left alone by
  Hearth's fail-safe (`watchdog.rs`, `beacon::job_pids`) while the page is waiting on something else.
- Everything Beacon prints is checked before the page sees it: known games only, safe mod ids, absolute paths, `https`
  links only, text without control characters and capped in length.
- A Beacon older than 0.0.6 answers only `status` as JSON: Game setup and Mods then say to update Beacon, and Play,
  Play with… and Game settings › Mod keep working.

## Starting a game

`game.launch {id, mod?}` → `Library::launch_with` → `beacon::launch`:

- `beacon launch <game> … --wait`, started with `procs::detach`. stdout and stderr go to
  `~/.local/share/nimbus/beacon-launch.log`.
- `HEARTH_GAME_ID=<n>` (`appid.rs`) is in its environment. Beacon starts Wine/Proton with its environment unchanged
  (its `Command::to_std` only adds variables), so every process of the game carries the number. With `--wait`,
  Beacon stays the parent of the Wine processes. Quick Resume, closing the game, gamescope's window numbering,
  per-game power and Couch Co-op therefore work as they do for Hearth's other games.
- The game's folder is a fallback for finding its processes. For a Steam copy started through Steam (`via_steam`),
  Steam's reaper app id is used too, since Steam doesn't pass Hearth's variable on.
- Play with… passes `--mod=<id>`, or `--vanilla` for no mod. Plain Play passes neither, so Beacon uses its saved
  choice.

**Asks of Beacon:** keep `--wait` waiting for the game, and keep the environment as it is (no `env_clear`).

## Noticing changes

Hearth looks every 4 seconds at the modification times of these places, native and Flatpak (`beacon::signature`):

- Beacon's config file: `$BEACON_CONFIG` or `~/.config/beacon/config.json`;
- its mods folder and each game's folder inside it: `$BEACON_DATA_DIR/mods`, `~/.local/share/beacon/mods/<game>`;
- whether `beacon` is installed.

When something changes, Hearth reads the games again in the background and the library updates. This happens when
you set up a game or add a mod in Beacon's window, or install Beacon. Changing the mod in Hearth reads them again
right away.

## In Hearth

- **Library › Beacon:** the tab appears only when Beacon is installed. Each tile gets a warm shade for its shelf
  (Middle-earth, Lord of the Rings, other games); Steam copies (UaW, SWGB) get Steam's art.
- **Library › Beacon** also has **Find games** (Beacon's `scan`, with its progress on the button) and **Open Beacon**.
- **Game page:** Play, plus Play with… for a game that has mods. The stats show the chosen mod. About shows
  Beacon's notes and what's stopping the game, if anything. A **Beacon** card has **Game setup** and **Mods**; a game
  that isn't set up has **Set up** instead of Play.
- **Game setup:** whether it's ready, Beacon's issues each with its fix (Find it, Add key, Choose a runner, Prepare,
  set up the game it needs, like BFME II for RotWK). Not set up: **Install from a disc or download** (pick the
  installer), **It's already installed** (pick the folder), **Find games on this device**. Set up: **CD key** (a
  hidden prompt; Change or Remove once saved), **Runner** (Automatic or one from Beacon's list), **Resolution**
  (Match the screen, common sizes, or the game's own) and **Windowed** for BFME games, **Virtual desktop** for the
  others, **Launch through Steam** for Steam copies, **Extra arguments**, **Game folder**. **Community versions**
  (Other Games): Version, and each setup step with Open its page / Run the installer / Add the file or folder you
  downloaded. **Tools:** Prepare the game, Run a patch or installer, AotR's own launcher, Game log, Mods, Open Beacon.
  Long jobs show a progress card at the top.
- **Mods:** Play with…, your mods (Play starts with it, Play with it now, Remove from Beacon for mods added from
  another folder), No mod, **Add a mod you downloaded** (file or folder), **Find mods on this device**, and **Mods
  and patches to get** from Beacon's catalog with "Installed as …" and **Open its page in Hearth Browser**.
- **Game log:** the end of the log Beacon writes when it prepares or starts the game.
- **⋯ menu:** Play with…, Game setup, Mods, Open Beacon.
- **Game settings:** Mod (saved in Beacon with `select-mod`), Game setup, Mods, Open Beacon, and Hearth's own options
  for while the game plays: controller layout, frame-rate cap and power limit.
- **Library › Stores:** Beacon's card shows whether it's installed and how many of its games are set up, with
  Open Beacon, Find games and Show games. When Beacon isn't installed it says how to get it: Hearth OS comes with it;
  elsewhere **Get Beacon** opens https://github.com/Dragongamer9/beacon/releases (AppImage, .deb, .rpm).
- **Opening Beacon:** `beacon-app --fullscreen` in Game Mode, `beacon-app` in Desktop Mode or in a windowed Hearth.
  It's a child of Hearth with no game number, so Game Mode puts its window in front like other apps Hearth opens
  (`gamemode.rs`, `APPS_ID`).
- **Desktop Mode (Mantel):** Beacon is in the launcher through its `beacon.desktop`. A new user's taskbar starts with
  Beacon pinned when it's installed (`hearth-desk`'s `DEFAULT_PINS`). Once you change your taskbar, your pins are
  kept.
- **Hearth OS:** each edition's build installs Beacon's latest release (Anatase: the `.rpm`; Ubuntu and Peppermint:
  the `.deb`; CachyOS: the `.tar.gz` under `/usr`). If there's no release to download, the build warns and carries on.
  The image check `beacon --version` is a warning, not a failure.

## Controller: the "Strategy (RTS)" layout

Hearth Input gives Beacon's games the **Strategy (RTS)** layout until you choose something else. Off is always
there: Game settings › Controller layout. LOTR: Conquest is an action game with its own controller support, so it
starts Off. The layout lives in `hosts/linux/desktop/shell/input-layouts.json` (template `rts`).

| Control | Does | Why |
|---|---|---|
| Right stick | Mouse pointer (a little faster, curve 2.0) | Selecting and ordering |
| RT / LT | Left click / right click | Select / order (BFME's default mouse) |
| D-pad, left stick | Arrow keys | Scrolls the map |
| A | Space | Jump to the latest event |
| B | S | Stop |
| X | Q | Select units (BFME's default) |
| Y | H | Jump to your fortress or citadel |
| RB (hold) | Ctrl | Ctrl+click, Ctrl+number |
| L3 (hold) | Shift | Add to a selection, queue orders |
| R3 | Middle click | Hold and move the right stick to turn the camera |
| Menu | Esc | Game menu |
| View | Radial menu | Pick with the right stick or the D-pad |
| L4 / R4, L5 / R5 | Shift, Ctrl, zoom out / in | Back buttons, where there are any |

While **LB is held** (the second set):

| Control | Does |
|---|---|
| D-pad up / right / down / left | 1 / 2 / 3 / 4: select control group 1–4 |
| Y / B / A / X (same places as the D-pad) | Ctrl+1 / Ctrl+2 / Ctrl+3 / Ctrl+4: make control group 1–4 |
| RT / LT | Scroll up / down: zoom |

**Radial menu (8):** 5, 6, 7, 8 (control groups 5–8), then F1, F2, F3, F4 (camera bookmarks).

Space, S, Esc and the arrow keys are the same in every BFME game. Q, H and the F-key bookmarks are the Generals
engine's usual keys and may differ in some mods. Change any of them in Game settings › Controller layout › Edit.

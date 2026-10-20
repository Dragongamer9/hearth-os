# Beacon in Hearth

[Beacon](https://github.com/Dragongamer9/beacon) is the Linux launcher for The Lord of the Rings: The Battle for
Middle-earth games (BFME, BFME II, The Rise of the Witch-king and their mods), the fan-made Age of the Ring,
The Lord of the Rings: Conquest, and the community versions of Universe at War, Armies of Exigo, Rise of Nations:
Rise of Legends and Star Wars: Galactic Battlegrounds. Hearth OS ships it, and Hearth shows its games in the
library next to Steam, Epic, GOG and the rest.

Hearth doesn't read Beacon's files or start the games itself. It runs **Beacon's own command-line tool, `beacon`**,
and reads what it prints. Beacon stays in charge of game folders, Wine/Proton, prefixes, CD keys, community files
and mods. The code is `hosts/hearth-core/src/beacon.rs`.

## What Hearth runs

Always Beacon's own executable with a list of arguments. Never a shell, and never anything Beacon's output names.

| Command | What for |
|---|---|
| `beacon status --json` | Every game Beacon knows, with its state (Beacon's `GameStatus` list) |
| `beacon mods <game>` | One game's mods (only for games that are set up) |
| `beacon select-mod <game> <mod id\|none>` | Game settings › Mod: the mod Play starts with. Beacon keeps it, so Beacon's window shows the same choice |
| `beacon launch <game> [--mod=<id> \| --vanilla] --wait` | Play (Beacon's chosen mod), or Play with… (one mod, or none, this time only) |
| `beacon-app` / `beacon-app --fullscreen` | Open Beacon, Manage mods in Beacon, Set up in Beacon. Full screen in Game Mode, a window in Desktop Mode |

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
| `installed` | Set up in Beacon. Games that aren't set up show **Set up in Beacon**, which opens Beacon |
| `ready`, `issues[{level, message}]` | The first `error` issue blocks Play. Hearth shows Beacon's own message and offers Beacon. A `warn` issue shows as a note |
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

**Welcome in Beacon:** a `beacon mods <game> --json` (Beacon's `ModEntry` list, with `name`) would let Hearth show
the real names and stop parsing text. A `beacon status --json` field with each game's art (or bundled pictures)
would let the tiles show it.

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
- **Game page:** Play, plus Play with… for a game that has mods. The stats show the chosen mod. About shows
  Beacon's notes and what's stopping the game, if anything.
- **⋯ menu:** Play with…, and Manage mods in Beacon.
- **Game settings:** Mod (saved in Beacon with `select-mod`), Manage mods in Beacon, and Hearth's own options for
  while the game plays: controller layout, frame-rate cap and power limit. Wine/Proton settings stay in Beacon.
- **Library › Stores:** Beacon's card shows whether it's installed and how many of its games are set up, with
  Open Beacon and Show games.
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

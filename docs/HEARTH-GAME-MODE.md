# Hearth Game Mode

**From Hearth OS 1.10 you pick which Game Mode your PC goes into:** **Steam Game Mode** (Valve's Steam big-screen mode,
like a Steam Deck; the default, exactly as before) or **Hearth Game Mode** (Hearth's own full-screen screens: every
store in one library, Hearth's Store, Quick Access and Settings). Nothing changes for anyone who doesn't pick.

## Where to pick

| Where | How |
| --- | --- |
| Desktop Mode (Mantel) › Settings › **Gaming** › **Game Mode** | Two picture cards. Up/Down to reach them, Left/Right between them, A to pick (or click). |
| Hearth Game Mode › Settings › **System** › **Game Mode** | The same two cards. |
| Steam Game Mode › Quick Access (…) › **Hearth Sync** › **Game Mode** | Two buttons in Hearth Sync's panel (Decky). |

Picking one saves it at once and it applies **the next time Game Mode starts** (turning on, signing in, the Game Mode
button, Server mode's Play). Each place then offers **Switch now** (from Steam Game Mode, Steam closes properly first).
Search Settings for "Game Mode" to find it.

## What opens which Game Mode

Every way into "Game Mode" opens the one you picked:

- turning on / signing in (Settings › Gaming › Start in › Game Mode), including automatic sign-in;
- Mantel's Game Mode buttons (taskbar, power menu, launcher, Quick Access, Super+G, Settings › Power);
- Server mode's Guide countdown, its Play button and its way back;
- Steam's own Game Mode switch (`steamos-session-select gamescope`);
- the end of first-time setup (only with the developer switch for the old setup);
- `nimbus-mode game` from a script.

Steam's **Power › Switch to Desktop** and Hearth Game Mode's **Switch to desktop** both open Mantel, as before. Hearth
Game Mode's power menu keeps **Steam Game Mode** for a one-off visit to Steam's.

## Steam games in Hearth Game Mode

They still play through the real Steam client: Hearth starts Steam hidden in the background (when Steam has been used on
this PC before) and hands it the game, so Steam's overlay, cloud saves and play time all work. Your other stores' games
(Epic, GOG, Battle.net, Xbox, emulators, Beacon, Recomps, games you added) play through Hearth, as they do from Steam's
library with Hearth Sync. Hearth Sync and Decky only run in Steam Game Mode.

## When Hearth Game Mode can't start

The device never gets stuck on a black screen, and your choice is never changed by Hearth:

| What happened | Where this start goes |
| --- | --- |
| gamescope or Hearth's app is missing | Steam Game Mode (Mantel if Steam can't run either) |
| Hearth's app crashes 3 times within 2 minutes | Steam Game Mode, then Mantel |
| Hearth Game Mode closes within 20 seconds without a switch you asked for | Steam Game Mode, then Mantel |
| Hearth Game Mode closes by itself after a while | Hearth Game Mode again (twice in 10 minutes pauses it, below) |
| Two failed runs within 10 minutes (crashes, quick closes, or starts that hung or took the PC down) | **Paused**: every Game Mode start opens Steam Game Mode until you try Hearth Game Mode again (or a run goes well) |
| B or Guide held as the session starts, `hearth.mode=desktop`, Recovery's Desktop Mode this time | Desktop Mode this time |
| Steam Game Mode then can't start either | Mantel, with both reasons |

Every fallback **says what happened**:

- in Steam Game Mode: a toast from Hearth Sync ("Hearth Game Mode didn't start" with the reason) and the reason in Hearth
  Sync's Game Mode section with **Try Hearth Game Mode again**;
- in Mantel: the "didn't start" card that opens by itself, with **Try Hearth Game Mode again**;
- in Settings › Gaming › Game Mode, with the same Try again;
- in Send logs: the session log (`hearth game mode: <why>; Steam Game Mode this time (your choice stays Hearth Game Mode)`)
  and a "Game Mode choice" section.

A run counts as failed until it has run 2 minutes or you switched on purpose, so a start that hangs or takes the device
down counts too, even across restarts. Try again (or `nimbus-mode hearth`) always runs and ends the pause.

## For developers

- **The choice**: one line of `~/.config/nimbus/session.conf`, `HEARTH_GAME_MODE_CHOICE=steam|hearth`. No line means Steam
  Game Mode. Only exactly `hearth` is Hearth Game Mode; anything else is Steam Game Mode, and a line Hearth doesn't
  understand is left alone. The old developer switch `HEARTH_LEGACY_GAME_MODE=1` still means Hearth Game Mode when the
  line isn't there (read, never written).
- **One writer**: `hosts/linux/desktop/shell/hearth-game-mode` (`get`, `choice`, `set steam|hearth`, `notice
  steam|desktop|hearth`), through `hearth_settings_io` (only that line changes; atomic, `.bak`, under the lock; nothing
  is written when it's already that). Mantel's `GameModeChoice.qml`, Hearth's `gameMode.get/set/switchNow`
  (`hosts/hearth-core/src/game_mode_choice.rs`) and Hearth Sync's `game_mode_*` (`hosts/decky-hearth-sync/main.py`)
  all go through it. docs/SETTINGS-PERSISTENCE.md.
- **The session** (`hosts/linux/session/nimbus-session`, the `game mode choice` block): `game_mode_choice` reads the line
  every time Game Mode starts (the same rules as the helper; a test compares them), `resolve_mode` sends "game" to
  `hgm_route` when Hearth Game Mode is picked, `run_hearth_mode` runs it (Hearth's app in gamescope through the
  watchdog, with Hearth Input) and decides the fallback. `nimbus-mode hearth` asks for Hearth Game Mode by name.
- **Files**: `$XDG_RUNTIME_DIR/hearth-game-mode-failed` (why it didn't start, this sign-in), `hearth-game-mode-crashed`
  (left by `nimbus-watchdog` when it gives up), `hearth-game-mode.pid` (running), `hearth-gm-notice-<place>` (shown);
  `~/.local/state/nimbus/hearth-gm-fails` and `hearth-gm-off` (the pause, with the reason inside).
- **Unchanged**: the live USB's installer (Hearth's app) and extra multi-seat players keep the old path. Nothing about
  booting, the boot menu or the login screen changes; every switch is inside the session.
- **Tests**: `hosts/linux/test/test_hearth_game_mode_choice.py` (the helper, the shell's reading, the whole session with
  stand-ins: default, picked, every fallback, the pause, the choice never written by the session),
  `test_hearth_steam_mode.py`, `hosts/decky-hearth-sync/tests/test_main.py` (`GameMode`), `tools/ui-check.js` (Hearth
  Game Mode's cards), `desktop-test.yml` (Mantel's cards with the controller, pictures in `shots/hearth-game-mode`).

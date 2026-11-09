# Settings persistence

**The owner's red-flag rule: settings must always be remembered and must never change by themselves.**

This page is the audit of where Hearth OS keeps settings (1.0, `bughunt` at `b3b97529`), the ways a setting could be
lost or flip without the person touching it, and what 1.1 does about each. Rules for new code are at the end.

## The safe helpers (use these, nothing else, for a settings file)

| Language | Helper | Used for |
|---|---|---|
| Rust | `hosts/hearth-core/src/settings_file.rs` (`read`, `read_json`, `update_json`, `merge_json`, `update_text`, `write_json`, `write_text`) | Hearth app, core service, Game Mode bridge |
| Python | `hosts/linux/desktop/shell/hearth_settings_io.py` (`load_json`, `update_json`, `merge_json`, `write_json`, `write_text`, `update_text`, `conf_set_text`, `notice`) | Mantel helpers, hearth-theme, hearth-wallpaper |
| QML | `hosts/linux/desktop/shell/SettingsWriter.qml` (`merge({key: value})`, `repair()`), which runs `hearth_settings_io.py merge` | Settings.qml, Buttons.qml, SetKeyboard.qml |
| Shell | `python3 hearth_settings_io.py merge/conf-set/conf-strip/write/notice`, or the session's `strip_mode_args` (flock, mktemp, sync, .bak, mv) | nimbus-session |
| Node (old Linux host) | `hosts/linux/src/settings-file.js` (`writeSettings`, `readSettings`, `mergeSettings`) | system-admin.js, display.js, profiles.js |
| Decky plugins | `_safe_write` / `_read_settings` inside each `main.py` (a plugin ships on its own) | decky-hearth, decky-hearth-sync |

All of them follow the same rules and file names, so different programs can share one file:

1. **Atomic**: a temporary file with its own name in the same folder, `fsync`, `rename` over the old file, `fsync` of
   the folder. A crash or power cut leaves the old file or the new one, never half of one. (`library.json` skips the
   fsync: it's written with Game Mode's bridge held and an fsync could wait seconds behind a big download; the rename
   still replaces it whole on ext4 and btrfs, and the `.bak` covers the rest.)
2. **`FILE.bak`**: the last good version is copied there before each write that changes the file.
3. **Never defaults over a damaged file**: a file that can't be read (empty, cut short, not a JSON object) is put back
   from `FILE.bak`. With no good `.bak` the damaged file is kept as `FILE.broken-<time>` before anything is written.
   Either way the person is told once (a notification where one can be shown, and
   `~/.local/state/nimbus/settings-events.log`, which Send logs includes).
4. **Merge on write**: the latest file is read under an exclusive lock (`.FILE.lock` beside it, `flock`), only the
   keys that changed are changed, then it's written. Two programs changing different keys at once both keep theirs.
   A copy held in memory (Hearth's `Store`) starts from the file again when someone else changed it.
5. **Defaults per key**: defaults fill in keys that are missing; a value that's there is never replaced.
6. **One source of truth per setting**, and a self-healing switch tells the person and can be undone.

## Where settings are kept

| File | Format | Writers | Readers | Was at risk of | 1.1 |
|---|---|---|---|---|---|
| `~/.local/share/nimbus/desktop.json` (Mantel: look, mode, accent, wallpaper, night light, clock, power timers, lock on wake…) | JSON | Settings.qml (FileView.setText of every key), hearth-wallpaper, hearth-core admin.rs (lockOnWake, wallpaper), Node system-admin.js | Settings.qml, hearth-theme, admin.rs, Game Mode | **Parse error → defaults kept in memory, then the next change saved all the defaults over the file** (look, accent, wallpaper… reset). Whole-object rewrite from QML put back values hearth-wallpaper / Game Mode had just written (last writer wins). A reload of the file triggered a save per key with a half-updated copy. admin.rs, hearth-wallpaper and system-admin.js turned a damaged file into `{}` + one key. hearth-theme fell back to Steam/dark on a damaged file | Per-key merges through SettingsWriter; values read from disk are never written back; damaged → `repair()` from .bak; all other writers merge under the lock; hearth-theme reads through the helper |
| `~/.config/nimbus/session.conf` (start mode, gamescope options = Game Mode display mode, Steam Game Mode default, developer switches) | shell `KEY=VALUE` | admin.rs (start mode), display.rs (gamescope args, revert), steam_mode.rs (HEARTH_STEAM_DEFAULT), decky-hearth (same), nimbus-session (`sed -i` dropping the display mode), hearth-recovery (`open(p,'w')`), Node system-admin.js / display.js (`writeFileSync`) | nimbus-session (sourced), admin.rs, display.rs, steam_mode.rs, SetSignin.qml, logs | Read-change-write races between writers with no lock; `writeFileSync`/`open('w')` truncate then write; **the session removed the display mode you chose without telling you** | All Rust, Node and Decky writers use lock + atomic + .bak, changing only their line; the session's `strip_mode_args` does the same, keeps your choice in `display-reverted.json`, and Game Mode tells you with **Try again** / OK. hearth-recovery is left (a booting file; see Remaining) |
| `~/.local/share/nimbus/library.json` (favorites, hidden, play time, sources, per-game options, setupDone, steamBackground, autoUpdateCheck, uiScale, profile…) | JSON | hearth-core `Store` (the app keeps a copy in memory for its lifetime), the core service (`Store::open` again), `nimbus --play-now`, hearth-tasks (steamBackground), Hearth Link (profile inbox), profiles (a new profile's copy) | hearth-core, nimbus-session (`setupDone` grep), hearth-theme (profile accent), Theme.qml | **"A damaged file starts fresh"**: defaults saved over favorites, hidden games, play time; `setupDone` gone → **first-time setup comes back**. Hearth's in-memory copy overwrote changes other processes made (hearth-tasks even refused while Hearth ran: "it would undo a change") | `Store::open` restores from .bak; `Store::update` merges on write under the lock and starts from the file when it changed; per-key defaults; hearth-tasks merges under the same lock |
| `~/.config/nimbus/osk.json` (on-screen keyboard) | JSON | SetKeyboard.qml (setText of its copy), hearth-core osk.rs (`osk.set` replaced the whole file with what Game Mode's page had) | Keyboard.qml, SetKeyboard.qml, Game Mode | Game Mode's stale copy put back Mantel's changes; keys not in osk.rs's list dropped; two copies (Game Mode's localStorage `hearth.osk` and the file) | Mantel merges one key; `osk.set` merges; Game Mode sends only the changed key; the file is the one copy (moved from localStorage once) |
| `~/.config/nimbus/buttons.json` (power button press/hold, louder than 100%) | JSON | Buttons.qml (setText), bridge.rs `set_button_prefs` | both | Lost updates between Desktop and Game Mode; damaged → defaults written | Both merge only what changed |
| `~/.config/nimbus/display.sh` (Desktop Mode screen modes, per output name) | shell | SetDisplay.qml (shared `.tmp`), display.rs `save_wlr` | labwc autostart | Two writers racing; tied to connector names (DP-1, HDMI-A-1) that change with ports or GPUs | Both under `.display.sh.lock`, synced, `.bak`. Connector names: Remaining |
| `~/.config/nimbus/display-pending.json` (mode on trial) | JSON | display.rs (`fs::write`) | display.rs, nimbus-session | Truncated file → trial lost | Atomic |
| `~/.config/nimbus/display-reverted.json` (new) | JSON | nimbus-session | display.rs, Game Mode | — | Your mode, kept after the session had to drop it |
| `~/.config/nimbus/profiles.json` (Hearth profiles) | JSON | hearth-core profiles.rs, Node profiles.js | both | Damaged → one default profile saved over everyone's | Restored from .bak; atomic with .bak |
| `~/.config/nimbus/controller.json`, `~/.local/state/nimbus/hhd-controller.json` (Settings › Controls, HHD's original modes) | JSON | hearth-controller | hearth-controller | Damaged → `{map:{}, target:''}` written over it; keys dropped by the whole-object write | Restored from .bak; merge |
| `~/.config/nimbus/accessibility.json` | JSON | hearth-access | hearth-access, Game Mode | Damaged → defaults; whole rewrite | Restored from .bak; only changed keys merged |
| `~/.config/nimbus/keyboard.json`, `pointer.json`, `~/.config/labwc/environment`, `mimeapps.list`, rc.xml (Mantel keyboard, mouse, default apps) | JSON / text | hearth-system (`write_atomic` with a shared temp name) | hearth-system, labwc | Two runs at once mixed one temp file; no fsync | Shared helper (unique temp, fsync, .bak); JSON reads restore from .bak |
| `~/.config/nimbus/desk.json` (taskbar pins) | JSON | hearth-desk | DeskState.qml | Damaged → empty taskbar saved over your pins | Restored from .bak |
| `~/.config/nimbus/files.json` (Hearth Files views) | JSON | hearth-files | hearth-files | Two windows → lost update (no lock) | `update_json` under the lock |
| `~/.config/nimbus/picker.json` (last folder per app) | JSON | hearth-portal | hearth-portal | Shared temp name | Safe helper |
| `~/.config/nimbus/game-mode-ui.json` (new) | JSON | Game Mode (`uiprefs.set`) | Game Mode | — | The file copy of Game Mode's appearance and button swaps |
| Game Mode's own settings in WebKit localStorage (`nimbus.look`: layout, text size, motion, clock, battery, sounds, style; `nimbus.swapAB/XY`; `hearth.osk`…) in `~/.local/share/io.github.dragongamer9.hearth` | WebKit storage | ui/index.html | ui/index.html | WebKit writes localStorage lazily: a power cut loses the last changes; a reset of that folder (or a new app id) loses all of them | Look and button swaps also in `game-mode-ui.json` (the file wins; old values copied once). Others: Remaining |
| Hearth apps' localStorage (Time's tab and view, Camera mode, Fonts size, Paint tool, Photos view, Share tab, Gaming saves folder) in `~/.local/share/io.github.dragongamer9.hearth.<app>` | WebKit storage | each app | each app | The same; and an app id change in an update would lose them | A test fails if any app's identifier changes. Moving them to files: Remaining |
| Hearth Link `link/ui.json`, `inbox.json`, `now-playing.json` | JSON | hearth-core link.rs (own lock + `write_atomic`) | link | Atomic already, has its own lock | Unchanged |
| Hearth Link `link/prefs.json` (new: the sync switches' shared keys) | JSON | the Hearth Link Decky plugin (`hearth_settings_io.update_json`), hearth-link sync.rs (`settings_file::update_json`) | both | — | One lock, atomic, .bak; incoming keys are merged one by one into each settings file with `merge_json`, only while that switch is on |
| Hearth Input layouts `~/.config/nimbus/input/*.json`, `global.json` | JSON | input_map.rs | input_map.rs, hearth-input | Damaged → the game's layout silently Off | Read back from .bak first; written with .bak |
| Decky plugins' `settings.json`, `applied.json`, `shortcuts.json`, `status.json` | JSON | decky-hearth, decky-hearth-sync | same | **`set_settings` started from the defaults**: a partial update turned other switches back on; damaged → defaults | Starts from what's saved; atomic + .bak; damaged → .bak |
| `~/.local/state/nimbus/steam-default-off`, `steam-default-fails` (Steam Game Mode paused after two failed starts) | text | nimbus-session | nimbus-session, Mantel SteamFallback, Game Mode, Decky | A self-healing switch | Already tells you (Mantel's banner, Game Mode's note, the reason inside) and is undone by "Try Steam Game Mode again" or any good run. Unchanged |
| `~/.config/nimbus/home-server.json` (new: Settings › Home server: Light mode, shares, snapshot plan, Tailscale switches, Show hidden items; stage 2: people, backups, backups from your devices, synced folders, alerts, power; stage 3: media, ai, web, dropoff, links, guard) | JSON | hearth-home-server (`update_json` / `merge_json`: only the key you changed, compared with what you have now including defaults; a key inside a group such as `alerts.on` or `power.sleepIdle` writes only that key of the group; lists such as `people`, `backups` and `syncs` are changed under the lock); Game Mode's Quick Access (the Hearth Link plugin) only through `hearth-home-server quick-set lightMode` | hearth-home-server, Settings › Home server's pages, the Quick Access tile; hearth-home-server-helper (root) reads only `lightMode` from it, read only | — | Defaults filled in when read, never saved; damaged → .bak / kept aside. The system's config (Samba, firewall, timers, NUT, the receiver's drop-in) is made from it by hearth-home-server-helper and never written back into it; what happened goes to state files (`~/.local/state/nimbus/home-server-applied.json`, `/var/lib/hearth/home-server-backups.json`, `/var/lib/hearth/home-server-alerts.json`, `/run/hearth/home-server-power.json`). **No password, code or key is ever in it**: those go to `smbpasswd`, to root's 0600 files in `/etc/hearth/backup` (and rclone's own config there), or stay in Syncthing's own config; stage 3's share-link secrets, link password hashes, the drop-off secret and the web password's hash are in 0600 state files (`~/.local/state/nimbus/home-server-links.json`, `home-server-web.json`). "Let paused jobs run now" is a flag in your runtime folder (gone after 4 hours or a restart), never a setting. Stage 3 adds only state beside it, never settings: what Light mode stopped (`/run/hearth/home-server-services.json`), the Command Center seen using the AI service (`/var/lib/hearth/home-server-seen.json`), the ransomware guard having tripped (`/var/lib/hearth/home-server-guard.json`: shares read only until you say so, told on screen, undone in one step), Safety's fixes with what was there before (Undo), download counts, cameras found, the last clean up and the smart folders. The services timer and the web server only read home-server.json |
| `~/.config/nimbus/remote-desktop.json` (new: Settings › Remote play › Use this PC from anywhere: on, reach, keepAwake) | JSON | hearth-remote-desktop (`merge_json`: only the key you changed, and only when it differs from what you have, defaults included) | hearth-remote-desktop, SetRemote.qml through it | — | Defaults filled in when read, never saved; damaged → .bak / kept aside. Saved only after turning on worked. **No password or runtime state in it**: Sunshine's login for Hearth is the 0600 `~/.local/share/nimbus/remote-play.json` (shared with hearth-core remote.rs); what Hearth changed in Sunshine's own `sunshine.conf` (merged with `update_text`, .bak) is remembered in `~/.local/state/nimbus/remote-desktop.json` and put back on Off only where you haven't changed it since; who is streaming and traffic samples are in `$XDG_RUNTIME_DIR/hearth-remote-desktop/` |
| `~/.local/state/nimbus/startup-off.json` (services you turned off) | JSON | hearth-tasks | hearth-tasks | Shared temp name | Safe helper |
| `~/.config/nimbus/taskmgr.json` (new: Task Manager's start page, update speed, always on top, the rail folded, column widths, sort, last page, last Performance device) | JSON | TaskManager.qml through SettingsWriter (`merge` of the one key you changed, and only when it differs) | TaskManager.qml (FileView) | — | Opening Task Manager (IPC, the launcher) writes nothing; defaults are filled in memory only; damaged → `repair()` from .bak, never replaced. Efficiency mode isn't a setting: what it changed is runtime state in `$XDG_RUNTIME_DIR` (`hearth-tasks-UID-eco.json`) |
| `~/.local/state/nimbus/run-history.json` (new: the Run box's recent entries; state, not a setting) | JSON | WinTools.qml through SettingsWriter (`merge` of `recent`, only after something typed was opened) | WinTools.qml (FileView, while Run is open) | — | Opening Run, Device Manager or the Super+X menu writes nothing; damaged → `repair()` from .bak, never replaced. Device Manager keeps nothing; Disable is an rfkill block the kernel and systemd-rfkill keep (docs/DEVICE-MANAGER.md) |
| `~/.config/nimbus/ai-share.json` (new: Share this device's AI, docs/AI-SHARE.md: on, hub, lan, pluggedOnly, notWhilePlaying, codeOn, which services, the device order) | JSON | hearth-ai-share (`update_json`: only the key you changed, compared with what you have now including defaults; `share.chat` and the others write only their key of the group) | hearth-ai-share (Settings › Home server › AI devices), its server (read only, never repairs or writes) | — | Defaults filled in when read, never saved; damaged → .bak / kept aside. The share code's hash is a 0600 state file (`~/.local/state/nimbus/ai-share.json`), devices seen before are a cache (`~/.cache/nimbus/ai-share-devices.json`), whether the firewall rule is loaded is `/run/hearth/ai-share-firewall.json`. Playing, on battery or offline only change what the device reports, never a setting |
| HHD (`~/.config/hhd/state.yml`) | YAML | HHD itself (Hearth only sends changes through its API, `hearth-hhd set`) | HHD | Owned by HHD | Hearth never rewrites it |
| `/etc/nimbus/no-autologin`, `/etc/hearth/seats.json`, `/etc/sddm.conf.d/zzz-hearth-seats.conf` | marker / JSON | hearth-admin (root) | session, login screen | `/etc` is merged by ostree (3-way: your change wins over the image's) | Fine: Hearth's own markers are created by you, never shipped in the image |
| Image defaults: `/etc/sddm.conf.d/zz-hearth-theme.conf`, `/etc/xdg/nimbus-xdg-terminals.list`, `/usr/share/nimbus/desktop/theme/palettes.json`, `/usr/share/nimbus/desktop/labwc/*` | text | the image | login screen, Mantel, hearth-theme | `/usr` is replaced every update; anything a person changes must live in `~` or `/etc`, never `/usr` | Nothing writes personal settings under `/usr` (checked); the theme generator writes only to `~/.config/nimbus/theme`, and keeps your own `kdeglobals` (it only rewrites one that still carries its mark) |

## What the logs show

From `Dragongamer9/hearth-logs` (72 logs, 2026-10-20 to 2026-11-02):

- **The display mode self-heal dropped the owner's choice silently**: four logs from 2026-10-31 and 2026-11-01
  (CyberPowerPC, RTX 5070, Peppermint, Hearth 0.6.61: `32T4-3WJB-GJ`, `WMWL-M665-8B`, `P8GP-BFRS-AC`,
  `JRJB-P2AG-Z9`) show `gamescope ended right away (code 134) with the chosen screen mode; trying once more` then
  `gamescope couldn't start with the chosen screen mode (-W 2560 -H 1440 -r 120); back to the screen's own mode`.
  The session then `sed -i`'d session.conf: the 2560×1440 at 120 Hz choice was gone, and nothing on screen said so.
  1.1 keeps it in `display-reverted.json` and Game Mode asks.
- **Unclean shutdowns happen on the owner's devices**: `N7ZH-SCHT-8Q` (2026-11-01): `system.journal corrupted or
  uncleanly shut down`. That's exactly when a truncate-then-write (hearth-recovery, Node host, Decky, sed) leaves an
  empty file, which before 1.1 became the defaults (library.json, desktop.json, profiles.json, desk.json…).
- `desktop.json` and `osk.json` are missing at Mantel's start in 12 logs (fresh installs where nothing was changed
  yet; Settings.qml only writes on a change). Not a loss by itself; with 1.1 a first change writes only that key.

No log shows a parse failure message, because none of the old readers logged one: they silently used the defaults.
1.1 writes every restore or kept-aside file to `settings-events.log`, which Send logs now includes.

## Top findings (fixed in 1.1)

1. **Damaged file → defaults saved over it** in Store (library.json: favorites, play time, setupDone so first-time
   setup came back), Settings.qml (desktop.json), profiles.rs/profiles.js, hearth-desk (taskbar pins),
   hearth-controller, hearth-access, admin.rs, hearth-wallpaper, Decky. Now restored from `.bak`, never overwritten.
2. **Last writer wins with a stale copy**: Settings.qml and SetKeyboard.qml rewrote whole objects; Game Mode's
   `osk.set` sent its whole (stale) settings; Hearth's in-memory library copy overwrote Task Manager's change;
   Buttons.qml vs Game Mode's power-button settings; two Files windows. Now merge-on-write under one shared lock.
3. **Self-healing that hid the change**: the session dropped your display mode. Now remembered, told, retryable.
4. **Defaults over present values**: decky-hearth `set_settings` started from `DEFAULT_SETTINGS`. Now from what's saved.
5. **Non-atomic writes**: Node host (`writeFileSync` on session.conf, desktop.json), Decky session.conf, display-pending,
   shared `.tmp` names in many Python helpers (two writers could rename a mix into place), no fsync anywhere.
6. **Two copies of one setting**: Game Mode's OSK settings in localStorage and osk.json (the file is now the one copy).
   Game Mode's look lived only in WebKit storage (now also in a file, which wins).

## Remaining risks (not changed here)

- **hearth-recovery** (`os/system_files/usr/libexec/nimbus/hearth-recovery`, `os/recovery/src/hearth-recovery`):
  "reset screen" writes session.conf with `open(p, 'w')` (truncate then write). It's part of the recovery/boot
  path, so it's left for the owner to approve; the repo check lists it as an allowed exception.
- **Display settings tied to connector names**: `display.sh` lines are per output (`DP-1`, `HDMI-A-1`); moving the
  cable to another port or GPU makes Desktop Mode forget that screen's mode (nothing is overwritten; it just doesn't apply).
- **WebKit localStorage** still holds per-device conveniences in Game Mode (setup step, store tab, Hearth Link seen
  times) and in the Hearth apps (last tab, tool, sizes). A power cut can lose their last change. Moving each to a
  file needs a small Rust command per app.
- **Mantel and Game Mode have separate clock24 / reduceMotion** (desktop.json vs Game Mode's look). They're separate
  settings per mode today; linking them is the owner's choice.
- **Learned keyboard words** (`osk-words.json`): Keyboard.qml writes its whole copy; osk.rs merges counts. A
  simultaneous Game Mode + Desktop learn could drop a few words (not a setting).
- `hearth-browser` / `hearth-claude` install state and Hearth Link's files use their own atomic writers (no `.bak`).

## Rules for new code

- Settings files are written **only** through the helpers above. `hosts/linux/test/test_settings_persistence.py`
  fails CI on a `sed -i`, a shell `>` redirect, `fs::write`/`File::create`/`paths::write_atomic`, Python
  `open(…, 'w')`, Node `writeFileSync` or a QML `FileView.setText` on a known settings file.
- Write only the keys the person changed. Never write a value you just read back into the file.
- Never put a default into the file unless the person chose it; fill defaults in memory, per missing key.
- A switch Hearth turns off by itself says so on screen and can be turned back on in one step.
- A Hearth app's identifier never changes (its WebKit data lives under it); the same test checks it.
- Personal settings live in `~` (or `/etc` for system-wide choices the person makes), never `/usr`.

# The Server side's parts (Nov 26, 2026)

The owner, Nov 26: "Ok we're gonna expand heavily on the Server side of hearth OS", "Let's do it", then on the boards
"thoes look good" (docs/mockups/server-expansion, checklists docs/mockups/checklists/server-*.md). Standing rules: "All
this has to be updated thru the store" (every part arrives and updates through the Hearth Store, never as a Hearth OS
update) and "Debian system files are unlocked" (anything that needs the system installs itself, with the admin
password once).

## How it's put together

| Piece | Where | Updates through the Store as |
|---|---|---|
| The Home server helper (one window: its sidebar, Server home, every part's page) | `hosts/hearth-ui/apps/home-server` (`hearth-home-server-app`; a page per module in `src/pages`, listed once in `pages/mod.rs`) | its own app part, `hearth-home-server-app` |
| The list of parts (the sidebar and Settings' tiles are both made from it: a new part is one line) | `hosts/hearth-ui/apps/server-parts/src/parts.rs` | with the app and Settings |
| Settings › Home server, the Server section's home (board 11) | `hosts/hearth-ui/apps/settings/src/cats/hs_board.rs` (drawn from `server-parts/src/home.rs`) | Settings' part |
| The parts' engines (Python; they do the work) | `os/system_files/usr/libexec/nimbus/hearth-server-parts/<part>.py` with `common.py` | **inside the Home server helper's system part** (`hearth-home-server-helper`): quick-update.yml packs the folder next to the helper, so a Store update of the helper brings every engine with it. Root-owned once installed, run as you. |

Why the engines ride in the helper's system part: Hearth OS only installs system parts it already names
(`/usr/share/hearth/store/system-parts.json`, in the image), so a brand-new system part would need a Hearth OS update
before the Store could install it. The Home server helper is named already, so its Store update can bring new parts
today. The app finds them next to whichever helper is in use (`hearth-system-part path`), else the built-in folder.

`hearth-home-server-helper server-part NAME ARGS` runs `NAME.py` from the folder next to it (only names of letters,
digits and `-`, only files that are there): the parts' own timers start them this way, so they always use the Store's
copy. Through pkexec it's how the few commands that need the system run (Backups on a device); each engine checks for
itself.

## The rules every part keeps

- **Settings** (red flag 9): one file per part, `~/.config/nimbus/server-<part>.json`, through Hearth's
  `hearth_settings_io` (atomic, `.bak`, merged under the lock); only the key you changed is written; opening a page
  writes nothing; defaults are filled in when read and never saved. What a part did is state:
  `~/.local/state/hearth/server/<part>.json`.
- **Off until you turn it on, never in the way of starting** (red flag 2): every timer and service is your user's own
  (`~/.config/systemd/user`), written only when you turn its switch on and removed when you turn it off, wanted only by
  `timers.target` / `default.target` (never `multi-user.target`, never anything that starts the PC or Game Mode). The
  one system timer (a device's own backup) is written only when you join it to your server, and it's a oneshot
  started by its timer, so it can't hold up a start.
- **Nothing heavy while you play**: the slow work runs at the lowest CPU and disk priority and waits while Game Mode or
  a game runs, or someone streams from this PC (`hearth-playing` / `hearth-streaming` marks, gamescope).
- **Keys**: only in the system's password store (`secret-tool`, `hearth-app home-server key <name>`): R2's access key
  and secret (typed into the Model vault's setup, straight into the store), each device's backup key and code. Read
  only while they're needed, never printed, never logged, never on a command line (restic asks the store itself with
  `--password-command`; R2's requests are signed in the engine with AWS SigV4).

## The parts (feature/server-1)

- **Dashboard** (`dash.py`, boards 03 and 04): the PC's numbers from `/proc`, `nvidia-smi` (or amdgpu's sysfs), hwmon
  and the mounted drives; every service's health from its systemd unit (system or yours: active, restarts, memory, CPU)
  **and** its own local address answering (Ollama's `/api/ps`, Colibri's `/v1/models`, Jellyfin's
  `/System/Info/Public` and `/Sessions` with your key, ComfyUI's `/queue`, Kokoro, Vaultwarden's `/alive`, Nextcloud's
  `status.php`, Home Assistant with Hearth Home's token, AdGuard Home's `/control/filtering/status`, Sunshine,
  `tailscale status`). Not installed: not counted. Fixes: restart a service, Hearth Shield's moved block list swapped
  and every list fetched again, free GPU memory (models unloaded, ComfyUI's queue stopped), restart everything one at a
  time, clear old caches (only ones unused for a week), Send logs. History is a point a minute **while the dashboard is
  open** (nothing samples in the background); power comes from the helper's own power history. Alerts to your phone:
  a user timer every 10 minutes, only once you turn it on, with quiet hours. Updates by themselves: off unless you
  turn it on; app parts update with no password, the system part waits for yours and says so.
- **Model vault** (`vault.py`, boards 01 and 02): every model in Hearth OS's `ai-models.json` (Colibri's catalog and
  the Uncensored group), Hearth Forge's list (`vault-sources.json` plus Forge's own when it's installed), Listen's
  voices, Kokoro and Ollama's small model, kept as their exact files at a pinned revision with MODEL.json beside them
  (source, revision, licence, each file's size and SHA-256), on a vault drive and/or your own Cloudflare R2 bucket
  (multipart uploads, 64 MB parts, resumable, at the speed you pick, paused while you play, only at night if you
  like). Costs from R2's prices: Standard $0.015/GB-month with 10 GB free, Infrequent Access $0.01/GB-month and
  $0.01/GB to get back, 30-day minimum. Sundays at 3:00 AM it asks Hugging Face's API (and Ollama's registry) about
  every model: removed or private, files changed, licence changed, gated, renamed; alerts to the desktop and your phone
  as you choose. A new model is asked about first, with what it adds each month. Apps load from the vault first:
  Hugging Face's own cache points at the vault's files, so Forge, Listen, Kokoro, Imagine and Colibri load them with
  no internet. Nothing is deleted from the vault by itself; removing one asks twice.
- **Backups for every device** (`backups.py`, board 06): built on the home server's backup receiver (rclone's restic
  server on the Tailscale address only, one private folder and code per device) and restic. Add a device makes its
  code and key (kept in this PC's password store) and a join code for the device; on the device, joining (password
  once) puts the code in a root-only file and a system timer that looks every hour and backs up at its time, plugged
  in or at 50% battery, or when it comes home after a day away. What it backs up comes from the server (a small
  read-only page on the Tailscale address, answered only to that device's own code). Locked: the receiver takes new
  backups but never lets a device change or delete one (`--append-only`, `hearth-home-server-helper receive-lock`);
  only the server, which has the keys, forgets old ones by your rules (nights, then weeks). The server's own services
  at 2:00, a second copy on R2 on Sundays (restic's own copy, still encrypted), a restore test on the 1st. Restore a
  file into Home › Restored from backups (a new folder each time) and to the device over Taildrop; on the device, put
  back in place (the one there kept beside it) or beside it; the whole device (your Home, Hearth's settings and lists,
  /etc/hearth: Hearth OS and the boot chain are never touched). **Restoring a device that won't start** needs the
  Recovery USB's "Restore from my home server", which comes with the next Recovery ISO (the owner's go-ahead).

## Tests

`hosts/linux/test/test_server_parts.py` (home-server.yml): R2's prices; every list in the catalog; looking writes no
setting; a choice writes only its key; timers only when turned on; a pretend Hugging Face (removed, changed, new, a
file that doesn't match its checksum is never kept); a pretend R2 that checks each request's SigV4 signature
(multipart, Infrequent Access, the secret never sent, getting back checked by SHA-256); apps load from the vault
first; the dashboard's real health on a pretend PC; Hearth Shield's fix; quiet hours; a device's nights and versions
from a pretend restic; join codes checked; a device puts back only its own files; a device's root-only setup; when a
backup is due; the helper runs only its own engines. CI's pictures (`hearth-apps-rust-shots.d/home-server.sh`) run
the real engines against `home-server-fixture.py`'s pretend home server.

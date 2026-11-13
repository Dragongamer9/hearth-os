# Home server

Settings › **Home server** turns this PC into a home server for your other devices: Windows PCs, phones, TVs and
Hearth devices, at home and anywhere through Tailscale. It's built for the owner's Nvidia gaming PC, which also runs
Game Mode: **Light mode while you play** (on by default) makes big jobs wait while Game Mode or a game runs, and
sharing keeps working.

It builds on **Hearth Storage** (docs/HEARTH-LINK.md, `hosts/hearth-link/src/storage.rs`): Hearth Storage is the home
server for your *Hearth* devices over Hearth Link (game archive, backups of saves and screenshots, saves that follow
you). Home server adds what other devices need: Samba shares with their own settings, drives and snapshots, and
Tailscale. The two share one root helper family and one Samba: Hearth Storage's shares are in
`/etc/samba/hearth-storage.conf`, Home server's in `/etc/samba/hearth-home-server.conf`, share names never clash, and
neither stops Samba while the other still uses it.

It came in three stages. **Stage 1**: the Home server group with Overview, Shared folders, Drives & undo, Tailscale
and the setup steps. **Stage 2**: Backups & sync, People, Alerts & power, and the Game Mode Quick Access tile.
**Stage 3 (the last)**: Files & links, Services, Gaming, Family & home, AI and Safety (with the ransomware guard).

## What's in Settings

- **Overview** (`SetHomeServer.qml`): Light mode while you play; storage used, shared folders, Tailscale and drive
  health at a glance; the shares and Tailscale in short; **Set up your home server** until it's done; **Show hidden
  items** and how recently the Hearth AI Command Center must have been seen (7, 30, 90 days or a year).
- **Shared folders** (`SetHomeShares.qml`): your shares, each with Shared on/off, Access (read only or read & write),
  Reachable (home network or Tailscale only), Recycle bin (30 days), Who (you; people come in stage 2), Snapshots, and
  Stop sharing (never deletes files). Share a folder: Hearth Files' folder picker, a list of folders in Home and on
  drives, or a typed path. Your **sharing password** (Samba's own, at least 8 characters, straight to `smbpasswd`; Hearth
  only remembers *when* you set it). **How to open them**: Windows, phone or TV, Hearth devices, away through Tailscale.
- **Drives & undo** (`SetHomeDrives.qml`): every drive with its size, kind, temperature, space and SMART health; the
  system drive is marked and is never shared, formatted or picked. **Snapshots** on/off, keep counts (hourly, daily,
  weekly), which shares, **Browse & restore** (browse in Hearth Files, or restore into Home › Restored from snapshots,
  never over your files). **Add a drive**: more storage, a mirror (btrfs RAID1) or a backup target; "Show what it would
  do" (a dry run), then two confirmations that name the drive and its size.
- **Tailscale** (`SetHomeTailscale.qml`): turn it on, sign in with a QR code or a link, this PC's name and address,
  sign out; **Only reachable through Tailscale**, **Start Tailscale when this PC starts**, **exit node** (off); your
  devices with online/offline and ping (select one to send it a file with Taildrop); **Receive files** (Taildrop, into
  Downloads › Taildrop) and **Wake this PC from away**.
- **Set up** (`SetHomeSetup.qml`, opened from Overview): Name, Tailscale, First share, First backup (Set up a backup
  opens Backups & sync; the backups already made are listed; Hearth Storage's backups of your Hearth devices next to
  it), Done. All rows: Up/Down, A, B.
- **Backups & sync** (`SetHomeBackups.qml`, stage 2): your backups, each with what it backs up (any of your shares) →
  where (a backup drive, another Hearth device over Tailscale, or the cloud: Backblaze B2 or S3-compatible), how often
  (every hour, daily or weekly at a time), how many to keep (7, 14 or 30 days, then weekly and monthly), On/off, **Back
  up now**, the last run (files, size, how many kept, the weekly check) or why it didn't finish, **Restore…** (pick a
  backup, then a folder: it's copied into Home › Restored from backups, a new folder every time, never over your
  files), change its password, Remove (the job and its keys go; what it backed up stays). **Add a backup**: where,
  which shares, how often, and a password (restic's key: restoring on another PC needs it). **Devices backing up
  here**: Hearth Storage's backups of your Hearth devices (opens Hearth Link), and **Backups from your other Hearth
  devices** (on/off, the folder they go to, a code per device shown once, take a code away). **Phone photos**: how to
  point an Android or iPhone app (FolderSync, PhotoSync, Syncthing-Fork) at a share. **Folders kept the same on your
  Hearth devices** (Syncthing): this PC's sync ID, your synced folders (up to date or syncing, connected), folders
  offered by your devices (Accept), Sync a folder (a folder, a device on Tailscale and its sync ID).
- **People** (`SetHomePeople.qml`, stage 2): you (the owner), then each person with what they may open; Add a person
  (a name, then their password); per person: each share's access (No access, Read only, Read & write: only on a read &
  write share), **Their own folder** (Home › People › NAME, shared as `\\PC\NAME` with only them and you), a size limit
  (None, 50 GB, 200 GB, 500 GB, 1 TB; btrfs), change their password, Remove (never deletes their files). Shared folders'
  **Who** row shows who's in and opens People.
- **Alerts & power** (`SetHomeAlerts.qml`, stage 2): what's wrong now; **Alerts** (off until you turn it on) with A drive
  starts to fail (SMART, or 65 °C and up), A drive is almost full (80, 85, 90 or 95%), A backup fails or is late, A
  mirror is missing a drive, Also send them to my devices (Taildrop, to the devices you pick), Send a test alert.
  **Power**: Sleep when nobody needs it for (never, 30 minutes, 1, 2 or 4 hours), Stay awake while my devices use it,
  Wake-on-LAN (the same switch as Tailscale's Wake this PC from away), Sleep at night, wake in the morning (sleep at
  23:00–2:00, wake at 2:30–8:00), Spin down idle storage drives after (never, 10, 20, 30 minutes, 1 hour). **UPS**
  (only once one was plugged in, or with Show hidden items): Shut down cleanly on battery, and its charge.
- **Overview** also has Backups and Alerts tiles and rows for the three new pages.
- **Files & links** (`SetHomeFiles.qml`, stage 3): **Search every share** (names, every word you typed; hidden files never);
  select a result to **share a link** to it: works for 1, 7 or 30 days or until you stop it, a note for whom, an optional
  password; the links with their end date, password or not, downloads so far (and the most allowed), **Copy** and
  **Stop**. **Drop-off folder** (Home › Drop-off by default): friends add files with its link and can't see or delete
  anything. **Smart folders** ("Look through your shares", at low priority): recent screenshots (30 days), big files
  (4 GB and up), duplicates, not changed in a year; **Review** lists them (Show opens Hearth Files) and **Move to the
  recycle bin** moves them to their share's `.hearth-recycle` (30 days, never deleted now) with **Put back**. Changed
  this week; space per share.
- **Services** (`SetHomeServices.qml`, stage 3): **Media server** (Jellyfin: Install, on/off, Reachable: home network
  and Tailscale or Tailscale only, Pause in Game Mode, Open links, Remove), **Photo library** (in Jellyfin, with phone
  photos from Backups & sync), **Files on the web** (on/off, your web password, reachable: Tailscale only or also your
  home network, Open), **AI pictures (ComfyUI)** and **AI voices (Kokoro)** (always shown; below), the **AI service**
  (hidden until the Command Center is seen; opens AI), and Remote play, Folder sync and Game archive with their own pages.
- **Gaming** (`SetHomeGaming.qml`, stage 3): **Share a Games folder** (Home › Games, read only for your devices:
  installers, mods, emulator files), Steam's own LAN transfers (changed in Steam; Hearth never signs in to Steam),
  the game archive and saves (Hearth Link), Remote play over Tailscale, friends through Tailscale's node sharing, and
  **Light mode while you play** with what waits and what keeps running.
- **Family & home** (`SetHomeFamily.qml`, stage 3): **Make a Family folder** (Home › Family, read & write for you and
  everyone in People), People, calendar and notes as files there, **Look for cameras** (ONVIF on your home network;
  found cameras open their own page; camera items show only once one is found, or with Show hidden items).
- **AI** (`SetHomeAI.qml`, stage 3): **Install the AI service** (Ollama), on/off, who can reach it (Tailscale and this PC
  only), the **models** (qwen3:30b-a3b recommended, qwen3-coder:30b-a3b, gpt-oss:20b, qwen3:14b: each with a line on what
  it's for and its size; Download only when you pick one; Remove), **Ask the server** (a question to a model you
  have), **For the Command Center** (when it was seen and where, the address to paste with Copy, voices), **While you
  play** (Unload models when you play, on by default; Stop the AI service while I play, off; Unload models now),
  Remove Ollama, and **Pictures and voices** (AI pictures and AI voices in short, Open ComfyUI, the voices address with
  Copy; always shown). Everything tied to the Command Center is hidden until it's seen (below).
- **Safety** (`SetHomeSafety.qml`, stage 3): when the guard has tripped, **Your shares are read only** and **Make them
  writable again**; **Worth a look** (findings with **Fix** or **Open**, and **Undo** for each fix done); at a glance
  (Tailscale only or not, the firewall rule, the last backup, SMB3 and no guests, snapshots); the **Ransomware guard**
  (off until you turn it on; 500, 1,000 or 5,000 renames or deletes in 5 minutes); **Who can reach what**; **Open
  ports** (every network, Tailscale only or this PC only); updates keep the previous version startable.
- **Overview** also has Services, Safety and (once the Command Center is seen) AI service tiles, and rows for the six
  new pages.

## How it works

- **Your settings**: `~/.config/nimbus/home-server.json`, through `hearth_settings_io` (docs/SETTINGS-PERSISTENCE.md):
  written only when you change something and only that key (a value equal to what you have, default included, isn't
  written), atomic with `.bak`, merged under the lock; defaults are filled in when read. Shares are a list changed under
  the lock (`update_json`). What the system did last is state, not a setting: `~/.local/state/nimbus/home-server-applied.json`.
- **The Mantel helper** `hosts/linux/desktop/shell/hearth-home-server`: status, settings, shares, snapshots, restore,
  Tailscale (the `tailscale` CLI, as the operator), Taildrop (`hearth-taildrop.service`, a user service), and the
  hide-until-seen rule. It checks paths and names with the root helper's own rules, then calls the root helper.
- **The root helper** `os/system_files/usr/libexec/nimbus/hearth-home-server-helper` (pkexec; `49-hearth-storage.rules`
  lets the person at the device in the wheel group run it without a prompt). It imports Hearth Storage's helper for
  the Samba service, SELinux, the password and Wake-on-LAN. It checks everything again:
  - a share is a folder **inside** your Home (never Home itself), or on a drive under `/run/media/YOU`, `/media/YOU`,
    `/mnt` or `/var/mnt` that **isn't the system drive** (the disks under `/sysroot`, `/`, `/boot`, `/var`, `/var/home`…);
    never `/`, `/etc`, `/usr`, `/boot`, `/sysroot`, `/ostree` and other system folders, nothing hidden on the way, links
    judged by where they point; only letters, digits, spaces and `. - _ + @ , ( ) '` (no `%`, which Samba would expand,
    no quotes, `$`, backquotes, `;`, `|`, `&`, `<`, `>`, backslashes or control characters);
  - share names are 1–40 letters, digits, spaces, `-` or `_`, not Samba's own (`global`, `homes`, `printers`…);
  - nothing goes through a shell; Samba's config is checked with `testparm` and the last working one is put back if it
    fails.
- **Samba**: `[global]` SMB3 or newer, `map to guest = Never`, `restrict anonymous = 2`, no usershare guests, NTLMv2
  only, no NetBIOS (port 445 only). Each share: `valid users` and `force user` you, `guest ok = no`, hidden files vetoed,
  and with Recycle bin on (read & write shares) Samba's `recycle` module into the share's `.hearth-recycle` (files older
  than 30 days are emptied by the hourly job). The image's `/etc/samba/smb.conf` starts with nothing shared.
  Samba runs only while at least one share is on (enabled then, disabled again when Hearth started it and Hearth
  Storage doesn't use it).
- **Only reachable through Tailscale**: a Tailscale-only share gets Samba's own `hosts allow = 100.64.0.0/10
  fd7a:115c:a1e0::/48` / `hosts deny = 0.0.0.0/0 ::/0`. When the switch is on for everything (or every share is
  Tailscale only and Hearth Storage's Windows sharing, which is for this network, is off), the firewall closes Samba's
  ports to everything but Tailscale: `/etc/hearth/home-server.nft` is Hearth's own nftables table (`inet
  hearth_home_server`, input hook, priority -10: `lo` and `tailscale0` accepted, then TCP 139/445 and UDP 137/138
  dropped). A drop in any nftables table is final, whatever other firewall runs. It's loaded right away and before every
  Samba start (`smbd.service.d/hearth-home-server.conf`, `ExecStartPre=-`: a firewall that can't load never stops
  Samba, and Samba's own address check still holds). Turning it off deletes the table. Note: while it's on it also
  closes Hearth Storage's Windows sharing to this network.
- **Snapshots**: btrfs only. A share's snapshots are of the btrfs subvolume it's in (inode 256, up to the mount; your
  Home is the `home` subvolume on a Hearth install), never of the system's own root. Read-only snapshots go to
  `<subvolume>/.hearth-snapshots/<hourly|daily|weekly>-YYYYMMDD-HHMM`. `hearth-home-server-snapshots.timer` (hourly,
  5 minutes' random delay, no catch-up at start) runs `snapshot run`: a kind is due when its newest is at least about one
  interval old (95%), and the oldest beyond each keep count are deleted (only names Hearth made). With Light mode on it
  does nothing while gamescope runs or a `hearth-playing` file exists; the next hour catches up. Lowest CPU and disk
  priority. The timer is on only while snapshots (or a recycle bin) are on. Other drives: "Snapshots aren't available
  for this drive".
- **Adding a drive**: `format-plan` gives the drive's name and size, a token of its identity (path, serial, WWN, size)
  and the commands. `format` needs that token (a different drive at the same `/dev` path is refused), `confirm1: true`
  and `confirm2` equal to the drive's "name · size". The system drive and drives in use are refused. More storage /
  backup target: GPT, one btrfs partition, mounted at `/var/mnt/<name>` from `/etc/fstab` with `nofail` and
  `x-systemd.device-timeout=10s` (a missing or dead drive never stops the PC starting), owned by you. Mirror: `btrfs
  device add` to a Hearth-made btrfs drive under `/var/mnt` (never the system drive), then a RAID1 balance in a
  transient unit at idle priority. `dryRun: true` returns the plan and runs nothing.
- **Tailscale**: from Tailscale's own repository in the Debian image (its key checked against fingerprint
  `2596A99EAAB33821893C0A79458CA832957F5868` before it's trusted). `tailscaled` is disabled in the image; "Turn on"
  starts it for now and makes you its operator (`tailscale set --operator=YOU`) so Settings can sign in and change it;
  "Start when this PC starts" only enables or disables it (never stops it). Sign-in: `tailscale login` with its output
  thrown away, the link read from `tailscale status --json` (AuthURL) and drawn as a QR code (qrencode) in
  `$XDG_RUNTIME_DIR`; links and keys are never logged, and no auth keys are used. Exit node: IP forwarding
  (`/etc/sysctl.d/99-hearth-exit-node.conf`, removed when off) and `--advertise-exit-node`. Wake from away: Hearth
  Storage's `wol` for each wired card (magic packet, kept after restarts); another device at home sends the wake-up.

## Stage 2: how it works

- **Settings** stay in `home-server.json` through `hearth_settings_io` like stage 1: `people`, `backups`, `receive`,
  `syncs`, `alerts` and `power` (defaults filled in when read: everything off or empty). A setting inside a group
  (`alertsOn`, `sleepIdle`…) writes only that key of its group. **No password, code or key is ever in it**: people's
  passwords go to `smbpasswd` through stdin; a backup's password, a device's backup code and cloud keys go through
  stdin to the root helper, which writes them 0600 for root only (`/etc/hearth/backup/<id>.pass`, `<id>.rest`, and
  rclone's own config `/etc/hearth/backup/rclone.conf`, folder 0700); Syncthing's API key stays in Syncthing's config.
  What the system did is state: `/var/lib/hearth/home-server-backups.json` (each backup's last run, no secrets),
  `/var/lib/hearth/home-server-alerts.json`, `/run/hearth/home-server-power.json`.
- **People** are Samba-only accounts: `useradd --system --no-create-home --home-dir /nonexistent --shell
  /usr/sbin/nologin` with Hearth's comment (`Hearth home server person (sharing only)`) and the `hearth-share` group;
  a system account has a locked system password and isn't on the login screen. Names: 2–24 small letters, digits, `-`
  or `_`, starting with a letter; never `root`, `admin`, `nobody`, `guest`, you or any account Hearth didn't make.
  Removing one is `smbpasswd -x` and `userdel` (never `-r`), and only for an account with Hearth's comment and no login
  shell. In Samba each share's `valid users` is you plus the people you let in, read-only people go in `read list`
  (a read-only share stays read only for everyone), and `force user` stays you (files stay yours). A person's own
  folder is its own section `[name]` (you and them); it's a btrfs subvolume when the drive is btrfs, so a quota
  (`btrfs qgroup limit`, simple quotas when the kernel has them) can apply to it; other drives say a limit isn't possible.
- **Backups** are restic (Debian's 0.18). Where: a folder on a drive that isn't the system drive (under `/run/media/YOU`,
  `/media/YOU`, `/mnt` or `/var/mnt`; never inside a folder it backs up), another Hearth device (`rest:http://<its
  Tailscale name>:8095/<code name>/`, its code's password in the environment, never an argument), or the cloud
  (`rclone:hearth-<id>:<bucket/folder>`, rclone's B2 or S3 backend; or your own rclone remote in that file). restic
  encrypts everything here before it leaves. `hearth-home-server-backup.timer` (hourly, off until a backup is on) runs
  `backup run`: each backup that's due (its time came since its last good run; a failed one tries again after an
  hour), then `restic forget --prune --keep-daily/weekly/monthly`, and `restic check` once a week. With Light mode on
  it waits while Game Mode or a game runs (a Back up now that waits runs at the next hour); **Let paused jobs run
  now** in Quick Access lets it through for up to 4 hours (a flag in your runtime folder, not a setting). Back up now
  and Restore run in their own transient units (`systemd-run`) at idle priority; a backup holds off sleep
  (`systemd-inhibit --what=sleep`). Restore is `restic restore <backup> --include <folder> --target <a new, empty
  folder> --overwrite never`, then everything in it is made yours (links not followed) and a notification says so.
  The root helper reads Light mode live from your settings file (read only, no links followed, 1 MB at most), so the
  Quick Access switch works without a password.
- **Backups from your other Hearth devices**: `rclone serve restic --private-repos` on this PC's Tailscale address only
  (port 8095), as you, into the folder you picked (each device its own folder), with `--htpasswd` holding only the
  codes' hashes. `hearth-home-server-receive.timer` starts it a minute after the PC starts; the service has no
  `[Install]`, restarts every minute if Tailscale isn't up yet, and its address, folder and user are a drop-in the root
  helper writes. A code (`name-xxxx:XXXX-XXXX-XXXX-XXXX`) is shown once.
- **Folder sync** is Syncthing (your own user service, off until you sync a folder; Debian's 1.29), set to talk only
  over Tailscale: no global discovery, relays, NAT traversal, local discovery, usage or crash reports; it listens on
  this PC's Tailscale address. Each synced folder keeps changed or deleted files 30 days (trashcan versioning). Both
  devices add each other by sync ID; a folder one offers shows on the other with Accept (kept in Home › Synced).
  Syncthing stops again when no folder is synced. Folder sync is light and keeps running while you play.
- **Phone photos**: any SMB app on the phone (FolderSync or PhotoSync on Android, PhotoSync on iPhone), pointed at a
  read & write share with your or a person's name and password; at home or through Tailscale. Or Syncthing-Fork.
- **Alerts**: `hearth-home-server-alerts.timer` (hourly, off until you turn alerts on) runs `alerts run`: SMART
  (smartctl) and temperature of every drive, how full the drives with a share or a backup are, each backup's last run
  (failed, late: a day for hourly, 3 days for daily, 10 for weekly) and its check, and Hearth's mirrors (`btrfs
  filesystem show` missing a device, `btrfs device stats -c` errors). What's new is told once (again after a day) as a
  notification on your desktop (`notify-send` as you, through your own session bus; nothing when there's none), and
  with "Also send them to my devices" as a small text file through Taildrop to the devices you picked. No other
  service is used.
- **Power**: `hearth-home-server-power.timer` (every 5 minutes, off until a power choice is on) runs `power run`. Spin
  down: `hdparm -S` on spinning data drives only (never the system drive, an SSD or a removable drive), once per start
  or when the time changes. Stay awake: while a share is open (`smbstatus`) or a backup, snapshot or restore runs, a
  transient `systemd-inhibit --what=sleep` (10 minutes, renewed), which Settings › Power's idle sleep respects too.
  Sleep when nobody needs it: never while Game Mode or a game runs, a share is open or a job runs, nor while someone is
  signed in and not idle (the login screen doesn't count; when the desktop can't say, it counts as used and Settings ›
  Power's own timer decides); then `systemctl suspend --check-inhibitors=yes`. The schedule sleeps once a night at
  your time (unless busy) and wakes with `hearth-home-server-wake.timer` (`WakeSystem=true`, the time from a drop-in).
  Wake-on-LAN is stage 1's switch (Hearth Storage's `wol`).
- **UPS**: shown once one was seen (USB makers APC, CyberPower, Eaton, Tripp Lite and others; seeing one is state, never
  a setting). On: NUT standalone (`/etc/nut`: `usbhid-ups`, upsd on 127.0.0.1 and ::1 only, a random monitor password,
  shutdown when the battery runs low), nut-driver-enumerator, nut-server and nut-monitor enabled. A NUT set up by hand
  is left alone; off disables what Hearth enabled.
- **Game Mode's Quick Access tile** is in the Hearth Link Decky plugin (docs/HEARTH-LINK-PLUGIN.md), which Steam Game
  Mode already loads: a **Home server** section at the top, only on a PC that's a home server (set up, sharing or
  backing up). It shows this PC's name, shares, space and people, the backups in a line, what's wrong, **Light mode
  while you play** (saved with `hearth-home-server quick-set lightMode`, no password), what Light mode holds back now,
  and **Let paused jobs run now** / **Pause them again while I play**. The plugin asks only `quick`, `quick-set
  lightMode on|off` and `resume on|off` of `/usr/share/nimbus/desktop/shell/hearth-home-server`, as you, each with a
  10-second timeout; when the helper isn't there, fails or hangs the tile isn't shown, and its own React error boundary
  keeps any problem in it from touching the rest of the panel. Nothing here is part of starting Game Mode.

## Stage 3: how it works

- **Settings** stay in `home-server.json` through `hearth_settings_io`: `media` {on, reach, pause}, `ai` {on, unload,
  pause}, `web` {on, reach}, `dropoff` {on, path}, `links` (each link's path, label, for whom, end, password yes/no, most
  downloads) and `guard` {on, limit}; defaults filled in when read (all off; `web.reach` Tailscale; `ai.unload` on). A
  setting inside a group writes only that key. **No secret is in it**: a link's secret part, its password's hash, the
  drop-off's secret and your web password's hash (PBKDF2-SHA256, 200,000 rounds) are in 0600 state files
  (`~/.local/state/nimbus/home-server-links.json`, `home-server-web.json`). Download counts, Safety's fixes and how to
  undo them (`home-server-fixes.json`), cameras found, the last clean up (`home-server-recycled.json`) and the smart
  folders (`~/.cache/nimbus/home-server-files.json`) are state.
- **Apps that aren't in Debian** (Ollama, Jellyfin) are never in the image and never installed at build or boot:
  **Install** runs `hearth-home-server-helper app-install NAME` (pkexec), which starts a transient unit (idle I/O, nice
  10) running `app install-run NAME` (refused straight through pkexec). It finds the newest release, downloads it over
  https from the maker's hosts only (a redirect anywhere else stops it), and **checks its SHA-256**: Ollama's from its
  GitHub release (GitHub's own digest of the file and the release's `sha256sum.txt`; both must agree when both are
  there), Jellyfin's from the `.sha256sum` next to its newest stable build on repo.jellyfin.org. No checksum or a
  mismatch: not installed. The archive is listed first and refused with absolute paths, `..`, devices or links
  pointing outside it, then unpacked (`--no-same-owner`) beside the old copy and swapped in, into
  `/var/lib/hearth/apps/NAME` (root's). x86-64 only; free space checked first; progress and errors in
  `/var/lib/hearth/home-server-apps.json`. **Remove** deletes the app only: Jellyfin's data (`/var/lib/hearth/jellyfin`)
  and Ollama's models stay.
  - Why these: Jellyfin is the best-known open media server, with apps for TVs and phones, and its own builds bundle
    what they need (Debian's ffmpeg does the transcoding). Ollama is what the Command Center's GPU-server teammate
    talks to (its OpenAI-compatible `/v1`) and runs models on the RTX 5070 out of the box. Neither is in Debian
    trixie, so each comes from its maker on request instead of a third-party repository baked into the image. Immich
    (the photo app in the mockup) needs a database, Redis and a machine-learning service in containers: not now;
    Jellyfin's Photos libraries and stage 2's phone photos cover it. Files on the web, links and the drop-off are
    Hearth's own small server (Python's standard library): nothing to download.
- **Units**: `hearth-ollama.service` (DynamicUser, models in `/var/lib/hearth-ollama`, the graphics card through the
  video and render groups, ProtectSystem=strict, ProtectHome) and `hearth-jellyfin.service` (as you, from the drop-in;
  ProtectSystem=strict, your home read only, its data writable). Neither has `[Install]`; each starts only with its app
  in place **and** Hearth's drop-in (`ConditionPathExists`), from its timer (`hearth-ollama.timer`,
  `hearth-jellyfin.timer`: a minute after the PC starts, enabled only while it's on) or from Settings.
- **Reach** (stage 1's rules): the **AI service is only for Tailscale and this PC**. Its port (11434) is in Hearth's
  nftables table whenever it's on (`lo` and `tailscale0` let in, everything else dropped); its drop-in loads that rule
  first (`ExecStartPre=+nft -f`, without `-`: no rule, no start) and then listens on all addresses; if the rule can't
  load (no nft, a failed load) it listens on 127.0.0.1 only and Settings says so. The media server on "Tailscale only"
  (or with Only reachable through Tailscale on) gets the same rule for 8096/tcp and 7359/udp and isn't started when the
  rule doesn't load; on "home network" it listens normally (with its own accounts). The table now holds Samba's ports,
  the services' or both; with none it's deleted. Files on the web, links and the drop-off are your own user service
  (`hearth-home-server-web.service`, port 8097, on while one of them is on): on this PC and its Tailscale address only by
  default (it doesn't listen anywhere else), or on every address when you pick Home network.
- **Light mode for services**: `hearth-home-server-services.timer` (every minute, on only while the AI service is on,
  the media server pauses in Game Mode, or the guard is on) runs `services run`. While Game Mode or a game runs (Light
  mode on and not let through in Quick Access) it unloads every loaded model (`/api/ps`, then `keep_alive: 0`), stops
  the AI service if you chose that and the media server if you chose Pause in Game Mode, and starts what it stopped
  again once you stop playing (`/run/hearth/home-server-services.json`). It also loads the Tailscale-only rule again if
  something removed it. It never changes a setting.
- **Models** are pulled through the AI service's own API (`/api/pull`) as you, only the four listed; Ollama checks
  every part's SHA-256 as it downloads. Progress is in your runtime folder.
- **The address for the Command Center**: `http://<this PC's Tailscale name>:11434/v1` (Settings › Accounts › Your own
  GPU server in the Command Center: OpenAI-compatible chat, no key, a model such as `qwen3:30b-a3b`). On this PC:
  `http://localhost:11434/v1`. **Voices**: AI voices (Kokoro-FastAPI, below) gives the Command Center its speech server.
- **Share links and the drop-off** (`hearth-home-server-web`, Python's threading HTTP server): `/l/<secret>/` serves the
  linked file or folder (folders listed) while the link is on, not past its end, not over its most downloads and its
  share still on; with a password it asks (Basic). `/d/<secret>` is an upload page; each file is POSTed raw with its
  name (only the base name, safe characters, a new name when one's taken, 8 GB at most), never listed. **Files on the
  web** (`/`, `/s/<share id>/…`) asks for your user name and web password and lists only shares that are on. Every path
  is resolved and must stay inside its share; hidden files and folders are never listed or served; links are judged by
  where they point. Wrong passwords slow down; secrets in paths are never logged.
- **Smart folders and search** walk the shares that are on (hidden folders skipped, links not followed). Duplicates:
  files of 1 MB and up with the same size and the same first and last 64 KB. Clean up only moves files into their
  share's recycle bin (stage 1's 30 days), recorded for Put back.
- **Folder shares** (Games, Family) are ordinary stage 1 shares made in one step; the Family folder is read & write and
  every person gets write access to it.
- **Cameras**: "Look for cameras" sends one ONVIF WS-Discovery probe on your home network and reads the answers' names
  and addresses (only http(s) addresses kept). A camera found marks `camera` seen; it turns nothing on.
- **Safety** reads your settings, Samba's config (`/etc/samba/smb.conf` and Hearth's), what listens (`ss -tlnu`), the
  last backup and the firewall's last result. Fixes are setting changes you ask for (Everything Tailscale only,
  Snapshots on for every share that can take them, the ransomware guard on, the media server Tailscale only); each is
  remembered with what was there before, and **Undo** puts exactly that back.
- **The ransomware guard** (off until you turn it on): Samba's `full_audit` logs renames and deletes on read & write
  shares to the journal (`smbd_audit`). Each minute the services timer counts the last 5 minutes; at your limit (1,000
  by default) every share in Hearth's Samba config turns read only (checked with testparm, Samba reloaded), the trip is
  kept in `/var/lib/hearth/home-server-guard.json` (state, never a setting) so apply keeps them read only, and you get a
  notification. **Make them writable again** (`guard-reset`) clears it and applies your settings again. It watches
  changes made through Samba (your other PCs), which is how ransomware on a Windows PC would reach your shares.
- **Quick Access** shows the AI service in a line (ready, models unloaded while you play, stopped while you play) and
  "AI models" and "media server" in what Light mode holds back.

## AI devices (Share this device's AI)

Settings › Home server › **AI devices** (`SetHomeAIDevices.qml`, linked from AI): **Share this device's AI** (off),
which services to share, Share only when plugged in, Not while I'm playing, an optional share code; and on the home
server **Let the Command Center find my devices here** (off), the **Command Center address**
`http://<this PC's Tailscale name>:8189` with Copy, and every Hearth device's AI in the order the Command Center tries
them (Nvidia PC first by default; Move up / Move down). It's its own program (`hearth-ai-share`), user service
(`hearth-ai-share.service`) and firewall rule (`inet hearth_ai_share`, port 8189, Tailscale only), all off until you
turn them on, with its own settings file (`ai-share.json`). The protocol and the rest: docs/AI-SHARE.md.

## Voice Studio (Hearth's own voice studio)

**Services › Voice Studio** (and the **AI** page) is the main voices entry now: Hearth's own voice studio on this PC's
graphics card, for your phone, laptop and the Hearth AI Command Center. Kokoro (AI voices, below) keeps working as it was
for anyone who uses it, and Voice Studio lists Kokoro's voices too while it's on. Written for Hearth in Python
(`hosts/linux/desktop/shell/hearth-voice-studio`, the page `voice-studio-web.html`); ideas from VoiceStudio
(debpalash/voicestudio), whose code is AGPL-3.0 and was only read, never copied.

What it does (the page has a tab for each, phone, laptop and controller friendly):

- **Speak**: text to speech in any voice and 40+ listed languages (OmniVoice speaks 600+), speed, download as MP3, WAV,
  FLAC or Opus. Long text is spoken in whole sentences and joined.
- **Clone**: a voice from 5 to 15 seconds of one person talking: upload a file, or record (in the page on this PC; on a
  phone "Record" opens the voice recorder, because browsers allow the microphone only over HTTPS or on the PC itself).
  A consent box must be ticked ("this is my voice, or I have permission"); the time and device are kept with the voice.
  The words in the clip are written down by themselves when you don't type them.
- **Design**: describe a voice in your own words ("an old man with a deep British voice") or tap chips (who, age, pitch,
  whisper, accent); Hearth turns the words into OmniVoice's voice-design attributes. Hear it, then keep it: one sample is
  made and kept, so a designed voice stays the same person every time (OmniVoice alone makes a new person on each call).
- **Transcribe**: audio or video to text, SRT and VTT (and JSON), with "Speaker 1 / Speaker 2" labels (SpeechBrain's
  ECAPA voice fingerprints, grouped by Hearth), in the queue.
- **Your voices**: the library shared by every feature and the API: hear, use, make default, rename, delete. Eight
  ready-made voices (Narrator, Storyteller, Bright, Calm, Elder, Clear, Kid, Whisper) are designed voices rendered once
  the first time they're used; OpenAI's voice names (alloy, nova, onyx, ...) map onto them for apps that send those.
- **Audiobooks**: paste text, or a .txt / .epub (the book's reading order; covers and contents pages left out).
  Chapters are found at "Chapter 3", "# Title", "Prologue"... (or about 15,000-character parts). Narrated in the
  background: an MP3 per chapter, every chapter in a zip, and the whole book as an .m4b with chapter marks.
- **Video dubbing**: the sound is taken out (ffmpeg), the voices are lifted out of the music and sounds (Demucs
  htdemucs), what's said is written down with who said it (faster-whisper), translated by the home server's own AI
  service (Ollama: your choice of model, else a Qwen3 / gpt-oss / Llama one it has, asked for JSON, checked line for
  line), then spoken again **in each speaker's own voice** (cloned from their clearest 6 to 14 seconds in the video) or in
  a voice you pick. Each line is made to the length of the original line (OmniVoice's duration), sped up a little (at
  most 1.5x, pitch kept) if it still runs over, and placed at the original time; the new voices go over the kept
  background and are muxed back into the video (video stream copied). You also get subtitles in both languages.
  Without Demucs the original sound stays quietly underneath. Copying voices from a video asks for the same consent.
- **Apps & API** (OpenAI-compatible, `http://<this PC>:8890/v1`, with the API key):
  `POST /v1/audio/speech` (input, voice, response_format mp3|wav|opus|flac|aac|pcm, speed, language, and OpenAI's
  `instructions`, read as a voice description), `POST /v1/audio/transcriptions` and `/v1/audio/translations`
  (multipart file, language, response_format json|text|srt|vtt|verbose_json), `GET /v1/audio/voices`, `GET /v1/models`.
  In the Command Center: Settings › Voice › Add a speech server › **OpenAI-compatible speech API**, the address and the
  key from Settings (Services › Voice Studio › Address / API key, both copy).

**The queue** (audiobooks, dubbing, transcripts) is on disk: `~/.local/share/nimbus/voice-studio/jobs/<id>/job.json`
(written atomically, with a .bak) and its files beside it. One job at a time; Speak and previews take turns with it.
When Voice Studio stops (a game started and "Stop it while I play" is on, the PC restarted, Settings turned it off) the
running job goes back to the queue and carries on where it stopped next time: audiobooks from the chapter they reached,
dubs from the step and line they reached. Cancel, Try again and Delete are on each job. When a dub is done its big
in-between files go (the video you sent, the results and subtitles stay).

**Settings** (`~/.config/nimbus/voice-studio.json` through `hearth_settings_io`, only the key you change, defaults never
written): `language`, `voice` (the default voice), `translateModel`, `speakerLabels`. On/off and "stop while I play" are
the home server's `studio` {on, pause} in `home-server.json` (both through `hearth-home-server set studioOn|studioPause`;
default off, pause on because it uses the graphics card). Deleting your default voice never changes your settings: speech
uses the Narrator and the page says so. Install progress is state (`home-ai.json`, key `voicestudio`), never a setting.

**Who can open it**: the page and the API listen only on 127.0.0.1 and this PC's Tailscale address (port 8890, in
Hearth's Tailscale-only firewall rule while it's on, like the other AI services), and refuse any other peer. The page
asks for a PIN: Voice Studio's own (Settings › Services › Voice Studio › PIN, typed on Hearth's PIN pad and sent to the
helper on stdin, `hearth-home-server studio-pin`), or, with none of its own, **Server mode's PIN**. Only the hash is kept
(`~/.local/state/nimbus/voice-studio-web.json`, 0600, PBKDF2-SHA256); a new PIN signs every device out; five wrong PINs
wait five minutes. Sign-ins are an HttpOnly, SameSite=Strict cookie for 30 days; every write must come from the page
itself (Hearth's X-Hearth header, then Sec-Fetch-Site or Origin, as Server mode). Referrer-Policy is `same-origin`
(never `no-referrer`, which broke Server mode's PIN in 1.6). Apps send `Authorization: Bearer <key>`; the key is made the
first time, kept in the same 0600 file, compared in constant time, and **Make a new key** replaces it.

**Install** (only when you press Install; never in the image): `hearth-home-server studio-install` runs in its own
transient unit (like ComfyUI) and, as you, makes `~/.local/share/nimbus/voice-studio/venv` (Debian's `python3 -m venv`),
then pip installs (pip checks every file against the hash its index lists, over https): PyTorch and torchaudio from
PyTorch's own CUDA 12.8 index; `omnivoice` (k2-fsa), `faster-whisper==1.1.1` and `soundfile` from PyPI; `demucs==4.0.1`
without its old torchaudio pin (`--no-deps`) plus its own small needs; `speechbrain==1.0.3`. A part that fails (Demucs,
SpeechBrain) is noted on the page and the rest still works. Then OmniVoice's model and Whisper large-v3 turbo are
downloaded into `voice-studio/models` so the first use doesn't wait (they download on first use if that fails). About
14 GB; it refuses with less free space. Models are let go after ten idle minutes so the graphics card is free again.
**Remove** takes the venv, the models and the ready-made samples; your voices and finished work stay unless you confirm
twice (`studio-remove voices sure really-sure`).

**Licences** (checked November 2026; Settings shows the short version before Install):

| Part | Code | Model | Notes |
| --- | --- | --- | --- |
| OmniVoice (k2-fsa) | Apache-2.0 | CC BY-NC (personal, non-commercial use); its audio tokenizer is under Boson's Higgs Audio licence (Llama 3 terms, "Built with Llama") | downloaded by you from Hugging Face, never bundled |
| faster-whisper | MIT | Whisper large-v3 turbo: MIT | |
| Demucs | MIT | htdemucs: MIT | |
| SpeechBrain | Apache-2.0 | spkrec-ecapa-voxceleb: Apache-2.0 | |
| Kokoro (AI voices) | Apache-2.0 | Apache-2.0 | unchanged |

Nothing AGPL is used, and nothing non-commercial is in the Hearth image or the repository. pyannote's diarization model
was left out because it's gated (it needs a Hugging Face sign-in).

**The service**: `hearth-voice-studio.service` (user unit, no install section; `ConditionPathExists` on the venv, so
without the engines it doesn't start at all) and `hearth-voice-studio.timer` (a minute after sign-in, enabled only while
it's on; disabled globally in the image). `ExecStart` is `hearth-home-server ai-run voicestudio`, which execs the venv's
Python on `hearth-voice-studio serve` with its models folder and the CUDA libraries pip installed. It is never part of
starting the PC, the session or Game Mode; Light mode (the root helper's services timer) stops it while you play when
"Stop it while I play" is on and starts it again after, exactly like AI pictures.

**Stand-in engines** (`hearth-voice-studio mock-serve PORT`): the same page and API with tones instead of speech, for
tests and screenshots (`hosts/linux/test/test_hearth_voice_studio.py`).

## AI pictures (ComfyUI) and AI voices (Kokoro-FastAPI)

Two more services in **Services** (always shown, not hidden until seen: they're useful without the Command Center) and
on the **AI** page next to the chat models. Both are **your own user services** (`systemd --user`, as you), not root's.

- **Settings** (in `home-server.json`, through `hearth_settings_io`, only the key you change): `pictures` {on, pause}
  and `voices` {on, pause}; defaults: both off, `pictures.pause` on (ComfyUI holds graphics memory), `voices.pause` off
  (Kokoro uses the CPU). **Install and run state is never a setting**: `~/.local/state/nimbus/home-ai.json` (install
  progress and errors, versions, PyTorch and the graphics card it saw, what was taken over), where they listen in
  `$XDG_RUNTIME_DIR/hearth-home-server/home-ai-run.json`, and what Light mode stopped in root's
  `/run/hearth/home-server-services.json`.
- **AI pictures, how it's installed** (only when you press Install, never at build or boot): `hearth-home-server
  pictures-install` starts a transient unit of yours (`systemd-run --user`, nice 10, idle I/O) that
  1. `git clone --depth 1 --branch v0.43.2` of github.com/comfyanonymous/ComfyUI into `~/ComfyUI.hearth-new`, checks
     that the tag is commit `423cbd2…` (pinned in the helper; anything else is deleted and nothing is installed), then
     moves it to `~/ComfyUI`;
  2. makes a venv (`~/ComfyUI/venv`, Debian's `python3 -m venv`);
  3. `pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu128` (CUDA 12.8 builds:
     the RTX 50 series needs them), skipped when PyTorch is already there;
  4. `pip install -r requirements.txt` (from PyPI);
  5. checks PyTorch sees the graphics card and notes it.
  No model is ever downloaded: the page says where models go (`~/ComfyUI/models/checkpoints`, opened in Hearth Files
  with **Models folder**). **Update** fetches the pinned tag, checks its commit again, refuses when ComfyUI's own files
  were changed, never moves an install that's already past the release back, re-runs the requirements and restarts it.
  Hearth moves the pin with its updates. It needs git and python3-venv (now in the Debian image; Settings says so if
  they're missing) and about 15 GB free.
- **AI video (Wan 2.2)** (Hearth OS 1.7; in AI pictures' settings once ComfyUI is installed: **Add video**, or **Add
  only picture to video**; only when you press it): `hearth-home-server video-install [i2v-only]` starts the same kind
  of transient unit, which
  1. clones the ComfyUI-GGUF node (github.com/city96/ComfyUI-GGUF) at the commit Hearth pins into
     `~/ComfyUI/custom_nodes/ComfyUI-GGUF` (an existing one is used as it is) and pip-installs its requirements;
  2. picks the size from the graphics card's memory (`nvidia-smi`, else ComfyUI's own report): 12 GB or more gets Wan
     2.2 14B **Q4_K_M** (about 9.7 GB a file), 8 to 11 GB **Q3_K_M** (about 7.2 GB a file); under 8 GB it says it won't
     fit and downloads nothing;
  3. downloads from Hugging Face only these: QuantStack's `Wan2.2-I2V-A14B-GGUF` and (without `i2v-only`)
     `Wan2.2-T2V-A14B-GGUF`, HighNoise and LowNoise of that size, into `models/unet`; and Comfy-Org's
     `Wan_2.1_ComfyUI_repackaged` text encoder (`umt5_xxl_fp8_e4m3fn_scaled.safetensors`, `models/text_encoders`) and
     VAE (`wan_2.1_vae.safetensors`, `models/vae`). About 45 GB (25 GB for picture to video only); it checks the free
     space first.
  Each file's **size and SHA-256 come from Hugging Face's own listing** at download time (the LFS pointer), and the file
  is checked against them before it's put in place; a wrong one is thrown away. An interrupted download carries on from
  its `.part` file. Files already there (a hand install) are kept and not downloaded again. Nothing starts by itself: a
  ComfyUI that's running is restarted once, to load the node. In ComfyUI: Workflow › Browse Templates › Video › Wan 2.2
  14B, with the **Unet Loader (GGUF)** for the two models. Other LoRAs or models can be added by hand into
  `~/ComfyUI/models` (`loras`, `unet`…). **Remove video** takes away only what Hearth downloaded (and the node if Hearth
  got it); your own files stay. Progress and what was placed are state (`home-ai.json`, `comfyvideo`), never a setting.
  Tests: `hosts/linux/test/test_hearth_home_ai_video.py`.
- **What's verified, honestly**: ComfyUI's code is checked against the **commit Hearth pins** (git's own content
  hashes). PyTorch's wheels and the requirements are checked by pip against the **SHA-256 their index lists**
  (download.pytorch.org and pypi.org both publish one per file, over https): that catches a broken or swapped download
  on the way, but Hearth doesn't pin those hashes itself, so it trusts those two sites as pip always does.
  Kokoro's image is pulled and run **only by the digest Hearth pins** (Podman checks the manifest and every layer
  against it); CI (`home-server.yml`, kokoro-pin) checks that digest is still on ghcr.io and says when its release tag
  moved. Models you add yourself aren't checked by Hearth.
- **Your own ComfyUI is adopted, not installed twice**: an existing `~/ComfyUI` (a real folder with ComfyUI's files, not
  a link; anything else there is left alone) is used as it is: Install only adds what's missing (the venv, PyTorch,
  the requirements). If your own `comfyui.service` (from your install script) is there and on, Hearth's switch is
  refused until you press **Take over**: it runs `systemctl --user disable --now comfyui.service` (the file stays) and
  turns Hearth's on; **Give it back** turns Hearth's off and enables and starts yours again if it was. Remove is
  refused while yours still uses the folder.
- **AI voices, how it's installed**: `hearth-home-server voices-install` starts a transient unit that runs `podman pull
  ghcr.io/remsky/kokoro-fastapi-cpu@sha256:…` in your rootless Podman. It runs on the CPU, so no graphics card plumbing
  in containers is needed. Your own `kokoro` container (`podman run --name kokoro …`) is never deleted: while it runs,
  Hearth's switch is refused until **Take over** (`podman stop kokoro`); **Give it back** starts it again. Remove
  (`podman rmi` of Hearth's digest) leaves the image when your container still uses it.
- **Running**: `hearth-comfyui.service` and `hearth-kokoro.service` in `/usr/lib/systemd/user`, no `[Install]`, each
  started only by Settings or its timer (`hearth-comfyui.timer`, `hearth-kokoro.timer`: a minute after you sign in,
  enabled for your account only while it's on). Both run `hearth-home-server ai-run`, which becomes ComfyUI (`main.py
  --listen 127.0.0.1,<this PC's Tailscale IPv4> --port 8188`) or `podman run --rm --pull=never --cap-drop=all
  --security-opt no-new-privileges -p 127.0.0.1:8880:8880 -p <Tailscale IPv4>:8880:8880 <image@digest>`. Without
  Tailscale they listen on this PC only (the page says so). ComfyUI's own condition needs `~/ComfyUI/venv/bin/python`.
- **Reach**: only this PC and Tailscale, always. Besides listening only there, their ports (8188, 8880) are in Hearth's
  nftables table while they're on (`lo` and `tailscale0` let in, everything else dropped), through the same apply as
  the AI service; a firewall that can't load doesn't stop them (they still listen only on this PC and Tailscale).
  ComfyUI has no password, so it never gets a home-network choice.
- **Light mode while you play**: the services timer (root, every minute, on while one of them pauses) stops your
  `hearth-comfyui` / `hearth-kokoro` when Game Mode or a game runs (Light mode on, not let through in Quick Access),
  as you (`runuser -u YOU -- systemctl --user stop --no-block …`, through your own user manager; nothing when it isn't
  running), and starts it again once you stop playing if it's still on. Only these two units can be named. It never
  changes a setting. Note: Game Mode itself counts as playing, so with "Stop it while I play" on, ComfyUI is stopped
  the whole time Game Mode is up (Quick Access › Let paused jobs run now lets it run for up to 4 hours).
- **Status**: ComfyUI's `/system_stats` (the graphics card it uses) and Kokoro's `/health` and `/v1/audio/voices`, on
  127.0.0.1. **Addresses**: ComfyUI `http://<this PC's Tailscale name>:8188` (Open); Kokoro
  `http://<this PC's Tailscale name>:8880` for the Command Center's Settings › Voice › Speech servers › Kokoro-FastAPI
  (Copy). The Command Center using AI voices from one of your devices marks it seen, like the AI service.
- **Remove**: ComfyUI's code, venv and custom nodes go; `models`, `output` (your pictures), `input` and `user` (your
  workflows) stay. **Remove ComfyUI and its models** asks twice on the page and the helper only takes
  `pictures-remove models sure really-sure`; a `models` link to another drive is unlinked, never followed. Removing
  turns its switch off (you asked for it). Nothing outside `~/ComfyUI` is touched.
- **Overview and Quick Access**: the Services tile counts Pictures and Voices; Quick Access says "AI pictures: stopped
  while you play · AI voices: ready" and lists them in what Light mode holds back. Gaming lists them under what waits.

## Your shares on your other Hearth devices

Your other Hearth devices see this server's shares that are on in Hearth Files' sidebar under **Your devices**, by
themselves (its status over Hearth Link carries their names, Home / Whole drive, read only or read & write, and how to
reach them: `<name>.local` at home, the Tailscale name away; never a path). One click mounts one (gvfs); the first
time asks for your user name and sharing password, which **Remember** keeps in that device's keyring. Hearth Files'
"How to open from Windows or a phone" shows the same `\\host\share` and `smb://host/share` addresses as this
page's How to open them. Details: docs/HEARTH-LINK.md, "Your devices in Hearth Files".

## Sharing a whole drive, your Home or the system drive (Hearth 1.6)

Shared folders (and Drives & undo, for each data drive) can share more than a folder. Each starts **read only**, on the
**home network and Tailscale** (the share's reach stays changeable; "Only reachable through Tailscale" still wins), never
for guests and never for the internet, with a badge in the list:

- **Whole drive**: a data drive's mount point (under /run/media/YOU, /media/YOU, /mnt or /var/mnt, never one of the
  system's disks). Read only or read & write.
- **Home**: your whole Home folder, after one confirmation ("Everything in your Home is shared, including settings
  folders; hidden files stay blocked"). Read & write needs a second confirmation. Only you (no people).
- **System drive** (`/`): **read only, never read & write** (writing system files over the network can stop the PC from
  starting), after two confirmations that name the risk. Only you; no recycle bin or snapshots; never walked for Files &
  links' smart folders.

What keeps secrets out: Samba works as you (`force user`), so root-only files can't be read at all; every share vetoes
hidden files and folders (`veto files = /.*/`, `hide dot files`: .ssh, .config, .local with the keyrings); Home and the
system drive also `hide unreadable` with `wide links = no`; the system drive also vetoes `shadow`, `gshadow`, `ssh`,
`root`, `proc`, `sys`, `dev`, `run`, `hearth` (/var/lib/hearth's state and /etc/hearth) and `lost+found` by name. The
confirmations are kept with the share (`whole`, `confirm`) and the root helper checks them again (`share_path_problem`,
`check_shares`), so an old or hand-edited settings file can't share Home or `/` by accident, or `/` read & write ever.
People's folders, backups and the rest keep the stricter folder rules (`path_problem`). No shell, no symlink escapes (a
link is judged by where it points), no shell characters; Samba's testparm check still puts the last working config back.

## Uncensored (18+) models and any model by name (Hearth 1.6)

Settings › Home server › AI chat has a group **Uncensored (18+)** behind **Show 18+ models** (`showAdult`, off by default,
display only: saved only when you turn it on, so a family PC never lists them). They're ordinary models in Ollama,
downloaded only when you press Download (the same `ollama pull` path, every part SHA-256 checked); the Hearth AI Command
Center's own built-in limits still apply to anything asked through it. Each name was checked on ollama.com / Hugging Face
(November 2026):

| Model | Name | Size | What it's for |
|---|---|---|---|
| Qwen3 30B-A3B abliterated | `huihui_ai/qwen3-abliterated:30b-a3b` | 19 GB | the same model you use now, without the refusals (card + RAM) |
| Qwen3 14B abliterated | `huihui_ai/qwen3-abliterated:14b` | 9.0 GB | fully on the 12 GB card; can be the small model while you play |
| Dolphin 3 | `dolphin3:8b` | 4.9 GB | an uncensored all-rounder (8B) |
| Rocinante 12B | `hf.co/TheDrummer/Rocinante-12B-v1.1-GGUF:Q4_K_M` | 7.5 GB | a Mistral Nemo 12B tune for characters, roleplay, creative writing |

**Download any model by name**: a field on the same page takes an Ollama library name (`qwen3:8b`, `user/model:tag`) or a
Hugging Face GGUF (`hf.co/<user>/<repo>[:<quant>]`). Only letters, digits and `. : _ / -` (never a URL, a path or an
option; `hf.co/` needs both a user and a repo), checked in Settings and again in `hearth-home-server` (`model_name_ok`).

**Small model while you play** (Server mode's "switch to the small model"): qwen3:14b, or the abliterated 14B when it's
downloaded and you pick it (`aiSmallModel`; the choice shows only with the 18+ models shown). If the one you picked isn't
downloaded, qwen3:14b is used; if neither is, nothing is loaded. Nothing is ever downloaded or deleted for it.

## Server mode (Hearth 1.6)

Settings › Home server › **Server mode** (docs/SERVER-MODE.md) turns this PC's screen into a dashboard for the home server:
a third mode next to Game Mode and Desktop Mode, off until you turn it on. It doesn't copy anything here: what pauses when
you play is **Light mode** (this page's `lightMode`, `picturesPause`, `voicesPause`, `mediaPause`, `aiUnload`, `aiPause`
and the new `aiSmall`), the services are the ones above, and the services timer does the pausing and resuming (Server
mode's Play and return only ask it to run right away). New in Light mode for it:

- `aiSmall` (AI chat while you play: switch to the small model). Not chosen yet: on while Server mode is on, off otherwise
  (a default, never written). The services timer then unloads the big model and keeps qwen3:14b loaded on the processor
  (`num_gpu 0`) while you play, and unloads it after; only when it's downloaded, never deleting or downloading anything.
- ComfyUI stopped for your game finishes the picture it's making (up to 150 s) and its queue is kept and put back after.
- The root helper's `svc stop|start|restart ollama|jellyfin` (the dashboard's pause / restart) and `gpu-limit WATTS|reset`
  (Server mode's idle power, Nvidia), and Server mode's web page (port 8460) in the Tailscale-only rule while it's on.

## Never stopping the PC from starting

Every new service is **disabled until the owner turns it on**: smbd, nmbd, samba-ad-dc, tailscaled, smartd,
nftables, nut-server, nut-monitor, nut-driver-enumerator, nut.target, syncthing-resume, and the user services
hearth-taildrop and syncthing are disabled in the image (the build fails if one is left on, and the image check looks
again); the timers `hearth-home-server-snapshots`, `-backup`, `-alerts`, `-power`, `-wake`, `-receive` and (stage 3)
`-services`, `hearth-ollama` and `hearth-jellyfin` aren't enabled anywhere, and each is turned on only while something
you chose needs it. Stage 3's `hearth-ollama.service` and `hearth-jellyfin.service` have no `[Install]`, need their
app and Hearth's drop-in to start at all, and their apps are never in the image (the build fails if Ollama, Jellyfin or
`/var/lib/hearth/apps` is there); the user service `hearth-home-server-web` is disabled in the image too. AI pictures
and AI voices are user services with no `[Install]` whose timers are disabled globally in the image; the build fails if
either is enabled, if ComfyUI is in `/etc/skel` or Kokoro's image is in the image's Podman storage, and the image check
looks again (and that no PyTorch is installed system-wide). Their services have no `[Install]`
(only their timers start them) and none waits for the network at start. restic, rclone, Syncthing, hdparm and NUT are
nice-to-have packages: a missing one is said in Settings. The snapshot service has no
`[Install]`, a timeout and idle priority; nothing has `Requires=` or is wanted by the session, Game Mode or Steam Game
Mode. Nothing here touches the bootloader, the boot menu, the sessions or nimbus-session; data drives are `nofail`;
the previous image stays startable as always.

## Hidden until seen

Items tied to the Hearth AI Command Center show only when it was seen on one of your devices within `ccDays` (30 by
default); camera and UPS items only once one was found. `~/.local/state/nimbus/home-server-seen.json` holds when each
was last seen (`hearth-home-server seen commandCenter|camera|ups [device]`; Hearth Link and the hardware checks fill it
in stage 2). **Show hidden items** shows them anyway. It's display only: hiding never turns anything off, and seeing a
device never changes a setting. Stage 2 uses it for the UPS (Alerts & power: a UPS plugged in by USB is noted by `status`, at most once an hour).

Stage 3 uses it for the **Command Center** (the AI page, the AI service in Services and its Overview tile and row) and
**cameras** (Family & home). How the Command Center gets marked seen:
- **on this PC**: `status` sees its settings folder (`~/.config/Hearth AI Command Center`, Electron's own) changed
  within your window, and notes it (at most once an hour) with this PC's name;
- **from your other devices**: the services timer notes when one of your Tailscale devices has a connection open to
  the AI service (the Command Center's GPU-server teammate), with that device's Tailscale name, in root's
  `/var/lib/hearth/home-server-seen.json`, which `status` reads with yours (newest wins);
- **by hand or from another tool**: `hearth-home-server seen commandCenter DEVICE`.
An AI service that's installed or on is shown either way, so it can always be turned off.
AI pictures and AI voices are **not** hidden: they show in Services and in the AI page's Pictures and voices section
whether or not the Command Center was seen (their Command Center address also sits in the hidden "For the Command
Center" section, which follows the rule).

## Tests

`hosts/linux/test/test_hearth_home_server.py` (run by `.github/workflows/home-server.yml` and with every Python test in
hearth-link.yml): share config generation and every refusal (bad names, `/`, `/etc`, the system drive, Home itself,
hidden folders, links out, shell characters), Samba's config put back when testparm fails, the Tailscale-only rules and
when they apply, services off by default (the image's build, the units), snapshot due times, retention and Light mode,
the recycle bin, the double-confirmed format as a dry run (and the system drive, a drive in use and a swapped drive
refused), settings written only when changed (and never defaults over a damaged file), and the hide-until-seen rule.
`test_settings_persistence.py` guards `home-server.json` like the other settings files. desktop-test.yml takes
screenshots of the pages.

`hosts/linux/test/test_hearth_home_server_2.py` (stage 2): people's names and every refusal, passwords only through
stdin, removal never deleting files or someone else's account, per-share access and own folders in Samba's config;
backup targets never on the system drive, in Home or inside what they back up, secrets only in root's 0600 files (and
your own rclone remotes left alone), when a backup is due, a run (password never on a command line, forget with the
keep counts, a clean error), Light mode and Let paused jobs run now, restore only into a new empty folder with
`--overwrite never`; backup codes shown once with only their hash kept and the receiver on Tailscale only; alerts
found and told once a day; power never sleeping while playing, sharing or backing up, sleeping when nobody needs it
with inhibitors respected, the schedule and wake timer; the UPS leaving a hand-made NUT alone; timers on only when
needed, everything off by default in the image and the units; nested settings writing only their key; the Quick Access
tile's allow-list. The Hearth Link plugin's own tests (backend, `model.test.mjs`) and pictures of the tile run in
home-server.yml's quick-access job.

`hosts/linux/test/test_hearth_home_ai.py` (AI pictures and AI voices): their ports always behind the Tailscale-only
rule while on, the services timer only when something pauses, ComfyUI and Kokoro stopped as you when a game starts and
started again after (only if still on), nothing stopped with Light mode off, let through or pause off, only Hearth's
two units and real accounts, the Command Center noted from AI voices; settings off by default and written per key
(never defaults over a damaged file), state never in settings; listening only on 127.0.0.1 and the Tailscale address
(never 0.0.0.0), Kokoro only by its digest with `--pull=never`; the install (the release's commit checked or nothing
installed, PyTorch from the cu128 index, the requirements, never a model), adopting an existing `~/ComfyUI` and
leaving anything else or a link alone, git, venv and space checked, the update (your changes never written over,
never moved back, restarted), remove keeping models, pictures and workflows, models only with both confirmations and
links never followed, taking over and giving back your own comfyui.service and kokoro container; the units, the image's
checks and the pages' wiring.

`hosts/linux/test/test_hearth_home_server_3.py` (stage 3): the AI service's port always behind the Tailscale-only rule,
listening on all addresses only with the rule (this PC only without it), a Tailscale-only media server never started
without its rule, apps off stop and delete nothing, an app that isn't installed never started, the services timer on
only when needed; Ollama's and Jellyfin's checksums (both must agree, none means no install), downloads only over https
from the makers, archives refused with absolute paths, `..`, devices or links out, an install that checks then swaps
(a mismatch never unpacked), installs only in their own unit, removing keeps data; models unloaded when a game starts,
the service paused and started again, nothing with Light mode off or let through, the media server only if you chose
it, the Command Center noted from a Tailscale connection, the rule put back; the guard's audit lines, tripping over the
limit (told, read only, kept by apply, undone by reset); settings off by default and written per key, no secrets in
the payload or settings, links only inside shares that are on (secrets 0600), the web server only when needed, smart
folders and a clean up that can be put back, search, fixes and their undo, the four models, a camera's answer, Quick
Access's word for the AI service; the web server over a real socket (a link's folder, counts, used up, expired,
stopped, never outside a share or hidden, its password; Files on the web with your name and password; the drop-off
taking files under safe names and showing nothing); the units, the image's checks and the pages' wiring.

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

It comes in three stages. **Stage 1**: the Home server group with Overview, Shared folders, Drives & undo, Tailscale
and the setup steps. **Stage 2 (this one)**: Backups & sync, People, Alerts & power, and the Game Mode Quick Access
tile. Stage 3: Files & links, Services, Gaming, Family & home, AI and Safety (the ransomware guard).

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

## Never stopping the PC from starting

Every new service is **disabled until the owner turns it on**: smbd, nmbd, samba-ad-dc, tailscaled, smartd,
nftables, nut-server, nut-monitor, nut-driver-enumerator, nut.target, syncthing-resume, and the user services
hearth-taildrop and syncthing are disabled in the image (the build fails if one is left on, and the image check looks
again); the timers `hearth-home-server-snapshots`, `-backup`, `-alerts`, `-power`, `-wake` and `-receive` aren't
enabled anywhere, and each is turned on only while something you chose needs it. Their services have no `[Install]`
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
device never changes a setting. Stage 2 uses it for the UPS (Alerts & power: a UPS plugged in by USB is noted by `status`, at most once an hour); the Command Center and camera items come with stage 3.

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

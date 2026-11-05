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

It comes in three stages. **Stage 1 (this one)**: the Home server group with Overview, Shared folders, Drives & undo,
Tailscale and the setup steps. Stage 2: Files & links, Services, Gaming, Family & home, Backups & sync, People, the
Game Mode Quick Access tile. Stage 3: AI, Safety (the ransomware guard), Alerts & power.

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
- **Set up** (`SetHomeSetup.qml`, opened from Overview): Name, Tailscale, First share, First backup (Hearth Storage's
  backups until stage 2's Backups & sync), Done. All rows: Up/Down, A, B.

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

## Never stopping the PC from starting

Every new service is **disabled until the owner turns it on**: smbd, nmbd, samba-ad-dc, tailscaled, smartd and
nftables are disabled in the image (the build fails if one is left on, and the image check looks again);
`hearth-home-server-snapshots.timer` and `hearth-taildrop.service` aren't enabled anywhere. The snapshot service has no
`[Install]`, a timeout and idle priority; nothing has `Requires=` or is wanted by the session, Game Mode or Steam Game
Mode. Nothing here touches the bootloader, the boot menu, the sessions or nimbus-session; data drives are `nofail`;
the previous image stays startable as always.

## Hidden until seen

Items tied to the Hearth AI Command Center show only when it was seen on one of your devices within `ccDays` (30 by
default); camera and UPS items only once one was found. `~/.local/state/nimbus/home-server-seen.json` holds when each
was last seen (`hearth-home-server seen commandCenter|camera|ups [device]`; Hearth Link and the hardware checks fill it
in stage 2). **Show hidden items** shows them anyway. It's display only: hiding never turns anything off, and seeing a
device never changes a setting. Stage 1 has the rule and its tests; its items come with stages 2 and 3.

## Tests

`hosts/linux/test/test_hearth_home_server.py` (run by `.github/workflows/home-server.yml` and with every Python test in
hearth-link.yml): share config generation and every refusal (bad names, `/`, `/etc`, the system drive, Home itself,
hidden folders, links out, shell characters), Samba's config put back when testparm fails, the Tailscale-only rules and
when they apply, services off by default (the image's build, the units), snapshot due times, retention and Light mode,
the recycle bin, the double-confirmed format as a dry run (and the system drive, a drive in use and a swapped drive
refused), settings written only when changed (and never defaults over a damaged file), and the hide-until-seen rule.
`test_settings_persistence.py` guards `home-server.json` like the other settings files. desktop-test.yml takes
screenshots of the five pages.

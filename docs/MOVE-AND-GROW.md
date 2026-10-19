# Move games and make Hearth bigger (Beta Experimental)

**Beta Experimental: new and not tested on real devices yet. Back up first.**

This is for a PC where Hearth OS was installed next to another system (Windows, SteamOS, or another Linux) on
the same drive. Moving off that system happens bit by bit:

1. **Move** a game (or several) to Hearth: it's copied, every file is checked against the original, it's
   registered in Hearth, and only then is the original deleted from the other system.
2. **Make Hearth bigger**: the other system's partition is shrunk by the space that was freed, and Hearth gets
   that space.
3. Repeat. When nothing is left on the old system, *Remove the old system* (`hearth-takeover`) erases it and
   gives Hearth the rest.

Both are in Settings › Import (`pageImport`, `pageSpace` in `ui/index.html`).

| Piece | Where |
|---|---|
| Moving games (copy, check, register, delete) | `os/system_files/usr/libexec/nimbus/hearth-import` (`run` with `"mode": "move"`) |
| Growing Hearth (status, plan, shrink, add the space) | `os/system_files/usr/libexec/nimbus/hearth-space` |
| Root entry points, argument checks | `hearth-admin space-status`, `space-grow`, `space-clear-journal` (and `import-run`) |
| Bridge methods | `hosts/hearth-core/src/space.rs`, `importer.rs`, `bridge.rs` |
| Tests | `hosts/linux/test/test_hearth_import.py` (`Moving`, `Checking`), `hosts/linux/test/test_hearth_space.py` |

## Moving a game

`hearth-admin import-run` with `{"mode": "move", "items": [keys from scan], "root": "<games folder>"}` on stdin.
For each item, in order:

1. **Room check**: Hearth needs the game's size plus 1 GB free, or the item fails and nothing changes.
2. **Copy** with rsync into the same place a copy goes (Steam library, `~/Games/Epic|GOG|Lutris`, the home
   folders), plus Steam's Proton data (`compatdata/<appid>`) if Hearth doesn't have its own yet.
3. **Check** (`verify_tree`): `sync`, then every file and link under the original must exist in the copy with
   the same type, size and bytes. Contents are compared byte for byte; the copy is read back with
   `POSIX_FADV_DONTNEED` so it comes from the drive, not from memory. The file count is then checked from the
   copy's side too. Special files (pipes, devices) fail the check: rsync doesn't copy them.
4. **Register**: Steam: the manifest goes into Hearth's library (marked installed; Steam checks the files on
   its next start). Epic, GOG and Lutris games are registered by the Hearth app from the result, as for copies.
5. **Delete the original**, only after all of the above:
   * The drive is made writable: drives Hearth opened itself are remounted read-write. **Windows' NTFS drive
     is refused** if it was only readable by forcing it, or if `ntfs-3g.probe --readwrite` (or `ntfsfix -n`)
     says it's hibernated or was shut down with Fast Startup. It's mounted read-write without `force`
     (`ntfs3`, then `ntfs-3g`), so the kernel refuses a dirty volume too. A drive another app opened read-only
     is left alone. In all those cases the copy in Hearth stays, and the original is **kept** with a message
     ("start Windows, hold Shift while choosing Shut down, then move it again").
   * Only a game's own folder is deleted: it must be under that drive, not the drive itself, not a system
     folder (`Windows`, `Users`, `Program Files`, `steamapps`, `home`...), and it must not contain a home
     folder, a mounted drive or another game found by the scan (a Lutris game "installed" in `~/Games` is
     kept). Steam's `installdir` can't contain `/`, `\` or be `..`.
   * The folder is first renamed to `.hearth-moved-<name>` (so a half-deleted folder never looks installed),
     then the launcher's records are removed: Steam's `appmanifest_<id>.acf`, Epic's `.item` manifest
     (Windows), Heroic's `installed.json` entry, Lutris' `installed` flag in `pga.db`. Then the folder and, if it
     was copied, the Proton data are deleted. Personal folders (Documents...) are emptied, not removed, and
     keep `desktop.ini`.
6. The result says what moved and how much space each drive got back:
   `{"mode": "move", "done": [...], "moved": [{key, name, kind, freed, drive}], "kept": [{key, name, why}],
   "failed": [...], "skipped": [...], "freed": [{drive, dev, name, bytes}], "stopped": true?, "steam": n,
   "epic": [...], "gog": [...], "lutris": [...]}`.

Progress lines: `stage: {"phase": "copy"|"verify"|"delete", i, n, key, name, kind, done, size, bps, overall, total}`
(moving counts copying as the first half of each item and checking as the second).

**Stop** (`import.cancel`): SIGTERM, plus the file `/run/user/<uid>/hearth-import.cancel` (the app may not be
allowed to signal a root process started by pkexec). The item in progress stops; what it created in Hearth is
removed and its original is untouched. A delete that already started finishes first. Finished items stay, and
the result is still reported (with `"stopped": true`) so the app registers them. Copy mode now also removes the
unfinished item's partial copy.

## Making Hearth bigger

### `hearth-admin space-status` (read-only)

Works without root too (then without the numbers that need to read the drive: `"limited": true`). The app runs
it through pkexec.

```json
{"beta": true, "limited": false, "power": {"plugged": true, "battery": 80},
 "hearth": {"mount": "/sysroot", "uuid": "...", "devices": ["/dev/nvme0n1p6"], "disks": ["/dev/nvme0n1"],
            "size": 212600881152, "used": 171798691840, "free": 40802189312},
 "systems": [{"id": "nvme0n1p3", "dev": "/dev/nvme0n1p3", "disk": "/dev/nvme0n1", "kind": "windows",
              "name": "Windows", "fs": "ntfs", "label": "Windows", "partSize": 214748364800,
              "size": 214748360704, "used": 92341796864, "min": 85899345920, "keep": 113816633344,
              "canGive": 100931731456, "estimate": false,
              "blockers": [{"id": "fast-startup", "text": "..."}], "warnings": [{"id": "power", "text": "..."}]}],
 "free": [{"disk": "/dev/nvme0n1", "start": 845152256, "sectors": 62914560, "bytes": 32212254720,
           "after": "/dev/nvme0n1p6", "afterHearth": true, "canGive": 32212254720, "blockers": []}],
 "parts": [{"dev", "disk", "num", "kind": "efi|msr|recovery|boot|hearth|windows|linux|steamos-system|swap|other", "what", "fs", "label", "size", "start", "sectors"}],
 "problems": [...], "journal": null}
```

Only partitions on the drive(s) Hearth's own file system is on are looked at. Only the main data partition of
a system can give space: NTFS (Windows), ext4 or btrfs (another Linux; on SteamOS only its `home`
partition). **Never touched**: EFI, Microsoft reserved, Windows recovery, `/boot`, SteamOS' system partitions
(`rootfs-A/B`, `var-A/B`, `efi-A/B`, `esp`), swap, LUKS/LVM/XFS.

How much a system can give: `keep = max(min + max(2 GB, 5% of min), used + 20 GB)`, `canGive = size - keep`
in whole GB (GiB). `min` comes from the file system's own tool: `ntfsresize --info` ("You might resize at"),
`resize2fs -P` (with `dumpe2fs -h` for the block size), `btrfs inspect-internal min-dev-size` (mounted
read-only under `/run/hearth-space`). A drive the importer has open is read from its mount instead
(`"estimate": true`, can't be planned; growing lets go of it and reads it again).

Blockers (any one stops growing from that system): BitLocker; Windows hibernated / Fast Startup;
`ntfsresize` reporting bad sectors or a needed chkdsk; ext4 not cleanly unmounted; btrfs spanning several
devices; open in another app; less than 1 GB to give; a missing tool; an MBR drive; on battery below 50%; an
import running; an unfinished earlier grow; and Hearth not being able to confirm it would still start with
its file system on two partitions (see below).

### `hearth-admin space-grow --from DEV|free --gb N [--dry-run]`

hearth-admin checks the arguments before anything runs as root (`--from /dev/<name>` or `free`, `--gb 1..65536`,
each once). hearth-space reads everything again (paths never come from the caller: `DEV` must be one of the
systems status lists, without blockers) and plans exact sectors. `--dry-run` prints `{"dryRun": true, "plan":
{...}}` and changes nothing.

From a system (`--from /dev/nvme0n1p3`):

| Step | What | Undo |
|---|---|---|
| `release` | unmount the importer's read-only mount (anything else mounted: refuse) | - |
| `check` | NTFS: `ntfsresize --info` problems, `ntfsfix -n`, `ntfsresize --no-action --size NEW`. ext4: `e2fsck -fn` must be clean, then `e2fsck -fp` (marks it checked for resize2fs). btrfs: `btrfs check --readonly` | - |
| `shrink-fs` | `ntfsresize --force --size NEW` / `resize2fs DEV NEWK` / `btrfs filesystem resize DEVID:NEW` (mounted rw under `/run/hearth-space`) | grow it back to the old size |
| `shrink-part` | `sfdisk -N` with the same start and a size of the file system + 1 MiB, after checking the table is still as planned | the old size again |
| `new-part` | `sfdisk --append` a Linux partition named `hearth-more` in the freed space (1 MiB aligned), after checking it's free | delete it |
| `device-add` | `wipefs -a`, `btrfs device add -f NEW <rw mount>` | not undone (see below) |
| `verify` | `btrfs filesystem show` lists it (a warning if not, nothing is undone) | - |

From free space (`--from free`): right after one of Hearth's partitions, that partition grows (`sfdisk -N`,
same start) and then `btrfs filesystem resize DEVID:max`; anywhere else, a new partition is added as above.
Free space can be used even when an earlier grow didn't finish (that's how its leftover space is used).

The btrfs commands run on a **read-write mount** of Hearth's file system (`/var/home` or `/var`): `/sysroot`
is read-only on Fedora Atomic and `btrfs filesystem resize` needs write access to the mount it's given.

Progress: `progress: <0-100> <what>`, then one JSON line:
`{"ok": true, "from": "/dev/nvme0n1p3", "method": "device-add"|"grow-in-place", "gave": bytes, "newPart": "/dev/nvme0n1p7", "journal": "/var/lib/hearth/space-grow.json", "warnings": [], "hearth": {"size", "free"}}`.
Errors: `error: <what went wrong>. Everything was put back as it was` / `Nothing was changed` /
`Hearth couldn't put everything back (...)`.

**Journal and rollback.** `/var/lib/hearth/space-grow.json` (the previous one is kept as
`space-grow.prev.json`) gets every step, written atomically (`fsync`, rename) before it starts and after it
finishes, with the numbers needed to undo it. When a step fails, the finished steps are undone newest first:
the new partition is deleted, the partition gets its old size, the file system is grown back. The first undo
that fails stops the rollback (the earlier undos depend on it), the journal says `stuck`, and the message says
so. A journal left `running` or `stuck` (power cut, crash) blocks shrinking until someone looks
(`space-clear-journal`, the "I've checked it" row) after letting the other system check its drive. A
`device-add` that succeeded is never undone automatically (`btrfs device remove` has to move data back; that's
a decision for a person).

Once growing starts it ignores SIGTERM/SIGINT/SIGHUP (closing the app doesn't cut a resize off) and holds
`/run/hearth-space.lock`.

### Booting with Hearth on two partitions

`btrfs device add` keeps the file system's UUID. Hearth's installer writes `root=UUID=<fs uuid>
rootflags=subvol=root` on the kernel command line and `UUID=` lines in `/etc/fstab`
(`hearth-install`: `deploy`, `fstab_text`), so both still find it. The initramfs must wait until all devices
of the file system are there: dracut's btrfs module does that with udev rules (`64-btrfs.rules`: `btrfs
ready`, or its own `80-btrfs.rules`: `btrfs device scan`). Before growing onto a new partition, hearth-space
checks `/proc/cmdline` has `root=UUID=` matching Hearth's file system and that `lsinitrd` of the running
kernel's initramfs (`/usr/lib/modules/$(uname -r)/initramfs.img`) contains those rules or the `btrfs` tool;
otherwise it refuses. Growing Hearth's own partition in place doesn't need this.

`/etc/fstab` uses `x-systemd.device-timeout=0`: if one of Hearth's partitions went missing, start-up would wait
for it forever. Both are on the same drive, so that only happens if someone deletes the `hearth-more`
partition (in Windows' Disk Management, say). **Never delete Hearth's partitions from another system.**

## Bridge

| Method | Args | Answer |
|---|---|---|
| `import.move` | `{items: [keys]}` | the import state (as `import.start` with `mode: "move"`); progress as the `import` event, `progress.phase` = `copy`/`verify`/`delete` |
| `space.status` | `{}` | `space-status`'s JSON plus `busy` (answers from another thread; can take ~30 s) |
| `space.grow` | `{from: "/dev/..."\|"free", gb: N, dryRun?: bool}` | `{started: true}`, then events `space.grow.progress {pct, text}`, `space.grow.done {result}` or `space.grow.error {text}`. With `dryRun`: the plan |
| `space.clearJournal` | `{}` | `{ok: true}` |

## UI flow (Settings › Import)

* Each game has a **Move** button next to it, and the selection has **Copy N items** and **Move N items (Beta
  Experimental)**. Moving asks once (what happens, that the original stays if anything doesn't match or
  Windows wasn't shut down properly, Beta Experimental). Progress shows *Copying / Checking every file /
  Deleting the original*.
* The result lists what moved, what was kept and why, and for each drive that got space back:
  "46 GB freed on Windows. Give it to Hearth?", which opens *Make Hearth bigger* with that system chosen.
* **Make Hearth bigger (Beta Experimental)**: Hearth's space, then each system (and free space) with how much
  it can give or why it can't, a −/+/Max stepper (8 GB steps, 1 GB under 32 GB), the before/after sizes, and
  two confirmations ("Back up anything important first. The other system keeps working but gets smaller.").
  Then a progress card (no Stop: it finishes or puts things back) and the result.

## What needs testing on real hardware

Nothing here has run on a real PC yet. Before calling it more than Beta Experimental:

* **Windows 10/11 next to Hearth**: move a Steam game and an Epic game, with Fast Startup on (must copy and
  keep the original) and off (must delete), then shrink C: by 16 GB; Windows must still boot, `chkdsk` clean,
  and Hearth must boot with two partitions. Also BitLocker-on (must refuse) and a disk with WinRE after C:.
* **Steam Deck with SteamOS + Hearth**: move from `home`, shrink `home` (ext4), check SteamOS boots and its
  A/B partitions were untouched.
* **Another Linux (ext4 and btrfs)**: shrink, boot it, `e2fsck -fn` / `btrfs check` clean.
* **Interrupting**: pull the power during `shrink-fs` on a test machine with nothing to lose, and check what
  the journal and the page say; then use the leftover free space with `--from free`.
* **Rollback for real**: force a failure after `shrink-part` (e.g. a full partition table) and check Windows'
  partition and file system come back to the same size.
* **Boot**: after `device-add`, reboot, check `btrfs filesystem show`, a `bootc upgrade` and a rollback to the
  previous deployment still boot (the initramfs check passed on the running image; a future image must keep
  dracut's btrfs module).
* **4K-sector drives** (`sectorsize` 4096): the plan uses the table's sector size; not tried on hardware.
* **Cancel through pkexec**: the cancel file path is `/run/user/<uid>/hearth-import.cancel`; check the helper
  really stops from the UI's Stop.
* **ntfs3 vs ntfs-3g**: deleting large game folders on NTFS through ntfs3; check Windows sees the space freed
  and chkdsk is clean afterwards.

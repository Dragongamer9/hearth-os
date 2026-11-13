# Settings › Storage

**New: tested in CI only, not on a device yet.**

Settings › Games › Storage is Hearth's version of SteamOS' Storage page: every drive games can be on and what fills
it, every installed game by size, moving a game to another drive with one press, uninstalling, cleaning up, and a
warning when a games drive runs low. It's in Game Mode on Hearth OS only (Ember on Windows hides it, like Import;
see [EMBER-PARITY.md](EMBER-PARITY.md)). Not to be confused with **Hearth Storage**, the home server
([HEARTH-LINK.md](HEARTH-LINK.md)).

| Piece | Where |
|---|---|
| Drives, sizes, moving, clean up, warnings, the Send logs section | `hosts/hearth-core/src/diskspace.rs` |
| Bridge methods | `diskspace.*` in `hosts/hearth-core/src/bridge.rs` (and `diskspaceGog` among its internal messages) |
| Recomps follow a moved folder | `recomps::movable_dir`, `recomps::relocate` |
| The page | `pageDiskspace` and the `dsk*` functions in `ui/index.html` |
| Tests | `diskspace.rs` tests (cargo test), the Storage page block in `tools/ui-check.js`, screenshots: `layout-shots.yml` with the filter `diskspace` (`look-*-diskspace`) |

## What the page shows

**Drives**: the drive Hearth runs from ("This device"), SD cards, drives set up with Hearth Disks' *Use for games*
(`/var/mnt/games-*`), USB and other drives the desktop mounted (`/run/media/<you>`, `/media`, `/mnt`), and any drive
holding a Steam library or Hearth's install folder. Each with free/total, file system, and a bar split into games,
Proton prefixes (Steam's `compatdata`, Hearth's `prefixes`), shader caches (Steam's `shadercache`, Mesa's and
Nvidia's caches), downloads (Steam's `downloading` and `temp`) and other. Selecting a drive shows only its games.
Memory file systems and the image-based system's read-only root are left out (`real_fs`); read-only drives are
shown but never offered as a destination.

**Games by size**: every installed game the library knows (Steam, including Flatpak Steam's folders; Epic through
legendary; GOG; Battle.net; Xbox PC (Xodus); Beacon; Recomps; emulators; your own games), biggest first, or by name
or store, filtered by drive. Each shows its folder's size, and its Proton prefix and shader cache separately. Sizes
are added up in the background (like `du`: links not followed, other drives mounted inside left out, hard links
once) when the page opens and the last result is older than 30 minutes, and kept in
`~/.local/share/nimbus/diskspace-sizes.json` so the page shows them at once next time.

**Selecting a game**: *Move to <drive>* for each other writable drive (with its free space; "not enough space" when
it doesn't fit), *Uninstall*, or what Hearth can't do and why.

## Moving a game

| Store | What moves | How the store learns the new place |
|---|---|---|
| Steam | the game's folder, its Proton prefix (`compatdata/<id>`), shader cache, Workshop items (`workshop/content/<id>`, `appworkshop_<id>.acf`) into the Steam library on that drive (made with Hearth Disks' code, `steam::add_library`, if the drive has none: `<drive>/SteamLibrary`) | Steam has no supported way to be told to move a game, so it's done with **Steam closed** (closed properly first, never killed; refused while a Steam game runs or Steam downloads/updates anything): the record (`appmanifest_<id>.acf`) is written in the new library, the old one set aside, and the game's line in `libraryfolders.vdf` moved to the new library (left to Steam if the file doesn't look as expected). If Steam was started during the move, nothing is switched and the copy is removed. Steam is started again hidden afterwards if it was running (unless "Start Steam only when needed") |
| Epic | the game's folder, to `<install folder>/Epic` (or `<drive>/Games/Epic`) | `legendary move <app> <new base> --skip-move` (what legendary itself recommends for another drive), then checked in legendary's `installed.json` |
| GOG (Hearth's installs) | the game's folder (it must hold `goggame-<id>.info`), to `.../GOG` | library.json `gogInstalls` (through the bridge: `diskspaceGog`) |
| Recomps | a port Hearth installed (not a folder you pointed it at) to `<drive>/Hearth Recomps/<id>`, or back to Hearth's own folder | its `hearth.json`, start script and desktop shortcut (`recomps::relocate`) |
| Battle.net | not moved: its games live inside Battle.net's own Wine prefix (Faugus' or Hearth's). The page says to move them with Battle.net | - |
| Xbox PC (Xodus), Beacon | not moved: they manage their own folders. The page says where to do it | - |
| Emulators, your own games | not moved: Hearth doesn't know which files belong to them | - |

Epic and GOG games keep their Proton prefix (with saves) in Hearth's folder on the system drive.

Steps, all in the background with progress (`diskspace` events, `job`), one move at a time:

1. **Checks** (before anything): the game isn't running, put aside, downloading, or in use by any process (its
   folder as a program's or working folder); the destination is another, writable drive; nothing of the game is
   there already; there's room for the game plus 1 GB.
2. **Copy** each part to `.hearth-partial-<name>` beside its new place (never looks installed). Links are copied as
   links, permissions and times kept; pipes and devices refuse the move.
3. **Check**: `sync`, then every file, folder and link of the original must be in the copy with the same kind, size
   and bytes (compared byte for byte, the copy read with `POSIX_FADV_DONTNEED` so it comes from the drive, not from
   memory), and the copy must have nothing extra.
4. The copies take their real names (`placed`), the store's record is switched (`registered`).
5. Only then is the original deleted (`Guard::remove`, see below).

**Stop** works while copying and checking: the unfinished copy is removed and the game stays where it was.
Switching over and deleting can't be stopped (seconds).

**Cut off** (power, crash): `~/.local/share/nimbus/diskspace-move.json` records the phase. The next time the page
is opened (never at start-up) a move whose record already points at the copy is finished (the original deleted),
anything else is undone (the copy removed, Steam's record put back). The page says which.

## Uninstalling

Through the same paths the library uses: `game.uninstall` (Steam without Steam's dialog, Epic with legendary, GOG
Hearth's own folder), `recomps.uninstall` (saves backed up first). Battle.net, Xodus and Beacon games: the page says
to uninstall them there.

## Clean up

*Find things to clean up* lists, with sizes, nothing deleted yet:

| What | Rule |
|---|---|
| Steam shader caches (`steamapps/shadercache/<id>`, every library) | chosen at first; Steam rebuilds them |
| Mesa and Nvidia shader caches (`~/.cache/mesa_shader_cache*`, `radv_builtin_shaders`, `nvidia/GLCache`, `nvidia/DXCache`, `~/.nv/GLCache`, and the Steam Flatpak's) | chosen at first |
| Proton prefixes of games that aren't installed: Steam's `compatdata/<id>` with no record in any library (not `0`, not the numbers of games you added to Steam yourself), Hearth's `prefixes/epic-*`, `prefixes/gog-*` | **never** when the game is installed. Saves are looked for (Documents, Saved Games, AppData; Windows' and Proton's own folders left out) and listed. With saves: kept unless Steam Cloud has them (a `remote` folder with files in `userdata`); then off at first and confirmed again with the saves listed. Without saves: chosen at first |
| Compatibility tools in `compatibilitytools.d` (GE-Proton and others) | only ones not chosen for any game in Steam (`config.vdf` CompatToolMapping, also the default) or in Hearth's Game settings, and never the newest of their family; off at first |
| Unfinished downloads (`steamapps/downloading/<id>`, `temp/<id>`) | only of games that aren't downloading (no record, or installed with nothing waiting) |
| Unfinished copies (`.hearth-partial-*`) and import leftovers (`.hearth-moved-*`) | not while a move runs |

*Delete N items* asks once with the total, and again for prefixes with saves ("Delete them too" / "Keep those,
delete the rest"). Refused while a game runs or a move is busy. The list is made again right before deleting; items
that changed or aren't allowed are kept and reported.

## Safety rules

* Deleting goes through `Guard::remove` only: the path must be inside the folder it was found in (one level or
  more), its real parent inside that folder's real path (no reaching through links), not a mount point, not the
  home folder or one of its parents, and not under a system area (`/usr`, `/etc`, `/boot`, `/efi`, `/sysroot`,
  `/ostree`, `/proc`, `/sys`, `/dev`, `/bin`, `/sbin`, `/lib*`, `/opt`, `/root`, `/srv`, `/var/lib`, `/var/log`,
  `/var/cache`, `/var/tmp`, `/var/spool`, `/tmp`, `/run` except `/run/media`, `/snap`, `/nix`). Links are removed,
  never followed.
* Every delete is written to Hearth's log before it happens ("clean (dry run): …", "move plan: …").
* Nothing here runs at start-up and nothing is deleted automatically: no services, no timers. The low space check is
  only free space (`df`), a couple of minutes after Hearth starts and then every 15 minutes, while Hearth runs.

## Low space warning

Settings › Storage › *Warn when a games drive is low on space* (on, 15 GB; 5 to 100 GB, library.json `lowSpace`).
A games drive below it shows in notifications (Quick Access) once per drive per run, on the page, and before an
install Hearth starts (Steam to its chosen library, Epic and GOG to Hearth's install folder): *Install anyway* or
*Open Storage*.

## Send logs

A "Storage (drives and space)" section: each drive (name, kind, mount, file system, size, used, free, read-only),
the Steam libraries, the warning setting, the 25 biggest games by **folder name only** with their prefix and shader
cache sizes, and an unfinished or last move. No other file names.

## Bridge

| Method | Args | Answer |
|---|---|---|
| `diskspace.status` | `{scan?: bool}` | `{drives: [{id, name, kind, mount, fs, size, used, free, readOnly, games, steamLibs, parts: {games, prefixes, shader, downloads, other}}], games: [{id, title, source, store, folder, path, drive, size, prefix, shader, canMove, moveWhy, uninstall: "game"\|"recomp"\|null, uninstallWhy, running}], scan: {state, at}, job, warn: {on, gb}, low: [{name, free, mount}], recovered}` |
| `diskspace.scan` | `{}` | starts adding up sizes; events `diskspace` `{scan: {state, done, total, name}}` |
| `diskspace.move` | `{id, to: <drive mount>}` | `{job}`; events `diskspace` `{job: {state: moving\|done\|stopped\|error, phase: prepare\|steam\|copy\|verify\|switch\|delete, done, total, bps, error}}` |
| `diskspace.cancel` | `{}` | `{job}` |
| `diskspace.cleanList` | `{}` | `{items: [{key, kind, title, where, bytes, saves, cloud, selectable, on, confirm, why}]}` |
| `diskspace.clean` | `{keys, confirmed}` | `{freed, done, failed: [{title, why}]}` |
| `diskspace.low` | `{}` | `{on, gb, low}` |
| `diskspace.check` | `{id}` | `{low, free, gb, path}` for where that game would install |
| `diskspace.setWarn` | `{on, gb}` | `{on, gb}` |

## Disk Management (Hearth Disks)

Hearth Disks (`hosts/hearth-disks`) is laid out like Windows' Disk Management, in Hearth's dark blue look:

* **Menu bar**: File (Rescan disks, Make a bootable USB, Close), Action (the same as the right-click menu for what's
  selected), View, Help; then Rescan disks, Health (Properties › Health of the selected disk) and Make a bootable USB.
* **Volume list** (top): Volume, Layout, Type, File system, Status, Capacity, Free space, % Free.
* **Disk map** (bottom): each disk's label (Disk N, kind, size, online, a health dot) and its partitions as blocks
  sized by their size, with a stripe: **gold** Hearth runs from it (protected), **green** used for games, **blue** a
  normal partition, **black** unallocated. The selection is hatched; a legend and a status bar sit below.
* **Drive letters** are Hearth's own, from `hearth_drive_letters.py` (the ones Hearth Files and Windows apps use):
  C: is always Hearth. Disks only reads them (opening it writes nothing); Change drive letter… calls `set_letter`.
* **Controller**: A selects, X shows what can be done, B goes back, Y rescans, Start opens the menu bar, LB/RB turn
  Properties' pages. Mouse (right-click) and keyboard (arrows, Enter, Menu key or Shift+F10, Alt+Enter, Delete, F5)
  work too.

The right-click (X) menu depends on what's selected. Everything it can't do is greyed out with why (`blocked`, from
`safety.rs`):

| Selected | Offered |
|---|---|
| Unallocated space | New Simple Volume…, New volume for games…, Extend *the volume before it* into this space…, Properties |
| A normal volume | Open (in Hearth Files; mounts it first), Change drive letter…, Format…, Extend Volume…, Shrink Volume…, Delete Volume…, Use for games, Unmount / Mount / Safely remove, Properties |
| Hearth's own (system, EFI, start-up files, swap, and what Hearth is built on through LVM, RAID or encryption), and any start-up or recovery partition | Open and Properties only; the rest greyed out: "Hearth runs from this partition, so it can't be changed or deleted here." |
| A disk's label | Run a full check, Safely remove, Format the whole drive…, Use the whole drive for games…, Properties, Health |

* **New Simple Volume**: Size, Letter, Format (ext4 "best for Hearth and games", NTFS, exFAT, FAT32 up to 32 GB,
  Btrfs; a label; allocation unit Default; Quick format; Use it for games; Open it in Files when it's ready), Finish.
  Only the unallocated space is used.
* **Format…** shows the capacity, file system, label and Quick format (off: zeros are written over all of it), and in
  red what will be erased, with the games on it and their size (from Storage's sizes, `diskspace-sizes.json`, and
  Steam's records on the drive). The Format button stays greyed out until the volume's name is typed exactly.
  **Delete Volume…** asks for the name the same way.
* **Extend / Shrink**: Windows' numbers in MB (total before, what can be added or taken away, the amount, total
  after), a slider and a picture of the disk after the change. Done through udisks2 (`Partition.Resize`,
  `Filesystem.Resize`), in the order that never leaves a file system bigger than its partition: Extend grows the
  partition, then the file system; Shrink makes the file system a little smaller than the new size, then the
  partition, then fits the file system to it. A file system that only changes unmounted (ext4 shrinking, NTFS) is
  unmounted, checked (`Filesystem.Check`; ext also gets the check resize2fs asks for) and mounted again. Refused
  while Steam or a program uses the volume, when the file system isn't clean, on the drive Hearth runs from, inside
  an extended partition, and for exFAT and FAT32 (they can't be resized). A games drive keeps at least 10 GB free.
  When udisks2 says a resize tool is missing (`Manager.CanResize`, e.g. ntfsresize), the action is greyed out and
  says which.
* **Properties**: General (with Rename, Check for problems, Repair), Health (status, temperature, powered on, bad
  sectors or wear, the drive's SMART or NVMe lines, the last self-test, Run a full check: the drive's extended self-
  test) and Volumes. **Tell me if this drive starts to fail** is off unless you turn it on; it's kept per drive (by
  serial) in `~/.config/nimbus/disks.json` (`failAlerts`) through `settings_file.rs`. A user timer, `hearth-disks-health.timer`
  (daily, and about 10 minutes after signing in, with a random delay; only the timer is enabled, through the image's
  `timers.target.wants`, and nothing depends on it) runs `hearth-disks --health-watch` (`hearth-disks-health.service`:
  oneshot, Nice=19, idle I/O, 2 minutes at most). With no drive turned on it ends at once without looking at a drive.
  It never writes the settings; it sends a notification once per new problem (a drive turning to a warning or
  failing) and keeps what it has told you in `~/.local/state/nimbus/disks-health.json`.
* On **Windows** (Ember) Hearth's letters, Extend, Shrink, the full check and the failure notice aren't there.

Every change is checked twice in `safety.rs`: when the menu is drawn and again, from a fresh look at the drives, right
before it runs; the run is refused when the drive or partition isn't the one that was confirmed (`fingerprint`).

## Needs testing on a device

* Hearth Disks' Disk Management: Extend and Shrink of a games drive (ext4) with Steam closed and open (refused), an
  NTFS shrink, New Simple Volume with a letter and "Open it in Files", Change drive letter (Files shows the same),
  Properties › Health on a SATA drive and an NVMe drive, Run a full check, and the controller.
* Moving a Steam game to an SD card and back (ext4 and exFAT), with Steam open (closes, comes back hidden) and with
  "Start Steam only when needed"; the game starts from the new place without Steam re-downloading or re-validating,
  its saves (prefix) are there, Steam's Storage page lists it in the right library.
* Moving to a drive with no Steam library yet (Steam lists the new `SteamLibrary`).
* Epic: legendary accepts `--skip-move` and the game starts; GOG and Recomps start after a move.
* Pulling the power during the copy and during the delete, then opening the page (undone / finished).
* Clean up on a real install: which prefixes show saves, the Steam Cloud check, old GE-Proton removal, and that Steam
  rebuilds shader caches.
* The low space notification and the install warning.

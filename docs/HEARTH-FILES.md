# Hearth Files

Hearth Files is Desktop Mode's file manager (and the Open / Save window every app uses). Since 1.7 it's laid out like
Windows 11's File Explorer, in Hearth's colours and dark or light look. Code: `hosts/linux/desktop/shell/FileManager.qml`
(the window) and `hosts/linux/desktop/shell/hearth-files` (everything that touches files). Mockup: the owner's approved
design, "Hearth Files redesign".

## The window

- **Tabs** along the top, like a title bar: **+** or Ctrl+T opens the Home page in a new tab; middle-click a tab (or
  Ctrl+W) closes it; Ctrl+Tab / Ctrl+PgDn moves between them. Middle-click a folder, a sidebar item or a Quick access
  tile to open it in a new tab. Drop files on a crumb or a sidebar item to put them there.
- **Back, Forward, Up, Refresh** (Alt+Left, Alt+Right, Alt+Up, F5), the **address bar** (click the crumbs to jump; click
  the empty part, Ctrl+L or / to type a path) and **Search** (Ctrl+F, Ctrl+E or F3: typing filters this folder, Enter
  also looks in the folders inside).
- **The command bar**: New (folder, empty file, or a document from Templates), Cut, Copy, Paste, Rename, Copy path,
  Delete (Move to Trash; in the Trash, delete for good) | Sort (and group), View, Show (hidden files, file name
  extensions) | Undo, Redo, Details (the details pane). Buttons that can't do anything right now are dimmed.
- **The navigation pane**: Home, Favorites (your bookmarks: Ctrl+D or "Pin to the left menu"), This device (Home folder,
  Desktop, Documents…, in yellow), Games, then **This PC** opened out like a tree with Hearth (C:) and every drive
  under it by its letter (with free space; mount, unmount, safely remove, Change drive letter…), **Network** (Hearth
  Storage and Your devices: shared folders on your other devices, at home and over Tailscale), **Installed apps** and
  Trash. The item you're in has the blue bar; This PC's and Network's arrows fold them (for this session).
- **The Home page**: Quick access tiles (your usual folders, Favorites and Games) and Recent, the files you opened
  lately in any app (from `recently-used.xbel`; ones that were deleted or moved don't show). "Show in its folder" in
  a recent file's menu opens where it is.
- **The details pane** (Details button or Alt+P): a large thumbnail or icon, name, type, size, modified, where it is,
  Open and Properties; with several picked, how many and their total size.
- **The status bar**: how many items, how many picked and their size, what's on the clipboard, free space; copies and
  moves show their progress, speed and time left there with Cancel.

## This PC

Like Windows 11's This PC (the owner's approved mockup, "This PC in Hearth Files, the Windows 11 way"). Open it from
the navigation pane, the address bar (type "This PC"), or make it where Files opens: **View › Open Files to: Home /
This PC** (Home unless you pick This PC; kept in files.json `startPage`, only when you pick it). Code:
`FilesThisPc.qml` (the page, loaded with a Loader so a problem there never stops Files or Desktop Mode from opening),
`WinGlyph.qml` (yellow folders and drive glyphs), `hearth-files this-pc`.

- **Folders (6)**: Desktop, Documents, Downloads, Pictures, Music and Videos, wherever your XDG folders are
  (`~/.config/user-dirs.dirs`), with their real names.
- **Devices and drives**: **Hearth (C:)** (the drive Hearth runs from; its space is your home's), then internal data
  drives, Hearth Disks' games drives (`/var/mnt/games-*`), drives with a Steam library, USB sticks and SD cards
  (plugged in but not mounted ones too: open mounts them). Each tile: the drive glyph, "Name (X:)", the thin space bar
  (red above 90% full) and "x free of y".
- **Network locations**: your devices' shared folders that are connected now, connect by themselves, or already have a
  letter, named like "Games (\\Gaming PC) (G:)". Opening one connects it (and asks to sign in the first time).
- **Tiles** or **Details** (Name, Type, Total size, Free space) from View (Ctrl+1 / Ctrl+3); kept in files.json
  `thisPcView`, only when you change it.
- Right-click or X: Open, Open in new tab, Unmount / Safely remove (USB and SD), Connect automatically (network),
  **Change drive letter…**, Copy path / Copy Linux path, Properties.
- The controller and keyboard move through the groups like Windows (Down from the last row of Folders goes to the
  drives below); A opens, B goes back, Y types to filter.

## Drive letters

`hearth_drive_letters.py` (used by hearth-files and hearth-winapps):

- **C: is always Hearth.** It's never stored and can't be given to another drive.
- Every other drive gets the **next free letter, D: to Y:,** the first time it's seen, keyed by its file system UUID
  (or where it's mounted when it has none); network locations by their address. Z: is left alone: it's how Windows
  apps see the whole system.
- **A letter never changes by itself and never moves to another drive.** Unplugged drives keep theirs (so a new drive
  gets the next letter, not theirs). Only **Change drive letter…** (This PC's or the navigation pane's right-click
  menu) changes one, to a free letter, and that choice is kept.
- Kept in `~/.config/hearth/drive-letters.json` (state, through hearth_settings_io: atomic, a .bak, under its lock).
  It's written only when a drive is seen for the first time, when a drive's mount place changes, or when you change a
  letter: opening Files with only C: writes nothing. A damaged file is put back from its .bak, or kept aside once as
  `.broken-<time>` (never written over), and letters start again from what's seen next.
- **The address bar in letters**: "This PC › Games (D:) › SteamLibrary" on a lettered drive, "This PC › Documents"
  in your folders, "This PC › Hearth (C:) › usr" elsewhere on the system (the rest of your home folder keeps
  "Home › …"). Typing `D:\Games`, `d:/Games` or `C:\Users\<you>\Documents` goes there; clicking the address bar on
  a lettered drive shows the `D:\…` form. Code: `drive-paths.js`.
- **The details pane** shows where something is in letters and, under "On Linux", its real path.
- **Copy path** copies the letter form (`D:\SteamLibrary\steamapps`), **Copy Linux path** (Ctrl+Alt+C, as before)
  the real one; the command bar's Copy path offers both.

### Windows apps see the same letters

Each time a Windows app starts (hearth-winapps, `ensure_prefix` → `hearth_drive_letters.map_prefix`):

- `dosdevices/d:`, `e:` … in its prefix point at the same drives as in Files (network locations at their mounted
  folder). C: stays the prefix's own C:, Z: stays the whole system. A letter Wine or you set up yourself is left as it
  is; links Hearth made are remembered in `prefix/.hearth-drives.json`, so a changed letter moves its link and nothing
  else is touched.
- Documents, Desktop, Downloads, Pictures, Music and Videos of the prefix's user (Proton's `steamuser`) point at your
  own folders, when the folder there is missing or empty (or Hearth linked it before). One that already has files in
  it is left alone: nothing is moved.
- A brand-new prefix gets its letters at its second start (Proton makes the prefix on the first).

## Hearth (C:) the Windows way

Since 1.10, opening **Hearth (C:)** (This PC, the navigation pane, typing `C:\`) shows Hearth's drive with Windows
folder names (the owner's approved mockups, "Hearth (C:) drive layout"). **It's a view only: nothing on disk moves, and
nothing is made.** Each folder shows where its files really are. Code: `drive-paths.js` (the mapping and the pages, pure
functions), `FilesCDrive.qml` (the pages, behind a Loader like This PC), `FilesCProps.qml` (Properties, behind a Loader),
`hearth-files cdrive` (where things really are; reads only), `hearth_drive_letters.py` (the same mapping for the helper),
`SetDrives.qml` (Settings).

| In Hearth | Really | |
|---|---|---|
| `C:\Program Files` | one entry per installed app (Installed apps' list): **Hearth** (Hearth's own apps, look only), each Flatpak (`/var/lib/flatpak/app/<id>`), **Hearth Games** (the stores' folders), each Recomp | a view |
| `C:\Program Files (x86)` | each Windows app (`~/Games/Windows apps/<name>`) and **Steam** (its folder) | a view |
| `C:\ProgramData` | `/var/lib` | look only |
| `C:\Users` | the folder home folders are in (`/var/home`); `C:\Users\<you>` is your home folder | real |
| `C:\Users\<you>\AppData` | **Roaming** = `~/.config`, **Local** = `~/.local/share`, **Local\Temp** = `~/.cache` | a view (AppData shows in your home folder, Temp in Local) |
| `C:\Windows` | `/usr` (with a lock badge) | look only |
| `C:\etc` and the rest | the real top-level folder | real |

- **Opening** an entry goes to its real folder (Firefox in Program Files opens `/var/lib/flatpak/app/org.mozilla.firefox`).
  Inside real folders reached through the view (`Users\<you>\Documents`, an app's folder) everything works as always.
- The pages are the mockup's Details list: **Name, Type, Size, On Linux**, with a note at the top. The details pane
  shows **In Hearth** (`C:\Program Files\Firefox`) and **On Linux** (the real place or places) for everything, real files
  too.
- **"Show the Linux folders too"** (off unless you turn it on; on C:\ itself, View, or Settings) adds the real top-level
  folders under "THE LINUX FOLDERS".
- **The address bar** shows the `C:\Program Files\…` forms and the steps "This PC › Hearth (C:) › Program Files ›
  Firefox" (your own folders and the rest of your home folder keep their steps). Typing `C:\Program Files\Firefox`,
  `C:\Users\<you>\AppData\Roaming`, `C:\Windows\share` and so on goes to the real folder; Linux paths always work too.
  D:, E: and the other letters work exactly as before. Up goes to the step before (Firefox → Program Files → Hearth (C:)
  → This PC).
- **Right-click** (or X): Open, Open in new tab, Go to the real folder, Copy path (the `C:\` form, Ctrl+Shift+C), Copy
  Linux path (Ctrl+Alt+C), then **Modify / Uninstall** for apps (Installed apps' own actions) and **Properties**.
- **Properties** of a view folder: its size, how many apps (or folders) and which real places they come from, and that
  you can't put files straight into it. Real folders get the usual Properties window.
- **Writes into the view** are refused with a clear message: dropping or pasting into Program Files, Program Files (x86),
  AppData or `C:\` itself ("To add an app, install it, and it shows up here by itself"), and into Windows or ProgramData
  (look only). The view's names for other folders (AppData, Temp) can't be cut, copied, renamed or deleted; hearth-files
  refuses any change to a `hearth:c` path as a backstop.
- **Windows apps are unchanged**: inside a Wine prefix C: stays the prefix's own drive (`map_prefix` never touches `c:`).
- Open / Save windows keep the real folders (the view is for Files itself).

**Settings › System › Drives & folders** (and Files' View menu, the same switches): Show Hearth (C:) with Windows folder
names (on unless turned off; off, Files is exactly as in 1.9), Show the Linux folders too (off), The address bar shows
Windows names / Linux paths, Copy path gives the `C:\` form / the Linux path (the other is always in the menu), and the
drive letters (C: is always Hearth; **Change letter…** for the others, the same as This PC's). They're kept in
`~/.config/nimbus/files.json` (`cdrive`, `linuxFolders`, `addressBar`, `copyPath`) through `hearth-files prefs-set`
(hearth_settings_io: merged under the lock, atomic, a `.bak`, never defaults over a damaged file), **only when you change
one**: opening Files or Settings writes nothing, and a switch you set to what it already is writes nothing. Files reads
them again each time its window opens, so a change in Settings shows straight away.

## Installed apps

Like Windows Settings › Apps › Installed apps (`FilesApps.qml`, loaded with a Loader; `hearth-files installed-apps`):
every app with its icon, a kind tag (**Hearth, Flatpak, Windows app, Steam, Recomp, Hearth Games**), publisher if
known, install date if known, and size. Sort by name, size or install date and filter by kind (the "Sort by" /
"Filter by" buttons, or the command bar's Sort and Show). The data comes from what Hearth already keeps: Hearth's own
apps' menu entries, `hearth-flatpak installed` (sizes, publisher from the app's metainfo), `hearth-winapps list`
(sizes added up lazily and cached in `~/.cache/hearth/app-sizes.json`) and Storage's games list
(`nimbus --settings-call diskspace.status`: Steam, Hearth Games' stores and Recomps, with the sizes the Hearth app keeps;
pictures from the launcher's cache). Nothing scans the disk.

The **"···"** menu (or A / X on a row):

- **Open** / **Play** (games start through Hearth, like its Play button).
- **Modify**: Windows apps → Settings › Windows apps; Steam games → Steam's Properties; Flatpak apps → Flatseal (only
  when it's installed). Not shown where there's nothing to open.
- **Move to another drive**: only for games Hearth's Storage can move (Steam, Epic, GOG, Recomps): opens Storage in the
  Hearth app.
- **Uninstall**, through each kind's own way: Flatpak (`hearth-flatpak remove`) and Windows apps (`hearth-winapps
  remove`) after "Uninstall …?"; Steam games through Steam's own uninstall dialog; other games in Hearth's Storage
  page. Hearth's built-in apps say **Part of Hearth** and have no Uninstall.

## Everything else it does

- Pick with a click, Ctrl+click, Shift+click, or drag a rubber band on empty space; Ctrl+A select all, Ctrl+I invert,
  Esc clear. Type the start of a name to jump to it.
- Drag and drop to folders, tabs' crumbs, sidebar items, Quick access tiles and other apps: a plain drag moves (copies
  between drives), **Ctrl** copies, **Alt** makes a link. The drag shows the first item's picture and how many.
- Copy / cut / paste (Ctrl+C / X / V, also with files copied in other apps), with progress and a prompt when a name is
  taken (skip, keep both, replace; for all of them).
- **Undo and redo** (Ctrl+Z, Ctrl+Y or Ctrl+Shift+Z): rename, new folder or file, Move to Trash (put back from the
  Trash), copy, move (moved back) and links. The last 50 changes this session; deleting for good asks first and can't
  be undone.
- Rename (F2; several at once), Move to Trash (Del), delete for good (Shift+Del, asks first), New folder
  (Ctrl+Shift+N), new file (Ctrl+Alt+N), Open with, Properties (Alt+Enter), Open in Terminal (Shift+F4), Copy path
  (Ctrl+Alt+C), compress and extract, Preview (Space), Set as wallpaper, Install (.deb), Run (AppImage).
- **Views**: Large icons (Ctrl+4), Medium icons (Ctrl+1), Small icons (Ctrl+5), Tiles (Ctrl+6), List (Ctrl+2) and
  Details (Ctrl+3), sort by name, date, size or type and group by type or date; each folder remembers its own.
- Thumbnails for pictures and videos; hidden files (Ctrl+H) and file name extensions toggles.
- The controller and touch work everywhere: the D-pad moves, Up from the files reaches the address and command bar,
  Up again the tabs, A opens or presses, B goes back, X opens the actions menu, Y types (the on-screen keyboard),
  press and hold for the menu on touch.

## Briefcase

Windows 95–XP's Briefcase, made new. Code: `hosts/linux/desktop/shell/hearth-briefcase` (every file change; a CLI with
a JSON answer, like `hearth-files`), `Briefcase.qml` (what Files shows) and `BriefcaseDialog.qml` (its dialogs).

- **A Briefcase is a folder** with a small hidden list inside (`.briefcase.json`), so it travels with the folder: on a USB
  stick, a shared folder or another Hearth device. **A Briefcase is a folder you make**: **New › Briefcase** makes one
  anywhere. Files doesn't pin `~/Briefcase` in the navigation pane or make it by itself any more (1.10); a `~/Briefcase`
  you already have stays a normal folder in your home folder, never moved or deleted, and still works as a Briefcase
  with its icon (a Favorite you added for it yourself stays). Other Briefcases you made show in the navigation pane
  unless you turn off "Show in navigation pane" in Customize. Its icon is a briefcase (or a satchel, suitcase, backpack or box) in
  its colour, with its badge and a padlock when it's locked.
- **Adding**: drag files or folders onto it (its folder, its icon, its place in the navigation pane), or **Add to
  Briefcase ›** in the right-click menu (your Briefcases, and **New Briefcase…**: pick where it goes, then the items go
  in; with no Briefcase yet, that's the choice). Each item is copied in and remembers its original: a path on this PC, a USB
  drive (its filesystem ID and the path on it, so it's found again under any mount point) or a shared folder from Your
  devices (`smb://`, opened through gio), with the size, time and a hash (SHA-256; big files only when needed) of both
  copies at the last update.
- **Status** (the Status column in Details, a badge on each icon, worked out again when it's opened and every few
  seconds while it's shown): Up-to-date · Newer in Briefcase · Newer at original · Changes on both sides (a folder) ·
  Changed in both · **Original unavailable** ("Plug in “STICK”", "Desk is offline", "The original is on Laptop":
  never called an orphan because it's away) · **Orphan** (the original was deleted, or you split it).
- **Update All** (command bar or menu) / **Update** (the picked items) first shows a list of what it will do, one row
  per file: **Replace →** original, **← Replace** in the Briefcase, **Skip**, **Keep both** (for a file changed in both
  places: the other copy is saved beside it as `name (from <device> <date>).ext`), and for a file deleted on one side
  **"Deleted at original: keep or delete?"** (Delete here too / put it back; Skip by default, never automatic). Change any
  row (click it, or Left / Right), then **Update**. Copies go to a temporary file that's renamed over (a crash or power
  cut leaves the old file or the new one), with progress and Stop. A replaced or deleted version goes to the **Trash on
  its own side**: your Trash, or the USB stick's own `.Trash-<uid>` (the freedesktop spec); on a shared folder, where
  there's no Trash, into `.briefcase-replaced/<date>` inside the Briefcase (the message says so). A row whose file
  changed after the list was made is skipped. Folders sync file by file; new files on either side are added.
- **Split from original** (the copy stays, no longer updated: Orphan) and **Find original** (opens its folder).
- **Send Briefcase to…** a USB drive (or your home folder) copies the whole Briefcase with its list, the old floppy
  way. A Briefcase found on a USB stick or another device's share works the same, because originals are kept as drive-ID
  or share references with the device's name.
- **Between Briefcases**: drag an item from one Briefcase to another (Ctrl copies), or **Move to Briefcase ›** /
  **Copy to Briefcase ›** / **Swap with…**. The item keeps its link to its original. Copies are checked by their hash
  before the source's copy goes to the Trash; a crash in between leaves two copies, never none.
- **Customize…** (menu or command bar): name, colour (presets or your accent), icon, a badge (an emoji or up to four
  letters), a note (shown in its details pane), its own view and sort, Auto-sync, Auto-lock, Lock when the PC sleeps,
  Lock when the USB stick is ejected, Show in navigation pane. Kept in the Briefcase's own list (through
  `hearth_settings_io`: atomic, a `.bak`), so it travels with it; only what you change is saved, and the files inside
  aren't touched.

### Locked Briefcases

- **Lock this Briefcase…** asks for a password twice and says plainly: "If you forget this password, the files inside
  can't be recovered. Hearth can't reset it." The files are encrypted with **gocryptfs** into `.briefcase-vault` inside
  the folder (so it unlocks with the same password on a USB stick or another Hearth device), the encrypted copy is
  closed, opened again and compared file by file, and only then does the unlocked copy go to the Trash (empty the
  Trash, and on a USB stick its own Trash, to remove it). **Remove lock** does the reverse with the same checks.
  **Change password** re-wraps gocryptfs's key (nothing is encrypted again).
- **Unlock** asks for the password (the on-screen keyboard and the controller work in the box); "Remember on this
  device" (off unless you tick it) keeps it in the system keyring with `secret-tool`. The password goes to the helper on
  stdin, never on a command line, in the environment, a file, a log or Send logs. Unlocked, its files are shown from
  `$XDG_RUNTIME_DIR/hearth-briefcase/…`; locked, Files shows its name and Locked, with Unlock (Update All and
  moving items ask for the password first).
- **Locks by itself**: after the time in Customize › Auto-lock (Never, 1, 5, 15, 30 minutes, 1 hour, 4 hours or your
  own; 15 minutes when a lock is added), counted from the last time something in it was opened (in Files) or changed;
  a file open in an app keeps it unlocked until the app closes it. A minute before, a notification says
  "Briefcase “School” will lock in 1 minute" with **Keep unlocked**. Also when the PC goes to sleep and when its USB
  stick is ejected (each a switch, both on), and when you sign out. The details pane shows "Locks in 12 min". A small
  watcher (`hearth-briefcase guard`) runs only while a Briefcase is unlocked.

### Auto-sync

- Customize › Auto-sync: **Right away** (every new Briefcase starts with it; one already made keeps whatever it has),
  Every 5 / 15 / 30 minutes or your own, **When I lock it** (just before it locks), or **Off** (only Update All).
- Only **one-sided** changes and new files are copied by themselves, once the file has been left alone for about three
  seconds and no app has it open for writing. **Changed in both** and **deletions** always wait for you: the badge shows
  it, and a notification says "2 files in “School” need you" (with Open). Replaced versions go to the Trash as with
  Update All. An original on an unplugged drive or an offline device waits quietly and syncs when it's back (a drive
  plugged in or a shared folder opened is noticed). A locked Briefcase waits until it's unlocked. Syncs to a shared
  folder are throttled; it pauses while a game runs.
- "Synced 2 min ago" / "Syncing…" / "2 files need you" in the details pane and the status bar.
- It's the user service `hearth-briefcase-sync.service` (os/system_files/usr/lib/systemd/user): asked for with
  `--no-block` from Desktop Mode's autostart and by Files, skipped by systemd (`ExecCondition`) when no Briefcase has
  auto-sync on, stopping by itself when the last one is turned off, `Restart=on-failure` with a limit, never part of
  sign-in or Game Mode. Log: `~/.local/share/nimbus/briefcase.log` (Send logs includes it).

## Settings

View, sort, groups, hidden files, file name extensions and the details pane are kept in `~/.config/nimbus/files.json`
through `hearth-files prefs-set`, which merges under a lock with `hearth_settings_io` (atomic, a .bak kept, never
written over a file it couldn't read; docs/SETTINGS-PERSISTENCE.md). Nothing changes them except you.

## Checks

- `hosts/linux/test/test_hearth_this_pc.py`: drive letters (C: is Hearth, first seen gets the next free letter,
  stable across unplugging and order, nothing written for known drives or C: alone, your change kept and never
  moved, a bad file kept aside once or put back from .bak), paths in letters and back (`drive-paths.js` with node),
  This PC's drives from pretend mounts and lsblk, Windows apps' prefixes (letters, user folders, Wine's own links
  kept, a changed letter), and that the pages load through Loaders.
- The Desktop Mode test's `shots/files-this-pc/` (dark and light): This PC, inside the games drive, Installed apps,
  its "···" menu, the details pane with the Linux path, This PC's Details view. Opening them may only create or change
  `~/.config/hearth/drive-letters.json`; any other changed file under `~/.config` or Hearth's settings fails the test.
- `hosts/linux/test/test_hearth_cdrive.py`: Hearth (C:) the Windows way: C:\ forms and real paths both ways (drive-paths.js
  with node, and hearth_drive_letters.py's copy), the steps, the pages from an Installed apps list, AppData and Temp,
  writes into the view refused (and real folders reached through it working), D: and E: and the view switched off
  exactly as before, a Wine prefix's own C: untouched, nothing written when Files or Settings opens, only the changed
  key merged, a damaged files.json put back from .bak. The Desktop Mode test's `shots/files-cdrive/` (dark and light):
  C:\, Program Files and its Properties, Program Files (x86) and its right-click menu, your home folder with AppData,
  AppData, the Linux folders, Settings › Drives & folders; typed C:\ paths checked to reach the real folders; opening
  them may change no settings file, and the switch only `linuxFolders`.
- `hosts/linux/test/test_hearth_briefcase.py`: the Briefcase: statuses (unplugged versus deleted), every Update action,
  conflicts keeping both, the Trash (home, a USB stick's, `.briefcase-replaced`), a crash mid-copy (the helper killed),
  folders, a bad list kept aside, carrying, swapping, Customize, locks (gocryptfs where FUSE works, else a stand-in, with a
  `::notice` saying which), auto-lock and auto-sync. The Desktop Mode test's pictures are in `shots/briefcase/`.
- `hosts/linux/test/test_hearth_files.py`: untrash (undo of Move to Trash), links, recent files, and that the window
  keeps its buttons and features (`Redesign`).
- The Desktop Mode test takes pictures in `shots/files-redesign/` (dark and light: Home, a folder with three picked
  and the details pane, a drag in progress (staged over IPC, as CI has no mouse), Your devices, Large icons, Tiles,
  the actions menu) and publishes them to the `ci/desktop-shots-<branch>` branch.

## View menu (1.11)

Layout: Extra large icons (Ctrl+7), Large icons, Medium icons, Small icons, Tiles, List, Details. Show: Navigation
pane (hide the left menu; saved as `navPane` in files.json), Details pane (Alt+P), File name extensions, Hidden files,
and on Hearth (C:) the Linux folders and Linux paths in the address bar.


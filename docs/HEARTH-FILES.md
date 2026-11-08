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
- **The navigation pane**: Home, Favorites (your bookmarks: Ctrl+D or "Add to Favorites"), This device (Home folder,
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
  (Ctrl+Shift+N), new file (Ctrl+Alt+N), Open with, Properties (Alt+Enter), Open terminal here (Shift+F4), Copy path
  (Ctrl+Alt+C), compress and extract, Preview (Space), Set as wallpaper, Install (.deb), Run (AppImage).
- **Views**: Large icons (Ctrl+4), Medium icons (Ctrl+1), Small icons (Ctrl+5), Tiles (Ctrl+6), List (Ctrl+2) and
  Details (Ctrl+3), sort by name, date, size or type and group by type or date; each folder remembers its own.
- Thumbnails for pictures and videos; hidden files (Ctrl+H) and file name extensions toggles.
- The controller and touch work everywhere: the D-pad moves, Up from the files reaches the address and command bar,
  Up again the tabs, A opens or presses, B goes back, X opens the actions menu, Y types (the on-screen keyboard),
  press and hold for the menu on touch.

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

- `hosts/linux/test/test_hearth_files.py`: untrash (undo of Move to Trash), links, recent files, and that the window
  keeps its buttons and features (`Redesign`).
- The Desktop Mode test takes pictures in `shots/files-redesign/` (dark and light: Home, a folder with three picked
  and the details pane, a drag in progress (staged over IPC, as CI has no mouse), Your devices, Large icons, Tiles,
  the actions menu) and publishes them to the `ci/desktop-shots-<branch>` branch.

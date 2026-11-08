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
  Desktop, Documents…), Games, Drives (with free space; mount, unmount, safely remove), Hearth Storage, Your devices
  (shared folders on your other devices, at home and over Tailscale) and Trash.
- **The Home page**: Quick access tiles (your usual folders, Favorites and Games) and Recent, the files you opened
  lately in any app (from `recently-used.xbel`; ones that were deleted or moved don't show). "Show in its folder" in
  a recent file's menu opens where it is.
- **The details pane** (Details button or Alt+P): a large thumbnail or icon, name, type, size, modified, where it is,
  Open and Properties; with several picked, how many and their total size.
- **The status bar**: how many items, how many picked and their size, what's on the clipboard, free space; copies and
  moves show their progress, speed and time left there with Cancel.

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

## Briefcase

Windows 95–XP's Briefcase, made new. Code: `hosts/linux/desktop/shell/hearth-briefcase` (every file change; a CLI with
a JSON answer, like `hearth-files`), `Briefcase.qml` (what Files shows) and `BriefcaseDialog.qml` (its dialogs).

- **A Briefcase is a folder** with a small hidden list inside (`.briefcase.json`), so it travels with the folder: on a USB
  stick, a shared folder or another Hearth device. `~/Briefcase` is in the navigation pane (made the first time you open
  it); **New › Briefcase** makes another anywhere. Its icon is a briefcase (or a satchel, suitcase, backpack or box) in
  its colour, with its badge and a padlock when it's locked.
- **Adding**: drag files or folders onto it (its folder, its icon, its place in the navigation pane), or **Add to
  Briefcase ›** in the right-click menu. Each item is copied in and remembers its original: a path on this PC, a USB
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

- `hosts/linux/test/test_hearth_briefcase.py`: the Briefcase: statuses (unplugged versus deleted), every Update action,
  conflicts keeping both, the Trash (home, a USB stick's, `.briefcase-replaced`), a crash mid-copy (the helper killed),
  folders, a bad list kept aside, carrying, swapping, Customize, locks (gocryptfs where FUSE works, else a stand-in, with a
  `::notice` saying which), auto-lock and auto-sync. The Desktop Mode test's pictures are in `shots/briefcase/`.
- `hosts/linux/test/test_hearth_files.py`: untrash (undo of Move to Trash), links, recent files, and that the window
  keeps its buttons and features (`Redesign`).
- The Desktop Mode test takes pictures in `shots/files-redesign/` (dark and light: Home, a folder with three picked
  and the details pane, a drag in progress (staged over IPC, as CI has no mouse), Your devices, Large icons, Tiles,
  the actions menu) and publishes them to the `ci/desktop-shots-<branch>` branch.

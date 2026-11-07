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

## Settings

View, sort, groups, hidden files, file name extensions and the details pane are kept in `~/.config/nimbus/files.json`
through `hearth-files prefs-set`, which merges under a lock with `hearth_settings_io` (atomic, a .bak kept, never
written over a file it couldn't read; docs/SETTINGS-PERSISTENCE.md). Nothing changes them except you.

## Checks

- `hosts/linux/test/test_hearth_files.py`: untrash (undo of Move to Trash), links, recent files, and that the window
  keeps its buttons and features (`Redesign`).
- The Desktop Mode test takes pictures in `shots/files-redesign/` (dark and light: Home, a folder with three picked
  and the details pane, a drag in progress (staged over IPC, as CI has no mouse), Your devices, Large icons, Tiles,
  the actions menu) and publishes them to the `ci/desktop-shots-<branch>` branch.

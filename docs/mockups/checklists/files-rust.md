# Hearth Files and the Open / Save window in pure Rust (the approved Files and Hearth (C:) mockups, ported from Mantel)

Mockups: next-update/files-1-A-windows11-nav.png, next-update/files-3-A-home-page.png, next-update/files-4-A-narrow.png, next-update/files-5-this-pc.png, next-update/files-6-pictures-details-pane.png, next-update/files-7-right-click.png, next-update/files-8-view-menu.png, next-update/cdrive-1-save-window.png, next-update/cdrive-3-properties.png
Release: next

The pure-Rust Hearth Files (hosts/hearth-ui/apps/files: hearth-files-app, and hearth-file-chooser for every app's Open /
Save window) for Hearth OS, Fedora edition, where Mantel's QML Files can't run. Same backend (desktop/shell/hearth-files),
same settings file (files.json), same mockups as hearth-files.md and c-drive.md. Parts are ticked only once they're
built and pictured in CI (hearth-apps-rust.yml, .github/scripts/hearth-apps-rust-shots.d/files.sh).

## files-1 / files-4 (the window, the left menu)
- [ ] Tabs on top, like Windows 11 (files-6)
- [ ] Back, forward, up, refresh; the address with its steps (This PC › Hearth (C:) › Users › chris); Search chris
- [ ] Command bar: New ▾, cut, copy, paste, rename, share, delete, Sort ▾, View ▾, Details, ···
- [ ] Left menu: Home, Gallery, Home server (Hearth), your folders (pinned), This PC with Hearth (C:) open (Users, Program Files, Windows apps), the other drives, Network, Linux (hidden) at the bottom
- [ ] Narrow left menu (158 px) without the pins and notes; wider shows them
- [ ] Details: Name, Date modified, Type, Size, with Windows' type names (File folder, Python file)
- [ ] Status bar: "9 items" and "C: 96 GB free of 238 GB"

## files-3 (Home)
- [ ] QUICK ACCESS: your folders as tiles, "Pinned" (Saved Games: "Pinned · N games")
- [ ] RECENT: name, Today / Yesterday and the time, the C:\ folder

## files-5 (This PC)
- [ ] DRIVES as tiles with how full they are (the bar turns amber when nearly full) and "96 GB free of 238 GB"
- [ ] NETWORK LOCATIONS: the home server's drive with its letter and bar, Hearth Link devices

## files-6 (Pictures, large icons, details pane)
- [ ] Large icons with the pictures' thumbnails; the picked one outlined
- [ ] Details pane: preview, name, "JPEG image · 2.4 MB", Size (3840 × 2160), Taken, Where (C:\Users\chris\Pictures), Previous versions
- [ ] Details pane: Set as wallpaper · Open with ▾
- [ ] Status bar: "10 items · 1 selected, 2.4 MB"

## files-7 (right-click)
- [ ] The icons on top: cut, copy, rename, copy as path, delete
- [ ] Open (Enter), Open in new tab, Open with ▸, Pin to the left menu, Compress to ZIP, Lock as a Briefcase, Open in Terminal, Restore previous versions, Copy as path (C:\…)
- [ ] Properties (Alt+Enter), Show more options (the whole menu)

## files-8 (View)
- [ ] LAYOUT: Extra large icons, Large icons, List, Details (ticked), Compact view
- [ ] SHOW: Navigation pane, Details pane, File name extensions, Hidden items, Linux folders (/, /home, /usr…), Linux paths in the address bar

## cdrive-1 (every app's Save window)
- [ ] "Save As – Firefox" with the C:\ address on the same row
- [ ] Left menu: Home, Gallery, your folders, This PC with the drives, Network; no Linux folders
- [ ] Name, Date modified, Type, Size with Windows' type names (Disc image, PDF document, Python file)
- [ ] File name box, Cancel, Save
- [ ] hearth-portal opens it where there's no Quickshell, and Plasma's portals.conf sends apps' Open / Save here (KDE's dialog the fallback)

## cdrive-3 (Properties)
- [ ] Type, Location (C:\Users\chris\Desktop), Size, Created
- [ ] Linux path with "only with Show system folders"
- [ ] Copy path → C:\Users\chris\Desktop\Budget.ods

## Mantel's Files beyond the mockups (FileManager.qml, Files*.qml)
- [ ] Terminal panel under the files (F4, FilesTerminal.qml)
- [ ] Split view (F9, FilesSplitPane.qml)
- [ ] Tags and git badges on icons (FilesMarks.qml, FilesExtras.qml), Act as administrator, sharing a folder
- [ ] Briefcase statuses and dialogs (Briefcase.qml); Lock as a Briefcase only makes the Briefcase and adds the folder
- [ ] Drag and drop between windows (iced can't start a drag yet; files dropped on the window are copied in)
- [ ] Your devices' shared folders (net-open, sign in with a sharing password)
- [ ] The controller and Hearth's on-screen keyboard inside the Open / Save window (Game Mode)

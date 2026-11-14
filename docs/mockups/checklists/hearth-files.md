# Hearth Files (approved Nov 11–12, 2026; option A)

Mockups: next-update/files-1-A-windows11-nav.png, next-update/files-2-B-extra-compact-nav.png, next-update/files-3-A-home-page.png, next-update/files-4-A-narrow.png, next-update/files-5-this-pc.png, next-update/files-6-pictures-details-pane.png, next-update/files-7-right-click.png, next-update/files-8-view-menu.png
Release: 1.12

files-2-B was the option not chosen.

- [x] Left menu: Home, Gallery, pinned, This PC with drives, Network, Linux folded at the bottom — hosts/linux/desktop/shell/FileManager.qml: "label: \"Gallery\""
- [x] Home page — hosts/linux/desktop/shell/FileManager.qml: "label: \"Home\""
- [x] This PC: drives with how full they are, Home server — hosts/linux/desktop/shell/FileManager.qml: "label: \"Home server\""
- [x] Tabs and the details pane — hosts/linux/desktop/shell/FileManager.qml: "label: \"New tab\""
- [x] Right-click: Open in new tab — hosts/linux/desktop/shell/FileManager.qml: "label: \"Open in new tab\""
- [x] Right-click: Pin to the left menu — hosts/linux/desktop/shell/FileManager.qml: "label: \"Pin to the left menu\""
- [x] Right-click: Open in Terminal — hosts/linux/desktop/shell/FileManager.qml: "label: \"Open in Terminal\""
- [x] Right-click: Set as wallpaper — hosts/linux/desktop/shell/FileManager.qml: "label: \"Set as wallpaper\""
- [x] Right-click: Compress to ZIP — hosts/linux/desktop/shell/FileManager.qml: "label: \"Compress to ZIP\""
- [x] Right-click: Lock as a Briefcase — hosts/linux/desktop/shell/FileManager.qml: "label: \"Lock as a Briefcase\""
- [x] Right-click: Restore previous versions (needs snapshots of your files) — hosts/linux/desktop/shell/FileManager.qml: "label: \"Restore previous versions\""
- [x] Right-click: Copy as path — hosts/linux/desktop/shell/FileManager.qml: "label: \"Copy as path\""
- [x] Right-click: Show more options — hosts/linux/desktop/shell/FileManager.qml: "label: \"Show more options\""
- [x] Details pane: Previous versions — hosts/linux/desktop/shell/FileManager.qml: "rows.push([\"Previous versions\", String(app.versions.length)])"
- [x] Type names the Windows way (File folder, Python file, PDF document) — hosts/linux/desktop/shell/FileManager.qml: "if (e.dir) return \"File folder\";"
- [x] View: Extra large icons — hosts/linux/desktop/shell/FileManager.qml: "xlarge"
- [x] View: Linux folders and Linux paths in the address bar — hosts/linux/desktop/shell/FileManager.qml: "Linux paths"

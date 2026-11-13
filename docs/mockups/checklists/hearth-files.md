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
- [ ] Right-click: Compress to ZIP
- [ ] Right-click: Lock as a Briefcase
- [ ] Right-click: Restore previous versions (needs snapshots of your files)
- [ ] Right-click: Copy as path
- [ ] Right-click: Show more options
- [ ] Details pane: Previous versions
- [ ] Type names the Windows way (File folder, Python file, PDF document)
- [x] View: Extra large icons — hosts/linux/desktop/shell/FileManager.qml: "xlarge"
- [x] View: Linux folders and Linux paths in the address bar — hosts/linux/desktop/shell/FileManager.qml: "Linux paths"

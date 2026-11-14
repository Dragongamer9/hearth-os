# Hearth (C:) everywhere, step 1 (approved Nov 12, 2026)

Mockups: next-update/cdrive-1-save-window.png, next-update/cdrive-2-run-box.png, next-update/cdrive-3-properties.png, next-update/cdrive-4-settings.png
Release: 1.12

- [x] Every app's Save window shows Hearth (C:) and Windows folder names — hosts/linux/desktop/shell/FileManager.qml: "label: \"Hearth (C:)\""
- [x] Save window: type names the Windows way (File folder, PDF document, Python file) — hosts/linux/desktop/shell/FileManager.qml: "py: \"Python file\", pdf: \"PDF document\""
- [x] Run box: "Type the name of a program, folder, document or website, and Hearth will open it for you." — hosts/linux/desktop/shell/RunView.qml: "Type the name of a program, folder, document or website, and Hearth will open it for you."
- [x] Run box: %USERPROFILE% and other Windows names — hosts/linux/desktop/shell/run-map.js: "function expandVars(t, data)"
- [x] Properties: the C:\ location with the Linux path under it — hosts/linux/desktop/shell/FileManager.qml: "label: \"Copy Linux path\""
- [x] Settings › Drives & folders: Show system folders, Apps you install, Terminal speaks C:\ — hosts/linux/desktop/shell/FileManager.qml: "Linux folders"

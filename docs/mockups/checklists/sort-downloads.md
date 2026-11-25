# Sort my Downloads in Hearth Files (approved Nov 21, 2026: "looks good"; source: hearth-workspace hearth/mockups/1.17-sort-downloads)

Mockups: 1.17-sort-downloads/sort-downloads-1-preview.png, 1.17-sort-downloads/sort-downloads-2-sorted.png
Release: 1.17

The owner (Nov 21): "add that to the files app in the downloads section a auto sort downloads button". The same five
folders as the Nov 15 tidy (hearth-workspace other/assets/downloads-tidy): Hearth, Installers, Media, Documents,
_to_delete. Pictured in CI (hearth-apps-rust 44896710335): files-30-sort-downloads and files-31-sorted, opened
next to the mockups on Nov 21. Only moves, never deletes or overwrites; one Undo puts everything back.

## 1. The preview
- [x] ✦ Sort my Downloads on the command bar's right, only in Downloads (accent) — hosts/hearth-ui/apps/files/src/view.rs: "fn sort_button"
- [x] The status bar: "N items loose in Downloads" — hosts/hearth-ui/apps/files/src/view.rs: "loose in Downloads"
- [x] The dialog over a dimmed window: title, "The N things loose in Downloads go into folders by what they are. Nothing is deleted or overwritten, and one click puts everything back." — hosts/hearth-ui/apps/files/src/sortdl.rs: "go into folders by what they are. Nothing is deleted or overwritten, and one click puts everything back."
- [x] A row per folder, its colour, what goes in it and how many: Hearth, Installers, Media, Documents — hosts/hearth-ui/apps/files/src/sortdl.rs: "AppImages, .rpm, .deb and Flatpaks · disc images in ISOs · archives in Archives"
- [x] To delete (red-tinted): old copies, "kept for you to empty yourself" — hosts/hearth-ui/apps/files/src/sortdl.rs: "kept for you to empty yourself"
- [x] Show every file, Cancel, Sort N items — hosts/hearth-ui/apps/files/src/sortdl.rs: "Show every file"

## 2. Sorted
- [x] The five folders, each with its colour and count — hosts/hearth-ui/apps/files/src/view.rs: "Sort my Downloads' five folders, each in its colour"
- [x] The button back to plain (not accent) once nothing is loose — hosts/hearth-ui/apps/files/src/view.rs: "let lit = crate::sortdl::loose(a) > 0;"
- [x] The status bar: "5 folders · nothing loose" — hosts/hearth-ui/apps/files/src/view.rs: "· nothing loose"
- [x] The note: "✓ Sorted N items into 5 folders." with Undo, and "Ctrl+Z also puts them back" — hosts/hearth-ui/apps/files/src/sortdl.rs: "Ctrl+Z also puts them back"

## 3. In Mantel's Hearth Files (Debian's Desktop Mode; the owner, Nov 25: the same in Mantel's Files)
The rules are hearth_files_sortdl.py's (sortdl.rs's, tests mirrored in hosts/linux/test/test_files_sort_downloads.py).
Pictured in CI (Desktop Mode test, shots/app-blocks): files-sort-1-button, files-sort-2-preview, files-sort-3-every-file,
files-sort-4-sorted, files-sort-5-undone.
- [x] Mantel: ✦ Sort my Downloads on the command bar's right, only in Downloads (accent while anything is loose) — hosts/linux/desktop/shell/FileManager.qml: "primary: cmd.modelData.t === \"sortdl\" && sortDl.loose > 0"
- [x] Mantel: the status bar: "N items loose in Downloads" — hosts/linux/desktop/shell/FileManager.qml: " loose in Downloads\""
- [x] Mantel: the dialog over a dimmed window, title and "The N things loose in Downloads go into folders…" — hosts/linux/desktop/shell/FilesSortSheet.qml: "go into folders by what they are. Nothing is deleted or overwritten, and one click puts everything back."
- [x] Mantel: a row per folder, its colour, what goes in it and how many — hosts/linux/desktop/shell/FilesSortDownloads.qml: "AppImages, .rpm, .deb and Flatpaks · disc images in ISOs · archives in Archives"
- [x] Mantel: To delete (red-tinted): old copies, "kept for you to empty yourself" — hosts/linux/desktop/shell/FilesSortSheet.qml: "· kept for you to empty yourself"
- [x] Mantel: Show every file, Cancel, Sort N items — hosts/linux/desktop/shell/FilesSortSheet.qml: "\"Show every file\", \"Cancel\""
- [x] Mantel: unknown names asked of the AI while the sheet is open (names only), else the rules' place — hosts/linux/desktop/shell/hearth_files_sortdl.py: "'sort-ask', json.dumps({'names': ns})"
- [x] Mantel: the five folders, each with its colour and count — hosts/linux/desktop/shell/FileManager.qml: "tint: sortDl.colourOf(rowItem.modelData)"
- [x] Mantel: the button back to plain once nothing is loose — hosts/linux/desktop/shell/FilesSortDownloads.qml: "if (e.name.charAt(0) !== \".\" && top.indexOf(e.name) < 0) n++;"
- [x] Mantel: the status bar: "5 folders · nothing loose" — hosts/linux/desktop/shell/FileManager.qml: "\" · nothing loose\""
- [x] Mantel: the note "✓ Sorted N items into 5 folders." with Undo, and "Ctrl+Z also puts them back" — hosts/linux/desktop/shell/FilesSortSheet.qml: "Ctrl+Z also puts them back"
- [x] Mantel: one Undo (the note's button or Ctrl+Z) puts everything back; only moves, never overwrites — hosts/linux/desktop/shell/FileManager.qml: "call([isUndo ? \"sort-undo\" : \"sort-redo\", JSON.stringify(op.moves)]"

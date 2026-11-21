# Sort my Downloads in Hearth Files (approved Nov 21, 2026: "looks good"; source: hearth-workspace hearth/mockups/1.17-sort-downloads)

Mockups: 1.17-sort-downloads/sort-downloads-1-preview.png, 1.17-sort-downloads/sort-downloads-2-sorted.png
Release: 1.17

The owner (Nov 21): "add that to the files app in the downloads section a auto sort downloads button". The same five
folders as the Nov 15 tidy (hearth-workspace other/assets/downloads-tidy): Hearth, Installers, Media, Documents,
_to_delete. Only moves, never deletes or overwrites; one Undo puts everything back.

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

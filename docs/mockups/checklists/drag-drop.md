# Drag and drop, everything customizable: the desktop and every app (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/1.18-desktop-drag-drop)

Mockups: drag-drop/drag-drop-1.png, drag-drop/drag-drop-2.png, drag-drop/drag-drop-3.png
Release: next

The owner (Nov 26): "the blocks for the Desktop lay out need to be better, need to be a drag and drop system with 100%
customization", "Desktop Layout needs to be the same way, need 100% custimization instead of suggestions were things
go", "i like drag dop 1, need to expand on it in every app", "as i said i want to be able to create anything i can
think of", then on boards 2 and 3: "yep sounds good". No fixed places: any block anywhere, any size, any look; blocks
you make yourself; Describe it makes one when Hearth hasn't got it. Nothing changes until Keep this; Undo always works;
nothing made here can stop the desktop or an app from starting.

## 1. Editing your real desktop (board 1)
- [ ] Edit desktop opens over the real desktop (not a picture of it): every bar, panel and widget outlined (dashed, accent)
- [ ] A label under each one: what it is and its size ("TOP BAR · 44 px · FULL WIDTH", "LEFT PANEL · FLOATING · DRAG ITS ENDS", "CLOCK WIDGET · ON THE DESKTOP · 420 × 210")
- [ ] The Blocks window, its own window you can move: title "Blocks", "drag onto anything"
- [ ] Find a block (with the count), and kinds: All, Bar items, Widgets, Launchers, Info, Media, Your own, Made by your AI
- [ ] BAR ITEMS: App menu, Open apps, Clock, Quick Access, Downloads, Tray (and every other bar block Mantel has)
- [ ] WIDGETS · ANYWHERE ON THE DESKTOP: Big clock, Weather, Now playing, Friends online (and the rest)
- [ ] MAKE YOUR OWN: Button · runs anything; Text · picture · spacer
- [ ] Drag a block from Blocks onto any bar, any panel or straight onto the desktop
- [ ] Drag a block that's already placed to anywhere else: along its bar, to another bar or panel, onto the desktop, or back
- [ ] The block follows the pointer while dragged (tilted, with a shadow)
- [ ] A lit line where it will land in a bar, and a note saying where ("Drop: top bar, after the clock")
- [ ] Snap guides that line things up, with a note ("Lined up with the clock")
- [ ] Panels anywhere on the screen, not locked to the four sides (the owner, Nov 26: "i want to be able to place pannes ect anywere i want, dont want them locked to just 4 sides"): dragged to any spot, any length and thickness, across or up and down; an edge is only a snap you can use
- [ ] A panel's ends and sides dragged to size it (handles)
- [ ] An empty edge shows "+" while editing; dragging to it makes a new panel there ("Drag here for a panel on the right")
- [ ] Widgets free on the desktop at any place and size (corner handles)
- [ ] A floating panel holds blocks too (icons, "+" to add)
- [ ] The toolbar: Undo, Redo
- [ ] The toolbar: Snap to grid (with its size, 8 px), Guides, Layers, Screen (which screen)
- [ ] The toolbar: + Panel, Start from (a ready-made layout)
- [ ] The toolbar: Cancel, Try it, Keep this
- [ ] "EDITING YOUR DESKTOP · NOTHING IS KEPT UNTIL KEEP THIS · ESC CANCELS"
- [ ] Kept the safe way (merged, atomic, a .bak; nothing saved over a file that can't be read); Game Mode stays in a bar

## 2. One block picked: everything about it (board 2)
- [ ] The picked block lit in its place, with handles
- [ ] Right-click a block: Customize…, Copy its look, Paste a look, Duplicate, Move to › (any bar, panel, the desktop), Hide (keep it here), Take it out
- [ ] "Right-click any block · or pick it and use the panel →"
- [ ] The side panel: the block's name, where it is ("· in the top bar"), Reset
- [ ] Tabs: Look, Shows, Does, Place, When, Motion
- [ ] SIZE AND PLACE: Width (Fit, Fixed, Fill, and its px), Height, Space around it (each side), Space inside
- [ ] BACKGROUND AND EDGE: Fill (colours and any colour), See-through, Blur behind it, Corners, Edge (None, Line, Glow; colour; width), Shadow (None, Soft, Deep)
- [ ] ICON AND WORDS: Icon (pick any; colour; size), Words (with {count}-style values), Font (font, size, Light/Medium/Bold), Show (Icon, Words, Both)
- [ ] WHEN IT'S POINTED AT · PRESSED: Hover look (Same, Lighter, Accent, Own…), Pressed look (Same, Darker, Own…)
- [ ] Shows: what's in it (a count, a list, a picture, a clock…)
- [ ] Does: press, right-click, middle-click, scroll, hover — open, run, show a panel, nothing
- [ ] Place: which bar, panel or the desktop, which screen, which desktops, above or under windows
- [ ] When: always, only on hover, only when there's something to show, only in some apps
- [ ] Motion: how it appears, bounces when something new comes
- [ ] Save as my own block…, Copy its look, Apply to every bar item
- [ ] The bar itself picked (its empty space): Edge Top · Bottom · Left · Right · Floating anywhere
- [ ] The bar's Length (full, fit to its blocks, any % or px) and Offset from the corner
- [ ] The bar's Thickness, Corners, Fill, see-through, blur, Gap between blocks
- [ ] The bar's line-up: start · centre · end · spread · Islands or one strip
- [ ] The bar's Hide until pointed at, Above windows or keep their space, Which screen
- [ ] More than one panel on any edge, panels inside panels
- [ ] The small toolbar while a block is picked: Undo, Redo, Cancel, Try it, Keep this

## 3. The same in every app (board 3, Hearth Files shown)
- [x] The app's window marked EDITING; every part of it outlined (dashed, accent) — hosts/hearth-ui/src/studio.rs: "marks.push(Mark::Dash(inset, p.accent));"
- [x] A label on each part with its size ("COMMAND BAR · 48 px", "SIDEBAR · 216 px · DRAG ITS EDGE") — hosts/hearth-ui/src/studio.rs: "format!(\"{name} · {} px · DRAG ITS EDGE\", r.w.round() as i64)"
- [x] The app's own blocks window ("Files' blocks", "drag into Files"): Find a block (count), All, Buttons, Panes, Info, Your own — hosts/hearth-ui/src/studio.rs: "let kinds = [\"All\", \"Buttons\", \"Panes\", \"Info\", \"Your own\"];"
- [x] BUTTONS (Files: Sort my Downloads, Share, Open in terminal, Compress, and the rest) — hosts/hearth-ui/apps/files/src/view.rs: "tile(\"open-in-terminal\", \"Open in terminal\", G::Keyboard),"
- [x] PANES (Files: Preview, Details, Folder tree, Second folder) — hosts/hearth-ui/apps/files/src/view.rs: "tile(\"tree\", \"Folder tree\", G::Folder), tile(\"second\", \"Second folder\", G::Columns)"
- [x] MAKE YOUR OWN: Button · runs anything; Count · words · spacer — hosts/hearth-ui/src/studio.rs: "[(\"own:button\", \"Button · runs anything\"), (\"own:words\", \"Count · words · spacer\")]"
- [x] A block dragged into a bar of the app, with a lit line where it lands — hosts/hearth-ui/src/studio.rs: "marks.push(Mark::Fill(*line, p.accent));"
- [x] A pane sized by dragging its edge, with a guide and a note ("Preview: 514 px · lined up with Search") — hosts/hearth-ui/src/studio.rs: "format!(\"{title}: {} px · lined up with {n}\", size.round() as i64)"
- [x] The sidebar sized by dragging its edge — hosts/hearth-ui/src/studio.rs: "// where the app puts it: its width (or height) set, the parts beside it make room as they do"
- [x] The same toolbar: Undo, Redo, Snap to grid · 8 px, Guides, Layers, Start from, Menus…, Cancel, Try it, Keep this — hosts/hearth-ui/src/studio.rs: "tbtn(&p, \"Menus…\".into(), st.pop == Pop::Menus, false, Some(s(SMsg::Pop(Pop::Menus)))),"
- [x] Right-click any part › Customize opens board 2's side panel for it — hosts/hearth-ui/src/studio.rs: "let tabs = [\"Look\", \"Shows\", \"Does\", \"Place\", \"When\", \"Motion\"];"
- [x] "THE SAME IN EVERY HEARTH APP · SETTINGS · TASK MANAGER · CAPTURE · NOTES · STORE …" — hosts/hearth-ui/src/studio.rs: "ts(\"THE SAME IN EVERY HEARTH APP · SETTINGS · TASK MANAGER · CAPTURE · NOTES · STORE … · RIGHT-CLICK ANY PART › CUSTOMIZE\""
- [ ] Files (Rust and Mantel's)
- [x] Settings — hosts/hearth-ui/apps/settings/src/main.rs: ".with_pages(STATIC_BLOCKS).with_kit(&frame::KIT)"
- [x] Task Manager — hosts/hearth-ui/apps/task-manager/src/main.rs: ".with_pages(pages::BLOCKS).with_kit(&pages::KIT)"
- [x] Capture — hosts/hearth-ui/apps/capture/src/main.rs: ".with_pages(pages::BLOCKS).with_kit(&pages::KIT)"
- [x] Notes — hosts/hearth-ui/apps/notes/src/main.rs: ".with_pages(side::BLOCKS).with_kit(&side::KIT)"
- [x] Hearth Store — hosts/hearth-ui/apps/store/src/main.rs: ".with_pages(view::BLOCKS).with_kit(&view::KIT)"
- [x] G15, Family, Server mode, Resource monitor, System Restore, Reliability history, Ideas — hosts/hearth-ui/apps/g15/src/app/main.rs: ".with_pages(view::BLOCKS).with_kit(&view::KIT)"

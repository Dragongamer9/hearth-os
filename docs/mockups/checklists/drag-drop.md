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
- [x] Edit desktop opens over the real desktop (not a picture of it): every bar, panel and widget outlined (dashed, accent) — hosts/linux/desktop/shell/DesktopEditor.qml: "WlrLayershell.namespace: \"hearth-desktop-editor\""
- [x] A label under each one: what it is and its size ("TOP BAR · 44 px · FULL WIDTH", "LEFT PANEL · FLOATING · DRAG ITS ENDS", "CLOCK WIDGET · ON THE DESKTOP · 420 × 210") — hosts/linux/desktop/shell/desktop-edit.js: "\" · ON THE DESKTOP · \""
- [x] The Blocks window, its own window you can move: title "Blocks", "drag onto anything" — hosts/linux/desktop/shell/DesktopEditor.qml: "text: \"drag onto anything\""
- [x] Find a block (with the count), and kinds: All, Bar items, Widgets, Launchers, Info, Media, Your own, Made by your AI — hosts/linux/desktop/shell/DesktopEditor.qml: "[\"own\", \"Your own\"], [\"ai\", \"Made by your AI"
- [x] BAR ITEMS: App menu, Open apps, Clock, Quick Access, Downloads, Tray (and every other bar block Mantel has) — hosts/linux/desktop/shell/widget-tools.js: "{ id: \"quickaccess\", name: \"Quick Access\", icon: \"grid\", zone: \"end\", extra: true },"
- [ ] WIDGETS · ANYWHERE ON THE DESKTOP: Big clock, Weather, Now playing, Friends online (and the rest) (open: Hearth has Clock, Weather, Media player and the rest, but no Friends online widget yet, and the clock is named Clock, not Big clock)
- [x] MAKE YOUR OWN: Button · runs anything; Text · picture · spacer — hosts/linux/desktop/shell/DesktopEditor.qml: "LibTile { label: \"Text · picture · spacer\"; icon: \"edit\"; dashed: true; make: \"text\" }"
- [x] Drag a block from Blocks onto any bar, any panel or straight onto the desktop — hosts/linux/desktop/shell/desktop-edit.js: "function drop(draft, id, from, target, widgetInfo, newId) {"
- [x] Drag a block that's already placed to anywhere else: along its bar, to another bar or panel, onto the desktop, or back — hosts/linux/desktop/shell/DesktopEditor.qml: "function startDrag(id, from, p) {"
- [x] The block follows the pointer while dragged (tilted, with a shadow) — hosts/linux/desktop/shell/DesktopEditor.qml: "// ---- the block being dragged: it follows the pointer, tilted, with a shadow"
- [x] A lit line where it will land in a bar, and a note saying where ("Drop: top bar, after the clock") — hosts/linux/desktop/shell/DesktopEditor.qml: "note = \"Drop: \" + mainEdge + \" bar\""
- [x] Snap guides that line things up, with a note ("Lined up with the clock") — hosts/linux/desktop/shell/desktop-edit.js: "note = \"Lined up with \" + (o.name || \"it\");"
- [x] Panels anywhere on the screen, not locked to the four sides (the owner, Nov 26: "i want to be able to place pannes ect anywere i want, dont want them locked to just 4 sides"): dragged to any spot, any length and thickness, across or up and down; an edge is only a snap you can use — hosts/linux/desktop/shell/desktop-edit.js: "function placePanel(rec, r, sw, sh, snapEdges) {"
- [x] A panel's ends and sides dragged to size it (handles) — hosts/linux/desktop/shell/DesktopEditor.qml: "else if (m.mode === \"len1\") { if (down) b.h += dy; else b.w += dx; }"
- [x] An empty edge shows "+" while editing; dragging to it makes a new panel there ("Drag here for a panel on the right") — hosts/linux/desktop/shell/DesktopEditor.qml: "ed.note = \"Drag here for a panel on the \" + parent.modelData"
- [x] Widgets free on the desktop at any place and size (corner handles) — hosts/linux/desktop/shell/DesktopEditor.qml: "else if (m.mode === \"corner\") { b.w += dx; b.h += dy; }"
- [x] A floating panel holds blocks too (icons, "+" to add) — hosts/linux/desktop/shell/DesktopEditor.qml: "ed.note = \"Drag a block from Blocks onto this panel\";"
- [x] The toolbar: Undo, Redo — hosts/linux/desktop/shell/DesktopEditor.qml: "Btn { label: \"Undo\"; icon: \"undo\"; enabled: ed.undoStack.length > 0; onClicked: ed.undo() }"
- [x] The toolbar: Snap to grid (with its size, 8 px), Guides, Layers, Screen (which screen) — hosts/linux/desktop/shell/DesktopEditor.qml: "Btn { label: \"Snap to grid\" + (ed.snapOn ? \" · \" + ed.grid + \" px\" : \"\")"
- [x] The toolbar: + Panel, Start from (a ready-made layout) — hosts/linux/desktop/shell/DesktopEditor.qml: "Btn { label: \"Panel\"; icon: \"plus\"; onClicked: ed.addPanel(\"\") }"
- [x] The toolbar: Cancel, Try it, Keep this — hosts/linux/desktop/shell/DesktopEditor.qml: "Btn { label: \"Try it\"; onClicked: ed.tryIt() }"
- [x] "EDITING YOUR DESKTOP · NOTHING IS KEPT UNTIL KEEP THIS · ESC CANCELS" — hosts/linux/desktop/shell/DesktopEditor.qml: "text: \"EDITING YOUR DESKTOP · NOTHING IS KEPT UNTIL KEEP THIS · ESC CANCELS\""
- [x] Kept the safe way (merged, atomic, a .bak; nothing saved over a file that can't be read); Game Mode stays in a bar — hosts/linux/desktop/shell/DesktopEditor.qml: "if (Object.keys(c.bar).length) BarLayout.put(c.bar);"

## 2. One block picked: everything about it (board 2)
- [x] The picked block lit in its place, with handles — hosts/linux/desktop/shell/DesktopEditor.qml: "// picked: lit in its place, with handles at its ends (drag one to give it a width)"
- [x] Right-click a block: Customize…, Copy its look, Paste a look, Duplicate, Move to › (any bar, panel, the desktop), Hide (keep it here), Take it out — hosts/linux/desktop/shell/DesktopEditor.qml: "MenuRow { label: menuCard.hid ? \"Show it again\" : \"Hide (keep it here)\"; onClicked: { ed.hideBlock(ed.menuBlk); ed.menuBlk = null; } }"
- [x] "Right-click any block · or pick it and use the panel →" — hosts/linux/desktop/shell/DesktopEditor.qml: "text: \"Right-click any block · or pick it and use the panel →\""
- [x] The side panel: the block's name, where it is ("· in the top bar"), Reset — hosts/linux/desktop/shell/DesktopEditor.qml: "label: \"Reset\"; onClicked: { if (side.forBlock) ed.resetLook(); else ed.resetBar(); }"
- [x] Tabs: Look, Shows, Does, Place, When, Motion — hosts/linux/desktop/shell/DesktopEditor.qml: "model: [[\"look\", \"Look\"], [\"shows\", \"Shows\"], [\"does\", \"Does\"], [\"place\", \"Place\"], [\"when\", \"When\"], [\"motion\", \"Motion\"]]"
- [x] SIZE AND PLACE: Width (Fit, Fixed, Fill, and its px), Height, Space around it (each side), Space inside — hosts/linux/desktop/shell/BlockLook.qml: "readonly property real innerW: look.w === \"fixed\" && look.wpx > 0 ? look.wpx : contentW + padIn * 2"
- [x] BACKGROUND AND EDGE: Fill (colours and any colour), See-through, Blur behind it, Corners, Edge (None, Line, Glow; colour; width), Shadow (None, Soft, Deep) — hosts/linux/desktop/shell/BlockLook.qml: "model: bk.look.shadow === \"deep\" ? 3 : bk.look.shadow === \"soft\" ? 2 : 0"
- [x] ICON AND WORDS: Icon (pick any; colour; size), Words (with {count}-style values), Font (font, size, Light/Medium/Bold), Show (Icon, Words, Both) — hosts/linux/desktop/shell/block-look.js: "function fillWords(tpl, v) {"
- [x] WHEN IT'S POINTED AT · PRESSED: Hover look (Same, Lighter, Accent, Own…), Pressed look (Same, Darker, Own…) — hosts/linux/desktop/shell/BlockLook.qml: "readonly property string prs: bk.pressedNow ? BL.pressWash(bk.look) : \"\""
- [x] Shows: what's in it (a count, a list, a picture, a clock…) — hosts/linux/desktop/shell/DesktopEditor.qml: "Choices { options: [[\"own\", \"Its own (what Hearth shows)\"], [\"count\", \"A count\"], [\"list\", \"A list\"], [\"picture\", \"A picture\"], [\"clock\", \"A clock\"], [\"words\", \"Words\"]]"
- [x] Does: press, right-click, middle-click, scroll, hover — open, run, show a panel, nothing — hosts/linux/desktop/shell/BlockLook.qml: "onClicked: m => bk.run(BL.action(bk.look, bk.which(m.button)))"
- [x] Place: which bar, panel or the desktop, which screen, which desktops, above or under windows — hosts/linux/desktop/shell/block-look.js: "if (n.desktops.length && c.desk > 0 && n.desktops.indexOf(c.desk) < 0) return false;"
- [x] When: always, only on hover, only when there's something to show, only in some apps — hosts/linux/desktop/shell/block-look.js: "if (n.when === \"something\" && c.something === false) return false;"
- [x] Motion: how it appears, bounces when something new comes — hosts/linux/desktop/shell/BlockLook.qml: "onCountNowChanged: { if (look.bounce && lastCount >= 0 && countNow > lastCount && !Settings.reduceMotion) bounceAnim.restart(); lastCount = countNow; }"
- [x] Save as my own block…, Copy its look, Apply to every bar item — hosts/linux/desktop/shell/DesktopEditor.qml: "Btn { label: \"Apply to every bar item\"; onClicked: ed.applyAll() }"
- [x] The bar itself picked (its empty space): Edge Top · Bottom · Left · Right · Floating anywhere — hosts/linux/desktop/shell/Islands.qml: "margins.left: free && screen ? Math.round(mb.fx * Math.max(0, screen.width - implicitWidth))"
- [x] The bar's Length (full, fit to its blocks, any % or px) and Offset from the corner — hosts/linux/desktop/shell/widget-tools.js: "function lengthOf(len, lenV, along, want) {"
- [x] The bar's Thickness, Corners, Fill, see-through, blur, Gap between blocks — hosts/linux/desktop/shell/Islands.qml: "rowSpacing: isl.mb.gap >= 0 ? isl.mb.gap : isl.floating ? 10 : 6"
- [x] The bar's line-up: start · centre · end · spread · Islands or one strip — hosts/linux/desktop/shell/ExtraPanels.qml: "model: T.lineup(pw.p.items, pw.p.lineup)"
- [x] The bar's Hide until pointed at, Above windows or keep their space, Which screen — hosts/linux/desktop/shell/ExtraPanels.qml: "exclusiveZone: p.free || p.over || p.autoHide ? 0 : size + gap * 2"
- [x] More than one panel on any edge, panels inside panels — hosts/linux/desktop/shell/NestedPanel.qml: "readonly property bool wanted: !!p && depth <= 3 && pid !== \"\""
- [x] The small toolbar while a block is picked: Undo, Redo, Cancel, Try it, Keep this — hosts/linux/desktop/shell/DesktopEditor.qml: "// ---- the small toolbar while a block is picked: Undo, Redo, Cancel, Try it, Keep this (and while the bar itself is)"

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
- [x] Files (Rust and Mantel's; Mantel's Settings, Task Manager, Capture, Resource monitor, System Restore and Reliability history have it too) — hosts/linux/desktop/shell/FileManager.qml: "{ id: \"terminal\", label: \"Open in terminal\", icon: \"window\" }, { id: \"compress\", label: \"Compress\", icon: \"archive\" },"
- [x] Settings — hosts/hearth-ui/apps/settings/src/main.rs: ".with_pages(STATIC_BLOCKS).with_kit(&frame::KIT)"
- [x] Task Manager — hosts/hearth-ui/apps/task-manager/src/main.rs: ".with_pages(pages::BLOCKS).with_kit(&pages::KIT)"
- [x] Capture — hosts/hearth-ui/apps/capture/src/main.rs: ".with_pages(pages::BLOCKS).with_kit(&pages::KIT)"
- [x] Notes — hosts/hearth-ui/apps/notes/src/main.rs: ".with_pages(side::BLOCKS).with_kit(&side::KIT)"
- [x] Hearth Store — hosts/hearth-ui/apps/store/src/main.rs: ".with_pages(view::BLOCKS).with_kit(&view::KIT)"
- [x] G15, Family, Server mode, Resource monitor, System Restore, Reliability history, Ideas — hosts/hearth-ui/apps/g15/src/app/main.rs: ".with_pages(view::BLOCKS).with_kit(&view::KIT)"

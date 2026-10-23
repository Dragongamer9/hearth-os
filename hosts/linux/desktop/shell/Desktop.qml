// The desktop itself, between the wallpaper and the windows: icons for what's in your Desktop folder (on the
// main screen) and the desktop's right-click menu (every screen). Works like Windows' and KDE's desktop:
//   Icons: click selects, Ctrl+click adds or removes one, Shift+click selects a run of them, drag a box on the
//   empty desktop to select several; double-click opens (a tap on a selected one opens on a touchscreen; A, A on
//   the controller). Drag icons to move them (the places are remembered), onto a folder to move them into it, or
//   out to another app (a file manager, a browser, a chat app). Files dragged in from other apps land on the
//   desktop (moved when they're on the same drive, else copied; on a folder icon, into that folder).
//   Right-click (X, or press and hold) for Open, Open with, Cut, Copy, Rename, Move to Trash, Properties.
//   Keyboard: arrows, Enter opens, F2 renames, Delete moves to the Trash (Shift+Delete deletes for good, after
//   asking), Ctrl+A / C / X / V, F5 looks again, the Menu key opens the menu.
//   Background: right-click for New folder, New text file and your Templates, Paste, Sort by, Line up icons,
//   wallpaper, display settings, Files, and showing or hiding the icons.
// hearth-desk lists the folder and keeps the places (~/.config/nimbus/desk.json) and the clipboard (wl-clipboard,
// text/uri-list, so file managers paste what's copied here and the other way round); hearth-files does the file
// work. Nothing runs while the icons are hidden, and the folder is only looked at again when it changes.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets

PanelWindow {
    id: root
    property bool primary: false           // icons go on the main screen only
    property var menu: null                // PopMenu
    property var dialog: null              // DeskDialog
    property var settingsApp: null
    property var filesApp: null

    WlrLayershell.layer: WlrLayer.Bottom
    WlrLayershell.namespace: "hearth-desktop"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    // ------------------------------------------------ the grid (clear of the taskbar)
    readonly property bool portraitBar: screen && screen.height > screen.width
    readonly property int barSize: 60
    readonly property int cellW: 104
    readonly property int cellH: 112
    readonly property int areaX: portraitBar ? barSize + 10 : 12
    readonly property int areaY: 12
    readonly property int areaW: width - areaX - 12
    readonly property int areaH: height - areaY - (portraitBar ? 12 : barSize + 10)
    readonly property int cols: Math.max(1, Math.floor(areaW / cellW))
    readonly property int rows: Math.max(1, Math.floor(areaH / cellH))

    readonly property bool iconsOn: primary && DeskState.desktopIcons
    readonly property string home: Quickshell.env("HOME") || "/"
    property var entries: []
    property string dir: ""                // the Desktop folder ("" while unknown or set to Home)
    property string selected: ""           // the icon with the keyboard focus (a name in entries)
    property var picked: []                // every selected icon's name (selected is one of them)
    property var cutNames: []              // cut to the clipboard (shown faded until pasted)
    property var templates: []             // ~/Templates, for "New document"
    property string stamp: ""
    property bool dragging: false
    property int seq: 0

    readonly property var current: entries.find(e => e.name === selected) || null
    readonly property var pickedEntries: entries.filter(e => picked.indexOf(e.name) >= 0)

    function refresh() {
        if (!iconsOn || width <= 0) { entries = []; return; }
        const n = ++seq;
        DeskState.call(["icons", String(cols), String(rows)], (r, err) => {
            if (n !== seq || dragging) return;
            if (err || !r) { entries = []; return; }
            dir = r.dir || "";
            entries = r.entries || [];
            picked = picked.filter(p => entries.some(e => e.name === p));
            cutNames = cutNames.filter(p => entries.some(e => e.name === p));
            if (selected && !entries.some(e => e.name === selected)) selected = "";
        });
    }
    Timer { id: soon; interval: 200; onTriggered: root.refresh() }
    onIconsOnChanged: soon.restart()
    onColsChanged: soon.restart()
    onRowsChanged: soon.restart()
    Component.onCompleted: { soon.restart(); if (primary) DeskState.mainDesktop = root; }
    onPrimaryChanged: if (primary) DeskState.mainDesktop = root
    // Sorting or lining up changed (here or in another screen's menu): look again.
    Connections {
        target: DeskState
        function onAutoArrangeChanged() { soon.restart(); }
        function onSortByChanged() { soon.restart(); }
    }

    // The folder changed (a download, a file saved there, something deleted): look again.
    Process {
        id: statProc
        command: ["stat", "-c", "%Y", root.dir]
        stdout: StdioCollector {
            onStreamFinished: {
                const s = text.trim() || "missing";
                if (s !== root.stamp) {
                    const first = root.stamp === "";
                    root.stamp = s;
                    if (!first) root.refresh();
                }
            }
        }
    }
    Timer {
        interval: 3000
        repeat: true
        running: root.iconsOn && root.dir !== ""
        onTriggered: if (!root.dragging && !statProc.running) statProc.running = true
    }

    function say(t) { SettingsNav.toast(t); }

    // ------------------------------------------------ selecting
    function isPicked(name) { return picked.indexOf(name) >= 0; }
    function pickOnly(name) { selected = name; picked = name ? [name] : []; }
    function togglePick(name) {
        selected = name;
        picked = isPicked(name) ? picked.filter(p => p !== name) : picked.concat([name]);
    }
    // Shift+click: everything between the focused icon and this one, in the order the icons go down the columns.
    function pickRange(name) {
        const order = entries.slice().sort((a, b) => a.col - b.col || a.row - b.row).map(e => e.name);
        const a = order.indexOf(selected), b = order.indexOf(name);
        if (a < 0 || b < 0) { pickOnly(name); return; }
        picked = order.slice(Math.min(a, b), Math.max(a, b) + 1);
    }
    function pickAll() { picked = entries.map(e => e.name); if (!selected && entries.length) selected = entries[0].name; }
    function clearPicks() { picked = []; selected = ""; }
    function tileRect(e) { return Qt.rect(areaX + e.col * cellW + 4, areaY + e.row * cellH + 4, cellW - 8, cellH - 8); }

    // ------------------------------------------------ what the icons do
    function fileUrl(p) { return "file://" + p.split("/").map(encodeURIComponent).join("/"); }
    function genericIcon(e) {
        const m = e.mime || "";
        if (e.exec) return "application-x-executable";
        if (m.startsWith("image/")) return "image-x-generic";
        if (m.startsWith("video/")) return "video-x-generic";
        if (m.startsWith("audio/")) return "audio-x-generic";
        if (/zip|7z|rar|tar|xz|zstd|compressed/.test(m)) return "package-x-generic";
        if (/pdf|document|msword|opendocument/.test(m)) return "x-office-document";
        return "text-x-generic";
    }
    function iconFor(e) {
        if (e.launcher) return e.icon && e.icon.startsWith("/") ? fileUrl(e.icon) : Quickshell.iconPath(e.icon || "", "application-x-executable");
        if (e.dir) return Quickshell.iconPath("folder", "inode-directory");
        return Quickshell.iconPath((e.mime || "").replace("/", "-"), genericIcon(e));
    }
    function thumbFor(e) {
        if (e.dir || e.launcher) return "";
        return /^image\/(png|jpeg|gif|webp|bmp|svg)/.test(e.mime || "") && e.size < 25 * 1024 * 1024 ? fileUrl(e.path) : "";
    }
    function openEntry(e) {
        if (!e) return;
        if (e.launcher) DeskState.call(["launch", e.path], (r, err) => { if (err) say(err); });
        else if (e.dir && filesApp) filesApp.open(e.path);
        else DeskState.files(["open", e.path], (r, err) => { if (err) say(err); });
    }
    function openWith(e, appId) {
        DeskState.files(["open", e.path, appId], (r, err) => { if (err) say(err); });
    }
    function rename(e) {
        if (!e || !dialog) return;
        dialog.ask(screen, "Rename “" + (e.label || e.name) + "”", e.name, "Rename", v => {
            if (v === e.name) return;
            DeskState.files(["rename", e.path, v], (r, err) => {
                if (err) return say(err);
                const nn = r && r.path ? r.path.split("/").pop() : v;
                pickOnly(nn);
                // Keep its place under the new name.
                DeskState.call(["place", nn, String(e.col), String(e.row), String(cols), String(rows)], () => refresh());
            });
        });
    }
    function label(list) { return list.length === 1 ? "“" + (list[0].label || list[0].name) + "”" : list.length + " items"; }
    function trash(list) {
        if (!list || !list.length) return;
        DeskState.files(["trash"].concat(list.map(e => e.path)), (r, err) => {
            if (err) return say(err);
            say("Moved " + label(list) + " to the Trash");
            clearPicks();
            refresh();
        });
    }
    // Shift+Delete: gone for good, after asking (Cancel is picked first).
    function deleteForGood(list) {
        if (!list || !list.length || !dialog) return;
        dialog.confirm(screen, "Delete " + label(list) + " for good?", "They won't go to the Trash, so this can't be undone.", "Delete", () => {
            DeskState.files(["delete"].concat(list.map(e => e.path)), (r, err) => {
                if (err) return say(err);
                clearPicks();
                refresh();
            });
        });
    }
    function copy(list, cut) {
        if (!list || !list.length) return;
        DeskState.call(["clip-copy"].concat(cut ? ["--cut"] : []).concat(list.map(e => e.path)), (r, err) => {
            if (err) return say(err);
            cutNames = cut ? list.map(e => e.name) : [];
            say((cut ? "Cut " : "Copied ") + label(list) + (cut ? ". Paste somewhere to move " + (list.length > 1 ? "them" : "it") : ""));
        });
    }
    // Paste: files copied or cut here or in a file manager, or a picture (saved as "Pasted picture.png").
    function paste() {
        if (!dir) return say("There's no Desktop folder to paste into");
        DeskState.call(["clipboard"], (r, err) => {
            if (err) return say(err);
            if (r.paths && r.paths.length) {
                DeskState.files([r.cut ? "move" : "copy"].concat(r.paths).concat([dir]), (r2, err2) => {
                    if (err2) return say(err2);
                    if (r.cut) { DeskState.call(["pasted"]); cutNames = []; }
                    refresh();
                });
            } else if (r.image) {
                DeskState.call(["paste-image"], (r2, err2) => { if (err2) return say(err2); pickOnly(r2.name); refresh(); });
            } else say("There's nothing to paste. Copy files or a picture first");
        });
    }
    function human(n) {
        const u = ["bytes", "KB", "MB", "GB", "TB"];
        let i = 0;
        while (n >= 1024 && i < u.length - 1) { n /= 1024; i++; }
        return (i === 0 ? n : n.toFixed(n < 10 ? 1 : 0)) + " " + u[i];
    }
    function properties(e) {
        // The same Properties window as Hearth Files (FileProperties.qml, through FileManager.qml).
        if (!e || !filesApp) return;
        filesApp.properties([e.path]);
    }
    function newFolder() {
        if (!dialog) return;
        dialog.ask(screen, "New folder on the desktop", "New folder", "Create", v => {
            DeskState.call(["new-folder", v], (r, err) => {
                if (err) return say(err);
                if (!DeskState.desktopIcons) say("Made “" + r.name + "” in your Desktop folder");
                pickOnly(r.name);
                refresh();
            });
        });
    }
    // A new empty text file (or a copy of a template), then its name to change.
    function newFile(template) {
        DeskState.call(["new-file"].concat(template ? [template] : []), (r, err) => {
            if (err) return say(err);
            pickOnly(r.name);
            DeskState.call(["icons", String(cols), String(rows)], (r2, err2) => {
                if (!err2 && r2) { entries = r2.entries || []; rename(entries.find(e => e.name === r.name)); }
            });
        });
    }

    // ------------------------------------------------ dragging (system drag and drop)
    // file:// URIs of what's being dragged (the selection, when the dragged icon is part of it).
    function dragList(name) {
        const list = isPicked(name) ? pickedEntries : entries.filter(e => e.name === name);
        return list;
    }
    function uriList(list) { return list.map(e => fileUrl(e.path)).join("\r\n") + "\r\n"; }
    function localPaths(urls) {
        const out = [];
        for (const u of urls) {
            const s = u.toString();
            if (s.startsWith("file://")) out.push(decodeURIComponent(s.slice(7).replace(/^localhost/, "")));
        }
        return out;
    }
    // Icons dragged within the desktop: to where they were dropped, keeping their places relative to each other.
    function moveIcons(list, lead, x, y) {
        if (DeskState.autoArrange) { say("Icons are kept lined up. Turn that off in the desktop's menu to place them yourself"); return; }
        const col = Math.round((x - areaX - cellW / 2) / cellW), row = Math.round((y - areaY - cellH / 2) / cellH);
        const dc = col - lead.col, dr = row - lead.row;
        if (!dc && !dr) return;
        const args = ["place-many", String(cols), String(rows)];
        for (const e of list) args.push(e.name, String(Math.max(0, Math.min(cols - 1, e.col + dc))), String(Math.max(0, Math.min(rows - 1, e.row + dr))));
        DeskState.call(args, (r, err) => { if (err) say(err); refresh(); });
    }
    // Files from elsewhere (or desktop icons onto a folder): moved when on the same drive, else copied.
    function receive(paths, dest) {
        paths = paths.filter(p => p !== dest && p.substring(0, p.lastIndexOf("/")) !== dest);
        if (!paths.length || !dest) return;
        DeskState.call(["same-device", dest].concat(paths), (r, err) => {
            const op = !err && r && r.move ? "move" : "copy";
            DeskState.files([op].concat(paths).concat([dest]), (r2, err2) => { if (err2) say(err2); refresh(); });
        });
    }
    DropArea {
        anchors.fill: parent
        enabled: root.iconsOn && root.dir !== ""
        onEntered: drag => drag.accept(Qt.CopyAction)
        onDropped: drop => {
            const src = drop.source;
            if (src && src.deskTile && src.deskOwner === root) {
                root.moveIcons(root.dragList(src.entry.name), src.entry, drop.x - src.hotX + src.width / 2, drop.y - src.hotY + src.height / 2);
                drop.accept(Qt.MoveAction);
                return;
            }
            if (!drop.hasUrls) return;
            root.receive(root.localPaths(drop.urls), root.dir);
            drop.accept(Qt.CopyAction);
        }
    }

    // ------------------------------------------------ menus
    function itemMenu(e, x, y) {
        if (!menu || !e) return;
        if (!isPicked(e.name)) pickOnly(e.name);
        else selected = e.name;
        const many = pickedEntries.length > 1 ? pickedEntries : [e];
        const multi = many.length > 1;
        const list = [];
        if (multi) list.push({ label: "Open " + many.length + " items", icon: "open", run: () => many.forEach(x => openEntry(x)) });
        else list.push({ label: "Open", icon: "open", hint: "Enter", run: () => openEntry(e) });
        if (!multi && !e.dir && !e.launcher) list.push({ label: "Open with", icon: "open", sub: [{ label: "Looking…", enabled: false }], loadWith: e });
        if (!multi) list.push({ label: "Show in Files", icon: "folder", run: () => { if (filesApp) filesApp.open(e.path); } });
        list.push({ separator: true });
        list.push({ label: "Cut", icon: "file", hint: "Ctrl+X", run: () => copy(many, true) });
        list.push({ label: "Copy", icon: "clipboard", hint: "Ctrl+C", run: () => copy(many, false) });
        if (!multi) list.push({ label: "Rename", icon: "edit", hint: "F2", run: () => rename(e) });
        list.push({ separator: true });
        list.push({ label: "Move to Trash", icon: "trash", danger: true, hint: "Del", run: () => trash(many) });
        if (!multi) {
            list.push({ separator: true });
            list.push({ label: "Properties", icon: "info", run: () => properties(e) });
        }
        menu.popup(screen, x, y, { title: multi ? many.length + " items" : (e.label || e.name) }, list);
        // "Open with": the apps that can open it (hearth-files apps), filled in once known.
        const w = list.find(it => it.loadWith);
        if (w) DeskState.files(["apps", e.path], (r, err) => {
            const apps = (!err && r && r.apps) ? r.apps.slice(0, 10) : [];
            w.sub = apps.length ? apps.map(a => ({ label: a.name + (a.default ? " (default)" : ""), icon: "open", run: () => openWith(e, a.id) }))
                : [{ label: "No other apps can open this", enabled: false }];
        });
    }
    function backgroundMenu(x, y) {
        if (!menu) return;
        clearPicks();
        const noDir = dir === "" && iconsOn;
        const docs = [{ label: "Empty text file", icon: "file", run: () => newFile("") }];
        for (const t of templates) docs.push({ label: t.name, icon: "file", run: () => newFile(t.path) });
        const sorts = [["name", "Name"], ["type", "Type"], ["size", "Size"], ["date", "Date changed"]].map(s => ({
            label: s[1], checked: DeskState.sortBy === s[0],
            run: () => DeskState.call(["arrange", String(cols), String(rows), s[0]], (r, err) => { if (err) say(err); else DeskState.sortBy = s[0]; refresh(); })
        }));
        const list = [
            { label: "New folder", icon: "plus", hint: "Ctrl+Shift+N", enabled: !noDir, run: () => newFolder() },
            { label: "New document", icon: "file", enabled: !noDir, sub: docs },
            { label: "Paste", icon: "clipboard", hint: "Ctrl+V", enabled: !noDir, run: () => paste() },
            { separator: true }
        ];
        if (iconsOn) {
            list.push({ label: "Sort by", icon: "grid", sub: sorts });
            list.push({ label: "Keep icons lined up", checked: DeskState.autoArrange, run: () => DeskState.setAutoArrange(!DeskState.autoArrange) });
            list.push({ label: "Refresh", icon: "restart", hint: "F5", run: () => refresh() });
        }
        list.push({ label: "Show desktop icons", checked: DeskState.desktopIcons, run: () => DeskState.setDesktopIcons(!DeskState.desktopIcons) });
        list.push({ separator: true });
        list.push({ label: "Change wallpaper", icon: "palette", run: () => { if (settingsApp) settingsApp.open("appearance"); } });
        list.push({ label: "Display settings", icon: "display", run: () => { if (settingsApp) settingsApp.open("display"); } });
        list.push({ label: "Open in Files", icon: "folder", run: () => { if (filesApp) filesApp.open(dir || home); } });
        list.push({ label: "Open a terminal here", icon: "kbd", run: () => Quickshell.execDetached(["sh", "-c", "cd \"$1\" && exec sh \"$2\" terminal", "sh", dir || home, Quickshell.env("NIMBUS_SHARE") + "/desktop/open.sh"]) });
        menu.popup(screen, x, y, { title: "Desktop" }, list);
        // Your templates, for next time the menu opens (cheap: one small folder).
        DeskState.call(["templates"], (r, err) => { if (!err && r) templates = r.templates || []; });
    }

    // ------------------------------------------------ keyboard
    function step(dc, dr, extend) {
        if (!entries.length) return;
        const cur = current;
        if (!cur) { pickOnly(entries.slice().sort((a, b) => a.col - b.col || a.row - b.row)[0].name); return; }
        let best = null, bestD = 1e9;
        for (const e of entries) {
            const ddc = e.col - cur.col, ddr = e.row - cur.row;
            if (e === cur) continue;
            if ((dc && Math.sign(ddc) !== dc) || (dr && Math.sign(ddr) !== dr)) continue;
            const d = dc ? Math.abs(ddc) * 10 + Math.abs(ddr) * 11 : Math.abs(ddr) * 10 + Math.abs(ddc) * 11;
            if (d < bestD) { bestD = d; best = e; }
        }
        if (!best) return;
        if (extend) { selected = best.name; if (!isPicked(best.name)) picked = picked.concat([best.name]); }
        else pickOnly(best.name);
    }
    function cellCenter(e) {
        return Qt.point(areaX + e.col * cellW + cellW / 2, areaY + e.row * cellH + cellH / 2);
    }
    function targets() { return pickedEntries.length ? pickedEntries : current ? [current] : []; }

    Item {
        id: keys
        anchors.fill: parent
        focus: true
        Keys.onPressed: event => {
            const e = root.current;
            const ctrl = event.modifiers & Qt.ControlModifier, shift = event.modifiers & Qt.ShiftModifier;
            if (event.key === Qt.Key_Menu || (event.key === Qt.Key_F10 && shift)) {
                if (e) { const c = root.cellCenter(e); root.itemMenu(e, c.x, c.y); }
                else root.backgroundMenu(root.width / 2, root.height / 2);
            } else if (event.key === Qt.Key_N && ctrl && shift) root.newFolder();
            else if (event.key === Qt.Key_A && ctrl) root.pickAll();
            else if (event.key === Qt.Key_C && ctrl) root.copy(root.targets(), false);
            else if (event.key === Qt.Key_X && ctrl) root.copy(root.targets(), true);
            else if (event.key === Qt.Key_V && ctrl) root.paste();
            else if (event.key === Qt.Key_Delete && shift) root.deleteForGood(root.targets());
            else switch (event.key) {
            case Qt.Key_Left: root.step(-1, 0, shift); break;
            case Qt.Key_Right: root.step(1, 0, shift); break;
            case Qt.Key_Up: root.step(0, -1, shift); break;
            case Qt.Key_Down: root.step(0, 1, shift); break;
            case Qt.Key_Return: case Qt.Key_Enter: root.targets().forEach(x => root.openEntry(x)); break;
            case Qt.Key_F2: root.rename(e); break;
            case Qt.Key_F5: root.refresh(); break;
            case Qt.Key_Delete: root.trash(root.targets()); break;
            case Qt.Key_Escape: root.clearPicks(); break;
            default: return;
            }
            event.accepted = true;
        }
    }

    // ------------------------------------------------ the background: menus and the selection box
    MouseArea {
        id: bg
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        property point start: Qt.point(0, 0)
        property bool banding: false
        property var base: []              // already selected when Ctrl was held
        property rect band: Qt.rect(0, 0, 0, 0)
        onPressed: mouse => {
            keys.forceActiveFocus();
            if (mouse.button !== Qt.LeftButton) return;
            start = Qt.point(mouse.x, mouse.y);
            banding = false;
            base = (mouse.modifiers & Qt.ControlModifier) ? root.picked.slice() : [];
            if (!(mouse.modifiers & Qt.ControlModifier)) root.clearPicks();
        }
        onPositionChanged: mouse => {
            if (!(mouse.buttons & Qt.LeftButton) || !root.iconsOn) return;
            if (!banding && Math.abs(mouse.x - start.x) + Math.abs(mouse.y - start.y) > 8) banding = true;
            if (!banding) return;
            band = Qt.rect(Math.min(start.x, mouse.x), Math.min(start.y, mouse.y), Math.abs(mouse.x - start.x), Math.abs(mouse.y - start.y));
            const hit = root.entries.filter(e => {
                const r = root.tileRect(e);
                return r.x < band.x + band.width && r.x + r.width > band.x && r.y < band.y + band.height && r.y + r.height > band.y;
            }).map(e => e.name);
            root.picked = base.concat(hit.filter(n => base.indexOf(n) < 0));
            if (hit.length) root.selected = hit[hit.length - 1];
        }
        onReleased: banding = false
        onCanceled: banding = false
        onClicked: mouse => { if (mouse.button === Qt.RightButton) root.backgroundMenu(mouse.x, mouse.y); }
        onPressAndHold: mouse => { if (mouse.button === Qt.LeftButton && !banding) root.backgroundMenu(mouse.x, mouse.y); }
    }
    Rectangle {                            // the selection box
        visible: bg.banding
        x: bg.band.x; y: bg.band.y; width: bg.band.width; height: bg.band.height
        color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18)
        border.color: Theme.accent
        border.width: 1
        radius: 4
    }

    // ------------------------------------------------ the icons
    Item {
        anchors.fill: parent
        Repeater {
            model: root.iconsOn ? root.entries : []
            delegate: Item {
                id: tile
                required property var modelData
                readonly property var entry: modelData
                readonly property bool deskTile: true
                readonly property var deskOwner: root
                readonly property bool sel: root.picked.indexOf(modelData.name) >= 0
                readonly property bool focusRing: root.selected === modelData.name && keys.activeFocus
                readonly property string thumb: root.thumbFor(modelData)
                property bool wasSel: false
                property real hotX: width / 2
                property real hotY: height / 2
                property url dragImage: ""
                x: root.areaX + modelData.col * root.cellW + 4
                y: root.areaY + modelData.row * root.cellH + 4
                width: root.cellW - 8
                height: root.cellH - 8
                opacity: hit.drag.active ? 0.5 : root.cutNames.indexOf(modelData.name) >= 0 ? 0.55 : 1

                // System drag and drop: other apps get the files (text/uri-list); the desktop itself gets the drop
                // back (moving the icons) through the DropArea above.
                Drag.dragType: Drag.Automatic
                Drag.supportedActions: Qt.CopyAction | Qt.MoveAction | Qt.LinkAction
                Drag.proposedAction: Qt.MoveAction
                Drag.mimeData: ({ "text/uri-list": root.uriList(root.dragList(modelData.name)) })
                Drag.hotSpot.x: hotX
                Drag.hotSpot.y: hotY
                Drag.imageSource: dragImage
                Drag.active: hit.drag.active
                Drag.onDragFinished: dropAction => { root.dragging = false; soon.restart(); }

                // Folders take files dropped on them (from the desktop or other apps).
                DropArea {
                    anchors.fill: parent
                    enabled: tile.modelData.dir
                    onEntered: drag => { if (drag.source === tile) drag.accepted = false; else drag.accept(Qt.MoveAction); }
                    onDropped: drop => {
                        const src = drop.source;
                        const paths = src && src.deskTile ? root.dragList(src.entry.name).map(e => e.path) : root.localPaths(drop.urls || []);
                        root.receive(paths.filter(p => p !== tile.modelData.path), tile.modelData.path);
                        drop.accept(Qt.MoveAction);
                    }
                    Rectangle {
                        anchors.fill: parent
                        visible: parent.containsDrag
                        radius: 12
                        color: "transparent"
                        border.color: Theme.accent
                        border.width: 2
                    }
                }

                Rectangle {
                    anchors.fill: parent
                    radius: 12
                    color: tile.sel ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.28) : hit.containsMouse ? Qt.rgba(1, 1, 1, 0.08) : "transparent"
                    border.width: tile.sel || tile.focusRing ? 1 : 0
                    border.color: Theme.accent
                }
                Item {
                    id: pic
                    anchors { top: parent.top; topMargin: 8; horizontalCenter: parent.horizontalCenter }
                    width: 52; height: 52
                    IconImage {
                        anchors.centerIn: parent
                        implicitSize: 48
                        visible: tile.thumb === "" || shot.status !== Image.Ready
                        source: root.iconFor(tile.modelData)
                    }
                    Image {
                        id: shot
                        anchors.fill: parent
                        visible: tile.thumb !== "" && status === Image.Ready
                        source: tile.thumb
                        sourceSize: Qt.size(104, 104)
                        fillMode: Image.PreserveAspectFit
                        asynchronous: true
                        smooth: true
                    }
                }
                // The name, readable on any wallpaper: a soft dark copy underneath.
                Text {
                    x: label.x + 1; y: label.y + 1
                    width: label.width
                    text: label.text
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                    color: "#cc000000"
                    font: label.font
                }
                Text {
                    id: label
                    anchors { top: pic.bottom; topMargin: 6; horizontalCenter: parent.horizontalCenter }
                    width: parent.width - 8
                    text: tile.modelData.label || tile.modelData.name
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                    color: "#ffffff"
                    font.family: Theme.font
                    font.pixelSize: 12
                }

                Item { id: ghost }                 // what the pointer drags (the icon itself stays put)
                MouseArea {
                    id: hit
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    drag.target: ghost
                    drag.threshold: 8
                    property bool clickPicks: false    // a press on part of a selection: pick just this one on release
                    onPressed: mouse => {
                        keys.forceActiveFocus();
                        tile.wasSel = tile.sel;
                        clickPicks = false;
                        const name = tile.modelData.name;
                        if (mouse.button === Qt.RightButton) { hit.drag.target = null; return; }
                        if (mouse.modifiers & Qt.ControlModifier) root.togglePick(name);
                        else if (mouse.modifiers & Qt.ShiftModifier) root.pickRange(name);
                        else if (!tile.sel) root.pickOnly(name);
                        else { root.selected = name; clickPicks = root.picked.length > 1; }
                        tile.hotX = mouse.x; tile.hotY = mouse.y;
                        ghost.x = 0; ghost.y = 0;
                        tile.grabToImage(r => tile.dragImage = r.url, Qt.size(tile.width, tile.height));
                    }
                    onReleased: mouse => { hit.drag.target = ghost; if (!hit.drag.active) root.dragging = false; }
                    onClicked: mouse => {
                        if (mouse.button === Qt.RightButton) {
                            const p = tile.mapToItem(null, mouse.x, mouse.y);
                            root.itemMenu(tile.modelData, p.x, p.y);
                            return;
                        }
                        if (mouse.modifiers & (Qt.ControlModifier | Qt.ShiftModifier)) return;
                        if (clickPicks) root.pickOnly(tile.modelData.name);
                        // A touchscreen tap on a selected icon opens it (a mouse needs a double-click).
                        else if (tile.wasSel && mouse.source !== undefined && mouse.source !== Qt.MouseEventNotSynthesized) root.openEntry(tile.modelData);
                    }
                    onDoubleClicked: mouse => { if (mouse.button === Qt.LeftButton) root.openEntry(tile.modelData); }
                    onPressAndHold: mouse => {
                        if (hit.drag.active || mouse.button !== Qt.LeftButton) return;
                        const p = tile.mapToItem(null, mouse.x, mouse.y);
                        root.itemMenu(tile.modelData, p.x, p.y);
                    }
                    Connections {
                        target: hit.drag
                        function onActiveChanged() { if (hit.drag.active) root.dragging = true; }
                    }
                }
            }
        }
    }
}

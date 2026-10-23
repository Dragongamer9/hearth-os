// The desktop itself, between the wallpaper and the windows: icons for what's in your Desktop folder (on the
// main screen) and the desktop's right-click menu (every screen).
//   Icons: click selects, click again or double-click opens (A, A on the controller), drag to move (the place
//   is remembered), right-click (X) for Open, Rename, Move to Trash, Properties. Keyboard: arrows, Enter opens,
//   F2 renames, Delete moves to the Trash, the Menu key opens the menu.
//   Background: right-click (X, or press and hold) for a new folder, wallpaper, display settings, Files, and
//   showing or hiding the icons.
// hearth-desk lists the folder and keeps the places (~/.config/nimbus/desk.json); hearth-files does the file
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
    property string selected: ""           // a name in entries
    property string stamp: ""
    property bool dragging: false
    property int seq: 0

    readonly property var current: entries.find(e => e.name === selected) || null

    function refresh() {
        if (!iconsOn || width <= 0) { entries = []; return; }
        const n = ++seq;
        DeskState.call(["icons", String(cols), String(rows)], (r, err) => {
            if (n !== seq || dragging) return;
            if (err || !r) { entries = []; return; }
            dir = r.dir || "";
            entries = r.entries || [];
            if (selected && !entries.some(e => e.name === selected)) selected = "";
        });
    }
    Timer { id: soon; interval: 200; onTriggered: root.refresh() }
    onIconsOnChanged: soon.restart()
    onColsChanged: soon.restart()
    onRowsChanged: soon.restart()
    Component.onCompleted: { soon.restart(); if (primary) DeskState.mainDesktop = root; }
    onPrimaryChanged: if (primary) DeskState.mainDesktop = root

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
    function rename(e) {
        if (!e || !dialog) return;
        dialog.ask(screen, "Rename “" + (e.label || e.name) + "”", e.name, "Rename", v => {
            if (v === e.name) return;
            DeskState.files(["rename", e.path, v], (r, err) => {
                if (err) return say(err);
                const nn = r && r.path ? r.path.split("/").pop() : v;
                selected = nn;
                // Keep its place under the new name.
                DeskState.call(["place", nn, String(e.col), String(e.row), String(cols), String(rows)], () => refresh());
            });
        });
    }
    function trash(e) {
        if (!e) return;
        DeskState.files(["trash", e.path], (r, err) => {
            if (err) return say(err);
            say("Moved “" + (e.label || e.name) + "” to the Trash");
            refresh();
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
                selected = r.name;
                refresh();
            });
        });
    }

    // ------------------------------------------------ menus
    function itemMenu(e, x, y) {
        if (!menu || !e) return;
        selected = e.name;
        menu.popup(screen, x, y, { title: e.label || e.name }, [
            { label: "Open", icon: "open", run: () => openEntry(e) },
            { label: "Show in Files", icon: "folder", run: () => { if (filesApp) filesApp.open(e.path); } },
            { separator: true },
            { label: "Rename", icon: "edit", run: () => rename(e) },
            { label: "Move to Trash", icon: "trash", danger: true, run: () => trash(e) },
            { separator: true },
            { label: "Properties", icon: "info", run: () => properties(e) }
        ]);
    }
    function backgroundMenu(x, y) {
        if (!menu) return;
        selected = "";
        const list = [
            { label: "New folder", icon: "plus", enabled: dir !== "" || !iconsOn, run: () => newFolder() },
            { separator: true },
            { label: "Change wallpaper", icon: "palette", run: () => { if (settingsApp) settingsApp.open("appearance"); } },
            { label: "Display settings", icon: "display", run: () => { if (settingsApp) settingsApp.open("display"); } },
            { label: "Open Files", icon: "folder", run: () => { if (filesApp) filesApp.open(dir || home); } },
            { separator: true }
        ];
        if (iconsOn && entries.length > 1)
            list.push({ label: "Line up icons by name", icon: "grid", run: () => DeskState.call(["arrange", String(cols), String(rows)], () => refresh()) });
        list.push({ label: "Show desktop icons", checked: DeskState.desktopIcons, run: () => DeskState.setDesktopIcons(!DeskState.desktopIcons) });
        menu.popup(screen, x, y, { title: "Desktop" }, list);
    }

    // ------------------------------------------------ keyboard
    function step(dc, dr) {
        if (!entries.length) return;
        const cur = current;
        if (!cur) { selected = entries.slice().sort((a, b) => a.col - b.col || a.row - b.row)[0].name; return; }
        let best = null, bestD = 1e9;
        for (const e of entries) {
            const ddc = e.col - cur.col, ddr = e.row - cur.row;
            if (e === cur) continue;
            if ((dc && Math.sign(ddc) !== dc) || (dr && Math.sign(ddr) !== dr)) continue;
            const d = dc ? Math.abs(ddc) * 10 + Math.abs(ddr) * 11 : Math.abs(ddr) * 10 + Math.abs(ddc) * 11;
            if (d < bestD) { bestD = d; best = e; }
        }
        if (best) selected = best.name;
    }
    function cellCenter(e) {
        return Qt.point(areaX + e.col * cellW + cellW / 2, areaY + e.row * cellH + cellH / 2);
    }

    Item {
        id: keys
        anchors.fill: parent
        focus: true
        Keys.onPressed: event => {
            const e = root.current;
            if (event.key === Qt.Key_Menu || (event.key === Qt.Key_F10 && (event.modifiers & Qt.ShiftModifier))) {
                if (e) { const c = root.cellCenter(e); root.itemMenu(e, c.x, c.y); }
                else root.backgroundMenu(root.width / 2, root.height / 2);
            } else if (event.key === Qt.Key_N && (event.modifiers & Qt.ControlModifier) && (event.modifiers & Qt.ShiftModifier)) {
                root.newFolder();
            } else switch (event.key) {
            case Qt.Key_Left: root.step(-1, 0); break;
            case Qt.Key_Right: root.step(1, 0); break;
            case Qt.Key_Up: root.step(0, -1); break;
            case Qt.Key_Down: root.step(0, 1); break;
            case Qt.Key_Return: case Qt.Key_Enter: root.openEntry(e); break;
            case Qt.Key_F2: root.rename(e); break;
            case Qt.Key_Delete: root.trash(e); break;
            case Qt.Key_Escape: root.selected = ""; break;
            default: return;
            }
            event.accepted = true;
        }
    }

    // ------------------------------------------------ the background
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onPressed: mouse => { keys.forceActiveFocus(); if (mouse.button === Qt.LeftButton) root.selected = ""; }
        onClicked: mouse => { if (mouse.button === Qt.RightButton) root.backgroundMenu(mouse.x, mouse.y); }
        onPressAndHold: mouse => { if (mouse.button === Qt.LeftButton) root.backgroundMenu(mouse.x, mouse.y); }
    }

    // ------------------------------------------------ the icons
    Item {
        anchors.fill: parent
        Repeater {
            model: root.iconsOn ? root.entries : []
            delegate: Item {
                id: tile
                required property var modelData
                readonly property bool sel: root.selected === modelData.name
                readonly property string thumb: root.thumbFor(modelData)
                property bool wasSel: false
                x: root.areaX + modelData.col * root.cellW + 4
                y: root.areaY + modelData.row * root.cellH + 4
                width: root.cellW - 8
                height: root.cellH - 8
                z: hit.drag.active ? 10 : 1
                opacity: hit.drag.active ? 0.85 : 1

                Rectangle {
                    anchors.fill: parent
                    radius: 12
                    color: tile.sel ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.28) : hit.containsMouse ? Qt.rgba(1, 1, 1, 0.08) : "transparent"
                    border.width: tile.sel ? 1 : 0
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

                MouseArea {
                    id: hit
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    drag.target: tile
                    drag.threshold: 8
                    drag.minimumX: 0
                    drag.minimumY: 0
                    drag.maximumX: root.width - tile.width
                    drag.maximumY: root.height - tile.height
                    onPressed: mouse => {
                        keys.forceActiveFocus();
                        tile.wasSel = tile.sel;
                        root.selected = tile.modelData.name;
                        if (mouse.button === Qt.RightButton) hit.drag.target = null;
                    }
                    onReleased: mouse => {
                        hit.drag.target = tile;
                        if (!root.dragging) return;
                        root.dragging = false;
                        const col = Math.max(0, Math.min(root.cols - 1, Math.round((tile.x - 4 - root.areaX) / root.cellW)));
                        const row = Math.max(0, Math.min(root.rows - 1, Math.round((tile.y - 4 - root.areaY) / root.cellH)));
                        DeskState.call(["place", tile.modelData.name, String(col), String(row), String(root.cols), String(root.rows)], (r, err) => {
                            if (err) root.say(err);
                            root.refresh();
                        });
                    }
                    onClicked: mouse => {
                        if (mouse.button === Qt.RightButton) {
                            const p = tile.mapToItem(null, mouse.x, mouse.y);
                            root.itemMenu(tile.modelData, p.x, p.y);
                        } else if (tile.wasSel) root.openEntry(tile.modelData);
                    }
                    onPressAndHold: mouse => {
                        if (root.dragging || mouse.button !== Qt.LeftButton) return;
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

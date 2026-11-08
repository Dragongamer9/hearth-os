// Hearth Files' This PC page (docs/HEARTH-FILES.md "This PC"), the Windows 11 way: Folders (your XDG folders),
// Devices and drives (Hearth (C:), then each drive with the letter it keeps and the thin space bar, red when nearly
// full) and Network locations (your devices' shared folders, with letters). Grid or Details (View).
// Loaded by FileManager.qml with a Loader, so a problem here never stops Files (or Desktop Mode) from opening; the
// items are FileManager's `entries` (pc: folder | drive | net), so the keyboard and the controller move through them
// as through a folder (FileManager.pcStep walks the groups).
import QtQuick
import QtQuick.Layouts

Item {
    id: page
    property var fm: null
    property var keysItem: null
    readonly property var ents: fm && fm.onThisPc ? fm.entries : []
    readonly property bool details: !!fm && fm.pcView === "details"
    readonly property real gap: 8
    readonly property int cols: details ? 1 : Math.max(1, Math.floor((width - 36 + gap) / (290 + gap)))
    readonly property real tileW: Math.floor((width - 36 - (cols - 1) * gap) / cols)
    onColsChanged: if (fm) fm.pcCols = cols
    onFmChanged: if (fm) fm.pcCols = cols
    Component.onCompleted: if (fm) fm.pcCols = cols

    readonly property var groups: {
        const out = [];
        const defs = [["folder", "Folders"], ["drive", "Devices and drives"], ["net", "Network locations"]];
        for (const d of defs) {
            const items = [];
            for (let i = 0; i < ents.length; i++) if (ents[i].pc === d[0]) items.push({ e: ents[i], i: i });
            if (items.length) out.push({ key: d[0], title: d[1] + " (" + items.length + ")", items: items });
        }
        return out;
    }

    function used(d) { return d && d.total ? Math.max(0, Math.min(1, 1 - (d.free || 0) / d.total)) : 0; }
    function typeName(e) {
        if (e.pc === "folder") return "Folder";
        const d = e.item || {};
        return ({ system: "Local Disk (Hearth)", drive: "Local Disk", games: "Games drive", usb: "USB drive", sd: "SD card", network: "Network location" })[d.kind] || "Drive";
    }
    function spaceLine(d) {
        if (!d) return "";
        if (d.mounted === false) return d.net ? (d.sub || "Not connected") : "Not mounted";
        if (d.total && d.free !== undefined && d.free !== null) return fm.fmtSize(d.free) + " free of " + fm.fmtSize(d.total);
        return d.sub || "";
    }
    function ensure(item) {
        const p = item.mapToItem(col, 0, 0);
        if (p.y < flick.contentY + 4) flick.contentY = Math.max(0, p.y - 40);
        else if (p.y + item.height > flick.contentY + flick.height - 4) flick.contentY = Math.min(Math.max(0, flick.contentHeight - flick.height), p.y + item.height - flick.height + 12);
    }
    function clicked(it, mouse, area) {
        if (keysItem) keysItem.forceActiveFocus();
        fm.zone = "main";
        fm.cur = it.i;
        if (mouse.button === Qt.RightButton) {
            const p = area.mapToItem(keysItem, mouse.x, mouse.y);
            fm.viaPad = false;
            fm.openMenu(false, p.x, p.y);
        } else if (mouse.button === Qt.MiddleButton) {
            if (it.e.path && it.e.path.indexOf("/") === 0) fm.newTab(it.e.path);
        }
    }

    Flickable {
        id: flick
        anchors.fill: parent
        contentHeight: col.implicitHeight + 24
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        Column {
            id: col
            x: 18
            width: page.width - 36
            spacing: 4
            topPadding: 10
            Repeater {
                model: page.groups
                delegate: Column {
                    id: grp
                    required property var modelData
                    width: col.width
                    spacing: 4
                    // "Folders (6)" with a rule, like Windows' group headers.
                    Item {
                        width: parent.width
                        height: 40
                        Row {
                            id: head
                            anchors { left: parent.left; verticalCenter: parent.verticalCenter }
                            spacing: 8
                            Text { anchors.verticalCenter: parent.verticalCenter; text: "⌄"; color: Theme.muted; font.pixelSize: 12 }
                            Text { anchors.verticalCenter: parent.verticalCenter; text: grp.modelData.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                        }
                        Rectangle { anchors { left: head.right; leftMargin: 12; right: parent.right; verticalCenter: parent.verticalCenter } height: 1; color: Theme.line }
                    }
                    // Details: a header row once per group.
                    RowLayout {
                        visible: page.details
                        width: parent.width
                        height: visible ? 28 : 0
                        spacing: 0
                        Text { Layout.fillWidth: true; Layout.leftMargin: 12; text: "Name"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                        Text { Layout.preferredWidth: 170; text: "Type"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                        Text { Layout.preferredWidth: 110; text: grp.modelData.key === "folder" ? "" : "Total size"; horizontalAlignment: Text.AlignRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                        Text { Layout.preferredWidth: 120; Layout.rightMargin: 12; text: grp.modelData.key === "folder" ? "" : "Free space"; horizontalAlignment: Text.AlignRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                    }
                    Flow {
                        width: parent.width
                        spacing: page.details ? 2 : page.gap
                        Repeater {
                            model: grp.modelData.items
                            delegate: Rectangle {
                                id: tile
                                required property var modelData
                                readonly property var e: modelData.e
                                readonly property var d: e.item || ({})
                                readonly property bool isCur: !!page.fm && page.fm.cur === modelData.i
                                readonly property bool lit: isCur && page.fm.zone === "main"
                                readonly property real use: page.used(d)
                                width: page.details ? col.width : page.tileW
                                height: page.details ? 38 : (e.pc === "folder" ? 68 : 80)
                                radius: Theme.r(6)
                                color: isCur ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : tileMouse.containsMouse ? Theme.panel2 : "transparent"
                                border.width: lit ? 2 : isCur ? 1 : 0
                                border.color: lit ? Theme.accent : Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.45)
                                opacity: d.mounted === false || d.offline === true ? 0.72 : 1
                                onIsCurChanged: if (isCur) Qt.callLater(page.ensure, tile)
                                // ---- grid tile
                                RowLayout {
                                    visible: !page.details
                                    anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                                    spacing: 14
                                    WinGlyph { kind: tile.e.pc === "folder" ? (tile.d.kind || "folder") : (tile.d.kind || "drive"); size: 46 }
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 4
                                        Text { Layout.fillWidth: true; text: tile.e.name; color: Theme.text; elide: Text.ElideRight; font.family: Theme.font; font.pixelSize: 14 }
                                        Rectangle {
                                            visible: tile.e.pc !== "folder" && !!tile.d.total && tile.d.mounted !== false
                                            Layout.preferredWidth: Math.min(200, parent.width)
                                            Layout.preferredHeight: 5
                                            radius: 1
                                            color: Theme.panel3
                                            Rectangle {
                                                width: parent.width * tile.use
                                                height: parent.height
                                                radius: 1
                                                color: tile.use > 0.9 ? "#da2626" : Theme.accent
                                            }
                                        }
                                        Text {
                                            Layout.fillWidth: true
                                            text: tile.e.pc === "folder" ? "Your folder" : page.spaceLine(tile.d)
                                            color: Theme.muted; elide: Text.ElideRight; font.family: Theme.font; font.pixelSize: 12
                                        }
                                    }
                                }
                                // ---- details row
                                RowLayout {
                                    visible: page.details
                                    anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                                    spacing: 0
                                    WinGlyph { kind: tile.e.pc === "folder" ? (tile.d.kind || "folder") : (tile.d.kind || "drive"); size: 20; Layout.rightMargin: 10 }
                                    Text { Layout.fillWidth: true; text: tile.e.name; color: Theme.text; elide: Text.ElideRight; font.family: Theme.font; font.pixelSize: 13 }
                                    Text { Layout.preferredWidth: 170; text: page.typeName(tile.e); color: Theme.muted; elide: Text.ElideRight; font.family: Theme.font; font.pixelSize: 12 }
                                    Text { Layout.preferredWidth: 110; text: tile.d.total ? page.fm.fmtSize(tile.d.total) : ""; horizontalAlignment: Text.AlignRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                                    Text {
                                        Layout.preferredWidth: 120
                                        text: tile.d.total && tile.d.free !== undefined && tile.d.free !== null ? page.fm.fmtSize(tile.d.free) : (tile.e.pc === "folder" ? "" : page.spaceLine(tile.d))
                                        horizontalAlignment: Text.AlignRight; color: tile.use > 0.9 ? "#da2626" : Theme.muted; elide: Text.ElideRight; font.family: Theme.font; font.pixelSize: 12
                                    }
                                }
                                MouseArea {
                                    id: tileMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: mouse => page.clicked(tile.modelData, mouse, tileMouse)
                                    onDoubleClicked: mouse => { if (mouse.button === Qt.LeftButton) page.fm.activate(tile.e); }
                                    onPressAndHold: mouse => {
                                        if (page.keysItem) page.keysItem.forceActiveFocus();
                                        page.fm.cur = tile.modelData.i;
                                        const p = tileMouse.mapToItem(page.keysItem, mouse.x, mouse.y);
                                        page.fm.openMenu(false, p.x, p.y);
                                    }
                                }
                            }
                        }
                    }
                    Item { width: 1; height: 6 }
                }
            }
            Text {
                visible: page.groups.length === 0
                topPadding: 40
                width: col.width
                horizontalAlignment: Text.AlignHCenter
                text: page.fm && page.fm.query ? "Nothing here matches “" + page.fm.query + "”" : "Looking at your drives…"
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
            }
        }
    }
}

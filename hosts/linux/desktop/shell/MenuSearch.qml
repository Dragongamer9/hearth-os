// Search results for the app menus (Centred, Tiles, Start screen, Classic): apps, games, Settings pages and files
// in your home folder (Launcher.qml's searchAll), the best match first, in groups; filter chips All / Apps / Games /
// Settings / Files (LB / RB, or Tab); and, when there's room, a preview of the one picked with its actions.
// The style owns the text field and forwards keys: move(d), activate(), options(), nextFilter(d).
import QtQuick
import QtQuick.Layouts
import Quickshell

Item {
    id: ms
    property var menu: null
    property string query: ""
    property string filter: "all"
    property bool preview: width > 760
    property bool chips: true
    property real k: 1                       // bigger for the Start screen
    property int current: 0
    readonly property var result: menu ? menu.searchAll(query, filter) : ({ best: null, apps: [], games: [], pages: [], files: [] })
    readonly property var rows: menu ? menu.flatResults(result) : []
    readonly property var row: rows[Math.min(current, rows.length - 1)] || null
    readonly property var filters: [["all", "All"], ["apps", "Apps"], ["games", "Games"], ["pages", "Settings"], ["files", "Files"]]
    onQueryChanged: { current = 0; if (menu) menu.findFiles(query); }
    onFilterChanged: current = 0

    function move(d) { if (rows.length) { current = Math.max(0, Math.min(rows.length - 1, current + d)); list.positionViewAtIndex(current, ListView.Contain); } }
    function activate() { if (menu && row) menu.openRow(row); }
    function options() { const it = list.itemAtIndex(current); if (menu && row && it) menu.rowMenu(row, it); }
    function nextFilter(d) { const i = filters.findIndex(f => f[0] === filter); filter = filters[(i + d + filters.length) % filters.length][0]; }

    ColumnLayout {
        anchors.fill: parent
        spacing: 12 * ms.k
        RowLayout {
            visible: ms.chips
            Layout.fillWidth: true
            spacing: 8
            Repeater {
                model: ms.filters
                delegate: Rectangle {
                    required property var modelData
                    readonly property bool lit: ms.filter === modelData[0]
                    implicitWidth: chipText.implicitWidth + 26 * ms.k
                    implicitHeight: 32 * ms.k
                    radius: height / 2
                    color: lit ? Theme.accent : chipArea.containsMouse ? Theme.panel3 : Theme.panel2
                    Text { id: chipText; anchors.centerIn: parent; text: modelData[1]; color: parent.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 13 * ms.k; font.weight: parent.lit ? Font.DemiBold : Font.Normal }
                    MouseArea { id: chipArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: ms.filter = modelData[0] }
                }
            }
            Item { Layout.fillWidth: true }
            MenuHints { model: [{ b: "LB", t: "" }, { b: "RB", t: "Filter" }]; k: ms.k }
        }
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 16 * ms.k
            ListView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: ms.rows
                currentIndex: ms.current
                spacing: 2
                boundsBehavior: Flickable.StopAtBounds
                delegate: Item {
                    id: rowItem
                    required property var modelData
                    required property int index
                    readonly property bool lit: index === ms.current
                    readonly property bool head: index === 0 || ms.rows[index - 1].group !== modelData.group
                    width: ListView.view.width
                    implicitHeight: (head ? 26 * ms.k : 0) + 52 * ms.k
                    Text {
                        visible: rowItem.head
                        text: rowItem.modelData.group; leftPadding: 10; topPadding: 6
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 * ms.k; font.weight: Font.DemiBold
                    }
                    Rectangle {
                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                        height: 52 * ms.k
                        radius: Theme.r(10)
                        color: rowItem.lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : rowArea.containsMouse ? Theme.panel2 : "transparent"
                        border.width: rowItem.lit ? 2 : 0
                        border.color: Theme.accent
                        RowLayout {
                            anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                            spacing: 12
                            Item {
                                implicitWidth: 34 * ms.k; implicitHeight: 34 * ms.k
                                AppIcon { anchors.centerIn: parent; visible: rowItem.modelData.kind === "app"; size: 32 * ms.k; icon: rowItem.modelData.kind === "app" ? (rowItem.modelData.item.icon || "") : ""; appId: rowItem.modelData.kind === "app" ? (rowItem.modelData.item.id || "") : "" }
                                Image {
                                    anchors.fill: parent; visible: rowItem.modelData.kind === "game" && status === Image.Ready
                                    fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 80
                                    source: rowItem.modelData.kind === "game" ? (rowItem.modelData.item.icon || rowItem.modelData.item.cover || "") : ""
                                }
                                Rectangle {
                                    anchors.fill: parent; visible: rowItem.modelData.kind === "page" || rowItem.modelData.kind === "file" || (rowItem.modelData.kind === "game" && !rowItem.modelData.item.icon && !rowItem.modelData.item.cover)
                                    radius: Theme.r(8); color: Theme.panel3
                                    Icon { anchors.centerIn: parent; size: 18 * ms.k; name: rowItem.modelData.kind === "page" ? (rowItem.modelData.item.icon || "gear") : rowItem.modelData.kind === "game" ? "pad" : rowItem.modelData.item.dir ? "folder" : "file" }
                                }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1
                                Text { Layout.fillWidth: true; text: ms.menu ? ms.menu.rowTitle(rowItem.modelData) : ""; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 * ms.k; font.weight: Font.DemiBold }
                                Text { Layout.fillWidth: true; text: ms.menu ? ms.menu.rowSub(rowItem.modelData) : ""; elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 * ms.k }
                            }
                        }
                        MouseArea {
                            id: rowArea
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            cursorShape: Qt.PointingHandCursor
                            onClicked: mouse => { ms.current = rowItem.index; if (mouse.button === Qt.RightButton) ms.options(); else ms.activate(); }
                        }
                    }
                }
                Text {
                    anchors.centerIn: parent
                    width: parent.width - 40
                    visible: ms.rows.length === 0
                    horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap
                    text: ms.query.trim() === "" ? "Type to search apps, games, Settings and files" : "Nothing matches \"" + ms.query + "\""
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 * ms.k
                }
            }
            // The preview: the one picked, bigger, with what you can do with it.
            Rectangle {
                visible: ms.preview && ms.row !== null
                Layout.preferredWidth: Math.min(340 * ms.k, ms.width * 0.38)
                Layout.fillHeight: true
                radius: Theme.r(14)
                color: Qt.rgba(Theme.panel2.r, Theme.panel2.g, Theme.panel2.b, 0.7)
                border.color: Theme.line
                ColumnLayout {
                    anchors { fill: parent; margins: 20 }
                    spacing: 10
                    Item {
                        Layout.alignment: Qt.AlignHCenter
                        implicitWidth: 72 * ms.k; implicitHeight: 72 * ms.k
                        AppIcon { anchors.centerIn: parent; visible: !!ms.row && ms.row.kind === "app"; size: 68 * ms.k; icon: ms.row && ms.row.kind === "app" ? (ms.row.item.icon || "") : ""; appId: ms.row && ms.row.kind === "app" ? (ms.row.item.id || "") : "" }
                        Rectangle {
                            anchors.fill: parent; visible: !!ms.row && ms.row.kind !== "app"; radius: Theme.r(16); color: Theme.panel3; clip: true
                            Image { anchors.fill: parent; visible: status === Image.Ready; fillMode: Image.PreserveAspectCrop; asynchronous: true; source: ms.row && ms.row.kind === "game" ? (ms.row.item.cover || ms.row.item.icon || "") : "" }
                            Icon { anchors.centerIn: parent; size: 30 * ms.k; visible: !ms.row || ms.row.kind !== "game"; name: !ms.row ? "file" : ms.row.kind === "page" ? (ms.row.item.icon || "gear") : ms.row.item.dir ? "folder" : "file" }
                        }
                    }
                    Text { Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; text: ms.menu ? ms.menu.rowTitle(ms.row) : ""; wrapMode: Text.Wrap; color: Theme.text; font.family: Theme.font; font.pixelSize: 19 * ms.k; font.weight: Font.DemiBold }
                    Text { Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; text: ms.menu ? ms.menu.rowSub(ms.row) : ""; wrapMode: Text.Wrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 * ms.k }
                    Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.line }
                    Repeater {
                        model: {
                            const r = ms.row;
                            if (!r || !ms.menu) return [];
                            const out = [{ label: r.kind === "game" ? "Play" : "Open", icon: r.kind === "game" ? "pad" : "open", run: () => ms.menu.openRow(r) }];
                            if (r.kind === "app") for (const j of ms.menu.jumpList(r.item).slice(0, 5)) out.push(j);
                            if (r.kind === "app") out.push({ label: DeskState.isPinned(r.item.id) ? "Unpin" : "Pin to the bar", icon: DeskState.isPinned(r.item.id) ? "unpin" : "pin", run: () => DeskState.togglePin(r.item.id) });
                            if (r.kind === "file") out.push({ label: "Open the folder it's in", icon: "folder", run: () => ms.menu.openFolder(r.item.path.replace(/\/[^/]*$/, "") || "/") });
                            return out;
                        }
                        delegate: Rectangle {
                            required property var modelData
                            Layout.fillWidth: true
                            implicitHeight: 38 * ms.k
                            radius: Theme.r(8)
                            color: actArea.containsMouse ? Theme.panel3 : "transparent"
                            RowLayout {
                                anchors { fill: parent; leftMargin: 10 }
                                spacing: 10
                                Icon { name: modelData.icon || "open"; size: 16 * ms.k; color: Theme.accent }
                                Text { Layout.fillWidth: true; text: modelData.label; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 * ms.k }
                            }
                            MouseArea { id: actArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: modelData.run() }
                        }
                    }
                    Item { Layout.fillHeight: true }
                }
            }
        }
    }
}

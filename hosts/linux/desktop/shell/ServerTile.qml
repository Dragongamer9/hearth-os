// Server mode (docs/SERVER-MODE.md): one service's tile on the dashboard (server-mode.qml, ServerDash.qml). Draw-only:
// everything it shows comes from `hearth-server-mode dash` (a tile: title, engine, icon, chip, big + unit, sub, rows,
// bar, boxes, list). Never changes a setting.
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: tile
    property var t: ({})
    property bool focused: false
    property real k: 1
    // Server mode's gold look (the Hearth AI Command Center's): ServerDash passes gold: true.
    property bool gold: false
    readonly property color cCard: gold ? "#140c03" : SetStyle.card
    readonly property color cLine: gold ? "#4a3212" : Theme.line
    readonly property color cText: gold ? "#fff4df" : Theme.text
    readonly property color cMuted: gold ? "#c9a372" : Theme.muted
    readonly property color cAcc: gold ? "#ffb347" : Theme.accent
    readonly property color cOk: gold ? "#6fe39a" : SetStyle.ok
    readonly property color cWarn: gold ? "#ff8a4c" : SetStyle.warn
    readonly property color cSunk: gold ? "#0c0702" : SetStyle.sunk
    readonly property bool warn: t.state === "warn" || t.state === "stopped"
    readonly property bool off: t.state === "off"
    readonly property bool paused: t.state === "paused" || t.state === "paused-you"
    radius: (gold ? 14 : 18) * k
    color: tile.cCard
    border.width: focused ? 3 * k : 1
    border.color: focused ? tile.cAcc : tile.cLine
    opacity: off ? 0.62 : 1

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 22 * k
        spacing: 8 * k
        RowLayout {
            Layout.fillWidth: true
            spacing: 14 * k
            Rectangle {
                width: 40 * k; height: 40 * k; implicitWidth: 40 * k; implicitHeight: 40 * k; radius: 10 * k
                color: Qt.rgba(tile.cAcc.r, tile.cAcc.g, tile.cAcc.b, 0.16)
                Icon { anchors.centerIn: parent; name: tile.t.icon || "dot"; size: 22 * tile.k; color: Theme.accent }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0
                Text { text: tile.t.title || ""; color: tile.cText; font.family: Theme.font; font.pixelSize: 18 * tile.k; font.weight: Font.DemiBold; elide: Text.ElideRight; Layout.fillWidth: true }
                Text { text: tile.t.engine || ""; color: tile.cMuted; font.family: Theme.font; font.pixelSize: 13 * tile.k; elide: Text.ElideRight; Layout.fillWidth: true }
            }
            Rectangle {
                visible: (tile.t.chip || "") !== ""
                implicitWidth: chipText.implicitWidth + 18 * tile.k; height: 26 * tile.k; radius: 8 * tile.k
                readonly property color c: tile.warn ? tile.cWarn : tile.paused ? tile.cWarn : tile.off ? tile.cMuted : tile.cOk
                color: Qt.rgba(c.r, c.g, c.b, 0.16)
                Text { id: chipText; anchors.centerIn: parent; text: tile.t.chip || ""; color: parent.c; font.family: Theme.font; font.pixelSize: 12.5 * tile.k; font.weight: Font.DemiBold }
            }
        }
        // The big number and its unit ("4 questions a minute", "2.6 TB of 8 TB used").
        Row {
            visible: (tile.t.big || "") !== "" && !tile.t.boxes
            Layout.topMargin: 8 * tile.k
            spacing: 10 * tile.k
            Text { id: bigT; text: tile.t.big || ""; color: tile.cText; font.family: Theme.font; font.pixelSize: 34 * tile.k; font.weight: Font.DemiBold }
            Text { anchors.baseline: bigT.baseline; text: tile.t.unit || ""; color: tile.cMuted; font.family: Theme.font; font.pixelSize: 18 * tile.k }
        }
        Text {
            visible: (tile.t.sub || "") !== ""
            Layout.fillWidth: true
            text: tile.t.sub || ""; color: tile.cMuted; font.family: Theme.font; font.pixelSize: 14 * tile.k
            wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight
        }
        // Who's connected: a dot per device.
        Repeater {
            model: tile.t.list || []
            delegate: RowLayout {
                required property var modelData
                Layout.fillWidth: true
                spacing: 10 * tile.k
                Rectangle { width: 8 * tile.k; height: width; implicitWidth: 8 * tile.k; implicitHeight: 8 * tile.k; radius: width / 2; color: tile.cOk }
                ColumnLayout {
                    spacing: 0
                    Layout.fillWidth: true
                    Text { text: modelData[0]; color: tile.cText; font.family: Theme.font; font.pixelSize: 14.5 * tile.k; elide: Text.ElideRight; Layout.fillWidth: true }
                    Text { text: modelData[1]; color: tile.cMuted; font.family: Theme.font; font.pixelSize: 12.5 * tile.k; elide: Text.ElideRight; Layout.fillWidth: true }
                }
            }
        }
        // Heat & power: four small boxes.
        GridLayout {
            visible: !!tile.t.boxes
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.topMargin: 6 * tile.k
            columns: 2; rowSpacing: 10 * tile.k; columnSpacing: 10 * tile.k
            Repeater {
                model: tile.t.boxes || []
                delegate: Rectangle {
                    required property var modelData
                    Layout.fillWidth: true; Layout.fillHeight: true
                    radius: 10 * tile.k; color: tile.cSunk
                    Column {
                        anchors { left: parent.left; leftMargin: 14 * tile.k; verticalCenter: parent.verticalCenter }
                        spacing: 2 * tile.k
                        Text { text: modelData[0]; color: tile.cMuted; font.family: Theme.font; font.pixelSize: 13 * tile.k }
                        Text { text: modelData[1]; color: tile.cText; font.family: Theme.font; font.pixelSize: 24 * tile.k; font.weight: Font.DemiBold }
                        Text { text: modelData[2]; color: tile.cMuted; font.family: Theme.font; font.pixelSize: 12.5 * tile.k; visible: modelData[2] !== "" }
                    }
                }
            }
        }
        Item { Layout.fillHeight: true; visible: !tile.t.boxes }
        // A bar (storage used) and the rows at the bottom.
        Rectangle {
            visible: tile.t.bar !== undefined && tile.t.bar !== null
            Layout.fillWidth: true
            height: 8 * tile.k; radius: 4 * tile.k; color: tile.cSunk
            Rectangle { width: parent.width * Math.min(100, tile.t.bar || 0) / 100; height: parent.height; radius: parent.radius; color: tile.cAcc }
        }
        Repeater {
            model: tile.t.rows || []
            delegate: RowLayout {
                required property var modelData
                Layout.fillWidth: true
                Text { text: modelData[0]; color: tile.cMuted; font.family: Theme.font; font.pixelSize: 14 * tile.k; elide: Text.ElideMiddle; Layout.fillWidth: true }
                Text { text: modelData[1]; color: tile.cText; font.family: Theme.font; font.pixelSize: 14 * tile.k; font.weight: Font.DemiBold; elide: Text.ElideRight; Layout.maximumWidth: tile.width * 0.6 }
            }
        }
    }
}

// Server mode: one line of the switch countdown's lists ("Pictures stop", "Shared folders"): an icon in its tone, a title
// and a short explanation. Draw-only.
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: line
    property real k: 1
    property string icon: "dot"
    property string title: ""
    property string sub: ""
    property color tone: Theme.accent
    implicitHeight: 64 * k
    radius: 12 * k
    color: SetStyle.card
    RowLayout {
        anchors.fill: parent; anchors.leftMargin: 14 * line.k; anchors.rightMargin: 14 * line.k
        spacing: 14 * line.k
        Rectangle { width: 40 * line.k; height: width; implicitWidth: 40 * line.k; implicitHeight: 40 * line.k; radius: 10 * line.k; color: Qt.rgba(line.tone.r, line.tone.g, line.tone.b, 0.15)
            Icon { anchors.centerIn: parent; name: line.icon; size: 22 * line.k; color: line.tone } }
        ColumnLayout { Layout.fillWidth: true; spacing: 1
            Text { text: line.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 16 * line.k; font.weight: Font.DemiBold; elide: Text.ElideRight; Layout.fillWidth: true }
            Text { text: line.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13.5 * line.k; elide: Text.ElideRight; Layout.fillWidth: true } }
    }
}

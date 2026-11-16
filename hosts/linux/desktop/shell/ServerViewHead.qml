// Server mode: the top of an opened tile's page: its icon, title, the line under it, the On / Off chip, and the
// controller's focus ring when the feature itself is picked (A turns it on or off). Draw-only.
import QtQuick
import QtQuick.Layouts

Item {
    id: h
    property real k: 1
    property string icon: "dot"
    property string title: ""
    property string sub: ""
    property string chip: ""
    property color chipColor: "#6fe39a"
    property bool picked: false
    Layout.fillWidth: true
    implicitHeight: row.implicitHeight
    Rectangle { anchors.fill: parent; anchors.margins: -8 * h.k; visible: h.picked; color: "transparent"; border.color: Theme.accent; border.width: 2.5 * h.k; radius: 12 * h.k }
    RowLayout {
        id: row
        anchors { left: parent.left; right: parent.right; top: parent.top }
        spacing: 18 * h.k
        Rectangle {
            implicitWidth: 58 * h.k; implicitHeight: 58 * h.k; radius: 12 * h.k
            color: "#3a2614"
            Icon { anchors.centerIn: parent; name: h.icon; size: 30 * h.k; color: "#ff8c42" }
        }
        ColumnLayout {
            spacing: 2
            Layout.fillWidth: true
            Text { text: h.title; color: "#fff4df"; font.family: Theme.font; font.pixelSize: 38 * h.k; font.weight: Font.DemiBold }
            Text { Layout.fillWidth: true; text: h.sub; color: "#c9b79f"; font.family: Theme.font; font.pixelSize: 18 * h.k; elide: Text.ElideRight }
        }
        Rectangle {
            Layout.alignment: Qt.AlignTop
            visible: h.chip !== ""
            implicitWidth: chipT.implicitWidth + 28 * h.k; implicitHeight: 38 * h.k; radius: 8 * h.k
            color: Qt.rgba(h.chipColor.r, h.chipColor.g, h.chipColor.b, 0.16)
            Text { id: chipT; anchors.centerIn: parent; text: h.chip; color: h.chipColor; font.family: Theme.font; font.pixelSize: 19 * h.k }
        }
    }
}

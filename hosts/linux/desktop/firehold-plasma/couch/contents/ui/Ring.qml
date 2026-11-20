// The controller's focus (the mockups' .focus): a 4 px accent ring 4 px outside the item, and its soft glow.
// Put it next to the item (not inside a clipped one): it draws outside its parent.
import QtQuick

Item {
    id: ring
    property real k: 1
    property real r: 14            // the item's corner radius
    property real gap: 4
    property color color: "#ff7a2f"
    anchors.fill: parent
    Repeater {
        model: [[22, 0.05], [17, 0.10], [12, 0.20]]
        delegate: Rectangle {
            required property var modelData
            anchors.fill: parent
            anchors.margins: -modelData[0] * ring.k
            radius: (ring.r + modelData[0]) * ring.k
            color: "transparent"
            border.width: 6 * ring.k
            border.color: Qt.rgba(1, 122 / 255, 47 / 255, modelData[1])
        }
    }
    Rectangle {
        anchors.fill: parent
        anchors.margins: -(ring.gap + 4) * ring.k
        radius: (ring.r + ring.gap + 4) * ring.k
        color: "transparent"
        border.width: 4 * ring.k
        border.color: ring.color
    }
}

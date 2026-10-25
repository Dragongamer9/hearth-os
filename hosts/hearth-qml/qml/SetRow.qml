// A line of Settings: a title, what it does, and its value (› opens its choices).
import QtQuick

Btn {
    kind: "menu"
    property bool card: true
    Rectangle {
        anchors.fill: parent
        z: -2
        radius: parent.radius
        color: parent.card && !(parent.isFocused || parent.hovered) ? Theme.panel : "transparent"
    }
}

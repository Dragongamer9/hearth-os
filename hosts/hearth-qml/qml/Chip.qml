// A small choice in a row of choices (a frame-rate limit, a library filter); `on` when it's the current one.
import QtQuick

Focusable {
    id: c
    property string text: ""
    property bool on: false
    radius: height / 2
    implicitHeight: Theme.u(34)
    implicitWidth: label.implicitWidth + Theme.u(28)
    Rectangle {
        anchors.fill: parent
        radius: height / 2
        color: c.on ? Theme.accent : (c.isFocused || c.hovered ? Theme.panel3 : Theme.panel2)
        Behavior on color { ColorAnimation { duration: Theme.quick } }
    }
    Text {
        id: label
        anchors.centerIn: parent
        text: c.text
        color: c.on ? Theme.accentInk : Theme.text
        font.family: Theme.font
        font.pixelSize: Theme.u(13)
        font.weight: Font.DemiBold
    }
}

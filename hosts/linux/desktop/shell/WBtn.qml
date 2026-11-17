// A small button for the widgets: text and/or one of Hearth's icons, lit when on, a focus ring for the keyboard
// and the controller (Tab / the D-pad in a widget's settings).
import QtQuick

Rectangle {
    id: b
    property string text: ""
    property string icon: ""
    property bool on: false
    property bool flat: false
    property real size: 30
    property color ink: on ? Theme.accentInk : Theme.text
    signal clicked()
    activeFocusOnTab: true
    implicitHeight: size
    implicitWidth: Math.max(size, row.implicitWidth + (text ? 20 : 0))
    radius: Math.min(height / 2, Theme.r(10))
    color: on ? Theme.accent : ma.containsMouse || activeFocus ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.14)
        : flat ? "transparent" : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.07)
    border.width: activeFocus ? Theme.focusWidth : 0
    border.color: Theme.accent
    opacity: enabled ? 1 : 0.4
    Row {
        id: row
        anchors.centerIn: parent
        spacing: 6
        Icon { visible: b.icon !== ""; name: b.icon; size: b.size * 0.55; color: b.ink; anchors.verticalCenter: parent.verticalCenter }
        Text { visible: b.text !== ""; text: b.text; color: b.ink; font.family: Theme.font; font.pixelSize: Math.round(b.size * 0.45); font.weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter }
    }
    MouseArea { id: ma; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: b.clicked() }
    Keys.onReturnPressed: b.clicked()
    Keys.onEnterPressed: b.clicked()
    Keys.onSpacePressed: b.clicked()
}

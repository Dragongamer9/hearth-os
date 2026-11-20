// Firehold's Quick settings tile (style "firehold", mockups 07-08): its round icon, name and state; lit when on.
import QtQuick
import "fh.js" as F

Rectangle {
    id: t
    property string glyph: ""
    property string title: ""
    property string sub: ""
    property bool on: false
    property bool focused: false
    signal pressed()
    function activate() { pressed(); }
    radius: 12
    color: on ? F.tileOn : F.tile
    border.width: focused ? 2 : 0
    border.color: F.accent
    implicitHeight: 64
    Rectangle {
        id: dotR
        x: 16; anchors.verticalCenter: parent.verticalCenter
        width: 36; height: 36; radius: 18
        color: t.on ? F.accent : F.dot
        Icon { anchors.centerIn: parent; name: t.glyph; size: 18; color: "#ffffff" }
    }
    Column {
        anchors { left: dotR.right; leftMargin: 14; right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter }
        spacing: 2
        Text { width: parent.width; text: t.title; color: F.text; font.pixelSize: 16; font.weight: Font.DemiBold; font.family: F.font(Qt.fontFamilies()); elide: Text.ElideRight }
        Text { width: parent.width; text: t.sub; color: F.muted; font.pixelSize: 13; font.family: F.font(Qt.fontFamilies()); elide: Text.ElideRight; visible: text !== "" }
    }
    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: t.pressed() }
}

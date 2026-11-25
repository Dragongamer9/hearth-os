// "Edit layout", always findable in an app's own bar (the owner: never buried in a menu): a small button with the
// blocks' picture; its name shows on hover. A click turns the app into Edit layout in place (AppBlocks.qml); a
// right-click gives Describe it… and Back to the original too.
import QtQuick

Rectangle {
    id: eb
    property var blocks: null
    implicitWidth: hov.containsMouse ? row.implicitWidth + 20 : 30
    implicitHeight: 30
    width: implicitWidth; height: implicitHeight
    radius: 15
    visible: !!blocks
    color: hov.containsMouse || (blocks && blocks.editing) ? Theme.panel3 : "transparent"
    border.color: blocks && blocks.editing ? Theme.accent : hov.containsMouse ? Theme.line : "transparent"
    Behavior on implicitWidth { NumberAnimation { duration: Theme.quick } }
    Row {
        id: row
        anchors { left: parent.left; leftMargin: 7; verticalCenter: parent.verticalCenter }
        spacing: 6
        Icon { name: "grid"; size: 16; color: eb.blocks && eb.blocks.editing ? Theme.accent : Theme.muted; anchors.verticalCenter: parent.verticalCenter }
        Text { visible: hov.containsMouse; text: "Edit layout"; color: Theme.text; font.family: Theme.font; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter }
    }
    MouseArea {
        id: hov
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        cursorShape: Qt.PointingHandCursor
        onClicked: m => {
            if (!eb.blocks) return;
            if (m.button === Qt.RightButton) { const q = mapToItem(eb.blocks, m.x, m.y); eb.blocks.partMenu(eb.blocks.defs.length ? eb.blocks.defs[0].id : "", q.x, q.y); }
            else if (eb.blocks.editing) eb.blocks.done();
            else eb.blocks.edit("");
        }
    }
}

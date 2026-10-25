// A short notice at the bottom of the screen (the core's toasts, and the screens' own).
import QtQuick

Rectangle {
    id: toast
    property string text: ""
    property int seq: 0
    anchors.horizontalCenter: parent.horizontalCenter
    y: parent.height - height - Theme.u(70) + (shown ? 0 : Theme.u(20))
    width: Math.min(parent.width - Theme.u(60), label.implicitWidth + Theme.u(40))
    height: label.implicitHeight + Theme.u(24)
    radius: Theme.u(12)
    color: Theme.panel3
    border.color: Theme.line
    opacity: shown ? 1 : 0
    visible: opacity > 0
    z: 100
    property bool shown: false
    Behavior on opacity { NumberAnimation { duration: Theme.calm } }
    Behavior on y { NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
    onSeqChanged: { if (text) { shown = true; hide.restart(); } }
    Timer { id: hide; interval: 4200; onTriggered: toast.shown = false }
    Text {
        id: label
        anchors.centerIn: parent
        width: Math.min(implicitWidth, toast.parent ? toast.parent.width - Theme.u(100) : 600)
        text: toast.text
        color: Theme.text
        wrapMode: Text.WordWrap
        horizontalAlignment: Text.AlignHCenter
        font.family: Theme.font
        font.pixelSize: Theme.u(14)
    }
}

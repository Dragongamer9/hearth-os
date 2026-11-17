// A one-line text box for the widgets' settings (a place, a feed's address, a note).
import QtQuick

Rectangle {
    id: f
    property alias text: input.text
    property string placeholder: ""
    property alias input: input
    property real size: 32
    property bool grab: false              // takes the keyboard when it's shown (a widget's settings opened)
    signal accepted()
    signal upPressed()
    signal downPressed()
    onGrabChanged: if (grab) input.forceActiveFocus()
    implicitHeight: size
    implicitWidth: 200
    radius: Theme.r(8)
    color: Qt.rgba(Theme.bg.r, Theme.bg.g, Theme.bg.b, 0.55)
    border.width: input.activeFocus ? Theme.focusWidth : 1
    border.color: input.activeFocus ? Theme.accent : Theme.line
    TextInput {
        id: input
        anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
        verticalAlignment: TextInput.AlignVCenter
        color: Theme.text
        selectionColor: Theme.accent
        selectedTextColor: Theme.accentInk
        font.family: Theme.font
        font.pixelSize: Math.round(f.size * 0.45)
        clip: true
        activeFocusOnTab: true
        onAccepted: f.accepted()
        Keys.onUpPressed: f.upPressed()
        Keys.onDownPressed: f.downPressed()
        Text { anchors.fill: parent; verticalAlignment: Text.AlignVCenter; visible: !input.text && !input.activeFocus; text: f.placeholder; color: Theme.muted; font: input.font; elide: Text.ElideRight }
    }
    MouseArea { anchors.fill: parent; cursorShape: Qt.IBeamCursor; onPressed: mouse => { input.forceActiveFocus(); mouse.accepted = false; } }
}

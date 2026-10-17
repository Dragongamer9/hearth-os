// A text box (Wi-Fi passwords, names). Type with a keyboard, or with the on-screen keyboard (Y on a
// controller). Enter submits.
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: f
    property string placeholder
    property bool secret: false
    property alias text: input.text
    signal submitted(string text)
    Layout.fillWidth: true
    implicitHeight: 52
    radius: 12
    color: Theme.panel3
    border.width: 2
    border.color: input.activeFocus ? Theme.accent : "transparent"
    function focusInput() { input.forceActiveFocus(); }

    TextInput {
        id: input
        anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
        verticalAlignment: TextInput.AlignVCenter
        color: Theme.text
        font.family: Theme.font
        font.pixelSize: 15
        echoMode: f.secret ? TextInput.Password : TextInput.Normal
        selectByMouse: true
        activeFocusOnTab: true
        clip: true
        onAccepted: f.submitted(text)
        Keys.onPressed: e => {
            if (e.key === Qt.Key_Down) { input.nextItemInFocusChain(true).forceActiveFocus(); e.accepted = true; }
            else if (e.key === Qt.Key_Up) { input.nextItemInFocusChain(false).forceActiveFocus(); e.accepted = true; }
            else if (e.key === Qt.Key_Escape) { SettingsNav.toNav(); e.accepted = true; }
        }
        Text { anchors.verticalCenter: parent.verticalCenter; visible: !input.text && !input.activeFocus; text: f.placeholder; color: Theme.muted; font: input.font }
    }
}

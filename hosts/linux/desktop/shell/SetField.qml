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
    radius: SetStyle.radius
    color: SetStyle.card2
    border.width: input.activeFocus ? 2 : 1
    border.color: input.activeFocus ? Theme.accent : Theme.line
    z: input.activeFocus ? 2 : 0
    function focusInput() { input.forceActiveFocus(); }

    Rectangle {
        visible: input.activeFocus
        anchors { fill: parent; margins: -7 }
        radius: f.radius + 7
        color: "transparent"
        border.width: 5
        border.color: SetStyle.tint(Theme.accent, 0.22)
    }
    TextInput {
        id: input
        anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
        verticalAlignment: TextInput.AlignVCenter
        color: Theme.text
        selectionColor: Theme.accent
        selectedTextColor: Theme.accentInk
        font.family: Theme.font
        font.pixelSize: 15
        echoMode: f.secret ? TextInput.Password : TextInput.Normal
        selectByMouse: true
        activeFocusOnTab: true
        clip: true
        onAccepted: f.submitted(text)
        onActiveFocusChanged: if (activeFocus) f.scrollIntoView()
        Keys.onPressed: e => {
            if (e.key === Qt.Key_Down) { input.nextItemInFocusChain(true).forceActiveFocus(); e.accepted = true; }
            else if (e.key === Qt.Key_Up) { input.nextItemInFocusChain(false).forceActiveFocus(); e.accepted = true; }
            else if (e.key === Qt.Key_Escape) { SettingsNav.back(); e.accepted = true; }
        }
        Text { anchors.verticalCenter: parent.verticalCenter; visible: !input.text && !input.activeFocus; text: f.placeholder; color: Theme.muted; font: input.font }
    }
    function scrollIntoView() {
        let p = f.parent;
        while (p && !(p.contentItem !== undefined && p.contentHeight !== undefined && p.contentY !== undefined)) p = p.parent;
        if (!p) return;
        const y = f.mapToItem(p.contentItem, 0, 0).y;
        if (y < p.contentY + 12) p.contentY = Math.max(0, y - 12);
        else if (y + f.height > p.contentY + p.height - 12) p.contentY = Math.min(p.contentHeight - p.height, y + f.height - p.height + 12);
    }
}

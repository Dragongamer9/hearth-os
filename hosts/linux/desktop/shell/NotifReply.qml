// The Reply box on a notification pop-up (Notifications.qml) for notifications that take a reply: your phone's
// messages through Hearth Connect (docs/PHONE-LINK.md), or any app that asks for an inline reply. Type and press Enter
// (or Send); the text goes back to the app (NotificationReplied), which sends it from the phone. The pop-up takes the
// keyboard only while you type here (`typing`), and stays up meanwhile. Hidden where the notification has no reply.
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: nr
    property var notification: null
    readonly property bool wanted: !!notification && notification.hasInlineReply === true
    property bool engaged: false       // clicked into: the pop-up asks for the keyboard (it has none till then)
    readonly property bool typing: engaged || input.activeFocus
    signal sent()
    visible: wanted
    Layout.fillWidth: true
    implicitHeight: wanted ? 36 : 0
    radius: Theme.r(10)
    color: Theme.panel2
    border.width: input.activeFocus ? 2 : 1
    border.color: input.activeFocus ? Theme.accent : Theme.line

    function send() {
        const t = input.text.trim();
        if (!t || !wanted) return;
        notification.sendInlineReply(t);
        input.text = "";
        input.focus = false;
        engaged = false;
        nr.sent();
    }

    TextInput {
        id: input
        anchors { left: parent.left; right: sendBtn.left; verticalCenter: parent.verticalCenter; leftMargin: 12; rightMargin: 8 }
        color: Theme.text
        selectionColor: Theme.accent
        selectedTextColor: Theme.accentInk
        font.family: Theme.font
        font.pixelSize: 13
        clip: true
        selectByMouse: true
        onAccepted: nr.send()
        Keys.onEscapePressed: { input.focus = false; nr.engaged = false; }
        MouseArea { anchors.fill: parent; cursorShape: Qt.IBeamCursor; onClicked: { nr.engaged = true; input.forceActiveFocus(); } }
        Text {
            visible: !input.text && !input.activeFocus
            anchors.verticalCenter: parent.verticalCenter
            text: nr.notification && nr.notification.inlineReplyPlaceholder ? nr.notification.inlineReplyPlaceholder : "Reply…"
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
        }
    }
    Rectangle {
        id: sendBtn
        anchors { right: parent.right; verticalCenter: parent.verticalCenter; rightMargin: 4 }
        width: sendText.implicitWidth + 20; height: 28
        radius: Theme.r(8)
        color: input.text.trim() ? Theme.accent : Theme.panel3
        Text { id: sendText; anchors.centerIn: parent; text: "Send"; color: input.text.trim() ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: nr.send() }
    }
}

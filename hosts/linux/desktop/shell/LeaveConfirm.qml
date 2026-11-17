// 1.14: "Log out?" (Settings › Startup › "Confirm before logging out", off unless you turn it on; Leave.qml asks for
// it). Log out is picked; Left / Right (or Tab) move, Enter / A does it, Esc / B or a click outside cancels.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: root
    property bool shown: false
    property int sel: 0                  // 0 Log out, 1 Cancel
    Component.onCompleted: Leave.canAsk = true
    Component.onDestruction: Leave.canAsk = false
    Connections { target: Leave; function onConfirmRequested() { root.sel = 0; root.shown = true; } }
    function pick(i) { shown = false; if (i === 0) Leave.now(); }

    visible: shown
    color: "transparent"
    anchors { top: true; bottom: true; left: true; right: true }
    exclusiveZone: -1
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-leave"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None

    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(0, 0, 0, 0.55)
        MouseArea { anchors.fill: parent; onClicked: root.shown = false }
    }
    Rectangle {
        id: card
        anchors.centerIn: parent
        width: 400
        height: col.implicitHeight + 44
        radius: Theme.r(20)
        color: Theme.glass
        border.color: Theme.line
        focus: root.shown
        MouseArea { anchors.fill: parent }   // clicks on the card don't cancel
        Keys.onPressed: e => {
            if (e.key === Qt.Key_Left || e.key === Qt.Key_Right || e.key === Qt.Key_Tab || e.key === Qt.Key_Backtab) root.sel = 1 - root.sel;
            else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) root.pick(root.sel);
            else if (e.key === Qt.Key_Escape) root.shown = false;
            else return;
            e.accepted = true;
        }
        ColumnLayout {
            id: col
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 22 }
            spacing: 10
            RowLayout {
                spacing: 12
                Icon { name: "logout"; size: 26; color: Theme.accent }
                Text { text: "Log out?"; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold }
            }
            Text {
                Layout.fillWidth: true
                text: "Your apps close and you go back to the sign-in screen. Save your work first."
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
            }
            RowLayout {
                Layout.topMargin: 8
                Layout.fillWidth: true
                spacing: 10
                Repeater {
                    model: [{ label: "Log out", icon: "logout" }, { label: "Cancel", icon: "close" }]
                    delegate: Rectangle {
                        id: b
                        required property var modelData
                        required property int index
                        readonly property bool lit: root.sel === index
                        Layout.fillWidth: true
                        implicitHeight: 48
                        radius: Theme.r(12)
                        color: lit ? Theme.accent : area.containsMouse ? Theme.panel3 : Theme.panel2
                        RowLayout {
                            anchors.centerIn: parent
                            spacing: 10
                            Icon { name: b.modelData.icon; size: 18; color: b.lit ? Theme.accentInk : Theme.text }
                            Text { text: b.modelData.label; color: b.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: b.lit ? Font.DemiBold : Font.Normal }
                        }
                        MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; onEntered: root.sel = b.index; onClicked: root.pick(b.index) }
                    }
                }
            }
            Text {
                Layout.fillWidth: true
                text: "Turn this question off in Settings › Startup"
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; opacity: 0.8
            }
        }
    }
}

// The power menu (Desktop Mode): what the power button can open (Settings › Power › Power button).
// Up/Down (or the D-pad) and Enter/A pick, Esc/B closes.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: root
    property bool shown: false
    property int sel: 0
    readonly property var items: [
        { icon: "lock", label: "Lock", cmd: ["sh", "-c", "exec quickshell ipc -p \"$NIMBUS_SHARE/desktop/shell\" call lock lock"] },
        { icon: "sleep", label: "Sleep", cmd: ["python3", Buttons.helper, "sleep", "Desktop Mode: power menu"] },
        { icon: "restart", label: "Restart", cmd: ["systemctl", "reboot"] },
        { icon: "power", label: "Shut down", cmd: ["systemctl", "poweroff"] },
        { icon: "pad", label: "Game Mode", cmd: ["nimbus-mode", "game"] },
        { icon: "close", label: "Cancel", cmd: [] }
    ]
    function open() { sel = 0; shown = true; }
    function pick(i) {
        shown = false;
        const c = items[i] ? items[i].cmd : [];
        if (c.length) Quickshell.execDetached(c);
    }
    Connections { target: Buttons; function onPowerMenuRequested() { root.shown ? root.pick(root.sel) : root.open(); } }

    visible: shown
    color: "transparent"
    anchors { top: true; bottom: true; left: true; right: true }
    exclusiveZone: -1
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-power"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None

    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(0, 0, 0, 0.55)
        MouseArea { anchors.fill: parent; onClicked: root.shown = false }
    }
    Rectangle {
        id: card
        anchors.centerIn: parent
        width: 360
        height: col.implicitHeight + 40
        radius: 20
        color: Theme.glass
        border.color: Theme.line
        focus: root.shown
        Keys.onPressed: e => {
            if (e.key === Qt.Key_Down || e.key === Qt.Key_Tab) root.sel = (root.sel + 1) % root.items.length;
            else if (e.key === Qt.Key_Up || e.key === Qt.Key_Backtab) root.sel = (root.sel + root.items.length - 1) % root.items.length;
            else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) root.pick(root.sel);
            else if (e.key === Qt.Key_Escape) root.shown = false;
            else return;
            e.accepted = true;
        }
        ColumnLayout {
            id: col
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 20 }
            spacing: 6
            Text { text: "Power"; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold; Layout.bottomMargin: 6; Layout.leftMargin: 4 }
            Repeater {
                model: root.items
                delegate: Rectangle {
                    id: it
                    required property var modelData
                    required property int index
                    readonly property bool lit: root.sel === index
                    Layout.fillWidth: true
                    implicitHeight: 52
                    radius: 12
                    color: lit ? Theme.accent : area.containsMouse ? Theme.panel3 : Theme.panel2
                    RowLayout {
                        anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
                        spacing: 14
                        Icon { name: it.modelData.icon; size: 20; color: it.lit ? Theme.accentInk : Theme.text }
                        Text { Layout.fillWidth: true; text: it.modelData.label; color: it.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: it.lit ? Font.DemiBold : Font.Normal }
                    }
                    MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; onEntered: root.sel = it.index; onClicked: root.pick(it.index) }
                }
            }
        }
    }
}

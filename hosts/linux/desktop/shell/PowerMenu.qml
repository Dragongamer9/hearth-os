// The power menu (Desktop Mode): what the power button can open (Settings › Power › Power button).
// Up/Down (or the D-pad) and Enter/A pick, Esc/B closes.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
    id: root
    property bool shown: false
    property int sel: 0
    // 1.14, KDE's Leave screen: Switch user, Hibernate, Restart into firmware setup and Log out too, each only where it
    // works here (hearth-session-actions can, asked each time the menu opens).
    property var can: ({})
    readonly property string actions: decodeURIComponent(Qt.resolvedUrl("hearth-session-actions").toString().replace(/^file:\/\//, ""))
    readonly property var items: [
        { icon: "lock", label: "Lock", cmd: ["sh", "-c", "exec quickshell ipc -p \"$NIMBUS_SHARE/desktop/shell\" call lock lock"] }
    ].concat(can.switchUser && can.otherAccounts > 0 ? [{ icon: "switch-user", label: "Switch user", cmd: ["python3", actions, "switch-user"] }] : []).concat([
        { icon: "sleep", label: "Sleep", cmd: ["python3", Buttons.helper, "sleep", "Desktop Mode: power menu"] }
    ]).concat(can.hibernate ? [{ icon: "moon", label: "Hibernate", cmd: ["python3", actions, "hibernate"] }] : []).concat([
        { icon: "restart", label: "Restart", cmd: ["systemctl", "reboot"] }
    ]).concat(can.firmware ? [{ icon: "cpu", label: "Restart into firmware setup", cmd: ["python3", actions, "firmware"] }] : []).concat([
        { icon: "power", label: "Shut down", cmd: ["systemctl", "poweroff"] },
        { icon: "logout", label: "Log out", cmd: [], logout: true },
        { icon: "pad", label: "Game Mode", cmd: ["nimbus-mode", "game"] },
        { icon: "close", label: "Cancel", cmd: [] }
    ])
    Process {
        id: canProc
        command: ["python3", root.actions, "can"]
        stdout: StdioCollector { onStreamFinished: { try { const o = JSON.parse(text.trim().split("\n").pop() || "{}"); if (o && typeof o === "object") root.can = o; } catch (e) {} } }
    }
    function open() { sel = 0; shown = true; canProc.running = false; canProc.running = true; }
    IpcHandler {
        target: "powermenu"
        function open(): void { root.open(); }
        function close(): void { root.shown = false; }
        function demo(): void { root.can = { switchUser: true, otherAccounts: 1, hibernate: true, firmware: true }; root.sel = 0; root.shown = true; }
    }
    function pick(i) {
        shown = false;
        // Log out from here always asks first (LeaveConfirm.qml): logging out ends every app within seconds, and a
        // D-pad press one row too far mustn't lose work. Without the question (it didn't load), as Leave.qml does.
        if (items[i] && items[i].logout) { if (Leave.canAsk) Leave.confirmRequested(); else Leave.now(); return; }
        const c = items[i] ? items[i].cmd : [];
        if (c.length) Quickshell.execDetached(c);
    }
    Connections { target: Buttons; function onPowerMenuRequested() { root.shown ? root.pick(root.sel) : root.open(); } }
    // 1.14: Settings › Startup › "Default leave option" is what's picked when it opens ("": the first, Lock).
    onShownChanged: if (shown && Settings.leaveDefault !== "") { const i = items.findIndex(x => x.label === Settings.leaveDefault); if (i >= 0) sel = i; }

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
        radius: Theme.r(20)
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
                    radius: Theme.r(12)
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

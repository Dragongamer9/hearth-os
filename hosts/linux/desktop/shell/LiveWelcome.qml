// The welcome card on the Hearth live USB (/etc/hearth-live): install Hearth, connect to Wi-Fi first (the
// installer downloads Hearth), or try Game Mode. Controller: Up/Down and A; B closes it (the Hearth menu and
// the launcher's "Install Hearth" bring the installer back).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
    id: root
    required property var settingsApp
    property bool live: false
    property bool shown: false
    property int sel: 0
    readonly property var items: [
        { icon: "update", label: "Install Hearth", sub: "Puts Hearth on this device's drive" },
        { icon: "wifi", label: "Connect to Wi-Fi", sub: "The installer downloads Hearth, so connect first" },
        { icon: "pad", label: "Try Game Mode", sub: "Come back here from the Power menu › Switch to desktop" },
        { icon: "close", label: "Keep exploring", sub: "Install Hearth is also in the app launcher" }
    ]
    function pick(i) {
        shown = false;
        if (i === 0) Quickshell.execDetached(["sh", "-c", "command -v liveinst >/dev/null && exec liveinst || exec anaconda-webui"]);
        else if (i === 1) settingsApp.open("wifi");
        else if (i === 2) Quickshell.execDetached(["nimbus-mode", "game"]);
    }
    Process {
        running: true
        command: ["test", "-e", "/etc/hearth-live"]
        onExited: code => { root.live = code === 0; root.shown = root.live; }
    }

    visible: shown
    color: "transparent"
    anchors { top: true; bottom: true; left: true; right: true }
    exclusiveZone: -1
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "hearth-live"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None

    Rectangle { anchors.fill: parent; color: Qt.rgba(0, 0, 0, 0.5) }
    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, 620)
        height: col.implicitHeight + 56
        radius: 24
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
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 28 }
            spacing: 10
            RowLayout {
                spacing: 16
                Layout.bottomMargin: 4
                Image { source: Qt.resolvedUrl("images/hearth-logo.png"); Layout.preferredWidth: 64; Layout.preferredHeight: 64; fillMode: Image.PreserveAspectFit; smooth: true }
                ColumnLayout {
                    spacing: 2
                    Text { text: "Welcome to Hearth"; color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold }
                    Text { Layout.fillWidth: true; text: "You're running Hearth from the USB stick. Look around, then install it when you're ready. Games won't run well until it's installed."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
                }
            }
            Repeater {
                model: root.items
                delegate: Rectangle {
                    id: it
                    required property var modelData
                    required property int index
                    readonly property bool lit: root.sel === index
                    Layout.fillWidth: true
                    implicitHeight: 64
                    radius: 14
                    color: lit ? Theme.accent : area.containsMouse ? Theme.panel3 : Theme.panel2
                    RowLayout {
                        anchors { fill: parent; leftMargin: 18; rightMargin: 18 }
                        spacing: 14
                        Icon { name: it.modelData.icon; size: 22; color: it.lit ? Theme.accentInk : Theme.text }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Text { text: it.modelData.label; color: it.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                            Text { Layout.fillWidth: true; text: it.modelData.sub; color: it.lit ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
                        }
                    }
                    MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; onEntered: root.sel = it.index; onClicked: root.pick(it.index) }
                }
            }
        }
    }
}

// Why Desktop Mode started the way it did, shown once a few seconds after Mantel starts (1.14): 1.13's KDE Plasma or
// GNOME choice was retired (the owner, Nov 16: a choice that changes must tell the person). The session leaves the
// notice (nimbus-session's desk_notice); `hearth-your-desktop notice` hands it over once and marks it as shown, and
// Settings › System › Your desktop keeps showing the last one. Controller: Up/Down and A; B closes it.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
    id: root
    property var settingsApp: null
    property bool shown: false
    property var notice: ({})
    property int sel: 0
    readonly property var items: [{ icon: "check", label: "Got it", sub: "Mantel is your desktop", cmd: "" },
                                  { icon: "desktops", label: "Open Your desktop in Settings", sub: "Settings › System › Your desktop", cmd: "settings" }]
    function pick(i) {
        shown = false;
        const c = items[i] ? items[i].cmd : "";
        if (c === "settings" && settingsApp) settingsApp.open("your-desktop");
    }
    Process {
        id: check
        command: ["python3", decodeURIComponent(Qt.resolvedUrl("hearth-your-desktop").toString().replace(/^file:\/\//, "")), "notice"]
        stdout: StdioCollector { onStreamFinished: {
            let n = {}; try { n = JSON.parse(text.trim() || "{}"); } catch (e) {}
            if (n && n.title) { root.notice = n; root.sel = 0; root.shown = true; }
        } }
    }
    // After SteamFallback's check (1.5 s), so two cards never ask at the same moment.
    Timer { interval: 4000; running: true; onTriggered: check.running = true }

    visible: shown
    color: "transparent"
    anchors { top: true; bottom: true; left: true; right: true }
    exclusiveZone: -1
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "hearth-desktop-notice"
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
            ColumnLayout {
                spacing: 4
                Layout.bottomMargin: 6
                Text { text: root.notice.title || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold }
                Text { Layout.fillWidth: true; text: root.notice.sub || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
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

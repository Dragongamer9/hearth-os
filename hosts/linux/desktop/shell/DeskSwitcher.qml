// Virtual desktops on the taskbar: one small button per desktop (labwc's <desktops> in rc.xml), the one you're
// on lit. Click to switch, scroll to step through them. Super+Ctrl+Left/Right switch from the keyboard.
// Loaded by Bar.qml through a Loader: it needs Quickshell 0.3's WindowManager module and a labwc with the
// ext-workspace protocol (0.8.3+); without them it stays empty and the taskbar simply doesn't show it.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.WindowManager

Item {
    id: root
    property bool portrait: false
    // The desktops, in order: [{ name, active, set }] (set: the Windowset, for activate()).
    readonly property var sets: {
        const all = Array.from(WindowManager.windowsets || []).filter(w => w && w.shouldDisplay !== false);
        all.sort((a, b) => {
            const ca = a.coordinates || [], cb = b.coordinates || [];
            for (let i = 0; i < Math.max(ca.length, cb.length); i++) if ((ca[i] || 0) !== (cb[i] || 0)) return (ca[i] || 0) - (cb[i] || 0);
            // No coordinates (labwc doesn't send them): by name, numbers in order ("Desktop 2" before "Desktop 10"),
            // so the first desktop is the first button and the one you start on isn't shown as "3".
            return String(a.name || "").localeCompare(String(b.name || ""), undefined, { numeric: true });
        });
        return all.map(w => ({ name: w.name || "", active: w.active, set: w }));
    }
    readonly property int activeIndex: sets.findIndex(s => s.active)
    function go(i) {
        const s = sets[i];
        if (s && s.set.canActivate !== false) s.set.activate();
    }

    readonly property bool wanted: sets.length > 1
    implicitWidth: wanted ? grid.implicitWidth + 8 : 0
    implicitHeight: wanted ? grid.implicitHeight + 8 : 0

    Rectangle {
        anchors.fill: parent
        radius: 10
        color: Theme.bg2
        border.color: Theme.line
    }
    GridLayout {
        id: grid
        anchors.centerIn: parent
        flow: root.portrait ? GridLayout.TopToBottom : GridLayout.LeftToRight
        rows: root.portrait ? -1 : 1
        columns: root.portrait ? 1 : -1
        rowSpacing: 3
        columnSpacing: 3
        Repeater {
            model: root.sets
            delegate: Rectangle {
                id: pill
                required property var modelData
                required property int index
                implicitWidth: 24
                implicitHeight: 24
                radius: 7
                color: modelData.active ? Theme.accent : pa.containsMouse ? Theme.panel3 : "transparent"
                Text {
                    anchors.centerIn: parent
                    text: pill.index + 1
                    color: pill.modelData.active ? Theme.accentInk : Theme.muted
                    font.family: Theme.font
                    font.pixelSize: 12
                    font.bold: pill.modelData.active
                }
                MouseArea {
                    id: pa
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.go(pill.index)
                }
            }
        }
    }
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.NoButton
        onWheel: w => {
            const n = root.sets.length;
            if (n > 1 && w.angleDelta.y !== 0) root.go(Math.max(0, Math.min(n - 1, root.activeIndex + (w.angleDelta.y > 0 ? -1 : 1))));
        }
    }
}

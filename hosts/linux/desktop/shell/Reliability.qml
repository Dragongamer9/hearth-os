// Reliability history (docs/SYSTEM-RESTORE.md, "Reliability history"): how Hearth has been running, day by day. A
// normal window like Task Manager; the page is ReliabilityView.qml, loaded by file name, and this whole file is loaded
// by shell.qml through a Loader, so a problem in either only loses Reliability history, never the desktop.
// Reading only: it never changes anything. The journal is read when it opens (hearth_reliability.py summary), and
// finished days are kept small in ~/.cache/hearth/reliability.json.
// Opened from the app menu (hearth-reliability.desktop), Settings › Updates & recovery, `hearth-reliability`, or:
//   quickshell ipc -p <shell folder> call reliability open | close | toggle
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: rl
    property bool shown: false
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth_reliability.py").toString().replace(/^file:\/\//, ""))
    function open() { shown = true; }
    function close() { shown = false; }

    IpcHandler {
        target: "reliability"
        function open(): void { rl.open(); }
        function close(): void { rl.close(); }
        function toggle(): void { rl.shown = !rl.shown; }
    }

    LazyLoader {
        active: rl.shown
        FloatingWindow {
            title: "Reliability history"
            implicitWidth: 1200
            implicitHeight: 720
            color: "#171d25"
            visible: true
            onVisibleChanged: if (!visible) rl.shown = false
            Loader {
                id: view
                anchors.fill: parent
                focus: true
                source: "ReliabilityView.qml"
                onStatusChanged: if (status === Loader.Error) console.warn("Reliability history: its page (ReliabilityView.qml) couldn't load")
                onLoaded: { item.helper = rl.helper; item.closeWindow = () => rl.close(); item.start(); }
            }
            Text {
                anchors.centerIn: parent
                visible: view.status === Loader.Error
                width: parent.width - 80
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: "Reliability history couldn't load its page. Send logs from Settings › Updates & recovery › Backup and help so it can be fixed."
                color: "#9fb2c3"; font.family: Theme.font; font.pixelSize: 14
            }
        }
    }
}

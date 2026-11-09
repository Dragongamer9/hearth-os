// System Restore (docs/SYSTEM-RESTORE.md): Hearth and its settings back to an earlier restore point. A normal window
// like Task Manager; the page is RestoreView.qml, loaded by file name, and this whole file is loaded by shell.qml
// through a Loader, so a problem in either only loses System Restore, never the desktop (or Game Mode).
// Opened from the app menu (hearth-restore.desktop), Settings › Updates & recovery, `hearth-restore`, or:
//   quickshell ipc -p <shell folder> call restore open | close | toggle
// Opening it writes nothing: it lists the points (hearth_restore.py overview) and asks the system which versions it
// keeps (hearth-admin update-status, read-only). Settings go back only after the person confirms.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: rs
    property bool shown: false
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth_restore.py").toString().replace(/^file:\/\//, ""))
    function open() { shown = true; }
    function close() { shown = false; }

    IpcHandler {
        target: "restore"
        function open(): void { rs.open(); }
        function close(): void { rs.close(); }
        function toggle(): void { rs.shown = !rs.shown; }
    }

    LazyLoader {
        active: rs.shown
        FloatingWindow {
            title: "System Restore"
            implicitWidth: 960
            implicitHeight: 640
            color: "#171d25"
            visible: true
            onVisibleChanged: if (!visible) rs.shown = false
            Loader {
                id: view
                anchors.fill: parent
                focus: true
                source: "RestoreView.qml"
                onStatusChanged: if (status === Loader.Error) console.warn("System Restore: its page (RestoreView.qml) couldn't load")
                onLoaded: { item.helper = rs.helper; item.closeWindow = () => rs.close(); item.start(); }
            }
            Text {
                anchors.centerIn: parent
                visible: view.status === Loader.Error
                width: parent.width - 80
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: "System Restore couldn't load its page. Going back is still in Settings › Updates & recovery."
                color: "#9fb2c3"; font.family: Theme.font; font.pixelSize: 14
            }
        }
    }
}

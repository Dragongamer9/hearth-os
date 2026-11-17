// The capture dot: a small pill at the top right, under the bar, whenever this PC's screen is being taken somewhere,
// so it's never done without you seeing it. "Recording 1:23" with Stop while the Recorder records (hearth-record), and
// "<device> is connected" with Disconnect while someone uses this PC with Remote desktop (hearth-vnc). Gone otherwise.
// It only watches the two helpers' runtime files ($XDG_RUNTIME_DIR/hearth-record/state.json and
// $XDG_RUNTIME_DIR/hearth-vnc/clients.json): no polling, nothing written. Clicks go through around it.
// Loaded by shell.qml through a Loader: a problem here only loses the dot.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: ci
    readonly property string run: Quickshell.env("XDG_RUNTIME_DIR") || "/tmp"
    property var rec: ({ state: "idle" })
    property var vnc: ({ clients: [] })
    property int tick: 0
    readonly property bool recording: rec.state === "recording" || rec.state === "stopping"
    readonly property var viewers: (vnc && vnc.clients) || []
    readonly property bool shown: recording || viewers.length > 0
    function shell(n) { return decodeURIComponent(Qt.resolvedUrl(n).toString().replace(/^file:\/\//, "")); }
    function elapsed() {
        void ci.tick;
        if (!rec.since) return "0:00";
        const s = Math.max(0, Math.floor(Date.now() / 1000 - rec.since));
        const m = Math.floor(s / 60), h = Math.floor(m / 60);
        return (h ? h + ":" + ("0" + (m % 60)).slice(-2) : m) + ":" + ("0" + (s % 60)).slice(-2);
    }
    function who() {
        if (!viewers.length) return "";
        const n = viewers[0].name || viewers[0].ip;
        return viewers.length > 1 ? n + " and " + (viewers.length - 1) + " more" : n;
    }

    // A file that isn't there yet can't be watched (it's made when the first recording or connection starts), so both
    // are also read again every 2 seconds: two tiny reads, with "file not found" kept out of the log.
    FileView {
        id: recFile
        path: ci.run + "/hearth-record/state.json"
        printErrors: false
        watchChanges: true
        onFileChanged: reload()
        onLoaded: { try { ci.rec = JSON.parse(text()); } catch (e) {} }
        onLoadFailed: ci.rec = { state: "idle" }
    }
    FileView {
        id: vncFile
        path: ci.run + "/hearth-vnc/clients.json"
        printErrors: false
        watchChanges: true
        onFileChanged: reload()
        onLoaded: { try { ci.vnc = JSON.parse(text()); } catch (e) {} }
        onLoadFailed: ci.vnc = { clients: [] }
    }
    Timer { interval: 2000; running: true; repeat: true; onTriggered: { recFile.reload(); vncFile.reload(); } }
    // The time while the dot shows.
    Timer { interval: 1000; running: ci.shown; repeat: true; onTriggered: ci.tick++ }

    Process { id: stopRec; command: ["python3", ci.shell("hearth-record"), "stop"] }
    Process { id: kick; command: ["python3", ci.shell("hearth-vnc"), "disconnect", "all"] }

    PanelWindow {
        visible: ci.shown
        screen: Quickshell.screens.length ? Quickshell.screens[0] : null
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-capture"
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; right: true }
        margins { top: 56; right: 12 }
        implicitWidth: pills.implicitWidth
        implicitHeight: pills.implicitHeight
        color: "transparent"

        ColumnLayout {
            id: pills
            anchors.right: parent.right
            spacing: 6
            Repeater {
                model: [ci.recording ? "rec" : "", ci.viewers.length ? "vnc" : ""].filter(k => k !== "")
                delegate: Rectangle {
                    required property string modelData
                    readonly property bool isRec: modelData === "rec"
                    Layout.alignment: Qt.AlignRight
                    implicitHeight: 34
                    implicitWidth: row.implicitWidth + 22
                    radius: 17
                    color: Theme.glass
                    border.color: isRec ? "#e5484d" : Theme.accent
                    border.width: 1.5
                    RowLayout {
                        id: row
                        anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter }
                        spacing: 8
                        Rectangle {
                            visible: isRec
                            implicitWidth: 10; implicitHeight: 10; radius: 5; color: "#e5484d"
                            SequentialAnimation on opacity { running: isRec; loops: Animation.Infinite; NumberAnimation { to: 0.3; duration: 700 } NumberAnimation { to: 1; duration: 700 } }
                        }
                        Icon { visible: !isRec; name: "eye"; size: 16; color: Theme.text }
                        Text {
                            text: isRec ? "Recording " + ci.elapsed() : ci.who() + (ci.viewers.length > 1 ? " are" : " is") + " connected"
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 13
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                                onClicked: isRec ? Quickshell.execDetached(["quickshell", "ipc", "-p", decodeURIComponent(Qt.resolvedUrl(".").toString().replace(/^file:\/\//, "")), "call", "record", "open"])
                                                 : SettingsNav.openPage("remote-desktop") }
                        }
                        Rectangle {
                            implicitWidth: act.implicitWidth + 16; implicitHeight: 24; radius: 12
                            color: am.containsMouse ? (isRec ? "#e5484d" : Theme.accent) : Theme.panel3
                            Text { id: act; anchors.centerIn: parent; text: isRec ? "Stop" : "Disconnect"; color: am.containsMouse ? "white" : Theme.text; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                            MouseArea { id: am; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                onClicked: { if (isRec) { if (!stopRec.running) stopRec.running = true; } else if (!kick.running) kick.running = true; } }
                        }
                    }
                }
            }
        }
    }
}

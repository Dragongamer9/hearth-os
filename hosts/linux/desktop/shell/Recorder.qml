// The Recorder panel: record the screen from Desktop Mode, on any desktop (it opens from the bar's capture dot and
// `quickshell ipc ... call record open`; Super+Alt+R is its Start / Stop where the Hearth Recorder app can't record:
// open.sh, and KWin's own shortcut). What to record (a screen, a window, or pick when
// it starts), the sound (none, this PC's, the microphone, both), 30 or 60 frames a second and the pointer, a big Start
// / Stop, and the last few recordings with Open and Show in folder.
// Its helper is hearth-record (the desktop's screen-sharing portal and GStreamer: KWin and labwc alike). Opening the
// panel only reads (hearth-record status); a choice is saved when you change it (~/.config/nimbus/record.json). While
// recording, the bar's capture dot (CaptureIndicator.qml) shows the time and stops it.
// Loaded by shell.qml through a Loader: a problem here only loses the panel.
//   quickshell ipc -p <shell folder> call record open|close|toggle|start|stop|state
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: rp
    property bool shown: false
    property var st: null                // hearth-record status
    property string msg: ""
    property bool bad: false
    property bool busy: false
    property var live: ({ state: "idle" }) // the runtime state file (what's recording now)
    readonly property bool recording: live.state === "recording" || live.state === "starting" || live.state === "stopping"
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-record").toString().replace(/^file:\/\//, ""))
    readonly property string statePath: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hearth-record/state.json"
    readonly property var choices: st && st.choices ? st.choices : ({ what: "any", sound: "desktop", fps: 30, pointer: true })
    ExtrasPal { id: pal }

    function open() { shown = true; msg = ""; load(); }
    function close() { shown = false; }
    function toggle() { shown ? close() : open(); }
    function load() { statusProc.running = true; }
    function say(t, b) { msg = t; bad = !!b; }
    function call(args, done) {
        if (actProc.running) return;
        actProc.done = done || null;
        actProc.command = ["python3", rp.helper].concat(args);
        actProc.running = true;
    }
    function start() {
        // The panel closes first: the desktop asks what to share in its own window.
        close();
        busy = true;
        call(["start"], r => { busy = false; if (r.error) { say(r.error, true); open(); } });
    }
    function stop() { busy = true; call(["stop"], r => { busy = false; if (r.error) say(r.error, true); else if (r.file) say("Saved " + r.file.replace(/^.*\//, ""), false); load(); }); }
    function startStop() { recording ? stop() : start(); }
    function choose(key, value) {
        call(["set", key, typeof value === "boolean" ? (value ? "on" : "off") : String(value)], r => { if (r.error) say(r.error, true); load(); });
        if (st) { const c = Object.assign({}, st.choices); c[key] = value; const s = Object.assign({}, st); s.choices = c; st = s; }
    }
    function elapsed() {
        if (!live.since) return "0:00";
        const s = Math.max(0, Math.floor(Date.now() / 1000 - live.since));
        const m = Math.floor(s / 60), h = Math.floor(m / 60);
        return (h ? h + ":" + ("0" + (m % 60)).slice(-2) : m) + ":" + ("0" + (s % 60)).slice(-2);
    }
    function sizeText(b) { return b > 1e9 ? (b / 1e9).toFixed(1) + " GB" : b > 1e6 ? Math.round(b / 1e6) + " MB" : Math.max(1, Math.round(b / 1e3)) + " KB"; }

    Process {
        id: statusProc
        command: ["python3", rp.helper, "status"]
        stdout: StdioCollector {
            onStreamFinished: {
                const l = text.trim().split("\n").pop() || "";
                if (l.startsWith("error: ")) { rp.say(l.slice(7), true); return; }
                try { rp.st = JSON.parse(l); } catch (e) {}
            }
        }
    }
    Process {
        id: actProc
        property var done: null
        stdout: StdioCollector {
            onStreamFinished: {
                const l = text.trim().split("\n").pop() || "";
                let r = {};
                if (l.startsWith("error: ")) r = { error: l.slice(7) };
                else { try { r = JSON.parse(l); } catch (e) { r = {}; } }
                const d = actProc.done; actProc.done = null;
                if (d) d(r);
            }
        }
    }
    FileView {
        path: rp.statePath
        watchChanges: true
        onFileChanged: reload()
        onLoaded: { try { rp.live = JSON.parse(text()); } catch (e) {} }
        onLoadFailed: rp.live = { state: "idle" }
    }
    Timer { interval: 1000; running: rp.shown && rp.recording; repeat: true; onTriggered: clock.text = rp.elapsed() }

    IpcHandler {
        target: "record"
        function open(): void { rp.open(); }
        function close(): void { rp.close(); }
        function toggle(): void { rp.toggle(); }
        function start(): void { if (!rp.recording) rp.start(); }
        function stop(): void { if (rp.recording) rp.stop(); }
        function startStop(): void { rp.startStop(); }
        function state(): string { return JSON.stringify({ shown: rp.shown, recording: rp.recording, live: rp.live, choices: rp.choices }); }
    }

    PanelWindow {
        id: win
        visible: rp.shown
        screen: Quickshell.screens.length ? Quickshell.screens[0] : null
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-recorder"
        WlrLayershell.keyboardFocus: rp.shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "transparent"
        onVisibleChanged: if (visible) keys.forceActiveFocus()

        MouseArea { anchors.fill: parent; onClicked: rp.close() }
        Item {
            id: keys; focus: true
            Keys.onPressed: e => {
                if (e.key === Qt.Key_Escape) { rp.close(); e.accepted = true; }
                else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) { rp.startStop(); e.accepted = true; }
            }
        }

        Rectangle {
            id: card
            width: Math.min(440, parent.width - 24)
            height: Math.min(parent.height - 24, col.implicitHeight + 36)
            x: parent.width - width - 12
            y: 52
            radius: Theme.r(12)
            color: pal.card
            border.color: pal.edge
            clip: true
            MouseArea { anchors.fill: parent }

            ColumnLayout {
                id: col
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 18 }
                spacing: 12

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10
                    Rectangle {
                        implicitWidth: 12; implicitHeight: 12; radius: 6
                        color: rp.recording ? "#e5484d" : pal.muted
                        SequentialAnimation on opacity { running: rp.recording; loops: Animation.Infinite; NumberAnimation { to: 0.3; duration: 700 } NumberAnimation { to: 1; duration: 700 } }
                    }
                    Text {
                        Layout.fillWidth: true
                        text: rp.recording ? "Recording" : "Record the screen"
                        color: pal.text; font.family: pal.font; font.pixelSize: 17; font.weight: Font.DemiBold
                    }
                    Text { id: clock; visible: rp.recording; text: rp.elapsed(); color: pal.text; font.family: pal.font; font.pixelSize: 15 }
                }

                // what, sound, frame rate
                Repeater {
                    model: [
                        { key: "what", label: "Record", opts: [["screen", "A screen"], ["window", "A window"], ["any", "Pick when it starts"]] },
                        { key: "sound", label: "Sound", opts: [["none", "None"], ["desktop", "This PC"], ["mic", "Microphone"], ["both", "Both"]] },
                        { key: "fps", label: "Frames", opts: [[30, "30 a second"], [60, "60 a second"]] }
                    ]
                    delegate: ColumnLayout {
                        id: grp
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: 6
                        enabled: !rp.recording
                        opacity: enabled ? 1 : 0.5
                        Text { text: modelData.label; color: pal.muted; font.family: pal.font; font.pixelSize: 12 }
                        Flow {
                            Layout.fillWidth: true
                            spacing: 6
                            Repeater {
                                model: modelData.opts
                                delegate: Rectangle {
                                    required property var modelData
                                    readonly property bool lit: rp.choices[grp.modelData.key] === modelData[0]
                                    implicitWidth: lbl.implicitWidth + 24; implicitHeight: 32; radius: Theme.r(8)
                                    color: lit ? Theme.accent : ma.containsMouse ? pal.sel : pal.box
                                    border.color: lit ? Theme.accent : pal.line
                                    Text { id: lbl; anchors.centerIn: parent; text: modelData[1]; color: parent.lit ? Theme.accentInk : pal.text; font.family: pal.font; font.pixelSize: 13 }
                                    MouseArea { id: ma; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: if (!parent.lit) rp.choose(grp.modelData.key, modelData[0]) }
                                }
                            }
                        }
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    enabled: !rp.recording
                    opacity: enabled ? 1 : 0.5
                    Text { Layout.fillWidth: true; text: "Show the pointer"; color: pal.text; font.family: pal.font; font.pixelSize: 13 }
                    Rectangle {
                        implicitWidth: 40; implicitHeight: 22; radius: 11
                        color: rp.choices.pointer ? Theme.accent : pal.line
                        Rectangle { width: 16; height: 16; radius: 8; y: 3; x: rp.choices.pointer ? 21 : 3; color: "white"; Behavior on x { NumberAnimation { duration: 120 } } }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: rp.choose("pointer", !rp.choices.pointer) }
                    }
                }

                // Start / Stop
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 44; radius: Theme.r(10)
                    color: rp.recording ? "#e5484d" : Theme.accent
                    opacity: rp.busy ? 0.6 : sm.containsMouse ? 0.9 : 1
                    RowLayout {
                        anchors.centerIn: parent
                        spacing: 10
                        Rectangle { implicitWidth: 14; implicitHeight: 14; radius: rp.recording ? 2 : 7; color: rp.recording ? "white" : Theme.accentInk }
                        Text {
                            text: rp.busy ? (rp.recording ? "Saving…" : "Starting…") : rp.recording ? "Stop and save" : "Start recording"
                            color: rp.recording ? "white" : Theme.accentInk; font.family: pal.font; font.pixelSize: 15; font.weight: Font.DemiBold
                        }
                    }
                    MouseArea { id: sm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; enabled: !rp.busy; onClicked: rp.startStop() }
                }
                Text {
                    Layout.fillWidth: true
                    visible: !!rp.st && !!rp.st.can && (!rp.st.can.portal || !rp.st.can.gst || !(rp.st.can.encoders || []).length)
                    text: !rp.st || !rp.st.can ? "" : !rp.st.can.portal ? "This desktop can't share its screen: " + rp.st.can.portalWhy
                          : !rp.st.can.gst ? "GStreamer's PipeWire plugin (gstreamer1.0-pipewire) is missing"
                          : "GStreamer has no H.264 encoder here"
                    color: pal.warn; font.family: pal.font; font.pixelSize: 12; wrapMode: Text.WordWrap
                }
                Text {
                    Layout.fillWidth: true
                    visible: rp.msg !== ""
                    text: rp.msg; color: rp.bad ? pal.warn : pal.muted; font.family: pal.font; font.pixelSize: 12; wrapMode: Text.WordWrap
                }

                // the last recordings
                Text {
                    visible: !!rp.st && (rp.st.recent || []).length > 0
                    text: "Recent"; color: pal.muted; font.family: pal.font; font.pixelSize: 12
                }
                Repeater {
                    model: rp.st ? (rp.st.recent || []).slice(0, 3) : []
                    delegate: Rectangle {
                        required property var modelData
                        Layout.fillWidth: true
                        implicitHeight: 44; radius: Theme.r(8)
                        color: rm.containsMouse ? pal.sel : pal.box
                        RowLayout {
                            anchors { fill: parent; leftMargin: 10; rightMargin: 6 }
                            spacing: 10
                            Icon { name: "film"; size: 18; color: pal.text }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                Text { Layout.fillWidth: true; text: modelData.name; elide: Text.ElideMiddle; color: pal.text; font.family: pal.font; font.pixelSize: 13 }
                                Text { text: rp.sizeText(modelData.size) + " · " + new Date(modelData.time * 1000).toLocaleString(Qt.locale(), "d MMM, h:mm"); color: pal.muted; font.family: pal.font; font.pixelSize: 11 }
                            }
                            Rectangle {
                                implicitWidth: 30; implicitHeight: 30; radius: Theme.r(6)
                                color: fm.containsMouse ? pal.sel : "transparent"
                                Icon { anchors.centerIn: parent; name: "folder"; size: 16; color: pal.text }
                                MouseArea { id: fm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                    onClicked: { rp.close(); Quickshell.execDetached(["python3", decodeURIComponent(Qt.resolvedUrl("hearth-portal").toString().replace(/^file:\/\//, "")), "show", modelData.path]); } }
                            }
                        }
                        MouseArea { id: rm; anchors { fill: parent; rightMargin: 40 } hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                            onClicked: { rp.close(); Quickshell.execDetached(["xdg-open", modelData.path]); } }
                    }
                }
                Text {
                    Layout.fillWidth: true
                    text: "Recordings go to " + (rp.st ? rp.st.folder.replace(Quickshell.env("HOME"), "~") : "Videos/Recordings") + "."
                    color: pal.muted; font.family: pal.font; font.pixelSize: 11; wrapMode: Text.WordWrap
                }
            }
        }
    }
}

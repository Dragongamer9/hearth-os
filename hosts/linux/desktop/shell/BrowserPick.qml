// "Pick your browser" (1.12, the owner's request): shown once in Desktop Mode after updating to 1.12, when you haven't
// picked a browser yet (a fresh install picks it in first-time setup). Two cards: Hearth Browser (Firefox), which comes
// with Hearth OS, and Google Chrome, downloaded from Google's signed package site when picked (hearth_chrome.py), with
// "Must Have For Xodus Nexus" in red under it. Closing it keeps Hearth Browser and it isn't shown again; Settings ›
// Web browser changes it any time. Your choice is saved by the hearth-browser helper (one key, atomic, .bak).
// Controller: Left/Right, A to pick, B to close. Loaded by shell.qml by file name, so a problem here only loses this.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: bp
    property var ch: null            // hearth-browser choice
    property bool shown: false
    property int pick: 0
    property bool busy: false
    property string line: ""
    property real progress: -1
    property string error: ""
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-browser").toString().replace(/^file:\/\//, ""))
    readonly property var cards: [
        { id: "firefox", title: "Hearth Browser", tag: "Firefox · comes with Hearth OS", body: "Google sign-in, Netflix and add-ons work. Nothing to download.", note: "" },
        { id: "chrome", title: "Google Chrome", tag: "Downloaded from Google · about 120 MB", body: "Google's own browser, checked against Google's signature and kept in your home folder.", note: "Must Have For Xodus Nexus" }
    ]

    // A minute after Desktop Mode starts, so it never gets in the way of signing in or of the session coming up.
    Timer { interval: 60000; running: true; repeat: false; onTriggered: bp.check() }
    IpcHandler {
        target: "browserpick"
        function open(): void { bp.shown = true; }
        function close(): void { bp.later(); }
    }

    function check() { if (!chk.running) { chk.command = ["python3", bp.helper, "choice"]; chk.running = true; } }
    function choose(i) {
        if (busy) return;
        busy = true; error = ""; line = i === 1 ? "Getting Google Chrome…" : "Saving…"; progress = -1;
        act.command = ["python3", bp.helper, "pick", bp.cards[i].id];
        act.running = true;
    }
    function later() {
        if (busy) return;
        shown = false;
        if (!done.running) { done.command = ["python3", bp.helper, "asked"]; done.running = true; }
    }

    Process {
        id: chk
        stdout: SplitParser {
            onRead: l => {
                if (!l.startsWith("{")) return;
                try { bp.ch = JSON.parse(l); } catch (e) { return; }
                if (bp.ch && !bp.ch.picked && !bp.ch.asked) bp.shown = true;
            }
        }
    }
    Process {
        id: act
        stdout: SplitParser {
            onRead: l => {
                const m = l.match(/^progress: (\d+) ?(.*)$/);
                if (m) { bp.progress = parseInt(m[1]) / 100; bp.line = m[2] || bp.line; }
                else if (l.startsWith("error: ")) bp.error = l.slice(7);
                else if (l.startsWith("{")) { try { bp.ch = JSON.parse(l); } catch (e) {} }
            }
        }
        onExited: code => { bp.busy = false; if (code === 0 && bp.error === "") bp.shown = false; }
    }
    Process { id: done }

    PanelWindow {
        visible: bp.shown
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-browser-pick"
        WlrLayershell.keyboardFocus: bp.shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "#b0080b10"
        onVisibleChanged: if (visible) keys.forceActiveFocus()
        MouseArea { anchors.fill: parent; onClicked: bp.later() }

        Rectangle {
            anchors.centerIn: parent
            width: Math.min(parent.width - 32, 860)
            height: col.implicitHeight + 48
            radius: 20
            color: Theme.panel
            border.color: Theme.line
            MouseArea { anchors.fill: parent }

            Item {
                id: keys
                anchors.fill: parent
                focus: true
                Keys.onPressed: e => {
                    switch (e.key) {
                    case Qt.Key_Left: case Qt.Key_Up: bp.pick = 0; break;
                    case Qt.Key_Right: case Qt.Key_Down: case Qt.Key_Tab: bp.pick = 1; break;
                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: bp.choose(bp.pick); break;
                    case Qt.Key_Escape: case Qt.Key_Back: bp.later(); break;
                    default: return;
                    }
                    e.accepted = true;
                }
            }

            ColumnLayout {
                id: col
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 24 }
                spacing: 14
                Text { text: "Pick your browser"; color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold }
                Text {
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    text: "New in Hearth OS 1.12: Google Chrome is a choice now. Links from other apps open in the browser you pick. You can change it any time in Settings › Web browser."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 14
                    Repeater {
                        model: bp.cards
                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            Layout.fillWidth: true
                            implicitHeight: cc.implicitHeight + 28
                            radius: 14
                            color: bp.pick === index ? Theme.panel3 : Theme.panel2
                            border.color: bp.pick === index ? Theme.accent : Theme.line
                            border.width: bp.pick === index ? 2 : 1
                            ColumnLayout {
                                id: cc
                                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14 }
                                spacing: 4
                                Text { text: modelData.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 18; font.weight: Font.DemiBold }
                                Text { text: modelData.tag; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                                Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: modelData.body; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                                Text {
                                    visible: modelData.note !== ""
                                    text: modelData.note; color: Theme.danger
                                    font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
                                }
                            }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onEntered: bp.pick = index; hoverEnabled: true; onClicked: bp.choose(index) }
                        }
                    }
                }
                Text {
                    visible: bp.busy || bp.error !== ""
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    text: bp.error !== "" ? bp.error : bp.line + (bp.progress >= 0 ? "  " + Math.round(bp.progress * 100) + "%" : "")
                    color: bp.error !== "" ? Theme.danger : Theme.text; font.family: Theme.font; font.pixelSize: 13
                }
                Text {
                    text: "A  pick    B  keep Hearth Browser"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                }
            }
        }
    }
}

// Hearth Files' terminal panel (F4, like Dolphin's): a command line under the folder you're in. It runs what you type
// there (sh) and shows what it prints; "cd somewhere" moves Files there too, and moving in Files moves the panel.
// Not a full terminal (no programs that take over the screen, like nano or htop): "Open in Terminal" is beside it for
// those. Up/Down: commands you typed before (this session). Ctrl+C stops the one running. Esc goes back to the files.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Rectangle {
    id: term
    property string folder: ""
    signal cdRequested(string path)
    signal closeRequested()
    signal terminalRequested()
    property var lines: []
    property var history: []
    property int hpos: -1
    readonly property bool busy: proc.running
    color: Theme.panel
    function add(t, kind) { lines = lines.concat([{ t: String(t), k: kind || "" }]).slice(-400); Qt.callLater(() => outView.positionViewAtEnd()); }
    function focusInput() { input.forceActiveFocus(); }
    function home() { return Quickshell.env("HOME") || "/"; }
    function resolve(p) {
        p = p.trim().replace(/^['"]|['"]$/g, "");
        if (p === "" || p === "~") return home();
        if (p.startsWith("~/")) p = home() + p.slice(1);
        if (!p.startsWith("/")) p = folder.replace(/\/$/, "") + "/" + p;
        const parts = [];
        for (const s of p.split("/")) { if (s === "" || s === ".") continue; if (s === "..") parts.pop(); else parts.push(s); }
        return "/" + parts.join("/");
    }
    function run(cmd) {
        cmd = cmd.trim();
        if (!cmd) return;
        history = history.filter(h => h !== cmd).concat([cmd]).slice(-50); hpos = -1;
        add("$ " + cmd, "cmd");
        if (cmd === "clear") { lines = []; return; }
        const cd = /^cd(\s+(.*))?$/.exec(cmd);
        if (cd) { cdCheck.target = resolve(cd[2] || ""); cdCheck.running = true; return; }
        if (proc.running) { add("Still running the last one (Ctrl+C stops it)", "err"); return; }
        proc.command = ["sh", "-c", 'cd -- "$1" 2>/dev/null || cd; shift; exec sh -c "$1" 2>&1', "sh", folder, cmd];
        proc.running = true;
    }
    Process {
        id: proc
        stdout: SplitParser { onRead: l => term.add(l) }
        onExited: code => { if (code !== 0) term.add("(exit code " + code + ")", "err"); }
    }
    Process {
        id: cdCheck
        property string target: ""
        command: ["test", "-d", target]
        onExited: code => { if (code === 0) term.cdRequested(target); else term.add("cd: " + target + ": no such folder", "err"); }
    }

    Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }
    ColumnLayout {
        anchors { fill: parent; margins: 8; topMargin: 6 }
        spacing: 4
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Icon { name: "kbd"; size: 14; color: Theme.accent }
            Text { Layout.fillWidth: true; text: "Terminal · " + term.folder; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideMiddle }
            Text {
                text: "Open in Terminal"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 12
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: term.terminalRequested() }
            }
            Text {
                text: "✕"; color: Theme.muted; font.pixelSize: 13
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: term.closeRequested() }
            }
        }
        ListView {
            id: outView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: term.lines
            boundsBehavior: Flickable.StopAtBounds
            delegate: Text {
                required property var modelData
                width: ListView.view.width
                text: modelData.t
                wrapMode: Text.WrapAnywhere
                color: modelData.k === "cmd" ? Theme.accent : modelData.k === "err" ? Theme.danger : Theme.text
                font.family: "monospace"; font.pixelSize: 13
            }
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 30
            radius: Theme.r(6)
            color: Theme.panel2
            border.color: input.activeFocus ? Theme.accent : Theme.line
            RowLayout {
                anchors { fill: parent; leftMargin: 8; rightMargin: 8 }
                spacing: 6
                Text { text: term.busy ? "…" : "$"; color: Theme.accent; font.family: "monospace"; font.pixelSize: 13 }
                TextInput {
                    id: input
                    Layout.fillWidth: true
                    color: Theme.text; selectionColor: Theme.accent
                    font.family: "monospace"; font.pixelSize: 13
                    clip: true
                    Keys.onReturnPressed: { term.run(text); text = ""; }
                    Keys.onEnterPressed: { term.run(text); text = ""; }
                    Keys.onEscapePressed: term.closeRequested()
                    Keys.onUpPressed: { if (!term.history.length) return; term.hpos = term.hpos < 0 ? term.history.length - 1 : Math.max(0, term.hpos - 1); text = term.history[term.hpos]; }
                    Keys.onDownPressed: { if (term.hpos < 0) return; term.hpos++; if (term.hpos >= term.history.length) { term.hpos = -1; text = ""; } else text = term.history[term.hpos]; }
                    Keys.onPressed: e => { if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_C && proc.running) { proc.signal(2); term.add("^C", "err"); e.accepted = true; } }
                }
            }
        }
    }
}

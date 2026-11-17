// Hearth Files' split view (F9, like Dolphin's Split): a second folder beside the one you're in, to copy or move
// between them. It starts in the same folder; double-click (or Enter) opens a folder in it, Backspace goes up, and
// "Copy here" / "Move here" put what's selected on the left into this folder (Files' own copy and move, with Undo).
// Drop files on it to copy them there. Middle-click a folder to open it on the left.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Rectangle {
    id: pane
    property string helper: ""
    property string path: ""
    property var entries: []
    property string error: ""
    property int current: 0
    property int selCount: 0                  // what's selected on the left
    signal copyHere(string dest)
    signal moveHere(string dest)
    signal openLeft(string path)
    signal closeRequested()
    signal openFile(string path)
    color: Theme.panel
    function load(p) {
        if (p) path = p;
        lister.running = false;
        lister.command = ["python3", helper, "list", path];
        lister.running = true;
    }
    function up() { const p = path.replace(/\/[^/]*\/?$/, "") || "/"; load(p); }
    function activate(e) { if (!e) return; if (e.dir) load(e.path); else openFile(e.path); }
    function human(n) { return n > 1e9 ? (n / 1e9).toFixed(1) + " GB" : n > 1e6 ? (n / 1e6).toFixed(1) + " MB" : n > 1e3 ? Math.round(n / 1e3) + " KB" : n + " B"; }
    onVisibleChanged: if (visible) load()
    Process {
        id: lister
        stdout: StdioCollector {
            onStreamFinished: {
                const last = text.trim().split("\n").pop() || "";
                if (last.startsWith("error: ")) { pane.error = last.slice(7); pane.entries = []; return; }
                try {
                    const r = JSON.parse(last);
                    pane.error = "";
                    pane.path = r.path || pane.path;
                    pane.entries = (r.entries || []).slice().sort((a, b) => (b.dir - a.dir) || a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: "base" }));
                    pane.current = 0;
                } catch (e) { pane.error = "Couldn't open this folder"; }
            }
        }
    }

    Rectangle { anchors { left: parent.left; top: parent.top; bottom: parent.bottom } width: 1; color: Theme.line }
    ColumnLayout {
        anchors { fill: parent; margins: 8; leftMargin: 10 }
        spacing: 6
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Text {
                text: "↑"; color: Theme.text; font.pixelSize: 16
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: pane.up() }
            }
            Text { Layout.fillWidth: true; text: pane.path; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold; elide: Text.ElideMiddle }
            Text {
                text: "✕"; color: Theme.muted; font.pixelSize: 13
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: pane.closeRequested() }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: [["copy", "Copy here"], ["move", "Move here"]]
                delegate: Rectangle {
                    required property var modelData
                    readonly property bool can: pane.selCount > 0
                    implicitWidth: btnText.implicitWidth + 22; implicitHeight: 28; radius: Theme.r(6)
                    color: can ? (btnArea.containsMouse ? Theme.panel3 : Theme.panel2) : "transparent"
                    border.color: Theme.line
                    opacity: can ? 1 : 0.5
                    Text { id: btnText; anchors.centerIn: parent; text: modelData[1] + (pane.selCount ? " (" + pane.selCount + ")" : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 12 }
                    MouseArea { id: btnArea; anchors.fill: parent; hoverEnabled: true; enabled: parent.can; cursorShape: Qt.PointingHandCursor
                        onClicked: modelData[0] === "copy" ? pane.copyHere(pane.path) : pane.moveHere(pane.path) }
                }
            }
            Item { Layout.fillWidth: true }
            Text { text: pane.entries.length + " items"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
        }
        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: pane.entries
            currentIndex: pane.current
            boundsBehavior: Flickable.StopAtBounds
            highlight: Rectangle { radius: Theme.r(6); color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18) }
            highlightMoveDuration: 0
            Keys.onReturnPressed: pane.activate(pane.entries[pane.current])
            Keys.onUpPressed: pane.current = Math.max(0, pane.current - 1)
            Keys.onDownPressed: pane.current = Math.min(pane.entries.length - 1, pane.current + 1)
            Keys.onPressed: e => { if (e.key === Qt.Key_Backspace) { pane.up(); e.accepted = true; } }
            delegate: Item {
                id: row
                required property var modelData
                required property int index
                width: ListView.view.width
                height: 30
                RowLayout {
                    anchors { fill: parent; leftMargin: 6; rightMargin: 6 }
                    spacing: 8
                    Icon { name: row.modelData.dir ? "folder" : "file"; size: 16; color: row.modelData.dir ? Theme.accent : Theme.muted }
                    Text { Layout.fillWidth: true; text: row.modelData.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                    Text { visible: !row.modelData.dir; text: pane.human(row.modelData.size); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                }
                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                    onClicked: mouse => { pane.current = row.index; list.forceActiveFocus(); if (mouse.button === Qt.MiddleButton && row.modelData.dir) pane.openLeft(row.modelData.path); }
                    onDoubleClicked: pane.activate(row.modelData)
                }
            }
        }
        Text { visible: pane.error !== ""; Layout.fillWidth: true; text: pane.error; color: Theme.danger; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
    }
    DropArea {
        anchors.fill: parent
        keys: ["text/uri-list"]
        onDropped: drop => pane.droppedHere(drop)
    }
    signal droppedHere(var drop)
}

// Clipboard history for Desktop Mode: Super+V opens a list of what you copied lately (text and pictures).
// Click an item or press A / Enter to paste it into the app you were in; X (or Delete) forgets one, and "Clear
// all" at the end forgets everything; B / Esc closes. While Settings › Clipboard has history on, the shell keeps
// `wl-paste --watch cliphist store` running (copies marked secret by password managers are skipped).
//   quickshell ipc -p <shell folder> call clipboard toggle|open|close
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: cb
    property bool shown: false
    property var items: []                // [{id, kind: text|image|binary, text, image}]
    property bool available: true
    property bool loading: false
    property int current: 0               // items.length = the "Clear all" row
    property bool confirmClear: false
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-clipboard").toString().replace(/^file:\/\//, ""))

    function open() { shown = true; current = 0; confirmClear = false; refresh(); }
    function close() { shown = false; }
    function toggle() { shown ? close() : open(); }
    onShownChanged: SettingsNav.clipShown = shown
    Component.onCompleted: SettingsNav.clipboard = cb

    // ------------------------------------------------ keeping the history
    property bool tools: false
    Process { running: true; command: ["sh", "-c", "command -v wl-paste && command -v cliphist"]; onExited: code => cb.tools = code === 0 }
    readonly property bool wanted: tools && Settings.loaded && Settings.clipboardHistory
    onWantedChanged: watcher.running = wanted
    Process {
        id: watcher
        // wl-paste (2.2 and newer) says CLIPBOARD_STATE=sensitive for passwords a password manager marked secret.
        command: ["wl-paste", "--watch", "sh", "-c", '[ "$CLIPBOARD_STATE" = sensitive ] || exec cliphist store']
        onExited: if (cb.wanted) retry.start()
    }
    Timer { id: retry; interval: 3000; onTriggered: if (cb.wanted && !watcher.running) watcher.running = true }

    // ------------------------------------------------ the list
    Process {
        id: lister
        command: ["python3", cb.helper, "list"]
        stdout: StdioCollector {
            onStreamFinished: {
                cb.loading = false;
                try {
                    const r = JSON.parse(text.trim().split("\n").pop());
                    cb.available = r.available;
                    cb.items = r.items || [];
                } catch (e) { cb.items = []; }
                cb.current = Math.min(cb.current, Math.max(0, cb.items.length - 1));
            }
        }
    }
    function refresh() { if (!lister.running) { loading = true; lister.running = true; } }
    Process { id: worker; onExited: cb.refresh() }
    function forget(i) {
        const it = items[i];
        if (!it || worker.running) return;
        items = items.filter((x, k) => k !== i);
        current = Math.min(current, items.length);
        worker.command = ["python3", helper, "delete", it.id];
        worker.running = true;
    }
    function clearAll() {
        if (!confirmClear) { confirmClear = true; return; }
        confirmClear = false;
        items = [];
        current = 0;
        worker.command = ["python3", helper, "wipe"];
        worker.running = true;
    }
    function paste(i) {
        const it = items[i];
        if (!it) return;
        close();
        // Detached: the picker has closed and the app you were in has the keyboard back when Ctrl+V is pressed.
        Quickshell.execDetached(["python3", helper, "paste", it.id]);
    }
    function activate() { if (current >= items.length) clearAll(); else paste(current); }

    IpcHandler {
        target: "clipboard"
        function toggle(): void { cb.toggle(); }
        function open(): void { cb.open(); }
        function close(): void { cb.close(); }
    }

    PanelWindow {
        id: win
        visible: cb.shown
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-clipboard"
        WlrLayershell.keyboardFocus: cb.shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "#b0080b10"
        onVisibleChanged: if (visible) keys.forceActiveFocus()

        MouseArea { anchors.fill: parent; onClicked: cb.close() }   // click outside closes

        Rectangle {
            anchors.centerIn: parent
            width: Math.min(parent.width - 48, 640)
            height: Math.min(parent.height - 96, 620)
            radius: 20
            color: Theme.panel
            border.color: Theme.line
            MouseArea { anchors.fill: parent }

            Item {
                id: keys
                anchors.fill: parent
                focus: true
                Keys.onPressed: e => {
                    const last = cb.items.length;      // the "Clear all" row
                    switch (e.key) {
                    case Qt.Key_Down: case Qt.Key_Tab: cb.current = Math.min(last, cb.current + 1); cb.confirmClear = false; break;
                    case Qt.Key_Up: case Qt.Key_Backtab: cb.current = Math.max(0, cb.current - 1); cb.confirmClear = false; break;
                    case Qt.Key_Home: cb.current = 0; break;
                    case Qt.Key_End: cb.current = last; break;
                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: cb.activate(); break;
                    case Qt.Key_Delete: case Qt.Key_Menu: case Qt.Key_X: if (cb.current < last) cb.forget(cb.current); break;
                    case Qt.Key_Escape: cb.close(); break;
                    default: return;
                    }
                    list.positionViewAtIndex(Math.min(cb.current, Math.max(0, last - 1)), ListView.Contain);
                    if (cb.current === last) list.positionViewAtEnd();
                    e.accepted = true;
                }
            }

            ColumnLayout {
                anchors { fill: parent; margins: 20 }
                spacing: 12
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10
                    Icon { name: "clipboard"; size: 22 }
                    Text { Layout.fillWidth: true; text: "Clipboard history"; color: Theme.text; font.family: Theme.font; font.pixelSize: 19; font.weight: Font.DemiBold }
                    Text { text: "Ⓐ paste  Ⓧ forget  Ⓑ close"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                }
                ListView {
                    id: list
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: 6
                    model: cb.items
                    boundsBehavior: Flickable.StopAtBounds
                    delegate: Rectangle {
                        id: row
                        required property var modelData
                        required property int index
                        readonly property bool lit: cb.current === index
                        width: ListView.view.width
                        height: modelData.kind === "image" ? 112 : Math.min(84, Math.max(52, txt.implicitHeight + 24))
                        radius: 12
                        color: lit ? Theme.panel3 : rowMouse.containsMouse ? Theme.panel2 : Theme.bg2
                        border.width: 2
                        border.color: lit ? Theme.accent : "transparent"
                        Text {
                            id: txt
                            visible: row.modelData.kind !== "image"
                            anchors { left: parent.left; right: del.left; verticalCenter: parent.verticalCenter; leftMargin: 14; rightMargin: 8 }
                            text: row.modelData.kind === "binary" ? "Something copied (" + row.modelData.format + ", " + row.modelData.size + ")" : row.modelData.text.replace(/\s+/g, " ").trim()
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 14
                            wrapMode: Text.Wrap; maximumLineCount: 2; elide: Text.ElideRight
                            textFormat: Text.PlainText
                        }
                        Image {
                            visible: row.modelData.kind === "image"
                            anchors { left: parent.left; top: parent.top; bottom: parent.bottom; margins: 8; leftMargin: 14 }
                            width: Math.min(parent.width - 80, 240)
                            source: row.modelData.image ? "file://" + row.modelData.image : ""
                            fillMode: Image.PreserveAspectFit
                            horizontalAlignment: Image.AlignLeft
                            sourceSize: Qt.size(480, 200)
                            asynchronous: true
                        }
                        Text {
                            visible: row.modelData.kind === "image"
                            anchors { right: del.left; verticalCenter: parent.verticalCenter; rightMargin: 10 }
                            text: "Picture" + (row.modelData.dims ? " · " + row.modelData.dims : "")
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                        }
                        MouseArea {
                            id: rowMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: cb.paste(row.index)
                        }
                        Rectangle {
                            id: del
                            anchors { right: parent.right; verticalCenter: parent.verticalCenter; rightMargin: 10 }
                            width: 32; height: 32; radius: 8
                            visible: row.lit || rowMouse.containsMouse || delMouse.containsMouse
                            color: delMouse.containsMouse ? Theme.panel3 : "transparent"
                            Icon { anchors.centerIn: parent; name: "close"; size: 16; color: Theme.muted }
                            MouseArea { id: delMouse; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: cb.forget(row.index) }
                        }
                    }
                    footer: Item {
                        width: ListView.view.width
                        height: cb.items.length ? 62 : 0
                        visible: cb.items.length > 0
                        Rectangle {
                            anchors { fill: parent; topMargin: 8 }
                            radius: 12
                            readonly property bool lit: cb.current === cb.items.length
                            color: lit ? Theme.panel3 : clearMouse.containsMouse ? Theme.panel2 : "transparent"
                            border.width: 2
                            border.color: lit ? Theme.accent : Theme.line
                            Text {
                                anchors.centerIn: parent
                                text: cb.confirmClear ? "Clear everything? Press again to clear" : "Clear all"
                                color: cb.confirmClear ? Theme.danger : Theme.text; font.family: Theme.font; font.pixelSize: 14
                            }
                            MouseArea { id: clearMouse; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { cb.current = cb.items.length; cb.clearAll(); } }
                        }
                    }
                    Text {
                        anchors.centerIn: parent
                        width: parent.width - 40
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        visible: cb.items.length === 0
                        text: !cb.available ? "Clipboard history needs cliphist and wl-clipboard, which aren't installed."
                            : cb.loading ? "Loading…"
                            : !Settings.clipboardHistory ? "Clipboard history is off. Turn it on in Settings › Clipboard."
                            : "Nothing copied yet. Copy some text or a picture and it shows up here."
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
                    }
                }
            }
        }
    }
}

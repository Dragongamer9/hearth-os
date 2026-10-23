// Small questions and details for the desktop: rename, a new folder's name, a file's properties.
//   dialog.ask(screen, "Rename", "notes.txt", "Rename", text => ...)     a text box with OK / Cancel
//   dialog.info(screen, "notes.txt", [["Kind", "Text"], ["Size", "2 KB"]]) a card of details
//   dialog.confirm(screen, "Delete 3 items?", "This can't be undone.", "Delete", () => ...)   OK / Cancel
// Enter (A) is OK, Esc (B) cancels, Tab moves between the box and the buttons; Y on the controller types.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: root
    property bool shown: false
    property string mode: "ask"            // ask | info | confirm
    property string message: ""            // confirm: the text under the title
    property string title: ""
    property string okLabel: "OK"
    property var rows: []
    property var done: null
    property int focusIndex: 0             // 0 the box (or Close), 1 OK, 2 Cancel

    function ask(scr, t, value, ok, cb) {
        if (scr) screen = scr;
        mode = "ask"; title = t; okLabel = ok || "OK"; done = cb; rows = [];
        field.text = value || "";
        shown = true;
        focusIndex = 0;
        field.forceActiveFocus();
        // Select the name without its extension, like file managers do.
        const dot = field.text.lastIndexOf(".");
        field.select(0, dot > 0 ? dot : field.text.length);
    }
    function info(scr, t, details) {
        if (scr) screen = scr;
        mode = "info"; title = t; rows = details || []; done = null;
        shown = true;
        focusIndex = 0;
        keys.forceActiveFocus();
    }
    function confirm(scr, t, msg, ok, cb) {
        if (scr) screen = scr;
        mode = "confirm"; title = t; message = msg || ""; okLabel = ok || "OK"; done = cb; rows = [];
        shown = true;
        focusIndex = 2;                    // Cancel first: Enter alone doesn't delete anything
        keys.forceActiveFocus();
    }
    function accept() {
        if (mode === "confirm") {
            const cb = done;
            shown = false;
            if (cb) cb();
        } else if (mode === "ask") {
            const v = field.text.trim();
            if (!v) return;
            const cb = done;
            shown = false;
            if (cb) cb(v);
        } else shown = false;
    }
    function cancel() { shown = false; }

    visible: shown
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-dialog"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "#80000000"

    MouseArea { anchors.fill: parent; onClicked: root.cancel() }

    Item {
        id: keys
        anchors.fill: parent
        Keys.onPressed: event => {
            switch (event.key) {
            case Qt.Key_Escape: root.cancel(); break;
            case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space:
                if (root.mode !== "info" && root.focusIndex === 2) root.cancel(); else root.accept();
                break;
            case Qt.Key_Tab: case Qt.Key_Right: case Qt.Key_Down:
                if (root.mode === "ask") root.focusIndex = Math.min(2, root.focusIndex + 1);
                else if (root.mode === "confirm") root.focusIndex = 2;
                break;
            case Qt.Key_Backtab: case Qt.Key_Left: case Qt.Key_Up:
                if (root.mode === "ask") { root.focusIndex = Math.max(0, root.focusIndex - 1); if (root.focusIndex === 0) field.forceActiveFocus(); }
                else if (root.mode === "confirm") root.focusIndex = 1;
                break;
            default: return;
            }
            event.accepted = true;
        }
    }

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - 48, 460)
        height: body.implicitHeight + 44
        radius: Theme.radius + 6
        color: Theme.glass
        border.color: Theme.line
        MouseArea { anchors.fill: parent }

        ColumnLayout {
            id: body
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 22 }
            spacing: 14

            Text {
                Layout.fillWidth: true
                text: root.title
                color: Theme.text
                font.family: Theme.font
                font.pixelSize: 19
                font.weight: Font.DemiBold
                elide: Text.ElideMiddle
            }

            // ask: the text box
            Rectangle {
                visible: root.mode === "ask"
                Layout.fillWidth: true
                implicitHeight: 46
                radius: 12
                color: Theme.bg2
                border.color: root.focusIndex === 0 ? Theme.accent : Theme.line
                TextInput {
                    id: field
                    anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.text
                    selectionColor: Theme.accent
                    selectedTextColor: Theme.accentInk
                    font.pixelSize: 16
                    font.family: Theme.font
                    clip: true
                    onActiveFocusChanged: if (activeFocus) root.focusIndex = 0
                    Keys.onReturnPressed: root.accept()
                    Keys.onEnterPressed: root.accept()
                    Keys.onEscapePressed: root.cancel()
                    Keys.onTabPressed: { root.focusIndex = 1; keys.forceActiveFocus(); }
                    Keys.onDownPressed: { root.focusIndex = 1; keys.forceActiveFocus(); }
                }
            }
            Text {
                visible: root.mode === "ask"
                Layout.fillWidth: true
                text: "Controller: Y opens the keyboard"
                color: Theme.muted
                font.family: Theme.font
                font.pixelSize: 12
            }

            // confirm: what will happen
            Text {
                visible: root.mode === "confirm" && root.message !== ""
                Layout.fillWidth: true
                text: root.message
                color: Theme.muted
                font.family: Theme.font
                font.pixelSize: 14
                wrapMode: Text.WordWrap
            }

            // info: the details
            GridLayout {
                visible: root.mode === "info"
                Layout.fillWidth: true
                columns: 2
                columnSpacing: 16
                rowSpacing: 8
                Repeater {
                    model: root.rows.length * 2
                    delegate: Text {
                        required property int index
                        readonly property var pair: root.rows[Math.floor(index / 2)]
                        Layout.fillWidth: index % 2 === 1
                        Layout.alignment: Qt.AlignTop
                        text: pair ? String(pair[index % 2]) : ""
                        color: index % 2 === 0 ? Theme.muted : Theme.text
                        font.family: Theme.font
                        font.pixelSize: 14
                        wrapMode: index % 2 === 1 ? Text.WrapAnywhere : Text.NoWrap
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: 4
                spacing: 8
                Item { Layout.fillWidth: true }
                DlgButton { visible: root.mode !== "info"; label: "Cancel"; lit: root.focusIndex === 2; onPicked: root.cancel() }
                DlgButton { label: root.mode !== "info" ? root.okLabel : "Close"; primary: true; lit: root.mode === "info" || root.focusIndex === 1; onPicked: root.accept() }
            }
        }
    }

    component DlgButton: Rectangle {
        id: b
        property string label
        property bool primary: false
        property bool lit: false
        signal picked()
        implicitWidth: bt.implicitWidth + 36
        implicitHeight: 42
        radius: 10
        color: primary ? (ba.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent) : (ba.containsMouse ? Theme.panel3 : Theme.panel2)
        border.width: 2
        border.color: lit ? (primary ? Theme.text : Theme.accent) : "transparent"
        Text { id: bt; anchors.centerIn: parent; text: b.label; color: b.primary ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 14; font.bold: b.primary }
        MouseArea { id: ba; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: b.picked() }
    }
}

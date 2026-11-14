// The Run box's page (loaded by WinTools.qml, which does the opening: wt.act, wt.remember…). Windows' Run dialog in
// Hearth's look: "Open:" with suggestions as you type (your recent entries, Windows commands, %folders%, apps),
// OK / Cancel / Browse…. What a command opens is in win-commands.json; what's typed is worked out by run-map.js.
// Keyboard and controller: type (Y opens the on-screen keyboard), Up / Down pick a suggestion, Tab fills it in,
// Enter (A) opens, Esc (B) closes; Down from the field past the suggestions reaches the buttons.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "run-map.js" as RunMap

FocusScope {
    id: v
    focus: true
    property string text: ""
    property int sel: -1            // the highlighted suggestion (-1: what's typed)
    property int btn: -1            // the focused button (0 OK, 1 Cancel, 2 Browse…), -1: the field
    property string error: ""
    readonly property var env: ({
        home: wt.home,
        installed: n => wt.installed(n),
        fromWindows: t => wt.fromWindows(t),
        apps: wt.launcher ? wt.launcher.apps.map(a => ({ id: a.id, name: a.name })) : []
    })
    readonly property var hits: RunMap.suggest(text, wt.data, env, wt.history, 6)
    readonly property string mono: "JetBrains Mono, DejaVu Sans Mono, monospace"

    function focusField() { btn = -1; field.forceActiveFocus(); field.selectAll(); }
    function setText(t) { field.text = t; v.text = t; sel = -1; error = ""; field.cursorPosition = field.text.length; field.forceActiveFocus(); }
    function stateJson() { return JSON.stringify({ text: text, hits: hits, sel: sel, error: error, history: wt.history }); }

    // Open what's typed (or the highlighted suggestion).
    function go() {
        const entry = (sel >= 0 && hits[sel] ? hits[sel].cmd : text).trim();
        if (!entry) return;
        const r = RunMap.resolve(entry, wt.data, env);
        if (!r) return;
        if (r.kind === "error" || r.kind === "missing") { error = r.text; return; }
        if (r.kind === "command") { checkCmd.entry = entry; checkCmd.argv = r.argv; checkCmd.command = ["sh", "-c", 'command -v "$1" >/dev/null 2>&1', "sh", r.argv[0]]; checkCmd.running = true; return; }
        if (!wt.act(r)) { error = "Hearth couldn't open that right now. Try again in a moment."; return; }
        wt.remember(entry);
        wt.closeRun();
    }
    Process {
        id: checkCmd
        property string entry: ""
        property var argv: []
        onExited: code => {
            if (code !== 0) { v.error = "Hearth can't find “" + entry + "”. Check the name, then try again."; return; }
            Quickshell.execDetached(argv);
            wt.remember(entry);
            wt.closeRun();
        }
    }

    // Browse…: Hearth Files' Open dialog (picker.qml, as every app's), then the picked file goes in the field.
    readonly property string runDir: Quickshell.env("XDG_RUNTIME_DIR") || "/tmp"
    Process {
        id: browse
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null;
                try { r = JSON.parse(text.trim() || "{}"); } catch (e) { r = null; }
                if (r && r.paths && r.paths.length) v.setText(String(r.paths[0]).replace(/^file:\/\//, ""));
                else v.focusField();
            }
        }
    }
    function openBrowse() {
        if (browse.running) return;
        const req = { kind: "open", title: "Browse", accept: "Open", multiple: false, directory: false, filters: [], filter: -1,
                      folder: wt.home, name: "", files: [], choices: [], app: "", parent: "", raw: [] };
        const rq = runDir + "/hearth-run-browse-req.json", rs = runDir + "/hearth-run-browse-res.json";
        browse.command = ["sh", "-c", 'rm -f "$3"; printf "%s" "$1" > "$2" && env HEARTH_PICK_REQUEST="$2" HEARTH_PICK_RESULT="$3" quickshell -p "$4" >/dev/null 2>&1; cat "$3" 2>/dev/null; rm -f "$2" "$3"',
                          "sh", JSON.stringify(req), rq, rs, wt.shellDir + "/picker.qml"];
        browse.running = true;
    }

    Rectangle { anchors.fill: parent; color: Theme.bg }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        ColumnLayout {
            Layout.fillWidth: true
            Layout.margins: 18
            Layout.bottomMargin: 0
            spacing: 12
            RowLayout {
                spacing: 14
                Rectangle {
                    Layout.alignment: Qt.AlignTop
                    implicitWidth: 40; implicitHeight: 40; radius: Theme.r(8)
                    color: Theme.panel2
                    Icon { anchors.centerIn: parent; name: "window"; size: 24; color: Theme.accent }
                }
                Text {
                    Layout.fillWidth: true
                    text: "Type the name of a program, folder, document or website, and Hearth will open it for you."
                    wrapMode: Text.WordWrap; lineHeight: 1.2
                    color: Theme.text; opacity: 0.85; font.family: Theme.font; font.pixelSize: 13
                }
            }
            RowLayout {
                spacing: 10
                Text { text: "Open:"; Layout.preferredWidth: 44; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 36
                    radius: Theme.r(6)
                    color: Theme.bg2
                    border.width: field.activeFocus ? 2 : 1
                    border.color: field.activeFocus ? Theme.accent : Theme.line
                    TextInput {
                        id: field
                        objectName: "runField"
                        anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                        verticalAlignment: TextInput.AlignVCenter
                        clip: true
                        focus: true
                        color: Theme.text; selectionColor: Theme.accent; selectedTextColor: Theme.accentInk
                        font.family: v.mono; font.pixelSize: 14
                        onTextChanged: { v.text = text; v.sel = -1; v.error = ""; }
                        Keys.onPressed: event => {
                            switch (event.key) {
                            case Qt.Key_Down:
                                if (v.sel < v.hits.length - 1) v.sel++;
                                else { v.sel = -1; v.btn = 0; buttons.forceActiveFocus(); }
                                break;
                            case Qt.Key_Up: if (v.sel >= 0) v.sel--; break;
                            case Qt.Key_Tab:
                                if (v.hits.length) { const i = v.sel >= 0 ? v.sel : 0; v.setText(v.hits[i].cmd); }
                                break;
                            case Qt.Key_Return: case Qt.Key_Enter: v.go(); break;
                            case Qt.Key_Escape: wt.closeRun(); break;
                            default: return;
                            }
                            event.accepted = true;
                        }
                    }
                }
            }
        }

        // Suggestions as you type
        Rectangle {
            Layout.fillWidth: true
            Layout.leftMargin: 72; Layout.rightMargin: 18; Layout.topMargin: 8
            visible: v.hits.length > 0
            implicitHeight: hitCol.implicitHeight + 12
            radius: Theme.r(8)
            color: Theme.panel2
            border.color: Theme.line
            Column {
                id: hitCol
                x: 6; y: 6
                width: parent.width - 12
                spacing: 1
                Repeater {
                    model: v.hits
                    delegate: Rectangle {
                        id: hit
                        required property var modelData
                        required property int index
                        width: hitCol.width
                        height: 30
                        radius: Theme.r(6)
                        color: v.sel === index ? Theme.panel3 : hma.containsMouse ? Qt.rgba(Theme.panel3.r, Theme.panel3.g, Theme.panel3.b, 0.6) : "transparent"
                        border.width: v.sel === index ? 1 : 0
                        border.color: Theme.accent
                        Row {
                            anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter }
                            spacing: 10
                            Text { width: 160; elide: Text.ElideRight; text: hit.modelData.cmd; color: Theme.text; font.family: v.mono; font.pixelSize: 12 }
                            Text { text: "→"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                            Text { width: hitCol.width - 220; elide: Text.ElideRight; text: hit.modelData.opens; color: Theme.text; opacity: 0.85; font.family: Theme.font; font.pixelSize: 12 }
                        }
                        MouseArea {
                            id: hma
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: { v.sel = hit.index; v.go(); }
                        }
                    }
                }
            }
        }

        // Why something couldn't be opened
        Text {
            Layout.fillWidth: true
            Layout.leftMargin: 72; Layout.rightMargin: 18; Layout.topMargin: 10
            visible: v.error !== ""
            text: v.error
            wrapMode: Text.WordWrap
            color: Theme.flame; font.family: Theme.font; font.pixelSize: 13
        }

        Item { Layout.fillHeight: true }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 60
            color: Theme.panel
            Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }
            FocusScope {
                id: buttons
                anchors.fill: parent
                Keys.onPressed: event => {
                    switch (event.key) {
                    case Qt.Key_Left: v.btn = Math.max(0, v.btn - 1); break;
                    case Qt.Key_Right: v.btn = Math.min(2, v.btn + 1); break;
                    case Qt.Key_Up: v.focusField(); break;
                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space:
                        if (v.btn === 0) v.go(); else if (v.btn === 1) wt.closeRun(); else v.openBrowse();
                        break;
                    case Qt.Key_Escape: wt.closeRun(); break;
                    default: return;
                    }
                    event.accepted = true;
                }
                Row {
                    anchors { right: parent.right; rightMargin: 16; verticalCenter: parent.verticalCenter }
                    spacing: 8
                    Repeater {
                        model: ["OK", "Cancel", "Browse…"]
                        delegate: Rectangle {
                            id: b
                            required property string modelData
                            required property int index
                            readonly property bool lit: buttons.activeFocus && v.btn === index
                            implicitWidth: Math.max(index === 0 ? 76 : 86, bl.implicitWidth + 36)
                            implicitHeight: 34
                            radius: Theme.r(6)
                            color: index === 0 ? Theme.accent : bma.containsMouse ? Theme.panel3 : "transparent"
                            border.width: lit ? 2 : index === 0 ? 0 : 1
                            border.color: lit ? (index === 0 ? Theme.text : Theme.accent) : Theme.line
                            Text {
                                id: bl
                                anchors.centerIn: parent
                                text: b.modelData
                                color: b.index === 0 ? Theme.accentInk : Theme.text
                                font.family: Theme.font; font.pixelSize: 13; font.weight: b.index === 0 ? Font.DemiBold : Font.Normal
                            }
                            MouseArea {
                                id: bma
                                anchors.fill: parent
                                hoverEnabled: true
                                onClicked: { if (b.index === 0) v.go(); else if (b.index === 1) wt.closeRun(); else v.openBrowse(); }
                            }
                        }
                    }
                }
            }
        }
    }
}

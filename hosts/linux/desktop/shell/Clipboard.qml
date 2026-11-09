// Clipboard history for Desktop Mode (Hearth 1.10 look), like Windows 11's Super+V: what you copied lately (text and
// pictures, with the app it came from and when), newest first with your pinned items on top. Click an item (or A /
// Enter) to paste it into the app you were in. Pin keeps an item through Clear all and past the 25-item limit.
//
// History is OFF until you turn it on: the switch at the top of this popup, or Settings › Clipboard (the setting
// `clipboardHistory`; a new install's desktop.json starts with it off). Only while it's on does the shell run
// `wl-paste --watch hearth-clipboard store`; copies a password manager marks as secret are never kept.
// Keys and controller: Up/Down items (Up from the first: the switch and Clear all), Left/Right along an item (paste,
// pin, forget), A / Enter does it, X / the Menu key pins or unpins, Delete forgets, B / Esc closes.
// Loaded by shell.qml through a Loader: a problem here only loses the clipboard popup.
//   quickshell ipc -p <shell folder> call clipboard toggle|open|close|state
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: cb
    property bool shown: false
    property var items: []                // [{id, kind: text|image, text, path, app, time, pinned, dims}]
    property bool available: true
    property bool loading: false
    property int current: 0               // -1 the top row (the switch, Clear all), else an item
    property int column: 0                // on an item: 0 paste, 1 pin, 2 forget; on the top row: 0 switch, 1 Clear all
    property bool confirmClear: false
    property real now: Date.now() / 1000
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-clipboard").toString().replace(/^file:\/\//, ""))
    ExtrasPal { id: pal }

    function open() { shown = true; current = items.length ? 0 : -1; column = 0; confirmClear = false; now = Date.now() / 1000; refresh(); }
    function close() { shown = false; }
    function toggle() { shown ? close() : open(); }
    onShownChanged: SettingsNav.clipShown = shown
    Component.onCompleted: SettingsNav.clipboard = cb

    // ------------------------------------------------ keeping the history (only while it's on)
    property bool tools: false
    Process { running: true; command: ["sh", "-c", "command -v wl-paste && command -v wl-copy"]; onExited: code => cb.tools = code === 0 }
    readonly property bool isOn: Settings.loaded && Settings.clipboardHistory
    readonly property bool wanted: tools && isOn
    onWantedChanged: { watcher.running = wanted; noteApp(); }
    Process {
        id: watcher
        // wl-paste runs the helper with each copy on its standard input (CLIPBOARD_STATE=sensitive for a secret one).
        command: ["wl-paste", "--watch", "python3", cb.helper, "store"]
        onExited: if (cb.wanted) retry.start()
    }
    Timer { id: retry; interval: 3000; onTriggered: if (cb.wanted && !watcher.running) watcher.running = true }
    // The app in front, for "Firefox · 2 min ago": its name in a small file in your runtime folder, only while history is on.
    readonly property var activeWin: ToplevelManager.activeToplevel
    readonly property string activeName: {
        const t = activeWin;
        if (!t) return "";
        const id = String(t.appId || "");
        if (/quickshell/i.test(id)) return String(t.title || "Hearth");
        const e = AppIcons.entryFor(id);
        return e && e.name ? e.name : id;
    }
    onActiveNameChanged: noteApp()
    function noteApp() {
        if (!wanted || !activeName) return;
        const f = (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hearth-clipboard-app";
        Quickshell.execDetached(["sh", "-c", 'printf "%s\\n" "$1" > "$0"', f, activeName.slice(0, 80)]);
    }

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
                // The item you just pinned or unpinned stays selected where it moved to.
                if (cb.keepId !== "") { const k = cb.items.findIndex(x => x.id === cb.keepId); if (k >= 0) cb.current = k; cb.keepId = ""; }
                if (cb.current >= cb.items.length) cb.current = cb.items.length - 1;
            }
        }
    }
    function refresh() { if (!lister.running) { loading = true; lister.running = true; } }
    Process {
        id: worker
        stdout: StdioCollector {
            onStreamFinished: { const err = text.split("\n").find(l => l.startsWith("error: ")); if (err) SettingsNav.toast(err.slice(7)); }
        }
        onExited: cb.refresh()
    }
    function run(args) { if (worker.running) return false; worker.command = ["python3", helper].concat(args); worker.running = true; return true; }
    function forget(i) {
        const it = items[i];
        if (!it || !run(["delete", it.id])) return;
        items = items.filter((x, k) => k !== i);
        current = Math.min(current, items.length - 1);
        column = 0;
    }
    property string keepId: ""
    function togglePin(i) {
        const it = items[i];
        if (!it) return;
        if (run([it.pinned ? "unpin" : "pin", it.id])) keepId = it.id;
    }
    function clearAll() {
        if (!confirmClear) { confirmClear = true; return; }
        confirmClear = false;
        items = items.filter(x => x.pinned);
        current = items.length ? 0 : -1;
        run(["clear"]);
    }
    function paste(i) {
        const it = items[i];
        if (!it) return;
        close();
        // Detached: the popup has closed and the app you were in has the keyboard back when Ctrl+V is pressed.
        Quickshell.execDetached(["python3", helper, "paste", it.id]);
    }
    // The switch: what you choose is kept (Settings.qml merges just this key); never turned on by anything else.
    function setOn(v) { Settings.clipboardHistory = v; if (v) refresh(); }
    function activate() {
        if (current < 0) { if (column === 0) setOn(!isOn); else clearAll(); return; }
        if (column === 1) togglePin(current);
        else if (column === 2) forget(current);
        else paste(current);
    }
    function ago(t) {
        const s = Math.max(0, now - (t || 0));
        if (s < 60) return "just now";
        if (s < 3600) return Math.floor(s / 60) + " min ago";
        if (s < 86400) return Math.floor(s / 3600) + " h ago";
        if (s < 172800) return "yesterday";
        return Math.floor(s / 86400) + " days ago";
    }
    // Commands, addresses and code in a typewriter font, like the mockup.
    function looksLikeCode(t) {
        const s = String(t || "").trim();
        return (/^\S+$/.test(s) && /[\/.:_=]/.test(s)) || /^(sudo|git|cd|ls|apt|dnf|flatpak|python3?|pip|npm|cargo|ssh|curl|wget|systemctl)\s/.test(s) || /[{};]\s*$/.test(s);
    }

    IpcHandler {
        target: "clipboard"
        function toggle(): void { cb.toggle(); }
        function open(): void { cb.open(); }
        function close(): void { cb.close(); }
        function state(): string { return JSON.stringify({ shown: cb.shown, on: cb.isOn, watching: watcher.running, items: cb.items.length, pinned: cb.items.filter(x => x.pinned).length }); }
    }

    PanelWindow {
        id: win
        visible: cb.shown
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-clipboard"
        WlrLayershell.keyboardFocus: cb.shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "transparent"
        onVisibleChanged: if (visible) keys.forceActiveFocus()

        MouseArea { anchors.fill: parent; onClicked: cb.close() }   // click outside closes

        Item {
            id: keys
            focus: true
            Keys.onPressed: e => {
                const n = cb.items.length;
                switch (e.key) {
                case Qt.Key_Down: case Qt.Key_Tab:
                    if (cb.current < n - 1) { cb.current++; cb.column = 0; } cb.confirmClear = false; break;
                case Qt.Key_Up: case Qt.Key_Backtab:
                    if (cb.current > -1) { cb.current--; cb.column = 0; } cb.confirmClear = false; break;
                case Qt.Key_Left: cb.column = Math.max(0, cb.column - 1); cb.confirmClear = false; break;
                case Qt.Key_Right: cb.column = Math.min(cb.current < 0 ? (n ? 1 : 0) : 2, cb.column + 1); break;
                case Qt.Key_Home: cb.current = n ? 0 : -1; cb.column = 0; break;
                case Qt.Key_End: cb.current = n - 1; cb.column = 0; break;
                case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: cb.activate(); break;
                case Qt.Key_Menu: case Qt.Key_X: case Qt.Key_P: if (cb.current >= 0) cb.togglePin(cb.current); break;
                case Qt.Key_Delete: if (cb.current >= 0) cb.forget(cb.current); break;
                case Qt.Key_Escape: cb.close(); break;
                default: return;
                }
                if (cb.current >= 0) list.positionViewAtIndex(cb.current, ListView.Contain);
                e.accepted = true;
            }
        }

        Rectangle {
            id: card
            anchors.centerIn: parent
            width: Math.min(parent.width - 32, 460)
            height: Math.min(parent.height - 96, 600)
            radius: Theme.r(12)
            color: pal.card
            border.color: pal.edge
            MouseArea { anchors.fill: parent }

            ColumnLayout {
                anchors { fill: parent; margins: 12 }
                spacing: 8

                // Clipboard · [History switch] · Clear all
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Text { Layout.fillWidth: true; text: "Clipboard"; color: pal.text; font.family: pal.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                    Rectangle {
                        id: sw
                        readonly property bool lit: cb.current === -1 && cb.column === 0
                        implicitWidth: swRow.implicitWidth + 16; implicitHeight: 28
                        radius: Theme.r(6)
                        color: lit ? pal.sel : swM.containsMouse ? pal.box : "transparent"
                        border.color: lit ? pal.accent : "transparent"
                        Row {
                            id: swRow
                            anchors.centerIn: parent
                            spacing: 8
                            Text { anchors.verticalCenter: parent.verticalCenter; text: "History"; color: pal.muted; font.family: pal.font; font.pixelSize: 12 }
                            Rectangle {
                                anchors.verticalCenter: parent.verticalCenter
                                width: 34; height: 18; radius: 9
                                color: cb.isOn ? pal.accent : pal.line
                                Rectangle { width: 14; height: 14; radius: 7; y: 2; x: cb.isOn ? 18 : 2; color: "#ffffff"; Behavior on x { NumberAnimation { duration: 120 } } }
                            }
                        }
                        MouseArea { id: swM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { cb.current = -1; cb.column = 0; cb.setOn(!cb.isOn); } }
                    }
                    Rectangle {
                        visible: cb.items.length > 0
                        readonly property bool lit: cb.current === -1 && cb.column === 1
                        implicitWidth: clr.implicitWidth + 20; implicitHeight: 28
                        radius: Theme.r(6)
                        color: lit ? pal.sel : clrM.containsMouse ? pal.box : "transparent"
                        border.color: lit ? pal.accent : "transparent"
                        Text { id: clr; anchors.centerIn: parent; text: cb.confirmClear ? "Clear? Press again" : "Clear all"; color: cb.confirmClear ? pal.warn : pal.muted; font.family: pal.font; font.pixelSize: 13 }
                        MouseArea { id: clrM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { cb.current = -1; cb.column = 1; cb.clearAll(); } }
                    }
                }

                ListView {
                    id: list
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: 8
                    model: cb.items
                    boundsBehavior: Flickable.StopAtBounds
                    delegate: Rectangle {
                        id: row
                        required property var modelData
                        required property int index
                        readonly property bool lit: cb.current === index
                        readonly property bool code: modelData.kind === "text" && cb.looksLikeCode(modelData.text)
                        width: ListView.view.width
                        height: body.implicitHeight + 20
                        radius: Theme.r(8)
                        color: lit ? pal.sel : rowMouse.containsMouse ? Qt.lighter(pal.box, 1.1) : pal.box
                        border.width: lit && cb.column === 0 ? 1 : 0
                        border.color: pal.accent
                        MouseArea {
                            id: rowMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: cb.paste(row.index)
                        }
                        ColumnLayout {
                            id: body
                            anchors { left: parent.left; right: rowTools.left; top: parent.top; margins: 10; leftMargin: 12; rightMargin: 8 }
                            spacing: 6
                            Text {
                                visible: row.modelData.kind === "text"
                                Layout.fillWidth: true
                                text: String(row.modelData.text || "").replace(/\r/g, "").trim()
                                color: pal.text
                                font.family: row.code ? pal.mono : pal.font
                                font.pixelSize: row.code ? 12 : 13
                                lineHeight: 1.15
                                wrapMode: Text.WrapAnywhere; maximumLineCount: 2; elide: Text.ElideRight
                                textFormat: Text.PlainText
                            }
                            Rectangle {
                                visible: row.modelData.kind === "image"
                                Layout.preferredWidth: 120; Layout.preferredHeight: 64
                                radius: Theme.r(6)
                                clip: true
                                gradient: Gradient { orientation: Gradient.Horizontal; GradientStop { position: 0; color: "#2a475e" }
                                    GradientStop { position: 1; color: "#3fb8c9" } }
                                Image {
                                    anchors.fill: parent
                                    source: row.modelData.kind === "image" && row.modelData.path ? "file://" + row.modelData.path : ""
                                    fillMode: Image.PreserveAspectCrop
                                    sourceSize: Qt.size(240, 128)
                                    asynchronous: true
                                }
                                Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                                    height: 18; color: "#80000000" }
                                Text {
                                    anchors { left: parent.left; bottom: parent.bottom; leftMargin: 6; bottomMargin: 3 }
                                    text: "Picture" + (row.modelData.dims ? " · " + row.modelData.dims.replace("x", "×") : "")
                                    color: "#ffffff"; font.family: pal.font; font.pixelSize: 11
                                }
                            }
                            Text {
                                Layout.fillWidth: true
                                text: (row.modelData.app ? row.modelData.app + " · " : "") + cb.ago(row.modelData.time)
                                color: pal.muted; font.family: pal.font; font.pixelSize: 11
                                elide: Text.ElideRight
                            }
                        }
                        Row {
                            id: rowTools
                            anchors { right: parent.right; top: parent.top; margins: 8 }
                            spacing: 2
                            Rectangle {
                                readonly property bool lit: row.lit && cb.column === 1
                                width: 28; height: 28; radius: Theme.r(6)
                                color: lit ? pal.card : pinM.containsMouse ? pal.card : "transparent"
                                border.color: lit ? pal.accent : "transparent"
                                Icon { anchors.centerIn: parent; name: "pin"; size: 14; color: row.modelData.pinned ? pal.accent : pal.muted }
                                MouseArea { id: pinM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { cb.current = row.index; cb.column = 1; cb.togglePin(row.index); } }
                            }
                            Rectangle {
                                readonly property bool lit: row.lit && cb.column === 2
                                visible: row.lit || rowMouse.containsMouse || delM.containsMouse || pinM.containsMouse
                                width: 28; height: 28; radius: Theme.r(6)
                                color: lit ? pal.card : delM.containsMouse ? pal.card : "transparent"
                                border.color: lit ? pal.accent : "transparent"
                                Icon { anchors.centerIn: parent; name: "close"; size: 13; color: pal.muted }
                                MouseArea { id: delM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: cb.forget(row.index) }
                            }
                        }
                    }
                    // Empty, off or missing tools.
                    Column {
                        anchors.centerIn: parent
                        width: parent.width - 40
                        spacing: 14
                        visible: cb.items.length === 0
                        Text {
                            width: parent.width
                            horizontalAlignment: Text.AlignHCenter
                            wrapMode: Text.WordWrap
                            text: !cb.tools && !cb.loading ? "Clipboard history needs wl-clipboard, which isn't installed."
                                : !cb.isOn ? "Clipboard history is off.\nTurn it on to keep what you copy (text and pictures) and paste it again with Super+V."
                                : cb.loading ? "Loading…"
                                : "Nothing copied yet. Copy some text or a picture and it shows up here."
                            color: pal.muted; font.family: pal.font; font.pixelSize: 13; lineHeight: 1.2
                        }
                        Rectangle {
                            visible: !cb.isOn && cb.tools
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: onTxt.implicitWidth + 28; height: 34; radius: Theme.r(6)
                            color: onM.containsMouse ? Qt.lighter(pal.accent, 1.1) : pal.accent
                            Text { id: onTxt; anchors.centerIn: parent; text: "Turn on clipboard history"; color: Theme.accentInk; font.family: pal.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                            MouseArea { id: onM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: cb.setOn(true) }
                        }
                    }
                }

                Text {
                    Layout.fillWidth: true
                    text: "Ⓐ paste  Ⓧ pin  ◀ ▶ pin or forget  Ⓑ close · pinned items stay when you clear"
                    color: pal.muted; font.family: pal.font; font.pixelSize: 11
                    elide: Text.ElideRight
                }
            }
        }
    }
}

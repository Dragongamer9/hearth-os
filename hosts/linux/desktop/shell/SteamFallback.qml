// "Steam Game Mode didn't start": shown once when the session opened Mantel because Steam Game Mode couldn't start
// (nimbus-session: Steam missing, safe graphics, Steam ending right away, or failing twice). The reason is the
// session's note $XDG_RUNTIME_DIR/hearth-steam-mode-failed; $XDG_RUNTIME_DIR/hearth-steam-notice-shown marks it as
// shown, so switching to Desktop Mode later doesn't bring it back. "Try Steam Game Mode again" runs
// `nimbus-mode steam` (a request you make always runs, and a good run ends the session's pause). Controller: Up/Down
// and A; B closes it. docs/STEAM-GAME-MODE.md "When Steam Game Mode can't start".
// Hearth Game Mode (docs/HEARTH-GAME-MODE.md): when you picked it and it couldn't start either (the session went on to
// Steam Game Mode, then here), or went straight here (no Steam), its reason shows too, once (hearth-game-mode notice
// desktop), with "Try Hearth Game Mode again" (`nimbus-mode hearth`). Your choice is never changed here.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
    id: root
    property bool shown: false
    property string reason: ""
    property string hearthReason: ""   // Hearth Game Mode's, when you picked it and it didn't start
    property int sel: 0
    readonly property string gmHelper: decodeURIComponent(Qt.resolvedUrl("hearth-game-mode").toString().replace(/^file:\/\//, ""))
    readonly property var items: (reason !== "" ? [{ icon: "pad", label: "Try Steam Game Mode again", sub: "Steam's gamepad screens, like SteamOS", cmd: "steam" }] : [])
        .concat(hearthReason !== "" ? [{ icon: "pad", label: "Try Hearth Game Mode again", sub: "Hearth's own Game Mode, the one you picked", cmd: "hearth" }] : [])
        .concat([{ icon: "close", label: "Stay in Desktop Mode", sub: "Game Mode on the taskbar or the power menu tries again any time", cmd: "" }])
    function pick(i) {
        shown = false;
        const c = items[i] ? items[i].cmd : "";
        if (c !== "") Quickshell.execDetached(["nimbus-mode", c]);
    }
    // Once, a few seconds after Mantel starts (after the live USB's welcome, which never has this note).
    Process {
        id: check
        command: ["sh", "-c", 'r="${XDG_RUNTIME_DIR:-/tmp}"; f="$r/hearth-steam-mode-failed"; s="$r/hearth-steam-notice-shown"; if [ -s "$f" ] && ! { [ -e "$s" ] && [ ! "$f" -nt "$s" ]; }; then : >"$s"; printf "steam:"; head -c 300 "$f" | tr "\n" " "; echo; fi; [ -f "$1" ] && python3 "$1" notice desktop 2>/dev/null | sed "s/^/hearth:/"', "sh", root.gmHelper]
        stdout: StdioCollector { onStreamFinished: {
            const st = text.match(/^steam:(.*)$/m), hm = text.match(/^hearth:(\{.*\})$/m);
            let h = ""; try { h = hm ? (JSON.parse(hm[1]).text || "") : ""; } catch (e) {}
            root.reason = st ? st[1].trim() : ""; root.hearthReason = h.trim();
            if (root.reason !== "" || root.hearthReason !== "") { root.sel = 0; root.shown = true; }
        } }
    }
    Timer { interval: 1500; running: true; onTriggered: check.running = true }

    visible: shown
    color: "transparent"
    anchors { top: true; bottom: true; left: true; right: true }
    exclusiveZone: -1
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "hearth-steam-fallback"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None

    Rectangle { anchors.fill: parent; color: Qt.rgba(0, 0, 0, 0.5) }
    Rectangle {
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, 620)
        height: col.implicitHeight + 56
        radius: 24
        color: Theme.glass
        border.color: Theme.line
        focus: root.shown
        Keys.onPressed: e => {
            if (e.key === Qt.Key_Down || e.key === Qt.Key_Tab) root.sel = (root.sel + 1) % root.items.length;
            else if (e.key === Qt.Key_Up || e.key === Qt.Key_Backtab) root.sel = (root.sel + root.items.length - 1) % root.items.length;
            else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) root.pick(root.sel);
            else if (e.key === Qt.Key_Escape) root.shown = false;
            else return;
            e.accepted = true;
        }
        ColumnLayout {
            id: col
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 28 }
            spacing: 10
            ColumnLayout {
                spacing: 4
                Layout.bottomMargin: 6
                Text { text: root.reason !== "" ? "Steam Game Mode didn't start" : "Hearth Game Mode didn't start"; color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold }
                Text { visible: root.reason !== ""; Layout.fillWidth: true; text: "Why: " + root.reason + ". You're in Desktop Mode, so you can fix it here (install or update Steam, connect to the internet) and try again."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
                Text { visible: root.hearthReason !== ""; Layout.fillWidth: true; text: (root.reason !== "" ? "Hearth Game Mode, the one you picked, didn't start before that either: " : "Why: ") + root.hearthReason + ". Your choice is still Hearth Game Mode (Settings › Gaming › Game Mode)."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
            }
            Repeater {
                model: root.items
                delegate: Rectangle {
                    id: it
                    required property var modelData
                    required property int index
                    readonly property bool lit: root.sel === index
                    Layout.fillWidth: true
                    implicitHeight: 64
                    radius: 14
                    color: lit ? Theme.accent : area.containsMouse ? Theme.panel3 : Theme.panel2
                    RowLayout {
                        anchors { fill: parent; leftMargin: 18; rightMargin: 18 }
                        spacing: 14
                        Icon { name: it.modelData.icon; size: 22; color: it.lit ? Theme.accentInk : Theme.text }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Text { text: it.modelData.label; color: it.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                            Text { Layout.fillWidth: true; text: it.modelData.sub; color: it.lit ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
                        }
                    }
                    MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; onEntered: root.sel = it.index; onClicked: root.pick(it.index) }
                }
            }
        }
    }
}

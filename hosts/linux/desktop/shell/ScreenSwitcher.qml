// Super+P: the screen switcher (1.14), Hearth's own: This screen only, Duplicate, Extend, Second screen only, drawn
// as little screens in Hearth's look. Super+P again (or the arrows / D-pad) moves along, Enter / A picks, Esc / B
// closes. A choice changes the screens at once and waits for "Keep these settings?" (15 s, as Settings › Display):
// not kept in time, it goes back by itself and says so, so a screen that can't show it never stays dark. Kept, it's
// saved in screens.json (hearth-screens keep, the one source of truth). Duplicate (mirroring) says why it can't be
// picked (neither labwc nor KWin 6.3 can mirror). Both compositors: hearth-screens plan works out the changes.
// Loaded by shell.qml through a Loader: a problem here only loses this.
//   quickshell ipc -p <shell folder> call screens switcher | next | pick main|mirror|extend|other | keep | back | close | state
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: sw
    property bool shown: false
    property int current: 0
    property var sinfo: null
    property var prev: null           // {outputs: {...}}: what to go back to
    property var names: []
    property int countdown: 0
    property string problem: ""
    property bool busy: false
    readonly property var outs: sinfo ? (sinfo.outputs || []) : []
    readonly property bool several: outs.length > 1
    readonly property string mirrorWhy: sinfo && sinfo.mirror ? sinfo.mirror.why : ""
    readonly property var choices: [
        { mode: "main", label: "This screen only", sub: several ? mainName() : "The only screen connected", on: true },
        { mode: "mirror", label: "Duplicate", sub: mirrorWhy || "The same picture on both", on: false },
        { mode: "extend", label: "Extend", sub: several ? "One desktop across both" : "Connect another screen", on: several },
        { mode: "other", label: "Second screen only", sub: several ? otherName() : "Connect another screen", on: several }
    ]
    ExtrasPal { id: pal }
    SetJob { id: job; helper: "hearth-screens" }

    function label(o) { return o ? (o.model || o.name) : ""; }
    function mainName() { return label(outs.find(o => o.priority === 1) || outs[0]); }
    function otherName() { const m = outs.find(o => o.priority === 1) || outs[0]; return label(outs.find(o => o !== m)); }

    function open() {
        if (shown) { next(); return; }
        problem = "";
        job.run(["info"], r => {
            sw.sinfo = r;
            const i = sw.choices.findIndex(c => c.mode === r.switch);
            sw.current = i < 0 ? 0 : i;
            sw.shown = true;
        }, e => { SettingsNav.toast("Screens: " + e); });
    }
    function next() { if (countdown === 0) current = (current + 1) % choices.length; }
    function close() { if (countdown > 0) back(false); shown = false; }
    function pick(i) {
        const c = choices[i];
        if (!c || busy || countdown > 0) return;
        if (!c.on) { problem = c.mode === "mirror" ? (mirrorWhy || "Mirroring isn't possible here.") : "Only one screen is connected."; return; }
        problem = "";
        busy = true;
        job.run(["plan", c.mode], p => {
            if (!Object.keys(p.changes.outputs).length) { sw.busy = false; sw.shown = false; SettingsNav.toast(c.label + " is in use already"); return; }
            sw.prev = p.prev;
            sw.names = p.names;
            job.run(["apply", JSON.stringify(p.changes)], r => {
                sw.busy = false;
                sw.sinfo = r;
                if (r.problem) { sw.problem = r.problem; sw.back(false); return; }
                sw.countdown = 15;
            }, e => { sw.busy = false; sw.problem = e; sw.back(false); });
        }, e => { sw.busy = false; sw.problem = e; });
    }
    function keep() {
        if (countdown === 0) return;
        countdown = 0;
        job.run(["keep", JSON.stringify(names)], r => { sw.sinfo = r; sw.shown = false; SettingsNav.toast("Screens: " + sw.choices[sw.current].label + " kept"); },
                e => { sw.problem = "Couldn't save it: " + e; });
    }
    // Back to what was there (Go back, Esc, or by itself when the countdown ends: then it says so).
    function back(byItself) {
        countdown = 0;
        if (!prev) return;
        const p = prev;
        prev = null;
        job.run(["apply", JSON.stringify(p)], r => { sw.sinfo = r; }, e => { sw.problem = e; });
        if (byItself) { problem = "Went back to the screens you had: the new ones weren't kept within 15 seconds."; SettingsNav.toast("Screens: went back to what you had"); }
    }
    Timer { interval: 1000; repeat: true; running: sw.countdown > 0; onTriggered: { sw.countdown--; if (sw.countdown === 0) sw.back(true); } }

    IpcHandler {
        target: "screens"
        function switcher(): void { sw.open(); }
        function next(): void { sw.next(); }
        function pick(mode: string): void { const i = sw.choices.findIndex(c => c.mode === mode); if (i >= 0) { sw.current = i; sw.pick(i); } }
        function keep(): void { sw.keep(); }
        function back(): void { sw.back(false); }
        function close(): void { sw.close(); }
        function state(): string { return JSON.stringify({ shown: sw.shown, current: (sw.choices[sw.current] || {}).mode || "", countdown: sw.countdown, problem: sw.problem, screens: sw.outs.map(o => o.name + (o.enabled ? "" : " (off)")) }); }
    }

    PanelWindow {
        id: panel
        visible: sw.shown
        screen: Quickshell.screens.length ? Quickshell.screens[0] : null
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-screens-switch"
        WlrLayershell.keyboardFocus: sw.shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: pal.dim
        onVisibleChanged: if (visible) keys.forceActiveFocus()

        MouseArea { anchors.fill: parent; onClicked: sw.close() }

        Item {
            id: keys
            focus: true
            Keys.onPressed: e => {
                if (sw.countdown > 0) {
                    if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) sw.keep();
                    else if (e.key === Qt.Key_Escape || e.key === Qt.Key_Backspace) sw.back(false);
                    else return;
                } else if (e.key === Qt.Key_Right || e.key === Qt.Key_Down || e.key === Qt.Key_Tab || (e.key === Qt.Key_P && (e.modifiers & Qt.MetaModifier)))
                    sw.current = (sw.current + 1) % sw.choices.length;
                else if (e.key === Qt.Key_Left || e.key === Qt.Key_Up || e.key === Qt.Key_Backtab)
                    sw.current = (sw.current + sw.choices.length - 1) % sw.choices.length;
                else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) sw.pick(sw.current);
                else if (e.key === Qt.Key_Escape || e.key === Qt.Key_Backspace) sw.close();
                else return;
                e.accepted = true;
            }
        }

        Rectangle {
            id: card
            anchors.centerIn: parent
            width: row.implicitWidth + 48
            height: col.implicitHeight + 44
            radius: Theme.r(18)
            color: pal.card
            border.color: pal.edge
            MouseArea { anchors.fill: parent }

            ColumnLayout {
                id: col
                anchors { top: parent.top; topMargin: 22; horizontalCenter: parent.horizontalCenter }
                spacing: 16
                RowLayout {
                    spacing: 12
                    Icon { name: "display"; size: 24 }
                    Text { text: sw.countdown > 0 ? "Keep these screen settings?" : "Screens"; color: pal.text; font.family: pal.font; font.pixelSize: 22; font.weight: Font.DemiBold }
                    Item { Layout.fillWidth: true }
                    Text { text: sw.countdown > 0 ? "Going back in " + sw.countdown + " s" : "Super+P"; color: pal.muted; font.family: pal.font; font.pixelSize: 14 }
                }
                Row {
                    id: row
                    spacing: 14
                    Repeater {
                        model: sw.choices
                        delegate: Rectangle {
                            id: tile
                            required property var modelData
                            required property int index
                            readonly property bool lit: index === sw.current
                            width: 196; height: 188
                            radius: Theme.r(14)
                            color: lit ? pal.sel : pal.box
                            border.width: lit ? 3 : 1
                            border.color: lit ? pal.accent : pal.line
                            opacity: modelData.on ? 1 : 0.55
                            // The little screens: 1 lit, 2 lit, both (one wide), or the same on both.
                            Item {
                                id: pic
                                width: 150; height: 70
                                anchors { top: parent.top; topMargin: 18; horizontalCenter: parent.horizontalCenter }
                                readonly property string m: tile.modelData.mode
                                Rectangle {
                                    x: pic.m === "extend" ? 4 : 0; y: 8; width: pic.m === "extend" ? 71 : 66; height: 50; radius: 6
                                    color: pic.m === "other" ? "transparent" : SetStyle.tint(pal.accent, tile.lit ? 0.55 : 0.35)
                                    border.width: 2; border.color: pic.m === "other" ? pal.off : pal.accent
                                    Text { anchors.centerIn: parent; text: pic.m === "mirror" ? "A" : "1"; color: pal.text; font.family: pal.font; font.pixelSize: 22; font.weight: Font.Bold }
                                }
                                Rectangle {
                                    x: pic.m === "extend" ? 75 : 84; y: 8; width: pic.m === "extend" ? 71 : 66; height: 50; radius: 6
                                    color: pic.m === "main" ? "transparent" : SetStyle.tint(pal.accent, tile.lit ? 0.55 : 0.35)
                                    border.width: 2; border.color: pic.m === "main" ? pal.off : pal.accent
                                    Text { anchors.centerIn: parent; text: pic.m === "mirror" ? "A" : "2"; color: pal.text; font.family: pal.font; font.pixelSize: 22; font.weight: Font.Bold }
                                }
                            }
                            Column {
                                anchors { left: parent.left; right: parent.right; top: pic.bottom; topMargin: 14; margins: 12 }
                                spacing: 4
                                Text { width: parent.width; horizontalAlignment: Text.AlignHCenter; text: tile.modelData.label; color: pal.text; font.family: pal.font; font.pixelSize: 16; font.weight: Font.DemiBold; elide: Text.ElideRight }
                                Text { width: parent.width; horizontalAlignment: Text.AlignHCenter; text: tile.modelData.sub; color: pal.muted; font.family: pal.font; font.pixelSize: 12; wrapMode: Text.WordWrap; maximumLineCount: 3; elide: Text.ElideRight }
                            }
                            MouseArea {
                                anchors.fill: parent
                                hoverEnabled: true
                                onEntered: if (sw.countdown === 0) sw.current = tile.index
                                onClicked: { sw.current = tile.index; sw.pick(tile.index); }
                            }
                        }
                    }
                }
                RowLayout {
                    visible: sw.countdown > 0
                    Layout.fillWidth: true
                    spacing: 10
                    Item { Layout.fillWidth: true }
                    Rectangle {
                        Layout.preferredWidth: 150; Layout.preferredHeight: 44; radius: 22; color: pal.box; border.color: pal.line
                        Text { anchors.centerIn: parent; text: "Go back"; color: pal.text; font.family: pal.font; font.pixelSize: 15 }
                        MouseArea { anchors.fill: parent; onClicked: sw.back(false) }
                    }
                    Rectangle {
                        Layout.preferredWidth: 150; Layout.preferredHeight: 44; radius: 22; color: pal.accent
                        Text { anchors.centerIn: parent; text: "Keep"; color: Theme.accentInk; font.family: pal.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                        MouseArea { anchors.fill: parent; onClicked: sw.keep() }
                    }
                }
                Text {
                    Layout.fillWidth: true
                    Layout.maximumWidth: row.implicitWidth
                    text: sw.problem !== "" ? sw.problem
                        : sw.countdown > 0 ? "Enter or A: keep  ·  Esc or B: go back"
                        : "Super+P or arrows: choose  ·  Enter or A: switch  ·  Esc or B: close"
                    color: sw.problem !== "" ? pal.warn : pal.muted
                    font.family: pal.font; font.pixelSize: 13
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }
        }
    }
}

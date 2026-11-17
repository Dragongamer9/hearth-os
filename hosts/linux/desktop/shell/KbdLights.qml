// 1.14: the keyboard's RGB lights follow the accent (Settings › Appearance › "Keyboard lights follow the accent", off
// unless you turn it on; KDE Plasma 6.1's feature in Hearth). With the switch off nothing here runs and nothing is
// written. hearth-kbd-lights does the colouring (OpenRGB when installed, else the kernel's multicolour keyboard LEDs)
// and checks the switch in desktop.json again itself. A failure is only logged: nothing else waits for it.
import QtQuick
import Quickshell
import Quickshell.Io

Item {
    id: root
    visible: false
    readonly property bool on: Settings.loaded && Settings.kbdLights
    readonly property string color: hex(Theme.accent)
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-kbd-lights").toString().replace(/^file:\/\//, ""))
    property string sent: ""
    property string last: ""             // what the helper said last (for the checks: shell.qml's "kbdlights" IPC)
    function hex(c) { return "#" + [c.r, c.g, c.b].map(v => ("0" + Math.round(v * 255).toString(16)).slice(-2)).join(""); }
    onOnChanged: { if (on) later.restart(); else { later.stop(); sent = ""; } }
    onColorChanged: if (on) later.restart()
    // A moment after the accent settles (dragging the hue slider changes it many times).
    Timer { id: later; interval: 1200; onTriggered: root.apply() }
    function apply() {
        if (!on) return;
        if (proc.running) { later.restart(); return; }
        if (color === sent) return;
        sent = color;
        proc.command = ["sh", "-c", 'exec python3 "$0" apply "$1" 2>&1', helper, color];
        proc.running = true;
    }
    Process {
        id: proc
        stdout: StdioCollector {
            onStreamFinished: {
                root.last = text.trim();
                if (root.last.indexOf("error:") >= 0) console.warn("Mantel: keyboard lights: " + root.last);
            }
        }
    }
}

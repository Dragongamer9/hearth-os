// Quick Access's Phone tile (docs/PHONE-LINK.md): Phone link on or off and your phone's state in a few words
// ("Pixel 8 · 81%"), read through hearth-phone while Quick Access is open. Tap the tile to turn Phone link on or off;
// its chevron opens Settings › Phone. Hearth Connect's service owns the setting (`hearth-connect --turn-on/--turn-off`).
import QtQuick
import Quickshell
import Quickshell.Io

Item {
    id: ps
    visible: false
    property bool active: false        // Quick Access is open
    property bool installed: true
    property bool on: false
    property string line: "…"
    property bool connected: false
    property bool busy: false
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-phone").toString().replace(/^file:\/\//, ""))

    function refresh() { if (!read.running) read.running = true; }
    function toggle() {
        if (busy || !installed) return;
        busy = true;
        on = !on;
        line = on ? "Turning on…" : "Off";
        set.command = ["python3", helper, on ? "on" : "off"];
        set.running = true;
    }
    onActiveChanged: if (active) refresh()
    Timer { interval: 3000; running: ps.active; repeat: true; onTriggered: ps.refresh() }

    function take(text) {
        try {
            const r = JSON.parse(text.trim().split("\n").pop() || "{}");
            ps.installed = r.installed !== false;
            ps.on = !!r.running;
            ps.connected = (r.phones || []).some(p => p.reachable);
            ps.line = r.line || (ps.on ? "On" : "Off");
        } catch (e) { ps.line = "Not available"; }
    }
    Process { id: read; command: ["python3", ps.helper, "tile"]; stdout: StdioCollector { onStreamFinished: ps.take(text) } }
    Process { id: set; onExited: { ps.busy = false; ps.refresh(); } }
}

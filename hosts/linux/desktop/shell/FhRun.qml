// hearth-firehold-plasma for Firehold on labwc (1.16): one process per question, its last line to the caller's
// callback, the process gone when it ends. Used by the Couch's host (FhCouchHost.qml) and the live modes (FhModes.qml).
// Nothing runs by itself here.
import QtQuick
import Quickshell
import Quickshell.Io

Item {
    id: r
    // (an absolute path: Quickshell keeps a relative one from leaving the shell's folder)
    readonly property string helper: Quickshell.shellDir.replace(/\/shell\/?$/, "") + "/firehold-plasma/hearth-firehold-plasma"
    Component {
        id: procComp
        Process {
            id: p
            property var cb: null
            stdout: StdioCollector { onStreamFinished: { const f = p.cb; p.cb = null; if (f) f(text); } }
            onExited: Qt.callLater(() => p.destroy())
        }
    }
    function run(args, cb) {
        if (!args || !args.length) return;
        const p = procComp.createObject(r, { command: ["python3", helper].concat(args.map(a => String(a))), cb: cb || null });
        if (p) p.running = true;
    }
    function lastJson(t) { try { return JSON.parse(String(t || "").trim().split("\n").pop()); } catch (e) { return null; } }
}

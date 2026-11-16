// The change history (1.14, the new Settings), read from hearth_settings_io.py history: newest first. Settings ›
// Home's Changed lately and All changes show it. Only reads; Undo is SetChangeRow's.
import QtQuick
import Quickshell
import Quickshell.Io
import "settings-changes.js" as Ch

Scope {
    id: c
    property bool running: true
    property int count: 50
    property var items: []
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth_settings_io.py").toString().replace(/^file:\/\//, ""))
    function load() { if (!proc.running) proc.running = true; }
    // The newest change for KEY, or null (the help panel's "last changed").
    function last(key) { return items.find(x => x.key === key) || null; }
    onRunningChanged: if (running) load()
    Component.onCompleted: load()
    Timer { interval: 4000; repeat: true; running: c.running; onTriggered: c.load() }
    Process {
        id: proc
        command: ["python3", c.helper, "history", String(c.count)]
        stdout: StdioCollector {
            onStreamFinished: {
                let v = [];
                try { v = JSON.parse(text); } catch (e) { v = []; }
                v = Array.isArray(v) ? v.filter(Ch.shown) : [];
                if (JSON.stringify(v) !== JSON.stringify(c.items)) c.items = v;
            }
        }
    }
}

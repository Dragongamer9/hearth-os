// The safe way QML saves a settings file (docs/SETTINGS-PERSISTENCE.md): only the keys that changed are merged
// into the latest file by hearth_settings_io.py (atomic write, fsync, FILE.bak, under the same lock the Rust and
// Python writers use). FileView.setText wrote the whole object QML had in memory: a value another program had just
// changed was put back, and a file QML had failed to read was replaced with the defaults.
//
//   SettingsWriter { id: w; path: ".../desktop.json" }
//   w.merge({ nightLight: true })        // queued; one helper run at a time, later changes batched
//   w.repair()                           // a file that failed to parse: put back from FILE.bak (told once)
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: w
    property string path: ""
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth_settings_io.py").toString().replace(/^file:\/\//, ""))
    property var pending: ({})
    property var inflight: ({})
    property bool busy: proc.running
    signal saved()

    function merge(changes) {
        const p = Object.assign({}, pending);
        for (const k in changes) p[k] = changes[k] === undefined ? null : changes[k];
        pending = p;
        if (!proc.running) flush();
    }
    // A key still on its way to the file: a reload of the file (from an earlier write) mustn't put its old value back.
    function owns(k) { return pending[k] !== undefined || inflight[k] !== undefined; }
    function flush() {
        const keys = Object.keys(pending);
        if (keys.length === 0 || path === "") return;
        proc.command = ["python3", helper, "merge", path, JSON.stringify(pending)];
        inflight = pending;
        pending = {};
        proc.running = true;
    }
    function repair() {
        if (path === "" || fixer.running) return;
        fixer.command = ["python3", helper, "repair", path];
        fixer.running = true;
    }

    Process {
        id: proc
        onExited: code => {
            if (code !== 0) console.warn("settings: couldn't save " + w.path + " (code " + code + ")");
            w.inflight = {};
            w.saved();
            w.flush();
        }
    }
    Process {
        id: fixer
        onExited: code => console.info("settings: checked " + w.path + " (code " + code + ")")
    }
}

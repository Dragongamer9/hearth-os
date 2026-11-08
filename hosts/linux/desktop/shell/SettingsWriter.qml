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
        if (retry.running) return;   // waiting to try a failed save again: everything goes together then
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

    // A save that failed (the file was locked too long, the disk was briefly full) is tried again, a few times, rather
    // than dropped: dropped, the page kept showing your choice while the file kept the old value, which came back by
    // itself at the next reload or sign-in. Newer changes to the same key win.
    property int failures: 0
    Process {
        id: proc
        onExited: code => {
            if (code !== 0) {
                console.warn("settings: couldn't save " + w.path + " (code " + code + ")" + (w.failures < 5 ? "; trying again" : ""));
                if (w.failures < 5) {
                    w.failures++;
                    w.pending = Object.assign({}, w.inflight, w.pending);
                    w.inflight = {};
                    retry.interval = 1000 * w.failures;
                    retry.restart();
                    return;
                }
            }
            w.failures = 0;
            w.inflight = {};
            w.saved();
            w.flush();
        }
    }
    Timer { id: retry; repeat: false; onTriggered: if (!proc.running) w.flush() }
    Process {
        id: fixer
        onExited: code => console.info("settings: checked " + w.path + " (code " + code + ")")
    }
}

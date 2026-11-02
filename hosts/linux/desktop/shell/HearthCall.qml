// Asks Hearth's core for a Settings page, the same call the archived Game Mode's Settings made, so it behaves the
// same: `nimbus --settings-call <method> <json>` (hearth-core settings_cli.rs; only the calls listed there) answers
// one line, {"ok": true, "value": ...} or {"ok": false, "error": "..."}. One call at a time; more wait their turn.
//   HearthCall { id: core }   core.call("remote.status", {}, v => ..., e => ...)
// openPage(): what has no app of its own yet opens in the Hearth app's window at that page (`nimbus --windowed
// --page <page>`, hearth-core start_page), docs/SETTINGS-MOVE.md "Interim".
import QtQuick
import Quickshell
import Quickshell.Io

Item {
    id: hc
    visible: false
    readonly property bool busy: proc.running || queue.length > 0
    property string what: ""          // what the running call is for ("Pairing…"): pages show it
    property var queue: []

    function call(method, args, done, failed, label) {
        queue = queue.concat([{ method: method, args: JSON.stringify(args || {}), done: done || null, failed: failed || null, label: label || "" }]);
        if (!proc.running) next();
    }
    function next() {
        if (proc.running) return;
        if (!queue.length) { what = ""; return; }
        const j = queue[0];
        queue = queue.slice(1);
        proc.job = j; proc.answer = null; proc.tail = "";
        what = j.label;
        proc.command = ["nimbus", "--settings-call", j.method, j.args];
        proc.running = true;
    }
    function openPage(page, said) {
        Quickshell.execDetached(["nimbus", "--windowed", "--page", page]);
        SettingsNav.toast(said || "Opening it in the Hearth app…");
    }

    Process {
        id: proc
        property var job: null
        property var answer: null
        property string tail: ""
        stdout: SplitParser {
            splitMarker: "\n"
            onRead: l => {
                if (l.startsWith("{")) { try { proc.answer = JSON.parse(l); return; } catch (e) {} }
                if (l.trim()) proc.tail = l.trim();
            }
        }
        onExited: code => {
            const j = proc.job, a = proc.answer;
            if (j) {
                if (a && a.ok === true) { if (j.done) j.done(a.value); }
                else if (j.failed) j.failed(a && a.error ? a.error : code === 127 ? "The Hearth app isn't on this system" : (proc.tail || "Hearth couldn't do that"));
            }
            hc.next();
        }
    }
}

// Runs one of Hearth's helpers for a settings page (or Apps) and hands back what it answered: its JSON line,
// or its "error: ..." line. Progress lines ("progress: <0-100> <what>") move `progress` and `line`.
// One command at a time; more wait their turn. With admin: true it goes through pkexec (hearth-admin; Hearth's
// polkit rule lets the person at the device do this without a prompt). Text for the helper's stdin goes
// through the environment (readable only by you), never the command line.
//   SetJob { id: job; helper: "hearth-system" }   job.run(["time"], r => ..., e => ...)
import QtQuick
import Quickshell
import Quickshell.Io

Item {
    id: job
    visible: false
    property string helper                  // a file next to this one ("hearth-system"), or an absolute path
    property bool admin: false
    readonly property bool busy: proc.running || queue.length > 0
    property string what: ""                // what the running command is for (pages show "Saving…" etc.)
    property real progress: -1
    property string line: ""
    property var queue: []
    readonly property string path: helper.startsWith("/") ? helper
        : decodeURIComponent(Qt.resolvedUrl(helper).toString().replace(/^file:\/\//, ""))

    // args: list of strings; done(result), failed(message): optional; input: text for stdin; label: sets `what`.
    function run(args, done, failed, input, label) {
        queue = queue.concat([{ args: args, done: done || null, failed: failed || null, input: input || "", label: label || "" }]);
        if (!proc.running) next();
    }
    function next() {
        if (proc.running) return;
        if (!queue.length) { what = ""; return; }
        const j = queue[0];
        queue = queue.slice(1);
        proc.job = j;
        proc.result = null; proc.err = ""; proc.tail = "";
        what = j.label; progress = -1; line = "";
        proc.environment = { HEARTH_INPUT: j.input, LC_ALL: "C" };
        if (admin) proc.command = ["sh", "-c", 'printf "%s" "$HEARTH_INPUT" | exec pkexec "$@" 2>&1', "sh", path].concat(j.args);
        else proc.command = ["sh", "-c", 'printf "%s" "$HEARTH_INPUT" | exec python3 "$@" 2>&1', "sh", path].concat(j.args);
        proc.running = true;
    }

    Process {
        id: proc
        property var job: null
        property var result: null
        property string err: ""
        property string tail: ""
        stdout: SplitParser {
            splitMarker: "\n"
            onRead: l => {
                const m = l.match(/^progress: ([\d.]+) ?(.*)$/);
                if (m) { job.progress = parseFloat(m[1]) / 100; job.line = m[2]; return; }
                if (l.startsWith("error: ")) { proc.err = l.slice(7); return; }
                if (l.startsWith("{")) { try { proc.result = JSON.parse(l); return; } catch (e) {} }
                if (l.trim() && !l.startsWith("log: ")) proc.tail = l.trim();
            }
        }
        onExited: code => {
            const j = proc.job;
            let err = proc.err;
            if (!err && job.admin && (code === 126 || code === 127) && proc.result === null)
                err = "Not allowed (Hearth's polkit rule is missing, or the request was cancelled).";
            if (!err && code !== 0 && proc.result === null) err = proc.tail || "That didn't work";
            job.progress = -1;
            if (j) {
                if (err) { if (j.failed) j.failed(err); }
                else if (j.done) j.done(proc.result || {});
            }
            job.next();
        }
    }
}

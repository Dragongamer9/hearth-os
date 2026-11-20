// Quickshell's Process (plasma-hearth): runs `command` (an argument list) through Plasma's executable source when
// `running` is set, gives the output to `stdout` (a StdioCollector: all of it; a SplitParser: a line at a time) and
// says `exited(code)`. Setting running to false forgets a run still going (its answer is dropped).
// A program Mantel runs with "--watch N" (hearth-stats: a line every N seconds, while a page shows it) is run once
// every N seconds instead, only while `running` (Plasma's source gives the output when the program ends).
// hearth-files' folder watch ("hearth-files watch DIR", Mantel's desktop icons: a line whenever the folder changes) is
// run as "watch --once DIR" and started again each time it answers, so a file added to the Desktop folder (a game's
// icon from Hearth Games, a download) shows at once here too.
import QtQuick

Item {
    id: p
    property var command: []
    property bool running: false
    property QtObject stdout: null
    property QtObject stderr: null
    property var environment: ({})
    property string workingDirectory: ""
    signal exited(int exitCode, int exitStatus)
    signal started()
    property int gen: 0
    property bool ready: false
    readonly property int watchAt: Array.from(command || []).map(String).indexOf("--watch")
    function startDetached() { Hearth.run(command); }
    function give(out) {
        if (!p.stdout) return;
        if (p.stdout.splitMarker !== undefined) {
            const parts = String(out).split(p.stdout.splitMarker || "\n");
            for (const l of parts) if (l !== "") p.stdout.read(l);
        } else {
            p.stdout.text = out;
            p.stdout.streamFinished();
        }
    }
    function begin() {
        const g = ++gen;
        let cmd = Array.from(command || []).map(String);
        if (!cmd.length) { running = false; return; }
        const w = cmd.indexOf("--watch");
        if (w >= 0) { cmd.splice(w, 2); }
        const fw = cmd.length >= 3 && /hearth-files$/.test(cmd[1]) && cmd[2] === "watch" && cmd[3] !== "--once";
        if (fw) cmd.splice(3, 0, "--once");
        started();
        Hearth.call(cmd, (out, code) => {
            if (!p || g !== p.gen) return;
            p.give(out);
            if (w >= 0) { if (p.running) again.restart(); return; }
            if (fw && code === 0 && String(out).trim() !== "gone") { if (p.running) rewatch.restart(); return; }
            p.running = false;
            p.exited(code, 0);
        });
    }
    Timer { id: rewatch; interval: 100; onTriggered: if (p.running) p.begin() }
    Timer {
        id: again
        interval: Math.max(1, Number(p.watchAt >= 0 ? p.command[p.watchAt + 1] : 2) || 2) * 1000
        onTriggered: if (p.running) p.begin()
    }
    onRunningChanged: { if (!ready) return; if (running) begin(); else { gen++; again.stop(); rewatch.stop(); } }
    Component.onCompleted: { ready = true; if (running) begin(); }
}

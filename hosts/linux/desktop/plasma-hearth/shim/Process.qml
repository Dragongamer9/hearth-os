// Quickshell's Process (plasma-hearth): runs `command` (an argument list) through Plasma's executable source when
// `running` is set, gives the output to `stdout` (a StdioCollector) and says `exited(code)`. Setting running to false
// forgets a run still going (its answer is dropped).
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
    function startDetached() { Hearth.run(command); }
    function begin() {
        const g = ++gen;
        const cmd = Array.from(command || []).map(String);
        if (!cmd.length) { running = false; return; }
        started();
        Hearth.call(cmd, (out, code) => {
            if (g !== p.gen) return;
            if (p.stdout) { p.stdout.text = out; p.stdout.streamFinished(); }
            p.running = false;
            p.exited(code, 0);
        });
    }
    onRunningChanged: { if (!ready) return; if (running) begin(); else gen++; }
    Component.onCompleted: { ready = true; if (running) begin(); }
}

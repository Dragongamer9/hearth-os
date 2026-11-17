// Push to talk (Settings › Sound › Push to talk, 1.14): runs hearth-ptt while the switch is on under Mantel on KWin, so
// the microphone is muted except while you hold the key. Loaded by shell.qml by file name: a problem here only loses
// push to talk. If hearth-ptt stops by itself it's tried again twice, then it says why (the switch stays as you set it;
// Settings › Sound › Push to talk shows the problem, and turning it off and on tries again).
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: ptt
    readonly property bool want: Settings.loaded && Settings.pttOn && Quickshell.env("HEARTH_WM") === "kwin"
    property int fails: 0
    property string problem: ""
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-ptt").toString().replace(/^file:\/\//, ""))
    // Started and stopped here only (never by a binding), so turning it off always stops it.
    function sync() {
        if (want && fails < 3) { if (!proc.running) proc.running = true; }
        else if (proc.running) proc.running = false;
    }
    onWantChanged: { fails = 0; problem = ""; SettingsNav.pttProblem = ""; sync(); }
    Component.onCompleted: sync()
    // Another key: hearth-ptt starts again with it.
    Connections { target: Settings; function onPttKeyChanged() { ptt.fails = 0; if (proc.running) proc.running = false; } }
    Process {
        id: proc
        command: ["python3", ptt.helper, "run", Settings.pttKey]
        stdout: SplitParser { onRead: l => { if (l.startsWith("error: ")) { ptt.problem = l.slice(7); SettingsNav.pttProblem = ptt.problem; } } }
        onExited: code => {
            if (!ptt.want) return;
            if (code !== 0) ptt.fails++;
            if (ptt.fails >= 3) console.warn("Mantel: push to talk stopped: " + (ptt.problem || "hearth-ptt exited " + code));
            else retry.restart();
        }
    }
    Timer { id: retry; interval: 3000; onTriggered: ptt.sync() }
}

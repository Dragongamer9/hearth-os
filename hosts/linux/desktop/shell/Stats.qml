// CPU, GPU, memory and temperatures (and the frame rate when MangoHud logs it) from hearth-stats, every 2 seconds,
// only while `running` (the Gaming look's performance island, the dashboard). Values that can't be read are -1.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: st
    property bool running: false
    property int cpu: -1
    property int gpu: -1
    property int cpuTemp: -1
    property int gpuTemp: -1
    property real ram: -1
    property real ramTotal: -1
    property int fps: -1                   // -1: not readable (no MangoHud log): the FPS item hides
    property var history: []               // the last 24 CPU readings, for a small graph
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-stats").toString().replace(/^file:\/\//, ""))
    Process {
        running: st.running
        command: ["python3", st.helper, "--watch", "2"]
        stdout: SplitParser {
            onRead: line => {
                let o = null;
                try { o = JSON.parse(line); } catch (e) { return; }
                st.cpu = o.cpu !== undefined ? o.cpu : -1;
                st.gpu = o.gpu !== undefined ? o.gpu : -1;
                st.cpuTemp = o.cpuTemp !== undefined ? o.cpuTemp : -1;
                st.gpuTemp = o.gpuTemp !== undefined ? o.gpuTemp : -1;
                st.ram = o.ram !== undefined ? o.ram : -1;
                st.ramTotal = o.ramTotal !== undefined ? o.ramTotal : -1;
                st.fps = o.fps !== undefined ? o.fps : -1;
                if (st.cpu >= 0) st.history = st.history.concat([st.cpu]).slice(-24);
            }
        }
    }
}

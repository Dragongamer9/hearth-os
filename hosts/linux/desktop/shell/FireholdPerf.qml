// Firehold's performance overlay (the Guide panel's Performance overlay: Basic or Full), top left of the game's
// screen: frames a second (when MangoHud logs them), the GPU's use and temperature and its power, from hearth-stats and
// hearth-gputune (read only). Never takes the pointer or the keys.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "firehold.js" as F

PanelWindow {
    id: pf
    property var fh
    readonly property var game: fh.frontGame()
    screen: game && game.screens && game.screens.length ? game.screens[0] : Quickshell.screens[0]
    readonly property real k: Math.min((screen ? screen.width : 1920) / 1920, (screen ? screen.height : 1080) / 1080) || 1
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-firehold-perf"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; left: true }
    margins { top: 28 * k; left: 32 * k }
    implicitWidth: box.width
    implicitHeight: box.height
    color: "transparent"
    mask: Region {}

    property var s: ({})
    Process {
        id: stats
        running: true
        command: ["python3", Quickshell.shellDir + "/hearth-stats", "--watch", "1"]
        stdout: SplitParser { onRead: line => { try { pf.s = JSON.parse(line); } catch (e) {} } }
    }
    readonly property string line: {
        const p = [];
        if (s.fps) p.push(Math.round(s.fps) + " fps", (1000 / s.fps).toFixed(1) + " ms");
        if (s.gpu !== undefined) p.push("GPU " + Math.round(s.gpu) + "%");
        if (s.gpuTemp !== undefined) p.push(Math.round(s.gpuTemp) + " °C");
        if (fh.overlay === "full") {
            if (s.cpu !== undefined) p.push("CPU " + Math.round(s.cpu) + "%");
            if (s.ram !== undefined) p.push(s.ram.toFixed(1) + " GB");
        }
        if (s.gpuPower !== undefined) p.push(Math.round(s.gpuPower) + " W");
        return p.length ? p.join(" · ") : "Reading…";
    }
    Rectangle {
        id: box
        width: t.implicitWidth + 28 * pf.k; height: t.implicitHeight + 16 * pf.k; radius: 10 * pf.k
        color: Qt.rgba(15 / 255, 11 / 255, 9 / 255, 0.6)
        Text { id: t; anchors.centerIn: parent; text: pf.line; color: F.C.green; font.family: "monospace"; font.pixelSize: 15 * pf.k; font.weight: Font.DemiBold }
    }
}

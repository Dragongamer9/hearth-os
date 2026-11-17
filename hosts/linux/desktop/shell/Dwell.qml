// Dwell click (1.14, Settings › Accessibility): rest the pointer and it clicks. hearth-dwell does the work (where the
// pointer is, through a KWin script; the click, through a virtual mouse); this draws its ring at the pointer while it
// fills, on the screen the pointer is on. Clicks go through the ring and it never takes the keyboard. Runs only while
// "dwellClick" is on in accessibility.json (hearth-access saves it) and only on Mantel on KWin; Esc is given back to
// the apps (hearth-dwell release) at every start and whenever the helper ends. Loaded by shell.qml by file name: a
// problem here only loses dwell click.
//   quickshell ipc -p <shell folder> call dwell demo X Y P KIND    (the ring, for checks and pictures)   … call dwell hide
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: root
    readonly property bool kwin: Quickshell.env("HEARTH_WM") === "kwin"
    property bool wanted: false             // accessibility.json "dwellClick"
    property bool gaveUp: false             // the helper said something is missing (exit 2/3/4): not restarted
    property string problem: ""
    onWantedChanged: proc.running = root.kwin && root.wanted && !root.gaveUp
    // The ring: where, how full (0..1) and which click it will be.
    property bool ringOn: false
    property int rx: 0
    property int ry: 0
    property real rp: 0
    property string kind: "left"
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-dwell").toString().replace(/^file:\/\//, ""))

    FileView {
        path: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/accessibility.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            let o = null;
            try { o = JSON.parse(text()); } catch (e) { return; }       // a damaged file changes nothing here
            const on = !!(o && o.dwellClick === true);
            if (on && !root.wanted) root.gaveUp = false;                // turned on again: try again
            root.wanted = on;
        }
    }

    // Esc back to the apps at the start (a helper that was killed may have held it).
    Process { id: release; command: ["python3", root.helper, "release"]; running: root.kwin }

    Process {
        id: proc
        command: ["python3", root.helper, "serve"]
        stdout: SplitParser { onRead: line => root.handle(line) }
        stderr: SplitParser { onRead: line => console.warn("hearth-dwell:", line) }
        onExited: code => {
            root.ringOn = false;
            release.running = false; release.running = root.kwin;
            if (code === 2 || code === 3 || code === 4) root.gaveUp = true;
            else if (root.wanted) again.start();
        }
    }
    Timer { id: again; interval: 3000; onTriggered: if (root.wanted && !proc.running) proc.running = true }

    function handle(line) {
        let m;
        try { m = JSON.parse(line); } catch (e) { return; }
        switch (m.t) {
        case "ready": problem = ""; break;
        case "ring": rx = m.x; ry = m.y; rp = m.p; kind = m.kind || "left"; ringOn = true; break;
        case "hide": ringOn = false; break;
        case "click": ringOn = false; break;
        case "error": problem = m.text || ""; console.warn("Mantel: dwell click:", problem); break;
        }
    }

    IpcHandler {
        target: "dwell"
        function demo(x: int, y: int, p: real, k: string): void { root.rx = x; root.ry = y; root.rp = p; root.kind = k || "left"; root.ringOn = true; }
        function hide(): void { root.ringOn = false; }
    }

    Variants {
        model: Quickshell.screens
        delegate: PanelWindow {
            id: win
            required property var modelData
            screen: modelData
            readonly property int size: 64
            readonly property bool here: root.ringOn && root.rx >= modelData.x && root.rx < modelData.x + modelData.width
                                         && root.ry >= modelData.y && root.ry < modelData.y + modelData.height
            visible: here
            color: "transparent"
            anchors { top: true; left: true }
            margins.left: Math.max(0, root.rx - modelData.x - size / 2)
            margins.top: Math.max(0, root.ry - modelData.y - size / 2)
            implicitWidth: size
            implicitHeight: size
            exclusiveZone: 0
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "hearth-dwell"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
            mask: Region {}                  // clicks go through: the click lands on what's under the pointer

            Canvas {
                id: ring
                anchors.fill: parent
                property real p: root.rp
                onPChanged: requestPaint()
                onPaint: {
                    const c = getContext("2d");
                    const r = width / 2 - 6;
                    c.reset();
                    c.lineCap = "round";
                    c.lineWidth = 6;
                    c.strokeStyle = Qt.rgba(0, 0, 0, 0.45);
                    c.beginPath(); c.arc(width / 2, height / 2, r, 0, Math.PI * 2); c.stroke();
                    c.lineWidth = 4;
                    c.strokeStyle = Qt.rgba(1, 1, 1, 0.35);
                    c.beginPath(); c.arc(width / 2, height / 2, r, 0, Math.PI * 2); c.stroke();
                    c.strokeStyle = Theme.flame;
                    c.beginPath(); c.arc(width / 2, height / 2, r, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * Math.max(0.02, p)); c.stroke();
                }
            }
            // Which click it will be (nothing for a plain left click).
            Rectangle {
                visible: root.kind !== "left"
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottom: parent.bottom
                width: tag.implicitWidth + 10; height: 16; radius: 8
                color: Theme.glass; border.color: Theme.line
                Text {
                    id: tag
                    anchors.centerIn: parent
                    text: ({ double: "Double", right: "Right", middle: "Middle" })[root.kind] || ""
                    color: Theme.text; font.family: Theme.font; font.pixelSize: 10; font.weight: Font.DemiBold
                }
            }
        }
    }
}

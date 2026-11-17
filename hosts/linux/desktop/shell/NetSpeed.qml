pragma Singleton

// The network's speed, down and up, for the bar (1.14: Settings › Bar and dock › Network speed in the bar). Read from
// /proc/net/dev every 2 seconds, only while it's turned on; the network cards' own traffic (Wi-Fi, wired, mobile, USB),
// so VPNs and containers aren't counted twice. Bar.qml and Islands.qml show `text`.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root
    readonly property bool on: Settings.netSpeed
    property string text: ""
    property real lastRx: -1
    property real lastTx: -1
    property real lastAt: 0
    // The bar's right-click "Show / Hide the network speed": your press, saved like the Settings switch.
    function toggle() { Settings.netSpeed = !Settings.netSpeed; }
    function rate(b) { return b >= 1048576 ? (b / 1048576).toFixed(b >= 10485760 ? 0 : 1) + " MB/s" : Math.round(b / 1024) + " KB/s"; }
    Process {
        id: dev
        command: ["cat", "/proc/net/dev"]
        stdout: StdioCollector {
            onStreamFinished: {
                let rx = 0, tx = 0;
                for (const line of text.split("\n")) {
                    const m = line.match(/^\s*([^:\s]+):\s*(\d+)(?:\s+\d+){7}\s+(\d+)/);
                    if (!m || !/^(en|eth|wl|ww|usb)/.test(m[1])) continue;
                    rx += parseFloat(m[2]); tx += parseFloat(m[3]);
                }
                const now = Date.now();
                if (root.lastRx >= 0 && now > root.lastAt) {
                    const s = (now - root.lastAt) / 1000;
                    root.text = "↓ " + root.rate(Math.max(0, rx - root.lastRx) / s) + "  ↑ " + root.rate(Math.max(0, tx - root.lastTx) / s);
                }
                root.lastRx = rx; root.lastTx = tx; root.lastAt = now;
            }
        }
    }
    Timer {
        interval: 2000; repeat: true; triggeredOnStart: true; running: root.on
        onTriggered: { if (!dev.running) dev.running = true; }
        onRunningChanged: { if (!running) { root.lastRx = -1; root.text = ""; } }
    }
}

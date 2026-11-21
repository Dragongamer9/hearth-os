pragma Singleton

// The bar's network and Bluetooth state, read when it changes instead of every 10 and 15 seconds (1.16, the rest
// fixes: docs/LABWC-FEDORA.md). Before, every bar (one per screen, Bar.qml and Islands.qml) started nmcli every 10 s
// and two bluetoothctl runs of up to 3 s each every 15 s, all day, which kept the processor busy at rest. Now:
//  - Network: `nmcli monitor` (NetworkManager tells it about every change; it sleeps otherwise), then one
//    `nmcli -t -f TYPE,STATE,CONNECTION device`, a second after the last change.
//  - Bluetooth: `gdbus monitor` on BlueZ (it wakes only for BlueZ's own signals), then the same bluetoothctl reading
//    as before, two seconds after the last signal; `refreshBt()` right after the bar turns Bluetooth on or off.
//  - When a watcher isn't there or ends, the reading is done every 2 minutes instead (and the watcher tried again
//    then), never faster.
// Reads only; nothing here writes a setting.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root
    property string netKind: ""          // "wifi", "ethernet" or "" (offline)
    property string netName: ""
    property bool btPresent: false
    property bool btPowered: false
    property var btConnected: []         // names of connected devices

    property bool noNet: false           // nmcli isn't installed
    property bool noBt: false            // bluetoothctl or gdbus isn't installed
    function refreshNet() { if (!noNet && !nmRead.running) nmRead.running = true; }
    function refreshBt() { if (!noBt) btSoon.restart(); }

    // ---------------------------------------------------------------- network
    Process {
        id: nmRead
        command: ["nmcli", "-t", "-f", "TYPE,STATE,CONNECTION", "device"]
        stdout: StdioCollector {
            onStreamFinished: {
                let k = "", n = "";
                for (const line of text.split("\n")) {
                    const f = line.split(":");
                    // (nmcli -t puts a backslash before ":" in names)
                    if (f[1] === "connected" && (f[0] === "wifi" || f[0] === "ethernet") && !k) { k = f[0]; n = f.slice(2).join(":").replace(/\\(.)/g, "$1"); }
                }
                root.netKind = k; root.netName = n;
            }
        }
    }
    Process {
        id: nmWatch
        running: true
        command: ["sh", "-c", "command -v nmcli >/dev/null || exit 3; exec nmcli monitor"]
        stdout: SplitParser { onRead: _ => netSoon.restart() }
        // (3: nmcli isn't installed: nothing to watch or read, never tried again)
        onExited: (code, status) => { if (code === 3) root.noNet = true; else slow.start(); }
    }
    Timer { id: netSoon; interval: 1000; onTriggered: root.refreshNet() }

    // ---------------------------------------------------------------- Bluetooth
    Process {
        id: btRead
        command: ["sh", "-c", "command -v bluetoothctl >/dev/null || exit 3; bluetoothctl --timeout 3 show 2>/dev/null | grep -E '^Controller|Powered:'; echo ---; bluetoothctl --timeout 3 devices Connected 2>/dev/null"]
        stdout: StdioCollector {
            onStreamFinished: {
                const parts = text.split("---");
                root.btPresent = /Controller/.test(parts[0] || "");
                root.btPowered = /Powered:\s*yes/.test(parts[0] || "");
                root.btConnected = (parts[1] || "").split("\n").map(l => l.match(/^Device\s+\S+\s+(.+)$/)).filter(Boolean).map(m => m[1].trim());
            }
        }
    }
    Process {
        id: btWatch
        running: true
        command: ["sh", "-c", "command -v gdbus >/dev/null && command -v bluetoothctl >/dev/null || exit 3; exec gdbus monitor --system --dest org.bluez"]
        stdout: SplitParser { onRead: line => { if (/PropertiesChanged|InterfacesAdded|InterfacesRemoved|org\.bluez/.test(line)) btSoon.restart(); } }
        onExited: (code, status) => { if (code === 3) root.noBt = true; else slow.start(); }
    }
    Timer { id: btSoon; interval: 2000; onTriggered: if (!root.noBt && !btRead.running) btRead.running = true }

    // ---------------------------------------------------------------- the first reading, and the slow fallback
    Component.onCompleted: { nmRead.running = true; btSoon.restart(); }
    Timer {
        id: slow
        interval: 120000
        repeat: true
        onTriggered: {
            if (!root.noNet && !nmWatch.running) { root.refreshNet(); nmWatch.running = true; }
            if (!root.noBt && !btWatch.running) { if (!btRead.running) btRead.running = true; btWatch.running = true; }
            if ((root.noNet || nmWatch.running) && (root.noBt || btWatch.running)) stop();
        }
    }
}

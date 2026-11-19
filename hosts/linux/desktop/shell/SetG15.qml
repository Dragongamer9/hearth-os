// Hearth Settings › Bluetooth & devices › Logitech G15 (docs/G15.md): one row that opens the Logitech G15 app
// (hearth-g15-app: G-keys and macros, the keyboard's screen, its lights), with whether the keyboard is plugged in
// (hearth-g15 status, read once when the page opens). Opening the page writes nothing.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Logitech G15"
    property var status: null
    Process {
        id: probe
        running: true
        command: ["sh", "-c", "timeout 5 hearth-g15 status 2>/dev/null || echo '{}'"]
        stdout: StdioCollector { onStreamFinished: { try { page.status = JSON.parse(text.trim() || "{}"); } catch (e) { page.status = {}; } } }
    }
    Process { id: opener; command: ["sh", "-c", "command -v hearth-g15-app >/dev/null && exec hearth-g15-app"] }
    readonly property bool plugged: !!page.status && page.status.plugged === true
    SetRow {
        icon: "kbd"; title: "Logitech G15"; chevron: true
        sub: page.status === null ? "Looking for the keyboard…"
             : (page.plugged ? (page.status.model || "G15") + " plugged in" : "Not plugged in") + " · G-keys and macros, its screen, its lights"
        onActivated: if (!opener.running) opener.running = true
    }
}

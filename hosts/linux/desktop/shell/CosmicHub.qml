// Hearth COSMIC: Hearth Link and Hearth Storage in one card at the bottom right, above the panel (docs/COSMIC.md). The
// short version of Hearth Classic's Settings › Hearth Link, as Hearth Plasma's org.hearth.link / org.hearth.storage
// widgets are: this device, the paired devices, a pairing code, sync now; Hearth Storage on or off, your devices'
// health, back up now. Everything goes through Mantel's own helper, hearth-link-settings (Hearth Link's socket).
// Opened from the panel's Hearth Link / Hearth Storage entries (hearth-cosmic link | storage) or Quick Access.
// Controller and keyboard: Up / Down (D-pad) move between the buttons, A / Enter presses, B / Esc closes.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
    id: root
    property bool shown: false
    property string section: "link"          // link | storage: which part is shown first
    property var st: null
    property var devs: null
    property string message: ""
    property int sel: 0
    readonly property var peers: st && st.peers ? st.peers : []
    readonly property var stc: st && st.storage ? (st.storage.config || {}) : null
    readonly property var devices: devs && devs.devices ? devs.devices : []
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-link-settings").toString().replace(/^file:\/\//, ""))

    function open(which) { section = which || "link"; shown = true; }
    function toggle(which) { if (shown && section === (which || "link")) shown = false; else open(which); }

    visible: shown
    onShownChanged: if (shown) { message = ""; sel = 0; refresh(); keys.forceActiveFocus(); }
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-cosmic-hub"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    MouseArea { anchors.fill: parent; onClicked: root.shown = false }

    function lastLine(t) { const l = String(t || "").trim().split("\n"); return l[l.length - 1] || ""; }
    Process {
        id: statusProc
        command: ["python3", root.helper, "status"]
        stdout: StdioCollector {
            onStreamFinished: {
                const l = root.lastLine(text);
                if (l.startsWith("error: ")) root.st = { error: l.slice(7) };
                else { try { root.st = JSON.parse(l); } catch (e) { root.st = null; } }
                if (root.stc && !devProc.running) devProc.running = true;
            }
        }
    }
    Process {
        id: devProc
        command: ["python3", root.helper, "devices"]
        stdout: StdioCollector { onStreamFinished: { try { const d = JSON.parse(root.lastLine(text)); if (d) root.devs = d; } catch (e) {} } }
    }
    Process {
        id: actProc
        property string done: ""
        stdout: StdioCollector {
            onStreamFinished: {
                const l = root.lastLine(text);
                root.message = l.startsWith("error: ") ? l.slice(7) : actProc.done;
                root.refresh();
            }
        }
    }
    function refresh() { if (!statusProc.running) statusProc.running = true; }
    function act(args, done) {
        if (actProc.running) return;
        actProc.done = done || "";
        actProc.command = ["python3", root.helper].concat(args);
        actProc.running = true;
    }
    Timer { interval: 3000; running: root.shown; repeat: true; onTriggered: root.refresh() }

    function bytes(n) {
        const u = ["B", "KB", "MB", "GB", "TB"]; let i = 0; let v = Number(n) || 0;
        while (v >= 1000 && i < u.length - 1) { v /= 1000; i++; }
        return (i ? v.toFixed(1) : v) + " " + u[i];
    }
    function devLine(d) {
        const b = d.battery;
        return [d.me ? "This device" : d.online ? "Online" : "Off",
                b && b.percent !== undefined && b.percent !== null ? "Battery " + b.percent + "%" : "",
                d.free !== undefined && d.free !== null ? bytes(d.free) + " free" : "",
                d.lastBackup ? "Backed up " + new Date(d.lastBackup).toLocaleDateString() : ""].filter(x => x).join(" · ");
    }

    // The buttons, in the order Up / Down walks them.
    readonly property var actions: {
        const a = [{ id: "tab", label: section === "link" ? "Show Hearth Storage" : "Show Hearth Link" }];
        if (section === "link" && st && st.me) {
            a.push({ id: "pair", label: st.pairing ? "Stop pairing" : "Pair a device" });
            if (peers.length > 0) a.push({ id: "sync", label: "Sync now" });
        }
        if (section === "storage" && stc) {
            a.push({ id: "server", label: stc.server ? "Stop using this device as Hearth Storage" : "Use this device as Hearth Storage" });
            if (!stc.server) a.push({ id: "backup", label: "Back up now" });
        }
        a.push({ id: "close", label: "Close" });
        return a;
    }
    function press(id) {
        switch (id) {
        case "tab": section = section === "link" ? "storage" : "link"; sel = 0; break;
        case "pair": act([st && st.pairing ? "pair-close" : "pair-open"]); break;
        case "sync": act(["sync"], "Syncing with your devices"); break;
        case "server": act(["storage-set", "server", stc && stc.server ? "off" : "on"]); break;
        case "backup": act(["backup-now"], "Backing up to your Hearth Storage"); break;
        case "close": shown = false; break;
        }
    }

    Rectangle {
        id: card
        anchors { right: parent.right; bottom: parent.bottom; rightMargin: 16; bottomMargin: 84 }
        width: Math.min(460, root.width - 32)
        height: body.implicitHeight + 40
        radius: Theme.radius
        color: Theme.glass
        border.color: Theme.line
        MouseArea { anchors.fill: parent }      // clicks inside don't close it

        Item {
            id: keys
            focus: true
            Keys.onPressed: e => {
                if (e.key === Qt.Key_Escape) { root.shown = false; e.accepted = true; }
                else if (e.key === Qt.Key_Down || e.key === Qt.Key_Tab) { root.sel = (root.sel + 1) % root.actions.length; e.accepted = true; }
                else if (e.key === Qt.Key_Up || e.key === Qt.Key_Backtab) { root.sel = (root.sel + root.actions.length - 1) % root.actions.length; e.accepted = true; }
                else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) { root.press(root.actions[root.sel].id); e.accepted = true; }
                else if (e.key === Qt.Key_Left || e.key === Qt.Key_Right) { root.press("tab"); e.accepted = true; }
            }
        }

        ColumnLayout {
            id: body
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 20 }
            spacing: 10

            Text {
                text: root.section === "link" ? "Hearth Link" : "Hearth Storage"
                color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold
            }
            Text {
                Layout.fillWidth: true
                visible: !root.st || !!root.st.error
                text: !root.st ? "Hearth Link isn't running." : (root.st.error || "")
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 15; wrapMode: Text.WordWrap
            }

            // ---- Hearth Link
            Text {
                Layout.fillWidth: true
                visible: root.section === "link" && !!root.st && !!root.st.me
                text: root.st && root.st.me ? "This device: " + root.st.me.name : ""
                color: Theme.text; font.family: Theme.font; font.pixelSize: 16; elide: Text.ElideRight
            }
            Text {
                visible: root.section === "link" && !!root.st && !!root.st.pairing && !!root.st.pairing.code
                text: root.st && root.st.pairing && root.st.pairing.code ? root.st.pairing.code.slice(0, 3) + " " + root.st.pairing.code.slice(3) : ""
                color: Theme.accent; font.family: Theme.font; font.pixelSize: 34; font.weight: Font.Bold
            }
            Text {
                Layout.fillWidth: true
                visible: root.section === "link" && !!root.st && !!root.st.pairing
                text: root.st && root.st.me ? "On the other device: Settings › Hearth Link, pick " + root.st.me.name + " and type this code." : ""
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
            }
            Repeater {
                model: root.section === "link" ? root.peers : []
                delegate: Text {
                    required property var modelData
                    Layout.fillWidth: true
                    text: modelData.name + (modelData.server ? " · Hearth Storage" : "") + "   " + (modelData.online ? "Online" : "Off")
                    color: modelData.online ? Theme.text : Theme.muted; font.family: Theme.font; font.pixelSize: 15; elide: Text.ElideRight
                }
            }
            Text {
                Layout.fillWidth: true
                visible: root.section === "link" && !!root.st && !!root.st.me && root.peers.length === 0
                text: "No devices paired yet. Pair one to share games, saves and files between your Hearth devices."
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
            }

            // ---- Hearth Storage
            Text {
                Layout.fillWidth: true
                visible: root.section === "storage"
                text: !root.stc ? "Hearth Storage works through Hearth Link, which isn't running."
                      : root.stc.server ? "This device is your Hearth Storage: shared folders, games kept for your devices, their backups and saves. Only paired devices; nothing is opened to the internet."
                      : "Using your Hearth Storage for backups and shared folders."
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
            }
            Repeater {
                model: root.section === "storage" ? root.devices : []
                delegate: Column {
                    required property var modelData
                    Layout.fillWidth: true
                    Text { width: parent.width; text: modelData.name + (modelData.server ? " · Hearth Storage" : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 15; elide: Text.ElideRight }
                    Text { width: parent.width; text: root.devLine(modelData); color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                }
            }

            Text {
                Layout.fillWidth: true
                visible: root.message !== ""
                text: root.message
                color: Theme.flame; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
            }

            // Big, controller-friendly buttons.
            Repeater {
                model: root.actions
                delegate: Rectangle {
                    id: btn
                    required property var modelData
                    required property int index
                    readonly property bool lit: root.sel === index
                    Layout.fillWidth: true
                    implicitHeight: 48
                    radius: Theme.radius - 4
                    color: lit ? Theme.accent : (hov.containsMouse ? Theme.panel3 : Theme.panel2)
                    border.color: lit ? Qt.lighter(Theme.accent, 1.2) : Theme.line
                    Text {
                        anchors.centerIn: parent
                        text: btn.modelData.label
                        color: btn.lit ? Theme.accentInk : Theme.text
                        font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold
                    }
                    MouseArea {
                        id: hov
                        anchors.fill: parent
                        hoverEnabled: true
                        onEntered: root.sel = btn.index
                        onClicked: root.press(btn.modelData.id)
                    }
                }
            }
        }
    }
}

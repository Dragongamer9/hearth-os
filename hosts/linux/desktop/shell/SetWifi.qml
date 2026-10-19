// Hearth Settings › Wi-Fi (NetworkManager, through nmcli).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Wi-Fi"
    property bool on: false
    property var nets: []          // [{ssid, signal, secure, inUse}]
    property var saved: []
    property bool scanning: false
    property string asking: ""     // the network we're asking a password for
    property string busy: ""
    property string message: ""

    // nmcli -t escapes ':' inside fields as '\:'
    function fields(line) {
        const out = []; let cur = "";
        for (let i = 0; i < line.length; i++) {
            const c = line[i];
            if (c === "\\" && i + 1 < line.length) { cur += line[++i]; continue; }
            if (c === ":") { out.push(cur); cur = ""; continue; }
            cur += c;
        }
        out.push(cur);
        return out;
    }
    function refresh() { radio.running = true; savedProc.running = true; scan.running = true; scanning = true; }
    Component.onCompleted: refresh()
    // Not while a password is being typed (the field would be rebuilt under you).
    Timer { interval: 15000; running: page.visible && page.asking === ""; repeat: true; onTriggered: page.refresh() }

    Process {
        id: radio
        command: ["nmcli", "radio", "wifi"]
        stdout: StdioCollector { onStreamFinished: page.on = text.trim() === "enabled" }
    }
    Process {
        id: savedProc
        command: ["nmcli", "-t", "-f", "NAME,TYPE", "connection", "show"]
        stdout: StdioCollector { onStreamFinished: page.replaceList("saved", text.split("\n").map(l => page.fields(l)).filter(f => /wireless/.test(f[1] || "")).map(f => f[0])) }
    }
    Process {
        id: scan
        command: ["nmcli", "-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "device", "wifi", "list", "--rescan", "auto"]
        stdout: StdioCollector {
            onStreamFinished: {
                const seen = {}, out = [];
                for (const l of text.split("\n")) {
                    const f = page.fields(l);
                    if (f.length < 4 || !f[1] || seen[f[1]]) continue;
                    seen[f[1]] = true;
                    out.push({ ssid: f[1], signal: parseInt(f[2]) || 0, secure: f[3] !== "" && f[3] !== "--", inUse: f[0] === "*" });
                }
                out.sort((a, b) => (b.inUse - a.inUse) || (b.signal - a.signal));
                page.replaceList("nets", out);
                page.scanning = false;
            }
        }
    }
    Process {
        id: action
        property string okText
        stdout: StdioCollector {}
        stderr: StdioCollector { id: actErr }
        onExited: code => { page.busy = ""; page.message = code === 0 ? action.okText : (actErr.text.trim().split("\n").pop() || "That didn't work"); page.refresh(); }
    }
    function run(cmd, okText) { action.okText = okText; action.command = cmd; action.running = true; }
    function connect(n, pw) {
        busy = n.ssid; asking = ""; message = "";
        if (pw) run(["nmcli", "device", "wifi", "connect", n.ssid, "password", pw], "Connected to " + n.ssid);
        else if (saved.includes(n.ssid)) run(["nmcli", "connection", "up", "id", n.ssid], "Connected to " + n.ssid);
        else run(["nmcli", "device", "wifi", "connect", n.ssid], "Connected to " + n.ssid);
    }
    function bars(s) { return s > 75 ? "▂▄▆█" : s > 50 ? "▂▄▆" : s > 25 ? "▂▄" : "▂"; }

    SetSwitch {
        icon: "wifi"; title: "Wi-Fi"; sub: page.on ? (page.nets.find(n => n.inUse) ? "Connected to " + page.nets.find(n => n.inUse).ssid : "Not connected") : "Off"
        checked: page.on
        onToggled: v => { page.on = v; page.run(["nmcli", "radio", "wifi", v ? "on" : "off"], v ? "Wi-Fi on" : "Wi-Fi off"); }
    }
    Text { visible: page.message !== ""; text: page.message; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; Layout.leftMargin: 4 }

    SetHeader { text: page.scanning ? "Networks · looking…" : "Networks"; visible: page.on }
    Repeater {
        model: page.on ? page.nets : []
        delegate: ColumnLayout {
            id: netItem
            required property var modelData
            Layout.fillWidth: true
            spacing: 8
            SetRow {
                icon: "wifi"
                title: netItem.modelData.ssid
                sub: netItem.modelData.inUse ? "Connected" : page.busy === netItem.modelData.ssid ? "Connecting…" : page.saved.includes(netItem.modelData.ssid) ? "Saved" : netItem.modelData.secure ? "Secured" : "Open"
                value: page.bars(netItem.modelData.signal) + (netItem.modelData.secure ? "  🔒" : "")
                onActivated: {
                    const n = netItem.modelData;
                    if (n.inUse) page.run(["nmcli", "connection", "down", "id", n.ssid], "Disconnected from " + n.ssid);
                    else if (n.secure && !page.saved.includes(n.ssid)) { page.asking = n.ssid; Qt.callLater(() => pw.focusInput()); }
                    else page.connect(n, "");
                }
            }
            RowLayout {
                visible: page.asking === netItem.modelData.ssid
                Layout.fillWidth: true
                Layout.leftMargin: 52
                spacing: 8
                SetField { id: pw; placeholder: "Password for " + netItem.modelData.ssid; secret: true; onSubmitted: t => page.connect(netItem.modelData, t) }
                SetRow { Layout.preferredWidth: 140; Layout.fillWidth: false; title: "Connect"; chevron: false; onActivated: page.connect(netItem.modelData, pw.text) }
            }
        }
    }
    SetRow { visible: page.on; icon: "restart"; title: "Look again"; chevron: false; onActivated: page.refresh() }
    Text { visible: !page.on; text: "Turn on Wi-Fi to see networks."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
}

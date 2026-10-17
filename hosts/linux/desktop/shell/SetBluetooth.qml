// Hearth Settings › Bluetooth (BlueZ, through bluetoothctl): turn on/off, your devices, pair new ones.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Bluetooth"
    property bool on: false
    property var paired: []        // [{mac, name, connected}]
    property var found: []
    property bool scanning: false
    property string busy: ""
    property string message: ""

    function parseDevices(text) {
        return text.split("\n").map(l => l.match(/^Device ([0-9A-F:]{17}) (.*)$/i)).filter(Boolean).map(m => ({ mac: m[1], name: m[2] }));
    }
    function refresh() { show.running = true; pairedProc.running = true; }
    Component.onCompleted: refresh()
    Timer { interval: 8000; running: page.visible; repeat: true; onTriggered: page.refresh() }

    Process {
        id: show
        command: ["bluetoothctl", "show"]
        stdout: StdioCollector { onStreamFinished: page.on = /Powered: yes/.test(text) }
    }
    Process {
        id: pairedProc
        command: ["sh", "-c", "bluetoothctl devices Paired; echo ---; bluetoothctl devices Connected"]
        stdout: StdioCollector {
            onStreamFinished: {
                const parts = text.split("---"), p = parts[0], c = parts[1];
                const conn = page.parseDevices(c || "").map(d => d.mac);
                page.paired = page.parseDevices(p || "").map(d => ({ mac: d.mac, name: d.name, connected: conn.includes(d.mac) }));
            }
        }
    }
    Process {
        id: scanProc
        command: ["sh", "-c", "bluetoothctl --timeout 12 scan on >/dev/null; bluetoothctl devices"]
        stdout: StdioCollector {
            onStreamFinished: {
                const known = page.paired.map(d => d.mac);
                page.found = page.parseDevices(text).filter(d => !known.includes(d.mac) && !/^([0-9A-F]{2}-){5}/i.test(d.name));
                page.scanning = false;
            }
        }
    }
    Process {
        id: action
        property string okText
        stdout: StdioCollector { id: actOut }
        onExited: code => { page.busy = ""; page.message = code === 0 ? action.okText : (actOut.text.trim().split("\n").pop() || "That didn't work"); page.refresh(); }
    }
    function run(cmd, okText) { action.okText = okText; action.command = ["sh", "-c", cmd]; action.running = true; }

    SetSwitch {
        icon: "bt"; title: "Bluetooth"; sub: page.on ? (page.paired.some(d => d.connected) ? page.paired.filter(d => d.connected).map(d => d.name).join(", ") + " connected" : "On") : "Off"
        checked: page.on
        onToggled: v => { page.on = v; page.run("bluetoothctl power " + (v ? "on" : "off"), v ? "Bluetooth on" : "Bluetooth off"); }
    }
    Text { visible: page.message !== ""; text: page.message; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; Layout.leftMargin: 4 }

    SetHeader { text: "Your devices"; visible: page.on }
    Repeater {
        model: page.on ? page.paired : []
        delegate: SetRow {
            required property var modelData
            icon: "pad"
            title: modelData.name
            sub: page.busy === modelData.mac ? "Working…" : modelData.connected ? "Connected · select to disconnect" : "Not connected · select to connect"
            value: modelData.connected ? "Connected" : ""
            onActivated: { page.busy = modelData.mac; page.run("bluetoothctl " + (modelData.connected ? "disconnect " : "connect ") + modelData.mac, modelData.name + (modelData.connected ? " disconnected" : " connected")); }
        }
    }
    Text { visible: page.on && !page.paired.length; text: "No devices yet. Put a controller or headphones in pairing mode, then look for devices."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }

    SetHeader { text: page.scanning ? "New devices · looking…" : "New devices"; visible: page.on }
    SetRow {
        visible: page.on
        icon: "bt"; title: page.scanning ? "Looking for devices…" : "Look for devices"; sub: "Put the device in pairing mode first"; chevron: false
        onActivated: if (!page.scanning) { page.scanning = true; page.found = []; scanProc.running = true; }
    }
    Repeater {
        model: page.on ? page.found : []
        delegate: SetRow {
            required property var modelData
            icon: "dot"; title: modelData.name; sub: page.busy === modelData.mac ? "Pairing…" : "Select to pair"
            onActivated: { page.busy = modelData.mac; page.run(`bluetoothctl pair ${modelData.mac} && bluetoothctl trust ${modelData.mac} && bluetoothctl connect ${modelData.mac}`, modelData.name + " is paired"); }
        }
    }
    SetHeader { text: "Manage"; visible: page.on && page.paired.length > 0 }
    Repeater {
        model: page.on ? page.paired : []
        delegate: SetRow {
            required property var modelData
            icon: "close"; danger: true; title: "Forget " + modelData.name; chevron: false
            onActivated: page.run("bluetoothctl remove " + modelData.mac, modelData.name + " forgotten")
        }
    }
}

// Task Manager › Performance › <device> › … & settings (1.11, the approved mockups tm-3 … tm-7 in
// docs/mockups/next-update): CPU (power mode, turbo boost, top speed, temperature and throttling), Disk (what it is,
// TRIM, health in Hearth Disks), Network (metered, a hotspot), GPU (the driver, tuning with LACT) and Battery (health,
// cycles, a charge limit, what happens when it runs low). Reads hearth-power, hearth-hwset and nmcli; changes only what
// you press. Opening it writes nothing.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Item {
    id: ds
    property string kind: "cpu"           // cpu | disk | net | gpu | battery
    property var dev: null                // the Performance page's device (its sample)
    property var sample: null             // the whole sample (the CPU's temperature)
    property var hw: null                 // hearth-hwset status
    property var power: null              // hearth-power status
    property var net: null                // {name, metered, hotspot}
    property var drives: null             // hearth-hwset drives (Optimize drives)
    property string note: ""
    property bool noteBad: false
    implicitHeight: col.implicitHeight
    readonly property string shellDir: decodeURIComponent(Qt.resolvedUrl(".").toString().replace(/^file:\/\//, "")).replace(/\/$/, "")
    function refresh() {
        if (!hwProc.running) hwProc.running = true;
        if (kind === "cpu" && !powerProc.running) powerProc.running = true;
        if (kind === "net" && !netProc.running) netProc.running = true;
        if (kind === "disk" && !drvProc.running) drvProc.running = true;
    }
    onKindChanged: refresh()
    Component.onCompleted: refresh()
    Timer { interval: 5000; repeat: true; running: ds.visible; onTriggered: ds.refresh() }
    function lastJson(t) { try { return JSON.parse(t.trim().split("\n").pop()); } catch (e) { return null; } }
    Process { id: hwProc; command: ["python3", "/usr/libexec/nimbus/hearth-hwset", "status"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.hw = r; } } }
    Process { id: drvProc; command: ["python3", "/usr/libexec/nimbus/hearth-hwset", "drives"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.drives = r; } } }
    Process { id: powerProc; command: ["python3", ds.shellDir + "/hearth-power", "status"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.power = r; } } }
    Process {
        id: netProc
        command: ["sh", "-c", "c=$(nmcli -t -f NAME,TYPE connection show --active 2>/dev/null | grep -v ':loopback' | head -n1 | cut -d: -f1); m=$(nmcli -g connection.metered connection show \"$c\" 2>/dev/null); h=$(nmcli -t -f NAME connection show --active 2>/dev/null | grep -c '^Hotspot'); printf '{\"name\":\"%s\",\"metered\":\"%s\",\"hotspot\":%s}\\n' \"$c\" \"$m\" \"$h\""]
        stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.net = r; } }
    }
    Process {
        id: act
        property string okText: "Saved ✓"
        stdout: StdioCollector {
            onStreamFinished: {
                const l = text.trim().split("\n").pop() || "";
                if (l.indexOf("error: ") === 0) { ds.note = l.slice(7); ds.noteBad = true; }
                else { ds.note = act.okText; ds.noteBad = false; }
                ds.refresh();
            }
        }
    }
    function run(cmd, ok) { if (act.running) return; act.okText = ok || "Saved ✓"; act.command = cmd; act.running = true; }
    function hwset(key, value) { run(["pkexec", "/usr/libexec/nimbus/hearth-hwset", "set", key, String(value)]); }

    component Card: Rectangle {
        id: card
        default property alias body: inner.data
        property string title
        property string sub
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignTop
        implicitHeight: head.implicitHeight + inner.implicitHeight + 46
        radius: Theme.r(10); color: Theme.panel; border.color: Theme.line
        ColumnLayout {
            id: head
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 18 }
            spacing: 4
            Text { text: card.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
            Text { Layout.fillWidth: true; visible: card.sub !== ""; text: card.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
        }
        ColumnLayout { id: inner; anchors { left: parent.left; right: parent.right; top: head.bottom; leftMargin: 18; rightMargin: 18; topMargin: 12 } spacing: 12 }
    }
    component Line: RowLayout {
        id: line
        property string title
        property string sub
        default property alias tailItems: tail.data   // not "right": Item.right is FINAL
        Layout.fillWidth: true
        spacing: 12
        ColumnLayout {
            Layout.fillWidth: true; spacing: 2
            Text { text: line.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
            Text { Layout.fillWidth: true; visible: line.sub !== ""; text: line.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; wrapMode: Text.WordWrap }
        }
        RowLayout { id: tail; spacing: 8 }
    }
    component Chips: Row {
        id: chips
        property var options: []
        property string value
        signal chose(string v)
        Repeater {
            model: chips.options
            delegate: Rectangle {
                required property var modelData
                readonly property bool on: chips.value === modelData[0]
                width: ct.implicitWidth + 22; height: 30; radius: Theme.r(6)
                color: on ? Theme.accent : cm.containsMouse ? Theme.panel3 : Theme.panel2; border.color: Theme.line
                Text { id: ct; anchors.centerIn: parent; text: parent.modelData[1]; color: parent.on ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 12 }
                MouseArea { id: cm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: chips.chose(parent.modelData[0]) }
            }
        }
    }
    component Toggle: Rectangle {
        property bool checked
        signal flipped()
        width: 42; height: 22; radius: 11
        color: checked ? Theme.accent : "transparent"; border.width: checked ? 0 : 1; border.color: Theme.muted
        Rectangle { width: 14; height: 14; radius: 7; y: 4; x: parent.checked ? parent.width - 18 : 4; color: parent.checked ? Theme.accentInk : Theme.muted }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: parent.flipped() }
    }
    component Value: Text { color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
    component Btn: Rectangle {
        property string text
        signal clicked()
        width: bt.implicitWidth + 26; height: 32; radius: Theme.r(8)
        color: bm.containsMouse ? Theme.panel3 : Theme.panel2; border.color: Theme.line
        Text { id: bt; anchors.centerIn: parent; text: parent.text; color: Theme.text; font.family: Theme.font; font.pixelSize: 12 }
        MouseArea { id: bm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: parent.clicked() }
    }

    ColumnLayout {
        id: col
        width: parent.width
        spacing: 14

        GridLayout {
            Layout.fillWidth: true
            columns: ds.width > 900 ? 2 : 1
            columnSpacing: 14; rowSpacing: 14

            // ---- CPU
            Card {
                visible: ds.kind === "cpu"
                title: "Power mode"; sub: "How hard the processor works. Changes the fans and battery life too."
                Line {
                    title: "Mode"
                    Chips {
                        options: [["power-saver", "Battery saver"], ["balanced", "Balanced"], ["performance", "Performance"]]
                        value: ds.power && ds.power.mode ? (ds.power.mode.active || "") : ""
                        onChose: v => ds.run(["python3", ds.shellDir + "/hearth-power", "mode", v], "Power mode: " + v)
                    }
                }
                Line {
                    visible: !!(ds.hw && ds.hw.turbo.can)
                    title: "Turbo boost"; sub: "faster bursts, more heat"
                    Toggle { checked: !!(ds.hw && ds.hw.turbo.on); onFlipped: ds.hwset("cpu.turbo", checked ? "off" : "on") }
                }
            }
            Card {
                visible: ds.kind === "cpu"
                title: "Limits & heat"; sub: "Keep the PC cool and quiet, or let it go all out."
                Line {
                    visible: !!(ds.hw && ds.hw.maxPct.can)
                    title: "Max processor speed"; sub: "100% = no limit"
                    Chips {
                        options: [["50", "50%"], ["70", "70%"], ["85", "85%"], ["100", "100%"]]
                        value: ds.hw ? String(ds.hw.maxPct.value) : "100"
                        onChose: v => ds.hwset("cpu.maxPct", v)
                    }
                }
                Line { title: "Temperature now"; Value { text: ds.sample && ds.sample.cpu && ds.sample.cpu.temp ? Math.round(ds.sample.cpu.temp) + "°C" : ds.sample && ds.sample.temps && ds.sample.temps.cpu ? Math.round(ds.sample.temps.cpu) + "°C" : "–" } }
                Line {
                    title: "Throttling"; sub: "times the processor slowed itself down because of heat, since it started"
                    Value { text: ds.hw ? (ds.hw.throttled ? String(ds.hw.throttled) : "none") : "–"; color: ds.hw && !ds.hw.throttled ? "#5fd08a" : Theme.text }
                }
                Text {
                    Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                    text: "One app taking the whole processor? Right-click it in Processes: Set priority, Run on these cores, or Limit CPU / memory."
                }
            }

            // ---- Disk
            Card {
                visible: ds.kind === "disk"
                title: "This drive"; sub: ds.dev && ds.dev.disk ? (ds.dev.disk.model || ds.dev.disk.name || "") : ""
                Line { title: "Type"; Value { text: ds.dev && ds.dev.disk ? (ds.dev.disk.kind || ds.dev.disk.type || "–") : "–" } }
                Line { title: "Health"; sub: "the drive's own self-check, with a warning when it starts to fail"; Btn { text: "Open Hearth Disks"; onClicked: Quickshell.execDetached(["hearth-disks"]) } }
                // Optimize drives the Windows way (1.11): SSDs and NVMe drives are TRIMmed weekly, never defragmented.
                Line {
                    title: "Optimize"
                    sub: { const d = ds.drives ? ds.drives.drives.find(x => ds.dev && ds.dev.disk && ds.dev.disk.name && ds.dev.disk.name.indexOf(x.name) === 0) : null;
                           return (d ? d.kind + " · " + d.what : "TRIM keeps SSDs fast") + (ds.drives && ds.drives.timer.last ? " · last run " + ds.drives.timer.last : ""); }
                    Btn { text: "Optimize now"; onClicked: ds.run(["pkexec", "/usr/libexec/nimbus/hearth-hwset", "optimize"], "Optimized ✓ (TRIM done)") }
                }
            }

            // ---- Network
            Card {
                visible: ds.kind === "net"
                title: "This connection"; sub: ds.net && ds.net.name ? ds.net.name : "Not connected"
                Line {
                    title: "Metered connection"; sub: "updates and big downloads wait while this is on (a phone's hotspot, a data cap)"
                    Toggle {
                        checked: !!(ds.net && /^yes/.test(ds.net.metered))
                        onFlipped: if (ds.net && ds.net.name) ds.run(["nmcli", "connection", "modify", ds.net.name, "connection.metered", checked ? "no" : "yes"], checked ? "Not metered" : "Metered ✓")
                    }
                }
            }
            Card {
                visible: ds.kind === "net"
                title: "Mobile hotspot"; sub: "Share this PC's internet over Wi-Fi with your phone or another PC."
                Line {
                    title: ds.net && ds.net.hotspot ? "On" : "Off"; sub: "network name: " + (Quickshell.env("HOSTNAME") || "Hearth") + " · the password is shown in Settings › Wi-Fi"
                    Toggle {
                        checked: !!(ds.net && ds.net.hotspot)
                        onFlipped: ds.run(checked ? ["nmcli", "connection", "down", "Hotspot"] : ["nmcli", "device", "wifi", "hotspot", "ssid", Quickshell.env("HOSTNAME") || "Hearth"], checked ? "Hotspot off" : "Hotspot on ✓")
                    }
                }
            }

            // ---- GPU
            Card {
                visible: ds.kind === "gpu"
                title: "Graphics card"; sub: ds.dev && ds.dev.gpu ? ds.dev.gpu.name : ""
                Line { title: "Driver"; Value { text: ds.dev && ds.dev.gpu ? (ds.dev.gpu.driver || ds.dev.gpu.vendor || "–") : "–" } }
                Line { title: "Temperature"; Value { text: ds.dev && ds.dev.gpu && ds.dev.gpu.temp ? Math.round(ds.dev.gpu.temp) + "°C" : "–" } }
                Line {
                    title: "Tuning"; sub: "fan curves, power limits and undervolting (LACT); off until you change something, every change undoable"
                    // LACT's daemon is off until you open tuning the first time (it asks for your password once to turn it on).
                    Btn { text: "Open GPU tuning"; onClicked: Quickshell.execDetached(["sh", "-c", "command -v lact >/dev/null || { notify-send -a Hearth 'GPU tuning' 'LACT isn'\\''t installed on this Hearth.'; exit 0; }; systemctl is-active -q lactd || pkexec systemctl enable --now lactd; exec lact gui"]) }
                }
            }

            // ---- Battery
            Card {
                visible: ds.kind === "battery"
                title: "Battery health"; sub: ds.hw && ds.hw.batteries.length ? (ds.hw.batteries[0].model || ds.hw.batteries[0].name) + (ds.hw.batteries[0].tech ? " · " + ds.hw.batteries[0].tech : "") : "No battery"
                Line { title: "Health"; sub: "how much it holds compared with new"; Value { text: ds.hw && ds.hw.batteries.length && ds.hw.batteries[0].health ? ds.hw.batteries[0].health + "%" : "–" } }
                Line { title: "Charge cycles"; Value { text: ds.hw && ds.hw.batteries.length && ds.hw.batteries[0].cycles ? String(ds.hw.batteries[0].cycles) : "–" } }
            }
            Card {
                visible: ds.kind === "battery"
                title: "Charging & low battery"; sub: "A charge limit keeps a battery that's always plugged in healthy for longer."
                Line {
                    visible: !!(ds.hw && ds.hw.batteries.length && ds.hw.batteries[0].canLimit)
                    title: "Stop charging at"
                    Chips {
                        options: [["60", "60%"], ["80", "80%"], ["90", "90%"], ["100", "Full"]]
                        value: ds.hw && ds.hw.batteries.length && ds.hw.batteries[0].limit ? String(ds.hw.batteries[0].limit) : "100"
                        onChose: v => ds.hwset("battery.limit", v)
                    }
                }
                Line { title: "When it runs low"; sub: "a warning at 10% and 5%; at 3% Hearth hibernates (where it has worked before) or shuts down cleanly"; Value { text: "on"; color: "#5fd08a" } }
            }
        }
        Text {
            visible: ds.note !== ""
            Layout.fillWidth: true; wrapMode: Text.WordWrap
            text: ds.note; color: ds.noteBad ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 12
        }
    }
}

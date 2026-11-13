// Task Manager › Performance › <device> › … & settings: CPU, Disk, Network, GPU and Battery, built completely to the
// approved mockups tm-3 … tm-7 (docs/mockups/next-update; red flag 11, docs/mockups/checklists/task-manager.md).
// Each page: its numbers on top, then its cards. Reads hearth-hwset, hearth-power, hearth-netinfo, hearth-gputune and
// hearth-tasks; changes only what you press (saved in /etc/hearth/hardware.json or by the tool that owns it, so it
// comes back after a restart). Opening it writes nothing.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Item {
    id: ds
    property string kind: "cpu"           // cpu | disk | net | gpu | battery
    property var dev: null                // the Performance page's device (its sample)
    property var sample: null             // the whole sample
    property var view: null               // TaskManagerView (its groups: the programs on the graphics card)
    property var hw: null                 // hearth-hwset status
    property var power: null              // hearth-power status
    property var net: null                // {name, metered, hotspot}
    property var netInfo: null            // hearth-netinfo info
    property var netApps: null            // hearth-netinfo apps
    property var drives: null             // hearth-hwset drives (Optimize drives)
    property var health: null             // pkexec hearth-hwset disk-health
    property var space: null              // pkexec hearth-hwset space
    property var busy: null               // hearth-tasks disk-busy
    property var gpu: null                // hearth-gputune status
    property var battUse: null            // hearth-tasks battery-use
    property var speed: null              // the last speed test
    property var fixed: null              // the last Fix my connection
    property string note: ""
    property bool noteBad: false
    implicitHeight: col.implicitHeight
    readonly property string shellDir: decodeURIComponent(Qt.resolvedUrl(".").toString().replace(/^file:\/\//, "")).replace(/\/$/, "")
    readonly property string hwsetBin: "/usr/libexec/nimbus/hearth-hwset"
    readonly property var saved: hw && hw.saved ? hw.saved : ({})
    function refresh() {
        if (!hwProc.running) hwProc.running = true;
        if (kind === "cpu" && !powerProc.running) powerProc.running = true;
        if (kind === "net") { if (!netProc.running) netProc.running = true; if (!netAppsProc.running) netAppsProc.running = true; }
        if (kind === "disk") { if (!drvProc.running) drvProc.running = true; if (!busyProc.running) busyProc.running = true; }
        if (kind === "gpu" && !gpuProc.running) gpuProc.running = true;
        if (kind === "battery" && !battProc.running) battProc.running = true;
    }
    function slowRefresh() {
        if (kind === "disk") { if (!healthProc.running) healthProc.running = true; if (!spaceProc.running && !space) spaceProc.running = true; }
        if (kind === "net" && !netInfoProc.running) netInfoProc.running = true;
    }
    onKindChanged: { refresh(); slowRefresh(); }
    Component.onCompleted: { refresh(); slowRefresh(); }
    Timer { interval: 3000; repeat: true; running: ds.visible; onTriggered: ds.refresh() }
    Timer { interval: 20000; repeat: true; running: ds.visible; onTriggered: ds.slowRefresh() }
    function lastJson(t) { try { return JSON.parse(t.trim().split("\n").pop()); } catch (e) { return null; } }
    Process { id: hwProc; command: ["python3", ds.hwsetBin, "status"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.hw = r; } } }
    Process { id: drvProc; command: ["python3", ds.hwsetBin, "drives"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.drives = r; } } }
    Process { id: healthProc; command: ["pkexec", ds.hwsetBin, "disk-health"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.health = r; } } }
    Process { id: spaceProc; command: ["pkexec", ds.hwsetBin, "space"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.space = r; } } }
    Process { id: busyProc; command: ["python3", ds.shellDir + "/hearth-tasks", "disk-busy"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.busy = r; } } }
    Process { id: powerProc; command: ["python3", ds.shellDir + "/hearth-power", "status"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.power = r; } } }
    Process { id: netInfoProc; command: ["python3", ds.shellDir + "/hearth-netinfo", "info"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.netInfo = r; } } }
    Process { id: netAppsProc; command: ["python3", ds.shellDir + "/hearth-netinfo", "apps"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.netApps = r; } } }
    Process { id: gpuProc; command: ["python3", ds.shellDir + "/hearth-gputune", "status"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.gpu = r; } } }
    Process { id: battProc; command: ["python3", ds.shellDir + "/hearth-tasks", "battery-use"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.battUse = r; } } }
    Process {
        id: netProc
        command: ["sh", "-c", "c=$(nmcli -t -f NAME,TYPE connection show --active 2>/dev/null | grep -v ':loopback' | head -n1 | cut -d: -f1); m=$(nmcli -g connection.metered connection show \"$c\" 2>/dev/null); h=$(nmcli -t -f NAME connection show --active 2>/dev/null | grep -c '^Hotspot'); printf '{\"name\":\"%s\",\"metered\":\"%s\",\"hotspot\":%s}\\n' \"$c\" \"$m\" \"$h\""]
        stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); if (r) ds.net = r; } }
    }
    Process { id: speedProc; command: ["python3", ds.shellDir + "/hearth-netinfo", "speedtest"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); ds.speed = r || { error: text.trim() }; } } }
    Process { id: fixProc; command: ["python3", ds.shellDir + "/hearth-netinfo", "fix"]; stdout: StdioCollector { onStreamFinished: { const r = ds.lastJson(text); ds.fixed = r || { found: [text.trim()] }; } } }
    Process {
        id: act
        property string okText: "Saved ✓"
        stdout: StdioCollector {
            onStreamFinished: {
                const l = text.trim().split("\n").pop() || "";
                if (l.indexOf("error: ") === 0) { ds.note = l.slice(7); ds.noteBad = true; }
                else { ds.note = act.okText; ds.noteBad = false; }
                ds.refresh();
                if (ds.kind === "disk") { ds.space = null; ds.slowRefresh(); }
                if (ds.kind === "net") ds.slowRefresh();
            }
        }
    }
    function run(cmd, ok) { if (act.running) return; act.okText = ok || "Saved ✓"; act.command = cmd; act.running = true; }
    function hwset(key, value) { run(["pkexec", hwsetBin, "set", key, String(value)]); }
    function gputune(args, ok) { run(["python3", shellDir + "/hearth-gputune"].concat(args), ok); }

    // ---- small helpers
    function size(b) {
        if (b === undefined || b === null) return "–";
        const u = ["B", "KB", "MB", "GB", "TB"]; let i = 0, x = b;
        while (x >= 1000 && i < u.length - 1) { x /= 1000; i++; }
        return (i === 0 || x >= 100 ? String(Math.round(x)) : x.toFixed(1)) + " " + u[i];
    }
    function rate(b) { return !b ? "0" : b < 1e6 ? Math.round(b / 1000) + " KB/s" : (b / 1e6).toFixed(1) + " MB/s"; }
    function allowOptions(cur) {
        const o = [["allow", "Allowed"], ["limit:1", "Limit to 1 MB/s"], ["limit:5", "Limit to 5 MB/s"], ["limit:20", "Limit to 20 MB/s"], ["block", "Blocked (offline game)"]];
        return o.some(x => x[0] === cur) ? o : o.concat([[cur, "Limit to " + String(cur).replace("limit:", "") + " MB/s"]]);
    }
    function thousands(n) { return n === null || n === undefined ? "–" : String(Math.round(n)).replace(/\B(?=(\d{3})+(?!\d))/g, ","); }
    readonly property var diskName: dev && dev.disk ? String(dev.disk.name || "").replace(/p?\d+$/, "") : ""
    readonly property var smart: health && health.drives ? (health.drives[diskName] || health.drives[String(dev && dev.disk ? dev.disk.name : "")] || null) : null
    readonly property var bat: hw && hw.batteries && hw.batteries.length ? hw.batteries[0] : null

    component Card: Rectangle {
        id: card
        default property alias body: inner.data
        property string title
        property string sub
        property bool isNew: false
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignTop
        implicitHeight: head.implicitHeight + inner.implicitHeight + 46
        radius: Theme.r(10); color: Theme.panel; border.color: Theme.line
        ColumnLayout {
            id: head
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 18 }
            spacing: 4
            Row {
                spacing: 8
                Text { text: card.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                Rectangle {
                    visible: card.isNew; anchors.verticalCenter: parent.verticalCenter
                    width: nw.implicitWidth + 10; height: 18; radius: 4; color: "transparent"; border.color: Theme.flame
                    Text { id: nw; anchors.centerIn: parent; text: "NEW"; color: Theme.flame; font.family: Theme.font; font.pixelSize: 10; font.weight: Font.DemiBold }
                }
            }
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
    // A dropdown, as in the mockups: the choice with ▾; a click opens the list under it.
    component Drop: Item {
        id: drop
        property var options: []              // [[value, label]]
        property string value
        property bool open: false
        signal chose(string v)
        readonly property var cur: options.find(o => o[0] === value) || options[0] || ["", "–"]
        implicitWidth: Math.max(dropBtn.implicitW, 90); implicitHeight: 32
        z: open ? 50 : 0
        Rectangle {
            id: dropBtn
            readonly property real implicitW: dt.implicitWidth + 34
            anchors.fill: parent; radius: Theme.r(6)
            color: dm.containsMouse ? Theme.panel3 : Theme.panel2; border.color: drop.open ? Theme.accent : Theme.line
            Text { id: dt; anchors.verticalCenter: parent.verticalCenter; x: 12; text: drop.cur[1] + "  ▾"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
            MouseArea { id: dm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: drop.open = !drop.open }
        }
        Rectangle {
            visible: drop.open
            anchors.top: dropBtn.bottom; anchors.topMargin: 4; anchors.right: dropBtn.right
            width: Math.max(dropBtn.width, optCol.implicitWidth + 8); height: optCol.implicitHeight + 8
            radius: Theme.r(8); color: Theme.panel2; border.color: Theme.line
            Column {
                id: optCol; x: 4; y: 4
                Repeater {
                    model: drop.options
                    delegate: Rectangle {
                        required property var modelData
                        width: Math.max(ot.implicitWidth + 24, dropBtn.width - 8); height: 30; radius: Theme.r(5)
                        color: om.containsMouse ? Theme.panel3 : modelData[0] === drop.value ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18) : "transparent"
                        Text { id: ot; anchors.verticalCenter: parent.verticalCenter; x: 12; text: parent.modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                        MouseArea { id: om; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { drop.open = false; if (parent.modelData[0] !== drop.value) drop.chose(parent.modelData[0]); } }
                    }
                }
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
        property bool busy: false
        signal clicked()
        width: bt.implicitWidth + 26; height: 32; radius: Theme.r(8)
        color: bm.containsMouse ? Theme.panel3 : Theme.panel2; border.color: Theme.line
        opacity: busy ? 0.6 : 1
        Text { id: bt; anchors.centerIn: parent; text: parent.busy ? "…" : parent.text; color: Theme.text; font.family: Theme.font; font.pixelSize: 12 }
        MouseArea { id: bm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; enabled: !parent.busy; onClicked: parent.clicked() }
    }
    // A slider with its value on the right (Max processor speed, Power limit); the change is sent when you let go.
    component Slide: RowLayout {
        id: sl
        property real from: 0
        property real to: 100
        property real stepSize: 5
        property real value: 0
        property string unit: "%"
        property real shown: value
        signal moved(real v)
        spacing: 12
        Item {
            Layout.preferredWidth: 260; Layout.preferredHeight: 24
            Rectangle { id: track; anchors.verticalCenter: parent.verticalCenter; width: parent.width; height: 4; radius: 2; color: Theme.panel3 }
            Rectangle { anchors.verticalCenter: parent.verticalCenter; width: track.width * Math.max(0, Math.min(1, (sl.shown - sl.from) / Math.max(1, sl.to - sl.from))); height: 4; radius: 2; color: Theme.accent }
            Rectangle {
                width: 18; height: 18; radius: 9; color: Theme.text; border.color: Theme.accent; border.width: 3
                anchors.verticalCenter: parent.verticalCenter
                x: (track.width - width) * Math.max(0, Math.min(1, (sl.shown - sl.from) / Math.max(1, sl.to - sl.from)))
            }
            MouseArea {
                anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                function at(x) { const f = Math.max(0, Math.min(1, x / width)); return Math.round((sl.from + f * (sl.to - sl.from)) / sl.stepSize) * sl.stepSize; }
                onPressed: m => sl.shown = at(m.x)
                onPositionChanged: m => { if (pressed) sl.shown = at(m.x); }
                onReleased: m => { const v = at(m.x); sl.shown = v; if (v !== sl.value) sl.moved(v); }
            }
        }
        Value { text: Math.round(sl.shown) + (sl.unit === "%" ? "%" : " " + sl.unit); Layout.preferredWidth: 64; horizontalAlignment: Text.AlignRight }
        onValueChanged: shown = value
    }
    // The numbers on top of a page (Health · Written so far · …), as in the mockups.
    component Stats: Flow {
        property var items: []                 // [[label, big, small, color]]
        Layout.fillWidth: true
        spacing: 34
        Repeater {
            model: parent.items
            delegate: Column {
                required property var modelData
                spacing: 2
                Text { text: modelData[0]; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                Row {
                    spacing: 6
                    Text { id: big; text: modelData[1]; color: modelData[3] || Theme.text; font.family: Theme.font; font.pixelSize: 26 }
                    Text { visible: (modelData[2] || "") !== ""; text: modelData[2] || ""; anchors.baseline: big.baseline; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                }
            }
        }
    }
    // A small table inside a card (Busiest programs, Which programs use the network, …). A cell is text, or
    // {t, tag} (a name with a tag), or {drop: [[value, label]], value, name} (the Allow dropdown: picked(name, value)).
    component Table: ColumnLayout {
        id: tbl
        property var cols: []                  // [[title, width (0 = fills), right]]
        property var rows: []
        signal picked(string name, string value)
        Layout.fillWidth: true
        spacing: 0
        Repeater {
            model: [null].concat(tbl.rows)
            delegate: Rectangle {
                id: trow
                required property var modelData
                required property int index
                readonly property bool header: index === 0
                Layout.fillWidth: true
                implicitHeight: 36
                color: "transparent"
                Rectangle { visible: !trow.header; width: parent.width; height: 1; color: Theme.line; opacity: 0.6 }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 8; anchors.rightMargin: 8
                    spacing: 12
                    Repeater {
                        model: tbl.cols
                        delegate: Item {
                            id: tcell
                            required property var modelData
                            required property int index
                            readonly property var cell: trow.header ? modelData[0] : (trow.modelData ? trow.modelData[index] : "")
                            readonly property bool isObj: !!cell && typeof cell === "object"
                            readonly property string txt: isObj ? (cell.t || "") : String(cell === null || cell === undefined ? "" : cell)
                            readonly property string tag: isObj ? (cell.tag || "") : ""
                            Layout.fillWidth: modelData[1] === 0
                            Layout.preferredWidth: modelData[1] || 100
                            implicitHeight: 32
                            Row {
                                visible: !(tcell.isObj && tcell.cell.drop)
                                anchors.verticalCenter: parent.verticalCenter
                                anchors.right: tcell.modelData[2] ? parent.right : undefined
                                spacing: 8
                                Text {
                                    text: tcell.txt
                                    color: trow.header ? Theme.muted : Theme.text
                                    font.family: Theme.font; font.pixelSize: 13
                                    elide: Text.ElideRight
                                    width: Math.min(implicitWidth, Math.max(30, tcell.width - (tagBox.visible ? tagBox.width + 8 : 0)))
                                }
                                Rectangle {
                                    id: tagBox
                                    visible: tcell.tag !== ""
                                    width: tagText.implicitWidth + 12; height: 20; radius: Theme.r(4)
                                    color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18); border.color: Theme.accent
                                    Text { id: tagText; anchors.centerIn: parent; text: tcell.tag; color: Theme.accent; font.family: Theme.font; font.pixelSize: 11 }
                                }
                            }
                            Drop {
                                visible: tcell.isObj && !!tcell.cell.drop
                                anchors.verticalCenter: parent.verticalCenter
                                options: tcell.isObj && tcell.cell.drop ? tcell.cell.drop : []
                                value: tcell.isObj && tcell.cell.value ? tcell.cell.value : ""
                                onChose: v => tbl.picked(tcell.cell.name, v)
                            }
                        }
                    }
                }
            }
        }
    }

    ColumnLayout {
        id: col
        width: parent.width
        spacing: 14

        Stats { visible: ds.topStats.length > 0; items: ds.topStats }

        // ======================================================================== CPU (tm-3)
        // the cores strip: each core's use, core 0 with the speed
        Row {
            visible: ds.kind === "cpu" && !!(ds.sample && ds.sample.cpu && ds.sample.cpu.cores && ds.sample.cpu.cores.length > 1)
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: ds.sample && ds.sample.cpu ? ds.sample.cpu.cores : []
                delegate: Rectangle {
                    required property real modelData
                    required property int index
                    width: (col.width - 6 * ((ds.sample.cpu.cores.length || 1) - 1)) / Math.max(1, ds.sample.cpu.cores.length)
                    height: 64; color: "transparent"; border.color: Theme.line; border.width: 1
                    Rectangle { anchors.bottom: parent.bottom; anchors.left: parent.left; anchors.right: parent.right; anchors.margins: 1; height: Math.max(2, (parent.height - 2) * Math.min(100, parent.modelData) / 100); color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.55) }
                    Text { x: 6; y: 4; text: parent.index === 0 && ds.sample.cpu.mhz ? "0 · " + (ds.sample.cpu.mhz / 1000).toFixed(1) + " GHz" : String(parent.index); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                }
            }
        }

        GridLayout {
            Layout.fillWidth: true
            columns: ds.width > 900 ? 2 : 1
            columnSpacing: 14; rowSpacing: 14

            Card {
                visible: ds.kind === "cpu"
                title: "Power mode"; sub: "How hard the processor works. Changes the fans and battery life too."
                Line {
                    title: "Mode"
                    Chips {
                        options: [["power-saver", "Battery saver"], ["balanced", "Balanced"], ["performance", "Performance"]]
                        value: ds.power && ds.power.mode ? (ds.power.mode.active || "") : ""
                        onChose: v => {
                            ds.run(["python3", ds.shellDir + "/hearth-power", "mode", v], "Power mode: " + v);
                            // on battery, this is also the mode the PC goes back to when you unplug (When plugged in)
                            if (ds.hw && ds.hw.onAC === false && (ds.saved["power.plugged"] || "off") !== "off") Quickshell.execDetached(["pkexec", ds.hwsetBin, "set", "power.unplugged", v]);
                        }
                    }
                }
                Line {
                    visible: !!(ds.hw && ds.hw.onAC !== null && ds.hw.onAC !== undefined)
                    title: "When plugged in"; sub: "switches by itself when you plug in or unplug"
                    Drop {
                        options: [["off", "Don't switch"], ["performance", "Performance"], ["balanced", "Balanced"], ["power-saver", "Battery saver"]]
                        value: ds.saved["power.plugged"] || "off"
                        onChose: v => {
                            ds.hwset("power.plugged", v);
                            if (v !== "off" && !ds.saved["power.unplugged"] && ds.power && ds.power.mode && ds.power.mode.active)
                                Quickshell.execDetached(["pkexec", ds.hwsetBin, "set", "power.unplugged", ds.power.mode.active]);
                        }
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
                title: "Limits & heat"; sub: "Keep the laptop cool and quiet, or let it go all out."
                Line {
                    visible: !!(ds.hw && ds.hw.maxPct.can)
                    title: "Max processor speed"; sub: "100% = no limit"
                    Slide { from: 30; to: 100; stepSize: 5; value: ds.hw ? ds.hw.maxPct.value : 100; onMoved: v => ds.hwset("cpu.maxPct", v) }
                }
                Line {
                    title: "Temperature now · limit"
                    Value {
                        readonly property var now: ds.sample && ds.sample.cpu && ds.sample.cpu.temp ? ds.sample.cpu.temp : null
                        text: (now ? Math.round(now) + "°C" : "–") + " · " + (ds.hw && ds.hw.tempLimit ? ds.hw.tempLimit + "°C" : "–")
                    }
                }
                Line {
                    title: "Throttling"; sub: "when the CPU slowed itself down because of heat"
                    Value {
                        readonly property var n: ds.hw ? (ds.hw.throttledToday !== null && ds.hw.throttledToday !== undefined ? ds.hw.throttledToday : ds.hw.throttled) : null
                        text: n === null ? "–" : n === 0 ? "none today" : n + (n === 1 ? " time today" : " times today")
                        color: n === 0 ? "#5fd08a" : Theme.text
                    }
                }
            }
            // Game cores (1.11, mockup tm-3): off unless you turn it on (hearth-hwset game.cores)
            Card {
                visible: ds.kind === "cpu" && !!(ds.hw && ds.hw.gameCores && ds.hw.gameCores.can)
                readonly property var cores: ds.hw ? ds.hw.gameCores : null
                readonly property bool isOn: !!cores && cores.value !== "off"
                id: gcCard
                title: "Game cores"; isNew: true; sub: "Give the game you're playing its own cores, so background work never stutters it."
                Line {
                    title: "Reserve for games"; sub: "only while a game is running; everything shares every core again after"
                    Toggle { checked: gcCard.isOn; onFlipped: ds.hwset("game.cores", checked ? "off" : "on") }
                }
                Line {
                    visible: gcCard.isOn
                    title: "Cores for the game"
                    Drop {
                        readonly property int n: gcCard.cores ? gcCard.cores.threads : 8
                        options: [[Math.floor(n / 4) + "-" + (n - 1), (n - Math.floor(n / 4)) + " of " + n + " (cores " + Math.floor(n / 4) + "–" + (n - 1) + ")"],
                                  [Math.floor(n / 2) + "-" + (n - 1), (n - Math.floor(n / 2)) + " of " + n + " (cores " + Math.floor(n / 2) + "–" + (n - 1) + ")"],
                                  [Math.floor(3 * n / 4) + "-" + (n - 1), (n - Math.floor(3 * n / 4)) + " of " + n + " (cores " + Math.floor(3 * n / 4) + "–" + (n - 1) + ")"]]
                        value: gcCard.cores ? gcCard.cores.value : ""
                        onChose: v => ds.hwset("game.cores", v)
                    }
                }
                Line {
                    visible: gcCard.isOn
                    title: "Background work (AI, updates, builds)"; sub: "runs on the other cores at low priority"
                    Value { text: { if (!gcCard.cores) return "–"; const m = (gcCard.cores.value.match(/^(\d+)/) || [0, 0])[1]; return m > 0 ? "Cores 0–" + (m - 1) : "–"; } }
                }
            }
            // Per-app limits (mockup tm-3): a program's top share of the processor, or low priority; kept, put back
            // whenever it runs (hearth-app-rules.service), its AI jobs and helpers included.
            Card {
                id: palCard
                visible: ds.kind === "cpu"
                title: "Per-app limits"; isNew: true; sub: "Stop one program from taking the whole processor."
                readonly property var rules: ds.saved["apps.cpu"] || ({})
                readonly property var opts: [["low", "Low priority"], ["25", "Max 25%"], ["50", "Max 50%"], ["75", "Max 75%"], ["remove", "No limit"]]
                Repeater {
                    model: Object.keys(palCard.rules).sort()
                    delegate: Line {
                        required property var modelData
                        title: modelData
                        sub: { const g = ds.view ? (ds.view.groups || []).find(x => x.name === modelData || String(x.exe || "").replace(/^.*\//, "") === modelData) : null;
                               return g ? "PID " + g.pid + " · its AI jobs included" : "applies whenever it runs"; }
                        Drop { options: palCard.opts; value: palCard.rules[modelData]; onChose: v => ds.run(["pkexec", ds.hwsetBin, "app-rule", "cpu", modelData, v]) }
                    }
                }
                Line {
                    title: "Updates and downloads"
                    Drop { options: [["low", "Low priority"], ["normal", "Normal"]]; value: ds.saved["updates.priority"] || "normal"; onChose: v => ds.hwset("updates.priority", v) }
                }
                Line {
                    id: addLine
                    property bool picking: false
                    title: picking ? "Which program?" : ""
                    Text {
                        visible: !addLine.picking
                        text: "+ Add a program"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 14
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: addLine.picking = true }
                    }
                    Drop {
                        visible: addLine.picking
                        options: (ds.view ? (ds.view.groups || []) : []).filter(g => (g.kind === "app" || g.kind === "game") && !palCard.rules[g.exe ? g.exe.replace(/^.*\//, "") : g.name])
                            .map(g => [g.exe ? g.exe.replace(/^.*\//, "") : g.name, g.name]).concat([["", "Cancel"]])
                        value: ""
                        onChose: v => { addLine.picking = false; if (v !== "") ds.run(["pkexec", ds.hwsetBin, "app-rule", "cpu", v, "50"], "Limited to 50% ✓ (change it on the left)"); }
                    }
                }
            }

            // ======================================================================== Disk (tm-4)
            Card {
                visible: ds.kind === "disk"
                title: "Optimize"; sub: "SSDs are trimmed, never defragmented. Hard drives get a fragmentation check."
                Line {
                    title: "TRIM"
                    sub: { const d = ds.drives ? ds.drives.drives.find(x => ds.diskName.indexOf(x.name) === 0) : null;
                           return (d && !d.trim ? d.what : (ds.drives && ds.drives.timer.last ? "last run " + ds.drives.timer.last.replace(/^\w+ /, "").replace(/ \d+:\d+:\d+.*$/, "") : "not run yet")) + " · weekly"; }
                    Btn { text: "Optimize now"; busy: act.running; onClicked: ds.run(["pkexec", ds.hwsetBin, "optimize"], "Optimized ✓ (TRIM done)") }
                }
                Line {
                    visible: !!(ds.hw && ds.hw.home && ds.hw.home.type === "btrfs")
                    title: "Check the file system (btrfs scrub)"
                    sub: "reads everything and checks it isn't damaged" + (ds.health && ds.health.scrub && ds.health.scrub.last ? " · last " + ds.health.scrub.last + (ds.health.scrub.ok ? "" : " · it found problems") : "")
                    Drop { options: [["off", "Off"], ["weekly", "Weekly"], ["monthly", "Monthly"]]; value: ds.saved["disk.scrub"] || "off"; onChose: v => ds.hwset("disk.scrub", v) }
                }
                Line {
                    title: "Warn when C: is over"
                    Drop { options: [["80", "80% full"], ["85", "85% full"], ["90", "90% full"], ["95", "95% full"], ["off", "Only when nearly full"]]; value: ds.saved["disk.warnPct"] || "90"; onChose: v => ds.hwset("disk.warnPct", v) }
                }
            }
            Card {
                visible: ds.kind === "disk"
                title: "What's using space"; isNew: true; sub: "C: at a glance, biggest first."
                Text { visible: !ds.space; text: "Measuring…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                Repeater {
                    model: ds.space ? ds.space.rows : []
                    delegate: Line {
                        required property var modelData
                        title: modelData.label
                        sub: modelData.id === "restore" && modelData.snapshots ? modelData.snapshots + " snapshots of your files too" : ""
                        Value { text: ds.size(modelData.bytes) }
                        Btn { visible: !!modelData.clean; text: "Clean up"; busy: act.running; onClicked: ds.run(["pkexec", ds.hwsetBin, "cleanup"], "Cleaned up ✓") }
                    }
                }
            }
            Card {
                visible: ds.kind === "disk"
                title: "Writes & safety"
                Line {
                    visible: !!(ds.hw && ds.hw.home && ds.hw.home.type === "btrfs")
                    title: "Save to the drive every"; sub: "shorter = less lost if the power cuts"
                    Chips { options: [["5", "5 s"], ["10", "10 s"], ["30", "30 s"]]; value: ds.saved["disk.commit"] || (ds.hw && ds.hw.home ? ds.hw.home.commit : "30"); onChose: v => ds.hwset("disk.commit", v) }
                }
                Line {
                    visible: !!(ds.hw && ds.hw.home && ds.hw.home.type === "btrfs")
                    title: "Compress files on C: (zstd)"; sub: "saves space, a little CPU"
                    Toggle { checked: (ds.saved["disk.compress"] || (ds.hw && ds.hw.home ? ds.hw.home.compress : "on")) === "on"; onFlipped: ds.hwset("disk.compress", checked ? "off" : "on") }
                }
                Line {
                    visible: !!(ds.hw && ds.hw.home && ds.hw.home.type === "btrfs")
                    title: "Snapshots of your files"; sub: "Previous versions in Hearth Files"
                    Drop { options: [["off", "Off"], ["hourly", "Every hour"], ["daily", "Every day"]]; value: ds.saved["disk.snapshots"] || "off"; onChose: v => ds.hwset("disk.snapshots", v) }
                }
                Text { visible: !!(ds.hw && ds.hw.home && ds.hw.home.type !== "btrfs"); text: "Your files aren't on btrfs here, so these don't apply."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            }
            Card {
                visible: ds.kind === "disk"
                title: "Busiest programs"
                Table {
                    cols: [["Program", 0, false], ["PID", 80, true], ["Read", 90, true], ["Write", 90, true]]
                    rows: (ds.busy ? ds.busy.rows : []).map(r => [r.name, r.pid ? String(r.pid) : "—", ds.rate(r.read), ds.rate(r.write)])
                }
            }

            // ======================================================================== Network (tm-5)
            Card {
                visible: ds.kind === "net"
                Layout.columnSpan: ds.width > 900 ? 2 : 1
                title: "Which programs use the network"; isNew: true
                Table {
                    cols: [["Program", 0, false], ["PID", 80, true], ["Down", 90, true], ["Up", 90, true], ["Today", 90, true], ["Talking to", 260, false], ["Allow", 200, false]]
                    rows: (ds.netApps ? ds.netApps.apps : []).slice(0, 12).map(a => [a.name, a.pid ? String(a.pid) : "—", ds.rate(a.down), ds.rate(a.up), ds.size(a.today),
                        a.hosts.length ? a.hosts.slice(0, 2).join(", ") + (a.hosts.length > 2 ? " +" + (a.hosts.length - 2) : "") : "—",
                        { drop: ds.allowOptions(a.allow), value: a.allow, name: a.name }])
                    // kept; hearth-app-rules.service puts it back whenever the program runs
                    onPicked: (name, v) => ds.run(["pkexec", ds.hwsetBin, "app-rule", "net", name, v])
                }
                Text {
                    Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                    text: "Blocked and limited programs stay that way whenever they run, until you change them here."
                }
            }
            Card {
                visible: ds.kind === "net"
                title: "Connection"; sub: ds.net && ds.net.name ? ds.net.name : "Not connected"
                Line {
                    title: "Metered connection"; sub: "pauses updates and big downloads (phone hotspots)"
                    Toggle {
                        checked: !!(ds.net && /^yes/.test(ds.net.metered))
                        onFlipped: if (ds.net && ds.net.name) ds.run(["nmcli", "connection", "modify", ds.net.name, "connection.metered", checked ? "no" : "yes"], checked ? "Not metered" : "Metered ✓")
                    }
                }
                Line {
                    title: "DNS"
                    Drop {
                        options: [["auto", "Automatic"], ["cloudflare", "Cloudflare (1.1.1.1)"], ["google", "Google (8.8.8.8)"], ["quad9", "Quad9 (9.9.9.9)"]].concat(ds.netInfo && ds.netInfo.dns.choice === "custom" ? [["custom", "Your own"]] : [])
                        value: ds.netInfo ? ds.netInfo.dns.choice : "auto"
                        onChose: v => { if (v !== "custom") ds.run(["python3", ds.shellDir + "/hearth-netinfo", "dns", v], "DNS: " + v + " ✓"); }
                    }
                }
                Line {
                    visible: !!(ds.netInfo && ds.netInfo.powersave !== null && ds.netInfo.powersave !== undefined)
                    title: "Wi-Fi power saving"; sub: "off = lower ping in games"
                    Toggle { checked: !!(ds.netInfo && ds.netInfo.powersave); onFlipped: ds.run(["python3", ds.shellDir + "/hearth-netinfo", "powersave", checked ? "off" : "on"]) }
                }
            }
            Card {
                visible: ds.kind === "net"
                title: "Tools"
                Line {
                    title: "Speed test"
                    sub: speedProc.running ? "testing… (about 30 seconds)" : ds.speed ? (ds.speed.error ? ds.speed.error.replace(/^error: /, "") : "↓ " + ds.speed.down + " Mbps · ↑ " + ds.speed.up + " Mbps · " + ds.speed.latency + " ms") : ""
                    Btn { text: "Run"; busy: speedProc.running; onClicked: speedProc.running = true }
                }
                Line {
                    title: "Fix my connection"
                    sub: fixProc.running ? "looking…" : ds.fixed ? ds.fixed.found.join(" · ") : "resets Wi-Fi and DNS, tells you what it found"
                    Btn { text: "Run"; busy: fixProc.running; onClicked: fixProc.running = true }
                }
                Line {
                    title: "Share this connection (hotspot)"; sub: "network name: " + (Quickshell.env("HOSTNAME") || "Hearth") + " · the password is in Settings › Wi-Fi"
                    Toggle {
                        checked: !!(ds.net && ds.net.hotspot)
                        onFlipped: ds.run(checked ? ["nmcli", "connection", "down", "Hotspot"] : ["nmcli", "device", "wifi", "hotspot", "ssid", Quickshell.env("HOSTNAME") || "Hearth"], checked ? "Hotspot off" : "Hotspot on ✓")
                    }
                }
            }

            // ======================================================================== GPU (tm-6)
            Card {
                id: fanCard
                visible: ds.kind === "gpu"
                title: "Fan curve"; sub: ds.gpu && !ds.gpu.installed ? "LACT isn't installed on this Hearth, so the fans can't be tuned here." : "Faster fans as it gets hotter. Drag the points."
                property var pts: []               // [[°C, %]] while you drag
                readonly property var saved: (ds.gpu ? ds.gpu.curve : "30:20,50:30,65:45,75:65,90:100").split(",").map(p => p.split(":").map(Number))
                Component.onCompleted: pts = saved
                onSavedChanged: if (!fanArea.pressed) pts = saved
                Item {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 190
                    Canvas {
                        id: fanCanvas
                        anchors.fill: parent
                        property var pts: fanCard.pts
                        onPtsChanged: requestPaint()
                        onWidthChanged: requestPaint()
                        function css(c, a) { return "rgba(" + Math.round(c.r * 255) + "," + Math.round(c.g * 255) + "," + Math.round(c.b * 255) + "," + a + ")"; }
                        function xOf(t) { return 16 + (width - 32) * (t - 30) / 60; }
                        function yOf(s) { return height - 14 - (height - 28) * s / 100; }
                        onPaint: {
                            const c = getContext("2d"); c.reset();
                            c.fillStyle = css(Theme.bg, 1); c.fillRect(0, 0, width, height);
                            c.strokeStyle = css(Theme.line, 1); c.strokeRect(0.5, 0.5, width - 1, height - 1);
                            if (!pts || pts.length < 2) return;
                            c.beginPath(); c.moveTo(xOf(Math.max(30, pts[0][0] - 10)), yOf(pts[0][1]));
                            for (const p of pts) c.lineTo(xOf(p[0]), yOf(p[1]));
                            c.strokeStyle = css(Theme.accent, 1); c.lineWidth = 2.5; c.stroke();
                            c.fillStyle = css(Theme.text, 1);
                            for (const p of pts.slice(1, -1)) { c.beginPath(); c.arc(xOf(p[0]), yOf(p[1]), 6, 0, 2 * Math.PI); c.fill(); }
                            c.fillStyle = css(Theme.muted, 1); c.font = "11px sans-serif"; c.fillText("30°C", 10, height - 2); c.fillText("90°C", width - 40, height - 2);
                        }
                    }
                    MouseArea {
                        id: fanArea
                        anchors.fill: parent
                        enabled: !!(ds.gpu && ds.gpu.installed)
                        property int which: -1
                        onPressed: m => {
                            let best = -1, d = 999;
                            for (let i = 1; i < fanCard.pts.length - 1; i++) {
                                const dx = fanCanvas.xOf(fanCard.pts[i][0]) - m.x, dy = fanCanvas.yOf(fanCard.pts[i][1]) - m.y;
                                const dd = Math.sqrt(dx * dx + dy * dy);
                                if (dd < d) { d = dd; best = i; }
                            }
                            which = d < 30 ? best : -1;
                        }
                        onPositionChanged: m => {
                            if (which < 0) return;
                            const p = fanCard.pts.map(x => x.slice());
                            const s = Math.round(Math.max(p[which - 1][1], Math.min(p[which + 1][1], 100 * (fanCanvas.height - 14 - m.y) / (fanCanvas.height - 28))));
                            p[which][1] = s;
                            fanCard.pts = p;
                        }
                        onReleased: { if (which >= 0) ds.gputune(["fan", "curve", fanCard.pts.map(p => p[0] + ":" + p[1]).join(",")], "Your fan curve ✓"); which = -1; }
                    }
                }
                Line {
                    title: "Mode"
                    Chips {
                        options: [["auto", "Automatic"], ["curve", "My curve"], ["quiet", "Quiet"]]
                        value: ds.gpu ? ds.gpu.fanMode : "auto"
                        onChose: v => v === "curve" ? ds.gputune(["fan", "curve", fanCard.pts.map(p => p[0] + ":" + p[1]).join(",")], "My curve ✓") : ds.gputune(["fan", v], v === "auto" ? "Automatic ✓" : "Quiet ✓")
                    }
                }
            }
            Card {
                visible: ds.kind === "gpu"
                title: "Power & clocks"; sub: "Off until you change something. Every change remembered and undoable (LACT)."
                Line {
                    visible: !!(ds.gpu && (ds.gpu.powerMax || ds.gpu.powerCap))
                    title: "Power limit"
                    Slide {
                        from: ds.gpu && ds.gpu.powerMin ? ds.gpu.powerMin : 10; to: ds.gpu && ds.gpu.powerMax ? ds.gpu.powerMax : 300; stepSize: 5; unit: "W"
                        value: ds.gpu ? (ds.gpu.powerCap || ds.gpu.powerMax || 0) : 0
                        onMoved: v => ds.gputune(["power", String(Math.round(v))], "Power limit " + Math.round(v) + " W ✓")
                    }
                }
                Line {
                    visible: !!(ds.gpu && ds.gpu.canUndervolt)
                    title: "Undervolt"; sub: "same speed, cooler and quieter"
                    Drop {
                        options: [["0", "None"], ["-25", "−25 mV"], ["-50", "−50 mV"], ["-75", "−75 mV"], ["-100", "−100 mV"]]
                        value: ds.gpu ? String(ds.gpu.undervolt || 0) : "0"
                        onChose: v => ds.gputune(["undervolt", v], "Undervolt " + v + " mV ✓")
                    }
                }
                Line {
                    title: "Profile"
                    Chips { options: [["quiet", "Quiet"], ["balanced", "Balanced"], ["max", "Max"]]; value: ds.gpu ? ds.gpu.profile : "balanced"; onChose: v => ds.gputune(["profile", v], "Profile: " + v + " ✓") }
                }
                RowLayout {
                    Layout.fillWidth: true
                    Item { Layout.fillWidth: true }
                    Btn { text: "Back to default"; onClicked: ds.gputune(["reset"], "Back to default ✓") }
                }
            }
            Card {
                visible: ds.kind === "gpu"
                Layout.columnSpan: ds.width > 900 ? 2 : 1
                title: "Programs on the graphics card"
                Table {
                    cols: [["Program", 0, false], ["PID", 80, true], ["GPU", 70, true], ["Video memory", 120, true], ["Uses", 160, false]]
                    rows: (ds.view ? (ds.view.groups || []) : []).filter(g => (g.gpu || 0) > 0 || (g.vram || 0) > 0).sort((a, b) => (b.gpu || 0) - (a.gpu || 0)).slice(0, 10)
                        .map(g => [{ t: g.name, tag: g.tag === "Game · Proton" ? "Proton · DXVK" : "" }, String(g.pid), Math.round(g.gpu || 0) + "%", g.vram ? ds.size(g.vram) : "—", (g.uses || []).join(", ") || "—"])
                }
            }

            // ======================================================================== Battery (tm-7)
            Card {
                visible: ds.kind === "battery"
                title: "When the battery runs low"; isNew: true; sub: "So it never dies with your work open."
                readonly property string warn: ds.saved["battery.warn"] || "10,5"
                readonly property int last: warn === "off" ? 5 : parseInt(warn.split(",").pop())
                id: lowCard
                Line {
                    title: "Warn at"
                    Drop { options: [["10,5", "10% and 5%"], ["15,5", "15% and 5%"], ["20,10", "20% and 10%"], ["off", "Don't warn"]]; value: lowCard.warn; onChose: v => ds.hwset("battery.warn", v) }
                }
                Line {
                    title: "At " + lowCard.last + "%"; sub: "hibernate is offered only once it's proven to work on this PC"
                    Chips {
                        options: (ds.hw && ds.hw.hibernate === "works" ? [["hibernate", "Hibernate"]] : []).concat([["shutdown", "Shut down"], ["sleep", "Sleep"]])
                        value: { const v = ds.saved["battery.at5"] || "hibernate"; return v === "hibernate" && !(ds.hw && ds.hw.hibernate === "works") ? "shutdown" : v; }
                        onChose: v => ds.hwset("battery.at5", v)
                    }
                }
                Line {
                    title: "Hibernation on this laptop"
                    Value {
                        text: !ds.hw ? "–" : ds.hw.hibernate === "works" ? "tested · works" : ds.hw.hibernate === "untested" ? "not tested yet" : "not available"
                        color: ds.hw && ds.hw.hibernate === "works" ? "#5fd08a" : Theme.muted
                    }
                }
            }
            Card {
                visible: ds.kind === "battery"
                title: "Look after the battery"
                Line {
                    visible: !!(ds.bat && ds.bat.canLimit)
                    title: "Stop charging at"; sub: "longer battery life if it's plugged in a lot"
                    Chips {
                        options: [["60", "60%"], ["80", "80%"], ["100", "100%"]]
                        value: ds.bat && ds.bat.limit ? String(ds.bat.limit >= 95 ? 100 : ds.bat.limit) : "100"
                        onChose: v => ds.hwset("battery.limit", v)
                    }
                }
                Line {
                    title: "Battery saver from"
                    Drop { options: [["off", "Off"], ["10", "10%"], ["20", "20%"], ["30", "30%"], ["50", "50%"]]; value: ds.saved["battery.saver"] || "off"; onChose: v => ds.hwset("battery.saver", v) }
                }
                Line {
                    title: "Dim the screen on battery"
                    Toggle { checked: ds.saved["battery.dim"] === "on"; onFlipped: ds.hwset("battery.dim", checked ? "off" : "on") }
                }
            }
            Card {
                visible: ds.kind === "battery"
                Layout.columnSpan: ds.width > 900 ? 2 : 1
                title: "What used the battery today"
                Text { visible: !!ds.battUse && ds.battUse.rows.length === 0; text: "Nothing yet today: it counts while the PC runs on battery."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                Table {
                    cols: [["Program", 0, false], ["PID", 80, true], ["Share", 70, true], ["Power now", 100, true], ["Tip", 280, false]]
                    rows: (ds.battUse ? ds.battUse.rows : []).map(r => {
                        const g = r.screen || !ds.view ? null : (ds.view.groups || []).find(x => x.name === r.name);
                        return [r.name, g ? String(g.pid) : "—", r.share + "%", r.watts === null || r.watts === undefined ? "—" : r.watts + " W", r.tip];
                    })
                }
            }
        }
        Text {
            visible: ds.note !== ""
            Layout.fillWidth: true; wrapMode: Text.WordWrap
            text: ds.note; color: ds.noteBad ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 12
        }
    }

    // What the page's top-right says (as in the mockups): "Intel Core i3-N305 · 8 cores", "256 GB NVMe · SK hynix",
    // "Home-5G · Wi-Fi 6 · 866 Mbps · signal ▂▄▆█", "NVIDIA GeForce RTX 3060 · 12 GB", "Lenovo 82XB · 47 Wh"
    readonly property string headline: {
        if (kind === "cpu") return (hw && hw.cpuModel ? hw.cpuModel : sample && sample.cpu ? sample.cpu.name : "") + (sample && sample.cpu && sample.cpu.coreCount ? " · " + sample.cpu.coreCount + " cores" : "");
        if (kind === "disk") { const d = dev && dev.disk ? dev.disk : {}; return [d.size ? size(d.size) + " " + (d.type || "") : (d.type || ""), d.model || (smart ? smart.model : "")].filter(x => x).join(" · "); }
        if (kind === "net") {
            const w = netInfo && netInfo.wifi ? netInfo.wifi : {};
            if (!w.ssid) return dev && dev.net ? dev.net.name + (dev.net.speed ? " · " + dev.net.speed + " Mbps" : "") : "";
            return [w.ssid, w.standard, w.mbps ? w.mbps + " Mbps" : "", w.bars ? "signal " + "▂▄▆█".slice(0, w.bars) : ""].filter(x => x).join(" · ");
        }
        if (kind === "gpu") { const g = dev && dev.gpu ? dev.gpu : {}; return (g.name || "") + (g.vramTotal ? " · " + Math.round(g.vramTotal / 1073741824) + " GB" : ""); }
        if (kind === "battery") return bat ? [bat.model || bat.name, bat.designWh ? bat.designWh + " Wh" : ""].filter(x => x).join(" · ") : "";
        return "";
    }

    // The numbers on top of each page (shown by TaskManagerView above the cards: see topStats)
    readonly property var topStats: {
        if (kind === "disk") {
            const h = smart || {};
            const life = h.lifeLeft;
            return [["Health", h.ok === false ? "Failing" : h.ok ? "Good" : "–", life !== null && life !== undefined ? life + "% life left" : "", h.ok === false ? Theme.danger : h.ok ? "#5fd08a" : null],
                    ["Written so far", h.written ? size(h.written) : "–", h.written && life !== null && life !== undefined && life < 100 ? "of ~" + size(h.written * 100 / Math.max(1, 100 - life)) + " rated" : ""],
                    ["Temperature", h.temp ? h.temp + "°C" : "–", ""],
                    ["Free on C:", hw && hw.c ? size(hw.c.free) : "–", hw && hw.c ? "of " + size(hw.c.size) : ""],
                    ["Power-on hours", h.hours ? thousands(h.hours) : "–", ""]];
        }
        if (kind === "net") {
            const n = netInfo || {};
            return [["Ping (to the internet)", n.ping !== null && n.ping !== undefined ? n.ping + " ms" : "–", n.jitter !== null && n.jitter !== undefined ? "jitter " + n.jitter + " ms" : ""],
                    ["Used today", n.today !== undefined ? size(n.today) : "–", ""],
                    ["IP", (n.ips && n.ips.length ? n.ips[0] : "–") + (n.tailscale ? " · Tailscale " + n.tailscale : ""), ""],
                    ["DNS", !n.dns ? "–" : n.dns.choice === "auto" ? "Automatic (router)" : n.dns.servers.join(", "), ""]];
        }
        if (kind === "gpu") {
            const g = dev && dev.gpu ? dev.gpu : {}, t = gpu || {};
            return [["Busy", g.busy !== null && g.busy !== undefined ? Math.round(g.busy) + "%" : "–", ""],
                    ["Video memory", g.vramTotal ? ((g.vramUsed || 0) / 1073741824).toFixed(1) : "–", g.vramTotal ? "/ " + Math.round(g.vramTotal / 1073741824) + " GB" : ""],
                    ["Temperature", g.temp ? Math.round(g.temp) + "°C" : "–", ""],
                    ["Power", t.power !== null && t.power !== undefined ? t.power + " W" : "–", t.powerCap || t.powerMax ? "/ " + Math.round(t.powerCap || t.powerMax) : ""],
                    ["Fan", t.fanPct !== null && t.fanPct !== undefined ? t.fanPct + "%" : "–", t.fanRpm ? thousands(t.fanRpm) + " rpm" : ""],
                    ["Game FPS", t.fps ? String(Math.round(t.fps)) : "–", ""]];
        }
        if (kind === "battery") {
            const b = dev && dev.battery ? dev.battery : {};
            const hb = bat || {};
            return [["Charge", b.percent !== undefined ? b.percent + "%" : "–", b.status === "Discharging" ? "on battery" : b.status ? b.status.toLowerCase() : ""],
                    [b.status === "Charging" ? "Full in" : "Time left", b.hours ? Math.floor(b.hours) + " h " + Math.round((b.hours % 1) * 60) + " min" : "–", ""],
                    ["Using now", b.watts ? b.watts.toFixed(1) + " W" : "–", ""],
                    ["Health", hb.health ? hb.health + "%" : "–", hb.fullWh && hb.designWh ? hb.fullWh + " of " + hb.designWh + " Wh" : "", hb.health >= 80 ? "#5fd08a" : null],
                    ["Charge cycles", hb.cycles ? String(hb.cycles) : "–", ""]];
        }
        return [];
    }
}

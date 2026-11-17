// System monitor widget: CPU, graphics, memory, disk and network, each with its graph of the last minute. The numbers
// are Task Manager's own (hearth-tasks sample, the same sampler as Task Manager and Resource Monitor), every 2 s
// while the widget is on screen. Settings: which graphs to show. In the bar: CPU and memory.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "widget-layout.js" as L

WidgetBase {
    id: w
    readonly property string tasks: decodeURIComponent(Qt.resolvedUrl("hearth-tasks").toString().replace(/^file:\/\//, ""))
    readonly property var all: [
        { id: "cpu", name: "CPU", tint: "#66c0f4" }, { id: "gpu", name: "Graphics", tint: "#b38cff" },
        { id: "mem", name: "Memory", tint: "#6fd08c" }, { id: "disk", name: "Disk", tint: "#ffb454" }, { id: "net", name: "Network", tint: "#ff7aa8" }]
    readonly property var show: list("show") || ["cpu", "gpu", "mem", "disk", "net"]
    readonly property var shown: all.filter(g => show.indexOf(g.id) >= 0 && (g.id !== "gpu" || hasGpu))
    property var hist: ({ cpu: [], gpu: [], mem: [], disk: [], net: [] })
    property var last: ({})
    property bool hasGpu: false
    implicitWidth: compact ? barRow.implicitWidth : 380
    implicitHeight: compact ? 24 : 300

    Process {
        id: sample
        command: ["python3", w.tasks, "sample"]
        stdout: StdioCollector {
            onStreamFinished: {
                let o = null;
                try { o = JSON.parse(text.trim().split("\n").pop()); } catch (e) { return; }
                const t = o.totals || {};
                const g = (o.gpus || [])[0];
                w.hasGpu = !!g;
                const memPct = o.mem && o.mem.total ? o.mem.used / o.mem.total * 100 : 0;
                const h = Object.assign({}, w.hist);
                h.cpu = L.pushSample(h.cpu, t.cpu, 30);
                h.gpu = L.pushSample(h.gpu, t.gpu || (g ? g.busy : 0), 30);
                h.mem = L.pushSample(h.mem, memPct, 30);
                h.disk = L.pushSample(h.disk, t.disk, 30);
                h.net = L.pushSample(h.net, t.netMbps, 30);
                w.hist = h;
                w.last = { cpu: t.cpu || 0, gpu: t.gpu || (g ? g.busy : 0) || 0, mem: memPct, memUsed: o.mem ? o.mem.used : 0, memTotal: o.mem ? o.mem.total : 0,
                           disk: t.disk || 0, net: t.netMbps || 0, cpuName: o.cpu ? o.cpu.name : "", gpuName: g ? g.name : "" };
            }
        }
    }
    Timer { interval: 2000; running: w.live; repeat: true; triggeredOnStart: true; onTriggered: if (!sample.running) sample.running = true }

    function gb(b) { return (b / 1073741824).toFixed(1); }
    function valueText(id) {
        const l = last;
        if (l.cpu === undefined) return "…";
        if (id === "mem") return Math.round(l.mem) + " %  ·  " + gb(l.memUsed) + " / " + gb(l.memTotal) + " GB";
        if (id === "net") return (l.net < 10 ? l.net.toFixed(1) : Math.round(l.net)) + " Mbit/s";
        return Math.round(l[id] || 0) + " %";
    }

    RowLayout {
        id: barRow
        visible: w.compact
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8
        Icon { name: "cpu"; size: 15; color: Theme.muted }
        Text { text: "CPU " + Math.round(w.last.cpu || 0) + "%  RAM " + Math.round(w.last.mem || 0) + "%"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
        Spark { Layout.preferredWidth: 46; Layout.preferredHeight: 18; values: w.hist.cpu; tint: "#66c0f4" }
    }

    ColumnLayout {
        visible: !w.compact && !w.configuring
        anchors { fill: parent; margins: 14 }
        spacing: 6
        Repeater {
            model: w.shown
            delegate: ColumnLayout {
                required property var modelData
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 2
                RowLayout {
                    Layout.fillWidth: true
                    Text { text: modelData.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                    Text { Layout.fillWidth: true; text: modelData.id === "cpu" ? w.last.cpuName || "" : modelData.id === "gpu" ? w.last.gpuName || "" : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; elide: Text.ElideRight }
                    Text { text: w.valueText(modelData.id); color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                }
                Spark {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.minimumHeight: 14
                    values: w.hist[modelData.id] || []
                    max: modelData.id === "net" ? 1 : 100
                    tint: modelData.tint
                }
            }
        }
    }
    ColumnLayout {
        visible: !w.compact && w.configuring
        anchors { fill: parent; margins: 14 }
        spacing: 8
        Text { text: "Show these graphs"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: w.all
                delegate: WBtn {
                    required property var modelData
                    text: modelData.name
                    on: w.show.indexOf(modelData.id) >= 0
                    onClicked: w.set("show", on ? w.show.filter(x => x !== modelData.id) : w.all.map(g => g.id).filter(x => x === modelData.id || w.show.indexOf(x) >= 0))
                }
            }
        }
        Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "The same numbers as Task Manager. Graphics shows only when Hearth can read your graphics card."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
        WBtn { text: "Open Task Manager"; icon: "cpu"; onClicked: Quickshell.execDetached(["quickshell", "ipc", "-p", decodeURIComponent(Qt.resolvedUrl(".").toString().replace(/^file:\/\//, "")), "call", "tasks", "tab", "performance"]) }
        Item { Layout.fillHeight: true }
    }
}

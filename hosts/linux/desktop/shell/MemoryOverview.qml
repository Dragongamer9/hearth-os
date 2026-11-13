import QtQuick
import QtQuick.Layouts
import Quickshell.Io

// Task Manager › Performance › Memory › Overview (1.12, the approved mockup task-manager-memory.png):
// the programs' memory over the last minute with compressed memory (zram) as a dashed line, what memory is made of,
// the numbers, Swap, Memory pressure, your memory settings at a glance and the programs using the most.
// It only reads: the sampler (hearth-tasks) and `hearth-memory status`; changing anything is in Swap & settings.
ColumnLayout {
    id: mo
    property var view                      // TaskManagerView: size(), sizeOf(), pct(), groups, hist, histMax
    property var s                         // the sample
    signal openSettings()
    spacing: 8

    readonly property var m: s && s.mem ? s.mem : ({})
    readonly property real total: Math.max(1, m.total || 0)
    readonly property real zmem: m.zramMem || 0
    readonly property real programs: Math.max(0, (m.used || 0) - zmem)
    readonly property real cached: Math.max(0, m.cached || 0)
    readonly property real free: Math.max(0, m.free || 0)
    readonly property bool hasZram: (m.zramSize || 0) > 0
    readonly property color cProg: Qt.color("#8b80f9")
    readonly property color cZram: Qt.color("#3fd0e0")
    readonly property color cCache: Qt.color("#cfc9d8")
    readonly property color cOk: Qt.color("#4ec98a")
    function pressColor(p) { return p < 1 ? cOk : p < 10 ? Theme.flame : p < 30 ? Theme.ember : Theme.danger; }
    function pressWord(p) { return p === null || p === undefined ? "–" : p < 1 ? "None" : p < 10 ? "Light" : p < 30 ? "Medium" : "Heavy"; }

    // the memory's speed and slots, from the board (dmidecode, through hearth-hwset), read once
    property var board: null
    Process {
        id: boardProc
        running: true
        command: ["pkexec", "/usr/libexec/nimbus/hearth-hwset", "memory-info"]
        stdout: StdioCollector { onStreamFinished: { try { mo.board = JSON.parse(text.trim().split("\n").pop()); } catch (e) { } } }
    }
    readonly property string headline: view ? view.size(m.total) + (board && board.speed ? " · " + (board.type ? board.type + " " : "") + board.speed + " MT/s" : "")
        + (board && board.slots ? " · " + board.used + " of " + board.slots + " slots" : "") : ""

    // your memory settings, read once when this opens and every 30 s (nothing is written)
    property var st: null
    Process {
        id: statusProc
        command: ["python3", "/usr/libexec/nimbus/hearth-memory", "status"]
        stdout: StdioCollector { onStreamFinished: { try { mo.st = JSON.parse(text); } catch (e) { } } }
    }
    Timer { interval: 30000; repeat: true; running: mo.visible; triggeredOnStart: true; onTriggered: if (!statusProc.running) statusProc.running = true }
    readonly property var set: st && st.settings ? st.settings : null
    function keepLabel(k) {
        const mode = k && k.mode ? k.mode : "2gb";
        return mode === "off" ? "Off" : mode === "2gb" ? "2 GB" : mode === "50%" ? "Half (50%)" : mode === "25%" ? "A quarter (25%)" : mode === "custom" ? "Custom" : mode;
    }

    component Lbl: Text { color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
    component Val: Text { color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
    component Card: Rectangle {
        id: card
        default property alias content: inner.data
        property string title
        Layout.fillWidth: true
        Layout.preferredWidth: 1
        Layout.fillHeight: true
        implicitHeight: inner.implicitHeight + 28
        radius: Theme.r(10)
        color: "transparent"
        border.color: Theme.line
        ColumnLayout {
            id: inner
            x: 14; y: 12
            width: parent.width - 28
            spacing: 8
            Text { text: card.title.toUpperCase(); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 1 }
        }
    }
    component KV: RowLayout {
        property string k
        property string val
        property color valColor: Theme.text
        Layout.fillWidth: true
        Val { text: parent.k; color: Theme.muted; Layout.fillWidth: true; elide: Text.ElideRight }
        Val { text: parent.val; color: parent.valColor }
    }
    component Meter: Rectangle {
        property real frac
        property color fill
        Layout.fillWidth: true
        implicitHeight: 6; radius: 3
        color: Theme.panel3
        Rectangle { width: Math.max(frac > 0 ? 4 : 0, parent.width * Math.min(1, frac)); height: parent.height; radius: 3; color: parent.fill }
    }

    // the graph
    RowLayout {
        Layout.fillWidth: true
        Lbl { text: "Memory in use (programs) · 60 seconds" }
        Item { Layout.fillWidth: true }
        Lbl { text: mo.view.size(mo.m.total) }
    }
    Canvas {
        id: chart
        Layout.fillWidth: true
        Layout.preferredHeight: 230
        readonly property var prog: mo.view.hist["memprog"] || []
        readonly property var zr: mo.view.hist["memz"] || []
        onProgChanged: requestPaint()
        onWidthChanged: requestPaint()
        function css(c, a) { return "rgba(" + Math.round(c.r * 255) + "," + Math.round(c.g * 255) + "," + Math.round(c.b * 255) + "," + a + ")"; }
        onPaint: {
            const ctx = getContext("2d"), w = width, h = height, cap = Math.max(2, mo.view.histMax);
            ctx.reset();
            ctx.fillStyle = css(Theme.bg, 1); ctx.fillRect(0, 0, w, h);
            const stepX = w / (cap - 1);
            const yOf = n => h - 1 - (h - 2) * Math.max(0, Math.min(100, n)) / 100;
            if (prog.length >= 2) {
                const x0 = w - (prog.length - 1) * stepX;
                ctx.beginPath(); ctx.moveTo(x0, h);
                for (let i = 0; i < prog.length; i++) ctx.lineTo(x0 + i * stepX, yOf(prog[i]));
                ctx.lineTo(w, h); ctx.closePath();
                ctx.fillStyle = css(mo.cProg, 0.22); ctx.fill();
                ctx.beginPath();
                for (let i = 0; i < prog.length; i++) { const x = x0 + i * stepX, y = yOf(prog[i]); if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y); }
                ctx.strokeStyle = css(mo.cProg, 1); ctx.lineWidth = 2; ctx.lineJoin = "round"; ctx.stroke();
            }
            if (mo.hasZram && zr.length >= 2) {
                const x0 = w - (zr.length - 1) * stepX;
                ctx.setLineDash([5, 4]);
                ctx.beginPath();
                for (let i = 0; i < zr.length; i++) { const x = x0 + i * stepX, y = yOf(zr[i]); if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y); }
                ctx.strokeStyle = css(mo.cZram, 1); ctx.lineWidth = 1.5; ctx.stroke();
                ctx.setLineDash([]);
            }
            ctx.strokeStyle = css(mo.cProg, 0.9); ctx.lineWidth = 1; ctx.strokeRect(0.5, 0.5, w - 1, h - 1);
        }
        Text {
            visible: mo.hasZram
            anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.margins: 10
            text: "- - compressed (zram)"; color: mo.cZram; font.family: Theme.font; font.pixelSize: 12
        }
    }

    // what memory is made of
    Lbl { text: "Memory composition"; Layout.topMargin: 6 }
    Row {
        id: bar
        Layout.fillWidth: true
        height: 26
        readonly property var parts: [[mo.programs, mo.cProg, "In use"], [mo.zmem, mo.cZram, "Compressed"], [mo.cached, mo.cCache, "Cached"], [mo.free, Theme.panel3, "Free"]]
        readonly property real sum: Math.max(1, mo.programs + mo.zmem + mo.cached + mo.free)
        Repeater {
            model: bar.parts
            delegate: Rectangle {
                required property var modelData
                required property int index
                width: bar.width * modelData[0] / bar.sum
                height: bar.height
                color: modelData[1]
                radius: index === 0 || index === 3 ? 4 : 0
                clip: true
                Text {
                    anchors.centerIn: parent
                    visible: implicitWidth + 10 < parent.width
                    text: parent.modelData[2] + " " + mo.view.size(parent.modelData[0])
                    color: parent.index === 3 ? Theme.text : "#14101c"
                    font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold
                }
            }
        }
    }
    Flow {
        Layout.fillWidth: true
        spacing: 18
        Repeater {
            model: [[mo.cProg, "In use: what programs hold"], [mo.cZram, "Compressed: zram's real size"], [mo.cCache, "Cached: given back when needed"], [Theme.panel3, "Free"]]
            delegate: Row {
                required property var modelData
                spacing: 6
                Rectangle { width: 11; height: 11; radius: 2; color: parent.modelData[0]; anchors.verticalCenter: parent.verticalCenter }
                Lbl { text: parent.modelData[1] }
            }
        }
    }

    // the numbers
    Flow {
        Layout.fillWidth: true
        Layout.topMargin: 6
        spacing: 30
        Repeater {
            model: [["In use", mo.view.size(mo.programs), ""], ["Available", mo.view.size(mo.m.available), ""], ["Cached", mo.view.size(mo.cached), ""], ["Free", mo.view.size(mo.free), ""]]
                .concat(mo.hasZram ? [["Compressed", mo.view.size(mo.m.zramOrig || 0), "in " + mo.view.size(mo.zmem) + (mo.zmem > 0 && mo.m.zramOrig ? " · " + Math.max(1, Math.round(mo.m.zramOrig / mo.zmem)) + ":1" : "")]] : [])
                .concat([["Committed", mo.view.size(mo.m.committed), "/ " + mo.view.size(mo.m.commitLimit)]])
            delegate: Column {
                required property var modelData
                spacing: 1
                Lbl { text: modelData[0] }
                Row {
                    spacing: 6
                    Text { id: bigNum; text: modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: 24 }
                    Lbl { visible: text !== ""; text: modelData[2]; anchors.baseline: bigNum.baseline }
                }
            }
        }
    }

    // Swap · Memory pressure · Memory setting
    GridLayout {
        Layout.fillWidth: true
        Layout.topMargin: 8
        columns: mo.width > 760 ? 3 : 1
        columnSpacing: 14
        rowSpacing: 14
        Card {
            title: "Swap"
            KV { k: "Zram (in memory, fast)"; val: mo.hasZram ? mo.view.sizeOf(mo.m.zramUsed, mo.m.zramSize) : "Off" }
            Meter { visible: mo.hasZram; frac: (mo.m.zramUsed || 0) / Math.max(1, mo.m.zramSize || 0); fill: mo.cZram }
            KV { k: "Drive swap file (slow)"; val: (mo.m.driveSize || 0) > 0 ? mo.view.sizeOf(mo.m.driveUsed, mo.m.driveSize) : "None" }
            Meter { visible: (mo.m.driveSize || 0) > 0; frac: (mo.m.driveUsed || 0) / Math.max(1, mo.m.driveSize || 0); fill: Theme.ember }
            Lbl {
                Layout.fillWidth: true; wrapMode: Text.WordWrap
                text: mo.hasZram ? "The drive is used only when zram is full." + (mo.st && mo.st.swapfile && mo.st.swapfile.hibernate ? " Also used for hibernation." : "") : "Memory isn't compressed; Swap & settings can turn zram on."
            }
        }
        Card {
            title: "Memory pressure"
            RowLayout {
                Layout.fillWidth: true
                Val { text: "Now"; color: Theme.muted; Layout.fillWidth: true }
                Rectangle {
                    implicitWidth: pw.implicitWidth + 16; implicitHeight: 24; radius: Theme.r(6)
                    color: Qt.rgba(mo.pressColor(mo.m.psi10 || 0).r, mo.pressColor(mo.m.psi10 || 0).g, mo.pressColor(mo.m.psi10 || 0).b, 0.18)
                    Text { id: pw; anchors.centerIn: parent; text: mo.pressWord(mo.m.psi10); color: mo.pressColor(mo.m.psi10 || 0); font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                }
            }
            KV { k: "Last 10 s · last minute"; val: mo.m.psi10 === null || mo.m.psi10 === undefined ? "–" : mo.m.psi10.toFixed(1) + "% · " + (mo.m.psi60 || 0).toFixed(1) + "%" }
            Item {
                Layout.fillWidth: true
                implicitHeight: 34
                Row {
                    anchors.bottom: parent.bottom
                    anchors.right: parent.right
                    spacing: 2
                    Repeater {
                        model: mo.view.hist["mempsi"] || []
                        delegate: Rectangle {
                            required property real modelData
                            width: Math.max(2, (parent.parent.width - 2 * mo.view.histMax) / mo.view.histMax)
                            height: Math.max(2, Math.min(30, 3 + modelData * 1.5))
                            anchors.bottom: parent.bottom
                            color: mo.pressColor(modelData)
                        }
                    }
                }
            }
            Lbl { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "How often programs waited for memory. High = stutter." }
        }
        Card {
            title: "Memory setting"
            KV { k: "Keep free"; val: mo.set ? mo.keepLabel(mo.set.keep) : "–" }
            KV { k: "Zram"; val: !mo.set ? "–" : mo.set.zram && mo.set.zram.on === false ? "Off" : mo.set.zram ? mo.set.zram.gb + " GB · " + mo.set.zram.algo : "–" }
            KV { k: "Drive swap file"; val: !mo.st ? "–" : mo.st.swapfile && mo.st.swapfile.exists ? mo.st.swapfile.gb + " GB" + (mo.set && mo.set.swapfile && mo.set.swapfile.everyday === false ? " · only for hibernating" : "") : "None" }
            KV { k: "When memory runs out"; val: !mo.set || !mo.set.guard ? "–" : mo.set.guard.level === "strong" ? "Close something early" : mo.set.guard.level === "off" ? "The system decides" : "Close something gently" }
            Text {
                text: "Change in Swap & settings ›"
                color: Theme.accent; font.family: Theme.font; font.pixelSize: 13
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: mo.openSettings() }
            }
        }
    }

    // the programs using the most
    Rectangle {
        id: users
        Layout.fillWidth: true
        Layout.topMargin: 8
        implicitHeight: tbl.implicitHeight + 28
        radius: Theme.r(10)
        color: "transparent"
        border.color: Theme.line
        readonly property bool drive: (mo.m.driveUsed || 0) > 0
        // swapped memory is in zram or on the drive; Linux doesn't say which per program, so each gets the PC's split
        readonly property real driveShare: drive ? (mo.m.driveUsed || 0) / Math.max(1, (mo.m.driveUsed || 0) + (mo.m.zramUsed || 0)) : 0
        readonly property var top: (mo.view.groups || []).slice().sort((a, b) => (b.mem || 0) - (a.mem || 0)).slice(0, 6)
        ColumnLayout {
            id: tbl
            x: 14; y: 12
            width: parent.width - 28
            spacing: 0
            Text { text: "BIGGEST USERS"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 1; Layout.bottomMargin: 8 }
            Repeater {
                model: [{ name: "Program", head: true }].concat(users.top)
                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    implicitHeight: 30
                    color: "transparent"
                    Rectangle { visible: index > 0; width: parent.width; height: 1; color: Theme.line; opacity: 0.6 }
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 8; anchors.rightMargin: 8
                        readonly property color c: modelData.head ? Theme.muted : Theme.text
                        Val { Layout.fillWidth: true; elide: Text.ElideRight; color: parent.c; text: modelData.head ? "Program" : modelData.name + ((modelData.count || 1) > 1 ? " (" + modelData.count + ")" : "") }
                        Val { Layout.preferredWidth: 110; horizontalAlignment: Text.AlignRight; color: parent.c; text: modelData.head ? "In use" : mo.view.size(modelData.mem || 0) }
                        Val { Layout.preferredWidth: 110; horizontalAlignment: Text.AlignRight; color: parent.c; text: modelData.head ? "Shared" : mo.view.size(modelData.shared || 0) }
                        Val { Layout.preferredWidth: 130; horizontalAlignment: Text.AlignRight; color: parent.c; text: modelData.head ? "Compressed" : mo.view.size(Math.round((modelData.swap || 0) * (1 - users.driveShare))) }
                        Val { Layout.preferredWidth: 100; horizontalAlignment: Text.AlignRight; color: parent.c; text: modelData.head ? "On drive" : users.driveShare > 0 ? mo.view.size(Math.round((modelData.swap || 0) * users.driveShare)) : "0" }
                    }
                }
            }
            Lbl { visible: users.top.length === 0; text: "Looking at what's running…"; Layout.topMargin: 4 }
        }
    }
}

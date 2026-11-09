// Reliability history's page (loaded by Reliability.qml). The approved mockup: a stability line over 14 days (or
// weeks), a row of marks per kind (App stopped working, Hearth problems, Warnings, Updates and changes), the days, and
// the picked day's events with Details and Send logs (problems only).
// Mouse, touch, keyboard and the controller (hearth-pad types keys while this window is in front: A Enter, B Esc):
//   Left / Right pick a day, Up goes to Days / Weeks, Down goes into the day's events (Up / Down between them,
//   Left / Right between Details and Send logs), A presses, B closes Details or the window.
// Reading only. Send logs is Hearth's own (hearth-send-logs, the same as Settings › Backup and help), with the event
// as the note.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

FocusScope {
    id: v
    focus: true
    property string helper: ""
    property var closeWindow: null

    readonly property color cBg: "#171d25"
    readonly property color cTitle: "#10151b"
    readonly property color cHead: "#1b2838"
    readonly property color cPanel: "#213347"
    readonly property color cLine: "#2f4a63"
    readonly property color cGrid: "#22344a"
    readonly property color cSelCol: "#22405a"
    readonly property color cText: "#e6eef5"
    readonly property color cMuted: "#9fb2c3"
    readonly property color cAccent: "#66c0f4"
    readonly property color cInk: "#06121c"
    readonly property string font: "Inter"

    property var summary: null
    property bool weeks: false
    property bool loading: true
    property int sel: 13
    property string zone: "chart"      // toggle | chart | events
    property int evIdx: 0
    property int evBtn: 0              // 0 Details, 1 Send logs
    property var details: null         // the event shown in Details
    property int dlgIdx: 0
    property string sendMsg: ""
    property bool sending: false

    readonly property var kinds: [
        { id: "app", label: "App stopped working", color: "#ff8a7a", glyph: "×", radius: 9 },
        { id: "hearth", label: "Hearth problems", color: "#ff8a7a", glyph: "!", radius: 4 },
        { id: "warn", label: "Warnings", color: "#f2c14e", glyph: "!", radius: 4 },
        { id: "info", label: "Updates and changes", color: "#66c0f4", glyph: "i", radius: 9 }
    ]
    function kind(id) { for (const k of kinds) if (k.id === id) return k; return kinds[3]; }

    function start() { load(); }
    function load() {
        if (proc.running) return;
        loading = true;
        proc.command = ["python3", helper, "summary"].concat(weeks ? ["--weeks"] : []);
        proc.running = true;
    }
    readonly property var rows: summary && summary.rows ? summary.rows : []
    readonly property var curRow: sel >= 0 && sel < rows.length ? rows[sel] : null
    readonly property var events: curRow ? curRow.events : []

    function dateOf(s) { const p = String(s).split("-"); return new Date(Number(p[0]), Number(p[1]) - 1, Number(p[2])); }
    function dayLabel(i) {
        const r = rows[i];
        if (!r) return "";
        if (!weeks && i === rows.length - 1) return "Today";
        const d = dateOf(r.date);
        if (weeks || i === 0 || d.getDate() === 1) return d.toLocaleDateString(Qt.locale(), "MMM d");
        return String(d.getDate());
    }
    function longDay(r) {
        if (!r) return "";
        const d = dateOf(r.date);
        if (weeks) return "Week of " + d.toLocaleDateString(Qt.locale(), "MMM d");
        return r.date === (summary ? summary.today : "") ? "Today" : d.toLocaleDateString(Qt.locale(), "MMM d");
    }
    function timeOf(ts) {
        const d = new Date(ts * 1000);
        const t = d.toLocaleTimeString(Qt.locale(), Locale.ShortFormat);
        return weeks ? d.toLocaleDateString(Qt.locale(), "MMM d") + ", " + t : t;
    }
    function pickDefault() {
        // the latest day with a problem, else the last day
        let s = rows.length - 1;
        for (let i = rows.length - 1; i >= 0; i--) { const c = rows[i].counts; if (c.app || c.hearth || c.warn) { s = i; break; } }
        sel = Math.max(0, s);
    }

    Process {
        id: proc
        stdout: StdioCollector { id: out }
        onExited: code => {
            let d = null;
            try { d = JSON.parse(out.text); } catch (e) { d = null; }
            v.summary = d && d.rows ? d : { rows: [] };
            v.loading = false;
            v.pickDefault();
            v.evIdx = 0;
        }
    }
    Process {
        id: sendProc
        stdout: StdioCollector { id: sendOut }
        stderr: StdioCollector { id: sendErr }
        onExited: code => {
            v.sending = false;
            const m = /code: (\S+)/.exec(sendOut.text || "");
            v.sendMsg = code === 0 && m ? "Sent. Share this code with the developer: " + m[1]
                : "Couldn't send them: " + (((sendErr.text || "") + (sendOut.text || "")).trim().split("\n").pop() || "Hearth's Send logs isn't on this system");
        }
    }
    function sendLogs(e) {
        if (sending || !e) return;
        sending = true; sendMsg = "";
        const note = "Reliability history: " + e.t + " · " + new Date(e.time * 1000).toLocaleString(Qt.locale(), Locale.ShortFormat) + (e.s ? " · " + e.s : "");
        sendProc.command = ["hearth-send-logs", note.slice(0, 400)];
        sendProc.running = true;
    }
    function openDetails(e) { details = e; dlgIdx = 0; sendMsg = ""; }
    function closeAll() { if (closeWindow) closeWindow(); }
    function setWeeks(w) { if (weeks === w) return; weeks = w; summary = null; load(); }

    readonly property var dlgButtons: details ? (details.cat === "app" || details.cat === "hearth" ? ["send", "close"] : ["close"]) : []
    Keys.onPressed: e => {
        const k = e.key;
        if (details) {
            if (k === Qt.Key_Escape || k === Qt.Key_Back) details = null;
            else if (k === Qt.Key_Left) dlgIdx = Math.max(0, dlgIdx - 1);
            else if (k === Qt.Key_Right || k === Qt.Key_Tab) dlgIdx = Math.min(dlgButtons.length - 1, dlgIdx + 1);
            else if (k === Qt.Key_Return || k === Qt.Key_Enter || k === Qt.Key_Space) { if (dlgButtons[dlgIdx] === "send") sendLogs(details); else details = null; }
            e.accepted = true;
            return;
        }
        if (k === Qt.Key_Escape || k === Qt.Key_Back) closeAll();
        else if (zone === "toggle") {
            if (k === Qt.Key_Left) setWeeks(false);
            else if (k === Qt.Key_Right) setWeeks(true);
            else if (k === Qt.Key_Down || k === Qt.Key_Tab) zone = "chart";
            else if (k === Qt.Key_Return || k === Qt.Key_Enter) setWeeks(!weeks);
        } else if (zone === "chart") {
            if (k === Qt.Key_Left) { sel = Math.max(0, sel - 1); evIdx = 0; }
            else if (k === Qt.Key_Right) { sel = Math.min(rows.length - 1, sel + 1); evIdx = 0; }
            else if (k === Qt.Key_Up) zone = "toggle";
            else if ((k === Qt.Key_Down || k === Qt.Key_Tab || k === Qt.Key_Return || k === Qt.Key_Enter) && events.length) { zone = "events"; evIdx = 0; evBtn = 0; }
        } else if (zone === "events") {
            if (k === Qt.Key_Up) { if (evIdx > 0) evIdx--; else zone = "chart"; }
            else if (k === Qt.Key_Down) evIdx = Math.min(events.length - 1, evIdx + 1);
            else if (k === Qt.Key_Left) evBtn = 0;
            else if (k === Qt.Key_Right) { const ev = events[evIdx]; if (ev && (ev.cat === "app" || ev.cat === "hearth")) evBtn = 1; }
            else if (k === Qt.Key_Tab) zone = "toggle";
            else if (k === Qt.Key_Return || k === Qt.Key_Enter) { const ev = events[evIdx]; if (ev) { if (evBtn === 1) { openDetails(ev); sendLogs(ev); } else openDetails(ev); } }
        }
        e.accepted = true;
    }

    component Btn: Rectangle {
        id: b
        property string label: ""
        property bool primary: false
        property bool focused: false
        signal clicked()
        implicitWidth: bl.implicitWidth + 20
        implicitHeight: 28
        radius: 6
        color: primary ? v.cAccent : (ma.containsMouse ? "#2a3f55" : "transparent")
        border.width: primary ? 0 : 1
        border.color: v.cLine
        Rectangle { anchors.fill: parent; anchors.margins: -3; radius: 9; color: "transparent"; border.width: 2; border.color: v.cAccent; visible: b.focused }
        Text { id: bl; anchors.centerIn: parent; text: b.label; color: b.primary ? v.cInk : v.cText; font.family: v.font; font.pixelSize: 13; font.weight: b.primary ? Font.DemiBold : Font.Normal }
        MouseArea { id: ma; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: b.clicked() }
    }
    component Mark: Rectangle {
        property var k: v.kinds[0]
        width: 18; height: 18; radius: k.radius; color: k.color
        Text { anchors.centerIn: parent; text: parent.k.glyph; color: v.cInk; font.family: v.font; font.pixelSize: 11; font.weight: Font.Black }
    }

    Rectangle { anchors.fill: parent; color: v.cBg }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        // title bar
        Rectangle {
            Layout.fillWidth: true; Layout.preferredHeight: 40; color: v.cTitle
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 14; spacing: 10
                Canvas {
                    Layout.preferredWidth: 16; Layout.preferredHeight: 16
                    onPaint: { const c = getContext("2d"); c.reset(); c.scale(width / 24, height / 24); c.strokeStyle = v.cAccent; c.lineWidth = 2; c.lineCap = "round"; c.lineJoin = "round"; c.path = "M3 17l5-6 4 3 5-8 4 5"; c.stroke(); }
                }
                Text { Layout.fillWidth: true; text: "Reliability history"; color: v.cText; font.family: v.font; font.pixelSize: 13; font.weight: Font.Medium }
                Item {
                    Layout.preferredWidth: 46; Layout.fillHeight: true
                    Text { anchors.centerIn: parent; text: "✕"; color: v.cMuted; font.pixelSize: 12 }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: v.closeAll() }
                }
            }
        }
        // header
        Rectangle {
            Layout.fillWidth: true; Layout.preferredHeight: 64; color: v.cHead
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: v.cLine }
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 20; anchors.rightMargin: 20; spacing: 16
                ColumnLayout {
                    Layout.fillWidth: true; spacing: 2
                    Text { text: "How Hearth has been running"; color: v.cText; font.family: v.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                    Text { Layout.fillWidth: true; elide: Text.ElideRight; text: "Problems and changes on this PC, " + (v.weeks ? "week by week" : "day by day") + ". Click a " + (v.weeks ? "week" : "day") + " to see what happened."; color: v.cMuted; font.family: v.font; font.pixelSize: 13 }
                }
                Rectangle {
                    implicitWidth: tg.implicitWidth + 8; implicitHeight: 36; radius: 8; color: "transparent"
                    border.width: v.zone === "toggle" ? 2 : 0; border.color: v.cAccent
                    Row {
                        id: tg
                        anchors.centerIn: parent; spacing: 4
                        Repeater {
                            model: [["Days", false], ["Weeks", true]]
                            Rectangle {
                                required property var modelData
                                width: tt.implicitWidth + 24; height: 28; radius: 6
                                color: v.weeks === modelData[1] ? v.cPanel : "transparent"
                                Text { id: tt; anchors.centerIn: parent; text: modelData[0]; color: v.weeks === modelData[1] ? v.cText : v.cMuted; font.family: v.font; font.pixelSize: 13; font.weight: v.weeks === modelData[1] ? Font.DemiBold : Font.Normal }
                                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: v.setWeeks(modelData[1]) }
                            }
                        }
                    }
                }
            }
        }
        // chart
        Item {
            id: chart
            Layout.fillWidth: true
            Layout.preferredHeight: 120 + 4 * 30 + 26 + 12
            readonly property real labelW: 150
            readonly property int n: Math.max(1, v.rows.length)
            readonly property real colW: (width - 40 - labelW) / n
            Item {
                x: 20; y: 12; width: parent.width - 40; height: parent.height - 12
                // selected column
                Rectangle {
                    visible: v.rows.length > 0
                    x: chart.labelW + v.sel * chart.colW; y: 120; width: chart.colW; height: 4 * 30
                    color: v.cSelCol
                    border.width: v.zone === "chart" ? 1 : 0; border.color: v.cAccent
                }
                // y labels
                Item {
                    width: chart.labelW - 10; height: 110
                    Text { anchors.top: parent.top; width: parent.width; horizontalAlignment: Text.AlignRight; text: "10 · steady"; color: v.cMuted; font.family: v.font; font.pixelSize: 11 }
                    Text { anchors.verticalCenter: parent.verticalCenter; width: parent.width; horizontalAlignment: Text.AlignRight; text: "5"; color: v.cMuted; font.family: v.font; font.pixelSize: 11 }
                    Text { anchors.bottom: parent.bottom; width: parent.width; horizontalAlignment: Text.AlignRight; text: "1 · rocky"; color: v.cMuted; font.family: v.font; font.pixelSize: 11 }
                }
                Canvas {
                    id: line
                    x: chart.labelW; y: 0; width: chart.colW * chart.n; height: 110
                    property var scores: v.rows.map(r => r.score)
                    onScoresChanged: requestPaint()
                    onWidthChanged: requestPaint()
                    onPaint: {
                        const c = getContext("2d");
                        c.reset();
                        if (!scores.length) return;
                        c.strokeStyle = "#1e3044"; c.lineWidth = 1;
                        for (const g of [10, 5.5, 1]) { const y = 110 - (g - 1) / 9 * 104 - 3; c.beginPath(); c.moveTo(0, y); c.lineTo(width, y); c.stroke(); }
                        c.strokeStyle = v.cAccent; c.lineWidth = 3; c.lineJoin = "round"; c.lineCap = "round";
                        c.beginPath();
                        scores.forEach((s, i) => { const x = (i + 0.5) * chart.colW, y = 110 - (s - 1) / 9 * 104 - 3; if (i === 0) c.moveTo(x, y); else c.lineTo(x, y); });
                        c.stroke();
                    }
                    Accessible.role: Accessible.Graphic
                    Accessible.name: "Stability over " + v.rows.length + (v.weeks ? " weeks" : " days")
                }
                // the marks
                Repeater {
                    model: v.kinds
                    Item {
                        required property var modelData
                        required property int index
                        x: 0; y: 120 + index * 30; width: parent.width; height: 30
                        Rectangle { width: parent.width; height: 1; color: v.cGrid }
                        Text { width: chart.labelW - 10; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight; text: modelData.label; color: "#c9d6e2"; font.family: v.font; font.pixelSize: 12 }
                        Repeater {
                            model: v.rows
                            Item {
                                required property var modelData
                                required property int index
                                readonly property var kd: parent.modelData
                                x: chart.labelW + index * chart.colW; width: chart.colW; height: 30
                                Mark { anchors.centerIn: parent; k: kd; visible: (modelData.counts[kd.id] || 0) > 0 }
                            }
                        }
                    }
                }
                // days
                Item {
                    y: 120 + 4 * 30; width: parent.width; height: 26
                    Rectangle { width: parent.width; height: 1; color: v.cGrid }
                    Repeater {
                        model: v.rows.length
                        Text {
                            required property int index
                            x: chart.labelW + index * chart.colW; width: chart.colW; height: 26
                            horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
                            text: v.dayLabel(index)
                            color: index === v.sel ? v.cText : v.cMuted; font.family: v.font; font.pixelSize: 11; font.weight: index === v.sel ? Font.DemiBold : Font.Normal
                        }
                    }
                }
                // a click picks the day (or week)
                MouseArea {
                    x: chart.labelW; width: chart.colW * chart.n; height: parent.height
                    cursorShape: Qt.PointingHandCursor
                    onClicked: m => { v.sel = Math.max(0, Math.min(v.rows.length - 1, Math.floor(m.x / chart.colW))); v.evIdx = 0; v.zone = "chart"; v.forceActiveFocus(); }
                }
            }
        }
        // the day's events
        Rectangle {
            Layout.fillWidth: true; Layout.fillHeight: true
            Layout.leftMargin: 20; Layout.rightMargin: 20; Layout.bottomMargin: 16; Layout.topMargin: 0
            radius: 10; color: v.cPanel; border.width: 1; border.color: v.cLine
            clip: true
            ColumnLayout {
                anchors.fill: parent; anchors.margins: 1; spacing: 0
                Item {
                    Layout.fillWidth: true; Layout.preferredHeight: 40
                    Text {
                        anchors.verticalCenter: parent.verticalCenter; x: 14
                        text: v.loading ? "Reading the system's log…" : v.curRow ? v.longDay(v.curRow) + " · " + v.events.length + (v.events.length === 1 ? " event" : " events") : "Nothing to show yet"
                        color: v.cText; font.family: v.font; font.pixelSize: 13; font.weight: Font.DemiBold
                    }
                    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: v.cLine }
                }
                ListView {
                    id: evList
                    Layout.fillWidth: true; Layout.fillHeight: true
                    clip: true
                    model: v.events
                    currentIndex: v.evIdx
                    onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                    boundsBehavior: Flickable.StopAtBounds
                    delegate: Rectangle {
                        id: er
                        required property var modelData
                        required property int index
                        readonly property bool problem: modelData.cat === "app" || modelData.cat === "hearth"
                        readonly property bool here: v.zone === "events" && v.evIdx === index
                        width: evList.width; height: 52
                        color: here ? "#2a3f55" : "transparent"
                        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: v.cGrid }
                        RowLayout {
                            anchors.fill: parent; anchors.leftMargin: 14; anchors.rightMargin: 14; spacing: 10
                            Item { Layout.preferredWidth: 28; Layout.fillHeight: true; Mark { anchors.verticalCenter: parent.verticalCenter; k: v.kind(er.modelData.cat) } }
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 2
                                Text { Layout.fillWidth: true; elide: Text.ElideRight; text: er.modelData.t; color: v.cText; font.family: v.font; font.pixelSize: 13; font.weight: Font.Medium }
                                Text { Layout.fillWidth: true; elide: Text.ElideRight; text: er.modelData.s || ""; color: v.cMuted; font.family: v.font; font.pixelSize: 12 }
                            }
                            Text { Layout.preferredWidth: v.weeks ? 130 : 90; text: v.timeOf(er.modelData.time); color: v.cMuted; font.family: v.font; font.pixelSize: 13 }
                            Row {
                                Layout.preferredWidth: 200; layoutDirection: Qt.RightToLeft; spacing: 6
                                Btn { visible: er.problem; primary: true; label: "Send logs"; focused: er.here && v.evBtn === 1; onClicked: { v.openDetails(er.modelData); v.sendLogs(er.modelData); } }
                                Btn { label: "Details"; focused: er.here && v.evBtn === 0; onClicked: v.openDetails(er.modelData) }
                            }
                        }
                        MouseArea { anchors.fill: parent; z: -1; onClicked: { v.zone = "events"; v.evIdx = er.index; v.forceActiveFocus(); } }
                    }
                }
                Text {
                    visible: !v.loading && v.curRow !== null && v.events.length === 0
                    Layout.fillWidth: true; Layout.margins: 14
                    text: "Nothing happened " + (v.weeks ? "this week" : "on this day") + ". Hearth ran steadily."
                    color: v.cMuted; font.family: v.font; font.pixelSize: 13
                }
            }
        }
    }

    // ---- Details
    Rectangle {
        anchors.fill: parent
        visible: v.details !== null
        color: "#b3000000"
        MouseArea { anchors.fill: parent; onClicked: v.details = null }
        Rectangle {
            anchors.centerIn: parent
            width: Math.min(parent.width - 80, 600)
            height: dcol.implicitHeight + 40
            radius: 10; color: v.cPanel; border.width: 1; border.color: v.cLine
            MouseArea { anchors.fill: parent }
            ColumnLayout {
                id: dcol
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 20 }
                spacing: 10
                RowLayout {
                    Layout.fillWidth: true; spacing: 10
                    Mark { k: v.kind(v.details ? v.details.cat : "info") }
                    Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: v.details ? v.details.t : ""; color: v.cText; font.family: v.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                }
                Text { Layout.fillWidth: true; text: v.details ? new Date(v.details.time * 1000).toLocaleString(Qt.locale(), Locale.LongFormat) : ""; color: v.cMuted; font.family: v.font; font.pixelSize: 12 }
                Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; visible: text !== ""; text: v.details ? (v.details.s || "") : ""; color: v.cText; font.family: v.font; font.pixelSize: 13 }
                Rectangle {
                    Layout.fillWidth: true; visible: v.details && v.details.detail
                    implicitHeight: dt.implicitHeight + 20; radius: 6; color: "#1b2838"
                    Text { id: dt; x: 10; y: 10; width: parent.width - 20; wrapMode: Text.WrapAnywhere; text: v.details ? (v.details.detail || "") : ""; color: "#c9d6e2"; font.family: "monospace"; font.pixelSize: 12 }
                }
                Text {
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    visible: v.sending || v.sendMsg !== ""
                    text: v.sending ? "Sending Hearth's logs…" : v.sendMsg
                    color: v.cMuted; font.family: v.font; font.pixelSize: 13
                }
                Row {
                    Layout.alignment: Qt.AlignRight; spacing: 8
                    Repeater {
                        model: v.dlgButtons
                        Btn {
                            required property string modelData
                            required property int index
                            primary: modelData === "send"
                            label: modelData === "send" ? (v.sending ? "Sending…" : "Send logs") : "Close"
                            focused: v.dlgIdx === index
                            onClicked: modelData === "send" ? v.sendLogs(v.details) : (v.details = null)
                        }
                    }
                }
            }
        }
    }
}

// Timer and stopwatch widget. Timer: pick 1, 5, 10, 15, 25 or 60 minutes (or − / + a minute), Start, Pause, Reset;
// when it's done you get a notification and the widget glows. Stopwatch: Start, Lap, Pause, Reset. The minutes you
// picked last are remembered; a running timer isn't (it's gone after a restart, as on a phone). In the bar: the time
// left while it runs.
import QtQuick
import QtQuick.Layouts
import Quickshell
import "widget-layout.js" as L

WidgetBase {
    id: w
    property string mode: "timer"
    readonly property int minutes: c("minutes", 5)
    property double endsAt: 0              // timer running: when it's done (ms)
    property double leftMs: minutes * 60000  // timer paused: what's left
    property bool done: false
    property double swStart: 0             // stopwatch running since (ms)
    property double swAcc: 0
    property var laps: []
    property double now: Date.now()
    readonly property bool timerRunning: endsAt > 0
    readonly property bool swRunning: swStart > 0
    readonly property double remaining: timerRunning ? Math.max(0, endsAt - now) : leftMs
    readonly property double elapsed: swAcc + (swRunning ? now - swStart : 0)
    implicitWidth: compact ? barRow.implicitWidth : 290
    implicitHeight: compact ? 24 : 230
    onMinutesChanged: if (!timerRunning) { leftMs = minutes * 60000; done = false; }

    Timer {
        interval: w.swRunning ? 100 : 250
        running: w.timerRunning || w.swRunning
        repeat: true
        onTriggered: {
            w.now = Date.now();
            if (w.timerRunning && w.now >= w.endsAt) {
                w.endsAt = 0; w.leftMs = 0; w.done = true;
                Quickshell.execDetached(["notify-send", "-a", "Hearth", "-i", "alarm-symbolic", "Timer", "Your " + w.minutes + " minute timer is done"]);
            }
        }
    }
    function startTimer() { if (leftMs <= 0) leftMs = minutes * 60000; done = false; now = Date.now(); endsAt = now + leftMs; }
    function pauseTimer() { leftMs = remaining; endsAt = 0; }
    function resetTimer() { endsAt = 0; leftMs = minutes * 60000; done = false; }
    function pick(m) { set("minutes", Math.max(1, Math.min(600, m))); if (!timerRunning) { leftMs = Math.max(1, Math.min(600, m)) * 60000; done = false; } }

    RowLayout {
        id: barRow
        visible: w.compact
        anchors.verticalCenter: parent.verticalCenter
        spacing: 4
        Icon { name: "bell"; size: 15; color: w.done ? Theme.accent : Theme.muted }
        Text { text: w.timerRunning || w.done ? L.fmtDuration(w.remaining) : w.swRunning ? L.fmtDuration(w.elapsed) : "Timer"; color: w.done ? Theme.accent : Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold
                MouseArea { anchors.fill: parent; onClicked: w.timerRunning ? w.pauseTimer() : w.startTimer() } }
    }

    Rectangle {
        visible: !w.compact && w.done
        anchors.fill: parent
        radius: Theme.r(14)
        color: "transparent"
        border.width: 3
        border.color: Theme.accent
        SequentialAnimation on opacity { running: w.done && !Settings.reduceMotion; loops: Animation.Infinite; NumberAnimation { to: 0.3; duration: 600 } NumberAnimation { to: 1; duration: 600 } }
    }
    ColumnLayout {
        visible: !w.compact
        anchors { fill: parent; margins: 12 }
        spacing: 6
        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 4
            WBtn { text: "Timer"; on: w.mode === "timer"; size: 26; onClicked: w.mode = "timer" }
            WBtn { text: "Stopwatch"; on: w.mode === "stopwatch"; size: 26; onClicked: w.mode = "stopwatch" }
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            Layout.fillHeight: true
            verticalAlignment: Text.AlignVCenter
            text: w.mode === "timer" ? (w.done ? "Done" : L.fmtDuration(w.remaining)) : L.fmtStopwatch(w.elapsed)
            color: w.done && w.mode === "timer" ? Theme.accent : Theme.text
            font.family: Theme.font; font.pixelSize: Math.round(46 * w.k); font.weight: Font.Bold
        }
        Flow {
            visible: w.mode === "timer" && !w.timerRunning && w.height > 200
            Layout.alignment: Qt.AlignHCenter
            Layout.maximumWidth: parent.width
            spacing: 4
            Repeater {
                model: [1, 5, 10, 15, 25, 60]
                delegate: WBtn { required property var modelData; text: modelData + "m"; size: 24; flat: true; on: w.minutes === modelData; onClicked: w.pick(modelData) }
            }
        }
        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 8
            WBtn { visible: w.mode === "timer"; text: "−"; enabled: !w.timerRunning; onClicked: w.pick(w.minutes - 1) }
            WBtn { visible: w.mode === "timer"; text: w.timerRunning ? "Pause" : "Start"; on: true; onClicked: w.timerRunning ? w.pauseTimer() : w.startTimer() }
            WBtn { visible: w.mode === "timer"; text: "Reset"; onClicked: w.resetTimer() }
            WBtn { visible: w.mode === "timer"; text: "+"; enabled: !w.timerRunning; onClicked: w.pick(w.minutes + 1) }
            WBtn { visible: w.mode === "stopwatch"; text: w.swRunning ? "Pause" : "Start"; on: true; onClicked: { if (w.swRunning) { w.swAcc += Date.now() - w.swStart; w.swStart = 0; } else { w.now = Date.now(); w.swStart = w.now; } } }
            WBtn { visible: w.mode === "stopwatch"; text: "Lap"; enabled: w.swRunning; onClicked: w.laps = [w.elapsed].concat(w.laps).slice(0, 20) }
            WBtn { visible: w.mode === "stopwatch"; text: "Reset"; onClicked: { w.swStart = 0; w.swAcc = 0; w.laps = []; } }
        }
        Text {
            visible: w.mode === "stopwatch" && w.laps.length > 0
            Layout.alignment: Qt.AlignHCenter
            text: w.laps.slice(0, 3).map((l, i) => "Lap " + (w.laps.length - i) + "  " + L.fmtStopwatch(l)).join("   ")
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
        }
    }
}

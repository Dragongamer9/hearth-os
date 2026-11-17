// Settings › Display › Calibrate HDR (1.14, Mantel on KWin): Hearth's own three steps, full screen on the screen being
// set, like Plasma 6.4's: the brightest a small spot gets (peak), the brightest the whole screen gets (average), and
// the darkest it shows (lowest). Each step shows a test picture and a value you move with Left/Right (the D-pad) or the
// mouse; the screen changes as you move it (hearth-screens apply: KWin's max/average/min brightness overrides).
// Finish hands the three values to the page's "Keep these settings?" (so a wrong one goes back by itself); Cancel or
// Esc / B puts back what was there. "What the screen says" (no override) is one press of Reset in each step.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

Scope {
    id: cal
    required property var page
    property bool shown: false
    property int step: 0              // 0 peak, 1 average, 2 lowest
    property var start0: null         // hdrCal as it was (to go back to)
    property real peak: 0
    property real avg: 0
    property real low: 0
    readonly property var o: page.out
    readonly property var c: o && o.hdrCal ? o.hdrCal : null
    readonly property var steps: [
        { title: "Brightest spot", sub: "Move it down until the small square in the middle only just stands out from the box around it, then press Enter.", lo: 100, hi: 2000, unit: "nits" },
        { title: "Brightest across the screen", sub: "Move it down until the white picture looks as bright as it can without flickering or dimming after a moment.", lo: 80, hi: 1500, unit: "nits" },
        { title: "Darkest", sub: "Move it until you can just see every one of the dark squares, and the background stays black.", lo: 0, hi: 0.5, unit: "nits" }
    ]
    readonly property var cur: steps[step]
    readonly property real val: step === 0 ? peak : step === 1 ? avg : low

    function start() {
        if (!c || !c.capable) return;
        start0 = { peak: c.peakSet, avg: c.avgSet, min: c.minSet };
        peak = c.peakSet || c.peak || 600;
        avg = c.avgSet || c.avg || Math.min(peak, 400);
        low = c.minSet !== null && c.minSet !== undefined ? c.minSet : (c.min || 0.005);
        step = 0;
        shown = true;
    }
    function set(v) {
        const s = steps[step];
        v = Math.max(s.lo, Math.min(s.hi, v));
        if (step === 0) peak = Math.round(v); else if (step === 1) avg = Math.round(Math.min(v, peak)); else low = Math.round(v * 10000) / 10000;
        push.restart();
    }
    function nudge(d) { set(val + d * (step === 2 ? 0.002 : step === 0 ? 20 : 10)); }
    function reset() {
        if (step === 0) peak = c.peak || 600; else if (step === 1) avg = c.avg || 400; else low = c.min || 0;
        push.restart();
    }
    // The screen shows each change at once (nothing saved).
    Timer { id: push; interval: 250; onTriggered: { const ch = {}; ch[cal.o.name] = { hdrCal: { peak: cal.peak, avg: cal.avg, min: cal.low } }; calJob.run(["apply", JSON.stringify({ outputs: ch })], () => {}, e => { cal.page.problem = e; }); } }
    SetJob { id: calJob; helper: "hearth-screens" }
    function finish() {
        shown = false;
        // The page's Keep countdown: going back puts start0 back.
        const ch = {}; ch[o.name] = { hdrCal: { peak: peak, avg: avg, min: low } };
        const back = {}; back[o.name] = { hdrCal: start0 };
        page.prev = { outputs: back };
        page.trying = [o.name];
        page.problem = ""; page.notice = "";
        page.countdown = 15;
        calJob.run(["apply", JSON.stringify({ outputs: ch })], r => page.took(r), e => { page.problem = e; page.countdown = 0; });
    }
    function cancel() {
        shown = false;
        const ch = {}; ch[o.name] = { hdrCal: start0 };
        calJob.run(["apply", JSON.stringify({ outputs: ch })], r => page.took(r), e => { page.problem = e; });
    }

    PanelWindow {
        visible: cal.shown
        screen: Quickshell.screens.find(s => cal.o && s.name === cal.o.name) || (Quickshell.screens.length ? Quickshell.screens[0] : null)
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-hdr-calibrate"
        WlrLayershell.keyboardFocus: cal.shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "black"
        onVisibleChanged: if (visible) keys.forceActiveFocus()

        Item {
            id: keys
            anchors.fill: parent
            focus: true
            Keys.onPressed: e => {
                if (e.key === Qt.Key_Left || e.key === Qt.Key_Down) cal.nudge(-1);
                else if (e.key === Qt.Key_Right || e.key === Qt.Key_Up) cal.nudge(1);
                else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space) { if (cal.step < 2) cal.step++; else cal.finish(); }
                else if (e.key === Qt.Key_Backspace) { if (cal.step > 0) cal.step--; else cal.cancel(); }
                else if (e.key === Qt.Key_Escape) cal.cancel();
                else if (e.key === Qt.Key_R) cal.reset();
                else return;
                e.accepted = true;
            }

            // The test pictures.
            Rectangle {     // 0: a white box with a slightly less bright square in the middle (a spot of 10 % of the screen)
                visible: cal.step === 0
                anchors.centerIn: parent
                width: parent.width * 0.32; height: width
                color: "white"
                Rectangle { anchors.centerIn: parent; width: parent.width * 0.4; height: width; color: "#f4f4f4" }
            }
            Rectangle {     // 1: the whole screen white
                visible: cal.step === 1
                anchors.fill: parent
                color: "white"
            }
            Row {           // 2: near-black squares on black
                visible: cal.step === 2
                anchors.centerIn: parent
                spacing: 24
                Repeater {
                    model: [1, 2, 3, 4, 6, 8]
                    Rectangle { required property int modelData; width: 110; height: 110; radius: 8; color: Qt.rgba(modelData / 255, modelData / 255, modelData / 255, 1) }
                }
            }

            // The card with the step and its value.
            Rectangle {
                anchors { bottom: parent.bottom; bottomMargin: 48; horizontalCenter: parent.horizontalCenter }
                width: Math.min(parent.width - 64, 760)
                height: card.implicitHeight + 40
                radius: Theme.r(18)
                color: Theme.panel
                border.color: Theme.line
                ColumnLayout {
                    id: card
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 20 }
                    spacing: 10
                    RowLayout {
                        Layout.fillWidth: true
                        Text { text: "Calibrate HDR · " + (cal.step + 1) + " of 3 · " + cal.cur.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold; Layout.fillWidth: true; elide: Text.ElideRight }
                        Text { text: (cal.step === 2 ? cal.val.toFixed(3) : Math.round(cal.val)) + " " + cal.cur.unit; color: Theme.accent; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.Bold }
                    }
                    Text { Layout.fillWidth: true; text: cal.cur.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap }
                    Item {
                        id: track
                        Layout.fillWidth: true
                        implicitHeight: 24
                        readonly property real f: (cal.val - cal.cur.lo) / (cal.cur.hi - cal.cur.lo)
                        Rectangle { anchors.verticalCenter: parent.verticalCenter; width: parent.width; height: 6; radius: 3; color: Theme.panel3 }
                        Rectangle { anchors.verticalCenter: parent.verticalCenter; width: Math.max(6, parent.width * track.f); height: 6; radius: 3; color: Theme.accent }
                        Rectangle { x: parent.width * track.f - 11; anchors.verticalCenter: parent.verticalCenter; width: 22; height: 22; radius: 11; color: Theme.accent; border.width: 4; border.color: Theme.panel }
                        MouseArea {
                            anchors { fill: parent; topMargin: -10; bottomMargin: -10 }
                            onPressed: m => cal.set(cal.cur.lo + (cal.cur.hi - cal.cur.lo) * Math.max(0, Math.min(1, m.x / width)))
                            onPositionChanged: m => { if (pressed) cal.set(cal.cur.lo + (cal.cur.hi - cal.cur.lo) * Math.max(0, Math.min(1, m.x / width))); }
                        }
                    }
                    Text {
                        Layout.fillWidth: true
                        text: "The screen says: " + (cal.step === 0 ? Math.round(cal.c ? cal.c.peak : 0) : cal.step === 1 ? Math.round(cal.c ? cal.c.avg : 0) : (cal.c ? cal.c.min : 0)) + " nits  ·  Left/Right or D-pad: change  ·  R: what the screen says  ·  Enter or A: " + (cal.step < 2 ? "next" : "finish") + "  ·  Esc or B: cancel"
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
                    }
                    RowLayout {
                        spacing: 10
                        Item { Layout.fillWidth: true }
                        SetButton { ghost: true; text: "Cancel"; onActivated: cal.cancel() }
                        SetButton { ghost: true; text: "Reset"; onActivated: cal.reset() }
                        SetButton { text: cal.step < 2 ? "Next" : "Finish"; onActivated: { if (cal.step < 2) cal.step++; else cal.finish(); } }
                    }
                }
            }
        }
    }
}

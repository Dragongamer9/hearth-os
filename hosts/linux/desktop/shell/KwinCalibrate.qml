// Touchscreen calibration (Settings › Drawing tablet & touchscreen, Mantel on KWin): a full-screen layer on the touch
// screen with four targets. The touchscreen's calibration is set to none first, so each touch lands where the screen
// reports it; the four touches then give the correction (least squares, libinput's 2×3 matrix in KWin's 4×4 form),
// handed back in done(matrix). Esc or B, or 20 seconds without a touch, cancels and done("") puts the old one back.
import QtQuick
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: cal
    property var target: null          // the Quickshell screen
    property var points: []            // [{tx, ty, rx, ry}] target and touched, 0..1
    signal done(string matrix)
    screen: target
    color: "#e0101418"
    anchors { top: true; bottom: true; left: true; right: true }
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-calibrate"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    readonly property var targets: [[0.1, 0.1], [0.9, 0.1], [0.9, 0.9], [0.1, 0.9]]
    readonly property int step: points.length

    function solve() {
        // For each output row (x, then y): a*rx + b*ry + c = t, least squares over the four touches.
        function row(sel) {
            let m = [[0, 0, 0], [0, 0, 0], [0, 0, 0]], v = [0, 0, 0];
            for (const p of cal.points) {
                const r = [p.rx, p.ry, 1], t = sel(p);
                for (let i = 0; i < 3; i++) { v[i] += r[i] * t; for (let j = 0; j < 3; j++) m[i][j] += r[i] * r[j]; }
            }
            // Gaussian elimination
            for (let i = 0; i < 3; i++) {
                let piv = i;
                for (let k = i + 1; k < 3; k++) if (Math.abs(m[k][i]) > Math.abs(m[piv][i])) piv = k;
                [m[i], m[piv]] = [m[piv], m[i]]; [v[i], v[piv]] = [v[piv], v[i]];
                if (Math.abs(m[i][i]) < 1e-9) return null;
                for (let k = 0; k < 3; k++) if (k !== i) {
                    const f = m[k][i] / m[i][i];
                    for (let j = 0; j < 3; j++) m[k][j] -= f * m[i][j];
                    v[k] -= f * v[i];
                }
            }
            return [v[0] / m[0][0], v[1] / m[1][1], v[2] / m[2][2]];
        }
        const a = row(p => p.tx), b = row(p => p.ty);
        if (!a || !b) return "";
        const r = x => (Math.round(x * 10000) / 10000).toString();
        // KWin's form (backends/libinput/device.cpp serializeMatrix): a 4×4 matrix, column by column.
        return [a[0], b[0], 0, 0, a[1], b[1], 0, 0, a[2], b[2], 1, 0, 0, 0, 0, 1].map(r).join(",");
    }
    Timer { id: giveUp; interval: 20000; running: cal.visible; onTriggered: cal.done("") }

    Item {
        anchors.fill: parent
        focus: true
        Keys.onEscapePressed: cal.done("")
        Text {
            anchors.centerIn: parent
            width: parent.width * 0.6
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            text: "Touch the centre of each cross with a finger, as exactly as you can (" + (cal.step + 1) + " of 4).\nEsc cancels."
            color: "white"; font.family: Theme.font; font.pixelSize: 22
        }
        Repeater {
            model: cal.targets
            delegate: Item {
                required property var modelData
                required property int index
                visible: index === cal.step
                x: parent.width * modelData[0] - 30; y: parent.height * modelData[1] - 30
                width: 60; height: 60
                Rectangle { anchors.centerIn: parent; width: 60; height: 3; color: Theme.accent }
                Rectangle { anchors.centerIn: parent; width: 3; height: 60; color: Theme.accent }
                Rectangle { anchors.centerIn: parent; width: 18; height: 18; radius: 9; color: "transparent"; border.width: 2; border.color: "white" }
            }
        }
        MultiPointTouchArea {
            anchors.fill: parent
            maximumTouchPoints: 1
            onReleased: pts => {
                if (!pts.length || cal.step >= 4) return;
                const t = cal.targets[cal.step];
                cal.points = cal.points.concat([{ tx: t[0], ty: t[1], rx: pts[0].x / width, ry: pts[0].y / height }]);
                giveUp.restart();
                if (cal.points.length === 4) cal.done(cal.solve());
            }
        }
    }
}

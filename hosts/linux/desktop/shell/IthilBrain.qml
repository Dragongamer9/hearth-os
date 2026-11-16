// 1.13 (mockups next-update/server-ithil-dash.png, server-ithil-detail.png): Ithil's live 3D / 4D brain in Server
// mode, drawn from ithil-brain.js (the team's viewer, the same brain as her app's Mind screen). Draw-only: `brain` is
// `hearth-server-mode ithil-brain <hours>` (her real event log, read-only). live: the last seconds, as they happen;
// otherwise tpos moves through the window (0..120, the 4th dimension).
import QtQuick
import "ithil-brain.js" as B

Canvas {
    id: cv
    property var brain: null
    property bool compact: false          // the dashboard tile: fewer points, no labels
    property bool live: true
    property bool asleep: false
    property real tpos: B.TOTAL      // where in the window (0..120, the 4th dimension)
    property bool xray: false
    property bool labels: false
    property bool links: true
    property bool autoRot: true
    property int sel: -1
    property real yaw: -1.1
    property real pitch: 0.22
    property real zoom: compact ? 1.05 : 1
    property var prepared: B.prepare(null)
    property var act: []
    readonly property var parts: B.R
    // Canvas.Image (the default) on purpose: FramebufferObject is OpenGL-only and left the software-drawn dashboard blank
    renderStrategy: Canvas.Cooperative
    onBrainChanged: prepared = B.prepare(brain)

    // The right stick moves the pointer: over her brain it turns it (no click needed); dragging works too.
    MouseArea {
        anchors.fill: parent
        hoverEnabled: !cv.compact
        enabled: !cv.compact
        property real lx: -1
        property real ly: -1
        onPositionChanged: m => {
            if (lx >= 0) { cv.yaw += (m.x - lx) * 0.01; cv.pitch = Math.max(-1.4, Math.min(1.4, cv.pitch + (m.y - ly) * 0.01)); }
            lx = m.x; ly = m.y;
        }
        onExited: { lx = -1; ly = -1; }
        onWheel: w => cv.zoom = Math.max(0.5, Math.min(2.5, cv.zoom * Math.exp(-w.angleDelta.y * 0.001)))
    }

    Timer {
        interval: cv.compact ? 120 : 50
        running: cv.visible
        repeat: true
        property double last: Date.now()
        onTriggered: {
            const now = Date.now(), dt = Math.min(0.2, (now - last) / 1000); last = now;
            if (cv.autoRot) cv.yaw += dt * 0.18;
            cv.act = B.activity(cv.prepared, cv.tpos, now, cv.live, cv.asleep);
            cv.requestPaint();
        }
    }
    onPaint: {
        const ctx = getContext("2d");
        if (!ctx || width < 2 || height < 2) return;
        B.draw(ctx, { W: width, H: height, dpr: 1, yaw: yaw, pitch: pitch, zoom: zoom, act: act.length ? act : B.R.map(() => 0.08), sel: sel,
                      xray: xray, labels: labels && !compact, links: links, step: compact ? 3 : 1, data: prepared, T: tpos, nowMs: Date.now(), live: live });
    }
    function moodAt(t) { return B.moodAt(prepared, t); }
    function sleepiness(t) { return B.sleepiness(prepared, t); }
}

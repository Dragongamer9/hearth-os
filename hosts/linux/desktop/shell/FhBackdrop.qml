// Firehold's backdrop: a soft ellipse of firelight (the mockups' radial-gradient(RX RY at CX CY, C0, C1 50%, C2)).
import QtQuick

Canvas {
    id: c
    property real cx: 0.7
    property real cy: 0.2
    property real rx: 1400
    property real ry: 800
    property real k: 1
    property string c0: "#4a2410"
    property string c1: "#1d120c"
    property string c2: "#0f0b09"
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()
    onC0Changed: requestPaint()
    onPaint: {
        const ctx = getContext("2d");
        ctx.reset();
        ctx.fillStyle = c2;
        ctx.fillRect(0, 0, width, height);
        const r = rx * k, sy = ry / rx;
        ctx.save();
        ctx.translate(cx * width, cy * height);
        ctx.scale(1, sy);
        const g = ctx.createRadialGradient(0, 0, 0, 0, 0, r);
        g.addColorStop(0, c0);
        g.addColorStop(0.5, c1);
        g.addColorStop(1, c2);
        ctx.fillStyle = g;
        ctx.fillRect(-cx * width, -cy * height / sy, width, height / sy);
        ctx.restore();
    }
}

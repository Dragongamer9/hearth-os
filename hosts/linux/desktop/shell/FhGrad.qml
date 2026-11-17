// A rounded box filled with a CSS-style angled gradient (Firehold's game art placeholders: 110deg, ember to night).
//   stops: [[0, "#2b1a2e"], [0.55, "#5b2b1a"], [1, "#a3541e"]]; angle in degrees as CSS (0 = up, 90 = right)
import QtQuick

Canvas {
    id: c
    property var stops: [[0, "#2b1a2e"], [0.55, "#5b2b1a"], [1, "#a3541e"]]
    property real angle: 110
    property real radius: 0
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()
    onStopsChanged: requestPaint()
    onRadiusChanged: requestPaint()
    onPaint: {
        const ctx = getContext("2d");
        ctx.reset();
        if (width <= 0 || height <= 0) return;
        const a = angle * Math.PI / 180, dx = Math.sin(a), dy = -Math.cos(a);
        const L = Math.abs(width * dx) + Math.abs(height * dy), x0 = width / 2, y0 = height / 2;
        const g = ctx.createLinearGradient(x0 - dx * L / 2, y0 - dy * L / 2, x0 + dx * L / 2, y0 + dy * L / 2);
        for (const s of stops) g.addColorStop(s[0], s[1]);
        const r = Math.min(radius, width / 2, height / 2);
        ctx.beginPath();
        ctx.moveTo(r, 0);
        ctx.lineTo(width - r, 0); ctx.arcTo(width, 0, width, r, r);
        ctx.lineTo(width, height - r); ctx.arcTo(width, height, width - r, height, r);
        ctx.lineTo(r, height); ctx.arcTo(0, height, 0, height - r, r);
        ctx.lineTo(0, r); ctx.arcTo(0, 0, r, 0, r);
        ctx.closePath();
        ctx.fillStyle = g;
        ctx.fill();
    }
}

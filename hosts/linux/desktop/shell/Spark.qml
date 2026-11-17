// A small graph of the last samples (the system monitor widget): a line with a soft fill under it, 0–max.
import QtQuick

Canvas {
    id: sp
    property var values: []
    property real max: 100
    property color tint: Theme.accent
    onValuesChanged: requestPaint()
    onTintChanged: requestPaint()
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()
    onPaint: {
        const g = getContext("2d");
        g.reset();
        const v = values || [], n = Math.max(2, v.length), w = width, h = height;
        if (v.length < 2) return;
        const mx = Math.max(max, ...v, 1);
        g.beginPath();
        for (let i = 0; i < v.length; i++) {
            const x = (i + n - v.length) / (n - 1) * w, y = h - Math.max(0, v[i]) / mx * (h - 2) - 1;
            if (i === 0) g.moveTo(x, y); else g.lineTo(x, y);
        }
        g.strokeStyle = tint;
        g.lineWidth = 2;
        g.stroke();
        g.lineTo(w, h);
        g.lineTo((n - v.length) / (n - 1) * w, h);
        g.closePath();
        g.fillStyle = Qt.rgba(tint.r, tint.g, tint.b, 0.18);
        g.fill();
    }
}

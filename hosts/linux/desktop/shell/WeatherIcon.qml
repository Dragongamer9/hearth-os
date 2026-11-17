// The weather widget's picture, drawn (no font needed): sun, moon, cloud, rain, snow, fog or storm
// (widget-layout.js weatherKind).
import QtQuick

Canvas {
    id: wi
    property string kind: "cloud"
    property color sun: "#ffc94d"
    property color cloud: Theme.text
    onKindChanged: requestPaint()
    onWidthChanged: requestPaint()
    onPaint: {
        const g = getContext("2d");
        g.reset();
        const s = Math.min(width, height), k = kind;
        function circle(x, y, r, c) { g.beginPath(); g.arc(x * s, y * s, r * s, 0, Math.PI * 2); g.fillStyle = c; g.fill(); }
        function puff(dx, dy, sc, c) {
            circle(0.42 + dx, 0.58 + dy, 0.16 * sc, c); circle(0.6 + dx, 0.52 + dy, 0.2 * sc, c);
            circle(0.76 + dx, 0.62 + dy, 0.13 * sc, c);
            g.fillStyle = c; g.fillRect((0.3 + dx) * s, (0.6 + dy) * s, 0.5 * s * sc, 0.15 * s * sc);
        }
        if (k === "sun" || k === "sun-cloud") {
            const cx = k === "sun" ? 0.5 : 0.36, cy = k === "sun" ? 0.5 : 0.38, r = k === "sun" ? 0.2 : 0.15;
            g.strokeStyle = wi.sun; g.lineWidth = Math.max(2, s * 0.05); g.lineCap = "round";
            for (let i = 0; i < 8; i++) {
                const a = i * Math.PI / 4;
                g.beginPath(); g.moveTo((cx + Math.cos(a) * (r + 0.07)) * s, (cy + Math.sin(a) * (r + 0.07)) * s);
                g.lineTo((cx + Math.cos(a) * (r + 0.15)) * s, (cy + Math.sin(a) * (r + 0.15)) * s); g.stroke();
            }
            circle(cx, cy, r, wi.sun);
        }
        if (k === "moon" || k === "moon-cloud") {
            circle(0.42, 0.4, 0.2, "#f1e7c8");
            circle(0.5, 0.33, 0.17, Qt.rgba(0, 0, 0, 0));
            g.globalCompositeOperation = "destination-out"; circle(0.52, 0.32, 0.16, "#000"); g.globalCompositeOperation = "source-over";
        }
        if (k !== "sun" && k !== "moon") {
            const c = k === "storm" ? Qt.darker(wi.cloud, 1.6) : k === "rain" || k === "snow" ? Qt.darker(wi.cloud, 1.2) : wi.cloud;
            puff(0, -0.04, 1, c);
        }
        if (k === "rain" || k === "storm") {
            g.strokeStyle = "#66b3ff"; g.lineWidth = Math.max(2, s * 0.04); g.lineCap = "round";
            for (let i = 0; i < 3; i++) { g.beginPath(); g.moveTo((0.42 + i * 0.14) * s, 0.8 * s); g.lineTo((0.38 + i * 0.14) * s, 0.92 * s); g.stroke(); }
        }
        if (k === "snow") for (let i = 0; i < 3; i++) circle(0.42 + i * 0.14, 0.86, 0.035, "#ffffff");
        if (k === "fog") { g.fillStyle = wi.cloud; for (let i = 0; i < 2; i++) g.fillRect(0.28 * s, (0.8 + i * 0.08) * s, 0.5 * s, 0.035 * s); }
        if (k === "storm") {
            g.beginPath(); g.moveTo(0.58 * s, 0.7 * s); g.lineTo(0.48 * s, 0.86 * s); g.lineTo(0.56 * s, 0.86 * s); g.lineTo(0.5 * s, 0.98 * s);
            g.lineTo(0.66 * s, 0.8 * s); g.lineTo(0.58 * s, 0.8 * s); g.closePath(); g.fillStyle = "#ffd24d"; g.fill();
        }
    }
}

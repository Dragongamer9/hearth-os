// Hearth Capture's tool pictures, drawn (Hearth's icon set has no arrow, highlighter or QR code): one colour, rounded
// 2px lines, like the rest of Mantel's icons. Names: arrow box pen highlighter text blur pixelate steps crop area
// window screen all qr scan record timer pointer redo.
import QtQuick

Canvas {
    id: g
    property string name: ""
    property color color: Theme.text
    property real size: 18
    implicitWidth: size
    implicitHeight: size
    width: size
    height: size
    onNameChanged: requestPaint()
    onColorChanged: requestPaint()
    onSizeChanged: requestPaint()
    onPaint: {
        const c = getContext("2d");
        c.reset();
        const k = width / 24;
        c.scale(k, k);
        c.strokeStyle = g.color; c.fillStyle = g.color;
        c.lineWidth = 2; c.lineCap = "round"; c.lineJoin = "round";
        const line = (pts) => { c.beginPath(); c.moveTo(pts[0], pts[1]); for (let i = 2; i < pts.length; i += 2) c.lineTo(pts[i], pts[i + 1]); c.stroke(); };
        const rect = (x, y, w, h) => { c.beginPath(); c.rect(x, y, w, h); c.stroke(); };
        switch (g.name) {
        case "arrow": line([5, 19, 18, 6]); line([10, 6, 18, 6, 18, 14]); break;
        case "box": rect(4, 6, 16, 12); break;
        case "pen": line([5, 19, 6, 15, 16, 5, 19, 8, 9, 18, 5, 19]); line([14, 7, 17, 10]); break;
        case "highlighter":
            line([7, 14, 14, 5, 19, 10, 12, 19]); line([7, 14, 12, 19]);
            c.globalAlpha = 0.45; c.fillRect(3, 19, 18, 3); c.globalAlpha = 1; break;
        case "text": line([6, 6, 18, 6]); line([12, 6, 12, 19]); line([9, 19, 15, 19]); break;
        case "blur":
            for (let i = 0; i < 3; i++) { c.globalAlpha = 1 - i * 0.3; c.beginPath(); c.arc(12, 12, 3 + i * 3.5, 0, Math.PI * 2); c.stroke(); }
            c.globalAlpha = 1; break;
        case "pixelate":
            for (let y = 0; y < 4; y++) for (let x = 0; x < 4; x++) if ((x + y) % 2 === 0) c.fillRect(4 + x * 4, 4 + y * 4, 4, 4);
            rect(4, 4, 16, 16); break;
        case "steps":
            c.beginPath(); c.arc(12, 12, 8, 0, Math.PI * 2); c.fill();
            c.globalCompositeOperation = "destination-out"; line([10.5, 9, 12.5, 7.5, 12.5, 16.5]); c.globalCompositeOperation = "source-over"; break;
        case "crop": line([7, 3, 7, 17, 21, 17]); line([3, 7, 17, 7, 17, 21]); break;
        case "area":
            if (typeof c.setLineDash === "function") c.setLineDash([2.5, 2.5]);
            rect(4, 5, 16, 14);
            if (typeof c.setLineDash === "function") c.setLineDash([]);
            c.fillRect(2.5, 3.5, 3, 3); c.fillRect(18.5, 17.5, 3, 3); break;
        case "window": rect(3, 5, 18, 14); line([3, 9, 21, 9]); c.fillRect(5, 6.4, 1.4, 1.4); c.fillRect(7.6, 6.4, 1.4, 1.4); break;
        case "screen": rect(3, 4, 18, 12); line([9, 20, 15, 20]); line([12, 16, 12, 20]); break;
        case "all": rect(2, 5, 11, 8); rect(11, 9, 11, 8); line([5, 19, 9, 19]); line([14, 20, 19, 20]); break;
        case "qr":
            rect(3, 3, 7, 7); rect(14, 3, 7, 7); rect(3, 14, 7, 7);
            c.fillRect(5.5, 5.5, 2, 2); c.fillRect(16.5, 5.5, 2, 2); c.fillRect(5.5, 16.5, 2, 2);
            c.fillRect(14, 14, 3, 3); c.fillRect(18, 18, 3, 3); c.fillRect(18, 14, 3, 2); c.fillRect(14, 19, 2, 2); break;
        case "scan":
            line([3, 8, 3, 3, 8, 3]); line([16, 3, 21, 3, 21, 8]); line([21, 16, 21, 21, 16, 21]); line([8, 21, 3, 21, 3, 16]);
            line([7, 9, 17, 9]); line([7, 12, 17, 12]); line([7, 15, 13, 15]); break;
        case "record":
            c.beginPath(); c.arc(12, 12, 8.5, 0, Math.PI * 2); c.stroke();
            c.beginPath(); c.arc(12, 12, 4.5, 0, Math.PI * 2); c.fill(); break;
        case "timer":
            c.beginPath(); c.arc(12, 13, 8, 0, Math.PI * 2); c.stroke(); line([12, 13, 12, 9]); line([10, 3, 14, 3]); break;
        case "pointer": c.beginPath(); c.moveTo(6, 3); c.lineTo(18, 14); c.lineTo(12.5, 14.5); c.lineTo(15.5, 20.5); c.lineTo(13, 21.5); c.lineTo(10, 15.5); c.lineTo(6, 19); c.closePath(); c.stroke(); break;
        case "redo": c.beginPath(); c.arc(11, 13, 6, Math.PI * 1.1, Math.PI * 0.1, false); c.stroke(); line([17, 7, 17, 12, 12, 12]); break;
        default: c.beginPath(); c.arc(12, 12, 3, 0, Math.PI * 2); c.fill();
        }
    }
}

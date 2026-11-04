// The new looks' own wallpapers (Mantel 2026), drawn here instead of shipped as pictures: Gaming's neon HUD
// ("anim:neon"), Zen's brush circle ("anim:zen"), Glass's aurora ("anim:glass") and Synthwave's sunset grid
// ("anim:synth"). Used by Wallpaper.qml (full screen), the Look cards in Settings › Appearance (small), and Glass's
// frosted islands (a copy behind each island). Still pictures: nothing moves, so they cost nothing after the first
// paint (Canvas repaints only when the size or the mode changes).
import QtQuick

Item {
    id: root
    property string name: ""          // neon | zen | glass | synth
    readonly property bool dark: Theme.dark
    readonly property var known: ["neon", "zen", "glass", "synth"]
    readonly property bool ok: known.indexOf(name) >= 0
    clip: true

    // ---------------------------------------------------------------- Gaming: near-black, cyan lines, a magenta HUD frame
    Item {
        anchors.fill: parent
        visible: root.name === "neon"
        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop { position: 0; color: root.dark ? "#06080c" : "#e9f0f5" }
                GradientStop { position: 1; color: root.dark ? "#0b0f18" : "#dbe5ee" }
            }
        }
        Canvas {
            id: neon
            anchors.fill: parent
            visible: root.name === "neon"
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            Connections { target: Theme; function onDarkChanged() { neon.requestPaint(); } }
            onPaint: {
                const c = getContext("2d"), w = width, h = height;
                c.reset();
                if (w <= 0 || h <= 0) return;
                // Faint diagonal lines over the lower half.
                c.strokeStyle = root.dark ? "rgba(34,228,255,0.08)" : "rgba(0,152,184,0.10)";
                c.lineWidth = Math.max(1, w / 1920);
                for (let x = -h; x < w; x += w / 12) { c.beginPath(); c.moveTo(x, h); c.lineTo(x + h * 0.48, h * 0.52); c.stroke(); }
                // A magenta glow in the right half, and a cut-corner frame.
                const g = c.createRadialGradient(w * 0.74, h * 0.34, 0, w * 0.74, h * 0.34, h * 0.45);
                g.addColorStop(0, root.dark ? "rgba(255,61,200,0.20)" : "rgba(192,24,154,0.12)");
                g.addColorStop(1, "rgba(0,0,0,0)");
                c.fillStyle = g; c.fillRect(0, 0, w, h);
                const fx = w * 0.615, fy = h * 0.185, fw = w * 0.24, fh = h * 0.31, cut = h * 0.08;
                c.strokeStyle = root.dark ? "rgba(255,61,200,0.75)" : "rgba(192,24,154,0.55)";
                c.lineWidth = Math.max(2, w / 800);
                c.beginPath(); c.moveTo(fx, fy); c.lineTo(fx + fw - cut, fy); c.lineTo(fx + fw, fy + cut); c.lineTo(fx + fw, fy + fh);
                c.lineTo(fx + cut, fy + fh); c.lineTo(fx, fy + fh - cut); c.closePath(); c.stroke();
                c.strokeStyle = root.dark ? "rgba(34,228,255,0.35)" : "rgba(0,152,184,0.35)";
                c.lineWidth = Math.max(1, w / 1600);
                const i = h * 0.04;
                c.beginPath(); c.moveTo(fx + i, fy + i); c.lineTo(fx + fw - cut - i * 0.5, fy + i); c.lineTo(fx + fw - i, fy + cut + i * 0.5); c.lineTo(fx + fw - i, fy + fh - i);
                c.lineTo(fx + cut + i * 0.5, fy + fh - i); c.lineTo(fx + i, fy + fh - cut - i * 0.5); c.closePath(); c.stroke();
            }
        }
        Text {
            x: parent.width * 0.655; y: parent.height * 0.285
            text: "HEARTH"
            color: root.dark ? "#2a2550" : "#c9d6e2"
            font.family: "DejaVu Sans Condensed"; font.bold: true; font.letterSpacing: parent.height * 0.012
            font.pixelSize: parent.height * 0.085
        }
    }

    // ---------------------------------------------------------------- Zen: warm grey, one brush circle
    Item {
        anchors.fill: parent
        visible: root.name === "zen"
        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop { position: 0; color: root.dark ? "#1d1c1a" : "#f1eee8" }
                GradientStop { position: 1; color: root.dark ? "#171614" : "#e6e2da" }
            }
        }
        Canvas {
            id: zen
            anchors.fill: parent
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            Connections { target: Theme; function onDarkChanged() { zen.requestPaint(); } }
            onPaint: {
                const c = getContext("2d"), w = width, h = height;
                c.reset();
                if (w <= 0 || h <= 0) return;
                const cx = w / 2, cy = h * 0.47, r = h * 0.2;
                c.lineCap = "round";
                // An ensō: a stroke that thins as it goes round, with a gap at the top right.
                const steps = 120;
                for (let s = 0; s < steps; s++) {
                    const a0 = -0.9 + (s / steps) * 5.6, a1 = -0.9 + ((s + 1) / steps) * 5.6;
                    const t = s / steps;
                    c.strokeStyle = root.dark ? "rgba(201,216,182," + (0.5 - t * 0.3) + ")" : "rgba(60,58,52," + (0.55 - t * 0.35) + ")";
                    c.lineWidth = r * (0.16 - t * 0.12);
                    c.beginPath(); c.arc(cx, cy, r, a0, a1); c.stroke();
                }
            }
        }
    }

    // ---------------------------------------------------------------- Glass: deep night with a soft aurora
    Item {
        anchors.fill: parent
        visible: root.name === "glass"
        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop { position: 0; color: root.dark ? "#081022" : "#dfe8f7" }
                GradientStop { position: 1; color: root.dark ? "#120d2a" : "#efe6f6" }
            }
        }
        Repeater {
            model: [{ src: "images/blob-sky.png", x: 0.22, y: 0.32, s: 0.9, o: 0.75 }, { src: "images/blob-plum.png", x: 0.68, y: 0.42, s: 1.0, o: 0.8 },
                    { src: "images/blob-blue.png", x: 0.45, y: 0.78, s: 0.8, o: 0.55 }, { src: "images/blob-sky.png", x: 0.86, y: 0.12, s: 0.5, o: 0.4 }]
            delegate: Image {
                required property var modelData
                source: Qt.resolvedUrl(modelData.src)
                width: root.height * modelData.s * 1.3
                height: width
                x: root.width * modelData.x - width / 2
                y: root.height * modelData.y - height / 2
                opacity: modelData.o * (root.dark ? 1 : 0.6)
                smooth: true
            }
        }
        Rectangle {   // a mint band through the sky
            width: parent.width * 1.2; height: parent.height * 0.16
            x: -parent.width * 0.1; y: parent.height * 0.3
            rotation: -8
            opacity: root.dark ? 0.35 : 0.25
            gradient: Gradient {
                orientation: Gradient.Vertical
                GradientStop { position: 0; color: "transparent" }
                GradientStop { position: 0.5; color: "#5cf2c0" }
                GradientStop { position: 1; color: "transparent" }
            }
        }
    }

    // ---------------------------------------------------------------- Synthwave: striped sun over a magenta grid
    Item {
        anchors.fill: parent
        visible: root.name === "synth"
        Canvas {
            id: synth
            anchors.fill: parent
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            Connections { target: Theme; function onDarkChanged() { synth.requestPaint(); } }
            onPaint: {
                const c = getContext("2d"), w = width, h = height;
                c.reset();
                if (w <= 0 || h <= 0) return;
                const hz = h * 0.6;
                let g = c.createLinearGradient(0, 0, 0, hz);
                g.addColorStop(0, root.dark ? "#12062a" : "#ffe3f3"); g.addColorStop(0.65, root.dark ? "#5a1460" : "#ffb3d9"); g.addColorStop(1, root.dark ? "#ff5a8a" : "#ff8fb8");
                c.fillStyle = g; c.fillRect(0, 0, w, hz);
                // Stars.
                c.fillStyle = root.dark ? "rgba(255,240,255,0.7)" : "rgba(255,255,255,0.8)";
                for (let i = 0; i < 70; i++) { const sx = ((i * 0.6180339) % 1) * w, sy = ((i * 0.7548776) % 1) * hz * 0.6; c.fillRect(sx, sy, 2, 2); }
                // The sun, cut by stripes near the bottom.
                const cx = w / 2, cy = hz - h * 0.02, r = h * 0.2;
                g = c.createLinearGradient(0, cy - r, 0, cy + r);
                g.addColorStop(0, "#ffe66d"); g.addColorStop(1, "#ff3fa4");
                c.save(); c.beginPath(); c.arc(cx, cy, r, Math.PI, 0); c.closePath(); c.clip();
                c.fillStyle = g; c.fillRect(cx - r, cy - r, r * 2, r * 2);
                c.fillStyle = root.dark ? "#3a0f4a" : "#ffb3d9";
                for (let k = 0; k < 6; k++) { const y = cy - r * 0.55 + k * r * 0.11; c.fillRect(cx - r, y, r * 2, 2 + k * 1.6); }
                c.restore();
                // Ground and the grid going to the horizon.
                g = c.createLinearGradient(0, hz, 0, h);
                g.addColorStop(0, root.dark ? "#1a0630" : "#f7d6ef"); g.addColorStop(1, root.dark ? "#07020f" : "#f1c5e6");
                c.fillStyle = g; c.fillRect(0, hz, w, h - hz);
                c.strokeStyle = root.dark ? "rgba(255,63,164,0.8)" : "rgba(216,27,132,0.55)";
                c.lineWidth = Math.max(1, w / 1400);
                for (let i = -14; i <= 14; i++) { c.beginPath(); c.moveTo(cx + i * w * 0.01, hz); c.lineTo(cx + i * w * 0.12, h); c.stroke(); }
                for (let j = 1; j < 14; j++) { const t = Math.pow(j / 13, 2.2); const y = hz + (h - hz) * t; c.beginPath(); c.moveTo(0, y); c.lineTo(w, y); c.stroke(); }
                // Scanlines and a soft vignette.
                c.fillStyle = "rgba(0,0,0,0.10)";
                for (let y = 0; y < h; y += 4) c.fillRect(0, y, w, 1);
                g = c.createRadialGradient(cx, h / 2, h * 0.3, cx, h / 2, w * 0.7);
                g.addColorStop(0, "rgba(0,0,0,0)"); g.addColorStop(1, "rgba(0,0,0,0.45)");
                c.fillStyle = g; c.fillRect(0, 0, w, h);
            }
        }
    }
}

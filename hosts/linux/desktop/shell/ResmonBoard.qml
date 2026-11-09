// Resource Monitor › Your PC: the board or device drawn from `hearth-resmon pc`'s view (docs/RESOURCE-MONITOR.md).
// Everything about where things go comes from the helper (parts with x, y, w, h, style, the hub each one is wired to
// and how many lanes); this file only paints it the way the approved mockups (BoardReal / PcView) do:
//   - one Canvas for what stands still (the board, its holes and capacitors, silkscreen, the parts, the wires: one line
//     per lane), painted again only when the drawing or the selection changes,
//   - one Canvas for the moving flow on busy links only, stepped by a timer while the window shows and only when
//     Settings › Accessibility › Reduce motion is off,
//   - lightning bolts on busy links (they hold still with Reduce motion),
//   - an item per part on top for clicks, its label and the selection's glow.
import QtQuick

Item {
    id: bd
    property var view: null                  // {w, h, stage, shapes, caps, silk, hubs, parts}
    property var live: ({})                  // part id → {busy}
    property string selected: ""
    property string cursor: ""               // where the controller's D-pad is (A selects it)
    property bool animate: true
    signal picked(string id)

    implicitWidth: 870
    implicitHeight: 664
    readonly property var parts: view && view.parts ? view.parts : []
    readonly property var byId: { const m = {}; for (const p of parts) m[p.id] = p; return m; }
    readonly property var colors: ({ cpu: "#66c0f4", ram: "#9a7cff", gpu: "#ff9a4d", drive: "#7ccf8a", usb: "#c9d6e2", net: "#f2c14e",
                                     audio: "#e0607e", power: "#f2c14e", fan: "#3fb8c9", misc: "#9fb2c3", chip: "#3fb8c9", input: "#7fe0a4", disp: "#66c0f4" })
    readonly property var lightText: ["cpu", "batt", "ioMetal", "ioUsbc", "pwr", "pciWhite", "ioDvi", "pcieArmor", "pcieGpu", "conn"]
    function colorOf(p) { return colors[p.kind] || "#9fb2c3"; }
    function busy(p) { return !!(live && live[p.id] && live[p.id].busy); }
    function center(p) { return [p.x + p.w / 2, p.y + p.h / 2]; }
    function hubPt(h) {
        if (view && view.hubs && view.hubs[h]) return view.hubs[h];
        const p = byId[h];
        return p ? center(p) : [435, 330];
    }

    // ---- the wires: one line per lane, from the part to what it talks to (an L: across, then up or down)
    readonly property var lines: {
        const out = [];
        for (const p of parts) {
            if (!p.hub || !p.lanes) continue;
            const a = center(p), h = hubPt(p.hub);
            const n = Math.min(p.lanes, 10), sp = p.kind === "power" ? 4.5 : 3, w = p.kind === "power" ? 3 : 1.6;
            const on = selected === p.id || selected === p.hub;
            const b = busy(p);
            for (let k = 0; k < n; k++) {
                const o = (k - (n - 1) / 2) * sp;
                out.push({ pts: [[a[0] + o, a[1] + o], [h[0] + o, a[1] + o], [h[0] + o, h[1] + o]], empty: p.status === "empty", on: on, w: on ? w + 0.6 : w,
                           busy: b, color: colorOf(p), dur: p.kind === "gpu" ? 0.35 : p.kind === "ram" ? 0.5 : p.kind === "power" ? 0.8 : 1.2, delay: k * 0.07 });
            }
        }
        return out;
    }
    readonly property bool anyBusy: { for (const l of lines) if (l.busy) return true; return false; }
    readonly property var bolts: {
        const out = [];
        for (const p of parts) {
            if (!p.hub || !p.lanes) continue;
            const on = selected === p.id || selected === p.hub;
            if (!(busy(p) || (on && p.status !== "empty"))) continue;
            const a = center(p), h = hubPt(p.hub);
            const L1 = Math.abs(h[0] - a[0]), L2 = Math.abs(h[1] - a[1]), half = (L1 + L2) / 2;
            const m = half <= L1 ? [a[0] + Math.sign(h[0] - a[0]) * half, a[1]] : [h[0], a[1] + Math.sign(h[1] - a[1]) * (half - L1)];
            out.push({ x: m[0], y: m[1], color: colorOf(p), delay: out.length * 130 });
        }
        return out;
    }

    onViewChanged: base.requestPaint()
    onSelectedChanged: base.requestPaint()
    onLinesChanged: { base.requestPaint(); flow.requestPaint(); }

    // ---- a small SVG path reader for the helper's outlines (M, L, H, V, Q, C, Z; absolute)
    function tracePath(ctx, d) {
        const t = String(d).match(/[MLHVQCZ]|-?[\d.]+/g) || [];
        let i = 0, x = 0, y = 0, cmd = "";
        const num = () => parseFloat(t[i++]);
        ctx.beginPath();
        while (i < t.length) {
            if (/[MLHVQCZ]/.test(t[i])) cmd = t[i++];
            switch (cmd) {
            case "M": x = num(); y = num(); ctx.moveTo(x, y); cmd = "L"; break;
            case "L": x = num(); y = num(); ctx.lineTo(x, y); break;
            case "H": x = num(); ctx.lineTo(x, y); break;
            case "V": y = num(); ctx.lineTo(x, y); break;
            case "Q": { const cx = num(), cy = num(); x = num(); y = num(); ctx.quadraticCurveTo(cx, cy, x, y); break; }
            case "C": { const a = num(), b = num(), c = num(), e = num(); x = num(); y = num(); ctx.bezierCurveTo(a, b, c, e, x, y); break; }
            case "Z": ctx.closePath(); break;
            default: i++;
            }
        }
    }
    function rr(ctx, x, y, w, h, r) {
        r = Math.min(r, w / 2, h / 2);
        ctx.beginPath();
        ctx.moveTo(x + r, y); ctx.lineTo(x + w - r, y); ctx.arcTo(x + w, y, x + w, y + r, r);
        ctx.lineTo(x + w, y + h - r); ctx.arcTo(x + w, y + h, x + w - r, y + h, r);
        ctx.lineTo(x + r, y + h); ctx.arcTo(x, y + h, x, y + h - r, r);
        ctx.lineTo(x, y + r); ctx.arcTo(x, y, x + r, y, r);
        ctx.closePath();
    }
    function lin(ctx, x0, y0, x1, y1, stops) {
        const g = ctx.createLinearGradient(x0, y0, x1, y1);
        for (const s of stops) g.addColorStop(s[0], s[1]);
        return g;
    }
    // Dashes along a polyline (axis-aligned pieces), starting `phase` px into the pattern.
    function dashes(ctx, pts, on, off, phase) {
        const period = on + off;
        let pos = ((phase % period) + period) % period;
        ctx.beginPath();
        for (let s = 0; s + 1 < pts.length; s++) {
            const ax = pts[s][0], ay = pts[s][1], bx = pts[s + 1][0], by = pts[s + 1][1];
            const len = Math.abs(bx - ax) + Math.abs(by - ay);
            if (len <= 0) continue;
            const dx = (bx - ax) / len, dy = (by - ay) / len;
            let t = 0;
            while (t < len) {
                const inOn = pos < on;
                const step = Math.min(len - t, inOn ? on - pos : period - pos);
                if (inOn) { ctx.moveTo(ax + dx * t, ay + dy * t); ctx.lineTo(ax + dx * (t + step), ay + dy * (t + step)); }
                t += step;
                pos = (pos + step) % period;
            }
        }
        ctx.stroke();
    }
    function roundStyle(s) { return s === "batt" || s === "fan" || s === "stick" || s === "btn"; }

    function paintPart(ctx, p) {
        const x = p.x, y = p.y, w = p.w, h = p.h, s = p.style;
        const r = roundStyle(s) ? Math.min(w, h) / 2 : s === "pack" ? 10 : (String(s).indexOf("dimm") === 0 || s === "ddr3") ? 3 : 4;
        ctx.save();
        ctx.shadowColor = "rgba(0,0,0,0.6)"; ctx.shadowBlur = s === "cpu" ? 8 : 3; ctx.shadowOffsetY = s === "cpu" ? 2 : 1;
        if (roundStyle(s)) { ctx.beginPath(); ctx.ellipse(x, y, w, h); } else rr(ctx, x, y, w, h, r);
        let fill = "#2b2f35";
        const v = (a, b) => lin(ctx, x, y, x, y + h, [[0, a], [1, b]]);
        const hz = (a, b, c, mid) => lin(ctx, x, y, x + w, y, [[0, a], [mid || 0.5, b], [1, c]]);
        const dg = (a, b) => lin(ctx, x, y, x + w, y + h, [[0, a], [1, b]]);
        switch (s) {
        case "ioUsbc": fill = v("#9aa1a8", "#5f666e"); break;
        case "ioMetal": fill = v("#b3b9c0", "#6c737b"); break;
        case "ioVga": fill = v("#2a5bb8", "#1d4189"); break;
        case "ioDvi": fill = v("#f2f3f5", "#c8cbcf"); break;
        case "ioOpt": case "sd": fill = v(s === "sd" ? "#22262b" : "#1b1e22", "#3a3f45"); break;
        case "cpu": fill = "#2b2f35"; break;
        case "apu": case "chip": case "lpddr": fill = s === "apu" ? dg("#3b4048", "#1d2126") : s === "chip" ? dg("#24282e", "#111418") : dg("#2a2d33", "#121418"); break;
        case "ddr3": fill = hz("#1f3d24", "#2f6b3a", "#1f3d24"); break;
        case "dimmFull": fill = hz("#3b3459", "#6b5c9c", "#3b3459"); break;
        case "dimmEmpty": fill = hz("#202328", "#2b2f35", "#202328"); break;
        case "pcieArmor": fill = v("#c7cdd3", "#7f868e"); break;
        case "pciWhite": fill = v("#f0f1f3", "#c7c9cc"); break;
        case "pcie1": fill = "#202328"; break;
        case "wifi": fill = dg("#1f5a35", "#164327"); break;
        case "shroud": fill = hz("#2c3137", "#3a4047", "#2c3137", 0.6); break;
        case "dpad": fill = v("#2b2f35", "#1b1e22"); break;
        case "trig": fill = v("#2e333a", "#16191c"); break;
        case "conn": fill = v("#e6c86a", "#b39443"); break;
        case "batt": { const g = ctx.createRadialGradient(x + w * 0.4, y + h * 0.35, 1, x + w / 2, y + h / 2, w * 0.6); g.addColorStop(0, "#f2f4f6"); g.addColorStop(1, "#9aa1a8"); fill = g; break; }
        case "stick": { const g = ctx.createRadialGradient(x + w / 2, y + h / 2, 0, x + w / 2, y + h / 2, w / 2); g.addColorStop(0, "#3a3f46"); g.addColorStop(0.45, "#3a3f46"); g.addColorStop(0.47, "#1b1e22"); g.addColorStop(0.7, "#1b1e22"); g.addColorStop(0.72, "#0f1114"); g.addColorStop(1, "#0f1114"); fill = g; break; }
        case "btn": { const g = ctx.createRadialGradient(x + w / 2, y + h / 2, 0, x + w / 2, y + h / 2, w / 2); g.addColorStop(0, "#2b2f35"); g.addColorStop(0.6, "#2b2f35"); g.addColorStop(0.62, "#15181b"); g.addColorStop(1, "#15181b"); fill = g; break; }
        case "heatRed": fill = "#4a1818"; break;
        case "heat": case "heatV": fill = "#33383e"; break;
        case "hdr": case "spk": case "ioAudio": case "ioAnt": fill = s === "hdr" || s === "spk" ? "#23272c" : "#2b2f35"; break;
        case "pwr": fill = "#e6e7e9"; break;
        case "pack": fill = "#2c3238"; break;
        case "m2heat": fill = "#444a52"; break;
        case "m2bare": fill = "#1d2126"; break;
        case "ssd": fill = hz("#12301f", "#1b4a2e", "#1b4a2e", 0.7); break;
        case "sata": case "sataUsed": fill = "#2b2f35"; break;
        case "ioUsb3": case "ioUsb2": fill = "#1b1e22"; break;
        case "ioPs2": fill = "#7d3fb5"; break;
        case "fan": fill = "#2a2f35"; break;
        case "pcieGpu": fill = "#c7cdd3"; break;
        }
        ctx.fillStyle = fill;
        ctx.fill();
        ctx.shadowColor = "transparent"; ctx.shadowBlur = 0; ctx.shadowOffsetY = 0;
        // ---- textures inside the shape
        ctx.clip();
        switch (s) {
        case "ioUsb3": case "ioUsb2": ctx.fillStyle = s === "ioUsb3" ? "#2d6cdf" : "#22262b"; ctx.fillRect(x + 3, y + h * 0.3, w - 6, h * 0.3); break;
        case "ioPs2": ctx.fillStyle = "#3d9a4a"; ctx.fillRect(x + w / 2, y, w / 2, h); break;
        case "heat": case "heatRed": ctx.fillStyle = s === "heat" ? "#545b64" : "#8a2a2a"; for (let i = x; i < x + w; i += 7) ctx.fillRect(i, y, 3, h); break;
        case "heatV": ctx.fillStyle = "#545b64"; for (let i = y; i < y + h; i += 7) ctx.fillRect(x, i, w, 3); break;
        case "cpu": {
            const g = ctx.createRadialGradient(x + w * 0.4, y + h * 0.35, 2, x + w / 2, y + h / 2, w * 0.7);
            g.addColorStop(0, "#e3e6ea"); g.addColorStop(0.6, "#a9afb6"); g.addColorStop(1, "#8d939a");
            ctx.fillStyle = g; rr(ctx, x + 9, y + 9, w - 18, h - 18, 3); ctx.fill(); break;
        }
        case "pcieGpu": ctx.fillStyle = "#ff9a4d"; ctx.fillRect(x, y + h * 0.25, w, h * 0.5); break;
        case "m2heat": ctx.strokeStyle = "#5e656e"; ctx.lineWidth = 2; ctx.beginPath(); for (let i = x - h; i < x + w + h; i += 6) { ctx.moveTo(i, y + h); ctx.lineTo(i + h, y); } ctx.stroke(); break;
        case "m2bare": ctx.fillStyle = "#8a8f96"; ctx.fillRect(x + w * 0.92, y, w * 0.04, h); break;
        case "ssd": ctx.fillStyle = "#c9a227"; ctx.fillRect(x + w * 0.7, y, w * 0.04, h); break;
        case "sata": case "sataUsed": ctx.fillStyle = s === "sata" ? "#d2702a" : "#e8833a"; ctx.fillRect(x + w * 0.18, y, w * 0.64, h); break;
        case "hdr": case "spk": case "pwr": {
            const step = s === "hdr" ? 6 : s === "spk" ? 5 : 8, rad = s === "hdr" ? 1.4 : s === "spk" ? 1.2 : 2;
            ctx.fillStyle = s === "hdr" ? "#d1aa3a" : "#0f1114";
            for (let yy = y + step / 2; yy < y + h; yy += step) for (let xx = x + step / 2; xx < x + w; xx += step) { ctx.beginPath(); ctx.arc(xx, yy, rad, 0, Math.PI * 2); ctx.fill(); }
            break;
        }
        case "ioAudio": {
            const cols = ["#66c26a", "#e8c34a", "#5a8ee0", "#e07aa8", "#9aa1a8", "#e0904a"];
            let n = 0;
            for (let yy = y + 9; yy < y + h - 4; yy += 14) for (let xx = x + 11; xx < x + w - 4; xx += 20) { ctx.fillStyle = cols[n++ % cols.length]; ctx.beginPath(); ctx.arc(xx, yy, 4, 0, Math.PI * 2); ctx.fill(); }
            break;
        }
        case "ioAnt": ctx.fillStyle = "#d7b45a"; for (const cx of [x + w * 0.25, x + w * 0.75]) { ctx.beginPath(); ctx.arc(cx, y + h / 2, Math.min(h / 2 - 1, 5), 0, Math.PI * 2); ctx.fill(); } break;
        case "pack": ctx.fillStyle = "#3a414a"; for (let i = x + 2; i < x + w; i += 70) ctx.fillRect(i, y, 66, h); break;
        case "fan": {
            const cx = x + w / 2, cy = y + h / 2, rad = w / 2;
            ctx.fillStyle = "#4a525c";
            for (let a = 0; a < 360; a += 36) { ctx.beginPath(); ctx.moveTo(cx, cy); ctx.arc(cx, cy, rad, a * Math.PI / 180, (a + 18) * Math.PI / 180); ctx.closePath(); ctx.fill(); }
            ctx.fillStyle = "#1b1e22"; ctx.beginPath(); ctx.arc(cx, cy, rad * 0.22, 0, Math.PI * 2); ctx.fill();
            break;
        }
        }
        ctx.restore();
        // ---- the edge: dashed for an empty slot or port
        ctx.save();
        ctx.lineWidth = 1;
        if (p.status === "empty") {
            ctx.strokeStyle = "#59616b";
            const pts = [[x + 0.5, y + 0.5], [x + w - 0.5, y + 0.5], [x + w - 0.5, y + h - 0.5], [x + 0.5, y + h - 0.5], [x + 0.5, y + 0.5]];
            if (roundStyle(s)) { ctx.beginPath(); ctx.ellipse(x + 0.5, y + 0.5, w - 1, h - 1); ctx.stroke(); }
            else dashes(ctx, pts, 3, 3, 0);
        } else {
            ctx.strokeStyle = "#0d0f12";
            if (roundStyle(s)) { ctx.beginPath(); ctx.ellipse(x + 0.5, y + 0.5, w - 1, h - 1); } else rr(ctx, x + 0.5, y + 0.5, w - 1, h - 1, r);
            ctx.stroke();
        }
        ctx.restore();
    }

    Canvas {
        id: base
        anchors.fill: parent
        renderStrategy: Canvas.Cooperative
        onPaint: {
            const ctx = getContext("2d");
            ctx.reset();
            const v = bd.view;
            if (!v) return;
            const board = v.stage === "board";
            // the stage
            ctx.fillStyle = board ? "#15181c" : (v.stage === "dark" ? "#0f1216" : "#101317");
            ctx.fillRect(0, 0, width, height);
            for (const sh of v.shapes || []) {
                bd.tracePath(ctx, sh.d);
                if (sh.fill === "board") {
                    const g = ctx.createRadialGradient(width * 0.4, height * 0.3, 10, width * 0.4, height * 0.3, width * 0.8);
                    g.addColorStop(0, "#22262c"); g.addColorStop(1, "#16191d");
                    ctx.fillStyle = g; ctx.fill();
                    ctx.save(); ctx.clip();
                    ctx.strokeStyle = "rgba(255,255,255,0.018)"; ctx.lineWidth = 1; ctx.beginPath();
                    for (let i = 0; i < width; i += 24) { ctx.moveTo(i + 0.5, 0); ctx.lineTo(i + 0.5, height); }
                    for (let i = 0; i < height; i += 24) { ctx.moveTo(0, i + 0.5); ctx.lineTo(width, i + 0.5); }
                    ctx.stroke(); ctx.restore();
                    bd.tracePath(ctx, sh.d);
                    ctx.strokeStyle = "#2a2f36"; ctx.lineWidth = 2; ctx.stroke();
                } else {
                    if (sh.fill && sh.fill !== "none") { ctx.fillStyle = sh.fill; ctx.fill(); }
                    if (sh.stroke) { ctx.strokeStyle = sh.stroke; ctx.lineWidth = sh.sw || 1; ctx.stroke(); }
                }
            }
            if (board) {   // the I/O shield's outline on the board
                ctx.strokeStyle = "#3a3f46"; ctx.lineWidth = 1.5;
                bd.dashes(ctx, [[8, (v.h || 664) - 164], [186, (v.h || 664) - 164], [186, (v.h || 664) - 8]], 3, 3, 0);
            }
            for (const c of v.caps || []) {
                ctx.beginPath(); ctx.arc(c.x, c.y, c.r, 0, Math.PI * 2);
                ctx.fillStyle = c.fill; ctx.fill();
                ctx.strokeStyle = c.fill === "#0d0f12" ? "#8a8f96" : "#0d0f12"; ctx.lineWidth = c.fill === "#0d0f12" ? 2 : 1; ctx.stroke();
            }
            // the wires (under the parts)
            ctx.lineCap = "round"; ctx.lineJoin = "round";
            for (const l of bd.lines) {
                ctx.globalAlpha = l.empty ? (l.on ? 0.6 : 0.25) : (l.on ? 0.7 : 0.35);
                ctx.strokeStyle = l.empty ? "#3a4048" : "#8a6d2e";
                ctx.lineWidth = l.w;
                ctx.beginPath(); ctx.moveTo(l.pts[0][0], l.pts[0][1]);
                for (let i = 1; i < l.pts.length; i++) ctx.lineTo(l.pts[i][0], l.pts[i][1]);
                ctx.stroke();
            }
            ctx.globalAlpha = 1;
            ctx.font = "8px 'JetBrains Mono', 'DejaVu Sans Mono', monospace";
            ctx.fillStyle = "rgba(200,204,209,0.55)";
            for (const s of v.silk || []) ctx.fillText(s.t, s.x, s.y);
            for (const p of bd.parts) bd.paintPart(ctx, p);
        }
    }

    // The flow on busy links: dashes that move toward what the part talks to.
    property real clock: 0
    Canvas {
        id: flow
        anchors.fill: parent
        visible: bd.anyBusy
        renderStrategy: Canvas.Cooperative
        onPaint: {
            const ctx = getContext("2d");
            ctx.reset();
            ctx.lineCap = "round";
            for (const l of bd.lines) {
                if (!l.busy) continue;
                ctx.globalAlpha = l.on ? 1 : 0.55;
                ctx.strokeStyle = l.color;
                ctx.lineWidth = l.w;
                bd.dashes(ctx, l.pts, 4, 9, -((bd.clock + l.delay) * 26 / l.dur));
            }
        }
    }
    Timer {
        interval: 50
        repeat: true
        running: bd.animate && bd.anyBusy && bd.visible
        onTriggered: { bd.clock += 0.05; flow.requestPaint(); }
    }

    // ---- the parts: clicks, labels, the selection's glow
    Repeater {
        model: bd.parts
        delegate: Item {
            required property var modelData
            readonly property var p: modelData
            readonly property bool sel: bd.selected === p.id
            readonly property bool cur: bd.cursor === p.id && !sel
            readonly property color col: bd.colorOf(p)
            x: p.x; y: p.y; width: p.w; height: p.h
            z: sel ? 3 : 1
            Rectangle {   // the glow
                visible: parent.sel
                anchors.fill: parent; anchors.margins: -5
                radius: bd.roundStyle(p.style) ? width / 2 : 7
                color: "transparent"
                border.width: 3; border.color: Qt.rgba(col.r, col.g, col.b, 0.4)
            }
            Rectangle {
                visible: parent.sel || parent.cur
                anchors.fill: parent; anchors.margins: -1
                radius: bd.roundStyle(p.style) ? width / 2 : 4
                color: "transparent"
                border.width: 2; border.color: parent.sel ? col : "#66c0f4"
                opacity: parent.sel ? 1 : 0.8
            }
            Text {
                anchors.centerIn: parent
                visible: p.text !== ""
                text: p.text
                rotation: p.vertical ? 90 : 0
                width: p.vertical ? parent.height - 4 : parent.width - 4
                horizontalAlignment: Text.AlignHCenter
                elide: Text.ElideRight
                color: bd.lightText.indexOf(p.style) >= 0 ? "#14171a" : "#e6eef5"
                font.family: Theme.font
                font.pixelSize: p.id === "cpu" ? 13 : (p.h < 22 || p.w < 46) && !p.vertical ? 8.5 : 10.5
                font.weight: Font.DemiBold
                style: bd.lightText.indexOf(p.style) >= 0 ? Text.Normal : Text.Raised
                styleColor: "#80000000"
            }
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: bd.picked(p.id)
            }
        }
    }
    Repeater {
        model: bd.bolts
        delegate: Item {
            required property var modelData
            x: modelData.x - 9; y: modelData.y - 9; width: 18; height: 18
            z: 4
            Rectangle { anchors.fill: parent; radius: 9; color: Qt.rgba(12 / 255, 14 / 255, 17 / 255, 0.85) }
            Canvas {
                anchors.centerIn: parent; width: 12; height: 12
                property color c: modelData.color
                onPaint: {
                    const ctx = getContext("2d"); ctx.reset(); ctx.scale(0.5, 0.5);
                    ctx.beginPath(); ctx.moveTo(13, 2); ctx.lineTo(4, 14); ctx.lineTo(11, 14); ctx.lineTo(10, 22); ctx.lineTo(19, 10); ctx.lineTo(12, 10); ctx.closePath();
                    ctx.fillStyle = c; ctx.fill();
                }
            }
            SequentialAnimation on opacity {
                running: bd.animate && bd.visible
                loops: Animation.Infinite
                PauseAnimation { duration: modelData.delay % 1100 }
                NumberAnimation { from: 0.4; to: 1; duration: 550; easing.type: Easing.InOutQuad }
                NumberAnimation { from: 1; to: 0.4; duration: 550; easing.type: Easing.InOutQuad }
            }
        }
    }
}

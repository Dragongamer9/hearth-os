// Hearth Capture's markup (CaptureEditor.qml) and area picker (Capture.qml): the shapes and how they're drawn, the
// area maths, pixelate and blur. Shapes are kept in the picture's own pixels, so the same list draws the editor's
// view (scaled to fit) and the saved picture (1:1). No Qt here, so node can check it (test_hearth_capture.py).
//   shape: { t: "arrow" | "box" | "pen" | "hl" | "text" | "blur" | "pix" | "step", c: "#rrggbb", w: width,
//            x1, y1, x2, y2 (arrow, box, blur, pix), pts: [x, y, x, y...] (pen, hl), x1, y1, text, size (text),
//            x1, y1, n (step) }
.pragma library

var WIDTHS = { s: 3, m: 6, l: 10 };
var COLORS = ["#ff4d4d", "#ff8a3d", "#ffd166", "#4cd97b", "#4aa8ff", "#ffffff", "#111111"];
var TOOLS = ["arrow", "box", "pen", "hl", "text", "blur", "pix", "step", "crop"];

function normRect(x1, y1, x2, y2) {
    return { x: Math.min(x1, x2), y: Math.min(y1, y2), w: Math.abs(x2 - x1), h: Math.abs(y2 - y1) };
}

// A rectangle inside 0..W, 0..H, whole pixels, at least 1×1.
function clampRect(r, W, H) {
    var x = Math.max(0, Math.min(W - 1, Math.round(r.x))), y = Math.max(0, Math.min(H - 1, Math.round(r.y)));
    var w = Math.max(1, Math.min(W - x, Math.round(r.w))), h = Math.max(1, Math.min(H - y, Math.round(r.h)));
    return { x: x, y: y, w: w, h: h };
}

// The scale that fits W×H into an area (never more than 2× for a small crop).
function fit(areaW, areaH, w, h) {
    if (w <= 0 || h <= 0 || areaW <= 0 || areaH <= 0) return 1;
    return Math.min(areaW / w, areaH / h, 2);
}

// The area picker works in the screen's own (logical) size; the picture of that screen can be bigger (a scaled
// screen: 2560 pixels for 1280 logical). The crop in the picture's pixels.
function pickerToCrop(sel, screenW, screenH, imgW, imgH) {
    var kx = screenW > 0 ? imgW / screenW : 1, ky = screenH > 0 ? imgH / screenH : 1;
    return clampRect({ x: sel.x * kx, y: sel.y * ky, w: sel.w * kx, h: sel.h * ky }, imgW, imgH);
}

// The picker's box moved (D-pad / arrows) or sized, kept on the screen.
function moveSel(sel, dx, dy, W, H) {
    return { x: Math.max(0, Math.min(W - sel.w, sel.x + dx)), y: Math.max(0, Math.min(H - sel.h, sel.y + dy)), w: sel.w, h: sel.h };
}
function resizeSel(sel, dw, dh, W, H, min) {
    var m = min || 24;
    return { x: sel.x, y: sel.y, w: Math.max(m, Math.min(W - sel.x, sel.w + dw)), h: Math.max(m, Math.min(H - sel.y, sel.h + dh)) };
}
// The box a controller starts with: the middle half of the screen.
function startSel(W, H) { return { x: Math.round(W / 4), y: Math.round(H / 4), w: Math.round(W / 2), h: Math.round(H / 2) }; }

function nextStep(shapes) {
    var n = 0;
    for (var i = 0; i < shapes.length; i++) if (shapes[i].t === "step" && shapes[i].n > n) n = shapes[i].n;
    return n + 1;
}

// The two back corners of an arrow's head at (x2, y2).
function arrowHead(x1, y1, x2, y2, len) {
    var a = Math.atan2(y2 - y1, x2 - x1), s = Math.PI / 7;
    return [x2 - len * Math.cos(a - s), y2 - len * Math.sin(a - s), x2 - len * Math.cos(a + s), y2 - len * Math.sin(a + s)];
}

// Pixelate DATA (RGBA, W×H) in place: each BLOCK×BLOCK square becomes its average colour.
function pixelate(data, w, h, block) {
    var b = Math.max(2, Math.round(block));
    for (var by = 0; by < h; by += b) {
        for (var bx = 0; bx < w; bx += b) {
            var r = 0, g = 0, bl = 0, a = 0, n = 0, ex = Math.min(w, bx + b), ey = Math.min(h, by + b), x, y, i;
            for (y = by; y < ey; y++) for (x = bx; x < ex; x++) { i = (y * w + x) * 4; r += data[i]; g += data[i + 1]; bl += data[i + 2]; a += data[i + 3]; n++; }
            r = Math.round(r / n); g = Math.round(g / n); bl = Math.round(bl / n); a = Math.round(a / n);
            for (y = by; y < ey; y++) for (x = bx; x < ex; x++) { i = (y * w + x) * 4; data[i] = r; data[i + 1] = g; data[i + 2] = bl; data[i + 3] = a; }
        }
    }
    return data;
}

// Blur: pixelate to RADIUS, then one box blur across and down (fast enough in QML's JavaScript for a big area, and
// nothing readable is left: the small squares are averaged before they're smoothed).
function blur(data, w, h, radius) {
    var rad = Math.max(1, Math.round(radius));
    pixelate(data, w, h, rad);
    var tmp = [], x, y, c, i, sum, n, k;
    for (y = 0; y < h; y++) {
        for (c = 0; c < 4; c++) {
            sum = 0; n = 0;
            for (k = 0; k <= Math.min(rad, w - 1); k++) { sum += data[(y * w + k) * 4 + c]; n++; }
            for (x = 0; x < w; x++) {
                tmp[(y * w + x) * 4 + c] = sum / n;
                if (x + rad + 1 < w) { sum += data[(y * w + x + rad + 1) * 4 + c]; n++; }
                if (x - rad >= 0) { sum -= data[(y * w + x - rad) * 4 + c]; n--; }
            }
        }
    }
    for (x = 0; x < w; x++) {
        for (c = 0; c < 4; c++) {
            sum = 0; n = 0;
            for (k = 0; k <= Math.min(rad, h - 1); k++) { sum += tmp[(k * w + x) * 4 + c]; n++; }
            for (y = 0; y < h; y++) {
                i = (y * w + x) * 4 + c;
                data[i] = Math.round(sum / n);
                if (y + rad + 1 < h) { sum += tmp[((y + rad + 1) * w + x) * 4 + c]; n++; }
                if (y - rad >= 0) { sum -= tmp[((y - rad) * w + x) * 4 + c]; n--; }
            }
        }
    }
    return data;
}

// Draws SHAPES on CTX (a Canvas 2D context) at scale S, with the picture's pixel (OX, OY) at the canvas's top left.
// DRAWIMAGE(sx, sy, sw, sh, dx, dy, dw, dh) draws part of the picture (for blur and pixelate, which need its pixels);
// CW, CH: the canvas's size, so a blurred area is cut to it.
function paint(ctx, shapes, s, ox, oy, drawImage, cw, ch) {
    var X = function (v) { return (v - ox) * s; }, Y = function (v) { return (v - oy) * s; };
    for (var i = 0; i < shapes.length; i++) {
        var sh = shapes[i], lw = Math.max(1, (sh.w || 6) * s);
        ctx.save();
        ctx.strokeStyle = sh.c || "#ff4d4d"; ctx.fillStyle = sh.c || "#ff4d4d";
        ctx.lineWidth = lw; ctx.lineCap = "round"; ctx.lineJoin = "round";
        if (sh.t === "arrow") {
            var hd = arrowHead(X(sh.x1), Y(sh.y1), X(sh.x2), Y(sh.y2), Math.max(10, lw * 3.2));
            var bx = (hd[0] + hd[2]) / 2, by = (hd[1] + hd[3]) / 2;
            ctx.beginPath(); ctx.moveTo(X(sh.x1), Y(sh.y1)); ctx.lineTo(bx, by); ctx.stroke();
            ctx.beginPath(); ctx.moveTo(X(sh.x2), Y(sh.y2)); ctx.lineTo(hd[0], hd[1]); ctx.lineTo(hd[2], hd[3]); ctx.closePath(); ctx.fill();
        } else if (sh.t === "box") {
            var r = normRect(X(sh.x1), Y(sh.y1), X(sh.x2), Y(sh.y2));
            ctx.beginPath(); ctx.rect(r.x, r.y, r.w, r.h); ctx.stroke();
        } else if (sh.t === "pen" || sh.t === "hl") {
            if (sh.t === "hl") { ctx.globalAlpha = 0.38; ctx.lineWidth = lw * 3.5; ctx.lineCap = "square"; }
            var p = sh.pts || [];
            if (p.length >= 2) {
                ctx.beginPath(); ctx.moveTo(X(p[0]), Y(p[1]));
                for (var j = 2; j < p.length; j += 2) ctx.lineTo(X(p[j]), Y(p[j + 1]));
                if (p.length === 2) ctx.lineTo(X(p[0]) + 0.1, Y(p[1]));
                ctx.stroke();
            }
        } else if (sh.t === "text") {
            var size = Math.max(6, (sh.size || 28) * s);
            ctx.font = "600 " + Math.round(size) + "px sans-serif";
            ctx.textBaseline = "top";
            ctx.lineWidth = Math.max(2, size / 7);
            ctx.strokeStyle = (sh.c === "#111111") ? "#ffffff" : "rgba(0,0,0,0.65)";
            var lines = String(sh.text || "").split("\n");
            for (var L = 0; L < lines.length; L++) {
                ctx.strokeText(lines[L], X(sh.x1), Y(sh.y1) + L * size * 1.2);
                ctx.fillText(lines[L], X(sh.x1), Y(sh.y1) + L * size * 1.2);
            }
        } else if (sh.t === "step") {
            var rad = Math.max(8, 16 * s * ((sh.w || 6) / 6 + 0.6) / 1.6);
            ctx.beginPath(); ctx.arc(X(sh.x1), Y(sh.y1), rad, 0, Math.PI * 2); ctx.fill();
            ctx.lineWidth = Math.max(1.5, rad / 7); ctx.strokeStyle = "#ffffff"; ctx.stroke();
            ctx.fillStyle = sh.c === "#ffffff" || sh.c === "#ffd166" ? "#111111" : "#ffffff";
            ctx.font = "700 " + Math.round(rad * 1.15) + "px sans-serif";
            ctx.textAlign = "center"; ctx.textBaseline = "middle";
            ctx.fillText(String(sh.n), X(sh.x1), Y(sh.y1) + rad * 0.05);
        } else if ((sh.t === "blur" || sh.t === "pix") && drawImage) {
            var q = normRect(X(sh.x1), Y(sh.y1), X(sh.x2), Y(sh.y2));
            var dx = Math.max(0, Math.floor(q.x)), dy = Math.max(0, Math.floor(q.y));
            var dw = Math.min(cw || 1e9, Math.ceil(q.x + q.w)) - dx, dh = Math.min(ch || 1e9, Math.ceil(q.y + q.h)) - dy;
            if (dw >= 2 && dh >= 2) {
                drawImage(ox + dx / s, oy + dy / s, dw / s, dh / s, dx, dy, dw, dh);
                var d = ctx.getImageData(dx, dy, dw, dh);
                if (sh.t === "pix") pixelate(d.data, dw, dh, Math.max(4, 14 * s));
                else blur(d.data, dw, dh, Math.max(3, 9 * s));
                ctx.putImageData(d, dx, dy);
            }
        }
        ctx.restore();
    }
}

// A shape's box in picture pixels (for the crop's "keep what you drew" check and tests).
function bounds(sh) {
    if (sh.pts && sh.pts.length) {
        var x0 = 1e9, y0 = 1e9, x1 = -1e9, y1 = -1e9;
        for (var i = 0; i < sh.pts.length; i += 2) { x0 = Math.min(x0, sh.pts[i]); x1 = Math.max(x1, sh.pts[i]); y0 = Math.min(y0, sh.pts[i + 1]); y1 = Math.max(y1, sh.pts[i + 1]); }
        return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    }
    if (sh.x2 !== undefined) return normRect(sh.x1, sh.y1, sh.x2, sh.y2);
    return { x: sh.x1, y: sh.y1, w: 0, h: 0 };
}

// A shape worth keeping (a click without a drag makes no arrow or box).
function worthKeeping(sh) {
    if (sh.t === "text") return String(sh.text || "").trim() !== "";
    if (sh.t === "step") return true;
    if (sh.t === "pen" || sh.t === "hl") return (sh.pts || []).length >= 2;
    var b = bounds(sh);
    return sh.t === "arrow" ? Math.abs(sh.x2 - sh.x1) + Math.abs(sh.y2 - sh.y1) >= 6 : b.w >= 4 && b.h >= 4;
}

// "Screenshot 2026-11-16 21-04-05.png", the name the helper and open.sh give a new picture.
function defaultName(d) {
    var p = function (n) { return (n < 10 ? "0" : "") + n; };
    return "Screenshot " + d.getFullYear() + "-" + p(d.getMonth() + 1) + "-" + p(d.getDate()) + " " + p(d.getHours()) + "-" + p(d.getMinutes()) + "-" + p(d.getSeconds()) + ".png";
}

// "Just now", "5 min ago", "Today 21:04", "Nov 12" for the recent list.
function when(t, now) {
    var s = Math.max(0, Math.round(now / 1000 - t));
    if (s < 60) return "Just now";
    if (s < 3600) return Math.floor(s / 60) + " min ago";
    var d = new Date(t * 1000), n = new Date(now);
    var hm = (d.getHours() < 10 ? "0" : "") + d.getHours() + ":" + (d.getMinutes() < 10 ? "0" : "") + d.getMinutes();
    if (d.toDateString() === n.toDateString()) return "Today " + hm;
    var m = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    return m[d.getMonth()] + " " + d.getDate();
}

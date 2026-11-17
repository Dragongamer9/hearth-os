.pragma library
// Search plugin: colours (like KRunner's colour runner). "#ff8800", "#f80", "rgb(255, 136, 0)", "hsl(32 100% 50%)":
// the colour as a swatch (the row's "swatch", drawn by the menus) and as hex, rgb and hsl. Enter copies the first.

function clamp(v, a, b) { return Math.max(a, Math.min(b, v)); }
function h2(n) { var s = Math.round(n).toString(16); return s.length < 2 ? "0" + s : s; }

function parse(qRaw) {
    var q = String(qRaw || "").trim().toLowerCase(), m;
    if ((m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/.exec(q)) && q[0] === "#") {
        var x = m[1].length === 3 ? m[1].replace(/(.)/g, "$1$1") : m[1];
        return [parseInt(x.slice(0, 2), 16), parseInt(x.slice(2, 4), 16), parseInt(x.slice(4, 6), 16)];
    }
    if ((m = /^rgba?\(\s*(\d{1,3})\s*[, ]\s*(\d{1,3})\s*[, ]\s*(\d{1,3})\s*(?:[,/]\s*[\d.]+%?\s*)?\)$/.exec(q))) {
        var r = +m[1], g = +m[2], b = +m[3];
        return r > 255 || g > 255 || b > 255 ? null : [r, g, b];
    }
    if ((m = /^hsla?\(\s*([\d.]+)(?:deg)?\s*[, ]\s*([\d.]+)%\s*[, ]\s*([\d.]+)%\s*(?:[,/]\s*[\d.]+%?\s*)?\)$/.exec(q))) {
        var hh = (+m[1] % 360) / 360, s = clamp(+m[2], 0, 100) / 100, l = clamp(+m[3], 0, 100) / 100;
        if (s === 0) return [l * 255, l * 255, l * 255].map(Math.round);
        var t = l < 0.5 ? l * (1 + s) : l + s - l * s, p = 2 * l - t;
        function f(x) { x = (x + 1) % 1; return x < 1 / 6 ? p + (t - p) * 6 * x : x < 0.5 ? t : x < 2 / 3 ? p + (t - p) * (2 / 3 - x) * 6 : p; }
        return [f(hh + 1 / 3), f(hh), f(hh - 1 / 3)].map(function (v) { return Math.round(v * 255); });
    }
    return null;
}

function hsl(c) {
    var r = c[0] / 255, g = c[1] / 255, b = c[2] / 255, mx = Math.max(r, g, b), mn = Math.min(r, g, b), l = (mx + mn) / 2, h = 0, s = 0;
    if (mx !== mn) {
        var d = mx - mn;
        s = l > 0.5 ? d / (2 - mx - mn) : d / (mx + mn);
        h = mx === r ? (g - b) / d + (g < b ? 6 : 0) : mx === g ? (b - r) / d + 2 : (r - g) / d + 4;
        h *= 60;
    }
    return "hsl(" + Math.round(h) + ", " + Math.round(s * 100) + "%, " + Math.round(l * 100) + "%)";
}

function results(qRaw) {
    var c = parse(qRaw);
    if (!c) return [];
    var hex = "#" + h2(c[0]) + h2(c[1]) + h2(c[2]), rgb = "rgb(" + c[0] + ", " + c[1] + ", " + c[2] + ")", hs = hsl(c);
    var forms = [[hex, "Hex"], [rgb, "RGB"], [hs, "HSL"]], out = [];
    for (var i = 0; i < forms.length; i++)
        out.push({ plugin: "colours", group: "Colour", title: forms[i][0], sub: forms[i][1] + " · Enter copies it", icon: "palette", swatch: hex,
                   score: 1860 - i, top: i === 0, action: { type: "copy", value: forms[i][0] } });
    return out;
}

// "From wallpaper" (Settings › Appearance › Accent colour): the wallpaper's main colours, worked out ONCE when you
// press From wallpaper or pick a wallpaper with it on, never by itself. The picture is shrunk to 32×32 in a Canvas
// and its pixels grouped (a small k-means); the most colourful, well-lit group becomes the accent. The moving and
// drawn wallpapers have a fixed set each. The result goes through Settings (SettingsWriter: one merged key each),
// and theme/apply.sh tints window frames and apps from it, like any accent you choose.
//   wa.compute(wall)   → found(swatches)  ["#rrggbb", ...] (the first is the accent), or failed()
import QtQuick

Item {
    id: wa
    signal found(var swatches)
    signal failed()
    width: 32; height: 32
    opacity: 0.01
    property string url: ""
    readonly property var fixed: ({
        "": ["#ff8a3d", "#ffd166", "#c2410c", "#7c2d12"],
        "anim:lava": ["#ff5a36", "#ffb347", "#b3264a", "#5a1830"], "anim:aurora": ["#ff9d5c", "#ffd9a0", "#8a3a2c", "#2a1410"],
        "anim:tide": ["#1a9fff", "#66c0f4", "#2a475e", "#171a21"], "anim:ridges": ["#4f9fe0", "#8fc4ef", "#2a475e", "#1b2838"],
        "anim:motes": ["#66c0f4", "#1a9fff", "#32404f", "#171a21"],
        "anim:neon": ["#22e4ff", "#ff3dc8", "#2a2550", "#07090d"], "anim:zen": ["#8fb39a", "#c9d8b6", "#77736a", "#1b1a18"],
        "anim:glass": ["#5cf2c0", "#7aa8ff", "#b07cff", "#0a1020"], "anim:synth": ["#ff3fa4", "#29e3ff", "#ffe66d", "#1a0a2e"]
    })
    function compute(wall) {
        if (fixed[wall] !== undefined) { found(fixed[wall]); return; }
        const u = wall.startsWith("builtin:") ? Qt.resolvedUrl("images/wallpapers/" + wall.slice(8) + ".jpg").toString()
            : wall.startsWith("/") ? "file://" + wall.split("/").map(encodeURIComponent).join("/") : "";
        if (!u) { failed(); return; }
        if (url === u && canvas.isImageLoaded(u)) { canvas.requestPaint(); return; }
        url = u;
        if (canvas.isImageLoaded(u)) canvas.requestPaint(); else canvas.loadImage(u);
        giveUp.restart();
    }
    Timer { id: giveUp; interval: 8000; onTriggered: { if (wa.url !== "") { wa.url = ""; wa.failed(); } } }

    function hsl(r, g, b) {
        const mx = Math.max(r, g, b), mn = Math.min(r, g, b), l = (mx + mn) / 2;
        if (mx === mn) return [0, 0, l];
        const d = mx - mn, s = l > 0.5 ? d / (2 - mx - mn) : d / (mx + mn);
        const h = mx === r ? ((g - b) / d + (g < b ? 6 : 0)) : mx === g ? (b - r) / d + 2 : (r - g) / d + 4;
        return [h / 6, s, l];
    }
    function hex(c) { const t = x => ("0" + Math.max(0, Math.min(255, Math.round(x * 255))).toString(16)).slice(-2); return "#" + t(c.r) + t(c.g) + t(c.b); }
    function pick(px) {
        // k-means with 5 groups, started from pixels spread over the picture.
        const k = 5, n = px.length;
        if (n < k) return [];
        let cs = [];
        for (let i = 0; i < k; i++) cs.push(px[Math.floor((i + 0.5) * n / k)].slice());
        let groups = [];
        for (let it = 0; it < 8; it++) {
            groups = cs.map(() => ({ s: [0, 0, 0], n: 0 }));
            for (const p of px) {
                let best = 0, bd = 1e9;
                for (let j = 0; j < k; j++) { const a0 = p[0] - cs[j][0], a1 = p[1] - cs[j][1], a2 = p[2] - cs[j][2], d = a0 * a0 + a1 * a1 + a2 * a2; if (d < bd) { bd = d; best = j; } }
                const g = groups[best]; g.s[0] += p[0]; g.s[1] += p[1]; g.s[2] += p[2]; g.n++;
            }
            cs = groups.map((g, j) => g.n ? [g.s[0] / g.n, g.s[1] / g.n, g.s[2] / g.n] : cs[j]);
        }
        const scored = cs.map((c, j) => {
            const h = hsl(c[0], c[1], c[2]);
            const lit = h[2] > 0.22 && h[2] < 0.82 ? 1 : 0.25;
            return { c: c, h: h, score: (groups[j].n / n + 0.08) * (0.15 + h[1]) * lit };
        }).sort((a, b) => b.score - a.score);
        // The accent: the best group, made colourful and bright enough to read as an accent.
        const top = scored[0].h;
        const accent = Qt.hsla(top[0], Math.max(0.55, top[1]), Math.min(0.62, Math.max(0.5, top[2])), 1);
        return [hex(accent)].concat(scored.slice(1, 4).map(x => hex(Qt.rgba(x.c[0], x.c[1], x.c[2], 1))));
    }
    Canvas {
        id: canvas
        width: 32; height: 32
        onImageLoaded: requestPaint()
        onPaint: {
            if (wa.url === "" || !isImageLoaded(wa.url)) return;
            const c = getContext("2d");
            c.reset();
            c.drawImage(wa.url, 0, 0, 32, 32);
            const d = c.getImageData(0, 0, 32, 32).data;
            const px = [];
            for (let i = 0; i + 3 < d.length; i += 4) px.push([d[i] / 255, d[i + 1] / 255, d[i + 2] / 255]);
            const sw = wa.pick(px);
            wa.url = "";
            giveUp.stop();
            if (sw.length) wa.found(sw); else wa.failed();
        }
    }
}

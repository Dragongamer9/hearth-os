// The rules behind 1.14's second round of widgets and the bar (no Qt here, so test_notify_widgets_114.py runs them
// with node): the calculator's sums, the fuzzy and binary clocks, colours, and which of the bar's own items go
// where (Bar items: BarArrange.qml, Islands.qml) and what an extra panel holds (ExtraPanel.qml).
.pragma library

// ---------------------------------------------------------------- calculator
// A small safe calculator: numbers, + − × ÷ % ^, brackets, and sqrt sin cos tan asin acos atan ln log abs round
// floor ceil, pi and e. Never eval. Returns {value} or {error}.
function calc(text, degrees) {
    var s = String(text || "").replace(/×/g, "*").replace(/÷/g, "/").replace(/−/g, "-").replace(/,/g, ".").replace(/π/g, "pi");
    var i = 0;
    function peek() { while (s[i] === " ") i++; return s[i]; }
    function fail(m) { throw new Error(m); }
    var FN = {
        sqrt: Math.sqrt, abs: Math.abs, round: Math.round, floor: Math.floor, ceil: Math.ceil, ln: Math.log,
        log: function (x) { return Math.log(x) / Math.LN10; },
        sin: function (x) { return Math.sin(degrees ? x * Math.PI / 180 : x); },
        cos: function (x) { return Math.cos(degrees ? x * Math.PI / 180 : x); },
        tan: function (x) { return Math.tan(degrees ? x * Math.PI / 180 : x); },
        asin: function (x) { var r = Math.asin(x); return degrees ? r * 180 / Math.PI : r; },
        acos: function (x) { var r = Math.acos(x); return degrees ? r * 180 / Math.PI : r; },
        atan: function (x) { var r = Math.atan(x); return degrees ? r * 180 / Math.PI : r; }
    };
    function primary() {
        var c = peek();
        if (c === "(") { i++; var v = expr(); if (peek() !== ")") fail("A bracket isn't closed"); i++; return v; }
        if (c === "-") { i++; return -power(); }
        if (c === "+") { i++; return power(); }
        var m = /^(\d+\.?\d*(e[+-]?\d+)?|\.\d+)/i.exec(s.slice(i));
        if (m) { i += m[0].length; return parseFloat(m[0]); }
        m = /^[a-z]+/i.exec(s.slice(i));
        if (m) {
            var w = m[0].toLowerCase();
            i += m[0].length;
            if (w === "pi") return Math.PI;
            if (w === "e") return Math.E;
            if (FN[w]) { if (peek() !== "(") fail(w + " needs brackets: " + w + "(…)"); i++; var a = expr(); if (peek() !== ")") fail("A bracket isn't closed"); i++; return FN[w](a); }
            fail("I don't know " + w);
        }
        fail(c === undefined ? "Something's missing at the end" : "I can't read " + c);
    }
    function postfix() { var v = primary(); while (peek() === "%") { i++; v = v / 100; } return v; }
    function power() { var b = postfix(); if (peek() === "^") { i++; return Math.pow(b, power()); } return b; }
    function term() {
        var v = power();
        for (;;) {
            var c = peek();
            if (c === "*") { i++; v *= power(); }
            else if (c === "/") { i++; var d = power(); if (d === 0) fail("Can't divide by zero"); v /= d; }
            else if (c === "(" || (c && /[a-z]/i.test(c))) v *= power();     // 2(3+4), 2pi
            else return v;
        }
    }
    function expr() {
        var v = term();
        for (;;) {
            var c = peek();
            if (c === "+") { i++; v += term(); } else if (c === "-") { i++; v -= term(); } else return v;
        }
    }
    try {
        if (!s.trim()) return { value: null };
        var v = expr();
        if (peek() !== undefined) fail("I can't read " + s[i]);
        if (!isFinite(v)) fail("That's too big");
        return { value: v };
    } catch (e) {
        return { error: e.message };
    }
}
// A result the way a calculator shows it: up to 12 significant digits, no 0.30000000000000004.
function fmt(v) {
    if (v === null || v === undefined) return "";
    if (Math.abs(v) >= 1e15 || (Math.abs(v) < 1e-9 && v !== 0)) return v.toExponential(8).replace(/\.?0+e/, "e");
    var r = parseFloat(v.toPrecision(12));
    return String(r);
}

// ---------------------------------------------------------------- clocks
var HOURS = ["twelve", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven"];
// The time in words. level 1: to five minutes ("ten past three"); 2: to the quarter hour ("quarter to four");
// 3: the part of the day ("Afternoon").
function fuzzy(h, m, level) {
    if (level >= 3) return h < 5 ? "Night" : h < 9 ? "Early morning" : h < 12 ? "Morning" : h < 14 ? "Lunchtime" : h < 18 ? "Afternoon" : h < 22 ? "Evening" : "Night";
    var step = level === 2 ? 15 : 5;
    var r = Math.round(m / step) * step, hh = h;
    if (r === 60) { r = 0; hh = h + 1; }
    var next = HOURS[(hh + 1) % 12], cur = HOURS[hh % 12];
    var words = { 0: cur + " o'clock", 5: "five past " + cur, 10: "ten past " + cur, 15: "quarter past " + cur, 20: "twenty past " + cur,
                  25: "twenty-five past " + cur, 30: "half past " + cur, 35: "twenty-five to " + next, 40: "twenty to " + next,
                  45: "quarter to " + next, 50: "ten to " + next, 55: "five to " + next };
    var t = words[r];
    if (r === 0 && hh % 24 === 0) t = "midnight";
    if (r === 0 && hh % 24 === 12) t = "noon";
    return t.charAt(0).toUpperCase() + t.slice(1);
}
// A binary clock's columns (binary-coded decimal, as KDE's): [[h tens bits], [h ones], [m tens], [m ones], ([s…])],
// each bits top (8) to bottom (1); tens columns have fewer rows.
function binaryColumns(h, m, s, withSeconds) {
    function bits(n, rows) { var out = []; for (var r = rows - 1; r >= 0; r--) out.push((n >> r) & 1); return out; }
    var cols = [bits(Math.floor(h / 10), 2), bits(h % 10, 4), bits(Math.floor(m / 10), 3), bits(m % 10, 4)];
    if (withSeconds) cols.push(bits(Math.floor(s / 10), 3), bits(s % 10, 4));
    return cols;
}

// ---------------------------------------------------------------- colours
function hex2(n) { var h = Math.max(0, Math.min(255, Math.round(n))).toString(16); return h.length < 2 ? "0" + h : h; }
function toHex(r, g, b) { return "#" + hex2(r) + hex2(g) + hex2(b); }
// "#rrggbb" or "rrggbb" or "#rgb" → {r, g, b} (0-255), or null.
function parseHex(t) {
    var m = /^#?([0-9a-f]{6}|[0-9a-f]{3})$/i.exec(String(t || "").trim());
    if (!m) return null;
    var h = m[1].length === 3 ? m[1].replace(/(.)/g, "$1$1") : m[1];
    return { r: parseInt(h.slice(0, 2), 16), g: parseInt(h.slice(2, 4), 16), b: parseInt(h.slice(4, 6), 16) };
}
function formats(hex) {
    var c = parseHex(hex);
    if (!c) return [];
    var r = c.r / 255, g = c.g / 255, b = c.b / 255, mx = Math.max(r, g, b), mn = Math.min(r, g, b), l = (mx + mn) / 2, h = 0, sat = 0;
    if (mx !== mn) {
        var d = mx - mn;
        sat = l > 0.5 ? d / (2 - mx - mn) : d / (mx + mn);
        h = mx === r ? (g - b) / d + (g < b ? 6 : 0) : mx === g ? (b - r) / d + 2 : (r - g) / d + 4;
        h *= 60;
    }
    return [toHex(c.r, c.g, c.b).toUpperCase(), "rgb(" + c.r + ", " + c.g + ", " + c.b + ")",
            "hsl(" + Math.round(h) + ", " + Math.round(sat * 100) + "%, " + Math.round(l * 100) + "%)",
            "Qt.rgba(" + (c.r / 255).toFixed(3) + ", " + (c.g / 255).toFixed(3) + ", " + (c.b / 255).toFixed(3) + ", 1)"];
}
// KWin's colour picker answers a 32-bit ARGB number; grim -t ppm gives the raw bytes.
function argbToHex(n) { n = Number(n) >>> 0; return toHex((n >> 16) & 255, (n >> 8) & 255, n & 255); }

// ---------------------------------------------------------------- the bar's own items (Bar items)
// Each can sit at the start or the end of the main bar, in any order, or be hidden. Game Mode always stays, last.
var BAR_ITEMS = [
    { id: "menu", name: "App menu (the flame)", icon: "menu", zone: "start", keep: true },
    { id: "search", name: "Search field (Tiles menu)", icon: "search", zone: "start" },
    { id: "desktops", name: "Desktops", icon: "desktops", zone: "start" },
    { id: "activity", name: "Activity", icon: "home", zone: "start" },
    { id: "window", name: "The window in front", icon: "window", zone: "start" },
    { id: "jobs", name: "Copies and downloads", icon: "sync", zone: "end" },
    { id: "tray", name: "Apps' tray icons", icon: "grid", zone: "end" },
    { id: "background", name: "Background apps", icon: "rocket", zone: "end" },
    { id: "privacy", name: "Camera and microphone in use", icon: "cam", zone: "end" },
    { id: "printers", name: "Printing", icon: "printer", zone: "end" },
    { id: "disks", name: "Disks and devices", icon: "drive", zone: "end" },
    { id: "discord", name: "Discord", icon: "chat", zone: "end" },
    { id: "keyboard", name: "On-screen keyboard", icon: "kbd", zone: "end" },
    { id: "layout", name: "Keyboard layout", icon: "kbd", zone: "end" },
    { id: "network", name: "Network", icon: "wifi", zone: "end" },
    { id: "bluetooth", name: "Bluetooth", icon: "bt", zone: "end" },
    { id: "volume", name: "Volume", icon: "vol", zone: "end" },
    { id: "battery", name: "Battery", icon: "batt", zone: "end" },
    { id: "notifications", name: "Notifications", icon: "bell", zone: "end" },
    // Desktop layouts (hosts/hearth-layout blocks.rs): in the bar only where a layout or you put them ("extra": never
    // added by themselves, so a bar without them stays as it was)
    { id: "overview", name: "Desktops pill (opens the overview)", icon: "desktops", zone: "start", extra: true },
    { id: "taskview", name: "Task view", icon: "desktops", zone: "start", extra: true },
    { id: "appname", name: "The app in front, by name", icon: "window", zone: "start", extra: true },
    { id: "appmenus", name: "App menus (File, Edit…)", icon: "menu", zone: "start", extra: true },
    { id: "quickaccess", name: "Quick Access", icon: "grid", zone: "end", extra: true },
    { id: "showdesktop", name: "Show desktop", icon: "desktops", zone: "end", extra: true },
    { id: "power", name: "Power (shut down, restart, sign out)", icon: "power", zone: "end", extra: true }
];
function barItem(id) {
    for (var i = 0; i < BAR_ITEMS.length; i++) if (BAR_ITEMS[i].id === id) return BAR_ITEMS[i];
    return ownItem(id);
}
// A block you made in the desktop editor ("own:<id>", its look in bar.json "b:<id>"; OwnBarBlock.qml): only where
// you put it, never added by itself.
function ownItem(id) {
    return typeof id === "string" && /^own:[a-z0-9]{1,24}$/.test(id) ? { id: id, name: "Your block", icon: "dot", zone: "start", extra: true, own: true } : null;
}
// The saved order ({start: [...], end: [...], hidden: [...]}, or nothing) → the full layout: every known item exactly
// once, new items (a newer Hearth's) in their default place, unknown ids dropped, the flame never hidden.
function barLayout(saved) {
    var o = saved && typeof saved === "object" ? saved : {};
    var seen = {}, out = { start: [], centre: [], end: [], hidden: [] };
    ["start", "centre", "end", "hidden"].forEach(function (z) {
        var l = Array.isArray(o[z]) ? o[z] : [];
        for (var i = 0; i < l.length; i++) {
            var it = barItem(l[i]);
            if (!it || seen[l[i]]) continue;
            seen[l[i]] = true;
            out[z === "hidden" && it.keep ? it.zone : z].push(l[i]);
        }
    });
    for (var k = 0; k < BAR_ITEMS.length; k++) {
        var b = BAR_ITEMS[k];
        if (seen[b.id]) continue;
        if (b.extra) { out.hidden.push(b.id); continue; }   // a layout's own block: only where it's put
        // a new item goes next to the one before it in the default list, so it lands where it would by default
        var z = out[b.zone], at = -1;
        for (var p = k - 1; p >= 0 && at < 0; p--) at = z.indexOf(BAR_ITEMS[p].id);
        z.splice(at >= 0 ? at + 1 : (b.zone === "start" ? z.length : 0), 0, b.id);
    }
    return out;
}
function isDefaultLayout(l) { return JSON.stringify(barLayout(l)) === JSON.stringify(barLayout(null)); }
// Move ID one place (dir -1 / +1) inside its zone, or to the other end of the bar (zone "start" | "end" | "hidden").
function barMove(saved, id, dir, zone) {
    var l = barLayout(saved), from = l.start.indexOf(id) >= 0 ? "start" : l.centre.indexOf(id) >= 0 ? "centre" : l.end.indexOf(id) >= 0 ? "end" : "hidden";
    var it = barItem(id);
    if (!it) return l;
    if (zone && zone !== from) {
        if (zone === "hidden" && it.keep) return l;
        l[from] = l[from].filter(function (x) { return x !== id; });
        if (zone === "start") l.start.push(id); else l[zone].unshift(id);
        return l;
    }
    var a = l[from], i = a.indexOf(id), j = i + (dir || 0);
    if (j < 0 || j >= a.length) return l;
    a[i] = a[j]; a[j] = id;
    return l;
}

// ---------------------------------------------------------------- extra panels (ExtraPanel.qml)
// What an extra panel can hold. "widget:<type>" is a bar widget's small form (Widget*.qml compact).
var PANEL_ITEMS = [
    { id: "menu", name: "App menu", icon: "menu" },
    { id: "apps", name: "Your apps (the dock)", icon: "grid" },
    { id: "windows", name: "Window list", icon: "window" },
    { id: "pager", name: "Desktops (pager)", icon: "desktops" },
    { id: "spacer", name: "Space", icon: "dot" },
    { id: "clock", name: "Clock", icon: "clock" },
    { id: "tray", name: "Apps' tray icons", icon: "grid" },
    { id: "status", name: "Volume, network and battery", icon: "vol" },
    { id: "notifications", name: "Notifications", icon: "bell" },
    { id: "layout", name: "Keyboard layout", icon: "kbd" },
    { id: "gamemode", name: "Game Mode", icon: "pad" },
    { id: "widget:weather", name: "Weather", icon: "cloud" },
    { id: "widget:monitor", name: "System monitor", icon: "cpu" },
    { id: "widget:battery", name: "Battery", icon: "batt" },
    { id: "widget:timer", name: "Timer", icon: "bell" },
    // Desktop layouts' blocks (hearth-layout blocks.rs; Block*.qml, the same as in the main bar)
    { id: "overview", name: "Desktops pill (opens the overview)", icon: "desktops" },
    { id: "taskview", name: "Task view", icon: "desktops" },
    { id: "appname", name: "The app in front, by name", icon: "window" },
    { id: "appmenus", name: "App menus (File, Edit…)", icon: "menu" },
    { id: "quickaccess", name: "Quick Access", icon: "grid" },
    { id: "showdesktop", name: "Show desktop", icon: "desktops" },
    { id: "power", name: "Power (shut down, restart, sign out)", icon: "power" }
];
function panelItem(id) { for (var i = 0; i < PANEL_ITEMS.length; i++) if (PANEL_ITEMS[i].id === id) return PANEL_ITEMS[i]; return ownItem(id); }
var EDGES = ["top", "bottom", "left", "right"];
// One saved panel ({screen, edge, items, size}) as the shell uses it, or null when it can't be shown.
function panelOf(id, rec) {
    if (!rec || typeof rec !== "object" || Array.isArray(rec)) return null;
    var edge = EDGES.indexOf(rec.edge) >= 0 ? rec.edge : "bottom";
    var items = (Array.isArray(rec.items) ? rec.items : []).filter(function (x) { return !!panelItem(x); });
    return { id: id, screen: typeof rec.screen === "string" ? rec.screen : "", edge: edge, items: items,
             size: typeof rec.size === "number" && rec.size >= 28 && rec.size <= 96 ? Math.round(rec.size) : 44,
             autoHide: rec.autoHide === true,
             // desktop layouts: "" full width, "floating" (off the edge, rounded), "centred" (only as long as it holds)
             style: rec.style === "floating" || rec.style === "centred" ? rec.style : "",
             // anywhere on the screen (drag and drop, the owner Nov 26: "dont want them locked to just 4 sides"):
             // place "free", its top-left corner as a share of the screen (fx, fy), its length in px (0: as long as it
             // holds) and which way it runs ("across" or "down"); it keeps no room from the windows
             free: rec.place === "free",
             fx: clamp01(rec.fx), fy: clamp01(rec.fy),
             length: typeof rec.length === "number" && rec.length >= 40 && rec.length <= 8000 ? Math.round(rec.length) : 0,
             orient: rec.orient === "down" ? "down" : "across" };
}
function clamp01(v) { return typeof v === "number" && isFinite(v) ? Math.max(0, Math.min(1, v)) : 0.1; }
// The main bar as a panel's items (Clone the main bar).
function cloneMain(layout) {
    var l = barLayout(layout), out = [];
    if (l.start.indexOf("menu") >= 0) out.push("menu");
    if (l.start.indexOf("desktops") >= 0 || l.end.indexOf("desktops") >= 0) out.push("pager");
    out.push("windows", "spacer", "clock", "spacer");
    if (l.end.indexOf("tray") >= 0) out.push("tray");
    if (l.end.indexOf("layout") >= 0) out.push("layout");
    out.push("status", "notifications", "gamemode");
    return out;
}
// Whether a new panel on SCREEN / EDGE would sit on the main bar or the dock ("" = it's free).
function panelClash(panels, screen, edge, mainScreen, mainEdge, dockOn) {
    if (screen === mainScreen && edge === mainEdge) return "The main bar is there";
    if (screen === mainScreen && edge === "bottom" && dockOn) return "The dock is there";
    for (var i = 0; i < panels.length; i++) if (!panels[i].free && panels[i].screen === screen && panels[i].edge === edge) return "There's a panel there already";
    return "";
}

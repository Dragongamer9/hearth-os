// The volume mixer's rules (Mixer.qml), kept apart so they can be checked without a screen (test_shell_extras.py
// runs them with node). No Qt here.
.pragma library

var MAX_APP = 1.5;            // the most an app's slider goes to (the same as over-amplification's ceiling)
var LETTER_COLORS = ["#f2c14e", "#9a7cff", "#ff9a4d", "#7ccf8a", "#66c0f4", "#ff7aa8", "#4fd1c5"];

function str(x) { return x === undefined || x === null ? "" : String(x).trim(); }

// The name an app's sound is remembered by: what the app calls itself to PipeWire (application.name), else the
// program's name, else the stream's own name. "" when there's nothing to go by (never remembered).
function appKey(props) {
    var p = props || {};
    var k = str(p["application.name"]) || str(p["application.process.binary"]) || str(p["node.name"]);
    return k.slice(0, 120);
}

// An icon name to look for (its own icon, else the program's name).
function iconName(props) {
    var p = props || {};
    return str(p["application.icon-name"]) || str(p["application.process.binary"]);
}

function clamp(v, max) {
    var m = typeof max === "number" && max > 0 ? max : MAX_APP;
    var n = Number(v);
    if (!isFinite(n)) n = 0;
    return Math.round(Math.max(0, Math.min(m, n)) * 100) / 100;
}

// Left / Right (or the D-pad) move a slider by 5 %.
function step(v, dir, max) { return clamp(Number(v || 0) + (dir > 0 ? 0.05 : -0.05), max); }

// What was remembered, cleaned: only names with a sensible level (a hand-edited or damaged file can't set an app
// to 900 %).
function cleanRemembered(obj) {
    var out = {};
    if (!obj || typeof obj !== "object" || Array.isArray(obj)) return out;
    for (var k in obj) {
        var v = obj[k];
        if (typeof k === "string" && k.length > 0 && k.length <= 120 && typeof v === "number" && isFinite(v) && v >= 0 && v <= MAX_APP)
            out[k] = Math.round(v * 100) / 100;
    }
    return out;
}

// The level to put back when an app starts playing: the one you chose for it last time, when it differs from what
// it has now; null otherwise (nothing remembered, or already there).
function restoreLevel(remembered, key, current) {
    if (!key) return null;
    var r = cleanRemembered(remembered)[key];
    if (r === undefined) return null;
    if (typeof current === "number" && Math.abs(current - r) < 0.005) return null;
    return r;
}

// The change to save when you move an app's slider: {name: level}, merged into app-volumes.json on its own (the
// other apps' levels stay as they are on disk). null when there's no name to remember it by.
function rememberChange(key, v) {
    if (!key) return null;
    var o = {};
    o[key] = clamp(v);
    return o;
}

// One row per app: an app with several streams (a browser's tabs) is one row whose slider moves all of them.
// streams: [{id, key, ...}] in PipeWire's order → [{key, streams: [...]}] in first-seen order.
function group(streams) {
    var rows = [], at = {};
    for (var i = 0; i < (streams || []).length; i++) {
        var s = streams[i];
        var k = s && s.key ? s.key : (s ? "stream " + s.id : "");
        if (!k) continue;
        if (at[k] === undefined) { at[k] = rows.length; rows.push({ key: k, streams: [] }); }
        rows[at[k]].streams.push(s);
    }
    return rows;
}

function letter(name) { var n = str(name); return n ? n.charAt(0).toUpperCase() : "?"; }
function color(name) {
    var n = str(name), h = 0;
    for (var i = 0; i < n.length; i++) h = (h * 31 + n.charCodeAt(i)) >>> 0;
    return LETTER_COLORS[h % LETTER_COLORS.length];
}

// The keyboard / controller: row 0 is the output picker, row 1 the system volume, then the apps.
function moveRow(cur, dir, appCount) {
    var last = 1 + Math.max(0, appCount || 0);
    return Math.max(0, Math.min(last, (cur || 0) + (dir > 0 ? 1 : -1)));
}

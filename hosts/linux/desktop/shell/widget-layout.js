// Desktop widgets' rules (Widgets.qml, WidgetStore.qml), kept apart so they can be checked without a screen
// (test_desktop_widgets.py runs them with node). No Qt here.
//
// The layout file (~/.config/nimbus/widgets.json) has one top-level key per widget, "w:<id>", so a change to one
// widget is merged in alone (SettingsWriter / hearth_settings_io.py) and two widgets changed at once both keep their
// change. A widget: { type, screen, x, y, w, h, activity, place, cfg }
//   screen    the screen's name it was put on (HDMI-A-1, eDP-1…); a screen that isn't plugged in now keeps its
//             widgets in the file: they are shown on the main screen until it's back, and never moved by that
//   activity  "default", another activity's id, or "*" (on every activity)
//   place     "desk" (on the desktop) or "bar" (a small one in the bar)
// An entry the shell doesn't understand (a newer Hearth's widget, a hand edit) is left in the file untouched.
.pragma library

var CATALOG = [
    { type: "clock", name: "Clock", icon: "clock", w: 300, h: 220, minW: 180, minH: 130, bar: true,
      about: "Analog or digital, with as many cities as you like" },
    { type: "calendar", name: "Calendar", icon: "calendar", w: 340, h: 380, minW: 260, minH: 260, bar: false,
      about: "This month and what's coming up" },
    { type: "weather", name: "Weather", icon: "cloud", w: 330, h: 230, minW: 220, minH: 140, bar: true,
      about: "Open-Meteo forecast for a place you pick by name" },
    { type: "notes", name: "Sticky note", icon: "edit", w: 260, h: 240, minW: 160, minH: 120, bar: false,
      about: "A coloured note that stays on the desktop" },
    { type: "monitor", name: "System monitor", icon: "cpu", w: 380, h: 300, minW: 240, minH: 160, bar: true,
      about: "CPU, graphics, memory, disk and network graphs" },
    { type: "battery", settings: false, name: "Battery", icon: "batt", w: 260, h: 150, minW: 180, minH: 110, bar: true,
      about: "Charge, time left and connected devices" },
    { type: "media", settings: false, name: "Media player", icon: "headphones", w: 380, h: 160, minW: 260, minH: 120, bar: false,
      about: "What's playing, with play, pause and skip" },
    { type: "folder", name: "Folder view", icon: "folder", w: 400, h: 320, minW: 220, minH: 180, bar: false,
      about: "A folder's files right on the desktop" },
    { type: "photos", name: "Photo frame", icon: "image", w: 380, h: 270, minW: 180, minH: 140, bar: false,
      about: "Your Pictures or Hearth Photos favourites, one after another" },
    { type: "apps", name: "App tiles", icon: "grid", w: 340, h: 210, minW: 120, minH: 110, bar: false,
      about: "Big tiles for the apps you open most" },
    { type: "news", name: "News", icon: "globe", w: 400, h: 360, minW: 260, minH: 200, bar: false,
      about: "Headlines from any RSS or Atom feed" },
    { type: "timer", settings: false, name: "Timer & stopwatch", icon: "bell", w: 290, h: 230, minW: 220, minH: 160, bar: true,
      about: "Count down or time something" },
    { type: "quicknotes", settings: false, name: "Quick notes", icon: "list", w: 310, h: 330, minW: 220, minH: 180, bar: false,
      about: "A short list you can tick off" }
];

// Whether a widget has settings (its gear): all but those marked settings: false.
function hasSettings(type) { var t = info(type); return !!t && t.settings !== false; }

function info(type) {
    for (var i = 0; i < CATALOG.length; i++) if (CATALOG[i].type === type) return CATALOG[i];
    return null;
}

function num(v, d) { return typeof v === "number" && isFinite(v) ? v : d; }

// One entry of the file as the shell uses it, or null when it isn't one it can show (left in the file as it is).
function normalize(id, rec) {
    if (!rec || typeof rec !== "object" || Array.isArray(rec)) return null;
    var t = info(rec.type);
    if (!t) return null;
    var place = rec.place === "bar" ? "bar" : "desk";
    if (place === "bar" && !t.bar) return null;
    return {
        id: id, type: rec.type, place: place,
        screen: typeof rec.screen === "string" ? rec.screen : "",
        x: Math.round(num(rec.x, 0)), y: Math.round(num(rec.y, 0)),
        w: Math.round(Math.max(t.minW, num(rec.w, t.w))), h: Math.round(Math.max(t.minH, num(rec.h, t.h))),
        activity: typeof rec.activity === "string" && rec.activity !== "" ? rec.activity : "default",
        order: num(rec.order, 0),
        cfg: rec.cfg && typeof rec.cfg === "object" && !Array.isArray(rec.cfg) ? rec.cfg : {}
    };
}

// The whole file (an object) → { widgets: [...], skipped: n }. Keys that aren't widgets are ignored.
function parse(obj) {
    var out = [], skipped = 0;
    if (!obj || typeof obj !== "object" || Array.isArray(obj)) return { widgets: out, skipped: 0 };
    for (var k in obj) {
        if (k.indexOf("w:") !== 0) continue;
        var n = normalize(k.slice(2), obj[k]);
        if (n) out.push(n); else skipped++;
    }
    out.sort(function (a, b) { return a.order - b.order || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0); });
    return { widgets: out, skipped: skipped };
}

// What goes back into the file for one widget (only the fields the shell owns; anything else stays as it was:
// the caller merges this over the entry read from the file, see entryFor).
function record(w) {
    return { type: w.type, place: w.place || "desk", screen: w.screen || "", x: Math.round(w.x), y: Math.round(w.y),
             w: Math.round(w.w), h: Math.round(w.h), activity: w.activity || "default", order: w.order || 0, cfg: w.cfg || {} };
}

// The new entry for widget ID: the one in the file (so fields a newer Hearth added are kept) with CHANGES on top.
function entryFor(fileObj, id, changes) {
    var cur = fileObj && fileObj["w:" + id] && typeof fileObj["w:" + id] === "object" ? fileObj["w:" + id] : {};
    var o = {};
    for (var k in cur) o[k] = cur[k];
    for (var c in changes) o[c] = changes[c];
    return o;
}

function onActivity(w, activity) {
    return w.activity === "*" || w.activity === (activity || "default");
}

// The desktop widgets shown on SCREEN (a name) now: its own, and, on the main screen, those whose screen isn't
// plugged in (or was never named). CONNECTED is the list of the screens' names.
function forScreen(widgets, screen, connected, primary, activity) {
    var out = [];
    for (var i = 0; i < widgets.length; i++) {
        var w = widgets[i];
        if (w.place !== "desk" || !onActivity(w, activity)) continue;
        var home = connected.indexOf(w.screen) >= 0 ? w.screen : primary;
        if (home === screen) out.push(w);
    }
    return out;
}

function forBar(widgets, activity) {
    return widgets.filter(function (w) { return w.place === "bar" && onActivity(w, activity); });
}

// Where a widget is drawn on a screen of SW × SH (inside the bars' INSETS {top, bottom, left, right}): its own
// place, pulled back inside when the screen is smaller now. Only drawn there: the file keeps the saved place.
function shown(w, sw, sh, insets) {
    var ins = insets || {};
    var l = num(ins.left, 0), t = num(ins.top, 0), r = num(ins.right, 0), b = num(ins.bottom, 0);
    var t0 = info(w.type) || { minW: 100, minH: 80 };
    var aw = Math.max(t0.minW, sw - l - r), ah = Math.max(t0.minH, sh - t - b);
    var ww = Math.min(w.w, aw), hh = Math.min(w.h, ah);
    var x = Math.max(l, Math.min(w.x, l + aw - ww)), y = Math.max(t, Math.min(w.y, t + ah - hh));
    return { x: Math.round(x), y: Math.round(y), w: Math.round(ww), h: Math.round(hh) };
}

function overlaps(a, b, gap) {
    var g = gap || 0;
    return a.x < b.x + b.w + g && b.x < a.x + a.w + g && a.y < b.y + b.h + g && b.y < a.y + a.h + g;
}

// A free place for a new W × H widget: down the right side first (as KDE Plasma does), then the columns to its left;
// when everything is full, a little off from the top right so it can be seen.
function findSpot(rects, w, h, sw, sh, insets) {
    var ins = insets || {};
    var l = num(ins.left, 0) + 24, t = num(ins.top, 0) + 24, r = sw - num(ins.right, 0) - 24, b = sh - num(ins.bottom, 0) - 24;
    var step = 24;
    for (var x = r - w; x >= l; x -= step) {
        for (var y = t; y + h <= b; y += step) {
            var c = { x: x, y: y, w: w, h: h }, free = true;
            for (var i = 0; i < rects.length; i++) if (overlaps(c, rects[i], 12)) { free = false; break; }
            if (free) return { x: x, y: y };
        }
    }
    var n = rects.length % 8;
    return { x: Math.max(l, r - w - n * 32), y: t + n * 32 };
}

// The keyboard / controller in Arrange: the D-pad moves the picked widget, or (resize on) changes its size.
function nudge(w, dx, dy, resize, sw, sh, insets) {
    var t = info(w.type) || { minW: 100, minH: 80 };
    var o = { x: w.x, y: w.y, w: w.w, h: w.h };
    if (resize) {
        o.w = Math.max(t.minW, Math.min(sw, w.w + dx));
        o.h = Math.max(t.minH, Math.min(sh, w.h + dy));
    } else {
        o.x = w.x + dx;
        o.y = w.y + dy;
    }
    var s = shown(o, sw, sh, insets);
    return { x: s.x, y: s.y, w: resize ? s.w : w.w, h: resize ? s.h : w.h, type: w.type };
}

// The next widget picked with LB / RB (or Tab): in reading order, top to bottom, left to right.
function cycle(list, curId, dir) {
    if (!list.length) return "";
    var s = list.slice().sort(function (a, b) { return Math.round(a.y / 80) - Math.round(b.y / 80) || a.x - b.x; });
    var i = -1;
    for (var k = 0; k < s.length; k++) if (s[k].id === curId) i = k;
    i = i < 0 ? 0 : (i + (dir < 0 ? -1 : 1) + s.length) % s.length;
    return s[i].id;
}

function newId(now, rnd) {
    return (Math.floor(num(now, 0))).toString(36) + Math.floor(num(rnd, 0.5) * 1679616).toString(36);
}

// The gallery's grid (Add widgets): the D-pad moves through COLS columns.
function galleryMove(i, dx, dy, cols, n) {
    if (n <= 0) return 0;
    var r = Math.floor(i / cols), c = i % cols;
    if (dx) c = Math.max(0, Math.min(cols - 1, c + dx));
    if (dy) r = Math.max(0, Math.min(Math.ceil(n / cols) - 1, r + dy));
    return Math.min(n - 1, r * cols + c);
}

// ---- clocks: a city's time from its UTC offset (hearth-widgets zones gives the offsets, in minutes)
function cityTime(utcMs, offsetMin) {
    var d = new Date(utcMs + offsetMin * 60000);
    return { h: d.getUTCHours(), m: d.getUTCMinutes(), s: d.getUTCSeconds(), day: d.getUTCDay(), date: d.getUTCDate() };
}
function two(n) { return (n < 10 ? "0" : "") + n; }
function fmtTime(t, h24) {
    if (h24) return two(t.h) + ":" + two(t.m);
    var h = t.h % 12; if (h === 0) h = 12;
    return h + ":" + two(t.m) + (t.h < 12 ? " AM" : " PM");
}
// "Same day" / "Tomorrow" / "Yesterday" for a city against here.
function dayWord(cityDay, hereDay) {
    if (cityDay === hereDay) return "Today";
    if ((hereDay + 1) % 7 === cityDay) return "Tomorrow";
    return "Yesterday";
}

// ---- timer
function fmtDuration(ms) {
    ms = Math.max(0, Math.round(ms));
    var s = Math.floor(ms / 1000), h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), sec = s % 60;
    return (h ? h + ":" + two(m) : m) + ":" + two(sec);
}
function fmtStopwatch(ms) {
    ms = Math.max(0, Math.round(ms));
    return fmtDuration(ms) + "." + Math.floor((ms % 1000) / 100);
}

// ---- graphs: the last N samples of a value, newest last
function pushSample(list, v, n) {
    var o = (list || []).slice(-(n - 1));
    o.push(typeof v === "number" && isFinite(v) ? v : 0);
    return o;
}

// ---- calendar: the 6 × 7 days shown for a month (Monday first), each { y, m, d, inMonth }
function monthGrid(year, month) {
    var first = new Date(year, month, 1);
    var lead = (first.getDay() + 6) % 7;
    var out = [];
    for (var i = 0; i < 42; i++) {
        var d = new Date(year, month, 1 - lead + i);
        out.push({ y: d.getFullYear(), m: d.getMonth(), d: d.getDate(), inMonth: d.getMonth() === month });
    }
    return out;
}

// ---- weather words (the same as Hearth Weather, hosts/hearth-time/src/weather.rs)
function weatherWord(code) {
    var m = { 0: "Clear", 1: "Mostly clear", 2: "Partly cloudy", 3: "Cloudy", 45: "Fog", 48: "Fog", 51: "Drizzle", 53: "Drizzle",
              55: "Drizzle", 56: "Freezing drizzle", 57: "Freezing drizzle", 61: "Light rain", 63: "Rain", 65: "Heavy rain",
              66: "Freezing rain", 67: "Freezing rain", 71: "Light snow", 73: "Snow", 75: "Heavy snow", 77: "Snow grains",
              80: "Showers", 81: "Showers", 82: "Heavy showers", 85: "Snow showers", 86: "Snow showers", 95: "Thunderstorm",
              96: "Thunderstorm with hail", 99: "Thunderstorm with hail" };
    return m[code] || "—";
}
// A simple drawn sign for the code: sun, sun-cloud, cloud, fog, rain, snow, storm
function weatherKind(code, day) {
    if (code === 0 || code === 1) return day === false ? "moon" : "sun";
    if (code === 2) return day === false ? "moon-cloud" : "sun-cloud";
    if (code === 3) return "cloud";
    if (code === 45 || code === 48) return "fog";
    if ((code >= 51 && code <= 67) || (code >= 80 && code <= 82)) return "rain";
    if ((code >= 71 && code <= 77) || code === 85 || code === 86) return "snow";
    if (code >= 95) return "storm";
    return "cloud";
}

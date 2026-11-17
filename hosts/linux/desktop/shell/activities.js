// Activities' rules (Activities.qml, ActivitySwitcher.qml), kept apart so they can be checked without a screen
// (test_desktop_widgets.py runs them with node). No Qt here.
//
// An activity is its own set of windows, wallpaper, widgets and pinned apps (Work, Games…), like KDE Plasma's, but
// Hearth's own: it never needs KDE's activity manager. ~/.config/nimbus/activities.json, one top-level key each so a
// change merges in alone:
//   "current": "<id>"                          the one you're on ("default" when missing)
//   "act:<id>": { name, icon, wallpaper, pins, order }
//       wallpaper  "" = the desktop's own (Settings › Appearance); else a picture or "anim:…" as Settings.wallpaper
//       pins       null/missing = the taskbar's own pins (desk.json); a list = this activity's own pinned apps
//   "app:<app id>": "<id>"                     an app's windows open on this activity (set by Move to activity, or
//                                              by "Always open here")
// "default" (Main) always exists, even with no key: Hearth with no activities file is exactly as before.
// Which window is on which activity is kept for the session in $XDG_RUNTIME_DIR/hearth-activities-windows.json, by
// the window's key (KWin's window id through WinList, else app id + title).
.pragma library

var DEFAULT = "default";
var ICONS = ["home", "doc", "pad", "film", "palette", "globe", "chat", "rocket", "leaf", "steam"];

// A list from the settings file, whatever form Qt hands it over in (a JavaScript array, or the list type a
// value stored in a QML property can come back as, where Array.isArray says false); null when it isn't one.
function asList(v) {
    if (v === null || v === undefined || typeof v === "string" || typeof v !== "object") return null;
    if (Array.isArray(v)) return v;
    if (typeof v.length !== "number") return null;
    var o = [];
    for (var i = 0; i < v.length; i++) o.push(v[i]);
    return o;
}

function str(x) { return typeof x === "string" ? x : ""; }

function parse(obj) {
    var o = obj && typeof obj === "object" && !Array.isArray(obj) ? obj : {};
    var list = [], seen = {};
    for (var k in o) {
        if (k.indexOf("act:") !== 0) continue;
        var id = k.slice(4), a = o[k];
        if (!id || !a || typeof a !== "object" || Array.isArray(a)) continue;
        seen[id] = true;
        list.push({ id: id, name: str(a.name).trim() || (id === DEFAULT ? "Main" : "Activity"),
                    icon: ICONS.indexOf(a.icon) >= 0 ? a.icon : (id === DEFAULT ? "home" : "desktops"),
                    wallpaper: str(a.wallpaper), pins: asList(a.pins) ? asList(a.pins).filter(function (p) { return typeof p === "string"; }) : null,
                    order: typeof a.order === "number" ? a.order : (id === DEFAULT ? -1 : 0) });
    }
    if (!seen[DEFAULT]) list.push({ id: DEFAULT, name: "Main", icon: "home", wallpaper: "", pins: null, order: -1 });
    list.sort(function (a, b) { return a.order - b.order || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0); });
    var apps = {};
    for (var j in o) if (j.indexOf("app:") === 0 && typeof o[j] === "string") apps[j.slice(4)] = o[j];
    var cur = str(o.current);
    if (!list.some(function (a) { return a.id === cur; })) cur = DEFAULT;   // shown as Main; the file isn't changed
    return { current: cur, list: list, apps: apps };
}

function find(list, id) {
    for (var i = 0; i < list.length; i++) if (list[i].id === id) return list[i];
    return null;
}

// A name for a new activity that isn't taken yet: "Work", then "Games", "Study"…, then "Activity 2"…
function newName(list) {
    var taken = {};
    for (var i = 0; i < list.length; i++) taken[list[i].name.toLowerCase()] = true;
    var ideas = ["Work", "Games", "Study", "Media", "Projects"];
    for (var j = 0; j < ideas.length; j++) if (!taken[ideas[j].toLowerCase()]) return ideas[j];
    for (var n = 2; ; n++) if (!taken["activity " + n]) return "Activity " + n;
}

function newId(now, rnd) {
    return "a" + Math.floor(now || 0).toString(36) + Math.floor((rnd || 0.5) * 46656).toString(36);
}

// The key a window is remembered by: KWin's own id (the window bridge, WinList) when there is one; otherwise the app
// id and the title it first had.
function winKey(t) {
    if (!t) return "";
    var id = t.kwinId || t.internalId || t.uuid || t.windowId || "";
    if (id) return "k:" + id;
    return "t:" + str(t.appId) + "|" + str(t.title);
}

// Which activity a window is on: what it was put on this session, else where its app's windows go, else the one you
// were on when it opened (decided once, by the caller, and kept in the session map).
function activityOf(key, appId, map, apps, list, current) {
    var a = map && map[key];
    if (a && (a === "*" || find(list, a))) return a;
    var b = apps && apps[appId];
    if (b && find(list, b)) return b;
    return current || DEFAULT;
}

// Switching FROM → TO: windows to hide and to bring back. WINDOWS: [{ key, act, minimized }]. HIDDEN: the keys of
// windows Hearth hid when leaving an activity (only those come back: a window you minimized yourself stays down).
function planSwitch(windows, from, to, hidden) {
    var hide = [], show = [], keep = {};
    var h = {};
    for (var i = 0; i < (hidden || []).length; i++) h[hidden[i]] = true;
    for (var j = 0; j < windows.length; j++) {
        var w = windows[j];
        if (w.act === "*") continue;
        if (w.act === to) {
            if (h[w.key] && w.minimized) show.push(w.key);
        } else if (!w.minimized) {
            hide.push(w.key);
            keep[w.key] = true;
        } else if (h[w.key]) {
            keep[w.key] = true;     // still hidden by Hearth (another activity's): it comes back on its own one
        }
    }
    var still = [];
    for (var k in keep) still.push(k);
    return { hide: hide, show: show, hidden: still };
}

// LB / RB (or Super+Ctrl+A / Super+Shift+Ctrl+A): the next activity in order.
function step(list, cur, dir) {
    if (!list.length) return DEFAULT;
    var i = -1;
    for (var k = 0; k < list.length; k++) if (list[k].id === cur) i = k;
    return list[((i < 0 ? 0 : i) + (dir < 0 ? -1 : 1) + list.length) % list.length].id;
}

// The pins shown in the taskbar on activity ACT: its own list, or the taskbar's own (desk.json) when it has none.
function pinsFor(list, act, deskPins) {
    var a = find(list, act);
    return a && asList(a.pins) ? asList(a.pins) : deskPins;
}

// Pin or unpin ID on an activity with its own list (a copy; never touches desk.json's).
function togglePin(pins, id, on) {
    var p = (pins || []).filter(function (x) { return x !== id; });
    if (on) p.push(id);
    return p;
}

// Session map tidy-up: windows that are gone are forgotten (KEYS: the keys of the windows open now).
function prune(map, keys) {
    var o = {}, live = {};
    for (var i = 0; i < keys.length; i++) live[keys[i]] = true;
    for (var k in map) if (live[k]) o[k] = map[k];
    return o;
}

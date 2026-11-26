.pragma library
// A block's look and what it does (drag and drop board 2, "One block picked: everything about it";
// docs/mockups/drag-drop/drag-drop-2.png, checklist docs/mockups/checklists/drag-drop.md part 2). No Qt here, so
// test_desktop_edit.py runs these rules with node. BlockLook.qml draws a look around every block of the main bar
// (Islands.qml), the extra panels (ExtraPanels.qml) and your own blocks (OwnBarBlock.qml); DesktopEditor.qml's side
// panel changes it.
//
// Kept in bar.json, one key per block so a merge only touches that block: "s:<block id>" (the id it has in its bar:
// "clock", "own:k1", "volume#k2" for a second copy). Only what differs from the defaults is kept (sparse), and a
// record that can't be read is the defaults (it never stops a block from drawing).

var DEFAULT = {
    // SIZE AND PLACE
    w: "fit", wpx: 0,                 // Width: fit (its own), fixed (wpx), fill (the room the bar has)
    h: 0,                             // Height in px (0: its own)
    mt: 0, mr: 0, mb: 0, ml: 0,       // Space around it, each side
    pad: -1,                          // Space inside (-1: its own)
    // BACKGROUND AND EDGE
    fill: "",                         // "" none (its own), "#rrggbb"
    see: 0,                           // See-through, %
    blur: false,                      // Blur behind it
    corners: -1,                      // -1 its own, 0..20 px, 99 round
    edge: "none", edgeColor: "", edgeW: 1,   // Edge: none | line | glow; its colour ("" the accent); width px
    shadow: "none",                   // none | soft | deep
    // ICON AND WORDS
    icon: "", iconColor: "", iconSize: 0,    // "" its own
    words: "",                        // with {count}, {percent}, {name}, {time}, {date} where the block has them
    font: "", fontSize: 0, weight: "medium", // "" the look's font; Light | Medium | Bold
    show: "both",                     // icon | words | both
    // WHEN IT'S POINTED AT · PRESSED
    hover: "same", hoverOwn: "",      // same | lighter | accent | own (hoverOwn: its colour)
    pressed: "same", pressedOwn: "",  // same | darker | own
    // Shows: what's in it
    shows: "own",                     // own | count | list | picture | clock | words
    list: "",                         // a list's lines, one per line
    pic: "",                          // a picture's file
    clock: "",                        // a clock's format ("" h:mm AP)
    // Does: press, right-click, middle-click, scroll, hover → its own | app | run | panel | nothing
    doPress: "default", argPress: "",
    doRight: "default", argRight: "",
    doMiddle: "default", argMiddle: "",
    doScroll: "default", argScroll: "",
    doHover: "default", argHover: "",
    // Place: which screen, which desktops ([] every one)
    screen: "", desktops: [],
    // When: always | hover | something | apps (only while one of APPS is in front)
    when: "always", apps: "",
    // Motion: how it appears, and a bounce when something new comes
    appear: "none", bounce: false,
    // Hide (keep it here)
    hidden: false
};

var CHOICES = {
    w: ["fit", "fixed", "fill"], edge: ["none", "line", "glow"], shadow: ["none", "soft", "deep"],
    weight: ["light", "medium", "bold"], show: ["icon", "words", "both"], hover: ["same", "lighter", "accent", "own"],
    pressed: ["same", "darker", "own"], shows: ["own", "count", "list", "picture", "clock", "words"],
    doPress: ["default", "app", "run", "panel", "nothing"], when: ["always", "hover", "something", "apps"],
    appear: ["none", "fade", "slide", "grow"]
};
["doRight", "doMiddle", "doScroll", "doHover"].forEach(function (k) { CHOICES[k] = CHOICES.doPress; });
var RANGES = { wpx: [0, 4000], h: [0, 400], mt: [0, 200], mr: [0, 200], mb: [0, 200], ml: [0, 200], pad: [-1, 100],
               see: [0, 100], edgeW: [1, 12], iconSize: [0, 128], fontSize: [0, 96] };
// "Copy its look" takes the Look tab (not what it shows, does, where or when).
var LOOK_KEYS = ["w", "wpx", "h", "mt", "mr", "mb", "ml", "pad", "fill", "see", "blur", "corners", "edge", "edgeColor",
                 "edgeW", "shadow", "icon", "iconColor", "iconSize", "words", "font", "fontSize", "weight", "show",
                 "hover", "hoverOwn", "pressed", "pressedOwn"];
// The panels a block can show (Does › Show a panel)
var PANELS = [["launcher", "Apps"], ["quick", "Quick settings"], ["calendar", "Notifications and calendar"],
              ["overview", "Overview"], ["clipboard", "Clipboard"]];

function isHex(v) { return typeof v === "string" && /^#[0-9a-fA-F]{6}$/.test(v); }
function copy(o) { return JSON.parse(JSON.stringify(o)); }

// One field, checked: what's stored, or its default when it can't be used.
function field(k, v) {
    var d = DEFAULT[k];
    if (CHOICES[k]) return CHOICES[k].indexOf(v) >= 0 ? v : d;
    if (RANGES[k]) return typeof v === "number" && isFinite(v) ? Math.round(Math.max(RANGES[k][0], Math.min(RANGES[k][1], v))) : d;
    if (k === "corners") return typeof v === "number" && isFinite(v) ? (v >= 99 ? 99 : v < 0 ? -1 : Math.round(Math.min(20, v))) : d;
    if (k === "fill" || k === "edgeColor" || k === "iconColor" || k === "hoverOwn" || k === "pressedOwn") return isHex(v) ? v : "";
    if (k === "desktops") return Array.isArray(v) ? v.filter(function (n) { return typeof n === "number" && n >= 1 && n <= 32; }).map(Math.round) : [];
    if (typeof d === "boolean") return v === true;
    if (typeof d === "string") return typeof v === "string" ? v.slice(0, 2000) : d;
    return d;
}
// A stored record (or anything else) → the whole look.
function normalize(raw) {
    var r = raw && typeof raw === "object" && !Array.isArray(raw) ? raw : {};
    var out = {};
    for (var k in DEFAULT) out[k] = r[k] === undefined ? copy(DEFAULT[k]) : field(k, r[k]);
    return out;
}
// Only what differs from the defaults (what's kept in bar.json). An empty look is null: the key is taken out.
function sparse(look) {
    var n = normalize(look), out = {}, any = false;
    for (var k in DEFAULT) if (JSON.stringify(n[k]) !== JSON.stringify(DEFAULT[k])) { out[k] = n[k]; any = true; }
    return any ? out : null;
}
// Merge changes into a look (one field or several).
function merge(look, changes) {
    var n = normalize(look);
    for (var k in changes) if (DEFAULT[k] !== undefined) n[k] = field(k, changes[k]);
    return n;
}
function isDefault(look) { return sparse(look) === null; }

// ---- Copy its look, Paste a look, Apply to every bar item
function copyLook(look) {
    var n = normalize(look), out = {};
    LOOK_KEYS.forEach(function (k) { out[k] = copy(n[k]); });
    return out;
}
function pasteLook(look, clip) {
    if (!clip || typeof clip !== "object") return normalize(look);
    var n = normalize(look);
    LOOK_KEYS.forEach(function (k) { if (clip[k] !== undefined) n[k] = field(k, clip[k]); });
    return n;
}
// looks: {id: sparse look}; ids: every block in that bar; from: the look to give them. Returns the new looks.
function applyToAll(looks, ids, from) {
    var out = copy(looks || {}), clip = copyLook(from);
    (ids || []).forEach(function (id) {
        if (id === "spacer" || id === "~") return;
        var s = sparse(pasteLook(out[id], clip));
        if (s) out[id] = s; else delete out[id];
    });
    return out;
}

// ---- what a block can show in its words, and its value now
// src: what the shell knows ({jobs, jobsPct, unseen, vol, muted, batt, net, title, app, tray, printers, disks, bg,
// time, date}); base: the block (without a copy's #key).
var VALUES = {
    jobs: ["count", "percent"], notifications: ["count"], volume: ["percent"], battery: ["percent"], status: ["percent"],
    network: ["name"], window: ["name"], appname: ["name"], tray: ["count"], printers: ["count"], disks: ["count"],
    background: ["count"], clock: ["time", "date"], windows: ["count"], desktops: ["count"], pager: ["count"],
    privacy: ["count"]
};
function valueKeys(base) { return (VALUES[base] || []).concat(["time", "date"]).filter(function (k, i, a) { return a.indexOf(k) === i; }); }
function hasValue(base) { return !!VALUES[base]; }
function values(base, src) {
    var s = src || {}, v = { time: s.time || "", date: s.date || "", count: "", percent: "", name: "" };
    if (base === "jobs") { v.count = String(s.jobs || 0); v.percent = s.jobsPct >= 0 ? s.jobsPct + "%" : ""; }
    else if (base === "notifications") v.count = String(s.unseen || 0);
    else if (base === "volume" || base === "status") v.percent = s.muted ? "Muted" : (s.vol || 0) + "%";
    else if (base === "battery") v.percent = (s.batt || 0) + "%";
    else if (base === "network") v.name = s.net || "Not connected";
    else if (base === "window" || base === "appname") v.name = s.title || s.app || "";
    else if (base === "tray") v.count = String(s.tray || 0);
    else if (base === "printers") v.count = String(s.printers || 0);
    else if (base === "disks") v.count = String(s.disks || 0);
    else if (base === "background") v.count = String(s.bg || 0);
    else if (base === "windows") v.count = String(s.windows || 0);
    else if (base === "desktops" || base === "pager") v.count = String(s.desks || 0);
    else if (base === "privacy") v.count = String(s.privacy || 0);
    return v;
}
// "Downloads · {count}" with the values put in; a value the block hasn't got is left out.
function fillWords(tpl, v) {
    return String(tpl || "").replace(/\{(count|percent|name|time|date)\}/g, function (m, k) { return v && v[k] !== undefined ? String(v[k]) : ""; });
}
// Whether there's something to show (When › only when there's something to show).
function something(base, v) {
    if (!hasValue(base)) return true;
    if (v.count !== "") return v.count !== "0";
    if (v.name !== "") return true;
    return v.percent !== "";
}

// ---- what a built-in block draws: a simple one (one icon, maybe a number) has its icon and words replaced by
// yours; one that draws many things of its own (the tray, your apps, the desktops) keeps them, with your icon and
// words before them.
var MANY = ["tray", "desktops", "pager", "apps", "windows", "window", "activity", "appmenus", "search", "privacy", "network"];
function simple(base) { return MANY.indexOf(base) < 0; }
function why(base) {
    return simple(base) ? "" : "It draws its own icons, so your icon and words go before them";
}
// Whether the block's own content is put away for yours (BlockLook.qml).
function replaces(base, look) {
    var n = normalize(look);
    if (n.shows !== "own") return true;
    if (!simple(base)) return false;
    return n.icon !== "" || n.iconColor !== "" || n.iconSize !== 0 || n.words !== "" || n.font !== "" || n.fontSize !== 0
        || n.weight !== "medium" || n.show !== "both";
}
// Whether it has a label before its own content (a block that draws many things, with your icon or words).
function labels(base, look) {
    var n = normalize(look);
    return n.shows === "own" && !simple(base) && (n.icon !== "" || n.words !== "");
}
// What's shown in its place: {icon, words} (show: icon | words | both). def: its own icon and name.
function content(base, look, v, def) {
    var n = normalize(look), d = def || {};
    var words = n.words !== "" ? fillWords(n.words, v) : "";
    if (n.shows === "count") words = (n.words !== "" ? fillWords(n.words, v) : (v.count || v.percent || v.name || "0"));
    else if (n.shows === "clock") words = v.clockNow || v.time || "";
    else if (n.shows === "list") words = String(n.list || "").split("\n").map(function (x) { return x.trim(); }).filter(Boolean).join("  ·  ");
    else if (n.shows === "words" && words === "") words = d.name || "";
    var icon = n.icon !== "" ? n.icon : (d.icon || "dot");
    var showIcon = n.show !== "words", showWords = n.show !== "icon";
    if (n.shows === "own" && n.words === "" && n.show === "words") words = d.name || "";
    return { icon: showIcon && n.shows !== "picture" ? icon : "", words: showWords ? words : "", pic: n.shows === "picture" ? n.pic : "" };
}
// Hover / pressed: the colour laid over it ("" nothing). accent: the look's accent.
function hoverWash(look, accent) {
    var n = normalize(look);
    if (n.hover === "lighter") return "#1affffff";
    if (n.hover === "accent") return accent ? "#59" + String(accent).replace("#", "").slice(-6) : "#59ff8a3d";
    if (n.hover === "own" && n.hoverOwn) return "#80" + n.hoverOwn.slice(1);
    return "";
}
function pressWash(look) {
    var n = normalize(look);
    if (n.pressed === "darker") return "#40000000";
    if (n.pressed === "own" && n.pressedOwn) return "#99" + n.pressedOwn.slice(1);
    return "";
}
// Whether anything it does was changed (the block then takes those buttons itself).
function takes(look) {
    var n = normalize(look);
    return n.doPress !== "default" || n.doRight !== "default" || n.doMiddle !== "default" || n.doScroll !== "default";
}
// What a press (or right-click…) does: null its own, or {kind: "app"|"run"|"panel"|"nothing", arg}
function action(look, which) {
    var n = normalize(look), d = n["do" + which], a = n["arg" + which];
    if (!d || d === "default") return null;
    if ((d === "run" || d === "app" || d === "panel") && !a) return null;
    return { kind: d, arg: a };
}
// The command it runs, as a list (never through a shell's string building, except your own command, which is yours).
function command(act, shellDir) {
    if (!act) return null;
    if (act.kind === "run") return ["sh", "-c", act.arg];
    if (act.kind === "app") return ["gtk-launch", act.arg];
    if (act.kind === "panel") {
        var ok = PANELS.some(function (p) { return p[0] === act.arg; });
        return ok ? ["quickshell", "ipc", "-p", shellDir || "", "call", act.arg, "toggle"] : null;
    }
    return null;
}

// ---- Place and When
// Whether it shows here now: hidden, a screen, some desktops, some apps in front, only when there's something.
// ctx: {screen, screens, desk (1-based, 0 unknown), app, something, editing}
function shownHere(look, ctx) {
    var n = normalize(look), c = ctx || {};
    if (c.editing) return true;
    if (n.hidden) return false;
    if (n.screen && (c.screens || []).indexOf(n.screen) >= 0 && n.screen !== c.screen) return false;
    if (n.desktops.length && c.desk > 0 && n.desktops.indexOf(c.desk) < 0) return false;
    if (n.when === "something" && c.something === false) return false;
    if (n.when === "apps") {
        var want = String(n.apps || "").split(/[,\n]/).map(function (x) { return x.trim().toLowerCase(); }).filter(Boolean);
        if (want.length && want.indexOf(String(c.app || "").toLowerCase()) < 0) return false;
    }
    return true;
}

// ---- Keep this: nothing is written over a file that couldn't be read
// readOk: bar.json was read (or isn't there yet). changes: what Keep this would write.
function keepPlan(readOk, changes) {
    var n = changes ? Object.keys(changes).length : 0;
    if (!readOk) return { write: false, why: "Your bar settings file couldn't be read, so nothing was saved over it (its last good copy is being put back)" };
    return { write: n > 0, why: n > 0 ? "" : "Nothing changed" };
}

// ---- the corner radius: -1 its own (def), 99 round
function radius(look, h, def) {
    var c = normalize(look).corners;
    if (c < 0) return def;
    if (c >= 99) return Math.round(h / 2);
    return c;
}
// "15 %" "Round" "8 px" for the side panel
function cornersLabel(c) { return c < 0 ? "Its own" : c >= 99 ? "Round" : c + " px"; }
// "#rrggbb" and a 0..1 alpha → "#aarrggbb" (a colour QML takes); a bad colour → fallback (or transparent)
function argb(hex, alpha, fallback) {
    var h = String(hex || "");
    if (/^#[0-9a-fA-F]{8}$/.test(h)) h = "#" + h.slice(3);
    if (!/^#[0-9a-fA-F]{6}$/.test(h)) return fallback || "#00000000";
    var a = Math.round(Math.max(0, Math.min(1, alpha === undefined ? 1 : alpha)) * 255).toString(16);
    return "#" + (a.length < 2 ? "0" + a : a) + h.slice(1);
}

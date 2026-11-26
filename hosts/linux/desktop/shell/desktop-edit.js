.pragma library
// The desktop editor's rules (DesktopEditor.qml; docs/mockups/checklists/drag-drop.md, board 1), kept apart so they
// can be checked without a screen (test_desktop_edit.py runs them with node). No Qt here.
//
// A draft is what the editor is changing, and nothing is written until Keep this:
//   { main: {start, centre, end, hidden},          the main bar's items (bar.json "items")
//     panels: { <id>: rec },                       the extra panels (bar.json "p:<id>", widget-tools.js panelOf)
//     widgets: { <id>: {type, x, y, w, h} },       the desktop widgets on this screen (widgets.json "w:<id>")
//     own: { <id>: {kind, label, icon, run, text, pic} } }   blocks you make (bar.json "b:<id>", items "own:<id>")
// Where a block is dropped: { kind: "main", zone, index } | { kind: "panel", id, index } | { kind: "desk", x, y } | null

var T = null;   // widget-tools.js (barItem, panelItem), given by the editor (and by the test)
function use(tools) { T = tools; }

function copy(o) { return JSON.parse(JSON.stringify(o)); }

// ---- what can go where
// Blocks the main bar and the panels call by different names (the panel's "status" is the bar's volume, network and
// battery together); a block is renamed when it's moved between them.
var TO_PANEL = { desktops: "pager", volume: "status", network: "status", battery: "status" };
var TO_MAIN = { pager: "desktops", status: "volume" };
function isOwn(id) { return typeof id === "string" && id.indexOf("own:") === 0; }
function widgetType(id) { return typeof id === "string" && id.indexOf("widget:") === 0 ? id.slice(7) : ""; }

// The id this block has in a target, or "" when it can't go there (and why, for the note).
function fit(id, target, widgetInfo) {
    if (!target) return { id: "", why: "" };
    if (target.kind === "main") {
        if (isOwn(id)) return { id: id, why: "" };
        var m = TO_MAIN[id] || id;
        if (T && T.barItem(m)) return { id: m, why: "" };
        return { id: "", why: "The main bar can't hold this one: put it in a panel or on the desktop" };
    }
    if (target.kind === "panel") {
        if (isOwn(id)) return { id: id, why: "" };
        var p = TO_PANEL[id] || id;
        if (T && T.panelItem(p)) return { id: p, why: "" };
        return { id: "", why: "A panel can't hold this one yet" };
    }
    if (target.kind === "desk") {
        var w = widgetType(id) || id;
        if (widgetInfo && widgetInfo(w)) return { id: "widget:" + w, why: "" };
        return { id: "", why: "Only widgets go straight on the desktop: put this one in a bar or a panel" };
    }
    return { id: "", why: "" };
}

// ---- where the pointer is, in a bar: the place between its blocks
// centres: the middle of each block along the bar, in order. Returns the index it would be put at.
function insertAt(centres, along) {
    for (var i = 0; i < centres.length; i++) if (along < centres[i]) return i;
    return centres.length;
}

// ---- taking a block out of where it was, putting it where it's dropped
// from: { kind: "main", zone } | { kind: "panel", id } | { kind: "desk", id } | { kind: "lib" }
function take(d, id, from) {
    if (!from) return;
    if (from.kind === "main") {
        ["start", "centre", "end"].forEach(function (z) { d.main[z] = d.main[z].filter(function (x) { return x !== id; }); });
        if (!isOwn(id) && d.main.hidden.indexOf(id) < 0) d.main.hidden.push(id);
    } else if (from.kind === "panel" && d.panels[from.id]) {
        var it = d.panels[from.id].items || [], k = it.indexOf(id);
        if (k >= 0) it.splice(k, 1);
    } else if (from.kind === "desk") {
        delete d.widgets[from.id];
    }
}
// Move block ID from FROM to TARGET (with the name it has there). Returns the new draft, or null when nothing changes.
// newWidget(type) gives a new widget's id and size; at is where it lands on the desktop.
function drop(draft, id, from, target, widgetInfo, newId) {
    var f = fit(id, target, widgetInfo);
    if (!f.id) return null;
    var d = copy(draft);
    if (target.kind === "main" && from && from.kind === "main") {
        // along the same bar: keep the index right after taking it out
        var z0 = d.main[from.zone] || [], i0 = z0.indexOf(id);
        if (from.zone === target.zone && i0 >= 0 && i0 < target.index) target = { kind: "main", zone: target.zone, index: target.index - 1 };
    }
    if (target.kind === "panel" && from && from.kind === "panel" && from.id === target.id) {
        var it0 = (d.panels[from.id].items || []), j0 = it0.indexOf(id);
        if (j0 >= 0 && j0 < target.index) target = { kind: "panel", id: target.id, index: target.index - 1 };
    }
    take(d, id, from);
    if (target.kind === "main") {
        ["start", "centre", "end", "hidden"].forEach(function (z) { d.main[z] = d.main[z].filter(function (x) { return x !== f.id; }); });
        var z = d.main[target.zone] || (d.main[target.zone] = []);
        z.splice(Math.max(0, Math.min(target.index, z.length)), 0, f.id);
    } else if (target.kind === "panel") {
        var p = d.panels[target.id];
        if (!p) return null;
        p.items = (p.items || []).filter(function (x) { return x !== f.id || x === "spacer"; });
        p.items.splice(Math.max(0, Math.min(target.index, p.items.length)), 0, f.id);
    } else if (target.kind === "desk") {
        var t = widgetType(f.id), info = widgetInfo(t);
        var wid = from && from.kind === "desk" ? from.id : newId();
        var old = from && from.kind === "desk" ? draft.widgets[from.id] : null;
        d.widgets[wid] = { type: t, x: Math.round(target.x), y: Math.round(target.y), w: old ? old.w : info.w, h: old ? old.h : info.h };
    }
    return d;
}

// ---- a block you make (Make your own): its first look
function ownBlock(kind) {
    if (kind === "text") return { kind: "text", label: "Your words", icon: "", run: "" };
    if (kind === "picture") return { kind: "picture", label: "", icon: "image", pic: "" };
    if (kind === "spacer") return { kind: "spacer", label: "", icon: "" };
    return { kind: "button", label: "My button", icon: "rocket", run: "" };
}

// ---- panels anywhere: from where it's dropped to its record (free, or on an edge when it's let go near one)
// r: the panel's box {x, y, w, h} on a screen sw × sh. Within SNAP px of an edge it goes on that edge.
var EDGE_SNAP = 24;
function placePanel(rec, r, sw, sh, snapEdges) {
    var o = copy(rec);
    var down = r.h > r.w;
    var near = !snapEdges ? "" : r.y <= EDGE_SNAP && !down ? "top" : r.y + r.h >= sh - EDGE_SNAP && !down ? "bottom"
        : r.x <= EDGE_SNAP && down ? "left" : r.x + r.w >= sw - EDGE_SNAP && down ? "right" : "";
    if (near) {
        o.place = ""; o.edge = near;
        // a short one on an edge is centred, a long one fills it
        o.style = (down ? r.h : r.w) < (down ? sh : sw) * 0.9 ? "centred" : "";
        delete o.fx; delete o.fy;
    } else {
        o.place = "free";
        o.orient = down ? "down" : "across";
        o.fx = sw > r.w ? Math.max(0, Math.min(1, r.x / (sw - r.w))) : 0;
        o.fy = sh > r.h ? Math.max(0, Math.min(1, r.y / (sh - r.h))) : 0;
        o.fx = Math.round(o.fx * 1000) / 1000; o.fy = Math.round(o.fy * 1000) / 1000;
    }
    o.size = Math.max(28, Math.min(96, Math.round(down ? r.w : r.h)));
    var len = Math.round(down ? r.h : r.w);
    if (o.place === "free") o.length = Math.max(40, len); else delete o.length;
    return o;
}
// A panel's box on the screen, from its record (the way ExtraPanels.qml puts it). want: its blocks' length.
function panelBox(p, sw, sh, want) {
    var down = p.free ? p.orient === "down" : p.edge === "left" || p.edge === "right";
    var size = p.size || 44;
    if (p.free) {
        var len = p.length > 0 ? p.length : Math.max(60, want || 200);
        var w = down ? size : len, h = down ? len : size;
        return { x: Math.round(p.fx * Math.max(0, sw - w)), y: Math.round(p.fy * Math.max(0, sh - h)), w: w, h: h, down: down };
    }
    var gap = p.style === "floating" || p.style === "centred" ? 8 : 0;
    var along = (down ? sh : sw) - gap * 2;
    var l = p.style === "centred" ? Math.min(along, Math.max(60, want || 200)) : along;
    if (p.edge === "top") return { x: (sw - l) / 2, y: gap, w: l, h: size, down: false };
    if (p.edge === "bottom") return { x: (sw - l) / 2, y: sh - gap - size, w: l, h: size, down: false };
    if (p.edge === "left") return { x: gap, y: (sh - l) / 2, w: size, h: l, down: true };
    return { x: sw - gap - size, y: (sh - l) / 2, w: size, h: l, down: true };
}

// ---- snapping: to the grid, and lined up with the other things (the guides)
// r: the box being moved; others: [{x, y, w, h, name}]; grid: px (0: off). Returns {x, y, guides: [{x}|{y}], note}.
var LINE_UP = 6;
function snap(r, others, grid, guidesOn) {
    var x = r.x, y = r.y, gx = null, gy = null, note = "";
    if (guidesOn) {
        var best = LINE_UP + 1;
        others.forEach(function (o) {
            [[o.x, r.x], [o.x + o.w, r.x + r.w], [o.x, r.x + r.w], [o.x + o.w, r.x]].forEach(function (pr, k) {
                var dd = Math.abs(pr[0] - pr[1]);
                if (dd < best) { best = dd; gx = pr[0]; x = r.x + (pr[0] - pr[1]); note = "Lined up with " + (o.name || "it"); }
            });
        });
        var bestY = LINE_UP + 1;
        others.forEach(function (o) {
            [[o.y, r.y], [o.y + o.h, r.y + r.h], [o.y, r.y + r.h], [o.y + o.h, r.y]].forEach(function (pr) {
                var dd = Math.abs(pr[0] - pr[1]);
                if (dd < bestY) { bestY = dd; gy = pr[0]; y = r.y + (pr[0] - pr[1]); if (!note) note = "Lined up with " + (o.name || "it"); }
            });
        });
    }
    if (grid > 0) {
        if (gx === null) x = Math.round(x / grid) * grid;
        if (gy === null) y = Math.round(y / grid) * grid;
    }
    var g = [];
    if (gx !== null) g.push({ x: gx });
    if (gy !== null) g.push({ y: gy });
    return { x: x, y: y, guides: g, note: note };
}

// ---- Keep this: what to write (bar.json keys, widgets to put, add and remove)
// base: the draft as it was opened. Returns { bar: {key: value|null}, widgets: {put: {id: rec}, add: [...], remove: [...]} }
function changes(base, d) {
    var bar = {};
    if (JSON.stringify(base.main) !== JSON.stringify(d.main)) bar.items = d.main;
    var k;
    for (k in d.panels) if (JSON.stringify(base.panels[k]) !== JSON.stringify(d.panels[k])) bar["p:" + k] = d.panels[k];
    for (k in base.panels) if (!d.panels[k]) bar["p:" + k] = null;
    for (k in d.own) if (JSON.stringify(base.own[k]) !== JSON.stringify(d.own[k])) bar["b:" + k] = d.own[k];
    for (k in base.own) if (!d.own[k]) bar["b:" + k] = null;
    var put = {}, add = [], remove = [];
    for (k in d.widgets) {
        if (!base.widgets[k]) add.push({ id: k, rec: d.widgets[k] });
        else if (JSON.stringify(base.widgets[k]) !== JSON.stringify(d.widgets[k])) put[k] = d.widgets[k];
    }
    for (k in base.widgets) if (!d.widgets[k]) remove.push(k);
    return { bar: bar, widgets: { put: put, add: add, remove: remove } };
}
function nothingChanged(c) {
    return Object.keys(c.bar).length === 0 && Object.keys(c.widgets.put).length === 0 && c.widgets.add.length === 0 && c.widgets.remove.length === 0;
}

// ---- labels: what each thing is and its size ("TOP BAR · 44 px · FULL WIDTH")
function panelLabel(p, box) {
    if (p.free) return "PANEL · ANYWHERE · " + Math.round(box.w) + " × " + Math.round(box.h) + " · DRAG ITS ENDS";
    var side = p.edge.toUpperCase() + " PANEL";
    return side + " · " + (p.size || 44) + " px · " + (p.style === "centred" ? "CENTRED" : p.style === "floating" ? "FLOATING" : "FULL LENGTH");
}
function mainLabel(edge, size) { return edge.toUpperCase() + " BAR · " + size + " px · FULL WIDTH"; }
function widgetLabel(name, w) { return name.toUpperCase() + " · ON THE DESKTOP · " + Math.round(w.w) + " × " + Math.round(w.h); }

// ---- the Blocks window: every block, by kind, found by its name
// bar: T.BAR_ITEMS, panel: T.PANEL_ITEMS, widgets: the widget catalog, own: the draft's own blocks
function library(bar, panel, widgets, own) {
    var out = [], seen = {};
    function add(id, name, icon, kind) { if (seen[id]) return; seen[id] = true; out.push({ id: id, name: name, icon: icon || "dot", kind: kind }); }
    bar.forEach(function (b) { if (b.id !== "menu") add(b.id, b.name, b.icon, b.id === "search" || b.id === "menu" ? "launchers" : "bar"); });
    panel.forEach(function (p) {
        if (p.id.indexOf("widget:") === 0) return;
        if (TO_MAIN[p.id] && seen[TO_MAIN[p.id]]) return;
        add(p.id, p.name, p.icon, p.id === "apps" || p.id === "menu" ? "launchers" : p.id === "clock" ? "info" : "bar");
    });
    widgets.forEach(function (w) { add("widget:" + w.type, w.name, w.icon, w.type === "media" || w.type === "photos" ? "media" : "widgets"); });
    for (var k in own) add("own:" + k, own[k].label || (own[k].kind === "spacer" ? "Spacer" : "Your block"), own[k].icon || "dot", "own");
    return out;
}
function find(list, kind, words) {
    var q = (words || "").trim().toLowerCase();
    return list.filter(function (b) {
        if (kind && kind !== "all" && b.kind !== kind) return false;
        return q === "" || b.name.toLowerCase().indexOf(q) >= 0;
    });
}

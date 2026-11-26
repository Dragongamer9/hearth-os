.pragma library
// The desktop editor's rules (DesktopEditor.qml; docs/mockups/checklists/drag-drop.md, board 1), kept apart so they
// can be checked without a screen (test_desktop_edit.py runs them with node). No Qt here.
//
// A draft is what the editor is changing, and nothing is written until Keep this:
//   { main: {start, centre, end, hidden},          the main bar's items (bar.json "items")
//     panels: { <id>: rec },                       the extra panels (bar.json "p:<id>", widget-tools.js panelOf)
//     widgets: { <id>: {type, x, y, w, h} },       the desktop widgets on this screen (widgets.json "w:<id>")
//     own: { <id>: {kind, label, icon, run, text, pic} },    blocks you make (bar.json "b:<id>", items "own:<id>")
//     looks: { <block id>: sparse look },          each block's look (board 2; bar.json "s:<block id>", block-look.js)
//     bar: { ... } }                               the main bar's own settings (board 2; bar.json "bar", barLook)
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
function baseOf(id) { return typeof id === "string" ? id.replace(/#[a-z0-9]{1,24}$/, "") : id; }
function suffixOf(id) { return typeof id === "string" ? id.slice(baseOf(id).length) : ""; }
function widgetType(id) { return typeof id === "string" && id.indexOf("widget:") === 0 ? id.slice(7) : ""; }

// The id this block has in a target, or "" when it can't go there (and why, for the note).
function fit(id, target, widgetInfo) {
    if (!target) return { id: "", why: "" };
    if (target.kind === "main") {
        if (isOwn(id)) return { id: id, why: "" };
        var m = (TO_MAIN[baseOf(id)] || baseOf(id)) + suffixOf(id);
        if (T && T.barItem(m)) return { id: m, why: "" };
        return { id: "", why: "The main bar can't hold this one: put it in a panel or on the desktop" };
    }
    if (target.kind === "panel") {
        if (isOwn(id)) return { id: id, why: "" };
        var p = (TO_PANEL[baseOf(id)] || baseOf(id)) + suffixOf(id);
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
        // Hearth's own blocks are kept hidden (so the bar doesn't put them back); a copy or a panel inside is gone
        if (!isOwn(id) && suffixOf(id) === "" && id.indexOf("panel:") !== 0 && d.main.hidden.indexOf(id) < 0) d.main.hidden.push(id);
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
    // its look goes with it, under the name it has there
    if (d.looks && f.id !== id && d.looks[id] !== undefined) { d.looks[f.id] = d.looks[id]; delete d.looks[id]; }
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
    var bl = base.looks || {}, dl = d.looks || {};
    for (k in dl) if (JSON.stringify(bl[k]) !== JSON.stringify(dl[k])) bar["s:" + k] = dl[k];
    for (k in bl) if (dl[k] === undefined) bar["s:" + k] = null;
    if (JSON.stringify(base.bar || {}) !== JSON.stringify(d.bar || {})) bar.bar = d.bar && Object.keys(d.bar).length ? d.bar : null;
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

// ---------------------------------------------------------------- board 2: one block picked
// The blocks of a bar or a panel, in order ("main" or a panel's id)
function blocksIn(d, where) {
    if (where === "main") return d.main.start.concat(d.main.centre).concat(d.main.end);
    var p = d.panels[where];
    return p ? (p.items || []).slice() : [];
}
// Where a block is ({kind: "main", zone} | {kind: "panel", id}), or null
function whereIs(d, id) {
    var zs = ["start", "centre", "end"];
    for (var i = 0; i < zs.length; i++) if (d.main[zs[i]].indexOf(id) >= 0) return { kind: "main", zone: zs[i] };
    for (var k in d.panels) if ((d.panels[k].items || []).indexOf(id) >= 0) return { kind: "panel", id: k };
    return null;
}
// "· in the top bar", "· in the left panel", "· in a panel anywhere", "· in a panel inside a panel"
function placeName(d, from, mainEdge) {
    if (!from) return "";
    if (from.kind === "main") return "in the " + (mainEdge === "free" ? "main bar (anywhere)" : (mainEdge || "top") + " bar");
    var p = d.panels[from.id];
    if (!p) return "in a panel";
    if (p.inside) return "in a panel inside a panel";
    if (p.place === "free") return "in a panel anywhere";
    return "in the " + (p.edge || "bottom") + " panel";
}
// Where Move to › can take it: [{label, dest}] (dest for moveTo). The desktop only takes widgets.
function moveTargets(d, id, from, mainEdge, widgetInfo) {
    var out = [];
    if (!from || from.kind !== "main") out.push({ label: (mainEdge === "free" ? "Main" : (mainEdge || "top").charAt(0).toUpperCase() + (mainEdge || "top").slice(1)) + " bar", dest: { kind: "main" }, ok: !!fit(id, { kind: "main" }, widgetInfo).id });
    var n = 0;
    for (var k in d.panels) {
        n++;
        if (from && from.kind === "panel" && from.id === k) continue;
        var p = d.panels[k];
        var nm = p.inside ? "Panel " + n + " (inside)" : p.place === "free" ? "Panel " + n + " (anywhere)" : (p.edge || "bottom").charAt(0).toUpperCase() + (p.edge || "bottom").slice(1) + " panel";
        out.push({ label: nm, dest: { kind: "panel", id: k }, ok: !!fit(id, { kind: "panel" }, widgetInfo).id });
    }
    out.push({ label: "Desktop", dest: { kind: "desk" }, ok: !!fit(id, { kind: "desk" }, widgetInfo).id });
    return out;
}
// Move it to the end of another bar or panel, or onto the desktop (at x, y). Returns the new draft or null.
function moveTo(d, id, from, dest, widgetInfo, newId, x, y) {
    var t;
    if (dest.kind === "main") t = { kind: "main", zone: "end", index: 0 };
    else if (dest.kind === "panel") t = { kind: "panel", id: dest.id, index: ((d.panels[dest.id] || {}).items || []).length };
    else t = { kind: "desk", x: x || 200, y: y || 200 };
    return drop(d, id, from, t, widgetInfo, newId);
}
// Duplicate: a copy right after it, with the same look. Your own block gets its own definition; Hearth's a "#key" copy.
function duplicate(draft, id, from, key) {
    var d = copy(draft), nid;
    if (isOwn(id)) {
        var k0 = id.slice(4);
        if (!d.own[k0]) return null;
        d.own[key] = copy(d.own[k0]);
        nid = "own:" + key;
    } else nid = baseOf(id) + "#" + key;
    if (!d.looks) d.looks = {};
    if (d.looks[id] !== undefined) d.looks[nid] = copy(d.looks[id]);
    if (from && from.kind === "main") {
        var z = d.main[from.zone], i = z.indexOf(id);
        if (i < 0) return null;
        z.splice(i + 1, 0, nid);
    } else if (from && from.kind === "panel" && d.panels[from.id]) {
        var it = d.panels[from.id].items, j = it.indexOf(id);
        if (j < 0) return null;
        it.splice(j + 1, 0, nid);
    } else return null;
    return d;
}
// Take it out (the right-click menu): from its bar or panel, and its look with it.
function takeOut(draft, id, from) {
    var d = copy(draft);
    take(d, id, from);
    if (d.looks && (isOwn(id) || suffixOf(id) !== "" || id.indexOf("panel:") === 0)) delete d.looks[id];
    if (id.indexOf("panel:") === 0) delete d.panels[id.slice(6)];
    return d;
}
// A panel inside a bar or a panel: its record (inside: where) and "panel:<key>" in that bar's blocks
function addInner(draft, where, key) {
    var d = copy(draft);
    d.panels[key] = { inside: where === "main" ? "main" : where, items: ["clock"], size: 34, len: "fit", corners: 10 };
    if (where === "main") d.main.end.unshift("panel:" + key);
    else if (d.panels[where]) d.panels[where].items.push("panel:" + key);
    else return null;
    return d;
}
// The main bar's box on the screen (the editor draws it there), from its settings. size: its thickness.
function mainBox(bar, side, sw, sh, size, want) {
    var b = bar || {}, len = b.len || "full", lenV = b.lenV || 0, off = typeof b.offset === "number" ? b.offset : -1;
    function along(total) {
        if (len === "fit") return Math.max(40, Math.min(total, want || total));
        if (len === "pct") return Math.max(40, Math.round(total * Math.max(1, Math.min(100, lenV || 50)) / 100));
        if (len === "px") return Math.max(40, Math.min(total, lenV || 400));
        return total;
    }
    var down = b.edge === "free" ? b.orient === "down" : side === "left" || side === "right";
    var L = along(down ? sh : sw);
    if (b.edge === "free") {
        var w = down ? size : L, h = down ? L : size;
        var fx = typeof b.fx === "number" ? b.fx : 0.1, fy = typeof b.fy === "number" ? b.fy : 0.1;
        return { x: Math.round(fx * Math.max(0, sw - w)), y: Math.round(fy * Math.max(0, sh - h)), w: w, h: h, down: down };
    }
    var start = len === "full" ? 0 : off < 0 ? Math.round(((down ? sh : sw) - L) / 2) : Math.min(off, (down ? sh : sw) - L);
    if (side === "bottom") return { x: start, y: sh - size, w: L, h: size, down: false };
    if (side === "left") return { x: 0, y: start, w: size, h: L, down: true };
    if (side === "right") return { x: sw - size, y: start, w: size, h: L, down: true };
    return { x: start, y: 0, w: L, h: size, down: false };
}
// The main bar's settings with one changed (an empty value takes it out, so the bar is as it was)
function setBar(bar, k, v) {
    var o = copy(bar || {});
    if (v === "" || v === null || v === undefined) delete o[k]; else o[k] = v;
    return o;
}

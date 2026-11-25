// Every Mantel app made of blocks (AppBlocks.qml, AppBlocksFlow.qml): the rules, with nothing of Qt in them, so the
// checks (hosts/linux/test/test_app_blocks.py) run them in node. The saved format is the Rust apps' own
// (hosts/hearth-ui/src/blocks.rs, hearth-layout app.rs): `~/.config/nimbus/apps/<app>.json`, `layout.blocks.<page>`:
// {block: {x, y, w, h, hidden, corners, seeThrough, font, size, accent}}, written only by `hearth-layout app-keep`
// (settings_file: atomic, .bak, merge-on-write). Mantel's pages are named "mantel-<page>" and laid on a 24 × 24 grid
// (its parts are finer than the Rust apps': a 40 px status bar needs a finer grid than 12 rows).
//
// Three ways a page can be:
//   own      nothing kept (or what's kept can't be read or doesn't check): the app exactly as it always was
//   natural  only hidden blocks kept (no place, no size): the app's own layout, with those parts taken out (the rest
//            closes up the way the app's own layout does it)
//   grid     a place or a size kept: every block where it was put on the grid, in its look
.pragma library

var COLS = 24;
var ROWS = 24;
var CORNERS = [null, 8, 16, 28, 0];
var SEE = [0, 15, 35, 60];
var ACCENTS = ["", "#ff7a2f", "#5b8cff", "#2f9e5a", "#b07cff", "#f2c14e", "#e05a6a"];
var FONTS = ["", "Noto Serif", "Noto Sans Mono", "Inter Display"];
var SIZES = ["", "bigger", "smaller"];
var KEYS = ["x", "y", "w", "h", "hidden", "corners", "seeThrough", "font", "size", "accent"];
var GEO = ["x", "y", "w", "h"];

function isObj(v) { return v !== null && typeof v === "object" && !Array.isArray(v); }
function uint(v) { return typeof v === "number" && isFinite(v) && v >= 0 && Math.floor(v) === v; }
function copy(v) { return v === undefined ? undefined : JSON.parse(JSON.stringify(v)); }

// One block as the app has it.
function own(d) {
    return { id: d.id, title: d.title || d.id, x: d.x, y: d.y, w: d.w, h: d.h, hidden: false,
             corners: null, seeThrough: 0, font: "", size: "", accent: "" };
}

// One kept entry read onto its block, or null when it doesn't check (the page then stays the app's own).
function readOne(d, v, cols, rows) {
    if (!isObj(v)) return null;
    for (var k in v) if (KEYS.indexOf(k) < 0) return null;
    var p = own(d);
    for (var i = 0; i < GEO.length; i++) {
        var g = GEO[i];
        if (v[g] === undefined || v[g] === null) continue;
        if (!uint(v[g])) return null;
        p[g] = v[g];
    }
    if (p.w < 1 || p.h < 1 || p.x + p.w > cols || p.y + p.h > rows) return null;
    if (v.hidden !== undefined && v.hidden !== null) { if (typeof v.hidden !== "boolean") return null; p.hidden = v.hidden; }
    if (v.corners !== undefined && v.corners !== null) { if (!uint(v.corners) || v.corners > 48) return null; p.corners = v.corners; }
    if (v.seeThrough !== undefined && v.seeThrough !== null) { if (!uint(v.seeThrough) || v.seeThrough > 90) return null; p.seeThrough = v.seeThrough; }
    if (v.font !== undefined && v.font !== null) { if (FONTS.indexOf(v.font) < 0) return null; p.font = v.font; }
    if (v.size !== undefined && v.size !== null) { if (SIZES.indexOf(v.size) < 0) return null; p.size = v.size; }
    if (v.accent !== undefined && v.accent !== null) {
        if (typeof v.accent !== "string" || (v.accent !== "" && !/^#[0-9a-fA-F]{6}$/.test(v.accent))) return null;
        p.accent = v.accent.toLowerCase();
    }
    return p;
}

function hasGeo(v) { if (!isObj(v)) return false; for (var i = 0; i < GEO.length; i++) if (v[GEO[i]] !== undefined && v[GEO[i]] !== null) return true; return false; }

// Where a page's blocks go from what's kept for it (`saved`: layout.blocks.<page>) and the app's knobs (`knobs`:
// layout; a knob named like a block with "hidden" or "off" hides it, as Describe it can say for Files' sidebar,
// preview and status bar). An id the app no longer has is passed over; a block the page doesn't name keeps its place.
function resolve(defs, saved, knobs, cols, rows) {
    cols = cols || COLS; rows = rows || ROWS;
    var out = { mode: "own", placed: defs.map(own) };
    var knobHidden = {};
    if (isObj(knobs)) for (var i = 0; i < defs.length; i++) { var kv = knobs[defs[i].id]; if (kv === "hidden" || kv === "off") knobHidden[defs[i].id] = true; }
    var named = isObj(saved) ? saved : {};
    var grid = false, any = false;
    var placed = [];
    for (var j = 0; j < defs.length; j++) {
        var d = defs[j];
        var v = named[d.id];
        var p = own(d);
        if (v !== undefined) {
            p = readOne(d, v, cols, rows);
            if (!p) return out;
            if (hasGeo(v)) grid = true;
            any = true;
        }
        if (knobHidden[d.id]) { p.hidden = true; any = true; }
        placed.push(p);
    }
    if (!any) return out;
    if (placed.every(function (p) { return p.hidden; })) return out;
    if (!grid) {
        if (!placed.some(function (p) { return p.hidden; })) return out;
        // only what's hidden counts: places and looks belong to a page laid on the grid
        return { mode: "natural", placed: placed.map(function (p, k) { var o = own(defs[k]); o.hidden = p.hidden; return o; }) };
    }
    return { mode: "grid", placed: placed };
}

// A page's blocks as they're kept. Natural: only what's hidden (every block named, so places kept before are gone);
// grid: every block's place, and only what differs from its own look.
function toJson(placed, natural) {
    var m = {};
    for (var i = 0; i < placed.length; i++) {
        var p = placed[i];
        var o = {};
        if (!natural) { o.x = p.x; o.y = p.y; o.w = p.w; o.h = p.h; }
        if (p.hidden) o.hidden = true;
        if (!natural) {
            if (p.corners !== null && p.corners !== undefined) o.corners = p.corners;
            if (p.seeThrough > 0) o.seeThrough = p.seeThrough;
            if (p.font) o.font = p.font;
            if (p.size) o.size = p.size;
            if (p.accent) o.accent = p.accent;
        }
        m[p.id] = o;
    }
    return m;
}

function sameGeo(a, b) {
    if (a.length !== b.length) return false;
    for (var i = 0; i < a.length; i++) if (a[i].x !== b[i].x || a[i].y !== b[i].y || a[i].w !== b[i].w || a[i].h !== b[i].h) return false;
    return true;
}
function plainLook(p) { return (p.corners === null || p.corners === undefined) && !p.seeThrough && !p.font && !p.size && !p.accent; }

// What Edit layout keeps for a page: nothing ({}) when it's the app's own again; only the hidden parts when nothing
// was moved, sized or restyled (the app's own layout closes up round them); else the whole page on the grid.
function pageJson(draft, start, startMode) {
    var anyHidden = draft.some(function (p) { return p.hidden; });
    var moved = startMode === "grid" || !sameGeo(draft, start) || !draft.every(plainLook);
    if (!moved) return anyHidden ? toJson(draft, true) : {};
    return toJson(draft, false);
}

function overlap(a, b) {
    var w = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x);
    var h = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
    return w > 0 && h > 0 ? w * h : 0;
}

// Where the blocks are drawn: a block that's hidden or has nothing to show right now (the details pane closed) gives
// its place to the neighbour beside it that fits along that side, so nothing is left empty (live, never saved).
function reflow(placed, shown) {
    var r = placed.map(function (p) { return { id: p.id, x: p.x, y: p.y, w: p.w, h: p.h }; });
    var gone = function (i) { return placed[i].hidden || (shown && shown[placed[i].id] === false); };
    for (var i = 0; i < r.length; i++) {
        if (!gone(i)) continue;
        var b = r[i];
        // a place another part already covers (a part kept over it) isn't given again: nothing ends up on top
        var covered = false;
        for (var k = 0; k < r.length; k++) if (k !== i && !gone(k) && overlap(r[k], b)) { covered = true; break; }
        if (covered) continue;
        var sides = [
            function (a) { return a.x + a.w === b.x && a.y >= b.y && a.y + a.h <= b.y + b.h; },
            function (a) { return a.x === b.x + b.w && a.y >= b.y && a.y + a.h <= b.y + b.h; },
            function (a) { return a.y + a.h === b.y && a.x >= b.x && a.x + a.w <= b.x + b.w; },
            function (a) { return a.y === b.y + b.h && a.x >= b.x && a.x + a.w <= b.x + b.w; }
        ];
        for (var s = 0; s < 4; s++) {
            var found = false;
            for (var j = 0; j < r.length; j++) {
                if (j === i || gone(j) || !sides[s](r[j])) continue;
                found = true;
                var a = r[j];
                if (s === 0) a.w += b.w;
                else if (s === 1) { a.x = b.x; a.w += b.w; }
                else if (s === 2) a.h += b.h;
                else { a.y = b.y; a.h += b.h; }
            }
            if (found) break;
        }
    }
    return r;
}

// A part inside another (the status bar inside the folder's column) or put beside one: the bigger one gives up the
// side the smaller one sits on, so Edit layout starts with nothing on top of anything.
function carve(placed) {
    var r = placed.map(function (p) { return Object.assign({}, p); });
    for (var i = 0; i < r.length; i++) {
        for (var j = 0; j < r.length; j++) {
            var a = r[i], b = r[j];
            if (i === j || !overlap(a, b) || a.w * a.h <= b.w * b.h) continue;
            if (b.x <= a.x && b.x + b.w >= a.x + a.w) {              // b spans a's width: top or bottom
                if (b.y + b.h >= a.y + a.h && b.y > a.y) a.h = b.y - a.y;
                else if (b.y <= a.y && b.y + b.h < a.y + a.h) { a.h = a.y + a.h - (b.y + b.h); a.y = b.y + b.h; }
            } else if (b.y <= a.y && b.y + b.h >= a.y + a.h) {       // b spans a's height: left or right
                if (b.x + b.w >= a.x + a.w && b.x > a.x) a.w = b.x - a.x;
                else if (b.x <= a.x && b.x + b.w < a.x + a.w) { a.w = a.x + a.w - (b.x + b.w); a.x = b.x + b.w; }
            }
        }
    }
    return r;
}

// A block that isn't showing when Edit layout opens (the details pane, split view) gets the side of the blocks it
// opens beside (def.beside: their ids, def.size: its width in columns), taken from them.
function besides(placed, defs, shown) {
    var r = placed.map(function (p) { return Object.assign({}, p); });
    for (var i = defs.length - 1; i >= 0; i--) {
        var d = defs[i];
        if (!d.beside || shown[d.id] !== false) continue;
        var near = r.filter(function (p) { return d.beside.indexOf(p.id) >= 0; });
        if (!near.length) continue;
        var x2 = Math.max.apply(null, near.map(function (p) { return p.x + p.w; }));
        var y1 = Math.min.apply(null, near.map(function (p) { return p.y; }));
        var y2 = Math.max.apply(null, near.map(function (p) { return p.y + p.h; }));
        var size = Math.max(1, d.size || 5);
        var me = r[i];
        me.x = Math.max(0, x2 - size); me.w = x2 - me.x; me.y = y1; me.h = Math.max(1, y2 - y1);
        for (var k = 0; k < near.length; k++) if (near[k].x + near[k].w > me.x && near[k].w > size) near[k].w = me.x - near[k].x;
    }
    return r;
}

function clamp(v, lo, hi) { return Math.max(lo, Math.min(hi, v)); }

// A block moved by (dx, dy) cells, staying whole and on the grid.
function moved(p, dx, dy, cols, rows) {
    var n = Object.assign({}, p);
    n.x = clamp(p.x + dx, 0, cols - p.w);
    n.y = clamp(p.y + dy, 0, rows - p.h);
    return n;
}

// A block sized by dragging edges (e: {l, r, t, b} which edges move) by (dx, dy) cells; the blocks beside a moving
// edge follow it (they share that line: the sidebar's edge pulls the folder's), none smaller than one cell.
function sized(list, id, e, dx, dy, cols, rows) {
    var r = list.map(function (p) { return Object.assign({}, p); });
    var i = r.findIndex(function (p) { return p.id === id; });
    if (i < 0) return r;
    var s = list[i], p = r[i];
    var follow = function (line, nl, vertical) {
        // the others whose opposite edge sits on `line`, beside this block
        for (var k = 0; k < r.length; k++) {
            if (k === i || r[k].hidden) continue;
            var q = r[k], o = list[k];
            if (vertical) {
                if (Math.min(o.y + o.h, s.y + s.h) <= Math.max(o.y, s.y)) continue;
                if (o.x === line && nl < o.x + o.w) { q.x = nl; q.w = o.x + o.w - nl; }
                else if (o.x + o.w === line && nl > o.x) q.w = nl - o.x;
            } else {
                if (Math.min(o.x + o.w, s.x + s.w) <= Math.max(o.x, s.x)) continue;
                if (o.y === line && nl < o.y + o.h) { q.y = nl; q.h = o.y + o.h - nl; }
                else if (o.y + o.h === line && nl > o.y) q.h = nl - o.y;
            }
        }
    };
    if (e.r) { var nr = clamp(s.x + s.w + dx, s.x + 1, cols); p.w = nr - s.x; follow(s.x + s.w, nr, true); }
    if (e.l) { var nl = clamp(s.x + dx, 0, s.x + s.w - 1); p.x = nl; p.w = s.x + s.w - nl; follow(s.x, nl, true); }
    if (e.b) { var nb = clamp(s.y + s.h + dy, s.y + 1, rows); p.h = nb - s.y; follow(s.y + s.h, nb, false); }
    if (e.t) { var nt = clamp(s.y + dy, 0, s.y + s.h - 1); p.y = nt; p.h = s.y + s.h - nt; follow(s.y, nt, false); }
    return r;
}

// Let go over another block (more than half of the smaller one covered): the two change places.
function dropped(list, id, start) {
    var r = list.map(function (p) { return Object.assign({}, p); });
    var i = r.findIndex(function (p) { return p.id === id; });
    if (i < 0) return r;
    var me = r[i], from = start[i];
    var best = -1, most = 0;
    for (var k = 0; k < r.length; k++) {
        if (k === i || r[k].hidden) continue;
        var o = overlap(me, start[k]);
        if (o * 2 >= Math.min(me.w * me.h, start[k].w * start[k].h) && o > most) { most = o; best = k; }
    }
    if (best < 0) return r;
    var t = start[best];
    me.x = t.x; me.y = t.y; me.w = t.w; me.h = t.h;
    r[best].x = from.x; r[best].y = from.y; r[best].w = from.w; r[best].h = from.h;
    return r;
}
// Which block a drop would change places with (for the outline while dragging), or "".
function swapWith(list, id, start) {
    var d = dropped(list, id, start);
    for (var k = 0; k < d.length; k++) if (d[k].id !== id && (d[k].x !== start[k].x || d[k].y !== start[k].y || d[k].w !== start[k].w || d[k].h !== start[k].h)) return d[k].id;
    return "";
}

function step(list, v) { var i = list.indexOf(v); return list[(i + 1) % list.length]; }
// The look with one part stepped on to its next value (round to the block's own again).
function nextLook(p, part) {
    var n = Object.assign({}, p);
    if (part === "corners") n.corners = step(CORNERS, p.corners === undefined ? null : p.corners);
    else if (part === "see") n.seeThrough = step(SEE, p.seeThrough || 0);
    else if (part === "accent") n.accent = step(ACCENTS, ACCENTS.indexOf(p.accent || "") >= 0 ? (p.accent || "") : "");
    return n;
}
function lookLabel(p, part) {
    if (part === "corners") return "Corners: " + (p.corners === null || p.corners === undefined ? "its own" : p.corners === 0 ? "square" : p.corners);
    if (part === "see") return "See-through: " + (p.seeThrough ? p.seeThrough + "%" : "no");
    return "Colour: " + (p.accent ? p.accent : "the look's");
}

// What hearth-layout is told of the app's blocks (HEARTH_APP_BLOCKS: what your AI may move, and the check).
function catalog(pages, cols, rows) {
    return { cols: cols || COLS, rows: rows || ROWS, pages: pages.map(function (pg) {
        return { id: pg.id, title: pg.title, blocks: pg.blocks.map(function (b) { return { id: b.id, title: b.title, x: b.x, y: b.y, w: b.w, h: b.h }; }) };
    }) };
}

// A menu as it was arranged (zone "menu:<kind>": the ids in the order wanted, "-id" left out of this menu): those
// first, a line, then the rest as they were. Edit layout's own items ("ab-…") always stay: they're the way back.
function arrangeMenu(items, order) {
    if (!Array.isArray(order) || !order.length) return items;
    var hidden = {}, want = [];
    for (var i = 0; i < order.length; i++) {
        var o = String(order[i]);
        if (o.charAt(0) === "-") hidden[o.slice(1)] = true; else want.push(o);
    }
    var rest = items.filter(function (it) { return it.sep || !it.id || String(it.id).indexOf("ab-") === 0 || !hidden[it.id]; });
    var out = [];
    for (var k = 0; k < want.length; k++) {
        var at = rest.findIndex(function (it) { return !it.sep && it.id === want[k]; });
        if (at >= 0) out.push(rest.splice(at, 1)[0]);
    }
    if (out.length && rest.length) out.push({ sep: true });
    return out.concat(rest);
}
// A menu's order for keeping, from its items as they're shown now (ids), with the ones left out.
function menuZone(ids, hidden) {
    return ids.filter(function (id) { return !!id && !hidden[id]; }).concat(Object.keys(hidden).filter(function (id) { return hidden[id]; }).map(function (id) { return "-" + id; }));
}

// A name for a section (Settings' pages: its heading) usable as a block id.
function slug(s) {
    var t = String(s || "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
    return (t || "part").slice(0, 40);
}

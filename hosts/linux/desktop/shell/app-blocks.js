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

// ==================================================================== board 3: the same in every app (drag and drop)
// docs/mockups/drag-drop/drag-drop-3.png, checklist docs/mockups/checklists/drag-drop.md part 3. While editing, the
// parts are on the window in pixels (anywhere, any size: not locked to its sides); blocks from the app's own Blocks
// window go into any bar of the app (a lit line where they land) or anywhere on the window; panes and the sidebar are
// sized by their edges (a guide and a note); every part and block has its own look (Customize: board 2's side panel).
// Nothing is written until Keep this (or Try it, which can be taken back). What's kept beside layout.blocks (which
// still holds every part's cell, for Describe it and the Rust apps' format) is one key per page in
// ~/.config/nimbus/app-looks/<app>.json, written only by AppBlocks through SettingsWriter (hearth_settings_io merge:
// atomic, .bak, merge-on-write):
//   "mantel-<page>": { v: 1, rects: {part: {x, y, w, h} as parts of the window, 0-1}, looks: {part or block: look},
//                      z: [parts and blocks, back to front], added: [block] }
//   block: { id: "b-<word>", kind: "action" | "button" | "count" | "words" | "spacer" | "pane", ref (the action, count
//            or pane it is), label, run (a button you made: your own command), in (the bar it's in, or "" on the
//            window), at (how many of the bar's own items come before it), x, y, w, h (on the window, 0-1) }
var PX_GRID = 8;
var LINE_TOL = 6;
var MIN_PX = 24;

function isColour(c) { return typeof c === "string" && (c === "" || /^#[0-9a-fA-F]{6}$/.test(c)); }
function intIn(v, lo, hi) { return typeof v === "number" && isFinite(v) && Math.floor(v) === v && v >= lo && v <= hi; }
function oneOf(v, list) { return typeof v === "string" && list.indexOf(v) >= 0; }
function plainText(s, n) { return typeof s === "string" && s.length <= n && !/[\u0000-\u001f]/.test(s); }

var W_MODES = ["", "fit", "fixed", "fill"];
var EDGES = ["", "none", "line", "glow"];
var SHADOWS = ["", "none", "soft", "deep"];
var WEIGHTS = ["", "light", "medium", "bold"];
var SHOW = ["", "icon", "words", "both"];
var HOVER = ["", "same", "lighter", "accent", "own"];
var PRESSED = ["", "same", "darker", "own"];
var SHOWS = ["", "own", "count", "words", "clock", "space"];
var DOES = ["", "nothing", "action", "open", "run", "panel"];
var WHEN = ["", "always", "hover", "something", "pages"];
var APPEAR = ["", "none", "fade", "slide", "pop"];
var HOW = ["press", "right", "middle", "scroll", "hover"];

// A look as it's kept, checked: what doesn't check is left out (that one setting is its own again), never guessed.
function checkLook(v) {
    if (!isObj(v)) return null;
    var o = {};
    var ok = {
        wMode: function (x) { return oneOf(x, W_MODES); }, wPx: function (x) { return intIn(x, 0, 6000); },
        hMode: function (x) { return oneOf(x, W_MODES); }, hPx: function (x) { return intIn(x, 0, 6000); },
        m: function (x) { return Array.isArray(x) && x.length === 4 && x.every(function (n) { return intIn(n, 0, 200); }); },
        pad: function (x) { return intIn(x, 0, 96); },
        fill: isColour, see: function (x) { return intIn(x, 0, 90); }, blur: function (x) { return typeof x === "boolean"; },
        corners: function (x) { return intIn(x, 0, 48); }, edge: function (x) { return oneOf(x, EDGES); },
        edgeColor: isColour, edgeW: function (x) { return intIn(x, 1, 12); }, shadow: function (x) { return oneOf(x, SHADOWS); },
        icon: function (x) { return typeof x === "string" && /^[a-z0-9-]{0,40}$/.test(x); }, iconColor: isColour,
        iconSize: function (x) { return intIn(x, 8, 96); },
        words: function (x) { return plainText(x, 160); }, font: function (x) { return plainText(x, 80); },
        fontSize: function (x) { return intIn(x, 8, 72); }, weight: function (x) { return oneOf(x, WEIGHTS); },
        show: function (x) { return oneOf(x, SHOW); }, textColor: isColour,
        hover: function (x) { return oneOf(x, HOVER); }, hoverColor: isColour,
        pressed: function (x) { return oneOf(x, PRESSED); }, pressedColor: isColour,
        shows: function (x) { return oneOf(x, SHOWS); }, showsRef: function (x) { return plainText(x, 300); },
        does: function (x) {
            if (!isObj(x)) return false;
            for (var k in x) {
                if (HOW.indexOf(k) < 0 || !isObj(x[k])) return false;
                if (!oneOf(x[k].k || "", DOES) || !plainText(x[k].a === undefined ? "" : x[k].a, 600)) return false;
            }
            return true;
        },
        when: function (x) { return oneOf(x, WHEN); },
        pages: function (x) { return Array.isArray(x) && x.length <= 40 && x.every(function (p) { return typeof p === "string" && /^[a-z0-9-]{1,48}$/.test(p); }); },
        appear: function (x) { return oneOf(x, APPEAR); }, bounce: function (x) { return typeof x === "boolean"; }
    };
    for (var k in v) {
        if (!ok[k] || v[k] === null || v[k] === undefined) continue;
        if (ok[k](v[k])) o[k] = copy(v[k]);
    }
    return o;
}
function lookEmpty(l) { if (!isObj(l)) return true; for (var k in l) if (l[k] !== "" && l[k] !== undefined) return false; return true; }
// Only what's different from the part's own look is kept.
function trimLook(l) {
    var o = {};
    if (!isObj(l)) return o;
    for (var k in l) {
        var v = l[k];
        if (v === "" || v === undefined || v === null) continue;
        if (k === "does") { var d = {}; for (var h in v) if (v[h] && v[h].k) d[h] = { k: v[h].k, a: v[h].a || "" }; if (Object.keys(d).length) o.does = d; continue; }
        if (k === "m" && v.every(function (n) { return n === 0; })) continue;
        if (k === "pages" && !v.length) continue;
        o[k] = copy(v);
    }
    return checkLook(o) || {};
}

function frac(n) { return Math.round(n * 10000) / 10000; }
function okRect(r) {
    return isObj(r) && ["x", "y", "w", "h"].every(function (k) { return typeof r[k] === "number" && isFinite(r[k]) && r[k] >= 0 && r[k] <= 1; })
        && r.w > 0 && r.h > 0 && r.x + r.w <= 1.0001 && r.y + r.h <= 1.0001;
}
function okBlock(b) {
    if (!isObj(b) || typeof b.id !== "string" || !/^b-[a-z0-9]{1,24}$/.test(b.id)) return false;
    if (["action", "button", "count", "words", "spacer", "pane"].indexOf(b.kind) < 0) return false;
    if (b.ref !== undefined && !plainText(b.ref, 60)) return false;
    if (b.label !== undefined && !plainText(b.label, 160)) return false;
    if (b.run !== undefined && !plainText(b.run, 600)) return false;
    if (b.in !== undefined && !(typeof b.in === "string" && /^[a-z0-9-]{0,48}$/.test(b.in))) return false;
    if (b.at !== undefined && !intIn(b.at, 0, 400)) return false;
    if (!b.in && !okRect({ x: b.x, y: b.y, w: b.w, h: b.h })) return false;
    return true;
}

// Your own blocks kept on a page's shelf (Customize › Save as my own block…): checked like placed ones, without a place.
function readMine(v) {
    if (!isObj(v) || !Array.isArray(v.mine)) return [];
    return copy(v.mine.filter(function (b) { return isObj(b) && okBlock(Object.assign({}, b, { in: "shelf", x: undefined, y: undefined, w: undefined, h: undefined })); }).slice(0, 40)
        .map(function (b) { var o = Object.assign({}, b); delete o.in; delete o.at; delete o.x; delete o.y; delete o.w; delete o.h; return o; }));
}
// What's kept for a page beside its cells, read and checked; {} when there's nothing (or it can't be read: the page is
// then the app's own look, and nothing is written over it until you keep something).
function readExtras(v, ids) {
    var out = { rects: {}, looks: {}, z: [], added: [] };
    if (!isObj(v)) return out;
    var known = {};
    (ids || []).forEach(function (i) { known[i] = true; });
    var added = Array.isArray(v.added) ? v.added.filter(okBlock).slice(0, 60) : [];
    added.forEach(function (b) { known[b.id] = true; });
    out.added = copy(added);
    if (isObj(v.rects)) for (var k in v.rects) if (known[k] && okRect(v.rects[k])) out.rects[k] = copy(v.rects[k]);
    if (isObj(v.looks)) for (var j in v.looks) if (known[j]) { var l = checkLook(v.looks[j]); if (l && !lookEmpty(l)) out.looks[j] = l; }
    if (Array.isArray(v.z)) out.z = v.z.filter(function (x) { return known[x]; });
    return out;
}
function extrasEmpty(x) {
    return !x || (Object.keys(x.rects || {}).length === 0 && Object.keys(x.looks || {}).length === 0 && (x.added || []).length === 0 && (x.z || []).length === 0);
}
// What's written for a page (null takes the key out: the page's own again).
function extrasJson(x) {
    if (extrasEmpty(x)) return null;
    var o = { v: 1 };
    if (Object.keys(x.rects || {}).length) o.rects = copy(x.rects);
    var looks = {};
    for (var k in (x.looks || {})) { var t = trimLook(x.looks[k]); if (!lookEmpty(t)) looks[k] = t; }
    if (Object.keys(looks).length) o.looks = looks;
    if (x.z && x.z.length) o.z = x.z.slice();
    if (x.added && x.added.length) o.added = copy(x.added);
    return Object.keys(o).length > 1 ? o : null;
}

// ---- cells and pixels
function cellPx(p, W, H, cols, rows) {
    var cw = W / (cols || COLS), ch = H / (rows || ROWS);
    var x = Math.round(p.x * cw), y = Math.round(p.y * ch);
    return { x: x, y: y, w: Math.round((p.x + p.w) * cw) - x, h: Math.round((p.y + p.h) * ch) - y };
}
function pxCell(r, W, H, cols, rows) {
    cols = cols || COLS; rows = rows || ROWS;
    var cw = W / cols, ch = H / rows;
    var x = clamp(Math.round(r.x / cw), 0, cols - 1), y = clamp(Math.round(r.y / ch), 0, rows - 1);
    var x2 = clamp(Math.round((r.x + r.w) / cw), x + 1, cols), y2 = clamp(Math.round((r.y + r.h) / ch), y + 1, rows);
    return { x: x, y: y, w: x2 - x, h: y2 - y };
}
function toFrac(r, W, H) {
    var x = clamp(r.x / W, 0, 1), y = clamp(r.y / H, 0, 1);
    return { x: frac(x), y: frac(y), w: frac(clamp(r.w / W, 0.0001, 1 - x)), h: frac(clamp(r.h / H, 0.0001, 1 - y)) };
}
function fromFrac(f, W, H) { return { x: Math.round(f.x * W), y: Math.round(f.y * H), w: Math.max(1, Math.round(f.w * W)), h: Math.max(1, Math.round(f.h * H)) }; }

// ---- the label on each part while editing ("COMMAND BAR · 48 px", "SIDEBAR · 216 px · DRAG ITS EDGE")
function shortName(t) { return String(t || "").replace(/^(the|a|an)\s+/i, "").replace(/['’]s own\b/i, "").trim(); }
function partKind(r, W, H) {
    if (r.h <= 80 && r.w >= r.h * 3) return "bar";
    if (r.w <= W * 0.34 && r.h >= r.w * 1.4) return "side";
    return "pane";
}
function partLabel(title, r, W, H) {
    var n = shortName(title).toUpperCase();
    var k = partKind(r, W, H);
    if (k === "bar") return n + " · " + Math.round(r.h) + " px";
    if (k === "side") return n + " · " + Math.round(r.w) + " px · DRAG ITS EDGE";
    return n + " · " + Math.round(r.w) + " × " + Math.round(r.h);
}

// ---- lining up: a value pulled onto the nearest line (another part's edge, or a mark such as Search) within LINE_TOL
// lines: [{v, name}]. Returns {v, name} (name "" when nothing was near).
function lineUp(v, lines, tol) {
    tol = tol === undefined ? LINE_TOL : tol;
    var best = null, d = tol + 1;
    for (var i = 0; i < lines.length; i++) { var dd = Math.abs(lines[i].v - v); if (dd < d) { d = dd; best = lines[i]; } }
    return best ? { v: best.v, name: best.name || "" } : { v: v, name: "" };
}
function snapTo(v, grid) { return grid > 0 ? Math.round(v / grid) * grid : v; }
// The lines a part can line up with: the other parts' edges and the app's marks ({name, x, y, w, h}).
function linesFor(list, skip, marks, vertical) {
    var out = [];
    list.forEach(function (p) {
        if (p.id === skip || p.hidden) return;
        var nm = p.name || p.title || p.id;
        if (vertical) { out.push({ v: p.x, name: nm }); out.push({ v: p.x + p.w, name: nm }); }
        else { out.push({ v: p.y, name: nm }); out.push({ v: p.y + p.h, name: nm }); }
    });
    (marks || []).forEach(function (m) {
        if (vertical) { out.push({ v: m.x, name: m.name }); out.push({ v: m.x + m.w, name: m.name }); }
        else { out.push({ v: m.y, name: m.name }); out.push({ v: m.y + m.h, name: m.name }); }
    });
    return out;
}

// ---- moving a part anywhere: by (dx, dy) px, snapped to the grid, lined up with the others (the guides)
// Returns {list, guides: [{x}|{y}], note}.
function movePx(list, id, dx, dy, W, H, opt) {
    opt = opt || {};
    var r = list.map(function (p) { return Object.assign({}, p); });
    var i = r.findIndex(function (p) { return p.id === id; });
    if (i < 0) return { list: r, guides: [], note: "" };
    var s = list[i], p = r[i];
    var x = s.x + dx, y = s.y + dy, gx = null, gy = null, note = "";
    if (opt.guides) {
        var vl = linesFor(list, id, opt.marks, true), hl = linesFor(list, id, opt.marks, false);
        var a = lineUp(x, vl), b = lineUp(x + s.w, vl);
        if (a.name && (!b.name || Math.abs(a.v - x) <= Math.abs(b.v - (x + s.w)))) { x = a.v; gx = a.v; note = "Lined up with " + a.name; }
        else if (b.name) { x = b.v - s.w; gx = b.v; note = "Lined up with " + b.name; }
        var c = lineUp(y, hl), e = lineUp(y + s.h, hl);
        if (c.name && (!e.name || Math.abs(c.v - y) <= Math.abs(e.v - (y + s.h)))) { y = c.v; gy = c.v; if (!note) note = "Lined up with " + c.name; }
        else if (e.name) { y = e.v - s.h; gy = e.v; if (!note) note = "Lined up with " + e.name; }
    }
    if (opt.grid > 0) { if (gx === null) x = snapTo(x, opt.grid); if (gy === null) y = snapTo(y, opt.grid); }
    p.x = clamp(Math.round(x), 0, Math.max(0, W - s.w));
    p.y = clamp(Math.round(y), 0, Math.max(0, H - s.h));
    var g = [];
    if (gx !== null) g.push({ x: gx });
    if (gy !== null) g.push({ y: gy });
    return { list: r, guides: g, note: note };
}

// ---- sizing a part by an edge (e: {l, r, t, b}), in px: the parts beside that edge follow it (they share that line),
// none smaller than MIN_PX; the edge snaps to the grid and lines up with the others and the marks.
// Returns {list, guides, note: "Preview: 514 px · lined up with Search"}.
function sizePx(list, id, e, dx, dy, W, H, opt) {
    opt = opt || {};
    var r = list.map(function (p) { return Object.assign({}, p); });
    var i = r.findIndex(function (p) { return p.id === id; });
    if (i < 0) return { list: r, guides: [], note: "" };
    var s = list[i], p = r[i], eps = 2, guides = [], lined = "";
    var place = function (v, vertical, lo, hi) {
        var n = v;
        if (opt.guides) {
            var l = lineUp(v, linesFor(list, id, opt.marks, vertical).filter(function (q) { return q.v > lo && q.v < hi; }));
            if (l.name) { n = l.v; lined = l.name; guides.push(vertical ? { x: n } : { y: n }); return n; }
        }
        if (opt.grid > 0) n = snapTo(n, opt.grid);
        return clamp(Math.round(n), lo, hi);
    };
    var follow = function (line, nl, vertical) {
        for (var k = 0; k < r.length; k++) {
            if (k === i || r[k].hidden) continue;
            var q = r[k], o = list[k];
            if (vertical) {
                if (Math.min(o.y + o.h, s.y + s.h) <= Math.max(o.y, s.y)) continue;
                if (Math.abs(o.x - line) <= eps && nl < o.x + o.w - MIN_PX) { q.x = nl; q.w = o.x + o.w - nl; }
                else if (Math.abs(o.x + o.w - line) <= eps && nl > o.x + MIN_PX) q.w = nl - o.x;
            } else {
                if (Math.min(o.x + o.w, s.x + s.w) <= Math.max(o.x, s.x)) continue;
                if (Math.abs(o.y - line) <= eps && nl < o.y + o.h - MIN_PX) { q.y = nl; q.h = o.y + o.h - nl; }
                else if (Math.abs(o.y + o.h - line) <= eps && nl > o.y + MIN_PX) q.h = nl - o.y;
            }
        }
    };
    if (e.r) { var nr = place(s.x + s.w + dx, true, s.x + MIN_PX, W); p.w = nr - s.x; follow(s.x + s.w, nr, true); }
    if (e.l) { var nl = place(s.x + dx, true, 0, s.x + s.w - MIN_PX); p.x = nl; p.w = s.x + s.w - nl; follow(s.x, nl, true); }
    if (e.b) { var nb = place(s.y + s.h + dy, false, s.y + MIN_PX, H); p.h = nb - s.y; follow(s.y + s.h, nb, false); }
    if (e.t) { var nt = place(s.y + dy, false, 0, s.y + s.h - MIN_PX); p.y = nt; p.h = s.y + s.h - nt; follow(s.y, nt, false); }
    var size = (e.l || e.r) && !(e.t || e.b) ? Math.round(p.w) + " px" : (e.t || e.b) && !(e.l || e.r) ? Math.round(p.h) + " px" : Math.round(p.w) + " × " + Math.round(p.h);
    var note = shortName(opt.title || s.title || s.id) + ": " + size + (lined ? " · lined up with " + lined : "");
    return { list: r, guides: guides, note: note.charAt(0).toUpperCase() + note.slice(1) };
}

// Let go over another part (more than half of the smaller one covered): the two change places (in px).
function swapPx(list, id, start) {
    var d = dropped(list, id, start);
    return d;
}

// ---- where the parts are drawn at rest: a part that's hidden or has nothing to show gives its place to the one
// beside it (as reflow, with a little give: px rects from different sources don't always meet to the pixel)
function reflowPx(placed, shown, eps) {
    eps = eps === undefined ? 2 : eps;
    var r = placed.map(function (p) { return { id: p.id, x: p.x, y: p.y, w: p.w, h: p.h }; });
    var gone = function (i) { return placed[i].hidden || (shown && shown[placed[i].id] === false); };
    var near = function (a, b) { return Math.abs(a - b) <= eps; };
    for (var i = 0; i < r.length; i++) {
        if (!gone(i)) continue;
        var b = r[i], covered = false;
        for (var k = 0; k < r.length; k++) if (k !== i && !gone(k) && overlap(r[k], b) > eps * Math.max(b.w, b.h)) { covered = true; break; }
        if (covered) continue;
        var sides = [
            function (a) { return near(a.x + a.w, b.x) && a.y >= b.y - eps && a.y + a.h <= b.y + b.h + eps; },
            function (a) { return near(a.x, b.x + b.w) && a.y >= b.y - eps && a.y + a.h <= b.y + b.h + eps; },
            function (a) { return near(a.y + a.h, b.y) && a.x >= b.x - eps && a.x + a.w <= b.x + b.w + eps; },
            function (a) { return near(a.y, b.y + b.h) && a.x >= b.x - eps && a.x + a.w <= b.x + b.w + eps; }
        ];
        for (var s = 0; s < 4; s++) {
            var found = false;
            for (var j = 0; j < r.length; j++) {
                if (j === i || gone(j) || !sides[s](r[j])) continue;
                found = true;
                var a = r[j];
                if (s === 0) a.w = b.x + b.w - a.x;
                else if (s === 1) { a.w = a.x + a.w - b.x; a.x = b.x; }
                else if (s === 2) a.h = b.y + b.h - a.y;
                else { a.h = a.y + a.h - b.y; a.y = b.y; }
            }
            if (found) break;
        }
    }
    return r;
}

// ---- a part's size from its look: Fixed (its px), Fit (what it needs), Fill (out to the next part or the window's edge)
function sizedByLook(r, look, need, others, W, H) {
    var o = { x: r.x, y: r.y, w: r.w, h: r.h };
    if (!look) return o;
    if (look.wMode === "fixed" && look.wPx > 0) o.w = look.wPx;
    else if (look.wMode === "fit" && need && need.w > 0) o.w = need.w;
    else if (look.wMode === "fill") {
        var right = W;
        (others || []).forEach(function (q) { if (q.x >= r.x + r.w - 1 && q.y < r.y + r.h && q.y + q.h > r.y) right = Math.min(right, q.x); });
        o.w = right - r.x;
    }
    if (look.hMode === "fixed" && look.hPx > 0) o.h = look.hPx;
    else if (look.hMode === "fit" && need && need.h > 0) o.h = need.h;
    else if (look.hMode === "fill") {
        var bottom = H;
        (others || []).forEach(function (q) { if (q.y >= r.y + r.h - 1 && q.x < r.x + r.w && q.x + q.w > r.x) bottom = Math.min(bottom, q.y); });
        o.h = bottom - r.y;
    }
    o.w = clamp(Math.round(o.w), MIN_PX, Math.max(MIN_PX, W - o.x));
    o.h = clamp(Math.round(o.h), 12, Math.max(12, H - o.y));
    return o;
}

// ---- When it shows
function showsNow(look, hovered, something, page) {
    if (!look || !look.when || look.when === "always") return true;
    if (look.when === "hover") return !!hovered;
    if (look.when === "something") return !!something;
    if (look.when === "pages") return !look.pages || !look.pages.length || look.pages.indexOf(page) >= 0;
    return true;
}
// Words with values in them: "Downloads · {count}" (values: {count: 3, …}; one not known is left as it was)
function fillWords(t, values) {
    return String(t || "").replace(/\{([a-z][a-z0-9-]*)\}/gi, function (m, k) { return values && values[k] !== undefined && values[k] !== null ? String(values[k]) : m; });
}

// ---- a bar's place for a block: between the bar's items (their centres along it) → the index it's put at
function insertAt(centres, along) {
    for (var i = 0; i < centres.length; i++) if (along < centres[i]) return i;
    return centres.length;
}
// Where a block goes into a bar's own items: the app's items (in their order, `own`: true or false for each child) and
// the blocks wanted there ([{id, at}], `at`: how many of the app's own items come before it). Returns the children's
// order wanted (ids for blocks, numbers for the app's own children by their index).
function barOrder(ownCount, blocks) {
    var out = [], bs = blocks.slice().sort(function (a, b) { return (a.at || 0) - (b.at || 0); }), k = 0;
    for (var i = 0; i <= ownCount; i++) {
        while (k < bs.length && Math.min(bs[k].at || 0, ownCount) === i) out.push(bs[k++].id);
        if (i < ownCount) out.push(i);
    }
    return out;
}
// The drop's note: "Drop: command bar, after Sort"
function dropNote(bar, after, onWindow) {
    if (onWindow) return "Drop: on the window, anywhere";
    return "Drop: " + shortName(bar).toLowerCase() + (after ? ", after " + after : ", at the start");
}

// ---- the app's Blocks window: its actions, panes, counts and what you make, found by name
// actions: [{id, label, icon}], panes: [{id, label, icon}], counts: [{id, label}], own: the page's added button blocks
function library(actions, panes, counts, own) {
    var out = [];
    (actions || []).forEach(function (a) { out.push({ id: "action:" + a.id, name: a.label, icon: a.icon || "dot", kind: "buttons" }); });
    (panes || []).forEach(function (p) { out.push({ id: "pane:" + p.id, name: p.label, icon: p.icon || "grid", kind: "panes" }); });
    (counts || []).forEach(function (c) { out.push({ id: "count:" + c.id, name: c.label, icon: c.icon || "info", kind: "info" }); });
    (own || []).forEach(function (b) { if (b.kind === "button" || b.kind === "words") out.push({ id: "own:" + b.id, name: b.label || "Your block", icon: (b.icon || "rocket"), kind: "own" }); });
    return out;
}
function find(list, kind, words) {
    var q = (words || "").trim().toLowerCase();
    return list.filter(function (b) {
        if (kind && kind !== "all" && b.kind !== kind) return false;
        return q === "" || b.name.toLowerCase().indexOf(q) >= 0;
    });
}
// A new block from the library: what it is (the id "b-…" is given by the caller)
function newBlock(libId, id, label) {
    var c = libId.indexOf(":"), kind = libId.slice(0, c), ref = libId.slice(c + 1);
    if (kind === "action") return { id: id, kind: "action", ref: ref, label: label || "" };
    if (kind === "pane") return { id: id, kind: "pane", ref: ref, label: label || "" };
    if (kind === "count") return { id: id, kind: "count", ref: ref, label: label || "" };
    if (kind === "make") {
        if (ref === "button") return { id: id, kind: "button", label: "My button", run: "" };
        if (ref === "count") return { id: id, kind: "count", ref: "", label: "" };
        if (ref === "words") return { id: id, kind: "words", label: "Your words" };
        return { id: id, kind: "spacer" };
    }
    return null;
}

// ---- Start from: ready-made layouts for any app, from its own parts (W × H px). Returns [{id, name, parts}] where
// parts are [{id, x, y, w, h, hidden}] in px.
function presets(own, W, H) {
    var base = own.map(function (p) { return Object.assign({}, p); });
    var bars = base.filter(function (p) { return !p.hidden && partKind(p, W, H) === "bar"; });
    var big = base.filter(function (p) { return !p.hidden; }).sort(function (a, b) { return b.w * b.h - a.w * a.h; })[0];
    var mirror = base.map(function (p) { var q = Object.assign({}, p); q.x = Math.max(0, W - p.x - p.w); return q; });
    var focus = base.map(function (p) { var q = Object.assign({}, p); if (!(big && p.id === big.id) && bars.indexOf(p) < 0 && !p.hidden) q.hidden = true; return q; });
    if (big) {
        var top = 0, bottom = H;
        bars.forEach(function (b) { if (b.y + b.h <= big.y + 2) top = Math.max(top, b.y + b.h); else if (b.y >= big.y + big.h - 2) bottom = Math.min(bottom, b.y); });
        focus.forEach(function (q) { if (q.id === big.id) { q.x = 0; q.w = W; q.y = top; q.h = Math.max(MIN_PX, bottom - top); } });
    }
    var barsBelow = base.map(function (p) {
        var q = Object.assign({}, p);
        if (bars.indexOf(p) >= 0 && p.y < H / 2) q.y = H - (p.y + p.h);
        else if (bars.indexOf(p) < 0) { var above = bars.filter(function (b) { return b.y < H / 2; }).reduce(function (n, b) { return n + b.h; }, 0); q.y = Math.max(0, p.y - above); }
        return q;
    });
    return [
        { id: "hearth", name: "Hearth", note: "as the app comes", parts: base },
        { id: "mirror", name: "Mirrored", note: "the sides swapped", parts: mirror },
        { id: "focus", name: "Just the content", note: "the bars and the biggest part", parts: focus },
        { id: "bars-below", name: "Bars at the bottom", note: "the top bars moved down", parts: barsBelow }
    ];
}

// ---- copy a look / paste a look (what a part shows and does stays its own)
var LOOK_ONLY = ["fill", "see", "blur", "corners", "edge", "edgeColor", "edgeW", "shadow", "iconColor", "iconSize", "font", "fontSize", "weight", "textColor", "hover", "hoverColor", "pressed", "pressedColor", "m", "pad", "appear", "bounce"];
function lookPart(l) { var o = {}; if (!isObj(l)) return o; LOOK_ONLY.forEach(function (k) { if (l[k] !== undefined && l[k] !== "") o[k] = copy(l[k]); }); return o; }
function pasteLook(onto, look) { var o = Object.assign({}, onto || {}); LOOK_ONLY.forEach(function (k) { delete o[k]; }); return Object.assign(o, lookPart(look)); }

// ---- the side panel's "· in the command bar" and the summary under its tabs
function whereIs(block, parts, appName) {
    if (block && block.in) { var p = (parts || []).find(function (x) { return x.id === block.in; }); return "in " + (p ? shortName(p.title).toLowerCase().replace(/^/, "the ") : "a bar"); }
    if (block && block.kind) return "on " + appName + "'s window";
    return "in " + appName;
}

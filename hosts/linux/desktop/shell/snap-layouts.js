// Snap layouts (SnapLayouts.qml, Super+Z or right-click on a window's Maximize button): the six layouts and their
// zones, in percent of the screen's usable area. Picking a zone presses the labwc shortcut that snaps the window
// there: Super+Left / Super+Right for the halves (Mantel's own snapping), and hidden Super+Ctrl+Shift+F1–F11 keys for
// the others, each bound to SnapToRegion with a <region> of the same name and size in labwc/rc.xml
// (test_shell_extras.py checks they match). No Qt here, so node can check it.
.pragma library

var HIDDEN = ["logo", "ctrl", "shift"];

function z(id, label, x, y, w, h, key, mods) {
    return { id: id, label: label, x: x, y: y, w: w, h: h, key: key, mods: mods || HIDDEN };
}

var LAYOUTS = [
    { id: "halves", label: "Halves", zones: [
        z("left-half", "Left half", 0, 0, 50, 100, "Left", ["logo"]),
        z("right-half", "Right half", 50, 0, 50, 100, "Right", ["logo"])] },
    { id: "two-thirds", label: "Two thirds and a third", zones: [
        z("left-two-thirds", "Left two thirds", 0, 0, 67, 100, "F1"),
        z("right-third", "Right third", 67, 0, 33, 100, "F2")] },
    { id: "thirds", label: "Thirds", zones: [
        z("left-third", "Left third", 0, 0, 33, 100, "F3"),
        z("middle-third", "Middle third", 33, 0, 34, 100, "F4"),
        z("right-third", "Right third", 67, 0, 33, 100, "F2")] },
    { id: "quarters", label: "Quarters", zones: [
        z("top-left", "Top left", 0, 0, 50, 50, "F5"),
        z("top-right", "Top right", 50, 0, 50, 50, "F6"),
        z("bottom-left", "Bottom left", 0, 50, 50, 50, "F7"),
        z("bottom-right", "Bottom right", 50, 50, 50, 50, "F8")] },
    { id: "half-quarters", label: "Half and two quarters", zones: [
        z("left-half", "Left half", 0, 0, 50, 100, "Left", ["logo"]),
        z("top-right", "Top right", 50, 0, 50, 50, "F6"),
        z("bottom-right", "Bottom right", 50, 50, 50, 50, "F8")] },
    { id: "narrow-wide", label: "Narrow, wide, narrow", zones: [
        z("narrow-left", "Left", 0, 0, 25, 100, "F9"),
        z("wide-middle", "Middle", 25, 0, 50, 100, "F10"),
        z("narrow-right", "Right", 75, 0, 25, 100, "F11")] }
];

var COLUMNS = 3;

// Every zone in the flyout, in reading order, with where its middle is (in layout cells), for the D-pad.
function flat() {
    var out = [];
    for (var li = 0; li < LAYOUTS.length; li++) {
        var col = li % COLUMNS, row = Math.floor(li / COLUMNS);
        for (var zi = 0; zi < LAYOUTS[li].zones.length; zi++) {
            var q = LAYOUTS[li].zones[zi];
            out.push({ layout: li, zone: zi, id: q.id, label: q.label,
                       cx: col + (q.x + q.w / 2) / 100, cy: row + (q.y + q.h / 2) / 100 });
        }
    }
    return out;
}

// The zone the D-pad goes to from zone `cur` (an index into flat()): the nearest one that way, keeping to the same
// line when it can. Stays put at the edge.
function step(cur, dir) {
    var all = flat();
    var a = all[Math.max(0, Math.min(all.length - 1, cur || 0))];
    var best = -1, bestScore = Infinity;
    for (var i = 0; i < all.length; i++) {
        var b = all[i];
        var dx = b.cx - a.cx, dy = b.cy - a.cy;
        var ok = dir === "left" ? dx < -0.01 : dir === "right" ? dx > 0.01 : dir === "up" ? dy < -0.01 : dir === "down" ? dy > 0.01 : false;
        if (!ok) continue;
        // Left/Right stay on the same line of layouts, Up/Down go to another line (or within the layout itself), so
        // Down at the bottom of a layout doesn't slide sideways into its neighbour.
        var sameLayout = b.layout === a.layout;
        var sameLine = Math.floor(b.layout / COLUMNS) === Math.floor(a.layout / COLUMNS);
        if (!sameLayout && ((dir === "left" || dir === "right") ? !sameLine : sameLine)) continue;
        var along = dir === "left" || dir === "right" ? Math.abs(dx) : Math.abs(dy);
        var across = dir === "left" || dir === "right" ? Math.abs(dy) : Math.abs(dx);
        var score = along + across * 3;
        if (score < bestScore) { bestScore = score; best = i; }
    }
    return best < 0 ? (cur || 0) : best;
}

// wtype's arguments for a zone's shortcut: the modifiers down, the key, the modifiers up.
function wtypeArgs(q) {
    var args = [], i;
    for (i = 0; i < q.mods.length; i++) args.push("-M", q.mods[i]);
    args.push("-k", q.key);
    for (i = q.mods.length - 1; i >= 0; i--) args.push("-m", q.mods[i]);
    return args;
}

// labwc's name for the same shortcut in rc.xml ("W-C-S-F1", "W-Left").
function labwcKey(q) {
    var m = { logo: "W", ctrl: "C", alt: "A", shift: "S" };
    return q.mods.map(function (x) { return m[x]; }).join("-") + "-" + q.key;
}

function byId(id) {
    for (var li = 0; li < LAYOUTS.length; li++)
        for (var zi = 0; zi < LAYOUTS[li].zones.length; zi++)
            if (LAYOUTS[li].zones[zi].id === id) return LAYOUTS[li].zones[zi];
    return null;
}

pragma Singleton
// The selection: which control has Hearth's focus ring, and moving it with the D-pad, the left stick or the arrow keys.
// Every control is a Focusable (it registers itself here); a move picks the nearest control in that direction among
// those of the open layer (`scope`: "view" for the page, or the layer on top: "qam", "power", "menu", "sheet",
// "search", "launch"), like the web page's spatial navigation. Remembers the selection per page, so Back returns to it.
import QtQuick

QtObject {
    id: nav
    property var items: []
    property Item focused: null
    property string scope: "view"
    property var remembered: ({})     // scope → last focused item's key
    property int moves: 0
    signal moved(Item item)

    function register(it) { items.push(it); }
    function unregister(it) {
        const i = items.indexOf(it);
        if (i >= 0) items.splice(i, 1);
        if (focused === it) focused = null;
    }
    function usable(it) {
        return it && it.visible && it.enabled && it.navScope === scope && it.width > 0 && it.height > 0 && it.opacity > 0.05;
    }
    function rect(it) {
        const p = it.mapToItem(null, 0, 0);
        return { x: p.x, y: p.y, w: it.width, h: it.height, cx: p.x + it.width / 2, cy: p.y + it.height / 2 };
    }
    function focus(it, quiet) {
        if (!it) return;
        if (focused === it) { if (!quiet) it.revealed(); return; }
        focused = it;
        if (it.navKey) { const r = remembered; r[it.navScope] = it.navKey; remembered = r; }
        it.revealed();
        moved(it);
    }
    function byKey(key, sc) {
        for (const it of items) if (it.navKey === key && it.navScope === (sc || scope) && it.visible) return it;
        return null;
    }
    function focusKey(key) { const it = byKey(key); if (it) { focus(it); return true; } return false; }
    // The first control of the scope: its preferred one (navFirst), else the remembered one, else the top-left one.
    function first(sc, noMemory) {
        const s = sc || scope;
        const rk = noMemory ? "" : remembered[s];
        let best = null, pref = null, mem = null;
        for (const it of items) {
            if (!(it && it.visible && it.enabled && it.navScope === s && it.width > 0)) continue;
            if (it.navFirst && !pref) pref = it;
            if (rk && it.navKey === rk && !mem) mem = it;
            const r = rect(it);
            if (!best || r.y < best.r.y - 4 || (Math.abs(r.y - best.r.y) <= 4 && r.x < best.r.x)) best = { it: it, r: r };
        }
        return mem || pref || (best ? best.it : null);
    }
    function reset(sc, preferRemembered) {
        scope = sc;
        const it = first(sc, preferRemembered === false);
        if (it) focus(it); else focused = null;
    }
    // A press in a direction: the focused control may use it itself (a slider, a list), else the nearest one that way.
    function move(dir) {
        moves++;
        if (focused && usable(focused) && focused.navMove && focused.navMove(dir)) return true;
        if (!focused || !usable(focused)) { const f = first(); if (f) focus(f); return !!f; }
        const a = rect(focused);
        let best = null, bestScore = 1e12;
        for (const it of items) {
            if (it === focused || !usable(it)) continue;
            const b = rect(it);
            let along, across, gap;
            if (dir === "right" || dir === "left") {
                along = dir === "right" ? b.x - (a.x + a.w) : a.x - (b.x + b.w);
                if ((dir === "right" ? b.cx <= a.cx + 1 : b.cx >= a.cx - 1)) continue;
                gap = Math.max(0, Math.max(a.y, b.y) - Math.min(a.y + a.h, b.y + b.h));
                across = Math.abs(a.cy - b.cy);
            } else {
                along = dir === "down" ? b.y - (a.y + a.h) : a.y - (b.y + b.h);
                if ((dir === "down" ? b.cy <= a.cy + 1 : b.cy >= a.cy - 1)) continue;
                gap = Math.max(0, Math.max(a.x, b.x) - Math.min(a.x + a.w, b.x + b.w));
                across = Math.abs(a.cx - b.cx);
            }
            const score = Math.max(along, -Math.min(a.w, a.h) * 0.5) + gap * 3 + across * 0.25 + (gap > 0 ? 40 : 0);
            if (score < bestScore) { bestScore = score; best = it; }
        }
        if (best) { focus(best); return true; }
        return false;
    }
    function activate() { if (focused && usable(focused)) { focused.activated(); return true; } return false; }
}

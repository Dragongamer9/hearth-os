pragma Singleton

// Moving around the login screen with the arrow keys or a controller (hearth-greeter-pad turns the D-pad and stick into
// arrow keys, A into Enter, B Backspace, X Tab, Y Escape while it's up): every Chip adds itself here, and an arrow
// picks the nearest one that way on the same layer ("main", or an open menu's). The ring shows once keys are used.
import QtQuick
import Quickshell

Singleton {
    id: nav
    property var items: []
    property Item current: null
    property string layer: "main"
    property bool ring: false
    function add(i) { items.push(i); }
    function remove(i) { const k = items.indexOf(i); if (k >= 0) items.splice(k, 1); if (current === i) current = null; }
    function shown(i) {
        for (let p = i; p; p = p.parent) if (!p.visible || p.opacity <= 0.01) return false;
        return true;
    }
    function candidates() { return items.filter(i => i && i.navigable && i.enabled && i.layer === layer && shown(i)); }
    function centre(i, root) { const p = i.mapToItem(root, i.width / 2, i.height / 2); return { x: p.x, y: p.y }; }
    function first(root) {
        const c = candidates();
        if (!c.length) return null;
        const pref = c.filter(i => i.preferred);
        return (pref.length ? pref : c)[0];
    }
    function ensure(root) {
        if (!current || candidates().indexOf(current) < 0) current = first(root);
        return current;
    }
    function move(dir, root) {
        ring = true;
        const c = candidates();
        if (!current || c.indexOf(current) < 0) { current = first(root); return !!current; }
        const a = centre(current, root);
        let best = null, score = 1e12;
        for (const i of c) {
            if (i === current) continue;
            const b = centre(i, root), dx = b.x - a.x, dy = b.y - a.y;
            let along, across;
            if (dir === "left") { along = -dx; across = Math.abs(dy); }
            else if (dir === "right") { along = dx; across = Math.abs(dy); }
            else if (dir === "up") { along = -dy; across = Math.abs(dx); }
            else { along = dy; across = Math.abs(dx); }
            if (along <= 4) continue;
            const s = along + across * 2.2;
            if (s < score) { score = s; best = i; }
        }
        if (best) current = best;
        return !!best;
    }
    function setLayer(l, root) { layer = l; current = null; ensure(root); }
}

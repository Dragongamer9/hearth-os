// Spatial moves for the app menus (MenuTiles.qml, MenuStart.qml, MenuCentred.qml): from the item at `cur`, the
// nearest one in a direction (dx, dy: -1, 0 or 1), by where they are on screen. -1 when there's none that way.
.pragma library

function nearest(items, cur, dx, dy) {
    const a = items[cur];
    if (!a) return items.length ? 0 : -1;
    const ac = a.mapToItem(null, a.width / 2, a.height / 2);
    let best = -1, bestScore = 1e12;
    for (let i = 0; i < items.length; i++) {
        if (i === cur || !items[i] || !items[i].visible) continue;
        const b = items[i], bc = b.mapToItem(null, b.width / 2, b.height / 2);
        const ddx = bc.x - ac.x, ddy = bc.y - ac.y;
        const along = dx !== 0 ? ddx * dx : ddy * dy;           // how far in the direction asked
        if (along <= 4) continue;
        const across = dx !== 0 ? Math.abs(ddy) : Math.abs(ddx); // how far off to the side
        const score = along + across * 2.2;
        if (score < bestScore) { bestScore = score; best = i; }
    }
    return best;
}

// Moving a tile (MenuTiles.qml, MenuStart.qml): `ids` with `moving` put where `target` is (before it when moving
// back, after it when moving on). The same list when either isn't there or they're the same tile.
function moveStep(ids, moving, target) {
    const from = ids.indexOf(moving), to = ids.indexOf(target);
    if (from < 0 || to < 0 || from === to) return ids;
    const out = ids.slice();
    out.splice(from, 1);
    out.splice(to, 0, moving);
    return out;
}

// The tile under a point on screen (sx, sy, as mapToItem(null, ...) gives), among `items`; null when none.
function tileAt(items, sx, sy) {
    for (const it of items) {
        if (!it || !it.visible) continue;
        const p = it.mapToItem(null, 0, 0);
        if (sx >= p.x && sx < p.x + it.width && sy >= p.y && sy < p.y + it.height) return it;
    }
    return null;
}

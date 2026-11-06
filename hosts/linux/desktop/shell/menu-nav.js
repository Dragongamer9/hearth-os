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

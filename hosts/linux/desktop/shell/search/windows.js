.pragma library
// Search plugin: open windows (like KRunner's). Launcher.qml's windowList: [{id, title, appId}], which the window-list
// work (Bar / Dock / TaskStrip on KWin) fills in; until it does the list is empty and this finds nothing.
// Enter switches to the window (Launcher.qml's activateWindow(id), from the same work).

function results(qRaw, windows) {
    var q = String(qRaw || "").trim().toLowerCase();
    if (q.length < 2 || !windows || !windows.length) return [];
    var out = [];
    for (var i = 0; i < windows.length; i++) {
        var w = windows[i], t = String(w.title || "").toLowerCase(), a = String(w.appId || "").toLowerCase();
        var s = t.indexOf(q) === 0 ? 820 : t.indexOf(q) > 0 ? 640 : a.indexOf(q) >= 0 ? 600 : 0;
        if (s) out.push({ plugin: "windows", group: "Open windows", title: w.title || w.appId, sub: "Switch to this window" + (w.appId ? " · " + w.appId : ""),
                          icon: "window", score: s - i, action: { type: "window", value: w.id } });
    }
    return out.slice(0, 5);
}

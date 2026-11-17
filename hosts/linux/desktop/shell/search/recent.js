.pragma library
// Search plugin: recent documents (like KRunner's), from the freedesktop recently-used list that Launcher.qml already
// reads (hearth-appmenu recent): [{name, path, folder, dir}]. Matches the name; newest first. Enter opens it.

function results(qRaw, recent) {
    var q = String(qRaw || "").trim().toLowerCase();
    if (q.length < 2 || !recent || !recent.length) return [];
    var out = [];
    for (var i = 0; i < recent.length && out.length < 4; i++) {
        var f = recent[i], n = String(f.name || "").toLowerCase();
        if (n.indexOf(q) < 0) continue;
        out.push({ plugin: "recent", group: "Recent documents", title: f.name, sub: "Opened lately · " + (f.folder || ""),
                   icon: f.dir ? "folder" : "file", score: (n.indexOf(q) === 0 ? 700 : 500) - i, action: { type: "file", value: f.path, dir: !!f.dir } });
    }
    return out;
}

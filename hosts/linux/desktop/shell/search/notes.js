.pragma library
// Search plugin: Hearth Notes (like KRunner's notes runner): the notes in ~/Documents/Notes (plain Markdown, Hearth
// Notes' folder), by title and by what's written in them, read by hearth-search-sources pim: [{title, path, text}]
// (text: the first few KB). Enter opens the note.

function results(qRaw, notes) {
    var q = String(qRaw || "").trim().toLowerCase();
    if (q.length < 3 || !notes || !notes.length) return [];
    var out = [];
    for (var i = 0; i < notes.length && out.length < 3; i++) {
        var n = notes[i], t = String(n.title || "").toLowerCase(), body = String(n.text || ""), at = body.toLowerCase().indexOf(q);
        var s = t.indexOf(q) === 0 ? 3 : t.indexOf(q) >= 0 ? 2 : at >= 0 ? 1 : 0;
        if (!s) continue;
        var snip = at >= 0 ? body.slice(Math.max(0, at - 30), at + 60).replace(/\s+/g, " ").trim() : "";
        out.push({ plugin: "notes", group: "Notes", title: n.title || n.path.replace(/^.*\//, ""), sub: snip ? "…" + snip + "…" : "Note in Hearth Notes",
                   icon: "edit", score: 620 + s * 50 - i, action: { type: "file", value: n.path, dir: false } });
    }
    return out;
}

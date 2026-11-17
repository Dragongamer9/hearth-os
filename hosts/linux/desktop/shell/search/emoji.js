.pragma library
// Search plugin: emoji (like KRunner's), from Hearth's Emoji & symbols data (emoji-data.js, made by
// tools/build-search-emoji.py from hosts/hearth-emoji/ui/emoji-data.js). "heart", "thumbs up", ":fire" (with ":" only
// emoji, and more of them). Enter copies the emoji.
.import "emoji-data.js" as Data

function fold(s) { return String(s || "").toLowerCase().replace(/[’']/g, "").trim(); }

// [{e, n}] best first. Names that are the query, then start with it, then a word, then a search word.
function find(qRaw, limit) {
    var q = fold(qRaw);
    if (q.length < 2) return [];
    var out = [], list = Data.EMOJI;
    for (var i = 0; i < list.length; i++) {
        var e = list[i], n = e[1], s = 0;
        if (n === q) s = 100;
        else if (n.indexOf(q) === 0) s = 80;
        else if ((" " + n).indexOf(" " + q) >= 0) s = 60;
        else if ((" " + e[2] + " ").indexOf(" " + q) >= 0) s = 40;
        if (s) out.push({ e: e[0], n: n, s: s - n.length / 100, i: i });
    }
    out.sort(function (a, b) { return b.s - a.s || a.i - b.i; });
    return out.slice(0, limit || 6);
}

function results(qRaw) {
    var q = String(qRaw || "").trim(), only = q.charAt(0) === ":";
    if (only) q = q.slice(1);
    if (!only && q.length < 3) return [];
    return find(q, only ? 24 : 4).map(function (x, k) {
        return { plugin: "emoji", group: "Emoji", title: x.e + "  " + x.n, sub: "Enter copies " + x.e, icon: "", glyph: x.e,
                 score: (only ? 1800 : 300) - k, top: only && k === 0, action: { type: "copy", value: x.e } };
    });
}

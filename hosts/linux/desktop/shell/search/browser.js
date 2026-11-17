.pragma library
// Search plugin: bookmarks and browser history (like KRunner's bookmarks runner), from Firefox and Chromium-based
// browsers' profiles. hearth-search-sources browser reads them from a COPY of each places.sqlite / History file (the
// live file is never opened, so a running browser is never locked or slowed) and Bookmarks; SearchSources.qml hands
// them over as {bookmarks: [{title, url, browser}], history: [{title, url, browser, visits}]}. Enter opens the page.

function words(s) { return String(s || "").toLowerCase(); }

function score(q, title, url) {
    var t = words(title), u = words(url).replace(/^[a-z]+:\/\/(www\.)?/, "");
    if (t.indexOf(q) === 0 || u.indexOf(q) === 0) return 3;
    if (t.indexOf(" " + q) >= 0) return 2;
    if (t.indexOf(q) >= 0 || u.indexOf(q) >= 0) return 1;
    return 0;
}

function results(qRaw, data) {
    var q = String(qRaw || "").trim().toLowerCase();
    if (q.length < 2 || !data) return [];
    var out = [], seen = {};
    var marks = data.bookmarks || [], hist = data.history || [];
    for (var i = 0; i < marks.length && out.length < 4; i++) {
        var b = marks[i], s = score(q, b.title, b.url);
        if (!s || !b.url || seen[b.url]) continue;
        seen[b.url] = true;
        out.push({ plugin: "browser", group: "Bookmarks", title: b.title || b.url, sub: b.url + (b.browser ? " · " + b.browser : ""),
                   icon: "pin", score: 600 + s * 60 - i, action: { type: "url", value: b.url } });
    }
    var n = 0;
    for (var j = 0; j < hist.length && n < 3; j++) {
        var h = hist[j], hs = score(q, h.title, h.url);
        if (!hs || !h.url || seen[h.url]) continue;
        seen[h.url] = true; n++;
        out.push({ plugin: "browser", group: "Browser history", title: h.title || h.url, sub: h.url + (h.browser ? " · " + h.browser : ""),
                   icon: "globe", score: 450 + hs * 40 - j, action: { type: "url", value: h.url } });
    }
    return out;
}

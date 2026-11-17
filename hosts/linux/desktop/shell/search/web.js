.pragma library
// Search plugin: web shortcuts (like KRunner's). "gg:hearth os", "wp:linux", "yt:lofi", "ddg:...", and a web address
// typed as is ("hearthos.fun", "https://…") opens in the browser. Enter opens it (xdg-open: your default browser).

var SHORTCUTS = {
    gg: ["Google", "https://www.google.com/search?q="],
    google: ["Google", "https://www.google.com/search?q="],
    ddg: ["DuckDuckGo", "https://duckduckgo.com/?q="],
    wp: ["Wikipedia", "https://en.wikipedia.org/wiki/Special:Search?search="],
    yt: ["YouTube", "https://www.youtube.com/results?search_query="],
    gh: ["GitHub", "https://github.com/search?q="],
    gm: ["Google Maps", "https://www.google.com/maps/search/"],
    osm: ["OpenStreetMap", "https://www.openstreetmap.org/search?query="],
    fh: ["Flathub", "https://flathub.org/apps/search?q="],
    deb: ["Debian packages", "https://packages.debian.org/search?keywords="],
    pdb: ["ProtonDB", "https://www.protondb.com/search?q="],
    steam: ["Steam store", "https://store.steampowered.com/search/?term="],
    aw: ["ArchWiki", "https://wiki.archlinux.org/index.php?search="],
    imdb: ["IMDb", "https://www.imdb.com/find/?q="],
    wt: ["Wiktionary", "https://en.wiktionary.org/wiki/Special:Search?search="],
    rd: ["Reddit", "https://www.reddit.com/search/?q="]
};

function url(qRaw) {
    var q = String(qRaw || "").trim();
    var m = /^([a-z]{2,6})\s*:\s*(.+)$/i.exec(q);
    if (m && SHORTCUTS[m[1].toLowerCase()]) {
        var s = SHORTCUTS[m[1].toLowerCase()];
        return { name: s[0], url: s[1] + encodeURIComponent(m[2].trim()).replace(/%20/g, "+"), words: m[2].trim() };
    }
    if (/^https?:\/\/\S+$/i.test(q)) return { name: "", url: q, words: q };
    // "hearthos.fun", "www.example.com/x": letters, a dot, a known-looking ending; no spaces
    if (/^(www\.)?[a-z0-9-]+(\.[a-z0-9-]+)*\.(com|org|net|fun|io|dev|app|de|uk|eu|fr|nl|ca|us|tv|gg|me|info|xyz|co|ai)(\/\S*)?$/i.test(q))
        return { name: "", url: "https://" + q, words: q };
    return null;
}

function results(q) {
    var u = url(q);
    if (!u) return [];
    return [{ plugin: "web", group: "Web", title: u.name ? "Search " + u.name + " for “" + u.words + "”" : "Open " + u.words,
              sub: u.url, icon: "globe", score: 1950, top: true, action: { type: "url", value: u.url } }];
}

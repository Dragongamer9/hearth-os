.pragma library
// Search plugin: App store results (like KRunner's Discover runner). "install krita" or "store krita": apps on Flathub
// (hearth-store search, looked up as you type it, kept a while), each opening its page in Settings › App store, where
// you install it. Only asked for with "install"/"store", so ordinary typing never goes online.
// store: {query, apps: [{id, name, summary}], error} from SearchSources.qml.

function asked(qRaw) {
    var m = /^\s*(?:install|store|get app)\s+(.+?)\s*$/i.exec(String(qRaw || ""));
    return m && m[1].length >= 2 ? m[1] : null;
}

function results(qRaw, store) {
    var w = asked(qRaw);
    if (!w) return [];
    var out = [];
    if (!store || String(store.query || "").toLowerCase() !== w.toLowerCase())
        return [{ plugin: "store", group: "App store", title: "Looking for “" + w + "” on Flathub…", sub: "App store", icon: "store", score: 1820, action: null }];
    var apps = store.apps || [];
    for (var i = 0; i < apps.length && i < 5; i++) {
        var a = apps[i];
        if (!a.id) continue;
        out.push({ plugin: "store", group: "App store", title: a.name || a.id, sub: (a.summary || a.id) + " · opens its page in the App store", icon: "store",
                   score: 1820 - i, top: i === 0, action: { type: "page", value: "store:app-" + a.id.replace(/\./g, "_") } });
    }
    if (!out.length) out.push({ plugin: "store", group: "App store", title: store.error ? "The App store couldn't be reached" : "Nothing called “" + w + "” on Flathub",
                                sub: store.error || "Open the App store", icon: "store", score: 1820, action: { type: "page", value: "store" } });
    return out;
}

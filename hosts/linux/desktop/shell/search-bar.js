.pragma library
// The search bar's own logic (SearchBar.qml, Alt+Space, like KDE's KRunner): the results of Launcher.qml's searchAll
// (apps, games, Settings, files and the search plugins in search/*.js) put in the order you chose in Settings › Search
// (hearth-search's "pluginOrder"), the "?" help list, whether Alt+Space belongs to the keyboard layout switch, and
// which typed keys start a search from the desktop. No QML here, so test_search_bar.py checks it with node.

// The groups the bar knows by name. Plugins it doesn't know (a new search/*.js) still show, after these, under their
// own group name; with a "help" line here they're in the "?" list too.
var BUILTIN = ["apps", "games", "pages", "files"];
var DEFAULT_ORDER = ["apps", "commands", "web", "calc", "units", "currency", "windows", "pages", "games", "files", "recent", "emoji"];
var NAMES = {
    apps: ["Apps", "Apps by name, what they do, or their Windows names", "", "grid"],
    games: ["Games", "Your installed games", "", "pad"],
    pages: ["Settings", "Settings pages and single settings", "wifi", "gear"],
    files: ["Files and folders", "By name (and what's in them when the file index reads documents)", "report", "folder"],
    commands: ["Commands", "Run a command (in the terminal or on its own)", ">", "kbd"],
    web: ["Web shortcuts", "gg: Google, wp: Wikipedia, yt: YouTube, ddg:, gh:, fh: Flathub, or a web address", "gg:", "globe"],
    calc: ["Calculator", "Sums as you type", "=", "plus"],
    units: ["Unit conversion", "10 km to mi, 72 f in c, 500 MB to GB", "10 km to mi", "sync"],
    currency: ["Currency conversion", "20 usd to eur", "20 usd to eur", "sync"],
    windows: ["Open windows", "Switch to a window by its title", "", "window"],
    recent: ["Recent documents", "Files you opened lately", "", "clock"],
    emoji: ["Emoji", ":fire, heart, thumbs up; Enter copies it", ":", "sparkle"]
};

function name(p) { return (NAMES[p] || [p.charAt(0).toUpperCase() + p.slice(1)])[0]; }
function icon(p) { return (NAMES[p] || [0, 0, 0, "search"])[3]; }

// Your order (saved; may be missing or old), then everything else known, in Hearth's order, then plugins not known
// yet (from hearth-search's list or the answers themselves). Every name once; nothing that isn't a short word.
function order(saved, plugins) {
    var out = [], seen = {};
    function add(p) { p = String(p || ""); if (/^[a-z0-9-]{1,24}$/.test(p) && !seen[p]) { seen[p] = true; out.push(p); } }
    if (Array.isArray(saved)) saved.forEach(add);
    DEFAULT_ORDER.forEach(add);
    if (Array.isArray(plugins)) plugins.forEach(add);
    return out;
}

// One step up (d = -1) or down (d = 1) in the list; the same list when it can't move.
function move(list, p, d) {
    var l = list.slice(), i = l.indexOf(p), j = i + d;
    if (i < 0 || j < 0 || j >= l.length) return l;
    l[i] = l[j]; l[j] = p;
    return l;
}

function groupOf(row) { return row.kind === "answer" ? String(row.item.plugin || "answer") : row.kind === "app" ? "apps" : row.kind === "game" ? "games" : row.kind === "page" ? "pages" : "files"; }

// The same result twice (the best match is also in its group): by kind and what it opens.
function same(kind, it) {
    if (!it) return "";
    var a = it.action ? JSON.stringify(it.action) : "";
    return kind + ":" + (it.plugin || "") + ":" + (it.id || it.path || it.page || a || it.title || it.name || "");
}

// r: Launcher.qml's searchAll(q, "all"). Rows [{kind, item, group, plugin}]: the best match first, then each group in
// your order (at most `per` rows each; plugins not in the order come last).
function arrange(r, ord, per) {
    per = per || 4;
    var rows = [];
    if (!r) return rows;
    var best = r.best ? r.best.item : null, bestKey = r.best ? same(r.best.kind, best) : "";
    if (r.best) rows.push({ kind: r.best.kind, item: r.best.item, group: "Best match", plugin: groupOf(r.best) });
    var byGroup = {};
    function put(kind, item) {
        if (item === best || (bestKey && same(kind, item) === bestKey)) return;
        var row = { kind: kind, item: item };
        var g = groupOf(row);
        (byGroup[g] = byGroup[g] || []).push(row);
    }
    (r.answers || []).forEach(function (x) { put("answer", x); });
    (r.apps || []).forEach(function (x) { put("app", x); });
    (r.games || []).forEach(function (x) { put("game", x); });
    (r.pages || []).forEach(function (x) { put("page", x); });
    (r.files || []).forEach(function (x) { put("file", x); });
    var names = order(ord, Object.keys(byGroup).sort());
    names.forEach(function (g) {
        var list = byGroup[g];
        if (!list) return;
        var label = list[0].kind === "answer" && list[0].item.group ? list[0].item.group : name(g);
        list.slice(0, g === "apps" ? per + 1 : per).forEach(function (row) { row.group = label; row.plugin = g; rows.push(row); });
    });
    return rows;
}

// "?" (or "?words"): the plugins, in your order, with what to type; the ones turned off say so. A row's "fill" is
// what Enter puts in the field.
function helpRows(q, ord, off) {
    var want = String(q || "").replace(/^\s*\?\s*/, "").toLowerCase();
    off = off || {};
    var rows = [];
    order(ord, []).forEach(function (p) {
        var n = NAMES[p];
        if (!n) return;
        if (want && (n[0] + " " + n[1] + " " + p).toLowerCase().indexOf(want) < 0) return;
        rows.push({ kind: "help", plugin: p, group: "What you can type", title: n[0] + (off[p] ? " (off in Settings › Search)" : ""),
                    sub: n[1], fill: n[2], icon: n[3], off: !!off[p] });
    });
    return rows;
}
function isHelp(q) { return /^\s*\?/.test(String(q || "")); }

// Alt+Space is the keyboard layout switch (Settings › Keyboard, ~/.config/nimbus/keyboard.json): only when it's chosen
// and there's more than one layout (with one layout there's nothing to switch: hearth-system leaves it out).
function layoutTakesAltSpace(kb) {
    return !!kb && kb.switch === "grp:alt_space_toggle" && Array.isArray(kb.layouts) && kb.layouts.length > 1;
}

// A key typed on the desktop that starts a search: a letter, digit or sign, no Ctrl / Alt / Super, not a space.
function startsSearch(text, ctrl, alt, meta) {
    if (ctrl || alt || meta) return false;
    var t = String(text || "");
    return t.length === 1 && t.charCodeAt(0) > 32 && t.charCodeAt(0) !== 127;
}

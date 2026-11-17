.pragma library
// Hearth's search plugins (like KRunner's runners), for the app menus' search (Launcher.qml's searchAll). Each plugin
// is its own file with results(query, ...) → [{plugin, group, title, sub, icon, score, top, action}]:
//   calc.js       sums                      "2+2", "=sqrt(2)", "15% of 80"
//   units.js      unit conversion           "10 km to mi", "72 f in c"
//   currency.js   currency, offline-safe    "20 usd to eur" (ECB rates when saved, else built-in, said so)
//   web.js        web shortcuts             "gg:…", "wp:…", "yt:…", "ddg:…", a web address
//   commands.js   run a command             ">htop"
//   emoji.js      emoji                     "heart", ":fire"
//   recent.js     recent documents          the recently-used list
//   windows.js    open windows              Launcher.qml's windowList (from the window-list work)
// Apps, games, Settings pages and files (the file index, hearth-search) stay in Launcher.qml, ranked by menu-search.js.
// action: {type: copy|url|command|file|window, value, ...}; Launcher.qml's openRow does it. top: may be the Best match.
.import "calc.js" as Calc
.import "units.js" as Units
.import "currency.js" as Currency
.import "web.js" as Web
.import "commands.js" as Commands
.import "emoji.js" as Emoji
.import "recent.js" as Recent
.import "windows.js" as Windows

var PLUGINS = ["commands", "web", "calc", "units", "currency", "windows", "recent", "emoji"];

// ctx: {rates, recent, windows, off: {plugin: true}} (off: the ones Settings › Search turned off)
function run(qRaw, ctx) {
    var q = String(qRaw || "");
    if (!q.trim()) return [];
    ctx = ctx || {};
    var off = ctx.off || {};
    var out = [];
    function add(name, fn) { if (off[name]) return; try { out = out.concat(fn() || []); } catch (e) {} }
    add("commands", function () { return Commands.results(q); });
    // A command is only a command: nothing else for ">…".
    if (out.length) return out;
    add("web", function () { return Web.results(q); });
    add("calc", function () { return Calc.results(q); });
    add("units", function () { return Units.results(q); });
    add("currency", function () { return Currency.results(q, ctx.rates); });
    add("windows", function () { return Windows.results(q, ctx.windows); });
    add("recent", function () { return Recent.results(q, ctx.recent); });
    add("emoji", function () { return Emoji.results(q); });
    out.sort(function (a, b) { return b.score - a.score; });
    return out;
}

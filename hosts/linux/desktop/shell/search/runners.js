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
// 1.14 wave 3 (their data from SearchSources.qml, in ctx.src; hearth-search-sources reads it):
//   browser.js bookmarks and history · desktops.js virtual desktops, activities · kill.js "kill <name>" (asks first)
//   worldclock.js "time in tokyo", "3pm london to tokyo" · spell.js "spell …", "define …" · chars.js "degree", "u+00e9"
//   colours.js "#ff8800", "rgb(…)" · power.js sleep, restart, lock, log out (asks first where it closes apps)
//   media.js play, pause, next · store.js "install krita" · contacts.js, events.js, notes.js Hearth Contacts, Calendar, Notes
// An action with confirm: {title, message, ok} is only done after the confirm dialog says yes (SearchSources.qml).
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
.import "browser.js" as Browser
.import "desktops.js" as Desktops
.import "kill.js" as Kill
.import "worldclock.js" as WorldClock
.import "spell.js" as Spell
.import "chars.js" as Chars
.import "colours.js" as Colours
.import "power.js" as Power
.import "media.js" as Media
.import "store.js" as Store
.import "contacts.js" as Contacts
.import "events.js" as Events
.import "notes.js" as Notes

var PLUGINS = ["commands", "web", "calc", "units", "currency", "windows", "recent", "emoji",
               "browser", "desktops", "kill", "worldclock", "spell", "chars", "colours", "power", "media", "store", "contacts", "events", "notes"];

// ctx: {rates, recent, windows, src (SearchSources.qml's data), off: {plugin: true}} (off: the ones Settings › Search turned off)
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
    var src = ctx.src || {}, pim = src.pim || {};
    add("browser", function () { return Browser.results(q, src.browser); });
    add("desktops", function () { return Desktops.results(q, src.desktops); });
    add("kill", function () { return Kill.results(q, src.procs); });
    add("worldclock", function () { return WorldClock.results(q, src.zones, src.now); });
    add("spell", function () { return Spell.results(q, src.spell); });
    add("chars", function () { return Chars.results(q); });
    add("colours", function () { return Colours.results(q); });
    add("power", function () { return Power.results(q); });
    add("media", function () { return Media.results(q, src.media); });
    add("store", function () { return Store.results(q, src.store); });
    add("contacts", function () { return Contacts.results(q, pim.contacts); });
    add("events", function () { return Events.results(q, pim.events, src.now); });
    add("notes", function () { return Notes.results(q, pim.notes); });
    out.sort(function (a, b) { return b.score - a.score; });
    return out;
}

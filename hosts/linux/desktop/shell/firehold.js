.pragma library
// Firehold's Couch (Firehold.qml, a hidden preview): the rules its screens follow, kept apart from the drawing so they
// can be checked on their own (hosts/linux/test/firehold.test.js, run in CI with node).
//   games: hearth-firehold library's list; windows: WinList's ({appId, title, fullscreen, activated, ...}).

// The badge colours of the mockups' placeholder tiles, used under a game that has no picture yet.
var TONES = ["#3d4a52", "#5b2b1a", "#2c2a3a", "#2f3d33", "#6b4a2a", "#4e3340"];
var GAME_GRADIENT = ["#2b1a2e", "#5b2b1a", "#a3541e"];

// Emulators and game runners whose windows are games (Wayland app ids and X11 classes, lower case).
var GAME_APPS = ["retroarch", "org.libretro.retroarch", "dolphin-emu", "org.dolphinemu.dolphin-emu", "pcsx2", "net.pcsx2.pcsx2",
    "rpcs3", "net.rpcs3.rpcs3", "duckstation", "org.duckstation.duckstation", "ppsspp", "org.ppsspp.ppsspp", "ryujinx",
    "org.ryujinx.ryujinx", "cemu", "info.cemu.cemu", "xemu", "app.xemu.xemu", "melonds", "net.kuribo64.melonds", "mgba",
    "io.mgba.mgba", "snes9x", "mednafen", "flycast", "org.flycast.flycast", "azahar", "io.github.lime3ds.lime3ds", "citra",
    "eden", "gamescope", "steam_proton", "hearth-game"];
// The shell's own windows (Settings, Task Manager...) aren't listed as "open on this PC".
var OWN_APPS = ["org.quickshell", "quickshell"];

function norm(t) { return String(t || "").toLowerCase().replace(/[^a-z0-9]+/g, ""); }

function toneFor(id) {
    var h = 0, s = String(id || "");
    for (var i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
    return TONES[h % TONES.length];
}

function ownWindow(w) { return !w || OWN_APPS.indexOf(String(w.appId || "").toLowerCase()) >= 0; }

// The library game a window is, or null: Steam's games (steam_app_<id>), then a game with the window's title.
function gameFor(w, games) {
    if (!w) return null;
    var a = String(w.appId || "").toLowerCase();
    var m = /^steam_app_(\d+)$/.exec(a);
    var list = games || [];
    if (m) {
        for (var i = 0; i < list.length; i++) if (list[i].id === "steam:" + m[1]) return list[i];
        return { id: "steam:" + m[1], title: w.title || "Steam game", store: "Steam", kind: "steam", installed: true };
    }
    var t = norm(w.title);
    if (t.length >= 3) {
        for (var j = 0; j < list.length; j++) {
            var g = list[j];
            if (g.kind === "winapp" ? false : norm(g.title) === t) return g;
        }
    }
    return null;
}

function isGame(w, games) {
    if (!w || ownWindow(w)) return false;
    if (gameFor(w, games)) return true;
    return GAME_APPS.indexOf(String(w.appId || "").toLowerCase()) >= 0;
}

// A window's name and its second line: the app's name (from its .desktop entry, when known) and what the title
// adds to it ("Pictures › Trips", "3 tabs").
function windowName(w, entryName) {
    var title = String(w && w.title || "").trim();
    var name = String(entryName || "").trim();
    if (!name) {
        var parts = title.split(/\s+[—–-]\s+/);
        name = parts.length > 1 ? parts[parts.length - 1] : (title || String(w && w.appId || "Window"));
    }
    var sub = title;
    var re = new RegExp("\\s*[—–-]\\s*" + name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "\\s*$", "i");
    sub = sub.replace(re, "");
    if (norm(sub) === norm(name)) sub = "";
    return { name: name, sub: sub };
}

// Everything open, as one list: games first, then windows, each {kind: "game" | "window", name, sub, win, game}.
// entryName(w): the app's name for a window, or "".
function openItems(wins, games, entryName, playing) {
    var gamesOut = [], winsOut = [];
    var list = wins || [];
    for (var i = 0; i < list.length; i++) {
        var w = list[i];
        if (ownWindow(w) || w.dialog === true) continue;
        var g = gameFor(w, games);
        if (g || isGame(w, games)) {
            var state = w.minimized ? "paused" : (w.activated ? "playing" : "running");
            var src = g ? String(g.store || "game").toLowerCase() : "emulator";
            gamesOut.push({ kind: "game", name: g ? g.title : String(w.title || "Game"), sub: (src === "emulator" ? "emulator" : "game") + " · " + state, win: w, game: g });
        } else {
            var n = windowName(w, entryName ? entryName(w) : "");
            winsOut.push({ kind: "window", name: n.name, sub: n.sub, win: w, game: null });
        }
    }
    return gamesOut.concat(winsOut);
}

// The games running now: their library ids.
function playingIds(wins, games) {
    var out = [];
    var list = wins || [];
    for (var i = 0; i < list.length; i++) {
        var g = gameFor(list[i], games);
        if (g && out.indexOf(g.id) < 0) out.push(g.id);
    }
    return out;
}

// A game's line on its tile.
function status(g, playing) {
    if (!g) return "";
    if ((playing || []).indexOf(g.id) >= 0) return "Playing";
    if (g.installed && g.update) return "Update ready";
    if (g.installed && g.devBuild) return "Dev build";
    if (g.installed) return "Installed";
    if (g.savesBackedUp) return "Cloud saves";
    return "Not installed";
}

// The Recent row: what you played last, newest first; filled up with installed games when there are few.
function recent(games, n, skipId) {
    var list = (games || []).filter(function (g) { return g.id !== skipId; });
    var played = list.filter(function (g) { return g.lastPlayed > 0; }).sort(function (a, b) { return b.lastPlayed - a.lastPlayed; });
    var out = played.slice(0, n);
    for (var i = 0; i < list.length && out.length < n; i++)
        if (list[i].installed && out.indexOf(list[i]) < 0) out.push(list[i]);
    return out;
}

// The Continue hero: the game running now, else the one played last (installed), else none.
function continueGame(games, playing) {
    var list = games || [];
    for (var i = 0; i < list.length; i++) if ((playing || []).indexOf(list[i].id) >= 0) return list[i];
    var best = null;
    for (var j = 0; j < list.length; j++) {
        var g = list[j];
        if (g.installed && g.lastPlayed > 0 && (!best || g.lastPlayed > best.lastPlayed)) best = g;
    }
    return best;
}

// "41 MIN AGO", "3 H AGO", "YESTERDAY", "4 DAYS AGO", "NOW".
function ago(t, now) {
    if (!t) return "";
    var s = Math.max(0, Math.round((now - t) / 1000));
    if (s < 60) return "NOW";
    if (s < 3600) return Math.floor(s / 60) + " MIN AGO";
    if (s < 86400) return Math.floor(s / 3600) + " H AGO";
    if (s < 172800) return "YESTERDAY";
    return Math.floor(s / 86400) + " DAYS AGO";
}

// "1 h 12 min", "8 min".
function duration(ms) {
    var m = Math.max(0, Math.floor(ms / 60000));
    if (m < 60) return m + " min";
    return Math.floor(m / 60) + " h " + (m % 60) + " min";
}

// The Library's filter chips, in the mockup's order, with their counts.
var FILTERS = [["all", "All"], ["installed", "Installed"], ["steam", "Steam"], ["gog", "GOG"], ["epic", "Epic"],
               ["winapp", "Windows apps"], ["emu", "Emulators"], ["yours", "Your games"]];
function chips(counts) {
    var c = counts || {};
    return FILTERS.map(function (f) { return { key: f[0], label: f[1], n: c[f[0]] || 0 }; });
}
// The Library's order: what's playing, then installed games (played last first, then A to Z), then the rest A to Z.
function libraryOrder(games, playing) {
    var p = playing || [];
    function rank(g) { return p.indexOf(g.id) >= 0 ? 0 : g.installed ? 1 : 2; }
    return (games || []).slice().sort(function (a, b) {
        var r = rank(a) - rank(b);
        if (r) return r;
        if (rank(a) === 1 && (b.lastPlayed || 0) !== (a.lastPlayed || 0)) return (b.lastPlayed || 0) - (a.lastPlayed || 0);
        var x = String(a.title).toLowerCase(), y = String(b.title).toLowerCase();
        return x < y ? -1 : x > y ? 1 : 0;
    });
}
function filterGames(games, key, query, playing) {
    var q = norm(query);
    return libraryOrder(games, playing).filter(function (g) {
        if (key === "installed" && !g.installed) return false;
        if (key && key !== "all" && key !== "installed" && g.kind !== key) return false;
        return q === "" || norm(g.title).indexOf(q) >= 0;
    });
}

// Posture: what Automatic does with a kind of input ("pad", "mouse", "keys"); Always Couch / Always Desk never move.
function nextPosture(mode, current, input) {
    if (mode === "couch") return "couch";
    if (mode === "desk") return "desk";
    if (input === "pad") return "couch";
    if (input === "mouse" || input === "keys") return "desk";
    return current;
}
// A screen's posture: its own choice (Per screen) over the PC's.
function screenPosture(perScreen, screenName, shared) {
    var v = perScreen && perScreen[screenName];
    return v === "couch" || v === "desk" ? v : shared;
}
// The posture a start opens in (Start in): "couch", "desk", or "last" (the one you were in).
function startPosture(startIn, mode, last) {
    if (mode === "couch" || mode === "desk") return mode;
    if (startIn === "couch" || startIn === "desk") return startIn;
    return last === "couch" ? "couch" : "desk";
}

// Moving the focus in a grid of `count` items, `cols` to a row: the new index (stays put at an edge).
function gridMove(i, count, cols, dir) {
    if (count <= 0) return -1;
    var r = Math.floor(i / cols), c = i % cols;
    if (dir === "left") return c > 0 ? i - 1 : i;
    if (dir === "right") return (c < cols - 1 && i + 1 < count) ? i + 1 : i;
    if (dir === "up") return r > 0 ? i - cols : -1;          // -1: above the grid (the tabs / chips)
    if (dir === "down") return i + cols < count ? i + cols : (Math.floor((count - 1) / cols) > r ? count - 1 : i);
    return i;
}

// The value a setting row shows.
var GUIDE = { quick: "Quick Access", open: "Everything open" };
var GAMES = { follow: "Follow the posture", fullscreen: "Always full screen", window: "Always in a window" };
var START = { couch: "Couch", desk: "Desk", last: "Where you left off" };
function perScreenText(perScreen, screens) {
    var parts = [];
    for (var i = 0; i < (screens || []).length; i++) {
        var s = screens[i];
        var v = perScreen && perScreen[s.name];
        if (v === "couch" || v === "desk") parts.push(s.label + ": " + (v === "couch" ? "Couch" : "Desk"));
    }
    return parts.length ? parts.join(" · ") : "Every screen the same";
}
// A screen's friendly name: a TV (HDMI, big) or a monitor.
function screenLabel(name, model, widthMm) {
    var n = String(name || "");
    if (/hdmi/i.test(n) && (!widthMm || widthMm > 700)) return "TV";
    if (/^(edp|lvds|dsi)/i.test(n)) return "Built-in";
    return model ? String(model) : "Monitor";
}

// The mockups' colours (Firehold has its own look: warm firelit darks, the ember accent).
var C = { bg: "#0f0b09", text: "#f4ece3", muted: "#a69484", soft: "#c9b8a6", sub: "#e8d9c8", line: "#3b2e26",
          panel: "#1c1512", panel2: "#17110e", card: "#261d18", card2: "#33271f", accent: "#ff8a3d", flame: "#ffd166",
          ink: "#2a1206", green: "#7ad6a0", blue: "#66c0f4", red: "#ff8a8a", redText: "#ffb3b3", glass: "#0fffffff" };
// The button hints' colours.
var KEY = { A: "#7ad6a0", X: "#66c0f4", Y: "#ffd166", B: "#ff8a8a", "≡": "#a69484", "LB RB": "#a69484" };
// The Open now cards' backgrounds, games then windows, as in the mockup.
var GAME_CARDS = [["#2b1a2e", "#a3541e"], ["#3d2410", "#6b4a2a"]];
var WINDOW_CARDS = ["#2a2420", "#22303a", "#2c2a3a", "#1e2a33"];
// The mockups' font, Inter, when it's installed; else the shell's own.
function font(fallback) { return Qt.fontFamilies().indexOf("Inter") >= 0 ? "Inter" : fallback; }

// ---- Firehold's window title bars on the Desk (FireholdFrames.qml): what of a rectangle stays visible under others.
// Rectangles are {x, y, w, h}; the result is a list of rectangles that together are A without any of CUTS.
function subtractRect(a, b) {
    var ax2 = a.x + a.w, ay2 = a.y + a.h, bx2 = b.x + b.w, by2 = b.y + b.h;
    if (b.x >= ax2 || bx2 <= a.x || b.y >= ay2 || by2 <= a.y) return [a];
    var out = [];
    if (b.y > a.y) out.push({ x: a.x, y: a.y, w: a.w, h: b.y - a.y });                       // above
    if (by2 < ay2) out.push({ x: a.x, y: by2, w: a.w, h: ay2 - by2 });                         // below
    var top = Math.max(a.y, b.y), bot = Math.min(ay2, by2);
    if (b.x > a.x) out.push({ x: a.x, y: top, w: b.x - a.x, h: bot - top });                   // left
    if (bx2 < ax2) out.push({ x: bx2, y: top, w: ax2 - bx2, h: bot - top });                   // right
    return out.filter(function (r) { return r.w > 0 && r.h > 0; });
}
function visibleParts(rects, cuts) {
    var parts = rects.slice();
    for (var i = 0; i < (cuts || []).length; i++) {
        var next = [];
        for (var j = 0; j < parts.length; j++) next = next.concat(subtractRect(parts[j], cuts[i]));
        parts = next;
    }
    return parts;
}
// The order windows are stacked in, top first: the ones you went to last (KWin raises a window you activate), then the
// rest, newest first. recent: window ids, most recent first.
function stackOrder(wins, recent, idOf) {
    var list = (wins || []).slice();
    var rank = function (w) { var i = (recent || []).indexOf(idOf(w)); return i < 0 ? 100000 - list.indexOf(w) : i; };
    return list.sort(function (a, b) { return rank(a) - rank(b); });
}
// A game window's title line: "Ember Trails · paused · 60 fps limit · VRR" (each part only when it's so).
function gameLine(title, paused, limit, vrr) {
    return [title, paused ? "paused" : "running", limit && limit !== "Off" ? limit + " fps limit" : "", vrr ? "VRR" : ""].filter(Boolean).join(" · ");
}

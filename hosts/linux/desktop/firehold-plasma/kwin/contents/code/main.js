// Firehold on Plasma (a test, docs/FIREHOLD.md "Firehold on Plasma"): KWin's side, through KWin's public script API only.
// Loaded for a Firehold session by hearth-firehold-plasma session (org.kde.kwin.Scripting.loadScript), never from
// kwinrc, so plain Plasma never runs it. What it does:
//   * Couch posture: a game in front goes full screen (Games: Follow the posture); back on the Desk, games go back in
//     their windows (as Mantel's Firehold does). Other windows are left alone.
//   * Meta+Ctrl+G: the Guide (as the controller's Guide button), through Plasma's own scripting interface.
// The posture comes from hearth-firehold-plasma through the shortcuts below (KDE's global shortcuts service).
// Event driven only: no timers, nothing runs while nothing happens.

var GAME_APPS = ["retroarch", "org.libretro.retroarch", "dolphin-emu", "org.dolphinemu.dolphin-emu", "pcsx2", "net.pcsx2.pcsx2",
    "rpcs3", "net.rpcs3.rpcs3", "duckstation", "org.duckstation.duckstation", "ppsspp", "org.ppsspp.ppsspp", "ryujinx",
    "org.ryujinx.ryujinx", "cemu", "info.cemu.cemu", "xemu", "app.xemu.xemu", "melonds", "net.kuribo64.melonds", "mgba",
    "io.mgba.mgba", "snes9x", "mednafen", "flycast", "org.flycast.flycast", "azahar", "io.github.lime3ds.lime3ds", "citra",
    "eden", "gamescope", "steam_proton", "hearth-game"];

var couch = false;
var couchOn = {};           // screens in Couch mode (each screen has its own mode: hearth-firehold-plasma mode)
var games = "follow";       // follow | fullscreen | window (Settings › Couch and Desk › Games)
var made = {};              // windows this script made full screen

function isGame(w) {
    if (!w || !w.normalWindow) return false;
    var c = String(w.resourceClass || "").toLowerCase();
    var n = String(w.resourceName || "").toLowerCase();
    return /^steam_app_\d+$/.test(c) || GAME_APPS.indexOf(c) >= 0 || GAME_APPS.indexOf(n) >= 0;
}

function key(w) { return String(w.internalId); }

function full(w) {
    if (games === "window" || !isGame(w) || w.fullScreen || !w.fullScreenable) return;
    w.fullScreen = true;
    made[key(w)] = true;
}

function setCouch(on) {
    couch = on;
    var list = workspace.windowList();
    for (var i = 0; i < list.length; i++) {
        var w = list[i];
        if (on) {
            if (w === workspace.activeWindow) full(w);
        } else if ((made[key(w)] || isGame(w)) && w.fullScreen && games === "follow") {
            w.fullScreen = false;
        }
    }
    if (!on) made = {};
}

// A game that opens on the Couch (or with Games: Always full screen) starts full screen. Going to a running game from
// the Couch is the Couch's own doing (it asks for full screen itself), so "Open in a window" stays a window.
function onCouchScreen(w) { return !!(w && w.output && couchOn[w.output.name]); }
// The Couch over the Desk on a Couch screen (the Desk bar's window "Firehold Couch on NAME"): on that screen, full
// screen, not in the task bar or the window switcher.
function placeCouch(w) {
    var m = /^Firehold Couch on (.+)$/.exec(String(w.caption || ""));
    if (!m) return false;
    var o = workspace.screens.filter(function (s) { return s.name === m[1]; })[0];
    if (o && w.output !== o) workspace.sendClientToScreen(w, o);
    w.skipTaskbar = true;
    w.skipSwitcher = true;
    w.skipPager = true;
    w.fullScreen = true;
    return true;
}
workspace.windowAdded.connect(function (w) {
    if (placeCouch(w)) return;
    // Its name can come a moment after the window (Qt sets the title after mapping): look again when it does.
    if (String(w.resourceClass || "").indexOf("plasmashell") >= 0 && w.captionChanged) {
        w.captionChanged.connect(function () { placeCouch(w); });
    }
    if (couch || onCouchScreen(w) || games === "fullscreen") full(w);
});

// Each screen's own mode: a game on a Couch screen goes full screen, back on a Desk screen it goes back in its window.
function setScreen(name, on) {
    couchOn[name] = on;
    var list = workspace.windowList();
    for (var i = 0; i < list.length; i++) {
        var w = list[i];
        if (!w.output || w.output.name !== name) continue;
        if (on) {
            if (w === workspace.activeWindow) full(w);
        } else if ((made[key(w)] || isGame(w)) && w.fullScreen && games === "follow") {
            w.fullScreen = false;
            delete made[key(w)];
        }
    }
}
var registered = {};
function screenShortcuts() {
    var s = workspace.screens;
    for (var i = 0; i < s.length; i++) {
        (function (name) {
            if (registered[name]) return;
            registered[name] = true;
            registerShortcut("Firehold Couch on " + name, "Firehold: Couch on " + name, "", function () { setScreen(name, true); });
            registerShortcut("Firehold Desk on " + name, "Firehold: Desk on " + name, "", function () { setScreen(name, false); });
        })(s[i].name);
    }
}
screenShortcuts();
workspace.screensChanged.connect(screenShortcuts);
workspace.windowRemoved.connect(function (w) { if (w) delete made[key(w)]; });

registerShortcut("Firehold Couch posture", "Firehold: Couch posture", "", function () { setCouch(true); });
registerShortcut("Firehold Desk posture", "Firehold: Desk posture", "", function () { setCouch(false); });
registerShortcut("Firehold games follow", "Firehold: games follow the posture", "", function () { games = "follow"; });
registerShortcut("Firehold games fullscreen", "Firehold: games always full screen", "", function () { games = "fullscreen"; });
// Do not disturb (Quick Settings): kept here so a game going full screen doesn't bring a pop-up over it while it's on.
var dnd = false;
// The Couch windows already open when this script starts (it can be loaded after plasmashell).
workspace.windowList().forEach(function (w) { placeCouch(w); });
registerShortcut("Firehold dnd on", "Firehold: Do not disturb on", "", function () { dnd = true; });
registerShortcut("Firehold dnd off", "Firehold: Do not disturb off", "", function () { dnd = false; });
registerShortcut("Firehold games window", "Firehold: games always in a window", "", function () { games = "window"; });

var GUIDE = 'var v="guide|"+Date.now();desktops().forEach(function(d){if(d.type=="org.hearthos.firehold.couch"){' +
    'd.currentConfigGroup=["General"];d.writeConfig("request",v);}});';
registerShortcut("Firehold Guide", "Firehold: the Guide", "Meta+Ctrl+G", function () {
    callDBus("org.kde.plasmashell", "/PlasmaShell", "org.kde.PlasmaShell", "evaluateScript", GUIDE);
});

// Super+Shift+C / D / A: the screen you're on to Couch, Desk or Away (mockup firehold-plasma 04's keys). KWin knows the
// screen; the Desk's mode pill on that screen (or the Couch there) asks hearth-firehold-plasma.
function modeKey(m) {
    var name = workspace.activeScreen ? workspace.activeScreen.name : "";
    var js = 'var v="' + m + '|' + name + '|"+Date.now();' +
        'panels().forEach(function(p){p.widgets().forEach(function(w){if(w.type=="org.hearthos.firehold.posture"){w.currentConfigGroup=["General"];w.writeConfig("key",v);}});});' +
        'desktops().forEach(function(d){if(d.type=="org.hearthos.firehold.couch"){d.currentConfigGroup=["General"];d.writeConfig("request","mode:' + m + '|"+Date.now());}});';
    callDBus("org.kde.plasmashell", "/PlasmaShell", "org.kde.PlasmaShell", "evaluateScript", js);
}
registerShortcut("Firehold this screen Couch", "Firehold: this screen to Couch", "Meta+Shift+C", function () { modeKey("couch"); });
registerShortcut("Firehold this screen Desk", "Firehold: this screen to Desk", "Meta+Shift+D", function () { modeKey("desk"); });
registerShortcut("Firehold this screen Away", "Firehold: this screen to Away", "Meta+Shift+A", function () { modeKey("away"); });

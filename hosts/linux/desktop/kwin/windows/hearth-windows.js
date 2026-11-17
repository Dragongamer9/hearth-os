// Hearth Desktop Mode on KWin: Mantel's window list, snapping, desktops and keyboard shortcuts (KWin script
// "hearthwindows"). KWin doesn't offer the window list protocol labwc does (wlr-foreign-toplevel), so Mantel's taskbar,
// dock, Task Manager, controller and wallpaper get their windows from here instead:
//
//   this script --Update(json)--> hearth-kwin-bridge (org.hearth.Windows) --stdout--> WinList.qml (the shell)
//   WinList.qml --stdin--> hearth-kwin-bridge --reply to Next()--> this script (activate, minimize, snap...)
//
// hearth-kwin-bridge fills in the %%...%% parts (shortcuts.json, snap-layouts.js) and loads this file through KWin's
// Scripting D-Bus API when the shell starts; it reloads it if the two stop talking. Without the bridge the calls just
// fail and KWin carries on as normal. Plain ES5 on purpose (KWin's script engine).
var SHORTCUTS = %%SHORTCUTS%%;
var ZONES = %%ZONES%%;
var GAP = %%GAP%%;
var RULES = %%RULES%%;
var BUS = "org.hearth.Windows", OBJ = "/org/hearth/Windows", IFACE = "org.hearth.Windows";

var order = [];       // the windows in the order they opened (the taskbar's order, as labwc gives it)
var hooked = {};      // id -> true once its signals are connected
var maxMode = {};     // id -> KWin's maximize mode from maximizedAboutToChange (0 none, 1 vertical, 2 horizontal, 3 both)
var ruled = {};       // id -> the rule that set keep-above, so it can be taken off again
var lastSent = "";

function idOf(w) { return String(w.internalId); }

// Into the bridge's log ($XDG_RUNTIME_DIR/hearth/kwin-bridge.log): KWin's own print() is usually not shown anywhere.
var said = {};
function say(text, once) {
    if (once) { if (said[text]) return; said[text] = true; }
    callDBus(BUS, OBJ, IFACE, "Log", String(text));
}

// What a taskbar shows: apps' normal windows and dialogs. Not Mantel's own bar, dock or flyouts: KWin gives layer-shell
// surfaces a window type from their namespace, Normal when it doesn't know it, but they can't be moved, resized or
// minimized, which no app window is.
function listed(w) {
    if (!w || w.deleted || w.skipTaskbar || w.specialWindow) return false;
    if (!(w.normalWindow || w.dialog)) return false;
    if (w.moveable === false && w.resizeable === false && w.minimizable === false) return false;
    return true;
}

function area(w) { return workspace.clientArea(KWin.MaximizeArea, w); }

function isMax(w) {
    if (typeof w.maximizeMode === "number") return w.maximizeMode === 3;
    var id = idOf(w);
    if (maxMode[id] !== undefined) return maxMode[id] === 3;
    var a = area(w), g = w.frameGeometry;
    return Math.abs(g.x - a.x) < 2 && Math.abs(g.y - a.y) < 2 && Math.abs(g.width - a.width) < 2 && Math.abs(g.height - a.height) < 2;
}

function deskIndex(d) {
    var all = workspace.desktops;
    for (var i = 0; i < all.length; i++) if (all[i] === d || (d && all[i].id === d.id)) return i + 1;
    return 0;
}

function info(w) {
    var desks = [], wd = w.desktops || [];
    for (var i = 0; i < wd.length; i++) desks.push(deskIndex(wd[i]));
    var parent = "";
    try { if (w.transientFor) parent = idOf(w.transientFor); } catch (e) {}
    return {
        id: idOf(w),
        appId: String(w.desktopFileName || w.resourceClass || ""),
        title: String(w.caption || ""),
        activated: workspace.activeWindow === w,
        minimized: w.minimized === true,
        maximized: isMax(w),
        fullscreen: w.fullScreen === true,
        keepAbove: w.keepAbove === true,
        desktops: desks,
        allDesktops: w.onAllDesktops === true,
        output: w.output ? String(w.output.name || "") : "",
        pid: w.pid || 0,
        dialog: w.dialog === true,
        parent: parent,
        zone: zoneOf(w)
    };
}

function snapshot() {
    var wins = [];
    for (var i = 0; i < order.length; i++) if (listed(order[i])) wins.push(info(order[i]));
    var ds = workspace.desktops, cur = workspace.currentDesktop, desks = [];
    for (var j = 0; j < ds.length; j++)
        desks.push({ name: String(ds[j].name || ("Desktop " + (j + 1))), active: !!cur && ds[j].id === cur.id });
    var a = workspace.activeWindow;
    return JSON.stringify({ windows: wins, desktops: desks, active: a && listed(a) ? idOf(a) : "" });
}

function push() {
    var s;
    try { s = snapshot(); } catch (e) { say("list: " + e, true); return; }
    if (s === lastSent) return;
    lastSent = s;
    callDBus(BUS, OBJ, IFACE, "Update", s);
}

// ------------------------------------------------------------------ snapping (the same zones as labwc's regions)
function zoneById(id) {
    for (var i = 0; i < ZONES.length; i++) if (ZONES[i].id === id) return ZONES[i];
    return null;
}

// A zone's place on the window's screen, with the same gap labwc leaves (rc.xml <gap>): GAP at the screen's edges and
// between two windows.
function rectFor(w, z) {
    var a = area(w), h = GAP / 2;
    var x0 = a.x + Math.round(a.width * z.x / 100), y0 = a.y + Math.round(a.height * z.y / 100);
    var x1 = a.x + Math.round(a.width * (z.x + z.w) / 100), y1 = a.y + Math.round(a.height * (z.y + z.h) / 100);
    x0 += z.x <= 0 ? GAP : h; y0 += z.y <= 0 ? GAP : h;
    x1 -= z.x + z.w >= 100 ? GAP : h; y1 -= z.y + z.h >= 100 ? GAP : h;
    return { x: x0, y: y0, width: Math.max(1, x1 - x0), height: Math.max(1, y1 - y0) };
}

// Which zone the window is in now, whoever put it there (Mantel, or KWin's own tiling when it's dragged to an edge,
// which leaves no gap): "" when none.
function zoneOf(w) {
    if (w.fullScreen || isMax(w)) return "";
    var g = w.frameGeometry, tol = GAP * 2 + 4;
    for (var i = 0; i < ZONES.length; i++) {
        var r = rectFor(w, ZONES[i]);
        if (Math.abs(g.x - r.x) <= tol && Math.abs(g.y - r.y) <= tol && Math.abs(g.width - r.width) <= tol * 2 && Math.abs(g.height - r.height) <= tol * 2)
            return ZONES[i].id;
    }
    return "";
}

// KWin takes a new place as a rect: a plain object, or (if this KWin won't take one) the rect it gave us with new
// numbers. The window gets there once the app has drawn itself at the new size, so it isn't checked here.
function setGeometry(w, r) {
    try {
        w.frameGeometry = { x: r.x, y: r.y, width: r.width, height: r.height };
        return true;
    } catch (e) {
        say("geometry: a plain rect was refused (" + e + "); trying KWin's own", true);
    }
    try {
        var g = w.frameGeometry;
        g.x = r.x; g.y = r.y; g.width = r.width; g.height = r.height;
        w.frameGeometry = g;
        return true;
    } catch (e2) {
        say("geometry: " + e2, true);
        return false;
    }
}

// Maximize or restore: KWin's setMaximize(vertically, horizontally), or its Maximize action on the window in front.
function setMax(w, on) {
    if (typeof w.setMaximize === "function") { w.setMaximize(on, on); return; }
    say("maximize: no setMaximize here (" + typeof w.setMaximize + "); using KWin's Maximize action", true);
    if (isMax(w) === on) return;
    workspace.activeWindow = w;
    workspace.slotWindowMaximize();
}

function snap(w, id) {
    var z = zoneById(id);
    if (!w || !z) return;
    if (w.fullScreen) w.fullScreen = false;
    if (isMax(w)) setMax(w, false);
    w.minimized = false;
    setGeometry(w, rectFor(w, z));
    workspace.activeWindow = w;
}

// Super+arrows the way labwc's rc.xml has them: Left/Right halves; Up on a half makes it the top quarter, on a bottom
// quarter the half again, otherwise maximizes; Down on a half makes it the bottom quarter, on a top quarter the half,
// on a maximized window restores it, otherwise minimizes. One press, one change.
function winKey(k) {
    var w = workspace.activeWindow;
    if (k.indexOf("go-desktop:") === 0) { goDesktop(parseInt(k.slice(11), 10)); return; }
    if (!w || !listed(w)) return;
    var z = zoneOf(w);
    if (k === "left") snap(w, "left-half");
    else if (k === "right") snap(w, "right-half");
    else if (k === "up") {
        if (z === "left-half") snap(w, "top-left");
        else if (z === "right-half") snap(w, "top-right");
        else if (z === "bottom-left") snap(w, "left-half");
        else if (z === "bottom-right") snap(w, "right-half");
        else setMax(w, true);
    } else if (k === "down") {
        if (z === "left-half") snap(w, "bottom-left");
        else if (z === "right-half") snap(w, "bottom-right");
        else if (z === "top-left") snap(w, "left-half");
        else if (z === "top-right") snap(w, "right-half");
        else if (isMax(w)) setMax(w, false);
        else w.minimized = true;
    } else if (k === "close") w.closeWindow();
    else if (k.indexOf("to-desktop:") === 0) toDesktop(w, parseInt(k.slice(11), 10));
}

// ------------------------------------------------------------------ virtual desktops
function goDesktop(n) {
    var d = workspace.desktops[n - 1];
    if (d) workspace.currentDesktop = d;
}

// To desktop n without following it (labwc's SendToDesktop follow="no").
function toDesktop(w, n) {
    var d = workspace.desktops[n - 1];
    if (!w || !d) return;
    w.onAllDesktops = false;
    w.desktops = [d];
}

// ------------------------------------------------------------------ what Mantel asks for
function find(id) {
    for (var i = 0; i < order.length; i++) if (idOf(order[i]) === id) return order[i];
    return null;
}

function run(c) {
    var w = c.id ? find(c.id) : workspace.activeWindow;
    switch (c.op) {
    case "activate": if (w) { w.minimized = false; workspace.activeWindow = w; } break;
    case "close": if (w) w.closeWindow(); break;
    case "minimize": if (w) w.minimized = !!c.on; break;
    case "maximize": if (w) { if (c.on) w.minimized = false; setMax(w, !!c.on); } break;
    case "fullscreen": if (w) w.fullScreen = !!c.on; break;
    case "keepAbove": if (w) w.keepAbove = !!c.on; break;
    case "allDesktops": if (w) w.onAllDesktops = !!c.on; break;
    case "snap": snap(w, c.zone); break;
    case "desktop": toDesktop(w, c.n); break;
    case "goDesktop": goDesktop(c.n); break;
    case "key": winKey(String(c.key || "")); break;
    case "menu": workspace.slotWindowOperations(); break;        // KWin's window menu (with Hearth's Snap), for checks
    case "refresh": lastSent = ""; break;
    case "probe":                                                 // what this KWin offers, into the log (checks)
        if (w) say("probe: " + JSON.stringify({ setMaximize: typeof w.setMaximize, maximizeMode: w.maximizeMode, move: typeof w.move,
            frame: { x: w.frameGeometry.x, y: w.frameGeometry.y, width: w.frameGeometry.width, height: w.frameGeometry.height },
            area: (function () { var a = area(w); return { x: a.x, y: a.y, width: a.width, height: a.height }; })(),
            moveable: w.moveable, resizeable: w.resizeable, tile: typeof w.tile }));
        break;
    }
}

// The bridge answers Next() when it has something for us (or after 20 s with nothing): ask again each time.
function poll() {
    callDBus(BUS, OBJ, IFACE, "Next", function (reply) {
        var list = [];
        try { list = JSON.parse(String(reply)); } catch (e) {}
        for (var i = 0; i < list.length; i++) {
            try { run(list[i]); } catch (e) { say(list[i].op + ": " + e); }
        }
        push();
        poll();
    });
}

// ------------------------------------------------------------------ keeping track of the windows
function applyRules(w) {
    var id = idOf(w), cap = String(w.caption || "");
    for (var i = 0; i < RULES.length; i++) {
        if (RULES[i].title === cap) {
            if (RULES[i].keepAbove && !w.keepAbove) { w.keepAbove = true; ruled[id] = true; }
            return;
        }
    }
    if (ruled[id]) { delete ruled[id]; w.keepAbove = false; }     // the title it was on top for has gone
}

function hook(w) {
    if (!w) return;
    var id = idOf(w);
    if (hooked[id]) return;
    hooked[id] = true;
    order.push(w);
    var sigs = ["captionChanged", "minimizedChanged", "maximizedChanged", "fullScreenChanged", "desktopsChanged",
                "skipTaskbarChanged", "outputChanged", "keepAboveChanged", "desktopFileNameChanged", "windowClassChanged",
                "activeChanged", "frameGeometryChanged", "onAllDesktopsChanged"];
    for (var i = 0; i < sigs.length; i++) {
        if (w[sigs[i]] && w[sigs[i]].connect) w[sigs[i]].connect(push);
    }
    if (w.maximizedAboutToChange) w.maximizedAboutToChange.connect(function (m) { maxMode[id] = m; });
    if (w.captionChanged) w.captionChanged.connect(function () { applyRules(w); });
    applyRules(w);
}

function unhook(w) {
    var id = idOf(w);
    for (var i = 0; i < order.length; i++) if (order[i] === w) { order.splice(i, 1); break; }
    delete hooked[id]; delete maxMode[id]; delete ruled[id];
    push();
}

// ------------------------------------------------------------------ Mantel's shortcuts and title bar menu
function registerShortcuts() {
    for (var i = 0; i < SHORTCUTS.length; i++) {
        (function (s) {
            registerShortcut("Hearth " + s.id, "Hearth: " + s.title, s.keys, function () {
                if (s.win) { winKey(s.win); push(); }
                else callDBus(BUS, OBJ, IFACE, "Shortcut", s.id);
            });
        })(SHORTCUTS[i]);
    }
}

// Right-click on a title bar (or Alt+F3): KWin's window menu gets Hearth's Snap, with Snap layouts at the top.
var MENU_ZONES = ["left-half", "right-half", "top-left", "top-right", "bottom-left", "bottom-right", "center"];
registerUserActionsMenu(function (w) {
    if (!listed(w)) return [];
    var items = [{ text: "Snap layouts…", title: "Snap layouts…", triggered: function () {
        workspace.activeWindow = w;
        callDBus(BUS, OBJ, IFACE, "Shortcut", "snap-layouts-open");
    } }];
    var here = zoneOf(w);
    for (var i = 0; i < MENU_ZONES.length; i++) {
        (function (id) {
            var z = zoneById(id);
            if (!z) return;
            items.push({ text: z.label, title: z.label, checkable: true, checked: here === id, triggered: function () { snap(w, id); push(); } });
        })(MENU_ZONES[i]);
    }
    return { text: "Snap", title: "Snap", items: items };
});

// ------------------------------------------------------------------ start
var start = workspace.stackingOrder;
for (var s = 0; s < start.length; s++) hook(start[s]);
workspace.windowAdded.connect(function (w) { hook(w); push(); });
workspace.windowRemoved.connect(unhook);
workspace.windowActivated.connect(function () { push(); });
if (workspace.currentDesktopChanged) workspace.currentDesktopChanged.connect(function () { push(); });
if (workspace.desktopsChanged) workspace.desktopsChanged.connect(function () { push(); });
registerShortcuts();
push();
poll();

pragma Singleton

// The open windows, for every part of Mantel that shows or moves them (the taskbar, the dock, Task Manager, snap
// layouts, the controller, the wallpaper, Discord's quick window...), whichever window manager is underneath:
//
//   labwc (HEARTH_WM unset or "labwc"): Quickshell's ToplevelManager as before (the wlr-foreign-toplevel protocol).
//   KWin  (HEARTH_WM=kwin): KWin has no such protocol, so this starts hearth-kwin-bridge, which loads KWin's
//         hearthwindows script (desktop/kwin/windows) and passes the list both ways. Each window is a Win below with
//         the same names ToplevelManager's have (appId, title, activated, minimized, maximized, fullscreen, screens,
//         activate(), close(), and minimized/maximized/fullscreen can be set), so code written for one works on both.
//   Firehold (HEARTH_WM=firehold, the 2.0 preview): Firehold's own compositor speaks the taskbar protocol too, and
//         its window IPC (firehold-comp ipc.rs) gives what KWin's script does: ids, frames, stacking, desktops and
//         window pictures. This starts hearth-firehold-bridge for it; the list comes from there once it's talking
//         (fhReady), and from ToplevelManager until then or if it stops.
//
// Use WinList.toplevels.values and WinList.activeToplevel where ToplevelManager's were used. Things labwc did through
// its own keys (typed with wtype, which KWin doesn't take) are functions here: snap(), moveToDesktop(), goDesktop().
//   quickshell ipc -p <shell folder> call winlist state|do <json>
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Singleton {
    id: wl

    readonly property string wm: Quickshell.env("HEARTH_WM") || "labwc"
    readonly property bool kwin: wm === "kwin"
    readonly property bool firehold: wm === "firehold"
    // Firehold's compositor is talking and its list has come (hearth-firehold-bridge).
    property bool fhReady: false
    // The window manager tells Mantel about its windows itself (KWin's script or Firehold's IPC): ids, frames, desktops.
    readonly property bool bridged: kwin || fhReady
    // Virtual desktops through WinList (desktops, goDesktop, moveToDesktop); labwc's come through ext-workspace.
    readonly property bool wmDesktops: bridged
    // Pictures of single windows (Everything open's cards, the dock's previews): KWin's or Firehold's.
    readonly property bool pictures: bridged
    // The window stacking order from the window manager (Firehold's IPC): ids, top first.
    property var stack: []
    signal shotTaken(var info)          // Firehold: a window's picture is ready ({id, ok, path, w, h})

    readonly property var toplevels: bridged ? kList : ToplevelManager.toplevels
    readonly property var activeToplevel: bridged ? kActive : ToplevelManager.activeToplevel
    // KWin's virtual desktops, in order: [{ name, active }] (labwc's come through Quickshell's WindowManager instead).
    property var desktops: []
    property bool connected: false      // KWin's script and the bridge are talking
    property string problem: ""         // why not, in words (Settings › About › Desktop Mode can show it)
    // KWin only (labwc can't do these, so Mantel hides them there):
    property bool previewsReady: false  // KWin's previews script is there: the dock and taskbar show live previews
    property bool captureHide: false    // this KWin paints a window hidden from sharing out of whole-screen shares too
    signal picked(var info)             // pickWindow()'s answer: {id, title, appId, pid} or {cancel: true}
    signal killed(var info)             // forceClose()'s: {id, ok, how|why}
    signal forceCloseAsked(var info)    // the title bar menu's "Force close…": {id, title, appId, pid}
    signal windowRulesAsked(var info)   // its More actions › "Special window settings…": {id, wmclass, title}

    QtObject { id: kList; property var values: [] }
    property var kActive: null
    property var byId: ({})
    property string lastIds: ""
    property string lastDesks: ""

    component Win: QtObject {
        property var hub: null
        property string wid: ""
        property string appId: ""
        property string title: ""
        property bool activated: false
        property bool minimized: false
        property bool maximized: false
        property bool fullscreen: false
        property bool keepAbove: false
        property var desktops: []           // 1-based desktop numbers it's on
        property bool allDesktops: false
        property string output: ""          // the screen's name (KWin's output)
        property int pid: 0
        property bool dialog: false
        property string parentId: ""        // the window it's a dialog for
        property string zone: ""            // the snap zone it's in ("left-half", "top-right"...), "" when none
        property var frame: ({})            // KWin's frame: {x, y, w, h}
        property bool noShare: false        // hidden from screen sharing (the title bar menu)
        property int stack: -1              // Firehold: 0 is the top window, -1 not shown (minimized, another desktop)
        property bool ssd: true             // Firehold: the compositor draws its frame
        property bool syncing: false        // set while KWin's own values come in, so they aren't sent back
        readonly property var screens: {
            const s = Quickshell.screens.filter(x => x.name === output);
            return s.length ? s : (Quickshell.screens.length ? [Quickshell.screens[0]] : []);
        }
        onMinimizedChanged: if (!syncing && hub) hub.send({ op: "minimize", id: wid, on: minimized })
        onMaximizedChanged: if (!syncing && hub) hub.send({ op: "maximize", id: wid, on: maximized })
        onFullscreenChanged: if (!syncing && hub) hub.send({ op: "fullscreen", id: wid, on: fullscreen })
        readonly property string kwinId: wid      // KWin's own id for the window (Activities keeps windows by it)
        function setMinimized(on) { minimized = !!on; }
        function activate() { if (hub) hub.send({ op: "activate", id: wid }); }
        function close() { if (hub) hub.send({ op: "close", id: wid }); }
    }
    Component { id: winMaker; Win {} }

    // ------------------------------------------------------------------ what Mantel asks for
    function send(cmd) {
        if (kwin && bridge.running) { bridge.write(JSON.stringify(cmd) + "\n"); return true; }
        if (firehold && fbridge.running) { fbridge.write(JSON.stringify(cmd) + "\n"); return true; }
        return false;
    }
    // Firehold: a desktop more (its name, or "Desktop N"), one fewer (its windows go to the one before).
    function addDesktop(name) { return firehold && send({ op: "addDesktop", name: name || "" }); }
    function removeDesktop(n) { return firehold && send({ op: "removeDesktop", n: n }); }
    // Firehold: a window's picture as a PNG at `path` (in $XDG_RUNTIME_DIR), at most `max` wide; shotTaken says when.
    function shot(w, path, max) { return firehold && send({ op: "shot", id: widOf(w), path: path, max: max || 640 }); }
    function widOf(w) { return w && w.wid !== undefined ? w.wid : ""; }
    // Snap a window to a zone of the snap layouts (snap-layouts.js ids: "left-half", "top-right", "left-third"...).
    function snap(w, zone) { return send({ op: "snap", id: widOf(w), zone: zone }); }
    // Send a window to desktop n (1-based) without following it.
    function moveToDesktop(w, n) { return send({ op: "desktop", id: widOf(w), n: n }); }
    function goDesktop(n) { return send({ op: "goDesktop", n: n }); }
    // Put a window at a place and size of Mantel's choosing ({x, y, w, h}; Discord's quick window).
    function place(w, g) { return !!g && send({ op: "place", id: widOf(w), x: Math.round(g.x), y: Math.round(g.y), w: Math.round(g.w), h: Math.round(g.h) }); }
    function setKeepAbove(w, on) { return send({ op: "keepAbove", id: widOf(w), on: !!on }); }
    // Hide a window from screen sharing (Hearth's portal leaves it out; KWin paints it out too when it can).
    function setNoShare(w, on) { return send({ op: "noShare", id: widOf(w), on: !!on }); }
    // KWin's own views: "overview", "windows" (Present windows: every window side by side), "grid" (Desktop grid),
    // "appwindows" (the app in front's windows).
    function effect(name) { return send({ op: "effect", name: name }); }
    // Live previews of these windows by the button the pointer rests on (KWin draws them: hearth-previews.qml).
    // edge: the bar's edge; near: how far the pointer is from the button's side facing the screen; along: from its
    // middle; key: the button (asking again for the same one changes nothing).
    function preview(wins, edge, near, along, key) {
        if (!kwin || !previewsReady) return false;
        return send({ op: "preview", ids: wins.map(w => widOf(w)).filter(i => i !== ""), edge: edge || "bottom",
                      near: Math.round(near || 0), along: Math.round(along || 0), key: key || "" });
    }
    function previewHide(now) { return kwin && previewsReady && send({ op: "previewHide", now: !!now }); }
    // Force closing a frozen window: KWin's click-a-window (picked), then its process ended (killed).
    function pickWindow() { return send({ op: "pick" }); }
    function forceClose(id) { return send({ op: "kill", id: id }); }

    // ------------------------------------------------------------------ the list from the bridge
    function apply(s) {
        const list = [], seen = {};
        const map = byId;
        for (const d of s.windows || []) {
            let w = map[d.id];
            if (!w) {
                w = winMaker.createObject(wl, { hub: wl, wid: d.id });
                map[d.id] = w;
            }
            w.syncing = true;
            w.appId = d.appId || "";
            w.title = d.title || "";
            w.activated = d.activated === true;
            w.minimized = d.minimized === true;
            w.maximized = d.maximized === true;
            w.fullscreen = d.fullscreen === true;
            w.keepAbove = d.keepAbove === true;
            if (JSON.stringify(w.desktops) !== JSON.stringify(d.desktops || [])) w.desktops = d.desktops || [];
            w.allDesktops = d.allDesktops === true;
            w.output = d.output || "";
            w.pid = d.pid || 0;
            w.dialog = d.dialog === true;
            w.parentId = d.parent || "";
            w.zone = d.zone || "";
            w.noShare = d.noShare === true;
            w.stack = typeof d.stack === "number" ? d.stack : -1;
            w.ssd = d.ssd !== false;
            if (JSON.stringify(w.frame) !== JSON.stringify(d.frame || {})) w.frame = d.frame || {};
            w.syncing = false;
            seen[d.id] = true;
            list.push(w);
        }
        const gone = [];
        for (const k in map) if (!seen[k]) gone.push(k);
        const ids = list.map(w => w.wid).join("\n");
        if (ids !== lastIds) { lastIds = ids; kList.values = list; }
        const act = map[s.active] || null;
        if (kActive !== act) kActive = act;
        for (const k of gone) { const w = map[k]; delete map[k]; w.destroy(1000); }   // a moment for anyone holding it
        captureHide = s.captureHide === true;
        const ds = JSON.stringify(s.desktops || []);
        if (ds !== lastDesks) { lastDesks = ds; desktops = s.desktops || []; }
        if (JSON.stringify(stack) !== JSON.stringify(s.stack || [])) stack = s.stack || [];
    }

    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-kwin-bridge").toString().replace(/^file:\/\//, ""))
    Process {
        id: bridge
        command: ["python3", wl.helper, "serve", "--stdio"]
        running: wl.kwin
        stdinEnabled: true
        stdout: SplitParser {
            onRead: line => {
                let m;
                try { m = JSON.parse(line); } catch (e) { return; }
                if (m.t === "windows") wl.apply(m);
                else if (m.t === "status") {
                    wl.connected = m.connected === true;
                    wl.problem = m.problem || "";
                    if (!wl.connected && wl.problem) console.warn("window list (KWin): " + wl.problem);
                } else if (m.t === "log") console.warn("window list (KWin): " + (m.text || ""));
                else if (m.t === "previews") wl.previewsReady = m.ready === true;
                else if (m.t === "picked") wl.picked(m);
                else if (m.t === "killed") wl.killed(m);
                else if (m.t === "forceClose") wl.forceCloseAsked(m);
                else if (m.t === "windowRules") wl.windowRulesAsked(m);
            }
        }
        onExited: code => {
            wl.connected = false;
            wl.previewsReady = false;
            if (wl.kwin) again.start();
        }
    }
    Timer { id: again; interval: 3000; onTriggered: bridge.running = wl.kwin }

    // Firehold's compositor (HEARTH_WM=firehold): the same lines, from its window IPC.
    readonly property string fhHelper: decodeURIComponent(Qt.resolvedUrl("hearth-firehold-bridge").toString().replace(/^file:\/\//, ""))
    Process {
        id: fbridge
        command: ["python3", wl.fhHelper, "serve", "--stdio"]
        running: wl.firehold
        stdinEnabled: true
        stdout: SplitParser {
            onRead: line => {
                let m;
                try { m = JSON.parse(line); } catch (e) { return; }
                if (m.t === "windows") { wl.apply(m); wl.connected = true; wl.fhReady = true; }
                else if (m.t === "status") {
                    wl.problem = m.problem || "";
                    if (!m.connected) { wl.fhReady = false; wl.connected = false; if (wl.problem) console.warn("window list (Firehold): " + wl.problem); }
                } else if (m.t === "shot") wl.shotTaken(m);
                else if (m.t === "reply" && m.ok === false && m.op !== "shot") console.warn("window list (Firehold): " + (m.op || "") + ": " + (m.error || ""));
            }
        }
        onExited: code => { wl.fhReady = false; wl.connected = false; if (wl.firehold) fagain.start(); }
    }
    Timer { id: fagain; interval: 3000; onTriggered: fbridge.running = wl.firehold }

    IpcHandler {
        target: "winlist"
        function state(): string {
            return JSON.stringify({
                wm: wl.wm, connected: wl.connected, problem: wl.problem,
                active: wl.activeToplevel ? (wl.activeToplevel.title || "") : "",
                windows: (wl.toplevels ? wl.toplevels.values : []).map(t => ({
                    appId: t.appId || "", title: t.title || "", activated: t.activated === true, minimized: t.minimized === true,
                    maximized: t.maximized === true, fullscreen: t.fullscreen === true, zone: t.zone || "", desktops: t.desktops || [],
                    noShare: t.noShare === true, id: wl.widOf(t), stack: t.stack === undefined ? -1 : t.stack
                })),
                desktops: wl.desktops, previews: wl.previewsReady, captureHide: wl.captureHide,
                bridged: wl.bridged, pictures: wl.pictures, stack: wl.stack
            });
        }
        // A command as WinList sends it, for checks: {"op":"snap","title":"…","zone":"left-half"} (title picks the window).
        function run(json: string): bool {
            let c;
            try { c = JSON.parse(json); } catch (e) { return false; }
            if (c.title !== undefined) {
                const w = (wl.toplevels ? wl.toplevels.values : []).find(t => t.title === c.title);
                if (!w) return false;
                c.id = wl.widOf(w);
                delete c.title;
            }
            return wl.send(c);
        }
        // Live previews of the window with this title ("*": every window), at a place on screen (checks: x, y in KWin's coordinates, the
        // card's bottom middle), the way the dock asks for them.
        function previewAt(title: string, x: int, y: int): bool {
            const w = (wl.toplevels ? wl.toplevels.values : []).filter(t => title === "*" || t.title === title);
            if (!w.length || !wl.kwin) return false;
            return wl.send({ op: "preview", ids: w.map(t => wl.widOf(t)), at: { x: x, y: y }, key: "check" });
        }
        function previewHide(): bool { return wl.previewHide(true); }
        function effect(name: string): bool { return wl.effect(name); }
        // The way the taskbar does it (Win's own setters and functions), for checks.
        function act(title: string, what: string): bool {
            const w = (wl.toplevels ? wl.toplevels.values : []).find(t => t.title === title);
            if (!w) return false;
            if (what === "activate") w.activate();
            else if (what === "close") w.close();
            else if (what === "minimize") w.minimized = true;
            else if (what === "maximize") w.maximized = true;
            else if (what === "restore") w.maximized = false;
            else return false;
            return true;
        }
    }
}

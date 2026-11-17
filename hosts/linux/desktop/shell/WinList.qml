pragma Singleton

// The open windows, for every part of Mantel that shows or moves them (the taskbar, the dock, Task Manager, snap
// layouts, the controller, the wallpaper, Discord's quick window...), whichever window manager is underneath:
//
//   labwc (HEARTH_WM unset or "labwc"): Quickshell's ToplevelManager as before (the wlr-foreign-toplevel protocol).
//   KWin  (HEARTH_WM=kwin): KWin has no such protocol, so this starts hearth-kwin-bridge, which loads KWin's
//         hearthwindows script (desktop/kwin/windows) and passes the list both ways. Each window is a Win below with
//         the same names ToplevelManager's have (appId, title, activated, minimized, maximized, fullscreen, screens,
//         activate(), close(), and minimized/maximized/fullscreen can be set), so code written for one works on both.
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

    readonly property var toplevels: kwin ? kList : ToplevelManager.toplevels
    readonly property var activeToplevel: kwin ? kActive : ToplevelManager.activeToplevel
    // KWin's virtual desktops, in order: [{ name, active }] (labwc's come through Quickshell's WindowManager instead).
    property var desktops: []
    property bool connected: false      // KWin's script and the bridge are talking
    property string problem: ""         // why not, in words (Settings › About › Desktop Mode can show it)

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
        property bool syncing: false        // set while KWin's own values come in, so they aren't sent back
        readonly property var screens: {
            const s = Quickshell.screens.filter(x => x.name === output);
            return s.length ? s : (Quickshell.screens.length ? [Quickshell.screens[0]] : []);
        }
        onMinimizedChanged: if (!syncing && hub) hub.send({ op: "minimize", id: wid, on: minimized })
        onMaximizedChanged: if (!syncing && hub) hub.send({ op: "maximize", id: wid, on: maximized })
        onFullscreenChanged: if (!syncing && hub) hub.send({ op: "fullscreen", id: wid, on: fullscreen })
        function activate() { if (hub) hub.send({ op: "activate", id: wid }); }
        function close() { if (hub) hub.send({ op: "close", id: wid }); }
    }
    Component { id: winMaker; Win {} }

    // ------------------------------------------------------------------ what Mantel asks for
    function send(cmd) {
        if (!kwin || !bridge.running) return false;
        bridge.write(JSON.stringify(cmd) + "\n");
        return true;
    }
    function widOf(w) { return w && w.wid !== undefined ? w.wid : ""; }
    // Snap a window to a zone of the snap layouts (snap-layouts.js ids: "left-half", "top-right", "left-third"...).
    function snap(w, zone) { return send({ op: "snap", id: widOf(w), zone: zone }); }
    // Send a window to desktop n (1-based) without following it.
    function moveToDesktop(w, n) { return send({ op: "desktop", id: widOf(w), n: n }); }
    function goDesktop(n) { return send({ op: "goDesktop", n: n }); }
    function setKeepAbove(w, on) { return send({ op: "keepAbove", id: widOf(w), on: !!on }); }

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
        const ds = JSON.stringify(s.desktops || []);
        if (ds !== lastDesks) { lastDesks = ds; desktops = s.desktops || []; }
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
            }
        }
        onExited: code => {
            wl.connected = false;
            if (wl.kwin) again.start();
        }
    }
    Timer { id: again; interval: 3000; onTriggered: bridge.running = wl.kwin }

    IpcHandler {
        target: "winlist"
        function state(): string {
            return JSON.stringify({
                wm: wl.wm, connected: wl.connected, problem: wl.problem,
                active: wl.activeToplevel ? (wl.activeToplevel.title || "") : "",
                windows: (wl.toplevels ? wl.toplevels.values : []).map(t => ({
                    appId: t.appId || "", title: t.title || "", activated: t.activated === true, minimized: t.minimized === true,
                    maximized: t.maximized === true, fullscreen: t.fullscreen === true, zone: t.zone || "", desktops: t.desktops || []
                })),
                desktops: wl.desktops
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

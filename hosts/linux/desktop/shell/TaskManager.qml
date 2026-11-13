// Task Manager, the Windows 11 way: a title bar with search, a left rail (Processes, Performance, Startup apps,
// Details, Services, Settings at the bottom), the process list with heat-map columns, the Performance graphs, your
// start-up apps and services. A normal window like Hearth Settings; the page itself is TaskManagerView.qml, loaded by
// file name, so a problem in it only loses Task Manager's page (a short note shows instead), never the desktop.
// Mouse, touch, keyboard and the controller (hearth-pad's keys while it's in front: A Enter, B Esc, X Menu):
//   Up / Down pick a page in the rail, Right / A go into it, Left / B come back, B on the rail closes; in a list A or X
//   opens the row's menu (End task, Efficiency mode, Open file location, Search online, Properties).
// The numbers come from `hearth-tasks serve` (next to this file), one sampler for every page, only while it's open.
// Your choices (start page, update speed, always on top, column widths, sort, the last page, the rail folded) are kept
// in ~/.config/nimbus/taskmgr.json through SettingsWriter (only what you change; never defaults; docs/SETTINGS-PERSISTENCE.md).
// Opened from the app launcher, Ctrl+Shift+Esc, or:
//   quickshell ipc -p <shell folder> call tasks open
//   quickshell ipc -p <shell folder> call tasks tab <processes|performance[:cpu|memory|gpu|disk|net]|startup|details|services|settings>
//   (the old names apps, perf and startup still work)
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: tm
    property bool shown: false
    property string page: "processes"
    property string perfDevice: "cpu"
    readonly property var pageIds: ["processes", "performance", "history", "startup", "details", "services", "tasks", "users", "settings"]
    readonly property var aliases: ({ apps: "processes", perf: "performance", tasks: "processes", startups: "startup" })
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-tasks").toString().replace(/^file:\/\//, ""))

    // ------------------------------------------------ your choices (taskmgr.json)
    readonly property string prefsPath: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/taskmgr.json"
    property var prefs: ({})
    function pref(k, d) { const v = prefs[k]; return v === undefined || v === null ? d : v; }
    // Only a change the person made is written, and only that key (merged into the latest file under its lock).
    function setPref(k, v) {
        if (JSON.stringify(prefs[k]) === JSON.stringify(v)) return;
        const p = Object.assign({}, prefs);
        p[k] = v;
        prefs = p;
        const c = {};
        c[k] = v;
        writer.merge(c);
    }
    SettingsWriter { id: writer; path: tm.prefsPath }
    FileView {
        id: prefsFile
        path: tm.prefsPath
        blockLoading: true
        printErrors: false
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            const t = text();
            let d = null;
            try { d = JSON.parse(t); } catch (e) { d = null; }
            if (d === null || typeof d !== "object" || Array.isArray(d)) {
                // Damaged: put back from taskmgr.json.bak (told once); nothing is written over it from here.
                if (t.trim() !== "") writer.repair();
                return;
            }
            for (const k in tm.prefs) if (writer.owns(k)) d[k] = tm.prefs[k];
            tm.prefs = d;
        }
    }

    readonly property string speed: ["high", "normal", "low", "paused"].indexOf(pref("speed", "normal")) >= 0 ? pref("speed", "normal") : "normal"
    readonly property real interval: speed === "high" ? 0.5 : speed === "low" ? 4 : speed === "paused" ? 0 : 1
    readonly property bool onTop: pref("onTop", false) === true
    readonly property bool railFolded: pref("railFolded", false) === true
    readonly property string startPage: {
        const s = pref("startPage", "processes");
        if (s === "last") { const l = pref("lastPage", "processes"); return pageIds.indexOf(l) >= 0 && l !== "settings" ? l : "processes"; }
        return pageIds.indexOf(s) >= 0 ? s : "processes";
    }

    function resolve(p) {
        const parts = String(p || "").split(":");
        const id = aliases[parts[0]] || parts[0];
        return { id: pageIds.indexOf(id) >= 0 ? id : "", device: parts[1] || "" };
    }
    // From IPC or the launcher: shows a page without remembering it (only the person's own moves are kept).
    function open(p) {
        const r = resolve(p);
        page = r.id !== "" ? r.id : (shown ? page : startPage);
        if (r.device !== "") perfDevice = r.device;
        shown = true;
    }
    // The person moved to a page (rail, keys): remembered as the last page.
    function go(id) {
        if (pageIds.indexOf(id) < 0) return;
        page = id;
        if (id !== "settings") setPref("lastPage", id);
    }
    function close() { shown = false; }
    // For checks (desktop-test.yml): the selected row's menu on the page shown, and a search.
    property var view: null
    function menu() { if (view && view.item && view.item.ipcMenu) view.item.ipcMenu(); }
    function search(q) { if (view && view.item && view.item.ipcSearch) view.item.ipcSearch(q); }
    function toggle() { shown ? close() : open(""); }
    // Always on top is a labwc window rule on the window's title (labwc/rc.xml); it takes effect when the window
    // opens, so changing it opens the window again.
    function reopen() { if (!shown) return; shown = false; Qt.callLater(() => tm.shown = true); }

    LazyLoader {
        active: tm.shown
        FloatingWindow {
            id: win
            title: tm.onTop ? "Task Manager (always on top)" : "Task Manager"
            implicitWidth: 1180
            implicitHeight: 760
            color: Theme.bg
            visible: true
            onVisibleChanged: if (!visible) tm.shown = false
            Component.onCompleted: tm.view = view
            Component.onDestruction: if (tm.view === view) tm.view = null

            Loader {
                id: view
                anchors.fill: parent
                focus: true
                source: "TaskManagerView.qml"
                onStatusChanged: if (status === Loader.Error) console.warn("Task Manager: its page (TaskManagerView.qml) couldn't load")
                onLoaded: if (item && item.start) item.start()
            }
            Text {
                anchors.centerIn: parent
                visible: view.status === Loader.Error
                width: parent.width - 80
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: "Task Manager couldn't load its page. Send logs from Settings › Help so it can be fixed."
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
            }
        }
    }
}

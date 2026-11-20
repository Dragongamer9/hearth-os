pragma Singleton

// The windows and desktops for Mantel's bar and dock (plasma-hearth): Plasma's own task manager library
// (org.kde.taskmanager, what KDE's task bar uses) instead of Mantel's KWin script bridge. Told by KWin when a window
// comes, goes or changes; nothing polls.
//   toplevels.values: [{appId, title, activated, minimized, maximized, activate(), close()}]  (setting minimized or
//   maximized asks KWin);  activeToplevel;  desktops: [{name, active}];  goDesktop(n);  moveToDesktop(w, n)
import QtQuick
import org.kde.taskmanager as TaskManager

Item {
    id: wl
    readonly property string wm: "kwin"
    readonly property bool kwin: true
    readonly property bool firehold: false
    readonly property bool bridged: true
    readonly property bool wmDesktops: true
    readonly property bool pictures: false
    readonly property bool previewsReady: false
    property bool connected: true
    property string problem: ""
    property bool captureHide: false
    property var stack: []

    TaskManager.VirtualDesktopInfo { id: vdi }
    TaskManager.ActivityInfo { id: act }
    TaskManager.TasksModel {
        id: tasks
        groupMode: TaskManager.TasksModel.GroupDisabled
        sortMode: TaskManager.TasksModel.SortAlpha
        filterByVirtualDesktop: false
        filterByScreen: false
        filterByActivity: true
        activity: act.currentActivity
        virtualDesktop: vdi.currentDesktop
        filterNotMinimized: false
        filterHidden: true
        launchInPlace: false
        separateLaunchers: true
    }
    property var wins: []
    Instantiator {
        id: inst
        model: tasks
        delegate: QtObject {
            id: w
            required property int index
            required property var model
            readonly property bool isWindow: model.IsWindow === true && model.IsLauncher !== true && model.IsStartup !== true
            readonly property string appId: String(model.AppId || "").replace(/\.desktop$/, "")
            readonly property string title: String(model.display || "")
            readonly property bool activated: model.IsActive === true
            property bool minimized: model.IsMinimized === true
            property bool maximized: model.IsMaximized === true
            readonly property bool fullscreen: model.IsFullScreen === true
            readonly property var desktops: model.VirtualDesktops || []
            readonly property var icon: model.decoration
            function idx() { return tasks.makeModelIndex(w.index); }
            function activate() { tasks.requestActivate(idx()); }
            function close() { tasks.requestClose(idx()); }
            onMinimizedChanged: if (minimized !== (model.IsMinimized === true)) { tasks.requestToggleMinimized(idx()); minimized = Qt.binding(() => w.model.IsMinimized === true); }
            onMaximizedChanged: if (maximized !== (model.IsMaximized === true)) { tasks.requestToggleMaximized(idx()); maximized = Qt.binding(() => w.model.IsMaximized === true); }
        }
        onObjectAdded: (i, o) => wl.relist()
        onObjectRemoved: (i, o) => wl.relist()
    }
    Connections { target: tasks; function onDataChanged() { relistLater.restart(); } }
    Timer { id: relistLater; interval: 30; onTriggered: wl.relist() }
    function relist() {
        const out = [];
        for (let i = 0; i < tasks.count; i++) {
            const o = inst.objectAt(i);
            if (o && o.isWindow) out.push(o);
        }
        wins = out;
    }
    readonly property var toplevels: ({ values: wins })
    readonly property var activeToplevel: wins.find(w => w.activated) || null

    readonly property var desktops: {
        const ids = Array.from(vdi.desktopIds || []), names = Array.from(vdi.desktopNames || []);
        return ids.map((id, i) => ({ id: id, name: names[i] || "", active: id === vdi.currentDesktop }));
    }
    function goDesktop(n) {
        const d = desktops[n - 1];
        if (!d) return false;
        if (typeof vdi.requestActivate === "function") vdi.requestActivate(d.id);
        else Hearth.run(["dbus-send", "--session", "--dest=org.kde.KWin", "/KWin", "org.kde.KWin.setCurrentDesktop", "int32:" + n]);
        return true;
    }
    function moveToDesktop(win, n) {
        const d = desktops[n - 1];
        if (!d || !win) return false;
        tasks.requestVirtualDesktops(win.idx(), [d.id]);
        return true;
    }
    function effect(name) {
        const sc = { overview: "Overview", windows: "ExposeAll", grid: "Grid View" }[name];
        if (sc) Hearth.run(["dbus-send", "--session", "--dest=org.kde.kglobalaccel", "/component/kwin", "org.kde.kglobalaccel.Component.invokeShortcut", "string:" + sc]);
        return true;
    }
    function preview() { return false; }
    function previewHide() { return false; }
    function send() { return false; }
    function snap() { return false; }
    function pictureUrl() { return ""; }
    function takePictures() {}
    function widOf(w) { return ""; }
}

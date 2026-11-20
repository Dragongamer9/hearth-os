// What the Couch (Couch.qml) needs from the desktop it runs on, under KDE Plasma: the helper (Plasma's executable
// source runs hearth-firehold-plasma), the clock (Plasma's time source, once a minute), the windows (Plasma's own task
// model) and what each does. Couch.qml loads this one unless a host is given to it; Firehold on labwc gives its own
// (desktop/shell/FhCouchHost.qml, 1.16): the Couch's pages are the same files on both.
import QtQuick
import org.kde.plasma.plasma5support as P5Support
import org.kde.taskmanager as TaskManager

Item {
    id: ph
    // Plasma decides what's in front itself (the Couch's window active), so these are Couch.qml's own.
    readonly property bool hosted: false
    readonly property string mediaFile: ""
    readonly property string iconFile: ""

    P5Support.DataSource {
        id: sh
        engine: "executable"
        property var cbs: ({})
        property int n: 0
        onNewData: function (source, data) {
            const cb = sh.cbs[source];
            delete sh.cbs[source];
            sh.disconnectSource(source);
            if (cb) cb(String(data["stdout"] || ""));
        }
    }
    function q(s) { return "'" + String(s).replace(/'/g, "'\\''") + "'"; }
    function run(args, cb) {
        sh.n += 1;
        const cmd = "hearth-firehold-plasma " + args.map(q).join(" ") + " #" + sh.n;
        sh.cbs[cmd] = cb || null;
        sh.connectSource(cmd);
    }

    P5Support.DataSource {
        id: clock
        engine: "time"
        connectedSources: ["Local"]
        interval: 60000
        intervalAlignment: P5Support.Types.AlignToMinute
    }
    readonly property var now: clock.data["Local"] && clock.data["Local"]["DateTime"] ? clock.data["Local"]["DateTime"] : new Date()

    TaskManager.TasksModel {
        id: tasks
        groupMode: TaskManager.TasksModel.GroupDisabled
        sortMode: TaskManager.TasksModel.SortLastActivated
        filterByVirtualDesktop: false
        filterByScreen: false
        filterByActivity: false
        filterNotMinimized: false
        onCountChanged: ph.readWins()
        onDataChanged: ph.readWins()
    }
    property var wins: []
    property string winsKey: ""
    function readWins() {
        const R = TaskManager.AbstractTasksModel;
        const out = [];
        for (let i = 0; i < tasks.count; i++) {
            const ix = tasks.makeModelIndex(i);
            if (!tasks.data(ix, R.IsWindow) || tasks.data(ix, R.IsLauncher) || tasks.data(ix, R.IsStartup)) continue;
            const ids = tasks.data(ix, R.WinIdList) || [];
            const uuid = ids.length ? String(ids[0]) : "";
            out.push({ row: i, id: uuid || String(i), uuid: uuid, appId: String(tasks.data(ix, R.AppId) || ""),
                       appName: String(tasks.data(ix, R.AppName) || ""), title: String(tasks.data(ix, Qt.DisplayRole) || ""),
                       activated: !!tasks.data(ix, R.IsActive), minimized: !!tasks.data(ix, R.IsMinimized),
                       fullscreen: !!tasks.data(ix, R.IsFullScreen), pid: Number(tasks.data(ix, R.AppPid) || 0) });
        }
        const key = JSON.stringify(out);
        if (key === winsKey) return;
        winsKey = key;
        wins = out;
    }
    Component.onCompleted: readWins()
    function idx(w) { return tasks.makeModelIndex(w.row); }
    function activate(w) { tasks.requestActivate(idx(w)); }
    function toggleFull(w) { tasks.requestToggleFullScreen(idx(w)); }
    function closeWin(w) { tasks.requestClose(idx(w)); }
}

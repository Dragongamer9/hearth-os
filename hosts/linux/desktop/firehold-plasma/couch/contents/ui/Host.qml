// What the Couch needs from its compositor and desktop, on KDE Plasma (KWin): the helper run as a command (Plasma's
// executable source), the clock (Plasma's time source, once a minute) and the windows (Plasma's task model). Everything
// else in the Couch (Couch.qml and its pages) is the same on every compositor: Firehold on labwc puts its own Host.qml
// with these same properties, signal and functions in place of this one.
import QtQuick
import org.kde.plasma.plasma5support as P5Support
import org.kde.taskmanager as TaskManager

Item {
    id: host
    // ---- the helper: exec(command line, function (stdout) {})
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
    function exec(cmd, cb) {
        sh.n += 1;
        const c = cmd + " #" + sh.n;
        sh.cbs[c] = cb || null;
        sh.connectSource(c);
    }
    // ---- the clock (a new value each minute, on the minute)
    P5Support.DataSource {
        id: clock
        engine: "time"
        connectedSources: ["Local"]
        interval: 60000
        intervalAlignment: P5Support.Types.AlignToMinute
    }
    readonly property var now: clock.data["Local"] && clock.data["Local"]["DateTime"] ? clock.data["Local"]["DateTime"] : new Date()
    // ---- the windows: windows() lists them ({row, id, uuid, appId, appName, title, activated, minimized, fullscreen,
    // pid}); windowsChanged() says the list may be different; activate / toggleFullScreen / close take a row.
    signal windowsChanged()
    TaskManager.TasksModel {
        id: tasks
        groupMode: TaskManager.TasksModel.GroupDisabled
        sortMode: TaskManager.TasksModel.SortLastActivated
        filterByVirtualDesktop: false
        filterByScreen: false
        filterByActivity: false
        filterNotMinimized: false
        onCountChanged: host.windowsChanged()
        onDataChanged: host.windowsChanged()
    }
    function windows() {
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
        return out;
    }
    function activate(row) { tasks.requestActivate(tasks.makeModelIndex(row)); }
    function toggleFullScreen(row) { tasks.requestToggleFullScreen(tasks.makeModelIndex(row)); }
    function close(row) { tasks.requestClose(tasks.makeModelIndex(row)); }
}

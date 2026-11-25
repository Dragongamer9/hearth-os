pragma Singleton

// Where Mantel's apps made of blocks (AppBlocks.qml) can be found: Describe it (DescribeIt.qml, opened for an app),
// shell.qml's `appblocks` IPC (the checks' pictures) and the apps' own pages (Settings' sections, AppBlocksFlow.qml)
// reach an app's blocks here. Nothing runs here: it only holds who's open.
import QtQuick
import Quickshell

Singleton {
    id: hub
    property var open: ({})        // app id → its AppBlocks, while its window is open
    property int rev: 0
    property var flow: null        // the Settings page shown, its groups as blocks (AppBlocksFlow.qml)
    // Describe it, asked for by an app: shell.qml opens DescribeIt.qml for it.
    signal describeAsked(string app, string name, string catalog)
    // What's kept for an app changed outside it (Describe it's Keep this, Undo): its blocks read the file again.
    signal changed(string app)

    function add(app, blocks) { const o = Object.assign({}, open); o[app] = blocks; open = o; rev++; }
    function remove(app, blocks) { if (open[app] !== blocks) return; const o = Object.assign({}, open); delete o[app]; open = o; rev++; }
    function get(app) { return open[app] || null; }
}

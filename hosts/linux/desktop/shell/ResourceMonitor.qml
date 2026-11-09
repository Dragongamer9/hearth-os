// Hearth Resource Monitor (docs/RESOURCE-MONITOR.md): Your PC (the board or device, drawn from what this machine really
// has), Overview, Processor, Memory, Disk, Network and Graphics, with per-program tables and live graphs. A normal window
// like Task Manager; its page is ResourceMonitorView.qml, loaded by file name, and this whole file is loaded by shell.qml
// through a Loader, so a problem in either only loses Resource Monitor, never the desktop (or Game Mode).
// Nothing runs until it opens: the numbers come from `hearth-resmon serve` (next to this file) once a second, only while
// the window is open. Opening it writes no setting (the memory and slot tables are cached in ~/.cache/hearth).
// Opened from the app menu (hearth-resmon.desktop), Task Manager (Open Resource Monitor), Settings › System › About,
// `hearth-resmon`, or:
//   quickshell ipc -p <shell folder> call resmon open | close | toggle
//   quickshell ipc -p <shell folder> call resmon tab <pc|overview|cpu|memory|disk|network|gpu>
//   quickshell ipc -p <shell folder> call resmon select <part id>        (Your PC: a part's details)
//   quickshell ipc -p <shell folder> call resmon fixture <file.json>     (Your PC drawn from a test machine: checks)
//   quickshell ipc -p <shell folder> call resmon menu | state
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: rm
    property bool shown: false
    property string tab: "pc"
    property string fixture: ""
    readonly property var tabs: ["pc", "overview", "cpu", "memory", "disk", "network", "gpu"]
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-resmon").toString().replace(/^file:\/\//, ""))
    property var view: null

    function open(t) {
        if (t && tabs.indexOf(t) >= 0) tab = t;
        else if (!shown) tab = "pc";
        shown = true;
    }
    function close() { shown = false; }

    IpcHandler {
        target: "resmon"
        function open(): void { rm.open(""); }
        function tab(name: string): void { rm.open(name); }
        function close(): void { rm.close(); }
        function toggle(): void { if (rm.shown) rm.close(); else rm.open(""); }
        function select(part: string): void { rm.open("pc"); if (rm.view && rm.view.item) rm.view.item.selectPart(part); }
        function fixture(path: string): void { rm.fixture = path; rm.open("pc"); if (rm.view && rm.view.item) rm.view.item.loadPc(); }
        function menu(): void { if (rm.view && rm.view.item) rm.view.item.openMenu(); }
        function state(): string { return JSON.stringify({ shown: rm.shown, tab: rm.tab, page: rm.view && rm.view.item ? rm.view.item.stateLine() : "" }); }
    }

    LazyLoader {
        active: rm.shown
        FloatingWindow {
            title: "Resource Monitor"
            implicitWidth: 1280
            implicitHeight: 800
            color: "#171d25"
            visible: true
            onVisibleChanged: if (!visible) rm.shown = false
            Component.onCompleted: rm.view = view
            Component.onDestruction: if (rm.view === view) rm.view = null
            Loader {
                id: view
                anchors.fill: parent
                focus: true
                source: "ResourceMonitorView.qml"
                onStatusChanged: if (status === Loader.Error) console.warn("Resource Monitor: its page (ResourceMonitorView.qml) couldn't load")
                onLoaded: { item.rm = rm; item.start(); }
            }
            Text {
                anchors.centerIn: parent
                visible: view.status === Loader.Error
                width: parent.width - 80
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                text: "Resource Monitor couldn't load its page. Task Manager still shows what's running; send logs from Settings › Help so it can be fixed."
                color: "#9fb2c3"; font.family: Theme.font; font.pixelSize: 14
            }
        }
    }
}

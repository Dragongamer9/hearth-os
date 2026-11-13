// The desktop shell keeps an eye on itself (1.11): every 10 minutes hearth-shell-health writes down how much memory and
// CPU it uses (~/.local/state/hearth/shell-health.log, in Send logs). When it has grown far past normal (over 1.2 GB
// and twice its first hour) or stays busy (over 60% of a core for half an hour), it refreshes itself, but only while
// nobody is looking: the screen off or locked, and no Settings, Task Manager or Files window or shell menu open. After a refresh
// a notification says so. Nothing is saved or changed: your windows in other apps aren't touched.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: sh
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-shell-health").toString().replace(/^file:\/\//, ""))
    property bool want: false
    property bool inUse: true             // set by shell.qml: Settings, Task Manager or Files is open
    onInUseChanged: maybe()
    readonly property bool nobodyLooking: DeskState.screenOff || DeskState.locked
    Timer { interval: 600000; repeat: true; running: true; onTriggered: if (!probe.running) probe.running = true }
    Timer {
        // checked again every minute while a refresh is wanted, so it happens as soon as the screen goes off
        interval: 60000; repeat: true; running: sh.want
        onTriggered: sh.maybe()
    }
    onNobodyLookingChanged: maybe()
    function maybe() {
        if (!want || !nobodyLooking) return;
        if (inUse || DeskState.overlayShown) return;
        want = false;
        Quickshell.execDetached(["sh", "-c", "sleep 8; notify-send -a Hearth -i view-refresh 'Desktop refreshed' 'The desktop shell had grown slow, so it refreshed itself while the screen was off. Nothing was closed.'"]);
        Quickshell.reload(true);
    }
    Process {
        id: probe
        command: ["python3", sh.helper, "sample"]
        stdout: StdioCollector { onStreamFinished: { try { const r = JSON.parse(text.trim().split("\n").pop()); if (r.refresh) { sh.want = true; sh.maybe(); } } catch (e) {} } }
    }
}

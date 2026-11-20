// Memory ran out (1.14, KDE's out-of-memory notification with tips, in Hearth's way). Hearth's memory guard tells you
// itself when it closes or pauses a program (hearth-memory guard); this is for the rest: when the kernel had to close
// something (the oom_kill count in /proc/vmstat went up), a notice says so with what helps, and Memory settings opens
// Settings › Memory. And when memory has been very tight for a while (memory pressure), one quiet tip, at most every six
// hours. Reads only. Off with Settings.oomNotice. Loaded by shell.qml by file name: a problem here only loses this.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: oo
    property var dialog: null                // DeskDialog
    property var settings: null
    property int kills: -1                    // the kernel's count when last read (-1: not read yet)
    property double tipAt: 0
    // 1.16 (the rest fixes): every 60 s (was 20 s), read straight from /proc (was a shell, grep and head each time);
    // the kernel's own count can't be missed, only told a little later. If /proc can't be read that way, the old way.
    Timer { interval: 60000; running: true; repeat: true; triggeredOnStart: true; onTriggered: oo.look() }
    FileView { id: vmstat; path: "/proc/vmstat"; blockLoading: true; printErrors: false }
    FileView { id: psi; path: "/proc/pressure/memory"; blockLoading: true; printErrors: false }
    property bool direct: true
    function look() {
        if (direct) {
            vmstat.reload(); psi.reload();
            const t = (vmstat.text() || "") + "\n" + (psi.text() || "");
            if (/^oom_kill \d+/m.test(t)) { if (oo.kills < 0) console.info("Mantel: the memory watch reads /proc directly"); take(t); return; }
            direct = false;
            console.info("Mantel: the memory watch can't read /proc directly; it asks grep instead");
        }
        if (!probe.running) probe.running = true;
    }
    Process {
        id: probe
        command: ["sh", "-c", "grep -m1 '^oom_kill ' /proc/vmstat; head -n1 /proc/pressure/memory 2>/dev/null; true"]
        stdout: StdioCollector { onStreamFinished: oo.take(text) }
    }
    function take(text) {
        {
            {
                const k = text.match(/^oom_kill (\d+)/m);
                const p = text.match(/^some avg10=([\d.]+) avg60=([\d.]+)/m);
                if (k) {
                    const n = parseInt(k[1]);
                    if (oo.kills >= 0 && n > oo.kills && Settings.oomNotice) oo.ranOut(n - oo.kills);
                    oo.kills = n;
                }
                if (p && Settings.oomNotice && parseFloat(p[2]) > 40 && Date.now() - oo.tipAt > 6 * 3600 * 1000) {
                    oo.tipAt = Date.now();
                    Quickshell.execDetached(["notify-send", "-a", "Hearth", "-i", "dialog-information", "Memory is nearly full",
                        "Things may slow down. Closing a browser with many tabs or a game you're not playing frees the most."]);
                }
            }
        }
    }
    readonly property string tips: "What helps:\n"
        + "• Close apps you aren't using, especially browsers with many tabs and games in the background.\n"
        + "• Turn on compressed memory (zram) in Settings › Memory: it fits much more in the same memory.\n"
        + "• A swap file on the drive gives room to spare (Settings › Memory).\n"
        + "• In Task Manager, right-click an app you never want closed and choose Never close when memory runs out."
    function ranOut(n) {
        const title = n === 1 ? "An app was closed: memory ran out" : n + " apps were closed: memory ran out";
        Quickshell.execDetached(["notify-send", "-a", "Hearth", "-i", "dialog-warning", "-u", "critical", title, "Your PC ran out of memory and had to close something to keep going."]);
        if (dialog) dialog.confirm(null, title, "Your PC ran out of memory and the system had to close " + (n === 1 ? "an app" : "some apps") + " to keep going. Anything not saved in it is lost.\n\n" + oo.tips,
            "Memory settings", () => { if (oo.settings) oo.settings.open("memory"); });
    }
    IpcHandler {
        target: "oomwatch"
        function test(): void { oo.ranOut(1); }
    }
}

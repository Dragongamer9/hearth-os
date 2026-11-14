// Main drive almost full (1.11): a pop-up when the drive Hearth OS starts from is 90% full or has under 10 GB free,
// and again, more urgently, under 3 GB. Hearth's partition holds the system and your files together, and a full one
// can stop the PC from starting properly (the login screen, the session and the next update all need room).
// "Free up space" opens Settings › Storage. Under 2 GB the 2 GB emergency reserve is freed (hearth-diskguard
// release) so there's room to clean up, and the pop-up says so. Reads the drive only; changes no setting.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: sw
    property var dialog: null                // DeskDialog
    property var openStorage: null           // () => Settings › Storage
    property int level: 0                    // 0 fine, 1 almost full, 2 nearly full: the last level shown
    property double shownAt: 0
    readonly property double again: 6 * 3600 * 1000     // the same level again after 6 hours

    function gb(b) { return (b / 1e9).toFixed(b < 10e9 ? 1 : 0) + " GB"; }

    // 0 fine, 1 almost full (90% or under 10 GB), 2 nearly full (under 3 GB)
    // "Warn when C: is over" (Task Manager › Disk, 1.12): 80–95% or off; under 10 GB and under 3 GB always warn
    property int warnPct: 90
    function levelOf(free, pct) {
        if (free < 3e9) return 2;
        if ((warnPct > 0 && pct >= warnPct) || free < 10e9) return 1;
        return 0;
    }

    Timer { interval: 600000; repeat: true; running: true; triggeredOnStart: true; onTriggered: if (!probe.running) probe.running = true }
    Timer { id: firstLook; interval: 90000; running: true; onTriggered: if (!probe.running) probe.running = true }   // once the desktop has settled
    Process {
        id: probe
        command: ["sh", "-c", "/usr/libexec/nimbus/hearth-diskguard status; echo; echo @@HEARTH@@; cat /etc/hearth/hardware.json 2>/dev/null; true"]
        stdout: StdioCollector {
            onStreamFinished: {
                const parts = text.split("@@HEARTH@@");
                try { const c = JSON.parse(parts[1] || "{}"); const w = c["disk.warnPct"]; sw.warnPct = w === "off" ? 0 : (parseInt(w) || 90); } catch (e) { sw.warnPct = 90; }
                sw.take(parts[0].trim());
            }
        }
    }
    Process { id: release; command: ["pkexec", "/usr/libexec/nimbus/hearth-diskguard", "release"] }

    function take(text) {
        let d;
        try { d = JSON.parse(text); } catch (e) { return; }
        if (!d || !d.size) return;
        const lv = levelOf(d.free, d.pct);
        if (lv === 0) { level = 0; return; }
        const now = Date.now();
        if (lv <= level && now - shownAt < again) return;
        let freed = false;
        if (d.free < 2e9 && d.reserve > 0 && !release.running) { release.running = true; freed = true; }
        level = lv; shownAt = now;
        show(lv, d, freed);
    }

    function show(lv, d, freed) {
        const title = lv === 2 ? "Your main drive is nearly full" : "Your main drive is almost full";
        let msg = Math.round(d.pct) + "% used, " + gb(d.free) + " free.\n\n"
            + "If the drive Hearth OS starts from fills up, your PC might not start properly. "
            + "Free up some space: Storage can clean up shader caches, old downloads and games you don't play.";
        if (freed) msg += "\n\nHearth freed 2 GB of emergency space it had set aside, so there's room to clean up.";
        if (lv === 2) msg += "\n\nSystem updates wait until there's at least 5 GB free.";
        Quickshell.execDetached(["notify-send", "-a", "Hearth", "-i", "drive-harddisk", "-u", lv === 2 ? "critical" : "normal", title, gb(d.free) + " free. If it fills up, your PC might not start properly."]);
        if (dialog) dialog.confirm(null, title, msg, "Free up space", () => { if (openStorage) openStorage(); });
    }
}

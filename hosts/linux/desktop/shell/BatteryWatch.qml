// Low battery (1.11): on battery, a warning at 10% and at 5%, and at 3% the PC is put away safely before the battery
// dies: hibernated where a hibernation has come back on this PC before (/var/lib/hearth/hibernate-proven, written by
// hearth-resumed), otherwise UPower shuts it down cleanly at 2% (its CriticalPowerAction, set in the image). Plugging
// in at any point cancels. Reads /sys/class/power_supply only; changes no setting.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: bw
    property int warned: 101                 // the lowest level already warned about (reset when charging)
    property bool acting: false
    function say(title, body, urgent) {
        Quickshell.execDetached(["notify-send", "-a", "Hearth", "-i", "battery-caution", "-u", urgent ? "critical" : "normal", title, body]);
    }
    Timer { interval: 30000; repeat: true; running: true; triggeredOnStart: true; onTriggered: if (!probe.running) probe.running = true }
    Process {
        id: probe
        // capacity and status of every battery; the lowest discharging one counts
        command: ["sh", "-c", "for b in /sys/class/power_supply/*; do [ \"$(cat $b/type 2>/dev/null)\" = Battery ] || continue; [ \"$(cat $b/scope 2>/dev/null)\" = Device ] && continue; echo \"$(cat $b/capacity 2>/dev/null) $(cat $b/status 2>/dev/null)\"; done; [ -f /var/lib/hearth/hibernate-proven ] && echo proven; grep -qw disk /sys/power/state 2>/dev/null && echo canhib; true"]
        stdout: StdioCollector { onStreamFinished: bw.take(text) }
    }
    function take(text) {
        let low = 101, discharging = false;
        const proven = /\bproven\b/.test(text), canHib = /\bcanhib\b/.test(text);
        for (const l of text.split("\n")) {
            const m = l.match(/^(\d+)\s+(\w+)/);
            if (!m) continue;
            const pct = parseInt(m[1]);
            if (m[2] === "Discharging") { discharging = true; low = Math.min(low, pct); }
        }
        if (!discharging) { warned = 101; acting = false; return; }
        if (low <= 3 && !acting) {
            acting = true;
            if (proven && canHib) {
                say("Battery at " + low + "%: hibernating", "Hearth is saving everything to the drive and turning off. Plug in and turn the PC on to carry on where you were.", true);
                Quickshell.execDetached(["sh", "-c", "sleep 15; [ \"$(cat /sys/class/power_supply/BAT*/status 2>/dev/null | head -n1)\" = Discharging ] && systemctl hibernate"]);
            } else {
                say("Battery at " + low + "%: save your work", "The PC shuts down cleanly at 2% so nothing is lost when the battery runs out. Plug in now to keep going.", true);
            }
        } else if (low <= 5 && warned > 5) {
            warned = 5;
            say("Battery very low (" + low + "%)", "Plug in soon. At 3% Hearth puts the PC away safely" + (proven && canHib ? " (hibernate)." : " (a clean shutdown at 2%)."), true);
        } else if (low <= 10 && warned > 10) {
            warned = 10;
            say("Battery low (" + low + "%)", "About time to plug in.", false);
        }
    }
}

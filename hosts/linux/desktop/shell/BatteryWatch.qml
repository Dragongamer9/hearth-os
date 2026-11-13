// Low battery (1.11; your choices 1.12, mockup tm-7): on battery, a warning at the levels you picked (10% and 5% by
// default); at the last one the PC is put away safely before the battery dies: what you chose for "At 5%"
// (Hibernate only where a hibernation has come back on this PC before, /var/lib/hearth/hibernate-proven; otherwise a
// clean shutdown), and UPower shuts it down cleanly at 2% (its CriticalPowerAction) whatever happens. Battery saver
// from N% switches the power mode to Battery saver until you plug in, then puts back the mode you had. Dim the screen
// on battery lowers the brightness when you unplug and puts yours back when you plug in. Your choices live in
// /etc/hearth/hardware.json (Task Manager › Battery › Health & settings); this only reads them.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: bw
    property int warned: 101                 // the lowest level already warned about (reset when charging)
    property bool acting: false
    property bool wasOnBattery: false
    property string modeBeforeSaver: ""      // the power mode Battery saver replaced (put back when you plug in)
    property int brightnessBefore: -1        // your brightness before Dim the screen on battery
    function say(title, body, urgent) {
        Quickshell.execDetached(["notify-send", "-a", "Hearth", "-i", "battery-caution", "-u", urgent ? "critical" : "normal", title, body]);
    }
    Timer { interval: 30000; repeat: true; running: true; triggeredOnStart: true; onTriggered: if (!probe.running) probe.running = true }
    Process {
        id: probe
        // capacity and status of every battery (the lowest discharging one counts), your choices, the power mode, brightness
        command: ["sh", "-c", "for b in /sys/class/power_supply/*; do [ \"$(cat $b/type 2>/dev/null)\" = Battery ] || continue; [ \"$(cat $b/scope 2>/dev/null)\" = Device ] && continue; echo \"$(cat $b/capacity 2>/dev/null) $(cat $b/status 2>/dev/null)\"; done; [ -f /var/lib/hearth/hibernate-proven ] && echo proven; grep -qw disk /sys/power/state 2>/dev/null && echo canhib; echo \"mode $(powerprofilesctl get 2>/dev/null)\"; echo \"bright $(brightnessctl -m -c backlight 2>/dev/null | head -n1 | cut -d, -f4 | tr -d %)\"; echo @@HEARTH@@; cat /etc/hearth/hardware.json 2>/dev/null; true"]
        stdout: StdioCollector { onStreamFinished: bw.take(text) }
    }
    function choices(text) {
        try { const j = JSON.parse(text.split("@@HEARTH@@")[1] || "{}"); return j && typeof j === "object" ? j : {}; } catch (e) { return {}; }
    }
    function take(text) {
        let low = 101, discharging = false;
        const head = text.split("@@HEARTH@@")[0];
        const proven = /\bproven\b/.test(head), canHib = /\bcanhib\b/.test(head);
        const mode = (head.match(/^mode (\S+)/m) || ["", ""])[1];
        const bright = parseInt((head.match(/^bright (\d+)/m) || ["", "-1"])[1]);
        for (const l of head.split("\n")) {
            const m = l.match(/^(\d+)\s+(\w+)/);
            if (!m) continue;
            const pct = parseInt(m[1]);
            if (m[2] === "Discharging") { discharging = true; low = Math.min(low, pct); }
        }
        const c = choices(text);
        const levels = (c["battery.warn"] || "10,5") === "off" ? [] : String(c["battery.warn"] || "10,5").split(",").map(x => parseInt(x)).filter(x => x > 0).sort((a, b) => b - a);
        const last = levels.length ? levels[levels.length - 1] : 5;
        const at5 = c["battery.at5"] || "hibernate";
        const saverAt = parseInt(c["battery.saver"] || "0") || 0;
        const dim = c["battery.dim"] === "on";

        if (!discharging) {
            // plugged in: put back what Battery saver and dimming changed
            if (modeBeforeSaver !== "") { Quickshell.execDetached(["powerprofilesctl", "set", modeBeforeSaver]); modeBeforeSaver = ""; }
            if (brightnessBefore >= 0) { Quickshell.execDetached(["brightnessctl", "-q", "-c", "backlight", "set", brightnessBefore + "%"]); brightnessBefore = -1; }
            warned = 101; acting = false; wasOnBattery = false;
            return;
        }
        if (!wasOnBattery) {
            wasOnBattery = true;
            Quickshell.execDetached(["python3", Quickshell.shellDir + "/hearth-tasks", "task-trigger", "battery"]);   // + New task: "When it goes on battery"
            if (dim && bright > 0 && brightnessBefore < 0) {
                brightnessBefore = bright;
                Quickshell.execDetached(["brightnessctl", "-q", "-c", "backlight", "set", Math.max(10, Math.round(bright * 0.7)) + "%"]);
            }
        }
        if (saverAt > 0 && low <= saverAt && modeBeforeSaver === "" && mode !== "" && mode !== "power-saver") {
            modeBeforeSaver = mode;
            Quickshell.execDetached(["powerprofilesctl", "set", "power-saver"]);
            say("Battery saver on (" + low + "%)", "Hearth switched to Battery saver to make the battery last. Your power mode comes back when you plug in.", false);
        }
        if (low <= last && !acting) {
            acting = true;
            if (at5 === "hibernate" && proven && canHib) {
                say("Battery at " + low + "%: hibernating", "Hearth is saving everything to the drive and turning off. Plug in and turn the PC on to carry on where you were.", true);
                Quickshell.execDetached(["sh", "-c", "sleep 15; [ \"$(cat /sys/class/power_supply/BAT*/status 2>/dev/null | head -n1)\" = Discharging ] && systemctl hibernate"]);
            } else if (at5 === "sleep") {
                say("Battery at " + low + "%: going to sleep", "Plug in soon: asleep, the battery still runs down slowly, and the PC shuts down cleanly at 2%.", true);
                Quickshell.execDetached(["sh", "-c", "sleep 15; [ \"$(cat /sys/class/power_supply/BAT*/status 2>/dev/null | head -n1)\" = Discharging ] && systemctl suspend"]);
            } else if (at5 === "shutdown") {
                say("Battery at " + low + "%: shutting down", "Hearth is shutting down cleanly so nothing is lost. Plug in now to keep going.", true);
                Quickshell.execDetached(["sh", "-c", "sleep 30; [ \"$(cat /sys/class/power_supply/BAT*/status 2>/dev/null | head -n1)\" = Discharging ] && systemctl poweroff"]);
            } else {
                say("Battery at " + low + "%: save your work", "The PC shuts down cleanly at 2% so nothing is lost when the battery runs out. Plug in now to keep going.", true);
            }
            return;
        }
        for (const lv of levels) {
            if (lv !== last && low <= lv && warned > lv) {
                warned = lv;
                const isLast = lv === last;
                say((isLast ? "Battery very low (" : "Battery low (") + low + "%)", isLast ? "Plug in soon. At 3% Hearth puts the PC away safely" + (at5 === "hibernate" && proven && canHib ? " (hibernate)." : at5 === "sleep" ? " (sleep)." : " (a clean shutdown).") : "About time to plug in.", isLast);
                break;
            }
        }
    }
}

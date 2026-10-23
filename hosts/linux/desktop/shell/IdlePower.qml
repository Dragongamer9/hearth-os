// When Desktop Mode isn't used for a while (Hearth Settings › Power): the screen goes off (wlopm), then the device
// goes to sleep. Apps that keep the screen on (a video playing, a game: the idle-inhibit protocol) hold both off.
// So do logind's "block" inhibitors (systemd-inhibit, and Hearth Player while it plays: "idle" keeps the screen on,
// "sleep" keeps the device awake); they're checked when the timer runs out, and again every minute while idle.
// Any key, click, touch or controller input turns the screen back on. Loaded by shell.qml (so a Quickshell
// without IdleMonitor only loses this).
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: root
    property bool hasWlopm: false
    property bool screenIsOff: false
    Process {
        running: true
        command: ["sh", "-c", "command -v wlopm"]
        onExited: code => root.hasWlopm = code === 0
    }

    function screenOff() { root.screenIsOff = true; Quickshell.execDetached(["wlopm", "--off", "*"]); }
    function sleepNow() {
        // The screen comes back on first, so it isn't dark after waking.
        if (root.screenIsOff) { root.screenIsOff = false; Quickshell.execDetached(["wlopm", "--on", "*"]); }
        Quickshell.execDetached(["systemctl", "suspend"]);
    }

    // ---- Hearth Player block (logind inhibitors): what's blocked right now, then do what's still allowed.
    property bool wantScreenOff: false
    property bool wantSleep: false
    Process {
        id: inhibitCheck
        command: ["busctl", "get-property", "org.freedesktop.login1", "/org/freedesktop/login1", "org.freedesktop.login1.Manager", "BlockInhibited"]
        stdout: StdioCollector {
            onStreamFinished: {
                const what = (text.match(/"([^"]*)"/) || ["", ""])[1].split(":");
                if (root.wantScreenOff && screenIdle.isIdle && what.indexOf("idle") < 0) { root.wantScreenOff = false; root.screenOff(); }
                if (root.wantSleep && sleepIdle.isIdle && what.indexOf("idle") < 0 && what.indexOf("sleep") < 0) { root.wantSleep = false; root.sleepNow(); }
            }
        }
    }
    function check() { if (!inhibitCheck.running) inhibitCheck.running = true; }
    Timer {
        interval: 60000
        repeat: true
        running: root.wantScreenOff || root.wantSleep
        onTriggered: root.check()
    }
    // ---- end of Hearth Player block

    IdleMonitor {
        id: screenIdle
        enabled: root.hasWlopm && Settings.screenOffMinutes > 0
        timeout: Math.max(1, Settings.screenOffMinutes) * 60
        respectInhibitors: true
        onIsIdleChanged: {
            if (isIdle) { root.wantScreenOff = true; root.check(); }
            else { root.wantScreenOff = false; if (root.screenIsOff) { root.screenIsOff = false; Quickshell.execDetached(["wlopm", "--on", "*"]); } }
        }
    }
    IdleMonitor {
        id: sleepIdle
        enabled: Settings.sleepMinutes > 0
        timeout: Math.max(1, Settings.sleepMinutes) * 60
        respectInhibitors: true
        onIsIdleChanged: {
            if (isIdle) { root.wantSleep = true; root.check(); }
            else root.wantSleep = false;
        }
    }
    // A setting changed while the screen was off: make sure it's on.
    Connections {
        target: Settings
        function onScreenOffMinutesChanged() { if (root.screenIsOff) { root.screenIsOff = false; Quickshell.execDetached(["wlopm", "--on", "*"]); } }
    }
}

// When Desktop Mode isn't used for a while (Hearth Settings › Power): the screen goes off (wlopm), then the device
// goes to sleep. Apps that keep the screen on (a video playing, a game: the idle-inhibit protocol) hold both off.
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

    IdleMonitor {
        id: screenIdle
        enabled: root.hasWlopm && Settings.screenOffMinutes > 0
        timeout: Math.max(1, Settings.screenOffMinutes) * 60
        respectInhibitors: true
        onIsIdleChanged: {
            if (isIdle) { root.screenIsOff = true; Quickshell.execDetached(["wlopm", "--off", "*"]); }
            else if (root.screenIsOff) { root.screenIsOff = false; Quickshell.execDetached(["wlopm", "--on", "*"]); }
        }
    }
    IdleMonitor {
        id: sleepIdle
        enabled: Settings.sleepMinutes > 0
        timeout: Math.max(1, Settings.sleepMinutes) * 60
        respectInhibitors: true
        onIsIdleChanged: {
            if (!isIdle) return;
            // The screen comes back on first, so it isn't dark after waking.
            if (root.screenIsOff) { root.screenIsOff = false; Quickshell.execDetached(["wlopm", "--on", "*"]); }
            Quickshell.execDetached(["systemctl", "suspend"]);
        }
    }
    // A setting changed while the screen was off: make sure it's on.
    Connections {
        target: Settings
        function onScreenOffMinutesChanged() { if (root.screenIsOff) { root.screenIsOff = false; Quickshell.execDetached(["wlopm", "--on", "*"]); } }
    }
}

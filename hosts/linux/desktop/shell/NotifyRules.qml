pragma Singleton

// Each app's notification settings (Settings › Notifications, 1.14; ~/.config/nimbus/notify-apps.json, written only by
// hearth-notify when you change one): allowed, a sound, on the lock screen, priority. Read here for Notifications.qml
// (what pops up and plays) and Lock.qml (what shows while locked). An app not in the file has the defaults; a file that
// can't be read means the defaults for everyone (the file itself is left for hearth-notify to put back from its .bak).
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root
    property var apps: ({})
    readonly property var defaults: ({ allow: true, sound: false, lockScreen: false, priority: "normal" })
    function rule(app) { return Object.assign({}, defaults, (app && apps[app]) || {}); }
    // A short sound for a notification from an app with Sound on (the freedesktop theme's, if it's installed).
    readonly property string soundFile: "/usr/share/sounds/freedesktop/stereo/message-new-instant.oga"
    function playSound() {
        Quickshell.execDetached(["sh", "-c", 'f="$1"; if [ -f "$f" ] && command -v pw-play >/dev/null; then exec pw-play "$f"; fi; command -v canberra-gtk-play >/dev/null && exec canberra-gtk-play -i message-new-instant', "sh", soundFile]);
    }
    FileView {
        id: file
        path: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/notify-apps.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            try { const o = JSON.parse(text()); root.apps = o && typeof o.apps === "object" && o.apps ? o.apps : {}; } catch (e) { root.apps = {}; }
        }
        onLoadFailed: root.apps = {}
    }
}

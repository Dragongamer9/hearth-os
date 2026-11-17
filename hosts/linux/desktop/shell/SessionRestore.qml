// Reopen apps from last time (Settings › System › Startup, Settings.restoreApps; off unless you turn it on, 1.14).
// While it's on, the apps open on the taskbar are noted (their .desktop ids, ~/.local/state/hearth/open-apps.json: a
// state file, not a setting) once the list has stayed the same for 20 s, so the windows closing as you sign out don't
// empty it. At sign-in, once per session (a mark in $XDG_RUNTIME_DIR, so a shell restart opens nothing twice), each app
// is opened again, a second apart, up to 12. Never Steam (Hearth starts it itself), Hearth's own windows or a game.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: root
    readonly property bool on: Settings.loaded && Settings.restoreApps
    readonly property string statePath: (Quickshell.env("XDG_STATE_HOME") || (Quickshell.env("HOME") + "/.local/state")) + "/hearth/open-apps.json"
    readonly property string mark: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hearth-apps-restored"
    readonly property var skip: /^(steam|steam_app_.*|steamwebhelper|nimbus|hearth|org\.hearthos\..*|gamescope.*|quickshell)$/i

    // The .desktop ids of the apps open now (Hearth's own windows and games left out).
    readonly property var openIds: {
        const out = [];
        void DesktopEntries.applications.values.length;
        for (const t of ToplevelManager.toplevels.values) {
            const a = t.appId || "";
            if (!a || /quickshell/i.test(a) || skip.test(a)) continue;
            const e = AppIcons.entryFor(a);
            if (!e || skip.test(e.id) || /Game/.test((e.categories || []).join(" "))) continue;
            if (out.indexOf(e.id) < 0) out.push(e.id);
        }
        return out;
    }
    onOpenIdsChanged: if (on && restored) noteTimer.restart()
    Timer { id: noteTimer; interval: 20000; onTriggered: root.note() }
    function note() {
        Quickshell.execDetached(["sh", "-c", 'mkdir -p "$(dirname "$1")" && printf "%s\\n" "$2" > "$1.tmp" && mv -f "$1.tmp" "$1"', "sh", statePath, JSON.stringify({ apps: openIds.slice(0, 12), at: Date.now() })]);
    }

    // At sign-in: once per session, only when it was already on as the desktop started (turning it on later only
    // starts noting the apps; it opens nothing then).
    property bool restored: false
    property bool started: false
    readonly property bool ready: Settings.loaded
    onReadyChanged: if (ready && !started) { started = true; if (Settings.restoreApps) startTimer.restart(); else restored = true; }
    Component.onCompleted: if (ready && !started) { started = true; if (Settings.restoreApps) startTimer.restart(); else restored = true; }
    onOnChanged: if (on && restored) noteTimer.restart()
    Timer { id: startTimer; interval: 6000; onTriggered: markCheck.running = true }
    Process {
        id: markCheck
        command: ["sh", "-c", 'if [ -e "$1" ]; then echo seen; else : > "$1"; echo first; fi', "sh", root.mark]
        stdout: StdioCollector {
            onStreamFinished: {
                if (text.trim() === "first") saved.path = root.statePath;    // loads it
                else root.restored = true;
            }
        }
    }
    property var queue: []
    FileView {
        id: saved
        path: ""
        printErrors: false
        onLoaded: {
            let ids = [];
            try { const o = JSON.parse(text()); ids = Array.isArray(o.apps) ? o.apps : []; } catch (e) {}
            root.queue = ids.filter(id => typeof id === "string" && !root.skip.test(id)).slice(0, 12);
            root.restored = true;
            if (root.queue.length) launchTimer.start();
        }
        onLoadFailed: root.restored = true
    }
    Timer {
        id: launchTimer
        interval: 1000; repeat: true
        onTriggered: {
            if (!root.queue.length) { stop(); return; }
            const id = root.queue[0];
            root.queue = root.queue.slice(1);
            if (root.openIds.indexOf(id) >= 0) return;        // already open (it started by itself)
            const e = DesktopEntries.byId(id);
            if (e) e.execute();
        }
    }
}

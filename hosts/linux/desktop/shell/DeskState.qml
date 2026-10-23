pragma Singleton

// What the taskbar and the desktop remember (~/.config/nimbus/desk.json, written by hearth-desk): apps pinned
// to the taskbar (in order), desktop icons on or off and how they're sorted, Do Not Disturb, apps whose
// notifications don't pop up. Also the notification history for this session
// (Notifications.qml adds to it, the clock's panel (ClockPanel.qml) lists it).
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root
    property var pins: []                  // .desktop ids, in taskbar order
    property bool desktopIcons: true
    property bool dnd: false               // Do Not Disturb: notifications go straight to the history
    property bool autoArrange: false       // desktop icons kept lined up in sorted order
    property string sortBy: "name"         // name | type | size | date
    property var muted: []                 // app names whose notifications only go to the history
    property var recent: []                // .desktop ids opened from the app menu, newest first
    property var history: []               // newest first: {key, app, summary, body, icon, time}
    property int unseen: 0                 // added since the panel was last opened
    property int seq: 0
    property bool overlayShown: false      // a menu, dialog or the clock's panel is open (Pad.qml: the controller types keys)
    property var mainDesktop: null         // the main screen's Desktop (for `quickshell ipc ... call desktop menu`)
    property var mainBar: null             // the main screen's taskbar (Super+1-9)
    property var shownDesktop: null        // Show desktop: {windows, front} that were minimized for it

    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-desk").toString().replace(/^file:\/\//, ""))
    readonly property string filesHelper: decodeURIComponent(Qt.resolvedUrl("hearth-files").toString().replace(/^file:\/\//, ""))

    // ------------------------------------------------ talking to hearth-desk / hearth-files
    Component {
        id: callComp
        Process {
            id: proc
            property var cb: null
            stdout: StdioCollector {
                onStreamFinished: {
                    const lines = text.trim().split("\n");
                    const last = lines.length ? lines[lines.length - 1] : "";
                    let res = null, err = "";
                    if (last.startsWith("error: ")) err = last.slice(7);
                    else {
                        try { res = JSON.parse(last); } catch (e) { err = "Something went wrong"; }
                    }
                    if (proc.cb) proc.cb(res, err);
                    proc.destroy();
                }
            }
        }
    }
    function call(args, cb) {
        const p = callComp.createObject(root, { command: ["python3", helper].concat(args), cb: cb || null });
        p.running = true;
    }
    function files(args, cb) {
        const p = callComp.createObject(root, { command: ["python3", filesHelper].concat(args), cb: cb || null });
        p.running = true;
    }
    function apply(r) {
        if (!r || !Array.isArray(r.pins)) return;
        pins = r.pins;
        desktopIcons = r.desktopIcons !== false;
        dnd = r.dnd === true;
        autoArrange = r.autoArrange === true;
        sortBy = r.sortBy || "name";
        muted = Array.isArray(r.muted) ? r.muted : [];
        recent = Array.isArray(r.recent) ? r.recent : [];
    }

    // ------------------------------------------------ taskbar pins
    function isPinned(id) { return !!id && pins.indexOf(id) >= 0; }
    function pin(id) {
        if (!id || isPinned(id)) return;
        pins = pins.concat([id]);
        call(["pin", id], (r, err) => { if (err) SettingsNav.toast(err); else apply(r); });
    }
    function unpin(id) {
        pins = pins.filter(p => p !== id);
        call(["unpin", id], (r, err) => { if (!err) apply(r); });
    }
    function togglePin(id) { isPinned(id) ? unpin(id) : pin(id); }
    function reorderPins(order) {
        pins = order;
        call(["pins-order"].concat(order), (r, err) => { if (err) SettingsNav.toast(err); else apply(r); });
    }

    // ------------------------------------------------ switches
    function setDnd(on) {
        dnd = on;
        call(["set", "dnd", on ? "on" : "off"], (r, err) => { if (!err) apply(r); });
    }
    function noteLaunch(id) {
        if (!id) return;
        recent = [id].concat(recent.filter(r => r !== id)).slice(0, 8);
        call(["recent", id], (r, err) => { if (!err) apply(r); });
    }
    function setAutoArrange(on) {
        autoArrange = on;
        call(["set", "autoArrange", on ? "on" : "off"], (r, err) => { if (!err) apply(r); });
    }
    // Notifications from an app: muted ones go to the history without popping up.
    function isMuted(app) { return !!app && muted.indexOf(app) >= 0; }
    function setMuted(app, on) {
        if (!app) return;
        muted = on ? muted.filter(m => m !== app).concat([app]) : muted.filter(m => m !== app);
        call(["mute-app", app, on ? "on" : "off"], (r, err) => { if (err) SettingsNav.toast(err); else apply(r); });
    }
    function setDesktopIcons(on) {
        desktopIcons = on;
        call(["set", "desktopIcons", on ? "on" : "off"], (r, err) => { if (!err) apply(r); });
    }

    // ------------------------------------------------ notification history
    function remember(n) {
        const img = String(n.image || "");
        const item = {
            key: ++seq,
            app: n.appName || "",
            summary: n.summary || "",
            body: String(n.body || "").replace(/<[^>]*>/g, ""),
            icon: n.appIcon || "",
            image: /^(file:\/\/|\/)/.test(img) ? img : "",
            time: new Date()
        };
        if (!item.summary && !item.body) return;
        history = [item].concat(history).slice(0, 100);
        unseen++;
    }
    function forget(key) { history = history.filter(h => h.key !== key); }
    function clearHistory() { history = []; unseen = 0; }

    FileView {
        id: file
        path: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/desk.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            try { root.apply(JSON.parse(text())); } catch (e) {}
        }
    }
}

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
    // Hearth's own notifications (a lock warning, a settings file put back from its .bak, a pairing request) can't be
    // turned off: the rule for them always allows them, whatever the file says (hearth-notify refuses it too).
    readonly property var hearthApp: /^(hearth|nimbus|mantel|task manager)\b/i
    function isHearth(app) { return !!app && hearthApp.test(app); }
    // Hearth's own warnings (1.14): never blocked by anything (an app rule, muting, Do Not Disturb, automatic quiet,
    // low priority). Urgent ones from any Hearth app, and anything from Hearth itself (not its apps like Hearth
    // Player): the system's own names, or the category "hearth.warning" a Hearth helper gives its warnings (not low ones).
    readonly property var hearthSystem: /^(hearth|hearth os|hearth settings|hearth system|hearth updates|hearth security|hearth restore|nimbus|mantel|task manager)$/i
    function isHearthWarning(n) {
        if (!n || !isHearth(n.appName)) return false;
        let cat = "";
        try { cat = String((n.hints && n.hints["category"]) || ""); } catch (e) {}
        return n.urgency === 2 || (n.urgency !== 0 && (hearthSystem.test(n.appName) || /^hearth\.warning/.test(cat)));
    }
    function rule(app) {
        const r = Object.assign({}, defaults, (app && apps[app]) || {});
        if (isHearth(app)) r.allow = true;
        return r;
    }
    // (The sound for an app with Sound on is Sounds.qml's "Notification" now: Settings › Sounds, 1.14.)
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

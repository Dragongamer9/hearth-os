pragma Singleton

// What the taskbar and the desktop remember (~/.config/nimbus/desk.json, written by hearth-desk): apps pinned
// to the taskbar (in order), desktop icons on or off and how they're sorted, Do Not Disturb, apps whose
// notifications don't pop up. Also the notification history, kept across restarts (1.14)
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
    property var history: []               // newest first: {key, app, summary, body, icon, entry, time, urgency, progress, actions}
    property int unseen: 0                 // added since the panel was last opened
    property double seq: Date.now()       // history keys: increasing across restarts too
    property bool overlayShown: false      // a menu, dialog or the clock's panel is open (Pad.qml: the controller types keys)
    property var mainDesktop: null         // the main screen's Desktop (for `quickshell ipc ... call desktop menu`)
    property var mainBar: null             // the main screen's taskbar (Super+1-9): the dock (Dock.qml), or the classic Bar.qml
    // ---- Mantel 2026: where the main bar and the dock are, so panels open next to them (Islands.qml, Dock.qml).
    // The main bar can be on any edge (Settings › Appearance › Bar layout); the classic bar (Bar.qml, the safe
    // fallback and upright screens) is at the bottom. Each writes only its own part, and only while it's the one in
    // use (barOwner / dockOwner), so a bar being replaced can't zero what the new one just set.
    property int barTop: 0                 // the main bar's reserved space on each edge (0 = not there, or auto-hide)
    property int barBottom: 0
    property int barLeft: 0
    property int barRight: 0
    property int dockBottom: 0             // the floating dock's (or the classic bar's) height plus gap
    property string barEdge: "top"         // where the main bar is: top | bottom | left | right
    property var barOwner: null
    property var dockOwner: null
    readonly property int topInset: barTop
    readonly property int bottomInset: barBottom + dockBottom
    readonly property int leftInset: barLeft
    readonly property int rightInset: barRight
    property bool classicBar: false        // the classic bottom taskbar is in use (fallback, or an upright screen)
    property bool dockPeek: false          // an auto-hide dock shown on purpose (hover, `ipc call dock show`)
    property double padSeen: 0             // when the controller was last used for Hearth's menus (the hint strip)
    property bool locked: false            // the lock screen is up (Lock.qml): moving wallpapers pause
    property bool screenOff: false         // the screen was turned off for idling (IdlePower.qml): they pause too
    property bool idle: false              // nothing typed, clicked or moved for 30 seconds (IdlePower.qml; 3 minutes before 1.16): they pause too
    property var shownDesktop: null        // Show desktop: {windows, front} that were minimized for it
    property double previewAt: 0           // KWin's window previews last shown or hidden (the dock's and taskbar's, so moving between apps is quick)

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
                    // Always destroyed, even when the callback throws (a window that closed meanwhile): a
                    // leftover Process per call was part of the shell slowly growing over a day (1.11).
                    try { if (proc.cb) proc.cb(res, err); } finally { proc.destroy(); }
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
        deskPins = r.pins;
        if (!activityPins) pins = r.pins;
        desktopIcons = r.desktopIcons !== false;
        dnd = r.dnd === true;
        autoArrange = r.autoArrange === true;
        sortBy = r.sortBy || "name";
        muted = Array.isArray(r.muted) ? r.muted : [];
        recent = Array.isArray(r.recent) ? r.recent : [];
    }

    // A change that couldn't be saved says so and shows what's really saved again: it used to keep showing your
    // choice while desk.json kept the old one, which came back by itself at the next sign-in.
    function failedOr(r, err) {
        if (!err) { apply(r); return; }
        SettingsNav.toast("Couldn't save that: " + err);
        file.reload();
    }

    // ------------------------------------------------ taskbar pins
    function isPinned(id) { return !!id && pins.indexOf(id) >= 0; }
    function pin(id) {
        if (!id || isPinned(id)) return;
        if (activityPins) { ownPins(pins.concat([id])); return; }
        pins = pins.concat([id]);
        call(["pin", id], (r, err) => { if (err) SettingsNav.toast(err); else apply(r); });
    }
    function unpin(id) {
        if (activityPins) { ownPins(pins.filter(p => p !== id)); return; }
        pins = pins.filter(p => p !== id);
        call(["unpin", id], (r, err) => failedOr(r, err));
    }
    function togglePin(id) { isPinned(id) ? unpin(id) : pin(id); }
    function reorderPins(order) {
        if (activityPins) { ownPins(order); return; }
        pins = order;
        call(["pins-order"].concat(order), (r, err) => { if (err) SettingsNav.toast(err); else apply(r); });
    }

    // ---- Activities (Activities.qml, 1.14): an activity with its own pinned apps shows those in the taskbar instead
    // of desk.json's, and pinning there changes only that activity's list (activities.json). Main, and any activity
    // without its own list, is the taskbar exactly as before.
    property var deskPins: []              // desk.json's pins (the taskbar's own)
    property var activityPins: null        // the current activity's own list, or null
    property var setActivityPins: null     // Activities.qml: saves the current activity's own list
    onActivityPinsChanged: pins = activityPins ? activityPins : deskPins
    function ownPins(list) { pins = list; if (setActivityPins) setActivityPins(list); }
    // ---- end Activities

    // ------------------------------------------------ switches
    function setDnd(on) {
        dnd = on;
        call(["set", "dnd", on ? "on" : "off"], (r, err) => failedOr(r, err));
    }
    function noteLaunch(id) {
        if (!id) return;
        recent = [id].concat(recent.filter(r => r !== id)).slice(0, 8);
        call(["recent", id], (r, err) => { if (!err) apply(r); });
    }
    function setAutoArrange(on) {
        autoArrange = on;
        call(["set", "autoArrange", on ? "on" : "off"], (r, err) => failedOr(r, err));
    }
    // Notifications from an app: muted ones go to the history without popping up.
    function isMuted(app) { return !!app && muted.indexOf(app) >= 0 && !NotifyRules.isHearth(app); }
    function setMuted(app, on) {
        if (!app) return;
        if (on && NotifyRules.isHearth(app)) { SettingsNav.toast("Hearth's own warnings always show"); return; }
        muted = on ? muted.filter(m => m !== app).concat([app]) : muted.filter(m => m !== app);
        call(["mute-app", app, on ? "on" : "off"], (r, err) => { if (err) SettingsNav.toast(err); else apply(r); });
    }
    function setDesktopIcons(on) {
        desktopIcons = on;
        call(["set", "desktopIcons", on ? "on" : "off"], (r, err) => failedOr(r, err));
    }

    // ------------------------------------------------ notification history
    // 1.14: kept across restarts (~/.local/state/nimbus/notifications.json, one key per notification, written through
    // SettingsWriter: atomic, .bak; Settings › Notifications › Keep the history turns it off). A notification stays
    // alive while it's in the history, so its buttons still work from the clock's panel (KDE keeps them the same way);
    // after a restart its app can't hear them any more, so only "Open <app>" is left on it.
    property var live: ({})                // key → the live Notification (Quickshell), while its app still has it
    property int liveRev: 0
    property var popups: []                // keys shown as pop-ups now (Notifications.qml)
    signal popped(var item)                // a pop-up came up (the bar's pill shows it for a moment)
    readonly property string historyPath: (Quickshell.env("XDG_STATE_HOME") || (Quickshell.env("HOME") + "/.local/state")) + "/nimbus/notifications.json"
    property bool keepHistory: true        // Settings › Notifications › Keep the list after a restart (Notifications.qml binds it)
    function liveOf(key) { void liveRev; return live[key] || null; }
    function hintValue(n) {
        try { const v = n.hints ? n.hints["value"] : undefined; return v === undefined || v === null || v === "" ? -1 : Math.max(0, Math.min(100, Number(v))); } catch (e) { return -1; }
    }
    function itemFrom(n, key, onLock) {
        // (its picture: Quickshell's image, else the image-path hint itself, 1.16: a recorded clip's picture on its card)
        let img = String(n.image || "");
        try { if (!/^(file:\/\/|\/)/.test(img) && n.hints) img = String(n.hints["image-path"] || n.hints["image_path"] || img); } catch (e) {}
        let cat = "";
        try { cat = String((n.hints && n.hints["category"]) || ""); } catch (e) {}
        return {
            key: key,
            app: n.appName || "",
            summary: n.summary || "",
            body: String(n.body || "").replace(/<[^>]*>/g, "").slice(0, 600),
            icon: n.appIcon || "",
            entry: n.desktopEntry || "",
            image: /^(file:\/\/|\/)/.test(img) ? img : "",
            time: new Date(),
            lock: onLock === true,         // its app shows on the lock screen (Settings › Notifications, 1.14)
            urgency: n.urgency === 2 ? "critical" : n.urgency === 0 ? "low" : "normal",
            category: cat,
            progress: hintValue(n),
            actions: Array.from(n.actions || []).map(a => ({ id: a.identifier, text: a.text })).filter(a => a.id !== "" && a.text !== "")
        };
    }
    // Returns the history item (null when it has nothing to show). Transient ones only pop up (never kept).
    function remember(n, onLock) {
        const key = ++seq;
        const item = itemFrom(n, key, onLock);
        if (!item.summary && !item.body) return null;
        const l = Object.assign({}, live); l[key] = n; live = l; liveRev++;
        n.closed.connect(() => root.gone(key));
        if (n.transient) return item;
        const old = history.slice(99);
        history = [item].concat(history).slice(0, 100);
        unseen++;
        save([item], old.map(h => h.key));
        for (const h of old) root.dropLive(h.key, false);
        return item;
    }
    // One of Hearth's own (a finished copy, "3 came in while you played"): not from an app, so nothing to keep alive.
    function rememberOwn(o) {
        const key = ++seq;
        const item = Object.assign({ app: "Hearth", summary: "", body: "", icon: "", entry: "", image: "", lock: false, urgency: "normal",
                                     category: "hearth", progress: -1, actions: [] }, o, { key: key, time: new Date() });
        const old = history.slice(99);
        history = [item].concat(history).slice(0, 100);
        unseen++;
        save([item], old.map(h => h.key));
        for (const h of old) root.dropLive(h.key, false);
        return key;
    }
    function item(key) { return history.find(h => h.key === key) || null; }
    // Its app closed it (or it expired after it was read): its buttons go, the history keeps it.
    function gone(key) {
        if (!live[key]) return;
        const n = live[key];
        const h = history.find(x => x.key === key);
        if (h) {   // the last words it had (a download that finished, a call that ended)
            const fresh = itemFrom(n, key, h.lock);
            fresh.time = h.time; fresh.actions = [];
            history = history.map(x => x.key === key ? fresh : x);
            save([fresh], []);
        }
        const l = Object.assign({}, live); delete l[key]; live = l; liveRev++;
        popups = popups.filter(k => k !== key);
    }
    function dropLive(key, dismissIt) {
        const n = live[key];
        const l = Object.assign({}, live); delete l[key]; live = l; liveRev++;
        popups = popups.filter(k => k !== key);
        if (n) { try { if (dismissIt) n.dismiss(); else n.expire(); } catch (e) {} }
    }
    function popup(key) { if (popups.indexOf(key) < 0) { popups = [key].concat(popups); const h = history.find(x => x.key === key); if (h) popped(h); } }
    function unpop(key) { popups = popups.filter(k => k !== key); }
    // A button on a notification (from its pop-up or the history). After a restart only "Open <app>" works.
    function invoke(key, id) {
        const n = live[key];
        if (n) {
            const a = Array.from(n.actions || []).find(x => x.identifier === id);
            if (a) { a.invoke(); forget(key, false); return true; }
        }
        const h = history.find(x => x.key === key);
        if (id === "default" && h && h.entry) {
            const e = AppEntries.byId(h.entry);
            if (e) { e.execute(); forget(key, false); return true; }
        }
        return false;
    }
    function forget(key, dismissIt) {
        history = history.filter(h => h.key !== key);
        if (live[key]) dropLive(key, dismissIt !== false);
        save([], [key]);
    }
    function clearHistory() {
        const keys = history.map(h => h.key);
        history = []; unseen = 0;
        for (const k of Object.keys(live)) dropLive(Number(k), true);
        save([], keys);
    }
    function serial(h) { return Object.assign({}, h, { time: h.time instanceof Date ? h.time.getTime() : Number(h.time) || Date.now(), actions: undefined }); }
    function save(items, removed) {
        if (!keepHistory) return;
        const o = {};
        for (const h of items) o["n:" + h.key] = serial(h);
        for (const k of removed) o["n:" + k] = null;
        if (Object.keys(o).length) histWriter.merge(o);
    }
    // Keep the history off: what was kept goes (your choice, from Settings); on again, from now on.
    onKeepHistoryChanged: if (!keepHistory && histLoaded) {
        const o = {};
        for (const k of histKeys) o[k] = null;
        if (Object.keys(o).length) histWriter.merge(o);
        histKeys = [];
    }
    property bool histLoaded: false
    property var histKeys: []
    SettingsWriter { id: histWriter; path: root.historyPath }
    FileView {
        id: histFile
        path: root.historyPath
        onLoaded: {
            root.histLoaded = true;
            let o = null;
            try { o = JSON.parse(text()); } catch (e) { o = null; }
            if (!o || typeof o !== "object" || Array.isArray(o)) {
                if (text().trim() !== "") { console.warn("notify: the notification history couldn't be read; putting back the last good copy"); histWriter.repair(); }
                return;
            }
            root.histKeys = Object.keys(o).filter(k => k.startsWith("n:"));
            if (!root.keepHistory) return;
            const old = [];
            for (const k of root.histKeys) {
                const h = o[k];
                if (!h || typeof h !== "object" || (!h.summary && !h.body)) continue;
                old.push(Object.assign({}, h, { key: Number(k.slice(2)) || 0, time: new Date(Number(h.time) || 0), actions: [], restored: true }));
            }
            old.sort((a, b) => b.key - a.key);
            const mine = root.history;
            root.history = mine.concat(old.filter(h => !mine.some(m => m.key === h.key))).slice(0, 100);
            root.seq = Math.max(root.seq, old.reduce((m, h) => Math.max(m, h.key), 0));
        }
        onLoadFailed: root.histLoaded = true
    }

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

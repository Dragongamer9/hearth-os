pragma Singleton

// Activities (1.14): separate sets of windows, wallpaper, widgets and pinned apps you switch between (Main, Work,
// Games…), like KDE Plasma's, written for Hearth: it never needs KDE's activity manager (kactivitymanagerd), under
// labwc or KWin alike. The rules are in activities.js; the switcher is ActivitySwitcher.qml (Widgets.qml makes it).
//
//   ~/.config/nimbus/activities.json   which activities there are (name, icon, wallpaper, own pinned apps), the one
//                                       you're on, and apps you sent to an activity ("app:<app id>"). One key per
//                                       thing, merged in alone (SettingsWriter); only something you did writes it.
//   $XDG_RUNTIME_DIR/hearth-activities-windows.json   which open window is on which activity (KWin's window id), so
//                                       a restart of the shell keeps them; gone at sign-out like the windows.
// Switching hides (minimizes) the windows of the activity you leave and brings back the ones Hearth hid on the one you
// go to; a window you minimized yourself stays minimized. The taskbars show only the current activity's windows
// (Activities.shows(window), see docs/WIDGETS.md for the hook). Widgets carry their activity (widgets.json).
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "activities.js" as A

Singleton {
    id: act
    readonly property string path: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/activities.json"
    readonly property string runPath: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hearth-activities-windows.json"
    property var raw: ({})
    property var list: [{ id: "default", name: "Main", icon: "home", wallpaper: "", pins: null, order: -1 }]
    property var apps: ({})
    property string current: "default"
    property bool loaded: false
    readonly property var cur: A.find(list, current) || list[0]
    readonly property string name: cur ? cur.name : "Main"
    readonly property string wallpaper: cur ? cur.wallpaper : ""     // Theme.wall uses it ("" = the desktop's own)
    readonly property bool many: list.length > 1
    property var switcher: null            // ActivitySwitcher.qml, when Widgets.qml loaded it
    signal switched(string id)

    function openSwitcher() { if (switcher) switcher.open(); }

    // ------------------------------------------------ the windows (the window bridge's WinList under KWin, else
    // labwc's foreign-toplevel list through Quickshell)
    function wins() {
        try { if (typeof WinList !== "undefined" && WinList && WinList.toplevels) return Array.from(WinList.toplevels.values || WinList.toplevels); } catch (e) {}
        return ToplevelManager.toplevels.values;
    }
    function ownWindow(t) {
        const a = t.appId || "";
        return /quickshell/i.test(a);
    }
    function setMin(t, on) {
        try {
            if (typeof t.setMinimized === "function") t.setMinimized(on);
            else if (on && typeof t.minimize === "function") t.minimize();
            else t.minimized = on;
        } catch (e) { console.warn("activities: couldn't " + (on ? "hide" : "bring back") + " " + (t.appId || t.title) + ": " + e); }
    }
    // This session's windows: [{ t, sid, key, act }] (sid: Hearth's own number for the window).
    property var seen: []
    property int nextSid: 1
    property var hidden: []                // sids Hearth hid when leaving their activity
    property var restored: ({})            // KWin ids → activity, from the runtime file (after a shell restart)
    function entryOf(t) { return seen.find(e => e.t === t) || null; }
    function track() {
        const now = wins().filter(t => t && !act.ownWindow(t));
        const keep = [];
        let changed = false;
        for (const t of now) {
            let e = entryOf(t);
            if (!e) {
                const key = A.winKey(t);
                e = { t: t, sid: "s" + (nextSid++), key: key, act: A.activityOf(key, t.appId || "", restored, apps, list, current) };
                changed = true;
            }
            keep.push(e);
        }
        if (keep.length !== seen.length) changed = true;
        seen = keep;
        if (changed) saveRun();
    }
    function actOf(t) { const e = entryOf(t); return e ? e.act : current; }
    // The hook for the taskbars, the window list and Alt+Tab: is this window on the activity you're on?
    function shows(t) {
        if (!many) return true;
        const a = actOf(t);
        return a === "*" || a === current;
    }
    function moveWindow(t, id) {
        const e = entryOf(t);
        if (!e || !A.find(list, id) && id !== "*") return;
        e.act = id;
        seen = seen.slice();
        if (id !== current && id !== "*" && !t.minimized) { setMin(t, true); hidden = hidden.concat([e.sid]); }
        saveRun();
    }
    // "Always open here": this app's new windows go to activity ID (a choice you made; null forgets it).
    function sendApp(appId, id) {
        if (!appId) return;
        const o = {}; o["app:" + appId] = id || null;
        writer.merge(o);
        const a = Object.assign({}, apps);
        if (id) a[appId] = id; else delete a[appId];
        apps = a;
    }
    Connections {
        target: ToplevelManager.toplevels
        function onValuesChanged() { act.track(); }
    }
    Timer { interval: 2000; running: true; repeat: true; onTriggered: act.track() }   // WinList, or a missed change

    // ------------------------------------------------ switching
    function switchTo(id) {
        if (!A.find(list, id) || id === current) return;
        track();
        const ws = seen.map(e => ({ key: e.sid, act: e.act, minimized: e.t.minimized === true }));
        const plan = A.planSwitch(ws, current, id, hidden);
        for (const e of seen) {
            if (plan.hide.indexOf(e.sid) >= 0) setMin(e.t, true);
            else if (plan.show.indexOf(e.sid) >= 0) setMin(e.t, false);
        }
        hidden = plan.hidden;
        current = id;
        writer.merge({ current: id });
        applyPins();
        switched(id);
        SettingsNav.toast(name);
    }
    function step(dir) { switchTo(A.step(list, current, dir)); }

    // ------------------------------------------------ making and changing activities (all from the switcher)
    function put(id, changes) {
        const k = "act:" + id;
        const base = raw[k] && typeof raw[k] === "object" ? raw[k] : (A.find(list, id) ? { name: A.find(list, id).name, icon: A.find(list, id).icon, order: A.find(list, id).order } : {});
        const e = Object.assign({}, base, changes);
        const r = Object.assign({}, raw); r[k] = e; raw = r;
        const o = {}; o[k] = e;
        writer.merge(o);
        reparse();
    }
    function create(nm, icon) {
        const id = A.newId(Date.now(), Math.random());
        put(id, { name: nm || A.newName(list), icon: icon || "desktops", wallpaper: "", order: list.reduce((m, a) => Math.max(m, a.order), 0) + 1 });
        return id;
    }
    function rename(id, nm) { if (nm && nm.trim()) put(id, { name: nm.trim() }); }
    function setIcon(id, icon) { put(id, { icon: icon }); }
    function setWallpaper(id, w) { put(id, { wallpaper: w || "" }); }
    // Own pinned apps: on = start from what the taskbar shows now; off = back to the taskbar's own.
    function setOwnPins(id, on) { put(id, { pins: on ? (DeskState.pins || []).slice() : null }); }
    function remove(id) {
        if (id === "default") return;
        if (current === id) switchTo("default");
        // Its windows go to Main (and come back on screen there); its widgets stay in the file, shown on Main.
        for (const e of seen) if (e.act === id) { e.act = "default"; if (hidden.indexOf(e.sid) >= 0 && current === "default") setMin(e.t, false); }
        const r = Object.assign({}, raw); delete r["act:" + id]; raw = r;
        const o = {}; o["act:" + id] = null;
        for (const a in apps) if (apps[a] === id) o["app:" + a] = null;
        writer.merge(o);
        for (const w of WidgetStore.widgets) if (w.activity === id) WidgetStore.setActivity(w.id, "default");
        reparse();
        saveRun();
    }
    function windowsOn(id) { return seen.filter(e => e.act === id).length; }

    function reparse() {
        const p = A.parse(raw);
        list = p.list;
        apps = p.apps;
        if (!A.find(list, current)) current = "default";
        applyPins();
    }
    function applyPins() {
        const a = A.find(list, current);
        DeskState.activityPins = a && Array.isArray(a.pins) ? a.pins : null;
    }
    Component.onCompleted: DeskState.setActivityPins = l => act.put(act.current, { pins: l })

    // ------------------------------------------------ files
    SettingsWriter { id: writer; path: act.path }
    FileView {
        id: file
        path: act.path
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            let o = null;
            try { o = JSON.parse(text()); } catch (e) { o = null; }
            if (!o || typeof o !== "object" || Array.isArray(o)) {
                if (text().trim() !== "") { console.warn("activities: activities.json couldn't be read; putting back the last good copy"); writer.repair(); }
                act.loaded = true;
                return;
            }
            const r = Object.assign({}, o);
            for (const k in act.raw) if (writer.owns(k)) r[k] = act.raw[k];
            act.raw = r;
            const first = !act.loaded;
            act.reparse();
            if (first || !writer.owns("current")) {
                const p = A.parse(r);
                act.current = p.current;
                act.applyPins();
            }
            act.loaded = true;
        }
        onLoadFailed: act.loaded = true
    }
    // The session's window map, so restarting the shell keeps windows on their activities (KWin ids only: labwc's list
    // has no lasting id, and its windows go to the activity you're on).
    function saveRun() {
        const m = {};
        for (const e of seen) if (e.key.indexOf("k:") === 0) m[e.key] = e.act;
        runWriter.setText(JSON.stringify(m));
    }
    FileView { id: runWriter; path: act.runPath; atomicWrites: true; blockLoading: true }
    FileView {
        path: act.runPath
        blockLoading: true
        onLoaded: { try { const o = JSON.parse(text()); if (o && typeof o === "object") act.restored = o; } catch (e) {} }
    }
}

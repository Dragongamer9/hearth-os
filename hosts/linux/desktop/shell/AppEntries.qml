pragma Singleton

// Every app's desktop entry, as the rest of Mantel and Firehold read them (1.16, the rest fixes).
//
// Quickshell's DesktopEntries.applications is changed one entry at a time when the app folders are looked at again,
// often only moving entries around, and every list built on it was made again at each step (the image check of
// Nov 20: 1.7 s of the processor each time, memory growing). And an entry can be let go and deleted when its file
// changes: a list that kept it crashed the shell (the image check after that, Nov 20). So:
//   - nothing outside this file keeps one of Quickshell's entries. What's handed out is a plain copy (a record):
//     the fields the lists read, and execute() / actions, which look the live entry up by its id when they're used;
//   - a new list comes at most once, 1.5 s after the folders have settled, and only when the apps themselves changed
//     (one added, gone or edited); the same apps in another order are no change.
// Nothing is written; nothing runs while nothing changes.
//   values       the records (in the order Quickshell gives them)
//   rev          goes up by one with each new list (for bindings that only look apps up by id)
//   byId(id)     the record for an app id, or null
//   wrap(entry)  a record for an entry from elsewhere (DesktopEntries.heuristicLookup)
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: ae
    property var values: []
    property int rev: 0
    property string sig: ""
    property var map: ({})

    function str(v) { return v === undefined || v === null ? "" : String(v); }
    function list(v) { try { return v ? Array.from(v).map(x => String(x)) : []; } catch (e) { return []; } }
    function wrap(e) {
        if (!e) return null;
        const id = str(e.id);
        if (!id) return null;
        const r = { id: id, name: str(e.name), genericName: str(e.genericName), comment: str(e.comment), icon: str(e.icon),
                    noDisplay: !!e.noDisplay, execString: str(e.execString), command: list(e.command),
                    startupClass: str(e.startupClass), keywords: list(e.keywords), categories: list(e.categories),
                    runInTerminal: !!e.runInTerminal, workingDirectory: str(e.workingDirectory) };
        r.execute = function () { const x = DesktopEntries.byId(id); if (x) x.execute(); };
        Object.defineProperty(r, "actions", { enumerable: false, get: function () {
            const x = DesktopEntries.byId(id);
            return x && x.actions ? Array.from(x.actions) : [];
        } });
        return r;
    }
    function key(r) { return [r.id, r.name, r.genericName, r.comment, r.icon, r.noDisplay, r.execString, r.startupClass,
                              r.keywords.join(";"), r.categories.join(";")].join("\u0001"); }
    function byId(id) {
        const k = str(id).replace(/\.desktop$/, "");
        if (!k) return null;
        if (map[k]) return map[k];
        // (an app that came in the last moment, before the folders settled: looked up live, copied)
        void rev;
        return wrap(DesktopEntries.byId(k));
    }
    function take() {
        const all = [];
        for (const e of (DesktopEntries.applications ? Array.from(DesktopEntries.applications.values) : [])) {
            const r = wrap(e);
            if (r) all.push(r);
        }
        const g = all.map(ae.key).sort().join("\u0002");
        if (g === sig) return;                  // the same apps (maybe in another order): nothing to make again
        const m = {};
        for (const r of all) if (!m[r.id]) m[r.id] = r;
        sig = g; map = m; values = all; rev = rev + 1;
    }
    Timer { id: settle; interval: 1500; onTriggered: ae.take() }
    Connections {
        target: DesktopEntries.applications
        // (the first list as soon as it's there; after that, once the folders have settled)
        function onValuesChanged() { if (!ae.values.length) Qt.callLater(ae.take); else settle.restart(); }
    }
    Component.onCompleted: take()
    // For the rest check: how many lists so far and how many apps (quickshell ipc call appentries state)
    IpcHandler {
        target: "appentries"
        function state(): string { return JSON.stringify({ rev: ae.rev, apps: ae.values.length, live: DesktopEntries.applications ? DesktopEntries.applications.values.length : 0 }); }
    }
}

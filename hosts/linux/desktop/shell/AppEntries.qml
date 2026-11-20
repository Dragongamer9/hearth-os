pragma Singleton

// Every app's desktop entry, as the rest of Mantel and Firehold read them (1.16, the rest fixes: the image check of
// Nov 20 had the shell busy 1.7 s at rest, and memory growing, each time the app folders were looked at again).
// Quickshell's DesktopEntries.applications is updated one entry at a time when a folder of .desktop files changes,
// often only moving entries around, and every list built on it was made again for each step. Here a new list comes
// at most once, 1.5 s after the folders have settled, and only when the apps themselves changed (one added, gone or
// edited); the same entries in another order are no change. Nothing is written; nothing runs while nothing changes.
//   values   the entries (in the order Quickshell gives them)
//   rev      goes up by one with each new list (for bindings that only look entries up by id)
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: ae
    property var values: []
    property int rev: 0
    property string sig: ""
    // (only what the app lists themselves read)
    function key(e) { return [e.id, e.name, e.icon, e.noDisplay, e.execString, e.startupClass].join("\u0001"); }
    function take(gone) {
        const all = DesktopEntries.applications ? Array.from(DesktopEntries.applications.values).filter(e => !!e && e.id !== undefined) : [];
        const g = all.map(ae.key).sort().join("\u0002");
        // the same apps, the same entries: nothing to make again (a moved entry is the same entry)
        if (!gone && g === sig && all.length === values.length) {
            const had = new Set(values);
            if (all.every(e => had.has(e))) return;
        }
        sig = g; values = all; rev = rev + 1;
    }
    Timer { id: settle; interval: 1500; onTriggered: ae.take() }
    Connections {
        target: DesktopEntries.applications
        // The first list as soon as it's there. After that: an entry Quickshell let go (a file that changed gets a new
        // entry, and the old one is deleted) means a new list at once, so nothing anywhere keeps an entry that's gone
        // (the image check of Nov 20: the shell crashed on one); entries only moved around wait until the folders
        // have settled.
        function onValuesChanged() {
            if (!ae.values.length) { Qt.callLater(ae.take); return; }
            const live = new Set(Array.from(DesktopEntries.applications.values));
            if (ae.values.some(e => !live.has(e))) ae.take(true);
            else settle.restart();
        }
    }
    Component.onCompleted: take()
    // For the rest check: how many lists so far and how many apps (quickshell ipc call appentries state)
    IpcHandler {
        target: "appentries"
        function state(): string { return JSON.stringify({ rev: ae.rev, apps: ae.values.length, live: DesktopEntries.applications ? DesktopEntries.applications.values.length : 0 }); }
    }
}

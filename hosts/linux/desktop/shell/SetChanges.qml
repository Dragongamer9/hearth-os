// The change history (1.14, the new Settings), newest first: read straight from the file hearth_settings_io.py keeps
// (~/.local/state/nimbus/settings-history.json), watched, so it's never a program left running when a page closes.
// Settings › Home's Changed lately, All changes and the controller look's help panel show it. Only reads; Undo is
// SetChangeRow's.
import QtQuick
import Quickshell
import Quickshell.Io
import "settings-changes.js" as Ch

Scope {
    id: c
    property bool running: true
    property int count: 50
    property var items: []
    function load() { file.reload(); }
    // The newest change for KEY, or null (the help panel's "last changed").
    function last(key) { return items.find(x => x.key === key) || null; }
    FileView {
        id: file
        path: (Quickshell.env("XDG_STATE_HOME") || (Quickshell.env("HOME") + "/.local/state")) + "/nimbus/settings-history.json"
        watchChanges: c.running
        onFileChanged: reload()
        onLoaded: {
            let v = [];
            try { const d = JSON.parse(text()); v = Array.isArray(d.changes) ? d.changes.slice().reverse() : []; } catch (e) { v = []; }
            v = v.filter(x => x && typeof x === "object" && Ch.shown(x)).slice(0, c.count);
            if (JSON.stringify(v) !== JSON.stringify(c.items)) c.items = v;
        }
    }
}

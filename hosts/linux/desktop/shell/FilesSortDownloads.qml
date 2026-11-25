// Sort my Downloads in Mantel's Hearth Files (the owner, Nov 21: "add that to the files app in the downloads section a
// auto sort downloads button"; the Rust Files' sortdl.rs, mockups docs/mockups/1.17-sort-downloads, checklist
// docs/mockups/checklists/sort-downloads.md). What's loose at the top of Downloads goes into five folders: Hearth,
// Installers, Media, Documents and _to_delete (same-size "name (1).ext" copies, for you to empty yourself). It only
// moves, never deletes or overwrites; one Undo (the note's button, or Ctrl+Z) puts everything back. What the rules
// don't know is asked of Sort my Downloads' AI while the sheet is open (hearth-layout sort-ask: names only).
// The work is hearth-files' (hearth_files_sortdl.py); this asks it and keeps what the window shows (the button, the
// sheet and the note are FilesSortSheet.qml, in the window). Nothing runs at rest: Downloads' folder counts are read
// only when Downloads is the folder shown.
import QtQuick
import Quickshell

Scope {
    id: sd
    property var app: null

    // The five folders, their titles and colours (the mockup's), and what goes in each.
    readonly property var top: ["Hearth", "Installers", "Media", "Documents", "_to_delete"]
    readonly property var titles: ["Hearth", "Installers", "Media", "Documents", "To delete"]
    readonly property var colours: ["#e9a43d", "#3d7ab8", "#7a5ab0", "#3f8f6a", "#b0563a"]
    readonly property var what: [
        "Hearth OS, AI Command Center, Remote Play and Hearth's other downloads",
        "AppImages, .rpm, .deb and Flatpaks · disc images in ISOs · archives in Archives",
        "Images, Videos and Sound",
        "PDFs, text and spreadsheets · scripts in Scripts · the rest in Unsorted",
        ""
    ]

    // ------------------------------------------------ is this Downloads, and what's loose
    property string downloads: ""              // hearth-files sort-state: user-dirs.dirs' XDG_DOWNLOAD_DIR, else ~/Downloads
    readonly property string shownPath: app && app.listing && !app.listing.error ? String(app.listing.path || "") : ""
    readonly property bool here: downloads !== "" && shownPath !== "" && shownPath.replace(/\/+$/, "") === downloads && !(app && app.picking)
    // What's at the top of Downloads that isn't one of the five folders (hidden things stay as they are).
    readonly property int loose: {
        if (!here || !app.listing) return 0;
        let n = 0;
        for (const e of (app.listing.entries || [])) if (e.name.charAt(0) !== "." && top.indexOf(e.name) < 0) n++;
        return n;
    }
    property var folders: ({})                 // name → how many things are inside (Downloads only)
    readonly property int folderCount: { let n = 0; for (const t of top) if (folders[t] !== undefined) n++; return n; }
    function colourOf(e) { const i = here && e && e.dir ? top.indexOf(e.name) : -1; return i >= 0 ? colours[i] : ""; }
    function countOf(e) { return here && e && e.dir && folders[e.name] !== undefined ? folders[e.name] : -1; }
    Component.onCompleted: if (app) app.call(["sort-state", ""], (r, err) => { if (r && r.downloads) sd.downloads = String(r.downloads).replace(/\/+$/, ""); })
    onHereChanged: readFolders()
    Connections {
        target: sd.app
        function onListingChanged() { if (sd.here) sd.readFolders(); }
    }
    function readFolders() {
        if (!here) { folders = ({}); return; }
        const p = shownPath;
        app.call(["sort-state", p], (r, err) => { if (r && r.here && p === sd.shownPath) sd.folders = r.folders || ({}); });
    }

    // ------------------------------------------------ the sheet (mockup 1)
    property bool open: false
    property var plan: null                    // {dir, moves: [{from, to, group}], unknown, counts}
    property bool every: false                 // Show every file
    property bool asking: false                // the AI is placing what the rules don't know
    property int seq: 0
    function count(g) { return plan ? plan.moves.filter(m => m.group === g).length : 0; }
    function openSheet() {
        if (!here) return;
        const s = ++seq;
        app.call(["sort-plan", shownPath], (r, err) => {
            if (s !== sd.seq) return;
            if (err || !r) return app.say(err || "Sort my Downloads couldn't look at Downloads", true);
            if (r.error) return app.say(r.error, true);
            if (!r.moves || !r.moves.length) return app.say("Nothing is loose in Downloads");
            sd.plan = r;
            sd.every = false;
            sd.open = true;
            if (r.unknown && r.unknown.length) {
                sd.asking = true;
                app.call(["sort-ai", JSON.stringify(r)], (a, err2) => {
                    if (s !== sd.seq) return;
                    sd.asking = false;
                    // no AI, or no answer: the rules' place stays
                    if (a && a.moves && sd.open && a.moves.length === sd.plan.moves.length) sd.plan = a;
                });
            }
        });
    }
    function cancel() { seq++; open = false; plan = null; asking = false; if (app.ui && app.ui.refocus) app.ui.refocus(); }
    function go() {
        if (!plan) return;
        const p = plan;
        seq++;
        open = false;
        plan = null;
        asking = false;
        if (app.ui && app.ui.refocus) app.ui.refocus();
        app.call(["sort-apply", JSON.stringify(p)], (r, err) => {
            if (err || !r) return app.say(err || "Sort my Downloads couldn't move anything", true);
            if (r.error) return app.say(r.error, true);
            const done = r.done || [];
            if (done.length) app.pushUndo({ kind: "sortdl", moves: done, items: done.map(m => m[1]) });
            sd.sorted = { n: done.length, folders: r.folders || 0, skipped: r.skipped || 0 };
            noteTimer.restart();
            app.reload();
        });
    }

    // ------------------------------------------------ the note after sorting, with Undo (mockup 2)
    property var sorted: null                  // {n, folders, skipped}
    Timer { id: noteTimer; interval: 12000; onTriggered: sd.sorted = null }
    readonly property string noteText: {
        if (!sorted) return "";
        let m = "✓ Sorted " + app.plural(sorted.n, "item", "items") + " into " + app.plural(sorted.folders, "folder", "folders") + ".";
        if (sorted.skipped > 0) m += " " + sorted.skipped + " stayed (a name was already there).";
        return m;
    }
    function undo() {
        sorted = null;
        noteTimer.stop();
        app.undo();
    }
}

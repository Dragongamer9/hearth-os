pragma Singleton

// Build anything (docs/mockups/checklists/build-anything.md): which desktop you've picked, kept in
// ~/.local/share/nimbus/screen.json. Off (Hearth OS Mantel, the bar, the dock and Quick Access as always) until you pick
// one of the ready-made desktops (screens/*.json next to this file) or build your own; Settings › Your desktop and the
// first-start choice (FirstStart.qml) change it. Red flag 9: only what you changed is merged into the file (atomic, a
// .bak kept), a file that can't be read is never written over with the defaults, and nothing here changes by itself.
// A screen that can't be drawn (Screen.qml failed) never leaves you without a desktop: the bar and dock come back.
//   on        a whole-screen desktop is in use
//   screen    the layout ({grid, background, look, blocks, bars}), as hearth-layout's screen.rs has it
//   from      which ready-made one it started from ("console", "iconrow", "cockpit", "cabin", "notebook", "retro", "")
//   said      the words that made it, kept with it
//   chosen    the first-start choice was made (Hearth OS Mantel or your own): it never shows again by itself
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: sl
    property bool loaded: false
    property bool unreadable: false
    property bool on: false
    property var screen: null
    property string from: ""
    property string said: ""
    property bool chosen: false
    // Screen.qml couldn't be drawn this time: Mantel's bar and dock come back (nothing is written).
    property bool failed: false
    // how many Screen.qml windows are up (for `quickshell ipc call screen state`)
    property int drawn: 0
    // the Guide asked for from elsewhere (the controller's Home button, `quickshell ipc call screen guide`)
    property int guideAsk: 0
    // The desktop covers Mantel's bar and dock only when it has its own way to everything they do (a guide, a menu or a
    // Game Mode block), and only while it's drawn.
    // (a property called "on" can't be listened to by name in QML: Mantel listens to this one)
    readonly property bool live: on
    readonly property bool hidesBars: on && !failed && !!screen && screen.bars === "hidden"
    // The ready-made desktops, in the order the gallery shows them.
    readonly property var readyMade: [
        { id: "console", name: "Console dashboard", from: "#1d3557", to: "#7a3b1d" },
        { id: "iconrow", name: "Icon row home", from: "#3a2550", to: "#6b4a3a" },
        { id: "cockpit", name: "Cockpit", from: "#062a33", to: "#0b1a2a" },
        { id: "cabin", name: "Cabin", from: "#4a2f17", to: "#1f140b" },
        { id: "notebook", name: "Paper notebook", from: "#efe6cf", to: "#d8ccb0" },
        { id: "retro", name: "1995 computer", from: "#008080", to: "#006a6a" }
    ]

    // hearth-layout: the one next to this Mantel (a Store update of Mantel carries its own, bin/), else Hearth OS's.
    function layoutCmd(args) {
        return ["sh", "-c", 'b="$1/../bin/hearth-layout"; [ -x "$b" ] || b=hearth-layout; shift; exec "$b" "$@"', "sh", Quickshell.shellDir].concat(args);
    }
    function readyFile(id) { return decodeURIComponent(Qt.resolvedUrl("screens/" + id + ".json").toString().replace(/^file:\/\//, "")); }

    // Use one of the ready-made desktops (read from its file, then kept in yours: changing it later changes your copy).
    function use(id) {
        if (unreadable) return "Your desktop's file couldn't be read, so nothing was written over it.";
        readyView.path = "";
        readyView.wanted = id;
        readyView.path = readyFile(id);
        return "";
    }
    // Back to Hearth OS Mantel: the bar, the dock and Quick Access. Your built desktop stays in the file, off.
    function useMantel() {
        if (unreadable) return "Your desktop's file couldn't be read, so nothing was written over it.";
        on = false; chosen = true; failed = false;
        writer.merge({ on: false, chosen: true });
        return "";
    }
    // Your own desktop (the block editor's Keep this): the layout and the words that made it.
    function keep(screenObj, words) {
        if (unreadable) return "Your desktop's file couldn't be read, so nothing was written over it.";
        screen = screenObj; on = true; chosen = true; failed = false; from = ""; said = words || "";
        writer.merge({ on: true, chosen: true, screen: screenObj, from: "", said: said });
        return "";
    }
    function markChosen() { if (!unreadable) { chosen = true; writer.merge({ chosen: true }); } }

    FileView {
        id: readyView
        property string wanted: ""
        path: ""
        onLoaded: {
            let o = null;
            try { o = JSON.parse(text()); } catch (e) { o = null; }
            if (!o || !Array.isArray(o.blocks)) { console.warn("Mantel: the ready-made desktop " + wanted + " couldn't be read"); return; }
            sl.screen = o; sl.on = true; sl.chosen = true; sl.failed = false; sl.from = wanted; sl.said = o.said || "";
            writer.merge({ on: true, chosen: true, screen: o, from: wanted, said: sl.said });
        }
    }

    SettingsWriter { id: writer; path: file.path; history: true }

    FileView {
        id: file
        path: Quickshell.env("HOME") + "/.local/share/nimbus/screen.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            let o = null;
            const raw = text();
            try { o = JSON.parse(raw); } catch (e) { o = null; }
            if (!o || typeof o !== "object") {
                if (raw.trim() !== "") { console.warn("Mantel: screen.json couldn't be read; putting back the last good copy"); sl.unreadable = true; writer.repair(); }
                sl.loaded = true;
                return;
            }
            sl.unreadable = false;
            if (!writer.owns("on")) sl.on = o.on === true && !!o.screen && Array.isArray(o.screen.blocks);
            if (!writer.owns("screen")) sl.screen = o.screen || null;
            if (!writer.owns("from")) sl.from = o.from || "";
            if (!writer.owns("said")) sl.said = o.said || "";
            if (!writer.owns("chosen")) sl.chosen = o.chosen === true;
            sl.loaded = true;
        }
        onLoadFailed: sl.loaded = true
    }
}

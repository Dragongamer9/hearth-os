// Hearth Files' "filemanager" IPC (1.14: one copy for both places Files runs). In Desktop Mode's shell, Files is made
// the first time it's used, so `get` makes it and `peek` only looks; in picker.qml (the file chooser, Files in Game
// Mode) both are that Files.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: ipc
    property var get: null      // () => the Files object (made if needed)
    property var peek: null     // () => the Files object, or null when it isn't made yet
    IpcHandler {
        target: "filemanager"
        // Forwarded to Hearth Files (the shell's Files, or the file chooser's).
        function view(v: string): void { ipc.get().setView(v); }
        function menu(): void { ipc.get().viaPad = false; ipc.get().openMenu(!ipc.get().current, -1, -1); }
        function search(q: string): void { ipc.get().runSearch(q); }
        function tab(p: string): void { ipc.get().open(""); ipc.get().newTab(p); }
        function select(name: string): void { const i = ipc.get().entries.findIndex(e => e.name === name); if (i >= 0) ipc.get().cur = i; }
        function activate(name: string): void { if (ipc.get().found !== null) ipc.get().clearSearch(); const i = ipc.get().entries.findIndex(e => e.name === name); if (i >= 0) { ipc.get().cur = i; ipc.get().activate(ipc.get().entries[i]); } }
        function close(): void { ipc.get().closeOverlay(); }
        function home(): void { ipc.get().open(""); ipc.get().navigate(ipc.get().homePage, ""); }
        // This PC and Installed apps (the Desktop Mode test's pictures); letter: the drive with that letter.
        function thispc(): void { ipc.get().open(""); ipc.get().navigate(ipc.get().thisPcPage, ""); }
        function installed(): void { ipc.get().open(""); ipc.get().navigate(ipc.get().appsPage, ""); }
        // Hearth (C:) with Windows folder names: "" for C:\, "Program Files", "Program Files (x86)", or any C:\ form.
        function hearthc(sub: string): void {
            ipc.get().open("");
            const r = sub.indexOf(":") === 1 ? ipc.get().fromLetters(sub) : { path: ipc.get().cRoot + (sub ? "/" + sub : "") };
            if (r && r.path) ipc.get().navigate(r.path, "");
        }
        // Properties of the Hearth (C:) page shown (the Desktop Mode test's pictures).
        function cprops(): void { if (ipc.get().onCView) ipc.get().showCProps(null); }
        function letter(l: string): void { const r = ipc.get().fromLetters(l + ":\\"); if (r && r.path) { ipc.get().open(""); ipc.get().navigate(r.path, ""); } }
        function details(on: string): void { ipc.get().detailsPane = on === "on"; ipc.get().rememberView(); }
        function undo(): void { ipc.get().undo(); }
        // Renaming in place (as F2 does with a keyboard); "" puts the name back without renaming.
        function rename(name: string): void { const e = ipc.get().entries.find(x => x.name === name); ipc.get().renaming = e ? e.path : ""; }
        function marks(names: string): void { const m = {}; for (const n of names.split("/")) { const e = ipc.get().entries.find(x => x.name === n); if (e) m[e.path] = true; } ipc.get().marked = m; }
        // For checks: the folder shown and the item under the cursor ("" while the window is closed).
        // A staged drag for the screenshots (CI has no mouse): the picked items' drag picture over the folder `onto`.
        function dragdemo(onto: string): void {
            const e = ipc.get().entries.find(x => x.name === onto);
            ipc.get().dragHint = e ? e.path : "";
            ipc.get().dragDemo = onto !== "";
        }
        // The navigation pane's first item of a group (start, bookmarks, places, drives, devices…), lit as with the controller.
        function section(name: string): void { const i = ipc.get().places.findIndex(q => q.section === name); if (i >= 0) { ipc.get().zone = "places"; ipc.get().placeIndex = i; } }
        function where(): string { const f = ipc.peek(); return f && f.shown ? f.path + "|" + (f.current ? f.current.name : "") : ""; }
        // The Briefcase, for the Desktop Mode test's pictures: Update All's list, run it, Customize, close a dialog.
        function bcUpdate(): void { ipc.get().open(""); ipc.get().bc.run("bc-update-all", [], null); }
        function bcApply(): void { if (ipc.get().bc.mode === "update") ipc.get().bc.runUpdate(); }
        function bcCustomize(): void { ipc.get().open(""); ipc.get().bc.run("bc-customize", [], null); }
        function bcState(): string { return JSON.stringify({ mode: ipc.get().bc.mode, st: ipc.get().bc.st ? { name: ipc.get().bc.st.name, counts: ipc.get().bc.st.counts || {}, locked: ipc.get().bc.st.locked } : null, rows: ipc.get().bc.plan ? ipc.get().bc.plan.rows.length : 0 }); }
    }
}

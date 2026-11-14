// Hearth Files: Desktop Mode's file manager. A normal window (labwc decorates and tiles it like any app) with
// your places on the left (folders, bookmarks, games, drives, Trash) and the folder on the right as icons, a
// compact list or details with sortable columns, in tabs. Works with mouse, touch, keyboard and the controller,
// like Hearth Settings:
//   arrows move · A / Enter opens (folders go in, files open in their app) · B / Esc goes up (B at Home closes)
//   · X / the Menu key: actions (copy, paste, rename, compress, Trash, Open with, Properties…) · Y: on-screen
//   keyboard, and typing filters this folder (Enter searches the folders inside too) · Up from the top row
//   reaches the toolbar (back, forward, up, the path, search, view, sort, hidden); Left from the first column
//   reaches your places.
// Mouse and touch: click selects (Ctrl adds, Shift picks a range, drag on empty space for a rubber band),
// double-click / double-tap opens, middle-click opens a folder in a new tab, right-click or press-and-hold for
// actions, drag files onto a folder, a place or another app (Ctrl: copy).
// Keyboard: the usual shortcuts (Ctrl+C/X/V, F2, Del / Shift+Del, Ctrl+A, Ctrl+I, Ctrl+H, Ctrl+L, Ctrl+F, F5,
// Alt+←/→/↑, Alt+Home, Ctrl+T/W/Tab, Ctrl+1/2/3, Ctrl+Shift+N, Ctrl+D, Alt+Enter, Space preview, Shift+F4 terminal).
// Copying, moving, compressing and unpacking run in hearth-files, with progress at the bottom; names that are
// already there ask Replace / Skip / Keep both. Properties is FileProperties.qml (shared with the desktop).
// Opened from the app launcher, the taskbar, "Open folder" in other apps (xdg-open), or:
//   quickshell ipc -p <shell folder> call files open <path>     (or: home, toggle, close)
//   quickshell ipc -p <shell folder> call filemanager view <grid|compact|details> (menu, search <text>, tab <path>)
// Picker mode (startPick, used by picker.qml): every app's Open / Save dialog, through hearth-portal. The same window
// with a bar at the bottom (the name to save as, which files to show, the app's options, Cancel and Open/Save); A /
// Enter on a file picks it, Down from the last row reaches the bar, and the answer goes out through picked().
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Widgets
import "osk/hearth-osk.js" as Osk
import "drive-paths.js" as DrivePaths

Scope {
    id: app
    property bool shown: false
    readonly property string home: Quickshell.env("HOME") || "/"
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-files").toString().replace(/^file:\/\//, ""))
    property var ui: null                      // the window, while it's open

    // Properties (also used by the desktop's icons: Desktop.qml calls properties()).
    FileProperties {
        id: propsWin
        helper: app.helper
        onApplied: (oldPath, newPath) => {
            if (app.listing) app.load(app.listing.path, app.base(newPath), "none");
            app.refreshPlaces();
        }
    }
    function properties(paths) {
        // Hearth (C:)'s own folders (hearth:c/…) have their own Properties (FilesCProps.qml), inside the window.
        if (paths && paths.length === 1 && DrivePaths.isCPath(paths[0])) {
            if (!shown) open("");
            return showCProps({ path: paths[0], name: base(paths[0]), cv: {} });
        }
        propsWin.show(paths);
    }

    // The Briefcase (Briefcase.qml, hearth-briefcase; docs/HEARTH-FILES.md › Briefcase): statuses, Update All, locks,
    // Customize, auto-sync. Its dialogs are BriefcaseDialog.qml, inside the window.
    readonly property var filesApp: app        // (inside Briefcase { }, "app" would mean its own property)
    Briefcase { id: briefcase; app: filesApp }

    // ------------------------------------------------ what's shown
    property string path: ""
    property var listing: null                 // hearth-files list: {path, entries, writable, trash, free, total, error?}
    property var places: []
    property bool loading: false
    property int loadSeq: 0
    property string view: "grid"               // grid | compact | details
    property string sort: "name"               // name | date | size | type
    property bool desc: false
    property string group: "none"              // none | type | date
    property bool hidden: false
    property bool showExt: true                // View › File name extensions
    property bool detailsPane: false           // the details pane on the right (the Details button, Alt+P)
    readonly property string homePage: "hearth:home"   // the Home page: Quick access and recent files
    readonly property bool onHome: path === homePage
    // This PC (FilesThisPc.qml) and Installed apps (FilesApps.qml): Hearth's own pages, like the Home page.
    readonly property string thisPcPage: "hearth:thispc"
    readonly property string appsPage: "hearth:apps"
    readonly property bool onThisPc: path === thisPcPage
    readonly property bool onApps: path === appsPage
    // Hearth (C:) with Windows folder names (docs/HEARTH-FILES.md "Hearth (C:) the Windows way"): a view, nothing on
    // disk moves. Its own pages are hearth:c paths (C:\, Program Files, Program Files (x86), Users\<you>\AppData), drawn
    // by FilesCDrive.qml (a Loader); every other folder in it is a real one. The switches are files.json keys saved only
    // when you change them (View here, or Settings › System › Drives & folders): cdrive (on unless turned off),
    // linuxFolders (off unless turned on), addressBar (windows | linux), copyPath (windows | linux).
    readonly property string cRoot: "hearth:c"
    // C: everywhere (1.11, step 1, the approved mockup cdrive-1): the Open / Save windows every app uses show Hearth (C:)
    // too. What they hand back is always the real path (an app can't open C:\…); a folder that's only a name in the
    // view (Program Files, C:\ itself) can't be saved into.
    readonly property bool cdriveOn: prefs.cdrive !== false
    readonly property bool cLinux: prefs.linuxFolders === true
    readonly property bool navShown: prefs.navPane !== false     // View › Navigation pane (1.11, files.json navPane)
    readonly property bool cAddrLinux: prefs.addressBar === "linux"
    readonly property bool cCopyLinux: prefs.copyPath === "linux"
    property var cdata: ({})                   // hearth-files cdrive (read only)
    readonly property string cPageNow: DrivePaths.cPage(path, pathCtx)
    readonly property bool onCView: cPageNow !== ""
    readonly property bool onSpecial: onThisPc || onApps || onCView
    // Where Files opens (View › Open Files to): Home unless you picked This PC (files.json "startPage").
    readonly property string startPage: prefs.startPage === "thispc" && !picking ? thisPcPage : homePage
    property var pc: ({ folders: [], drives: [], network: [] })   // hearth-files this-pc
    property string pcView: "grid"             // This PC: grid | details (files.json "thisPcView", only when you change it)
    property int pcCols: 3                     // This PC's tiles per row (FilesThisPc.qml says)
    property var appList: []                   // hearth-files installed-apps
    property bool appsLoading: false
    property string appsSort: "name"           // name | size | date
    property string appsFilter: "all"          // all | hearth | flatpak | winapp | steam | recomp | games
    property var navOpen: ({ thispc: true, network: true })   // the navigation pane's tree (this session only)
    property var recent: []                    // hearth-files recent (the Home page's list)
    property var undoStack: []                 // file changes that can be undone (Ctrl+Z), newest last
    property var redoStack: []
    // The view as drawn: the Home page is always a list (Quick access tiles above it).
    readonly property string shownView: onHome ? "details" : view
    readonly property var views: ["xlarge", "large", "grid", "small", "tiles", "compact", "details"]
    property var prefs: ({})
    property bool prefsLoaded: false
    property string query: ""                  // filters this folder as you type
    property var found: null                   // search results (the folders inside too), or null
    property string foundQuery: ""
    property bool searching: false
    property var marked: ({})                  // path → true (several picked)
    property int anchor: -1                    // where a Shift range starts
    readonly property int markCount: Object.keys(marked).length
    property var clip: ({ mode: "", paths: [] })   // copy | cut
    property var back: []
    property var fwd: []
    property var tabs: [{ path: "", back: [], fwd: [], sel: "" }]
    property int tabIndex: 0
    property int tabFocus: 0
    property bool editingPath: false
    property var templates: []
    property bool mouseMode: true              // the last pointer was a mouse (rubber band and drag), not touch
    property string dragUris: ""
    property string dragHint: ""               // a folder lit as where a drop goes (the CI screenshot's staged drag)
    property bool dragDemo: false               // filemanager dragdemo: shows the drag picture in the window

    // ------------------------------------------------ focus: tabs | toolbar | places | main, plus an overlay on top
    property string zone: "main"
    property int cur: 0
    property int placeIndex: 0
    property int toolIndex: 0
    property string overlay: ""                // "" | menu | input | confirm | apps | conflict | preview
    property var menuItems: []
    property int menuIndex: 0
    property string menuTitle: ""
    property real menuX: -1
    property real menuY: -1
    property bool viaPad: false                // the actions menu came from the controller (X): rename etc. open the on-screen keyboard
    property bool oskOpened: false
    property string dlgTitle: ""
    property bool dlgSecret: false             // the text box hides what's typed (a sharing password)
    property string dlgText: ""
    property string dlgOk: ""
    property bool dlgDanger: false
    property int dlgButton: 0
    property var dlgAction: null
    property var dlgInfo: null
    property var dlgApps: []
    property bool dlgAppsAll: false
    property int dlgIndex: 0
    property string dlgPath: ""
    property var cf: null                      // a name clash: {verb, paths, dest, list, i, decide, all, button}
    property var pv: null                      // quick preview: {e, text, error}
    property string dlgAlt: ""                 // a confirm's third button (Open in Hearth Text), "" for none
    property var dlgAltAction: null
    property var foPlan: null                  // opening a script, program or .desktop file: {e, plan, remember}
    property string pendingArchive: ""         // opened with this archive (hearth-filemanager): asked about once it's listed

    // ------------------------------------------------ picker mode (picker.qml, for hearth-portal)
    // pick: {kind: open|save|savefiles, title, accept, multiple, directory, filters: [{name, globs, mimes}], filter,
    //        folder, name, files, choices: [{id, label, options: [[id, label]], value}]}; null: the file manager.
    property var pick: null
    readonly property bool picking: pick !== null
    property bool pickDone: false
    property int pickFilter: -1
    property var pickChoice: ({})
    property string pickName: ""
    property int pickIndex: 0                  // the bar's control the keyboard / controller is on (zone "pick")
    // The on-screen keyboard drawn inside the window (picker.qml in Game Mode, where gamescope shows one window): Hearth's
    // shared keyboard (OskBoard.qml), its controller presses from picker.qml's hearth-pad, typing through it (oskSend).
    property bool oskOn: false
    property var oskEngine: null
    property var oskVm: null
    signal picked(var result)                  // {paths, filter, choices} or {cancel: true}
    signal oskSend(var msg)
    function oskWinOpen() {
        if (!oskEngine) {
            Osk.HearthOSK.configure({ settings: { suggestions: false } });
            oskEngine = Osk.HearthOSK.createEngine({
                host: "mantel",
                adapter: {
                    insert: (text, mods) => app.oskSend({ cmd: "emit", text: text, mods: mods ? Object.keys(mods).filter(k => mods[k]) : [] }),
                    deleteBack: n => app.oskSend({ cmd: "key", key: "bksp", n: n }),
                    key: (name, mods) => app.oskSend({ cmd: "key", key: name, n: 1, mods: mods ? Object.keys(mods).filter(k => mods[k]) : [] })
                },
                onChange: () => { app.oskVm = app.oskEngine.view(); },
                onHide: () => app.oskWinClose()
            });
        }
        oskEngine.reset("text", "Done");
        oskEngine.setHints(true);
        oskOn = true;
        oskSend({ cmd: "osk", open: true });
    }
    function oskWinClose() { oskOn = false; oskSend({ cmd: "osk", open: false }); }
    function oskPad(b, down) { if (oskOn && oskEngine) oskEngine.handle(b, down, Date.now()); }
    function oskAxes(lx, ly, rx, ry) { if (oskOn && oskEngine) oskEngine.axes(lx, ly, rx, ry, Date.now()); }
    Timer { interval: 16; repeat: true; running: app.oskOn; onTriggered: if (app.oskEngine) app.oskEngine.tick(Date.now()) }
    property alias propsShown: propsWin.shown

    // ------------------------------------------------ long copy / move / compress / extract
    property var job: null                     // {verb, count, name, destName, done, total, speed, file, cancelling}
    property var jobQueue: []
    property string jobLast: ""
    property var jobNow: null

    // ------------------------------------------------ messages in the bottom bar
    property string note: ""
    property bool noteError: false
    Timer { id: noteTimer; interval: 4500; onTriggered: app.note = "" }
    function say(t, err) { note = t; noteError = !!err; noteTimer.restart(); }

    IpcHandler {
        target: "filemanager"
        function view(v: string): void { app.setView(v); }
        function menu(): void { app.viaPad = false; app.openMenu(!app.current, -1, -1); }
        function search(q: string): void { app.runSearch(q); }
        function tab(p: string): void { app.open(""); app.newTab(p); }
        function select(name: string): void { const i = app.entries.findIndex(e => e.name === name); if (i >= 0) app.cur = i; }
        function activate(name: string): void { if (app.found !== null) app.clearSearch(); const i = app.entries.findIndex(e => e.name === name); if (i >= 0) { app.cur = i; app.activate(app.entries[i]); } }
        function close(): void { app.closeOverlay(); }
        function home(): void { app.open(""); app.navigate(app.homePage, ""); }
        // This PC and Installed apps (the Desktop Mode test's pictures); letter: the drive with that letter.
        function thispc(): void { app.open(""); app.navigate(app.thisPcPage, ""); }
        function installed(): void { app.open(""); app.navigate(app.appsPage, ""); }
        // Hearth (C:) with Windows folder names: "" for C:\, "Program Files", "Program Files (x86)", or any C:\ form.
        function hearthc(sub: string): void {
            app.open("");
            const r = sub.indexOf(":") === 1 ? app.fromLetters(sub) : { path: app.cRoot + (sub ? "/" + sub : "") };
            if (r && r.path) app.navigate(r.path, "");
        }
        // Properties of the Hearth (C:) page shown (the Desktop Mode test's pictures).
        function cprops(): void { if (app.onCView) app.showCProps(null); }
        function letter(l: string): void { const r = app.fromLetters(l + ":\\"); if (r && r.path) { app.open(""); app.navigate(r.path, ""); } }
        function details(on: string): void { app.detailsPane = on === "on"; app.rememberView(); }
        function undo(): void { app.undo(); }
        // Renaming in place (as F2 does with a keyboard); "" puts the name back without renaming.
        function rename(name: string): void { const e = app.entries.find(x => x.name === name); app.renaming = e ? e.path : ""; }
        function marks(names: string): void { const m = {}; for (const n of names.split("/")) { const e = app.entries.find(x => x.name === n); if (e) m[e.path] = true; } app.marked = m; }
        // For checks: the folder shown and the item under the cursor ("" while the window is closed).
        // A staged drag for the screenshots (CI has no mouse): the picked items' drag picture over the folder `onto`.
        function dragdemo(onto: string): void {
            const e = app.entries.find(x => x.name === onto);
            app.dragHint = e ? e.path : "";
            app.dragDemo = onto !== "";
        }
        // The navigation pane's first item of a group (start, bookmarks, places, drives, devices…), lit as with the controller.
        function section(name: string): void { const i = app.places.findIndex(q => q.section === name); if (i >= 0) { app.zone = "places"; app.placeIndex = i; } }
        function where(): string { return app.shown ? app.path + "|" + (app.current ? app.current.name : "") : ""; }
        // The Briefcase, for the Desktop Mode test's pictures: Update All's list, run it, Customize, close a dialog.
        function bcUpdate(): void { app.open(""); briefcase.run("bc-update-all", [], null); }
        function bcApply(): void { if (briefcase.mode === "update") briefcase.runUpdate(); }
        function bcCustomize(): void { app.open(""); briefcase.run("bc-customize", [], null); }
        function bcState(): string { return JSON.stringify({ mode: briefcase.mode, st: briefcase.st ? { name: briefcase.st.name, counts: briefcase.st.counts || {}, locked: briefcase.st.locked } : null, rows: briefcase.plan ? briefcase.plan.rows.length : 0 }); }
    }

    // ================================================ open / close
    function open(p) {
        let target = (p || "").trim();
        if (target.startsWith("file://")) target = decodeURIComponent(target.slice(7));
        if (target.startsWith("~")) target = home + target.slice(1);
        shown = true;
        // Opened with an archive (it's the default app for them): pick it and ask what to do with it.
        pendingArchive = target && /\.(zip|tar|tgz|txz|tbz2?|tzst|gz|xz|bz2|zst|7z|rar)$/i.test(target) ? base(target) : "";
        // smb://host/share (Settings › Your devices › Open): mounted (asking to sign in if needed), then opened.
        const smb = /^smb:\/\/([A-Za-z0-9._-]+)\/?([^/]*)\/?$/.exec(target);
        // hearthnet://<the JSON, URI-encoded> {"name", "share", "home", "away", "device", "wake"}: a device's share with both its names.
        let net = null;
        if (target.startsWith("hearthnet://")) { try { net = JSON.parse(decodeURIComponent(target.slice(12))); } catch (e) { net = null; } target = ""; }
        if (smb) target = "";
        const go = () => {
            if (net) { if (!listing) navigate(home, ""); netOpen(net); return; }
            if (smb) { if (!listing) navigate(home, ""); netOpen({ name: smb[1].split(".")[0], share: decodeURIComponent(smb[2] || ""), home: smb[1].endsWith(".ts.net") ? "" : smb[1], away: smb[1].endsWith(".ts.net") ? smb[1] : "" }); return; }
            // C:\Program Files, D:\Games… (hearth-filemanager, the launcher): the drive letters and Hearth (C:)'s names.
            const tl = target && !target.startsWith("/") ? fromLetters(target) : null;
            if (tl && tl.path) navigate(tl.path, "");
            else if (target) navigate(target, "");
            else if (!listing) navigate(picking ? home : startPage, "");
            else reload();
        };
        if (!prefsLoaded) loadPrefs(go);
        else { go(); refreshCPrefs(); }
        refreshPlaces();
        loadTemplates();
    }
    function close() {
        if (picking && !pickDone) { pickDone = true; picked({ cancel: true }); }
        shown = false;
        overlay = "";
        stopSearch();
        if (oskOpened) closeKeyboard();
    }
    function toggle() { shown ? close() : open(""); }

    // ================================================ picker mode
    function startPick(req) {
        pick = req;
        pickDone = false;
        const nf = (req.filters || []).length;
        pickFilter = nf ? Math.max(0, Math.min(nf - 1, req.filter >= 0 ? req.filter : 0)) : -1;
        const ch = {};
        for (const c of req.choices || []) ch[c.id] = c.value;
        pickChoice = ch;
        pickName = req.name || "";
        zone = "main";
        open(req.folder || home);
        if (req.kind === "save") { zone = "pick"; pickIndex = 0; Qt.callLater(() => { if (ui) ui.focusName(); }); }
    }
    // A glob ("*.tar.gz", "*.[jJ][pP][gG]", "Makefile") as a RegExp; patterns compare ignoring case.
    function globRe(g) {
        let s = "";
        for (let i = 0; i < g.length; i++) {
            const c = g[i];
            if (c === "*") s += ".*";
            else if (c === "?") s += ".";
            else if (c === "[") { const j = g.indexOf("]", i + 1); if (j > i) { s += "[" + g.slice(i + 1, j).replace(/^!/, "^").replace(/\\/g, "\\\\") + "]"; i = j; } else s += "\\["; }
            else s += c.replace(/[.+^${}()|\\\/\]]/g, "\\$&");
        }
        return new RegExp("^" + s + "$", "i");
    }
    readonly property var pickRes: {
        const f = picking && pickFilter >= 0 ? (pick.filters || [])[pickFilter] : null;
        if (!f) return null;
        return { res: (f.globs || []).map(globRe), mimes: f.mimes || [], any: (f.globs || []).indexOf("*") >= 0 || (f.mimes || []).indexOf("*/*") >= 0 || (f.mimes || []).indexOf("application/octet-stream") >= 0 };
    }
    // What the picker lists: folders always; files only when they pass the filter (none in folder mode).
    function pickShows(e) {
        if (e.dir) return true;
        if (pick.directory) return false;
        const r = pickRes;
        if (!r || r.any) return true;
        if (r.res.some(re => re.test(e.name))) return true;
        const m = e.mime || "";
        return r.mimes.some(x => x === m || (x.endsWith("/*") && m.startsWith(x.slice(0, -1))));
    }
    // What Open / Choose returns now: the picked items, else the one under the cursor (folder mode: this folder).
    function pickTargets() {
        if (!picking) return [];
        // Only real folders and files go back to the app (never a hearth:c name from the C:\ view).
        const sel = entries.filter(e => marked[e.path] && (pick.directory ? e.dir : !e.dir) && !DrivePaths.isCPath(e.path)).map(e => e.path);
        if (pick.directory) {
            if (sel.length) return pick.multiple ? sel : sel.slice(0, 1);
            return listing && !listing.error && found === null && !DrivePaths.isCPath(listing.path) ? [listing.path] : [];
        }
        if (sel.length) return pick.multiple ? sel : sel.slice(0, 1);
        return current && !current.dir && !DrivePaths.isCPath(current.path) ? [current.path] : [];
    }
    readonly property string pickAcceptLabel: {
        if (!picking) return "";
        if (pick.kind !== "open") return pick.accept;
        const t = pickTargets();
        if (pick.directory) return pick.accept + (t.length === 1 ? " “" + base(t[0]) + "”" : t.length > 1 ? " (" + t.length + ")" : "");
        return pick.accept + (t.length > 1 ? " (" + t.length + ")" : "");
    }
    // The bar's controls in order, for the keyboard and the controller.
    readonly property var pickItems: {
        if (!picking) return [];
        const out = [];
        if (pick.kind === "save") out.push({ t: "name" });
        if ((pick.filters || []).length > 1) out.push({ t: "filter" });
        (pick.choices || []).forEach((c, i) => out.push({ t: "choice", i: i }));
        out.push({ t: "cancel" }, { t: "accept" });
        return out;
    }
    function pickLit(t, i) { const it = pickItems[pickIndex]; return zone === "pick" && !!it && it.t === t && (i === undefined || it.i === i); }
    function pickRun(it) {
        if (!it) return;
        if (it.t === "name") { if (ui) ui.focusName(); }
        else if (it.t === "filter") filterMenu();
        else if (it.t === "choice") choiceRun(it.i);
        else if (it.t === "cancel") close();
        else if (it.t === "accept") pickAccept(null);
    }
    function filterMenu() {
        const ck = on => on ? "✓  " : "     ";
        showMenu("Show", (pick.filters || []).map((f, i) => ({ id: "pf:" + i, label: ck(i === pickFilter) + f.name })));
    }
    function choiceLabel(c) {
        const v = pickChoice[c.id];
        if (!c.options.length) return (v === "true" ? "☑  " : "☐  ") + c.label;
        const o = c.options.find(x => x[0] === v);
        return c.label + ": " + (o ? o[1] : v) + "  ▾";
    }
    function choiceRun(i) {
        const c = (pick.choices || [])[i];
        if (!c) return;
        if (!c.options.length) return setChoice(c.id, pickChoice[c.id] === "true" ? "false" : "true");
        const ck = on => on ? "✓  " : "     ";
        showMenu(c.label, c.options.map((o, j) => ({ id: "pc:" + i + ":" + j, label: ck(o[0] === pickChoice[c.id]) + o[1] })));
    }
    function setChoice(id, v) { const ch = Object.assign({}, pickChoice); ch[id] = v; pickChoice = ch; }
    function pickFinish(paths) {
        if (pickDone) return;
        pickDone = true;
        picked({ paths: paths, filter: pickFilter, choices: pickChoice });
        shown = false;
    }
    function pickAccept(given) {
        if (!picking || pickDone) return;
        if (pick.kind === "save") return pickSave();
        if (pick.kind === "savefiles") {
            if (!listing || listing.error || found !== null || DrivePaths.isCPath(path)) return say("Open the folder to save in first", true);
            const there = (listing.entries || []).filter(e => (pick.files || []).indexOf(e.name) >= 0).map(e => e.name);
            const dir = listing.path;
            if (!there.length) return pickFinish([dir]);
            return confirm(there.length === 1 ? "Replace “" + there[0] + "”?" : "Replace " + there.length + " files?",
                (there.length === 1 ? "A file with this name is" : "Files with these names are") + " already in “" + (listing.name || base(dir)) + "”. Replacing overwrites what's there.",
                "Replace", () => app.pickFinish([dir]));
        }
        const t = given || pickTargets();
        if (!t.length) return say(pick.directory ? "Pick a folder first" : "Pick a file first", true);
        pickFinish(t);
    }
    function pickSave() {
        let n = pickName.trim();
        if (!n) { if (ui) ui.focusName(); return say("Type a name first", true); }
        if (n.startsWith("~")) n = home + n.slice(1);
        // A typed C:\Users\chris\notes.txt (or G:\…) is the real path it stands for.
        if (/^[A-Za-z]:([\\/]|$)/.test(n)) {
            const w = DrivePaths.fromWindows(n, pathCtx);
            if (!w || !w.path || DrivePaths.isCPath(w.path)) return say(w && w.error ? w.error : "That's not a folder you can save in", true);
            n = w.path;
        }
        if (!n.startsWith("/") && DrivePaths.isCPath(path)) return say("Open a folder to save in first (your own folders are in Users)", true);
        const full = n.startsWith("/") ? n : (path === "/" ? "" : path) + "/" + n;
        const dir = parentOf(full), name = base(full);
        const decide = (list) => {
            const hit = list.find(e => e.name === name);
            if (hit && hit.dir) { pickName = ""; navigate(hit.path, ""); return; }
            if (hit) return confirm("Replace “" + name + "”?", "A file with this name is already in “" + base(dir) + "”. Replacing it overwrites what's there.",
                "Replace", () => app.pickFinish([full]));
            pickFinish([full]);
        };
        if (listing && dir === listing.path && (hidden || !name.startsWith("."))) return decide(listing.entries || []);
        call(["list", dir, "--hidden"], (r, err) => {
            if (err || !r || r.path !== dir) return say("There's no folder “" + dir + "”", true);
            decide(r.entries || []);
        });
    }

    // ================================================ talking to hearth-files
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
    Component {
        id: callInComp
        Process {
            id: procIn
            property var cb: null
            property string input: ""
            // On stdin, then stdin closes (see SetJob.qml: Process.environment can't take a JavaScript object on
            // Hearth's Debian, so the password never arrived and opening a shared folder with one failed).
            stdinEnabled: procIn.input !== ""
            onStarted: { if (procIn.input !== "") { procIn.write(procIn.input); procIn.input = ""; } }
            stdout: StdioCollector {
                onStreamFinished: {
                    const lines = text.trim().split("\n");
                    const last = lines.length ? lines[lines.length - 1] : "";
                    let res = null, err = "";
                    if (last.startsWith("error: ")) err = last.slice(7);
                    else { try { res = JSON.parse(last); } catch (e) { err = "Something went wrong"; } }
                    procIn.input = "";
                    // Always destroyed, even when the callback throws (a window that closed meanwhile): a
                    // leftover Process per call was part of the shell slowly growing over a day (1.11).
                    try { if (procIn.cb) procIn.cb(res, err); } finally { procIn.destroy(); }
                }
            }
        }
    }
    function call(args, cb) {
        const p = callComp.createObject(app, { command: ["python3", helper].concat(args), cb: cb || null });
        p.running = true;
    }
    // The same with text for the helper (a sharing password): written on its stdin, and put in HEARTH_INPUT (the
    // environment, which only you can read) by the sh wrapper (SetJob.qml's feed), never on the command line.
    readonly property string inputFeed: 'if [ "$1" = 1 ]; then HEARTH_INPUT=$(cat; printf x); HEARTH_INPUT=${HEARTH_INPUT%x}; else HEARTH_INPUT=; fi; export HEARTH_INPUT; shift; printf "%s" "$HEARTH_INPUT" | exec '
    function callIn(args, input, cb) {
        const p = callInComp.createObject(app, { command: ["sh", "-c", inputFeed + 'python3 "$@"', "sh", input ? "1" : "0", helper].concat(args), cb: cb || null, input: input || "" });
        p.running = true;
    }

    // ================================================ settings: the view, remembered per folder
    function loadPrefs(then) {
        call(["prefs"], (r, err) => {
            prefs = r || {};
            prefsLoaded = true;
            hidden = !!prefs.hidden;
            showExt = prefs.showExt !== false;
            detailsPane = !!prefs.detailsPane;
            pcView = prefs.thisPcView === "details" ? "details" : "grid";
            applyFolderView("");
            if (cdriveOn) loadCData(null);
            if (then) then();
        });
    }
    // Hearth (C:)'s switches can be changed in Settings while Files is open in the background: read again (only read;
    // nothing is written) each time the window opens.
    readonly property var cKeys: ["cdrive", "linuxFolders", "addressBar", "copyPath"]
    function refreshCPrefs() {
        call(["prefs"], (r, err) => {
            if (!r) return;
            const before = cKeys.map(k => JSON.stringify(prefs[k])).join();
            if (cKeys.map(k => JSON.stringify(r[k])).join() === before) return;
            const p = Object.assign({}, prefs);
            for (const k of cKeys) { if (r[k] === undefined) delete p[k]; else p[k] = r[k]; }
            prefs = p;
            places = buildNav(rawPlaces);
            if (cdriveOn && !Object.keys(cdata).length) loadCData(null);
            if (!cdriveOn && onCView) navigate(thisPcPage, "");
        });
    }
    function applyFolderView(p) {
        const f = (prefs.folders && p && prefs.folders[p]) || prefs;
        view = views.indexOf(f.view) >= 0 ? f.view : (f.view === "list" ? "details" : "grid");
        sort = ["name", "date", "size", "type"].indexOf(f.sort) >= 0 ? f.sort : "name";
        desc = !!f.desc;
        group = ["none", "type", "date"].indexOf(f.group) >= 0 ? f.group : "none";
    }
    // Only what you changed goes to the file (hearth-files merges it, one folder's view at a time): it used to send the
    // whole settings object this window had read when it opened, so a setting changed since (Settings › Devices'
    // "Show in Files", another Files window's folder views) was put back by itself the next time you changed a view.
    property var prefsPending: ({})
    Timer {
        id: prefsTimer; interval: 700
        onTriggered: { const ch = app.prefsPending; app.prefsPending = {}; if (Object.keys(ch).length) app.call(["prefs-set", JSON.stringify(ch)]); }
    }
    function rememberView() {
        const p = Object.assign({}, prefs);
        const folders = Object.assign({}, p.folders || {});
        const v = { view: view, sort: sort, desc: desc, group: group };
        const ch = Object.assign({}, prefsPending);
        if (path && !onHome && !onSpecial) {
            delete folders[path];
            folders[path] = v;               // newest last: hearth-files keeps the newest 400
            const cf = Object.assign({}, ch.folders || {});
            delete cf[path];
            cf[path] = v;
            ch.folders = cf;
        }
        p.folders = folders;
        p.view = view; p.sort = sort; p.desc = desc; p.group = group;
        p.hidden = hidden;
        p.showExt = showExt;
        p.detailsPane = detailsPane;
        prefs = p;
        ch.view = view; ch.sort = sort; ch.desc = desc; ch.group = group;
        ch.hidden = hidden; ch.showExt = showExt; ch.detailsPane = detailsPane;
        prefsPending = ch;
        prefsTimer.restart();
    }
    function loadTemplates() {
        call(["templates"], (r, err) => { if (r) templates = r.templates; });
    }

    // ================================================ moving between folders
    function navigate(p, sel) {
        zone = "main";
        editingPath = false;
        load(p, sel, "push");
    }
    function reload() {
        if (listing) load(listing.path, current ? current.name : "", "none");
    }
    function load(p, sel, hist) {
        if (p === homePage) return loadHome(hist);
        if (p === thisPcPage || p === appsPage) return loadPage(p, hist);
        // Hearth (C:)'s view: its own pages, or a name that stands for a real folder (AppData\Local\Temp = ~/.cache).
        if (DrivePaths.isCPath(p)) {
            const pg = DrivePaths.cPage(p, pathCtx);
            if (pg) return loadCPage(p, pg, sel, hist);
            const r = DrivePaths.cResolve(p, pathCtx);
            if (r.error || !r.path || DrivePaths.isCPath(r.path)) return say(r.error || "There's nothing called that on Hearth (C:)", true);
            return load(r.path, sel, hist);
        }
        const seq = ++loadSeq;
        loading = true;
        call(["list", p].concat(hidden ? ["--hidden"] : []), (r, err) => {
            if (seq !== loadSeq) return;
            loading = false;
            const prev = listing ? listing.path : "";
            if (err) r = { path: p, name: base(p), parent: parentOf(p), entries: [], writable: false, trash: false, error: err };
            if (r.path !== prev) {
                if (hist === "push" && prev) {
                    back = back.concat([prev]).slice(-100);
                    fwd = [];
                }
                query = "";
                stopSearch();
                found = null;
                marked = ({});
                anchor = -1;
                applyFolderView(r.path);
                // A Briefcase's own default view and sort (Customize), unless you chose one for this folder.
                if (r.briefcase && !(prefs.folders && prefs.folders[r.path])) {
                    if (views.indexOf(r.briefcase.view) >= 0) view = r.briefcase.view;
                    if (["name", "date", "size", "type"].indexOf(r.briefcase.sort) >= 0) sort = r.briefcase.sort;
                }
            }
            listing = r;
            path = r.path;
            briefcase.opened(r);
            const want = r.select || sel || "";
            const i = want ? entries.findIndex(e => e.name === want) : -1;
            cur = Math.max(0, i >= 0 ? i : (r.path === prev ? Math.min(cur, entries.length - 1) : 0));
            if (r.select) zone = "main";
            makeThumbs();
            if (pendingArchive) {
                const a = entries[cur];
                const want = pendingArchive;
                pendingArchive = "";
                if (a && a.name === want && !a.dir) openFile(a);
            }
        });
    }
    // The Home page (Win 11-style): Quick access tiles (the usual folders and your favorites) and recent files.
    function loadHome(hist) {
        loadSeq++;
        loading = false;
        const prev = listing ? listing.path : "";
        if (prev !== homePage) {
            if (hist === "push" && prev) {
                back = back.concat([prev]).slice(-100);
                fwd = [];
            }
            query = "";
            stopSearch();
            found = null;
            marked = ({});
            anchor = -1;
        }
        listing = { path: homePage, name: "Home", parent: "", entries: [], writable: false, trash: false, free: null, total: null };
        path = homePage;
        cur = 0;
        call(["recent"], (r, err) => { if (r && path === homePage) { recent = r.recent || []; cur = Math.min(cur, Math.max(0, entries.length - 1)); } });
    }
    // This PC and Installed apps: Hearth's own pages (the address bar says "This PC" / "This PC › Installed apps").
    function loadPage(p, hist) {
        loadSeq++;
        loading = false;
        const prev = listing ? listing.path : "";
        if (prev !== p) {
            if (hist === "push" && prev) {
                back = back.concat([prev]).slice(-100);
                fwd = [];
            }
            query = "";
            stopSearch();
            found = null;
            marked = ({});
            anchor = -1;
            cur = 0;
        }
        listing = { path: p, name: p === thisPcPage ? "This PC" : "Installed apps", parent: p === appsPage ? thisPcPage : "",
                    entries: [], writable: false, trash: false, free: null, total: null };
        path = p;
        if (p === thisPcPage) loadThisPc();
        else loadApps();
    }
    // Hearth (C:)'s own pages: C:\, Program Files, Program Files (x86) and Users\<you>\AppData. What they list comes
    // from hearth-files cdrive and Installed apps' list (read only); nothing is created anywhere.
    function loadCPage(p, pg, sel, hist) {
        loadSeq++;
        loading = false;
        const prev = listing ? listing.path : "";
        if (prev !== p) {
            if (hist === "push" && prev) {
                back = back.concat([prev]).slice(-100);
                fwd = [];
            }
            query = "";
            stopSearch();
            found = null;
            marked = ({});
            anchor = -1;
            cur = 0;
        }
        const names = { root: (rawPlaces.find(q => q.key === "system") || { label: "Hearth (C:)" }).label, pf: "Program Files", x86: "Program Files (x86)", appdata: "AppData" };
        listing = { path: p, name: names[pg] || "Hearth (C:)", parent: pg === "root" ? thisPcPage : pg === "appdata" ? home : cRoot,
                    entries: [], writable: false, trash: false, free: null, total: null };
        path = p;
        const pick = () => { if (sel) { const i = entries.findIndex(e => e.name === sel); if (i >= 0) cur = i; } };
        pick();
        const o = Object.assign({}, navOpen);
        if (!o.cdrive) { o.cdrive = true; navOpen = o; places = buildNav(rawPlaces); }
        loadCData(pick);
        if (!appsLoading && (pg === "pf" || pg === "x86" || (pg === "root" && !appList.length))) loadApps(pick);
    }
    function loadCData(then) {
        call(["cdrive"], (r, err) => {
            if (err) return say(err, true);
            if (r) cdata = r;
            if (then) then();
        });
    }
    function loadThisPc() {
        call(["this-pc"], (r, err) => {
            if (err) return say(err, true);
            if (r) pc = r;
            if (onThisPc) cur = Math.min(cur, Math.max(0, entries.length - 1));
        });
    }
    function loadApps(then) {
        appsLoading = true;
        call(["installed-apps"], (r, err) => {
            appsLoading = false;
            if (err) return say(err, true);
            appList = (r && r.apps) || [];
            if (onApps || onCView) cur = Math.min(cur, Math.max(0, entries.length - 1));
            if (then) then();
            // Sizes that need adding up (Windows apps' spaces), in the background, then shown.
            const need = appList.filter(a => a.sizePath && !a.size).map(a => a.sizePath);
            if (need.length) call(["app-sizes"].concat(need), (z, e2) => {
                if (!z || !z.sizes) return;
                appList = appList.map(a => a.sizePath && z.sizes[a.sizePath] !== undefined ? Object.assign({}, a, { size: z.sizes[a.sizePath] }) : a);
            });
        });
    }
    // This PC's items as entries (Folders, then Devices and drives, then Network locations).
    readonly property var pcEntries: {
        const out = [];
        for (const f of pc.folders || []) out.push({ name: f.label, path: f.path, dir: true, pc: "folder", item: f, size: 0, mtime: 0 });
        for (const d of pc.drives || []) out.push({ name: d.label, path: d.path || d.id, dir: true, pc: "drive", item: d, size: 0, mtime: 0 });
        for (const n of pc.network || []) out.push({ name: n.label, path: n.path || n.id, dir: true, pc: "net", item: n, size: 0, mtime: 0 });
        return out;
    }
    readonly property var appKinds: ["all", "hearth", "flatpak", "winapp", "steam", "recomp", "games"]
    function appKindName(k) {
        return ({ all: "All", hearth: "Hearth", flatpak: "Flatpak", winapp: "Windows app", steam: "Steam", recomp: "Recomp", games: "Hearth Games" })[k] || k;
    }
    readonly property var appEntries: {
        const f = appsFilter;
        let out = appList.filter(a => f === "all" || a.kind === f).map(a => ({ name: a.name, path: "app:" + a.id, dir: false, app: a, size: a.size || 0, mtime: a.date || 0 }));
        const by = appsSort;
        out.sort((a, b) => {
            let r = 0;
            if (by === "size") r = (b.size || 0) - (a.size || 0);
            else if (by === "date") r = (b.mtime || 0) - (a.mtime || 0);
            return r || natural(a.name, b.name);
        });
        return out;
    }
    // Walks This PC's groups the way Windows does: rows within a group, then on to the group above or below.
    // Returns the new index, -1 for "left of the first column" (the navigation pane), -2 for "above the first row".
    function pcStep(i, dx, dy) {
        const c = Math.max(1, pcView === "details" ? 1 : pcCols);
        const groups = [];
        for (let k = 0; k < entries.length; k++) {
            if (!groups.length || entries[k].pc !== entries[k - 1].pc) groups.push({ start: k, len: 0 });
            groups[groups.length - 1].len++;
        }
        const gi = groups.findIndex(g => i >= g.start && i < g.start + g.len);
        if (gi < 0) return 0;
        const g = groups[gi], col = (i - g.start) % c, row = Math.floor((i - g.start) / c);
        if (dx < 0) return col === 0 ? -1 : i - 1;
        if (dx > 0) return col < c - 1 && i + 1 < g.start + g.len ? i + 1 : i;
        if (dy < 0) {
            if (row > 0) return i - c;
            if (gi === 0) return -2;
            const p = groups[gi - 1];
            return Math.min(p.start + Math.floor((p.len - 1) / c) * c + col, p.start + p.len - 1);
        }
        if ((row + 1) * c < g.len) return Math.min(i + c, g.start + g.len - 1);
        if (gi === groups.length - 1) return i;
        const n = groups[gi + 1];
        return Math.min(n.start + col, n.start + n.len - 1);
    }
    function openPcItem(e) {
        const d = e.item || {};
        if (e.pc === "folder") return navigate(e.path, "");
        if (e.pc === "net") {
            if (d.path) return navigate(d.path, "");
            return netOpen(d.net);
        }
        if (d.mounted === false && d.device) return mountPlace({ label: d.name || d.label, device: d.device }, true);
        if (d.key === "system" && cdriveOn) return navigate(cRoot, "");
        if (d.path) navigate(d.path, "");
    }
    // ---- drive letters in the address bar (drive-paths.js)
    readonly property var pathCtx: {
        const drives = rawPlaces.filter(p => p.letter && p.path && p.mounted !== false).map(p => ({ letter: p.letter, path: p.path, label: p.label }));
        for (const n of pc.network || []) if (n.letter && n.path) drives.push({ letter: n.letter, path: n.path, label: n.label });
        const kinds = ["desktop", "documents", "downloads", "pictures", "music", "videos"];
        const folders = rawPlaces.filter(p => p.section === "places" && !p.child && kinds.indexOf(p.kind) >= 0).map(p => ({ label: p.label, path: p.path }));
        // Hearth (C:) with Windows folder names: the view's names (only when it's on; off, everything is as before).
        const cApps = DrivePaths.cAppList(cViewData, null).map(a => ({ name: a.name, path: a.path, x86: a.x86, noMap: !!a.noMap }));
        return { home: home, user: pc.user || cdata.user || base(home), drives: drives, folders: folders,
                 c: { on: cdriveOn, users: cdata.users || "", apps: cApps } };
    }
    // What the view's pages list (FilesCDrive.qml): Installed apps' list and hearth-files cdrive.
    readonly property var cViewData: ({ apps: appList, hearth: cdata.hearth || "", steam: cdata.steam || "", linux: cdata.linux || [],
                                        flatpak: cdata.flatpak || [], showLinux: cLinux })
    readonly property var cEntries: onCView ? DrivePaths.cEntries(cPageNow, pathCtx, cViewData) : []
    // The address bar's steps: the view's names unless you chose Linux paths (Settings › System › Drives & folders).
    readonly property var crumbCtx: cAddrLinux ? Object.assign({}, pathCtx, { c: Object.assign({}, pathCtx.c, { on: false }) }) : pathCtx
    function winPath(p) { return DrivePaths.toWindows(p, pathCtx); }
    function fromLetters(t) { return DrivePaths.fromWindows(t, pathCtx); }
    // What clicking the address bar shows: the view's C:\ form (or D:\ on a lettered drive), unless you chose Linux
    // paths; the view's own pages always by their C:\ name (they have no Linux path).
    function addressText(p) {
        if (DrivePaths.isCPath(p)) return winPath(p);
        if (cdriveOn && !cAddrLinux && p.indexOf("/") === 0 && !inTrash) return winPath(p);
        return DrivePaths.driveFor(p, pathCtx) && !(cdriveOn && cAddrLinux) ? winPath(p) : p;
    }
    // Copy path: the C:\ form or the Linux path, as Settings › System › Drives & folders says; the other one is always
    // in the menu too. The view's own folders have no Linux path: their C:\ form is copied.
    function copyForm(p, linux) {
        if (DrivePaths.isCPath(p)) return winPath(p);
        return linux ? p : (winPath(p) || p);
    }
    // "This PC › Games (D:) › SteamLibrary" for the details pane ("" when Files' own words fit better).
    function letterLocation(p) {
        const c = DrivePaths.crumbsFor(p, crumbCtx, thisPcPage);
        return c ? c.map(x => x.label).join(" › ") : "";
    }
    function driveLabelFor(mount) {
        const d = rawPlaces.find(p => p.letter && p.path === mount);
        return d ? d.label : base(mount);
    }
    readonly property var quickAccess: places.filter(q => q.path && q.path !== homePage && !q.child
        && (q.section === "places" || q.section === "bookmarks" || q.section === "games") && q.kind !== "trash").slice(0, 15)
    // The child of `to` that `from` is inside: picked when going back or up.
    function childOn(to, from) {
        if (!from || from === to || !under(from, to)) return "";
        return from.slice(to === "/" ? 1 : to.length + 1).split("/")[0];
    }
    function goBack() {
        if (!back.length) return;
        const to = back[back.length - 1], from = path;
        back = back.slice(0, -1);
        fwd = fwd.concat([from]);
        zone = "main";
        load(to, childOn(to, from), "none");
    }
    function goForward() {
        if (!fwd.length) return;
        const to = fwd[fwd.length - 1];
        fwd = fwd.slice(0, -1);
        back = back.concat([path]);
        zone = "main";
        load(to, "", "none");
    }
    function goUp(closeAtTop) {
        if (found !== null) { clearSearch(); return; }
        if (!listing) return;
        if (onHome || onThisPc) { if (closeAtTop) close(); return; }
        if (onApps) return navigate(thisPcPage, "");
        // Hearth (C:)'s view: up is the step before in the address bar (Firefox → Program Files → Hearth (C:) → This PC).
        const cc = crumbs;
        if (onCView || (cdriveOn && !cAddrLinux && cc.length >= 2 && cc[1].path === cRoot)) {
            if (cc.length >= 2) return navigate(cc[cc.length - 2].path, cc[cc.length - 1].label);
            return navigate(thisPcPage, "");
        }
        const p = listing.path;
        // Up from a drive (or the system's top): This PC, as in Windows.
        if (p === "/" || rawPlaces.some(q => q.letter && q.path === p)) return navigate(thisPcPage, "");
        if (p === home) {
            if (closeAtTop) close();
            return;
        }
        const r = rootOf(p);
        if (p === r.path) navigate(home, "");
        else navigate(parentOf(p), base(p));
    }
    function refreshPlaces() {
        call(["places"], (r, err) => {
            if (!r) return;
            const old = rawPlaces;
            rawPlaces = r.places.map(q => q.kind === "home" && !picking ? Object.assign({}, q, { label: "Home folder" }) : q);
            places = buildNav(rawPlaces);
            placeIndex = Math.min(placeIndex, places.length - 1);
            // A drive plugged in, taken out, mounted or unmounted while This PC is open: shown at once.
            const sig = l => l.filter(q => q.section === "drives").map(q => q.id + (q.mounted === false ? "-" : "+")).join("|");
            if (onThisPc && sig(old) !== sig(rawPlaces)) loadThisPc();
            // A drive that was taken out while you were in it: back to Home.
            for (const o of old) {
                if (o.section !== "drives" || !o.path || rawPlaces.some(n => n.path === o.path)) continue;
                if (path === o.path || path.startsWith(o.path + "/")) {
                    say("“" + o.label + "” was removed");
                    navigate(home, "");
                }
            }
        });
    }
    // 1.13: drives plugged in or taken out show at once (hearth-files watch --mounts); the slow look stays for shares.
    Timer { interval: 15000; repeat: true; running: app.shown; onTriggered: app.refreshPlaces() }
    Process {
        id: mountWatch
        command: ["python3", app.helper, "watch", "--mounts"]
        running: app.shown
        stdout: SplitParser { onRead: line => { if (String(line).trim() === "mounts") mountSettle.restart(); } }
    }
    Timer { id: mountSettle; interval: 600; onTriggered: { app.refreshPlaces(); if (app.onThisPc) app.loadThisPc(); } }

    // ------------------------------------------------ 1.13: the open folder updates by itself (like File Explorer)
    // hearth-files watch says "changed" when anything in the open folder is made, renamed, moved, deleted or written (by
    // any app, a download, the terminal or another PC on a share), and Files lists it again quietly: the selection,
    // the marks and where you scrolled stay. While you're renaming something it waits until you're done. "gone": the
    // folder itself was deleted or moved, so Files goes up to the nearest folder that's still there.
    readonly property string watchPath: (shown && listing && !listing.error && found === null && !onSpecial && !onHome && path === listing.path) ? listing.path : ""
    property bool refreshWaiting: false
    property int quietSeq: 0
    onWatchPathChanged: { folderWatch.running = false; if (watchPath !== "") watchStart.restart(); }
    Timer { id: watchStart; interval: 50; onTriggered: folderWatch.running = app.watchPath !== "" }
    Process {
        id: folderWatch
        command: ["python3", app.helper, "watch", app.watchPath]
        stdout: SplitParser { onRead: line => app.folderEvent(String(line).trim()) }
    }
    onRenamingChanged: if (renaming === "" && refreshWaiting) { refreshWaiting = false; refreshQuiet(); }
    function folderEvent(word) {
        if (word === "gone") {
            let up = parentOf(path);
            say("This folder was moved or deleted");
            return navigate(up || home, "");
        }
        if (word !== "changed") return;
        if (renaming !== "") { refreshWaiting = true; return; }
        refreshQuiet();
    }
    function refreshQuiet() {
        if (!listing || listing.error || loading) return;
        const p = listing.path;
        const seq = ++quietSeq;
        const keep = entries[cur] ? entries[cur].path : "";
        const y1 = grid.contentY, y2 = list.contentY;
        call(["list", p].concat(hidden ? ["--hidden"] : []), (r, err) => {
            if (seq !== quietSeq || err || !r || r.path !== p || path !== p || loading || found !== null) return;
            const sig = l => l.entries.map(e => e.name + "\u0001" + (e.size || 0) + "\u0001" + (e.modified || e.mtime || "")).join("\u0002");
            if (sig(r) === sig(listing)) return;
            listing = r;
            const still = {};
            for (const e of r.entries) if (marked[e.path]) still[e.path] = true;
            marked = still;
            const i = keep ? entries.findIndex(e => e.path === keep) : -1;
            cur = i >= 0 ? i : Math.max(0, Math.min(cur, entries.length - 1));
            makeThumbs();
            Qt.callLater(() => { grid.contentY = y1; list.contentY = y2; });
        });
    }
    // The navigation pane, Windows 11-style: Home, Favorites, your folders, Games, then This PC opened out with
    // Hearth (C:) and every lettered drive under it, Network (Hearth Storage and Your devices under it), Installed
    // apps and Trash. This PC's and Network's arrows fold them (for this session).
    property var rawPlaces: []
    // The left menu's width (1.11): 158 px unless you dragged it (files.json navWidth, saved when you let go).
    property real navW: prefs && prefs.navWidth >= 120 && prefs.navWidth <= 360 ? prefs.navWidth : 158
    function saveNavWidth() {
        const w = Math.round(navW);
        if (prefs && prefs.navWidth === w) return;
        prefs = Object.assign({}, prefs, { navWidth: w });
        call(["prefs-set", JSON.stringify({ navWidth: w })], (r, err) => { if (err) say(err, true); });
    }
    // The left menu the Windows 11 way (1.11, the owner's approved option A, docs/mockups/next-update/files-4-A-narrow.png):
    // Home, Gallery, Home server; your folders; This PC (Hearth (C:) open with Users, Program Files and Windows apps,
    // then the other drives); Network; the Linux folders folded under "Linux" (only with Show the Linux folders).
    function buildNav(list) {
        const out = [];
        const sorted = sortPlaces(list);
        const byId = id => sorted.find(q => q.id === id);
        if (!picking) out.push({ id: "start", label: "Home", path: homePage, kind: "start", section: "start" });
        const pics = byId("pictures");
        if (pics) out.push({ id: "gallery", label: "Gallery", path: pics.path, kind: "pictures", section: "start" });
        const devs = sorted.filter(q => q.section === "network" || q.section === "devices");
        if (devs.length) {
            out.push({ id: "homeserver", label: "Home server", path: "", kind: "netroot", section: "start", tree: "homeserver", open: !!navOpen.homeserver });
            if (navOpen.homeserver) for (const q of devs) out.push(Object.assign({}, q, { section: "start", depth: q.child ? 2 : 1 }));
        }
        // your folders (Home itself is This PC › Hearth (C:) › Users › you), then pinned ones, briefcases and games
        for (const q of sorted) if (q.section === "places" && q.id !== "home" && !q.child) out.push(Object.assign({}, q, { section: "folders" }));
        for (const q of sorted) if (q.section === "bookmarks" || q.section === "briefcases" || q.section === "games") out.push(Object.assign({}, q, { section: "folders" }));
        const drives = sorted.filter(q => q.section === "drives");
        const pcOpen = navOpen.thispc !== false;               // open unless you folded it
        out.push({ id: "thispc", label: "This PC", path: thisPcPage, kind: "thispc", section: "pc", tree: "thispc", open: pcOpen });
        if (pcOpen) for (const d of drives) {
            const sys = d.key === "system" && cdriveOn;
            const cOpen = navOpen.cdrive !== false;
            out.push(Object.assign({}, d, { section: "pc", child: true, depth: 1, kind: d.kind === "games" ? "drive" : d.kind, sub: "" },
                                   sys ? { path: cRoot, tree: "cdrive", open: cOpen } : {}));
            if (sys && cOpen) for (const f of cNavFolders) out.push(f);
        }
        out.push({ id: "netroot", label: "Network", path: "", kind: "netroot", section: "pc", tree: "network", open: !!navOpen.network, empty: !devs.length });
        if (navOpen.network) for (const q of devs) out.push(Object.assign({}, q, { section: "pc", depth: q.child ? 2 : 1 }));
        if (cLinux) {
            out.push({ id: "linux", label: "Linux", path: "", kind: "drive", section: "pc", tree: "linux", open: !!navOpen.linux });
            if (navOpen.linux) for (const f of linuxNavFolders) out.push(f);
        }
        return out;
    }
    readonly property var linuxNavFolders: [
        { id: "lx-root", label: "/", path: "/", kind: "folder", section: "pc", child: true, depth: 1 },
        { id: "lx-home", label: "/home", path: "/home", kind: "folder", section: "pc", child: true, depth: 1 },
        { id: "lx-usr", label: "/usr", path: "/usr", kind: "folder", section: "pc", child: true, depth: 1 },
        { id: "lx-etc", label: "/etc", path: "/etc", kind: "folder", section: "pc", child: true, depth: 1 },
        { id: "lx-var", label: "/var", path: "/var", kind: "folder", section: "pc", child: true, depth: 1 }
    ]
    readonly property var cNavFolders: [
        { id: "c-users", label: "Users", path: DrivePaths.usersDir(pathCtx), kind: "folder", section: "pc", child: true, depth: 2 },
        { id: "c-pf", label: "Program Files", path: cRoot + "/Program Files", kind: "folder", section: "pc", child: true, depth: 2 },
        { id: "c-apps", label: "Windows apps", path: appsPage, kind: "apps", section: "pc", child: true, depth: 2 }
    ]
    function toggleNav(k) {
        const o = Object.assign({}, navOpen);
        const wasOpen = (k === "thispc" || k === "cdrive") ? o[k] !== false : !!o[k];   // This PC and C: start open
        o[k] = !wasOpen;
        navOpen = o;
        const at = places[placeIndex] ? places[placeIndex].id : "";
        places = buildNav(rawPlaces);
        placeIndex = Math.max(0, places.findIndex(q => q.id === at));
    }

    // ------------------------------------------------ renaming in place
    property string renaming: ""               // the path whose name is a text box right now
    onPathChanged: renaming = ""
    function finishRename(text) {
        if (renaming === "") return;
        const e = entries.find(x => x.path === renaming);
        renaming = "";
        if (!e || text === null || text === "" || text === e.name) return;
        call(["rename", e.path, text], (r, err) => {
            if (err) return say(err, true);
            if (r.path !== e.path) pushUndo({ kind: "rename", from: e.path, to: r.path });
            load(listing.path, base(r.path), "none");
        });
    }

    // ------------------------------------------------ type-ahead
    property string typed: ""
    property double typedAt: 0
    function typeAhead(ch) {
        const now = Date.now();
        const buf = (now - typedAt < 1000 ? typed : "") + ch;
        const list = entries || [];
        const low = buf.toLowerCase();
        // The same letter again steps through the names starting with it.
        const same = buf.length > 1 && buf.split("").every(c => c.toLowerCase() === low[0]);
        const want = same ? low[0] : low;
        const start = same || buf.length === 1 ? cur + 1 : cur;
        for (let k = 0; k < list.length; k++) {
            const i = (start + k) % list.length;
            if (shownName(list[i]).toLowerCase().startsWith(want)) {
                typed = buf;
                typedAt = now;
                zone = "main";
                cur = i;
                anchor = i;
                marked = ({});
                return true;
            }
        }
        if (typed && now - typedAt < 1000) { typedAt = now; return true; }   // a typo in the middle: stay put
        return false;
    }

    // ------------------------------------------------ tabs
    function saveTab() {
        const t = tabs.slice();
        t[tabIndex] = { path: path, back: back, fwd: fwd, sel: current ? current.name : "" };
        tabs = t;
    }
    function newTab(p) {
        saveTab();
        tabs = tabs.concat([{ path: p || path || home, back: [], fwd: [], sel: "" }]);
        tabIndex = tabs.length - 1;
        back = [];
        fwd = [];
        zone = "main";
        load(tabs[tabIndex].path, "", "none");
    }
    // Dragging a tab along the strip puts it somewhere else (the open tab stays open).
    function moveTab(from, to) {
        if (from === to || from < 0 || to < 0 || from >= tabs.length || to >= tabs.length) return;
        saveTab();
        const t = tabs.slice();
        const open = t[tabIndex];
        const m = t.splice(from, 1)[0];
        t.splice(to, 0, m);
        tabs = t;
        tabIndex = t.indexOf(open);
    }
    function switchTab(i) {
        if (i === tabIndex || i < 0 || i >= tabs.length) return;
        saveTab();
        tabIndex = i;
        const t = tabs[i];
        back = t.back;
        fwd = t.fwd;
        load(t.path, t.sel, "none");
    }
    function closeTab(i) {
        if (tabs.length <= 1) return close();
        saveTab();
        const t = tabs.slice();
        t.splice(i, 1);
        if (i === tabIndex) {
            tabs = t;
            tabIndex = Math.min(i, t.length - 1);
            const n = t[tabIndex];
            back = n.back;
            fwd = n.fwd;
            load(n.path, n.sel, "none");
        } else {
            if (i < tabIndex) tabIndex--;
            tabs = t;
        }
        tabFocus = Math.min(tabFocus, tabs.length - 1);
        if (tabs.length <= 1 && zone === "tabs") zone = "toolbar";
    }

    // ================================================ the folder's entries, filtered, grouped and sorted
    readonly property var entries: {
        if (onCView) {
            const cq = query.trim().toLowerCase();
            return cq ? cEntries.filter(e => e.name.toLowerCase().indexOf(cq) >= 0) : cEntries;
        }
        if (path === thisPcPage || path === appsPage) {
            const pq = query.trim().toLowerCase();
            const all = path === thisPcPage ? pcEntries : appEntries;
            return pq ? all.filter(e => e.name.toLowerCase().indexOf(pq) >= 0) : all;
        }
        if (path === homePage) {
            const hq = query.trim().toLowerCase();
            return hq ? recent.filter(e => e.name.toLowerCase().indexOf(hq) >= 0) : recent;
        }
        // With the view on, your home folder shows AppData and AppData\Local shows Temp (names for other folders).
        const extra = found === null && listing && cdriveOn ? DrivePaths.cExtras(listing.path, pathCtx, listing.entries.map(e => e.name)) : [];
        const all = found !== null ? found : (listing ? listing.entries.concat(extra) : []);
        const q = found !== null ? "" : query.trim().toLowerCase();
        let out = q ? all.filter(e => e.name.toLowerCase().indexOf(q) >= 0) : all.slice();
        if (picking) out = out.filter(e => pickShows(e));
        // Sorted with every key worked out once per entry (1.11): comparing split names and dates inside the sort
        // made folders with thousands of files take seconds to open.
        const by = sort, d = desc ? -1 : 1, g = group;
        const day = (() => { const n = new Date(); return new Date(n.getFullYear(), n.getMonth(), n.getDate()).getTime() / 1000; })();
        const keyed = out.map(e => ({ e: e, g: g === "none" ? 0 : groupRankAt(e, g, day), n: naturalKey(e.name), x: by === "type" ? extOf(e) : "" }));
        keyed.sort((A, B) => {
            if (A.g !== B.g) return A.g - B.g;
            const a = A.e, b = B.e;
            if (a.dir !== b.dir) return a.dir ? -1 : 1;
            let r = 0;
            if (by === "date") r = a.mtime - b.mtime;
            else if (by === "size") r = a.size - b.size;
            else if (by === "type") r = A.x < B.x ? -1 : A.x > B.x ? 1 : 0;
            if (r === 0) r = naturalCmp(A.n, B.n, a.name, b.name);
            return d * r;
        });
        return keyed.map(k => k.e);
    }
    readonly property var current: entries.length > cur && cur >= 0 ? entries[cur] : null
    readonly property bool writable: !!(listing && listing.writable) && found === null
    readonly property bool inTrash: !!(listing && listing.trash)
    readonly property string trashPath: {
        const t = rawPlaces.find(p => p.kind === "trash");          // Trash isn't in the left menu (the desktop has it)
        return t ? t.path : home + "/.local/share/Trash/files";
    }
    function extOf(e) {
        if (e.dir) return "";
        const i = e.name.lastIndexOf(".");
        return i > 0 ? e.name.slice(i + 1).toLowerCase() : "";
    }
    readonly property var kindOrder: ["folder", "doc", "text", "image", "video", "audio", "archive", "app", "disc", "file"]
    function groupRank(e, g) {
        if (g === "type") return kindOrder.indexOf(kindOf(e));
        const now = new Date();
        const day = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime() / 1000;
        const t = e.mtime || 0;
        if (t >= day) return 0;
        if (t >= day - 86400) return 1;
        if (t >= day - 6 * 86400) return 2;
        if (t >= day - 30 * 86400) return 3;
        if (new Date(t * 1000).getFullYear() === now.getFullYear()) return 4;
        return 5 + (now.getFullYear() - new Date(t * 1000).getFullYear());
    }
    function groupName(e) {
        if (!e || group === "none") return "";
        if (group === "type") return ({ folder: "Folders", doc: "Documents", text: "Text", image: "Pictures", video: "Videos", audio: "Music and audio",
            archive: "Archives", app: "Apps and programs", disc: "Disc images", file: "Other files" })[kindOf(e)];
        const r = groupRank(e, "date");
        return ["Today", "Yesterday", "Earlier this week", "Earlier this month", "Earlier this year"][r] || String(new Date(e.mtime * 1000).getFullYear());
    }

    // "file 9" before "file 10"; case doesn't matter.
    // A name split once into text and number parts, for naturalCmp ("file 2" before "file 10").
    function naturalKey(name) { return name.toLowerCase().split(/(\d+)/); }
    function naturalCmp(x, y, a, b) {
        for (let i = 0; i < Math.min(x.length, y.length); i++) {
            if (x[i] === y[i]) continue;
            if (i % 2 === 1) return parseInt(x[i], 10) - parseInt(y[i], 10) || x[i].length - y[i].length;
            return x[i] < y[i] ? -1 : 1;
        }
        return x.length - y.length || (a < b ? -1 : a > b ? 1 : 0);
    }
    function groupRankAt(e, g, day) {
        if (g === "type") return kindOrder.indexOf(kindOf(e));
        return groupRank(e, g);
    }
    function natural(a, b) {
        const x = a.toLowerCase().split(/(\d+)/), y = b.toLowerCase().split(/(\d+)/);
        for (let i = 0; i < Math.min(x.length, y.length); i++) {
            if (x[i] === y[i]) continue;
            if (i % 2 === 1) return parseInt(x[i], 10) - parseInt(y[i], 10) || x[i].length - y[i].length;
            return x[i] < y[i] ? -1 : 1;
        }
        return x.length - y.length || (a < b ? -1 : a > b ? 1 : 0);
    }

    // ================================================ paths, words and numbers
    function base(p) {
        const s = p.replace(/\/+$/, "");
        return s.slice(s.lastIndexOf("/") + 1) || "/";
    }
    // The navigation pane's order (Win 11-style): Home, Favorites, This device, Games, Drives, Hearth Storage,
    // Your devices, Trash. Within a group hearth-files' order stays.
    function sortPlaces(list) {
        const rank = { bookmarks: 1, places: 2, briefcases: 2.5, games: 3, drives: 4, network: 5, devices: 6, trash: 7 };
        return list.map((q, i) => [q, i]).sort((a, b) => ((rank[a[0].section] || 5) - (rank[b[0].section] || 5)) || a[1] - b[1]).map(x => x[0]);
    }
    // A tab's or the window's name for a path: "Home" for the Home page.
    function titleOf(p) {
        if (!p || p === homePage) return "Home";
        if (p === thisPcPage) return "This PC";
        if (p === appsPage) return "Installed apps";
        if (DrivePaths.isCPath(p)) {
            const pg = DrivePaths.cPage(p, pathCtx);
            return pg === "root" ? (rawPlaces.find(q => q.key === "system") || { label: "Hearth (C:)" }).label : base(p);
        }
        const d = rawPlaces.find(q => q.letter && q.path === p);
        if (d) return d.label;
        return p === "/" ? "This device" : base(p);
    }
    function parentOf(p) {
        const s = p.replace(/\/+$/, "");
        const i = s.lastIndexOf("/");
        return i <= 0 ? "/" : s.slice(0, i);
    }
    function under(p, root) { return p === root || p.startsWith(root.replace(/\/$/, "") + "/"); }
    // Where a path starts from, for the path bar and for B: the Trash, a drive, Home, or the whole computer.
    readonly property var roots: {
        const r = [{ path: trashPath, label: "Trash" }];
        for (const p of rawPlaces) if (p.section === "drives" && p.path && p.path !== "/") r.push({ path: p.path, label: p.label });
        r.push({ path: home, label: "Home" });
        r.push({ path: "/", label: "Computer" });
        return r;
    }
    function rootOf(p) {
        let best = roots[roots.length - 1];
        for (const r of roots) if (under(p, r.path) && r.path.length > (best.path === "/" ? 0 : best.path.length)) best = r;
        return best;
    }
    readonly property var crumbs: {
        if (!path) return [];
        if (path === homePage) return [{ label: "Home", path: homePage }];
        if (path === thisPcPage) return [{ label: "This PC", path: thisPcPage }];
        if (path === appsPage) return [{ label: "This PC", path: thisPcPage }, { label: "Installed apps", path: appsPage }];
        // On a lettered drive, in your folders or elsewhere on the system: This PC › Games (D:) › … (drive-paths.js);
        // with Hearth (C:)'s view: This PC › Hearth (C:) › Program Files › …
        if (!inTrash) {
            const lc = DrivePaths.crumbsFor(path, crumbCtx, thisPcPage);
            if (lc) return lc;
        }
        const r = rootOf(path);
        const out = [{ label: r.label, path: r.path }];
        const rest = path.slice(r.path === "/" ? 1 : r.path.length + 1);
        let at = r.path === "/" ? "" : r.path;
        for (const part of rest.split("/")) {
            if (!part) continue;
            at += "/" + part;
            out.push({ label: part, path: at });
        }
        return out;
    }
    // The toolbar, left to right (for the keyboard and the controller).
    readonly property var toolItems: {
        // Left to right: the navigation row, then the command bar (Left / Right walks through all of it).
        const t = [{ t: "back" }, { t: "forward" }, { t: "up" }, { t: "refresh" }];
        for (const c of crumbs) t.push({ t: "crumb", path: c.path, label: c.label });
        t.push({ t: "edit" }, { t: "search" });
        if (!picking) for (const c of ["new", "cut", "copy", "paste", "rename", "copypath", "delete"]) t.push({ t: c });
        t.push({ t: "sort" }, { t: "view" }, { t: "hidden" });
        if (!picking) for (const c of ["bc-update-all", "bc-unlock", "bc-lock", "bc-customize"]) if (briefcase.toolShown(c)) t.push({ t: c });
        if (!picking) t.push({ t: "undo" }, { t: "redo" }, { t: "details" });
        return t;
    }
    function toolLit(name) { return zone === "toolbar" && !!toolItems[toolIndex] && toolItems[toolIndex].t === name; }

    function fmtSize(n) {
        if (n === undefined || n === null) return "";
        if (n < 1024) return n + (n === 1 ? " byte" : " bytes");
        const u = ["KB", "MB", "GB", "TB"];
        let v = n / 1024, i = 0;
        while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
        return (v >= 100 ? Math.round(v) : v.toFixed(1)) + " " + u[i];
    }
    function fmtDate(s) {
        if (!s) return "";
        const d = new Date(s * 1000), now = new Date();
        const t = Qt.formatTime(d, "hh:mm");
        const day = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
        if (d.getTime() >= day) return "Today, " + t;
        if (d.getTime() >= day - 86400000) return "Yesterday, " + t;
        if (d.getFullYear() === now.getFullYear()) return Qt.formatDate(d, "d MMM") + ", " + t;
        return Qt.formatDate(d, "d MMM yyyy");
    }
    function fmtTime(sec) {
        if (!isFinite(sec) || sec <= 0) return "";
        if (sec < 60) return "less than a minute";
        if (sec < 3600) return Math.round(sec / 60) + " min";
        return Math.floor(sec / 3600) + " h " + Math.round((sec % 3600) / 60) + " min";
    }
    function plural(n, one, many) { return n + " " + (n === 1 ? one : (many || one + "s")); }

    function kindOf(e) {
        if (!e) return "file";
        if (e.dir) return "folder";
        const m = e.mime || "";
        if (m.startsWith("image/")) return "image";
        if (m.startsWith("video/")) return "video";
        if (m.startsWith("audio/")) return "audio";
        if (/iso9660|cd-image|x-raw-disk/.test(m)) return "disc";
        if (/zip|x-7z|rar|x-tar|gzip|x-xz|zstd|bzip|x-compressed|x-rpm/.test(m)) return "archive";
        if (/appimage|ms-dos-executable|x-msi|x-desktop|flatpak|x-executable/.test(m) || e.exec) return "app";
        if (/pdf|msword|opendocument|officedocument|epub|rtf/.test(m)) return "doc";
        if (m.startsWith("text/") || /json|xml|javascript|shellscript|x-yaml|toml/.test(m)) return "text";
        return "file";
    }
    readonly property var winTypes: ({ py: "Python file", pdf: "PDF document", jpg: "JPEG image", jpeg: "JPEG image", png: "PNG image", gif: "GIF image",
        webp: "WebP image", svg: "SVG image", bmp: "Bitmap image", txt: "Text document", md: "Markdown file", json: "JSON file", xml: "XML document",
        html: "HTML document", htm: "HTML document", css: "CSS file", js: "JavaScript file", sh: "Shell script", zip: "Compressed (zipped) folder",
        "7z": "7-Zip archive", rar: "RAR archive", gz: "GZ archive", xz: "XZ archive", tar: "TAR archive", exe: "Application", msi: "Windows Installer package",
        iso: "Disc image file", mp4: "MP4 video", mkv: "MKV video", webm: "WebM video", avi: "AVI video", mov: "QuickTime movie", mp3: "MP3 audio",
        flac: "FLAC audio", ogg: "OGG audio", wav: "WAV audio", m4a: "M4A audio", odt: "OpenDocument text", ods: "OpenDocument spreadsheet",
        odp: "OpenDocument presentation", docx: "Microsoft Word document", doc: "Microsoft Word 97–2003 document", xlsx: "Microsoft Excel worksheet",
        pptx: "Microsoft PowerPoint presentation", csv: "CSV file", deb: "Debian package", appimage: "AppImage app", desktop: "Shortcut", log: "Text document",
        ini: "Configuration settings", cfg: "Configuration settings", conf: "Configuration settings", ttf: "TrueType font", otf: "OpenType font", qml: "QML file",
        rs: "Rust source file", c: "C source file", cpp: "C++ source file", h: "C header file", toml: "TOML file", yaml: "YAML file", yml: "YAML file" })
    // The Type column and the details pane the Windows way (1.12, mockups files-1 … -8, cdrive-1): "File folder", "Python file"
    function kindName(e) {
        if (!e) return "";
        if (e.dir) return "File folder";
        const x0 = extOf(e).toLowerCase();
        if (winTypes[x0]) return winTypes[x0];
        if (x0) return x0.toUpperCase() + " file";
        return "File";
    }
    function kindNameOld(e) {
        const k = kindOf(e);
        const x = extOf(e);
        return ({ folder: "Folder", image: "Picture", video: "Video", audio: "Audio", disc: "Disc image", archive: "Archive",
            app: "Program", doc: "Document", text: "Text", file: "File" })[k] + (x && k !== "folder" ? " (" + x.toUpperCase() + ")" : "");
    }
    function extLabel(e) {
        if (!e || e.dir) return "";
        const x = extOf(e).toUpperCase();
        return x && x.length <= 4 ? x : ({ image: "IMG", video: "VID", audio: "AUD", disc: "ISO", archive: "ZIP", app: "APP", doc: "DOC", text: "TXT", file: "" })[kindOf(e)];
    }
    function isAppImage(e) { return !!e && !e.dir && /\.appimage$/i.test(e.name); }
    function isDeb(e) { return !!e && !e.dir && /\.deb$/i.test(e.name); }
    // A Windows program: opened with the Proton built into Hearth OS in its own space (hearth-winapps, docs/WINDOWS-APPS.md).
    function isWinApp(e) { return !!e && !e.dir && /\.(exe|msi)$/i.test(e.name); }
    function openWinApp(e) { say("Opening “" + e.name + "” with Proton…"); Quickshell.execDetached(["hearth-winapps", "open", e.path]); }
    // An AppImage: runs (made runnable first, after "Run this app?" the first time). hearth-files falls back to
    // running it unpacked when FUSE can't mount it.
    function runAppImage(e) {
        const go = trust => {
            say("Starting “" + e.name + "”…");
            call(["appimage-run", e.path].concat(trust ? ["--trust"] : []), (r, err) => {
                if (err) return say(err, true);
                say("Started “" + e.name + "”" + (r && r.fallback ? " (unpacked first: FUSE isn't available)" : ""));
                if (trust) reload();
            });
        };
        if (e.exec) return go(false);
        ask("Run this app?", "“" + e.name + "” comes from " + parentOf(e.path).replace(home, "~")
            + ". Only run apps you trust: an app can do anything you can. Hearth marks it as runnable, so it won't ask again.", "Run", () => go(true));
    }
    function addAppImage(e) {
        const go = trust => {
            say("Adding “" + e.name + "” to the app menu…");
            call(["appimage-menu-add", e.path].concat(trust ? ["--trust"] : []), (r, err) => {
                if (err) return say(err, true);
                say((r.name || e.name) + " is in your app menu" + (r.moved ? " (the AppImage is now in Applications in your home folder)" : ""));
                reload();
            });
        };
        if (e.exec) return go(false);
        ask("Add this app to your menu?", "“" + e.name + "” comes from " + parentOf(e.path).replace(home, "~")
            + ". Only add apps you trust. It's moved to the Applications folder in your home folder and marked as runnable.", "Add", () => go(true));
    }
    function installDeb(e) {
        if (SettingsNav.debs) SettingsNav.debs.open(e.path);
        else Quickshell.execDetached(["hearth-install-package", e.path]);
    }
    function isArchive(e) {
        return !!e && !e.dir && /\.(zip|tar|tgz|txz|tbz2?|tzst|gz|xz|bz2|zst|7z|rar|cbz|cbr|jar|apk)$/i.test(e.name);
    }
    function archiveStem(name) {
        const m = name.match(/^(.*?)(\.tar\.(gz|xz|bz2|zst)|\.[a-z0-9]+)$/i);
        return m && m[1] ? m[1] : name;
    }
    // file:// URL for a path, encoded the way GLib does (thumbnail names are the MD5 of this).
    function fileUrl(p) {
        let s = "";
        const bytes = unescape(encodeURIComponent(p));
        for (let i = 0; i < bytes.length; i++) {
            const c = bytes.charAt(i);
            if (/[A-Za-z0-9\-._~!$&'()*+,;=:@\/]/.test(c)) s += c;
            else s += "%" + ("0" + bytes.charCodeAt(i).toString(16).toUpperCase()).slice(-2);
        }
        return "file://" + s;
    }
    function urlToPath(u) {
        const s = u.toString();
        if (!s.startsWith("file://")) return "";
        try { return decodeURIComponent(s.slice(7).replace(/^localhost/, "")); } catch (e) { return ""; }
    }
    readonly property string thumbDir: (Quickshell.env("XDG_CACHE_HOME") || home + "/.cache") + "/thumbnails/large/"
    function thumbFor(e) {
        if (!e || e.dir) return "";
        const k = kindOf(e);
        if (k === "image" && e.size < 80 * 1024 * 1024 && !/heic|jxl|avif|x-raw|tiff|psd|xcf/.test(e.mime)) return fileUrl(e.path);
        if ((k === "video" || e.mime === "application/pdf") && thumbHave[e.path]) return "file://" + thumbDir + Qt.md5(fileUrl(e.path)) + ".png";
        return "";
    }
    // Paths whose cached thumbnail hearth-files says is there: only those are shown (no "Cannot open" warnings).
    property var thumbHave: ({})
    // Video thumbnails nobody made yet: hearth-files makes them in the background (if ffmpeg is installed), and
    // says which videos and PDFs already have one. The details pane uses them too, so this runs in every view.
    function makeThumbs() {
        if (!listing) return;
        const want = (listing.entries || []).filter(e => !e.dir && (kindOf(e) === "video" || e.mime === "application/pdf"))
            .slice(0, 60).map(e => e.path);
        if (!want.length) return;
        const at = listing.path;
        call(["thumbs"].concat(want), (r, err) => {
            if (!r || !r.have || !listing || listing.path !== at) return;
            const h = {};
            for (const p of r.have) h[p] = true;
            thumbHave = h;
        });
    }

    // ================================================ picking things
    function activate(e) {
        if (!e) return;
        if (e.cv) return navigate(e.path, "");          // Hearth (C:)'s view: its folders open where they really are
        if (e.pc) return openPcItem(e);
        if (e.app) return openMenu(false, -1, -1);     // Installed apps: A / Enter is the "···" menu
        // Picking: folders go in; a file is the answer (opening), or the name to save as.
        if (picking && !e.dir) {
            if (pick.directory) return;
            if (pick.kind === "open") return pickAccept(marked[e.path] && pick.multiple ? null : [e.path]);
            if (pick.kind === "save") { pickName = e.name; zone = "pick"; pickIndex = 0; if (ui) ui.focusName(); }
            return;
        }
        if (inTrash && !picking) return openMenu(false, -1, -1);
        if (e.dir) {
            if (found !== null) clearSearch();
            return navigate(e.path, "");
        }
        if (isAppImage(e)) return runAppImage(e);
        if (isDeb(e)) return installDeb(e);
        if (isWinApp(e)) return openWinApp(e);
        openFile(e);
    }
    function openPlace(i) {
        const p = places[i];
        if (!p) return;
        placeIndex = i;
        if (p.path === homePage || p.path === thisPcPage || p.path === appsPage) return navigate(p.path, "");
        if (p.kind === "netroot") return toggleNav("network");
        if (p.net) return netOpen(p.net);
        if (p.kind === "howto") return showHowto(p.howto || []);
        if (p.kind === "briefcase") return briefcase.openPlace(p);
        if (p.kind === "device") {
            if (p.offline && p.wake) ask("Wake “" + p.label + "”?", "It's off or asleep. Hearth can wake it over the network (Wake-on-LAN).", "Wake it", () => netWake(p.device, p.label));
            else say(p.label + ": " + (p.sub || ""));
            return;
        }
        if (!p.path && p.device) return mountPlace(p, true);
        navigate(p.path, "");
    }
    // ================================================ your devices' shared folders (hearth_netshares.py)
    // One click: mounted with gvfs and opened (the home name at home, the Tailscale name away). The first time it asks
    // for the user name and sharing password; Remember keeps the password in the system keyring, never in our files.
    function netOpen(net, user, password, remember) {
        say("Opening “" + (net.share || net.name) + "”…");
        const args = ["net-open", JSON.stringify(net)].concat(user ? ["--user", user] : []).concat(remember ? ["--remember"] : []);
        callIn(args, password ? JSON.stringify({ password: password }) : "", (r, err) => {
            if (err) return say(err, true);
            if (r.ok && r.path) {
                if (r.remembered === false) say("Opened. The password couldn't be kept in the keyring, so it's asked again next time", true);
                else say("“" + (net.share || net.name) + "” is open" + (r.where === "away" ? " (through Tailscale)" : ""));
                return navigate(r.path, "");
            }
            if (r.need === "password") return netSignIn(net, r.user || user || "", !!password, password ? r.error : "");
            // A whole computer with several shared folders: which one (typed, the first one filled in).
            if (r.need === "pick" && r.shares && r.shares.length)
                return askText("Which shared folder on " + (net.name || "it") + "? " + r.shares.join(", "), r.shares[0], "Open", r.shares[0].length,
                               s => netOpen(Object.assign({}, net, { share: s }), r.user || user, password, false));
            if (r.need === "asleep" && (r.wake || net.wake) && net.device)
                return ask((net.name || "It") + " isn't answering", (r.error || "It may be off or asleep") + ". Hearth can wake it over the network (Wake-on-LAN), then try again.", "Wake it", () => netWake(net.device, net.name));
            // Everything else says plainly what went wrong and what to do (hearth_netshares.classify).
            ask("Couldn't open “" + (net.share || net.name || "it") + "”", (r.error || "Couldn't open it") + (r.detail && r.detail !== r.error ? "\n\n(" + r.detail + ")" : ""), "OK", () => {});
        });
    }
    function netSignIn(net, user, wrong, why) {
        askText((wrong ? (why || "Wrong user name or password") + ". " : "") + "Sign in to " + (net.name || "the device") + ": user name", user, "Next", user.length, u => {
            askText("Sharing password for " + u + " on " + (net.name || "the device"), "", "Sign in", 0, pw => {
                ask("Remember this password?", "Kept in this device's keyring (the system's password store), never in Hearth's files. Then it's one click next time.", "Remember", () => netOpen(net, u, pw, true));
                dlgAlt = "Just this time";
                dlgAltAction = () => netOpen(net, u, pw, false);
            });
            dlgSecret = true;
        });
    }
    function netWake(device, name) {
        call(["net-wake", device], (r, err) => say(err ? err : "Waking " + (name || "it") + "… try again in a minute", !!err));
    }
    function showHowto(list) {
        const text = list.length ? list.map(l => l.device + " › " + l.share + "\nWindows: " + l.windows + "\nPhone, Mac or TV: " + l.phone + (l.away ? "\nAway (Tailscale): " + l.away : "")).join("\n\n")
            : "Share a folder on a Hearth device (Settings › Home server › Sharing) and its addresses show here.";
        ask("How to open from Windows or a phone", "In Windows File Explorer, type the \\\\ address in the address bar. On a phone, Mac or TV, use a Files app with network storage (SMB) and the smb:// address.\n\n" + text,
            "Copy the addresses", () => { Quickshell.execDetached(["wl-copy", text]); say("Copied"); });
    }
    function mountPlace(p, go) {
        say("Mounting “" + p.label + "”…");
        call(["mount", p.device], (r, err) => {
            if (err) return say(err, true);
            say("“" + p.label + "” is ready");
            refreshPlaces();
            if (go && r.path) navigate(r.path, "");
        });
    }
    function toggleMark(e) {
        if (!e) return;
        const m = Object.assign({}, marked);
        if (m[e.path]) delete m[e.path];
        else m[e.path] = true;
        marked = m;
    }
    function selectAll() {
        const m = {};
        for (const e of entries) m[e.path] = true;
        marked = m;
    }
    function invertSelection() {
        const m = {};
        for (const e of entries) if (!marked[e.path]) m[e.path] = true;
        marked = m;
    }
    function selectRange(from, to) {
        const m = {};
        for (let i = Math.min(from, to); i <= Math.max(from, to); i++) if (entries[i]) m[entries[i].path] = true;
        marked = m;
    }
    function targets() {
        if (onSpecial) return [];             // This PC's folders and drives are opened, not cut, copied or deleted
        // Hearth (C:)'s names for other folders (AppData, Temp) are never cut, copied, renamed or deleted.
        if (markCount) return entries.filter(e => marked[e.path] && !DrivePaths.isCPath(e.path)).map(e => e.path);
        return current && !DrivePaths.isCPath(current.path) ? [current.path] : [];
    }
    function setHidden(on) {
        hidden = on;
        rememberView();
        reload();
        say(on ? "Showing hidden files" : "Hiding hidden files");
    }
    function setSort(k) {
        if (sort === k) desc = !desc;
        else {
            sort = k;
            desc = k === "date" || k === "size";
        }
        rememberView();
    }
    function setGroup(g) { group = g; rememberView(); }
    function setView(v) {
        if (views.indexOf(v) < 0) return;
        if (onThisPc) return setPcView(v === "details" ? "details" : "grid");
        if (onApps || onCView) return;         // (Hearth (C:)'s pages are always the mockup's Details list)
        view = v;
        rememberView();
        makeThumbs();
    }
    function cycleView() { setView(({ xlarge: "large", large: "grid", grid: "small", small: "tiles", tiles: "compact", compact: "details", details: "xlarge" })[view]); }
    readonly property var viewNames: ({ large: "Large icons", grid: "Medium icons", small: "Small icons", tiles: "Tiles", compact: "List", details: "Details" })
    function setShowExt(on) { showExt = on; rememberView(); say(on ? "Showing file name extensions" : "Hiding file name extensions"); }
    function toggleDetails() { detailsPane = !detailsPane; rememberView(); }
    // A file's name as shown: without its extension when View › File name extensions is off (never for folders
    // or names that are only an extension, like .bashrc).
    function shownName(e) {
        if (!e) return "";
        if (showExt || e.dir) return e.name;
        const i = e.name.lastIndexOf(".");
        return i > 0 ? e.name.slice(0, i) : e.name;
    }

    // The navigation row's and command bar's buttons (also reached with the controller: Up to the bar, A presses).
    // x, y: where a menu opens (a click); none: the menu opens in the middle (the controller).
    function command(t, x, y) {
        viaPad = x === undefined;
        if (t.startsWith("bc-")) return briefcase.command(t, x, y);
        switch (t) {
        case "refresh": reload(); refreshPlaces(); break;
        case "new":
            if (!writable) return say("You can't make anything here");
            menuOnFolder = true;
            showMenu("New", [
                { id: "newfolder", label: "Folder", key: "Ctrl+Shift+N" },
                { id: "newfile", label: "Empty file", key: "Ctrl+Alt+N" },
                { id: "bc-new", label: "Briefcase" }
            ].concat(templates.length ? [{ sep: true }].concat(templates.map((tp, i) => ({ id: "tpl:" + i, label: tp.name }))) : []), x, y);
            break;
        case "cut": if (!targets().length) return say("Pick something first"); setClip("cut", targets()); break;
        case "copy": if (!targets().length) return say("Pick something first"); setClip("copy", targets()); break;
        case "paste": if (!writable) return say(cRefusalHere() || "You can't paste here", !!cRefusalHere()); paste(); break;
        case "rename":
            if (current && current.cv && !onSpecial) return say("“" + current.name + "” is one of Hearth (C:)'s names: it can't be renamed");
            if (!current || onSpecial) return say("Pick something to rename"); menuOnFolder = false; runMenu("rename"); break;
        case "copypath": {
            menuOnFolder = !current || onThisPc;
            const cps = copyTargets(!current, targets());
            if (!cps.length) return say(onApps ? "Apps have no path to copy" : "Pick something first");
            const w = cps.map(p => copyForm(p, cCopyLinux)).join(", ");
            menuOnFolder = !current;
            showMenu("Copy path", copyItems(cps.length, cps.every(p => !DrivePaths.isCPath(p)), w), x, y);
            break;
        }
        case "delete": if (!targets().length) return say("Pick something first"); menuOnFolder = false; runMenu(inTrash ? "delete" : "trash"); break;
        case "sort":
            if (onApps) appsSortMenu(x, y);
            else if (onThisPc) say("This PC keeps Windows' order: your folders, then drives, then network locations");
            else if (onCView) say("Hearth (C:)'s folders keep Windows' order, by name");
            else sortMenu(x, y);
            break;
        case "view": viewMenu(x, y); break;
        case "hidden": {
            if (onApps) { appsFilterMenu(x, y); break; }
            const ck = on => on ? "✓  " : "     ";
            showMenu("Show", [
                { id: "hidden", label: ck(hidden) + "Hidden files", key: "Ctrl+H" },
                { id: "exts", label: ck(showExt) + "File name extensions" }
            ], x, y);
            break;
        }
        case "undo": undo(); break;
        case "redo": redo(); break;
        case "details": toggleDetails(); break;
        }
    }

    // ------------------------------------------------ undo / redo (Ctrl+Z, Ctrl+Y)
    // Each change Hearth Files makes is kept as what undoes it: a rename, something new, Move to Trash, a copy,
    // a move or links. Delete permanently can't be undone (it asks first). The last 50 are kept, for this session.
    function pushUndo(op) { undoStack = undoStack.concat([op]).slice(-50); redoStack = []; }
    function opName(op) {
        const n = op.items || op.paths || [op.path || op.to];
        const what = n.length === 1 ? "“" + base(n[0]) + "”" : n.length + " items";
        return ({ rename: "Rename", create: "New " + what, trash: "Trash " + what, copy: "Copy " + what, move: "Move " + what, link: "Link " + what })[op.kind] || "";
    }
    function undo() { runUndo(true); }
    function redo() { runUndo(false); }
    function runUndo(isUndo) {
        const stack = isUndo ? undoStack : redoStack;
        if (!stack.length) return say(isUndo ? "Nothing to undo" : "Nothing to redo");
        if (jobProc.running) return say("Wait for the copy or move to finish first");
        const op = stack[stack.length - 1];
        if (isUndo) undoStack = undoStack.slice(0, -1); else redoStack = redoStack.slice(0, -1);
        const done = (next, msg) => {
            if (isUndo) redoStack = redoStack.concat([next || op]); else undoStack = undoStack.concat([next || op]);
            say((isUndo ? "Undone: " : "Redone: ") + (msg || opName(op)));
            reload();
            refreshPlaces();
        };
        const failed = err => { say(err, true); reload(); };
        const k = op.kind;
        if (k === "rename") {
            const from = isUndo ? op.to : op.from, to = isUndo ? op.from : op.to;
            call(["rename", from, base(to)], (r, err) => err ? failed(err) : done(null, "Rename “" + base(op.from) + "”"));
        } else if (k === "trash") {
            call([isUndo ? "untrash" : "trash"].concat(op.paths), (r, err) => err ? failed(err) : done());
        } else if (k === "create" || k === "copy" || k === "link") {
            const list = op.items || [op.path];
            call([isUndo ? "trash" : "untrash"].concat(list), (r, err) => err ? failed(err) : done());
        } else if (k === "move") {
            if (isUndo) startJob({ verb: "move", paths: op.items, dest: op.back, decide: {}, undoOp: op, undoMode: "undo" });
            else startJob({ verb: "move", paths: op.srcs, dest: op.dest, decide: {}, undoOp: op, undoMode: "redo" });
        }
    }
    // What Copy path copies: what's picked, else the folder; on This PC the folder or drive under the cursor.
    function copyTargets(onFolder, t) {
        if (onApps) return [];
        if (onThisPc) return current && current.path.indexOf("/") === 0 ? [current.path] : [];
        if (onCView) return !onFolder && current ? [current.path] : (listing ? [listing.path] : []);
        if (!onFolder && current && current.cv && !markCount) return [current.path];
        if (t && t.length && !onFolder) return t;
        return listing && !onHome ? [listing.path] : [];
    }
    // Copy path (the form Settings › System › Drives & folders picked: C:\ or Linux) first, the other one after it.
    // linuxToo: there's a Linux path to copy (Hearth (C:)'s own folders have none). sub: what Copy path gives.
    function copyItems(n, linuxToo, sub) {
        const many = n > 1;
        const win = { id: "copywin", label: (cCopyLinux ? (many ? "Copy Windows paths" : "Copy Windows path") : (many ? "Copy paths" : "Copy path"))
                          + (!cCopyLinux && sub && sub.length <= 44 ? "  (" + sub + ")" : ""), key: cCopyLinux ? "" : "Ctrl+Shift+C", icon: "link" };
        const lin = { id: "copypath", label: (cCopyLinux ? (many ? "Copy paths" : "Copy path") : (many ? "Copy Linux paths" : "Copy Linux path"))
                          + (cCopyLinux && sub && sub.length <= 44 ? "  (" + sub + ")" : ""), key: "Ctrl+Alt+C", icon: "link" };
        if (!linuxToo) return [Object.assign({}, win, { label: "Copy path" + (sub && sub.length <= 44 ? "  (" + sub + ")" : "") })];
        return cCopyLinux ? [lin, win] : [win, lin];
    }
    function copyText(t) {
        Quickshell.execDetached(["wl-copy", "--", t]);
        say("Copied “" + t + "”");
    }

    // ================================================ search: the folders inside too
    property var foundBuf: []
    Process {
        id: searchProc
        stdout: SplitParser {
            onRead: line => {
                if (line.startsWith("found: ")) {
                    try { app.foundBuf.push(JSON.parse(line.slice(7))); } catch (e) {}
                } else if (line.startsWith("error: ")) {
                    app.say(line.slice(7), true);
                } else {
                    try {
                        const r = JSON.parse(line);
                        if (r.truncated) app.say("Showing the first " + r.count + " matches");
                    } catch (e) {}
                }
            }
        }
        onExited: { app.flushFound(); app.searching = false; }
    }
    Timer { interval: 200; repeat: true; running: app.searching; onTriggered: app.flushFound() }
    function flushFound() {
        if (!foundBuf.length || found === null) return;
        found = found.concat(foundBuf);
        foundBuf = [];
    }
    function runSearch(q) {
        q = (q || "").trim();
        if (!q || !listing || inTrash || onCView) return;      // (Hearth (C:)'s pages: typing filters them)
        stopSearch();
        foundBuf = [];
        found = [];
        foundQuery = q;
        query = "";
        marked = ({});
        cur = 0;
        searching = true;
        zone = "main";
        searchProc.command = ["python3", helper, "search", listing.path, q].concat(hidden ? ["--hidden"] : []);
        searchProc.running = true;
    }
    function stopSearch() {
        if (searchProc.running) searchProc.running = false;
        searching = false;
    }
    function clearSearch() {
        stopSearch();
        found = null;
        foundQuery = "";
        query = "";
        cur = 0;
    }

    // ================================================ the actions menu (X)
    function sep() { return { sep: true }; }
    function showMenu(title, items, x, y) {
        // No separator first, last or twice in a row.
        const out = [];
        for (const it of items) {
            if (it.sep && (!out.length || out[out.length - 1].sep)) continue;
            out.push(it);
        }
        while (out.length && out[out.length - 1].sep) out.pop();
        menuTitle = title;
        menuItems = out;
        menuIndex = Math.max(0, out.findIndex(i => !i.sep));
        menuX = x === undefined ? -1 : x;
        menuY = y === undefined ? -1 : y;
        overlay = "menu";
    }
    property bool menuOnFolder: false          // the menu is about the folder itself, not what's picked in it
    property bool menuFull: false              // Show more options: the whole menu this once (Windows 11's short one first)
    property real lastMenuX: -1
    property real lastMenuY: -1
    // Previous versions (1.12, mockups files-6 and -7): the snapshots of your files (Task Manager › Disk › Snapshots)
    // that hold this file or folder, newest first; Open one, or Restore it (what's there now goes to the Trash).
    property var versions: []
    property string versionsOf: ""
    Process {
        id: verProc
        property bool thenMenu: false
        stdout: StdioCollector {
            onStreamFinished: {
                app.versions = text.split("\n").filter(l => l.trim() !== "").sort().reverse();
                if (verProc.thenMenu) app.versionsMenu();
            }
        }
    }
    function lookForVersions(path, menu) {
        if (verProc.running) return;
        versionsOf = path;
        verProc.thenMenu = !!menu;
        verProc.command = ["sh", "-c", "r=$(realpath -- \"$1\" 2>/dev/null) || exit 0; for base in /var/home /home; do case \"$r\" in \"$base\"/*) rel=${r#\"$base\"/}; for d in \"$base\"/.hearth-snapshots/*/; do [ -e \"$d$rel\" ] && echo \"$d$rel\"; done; break;; esac; done; true", "sh", path];
        verProc.running = true;
    }
    function versionName(p) {
        const m = p.match(/\.hearth-snapshots\/(\d{4})-(\d\d)-(\d\d)_(\d\d)(\d\d)\//);
        if (!m) return p;
        const d = new Date(parseInt(m[1]), parseInt(m[2]) - 1, parseInt(m[3]), parseInt(m[4]), parseInt(m[5]));
        return fmtDate(d.getTime() / 1000);
    }
    function versionsMenu() {
        if (!versions.length) return say("No earlier versions of “" + base(versionsOf) + "” yet. Turn on Snapshots of your files in Task Manager › Disk › Health & settings.");
        showMenu("Previous versions of " + base(versionsOf), versions.slice(0, 30).map((v, i) => ({ id: "ver:" + i, label: versionName(v) + "  ›", icon: "clock" })), lastMenuX, lastMenuY);
    }
    function versionMenu(i) {
        const v = versions[i];
        if (!v) return;
        showMenu(base(versionsOf) + " · " + versionName(v), [
            { id: "veropen:" + i, label: "Open this version", icon: "open" },
            { id: "verrestore:" + i, label: "Restore it (yours now goes to the Trash)", icon: "undo" },
            { id: "vercopy:" + i, label: "Copy it next to yours", icon: "copy" }], lastMenuX, lastMenuY);
    }
    function versionAct(kind, i) {
        const v = versions[i], orig = versionsOf;
        if (!v) return;
        if (kind === "open") return navigate(parentOf(v), base(v));
        if (kind === "copy") {
            const to = parentOf(orig) + "/" + base(orig).replace(/(\.[^.\/]+)?$/, m => " (" + versionName(v).replace(/[\/:]/g, ".") + ")" + m);
            Quickshell.execDetached(["cp", "-a", "--", v, to]);
            say("Copied the " + versionName(v) + " version next to it");
            return Qt.callLater(reload);
        }
        Quickshell.execDetached(["sh", "-c", "gio trash -- \"$2\" 2>/dev/null || mv -- \"$2\" \"$2.before-restore\"; cp -a -- \"$1\" \"$2\"", "sh", v, orig]);
        say("Put back the " + versionName(v) + " version of “" + base(orig) + "” (yours is in the Trash)");
        Qt.callLater(reload);
    }
    function openMenu(forFolder, x, y) {
        if (onCView && !picking) return openCMenu(forFolder ? null : current, x, y);
        if (!forFolder && current && current.cv && !markCount && !picking) return openCMenu(current, x, y);
        if (onSpecial && !picking) return onThisPc ? openPcMenu(forFolder ? null : current, x, y) : openAppMenu(forFolder ? null : current, x, y);
        menuOnFolder = !!forFolder || !current;
        const e = forFolder ? null : current;
        const t = forFolder ? [] : targets();
        const n = t.length;
        const what = n === 1 ? "" : " (" + n + ")";
        const items = [];
        if (job) items.push({ id: "stop", label: "Stop " + jobVerbing(job.verb).toLowerCase(), danger: true }, sep());
        if (picking) {
            items.push({ id: "pick-accept", label: pickAcceptLabel, off: pick.kind === "open" && !pickTargets().length });
            if (pick.multiple && e) items.push({ id: "mark", label: marked[e.path] ? "Unselect" : "Select", key: "Ctrl+Space" });
            if ((pick.filters || []).length > 1) items.push({ id: "pick-filter", label: "Show: " + pick.filters[pickFilter].name + "  ›" });
            items.push({ id: "pick-cancel", label: "Cancel", key: "Esc" }, sep());
        }
        if (!picking && !inTrash && !menuFull && e && !briefcase.inBriefcase) {
            const q = [];
            if (writable) q.push({ id: "cut", label: "Cut" + what, key: "Ctrl+X", icon: "cut", quick: true });
            q.push({ id: "copy", label: "Copy" + what, key: "Ctrl+C", icon: "copy", quick: true });
            if (n === 1 && writable) q.push({ id: "rename", label: "Rename…", key: "F2", icon: "edit", quick: true });
            q.push({ id: "copywin", label: "Copy as path", icon: "share", quick: true });
            if (writable) q.push({ id: "trash", label: "Move to Trash" + what, short: "Trash", key: "Del", icon: "trash", quick: true });
            q.push({ id: "open", label: "Open", key: "Enter", icon: "open" });
            if (e.dir && n === 1) q.push({ id: "newtab", label: "Open in new tab", icon: "plus" });
            if (!e.dir) q.push({ id: "openwith", label: "Open with  ›", icon: "grid" });
            if (e.dir && !places.some(p => p.bookmark && p.path === e.path)) q.push({ id: "bookmark", label: "Pin to the left menu", key: "Ctrl+D", icon: "pin" });
            if (writable && !(n === 1 && isArchive(e))) q.push({ id: "compress", label: "Compress to ZIP", icon: "archive" });
            if (n === 1 && isArchive(e) && writable) q.push({ id: "extracthere", label: "Extract here", icon: "archive" });
            if (e.dir && n === 1 && !e.briefcase) q.push({ id: "lockbc", label: "Lock as a Briefcase", icon: "lock" });
            if (e.dir && n === 1) q.push({ id: "terminal", label: "Open in Terminal", key: "Shift+F4", icon: "kbd" });
            if (n === 1 && e.path.indexOf("/") === 0) q.push({ id: "prevver", label: "Restore previous versions", icon: "clock" });
            q.push({ id: "copywin", label: "Copy as path", key: "C:\\…", icon: "link" });
            q.push(sep(), { id: "props", label: "Properties", key: "Alt+Enter", icon: "info" }, { id: "more", label: "Show more options", icon: "menu" });
            lastMenuX = x === undefined ? -1 : x; lastMenuY = y === undefined ? -1 : y;
            return showMenu(n > 1 ? plural(n, "item") + " selected" : e.name, q, lastMenuX, lastMenuY);
        }
        menuFull = false;
        if (inTrash) {
            if (n) items.push({ id: "restore", label: "Put back" + what, short: "Put back", icon: "undo", quick: true });
            if (n) items.push({ id: "delete", label: "Delete permanently" + what, danger: true, key: "Del", icon: "trash", quick: true, short: "Delete" });
            items.push({ id: "empty", label: "Empty Trash", danger: true, icon: "trash" });
            if (e) items.push(sep(), { id: "props", label: "Properties", key: "Alt+Enter", icon: "info" });
        } else {
            if (e) items.push({ id: "open", label: "Open", key: "Enter", icon: "open", group: "Open" });
            if (e && !e.dir) items.push({ id: "openwith", label: "Open with…", icon: "grid", group: "Open" });
            if (e && !e.dir && n === 1) for (const it of fileOpenMenuItems(e)) items.push(Object.assign({ group: "Open" }, it));
            if (e && e.dir && n === 1) items.push({ id: "newtab", label: "Open in new tab", icon: "plus", group: "Open" });
            if (n === 1 && isAppImage(e)) items.push({ id: "airun", label: "Run" }, e.inMenu ? { id: "aimenurm", label: "Remove from app menu" } : { id: "aimenu", label: "Add to app menu" });
            if (n === 1 && isDeb(e)) items.push({ id: "debinstall", label: "Install" });
            if (n === 1 && isWinApp(e)) items.push({ id: "winapp", label: "Open with Hearth (Windows app)" });
            if (e && (found !== null || onHome)) items.push({ id: "showin", label: "Show in its folder", icon: "folder", group: "Open" });
            if (e && n === 1 && kindOf(e) !== "folder") items.push({ id: "preview", label: "Preview", key: "Space", icon: "eye", group: "Open" });
            for (const it of briefcase.menuItems(e, n, t)) items.push(it);
            items.push(sep());
            if (n) items.push({ id: "cut", label: "Cut" + what, key: "Ctrl+X", off: !(n && writable), icon: "cut", quick: true });
            if (n) items.push({ id: "copy", label: "Copy" + what, key: "Ctrl+C", icon: "copy", quick: true });
            if (writable) items.push({ id: "paste", label: (clip.mode === "cut" ? "Move here" : "Paste") + (clip.paths.length > 1 ? " (" + clip.paths.length + ")" : ""), key: "Ctrl+V", icon: "clipboard", quick: !n });
            // Copy path: the Windows way, with drive letters (D:\Games); Copy Linux path: the real one.
            for (const it of copyItems(n, true, "")) items.push(it);
            items.push(sep());
            if (n === 1 && writable) items.push({ id: "rename", label: "Rename…", key: "F2", icon: "edit", quick: true });
            if (n > 1 && writable) items.push({ id: "batchrename", label: "Rename " + n + " items…", key: "F2", icon: "edit", quick: true });
            if (n && writable && !(n === 1 && isArchive(e))) items.push({ id: "compress", label: "Compress…", icon: "archive" });
            if (n === 1 && isArchive(e) && writable) items.push({ id: "extracthere", label: "Extract here", icon: "archive" });
            if (n === 1 && isArchive(e)) items.push({ id: "extractto", label: "Extract to…", icon: "archive" });
            if (n > 1 && writable && t.every(p => isArchive(entries.find(x => x.path === p)))) items.push({ id: "extracteach", label: "Extract " + n + " archives here" });
            if (n === 1 && isArchive(e) && writable) items.push({ id: "compress", label: "Compress…", icon: "archive" });
            if (e && kindOf(e) === "image") items.push({ id: "wallpaper", label: "Set as wallpaper", icon: "image" });
            if (!e && writable) {
                items.push({ id: "newfolder", label: "New folder…", key: "Ctrl+Shift+N", icon: "folder", quick: true });
                items.push({ id: "newfile", label: "New empty file…", icon: "file", group: "New" });
                if (templates.length) items.push({ id: "sub-new", label: "New document  ›", icon: "doc", group: "New" });
            }
            items.push(sep());
            if (n && writable) items.push({ id: "trash", label: "Move to Trash" + what, short: "Trash", key: "Del", icon: "trash", quick: true });
            if (n && writable) items.push({ id: "delete", label: "Delete permanently" + what, danger: true, key: "Shift+Del", icon: "close" });
            items.push(sep());
            if (e) items.push({ id: "mark", label: marked[e.path] ? "Unselect" : "Select", key: "Ctrl+Space", group: "Select" });
            if (markCount) items.push({ id: "unmarkall", label: "Clear selection", key: "Esc", group: "Select" });
            if (entries.length) items.push({ id: "markall", label: "Select all", key: "Ctrl+A", group: "Select" });
            if (markCount) items.push({ id: "invert", label: "Invert selection", key: "Ctrl+I", group: "Select" });
            items.push(sep());
            if (!e) items.push({ id: "sub-view", label: "View  ›", icon: "grid", group: "View" }, { id: "sub-sort", label: "Sort and group  ›", icon: "menu", group: "View" });
            if (!e) items.push({ id: "hidden", label: hidden ? "Hide hidden files" : "Show hidden files", key: "Ctrl+H", icon: "eye", group: "View" });
            if (!e) items.push({ id: "newtab", label: "New tab", key: "Ctrl+T", icon: "plus", group: "View" });
            if (tabs.length > 1 && !e) items.push({ id: "closetab", label: "Close tab", key: "Ctrl+W", icon: "close", group: "View" });
            const bmPath = e ? (e.dir ? e.path : "") : (listing && !onHome ? listing.path : "");
            if (bmPath && !places.some(p => p.bookmark && p.path === bmPath)) items.push({ id: "bookmark", label: "Pin to the left menu", key: "Ctrl+D", icon: "pin" });
            if ((!e && !onHome) || (e && e.dir)) items.push({ id: "terminal", label: "Open in Terminal", key: "Shift+F4", icon: "kbd", group: "Open" });
            if (e || !onHome) items.push(sep(), { id: "props", label: e ? "Properties" : "Folder properties", key: "Alt+Enter", icon: "info" });
        }
        const title = e ? (n > 1 ? plural(n, "item") + " selected" : e.name) : (listing ? (listing.name || "Folder") : "Folder");
        showMenu(title, items.filter(i => !i.off), x === undefined ? -1 : x, y === undefined ? -1 : y);
    }
    // ================================================ This PC and Installed apps: their menus
    readonly property string shellDir: helper.slice(0, helper.lastIndexOf("/"))
    function callCmd(cmd, cb) {
        const p = callComp.createObject(app, { command: cmd, cb: cb || null });
        p.running = true;
    }
    function openSettingsPage(page) { Quickshell.execDetached(["quickshell", "ipc", "-p", shellDir, "call", "settings", "page", page]); }
    function openPcMenu(e, x, y) {
        const ck = on => on ? "✓  " : "     ";
        menuOnFolder = !e;
        if (!e) return showMenu("This PC", [
            { id: "tp-view:grid", label: ck(pcView === "grid") + "Tiles" }, { id: "tp-view:details", label: ck(pcView === "details") + "Details" },
            sep(), { id: "tp-apps", label: "Installed apps" }, { id: "tp-refresh", label: "Refresh", key: "F5" }
        ], x, y);
        const d = e.item || {};
        const real = e.path.indexOf("/") === 0;
        const items = [{ id: "open", label: d.mounted === false && e.pc === "drive" ? "Mount and open" : "Open", key: "Enter", icon: "open" }];
        if (real) items.push({ id: "tp-newtab", label: "Open in new tab", icon: "plus" });
        if (e.pc === "drive" && d.removable && d.mounted !== false) items.push(sep(), { id: "tp-unmount", label: "Unmount" }, { id: "tp-eject", label: "Safely remove" });
        if (e.pc === "net" && d.net) items.push(sep(), { id: "tp-auto", label: ck(d.sub === "Connects by itself") + "Connect automatically" });
        if (e.pc !== "folder" && d.key) items.push(sep(), { id: "tp-letter", label: "Change drive letter…", icon: "edit" });
        if (real) items.push(sep(), { id: "copywin", label: "Copy path", icon: "link" }, { id: "copypath", label: "Copy Linux path", key: "Ctrl+Alt+C", icon: "link" });
        if (real) items.push(sep(), { id: "tp-props", label: "Properties", key: "Alt+Enter", icon: "info" });
        showMenu(e.name, items, x, y);
    }
    // Hearth (C:)'s view (the owner's mockup): Open, Open in new tab, Go to the real folder, Copy path (C:\ form), Copy
    // Linux path, then Modify / Uninstall for apps (Installed apps' own actions) and Properties.
    function cRealOf(e) {
        if (!e) return "";
        if (e.path && e.path.indexOf("/") === 0) return e.path;
        const r = DrivePaths.cResolve(e.path, pathCtx);
        if (r.path && r.path.indexOf("/") === 0) return r.path;
        const first = ((e.cv && e.cv.real) || [])[0] || "";
        const p = first.replace(/^[^:\/~]*:\s*/, "").split(",")[0].trim();
        return p.startsWith("~") ? home + p.slice(1) : p.indexOf("/") === 0 ? p : "";
    }
    function openCMenu(e, x, y) {
        const ck = on => on ? "✓  " : "     ";
        menuOnFolder = !e;
        if (!e) {
            const its = [];
            if (cPageNow === "root") its.push({ id: "c-linux", label: ck(cLinux) + "Show the Linux folders too" });
            for (const it of copyItems(1, false, winPath(path))) its.push(it);
            its.push(sep(), { id: "tp-refresh", label: "Refresh", key: "F5" }, sep(), { id: "c-props", label: "Properties", key: "Alt+Enter", icon: "info" });
            return showMenu(listing ? listing.name : "Hearth (C:)", its, x, y);
        }
        const cv = e.cv || {};
        const real = cRealOf(e);
        const items = [{ id: "open", label: "Open", key: "Enter", icon: "open" }, { id: "c-newtab", label: "Open in new tab", icon: "plus" }, sep()];
        if (real) items.push({ id: "c-real", label: "Go to the real folder", sub: real.indexOf(home + "/") === 0 ? "~" + real.slice(home.length) : real, icon: "folder" });
        for (const it of copyItems(1, e.path.indexOf("/") === 0, cv.win || winPath(e.path))) items.push(Object.assign({}, it, { sub: it.id === "copywin" ? (cv.win || winPath(e.path)) : e.path }));
        const a = cv.app;
        if (a) {
            items.push(sep());
            if (appModifiable(a)) items.push({ id: "c-modify", label: "Modify", icon: "edit" });
            if (appUninstallable(a)) items.push({ id: "c-uninstall", label: "Uninstall", danger: true, icon: "trash" });
        } else if (cv.kind === "hearth") {
            items.push(sep(), { id: "c-part", label: "Part of Hearth", icon: "info" });
        }
        items.push(sep(), { id: "c-props", label: "Properties", key: "Alt+Enter", icon: "info" });
        // A sub line repeats the label's path: the label keeps just the words.
        for (let i = 0; i < items.length; i++) if (items[i].sub && /^Copy/.test(items[i].label || "")) items[i] = Object.assign({}, items[i], { label: items[i].label.replace(/\s\s\(.*\)$/, "") });
        showMenu(e.name, items, x, y);
    }
    function runCMenu(id, e) {
        switch (id) {
        case "c-newtab": if (e) newTab(e.path); break;
        case "c-real": { const r = cRealOf(e); if (r) navigate(r, ""); break; }
        case "c-linux": setFilesPref("linuxFolders", !cLinux); break;
        case "c-modify": if (e && e.cv && e.cv.app) appModify(e.cv.app); break;
        case "c-uninstall": if (e && e.cv && e.cv.app) appUninstall(e.cv.app); break;
        case "c-part": say("Hearth's own apps are part of Hearth OS: look only, and they can't be uninstalled"); break;
        case "c-props": showCProps(e); break;
        }
    }
    // ---- Properties of the view's folders (FilesCProps.qml): how big, how many apps, which real places they come from.
    property var cProps: null
    function showCProps(e) {
        // A real folder (an app's, Users, Windows…): the usual Properties window.
        if (e && e.path && e.path.indexOf("/") === 0) return properties([e.path]);
        const p = e ? e.path : (listing ? listing.path : cRoot);
        const pg = DrivePaths.cPage(p, pathCtx);
        if (!pg) { const r = cRealOf(e); if (r) properties([r]); return; }
        const list = DrivePaths.cEntries(pg, pathCtx, cViewData);
        const places = [];
        const T = q => q.indexOf(home + "/") === 0 ? "~" + q.slice(home.length) : q;
        const sumKind = k => list.filter(x => x.cv.kind === k).reduce((n, x) => n + (x.cv.size || 0), 0);
        const dirsOf = k => { const d = []; for (const x of list) if (x.cv.kind === k) for (const r of x.cv.real || []) if (d.indexOf(r) < 0) d.push(r); return d; };
        if (pg === "pf") {
            if (list.some(x => x.cv.kind === "hearth")) places.push({ name: "Hearth’s own apps (look only)", path: T(cdata.hearth || ""), size: 0, color: "#66c0f4" });
            if (list.some(x => x.cv.kind === "flatpak")) places.push({ name: "Flatpak apps", path: (cdata.flatpak || ["/var/lib/flatpak/app"]).map(T).join(", "), size: sumKind("flatpak"), color: "#9a7cff" });
            if (list.some(x => x.cv.kind === "games")) places.push({ name: "Hearth Games", path: dirsOf("games").join(", "), size: sumKind("games"), color: "#f2c14e" });
            if (list.some(x => x.cv.kind === "recomp")) {
                const rd = [];
                for (const x of list) if (x.cv.kind === "recomp") { const d = parentOf(x.path); if (rd.indexOf(T(d)) < 0) rd.push(T(d)); }
                places.push({ name: "Recomps", path: rd.join(", "), size: sumKind("recomp"), color: "#7ccf8a" });
            }
        } else if (pg === "x86") {
            if (list.some(x => x.cv.kind === "winapp")) {
                const wd = [];
                for (const x of list) if (x.cv.kind === "winapp") { const d = parentOf(x.path); if (wd.indexOf(T(d)) < 0) wd.push(T(d)); }
                places.push({ name: "Windows apps", path: wd.join(", "), size: sumKind("winapp"), color: "#66c0f4" });
            }
            if (list.some(x => x.cv.kind === "steam")) places.push({ name: "Steam", path: T(cdata.steam || ""), size: sumKind("steam"), color: "#7ccf8a" });
        } else {
            for (const x of list) if (!x.cv.linuxFolder) places.push({ name: x.name, path: (x.cv.real || []).join(", "), size: x.cv.size || 0, color: x.cv.look ? "#9fb2c3" : "#f2c14e" });
        }
        const apps = pg === "pf" || pg === "x86";
        const n = list.filter(x => !x.cv.linuxFolder).length;
        const size = places.reduce((t, q) => t + (q.size || 0), 0);
        cProps = { name: pg === "root" ? titleOf(cRoot) : (listing && listing.path === p ? listing.name : base(p)), page: pg,
                   location: pg === "root" ? "This PC" : pg === "appdata" ? "C:\\Users\\" + pathCtx.user : "C:\\",
                   size: size, contains: apps ? plural(n, pg === "pf" ? "app and game" : "app", pg === "pf" ? "apps and games" : "apps") : plural(n, "folder"),
                   places: places,
                   note: apps ? "You can’t put files straight into this folder. To add an app, install it, and it shows up here by itself."
                              : "You can’t put files straight into this folder. Open one of the folders in it first." };
        overlay = "cprops";
    }
    // The view's switches, from its menus and View (the same files.json keys as Settings › System › Drives & folders):
    // saved only when you change them, one key at a time (hearth-files prefs-set merges it under the lock).
    function setFilesPref(k, v) {
        const now = ({ cdrive: prefs.cdrive !== false, linuxFolders: cLinux, addressBar: cAddrLinux ? "linux" : "windows", copyPath: cCopyLinux ? "linux" : "windows", navPane: navShown })[k];
        if (now === v) return;                 // already so: nothing is written
        const o = {};
        o[k] = v;
        prefs = Object.assign({}, prefs, o);
        call(["prefs-set", JSON.stringify(o)], (r, err) => { if (err) say(err, true); });
        places = buildNav(rawPlaces);
        if (k === "cdrive" && !v && onCView) navigate(thisPcPage, "");
        say(({ cdrive: v ? "Hearth (C:) shows Windows folder names" : "Hearth (C:) shows its real folders",
               linuxFolders: v ? "Showing the Linux folders on Hearth (C:) too" : "The Linux folders are hidden on Hearth (C:)",
               addressBar: v === "linux" ? "The address bar shows Linux paths" : "The address bar shows Windows names",
               copyPath: v === "linux" ? "Copy path gives the Linux path" : "Copy path gives the C:\\ form",
               navPane: v ? "Navigation pane shown" : "Navigation pane hidden" })[k] || "Saved");
        if (k === "cdrive" && v && !Object.keys(cdata).length) loadCData(null);
    }
    // Change drive letter… (like Windows' Disk Management): the free letters, D: to Y:. C: is always Hearth.
    property var letterFor: null
    function letterMenu(d) {
        if (!d || !d.key) return;
        if (d.system || d.key === "system") return say("C: is always Hearth, the drive Hearth runs from");
        const taken = {};
        for (const k in (pc.taken || {})) taken[k] = pc.taken[k];
        for (const q of (pc.drives || []).concat(pc.network || [])) if (q.letter && q.key !== d.key) taken[q.letter] = q.name;
        const free = "DEFGHIJKLMNOPQRSTUVWXY".split("").filter(l => l === d.letter || !taken[l]);
        letterFor = d;
        const at = Math.max(0, free.indexOf(d.letter));
        const shown = free.slice(Math.max(0, at - 5), Math.max(0, at - 5) + 12);
        showMenu("Drive letter for " + d.name, shown.map(l => ({ id: "letter:" + l, label: (l === d.letter ? "✓  " : "     ") + l + ":" })), -1, -1);
    }
    function openAppMenu(e, x, y) {
        menuOnFolder = !e;
        if (!e) return showMenu("Installed apps", [
            { id: "ia-sortmenu", label: "Sort by  ›" }, { id: "ia-filtermenu", label: "Filter by  ›" }, sep(), { id: "tp-refresh", label: "Refresh", key: "F5" }
        ], x, y);
        const a = e.app;
        const game = ["steam", "recomp", "games"].indexOf(a.kind) >= 0;
        const items = [{ id: "ia-open", label: game ? "Play" : "Open", icon: "open" }];
        if (appModifiable(a)) items.push({ id: "ia-modify", label: "Modify", icon: "edit" });
        if (a.canMove) items.push({ id: "ia-move", label: "Move to another drive", icon: "folder" });
        if (a.part) items.push(sep(), { id: "ia-part", label: "Part of Hearth", icon: "info" });
        else if (appUninstallable(a)) items.push(sep(), { id: "ia-uninstall", label: "Uninstall", danger: true, icon: "trash" });
        showMenu(a.name, items, x, y);
    }
    function appsSortMenu(x, y) {
        const ck = on => on ? "✓  " : "     ";
        showMenu("Sort by", [{ id: "ia-sort:name", label: ck(appsSort === "name") + "Name" }, { id: "ia-sort:size", label: ck(appsSort === "size") + "Size" },
                             { id: "ia-sort:date", label: ck(appsSort === "date") + "Install date" }], x, y);
    }
    function appsFilterMenu(x, y) {
        showMenu("Filter by", appKinds.map(k => ({ id: "ia-filter:" + k, label: (appsFilter === k ? "✓  " : "     ") + appKindName(k) })), x, y);
    }
    function appModifiable(a) {
        return a.kind === "winapp" || (a.kind === "steam" && !!a.steamId) || (a.kind === "flatpak" && !!a.flatseal);
    }
    function appUninstallable(a) {
        if (a.part) return false;
        if (a.kind === "flatpak" || a.kind === "winapp") return true;
        if (a.kind === "steam") return !!a.steamId;
        return !!a.uninstall;
    }
    function appOpen(a) {
        if (!a) return;
        say("Starting “" + a.name + "”…");
        if (a.kind === "winapp") return Quickshell.execDetached(["hearth-winapps", "run", a.appId]);
        if (["steam", "recomp", "games"].indexOf(a.kind) >= 0) return Quickshell.execDetached(["python3", shellDir + "/hearth-games", "play", a.id]);
        if (a.kind === "flatpak" && !a.desktop) return Quickshell.execDetached(["flatpak", "run", a.appId]);
        if (a.desktop) call(["desktop-launch", a.desktop], (r, err) => { if (err) say(err, true); });
    }
    function appModify(a) {
        if (a.kind === "winapp") { openSettingsPage("winapps"); return say("Settings › Windows apps has “" + a.name + "”'s Proton, Windows version and more"); }
        if (a.kind === "steam") { Quickshell.execDetached(["steam", "steam://gameproperties/" + a.steamId]); return say("Opening “" + a.name + "”'s properties in Steam…"); }
        if (a.kind === "flatpak") { Quickshell.execDetached(["flatpak", "run", "com.github.tchx84.Flatseal"]); return say("Flatseal opens: pick “" + a.name + "” there to change what it may use"); }
    }
    function appMove(a) {
        Quickshell.execDetached(["nimbus", "--windowed", "--page", "diskspace"]);
        say("Storage opens in the Hearth app: pick “" + a.name + "” and the drive to move it to");
    }
    function appUninstall(a) {
        if (a.kind === "steam") {
            Quickshell.execDetached(["steam", "steam://uninstall/" + a.steamId]);
            return say("Steam asks to uninstall “" + a.name + "”");
        }
        if (a.kind !== "flatpak" && a.kind !== "winapp") {
            Quickshell.execDetached(["nimbus", "--windowed", "--page", "diskspace"]);
            return say("Storage opens in the Hearth app: uninstall “" + a.name + "” there");
        }
        const what = a.kind === "flatpak" ? "The app is removed (your settings and files in it stay, as Flatpak keeps them)."
                                          : "Its space (everything installed in it), menu entry and icon are deleted. Files you saved in your own folders stay.";
        confirm("Uninstall “" + a.name + "”?", what, "Uninstall", () => {
            app.say("Uninstalling “" + a.name + "”…");
            const cmd = a.kind === "flatpak" ? ["python3", app.shellDir + "/hearth-flatpak", "remove", a.appId]
                                             : ["python3", app.shellDir + "/hearth-winapps", "remove", a.appId];
            app.callCmd(cmd, (r, err) => {
                if (err) return app.say(err, true);
                app.say("“" + a.name + "” is uninstalled");
                if (app.onApps) app.loadApps();
            });
        });
    }
    function runPageMenu(id, e) {
        const d = e && e.item ? e.item : {};
        if (id.startsWith("tp-view:")) return setPcView(id.slice(8));
        if (id.startsWith("tp-start:")) return setStartPage(id.slice(9));
        if (id.startsWith("ia-sort:")) { appsSort = id.slice(8); cur = 0; return; }
        if (id.startsWith("ia-filter:")) { appsFilter = id.slice(10); cur = 0; return; }
        if (id.startsWith("letter:")) {
            const L = id.slice(7), dd = letterFor;
            letterFor = null;
            if (!dd || L === dd.letter) return;
            call(["letter-set", dd.key, L], (r, err) => {
                if (err) return say(err, true);
                say("“" + dd.name + "” is " + L + ": now (Windows apps see it there from their next start)");
                refreshPlaces();
                loadThisPc();
            });
            return;
        }
        switch (id) {
        case "tp-refresh": reload(); refreshPlaces(); break;
        case "tp-apps": navigate(appsPage, ""); break;
        case "tp-newtab": if (e && e.path.indexOf("/") === 0) newTab(e.path); break;
        case "tp-props": if (e && e.path.indexOf("/") === 0) properties([e.path]); break;
        case "tp-letter": letterMenu(d); break;
        case "tp-unmount": case "tp-eject": {
            if (!d.path) break;
            const ej = id === "tp-eject";
            say((ej ? "Removing" : "Unmounting") + " “" + d.label + "”…");
            call([ej ? "eject" : "unmount", d.path], (r, err) => {
                if (err) return say(err, true);
                say(ej ? "You can take out “" + d.label + "” now" : "“" + d.label + "” is unmounted");
                refreshPlaces();
                loadThisPc();
            });
            break;
        }
        case "tp-auto": {
            if (!d.net) break;
            const on = d.sub !== "Connects by itself";
            call(["net-auto", JSON.stringify(d.net), on ? "on" : "off"], (r, err) => {
                if (err) return say(err, true);
                say(on ? "“" + d.name + "” connects by itself after you sign in" : "“" + d.name + "” no longer connects by itself");
                if (on) Quickshell.execDetached(["python3", helper, "net-autoconnect"]);
                loadThisPc();
            });
            break;
        }
        case "ia-sortmenu": appsSortMenu(-1, -1); break;
        case "ia-filtermenu": appsFilterMenu(-1, -1); break;
        case "ia-open": if (e && e.app) appOpen(e.app); break;
        case "ia-modify": if (e && e.app) appModify(e.app); break;
        case "ia-move": if (e && e.app) appMove(e.app); break;
        case "ia-uninstall": if (e && e.app) appUninstall(e.app); break;
        case "ia-part": say("“" + (e && e.app ? e.app.name : "It") + "” is part of Hearth OS, so it can't be uninstalled"); break;
        }
    }
    // This PC's Tiles / Details and where Files opens: saved only when you pick them (merged into files.json).
    function setPcView(v) {
        if (v !== "grid" && v !== "details") return;
        pcView = v;
        prefs = Object.assign({}, prefs, { thisPcView: v });
        call(["prefs-set", JSON.stringify({ thisPcView: v })]);
    }
    function setStartPage(k) {
        if (k !== "home" && k !== "thispc") return;
        prefs = Object.assign({}, prefs, { startPage: k });
        call(["prefs-set", JSON.stringify({ startPage: k })], (r, err) => {
            if (err) return say(err, true);
            say(k === "thispc" ? "Files opens to This PC from now on" : "Files opens to Home from now on");
        });
    }
    function openPlaceMenu() {
        const p = places[placeIndex];
        if (!p) return;
        menuOnFolder = false;
        const items = [];
        if (p.path) items.push({ id: "p-open", label: "Open" }, { id: "p-newtab", label: "Open in new tab" });
        if (!p.path && p.device) items.push({ id: "p-mount", label: "Mount" });
        if (p.section === "drives" && p.path && p.removable) items.push({ id: "p-unmount", label: "Unmount" }, { id: "p-eject", label: "Safely remove" });
        if (p.section === "drives" && p.key) items.push({ id: "p-letter", label: "Change drive letter…" });
        if (p.bookmark) items.push({ id: "p-unbookmark", label: "Remove bookmark", danger: true });
        // A device's shared folder: open it now, or have it connect by itself after signing in (off until turned on).
        if (p.net && p.net.share) items.push({ id: "p-open", label: "Open" }, { id: "p-auto", label: (p.auto ? "✓  " : "     ") + "Connect automatically" });
        if (p.kind === "trash") items.push({ id: "empty", label: "Empty Trash", danger: true });
        if (p.path && p.path !== thisPcPage && p.path !== appsPage) items.push(sep(), { id: "p-props", label: "Properties" });
        showMenu(p.label, items, -1, -1);
    }
    function sortMenu(mx, my) {
        const ck = on => on ? "✓  " : "     ";
        showMenu("Sort and group", [
            { id: "sort:name", label: ck(sort === "name") + "Name" },
            { id: "sort:date", label: ck(sort === "date") + "Modified" },
            { id: "sort:size", label: ck(sort === "size") + "Size" },
            { id: "sort:type", label: ck(sort === "type") + "Type" },
            sep(),
            { id: "desc", label: desc ? "Ascending" : "Descending" },
            sep(),
            { id: "group:none", label: ck(group === "none") + "Don't group" },
            { id: "group:type", label: ck(group === "type") + "Group by type" },
            { id: "group:date", label: ck(group === "date") + "Group by date modified" }
        ], mx, my);
    }
    // Hearth (C:)'s switches, the same as Settings › System › Drives & folders (saved only when you pick one).
    function cViewItems() {
        if (picking) return [];
        const ck = on => on ? "✓  " : "     ";
        const out = [sep(), { id: "cset:cdrive=" + !cdriveOn, label: ck(cdriveOn) + "Show Hearth (C:) with Windows folder names" }];
        if (cdriveOn) out.push({ id: "cset:linuxFolders=" + !cLinux, label: ck(cLinux) + "Show the Linux folders too" },
                               { id: "cset:addressBar=windows", label: ck(!cAddrLinux) + "Address bar: Windows names" },
                               { id: "cset:addressBar=linux", label: ck(cAddrLinux) + "Address bar: Linux paths" },
                               { id: "cset:copyPath=windows", label: ck(!cCopyLinux) + "Copy path: C:\\ form" },
                               { id: "cset:copyPath=linux", label: ck(cCopyLinux) + "Copy path: Linux path" });
        return out;
    }
    function viewMenu(mx, my) {
        const ck = on => on ? "✓  " : "     ";
        // Where Files opens, like File Explorer's "Open File Explorer to" (kept only when you pick one).
        const start = picking ? [] : [sep(), { id: "tp-start:home", label: ck(startPage === homePage) + "Open Files to: Home" },
                                      { id: "tp-start:thispc", label: ck(startPage === thisPcPage) + "Open Files to: This PC" }].concat(cViewItems());
        if (onCView) return showMenu("View", [{ id: "detailspane", label: ck(detailsPane) + "Details pane", key: "Alt+P" }].concat(start), mx, my);
        if (onThisPc) return showMenu("View", [
            { id: "tp-view:grid", label: ck(pcView === "grid") + "Tiles", key: "Ctrl+1" },
            { id: "tp-view:details", label: ck(pcView === "details") + "Details", key: "Ctrl+3" },
            sep(), { id: "detailspane", label: ck(detailsPane) + "Details pane", key: "Alt+P" }
        ].concat(start), mx, my);
        if (onApps) return showMenu("View", [{ id: "detailspane", label: ck(detailsPane) + "Details pane", key: "Alt+P" }].concat(start), mx, my);
        showMenu("View", [
            { id: "view:xlarge", label: ck(view === "xlarge") + "Extra large icons", key: "Ctrl+7" },
            { id: "view:large", label: ck(view === "large") + "Large icons", key: "Ctrl+4" },
            { id: "view:grid", label: ck(view === "grid") + "Medium icons", key: "Ctrl+1" },
            { id: "view:small", label: ck(view === "small") + "Small icons", key: "Ctrl+5" },
            { id: "view:tiles", label: ck(view === "tiles") + "Tiles", key: "Ctrl+6" },
            { id: "view:compact", label: ck(view === "compact") + "List", key: "Ctrl+2" },
            { id: "view:details", label: ck(view === "details") + "Details", key: "Ctrl+3" },
            sep(),
            { id: "navpane", label: ck(navShown) + "Navigation pane" },
            { id: "detailspane", label: ck(detailsPane) + "Details pane", key: "Alt+P" },
            { id: "exts", label: ck(showExt) + "File name extensions" },
            { id: "hidden", label: ck(hidden) + "Hidden files", key: "Ctrl+H" }
        ].concat(start), mx, my);
    }
    function runMenu(id) {
        const it = menuItems.find(i => i.id === id);
        if (it && it.sep) return;
        overlay = "";
        const onFolder = menuOnFolder;
        menuOnFolder = false;
        const t = onFolder ? [] : targets();
        const e = onFolder ? null : current;
        if (id.startsWith("bc-")) return briefcase.run(id, t, e);
        if (id.startsWith("tp-") || id.startsWith("ia-") || id.startsWith("letter:")) return runPageMenu(id, e);
        if (id.startsWith("c-")) return runCMenu(id, e);
        if (id.startsWith("cset:")) { const kv = id.slice(5).split("="); return setFilesPref(kv[0], kv[1] === "true" ? true : kv[1] === "false" ? false : kv[1]); }
        if (id.startsWith("sort:")) return setSort(id.slice(5));
        if (id.startsWith("group:")) return setGroup(id.slice(6));
        if (id.startsWith("view:")) return setView(id.slice(5));
        if (id.startsWith("tpl:")) return newFromTemplate(templates[parseInt(id.slice(4), 10)]);
        if (id.startsWith("fo-")) return fileOpenChoice(id.slice(3), e);
        if (id.startsWith("pf:")) { pickFilter = parseInt(id.slice(3), 10); return; }
        if (id.startsWith("pc:")) {
            const ij = id.slice(3).split(":").map(x => parseInt(x, 10));
            const c = (pick.choices || [])[ij[0]];
            if (c && c.options[ij[1]]) setChoice(c.id, c.options[ij[1]][0]);
            return;
        }
        if (id === "more") { menuFull = true; menuOnFolder = onFolder; return openMenu(onFolder, lastMenuX, lastMenuY); }
        if (id === "lockbc") return e ? briefcase.lockAsBriefcase(e.path) : undefined;
        if (id === "prevver") return e ? lookForVersions(e.path, true) : undefined;
        if (id.startsWith("ver:")) return versionMenu(parseInt(id.slice(4), 10));
        if (id.startsWith("veropen:")) return versionAct("open", parseInt(id.slice(8), 10));
        if (id.startsWith("verrestore:")) return versionAct("restore", parseInt(id.slice(11), 10));
        if (id.startsWith("vercopy:")) return versionAct("copy", parseInt(id.slice(8), 10));
        if (id === "pick-accept") return pickAccept(null);
        if (id === "pick-cancel") return close();
        if (id === "pick-filter") return filterMenu();
        switch (id) {
        case "stop": cancelJob(); break;
        case "open": activate(e); break;
        case "airun": if (e) runAppImage(e); break;
        case "winapp": if (e) openWinApp(e); break;
        case "aimenu": if (e) addAppImage(e); break;
        case "aimenurm":
            if (e) call(["appimage-menu-remove", e.path], (r, err) => { if (err) return say(err, true); say("“" + e.name + "” is out of the app menu (the AppImage is still here)"); reload(); });
            break;
        case "debinstall": if (e) installDeb(e); break;
        case "openwith": showApps(e, false); break;
        case "newtab": newTab(e && e.dir ? e.path : (listing ? listing.path : home)); break;
        case "closetab": closeTab(tabIndex); break;
        case "showin": if (e) { const dir = parentOf(e.path); clearSearch(); navigate(dir, e.name); } break;
        case "preview": showPreview(e); break;
        case "mark": toggleMark(e); break;
        case "markall": selectAll(); break;
        case "unmarkall": marked = ({}); break;
        case "invert": invertSelection(); break;
        case "copy": setClip("copy", t); break;
        case "cut": setClip("cut", t); break;
        case "paste": paste(); break;
        case "copypath": { const cps = copyTargets(onFolder, t); if (cps.length) copyText(cps.map(p => copyForm(p, true)).join("\n")); break; }
        case "copywin": { const cps = copyTargets(onFolder, t); if (cps.length) copyText(cps.map(p => copyForm(p, false)).join("\n")); break; }
        case "rename":
            // With a mouse or keyboard the name turns into a text box where it is (Enter keeps it, Esc cancels);
            // from the controller it asks in a box with the on-screen keyboard.
            if (e && !viaPad && overlay === "" && entries.some(x => x.path === e.path)) { renaming = e.path; break; }
            if (e) askText("Rename", e.name, "Rename", e.dir ? e.name.length : (e.name.lastIndexOf(".") > 0 ? e.name.lastIndexOf(".") : e.name.length),
                text => app.call(["rename", e.path, text], (r, err) => {
                    if (err) return app.say(err, true);
                    if (r.path !== e.path) app.pushUndo({ kind: "rename", from: e.path, to: r.path });
                    app.load(app.listing.path, app.base(r.path), "none");
                }));
            break;
        case "batchrename":
            askText("Rename " + t.length + " items (# becomes 1, 2, 3…)", "Name #", "Rename all", 4, text => {
                app.call(["batch-rename", text].concat(t), (r, err) => {
                    if (err) return app.say(err, true);
                    app.marked = ({});
                    app.say("Renamed " + t.length + " items");
                    app.load(app.listing.path, r.paths.length ? app.base(r.paths[0]) : "", "none");
                });
            });
            break;
        case "compress": {
            const name = (t.length === 1 ? base(t[0]).replace(/\.[^.\/]+$/, "") || base(t[0]) : (listing ? listing.name : "Archive")) + ".zip";
            askText("Compress (end the name with .zip, .7z, .tar.gz or .tar.xz)", name, "Compress", name.length - 4, text => {
                const fmt = /\.tar\.gz$|\.tgz$/i.test(text) ? "tar.gz" : /\.tar\.xz$|\.txz$/i.test(text) ? "tar.xz" : /\.tar$/i.test(text) ? "tar" : /\.7z$/i.test(text) ? "7z" : "zip";
                const fileName = fmt === "zip" && !/\.zip$/i.test(text) ? text + ".zip" : text;
                app.startJob({ verb: "compress", paths: t, fmt: fmt, archive: app.listing.path + "/" + fileName, dest: app.listing.path });
            });
            break;
        }
        case "extracthere": if (e) startJob({ verb: "extract", paths: [e.path], archive: e.path, here: true, dest: listing.path }); break;
        case "extracteach": for (const p of t) startJob({ verb: "extract", paths: [p], archive: p, here: true, dest: listing.path }); break;
        case "extractto":
            if (e) {
                const into = (writable ? listing.path : home) + "/" + archiveStem(e.name);
                askText("Extract “" + e.name + "” to", into, "Extract", into.length, text => {
                    let d = text.trim();
                    if (d.startsWith("~")) d = app.home + d.slice(1);
                    if (!d.startsWith("/")) d = app.listing.path + "/" + d;
                    app.startJob({ verb: "extract", paths: [e.path], archive: e.path, here: false, dest: d });
                });
            }
            break;
        case "newfolder": newFolder(); break;
        case "newfile": newFile(); break;
        case "sub-new": {
            const items = [{ id: "newfile", label: "Empty file" }];
            templates.forEach((tp, i) => items.push({ id: "tpl:" + i, label: tp.name }));
            showMenu("New document", items);
            break;
        }
        case "sub-sort": sortMenu(); break;
        case "sub-view": viewMenu(); break;
        case "desc": desc = !desc; rememberView(); break;
        case "trash": trashPaths(t); break;
        case "delete":
            confirm(t.length === 1 ? "Delete “" + base(t[0]) + "” for good?" : "Delete " + t.length + " items for good?",
                "This can't be undone. To keep a way back, use Move to Trash instead.", "Delete", () => {
                    app.call(["delete"].concat(t), (r, err) => {
                        if (err) return app.say(err, true);
                        app.say(t.length === 1 ? "Deleted “" + app.base(t[0]) + "”" : "Deleted " + t.length + " items");
                        app.marked = ({});
                        app.reload();
                        app.refreshPlaces();
                    });
                });
            break;
        case "restore":
            call(["restore"].concat(t), (r, err) => {
                if (err) return say(err, true);
                say(t.length === 1 ? "Put “" + base(t[0]) + "” back" : "Put " + t.length + " items back");
                marked = ({});
                reload();
                refreshPlaces();
            });
            break;
        case "empty":
            confirm("Empty the Trash?", "Everything in the Trash will be deleted for good.", "Empty Trash", () => {
                app.call(["empty-trash"], (r, err) => {
                    if (err) return app.say(err, true);
                    app.say("The Trash is empty");
                    if (app.inTrash) app.reload();
                    app.refreshPlaces();
                });
            });
            break;
        case "wallpaper":
            // hearth-wallpaper (in /usr/bin on Hearth OS): the desktop picks it up straight away.
            Quickshell.execDetached(["hearth-wallpaper", e.path]);
            say("“" + e.name + "” is your wallpaper now");
            break;
        case "hidden": setHidden(!hidden); break;
        case "exts": setShowExt(!showExt); break;
        case "detailspane": toggleDetails(); break;
        case "navpane": setFilesPref("navPane", !navShown); break;
        case "undo": undo(); break;
        case "redo": redo(); break;
        case "home": navigate(homePage, ""); break;
        case "bookmark": {
            const bp = e && e.dir ? e.path : listing.path;
            if (bp === homePage) break;
            call(["bookmark-add", bp], (r, err) => {
                if (err) return say(err, true);
                say("Added “" + base(bp) + "” to Favorites");
                refreshPlaces();
            });
            break;
        }
        case "terminal":
            call(["terminal", e && e.dir ? e.path : onHome ? home : listing.path], (r, err) => { if (err) say(err, true); });
            break;
        case "props":
            if (onCView || (e && e.cv)) { showCProps(onFolder ? null : e); break; }
            if (onSpecial) { if (onThisPc && current && current.path.indexOf("/") === 0) properties([current.path]); break; }
            properties(t.length ? t : [listing && !onHome ? listing.path : home]);
            break;
        case "p-open": openPlace(placeIndex); break;
        case "p-letter": {
            const pl = places[placeIndex];
            if (!pl) break;
            if (pl.key === "system") { say("C: is always Hearth, the drive Hearth runs from"); break; }
            // The letters unplugged drives keep too: from This PC's list, then the menu.
            call(["this-pc"], (r, err) => {
                if (err) return say(err, true);
                pc = r;
                letterMenu((r.drives || []).find(d => d.key === pl.key) || pl);
            });
            break;
        }
        case "p-newtab": if (places[placeIndex]) newTab(places[placeIndex].path); break;
        case "p-props": if (places[placeIndex]) properties([places[placeIndex].path]); break;
        case "p-mount": if (places[placeIndex]) mountPlace(places[placeIndex], false); break;
        case "p-auto": {
            // Kept in Hearth Files' settings (files.json "autoConnect"), never a password: it uses what you signed in
            // with (the keyring). Turned on: it's opened in the background right away too.
            const p = places[placeIndex];
            if (!p || !p.net) break;
            const on = !p.auto;
            call(["net-auto", JSON.stringify(p.net), on ? "on" : "off"], (r, err) => {
                if (err) return say(err, true);
                say(on ? "“" + p.label + "” connects by itself after you sign in. Open it once to sign in if it asks" : "“" + p.label + "” no longer connects by itself");
                if (on) Quickshell.execDetached(["python3", helper, "net-autoconnect"]);
                refreshPlaces();
            });
            break;
        }
        case "p-unbookmark": {
            const p = places[placeIndex];
            if (p) call(["bookmark-remove", p.path], (r, err) => { if (err) say(err, true); refreshPlaces(); });
            break;
        }
        case "p-unmount": case "p-eject": {
            const p = places[placeIndex];
            if (!p) break;
            const ej = id === "p-eject";
            say((ej ? "Removing" : "Unmounting") + " “" + p.label + "”…");
            call([ej ? "eject" : "unmount", p.path], (r, err) => {
                if (err) return say(err, true);
                say(ej ? "You can take out “" + p.label + "” now" : "“" + p.label + "” is unmounted");
                if (under(path, p.path)) navigate(home, "");
                refreshPlaces();
            });
            break;
        }
        }
    }
    function newFolder() {
        if (!writable) return;
        const names = (listing ? listing.entries : []).map(x => x.name);
        let name = "New folder", n = 1;
        while (names.indexOf(name) >= 0) name = "New folder " + (++n);
        const dir = listing.path;
        askText("New folder", name, "Create", name.length, text => {
            app.call(["mkdir", dir + "/" + text], (r, err) => {
                if (err) return app.say(err, true);
                app.pushUndo({ kind: "create", path: r.path || dir + "/" + text });
                app.load(dir, text, "none");
            });
        });
    }
    function newFile() {
        if (!writable) return;
        const names = (listing ? listing.entries : []).map(x => x.name);
        let name = "New file.txt", n = 1;
        while (names.indexOf(name) >= 0) name = "New file " + (++n) + ".txt";
        const dir = listing.path;
        askText("New empty file", name, "Create", name.lastIndexOf("."), text => {
            app.call(["new-file", dir + "/" + text], (r, err) => {
                if (err) return app.say(err, true);
                app.pushUndo({ kind: "create", path: (r && r.path) || dir + "/" + text });
                app.load(dir, text, "none");
            });
        });
    }
    function newFromTemplate(tp) {
        if (!tp || !writable) return;
        const dir = listing.path;
        const dot = tp.file.lastIndexOf(".");
        askText("New " + tp.name, tp.file, "Create", dot > 0 ? dot : tp.file.length, text => {
            app.call(["from-template", tp.path, dir, text], (r, err) => {
                if (err) return app.say(err, true);
                if (r && r.path) app.pushUndo({ kind: "create", path: r.path });
                app.load(dir, app.base(r.path), "none");
            });
        });
    }
    function trashPaths(t) {
        if (!t.length) return;
        call(["trash"].concat(t), (r, err) => {
            if (err) return say(err, true);
            pushUndo({ kind: "trash", paths: (r && r.trashed && r.trashed.length) ? r.trashed : t });
            say(t.length === 1 ? "Moved “" + base(t[0]) + "” to the Trash" : "Moved " + t.length + " items to the Trash");
            marked = ({});
            reload();
            refreshPlaces();
        });
    }
    function setClip(mode, t) {
        if (!t.length) return;
        if (mode === "cut" && !writable) return;
        clip = { mode: mode, paths: t };
        call(["clip-set", mode].concat(t));
        if (mode === "cut") say("Open the folder they should go to and choose Move here.");
        else say(t.length === 1 ? "Copied “" + base(t[0]) + "”. Open a folder and choose Paste." : "Copied " + t.length + " items. Open a folder and choose Paste.");
    }
    // Why nothing can be pasted or dropped here: Hearth (C:)'s view folders, and Windows and ProgramData (look only).
    function cRefusalFor(p) {
        if (DrivePaths.isCPath(p)) return DrivePaths.cRefusal(p, pathCtx);
        return cdriveOn ? DrivePaths.cRefusal(p, pathCtx) : "";
    }
    function cRefusalHere() { return listing ? cRefusalFor(listing.path) : ""; }
    function paste() {
        if (!writable) { const why = cRefusalHere(); if (why) say(why, true); return; }
        const dest = listing.path;
        // Files copied in another app (Dolphin, a browser…) count too; Hearth's own copy wins if it's the same.
        call(["clip-get"], (r, err) => {
            let mode = clip.mode, paths = clip.paths;
            if (r && r.paths && r.paths.length && JSON.stringify(r.paths) !== JSON.stringify(clip.paths)) {
                mode = r.cut ? "cut" : "copy";
                paths = r.paths;
            }
            if (!paths.length) return say("Nothing to paste. Copy something first.");
            transferTo(mode === "cut" ? "move" : "copy", paths, dest);
            if (mode === "cut") clip = { mode: "", paths: [] };
        });
    }
    // Copy or move, asking about names that are already there first.
    function transferTo(verb, paths, dest) {
        if (!paths.length) return;
        call(["conflicts"].concat(paths).concat([dest]), (r, err) => {
            if (err) return say(err, true);
            if (!r.conflicts.length) return startJob({ verb: verb, paths: paths, dest: dest, decide: {} });
            cf = { verb: verb, paths: paths, dest: dest, list: r.conflicts, i: 0, decide: {}, all: false, button: 1 };
            overlay = "conflict";
        });
    }
    function conflictChoose(choice) {
        const c = cf;
        if (!c) return;
        const d = Object.assign({}, c.decide);
        const last = c.all ? c.list.length - 1 : c.i;
        for (let k = c.i; k <= last; k++) d[c.list[k].name] = choice;
        if (last + 1 >= c.list.length) {
            cf = null;
            overlay = "";
            if (ui) ui.refocus();
            startJob({ verb: c.verb, paths: c.paths, dest: c.dest, decide: d });
        } else {
            cf = Object.assign({}, c, { i: last + 1, decide: d, button: 1 });
        }
    }

    // ------------------------------------------------ drag and drop
    function startDrag(index) {
        const e = entries[index];
        if (!e || overlay !== "") return false;
        if (!marked[e.path]) { marked = ({}); cur = index; }
        const t = targets();
        if (!t.length) return false;           // Hearth (C:)'s names (AppData) aren't dragged
        dragUris = t.map(p => fileUrl(p)).join("\r\n") + "\r\n";
        return true;
    }
    function dropOn(drop, dest) {
        const paths = (drop.urls || []).map(u => urlToPath(u)).filter(p => p && p !== dest);
        if (!paths.length || !dest) return;
        // Into Hearth (C:)'s view folders (Program Files, C:\ …), Windows or ProgramData: refused, saying why.
        const why = cRefusalFor(dest);
        if (why) { say(why, true); return; }
        if (briefcase.drop(paths, dest, drop)) return;      // into a Briefcase: added (or moved from another one)
        if (drop.proposedAction === Qt.LinkAction && dest !== trashPath) {
            drop.accept(Qt.LinkAction);
            return call(["link"].concat(paths).concat([dest]), (r, err) => {
                if (err) return say(err, true);
                pushUndo({ kind: "link", items: r.items });
                say(paths.length === 1 ? "Made a link to “" + base(paths[0]) + "”" : "Made " + paths.length + " links");
                reload();
            });
        }
        drop.accept(drop.proposedAction === Qt.CopyAction ? Qt.CopyAction : Qt.MoveAction);
        if (dest === trashPath) return trashPaths(paths);
        const copy = drop.proposedAction === Qt.CopyAction || rootOf(paths[0]).path !== rootOf(dest).path;
        const list = copy ? paths : paths.filter(p => parentOf(p) !== dest);
        if (!list.length) return;
        transferTo(copy ? "copy" : "move", list, dest);
    }

    // ================================================ opening scripts, programs, .desktop files and text
    // hearth-files open-plan says what opening the file should do. Scripts and programs ask what to do (the
    // answer is remembered for that file until it changes), .desktop files show what they run before starting it,
    // text-like files no app claims open in Hearth Text, and other files without an app offer a read-only view.
    function openFile(e) {
        call(["open-plan", e.path], (r, err) => {
            if (err || !r) return say(err || "Couldn't open “" + e.name + "”", true);
            if (r.kind === "script" || r.kind === "program") {
                if (r.remembered) return fileOpenChoice(r.remembered, e, r);
                return askRunOrOpen(e, r);
            }
            if (r.kind === "desktop") return askDesktop(e, r);
            if (r.kind === "archive") return askArchive(e);
            if (r.kind === "text") return openInText(e, false);
            if (r.kind === "binary") {
                foPlan = { e: e, plan: r, remember: false };
                return showMenu("No app opens “" + e.name + "”", [
                    { id: "fo-view", label: "Open in Hearth Text (read-only)" },
                    { id: "fo-openwith", label: "Open with…" },
                    sep(),
                    { id: "fo-cancel", label: "Cancel" }
                ]);
            }
            say("Opening “" + e.name + "”…");
            call(["open", e.path], (r2, err2) => { if (err2) say(err2, true); });
        });
    }
    // An archive opens in Hearth Files (its default app on Hearth OS): unpack it here or somewhere else, or
    // another app (Settings › Default apps › Archives picks one for good).
    function askArchive(e) {
        foPlan = { e: e, plan: { kind: "archive" }, remember: false };
        showMenu("“" + e.name + "”", [
            listing && listing.writable ? { id: "extracthere", label: "Extract here" } : sep(),
            { id: "extractto", label: "Extract to…" },
            sep(),
            { id: "fo-openwith", label: "Open with…" },
            sep(),
            { id: "fo-cancel", label: "Cancel" }
        ]);
    }
    function askRunOrOpen(e, r) {
        foPlan = { e: e, plan: r, remember: true };
        const more = r.executable ? "" : "…";
        const items = [
            { id: "fo-terminal", label: "Run in Terminal" + more },
            { id: "fo-run", label: "Run" + more }
        ];
        if (r.text) items.push({ id: "fo-text", label: "Open in Hearth Text" });
        if (!r.executable && r.canChmod) items.push(sep(), { id: "fo-chmod", label: "Make it runnable…" });
        items.push(sep(), { id: "fo-cancel", label: "Cancel" });
        showMenu("Open “" + e.name + "” · " + (r.kind === "program" ? "a program" : (r.interpreter ? r.interpreter + " " : "") + "script"), items);
    }
    function fileOpenMenuItems(e) {
        const name = e.name.toLowerCase();
        if (/\.appimage$/.test(name)) return [];
        if (/\.desktop$/.test(name)) return [{ id: "fo-launch", label: "Start…" }, { id: "fo-text", label: "Open in Hearth Text" }];
        const script = /\.(sh|bash|zsh|ksh|command|py|pyw|pl|rb|lua|run|bats)$/.test(name);
        const out = [];
        if (script || e.exec) {
            out.push({ id: "fo-terminal", label: "Run in Terminal" }, { id: "fo-run", label: "Run" });
            if (!e.exec) out.push({ id: "fo-chmod", label: "Make it runnable…" });
        }
        if (script || ["text", "file"].indexOf(kindOf(e)) >= 0) out.push({ id: "fo-text", label: "Open in Hearth Text" });
        if (script || e.exec) out.push({ id: "fo-forget", label: "Ask what opening does" });
        return out;
    }
    // A choice from the open question (remembered for the file) or from the right-click menu.
    function fileOpenChoice(choice, e, plan) {
        const fp = foPlan;
        foPlan = null;
        if (fp && fp.e) { e = fp.e; plan = fp.plan; }
        if (!e || choice === "cancel") return;
        const remember = !!(fp && fp.remember) && ["terminal", "run", "text"].indexOf(choice) >= 0;
        if (remember) call(["open-choice", e.path, choice], (r, err) => {
            if (!err) say("Opening “" + e.name + "” does this from now on (right-click it for the other choices)");
        });
        switch (choice) {
        case "terminal":
        case "run":
            return runFileAs(e, choice === "terminal", plan);
        case "text": return openInText(e, false);
        case "view": return openInText(e, true);
        case "openwith": return showApps(e, false);
        case "chmod": return askRunnable(e, null);
        case "forget":
            return call(["open-choice", e.path], (r, err) => say(err ? err : "Opening “" + e.name + "” asks what to do again", !!err));
        case "launch":
            return call(["open-plan", e.path], (r, err) => { if (err) return say(err, true); askDesktop(e, r); });
        }
    }
    function runFileAs(e, inTerminal, plan) {
        const go = () => {
            say((inTerminal ? "Running “" : "Starting “") + e.name + (inTerminal ? "” in Terminal…" : "”…"));
            call(["run", e.path].concat(inTerminal ? ["--terminal"] : []), (r, err) => {
                if (err && err.indexOf("confirm:") === 0) return askRunnable(e, go);
                if (err) say(err, true);
            });
        };
        if (plan && !plan.executable) return askRunnable(e, go);
        go();
    }
    function askRunnable(e, then) {
        confirm("Make “" + e.name + "” runnable?",
            "Only run scripts and programs you trust. One from the internet or from someone you don't know can delete your files or install something harmful, and Hearth can't tell what it will do.",
            then ? "Make runnable and run" : "Make runnable", () => {
                call(["make-runnable", e.path], (r, err) => {
                    if (err) return say(err, true);
                    if (listing) reload();
                    if (then) then(); else say("“" + e.name + "” can run now");
                });
            });
        dlgDanger = false;
    }
    function askDesktop(e, r) {
        const d = r.desktop || {};
        if (!d.runnable) return openInText(e, false);
        const where = parentOf(e.path).replace(home, "~");
        confirm("Start “" + d.name + "”?",
            (d.comment ? d.comment + "\n\n" : "") + "This shortcut runs:  " + (d.type === "Link" ? d.url : d.exec) + (d.terminal ? "  (in a terminal)" : "")
            + "\n\nIt's in " + where + ". Only start shortcuts you trust: what it runs can do anything you can.",
            "Start", () => call(["desktop-launch", e.path], (r2, err) => say(err ? err : "Starting " + d.name + "…", !!err)));
        dlgDanger = false;
        dlgAlt = "Open in Hearth Text";
        dlgAltAction = () => openInText(e, false);
    }
    function openInText(e, readonly) {
        say("Opening “" + e.name + "” in Hearth Text…");
        call(["open-text", e.path].concat(readonly ? ["--readonly"] : []), (r, err) => { if (err) say(err, true); });
    }
    function acceptAlt() {
        const a = dlgAltAction;
        closeOverlay();
        if (a) a();
    }

    // ================================================ dialogs
    function askText(title, initial, ok, selectTo, action) {
        dlgSecret = false;
        dlgTitle = title;
        dlgOk = ok;
        dlgDanger = false;
        dlgAction = action;
        overlay = "input";
        if (ui) ui.startField(initial, selectTo);
        if (viaPad) openKeyboard();
    }
    // The same question without the red "this deletes something" look (Run this app?, Add to app menu?).
    function ask(title, text, ok, action) {
        confirm(title, text, ok, action);
        dlgDanger = false;
    }
    function confirm(title, text, ok, action) {
        dlgAlt = "";
        dlgAltAction = null;
        dlgTitle = title;
        dlgText = text;
        dlgOk = ok;
        dlgDanger = true;
        dlgAction = action;
        dlgButton = 0;                         // Cancel first: a stray A press doesn't delete anything
        overlay = "confirm";
    }
    function showApps(e, all) {
        if (!e) return;
        dlgTitle = "Open “" + e.name + "” with";
        dlgApps = [];
        dlgIndex = 0;
        dlgInfo = null;
        dlgAppsAll = !!all;
        dlgPath = e.path;
        overlay = "apps";
        call(["apps", e.path].concat(all ? ["--all"] : []), (r, err) => {
            if (dlgPath !== e.path) return;
            if (err) {
                overlay = "";
                return say(err, true);
            }
            dlgInfo = r;
            dlgApps = r.apps;
        });
    }
    function openWith(a) {
        if (!a) return;
        const p = dlgPath;
        if (a.other) {
            const e = entries.find(x => x.path === p) || { name: base(p), path: p };
            return showApps(e, !dlgAppsAll);
        }
        overlay = "";
        say("Opening with " + a.name + "…");
        call(["open", p, a.id], (r, err) => { if (err) say(err, true); });
    }
    function showPreview(e) {
        if (!e) return;
        pv = { e: e, text: "", error: "" };
        overlay = "preview";
        const k = kindOf(e);
        if (k === "text" || (!e.dir && k === "file" && e.size < 4 * 1024 * 1024)) {
            const at = e.path;
            call(["peek", e.path], (r, err) => {
                if (!pv || pv.e.path !== at) return;
                pv = { e: pv.e, text: r ? r.text + (r.more ? "\n…" : "") : "", error: err };
            });
        }
    }
    function previewStep(d) {
        const i = Math.max(0, Math.min(entries.length - 1, cur + d));
        cur = i;
        showPreview(entries[i]);
    }
    function accept(text) {
        const act = dlgAction;
        const kind = overlay;
        const secret = dlgSecret;
        closeOverlay();
        if (!act) return;
        if (kind === "input") {
            const t = secret ? (text || "") : (text || "").trim();
            if (t) act(t);
        } else {
            act();
        }
    }
    function closeOverlay() {
        if (briefcase.mode !== "" && briefcase.mode !== "job") briefcase.close();
        overlay = "";
        dlgSecret = false;
        menuOnFolder = false;
        dlgAction = null;
        dlgAlt = "";
        dlgAltAction = null;
        dlgPath = "";
        cf = null;
        pv = null;
        if (oskOpened) closeKeyboard();
        if (ui) ui.refocus();
    }
    function openKeyboard() {
        if (SettingsNav.pad) {
            SettingsNav.pad.openKeyboard();
            oskOpened = true;
        }
    }
    function closeKeyboard() {
        if (SettingsNav.pad) SettingsNav.pad.closeKeyboard();
        oskOpened = false;
    }

    // ================================================ copy / move / compress / extract in the background
    Process {
        id: jobProc
        stdout: SplitParser { onRead: line => app.jobLine(line) }
        stderr: SplitParser { onRead: line => console.warn("hearth-files:", line) }
        onExited: code => Qt.callLater(app.jobExited)
    }
    function jobVerbing(v) { return ({ copy: "Copying", move: "Moving", compress: "Compressing", extract: "Unpacking" })[v] || "Working on"; }
    function startJob(j) {
        if (!j.paths || !j.paths.length) return;
        if (jobProc.running) {
            jobQueue = jobQueue.concat([j]);
            say("That starts as soon as the current one is done");
            return;
        }
        let cmd;
        if (j.verb === "compress") cmd = ["compress", j.fmt, j.archive].concat(j.paths);
        else if (j.verb === "extract") cmd = ["extract", j.archive, j.here ? "--here" : j.dest];
        else {
            cmd = [j.verb].concat(j.paths).concat([j.dest]);
            if (j.decide && Object.keys(j.decide).length) cmd = cmd.concat(["--decide", JSON.stringify(j.decide)]);
        }
        const dest = j.verb === "compress" ? j.archive : j.dest;
        const destName = j.verb === "compress" ? base(j.archive) : (rootOf(dest).path === dest ? rootOf(dest).label : base(dest));
        job = { verb: j.verb, count: j.paths.length, name: j.paths.length === 1 ? base(j.paths[0]) : "", destName: destName,
            done: 0, total: 0, speed: 0, file: "", cancelling: false };
        jobNow = j;
        jobLast = "";
        jobProc.command = ["python3", helper].concat(cmd);
        jobProc.running = true;
    }
    function jobLine(line) {
        if (line.startsWith("progress: ")) {
            try {
                const p = JSON.parse(line.slice(10));
                if (job) job = Object.assign({}, job, { done: p.done, total: p.total, speed: p.speed, file: p.file });
            } catch (e) {}
        } else if (line.trim()) {
            jobLast = line.trim();
        }
    }
    function jobExited() {
        const j = job;
        const jn = jobNow;
        const last = jobLast;
        job = null;
        jobNow = null;
        let sel = "";
        if (last.startsWith("error: ")) {
            say(last.slice(7), true);
        } else {
            let r = null;
            try { r = JSON.parse(last); } catch (e) {}
            const what = j ? (j.name ? "“" + j.name + "”" : j.count + " items") : "";
            if (!r || r.cancelled) say(j && j.cancelling ? "Stopped. Nothing half-done was left behind." : "It stopped before finishing", !(j && j.cancelling));
            else if (j.verb === "compress") { say("Made “" + j.destName + "”"); sel = j.destName; }
            else if (j.verb === "extract") { say("Unpacked " + what + (r.path ? " into “" + base(r.path) + "”" : "")); sel = r.path ? base(r.path) : ""; }
            else if (jn && jn.undoOp) {
                // A move that undid (or redid) another one: the other list gets it, with where things are now.
                const op = jn.undoMode === "redo" ? Object.assign({}, jn.undoOp, { items: r.items || [] }) : jn.undoOp;
                if (jn.undoMode === "undo") redoStack = redoStack.concat([op]); else undoStack = undoStack.concat([op]);
                say((jn.undoMode === "undo" ? "Undone: " : "Redone: ") + opName(op));
            } else {
                const skipped = r.skipped && r.skipped.length ? " (skipped " + r.skipped.length + ")" : "";
                say((j.verb === "move" ? "Moved " : "Copied ") + what + " to “" + j.destName + "”" + skipped);
                if (jn && r.items && r.items.length) {
                    const parents = jn.paths.map(x => parentOf(x));
                    if (j.verb === "copy") pushUndo({ kind: "copy", items: r.items });
                    else if (parents.every(x => x === parents[0])) pushUndo({ kind: "move", items: r.items, srcs: jn.paths, dest: jn.dest, back: parents[0] });
                }
            }
        }
        if (listing) load(listing.path, sel || (current ? current.name : ""), "none");
        refreshPlaces();
        if (jobQueue.length) {
            const n = jobQueue[0];
            jobQueue = jobQueue.slice(1);
            startJob(n);
        }
    }
    function cancelJob() {
        if (!job || !jobProc.running) return;
        job = Object.assign({}, job, { cancelling: true });
        jobQueue = [];
        jobProc.running = false;               // SIGTERM: hearth-files removes the half-done item
    }

    // ================================================ the window
    LazyLoader {
        active: app.shown
        FloatingWindow {
            id: win
            title: app.picking ? (app.pick.title || "Choose") + " – Files" : app.path ? app.titleOf(app.path) + " – Files" : "Files"
            implicitWidth: 1180
            implicitHeight: 740
            color: Theme.bg
            visible: true
            onVisibleChanged: if (!visible) app.close()
            Component.onCompleted: {
                app.ui = win;
                keys.forceActiveFocus();
                if (app.picking && app.pick.kind === "save") Qt.callLater(win.focusName);
            }
            Component.onDestruction: if (app.ui === win) app.ui = null

            function refocus() { if (app.zone === "pick" && app.pickLit("name")) pickField.forceActiveFocus(); else keys.forceActiveFocus(); }
            // The picker's "Name" box (saving), with the name up to its extension selected.
            function focusName() {
                if (!app.picking || app.pick.kind !== "save") return;
                app.zone = "pick";
                app.pickIndex = 0;
                pickField.forceActiveFocus();
                const n = pickField.text, dot = n.lastIndexOf(".");
                pickField.select(0, dot > 0 ? dot : n.length);
            }
            function startField(text, selectTo) {
                field.text = text;
                field.forceActiveFocus();
                field.select(0, Math.max(0, selectTo));
            }
            function focusSearch(text) {
                app.zone = "toolbar";
                app.toolIndex = app.toolItems.findIndex(t => t.t === "search");
                if (text !== undefined) search.text = text;
                search.forceActiveFocus();
                search.cursorPosition = search.text.length;
            }
            function editPath() {
                app.editingPath = true;
                app.zone = "toolbar";
                app.toolIndex = app.toolItems.findIndex(t => t.t === "edit");
                // On a lettered drive the Windows form (D:\Games); This PC and Installed apps by name; with Hearth (C:)'s
                // view the C:\ form (unless you chose Linux paths); else the Linux path.
                pathField.text = app.onThisPc ? "This PC" : app.onApps ? "This PC\\Installed apps" : app.addressText(app.path);
                pathField.forceActiveFocus();
                pathField.selectAll();
            }
            function endEditPath(go) {
                const t = pathField.text.trim();
                app.editingPath = false;
                keys.forceActiveFocus();
                if (!go || !t) return;
                const lt = t.toLowerCase().replace(/[\\/]+$/, "");
                if (lt === "this pc") return app.navigate(app.thisPcPage, "");
                if (lt === "this pc\\installed apps" || lt === "this pc/installed apps" || lt === "installed apps") return app.navigate(app.appsPage, "");
                // D:\Games, d:/Games, C:\Users\<you>\Documents: the drive letters (drive-paths.js).
                const wl = app.fromLetters(t);
                if (wl) {
                    // C:\Program Files\Firefox before Installed apps' list is in: read it first, then try again.
                    if (wl.error && app.cdriveOn && /^\s*c:[\\/]+program files/i.test(t) && !app.appList.length)
                        return app.loadApps(() => { const w2 = app.fromLetters(t); if (w2 && w2.error) app.say(w2.error, true); else if (w2) app.navigate(w2.path, ""); });
                    if (wl.error) return app.say(wl.error, true);
                    return app.navigate(wl.path, "");
                }
                let p = t.startsWith("file://") ? decodeURIComponent(t.slice(7)) : t;
                if (p.startsWith("~")) p = app.home + p.slice(1);
                if (!p.startsWith("/")) p = app.path + "/" + p;
                app.navigate(p, "");
            }
            readonly property int cols: app.shownView === "details" ? 1 : Math.max(1, Math.floor(grid.width / grid.cellWidth))
            function showCurrent() {
                if (app.onSpecial) return;           // FilesThisPc.qml / FilesApps.qml keep their own item in view
                if (app.shownView === "details") list.positionViewAtIndex(app.cur, ListView.Contain);
                else grid.positionViewAtIndex(app.cur, GridView.Contain);
            }
            function wheel(ev) {
                if (app.onSpecial) return;
                const v = app.shownView === "details" ? list : grid;
                const dy = ev.pixelDelta.y !== 0 ? ev.pixelDelta.y : ev.angleDelta.y / 120 * 100;
                const top = v.originY, bottom = v.originY + Math.max(0, v.contentHeight - v.height);
                v.contentY = Math.max(top, Math.min(bottom, v.contentY - dy));
            }
            Connections {
                target: app
                function onCurChanged() { Qt.callLater(win.showCurrent); }
                function onViewChanged() { Qt.callLater(win.showCurrent); }
                function onQueryChanged() { if (search.text !== app.query) search.text = app.query; }
                function onPlaceIndexChanged() { placeList.positionViewAtIndex(app.placeIndex, ListView.Contain); }
            }

            // The invisible thing that's dragged to other folders, places and apps (as a list of file:// URLs).
            Item {
                id: dragProxy
                width: 1
                height: 1
                Drag.dragType: Drag.Automatic
                Drag.supportedActions: Qt.CopyAction | Qt.MoveAction | Qt.LinkAction
                Drag.proposedAction: Qt.MoveAction
                Drag.mimeData: ({ "text/uri-list": app.dragUris })
                Drag.onDragFinished: dropAction => { dragProxy.x = 0; dragProxy.y = 0; }
            }
            // What a drag looks like under the pointer: the first item's picture and how many (Ctrl copies, Alt makes
            // a link). Drawn off to the side and kept as the drag's image; shown in the window only for the staged
            // drag (filemanager dragdemo), because CI has no mouse.
            Rectangle {
                id: dragGhost
                readonly property var picked: app.targets()
                readonly property var first: app.entries.find(e => e.path === picked[0]) || null
                x: app.dragDemo ? Math.round(win.width * 0.52) : -4000
                y: app.dragDemo ? Math.round(win.height * 0.5) : -4000
                z: 40
                width: ghostRow.implicitWidth + 28
                height: 56
                radius: Theme.r(12)
                color: Theme.panel2
                border.width: 2
                border.color: Theme.accent
                opacity: 0.94
                Row {
                    id: ghostRow
                    anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter }
                    spacing: 10
                    FileGlyph { anchors.verticalCenter: parent.verticalCenter; kind: app.kindOf(dragGhost.first); label: app.extLabel(dragGhost.first); size: 34 }
                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 1
                        Text {
                            text: dragGhost.picked.length > 1 ? app.plural(dragGhost.picked.length, "item") : app.shownName(dragGhost.first)
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold
                        }
                        Text {
                            text: app.dragHint ? "Move to “" + app.base(app.dragHint) + "”" : "Ctrl copies · Alt makes a link"
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                        }
                    }
                }
                Rectangle {
                    visible: dragGhost.picked.length > 1
                    anchors { right: parent.right; top: parent.top; margins: -8 }
                    width: 24; height: 24; radius: 12
                    color: Theme.accent
                    Text { anchors.centerIn: parent; text: dragGhost.picked.length; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                }
            }
            // Renaming in place: a text box over the name (the extension isn't selected, like Windows).
            Component {
                id: inlineRename
                Rectangle {
                    color: Theme.bg
                    radius: Theme.r(4)
                    border.width: 2
                    border.color: Theme.accent
                    TextInput {
                        id: renameEd
                        anchors { fill: parent; leftMargin: 6; rightMargin: 6 }
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.text
                        selectionColor: Theme.accent
                        selectedTextColor: Theme.accentInk
                        selectByMouse: true
                        clip: true
                        font.family: Theme.font; font.pixelSize: 13
                        property bool done: false
                        Component.onCompleted: {
                            const e = app.entries.find(x => x.path === app.renaming);
                            text = e ? e.name : "";
                            select(0, e && !e.dir && text.lastIndexOf(".") > 0 ? text.lastIndexOf(".") : text.length);
                            forceActiveFocus();
                        }
                        function finish(ok) {
                            // Closed from outside (Esc elsewhere, a new folder, filemanager rename ""): nothing to do,
                            // and writing renaming again here would loop the Loader's binding.
                            if (done || app.renaming === "") { done = true; return; }
                            done = true;
                            const t = text.trim();
                            keys.forceActiveFocus();
                            app.finishRename(ok ? t : null);
                        }
                        Keys.onReturnPressed: finish(true)
                        Keys.onEnterPressed: finish(true)
                        Keys.onEscapePressed: finish(false)
                        // Clicking elsewhere keeps the new name, after the current binding pass (no Loader loop).
                        onActiveFocusChanged: if (!activeFocus && !done && app.renaming !== "") {
                            done = true;
                            const p = app.renaming, t = text.trim();
                            Qt.callLater(() => { if (app.renaming === p) app.finishRename(t); });
                        }
                    }
                }
            }
            // The drag picture: grabbed into a file (a grabToImage url makes Drag warn about sourceSize every time),
            // with a counter on the url so the picture cache shows the newest one.
            property int ghostN: 0
            readonly property string ghostFile: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/hearth-files-drag.png"
            Timer {
                id: ghostTimer
                interval: 250
                onTriggered: if (app.mouseMode && dragGhost.picked.length) dragGhost.grabToImage(r => {
                    if (r.saveToFile(win.ghostFile)) dragProxy.Drag.imageSource = "file://" + win.ghostFile + "?" + (++win.ghostN);
                }, Qt.size(dragGhost.width, dragGhost.height))
            }
            Connections {
                target: app
                function onMarkedChanged() { ghostTimer.restart(); }
                function onCurChanged() { ghostTimer.restart(); }
            }
            function itemDragged(index, active) {
                if (active && app.startDrag(index)) dragProxy.Drag.active = true;
            }

            // ------------------------------------------------ every key comes through here
            Item {
                id: keys
                anchors.fill: parent
                focus: true
                HoverHandler {
                    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
                    onPointChanged: if (!app.mouseMode) app.mouseMode = true
                }
                PointHandler {
                    acceptedDevices: PointerDevice.TouchScreen
                    onActiveChanged: if (active) app.mouseMode = false
                }
                Keys.onPressed: e => {
                    if (app.overlay !== "") return win.overlayKey(e);
                    const ctrl = (e.modifiers & Qt.ControlModifier) !== 0;
                    const shift = (e.modifiers & Qt.ShiftModifier) !== 0;
                    const alt = (e.modifiers & Qt.AltModifier) !== 0;
                    if (alt && !ctrl) {
                        switch (e.key) {
                        case Qt.Key_Left: app.goBack(); break;
                        case Qt.Key_Right: app.goForward(); break;
                        case Qt.Key_Up: app.goUp(false); break;
                        case Qt.Key_Home: app.navigate(app.home, ""); break;
                        case Qt.Key_P: app.toggleDetails(); break;
                        case Qt.Key_Return: case Qt.Key_Enter: app.runMenu("props"); break;
                        default: return;
                        }
                        e.accepted = true;
                        return;
                    }
                    if (ctrl) {
                        switch (e.key) {
                        case Qt.Key_C:
                            // Ctrl+Alt+C the Linux path; Ctrl+Shift+C Copy path (C:\ or Linux, as Drives & folders says).
                            if (alt) app.runMenu("copypath");
                            else if (shift) { app.menuOnFolder = !app.current; app.runMenu(app.cCopyLinux ? "copypath" : "copywin"); }
                            else app.setClip("copy", app.targets());
                            break;
                        case Qt.Key_X: app.setClip("cut", app.targets()); break;
                        case Qt.Key_V: app.paste(); break;
                        case Qt.Key_A: app.selectAll(); break;
                        case Qt.Key_I: app.invertSelection(); break;
                        case Qt.Key_H: app.setHidden(!app.hidden); break;
                        case Qt.Key_F: win.focusSearch(); break;
                        case Qt.Key_L: win.editPath(); break;
                        case Qt.Key_R: app.reload(); break;
                        case Qt.Key_D: app.runMenu("bookmark"); break;
                        case Qt.Key_T: app.newTab(app.picking ? app.path : app.startPage); break;
                        case Qt.Key_W: app.closeTab(app.tabIndex); break;
                        case Qt.Key_Tab: app.switchTab((app.tabIndex + 1) % app.tabs.length); break;
                        case Qt.Key_Backtab: app.switchTab((app.tabIndex - 1 + app.tabs.length) % app.tabs.length); break;
                        case Qt.Key_PageDown: app.switchTab((app.tabIndex + 1) % app.tabs.length); break;
                        case Qt.Key_PageUp: app.switchTab((app.tabIndex - 1 + app.tabs.length) % app.tabs.length); break;
                        case Qt.Key_N: if (shift) app.newFolder(); else if (alt) app.newFile(); else return; break;
                        case Qt.Key_Space: app.toggleMark(app.current); break;
                        case Qt.Key_1: app.setView("grid"); break;
                        case Qt.Key_2: app.setView("compact"); break;
                        case Qt.Key_3: app.setView("details"); break;
                        case Qt.Key_4: app.setView("large"); break;
                        case Qt.Key_7: app.setView("xlarge"); break;
                        case Qt.Key_5: app.setView("small"); break;
                        case Qt.Key_6: app.setView("tiles"); break;
                        case Qt.Key_Z: if (shift) app.redo(); else app.undo(); break;
                        case Qt.Key_Y: app.redo(); break;
                        case Qt.Key_E: win.focusSearch(); break;
                        case Qt.Key_Q: app.close(); break;
                        default: return;
                        }
                        e.accepted = true;
                        return;
                    }
                    if (e.key === Qt.Key_Menu || (e.key === Qt.Key_F10 && shift)) {
                        app.viaPad = e.key === Qt.Key_Menu;
                        if (app.zone === "places") app.openPlaceMenu();
                        else app.openMenu(app.zone !== "main" || !app.current);
                        e.accepted = true;
                        return;
                    }
                    app.viaPad = false;
                    if (e.key === Qt.Key_F5) { app.reload(); app.refreshPlaces(); e.accepted = true; return; }
                    if (e.key === Qt.Key_F3) { win.focusSearch(); e.accepted = true; return; }
                    if (e.key === Qt.Key_F4 && shift) { app.runMenu("terminal"); e.accepted = true; return; }
                    if (e.key === Qt.Key_Back) { app.goBack(); e.accepted = true; return; }
                    if (e.key === Qt.Key_Forward) { app.goForward(); e.accepted = true; return; }
                    if (app.zone === "places") return win.placesKey(e);
                    if (app.zone === "toolbar") return win.toolbarKey(e);
                    if (app.zone === "tabs") return win.tabsKey(e);
                    if (app.zone === "pick") return win.pickKey(e);
                    return win.mainKey(e, shift);
                }
            }
            function placesKey(e) {
                switch (e.key) {
                case Qt.Key_Down: case Qt.Key_Tab: app.placeIndex = Math.min(app.places.length - 1, app.placeIndex + 1); break;
                case Qt.Key_Up: case Qt.Key_Backtab: app.placeIndex = Math.max(0, app.placeIndex - 1); break;
                case Qt.Key_Right: case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: app.openPlace(app.placeIndex); break;
                case Qt.Key_Escape: app.close(); break;
                case Qt.Key_Backspace: app.zone = "main"; break;
                default: return;
                }
                e.accepted = true;
            }
            function tabsKey(e) {
                switch (e.key) {
                case Qt.Key_Left: case Qt.Key_Backtab: app.tabFocus = Math.max(0, app.tabFocus - 1); break;
                case Qt.Key_Right: case Qt.Key_Tab: app.tabFocus = Math.min(app.tabs.length, app.tabFocus + 1); break;
                case Qt.Key_Down: case Qt.Key_Escape: app.zone = "toolbar"; break;
                case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space:
                    if (app.tabFocus >= app.tabs.length) app.newTab(app.startPage);
                    else { app.switchTab(app.tabFocus); app.zone = "main"; }
                    break;
                case Qt.Key_Delete: case Qt.Key_W: if (app.tabFocus < app.tabs.length) app.closeTab(app.tabFocus); break;
                default: return;
                }
                e.accepted = true;
            }
            function toolbarKey(e) {
                const it = app.toolItems[app.toolIndex];
                switch (e.key) {
                case Qt.Key_Left: case Qt.Key_Backtab: app.toolIndex = Math.max(0, app.toolIndex - 1); break;
                case Qt.Key_Right: case Qt.Key_Tab: app.toolIndex = Math.min(app.toolItems.length - 1, app.toolIndex + 1); break;
                case Qt.Key_Down: case Qt.Key_Escape: app.zone = "main"; break;
                case Qt.Key_Up:
                    if (!app.picking) { app.tabFocus = app.tabIndex; app.zone = "tabs"; }
                    break;
                case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: {
                    if (!it) break;
                    if (it.t === "back") app.goBack();
                    else if (it.t === "forward") app.goForward();
                    else if (it.t === "up") app.goUp(false);
                    else if (it.t === "crumb") app.navigate(it.path, "");
                    else if (it.t === "edit") win.editPath();
                    else if (it.t === "search") win.focusSearch();
                    else app.command(it.t);
                    break;
                }
                default: return;
                }
                e.accepted = true;
            }
            // This PC (its groups, like Windows) and Installed apps (a list): arrows move, A / Enter opens (an app: its
            // "···" menu), B / Esc goes up; typing filters (the on-screen keyboard too).
            function pageKey(e) {
                const n = app.entries.length, i = app.cur;
                const step = (dx, dy) => {
                    if (app.onApps || app.onCView) {
                        if (dx < 0) win.toPlaces();
                        else if (dy < 0) { if (i > 0) app.cur = i - 1; else win.toToolbar(); }
                        else if (dy > 0 && i < n - 1) app.cur = i + 1;
                        return;
                    }
                    if (!n) { if (dx < 0) win.toPlaces(); else if (dy < 0) win.toToolbar(); return; }
                    const r = app.pcStep(i, dx, dy);
                    if (r === -1) win.toPlaces();
                    else if (r === -2) win.toToolbar();
                    else app.cur = r;
                };
                switch (e.key) {
                case Qt.Key_Left: step(-1, 0); break;
                case Qt.Key_Right: step(1, 0); break;
                case Qt.Key_Up: step(0, -1); break;
                case Qt.Key_Down: step(0, 1); break;
                case Qt.Key_Tab: app.cur = Math.min(Math.max(0, n - 1), i + 1); break;
                case Qt.Key_Backtab: app.cur = Math.max(0, i - 1); break;
                case Qt.Key_Home: app.cur = 0; break;
                case Qt.Key_End: app.cur = Math.max(0, n - 1); break;
                case Qt.Key_PageDown: app.cur = Math.min(Math.max(0, n - 1), i + 6); break;
                case Qt.Key_PageUp: app.cur = Math.max(0, i - 6); break;
                case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: app.activate(app.current); break;
                case Qt.Key_Escape: if (app.query) app.query = ""; else app.goUp(true); break;
                case Qt.Key_Backspace: if (app.query) app.query = app.query.slice(0, -1); else app.goUp(false); break;
                case Qt.Key_F2: case Qt.Key_Delete: break;
                default: return false;
                }
                return true;
            }
            function mainKey(e, shift) {
                if (app.onSpecial && win.pageKey(e)) { e.accepted = true; return; }
                const count = app.entries.length;
                const c = win.cols;
                const i = app.cur;
                const go = n => {
                    n = Math.max(0, Math.min(count - 1, n));
                    if (shift) {
                        if (app.anchor < 0) app.anchor = i;
                        app.selectRange(app.anchor, n);
                    } else if (!app.markCount) app.anchor = n;
                    app.cur = n;
                };
                switch (e.key) {
                case Qt.Key_Left:
                    if (app.shownView === "details" || i % c === 0) { if (!shift) win.toPlaces(); }
                    else go(i - 1);
                    break;
                case Qt.Key_Right:
                    if (app.shownView !== "details" && i + 1 < count && (i + 1) % c !== 0) go(i + 1);
                    break;
                case Qt.Key_Up:
                    if (i - c >= 0) go(i - c);
                    else if (!shift) win.toToolbar();
                    break;
                case Qt.Key_Down:
                    if (i + c < count) go(i + c);
                    else if (Math.floor(i / c) < Math.floor((count - 1) / c)) go(count - 1);
                    else if (app.picking && !shift) win.toPick();
                    break;
                case Qt.Key_Tab: app.cur = Math.min(count - 1, i + 1); break;
                case Qt.Key_Backtab: app.cur = Math.max(0, i - 1); break;
                case Qt.Key_PageDown: go(i + c * 4); break;
                case Qt.Key_PageUp: go(i - c * 4); break;
                case Qt.Key_Home: go(0); break;
                case Qt.Key_End: go(count - 1); break;
                case Qt.Key_Return: case Qt.Key_Enter: app.activate(app.current); break;
                case Qt.Key_Space: if (app.current) app.showPreview(app.current); break;
                case Qt.Key_F2:
                    if (app.writable && !app.inTrash && app.markCount > 1) app.runMenu("batchrename");
                    else if (app.current && app.writable && !app.inTrash) app.runMenu("rename");
                    break;
                case Qt.Key_Delete:
                    if (app.targets().length && (app.writable || app.inTrash)) app.runMenu(shift || app.inTrash ? "delete" : "trash");
                    break;
                case Qt.Key_Escape:
                    if (app.query) app.query = "";
                    else if (app.found !== null) app.clearSearch();
                    else if (app.markCount) app.marked = ({});
                    else app.goUp(true);
                    break;
                case Qt.Key_Backspace:
                    if (app.query) app.query = app.query.slice(0, -1);
                    else app.goUp(false);
                    break;
                default:
                    // Typing jumps to the first name that starts with what was typed (type quickly to add letters,
                    // like Windows); when no name starts with it, it filters this folder instead (the on-screen
                    // keyboard too: Y, then type).
                    if (e.text && e.text.length === 1 && e.text >= " " && !(e.modifiers & (Qt.AltModifier | Qt.MetaModifier))) {
                        if (e.text === "/" || e.text === "~") { win.editPath(); pathField.text = e.text === "~" ? "~/" : "/"; break; }
                        if (!app.query && app.typeAhead(e.text)) break;
                        win.focusSearch(app.query + e.text);
                        break;
                    }
                    return;
                }
                e.accepted = true;
            }
            // The picker's bar: Left / Right between its controls, Up back to the files, A / Enter uses one.
            function toPick() {
                app.zone = "pick";
                app.pickIndex = Math.max(0, app.pickItems.length - 1);
            }
            function pickKey(e) {
                switch (e.key) {
                case Qt.Key_Left: case Qt.Key_Backtab: app.pickIndex = Math.max(0, app.pickIndex - 1); break;
                case Qt.Key_Right: case Qt.Key_Tab: app.pickIndex = Math.min(app.pickItems.length - 1, app.pickIndex + 1); break;
                case Qt.Key_Up: case Qt.Key_Escape: app.zone = "main"; break;
                case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: app.pickRun(app.pickItems[app.pickIndex]); break;
                default: return;
                }
                e.accepted = true;
            }
            function toPlaces() {
                if (!app.navShown) return;       // View › Navigation pane is off
                const i = app.places.findIndex(p => p.path === app.path);
                if (i >= 0) app.placeIndex = i;
                app.zone = "places";
            }
            function toToolbar() {
                app.toolIndex = Math.max(0, app.toolItems.findIndex(t => t.t === "crumb" && t.path === app.path));
                app.zone = "toolbar";
            }
            function overlayKey(e) {
                const up = e.key === Qt.Key_Up || e.key === Qt.Key_Backtab;
                const down = e.key === Qt.Key_Down || e.key === Qt.Key_Tab;
                const ok = e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space;
                const back = e.key === Qt.Key_Escape || e.key === Qt.Key_Backspace || e.key === Qt.Key_Menu;
                if (app.overlay === "briefcase") {
                    if (!bcDialog.key(e)) return;
                } else if (app.overlay === "menu" && styledMenu.styled) {
                    // Another right-click menu look (Theme.menuStyle, MenuStyles.qml): it moves and chooses itself.
                    if (!styledMenu.item.key(e)) return;
                } else if (app.overlay === "menu") {
                    const n = app.menuItems.length;
                    const step = d => {
                        let i = app.menuIndex;
                        for (let k = 0; k < n; k++) {
                            i = Math.max(0, Math.min(n - 1, i + d));
                            if (!app.menuItems[i].sep) break;
                        }
                        if (!app.menuItems[i].sep) app.menuIndex = i;
                    };
                    if (up) step(-1);
                    else if (down) step(1);
                    else if (e.key === Qt.Key_Home) { app.menuIndex = 0; step(1); step(-1); }
                    else if (ok || e.key === Qt.Key_Right) app.runMenu(app.menuItems[app.menuIndex].id);
                    else if (back || e.key === Qt.Key_Left) app.closeOverlay();
                    else return;
                } else if (app.overlay === "confirm") {
                    // Buttons left to right: Cancel (0), the third one if there is one (2), OK (1).
                    const order = app.dlgAlt ? [0, 2, 1] : [0, 1];
                    const at = Math.max(0, order.indexOf(app.dlgButton));
                    if (e.key === Qt.Key_Left || up) app.dlgButton = order[Math.max(0, at - 1)];
                    else if (e.key === Qt.Key_Right || down) app.dlgButton = order[Math.min(order.length - 1, at + 1)];
                    else if (ok) { if (app.dlgButton === 1) app.accept(""); else if (app.dlgButton === 2) app.acceptAlt(); else app.closeOverlay(); }
                    else if (back) app.closeOverlay();
                    else return;
                } else if (app.overlay === "conflict") {
                    const c = app.cf;
                    const many = c && c.list.length - c.i > 1;
                    if (e.key === Qt.Key_Left) app.cf = Object.assign({}, c, { button: Math.max(0, Math.min(2, c.button) - 1) });
                    else if (e.key === Qt.Key_Right) app.cf = Object.assign({}, c, { button: Math.min(2, c.button + 1) });
                    else if (down && many) app.cf = Object.assign({}, c, { button: 3 });
                    else if (up && c.button === 3) app.cf = Object.assign({}, c, { button: 1 });
                    else if (ok) {
                        if (c.button === 3) app.cf = Object.assign({}, c, { all: !c.all });
                        else app.conflictChoose(["skip", "rename", "replace"][c.button]);
                    } else if (back) { app.closeOverlay(); app.say("Nothing was copied or moved"); }
                    else return;
                } else if (app.overlay === "apps") {
                    const n = app.dlgApps.length + 1;
                    if (up) app.dlgIndex = Math.max(0, app.dlgIndex - 1);
                    else if (down) app.dlgIndex = Math.min(n - 1, app.dlgIndex + 1);
                    else if (ok) app.openWith(app.dlgIndex >= app.dlgApps.length ? { other: true } : app.dlgApps[app.dlgIndex]);
                    else if (back || e.key === Qt.Key_Left) app.closeOverlay();
                    else return;
                } else if (app.overlay === "preview") {
                    if (e.key === Qt.Key_Left || up) app.previewStep(-1);
                    else if (e.key === Qt.Key_Right || down) app.previewStep(1);
                    else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) { const pe = app.pv ? app.pv.e : null; app.closeOverlay(); app.activate(pe); }
                    else if (back || e.key === Qt.Key_Space) app.closeOverlay();
                    else return;
                } else if (app.overlay === "input") {
                    // The text box has focus; this is only reached if it lost it (a click on the card).
                    if (ok) app.accept(field.text);
                    else if (back) app.closeOverlay();
                    else { field.forceActiveFocus(); return; }
                } else {
                    if (ok || back || e.key === Qt.Key_Left) app.closeOverlay();
                    else return;
                }
                e.accepted = true;
            }

            // Win 11-style: tabs on top, then back / forward / up / refresh, the address and search, then the command
            // bar; below them the navigation pane, the folder and the details pane.
            ColumnLayout {
                anchors.fill: parent
                spacing: 0
                // Tabs, along the top like a title bar (always there; + or Ctrl+T opens one, middle-click closes one).
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: app.picking ? 0 : 46
                    visible: !app.picking
                    color: Theme.bg2
                    Row {
                        anchors { left: parent.left; leftMargin: 10; bottom: parent.bottom }
                        spacing: 2
                        id: tabRow
                        Repeater {
                            id: tabRep
                            model: app.tabs
                            delegate: Rectangle {
                                id: tabChip
                                required property var modelData
                                required property int index
                                readonly property bool on: index === app.tabIndex
                                readonly property bool lit: app.zone === "tabs" && app.tabFocus === index
                                width: Math.min(240, Math.max(150, tabLabel.implicitWidth + 82))
                                height: 38
                                radius: Theme.r(9)
                                color: on ? Theme.bg : tabMouse.containsMouse ? Theme.panel2 : "transparent"
                                // Square bottom corners: the open tab runs into the bar below it.
                                Rectangle { visible: tabChip.on; anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                                    height: parent.radius; color: Theme.bg
                                }
                                PlaceIcon {
                                    anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter }
                                    kind: ({ "hearth:home": "start", "hearth:thispc": "thispc", "hearth:apps": "apps" })[tabChip.on ? app.path : tabChip.modelData.path] || "folder"
                                    tint: tabChip.on ? Theme.accent : Theme.muted
                                    size: 16
                                }
                                border.width: 2
                                border.color: lit ? Theme.accent : "transparent"
                                Text {
                                    id: tabLabel
                                    anchors { left: parent.left; leftMargin: 38; right: tabX.left; rightMargin: 6; verticalCenter: parent.verticalCenter }
                                    text: app.titleOf(tabChip.on ? app.path : tabChip.modelData.path)
                                    color: Theme.text
                                    elide: Text.ElideMiddle
                                    font.family: Theme.font; font.pixelSize: 13; font.weight: tabChip.on ? Font.DemiBold : Font.Normal
                                }
                                readonly property string folder: on ? app.path : modelData.path
                                MouseArea {
                                    id: tabMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                                    property real pressX: 0
                                    property bool moving: false
                                    onPressed: mouse => { pressX = mouse.x; moving = false; }
                                    // Drag a tab left or right to reorder the tabs.
                                    onPositionChanged: mouse => {
                                        if (!(mouse.buttons & Qt.LeftButton)) return;
                                        if (!moving && Math.abs(mouse.x - pressX) < 12) return;
                                        moving = true;
                                        const rx = tabChip.mapToItem(tabRow, mouse.x, 0).x;
                                        for (let k = 0; k < tabRep.count; k++) {
                                            const it = tabRep.itemAt(k);
                                            if (it && rx >= it.x && rx < it.x + it.width && k !== tabChip.index) {
                                                app.moveTab(tabChip.index, k);
                                                break;
                                            }
                                        }
                                    }
                                    onClicked: mouse => {
                                        keys.forceActiveFocus();
                                        if (moving) return;
                                        if (mouse.button === Qt.MiddleButton) app.closeTab(tabChip.index); else app.switchTab(tabChip.index);
                                    }
                                }
                                // Files dropped on a tab go into its folder; holding them over it opens the tab.
                                DropArea {
                                    id: tabDrop
                                    anchors.fill: parent
                                    keys: ["text/uri-list"]
                                    readonly property bool real: tabChip.folder.startsWith("/") && tabChip.folder !== app.homePage
                                    onEntered: drag => { if (!real) drag.accepted = false; else tabHold.restart(); }
                                    onExited: tabHold.stop()
                                    onDropped: drop => { tabHold.stop(); app.dropOn(drop, tabChip.folder); }
                                    Timer { id: tabHold; interval: 800; onTriggered: app.switchTab(tabChip.index) }
                                }
                                Rectangle {
                                    anchors.fill: parent; radius: parent.radius
                                    visible: tabDrop.containsDrag
                                    color: "transparent"; border.width: 2; border.color: Theme.accent
                                }
                                Text {
                                    id: tabX
                                    anchors { right: parent.right; rightMargin: 12; verticalCenter: parent.verticalCenter }
                                    text: "✕"; color: Theme.muted; font.pixelSize: 12
                                    MouseArea { anchors.fill: parent; anchors.margins: -6; onClicked: { keys.forceActiveFocus(); app.closeTab(tabChip.index); } }
                                }
                            }
                        }
                        Rectangle {
                            width: 34; height: 34; radius: Theme.r(9)
                            anchors.verticalCenter: parent.verticalCenter
                            color: newTabMouse.containsMouse ? Theme.panel2 : "transparent"
                            border.width: 2
                            border.color: app.zone === "tabs" && app.tabFocus >= app.tabs.length ? Theme.accent : "transparent"
                            Text { anchors.centerIn: parent; text: "+"; color: Theme.text; font.pixelSize: 18 }
                            MouseArea { id: newTabMouse; anchors.fill: parent; hoverEnabled: true; onClicked: { keys.forceActiveFocus(); app.newTab(app.startPage); } }
                        }
                    }
                }

                // Back, forward, up, the path (click the empty part, Ctrl+L or / to type one), search, view, sort, hidden.
                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: 14
                    Layout.bottomMargin: 8
                    spacing: 6
                    Chip { icon: "back"; label: ""; flat: true; dim: !app.back.length; focused: app.toolLit("back"); onClicked: { keys.forceActiveFocus(); app.goBack(); } }
                    Chip { icon: "forward"; label: ""; flat: true; dim: !app.fwd.length; focused: app.toolLit("forward"); onClicked: { keys.forceActiveFocus(); app.goForward(); } }
                    Chip { icon: "chevu"; label: ""; flat: true; dim: app.path === "/" || app.onHome; focused: app.toolLit("up"); onClicked: { keys.forceActiveFocus(); app.goUp(false); } }
                    Chip { icon: "sync"; label: ""; flat: true; focused: app.toolLit("refresh"); onClicked: { keys.forceActiveFocus(); app.command("refresh"); } }
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 40
                        radius: Theme.r(10)
                        color: Theme.panel2
                        border.width: 2
                        border.color: app.editingPath || app.toolLit("edit") ? Theme.accent : "transparent"
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.IBeamCursor
                            onClicked: win.editPath()
                        }
                        TextInput {
                            id: pathField
                            visible: app.editingPath
                            anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                            verticalAlignment: TextInput.AlignVCenter
                            color: Theme.text
                            selectionColor: Theme.accent
                            selectedTextColor: Theme.accentInk
                            font.family: Theme.font
                            font.pixelSize: 14
                            selectByMouse: true
                            clip: true
                            onActiveFocusChanged: if (!activeFocus && app.editingPath) win.endEditPath(false)
                            Keys.onPressed: e => {
                                switch (e.key) {
                                case Qt.Key_Return: case Qt.Key_Enter: win.endEditPath(true); break;
                                case Qt.Key_Escape: case Qt.Key_Down: win.endEditPath(false); app.zone = "main"; break;
                                case Qt.Key_Up: break;
                                default: return;
                                }
                                e.accepted = true;
                            }
                        }
                        Flickable {
                            id: crumbFlick
                            visible: !app.editingPath
                            anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
                            width: Math.min(parent.width, crumbRow.implicitWidth)
                            contentWidth: crumbRow.implicitWidth
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            flickableDirection: Flickable.HorizontalFlick
                            onContentWidthChanged: contentX = Math.max(0, contentWidth - width)
                            onWidthChanged: contentX = Math.max(0, contentWidth - width)
                            Row {
                                id: crumbRow
                                spacing: 2
                                Repeater {
                                    model: app.crumbs
                                    delegate: Row {
                                        id: crumb
                                        required property var modelData
                                        required property int index
                                        spacing: 2
                                        Text {
                                            visible: crumb.index > 0
                                            anchors.verticalCenter: parent.verticalCenter
                                            text: "›"; color: Theme.muted; font.pixelSize: 18
                                        }
                                        Chip {
                                            label: crumb.modelData.label
                                            strong: crumb.index === app.crumbs.length - 1
                                            focused: app.zone === "toolbar" && !!app.toolItems[app.toolIndex] && app.toolItems[app.toolIndex].t === "crumb" && app.toolItems[app.toolIndex].path === crumb.modelData.path
                                            flat: true
                                            onClicked: { keys.forceActiveFocus(); app.navigate(crumb.modelData.path, ""); }
                                            DropArea {
                                                anchors.fill: parent
                                                keys: ["text/uri-list"]
                                                onDropped: drop => app.dropOn(drop, crumb.modelData.path)
                                            }
                                            onFocusedChanged: if (focused) {
                                                const x = mapToItem(crumbRow, 0, 0).x;
                                                if (x < crumbFlick.contentX) crumbFlick.contentX = x;
                                                else if (x + width > crumbFlick.contentX + crumbFlick.width) crumbFlick.contentX = x + width - crumbFlick.width;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Rectangle {
                        id: searchBox
                        readonly property bool focused: app.toolLit("search")
                        Layout.preferredWidth: win.width < 1000 ? 170 : 240
                        Layout.preferredHeight: 40
                        radius: Theme.r(10)
                        color: Theme.panel2
                        border.width: 2
                        border.color: search.activeFocus || focused ? Theme.accent : "transparent"
                        Behavior on border.color { ColorAnimation { duration: Theme.quick } }
                        TextInput {
                            id: search
                            anchors { fill: parent; leftMargin: 14; rightMargin: 30 }
                            verticalAlignment: TextInput.AlignVCenter
                            color: Theme.text
                            selectionColor: Theme.accent
                            selectedTextColor: Theme.accentInk
                            font.family: Theme.font
                            font.pixelSize: 14
                            clip: true
                            onTextChanged: { if (app.query !== text) { if (app.found !== null && text) app.clearSearch(); app.query = text; app.cur = 0; } }
                            Keys.onPressed: e => {
                                switch (e.key) {
                                case Qt.Key_Return: case Qt.Key_Enter:
                                    // Enter: look in the folders inside too.
                                    if (search.text.trim()) app.runSearch(search.text);
                                    app.zone = "main";
                                    keys.forceActiveFocus();
                                    break;
                                case Qt.Key_Down: case Qt.Key_Escape:
                                    app.zone = "main";
                                    keys.forceActiveFocus();
                                    break;
                                case Qt.Key_Up: break;
                                default: return;
                                }
                                e.accepted = true;
                            }
                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                visible: !search.text
                                text: app.found !== null ? "Found “" + app.foundQuery + "”" : app.onHome ? "Search recent files" : "Search " + app.titleOf(app.path)
                                color: Theme.muted
                                elide: Text.ElideRight
                                width: parent.width
                                font: search.font
                            }
                        }
                        Text {
                            visible: search.text !== "" || app.found !== null
                            anchors { right: parent.right; rightMargin: 12; verticalCenter: parent.verticalCenter }
                            text: "✕"; color: Theme.muted; font.pixelSize: 13
                            MouseArea { anchors.fill: parent; anchors.margins: -8; onClicked: { app.clearSearch(); keys.forceActiveFocus(); } }
                        }
                        MouseArea {
                            anchors.fill: parent
                            anchors.rightMargin: 30
                            cursorShape: Qt.IBeamCursor
                            onClicked: win.focusSearch()
                        }
                    }
                }

                // ------------------------------------------------ the command bar: New, Cut, Copy, Paste, Rename, Copy path,
                // Delete | Sort, View, Show | Undo, Redo, Details. Up from the files reaches it with the controller.
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 52
                    color: Theme.bg
                    RowLayout {
                        id: cmdRow
                        anchors { fill: parent; leftMargin: 12; rightMargin: 12; bottomMargin: 8 }
                        spacing: 2
                        readonly property bool narrow: win.width < 1240
                        Repeater {
                            model: [
                                { t: "new", icon: "plus", label: "New", more: true }, { sep: true },
                                { t: "cut", icon: "", label: "Cut" }, { t: "copy", icon: "file", label: "Copy" },
                                { t: "paste", icon: "clipboard", label: "Paste" }, { t: "rename", icon: "edit", label: "Rename" },
                                { t: "copypath", icon: "link", label: "Copy path" }, { t: "delete", icon: "trash", label: "Delete" },
                                { sep: true },
                                { t: "sort", icon: "menu", label: "Sort", more: true }, { t: "view", icon: "grid", label: "View", more: true },
                                { t: "hidden", icon: "eye", label: "Show", more: true },
                                { sep: true, bc: true },
                                { t: "bc-update-all", icon: "sync", label: "Update All", bc: true }, { t: "bc-unlock", icon: "lock", label: "Unlock", bc: true },
                                { t: "bc-lock", icon: "lock", label: "Lock", bc: true }, { t: "bc-customize", icon: "bc-briefcase", label: "Customize", bc: true },
                                { fill: true },
                                { t: "undo", icon: "undo", label: "Undo" }, { t: "redo", icon: "", label: "Redo" },
                                { sep: true }, { t: "details", icon: "info", label: "Details" }
                            ]
                            delegate: Item {
                                id: cmd
                                required property var modelData
                                readonly property bool on: {
                                    const t = modelData.t;
                                    if (t === "cut" || t === "copy" || t === "delete" || t === "copypath") return app.targets().length > 0 || (t === "copypath" && !app.onHome && !app.onApps);
                                    if (t === "rename") return !!app.current && !app.onSpecial;
                                    if (t === "new" || t === "paste") return app.writable;
                                    if (t === "undo") return app.undoStack.length > 0;
                                    if (t === "redo") return app.redoStack.length > 0;
                                    return true;
                                }
                                // Choosing a file for an app: only Sort, View and Show.
                                visible: (!app.picking || !!modelData.fill || ["sort", "view", "hidden"].indexOf(modelData.t) >= 0)
                                    && (!modelData.bc || (modelData.t ? briefcase.toolShown(modelData.t) : briefcase.inBriefcase))
                                Layout.fillWidth: !!modelData.fill
                                implicitWidth: modelData.sep ? 13 : modelData.fill ? 0 : cmdChip.implicitWidth
                                implicitHeight: 40
                                Rectangle {
                                    visible: !!cmd.modelData.sep
                                    anchors.centerIn: parent
                                    width: 1; height: 24
                                    color: Theme.line
                                }
                                Chip {
                                    id: cmdChip
                                    visible: !!cmd.modelData.t
                                    anchors.fill: parent
                                    flat: true
                                    icon: cmd.modelData.icon || ""
                                    label: cmdRow.narrow && cmd.modelData.icon ? "" : (cmd.modelData.label || "") + (cmd.modelData.more && !cmdRow.narrow ? "  ⌄" : "")
                                    dim: !cmd.on
                                    dot: (cmd.modelData.t === "details" && app.detailsPane) || (cmd.modelData.t === "hidden" && app.hidden)
                                    focused: !!cmd.modelData.t && app.toolLit(cmd.modelData.t)
                                    onClicked: {
                                        keys.forceActiveFocus();
                                        const p = cmdChip.mapToItem(keys, 0, cmdChip.height + 4);
                                        app.command(cmd.modelData.t, p.x, p.y);
                                    }
                                }
                            }
                        }
                    }
                }
                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 0

                    // ------------------------------------------------ places
                    // The narrow Windows 11 left menu (1.11, option A): 158 px to start, drag its edge to resize
                    // (120–360 px, remembered in files.json as navWidth when you let go).
                    Rectangle {
                        id: navPane
                        visible: app.navShown
                        Layout.fillHeight: true
                        Layout.preferredWidth: app.navW
                        color: Theme.panel
                        MouseArea {
                            id: navGrip
                            z: 5
                            anchors { top: parent.top; bottom: parent.bottom; right: parent.right; rightMargin: -3 }
                            width: 6
                            cursorShape: Qt.SizeHorCursor
                            property real startX: 0
                            property real startW: 0
                            onPressed: m => { startX = mapToItem(null, m.x, 0).x; startW = app.navW; }
                            onPositionChanged: m => { if (pressed) app.navW = Math.max(120, Math.min(360, startW + mapToItem(null, m.x, 0).x - startX)); }
                            onReleased: app.saveNavWidth()
                            Rectangle { anchors.fill: parent; color: navGrip.containsMouse || navGrip.pressed ? Theme.accent : "transparent"; opacity: 0.5 }
                            hoverEnabled: true
                        }
                        ColumnLayout {
                            anchors { fill: parent; margins: 6; topMargin: 10 }
                            spacing: 2
                            ListView {
                                id: placeList
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                model: app.places
                                spacing: 1
                                clip: true
                                boundsBehavior: Flickable.StopAtBounds
                                keyNavigationEnabled: false
                                delegate: Item {
                                    id: placeItem
                                    required property var modelData
                                    required property int index
                                    // A heading where a new group starts (Bookmarks, Games, Drives and SD cards; a gap before Trash).
                                    readonly property bool starts: index > 0 && app.places[index - 1].section !== modelData.section
                                    readonly property string sec: modelData.section
                                    // A heading for your folders, Games, Favorites and Network's groups; a gap before This PC and Trash;
                                    // nothing between This PC's drives, Network and Installed apps (a tree, as in Windows).
                                    // Windows 11: no headings, a thin line between Home's group, your folders and This PC.
                                    readonly property real headH: starts ? 13 : 0
                                    width: ListView.view.width
                                    height: headH + place.height
                                    Rectangle {
                                        visible: placeItem.headH > 0
                                        anchors { left: parent.left; right: parent.right; leftMargin: 10; rightMargin: 10; top: parent.top; topMargin: 6 }
                                        height: 1; color: Theme.line
                                    }
                                    Rectangle {
                                        id: place
                                        readonly property var modelData: placeItem.modelData
                                        readonly property int index: placeItem.index
                                        readonly property bool here: !!modelData.path && app.path === modelData.path
                                        readonly property bool lit: app.zone === "places" && app.placeIndex === index
                                        readonly property color ink: lit ? Theme.accentInk : here ? Theme.accent : Theme.text
                                        y: placeItem.headH
                                        width: parent.width
                                        height: 30
                                        radius: Theme.r(6)
                                        color: lit ? Theme.accent : placeDrop.containsDrag ? Theme.panel3 : here ? Theme.panel3 : placeMouse.containsMouse ? Theme.panel2 : "transparent"
                                        border.width: placeDrop.containsDrag ? 2 : 0
                                        border.color: Theme.accent
                                        opacity: modelData.mounted === false || modelData.offline === true ? 0.7 : 1
                                        Behavior on color { ColorAnimation { duration: Theme.quick } }
                                        // The blue bar on the item you're in (Windows 11's navigation pane).
                                        Rectangle {
                                            visible: place.here && !place.lit
                                            anchors { left: parent.left; verticalCenter: parent.verticalCenter }
                                            width: 3; height: 18; radius: 2
                                            color: Theme.accent
                                        }
                                        // This PC's and Network's arrow: folds the tree.
                                        Text {
                                            z: 2
                                            visible: !!place.modelData.tree && !place.modelData.empty
                                            anchors { left: parent.left; leftMargin: 4 + (place.modelData.depth || 0) * 12; verticalCenter: parent.verticalCenter }
                                            text: place.modelData.open ? "⌄" : "›"
                                            color: place.lit ? Theme.accentInk : Theme.muted
                                            font.pixelSize: 12
                                            MouseArea { anchors.fill: parent; anchors.margins: -6; cursorShape: Qt.PointingHandCursor; onClicked: { keys.forceActiveFocus(); app.toggleNav(place.modelData.tree); } }
                                        }
                                        RowLayout {
                                            anchors { fill: parent; leftMargin: 16 + (place.modelData.depth !== undefined ? place.modelData.depth : (place.modelData.child ? 1 : 0)) * 12; rightMargin: 6 }
                                            spacing: 8
                                            PlaceIcon { kind: place.modelData.kind === "briefcase" ? "bc:" + ((place.modelData.look || {}).icon || "briefcase") : place.modelData.kind
                                                        tint: place.modelData.kind === "briefcase" ? briefcase.colorOf(place.modelData.look) : place.ink; size: 16 }
                                            ColumnLayout {
                                                Layout.fillWidth: true
                                                spacing: 1
                                                Text {
                                                    Layout.fillWidth: true
                                                    text: place.modelData.label
                                                    color: place.lit ? Theme.accentInk : Theme.text
                                                    elide: Text.ElideRight
                                                    font.family: Theme.font; font.pixelSize: 13; font.weight: place.here || place.lit ? Font.DemiBold : Font.Normal
                                                }
                                                Text {
                                                    Layout.fillWidth: true
                                                    visible: place.modelData.section === "drives" || (place.modelData.section === "devices" && !!place.modelData.sub)
                                                    text: place.modelData.section === "devices" ? (place.modelData.sub || "") : place.modelData.mounted === false ? (place.modelData.sub || "Not mounted")
                                                        : (place.modelData.sub || "") + (place.modelData.free !== undefined && place.modelData.free !== null ? " · " + app.fmtSize(place.modelData.free) + " free" : "")
                                                    color: place.lit ? Theme.accentInk : Theme.muted
                                                    elide: Text.ElideRight
                                                    font.family: Theme.font; font.pixelSize: 11
                                                }
                                                Rectangle {
                                                    Layout.fillWidth: true
                                                    Layout.preferredHeight: 3
                                                    visible: place.modelData.section === "drives" && !!place.modelData.total && place.modelData.free !== undefined && place.modelData.free !== null
                                                    radius: 2
                                                    color: place.lit ? Qt.rgba(0, 0, 0, 0.2) : Theme.panel3
                                                    Rectangle {
                                                        width: place.modelData.total ? parent.width * Math.max(0, Math.min(1, 1 - (place.modelData.free || 0) / place.modelData.total)) : 0
                                                        height: parent.height
                                                        radius: 2
                                                        color: place.lit ? Theme.accentInk : Theme.accent
                                                    }
                                                }
                                            }
                                            Text {
                                                visible: place.modelData.kind === "trash" && place.modelData.count > 0
                                                text: place.modelData.count || ""
                                                color: place.lit ? Theme.accentInk : Theme.muted
                                                font.family: Theme.font; font.pixelSize: 12
                                            }
                                        }
                                        DropArea {
                                            id: placeDrop
                                            anchors.fill: parent
                                            enabled: !!place.modelData.path
                                            keys: ["text/uri-list"]
                                            onDropped: drop => app.dropOn(drop, place.modelData.path)
                                        }
                                        MouseArea {
                                            id: placeMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: mouse => {
                                                keys.forceActiveFocus();
                                                app.placeIndex = place.index;
                                                if (mouse.button === Qt.RightButton) {
                                                    app.zone = "places";
                                                    app.viaPad = false;
                                                    app.openPlaceMenu();
                                                } else if (mouse.button === Qt.MiddleButton) {
                                                    if (place.modelData.path) app.newTab(place.modelData.path);
                                                } else {
                                                    app.openPlace(place.index);
                                                }
                                            }
                                            onPressAndHold: { keys.forceActiveFocus(); app.placeIndex = place.index; app.zone = "places"; app.openPlaceMenu(); }
                                        }
                                    }
                                }
                            }
                            Text {
                                Layout.fillWidth: true
                                Layout.leftMargin: 6
                                text: "Ⓐ open  Ⓑ back  Ⓧ actions  Ⓨ type"
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                                elide: Text.ElideRight
                            }
                        }
                    }
                    Rectangle { Layout.fillHeight: true; Layout.preferredWidth: 1; color: Theme.line }

                    // ------------------------------------------------ folder
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: 0

                        // The folder itself.
                        Item {
                            id: folderArea
                            Layout.fillWidth: true
                            Layout.fillHeight: true

                            WheelHandler {
                                acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
                                enabled: app.mouseMode
                                target: null
                                onWheel: event => win.wheel(event)
                            }
                            // Dropped on the folder (not on a folder in it): into this folder.
                            DropArea {
                                anchors.fill: parent
                                keys: ["text/uri-list"]
                                enabled: app.writable
                                onDropped: drop => app.dropOn(drop, app.listing.path)
                            }
                            MouseArea {
                                // Empty space: click clears the selection, drag picks with a rubber band (mouse),
                                // right-click / press-and-hold for the folder's actions.
                                id: bgMouse
                                anchors.fill: parent
                                acceptedButtons: Qt.LeftButton | Qt.RightButton
                                property point start
                                property var startMarks: ({})
                                property bool banding: false
                                function contentPoint(x, y) {
                                    const v = app.shownView === "details" ? list : grid;
                                    return v.contentItem.mapFromItem(bgMouse, x, y);
                                }
                                onPressed: mouse => {
                                    keys.forceActiveFocus();
                                    app.zone = "main";
                                    if (mouse.button !== Qt.LeftButton || !app.mouseMode || app.onSpecial) return;
                                    start = contentPoint(mouse.x, mouse.y);
                                    startMarks = (mouse.modifiers & Qt.ControlModifier) ? Object.assign({}, app.marked) : ({});
                                    banding = false;
                                }
                                onPositionChanged: mouse => {
                                    if (!(mouse.buttons & Qt.LeftButton) || !app.mouseMode) return;
                                    const p = contentPoint(mouse.x, mouse.y);
                                    if (!banding && Math.abs(p.x - start.x) + Math.abs(p.y - start.y) < 8) return;
                                    banding = true;
                                    win.band(start, p, startMarks);
                                }
                                onReleased: mouse => {
                                    if (banding) {
                                        banding = false;
                                        bandRect.visible = false;
                                        return;
                                    }
                                    if (mouse.button === Qt.LeftButton && !(mouse.modifiers & (Qt.ControlModifier | Qt.ShiftModifier))) app.marked = ({});
                                }
                                onClicked: mouse => {
                                    if (mouse.button === Qt.RightButton) {
                                        const p = mapToItem(keys, mouse.x, mouse.y);
                                        app.viaPad = false;
                                        app.marked = ({});
                                        app.openMenu(true, p.x, p.y);
                                    }
                                }
                                onPressAndHold: mouse => {
                                    if (banding) return;
                                    const p = mapToItem(keys, mouse.x, mouse.y);
                                    keys.forceActiveFocus();
                                    app.openMenu(true, p.x, p.y);
                                }
                            }

                            // The Home page: Quick access (your usual folders, Favorites and Games) above the recent files.
                            // Click opens, middle-click opens in a new tab, drop files on one to put them there.
                            Column {
                                id: quick
                                visible: app.onHome
                                anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 14; rightMargin: 14 }
                                height: visible ? implicitHeight : 0
                                spacing: 10
                                topPadding: 14
                                bottomPadding: 6
                                Text { text: "Quick access"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                                Flow {
                                    width: parent.width
                                    spacing: 8
                                    Repeater {
                                        model: app.quickAccess
                                        delegate: Rectangle {
                                            id: qa
                                            required property var modelData
                                            width: 196; height: 58
                                            radius: Theme.r(10)
                                            color: qaDrop.containsDrag ? Theme.panel3 : qaMouse.containsMouse ? Theme.panel3 : Theme.panel2
                                            border.width: qaDrop.containsDrag ? 2 : 0
                                            border.color: Theme.accent
                                            RowLayout {
                                                anchors { fill: parent; leftMargin: 14; rightMargin: 12 }
                                                spacing: 12
                                                PlaceIcon { kind: qa.modelData.kind; tint: Theme.accent; size: 24 }
                                                ColumnLayout {
                                                    Layout.fillWidth: true
                                                    spacing: 1
                                                    Text { Layout.fillWidth: true; text: qa.modelData.label; color: Theme.text; elide: Text.ElideRight; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                                                    Text {
                                                        Layout.fillWidth: true
                                                        text: qa.modelData.section === "bookmarks" ? "Favorite" : qa.modelData.section === "games" ? "Games" : "On this device"
                                                        color: Theme.muted; elide: Text.ElideRight; font.family: Theme.font; font.pixelSize: 11
                                                    }
                                                }
                                            }
                                            DropArea { id: qaDrop; anchors.fill: parent; keys: ["text/uri-list"]; onDropped: drop => app.dropOn(drop, qa.modelData.path) }
                                            MouseArea {
                                                id: qaMouse
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: mouse => { keys.forceActiveFocus(); if (mouse.button === Qt.MiddleButton) app.newTab(qa.modelData.path); else app.navigate(qa.modelData.path, ""); }
                                            }
                                        }
                                    }
                                }
                                Text { topPadding: 8; text: "Recent"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                            }

                            // Icons and Compact: a grid (Compact: small icons in columns).
                            GridView {
                                id: grid
                                visible: app.shownView !== "details" && !app.onSpecial
                                anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                                model: visible ? app.entries : []       // the hidden view makes no rows (1.11: big folders)
                                clip: true
                                interactive: !app.mouseMode
                                boundsBehavior: Flickable.StopAtBounds
                                // Large, medium and small icons; List (small icons in columns); Tiles (an icon and two lines).
                                readonly property int baseW: ({ xlarge: 300, large: 200, grid: 136, small: 100, compact: 250, tiles: 300 })[app.view] || 136
                                cellWidth: Math.floor(width / Math.max(1, Math.floor(width / baseW)))
                                cellHeight: ({ xlarge: 320, large: 224, grid: 156, small: 112, compact: 44, tiles: 78 })[app.view] || 156
                                keyNavigationEnabled: false
                                cacheBuffer: 600
                                delegate: Item {
                                    id: cell
                                    required property var modelData
                                    required property int index
                                    readonly property bool lit: app.cur === index
                                    readonly property bool picked: !!app.marked[modelData.path]
                                    readonly property bool cut: app.clip.mode === "cut" && app.clip.paths.indexOf(modelData.path) >= 0
                                    readonly property bool compact: app.view === "compact" || app.view === "tiles"
                                    readonly property bool tiles: app.view === "tiles"
                                    readonly property real artW: app.view === "xlarge" ? 250 : app.view === "large" ? 150 : app.view === "small" ? 54 : 84
                                    readonly property real artH: app.view === "xlarge" ? 226 : app.view === "large" ? 134 : app.view === "small" ? 50 : 76
                                    width: grid.cellWidth
                                    height: grid.cellHeight
                                    opacity: cut ? 0.45 : 1
                                    Rectangle {
                                        anchors { fill: parent; margins: cell.compact ? 2 : 4 }
                                        radius: cell.compact ? 8 : 12
                                        color: cellDrop.containsDrag ? Theme.panel3 : cell.picked ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22)
                                             : cell.lit ? Theme.panel3 : cellMouse.containsMouse ? Theme.panel2 : "transparent"
                                        border.width: 2
                                        border.color: cellDrop.containsDrag || app.dragHint === cell.modelData.path || (cell.lit && app.zone === "main" && app.overlay === "") ? Theme.accent : "transparent"
                                        Behavior on border.color { ColorAnimation { duration: Theme.quick } }
                                    }
                                    // Icons
                                    Item {
                                        id: art
                                        visible: !cell.compact
                                        anchors { top: parent.top; topMargin: app.view === "small" ? 10 : 14; horizontalCenter: parent.horizontalCenter }
                                        width: cell.artW; height: cell.artH
                                        FileGlyph {
                                            anchors.centerIn: parent
                                            visible: thumb.status !== Image.Ready
                                            kind: app.kindOf(cell.modelData)
                                            label: app.extLabel(cell.modelData)
                                            link: cell.modelData.isLink
                                            bc: cell.modelData.briefcase || null
                                            bcColor: briefcase.colorOf(cell.modelData.briefcase)
                                            badge: briefcase.badgeFor(cell.modelData)
                                            size: app.view === "xlarge" ? 180 : app.view === "large" ? 112 : app.view === "small" ? 42 : 64
                                        }
                                        Rectangle {
                                            anchors.fill: parent
                                            visible: thumb.status === Image.Ready
                                            radius: Theme.r(8)
                                            color: Theme.panel3
                                            Image {
                                                id: thumb
                                                anchors { fill: parent; margins: 3 }
                                                source: cell.compact ? "" : app.thumbFor(cell.modelData)
                                                sourceSize: app.view === "xlarge" ? Qt.size(512, 512) : app.view === "large" ? Qt.size(320, 320) : Qt.size(192, 192)
                                                asynchronous: true
                                                cache: true
                                                fillMode: Image.PreserveAspectCrop
                                                smooth: true
                                                clip: true
                                            }
                                        }
                                    }
                                    Text {
                                        visible: !cell.compact
                                        anchors { top: art.bottom; topMargin: 8; left: parent.left; right: parent.right; leftMargin: 10; rightMargin: 10 }
                                        horizontalAlignment: Text.AlignHCenter
                                        text: app.shownName(cell.modelData)
                                        color: cell.modelData.hidden ? Theme.muted : Theme.text
                                        wrapMode: Text.WrapAnywhere
                                        maximumLineCount: 2
                                        elide: Text.ElideRight
                                        font.family: Theme.font; font.pixelSize: app.view === "small" ? 12 : 13; font.weight: cell.lit ? Font.DemiBold : Font.Normal
                                        Loader {
                                            anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: -6; rightMargin: -6; topMargin: -3 }
                                            height: 26
                                            active: parent.visible && app.renaming !== "" && app.renaming === cell.modelData.path
                                            sourceComponent: inlineRename
                                        }
                                    }
                                    // Compact
                                    RowLayout {
                                        visible: cell.compact
                                        anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                                        spacing: 10
                                        Item {
                                            Layout.preferredWidth: cell.tiles ? 54 : 26; Layout.preferredHeight: cell.tiles ? 54 : 26
                                            FileGlyph {
                                                anchors.centerIn: parent
                                                visible: smallThumb.status !== Image.Ready
                                                kind: app.kindOf(cell.modelData)
                                                label: cell.tiles ? app.extLabel(cell.modelData) : ""
                                                link: cell.modelData.isLink
                                                bc: cell.modelData.briefcase || null
                                                bcColor: briefcase.colorOf(cell.modelData.briefcase)
                                                badge: briefcase.badgeFor(cell.modelData)
                                                size: cell.tiles ? 48 : 24
                                            }
                                            Image {
                                                id: smallThumb
                                                anchors.fill: parent
                                                source: cell.compact && (cell.tiles || app.kindOf(cell.modelData) === "image") ? app.thumbFor(cell.modelData) : ""
                                                sourceSize: cell.tiles ? Qt.size(108, 108) : Qt.size(52, 52)
                                                asynchronous: true
                                                fillMode: Image.PreserveAspectCrop
                                                clip: true
                                            }
                                        }
                                        ColumnLayout {
                                            Layout.fillWidth: true
                                            spacing: 2
                                            Text {
                                                Layout.fillWidth: true
                                                text: app.shownName(cell.modelData) + (!cell.tiles && cell.modelData.where && cell.modelData.where !== "." ? "  ·  " + cell.modelData.where : "")
                                                color: cell.modelData.hidden ? Theme.muted : Theme.text
                                                elide: Text.ElideMiddle
                                                font.family: Theme.font; font.pixelSize: cell.tiles ? 14 : 13; font.weight: cell.lit ? Font.DemiBold : Font.Normal
                                                Loader {
                                                    anchors { fill: parent; margins: -4 }
                                                    active: parent.visible && app.renaming !== "" && app.renaming === cell.modelData.path
                                                    sourceComponent: inlineRename
                                                }
                                            }
                                            Text {
                                                Layout.fillWidth: true
                                                visible: cell.tiles
                                                text: app.kindName(cell.modelData) + (cell.modelData.dir ? "" : "  ·  " + app.fmtSize(cell.modelData.size))
                                                color: Theme.muted
                                                elide: Text.ElideRight
                                                font.family: Theme.font; font.pixelSize: 12
                                            }
                                        }
                                    }
                                    Rectangle {
                                        visible: cell.picked
                                        anchors { top: parent.top; right: parent.right; margins: cell.compact ? 12 : 10 }
                                        width: 22; height: 22; radius: 11
                                        color: Theme.accent
                                        Icon { anchors.centerIn: parent; name: "check"; size: 14; color: Theme.accentInk }
                                    }
                                    DropArea {
                                        id: cellDrop
                                        anchors.fill: parent
                                        enabled: cell.modelData.dir && !app.inTrash
                                        keys: ["text/uri-list"]
                                        onDropped: drop => app.dropOn(drop, cell.modelData.path)
                                    }
                                    MouseArea {
                                        id: cellMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                                        drag.target: app.mouseMode ? dragProxy : null
                                        drag.threshold: 10
                                        drag.onActiveChanged: win.itemDragged(cell.index, drag.active)
                                        onClicked: mouse => win.itemClicked(cell.index, mouse, cellMouse)
                                        onDoubleClicked: mouse => { if (mouse.button === Qt.LeftButton) app.activate(cell.modelData); }
                                        onPressAndHold: mouse => win.itemMenu(cell.index, mouse, cellMouse)
                                    }
                                }
                            }

                            // Details: sortable columns, groups with a heading.
                            ListView {
                                id: list
                                visible: app.shownView === "details" && !app.onSpecial
                                anchors { fill: parent; leftMargin: 14; rightMargin: 14; topMargin: app.onHome ? quick.height : 0 }
                                model: visible ? app.entries : []       // the hidden view makes no rows (1.11: big folders)
                                clip: true
                                interactive: !app.mouseMode
                                spacing: 2
                                boundsBehavior: Flickable.StopAtBounds
                                keyNavigationEnabled: false
                                headerPositioning: ListView.OverlayHeader
                                readonly property bool wide: width > 760
                                header: Rectangle {
                                    width: ListView.view.width
                                    height: 34
                                    z: 2
                                    color: Theme.bg
                                    RowLayout {
                                        anchors { fill: parent; leftMargin: 56; rightMargin: 16 }
                                        spacing: 12
                                        ColumnHead { Layout.fillWidth: true; label: app.found !== null ? "Name · where" : "Name"; key: "name"; sortKey: app.sort; desc: app.desc; onPicked: k => app.setSort(k) }
                                        ColumnHead { Layout.preferredWidth: 190; visible: briefcase.inBriefcase; label: "Status"; key: "status"; sortKey: app.sort; desc: app.desc }
                                        ColumnHead { Layout.preferredWidth: 100; label: "Size"; key: "size"; sortKey: app.sort; desc: app.desc; alignRight: true; onPicked: k => app.setSort(k) }
                                        ColumnHead { Layout.preferredWidth: 150; visible: list.wide; label: "Type"; key: "type"; sortKey: app.sort; desc: app.desc; onPicked: k => app.setSort(k) }
                                        ColumnHead { Layout.preferredWidth: 160; label: "Modified"; key: "date"; sortKey: app.sort; desc: app.desc; onPicked: k => app.setSort(k) }
                                    }
                                    Rectangle {
                                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                                        height: 1
                                        color: Theme.line
                                    }
                                }
                                delegate: Item {
                                    id: rowItem
                                    required property var modelData
                                    required property int index
                                    readonly property string groupLabel: app.group === "none" ? "" : app.groupName(modelData)
                                    readonly property bool groupStarts: groupLabel !== "" && (index === 0 || app.groupName(app.entries[index - 1]) !== groupLabel)
                                    width: ListView.view.width
                                    height: (groupStarts ? 34 : 0) + 44
                                    Text {
                                        visible: rowItem.groupStarts
                                        anchors { left: parent.left; leftMargin: 12; top: parent.top; topMargin: 10 }
                                        text: rowItem.groupLabel
                                        color: Theme.accent; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold
                                    }
                                    Rectangle {
                                        id: row
                                        readonly property bool lit: app.cur === rowItem.index
                                        readonly property bool picked: !!app.marked[rowItem.modelData.path]
                                        readonly property bool cut: app.clip.mode === "cut" && app.clip.paths.indexOf(rowItem.modelData.path) >= 0
                                        y: rowItem.groupStarts ? 34 : 0
                                        width: parent.width
                                        height: 44
                                        radius: Theme.r(10)
                                        opacity: cut ? 0.45 : 1
                                        color: rowDrop.containsDrag ? Theme.panel3 : picked ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22)
                                             : lit ? Theme.panel3 : rowMouse.containsMouse ? Theme.panel2 : "transparent"
                                        border.width: 2
                                        border.color: rowDrop.containsDrag || (lit && app.zone === "main" && app.overlay === "") ? Theme.accent : "transparent"
                                        RowLayout {
                                            anchors { fill: parent; leftMargin: 12; rightMargin: 16 }
                                            spacing: 12
                                            Item {
                                                Layout.preferredWidth: 32; Layout.preferredHeight: 32
                                                FileGlyph {
                                                    anchors.centerIn: parent
                                                    visible: rowThumb.status !== Image.Ready
                                                    kind: app.kindOf(rowItem.modelData)
                                                    label: ""
                                                    link: rowItem.modelData.isLink
                                                    bc: rowItem.modelData.briefcase || null
                                                    bcColor: briefcase.colorOf(rowItem.modelData.briefcase)
                                                    size: 28
                                                }
                                                Image {
                                                    id: rowThumb
                                                    anchors.fill: parent
                                                    source: app.kindOf(rowItem.modelData) === "image" ? app.thumbFor(rowItem.modelData) : ""
                                                    sourceSize: Qt.size(64, 64)
                                                    asynchronous: true
                                                    fillMode: Image.PreserveAspectCrop
                                                    clip: true
                                                }
                                            }
                                            Text {
                                                Layout.fillWidth: true
                                                text: app.shownName(rowItem.modelData) + (rowItem.modelData.origPath ? "   ·   from " + app.parentOf(rowItem.modelData.origPath)
                                                    : rowItem.modelData.where && rowItem.modelData.where !== "." ? "   ·   " + rowItem.modelData.where : "")
                                                color: rowItem.modelData.hidden ? Theme.muted : Theme.text
                                                elide: Text.ElideMiddle
                                                font.family: Theme.font; font.pixelSize: 14; font.weight: row.lit ? Font.DemiBold : Font.Normal
                                                Loader {
                                                    anchors { fill: parent; margins: -4 }
                                                    active: parent.visible && app.renaming !== "" && app.renaming === rowItem.modelData.path
                                                    sourceComponent: inlineRename
                                                }
                                            }
                                            Icon { visible: row.picked; name: "check"; size: 16; color: Theme.accent }
                                            Row {
                                                Layout.preferredWidth: 190
                                                visible: briefcase.inBriefcase
                                                spacing: 6
                                                readonly property var bdg: briefcase.badgeFor(rowItem.modelData)
                                                Rectangle { visible: !!parent.bdg; width: 10; height: 10; radius: 5; color: parent.bdg ? parent.bdg.color : "transparent"; anchors.verticalCenter: parent.verticalCenter }
                                                Text {
                                                    width: 174
                                                    text: briefcase.statusLabel(rowItem.modelData)
                                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                                    elide: Text.ElideRight
                                                }
                                            }
                                            Text {
                                                Layout.preferredWidth: 100
                                                horizontalAlignment: Text.AlignRight
                                                text: rowItem.modelData.dir ? "" : app.fmtSize(rowItem.modelData.size)
                                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                            }
                                            Text {
                                                Layout.preferredWidth: 150
                                                visible: list.wide
                                                text: app.kindName(rowItem.modelData)
                                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                                elide: Text.ElideRight
                                            }
                                            Text {
                                                Layout.preferredWidth: 160
                                                text: app.fmtDate(rowItem.modelData.mtime)
                                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                                elide: Text.ElideRight
                                            }
                                        }
                                        DropArea {
                                            id: rowDrop
                                            anchors.fill: parent
                                            enabled: rowItem.modelData.dir && !app.inTrash
                                            keys: ["text/uri-list"]
                                            onDropped: drop => app.dropOn(drop, rowItem.modelData.path)
                                        }
                                        MouseArea {
                                            id: rowMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                                            drag.target: app.mouseMode ? dragProxy : null
                                            drag.threshold: 10
                                            drag.onActiveChanged: win.itemDragged(rowItem.index, drag.active)
                                            onClicked: mouse => win.itemClicked(rowItem.index, mouse, rowMouse)
                                            onDoubleClicked: mouse => { if (mouse.button === Qt.LeftButton) app.activate(rowItem.modelData); }
                                            onPressAndHold: mouse => win.itemMenu(rowItem.index, mouse, rowMouse)
                                        }
                                    }
                                }
                            }

                            // The rubber band.
                            Rectangle {
                                id: bandRect
                                visible: false
                                color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16)
                                border.width: 1
                                border.color: Theme.accent
                                radius: 4
                            }

                            // This PC and Installed apps, each in its own file behind a Loader: if one can't load, Files
                            // (and Desktop Mode) still open, and only that page stays empty.
                            Loader {
                                id: thisPcLoader
                                anchors.fill: parent
                                visible: app.onThisPc
                                Component.onCompleted: setSource("FilesThisPc.qml", { fm: app, keysItem: keys })
                                onStatusChanged: if (status === Loader.Error) console.warn("Files: This PC couldn't load (FilesThisPc.qml)")
                            }
                            Loader {
                                id: appsLoader
                                anchors.fill: parent
                                visible: app.onApps
                                Component.onCompleted: setSource("FilesApps.qml", { fm: app, keysItem: keys })
                                onStatusChanged: if (status === Loader.Error) console.warn("Files: Installed apps couldn't load (FilesApps.qml)")
                            }
                            // Hearth (C:) with Windows folder names (FilesCDrive.qml): its pages, behind a Loader too.
                            Loader {
                                id: cDriveLoader
                                anchors.fill: parent
                                visible: app.onCView
                                Component.onCompleted: setSource("FilesCDrive.qml", { fm: app, keysItem: keys })
                                onStatusChanged: if (status === Loader.Error) console.warn("Files: Hearth (C:) couldn't load (FilesCDrive.qml)")
                            }
                            Text {
                                anchors.centerIn: parent
                                visible: (app.onThisPc && thisPcLoader.status === Loader.Error) || (app.onApps && appsLoader.status === Loader.Error)
                                         || (app.onCView && cDriveLoader.status === Loader.Error)
                                text: "This page couldn't be shown. Your folders and drives are still in the navigation pane."
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
                            }

                            // Empty, loading, searching, or can't open.
                            Column {
                                anchors.centerIn: parent
                                width: Math.min(parent.width - 80, 460)
                                spacing: 8
                                visible: app.entries.length === 0 && !app.onSpecial
                                Text {
                                    width: parent.width
                                    horizontalAlignment: Text.AlignHCenter
                                    wrapMode: Text.WordWrap
                                    text: app.listing && app.listing.error ? "Can't open this folder"
                                        : app.loading && !app.listing ? "Opening…"
                                        : app.found !== null ? (app.searching ? "Searching for “" + app.foundQuery + "”…" : "Nothing inside matches “" + app.foundQuery + "”")
                                        : app.query ? "Nothing here matches “" + app.query + "”"
                                        : app.onHome ? "No recent files yet" : app.inTrash ? "The Trash is empty" : "This folder is empty"
                                    color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold
                                }
                                Text {
                                    width: parent.width
                                    horizontalAlignment: Text.AlignHCenter
                                    wrapMode: Text.WordWrap
                                    text: app.listing && app.listing.error ? app.listing.error
                                        : app.query ? "Press Enter to look in the folders inside too. Ⓑ clears it."
                                        : app.writable && !app.inTrash && app.found === null ? "Ⓧ for actions: paste, new folder… or drop files here" : ""
                                    visible: text !== ""
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                }
                            }
                        }

                        // ------------------------------------------------ picker bar: name, which files, the app's options, Cancel, Open/Save
                        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line; visible: app.picking }
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: Math.max(40, pickFlow.height) + 24
                            visible: app.picking
                            color: Theme.bg2
                            Flow {
                                id: pickFlow
                                anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 18; topMargin: 12; rightMargin: pickButtons.width + 28 }
                                spacing: 10
                                // Saving: the file's name.
                                Rectangle {
                                    visible: !!app.pick && app.pick.kind === "save"
                                    width: Math.max(200, Math.min(460, pickFlow.width - 200))
                                    height: 40
                                    radius: Theme.r(10)
                                    color: Theme.panel3
                                    border.width: 2
                                    border.color: pickField.activeFocus || app.pickLit("name") ? Theme.accent : "transparent"
                                    Text {
                                        id: nameLabel
                                        anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter }
                                        text: "Name"
                                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                    }
                                    TextInput {
                                        id: pickField
                                        anchors { left: nameLabel.right; leftMargin: 12; right: parent.right; rightMargin: 12; verticalCenter: parent.verticalCenter }
                                        text: app.pickName
                                        onTextEdited: app.pickName = text
                                        color: Theme.text
                                        selectionColor: Theme.accent
                                        selectedTextColor: Theme.accentInk
                                        font.family: Theme.font
                                        font.pixelSize: 15
                                        selectByMouse: true
                                        clip: true
                                        onActiveFocusChanged: if (activeFocus) { app.zone = "pick"; app.pickIndex = 0; }
                                        Keys.onPressed: e => {
                                            switch (e.key) {
                                            case Qt.Key_Return: case Qt.Key_Enter: app.pickAccept(null); break;
                                            case Qt.Key_Escape: app.close(); break;
                                            case Qt.Key_Up: app.zone = "main"; keys.forceActiveFocus(); break;
                                            case Qt.Key_Down: case Qt.Key_Tab: app.pickIndex = Math.min(1, app.pickItems.length - 1); keys.forceActiveFocus(); break;
                                            default: return;
                                            }
                                            e.accepted = true;
                                        }
                                    }
                                }
                                Chip {
                                    visible: !!app.pick && (app.pick.filters || []).length > 0
                                    label: app.pickFilter >= 0 && app.pick ? app.pick.filters[app.pickFilter].name + ((app.pick.filters || []).length > 1 ? "  ▾" : "") : ""
                                    focused: app.pickLit("filter")
                                    onClicked: { keys.forceActiveFocus(); if ((app.pick.filters || []).length > 1) app.filterMenu(); }
                                }
                                Repeater {
                                    model: app.pick ? app.pick.choices || [] : []
                                    delegate: Chip {
                                        required property var modelData
                                        required property int index
                                        label: app.choiceLabel(modelData)
                                        focused: app.pickLit("choice", index)
                                        onClicked: { keys.forceActiveFocus(); app.choiceRun(index); }
                                    }
                                }
                                Text {
                                    visible: !!app.pick && app.pick.kind === "savefiles"
                                    height: 40
                                    verticalAlignment: Text.AlignVCenter
                                    text: app.pick ? app.plural((app.pick.files || []).length, "file") + " go into “" + (app.listing ? app.listing.name || app.base(app.path) : "") + "”" : ""
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                }
                            }
                            Row {
                                id: pickButtons
                                anchors { right: parent.right; rightMargin: 14; bottom: parent.bottom; bottomMargin: 12 }
                                spacing: 10
                                Chip {
                                    label: "Cancel"
                                    focused: app.pickLit("cancel")
                                    onClicked: app.close()
                                }
                                Chip {
                                    primary: true
                                    label: app.pickAcceptLabel
                                    focused: app.pickLit("accept")
                                    dim: !!app.pick && app.pick.kind === "open" && !app.pickTargets().length
                                    onClicked: { keys.forceActiveFocus(); app.pickAccept(null); }
                                }
                            }
                        }

                        // ------------------------------------------------ bottom bar: status, search, or progress
                        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line }
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: app.job ? 66 : 44
                            color: Theme.panel

                            RowLayout {
                                visible: !app.job
                                anchors { fill: parent; leftMargin: 18; rightMargin: 12 }
                                spacing: 12
                                Text {
                                    Layout.fillWidth: true
                                    text: {
                                        if (app.note !== "") return app.note;
                                        let s = app.found !== null ? (app.searching ? "Searching… " : "") + app.plural(app.entries.length, "match", "matches") + " for “" + app.foundQuery + "”"
                                            : app.plural(app.entries.length, app.onApps ? "app" : "item");
                                        if (app.markCount) {
                                            const sel = app.entries.filter(e => app.marked[e.path]);
                                            const bytes = sel.reduce((n, e) => n + (e.dir ? 0 : e.size), 0);
                                            s += " · " + app.markCount + " selected" + (bytes ? " (" + app.fmtSize(bytes) + ")" : "");
                                        } else if (app.current && !app.current.dir && app.found === null && !app.onSpecial) {
                                            s += " · “" + app.current.name + "” " + app.fmtSize(app.current.size);
                                        }
                                        if (app.clip.paths.length) s += " · " + app.plural(app.clip.paths.length, "item") + (app.clip.mode === "cut" ? " to move" : " copied");
                                        if (app.onCView) s += " · double-click a folder to open it";
                                        else if (app.listing && !app.listing.writable && !app.listing.error) s += " · read-only";
                                        return s;
                                    }
                                    color: app.note !== "" && app.noteError ? Theme.danger : app.note !== "" ? Theme.text : Theme.muted
                                    elide: Text.ElideRight
                                    font.family: Theme.font; font.pixelSize: 13
                                }
                                Chip {
                                    visible: app.searching
                                    label: "Stop searching"
                                    onClicked: { keys.forceActiveFocus(); app.stopSearch(); }
                                }
                                Text {
                                    visible: briefcase.inBriefcase && (briefcase.syncLine !== "" || briefcase.lockLine !== "")
                                    text: [briefcase.lockLine, briefcase.syncLine].filter(x => x).join(" · ")
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                }
                                Text {
                                    visible: !!(app.listing && app.listing.free !== undefined && app.listing.free !== null)
                                    text: app.listing ? app.fmtSize(app.listing.free) + " free" : ""
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                }
                            }

                            RowLayout {
                                visible: !!app.job
                                anchors { fill: parent; leftMargin: 18; rightMargin: 14 }
                                spacing: 14
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 6
                                    readonly property var j: app.job || ({ done: 0, total: 0, speed: 0 })
                                    Text {
                                        Layout.fillWidth: true
                                        text: {
                                            const j = parent.j;
                                            const verb = j.cancelling ? "Stopping" : app.jobVerbing(j.verb);
                                            const what = j.name ? "“" + j.name + "”" : app.plural(j.count || 0, "item");
                                            if (j.verb === "compress") return verb + " " + what + " into “" + (j.destName || "") + "”";
                                            if (j.verb === "extract") return verb + " " + what;
                                            return verb + " " + what + " to “" + (j.destName || "") + "”";
                                        }
                                        color: Theme.text; elide: Text.ElideRight
                                        font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold
                                    }
                                    Rectangle {
                                        Layout.fillWidth: true
                                        Layout.preferredHeight: 6
                                        radius: 3
                                        color: Theme.panel3
                                        Rectangle {
                                            width: parent.width * (parent.parent.j.total > 0 ? Math.min(1, parent.parent.j.done / parent.parent.j.total) : 0)
                                            height: parent.height
                                            radius: 3
                                            color: Theme.accent
                                            Behavior on width { NumberAnimation { duration: 240 } }
                                        }
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: {
                                            const j = parent.j;
                                            if (!j.total) return "Getting ready…";
                                            const left = j.speed > 0 ? app.fmtTime((j.total - j.done) / j.speed) : "";
                                            return app.fmtSize(j.done) + " of " + app.fmtSize(j.total)
                                                + (j.speed > 0 ? " · " + app.fmtSize(j.speed) + "/s" : "")
                                                + (left ? " · about " + left + " left" : "")
                                                + (app.jobQueue.length ? " · " + app.jobQueue.length + " more waiting" : "");
                                        }
                                        color: Theme.muted; elide: Text.ElideRight
                                        font.family: Theme.font; font.pixelSize: 12
                                    }
                                }
                                Chip {
                                    label: "Cancel"
                                    onClicked: app.cancelJob()
                                }
                            }
                        }

                        // ------------------------------------------------ on-screen keyboard inside the window (picker.qml in Game
                        // Mode, where gamescope shows one window): Hearth's shared keyboard.
                        OskBoard {
                            Layout.fillWidth: true
                            Layout.preferredHeight: visible ? Math.min(330, Math.max(240, win.height * 0.4)) : 0
                            visible: app.oskOn
                            vm: app.oskVm
                            onTapped: (r, c) => { if (app.oskEngine) app.oskEngine.tap(r, c); }
                            onLongPressed: (r, c) => { if (app.oskEngine) app.oskEngine.longPress(r, c); }
                            onAltPicked: i => { if (app.oskEngine) app.oskEngine.pickAlt(i); }
                        }
                    }

                    // ------------------------------------------------ the details pane (the Details button, Alt+P)
                    Rectangle { Layout.fillHeight: true; Layout.preferredWidth: 1; color: Theme.line; visible: detailsBox.visible }
                    Rectangle {
                        id: detailsBox
                        visible: app.detailsPane && !app.picking
                        Layout.fillHeight: true
                        Layout.preferredWidth: win.width < 1100 ? 250 : 300
                        color: Theme.panel
                        readonly property var sel: app.markCount > 1 ? app.entries.filter(x => app.marked[x.path]) : []
                        readonly property var e: sel.length ? null : app.current
                        ColumnLayout {
                            anchors { fill: parent; margins: 18 }
                            spacing: 10
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: Math.round(width * 0.72)
                                radius: Theme.r(12)
                                color: Theme.panel2
                                FileGlyph {
                                    anchors.centerIn: parent
                                    visible: detailThumb.status !== Image.Ready
                                    kind: detailsBox.e ? app.kindOf(detailsBox.e) : "folder"
                                    label: detailsBox.e ? app.extLabel(detailsBox.e) : ""
                                    link: !!detailsBox.e && !!detailsBox.e.isLink
                                    size: 96
                                }
                                Image {
                                    id: detailThumb
                                    anchors { fill: parent; margins: 6 }
                                    source: detailsBox.e && !detailsBox.e.dir ? app.thumbFor(detailsBox.e) : ""
                                    sourceSize: Qt.size(480, 480)
                                    asynchronous: true
                                    fillMode: Image.PreserveAspectFit
                                    smooth: true
                                }
                                Rectangle {
                                    visible: detailsBox.sel.length > 0
                                    anchors { right: parent.right; bottom: parent.bottom; margins: 10 }
                                    width: 34; height: 34; radius: 17
                                    color: Theme.accent
                                    Text { anchors.centerIn: parent; text: detailsBox.sel.length; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                                }
                            }
                            Text {
                                Layout.fillWidth: true
                                text: detailsBox.sel.length ? detailsBox.sel.length + " items selected"
                                    : detailsBox.e ? detailsBox.e.name : app.titleOf(app.path)
                                color: Theme.text
                                wrapMode: Text.WrapAnywhere
                                maximumLineCount: 3
                                elide: Text.ElideRight
                                font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold
                            }
                            Text {
                                Layout.fillWidth: true
                                text: detailsBox.sel.length ? "" : detailsBox.e ? (detailsBox.e.cv ? detailsBox.e.cv.type || "Folder" : detailsBox.e.app ? app.appKindName(detailsBox.e.app.kind) + " app"
                                    : detailsBox.e.pc === "drive" ? "Drive" : detailsBox.e.pc === "net" ? "Network location" : app.kindName(detailsBox.e)) : app.plural(app.entries.length, "item")
                                visible: text !== ""
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                            }
                            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line }
                            Repeater {
                                model: {
                                    const e = detailsBox.e, sel = detailsBox.sel;
                                    if (sel.length) {
                                        const bytes = sel.reduce((n, x) => n + (x.dir ? 0 : (x.size || 0)), 0);
                                        const dirs = sel.filter(x => x.dir).length;
                                        return [["Size", app.fmtSize(bytes) + (dirs ? " (plus what's in " + app.plural(dirs, "folder") + ")" : "")],
                                                ["Folders", String(dirs)], ["Files", String(sel.length - dirs)]];
                                    }
                                    if (!e) {
                                        const lr = [];
                                        if (app.onCView) return [["In Hearth", app.winPath(app.path)], ["On Linux", "several places: see each folder"]];
                                        const ll = app.listing && !app.onSpecial && !app.onHome ? app.letterLocation(app.listing.path) : "";
                                        if (ll) lr.push(["Location", ll]);
                                        if (ll && app.cdriveOn) lr.push(["In Hearth", app.winPath(app.listing.path)]);
                                        if (ll) lr.push(["On Linux", app.listing.path]);
                                        if (app.listing && app.listing.free !== undefined && app.listing.free !== null) lr.push(["Free", app.fmtSize(app.listing.free)]);
                                        return briefcase.detailRows(null).concat(lr);
                                    }
                                    // This PC: a drive's space and where it really is; Installed apps: what the row says.
                                    if (e.pc) {
                                        const d = e.item || {};
                                        const pr = [];
                                        if (d.total) pr.push(["Size", app.fmtSize(d.total)]);
                                        if (d.free !== undefined && d.free !== null && d.total) pr.push(["Free", app.fmtSize(d.free)]);
                                        if (d.fs) pr.push(["File system", d.fs]);
                                        if (e.pc !== "folder" && d.letter) pr.push(["Letter", d.letter + ":" + (d.letter === "C" ? " (always Hearth)" : "")]);
                                        if (d.sub && e.pc !== "folder") pr.push(["Kind", d.sub]);
                                        if (e.path.indexOf("/") === 0) pr.push(["On Linux", e.path]);
                                        return pr;
                                    }
                                    // Hearth (C:)'s view: its name in Hearth, where it really is, and what it is.
                                    if (e.cv) {
                                        const cr = [["In Hearth", e.cv.win || app.winPath(e.path)], ["On Linux", (e.cv.real || [e.path]).join("\n")]];
                                        if (e.cv.size) cr.push(["Size", app.fmtSize(e.cv.size)]);
                                        if (e.cv.note) cr.push(["", e.cv.note]);
                                        return cr;
                                    }
                                    if (e.app) {
                                        const a = e.app;
                                        const ar = [["Kind", app.appKindName(a.kind)]];
                                        if (a.part) ar.push(["Part of", "Hearth OS"]);
                                        else if (a.publisher) ar.push(["Publisher", a.publisher]);
                                        if (a.version) ar.push(["Version", a.version]);
                                        if (a.size) ar.push(["Size", app.fmtSize(a.size)]);
                                        if (a.date) ar.push(["Installed", app.fmtDate(a.date)]);
                                        if (a.drive && a.drive !== "/") ar.push(["Drive", app.driveLabelFor(a.drive)]);
                                        return ar;
                                    }
                                    const rows = [];
                                    if (!e.dir && app.kindOf(e) === "image" && dimImg.status === Image.Ready && dimImg.implicitWidth) rows.push(["Size", dimImg.implicitWidth + " × " + dimImg.implicitHeight]);
                                    else if (!e.dir) rows.push(["Size", app.fmtSize(e.size)]);
                                    if (e.mtime) rows.push(["Modified", app.fmtDate(e.mtime)]);
                                    if (e.opened) rows.push(["Opened", app.fmtDate(Date.parse(e.opened) / 1000)]);
                                    // Where it is in drive letters (This PC › Games (D:) › …), and the Linux path for when you need it.
                                    const loc = e.path.indexOf("/") === 0 ? app.letterLocation(app.parentOf(e.path)) : "";
                                    rows.push(["Where", app.cdriveOn && e.path.indexOf("/") === 0 ? app.winPath(app.parentOf(e.path)) : (loc || app.parentOf(e.path))]);
                                    if (loc && app.cdriveOn && e.path.indexOf("/") === 0) rows.push(["In Hearth", app.winPath(e.path)]);
                                    if (loc) rows.push(["On Linux", e.path]);
                                    if (e.isLink) rows.push(["Link", "Points somewhere else"]);
                                    if (app.versionsOf === e.path) rows.push(["Previous versions", String(app.versions.length)]);
                                    return rows.concat(briefcase.detailRows(e));
                                }
                                delegate: RowLayout {
                                    required property var modelData
                                    Layout.fillWidth: true
                                    spacing: 10
                                    Text { Layout.preferredWidth: 76; Layout.alignment: Qt.AlignTop; visible: modelData[0] !== ""; text: modelData[0]; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                                    Text { Layout.fillWidth: true; text: modelData[1]; color: Theme.text; wrapMode: Text.WrapAnywhere; font.family: Theme.font; font.pixelSize: 13 }
                                }
                            }
                            // Set as wallpaper · Open with ▾ (mockup files-6)
                            Row {
                                visible: !!detailsBox.e && !detailsBox.e.dir && !detailsBox.e.pc && !detailsBox.e.app && !detailsBox.e.cv
                                spacing: 6
                                Text {
                                    visible: !!detailsBox.e && app.kindOf(detailsBox.e) === "image"
                                    text: "Set as wallpaper"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 13
                                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { keys.forceActiveFocus(); app.runMenu("wallpaper"); } }
                                }
                                Text { visible: !!detailsBox.e && app.kindOf(detailsBox.e) === "image"; text: "·"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                                Text {
                                    text: "Open with ▾"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 13
                                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { keys.forceActiveFocus(); app.showApps(detailsBox.e, false); } }
                                }
                            }
                            // the picture's own size (read only for the picked picture), and its earlier versions
                            Image {
                                id: dimImg
                                visible: false
                                asynchronous: true
                                source: detailsBox.e && !detailsBox.e.dir && detailsBox.e.path && app.kindOf(detailsBox.e) === "image" && app.detailsPane ? "file://" + detailsBox.e.path : ""
                            }
                            Connections {
                                target: detailsBox
                                function onEChanged() { if (detailsBox.e && detailsBox.e.path && detailsBox.e.path.indexOf("/") === 0 && app.detailsPane) app.lookForVersions(detailsBox.e.path, false); }
                            }
                            Item { Layout.fillHeight: true }
                            RowLayout {
                                Layout.fillWidth: true
                                visible: !!detailsBox.e
                                spacing: 8
                                Chip { Layout.fillWidth: true; label: "Open"; primary: true; onClicked: { keys.forceActiveFocus(); app.activate(detailsBox.e); } }
                                Chip { Layout.fillWidth: true; label: "Properties"; onClicked: { keys.forceActiveFocus(); app.menuOnFolder = false; app.runMenu("props"); } }
                            }
                        }
                    }
                }
            }

            function itemClicked(index, mouse, area) {
                keys.forceActiveFocus();
                app.zone = "main";
                const e = app.entries[index];
                if (mouse.button === Qt.RightButton) return win.itemMenu(index, mouse, area);
                if (mouse.button === Qt.MiddleButton) {
                    if (e && e.dir) app.newTab(e.path);
                    return;
                }
                if (mouse.modifiers & Qt.ControlModifier) {
                    // Ctrl-click: add or take away (the one that was current counts as picked too).
                    if (!app.markCount && app.current && app.cur !== index) app.toggleMark(app.current);
                    app.cur = index;
                    app.anchor = index;
                    app.toggleMark(e);
                    return;
                }
                if (mouse.modifiers & Qt.ShiftModifier) {
                    if (app.anchor < 0) app.anchor = app.cur;
                    app.selectRange(app.anchor, index);
                    app.cur = index;
                    return;
                }
                app.marked = ({});
                app.anchor = index;
                app.cur = index;
                // Picking: a click picks it (a folder too, for "Choose a folder"); a file's name goes in the Name box.
                if (app.picking && e) {
                    const m = {};
                    m[e.path] = true;
                    app.marked = m;
                    if (app.pick.kind === "save" && !e.dir) app.pickName = e.name;
                }
            }
            function itemMenu(index, mouse, area) {
                keys.forceActiveFocus();
                app.zone = "main";
                const e = app.entries[index];
                if (e && !app.marked[e.path]) app.marked = ({});
                app.cur = index;
                app.viaPad = false;
                const p = area.mapToItem(keys, mouse.x, mouse.y);
                app.openMenu(false, p.x, p.y);
            }
            // The rubber band from a to b (content coordinates of the view): picks what it touches.
            function band(a, b, base) {
                const v = app.shownView === "details" ? list : grid;
                const l = Math.min(a.x, b.x), r = Math.max(a.x, b.x), t = Math.min(a.y, b.y), btm = Math.max(a.y, b.y);
                const tl = folderArea.mapFromItem(v.contentItem, l, t);
                bandRect.x = Math.max(0, tl.x);
                bandRect.y = Math.max(0, tl.y);
                bandRect.width = Math.min(folderArea.width - bandRect.x, r - l - (bandRect.x - tl.x));
                bandRect.height = Math.min(folderArea.height - bandRect.y, btm - t - (bandRect.y - tl.y));
                bandRect.visible = true;
                const m = Object.assign({}, base);
                const n = app.entries.length;
                let last = -1;
                if (v === grid) {
                    const cw = grid.cellWidth, ch = grid.cellHeight, cols = win.cols;
                    const c0 = Math.max(0, Math.floor(l / cw)), c1 = Math.min(cols - 1, Math.floor(r / cw));
                    const r0 = Math.max(0, Math.floor(t / ch)), r1 = Math.floor(btm / ch);
                    for (let row = r0; row <= r1; row++) for (let col = c0; col <= c1; col++) {
                        const i = row * cols + col;
                        if (i < n) { m[app.entries[i].path] = true; last = i; }
                    }
                } else if (l < list.width) {
                    for (let y = t; y <= btm + 6; y += 6) {
                        const i = list.indexAt(60, Math.min(y, btm));
                        if (i >= 0) { m[app.entries[i].path] = true; last = i; }
                    }
                }
                app.marked = m;
                if (last >= 0) app.cur = last;
            }

            // ------------------------------------------------ overlays: actions menu and dialogs
            Rectangle {
                anchors.fill: parent
                visible: app.overlay !== ""
                color: app.overlay === "menu" ? "#55000000" : "#99000000"
                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onClicked: app.closeOverlay()
                }

                // The Briefcase's dialogs (Update All, passwords, Customize, choosing a Briefcase, progress).
                BriefcaseDialog { id: bcDialog; bc: briefcase }

                // Properties of Hearth (C:)'s own folders (FilesCProps.qml, behind a Loader like its page).
                Loader {
                    id: cPropsLoader
                    anchors.fill: parent
                    active: app.overlay === "cprops"
                    visible: active && status === Loader.Ready
                    Component.onCompleted: setSource("FilesCProps.qml", { fm: app })
                    onLoaded: item.fm = app
                    onStatusChanged: if (status === Loader.Error) { console.warn("Files: Hearth (C:)'s Properties couldn't load (FilesCProps.qml)"); app.closeOverlay(); }
                }

                // Actions menu in another look (Settings › Personalization › Bar and dock › Right-click menu,
                // Theme.menuStyle): MenuStyles.qml, shared with the desktop's menus (PopMenu.qml), loaded by file name;
                // if it can't load, the menu below (today's) shows.
                Loader {
                    id: styledMenu
                    readonly property bool styled: Theme.menuStyle !== "today" && status === Loader.Ready && item !== null
                    active: Theme.menuStyle !== "today"
                    source: "MenuStyles.qml"
                    visible: styled && app.overlay === "menu"
                    width: item ? item.implicitWidth : 0
                    height: item ? item.implicitHeight : 0
                    x: app.menuX < 0 ? (parent.width - width) / 2 : Math.max(8, Math.min(parent.width - width - 8, app.menuX))
                    y: app.menuY < 0 ? (parent.height - height) / 2 : Math.max(8, Math.min(parent.height - height - 8, app.menuY))
                    function reopen() { if (styled && app.overlay === "menu") item.open(app.menuTitle, app.menuItems); }
                    onLoaded: reopen()
                    onStatusChanged: if (status === Loader.Error) console.warn("Files: the right-click menu's look (MenuStyles.qml) couldn't load; using today's menu")
                    Connections {
                        target: app
                        function onMenuItemsChanged() { styledMenu.reopen(); }
                        function onOverlayChanged() { if (app.overlay === "menu") styledMenu.reopen(); }
                    }
                    Connections {
                        target: styledMenu.item
                        function onChosen(it) { app.runMenu(it.id); }
                        function onClosed() { app.closeOverlay(); }
                    }
                }
                Binding { target: styledMenu.item; when: styledMenu.item !== null; property: "look"; value: Theme.menuStyle }
                Binding { target: styledMenu.item; when: styledMenu.item !== null; property: "padHint"; value: app.viaPad }   // opened with the controller (X) or the Menu key
                Binding { target: styledMenu.item; when: styledMenu.item !== null; property: "spaceRight"; value: styledMenu.parent.width - styledMenu.x - styledMenu.width - 8 }
                Binding { target: styledMenu.item; when: styledMenu.item !== null; property: "maxHeight"; value: styledMenu.parent.height - 16 }
                Binding { target: styledMenu.item; when: styledMenu.item !== null; property: "maxBottom"; value: styledMenu.parent.height - 8 - styledMenu.y }
                Binding { target: styledMenu.item; when: styledMenu.item !== null; property: "minTop"; value: 8 - styledMenu.y }

                // Actions menu.
                Rectangle {
                    id: menuCard
                    visible: app.overlay === "menu" && !styledMenu.styled
                    width: 320
                    height: Math.min(parent.height - 16, menuHead.height + menuList.contentHeight + 20 + (menuPadHint.visible ? menuPadHint.height : 0))
                    x: app.menuX < 0 ? (parent.width - width) / 2 : Math.max(8, Math.min(parent.width - width - 8, app.menuX))
                    y: app.menuY < 0 ? (parent.height - height) / 2 : Math.max(8, Math.min(parent.height - height - 8, app.menuY))
                    radius: Theme.radius
                    color: Theme.panel
                    border.color: Theme.line
                    MouseArea { anchors.fill: parent }
                    Text {
                        id: menuHead
                        anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 22; rightMargin: 16; topMargin: 12 }
                        height: 26
                        text: app.menuTitle
                        color: Theme.muted
                        elide: Text.ElideMiddle
                        font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold
                    }
                    ListView {
                        id: menuList
                        anchors { left: parent.left; right: parent.right; top: menuHead.bottom; bottom: menuPadHint.visible ? menuPadHint.top : parent.bottom; margins: 8; topMargin: 0 }
                        model: app.overlay === "menu" ? app.menuItems : []
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        currentIndex: app.menuIndex
                        onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                        delegate: Item {
                            id: mi
                            required property var modelData
                            required property int index
                            readonly property bool lit: app.menuIndex === index && !modelData.sep
                            width: ListView.view.width
                            height: modelData.sep ? 11 : modelData.sub ? 52 : 40
                            Rectangle {
                                visible: !!mi.modelData.sep
                                anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 10; rightMargin: 10 }
                                height: 1
                                color: Theme.line
                            }
                            Rectangle {
                                visible: !mi.modelData.sep
                                anchors.fill: parent
                                radius: Theme.r(10)
                                color: mi.lit ? (mi.modelData.danger ? Theme.danger : Theme.accent) : miMouse.containsMouse ? Theme.panel3 : "transparent"
                                Column {
                                    anchors { left: parent.left; leftMargin: 14; right: miKey.left; rightMargin: 8; verticalCenter: parent.verticalCenter }
                                    spacing: 1
                                    Text {
                                        width: parent.width
                                        text: mi.modelData.label || ""
                                        elide: Text.ElideRight
                                        color: mi.lit ? (mi.modelData.danger ? "#ffffff" : Theme.accentInk) : mi.modelData.danger ? Theme.danger : Theme.text
                                        font.family: Theme.font; font.pixelSize: 14; font.weight: mi.lit ? Font.DemiBold : Font.Normal
                                    }
                                    // A second line in small monospace: the path a Copy or Go to item uses (Hearth (C:)'s menu).
                                    Text {
                                        width: parent.width
                                        visible: !!mi.modelData.sub
                                        text: mi.modelData.sub || ""
                                        elide: Text.ElideMiddle
                                        color: mi.lit ? (mi.modelData.danger ? "#ffffff" : Theme.accentInk) : Theme.muted
                                        font.family: "monospace"; font.pixelSize: 11
                                    }
                                }
                                Text {
                                    id: miKey
                                    anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter }
                                    text: mi.modelData.key || ""
                                    color: mi.lit ? (mi.modelData.danger ? "#ffffff" : Theme.accentInk) : Theme.muted
                                    font.family: Theme.font; font.pixelSize: 11
                                }
                                MouseArea {
                                    id: miMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onEntered: app.menuIndex = mi.index
                                    onClicked: app.runMenu(mi.modelData.id)
                                }
                            }
                        }
                    }
                    // Opened with the controller: what A and B do.
                    Item {
                        id: menuPadHint
                        visible: app.viaPad
                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                        height: 34
                        Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: 10; rightMargin: 10 } height: 1; color: Theme.line }
                        MenuHints {
                            anchors.centerIn: parent
                            k: 0.72
                            model: [{ b: "A", t: "Choose" }, { b: "B", t: "Back" }]
                        }
                    }
                }

                // Dialogs: text box, confirm, open with, name clash, preview.
                Rectangle {
                    id: card
                    visible: app.overlay !== "" && app.overlay !== "menu" && app.overlay !== "briefcase" && app.overlay !== "cprops"
                    anchors.centerIn: parent
                    width: Math.min(parent.width - 48, app.overlay === "preview" ? 760 : 560)
                    height: Math.min(parent.height - 48, cardCol.implicitHeight + 44)
                    radius: Theme.radius + 2
                    color: Theme.panel
                    border.color: Theme.line
                    MouseArea {
                        anchors.fill: parent
                        onClicked: {
                            if (app.overlay === "input") field.forceActiveFocus();
                            else keys.forceActiveFocus();
                        }
                    }
                    ColumnLayout {
                        id: cardCol
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 22 }
                        spacing: 14
                        Text {
                            Layout.fillWidth: true
                            text: app.overlay === "conflict" && app.cf ? "“" + app.cf.list[app.cf.i].name + "” is already in “" + app.base(app.cf.dest) + "”"
                                : app.overlay === "preview" && app.pv ? app.pv.e.name : app.dlgTitle
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold
                            elide: Text.ElideMiddle
                        }
                        Text {
                            Layout.fillWidth: true
                            visible: app.overlay === "confirm"
                            text: app.dlgText
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
                        }

                        // Text box (rename, new folder, compress…).
                        Rectangle {
                            Layout.fillWidth: true
                            visible: app.overlay === "input"
                            implicitHeight: 52
                            radius: Theme.r(12)
                            color: Theme.panel3
                            border.width: 2
                            border.color: field.activeFocus ? Theme.accent : "transparent"
                            TextInput {
                                id: field
                                anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
                                verticalAlignment: TextInput.AlignVCenter
                                color: Theme.text
                                selectionColor: Theme.accent
                                selectedTextColor: Theme.accentInk
                                font.family: Theme.font
                                font.pixelSize: 15
                                selectByMouse: true
                                clip: true
                                echoMode: app.dlgSecret ? TextInput.Password : TextInput.Normal
                                Keys.onPressed: e => {
                                    switch (e.key) {
                                    case Qt.Key_Return: case Qt.Key_Enter: app.accept(field.text); break;
                                    case Qt.Key_Escape: app.closeOverlay(); break;
                                    case Qt.Key_Up: case Qt.Key_Down: break;
                                    default: return;
                                    }
                                    e.accepted = true;
                                }
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            visible: app.overlay === "input"
                            text: "Ⓨ on-screen keyboard · ☰ or Enter to finish · Ⓑ cancel"
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                        }

                        // A name that's already there: Skip, Keep both, Replace (Merge for folders).
                        ColumnLayout {
                            Layout.fillWidth: true
                            visible: app.overlay === "conflict" && !!app.cf
                            spacing: 10
                            readonly property var c: app.cf ? app.cf.list[app.cf.i] : null
                            Text {
                                Layout.fillWidth: true
                                text: !parent.c ? "" : parent.c.targetDir && parent.c.srcDir
                                    ? "Merging puts what's in the new folder into the one that's there; files with the same name are replaced."
                                    : "Replacing it overwrites what's there."
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
                            }
                            GridLayout {
                                Layout.fillWidth: true
                                columns: 2
                                columnSpacing: 16
                                rowSpacing: 4
                                visible: !!parent.c
                                Text { text: "There now"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                                Text {
                                    Layout.fillWidth: true
                                    readonly property var c: app.cf ? app.cf.list[app.cf.i] : null
                                    text: !c ? "" : (c.targetDir ? "Folder" : app.fmtSize(c.targetSize)) + " · modified " + app.fmtDate(c.targetMtime)
                                    color: Theme.text; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight
                                }
                                Text { text: "New one"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                                Text {
                                    Layout.fillWidth: true
                                    readonly property var c: app.cf ? app.cf.list[app.cf.i] : null
                                    text: !c ? "" : (c.srcDir ? "Folder" : app.fmtSize(c.srcSize)) + " · modified " + app.fmtDate(c.srcMtime)
                                    color: Theme.text; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight
                                }
                            }
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 44
                                visible: !!app.cf && app.cf.list.length - app.cf.i > 1
                                radius: Theme.r(10)
                                color: Theme.panel2
                                border.width: 2
                                border.color: app.cf && app.cf.button === 3 ? Theme.accent : "transparent"
                                RowLayout {
                                    anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                                    spacing: 10
                                    Rectangle {
                                        Layout.preferredWidth: 22; Layout.preferredHeight: 22; radius: Theme.r(6)
                                        color: app.cf && app.cf.all ? Theme.accent : Theme.panel3
                                        border.width: 2; border.color: app.cf && app.cf.all ? Theme.accent : Theme.line
                                        Icon { anchors.centerIn: parent; visible: !!app.cf && app.cf.all; name: "check"; size: 14; color: Theme.accentInk }
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: app.cf ? "Do the same for the other " + (app.cf.list.length - app.cf.i - 1) + " with names that are there" : ""
                                        color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideRight
                                    }
                                }
                                MouseArea { anchors.fill: parent; onClicked: { keys.forceActiveFocus(); app.cf = Object.assign({}, app.cf, { all: !app.cf.all }); } }
                            }
                        }

                        // Quick preview (Space): a picture, the start of a text file, or the basics.
                        Item {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 420
                            visible: app.overlay === "preview" && !!app.pv
                            Image {
                                anchors.fill: parent
                                visible: !!app.pv && app.kindOf(app.pv.e) === "image"
                                source: app.pv && app.kindOf(app.pv.e) === "image" ? app.fileUrl(app.pv.e.path) : ""
                                sourceSize: Qt.size(1400, 1000)
                                fillMode: Image.PreserveAspectFit
                                asynchronous: true
                            }
                            Image {
                                anchors.fill: parent
                                visible: !!app.pv && (app.kindOf(app.pv.e) === "video" || app.pv.e.mime === "application/pdf") && status === Image.Ready
                                source: app.pv && (app.kindOf(app.pv.e) === "video" || app.pv.e.mime === "application/pdf") ? app.thumbFor(app.pv.e) : ""
                                fillMode: Image.PreserveAspectFit
                                asynchronous: true
                            }
                            Flickable {
                                anchors.fill: parent
                                visible: !!app.pv && app.pv.text !== ""
                                contentHeight: pvText.implicitHeight
                                clip: true
                                boundsBehavior: Flickable.StopAtBounds
                                Rectangle { anchors.fill: parent; color: Theme.bg2; radius: Theme.r(10); z: -1 }
                                Text {
                                    id: pvText
                                    width: parent.width
                                    padding: 14
                                    text: app.pv ? app.pv.text : ""
                                    color: Theme.text
                                    wrapMode: Text.WrapAnywhere
                                    textFormat: Text.PlainText
                                    font.family: "monospace"
                                    font.pixelSize: 13
                                }
                            }
                            Column {
                                anchors.centerIn: parent
                                spacing: 12
                                visible: !!app.pv && app.pv.text === "" && app.kindOf(app.pv.e) !== "image" && !(app.kindOf(app.pv.e) === "video" || app.pv.e.mime === "application/pdf")
                                FileGlyph {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    kind: app.pv ? app.kindOf(app.pv.e) : "file"
                                    label: app.pv ? app.extLabel(app.pv.e) : ""
                                    size: 120
                                }
                                Text {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    text: app.pv ? app.kindName(app.pv.e) + (app.pv.e.dir ? "" : " · " + app.fmtSize(app.pv.e.size)) + " · " + app.fmtDate(app.pv.e.mtime) : ""
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
                                }
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            visible: app.overlay === "preview"
                            text: "← → previous / next · Enter opens · Space or Ⓑ closes"
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                        }

                        // Open with.
                        Text {
                            Layout.fillWidth: true
                            visible: app.overlay === "apps" && (!app.dlgInfo || app.dlgApps.length === 0)
                            text: !app.dlgInfo ? "Looking for apps…" : "No installed app says it can open this kind of file. Try Other app…"
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
                        }
                        ListView {
                            id: appList
                            Layout.fillWidth: true
                            Layout.preferredHeight: Math.min(contentHeight, 360)
                            visible: app.overlay === "apps" && !!app.dlgInfo
                            model: app.overlay === "apps" ? app.dlgApps.concat([{ id: "", name: app.dlgAppsAll ? "Only the suggested apps" : "Other app…", other: true }]) : []
                            spacing: 4
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            currentIndex: app.dlgIndex
                            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                            delegate: Rectangle {
                                id: ap
                                required property var modelData
                                required property int index
                                readonly property bool lit: app.dlgIndex === index
                                width: ListView.view.width
                                height: 50
                                radius: Theme.r(10)
                                color: lit ? Theme.accent : apMouse.containsMouse ? Theme.panel3 : Theme.panel2
                                RowLayout {
                                    anchors { fill: parent; leftMargin: 12; rightMargin: 14 }
                                    spacing: 12
                                    IconImage {
                                        visible: !ap.modelData.other
                                        implicitSize: 30
                                        source: ap.modelData.other ? "" : AppIcons.source(ap.modelData.icon || "", ap.modelData.id || "")
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: ap.modelData.name
                                        color: ap.lit ? Theme.accentInk : Theme.text
                                        elide: Text.ElideRight
                                        font.family: Theme.font; font.pixelSize: 14; font.weight: ap.lit ? Font.DemiBold : Font.Normal
                                    }
                                    Text {
                                        visible: !!ap.modelData.default
                                        text: "Default"
                                        color: ap.lit ? Theme.accentInk : Theme.muted
                                        font.family: Theme.font; font.pixelSize: 12
                                    }
                                }
                                MouseArea {
                                    id: apMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: app.openWith(ap.modelData)
                                }
                            }
                        }

                        // Buttons.
                        RowLayout {
                            Layout.fillWidth: true
                            Layout.topMargin: 4
                            spacing: 10
                            Item { Layout.fillWidth: true }
                            Chip {
                                visible: app.overlay !== "conflict"
                                label: app.overlay === "apps" || app.overlay === "preview" ? "Close" : "Cancel"
                                focused: app.overlay === "confirm" && app.dlgButton === 0
                                onClicked: app.closeOverlay()
                            }
                            Chip {
                                visible: app.overlay === "confirm" && app.dlgAlt !== ""
                                label: app.dlgAlt
                                focused: app.overlay === "confirm" && app.dlgButton === 2
                                onClicked: app.acceptAlt()
                            }
                            Chip {
                                visible: app.overlay === "input" || app.overlay === "confirm"
                                label: app.dlgOk
                                primary: true
                                danger: app.dlgDanger
                                focused: app.overlay === "confirm" && app.dlgButton === 1
                                onClicked: app.accept(field.text)
                            }
                            Chip {
                                visible: app.overlay === "conflict"
                                label: "Skip"
                                focused: !!app.cf && app.cf.button === 0
                                onClicked: app.conflictChoose("skip")
                            }
                            Chip {
                                visible: app.overlay === "conflict"
                                label: "Keep both"
                                focused: !!app.cf && app.cf.button === 1
                                onClicked: app.conflictChoose("rename")
                            }
                            Chip {
                                visible: app.overlay === "conflict"
                                readonly property var c: app.cf ? app.cf.list[app.cf.i] : null
                                label: c && c.srcDir && c.targetDir ? "Merge" : "Replace"
                                primary: true
                                focused: !!app.cf && app.cf.button === 2
                                onClicked: app.conflictChoose("replace")
                            }
                        }
                    }
                }
            }
        }
    }

    // ================================================ small pieces
    // A button in the Hearth style: rounded, accent border when the controller / keyboard is on it.
    component Chip: Rectangle {
        id: chip
        property string label
        property string icon: ""
        property bool focused: false
        property bool dot: false
        property bool flat: false
        property bool strong: false
        property bool primary: false
        property bool danger: false
        property bool dim: false
        signal clicked()
        implicitWidth: chipRow.implicitWidth + (label === "" ? 22 : 28)
        implicitHeight: 40
        radius: Theme.r(10)
        opacity: dim ? 0.4 : 1
        color: primary ? (danger ? Theme.danger : Theme.accent)
             : chipMouse.containsMouse ? Theme.panel3 : flat ? "transparent" : Theme.panel2
        border.width: 2
        border.color: focused ? (primary ? Theme.text : Theme.accent) : "transparent"
        Behavior on border.color { ColorAnimation { duration: Theme.quick } }
        Row {
            id: chipRow
            anchors.centerIn: parent
            spacing: 8
            Icon {
                visible: chip.icon !== ""
                anchors.verticalCenter: parent.verticalCenter
                name: chip.icon
                size: 16
                color: Theme.text
            }
            Text {
                visible: chip.label !== ""
                anchors.verticalCenter: parent.verticalCenter
                text: chip.label
                color: chip.primary ? (chip.danger ? "#ffffff" : Theme.accentInk) : Theme.text
                font.family: Theme.font
                font.pixelSize: 13
                font.weight: chip.strong || chip.primary ? Font.DemiBold : Font.Normal
            }
            Rectangle {
                visible: chip.dot
                anchors.verticalCenter: parent.verticalCenter
                width: 8; height: 8; radius: 4
                color: Theme.accent
            }
        }
        MouseArea {
            id: chipMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: chip.clicked()
        }
    }

    // A details column title; click to sort by it (again to turn the order around).
    component ColumnHead: Text {
        id: head
        property string label
        property string key
        property string sortKey
        property bool desc: false
        property bool alignRight: false
        signal picked(string key)
        text: label + (sortKey === key ? (desc ? "  ▾" : "  ▴") : "")
        horizontalAlignment: alignRight ? Text.AlignRight : Text.AlignLeft
        color: sortKey === key ? Theme.text : Theme.muted
        elide: Text.ElideRight
        font.family: Theme.font
        font.pixelSize: 12
        font.weight: Font.DemiBold
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: head.picked(head.key) }
    }

    // A folder or file drawn in the Hearth palette: folders in the accent color, files as cards with a
    // colored band for their kind (pictures, videos, music, archives, apps…) and the extension.
    component FileGlyph: Item {
        id: g
        property string kind: "file"
        property string label: ""
        property bool link: false
        property real size: 64
        property var bc: null                   // a Briefcase folder: {icon, color, badge, locked} (hearth-files list)
        property var badge: null                // an item in a Briefcase: its status {color, glyph}
        property color bcColor: Theme.accent
        readonly property bool isBc: !!bc
        readonly property color band: ({ image: "#5fb98f", video: "#b36ad9", audio: "#4f9dde", archive: "#b98a5a", disc: "#8c7ae6",
            app: Theme.ember, doc: "#e06666", text: "#a69484", file: "#7d6a5c" })[kind] || "#7d6a5c"
        width: size
        height: size
        // A Briefcase: its icon style in its colour, its badge, a padlock when locked.
        Icon { anchors.fill: parent; visible: g.isBc; name: "bc-" + (g.bc && g.bc.icon ? g.bc.icon : "briefcase"); size: g.size; color: g.bcColor }
        Text {
            visible: g.isBc && !!g.bc.badge && g.size >= 24
            anchors { right: parent.right; top: parent.top }
            text: g.bc ? g.bc.badge || "" : ""
            color: Theme.text; font.family: Theme.font; font.pixelSize: Math.max(9, g.size * 0.24); font.weight: Font.Bold
        }
        Rectangle {
            visible: g.isBc && !!g.bc.locked
            anchors { right: parent.right; bottom: parent.bottom }
            width: g.size * 0.38; height: width; radius: width / 2
            color: Theme.panel
            border.color: Theme.line
            Icon { anchors.centerIn: parent; name: "lock"; size: parent.width * 0.66; color: Theme.text }
        }
        // Folder: a tab behind, the body in front.
        Rectangle {
            visible: g.kind === "folder" && !g.isBc
            x: g.size * 0.06; y: g.size * 0.14
            width: g.size * 0.42; height: g.size * 0.2
            radius: g.size * 0.06
            color: Qt.darker(Theme.accent, 1.35)
        }
        Rectangle {
            visible: g.kind === "folder" && !g.isBc
            x: g.size * 0.06; y: g.size * 0.24
            width: g.size * 0.88; height: g.size * 0.62
            radius: g.size * 0.08
            color: Theme.accent
            Rectangle {
                anchors { left: parent.left; right: parent.right; top: parent.top }
                height: Math.max(1, g.size * 0.03)
                radius: height / 2
                color: Qt.lighter(Theme.accent, 1.18)
            }
        }
        // File: a card with a folded corner and the kind's band.
        Rectangle {
            visible: g.kind !== "folder"
            x: g.size * 0.16; y: g.size * 0.06
            width: g.size * 0.68; height: g.size * 0.88
            radius: g.size * 0.08
            color: Theme.panel3
            border.width: Math.max(1, g.size * 0.025)
            border.color: Theme.line
            Rectangle {
                anchors { right: parent.right; top: parent.top }
                width: parent.width * 0.3; height: width
                radius: g.size * 0.05
                color: Theme.line
            }
            Rectangle {
                anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: parent.border.width }
                height: parent.height * 0.32
                radius: g.size * 0.06
                color: g.band
                Text {
                    anchors.centerIn: parent
                    visible: g.size >= 40 && g.label !== ""
                    text: g.label
                    color: "#ffffff"
                    font.family: Theme.font
                    font.pixelSize: Math.max(8, g.size * 0.15)
                    font.weight: Font.Bold
                }
            }
        }
        // Status in a Briefcase (Up-to-date, newer, changed in both…).
        Rectangle {
            visible: !!g.badge && g.size >= 20
            anchors { right: parent.right; top: parent.top }
            width: Math.max(12, g.size * 0.3); height: width; radius: width / 2
            color: g.badge ? g.badge.color : "transparent"
            border.color: Theme.bg
            border.width: 1
            Text { anchors.centerIn: parent; text: g.badge ? g.badge.glyph : ""; color: "#ffffff"; font.pixelSize: parent.width * 0.62; font.weight: Font.Bold }
        }
        // Shortcut (symlink) badge.
        Rectangle {
            visible: g.link
            anchors { left: parent.left; bottom: parent.bottom }
            width: g.size * 0.34; height: width; radius: width / 2
            color: Theme.panel
            border.color: Theme.line
            Text { anchors.centerIn: parent; text: "↗"; color: Theme.text; font.pixelSize: Math.max(8, g.size * 0.2) }
        }
    }

    // Places' icons: Hearth's own where one fits, otherwise a small drawn folder, drive or bin in the same tone.
    component PlaceIcon: Item {
        id: pi
        property string kind
        property color tint: Theme.text
        property real size: 20
        readonly property string hearthIcon: kind.startsWith("bc:") ? "bc-" + kind.slice(3)
            : ({ start: "home", home: "user", games: "pad", device: "desktops", network: "server", netroot: "server", howto: "help", apps: "grid" })[kind] || ""
        readonly property string mark: ({ documents: "≡", downloads: "↓", videos: "▸", pictures: "▲", music: "♪", desktop: "▭" })[kind] || ""
        // Windows 11-style: your folders in yellow, drives as drive glyphs (WinGlyph.qml draws the big ones).
        readonly property bool yellow: ["documents", "downloads", "videos", "pictures", "music", "desktop", "folder"].indexOf(kind) >= 0
        readonly property bool driveGlyph: ["usb", "sd", "drive", "system", "thispc"].indexOf(kind) >= 0
        implicitWidth: size
        implicitHeight: size
        Icon { anchors.fill: parent; visible: pi.hearthIcon !== ""; name: pi.hearthIcon; size: pi.size; color: pi.tint }
        // Yellow folder.
        Item {
            anchors.fill: parent
            visible: pi.yellow
            Rectangle { x: pi.size * 0.06; y: pi.size * 0.16; width: pi.size * 0.4; height: pi.size * 0.2; radius: Math.max(1, pi.size * 0.06); color: "#e8b23a" }
            Rectangle {
                x: pi.size * 0.06; y: pi.size * 0.26
                width: pi.size * 0.88; height: pi.size * 0.62
                radius: Math.max(1.5, pi.size * 0.1)
                gradient: Gradient {
                    GradientStop { position: 0; color: "#ffd76a" }
                    GradientStop { position: 1; color: "#e8b23a" }
                }
                Text { anchors.centerIn: parent; visible: pi.size >= 22; text: pi.mark; color: Qt.rgba(0.45, 0.28, 0.02, 0.75); font.pixelSize: pi.size * 0.42; font.weight: Font.Bold }
            }
        }
        // Drive glyph (This PC: a drive with the accent light; Hearth (C:) with Hearth's orange square).
        Rectangle {
            visible: pi.driveGlyph
            anchors.centerIn: parent
            width: pi.size * 0.92; height: pi.size * 0.66
            radius: Math.max(1.5, pi.size * 0.1)
            gradient: Gradient {
                GradientStop { position: 0; color: pi.kind === "usb" || pi.kind === "sd" ? "#cfcfcf" : "#d9d9d9" }
                GradientStop { position: 1; color: pi.kind === "usb" || pi.kind === "sd" ? "#7a7a7a" : "#9a9a9a" }
            }
            Rectangle {
                anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: parent.width * 0.2; rightMargin: parent.width * 0.2; bottomMargin: parent.height * 0.18 }
                height: Math.max(1.5, pi.size * 0.08)
                radius: height / 2
                color: Theme.accent
            }
            Rectangle {
                visible: pi.kind === "system"
                anchors { right: parent.right; top: parent.top; rightMargin: parent.width * 0.12; topMargin: parent.height * 0.14 }
                width: parent.width * 0.3; height: width; radius: Math.max(1, width * 0.2)
                color: "#e05a2b"
            }
        }
        // Folder outline (other kinds that had one).
        Item {
            anchors.fill: parent
            visible: false
            Rectangle { x: pi.size * 0.08; y: pi.size * 0.14; width: pi.size * 0.36; height: pi.size * 0.2; radius: 2; color: pi.tint }
            Rectangle {
                x: pi.size * 0.08; y: pi.size * 0.26
                width: pi.size * 0.84; height: pi.size * 0.6
                radius: pi.size * 0.1
                color: "transparent"
                border.width: Math.max(1.5, pi.size * 0.09)
                border.color: pi.tint
                Text { anchors.centerIn: parent; text: pi.mark; color: pi.tint; font.pixelSize: pi.size * 0.5; font.weight: Font.Bold }
            }
        }
        // Drive / SD card (outline; kept for anything that asks for it without the glyph).
        Rectangle {
            visible: false
            anchors.centerIn: parent
            width: pi.kind === "sd" ? pi.size * 0.64 : pi.size * 0.9
            height: pi.kind === "sd" ? pi.size * 0.84 : pi.size * 0.56
            radius: pi.size * 0.12
            color: "transparent"
            border.width: Math.max(1.5, pi.size * 0.09)
            border.color: pi.tint
            Rectangle {
                anchors { right: parent.right; rightMargin: pi.size * 0.14; verticalCenter: parent.verticalCenter }
                width: pi.size * 0.14; height: width; radius: width / 2
                color: pi.tint
            }
        }
        // Trash bin.
        Item {
            anchors.fill: parent
            visible: pi.kind === "trash"
            Rectangle { x: pi.size * 0.14; y: pi.size * 0.18; width: pi.size * 0.72; height: Math.max(1.5, pi.size * 0.09); radius: 1; color: pi.tint }
            Rectangle { x: pi.size * 0.38; y: pi.size * 0.08; width: pi.size * 0.24; height: pi.size * 0.12; radius: 1; color: pi.tint }
            Rectangle {
                x: pi.size * 0.22; y: pi.size * 0.32
                width: pi.size * 0.56; height: pi.size * 0.6
                radius: pi.size * 0.08
                color: "transparent"
                border.width: Math.max(1.5, pi.size * 0.09)
                border.color: pi.tint
            }
        }
    }
}

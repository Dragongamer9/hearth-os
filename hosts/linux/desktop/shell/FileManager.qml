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
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Widgets

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
    function properties(paths) { propsWin.show(paths); }

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
        function close(): void { app.closeOverlay(); }
    }

    // ================================================ open / close
    function open(p) {
        let target = (p || "").trim();
        if (target.startsWith("file://")) target = decodeURIComponent(target.slice(7));
        if (target.startsWith("~")) target = home + target.slice(1);
        shown = true;
        const go = () => {
            if (target) navigate(target, "");
            else if (!listing) navigate(home, "");
            else reload();
        };
        if (!prefsLoaded) loadPrefs(go);
        else go();
        refreshPlaces();
        loadTemplates();
    }
    function close() {
        shown = false;
        overlay = "";
        stopSearch();
        if (oskOpened) closeKeyboard();
    }
    function toggle() { shown ? close() : open(""); }

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
                    if (proc.cb) proc.cb(res, err);
                    proc.destroy();
                }
            }
        }
    }
    function call(args, cb) {
        const p = callComp.createObject(app, { command: ["python3", helper].concat(args), cb: cb || null });
        p.running = true;
    }

    // ================================================ settings: the view, remembered per folder
    function loadPrefs(then) {
        call(["prefs"], (r, err) => {
            prefs = r || {};
            prefsLoaded = true;
            hidden = !!prefs.hidden;
            applyFolderView("");
            if (then) then();
        });
    }
    function applyFolderView(p) {
        const f = (prefs.folders && p && prefs.folders[p]) || prefs;
        view = ["grid", "compact", "details"].indexOf(f.view) >= 0 ? f.view : (f.view === "list" ? "details" : "grid");
        sort = ["name", "date", "size", "type"].indexOf(f.sort) >= 0 ? f.sort : "name";
        desc = !!f.desc;
        group = ["none", "type", "date"].indexOf(f.group) >= 0 ? f.group : "none";
    }
    Timer { id: prefsTimer; interval: 700; onTriggered: app.call(["prefs-set", JSON.stringify(app.prefs)]) }
    function rememberView() {
        const p = Object.assign({}, prefs);
        const folders = Object.assign({}, p.folders || {});
        const v = { view: view, sort: sort, desc: desc, group: group };
        if (path) {
            delete folders[path];
            folders[path] = v;               // newest last: hearth-files keeps the newest 400
        }
        p.folders = folders;
        p.view = view; p.sort = sort; p.desc = desc; p.group = group;
        p.hidden = hidden;
        prefs = p;
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
            }
            listing = r;
            path = r.path;
            const want = r.select || sel || "";
            const i = want ? entries.findIndex(e => e.name === want) : -1;
            cur = Math.max(0, i >= 0 ? i : (r.path === prev ? Math.min(cur, entries.length - 1) : 0));
            if (r.select) zone = "main";
            makeThumbs();
        });
    }
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
        const p = listing.path;
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
            const old = places;
            places = r.places;
            placeIndex = Math.min(placeIndex, places.length - 1);
            // A drive that was taken out while you were in it: back to Home.
            for (const o of old) {
                if (o.section !== "drives" || !o.path || places.some(n => n.path === o.path)) continue;
                if (path === o.path || path.startsWith(o.path + "/")) {
                    say("“" + o.label + "” was removed");
                    navigate(home, "");
                }
            }
        });
    }
    Timer { interval: 4000; repeat: true; running: app.shown; onTriggered: app.refreshPlaces() }

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
        const all = found !== null ? found : (listing ? listing.entries : []);
        const q = found !== null ? "" : query.trim().toLowerCase();
        const out = q ? all.filter(e => e.name.toLowerCase().indexOf(q) >= 0) : all.slice();
        const by = sort, d = desc ? -1 : 1, g = group;
        out.sort((a, b) => {
            if (g !== "none") {
                const ga = groupRank(a, g), gb = groupRank(b, g);
                if (ga !== gb) return ga - gb;
            }
            if (a.dir !== b.dir) return a.dir ? -1 : 1;
            let r = 0;
            if (by === "date") r = a.mtime - b.mtime;
            else if (by === "size") r = a.size - b.size;
            else if (by === "type") r = (extOf(a) < extOf(b) ? -1 : extOf(a) > extOf(b) ? 1 : 0);
            if (r === 0) r = natural(a.name, b.name);
            return d * r;
        });
        return out;
    }
    readonly property var current: entries.length > cur && cur >= 0 ? entries[cur] : null
    readonly property bool writable: !!(listing && listing.writable) && found === null
    readonly property bool inTrash: !!(listing && listing.trash)
    readonly property string trashPath: {
        const t = places.find(p => p.kind === "trash");
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
    function parentOf(p) {
        const s = p.replace(/\/+$/, "");
        const i = s.lastIndexOf("/");
        return i <= 0 ? "/" : s.slice(0, i);
    }
    function under(p, root) { return p === root || p.startsWith(root.replace(/\/$/, "") + "/"); }
    // Where a path starts from, for the path bar and for B: the Trash, a drive, Home, or the whole computer.
    readonly property var roots: {
        const r = [{ path: trashPath, label: "Trash" }];
        for (const p of places) if (p.section === "drives" && p.path) r.push({ path: p.path, label: p.label });
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
        const t = [{ t: "back" }, { t: "forward" }, { t: "up" }];
        for (const c of crumbs) t.push({ t: "crumb", path: c.path, label: c.label });
        t.push({ t: "edit" }, { t: "search" }, { t: "view" }, { t: "sort" }, { t: "hidden" });
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
    function kindName(e) {
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
        if (k === "video" || e.mime === "application/pdf") return "file://" + thumbDir + Qt.md5(fileUrl(e.path)) + ".png";
        return "";
    }
    // Video thumbnails nobody made yet: hearth-files makes them in the background (if ffmpeg is installed).
    function makeThumbs() {
        if (!listing || view === "details") return;
        const vids = (listing.entries || []).filter(e => kindOf(e) === "video").slice(0, 30).map(e => e.path);
        if (!vids.length) return;
        const at = listing.path;
        call(["thumbs"].concat(vids), (r, err) => {
            if (r && r.made && r.made.length && listing && listing.path === at) reload();
        });
    }

    // ================================================ picking things
    function activate(e) {
        if (!e) return;
        if (inTrash) return openMenu(false, -1, -1);
        if (e.dir) {
            if (found !== null) clearSearch();
            return navigate(e.path, "");
        }
        if (isAppImage(e)) return runAppImage(e);
        if (isDeb(e)) return installDeb(e);
        say("Opening “" + e.name + "”…");
        call(["open", e.path], (r, err) => { if (err) say(err, true); });
    }
    function openPlace(i) {
        const p = places[i];
        if (!p) return;
        placeIndex = i;
        if (!p.path && p.device) return mountPlace(p, true);
        navigate(p.path, "");
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
        if (markCount) return entries.filter(e => marked[e.path]).map(e => e.path);
        return current ? [current.path] : [];
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
        if (["grid", "compact", "details"].indexOf(v) < 0) return;
        view = v;
        rememberView();
        makeThumbs();
    }
    function cycleView() { setView(({ grid: "compact", compact: "details", details: "grid" })[view]); }
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
        if (!q || !listing || inTrash) return;
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
    function openMenu(forFolder, x, y) {
        menuOnFolder = !!forFolder || !current;
        const e = forFolder ? null : current;
        const t = forFolder ? [] : targets();
        const n = t.length;
        const what = n === 1 ? "" : " (" + n + ")";
        const items = [];
        if (job) items.push({ id: "stop", label: "Stop " + jobVerbing(job.verb).toLowerCase(), danger: true }, sep());
        if (inTrash) {
            if (n) items.push({ id: "restore", label: "Put back" + what });
            if (n) items.push({ id: "delete", label: "Delete permanently" + what, danger: true, key: "Del" });
            items.push({ id: "empty", label: "Empty Trash", danger: true });
            if (e) items.push(sep(), { id: "props", label: "Properties", key: "Alt+Enter" });
        } else {
            if (e) items.push({ id: "open", label: "Open", key: "Enter" });
            if (e && !e.dir) items.push({ id: "openwith", label: "Open with…" });
            if (e && e.dir && n === 1) items.push({ id: "newtab", label: "Open in new tab" });
            if (n === 1 && isAppImage(e)) items.push({ id: "airun", label: "Run" }, e.inMenu ? { id: "aimenurm", label: "Remove from app menu" } : { id: "aimenu", label: "Add to app menu" });
            if (n === 1 && isDeb(e)) items.push({ id: "debinstall", label: "Install" });
            if (e && found !== null) items.push({ id: "showin", label: "Show in its folder" });
            if (e && n === 1 && kindOf(e) !== "folder") items.push({ id: "preview", label: "Preview", key: "Space" });
            items.push(sep());
            if (n) items.push({ id: "cut", label: "Cut" + what, key: "Ctrl+X", off: !(n && writable) });
            if (n) items.push({ id: "copy", label: "Copy" + what, key: "Ctrl+C" });
            if (writable) items.push({ id: "paste", label: (clip.mode === "cut" ? "Move here" : "Paste") + (clip.paths.length > 1 ? " (" + clip.paths.length + ")" : ""), key: "Ctrl+V" });
            items.push({ id: "copypath", label: n > 1 ? "Copy paths" : "Copy path", key: "Ctrl+Alt+C" });
            items.push(sep());
            if (n === 1 && writable) items.push({ id: "rename", label: "Rename…", key: "F2" });
            if (n > 1 && writable) items.push({ id: "batchrename", label: "Rename " + n + " items…", key: "F2" });
            if (n && writable && !(n === 1 && isArchive(e))) items.push({ id: "compress", label: "Compress…" });
            if (n === 1 && isArchive(e) && writable) items.push({ id: "extracthere", label: "Extract here" });
            if (n === 1 && isArchive(e)) items.push({ id: "extractto", label: "Extract to…" });
            if (n > 1 && writable && t.every(p => isArchive(entries.find(x => x.path === p)))) items.push({ id: "extracteach", label: "Extract " + n + " archives here" });
            if (n === 1 && isArchive(e) && writable) items.push({ id: "compress", label: "Compress…" });
            if (e && kindOf(e) === "image") items.push({ id: "wallpaper", label: "Set as wallpaper" });
            if (!e && writable) {
                items.push({ id: "newfolder", label: "New folder…", key: "Ctrl+Shift+N" });
                items.push({ id: "newfile", label: "New empty file…" });
                if (templates.length) items.push({ id: "sub-new", label: "New document  ›" });
            }
            items.push(sep());
            if (n && writable) items.push({ id: "trash", label: "Move to Trash" + what, key: "Del" });
            if (n && writable) items.push({ id: "delete", label: "Delete permanently" + what, danger: true, key: "Shift+Del" });
            items.push(sep());
            if (e) items.push({ id: "mark", label: marked[e.path] ? "Unselect" : "Select", key: "Ctrl+Space" });
            if (markCount) items.push({ id: "unmarkall", label: "Clear selection", key: "Esc" });
            if (entries.length) items.push({ id: "markall", label: "Select all", key: "Ctrl+A" });
            if (markCount) items.push({ id: "invert", label: "Invert selection", key: "Ctrl+I" });
            items.push(sep());
            if (!e) items.push({ id: "sub-view", label: "View  ›" }, { id: "sub-sort", label: "Sort and group  ›" });
            if (!e) items.push({ id: "hidden", label: hidden ? "Hide hidden files" : "Show hidden files", key: "Ctrl+H" });
            if (!e) items.push({ id: "newtab", label: "New tab", key: "Ctrl+T" });
            if (tabs.length > 1 && !e) items.push({ id: "closetab", label: "Close tab", key: "Ctrl+W" });
            const bmPath = e ? (e.dir ? e.path : "") : (listing ? listing.path : "");
            if (bmPath && !places.some(p => p.bookmark && p.path === bmPath)) items.push({ id: "bookmark", label: "Add to bookmarks", key: "Ctrl+D" });
            if (!e || e.dir) items.push({ id: "terminal", label: "Open terminal here", key: "Shift+F4" });
            items.push(sep(), { id: "props", label: e ? "Properties" : "Folder properties", key: "Alt+Enter" });
        }
        const title = e ? (n > 1 ? plural(n, "item") + " selected" : e.name) : (listing ? (listing.name || "Folder") : "Folder");
        showMenu(title, items.filter(i => !i.off), x === undefined ? -1 : x, y === undefined ? -1 : y);
    }
    function openPlaceMenu() {
        const p = places[placeIndex];
        if (!p) return;
        menuOnFolder = false;
        const items = [];
        if (p.path) items.push({ id: "p-open", label: "Open" }, { id: "p-newtab", label: "Open in new tab" });
        if (!p.path && p.device) items.push({ id: "p-mount", label: "Mount" });
        if (p.section === "drives" && p.path) items.push({ id: "p-unmount", label: "Unmount" }, { id: "p-eject", label: "Safely remove" });
        if (p.bookmark) items.push({ id: "p-unbookmark", label: "Remove bookmark", danger: true });
        if (p.kind === "trash") items.push({ id: "empty", label: "Empty Trash", danger: true });
        if (p.path) items.push(sep(), { id: "p-props", label: "Properties" });
        showMenu(p.label, items, -1, -1);
    }
    function sortMenu() {
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
        ]);
    }
    function viewMenu() {
        const ck = on => on ? "✓  " : "     ";
        showMenu("View", [
            { id: "view:grid", label: ck(view === "grid") + "Icons", key: "Ctrl+1" },
            { id: "view:compact", label: ck(view === "compact") + "Compact", key: "Ctrl+2" },
            { id: "view:details", label: ck(view === "details") + "Details", key: "Ctrl+3" },
            sep(),
            { id: "hidden", label: hidden ? "Hide hidden files" : "Show hidden files", key: "Ctrl+H" }
        ]);
    }
    function runMenu(id) {
        const it = menuItems.find(i => i.id === id);
        if (it && it.sep) return;
        overlay = "";
        const onFolder = menuOnFolder;
        menuOnFolder = false;
        const t = onFolder ? [] : targets();
        const e = onFolder ? null : current;
        if (id.startsWith("sort:")) return setSort(id.slice(5));
        if (id.startsWith("group:")) return setGroup(id.slice(6));
        if (id.startsWith("view:")) return setView(id.slice(5));
        if (id.startsWith("tpl:")) return newFromTemplate(templates[parseInt(id.slice(4), 10)]);
        switch (id) {
        case "stop": cancelJob(); break;
        case "open": activate(e); break;
        case "airun": if (e) runAppImage(e); break;
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
        case "copypath": copyText(t.length ? t.join("\n") : (listing ? listing.path : "")); break;
        case "rename":
            if (e) askText("Rename", e.name, "Rename", e.dir ? e.name.length : (e.name.lastIndexOf(".") > 0 ? e.name.lastIndexOf(".") : e.name.length),
                text => app.call(["rename", e.path, text], (r, err) => {
                    if (err) return app.say(err, true);
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
        case "bookmark": {
            const bp = e && e.dir ? e.path : listing.path;
            call(["bookmark-add", bp], (r, err) => {
                if (err) return say(err, true);
                say("Added “" + base(bp) + "” to your bookmarks");
                refreshPlaces();
            });
            break;
        }
        case "terminal":
            call(["terminal", e && e.dir ? e.path : listing.path], (r, err) => { if (err) say(err, true); });
            break;
        case "props": properties(t.length ? t : [listing ? listing.path : home]); break;
        case "p-open": openPlace(placeIndex); break;
        case "p-newtab": if (places[placeIndex]) newTab(places[placeIndex].path); break;
        case "p-props": if (places[placeIndex]) properties([places[placeIndex].path]); break;
        case "p-mount": if (places[placeIndex]) mountPlace(places[placeIndex], false); break;
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
                app.load(dir, app.base(r.path), "none");
            });
        });
    }
    function trashPaths(t) {
        if (!t.length) return;
        call(["trash"].concat(t), (r, err) => {
            if (err) return say(err, true);
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
    function paste() {
        if (!writable) return;
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
        dragUris = t.map(p => fileUrl(p)).join("\r\n") + "\r\n";
        return true;
    }
    function dropOn(drop, dest) {
        const paths = (drop.urls || []).map(u => urlToPath(u)).filter(p => p && p !== dest);
        if (!paths.length || !dest) return;
        drop.accept(drop.proposedAction === Qt.CopyAction ? Qt.CopyAction : Qt.MoveAction);
        if (dest === trashPath) return trashPaths(paths);
        const copy = drop.proposedAction === Qt.CopyAction || rootOf(paths[0]).path !== rootOf(dest).path;
        const list = copy ? paths : paths.filter(p => parentOf(p) !== dest);
        if (!list.length) return;
        transferTo(copy ? "copy" : "move", list, dest);
    }

    // ================================================ dialogs
    function askText(title, initial, ok, selectTo, action) {
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
        closeOverlay();
        if (!act) return;
        if (kind === "input") {
            const t = (text || "").trim();
            if (t) act(t);
        } else {
            act();
        }
    }
    function closeOverlay() {
        overlay = "";
        menuOnFolder = false;
        dlgAction = null;
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
            else {
                const skipped = r.skipped && r.skipped.length ? " (skipped " + r.skipped.length + ")" : "";
                say((j.verb === "move" ? "Moved " : "Copied ") + what + " to “" + j.destName + "”" + skipped);
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
            title: app.path ? app.base(app.path) + " – Files" : "Files"
            implicitWidth: 1180
            implicitHeight: 740
            color: Theme.bg
            visible: true
            onVisibleChanged: if (!visible) app.close()
            Component.onCompleted: {
                app.ui = win;
                keys.forceActiveFocus();
            }
            Component.onDestruction: if (app.ui === win) app.ui = null

            function refocus() { keys.forceActiveFocus(); }
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
                pathField.text = app.path;
                pathField.forceActiveFocus();
                pathField.selectAll();
            }
            function endEditPath(go) {
                const t = pathField.text.trim();
                app.editingPath = false;
                keys.forceActiveFocus();
                if (!go || !t) return;
                let p = t.startsWith("file://") ? decodeURIComponent(t.slice(7)) : t;
                if (p.startsWith("~")) p = app.home + p.slice(1);
                if (!p.startsWith("/")) p = app.path + "/" + p;
                app.navigate(p, "");
            }
            readonly property int cols: app.view === "details" ? 1 : Math.max(1, Math.floor(grid.width / grid.cellWidth))
            function showCurrent() {
                if (app.view === "details") list.positionViewAtIndex(app.cur, ListView.Contain);
                else grid.positionViewAtIndex(app.cur, GridView.Contain);
            }
            function wheel(ev) {
                const v = app.view === "details" ? list : grid;
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
                Drag.supportedActions: Qt.CopyAction | Qt.MoveAction
                Drag.proposedAction: Qt.MoveAction
                Drag.mimeData: ({ "text/uri-list": app.dragUris })
                Drag.onDragFinished: dropAction => { dragProxy.x = 0; dragProxy.y = 0; }
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
                        case Qt.Key_Return: case Qt.Key_Enter: app.runMenu("props"); break;
                        default: return;
                        }
                        e.accepted = true;
                        return;
                    }
                    if (ctrl) {
                        switch (e.key) {
                        case Qt.Key_C: if (alt) app.runMenu("copypath"); else app.setClip("copy", app.targets()); break;
                        case Qt.Key_X: app.setClip("cut", app.targets()); break;
                        case Qt.Key_V: app.paste(); break;
                        case Qt.Key_A: app.selectAll(); break;
                        case Qt.Key_I: app.invertSelection(); break;
                        case Qt.Key_H: app.setHidden(!app.hidden); break;
                        case Qt.Key_F: win.focusSearch(); break;
                        case Qt.Key_L: win.editPath(); break;
                        case Qt.Key_R: app.reload(); break;
                        case Qt.Key_D: app.runMenu("bookmark"); break;
                        case Qt.Key_T: app.newTab(app.path); break;
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
                    if (app.tabFocus >= app.tabs.length) app.newTab(app.path);
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
                    if (app.tabs.length > 1) { app.tabFocus = app.tabIndex; app.zone = "tabs"; }
                    break;
                case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: {
                    if (!it) break;
                    if (it.t === "back") app.goBack();
                    else if (it.t === "forward") app.goForward();
                    else if (it.t === "up") app.goUp(false);
                    else if (it.t === "crumb") app.navigate(it.path, "");
                    else if (it.t === "edit") win.editPath();
                    else if (it.t === "search") win.focusSearch();
                    else if (it.t === "view") app.cycleView();
                    else if (it.t === "sort") app.sortMenu();
                    else if (it.t === "hidden") app.setHidden(!app.hidden);
                    break;
                }
                default: return;
                }
                e.accepted = true;
            }
            function mainKey(e, shift) {
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
                    if (app.view === "details" || i % c === 0) { if (!shift) win.toPlaces(); }
                    else go(i - 1);
                    break;
                case Qt.Key_Right:
                    if (app.view !== "details" && i + 1 < count && (i + 1) % c !== 0) go(i + 1);
                    break;
                case Qt.Key_Up:
                    if (i - c >= 0) go(i - c);
                    else if (!shift) win.toToolbar();
                    break;
                case Qt.Key_Down:
                    if (i + c < count) go(i + c);
                    else if (Math.floor(i / c) < Math.floor((count - 1) / c)) go(count - 1);
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
                    // Typing filters this folder (with the on-screen keyboard too: Y, then type).
                    if (e.text && e.text.length === 1 && e.text >= " " && !(e.modifiers & (Qt.AltModifier | Qt.MetaModifier))) {
                        if (e.text === "/" || e.text === "~") { win.editPath(); pathField.text = e.text === "~" ? "~/" : "/"; break; }
                        win.focusSearch(app.query + e.text);
                        break;
                    }
                    return;
                }
                e.accepted = true;
            }
            function toPlaces() {
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
                if (app.overlay === "menu") {
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
                    if (e.key === Qt.Key_Left || up) app.dlgButton = 0;
                    else if (e.key === Qt.Key_Right || down) app.dlgButton = 1;
                    else if (ok) { if (app.dlgButton === 1) app.accept(""); else app.closeOverlay(); }
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

            RowLayout {
                anchors.fill: parent
                spacing: 0

                // ------------------------------------------------ places
                Rectangle {
                    Layout.fillHeight: true
                    Layout.preferredWidth: win.width < 900 ? 220 : 260
                    color: Theme.panel
                    ColumnLayout {
                        anchors { fill: parent; margins: 14 }
                        spacing: 6
                        RowLayout {
                            Layout.bottomMargin: 10; Layout.leftMargin: 6; Layout.topMargin: 6
                            spacing: 10
                            Image { source: Qt.resolvedUrl("images/hearth-logo.png"); Layout.preferredWidth: 30; Layout.preferredHeight: 30; fillMode: Image.PreserveAspectFit; smooth: true }
                            Text { text: "Files"; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold }
                        }
                        ListView {
                            id: placeList
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            model: app.places
                            spacing: 4
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            keyNavigationEnabled: false
                            delegate: Item {
                                id: placeItem
                                required property var modelData
                                required property int index
                                // A heading where a new group starts (Bookmarks, Games, Drives and SD cards; a gap before Trash).
                                readonly property bool starts: index > 0 && app.places[index - 1].section !== modelData.section
                                readonly property real headH: !starts ? 0 : modelData.section === "trash" ? 12 : 32
                                width: ListView.view.width
                                height: headH + place.height
                                Text {
                                    visible: placeItem.headH > 20
                                    anchors { left: parent.left; leftMargin: 14; top: parent.top; topMargin: 12 }
                                    text: ({ games: "Games", bookmarks: "Bookmarks" })[placeItem.modelData.section] || "Drives and SD cards"
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold
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
                                    height: modelData.section === "drives" ? 56 : 44
                                    radius: 10
                                    color: lit ? Theme.accent : placeDrop.containsDrag ? Theme.panel3 : here ? Theme.panel3 : placeMouse.containsMouse ? Theme.panel2 : "transparent"
                                    border.width: placeDrop.containsDrag ? 2 : 0
                                    border.color: Theme.accent
                                    opacity: modelData.mounted === false ? 0.75 : 1
                                    Behavior on color { ColorAnimation { duration: Theme.quick } }
                                    RowLayout {
                                        anchors { fill: parent; leftMargin: place.modelData.child ? 34 : 14; rightMargin: 12 }
                                        spacing: 12
                                        PlaceIcon { kind: place.modelData.kind; tint: place.ink; size: place.modelData.child ? 16 : 20 }
                                        ColumnLayout {
                                            Layout.fillWidth: true
                                            spacing: 1
                                            Text {
                                                Layout.fillWidth: true
                                                text: place.modelData.label
                                                color: place.lit ? Theme.accentInk : Theme.text
                                                elide: Text.ElideRight
                                                font.family: Theme.font; font.pixelSize: place.modelData.child ? 13 : 15; font.weight: place.here || place.lit ? Font.DemiBold : Font.Normal
                                            }
                                            Text {
                                                Layout.fillWidth: true
                                                visible: place.modelData.section === "drives"
                                                text: place.modelData.mounted === false ? (place.modelData.sub || "Not mounted")
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

                    // Tabs (when there's more than one).
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: app.tabs.length > 1 ? 46 : 0
                        visible: app.tabs.length > 1
                        color: Theme.bg2
                        Row {
                            anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter }
                            spacing: 6
                            Repeater {
                                model: app.tabs
                                delegate: Rectangle {
                                    id: tabChip
                                    required property var modelData
                                    required property int index
                                    readonly property bool on: index === app.tabIndex
                                    readonly property bool lit: app.zone === "tabs" && app.tabFocus === index
                                    width: Math.min(220, tabLabel.implicitWidth + 58)
                                    height: 34
                                    radius: 9
                                    color: on ? Theme.panel3 : tabMouse.containsMouse ? Theme.panel2 : "transparent"
                                    border.width: 2
                                    border.color: lit ? Theme.accent : "transparent"
                                    Text {
                                        id: tabLabel
                                        anchors { left: parent.left; leftMargin: 14; right: tabX.left; rightMargin: 6; verticalCenter: parent.verticalCenter }
                                        text: app.base(tabChip.on ? app.path : tabChip.modelData.path || app.home)
                                        color: Theme.text
                                        elide: Text.ElideMiddle
                                        font.family: Theme.font; font.pixelSize: 13; font.weight: tabChip.on ? Font.DemiBold : Font.Normal
                                    }
                                    MouseArea {
                                        id: tabMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                                        onClicked: mouse => { keys.forceActiveFocus(); if (mouse.button === Qt.MiddleButton) app.closeTab(tabChip.index); else app.switchTab(tabChip.index); }
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
                                width: 34; height: 34; radius: 9
                                color: newTabMouse.containsMouse ? Theme.panel2 : "transparent"
                                border.width: 2
                                border.color: app.zone === "tabs" && app.tabFocus >= app.tabs.length ? Theme.accent : "transparent"
                                Text { anchors.centerIn: parent; text: "+"; color: Theme.text; font.pixelSize: 18 }
                                MouseArea { id: newTabMouse; anchors.fill: parent; hoverEnabled: true; onClicked: { keys.forceActiveFocus(); app.newTab(app.path); } }
                            }
                        }
                    }

                    // Back, forward, up, the path (click the empty part, Ctrl+L or / to type one), search, view, sort, hidden.
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.margins: 14
                        Layout.bottomMargin: 8
                        spacing: 6
                        Chip { icon: "back"; label: ""; dim: !app.back.length; focused: app.toolLit("back"); onClicked: { keys.forceActiveFocus(); app.goBack(); } }
                        Chip { icon: "forward"; label: ""; dim: !app.fwd.length; focused: app.toolLit("forward"); onClicked: { keys.forceActiveFocus(); app.goForward(); } }
                        Chip { label: "↑"; dim: app.path === "/" ; focused: app.toolLit("up"); onClicked: { keys.forceActiveFocus(); app.goUp(false); } }
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 40
                            radius: 10
                            color: app.editingPath ? Theme.panel2 : "transparent"
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
                                                focused: app.zone === "toolbar" && app.toolIndex === 3 + crumb.index
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
                            radius: 10
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
                                    text: app.found !== null ? "Found “" + app.foundQuery + "”" : "Filter · Enter searches inside"
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
                        Chip {
                            icon: app.view === "details" ? "menu" : "grid"
                            label: ({ grid: "Icons", compact: "Compact", details: "Details" })[app.view]
                            focused: app.toolLit("view")
                            onClicked: { keys.forceActiveFocus(); app.cycleView(); }
                        }
                        Chip {
                            label: ({ name: "Name", date: "Modified", size: "Size", type: "Type" })[app.sort] + (app.desc ? " ↓" : " ↑")
                            focused: app.toolLit("sort")
                            onClicked: { keys.forceActiveFocus(); app.viaPad = false; app.sortMenu(); }
                        }
                        Chip {
                            label: "Hidden"
                            dot: app.hidden
                            focused: app.toolLit("hidden")
                            onClicked: { keys.forceActiveFocus(); app.setHidden(!app.hidden); }
                        }
                    }

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
                                const v = app.view === "details" ? list : grid;
                                return v.contentItem.mapFromItem(bgMouse, x, y);
                            }
                            onPressed: mouse => {
                                keys.forceActiveFocus();
                                app.zone = "main";
                                if (mouse.button !== Qt.LeftButton || !app.mouseMode) return;
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

                        // Icons and Compact: a grid (Compact: small icons in columns).
                        GridView {
                            id: grid
                            visible: app.view !== "details"
                            anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                            model: app.entries
                            clip: true
                            interactive: !app.mouseMode
                            boundsBehavior: Flickable.StopAtBounds
                            cellWidth: app.view === "compact" ? Math.floor(width / Math.max(1, Math.floor(width / 250))) : Math.floor(width / Math.max(1, Math.floor(width / 136)))
                            cellHeight: app.view === "compact" ? 44 : 156
                            keyNavigationEnabled: false
                            cacheBuffer: 600
                            delegate: Item {
                                id: cell
                                required property var modelData
                                required property int index
                                readonly property bool lit: app.cur === index
                                readonly property bool picked: !!app.marked[modelData.path]
                                readonly property bool cut: app.clip.mode === "cut" && app.clip.paths.indexOf(modelData.path) >= 0
                                readonly property bool compact: app.view === "compact"
                                width: grid.cellWidth
                                height: grid.cellHeight
                                opacity: cut ? 0.45 : 1
                                Rectangle {
                                    anchors { fill: parent; margins: cell.compact ? 2 : 4 }
                                    radius: cell.compact ? 8 : 12
                                    color: cellDrop.containsDrag ? Theme.panel3 : cell.picked ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22)
                                         : cell.lit ? Theme.panel3 : cellMouse.containsMouse ? Theme.panel2 : "transparent"
                                    border.width: 2
                                    border.color: cellDrop.containsDrag || (cell.lit && app.zone === "main" && app.overlay === "") ? Theme.accent : "transparent"
                                    Behavior on border.color { ColorAnimation { duration: Theme.quick } }
                                }
                                // Icons
                                Item {
                                    id: art
                                    visible: !cell.compact
                                    anchors { top: parent.top; topMargin: 14; horizontalCenter: parent.horizontalCenter }
                                    width: 84; height: 76
                                    FileGlyph {
                                        anchors.centerIn: parent
                                        visible: thumb.status !== Image.Ready
                                        kind: app.kindOf(cell.modelData)
                                        label: app.extLabel(cell.modelData)
                                        link: cell.modelData.isLink
                                        size: 64
                                    }
                                    Rectangle {
                                        anchors.fill: parent
                                        visible: thumb.status === Image.Ready
                                        radius: 8
                                        color: Theme.panel3
                                        Image {
                                            id: thumb
                                            anchors { fill: parent; margins: 3 }
                                            source: cell.compact ? "" : app.thumbFor(cell.modelData)
                                            sourceSize: Qt.size(192, 192)
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
                                    text: cell.modelData.name
                                    color: cell.modelData.hidden ? Theme.muted : Theme.text
                                    wrapMode: Text.WrapAnywhere
                                    maximumLineCount: 2
                                    elide: Text.ElideRight
                                    font.family: Theme.font; font.pixelSize: 13; font.weight: cell.lit ? Font.DemiBold : Font.Normal
                                }
                                // Compact
                                RowLayout {
                                    visible: cell.compact
                                    anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                                    spacing: 10
                                    Item {
                                        Layout.preferredWidth: 26; Layout.preferredHeight: 26
                                        FileGlyph {
                                            anchors.centerIn: parent
                                            visible: smallThumb.status !== Image.Ready
                                            kind: app.kindOf(cell.modelData)
                                            label: ""
                                            link: cell.modelData.isLink
                                            size: 24
                                        }
                                        Image {
                                            id: smallThumb
                                            anchors.fill: parent
                                            source: cell.compact && app.kindOf(cell.modelData) === "image" ? app.thumbFor(cell.modelData) : ""
                                            sourceSize: Qt.size(52, 52)
                                            asynchronous: true
                                            fillMode: Image.PreserveAspectCrop
                                            clip: true
                                        }
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: cell.modelData.name + (cell.modelData.where && cell.modelData.where !== "." ? "  ·  " + cell.modelData.where : "")
                                        color: cell.modelData.hidden ? Theme.muted : Theme.text
                                        elide: Text.ElideMiddle
                                        font.family: Theme.font; font.pixelSize: 13; font.weight: cell.lit ? Font.DemiBold : Font.Normal
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
                            visible: app.view === "details"
                            anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                            model: app.entries
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
                                    radius: 10
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
                                            text: rowItem.modelData.name + (rowItem.modelData.origPath ? "   ·   from " + app.parentOf(rowItem.modelData.origPath)
                                                : rowItem.modelData.where && rowItem.modelData.where !== "." ? "   ·   " + rowItem.modelData.where : "")
                                            color: rowItem.modelData.hidden ? Theme.muted : Theme.text
                                            elide: Text.ElideMiddle
                                            font.family: Theme.font; font.pixelSize: 14; font.weight: row.lit ? Font.DemiBold : Font.Normal
                                        }
                                        Icon { visible: row.picked; name: "check"; size: 16; color: Theme.accent }
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

                        // Empty, loading, searching, or can't open.
                        Column {
                            anchors.centerIn: parent
                            width: Math.min(parent.width - 80, 460)
                            spacing: 8
                            visible: app.entries.length === 0
                            Text {
                                width: parent.width
                                horizontalAlignment: Text.AlignHCenter
                                wrapMode: Text.WordWrap
                                text: app.listing && app.listing.error ? "Can't open this folder"
                                    : app.loading && !app.listing ? "Opening…"
                                    : app.found !== null ? (app.searching ? "Searching for “" + app.foundQuery + "”…" : "Nothing inside matches “" + app.foundQuery + "”")
                                    : app.query ? "Nothing here matches “" + app.query + "”"
                                    : app.inTrash ? "The Trash is empty" : "This folder is empty"
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
                                        : app.plural(app.entries.length, "item");
                                    if (app.markCount) {
                                        const sel = app.entries.filter(e => app.marked[e.path]);
                                        const bytes = sel.reduce((n, e) => n + (e.dir ? 0 : e.size), 0);
                                        s += " · " + app.markCount + " selected" + (bytes ? " (" + app.fmtSize(bytes) + ")" : "");
                                    } else if (app.current && !app.current.dir && app.found === null) {
                                        s += " · “" + app.current.name + "” " + app.fmtSize(app.current.size);
                                    }
                                    if (app.clip.paths.length) s += " · " + app.plural(app.clip.paths.length, "item") + (app.clip.mode === "cut" ? " to move" : " copied");
                                    if (app.listing && !app.listing.writable && !app.listing.error) s += " · read-only";
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
                const v = app.view === "details" ? list : grid;
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

                // Actions menu.
                Rectangle {
                    id: menuCard
                    visible: app.overlay === "menu"
                    width: 320
                    height: Math.min(parent.height - 16, menuHead.height + menuList.contentHeight + 20)
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
                        anchors { left: parent.left; right: parent.right; top: menuHead.bottom; bottom: parent.bottom; margins: 8; topMargin: 0 }
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
                            height: modelData.sep ? 11 : 40
                            Rectangle {
                                visible: !!mi.modelData.sep
                                anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 10; rightMargin: 10 }
                                height: 1
                                color: Theme.line
                            }
                            Rectangle {
                                visible: !mi.modelData.sep
                                anchors.fill: parent
                                radius: 10
                                color: mi.lit ? (mi.modelData.danger ? Theme.danger : Theme.accent) : miMouse.containsMouse ? Theme.panel3 : "transparent"
                                Text {
                                    anchors { left: parent.left; leftMargin: 14; right: miKey.left; rightMargin: 8; verticalCenter: parent.verticalCenter }
                                    text: mi.modelData.label || ""
                                    elide: Text.ElideRight
                                    color: mi.lit ? (mi.modelData.danger ? "#ffffff" : Theme.accentInk) : mi.modelData.danger ? Theme.danger : Theme.text
                                    font.family: Theme.font; font.pixelSize: 14; font.weight: mi.lit ? Font.DemiBold : Font.Normal
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
                }

                // Dialogs: text box, confirm, open with, name clash, preview.
                Rectangle {
                    id: card
                    visible: app.overlay !== "" && app.overlay !== "menu"
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
                            radius: 12
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
                                radius: 10
                                color: Theme.panel2
                                border.width: 2
                                border.color: app.cf && app.cf.button === 3 ? Theme.accent : "transparent"
                                RowLayout {
                                    anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                                    spacing: 10
                                    Rectangle {
                                        Layout.preferredWidth: 22; Layout.preferredHeight: 22; radius: 6
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
                                Rectangle { anchors.fill: parent; color: Theme.bg2; radius: 10; z: -1 }
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
                                radius: 10
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
        radius: 10
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
        readonly property color band: ({ image: "#5fb98f", video: "#b36ad9", audio: "#4f9dde", archive: "#b98a5a", disc: "#8c7ae6",
            app: Theme.ember, doc: "#e06666", text: "#a69484", file: "#7d6a5c" })[kind] || "#7d6a5c"
        width: size
        height: size
        // Folder: a tab behind, the body in front.
        Rectangle {
            visible: g.kind === "folder"
            x: g.size * 0.06; y: g.size * 0.14
            width: g.size * 0.42; height: g.size * 0.2
            radius: g.size * 0.06
            color: Qt.darker(Theme.accent, 1.35)
        }
        Rectangle {
            visible: g.kind === "folder"
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
        readonly property string hearthIcon: ({ home: "user", desktop: "display", pictures: "cam", music: "vol", games: "pad" })[kind] || ""
        readonly property string mark: ({ documents: "≡", downloads: "↓", videos: "▸" })[kind] || ""
        implicitWidth: size
        implicitHeight: size
        Icon { anchors.fill: parent; visible: pi.hearthIcon !== ""; name: pi.hearthIcon; size: pi.size; color: pi.tint }
        // Folder outline.
        Item {
            anchors.fill: parent
            visible: pi.hearthIcon === "" && (pi.kind === "documents" || pi.kind === "downloads" || pi.kind === "videos" || pi.kind === "folder")
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
        // Drive / SD card.
        Rectangle {
            visible: pi.kind === "usb" || pi.kind === "sd" || pi.kind === "drive"
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

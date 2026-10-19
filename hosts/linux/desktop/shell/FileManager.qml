// Hearth Files: Desktop Mode's file manager. A normal window (labwc decorates and tiles it like any app) with
// your places on the left and the folder on the right, as a grid or a list. Works with mouse, touch,
// keyboard and the controller, like Hearth Settings:
//   arrows move · A / Enter opens (folders go in, files open in their app) · B / Backspace goes up (B at
//   Home closes) · X / the Menu key: actions (copy, paste, rename, Trash, Open with…) · Y: on-screen
//   keyboard, and typing searches this folder · Up from the top row reaches the path, search and view
//   buttons; Left from the first column reaches your places.
// Mouse and touch: click selects, double-click / double-tap opens, right-click or press-and-hold for actions.
// Copying and moving big folders (games to an SD card) runs in hearth-files, with progress at the bottom.
// Opened from the app launcher, the taskbar, "Open folder" in other apps (xdg-open), or:
//   quickshell ipc -p <shell folder> call files open <path>     (or: home, toggle, close)
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

    // ------------------------------------------------ what's shown
    property string path: ""
    property var listing: null                 // hearth-files list: {path, entries, writable, trash, free, total, error?}
    property var places: []
    property bool loading: false
    property int loadSeq: 0
    property string view: "grid"               // grid | list
    property string sort: "name"               // name | date | size
    property bool hidden: false
    property string query: ""
    property var marked: ({})                  // path → true (several picked with Select / Space)
    readonly property int markCount: Object.keys(marked).length
    property var clip: ({ mode: "", paths: [] })   // copy | cut

    // ------------------------------------------------ focus: places | toolbar | main, plus an overlay on top
    property string zone: "main"
    property int cur: 0
    property int placeIndex: 0
    property int toolIndex: 0
    property string overlay: ""                // "" | menu | input | confirm | props | apps
    property var menuItems: []
    property int menuIndex: 0
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
    property int dlgIndex: 0
    property string dlgPath: ""

    // ------------------------------------------------ long copy / move
    property var job: null                     // {verb, count, name, destName, done, total, speed, file, cancelling}
    property var jobQueue: []
    property string jobLast: ""

    // ------------------------------------------------ messages in the bottom bar
    property string note: ""
    property bool noteError: false
    Timer { id: noteTimer; interval: 4500; onTriggered: app.note = "" }
    function say(t, err) { note = t; noteError = !!err; noteTimer.restart(); }

    // ================================================ open / close
    function open(p) {
        let target = (p || "").trim();
        if (target.startsWith("file://")) target = decodeURIComponent(target.slice(7));
        if (target.startsWith("~")) target = home + target.slice(1);
        shown = true;
        if (target) navigate(target, "");
        else if (!listing) navigate(home, "");
        else reload();
        refreshPlaces();
    }
    function close() {
        shown = false;
        overlay = "";
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

    function navigate(p, sel) {
        zone = "main";
        load(p, sel);
    }
    function reload() {
        if (listing) load(listing.path, current ? current.name : "");
    }
    function load(p, sel) {
        const seq = ++loadSeq;
        loading = true;
        call(["list", p].concat(hidden ? ["--hidden"] : []), (r, err) => {
            if (seq !== loadSeq) return;
            loading = false;
            const prev = listing ? listing.path : "";
            if (err) r = { path: p, name: base(p), parent: parentOf(p), entries: [], writable: false, trash: false, error: err };
            listing = r;
            path = r.path;
            if (r.path !== prev) {
                query = "";
                marked = ({});
            }
            const want = r.select || sel || "";
            const i = want ? entries.findIndex(e => e.name === want) : -1;
            cur = Math.max(0, i >= 0 ? i : (r.path === prev ? Math.min(cur, entries.length - 1) : 0));
            if (r.select) zone = "main";
        });
    }
    function refreshPlaces() {
        call(["places"], (r, err) => {
            if (!r) return;
            const old = places;
            places = r.places;
            placeIndex = Math.min(placeIndex, places.length - 1);
            // A drive that was taken out while you were in it: back to Home.
            for (const o of old) {
                if (o.section !== "drives" || places.some(n => n.path === o.path)) continue;
                if (path === o.path || path.startsWith(o.path + "/")) {
                    say("“" + o.label + "” was removed");
                    navigate(home, "");
                }
            }
        });
    }
    Timer { interval: 4000; repeat: true; running: app.shown; onTriggered: app.refreshPlaces() }

    // ================================================ the folder's entries, filtered and sorted
    readonly property var entries: {
        const all = listing ? listing.entries : [];
        const q = query.trim().toLowerCase();
        const out = q ? all.filter(e => e.name.toLowerCase().indexOf(q) >= 0) : all.slice();
        const by = sort;
        out.sort((a, b) => {
            if (a.dir !== b.dir) return a.dir ? -1 : 1;
            if (by === "date" && a.mtime !== b.mtime) return b.mtime - a.mtime;
            if (by === "size" && a.size !== b.size) return b.size - a.size;
            return natural(a.name, b.name);
        });
        return out;
    }
    readonly property var current: entries.length > cur && cur >= 0 ? entries[cur] : null
    readonly property bool writable: !!(listing && listing.writable)
    readonly property bool inTrash: !!(listing && listing.trash)
    readonly property string trashPath: {
        const t = places.find(p => p.kind === "trash");
        return t ? t.path : home + "/.local/share/Trash/files";
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
        for (const p of places) if (p.section === "drives") r.push({ path: p.path, label: p.label });
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
    readonly property var tools: ["search", "view", "sort", "hidden"]
    readonly property int toolCount: crumbs.length + tools.length

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
    function kindName(e, mime) {
        const m = mime || (e ? e.mime : "") || "";
        const k = kindOf(Object.assign({}, e || {}, { mime: m }));
        const ext = e && !e.dir && e.name.indexOf(".") > 0 ? " (" + e.name.slice(e.name.lastIndexOf(".") + 1).toUpperCase() + ")" : "";
        return ({ folder: "Folder", image: "Picture", video: "Video", audio: "Audio", disc: "Disc image", archive: "Archive",
            app: "App or program", doc: "Document", text: "Text", file: "File" })[k] + (k === "folder" ? "" : ext);
    }
    function extLabel(e) {
        if (!e || e.dir) return "";
        const i = e.name.lastIndexOf(".");
        const x = i > 0 ? e.name.slice(i + 1).toUpperCase() : "";
        return x && x.length <= 4 ? x : ({ image: "IMG", video: "VID", audio: "AUD", disc: "ISO", archive: "ZIP", app: "APP", doc: "DOC", text: "TXT", file: "" })[kindOf(e)];
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
    readonly property string thumbDir: (Quickshell.env("XDG_CACHE_HOME") || home + "/.cache") + "/thumbnails/large/"
    function thumbFor(e) {
        if (!e || e.dir) return "";
        const k = kindOf(e);
        if (k === "image" && e.size < 80 * 1024 * 1024 && !/heic|jxl|avif|x-raw|tiff|psd|xcf/.test(e.mime)) return fileUrl(e.path);
        if (k === "video" || e.mime === "application/pdf") return "file://" + thumbDir + Qt.md5(fileUrl(e.path)) + ".png";
        return "";
    }

    // ================================================ moving around
    function goUp(closeAtTop) {
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
    function activate(e) {
        if (!e) return;
        if (inTrash) return openMenu(false, -1, -1);
        if (e.dir) return navigate(e.path, "");
        say("Opening “" + e.name + "”…");
        call(["open", e.path], (r, err) => { if (err) say(err, true); });
    }
    function openPlace(i) {
        const p = places[i];
        if (!p) return;
        placeIndex = i;
        navigate(p.path, "");
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
    function targets() {
        if (markCount) return entries.filter(e => marked[e.path]).map(e => e.path);
        return current ? [current.path] : [];
    }
    function setHidden(on) {
        hidden = on;
        reload();
        say(on ? "Showing hidden files" : "Hiding hidden files");
    }
    function cycleSort() {
        sort = ({ name: "date", date: "size", size: "name" })[sort];
        cur = 0;
    }
    function setView(v) { view = v; }

    // ================================================ the actions menu (X)
    function openMenu(forFolder, x, y) {
        const e = forFolder ? null : current;
        const t = forFolder ? [] : targets();
        const n = t.length;
        const what = n === 1 ? "" : " (" + n + ")";
        const items = [];
        if (job) items.push({ id: "stop", label: "Stop " + (job.verb === "move" ? "moving" : "copying"), danger: true });
        if (inTrash) {
            if (n) items.push({ id: "restore", label: "Put back" + what });
            if (n) items.push({ id: "delete", label: "Delete permanently" + what, danger: true });
            items.push({ id: "empty", label: "Empty Trash", danger: true });
            if (e) items.push({ id: "props", label: "Properties" });
        } else {
            if (e) items.push({ id: "open", label: "Open" });
            if (e) items.push({ id: "openwith", label: "Open with…" });
            if (e) items.push({ id: "mark", label: marked[e.path] ? "Unselect" : "Select" });
            if (n) items.push({ id: "copy", label: "Copy" + what });
            if (n && writable) items.push({ id: "cut", label: "Cut" + what });
            if (clip.paths.length && writable) items.push({ id: "paste", label: (clip.mode === "cut" ? "Move here" : "Paste") + (clip.paths.length > 1 ? " (" + clip.paths.length + ")" : "") });
            if (e && n === 1 && writable) items.push({ id: "rename", label: "Rename" });
            if (e && kindOf(e) === "image") items.push({ id: "wallpaper", label: "Set as wallpaper" });
            if (writable) items.push({ id: "newfolder", label: "New folder" });
            if (n && writable) items.push({ id: "trash", label: "Move to Trash" + what });
            if (n && writable) items.push({ id: "delete", label: "Delete permanently" + what, danger: true });
            if (markCount) items.push({ id: "unmarkall", label: "Clear selection" });
            else if (entries.length) items.push({ id: "markall", label: "Select all" });
            items.push({ id: "hidden", label: hidden ? "Hide hidden files" : "Show hidden files" });
            items.push({ id: "props", label: e ? "Properties" : "Folder properties" });
        }
        menuItems = items;
        menuIndex = 0;
        menuX = x === undefined ? -1 : x;
        menuY = y === undefined ? -1 : y;
        overlay = "menu";
    }
    function openPlaceMenu() {
        const p = places[placeIndex];
        if (!p) return;
        const items = [{ id: "p-open", label: "Open" }];
        if (p.section === "drives") items.push({ id: "p-eject", label: "Safely remove" });
        if (p.kind === "trash") items.push({ id: "empty", label: "Empty Trash", danger: true });
        items.push({ id: "p-props", label: "Properties" });
        menuItems = items;
        menuIndex = 0;
        menuX = -1;
        menuY = -1;
        overlay = "menu";
    }
    function runMenu(id) {
        overlay = "";
        const t = targets();
        const e = current;
        switch (id) {
        case "stop": cancelJob(); break;
        case "open": activate(e); break;
        case "openwith": showApps(e); break;
        case "mark": toggleMark(e); break;
        case "markall": selectAll(); break;
        case "unmarkall": marked = ({}); break;
        case "copy":
            clip = { mode: "copy", paths: t };
            say(t.length === 1 ? "Copied “" + base(t[0]) + "”. Open a folder and choose Paste." : "Copied " + t.length + " items. Open a folder and choose Paste.");
            break;
        case "cut":
            clip = { mode: "cut", paths: t };
            say("Open the folder they should go to and choose Move here.");
            break;
        case "paste": paste(); break;
        case "rename":
            if (e) askText("Rename", e.name, "Rename", e.dir ? e.name.length : (e.name.lastIndexOf(".") > 0 ? e.name.lastIndexOf(".") : e.name.length),
                text => app.call(["rename", e.path, text], (r, err) => {
                    if (err) return app.say(err, true);
                    app.load(app.listing.path, app.base(r.path));
                }));
            break;
        case "newfolder": newFolder(); break;
        case "trash":
            call(["trash"].concat(t), (r, err) => {
                if (err) return say(err, true);
                say(t.length === 1 ? "Moved “" + base(t[0]) + "” to the Trash" : "Moved " + t.length + " items to the Trash");
                marked = ({});
                reload();
                refreshPlaces();
            });
            break;
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
        case "props": showProps(e ? e.path : (listing ? listing.path : home)); break;
        case "p-open": openPlace(placeIndex); break;
        case "p-props": if (places[placeIndex]) showProps(places[placeIndex].path); break;
        case "p-eject": {
            const p = places[placeIndex];
            if (!p) break;
            say("Removing “" + p.label + "”…");
            call(["eject", p.path], (r, err) => {
                if (err) return say(err, true);
                say("You can take out “" + p.label + "” now");
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
                app.load(dir, text);
            });
        });
    }
    function paste() {
        if (!clip.paths.length || !writable) return;
        startJob(clip.mode === "cut" ? "move" : "copy", clip.paths, listing.path);
        if (clip.mode === "cut") clip = { mode: "", paths: [] };
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
    function confirm(title, text, ok, action) {
        dlgTitle = title;
        dlgText = text;
        dlgOk = ok;
        dlgDanger = true;
        dlgAction = action;
        dlgButton = 0;                         // Cancel first: a stray A press doesn't delete anything
        overlay = "confirm";
    }
    function showProps(p) {
        dlgTitle = base(p);
        dlgInfo = null;
        dlgPath = p;
        overlay = "props";
        call(["info", p], (r, err) => {
            if (dlgPath !== p) return;
            if (err) {
                overlay = "";
                return say(err, true);
            }
            dlgInfo = r;
        });
    }
    function showApps(e) {
        if (!e) return;
        dlgTitle = "Open “" + e.name + "” with";
        dlgApps = [];
        dlgIndex = 0;
        dlgInfo = null;
        dlgPath = e.path;
        overlay = "apps";
        call(["apps", e.path], (r, err) => {
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
        overlay = "";
        say("Opening with " + a.name + "…");
        call(["open", p, a.id], (r, err) => { if (err) say(err, true); });
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
        dlgAction = null;
        dlgPath = "";
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

    // ================================================ copy / move in the background
    Process {
        id: jobProc
        stdout: SplitParser { onRead: line => app.jobLine(line) }
        stderr: SplitParser { onRead: line => console.warn("hearth-files:", line) }
        onExited: code => Qt.callLater(app.jobExited)
    }
    function startJob(verb, paths, dest) {
        if (!paths.length) return;
        if (jobProc.running) {
            jobQueue = jobQueue.concat([{ verb: verb, paths: paths, dest: dest }]);
            say("That starts as soon as the current one is done");
            return;
        }
        const destName = rootOf(dest).path === dest || base(dest) === "" ? rootOf(dest).label : base(dest);
        job = { verb: verb, count: paths.length, name: paths.length === 1 ? base(paths[0]) : "", destName: destName,
            done: 0, total: 0, speed: 0, file: "", cancelling: false };
        jobLast = "";
        jobProc.command = ["python3", helper, verb].concat(paths).concat([dest]);
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
        if (last.startsWith("error: ")) {
            say(last.slice(7), true);
        } else {
            let r = null;
            try { r = JSON.parse(last); } catch (e) {}
            const what = j ? (j.name ? "“" + j.name + "”" : j.count + " items") : "";
            if (!r || r.cancelled) say(j && j.cancelling ? "Stopped. Nothing half-copied was left behind." : "The copy stopped", !(j && j.cancelling));
            else say((j && j.verb === "move" ? "Moved " : "Copied ") + what + (j ? " to “" + j.destName + "”" : ""));
        }
        reload();
        refreshPlaces();
        if (jobQueue.length) {
            const n = jobQueue[0];
            jobQueue = jobQueue.slice(1);
            startJob(n.verb, n.paths, n.dest);
        }
    }
    function cancelJob() {
        if (!job || !jobProc.running) return;
        job = Object.assign({}, job, { cancelling: true });
        jobQueue = [];
        jobProc.running = false;               // SIGTERM: hearth-files removes the half-copied item
    }

    // ================================================ the window
    LazyLoader {
        active: app.shown
        FloatingWindow {
            id: win
            title: "Files"
            implicitWidth: 1100
            implicitHeight: 720
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
                app.toolIndex = app.crumbs.length;
                if (text !== undefined) search.text = text;
                search.forceActiveFocus();
                search.cursorPosition = search.text.length;
            }
            readonly property int cols: Math.max(1, Math.floor(grid.width / grid.cellWidth))
            function showCurrent() {
                if (app.view === "grid") grid.positionViewAtIndex(app.cur, GridView.Contain);
                else list.positionViewAtIndex(app.cur, ListView.Contain);
            }
            Connections {
                target: app
                function onCurChanged() { Qt.callLater(win.showCurrent); }
                function onViewChanged() { Qt.callLater(win.showCurrent); }
                function onQueryChanged() { if (search.text !== app.query) search.text = app.query; }
                function onPlaceIndexChanged() { placeList.positionViewAtIndex(app.placeIndex, ListView.Contain); }
            }

            // ------------------------------------------------ every key comes through here
            Item {
                id: keys
                anchors.fill: parent
                focus: true
                Keys.onPressed: e => {
                    if (app.overlay !== "") return win.overlayKey(e);
                    const ctrl = (e.modifiers & Qt.ControlModifier) !== 0;
                    const shift = (e.modifiers & Qt.ShiftModifier) !== 0;
                    if (ctrl) {
                        switch (e.key) {
                        case Qt.Key_C: if (app.targets().length) app.runMenu("copy"); break;
                        case Qt.Key_X: if (app.targets().length && app.writable) app.runMenu("cut"); break;
                        case Qt.Key_V: app.paste(); break;
                        case Qt.Key_A: app.selectAll(); break;
                        case Qt.Key_H: app.setHidden(!app.hidden); break;
                        case Qt.Key_F: win.focusSearch(); break;
                        case Qt.Key_R: app.reload(); break;
                        case Qt.Key_N: if (shift) app.newFolder(); break;
                        case Qt.Key_1: app.setView("grid"); break;
                        case Qt.Key_2: app.setView("list"); break;
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
                    if (app.zone === "places") return win.placesKey(e);
                    if (app.zone === "toolbar") return win.toolbarKey(e);
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
            function toolbarKey(e) {
                const n = app.crumbs.length;
                switch (e.key) {
                case Qt.Key_Left: case Qt.Key_Backtab: app.toolIndex = Math.max(0, app.toolIndex - 1); break;
                case Qt.Key_Right: case Qt.Key_Tab: app.toolIndex = Math.min(app.toolCount - 1, app.toolIndex + 1); break;
                case Qt.Key_Down: case Qt.Key_Escape: app.zone = "main"; break;
                case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: {
                    if (app.toolIndex < n) { app.navigate(app.crumbs[app.toolIndex].path, ""); break; }
                    const t = app.tools[app.toolIndex - n];
                    if (t === "search") win.focusSearch();
                    else if (t === "view") app.setView(app.view === "grid" ? "list" : "grid");
                    else if (t === "sort") app.cycleSort();
                    else if (t === "hidden") app.setHidden(!app.hidden);
                    break;
                }
                default: return;
                }
                e.accepted = true;
            }
            function mainKey(e, shift) {
                const count = app.entries.length;
                const c = app.view === "grid" ? win.cols : 1;
                const i = app.cur;
                switch (e.key) {
                case Qt.Key_Left:
                    if (app.view === "list" || i % c === 0) win.toPlaces();
                    else app.cur = i - 1;
                    break;
                case Qt.Key_Right:
                    if (app.view === "grid" && i + 1 < count && (i + 1) % c !== 0) app.cur = i + 1;
                    break;
                case Qt.Key_Up:
                    if (i - c >= 0) app.cur = i - c;
                    else win.toToolbar();
                    break;
                case Qt.Key_Down:
                    if (i + c < count) app.cur = i + c;
                    else if (Math.floor(i / c) < Math.floor((count - 1) / c)) app.cur = count - 1;
                    break;
                case Qt.Key_Tab: app.cur = Math.min(count - 1, i + 1); break;
                case Qt.Key_Backtab: app.cur = Math.max(0, i - 1); break;
                case Qt.Key_PageDown: app.cur = Math.min(count - 1, i + c * 4); break;
                case Qt.Key_PageUp: app.cur = Math.max(0, i - c * 4); break;
                case Qt.Key_Home: app.cur = 0; break;
                case Qt.Key_End: app.cur = Math.max(0, count - 1); break;
                case Qt.Key_Return: case Qt.Key_Enter: app.activate(app.current); break;
                case Qt.Key_Space: app.toggleMark(app.current); break;
                case Qt.Key_F2: if (app.current && app.writable && !app.inTrash) app.runMenu("rename"); break;
                case Qt.Key_Delete:
                    if (app.targets().length && app.writable) app.runMenu(shift || app.inTrash ? "delete" : "trash");
                    break;
                case Qt.Key_Escape:
                    if (app.query) app.query = "";
                    else if (app.markCount) app.marked = ({});
                    else app.goUp(true);
                    break;
                case Qt.Key_Backspace:
                    if (app.query) app.query = app.query.slice(0, -1);
                    else app.goUp(false);
                    break;
                default:
                    // Typing searches this folder (with the on-screen keyboard too: Y, then type).
                    if (e.text && e.text.length === 1 && e.text >= " " && !(e.modifiers & (Qt.AltModifier | Qt.MetaModifier))) {
                        win.focusSearch(app.query + (e.text === "/" && !app.query ? "" : e.text));
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
                app.toolIndex = Math.max(0, app.crumbs.length - 1);
                app.zone = "toolbar";
            }
            function overlayKey(e) {
                const up = e.key === Qt.Key_Up || e.key === Qt.Key_Backtab;
                const down = e.key === Qt.Key_Down || e.key === Qt.Key_Tab;
                const ok = e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space;
                const back = e.key === Qt.Key_Escape || e.key === Qt.Key_Backspace || e.key === Qt.Key_Menu;
                if (app.overlay === "menu") {
                    if (up) app.menuIndex = Math.max(0, app.menuIndex - 1);
                    else if (down) app.menuIndex = Math.min(app.menuItems.length - 1, app.menuIndex + 1);
                    else if (ok || e.key === Qt.Key_Right) app.runMenu(app.menuItems[app.menuIndex].id);
                    else if (back || e.key === Qt.Key_Left) app.closeOverlay();
                    else return;
                } else if (app.overlay === "confirm") {
                    if (e.key === Qt.Key_Left || up) app.dlgButton = 0;
                    else if (e.key === Qt.Key_Right || down) app.dlgButton = 1;
                    else if (ok) { if (app.dlgButton === 1) app.accept(""); else app.closeOverlay(); }
                    else if (back) app.closeOverlay();
                    else return;
                } else if (app.overlay === "apps") {
                    if (up) app.dlgIndex = Math.max(0, app.dlgIndex - 1);
                    else if (down) app.dlgIndex = Math.min(app.dlgApps.length - 1, app.dlgIndex + 1);
                    else if (ok) app.openWith(app.dlgApps[app.dlgIndex]);
                    else if (back || e.key === Qt.Key_Left) app.closeOverlay();
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
                                // A heading where a new group starts (Games, Drives and SD cards; a gap before Trash).
                                readonly property bool starts: index > 0 && app.places[index - 1].section !== modelData.section
                                readonly property real headH: !starts ? 0 : modelData.section === "trash" ? 12 : 32
                                width: ListView.view.width
                                height: headH + place.height
                                Text {
                                    visible: placeItem.headH > 20
                                    anchors { left: parent.left; leftMargin: 14; top: parent.top; topMargin: 12 }
                                    text: placeItem.modelData.section === "games" ? "Games" : "Drives and SD cards"
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold
                                }
                                Rectangle {
                                    id: place
                                    readonly property var modelData: placeItem.modelData
                                    readonly property int index: placeItem.index
                                    readonly property bool here: app.path === modelData.path
                                    readonly property bool lit: app.zone === "places" && app.placeIndex === index
                                    readonly property color ink: lit ? Theme.accentInk : here ? Theme.accent : Theme.text
                                    y: placeItem.headH
                                    width: parent.width
                                    height: modelData.free !== undefined && modelData.section === "drives" ? 56 : 44
                                    radius: 10
                                    color: lit ? Theme.accent : here ? Theme.panel3 : placeMouse.containsMouse ? Theme.panel2 : "transparent"
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
                                                visible: place.modelData.section === "drives" && place.modelData.free !== undefined && place.modelData.free !== null
                                                text: (place.modelData.sub || "") + " · " + app.fmtSize(place.modelData.free) + " free"
                                                color: place.lit ? Theme.accentInk : Theme.muted
                                                elide: Text.ElideRight
                                                font.family: Theme.font; font.pixelSize: 11
                                            }
                                        }
                                        Text {
                                            visible: place.modelData.kind === "trash" && place.modelData.count > 0
                                            text: place.modelData.count || ""
                                            color: place.lit ? Theme.accentInk : Theme.muted
                                            font.family: Theme.font; font.pixelSize: 12
                                        }
                                    }
                                    MouseArea {
                                        id: placeMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: mouse => {
                                            keys.forceActiveFocus();
                                            app.placeIndex = place.index;
                                            if (mouse.button === Qt.RightButton) {
                                                app.zone = "places";
                                                app.viaPad = false;
                                                app.openPlaceMenu();
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
                            text: "Ⓐ open  Ⓑ back  Ⓧ actions  Ⓨ search"
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

                    // Path, search and view buttons.
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.margins: 14
                        Layout.bottomMargin: 8
                        spacing: 8
                        Flickable {
                            id: crumbFlick
                            Layout.fillWidth: true
                            Layout.preferredHeight: 40
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
                                            focused: app.zone === "toolbar" && app.toolIndex === crumb.index
                                            flat: true
                                            onClicked: { keys.forceActiveFocus(); app.navigate(crumb.modelData.path, ""); }
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
                        Rectangle {
                            id: searchBox
                            readonly property bool focused: app.zone === "toolbar" && app.toolIndex === app.crumbs.length
                            Layout.preferredWidth: win.width < 1000 ? 170 : 230
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
                                onTextChanged: { if (app.query !== text) { app.query = text; app.cur = 0; } }
                                Keys.onPressed: e => {
                                    switch (e.key) {
                                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Down: case Qt.Key_Escape:
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
                                    text: "Search this folder"
                                    color: Theme.muted
                                    font: search.font
                                }
                            }
                            Text {
                                visible: search.text !== ""
                                anchors { right: parent.right; rightMargin: 12; verticalCenter: parent.verticalCenter }
                                text: "✕"; color: Theme.muted; font.pixelSize: 13
                                MouseArea { anchors.fill: parent; anchors.margins: -8; onClicked: { app.query = ""; keys.forceActiveFocus(); } }
                            }
                            MouseArea {
                                anchors.fill: parent
                                anchors.rightMargin: 30
                                cursorShape: Qt.IBeamCursor
                                onClicked: win.focusSearch()
                            }
                        }
                        Chip {
                            icon: app.view === "grid" ? "grid" : "menu"
                            label: app.view === "grid" ? "Grid" : "List"
                            focused: app.zone === "toolbar" && app.toolIndex === app.crumbs.length + 1
                            onClicked: { keys.forceActiveFocus(); app.setView(app.view === "grid" ? "list" : "grid"); }
                        }
                        Chip {
                            label: "Sort: " + ({ name: "Name", date: "Date", size: "Size" })[app.sort]
                            focused: app.zone === "toolbar" && app.toolIndex === app.crumbs.length + 2
                            onClicked: { keys.forceActiveFocus(); app.cycleSort(); }
                        }
                        Chip {
                            label: "Hidden"
                            dot: app.hidden
                            focused: app.zone === "toolbar" && app.toolIndex === app.crumbs.length + 3
                            onClicked: { keys.forceActiveFocus(); app.setHidden(!app.hidden); }
                        }
                    }

                    // The folder itself.
                    Item {
                        Layout.fillWidth: true
                        Layout.fillHeight: true

                        MouseArea {
                            // Empty space: right-click / press-and-hold for the folder's actions.
                            anchors.fill: parent
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            onClicked: mouse => {
                                keys.forceActiveFocus();
                                app.zone = "main";
                                if (mouse.button === Qt.RightButton) {
                                    const p = mapToItem(keys, mouse.x, mouse.y);
                                    app.viaPad = false;
                                    app.openMenu(true, p.x, p.y);
                                }
                            }
                            onPressAndHold: mouse => {
                                const p = mapToItem(keys, mouse.x, mouse.y);
                                keys.forceActiveFocus();
                                app.openMenu(true, p.x, p.y);
                            }
                        }

                        GridView {
                            id: grid
                            visible: app.view === "grid"
                            anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                            model: app.entries
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            cellWidth: Math.floor(width / Math.max(1, Math.floor(width / 136)))
                            cellHeight: 156
                            keyNavigationEnabled: false
                            cacheBuffer: 600
                            delegate: Item {
                                id: cell
                                required property var modelData
                                required property int index
                                readonly property bool lit: app.cur === index
                                readonly property bool picked: !!app.marked[modelData.path]
                                readonly property bool cut: app.clip.mode === "cut" && app.clip.paths.indexOf(modelData.path) >= 0
                                width: grid.cellWidth
                                height: grid.cellHeight
                                opacity: cut ? 0.45 : 1
                                Rectangle {
                                    anchors { fill: parent; margins: 4 }
                                    radius: 12
                                    color: cell.picked ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22)
                                         : cell.lit ? Theme.panel3 : cellMouse.containsMouse ? Theme.panel2 : "transparent"
                                    border.width: 2
                                    border.color: cell.lit && app.zone === "main" && app.overlay === "" ? Theme.accent : "transparent"
                                    Behavior on border.color { ColorAnimation { duration: Theme.quick } }
                                }
                                Item {
                                    id: art
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
                                            source: app.thumbFor(cell.modelData)
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
                                    anchors { top: art.bottom; topMargin: 8; left: parent.left; right: parent.right; leftMargin: 10; rightMargin: 10 }
                                    horizontalAlignment: Text.AlignHCenter
                                    text: cell.modelData.name
                                    color: cell.modelData.hidden ? Theme.muted : Theme.text
                                    wrapMode: Text.WrapAnywhere
                                    maximumLineCount: 2
                                    elide: Text.ElideRight
                                    font.family: Theme.font; font.pixelSize: 13; font.weight: cell.lit ? Font.DemiBold : Font.Normal
                                }
                                Rectangle {
                                    visible: cell.picked
                                    anchors { top: parent.top; right: parent.right; margins: 10 }
                                    width: 22; height: 22; radius: 11
                                    color: Theme.accent
                                    Icon { anchors.centerIn: parent; name: "check"; size: 14; color: Theme.accentInk }
                                }
                                MouseArea {
                                    id: cellMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                                    onClicked: mouse => win.itemClicked(cell.index, mouse, cellMouse)
                                    onDoubleClicked: mouse => { if (mouse.button === Qt.LeftButton) app.activate(cell.modelData); }
                                    onPressAndHold: mouse => win.itemMenu(cell.index, mouse, cellMouse)
                                }
                            }
                        }

                        ListView {
                            id: list
                            visible: app.view === "list"
                            anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                            model: app.entries
                            clip: true
                            spacing: 2
                            boundsBehavior: Flickable.StopAtBounds
                            keyNavigationEnabled: false
                            headerPositioning: ListView.OverlayHeader
                            header: Rectangle {
                                width: ListView.view.width
                                height: 34
                                z: 2
                                color: Theme.bg
                                RowLayout {
                                    anchors { fill: parent; leftMargin: 56; rightMargin: 16 }
                                    spacing: 12
                                    ColumnHead { Layout.fillWidth: true; label: "Name"; key: "name"; sortKey: app.sort; onPicked: k => app.sort = k }
                                    ColumnHead { Layout.preferredWidth: 100; label: "Size"; key: "size"; sortKey: app.sort; alignRight: true; onPicked: k => app.sort = k }
                                    ColumnHead { Layout.preferredWidth: 170; label: "Modified"; key: "date"; sortKey: app.sort; onPicked: k => app.sort = k }
                                }
                                Rectangle {
                                    anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                                    height: 1
                                    color: Theme.line
                                }
                            }
                            delegate: Rectangle {
                                id: row
                                required property var modelData
                                required property int index
                                readonly property bool lit: app.cur === index
                                readonly property bool picked: !!app.marked[modelData.path]
                                readonly property bool cut: app.clip.mode === "cut" && app.clip.paths.indexOf(modelData.path) >= 0
                                width: ListView.view.width
                                height: 46
                                radius: 10
                                opacity: cut ? 0.45 : 1
                                color: picked ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22)
                                     : lit ? Theme.panel3 : rowMouse.containsMouse ? Theme.panel2 : "transparent"
                                border.width: 2
                                border.color: lit && app.zone === "main" && app.overlay === "" ? Theme.accent : "transparent"
                                RowLayout {
                                    anchors { fill: parent; leftMargin: 12; rightMargin: 16 }
                                    spacing: 12
                                    Item {
                                        Layout.preferredWidth: 32; Layout.preferredHeight: 32
                                        FileGlyph {
                                            anchors.centerIn: parent
                                            visible: rowThumb.status !== Image.Ready
                                            kind: app.kindOf(row.modelData)
                                            label: ""
                                            link: row.modelData.isLink
                                            size: 28
                                        }
                                        Image {
                                            id: rowThumb
                                            anchors.fill: parent
                                            source: app.kindOf(row.modelData) === "image" ? app.thumbFor(row.modelData) : ""
                                            sourceSize: Qt.size(64, 64)
                                            asynchronous: true
                                            fillMode: Image.PreserveAspectCrop
                                            clip: true
                                        }
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: row.modelData.name + (row.modelData.origPath ? "   ·   from " + app.parentOf(row.modelData.origPath) : "")
                                        color: row.modelData.hidden ? Theme.muted : Theme.text
                                        elide: Text.ElideMiddle
                                        font.family: Theme.font; font.pixelSize: 14; font.weight: row.lit ? Font.DemiBold : Font.Normal
                                    }
                                    Icon { visible: row.picked; name: "check"; size: 16; color: Theme.accent }
                                    Text {
                                        Layout.preferredWidth: 100
                                        horizontalAlignment: Text.AlignRight
                                        text: row.modelData.dir ? "" : app.fmtSize(row.modelData.size)
                                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                    }
                                    Text {
                                        Layout.preferredWidth: 170
                                        text: app.fmtDate(row.modelData.mtime)
                                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                        elide: Text.ElideRight
                                    }
                                }
                                MouseArea {
                                    id: rowMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                                    onClicked: mouse => win.itemClicked(row.index, mouse, rowMouse)
                                    onDoubleClicked: mouse => { if (mouse.button === Qt.LeftButton) app.activate(row.modelData); }
                                    onPressAndHold: mouse => win.itemMenu(row.index, mouse, rowMouse)
                                }
                            }
                        }

                        // Empty, loading, or can't open.
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
                                    : app.query ? "Nothing here matches “" + app.query + "”"
                                    : app.inTrash ? "The Trash is empty" : "This folder is empty"
                                color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold
                            }
                            Text {
                                width: parent.width
                                horizontalAlignment: Text.AlignHCenter
                                wrapMode: Text.WordWrap
                                text: app.listing && app.listing.error ? app.listing.error
                                    : app.query ? "Only this folder is searched. Ⓑ clears the search."
                                    : app.writable && !app.inTrash ? "Ⓧ for actions: paste, new folder…" : ""
                                visible: text !== ""
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                            }
                        }
                    }

                    // ------------------------------------------------ bottom bar: status, or copy progress
                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line }
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: app.job ? 66 : 44
                        color: Theme.panel

                        RowLayout {
                            visible: !app.job
                            anchors { fill: parent; leftMargin: 18; rightMargin: 18 }
                            spacing: 12
                            Text {
                                Layout.fillWidth: true
                                text: app.note !== "" ? app.note
                                    : app.plural(app.entries.length, "item") + (app.markCount ? " · " + app.markCount + " selected" : "")
                                      + (app.clip.paths.length ? " · " + app.plural(app.clip.paths.length, "item") + (app.clip.mode === "cut" ? " to move" : " copied") : "")
                                      + (app.listing && !app.listing.writable && !app.listing.error ? " · read-only" : "")
                                color: app.note !== "" && app.noteError ? Theme.danger : app.note !== "" ? Theme.text : Theme.muted
                                elide: Text.ElideRight
                                font.family: Theme.font; font.pixelSize: 13
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
                                        const verb = j.cancelling ? "Stopping" : j.verb === "move" ? "Moving" : "Copying";
                                        const what = j.name ? "“" + j.name + "”" : app.plural(j.count || 0, "item");
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
                if (mouse.button === Qt.RightButton) return win.itemMenu(index, mouse, area);
                if (mouse.modifiers & Qt.ControlModifier) {
                    app.cur = index;
                    app.toggleMark(app.entries[index]);
                    return;
                }
                if ((mouse.modifiers & Qt.ShiftModifier) && app.current) {
                    const m = Object.assign({}, app.marked);
                    for (let i = Math.min(app.cur, index); i <= Math.max(app.cur, index); i++) m[app.entries[i].path] = true;
                    app.marked = m;
                }
                app.cur = index;
            }
            function itemMenu(index, mouse, area) {
                keys.forceActiveFocus();
                app.zone = "main";
                app.cur = index;
                app.viaPad = false;
                const p = area.mapToItem(keys, mouse.x, mouse.y);
                app.openMenu(false, p.x, p.y);
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
                    width: 290
                    height: menuCol.implicitHeight + 16
                    x: app.menuX < 0 ? (parent.width - width) / 2 : Math.max(8, Math.min(parent.width - width - 8, app.menuX))
                    y: app.menuY < 0 ? (parent.height - height) / 2 : Math.max(8, Math.min(parent.height - height - 8, app.menuY))
                    radius: Theme.radius
                    color: Theme.panel
                    border.color: Theme.line
                    MouseArea { anchors.fill: parent }
                    Column {
                        id: menuCol
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 8 }
                        spacing: 2
                        Repeater {
                            model: app.menuItems
                            delegate: Rectangle {
                                id: mi
                                required property var modelData
                                required property int index
                                readonly property bool lit: app.menuIndex === index
                                width: menuCol.width
                                height: 42
                                radius: 10
                                color: lit ? (modelData.danger ? Theme.danger : Theme.accent) : miMouse.containsMouse ? Theme.panel3 : "transparent"
                                Text {
                                    anchors { left: parent.left; leftMargin: 14; verticalCenter: parent.verticalCenter }
                                    text: mi.modelData.label
                                    color: mi.lit ? (mi.modelData.danger ? "#ffffff" : Theme.accentInk) : mi.modelData.danger ? Theme.danger : Theme.text
                                    font.family: Theme.font; font.pixelSize: 14; font.weight: mi.lit ? Font.DemiBold : Font.Normal
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

                // Dialogs: text box, confirm, properties, open with.
                Rectangle {
                    id: card
                    visible: app.overlay !== "" && app.overlay !== "menu"
                    anchors.centerIn: parent
                    width: Math.min(parent.width - 48, 540)
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
                            text: app.dlgTitle
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold
                            elide: Text.ElideMiddle
                        }
                        Text {
                            Layout.fillWidth: true
                            visible: app.overlay === "confirm"
                            text: app.dlgText
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
                        }

                        // Text box (rename, new folder).
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

                        // Properties.
                        GridLayout {
                            Layout.fillWidth: true
                            visible: app.overlay === "props"
                            columns: 2
                            columnSpacing: 18
                            rowSpacing: 8
                            Repeater {
                                model: {
                                    const i = app.dlgInfo;
                                    if (app.overlay !== "props") return [];
                                    if (!i) return [["", "Counting…"]];
                                    const rows = [["Kind", app.kindName(i, i.mime) + (i.isLink ? " (shortcut)" : "")]];
                                    rows.push(["Size", app.fmtSize(i.size) + (i.dir ? " · " + app.plural(i.files, "file") + ", " + app.plural(i.folders, "folder") : "")]);
                                    rows.push(["Location", i.parent]);
                                    if (i.target) rows.push(["Points to", i.target]);
                                    if (i.origPath) rows.push(["Came from", i.origPath]);
                                    if (i.deleted) rows.push(["Moved to Trash", i.deleted.replace("T", ", ")]);
                                    rows.push(["Modified", app.fmtDate(i.mtime)]);
                                    rows.push(["Last opened", app.fmtDate(i.atime)]);
                                    rows.push(["Permissions", i.mode + " · " + i.owner + (i.writable ? "" : " · read-only here")]);
                                    if (i.free !== null && i.free !== undefined) rows.push(["Free space", app.fmtSize(i.free) + " of " + app.fmtSize(i.total)]);
                                    return rows;
                                }
                                delegate: Item {
                                    id: prop
                                    required property var modelData
                                    Layout.columnSpan: 2
                                    Layout.fillWidth: true
                                    implicitHeight: Math.max(propK.implicitHeight, propV.implicitHeight)
                                    Text {
                                        id: propK
                                        width: 120
                                        text: prop.modelData[0]
                                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                                    }
                                    Text {
                                        id: propV
                                        x: 138
                                        width: parent.width - 138
                                        text: prop.modelData[1]
                                        color: Theme.text; font.family: Theme.font; font.pixelSize: 13
                                        wrapMode: Text.WrapAnywhere
                                    }
                                }
                            }
                        }

                        // Open with.
                        Text {
                            Layout.fillWidth: true
                            visible: app.overlay === "apps" && (!app.dlgInfo || app.dlgApps.length === 0)
                            text: !app.dlgInfo ? "Looking for apps…" : "No installed app says it can open this kind of file."
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
                        }
                        ListView {
                            id: appList
                            Layout.fillWidth: true
                            Layout.preferredHeight: Math.min(contentHeight, 360)
                            visible: app.overlay === "apps" && app.dlgApps.length > 0
                            model: app.overlay === "apps" ? app.dlgApps : []
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
                                        implicitSize: 30
                                        source: Quickshell.iconPath(ap.modelData.icon || "", "application-x-executable")
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: ap.modelData.name
                                        color: ap.lit ? Theme.accentInk : Theme.text
                                        elide: Text.ElideRight
                                        font.family: Theme.font; font.pixelSize: 14; font.weight: ap.lit ? Font.DemiBold : Font.Normal
                                    }
                                    Text {
                                        visible: ap.modelData.default
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
                                label: app.overlay === "props" || app.overlay === "apps" ? "Close" : "Cancel"
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
        signal clicked()
        implicitWidth: chipRow.implicitWidth + 28
        implicitHeight: 40
        radius: 10
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

    // A list view column title; click to sort by it.
    component ColumnHead: Text {
        id: head
        property string label
        property string key
        property string sortKey
        property bool alignRight: false
        signal picked(string key)
        text: label + (sortKey === key ? (key === "name" ? "  ▴" : "  ▾") : "")
        horizontalAlignment: alignRight ? Text.AlignRight : Text.AlignLeft
        color: sortKey === key ? Theme.text : Theme.muted
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

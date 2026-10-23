// Properties, as in any file manager, for Hearth Files and the desktop icons (one window, shared): a file, a
// folder or several things picked at once.
//   General: the name (rename it here), the type with its icon, where it is (Copy), size and size on disk,
//     what a folder holds (added up live in the background), created / modified / opened, a picture's size,
//     a song's or video's length, where a shortcut points, the drive's free space (a bar for a drive itself)
//     and the app it opens with (Change…).
//   Permissions: owner and group, read / write / execute for owner, group and others, "Allow executing as a
//     program".
//   Checksums: MD5, SHA-1 and SHA-256 on demand.
// Changes wait for Apply or OK; problems are shown in plain words above the buttons. Keyboard and controller:
// Up/Down move, Left/Right switch tabs (on the tab row), A / Enter / Space press or tick, B / Esc closes.
// hearth-files does the work (props, count, checksum, chmod, set-default, rename, apps).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Widgets

Scope {
    id: pr
    property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-files").toString().replace(/^file:\/\//, ""))
    // After Apply: the file manager and the desktop look again (old path → new path, the same if not renamed).
    signal applied(string oldPath, string newPath)

    property bool shown: false
    property var paths: []
    property var info: null                    // hearth-files props
    property string loadError: ""
    property string tab: "general"             // general | permissions | checksums
    property string focusId: "tabs"
    property bool editingName: false
    // What's changed, waiting for Apply.
    property string newName: ""
    property int newMode: -1
    property var newApp: null                  // {id, name, icon}
    property string error: ""
    property bool busy: false
    // A folder's contents, added up live: {size, disk, files, folders, unreadable, done}.
    property var counted: null
    // Checksums: algo → {running, sum, done, total}.
    property var sums: ({})
    property string sumAlgo: ""
    // Change… (the app list).
    property bool picking: false
    property var apps: []
    property int pickIndex: 0
    property bool pickAll: false

    readonly property bool multi: !!(info && info.multi)
    readonly property bool single: !!(info && !info.multi)
    readonly property bool canRename: single && info.canRename && !info.trashed
    readonly property bool isFile: single && !info.dir && !info.broken
    readonly property var tabs: {
        const t = [{ id: "general", label: "General" }];
        if (single && !info.trashed && !info.isLink) t.push({ id: "permissions", label: "Permissions" });
        if (isFile && !info.trashed) t.push({ id: "checksums", label: "Checksums" });
        return t;
    }
    readonly property int mode: newMode >= 0 ? newMode : (single ? info.mode : 0)
    readonly property bool dirty: single && ((newName !== "" && newName !== info.name) || (newMode >= 0 && newMode !== info.mode)
        || (newApp !== null && (!info.app || newApp.id !== info.app.id)))

    // ================================================ open / close
    function show(list) {
        const ps = (list || []).filter(p => !!p);
        if (!ps.length) return;
        stopCount();
        stopSum();
        paths = ps;
        info = null;
        loadError = "";
        tab = "general";
        focusId = "tabs";
        newName = "";
        newMode = -1;
        newApp = null;
        error = "";
        counted = null;
        sums = ({});
        picking = false;
        editingName = false;
        shown = true;
        load();
    }
    function close() {
        stopCount();
        stopSum();
        shown = false;
        picking = false;
    }
    function load() {
        const want = paths.slice();
        call(["props"].concat(want), (r, err) => {
            if (JSON.stringify(want) !== JSON.stringify(paths)) return;
            if (err) { loadError = err; return; }
            info = r;
            if (r.multi || r.dir) startCount(r.multi ? r.items.map(i => i.path) : [r.path]);
        });
    }

    // ================================================ hearth-files
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
        const p = callComp.createObject(pr, { command: ["python3", helper].concat(args), cb: cb || null });
        p.running = true;
    }

    Process {
        id: countProc
        stdout: SplitParser {
            onRead: line => {
                let t = line, done = false;
                if (t.startsWith("progress: ")) t = t.slice(10);
                else done = true;
                try {
                    const v = JSON.parse(t);
                    pr.counted = Object.assign({}, v, { done: done });
                } catch (e) {}
            }
        }
    }
    function startCount(list) {
        stopCount();
        counted = { size: 0, disk: 0, files: 0, folders: 0, done: false };
        countProc.command = ["python3", helper, "count"].concat(list);
        countProc.running = true;
    }
    function stopCount() { if (countProc.running) countProc.running = false; }

    Process {
        id: sumProc
        stdout: SplitParser {
            onRead: line => {
                const algo = pr.sumAlgo;
                const s = Object.assign({}, pr.sums);
                if (line.startsWith("progress: ")) {
                    try {
                        const v = JSON.parse(line.slice(10));
                        s[algo] = { running: true, done: v.done, total: v.total, sum: "" };
                    } catch (e) {}
                } else if (line.startsWith("error: ")) {
                    s[algo] = { running: false, sum: "", error: line.slice(7) };
                } else {
                    try {
                        const v = JSON.parse(line);
                        s[algo] = v.cancelled ? undefined : { running: false, sum: v.sum };
                    } catch (e) {}
                }
                pr.sums = s;
            }
        }
        onExited: {
            const s = Object.assign({}, pr.sums);
            if (s[pr.sumAlgo] && s[pr.sumAlgo].running) delete s[pr.sumAlgo];
            pr.sums = s;
            pr.sumAlgo = "";
            pr.nextSum();
        }
    }
    property var sumQueue: []
    function computeSum(algo) {
        if (!isFile) return;
        if (sums[algo] && (sums[algo].running || sums[algo].sum)) return;
        if (sumProc.running) {
            if (sumQueue.indexOf(algo) < 0 && sumAlgo !== algo) sumQueue = sumQueue.concat([algo]);
            const s = Object.assign({}, sums);
            s[algo] = { running: true, waiting: true, done: 0, total: 0, sum: "" };
            sums = s;
            return;
        }
        sumAlgo = algo;
        const s = Object.assign({}, sums);
        s[algo] = { running: true, done: 0, total: info.size, sum: "" };
        sums = s;
        sumProc.command = ["python3", helper, "checksum", algo, info.path];
        sumProc.running = true;
    }
    function nextSum() {
        if (!sumQueue.length || !shown) return;
        const a = sumQueue[0];
        sumQueue = sumQueue.slice(1);
        const s = Object.assign({}, sums);
        delete s[a];
        sums = s;
        computeSum(a);
    }
    function stopSum() {
        sumQueue = [];
        if (sumProc.running) sumProc.running = false;
    }

    // ================================================ changes
    function apply(thenClose) {
        if (!single || busy) return;
        error = "";
        if (!dirty) { if (thenClose) close(); return; }
        busy = true;
        const oldPath = info.path;
        let path = oldPath;
        const steps = [];
        if (newName !== "" && newName !== info.name) steps.push(next => call(["rename", path, newName], (r, err) => {
            if (err) return next(err);
            path = r.path;
            next("");
        }));
        if (newMode >= 0 && newMode !== info.mode) steps.push(next => call(["chmod", path, newMode.toString(8)], (r, err) => next(err)));
        if (newApp && (!info.app || newApp.id !== info.app.id)) steps.push(next => call(["set-default", path, newApp.id], (r, err) => next(err)));
        const run = i => {
            if (i >= steps.length) {
                busy = false;
                newName = "";
                newMode = -1;
                newApp = null;
                pr.applied(oldPath, path);
                if (thenClose) return close();
                paths = [path];
                const keep = counted;
                call(["props", path], (r, err) => { if (r) { info = r; counted = keep; } });
                return;
            }
            steps[i](err => {
                if (err) {
                    busy = false;
                    error = err;
                    if (path !== oldPath) {
                        // Renamed already: carry on from the new name next time.
                        pr.applied(oldPath, path);
                        paths = [path];
                        newName = "";
                        call(["props", path], (r, e2) => { if (r) info = r; });
                    }
                    return;
                }
                run(i + 1);
            });
        };
        run(0);
    }
    function setBit(bit, on) {
        if (!single || !info.mine) return;
        newMode = on ? (mode | bit) : (mode & ~bit);
    }
    function setExec(on) {
        if (!single || !info.mine) return;
        // Like other file managers: whoever can read it may run it.
        let m = mode;
        if (on) m = m | 0o100 | ((m & 0o040) ? 0o010 : 0) | ((m & 0o004) ? 0o001 : 0);
        else m = m & ~0o111;
        newMode = m;
    }
    function openPicker(all) {
        if (!isFile) return;
        pickAll = !!all;
        apps = [];
        pickIndex = 0;
        picking = true;
        call(["apps", info.path].concat(all ? ["--all"] : []), (r, err) => {
            if (err) { picking = false; error = err; return; }
            apps = r.apps;
            const cur = newApp ? newApp.id : (info.app ? info.app.id : "");
            const i = apps.findIndex(a => a.id === cur);
            pickIndex = Math.max(0, i);
        });
    }
    function pick(a) {
        picking = false;
        if (!a) return;
        newApp = { id: a.id, name: a.name, icon: a.icon };
    }
    function copyText(t) {
        Quickshell.execDetached(["wl-copy", "--", t]);
        if (SettingsNav) SettingsNav.toast("Copied");
    }

    // ================================================ words and numbers
    function fmtSize(n) {
        if (n === undefined || n === null) return "";
        if (n < 1024) return n + (n === 1 ? " byte" : " bytes");
        const u = ["KB", "MB", "GB", "TB"];
        let v = n / 1024, i = 0;
        while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
        return (v >= 100 ? Math.round(v) : v.toFixed(1)) + " " + u[i];
    }
    function exact(n) { return Number(n || 0).toLocaleString(Qt.locale(), "f", 0); }
    function fmtWhen(s) {
        if (!s) return "Unknown";
        return Qt.formatDateTime(new Date(s * 1000), "ddd d MMM yyyy, hh:mm:ss");
    }
    function fmtDuration(sec) {
        if (!sec) return "";
        sec = Math.round(sec);
        const h = Math.floor(sec / 3600), m = Math.floor((sec % 3600) / 60), s = sec % 60;
        const two = x => (x < 10 ? "0" : "") + x;
        return h ? h + ":" + two(m) + ":" + two(s) : m + ":" + two(s);
    }
    function plural(n, one, many) { return n + " " + (n === 1 ? one : (many || one + "s")); }
    function typeText(i) {
        if (!i) return "";
        if (i.broken) return "Broken shortcut";
        if (i.dir) return i.mount ? "Drive" : "Folder";
        return (i.desc || "File") + (i.mime ? "  (" + i.mime + ")" : "");
    }
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

    // ================================================ keyboard / controller focus
    readonly property var focusList: {
        const l = ["tabs"];
        if (!info) return l.concat(["cancel"]);
        if (tab === "general") {
            if (canRename) l.push("name");
            l.push("copyloc");
            if (single && info.target) l.push("copytarget");
            if (isFile && !info.trashed) l.push("change");
        } else if (tab === "permissions") {
            if (info.mine) {
                for (const w of ["u", "g", "o"]) for (const b of ["r", "w", "x"]) l.push("p-" + w + b);
                if (!info.dir) l.push("exec");
            }
        } else if (tab === "checksums") {
            for (const a of ["md5", "sha1", "sha256"]) l.push(sums[a] && sums[a].sum ? "copy-" + a : "sum-" + a);
        }
        l.push("cancel");
        if (single) l.push("apply");
        l.push("ok");
        return l;
    }
    function move(d) {
        const l = focusList;
        let i = l.indexOf(focusId);
        if (i < 0) i = 0;
        focusId = l[Math.max(0, Math.min(l.length - 1, i + d))];
    }
    function switchTab(d) {
        const i = tabs.findIndex(t => t.id === tab);
        const n = tabs[Math.max(0, Math.min(tabs.length - 1, i + d))];
        if (n) tab = n.id;
    }
    function bitOf(id) {
        const who = { u: 6, g: 3, o: 0 }[id.charAt(2)];
        const what = { r: 4, w: 2, x: 1 }[id.charAt(3)];
        return what << who;
    }
    function activate(id) {
        if (id === "tabs") return switchTab(1);
        if (id === "name") return pr.startEditName();
        if (id === "copyloc") return copyText(single ? info.path : (info.parent || ""));
        if (id === "copytarget") return copyText(info.target);
        if (id === "change") return openPicker(false);
        if (id === "exec") return setExec(!(mode & 0o100));
        if (id.startsWith("p-")) return setBit(bitOf(id), !(mode & bitOf(id)));
        if (id.startsWith("sum-")) return computeSum(id.slice(4));
        if (id.startsWith("copy-")) return copyText(sums[id.slice(5)].sum);
        if (id === "cancel") return close();
        if (id === "apply") return apply(false);
        if (id === "ok") return apply(true);
    }
    property var ui: null
    function startEditName() { if (canRename && ui) ui.editName(); }

    IpcHandler {
        target: "fileprops"
        function open(path: string): void { pr.show([path]); }
        function tab(name: string): void { pr.tab = name; }
        function close(): void { pr.close(); }
    }

    // ================================================ the window
    LazyLoader {
        active: pr.shown
        FloatingWindow {
            id: win
            title: pr.info ? (pr.multi ? pr.info.count + " items" : pr.info.name) + " – Properties" : "Properties"
            implicitWidth: 600
            implicitHeight: 700
            color: Theme.bg
            visible: true
            onVisibleChanged: if (!visible) pr.close()
            Component.onCompleted: { pr.ui = win; keys.forceActiveFocus(); }
            Component.onDestruction: if (pr.ui === win) pr.ui = null

            function editName() {
                pr.editingName = true;
                nameField.text = pr.newName !== "" ? pr.newName : pr.info.name;
                nameField.forceActiveFocus();
                const dot = pr.info.dir ? -1 : nameField.text.lastIndexOf(".");
                nameField.select(0, dot > 0 ? dot : nameField.text.length);
            }
            function endEditName(keep) {
                if (keep) {
                    const t = nameField.text.trim();
                    pr.newName = t && t !== pr.info.name ? t : "";
                }
                pr.editingName = false;
                nameField.text = Qt.binding(() => !pr.info ? "…" : pr.multi ? pr.plural(pr.info.count, "item") : (pr.newName !== "" ? pr.newName : pr.info.name));
                keys.forceActiveFocus();
            }

            Item {
                id: keys
                anchors.fill: parent
                focus: true
                Keys.onPressed: e => {
                    if (pr.picking) {
                        switch (e.key) {
                        case Qt.Key_Up: case Qt.Key_Backtab: pr.pickIndex = Math.max(0, pr.pickIndex - 1); break;
                        case Qt.Key_Down: case Qt.Key_Tab: pr.pickIndex = Math.min(pr.apps.length, pr.pickIndex + 1); break;
                        case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space:
                            if (pr.pickIndex >= pr.apps.length) pr.openPicker(!pr.pickAll);
                            else pr.pick(pr.apps[pr.pickIndex]);
                            break;
                        case Qt.Key_Escape: case Qt.Key_Backspace: case Qt.Key_Left: pr.picking = false; break;
                        default: return;
                        }
                        e.accepted = true;
                        return;
                    }
                    const ctrl = (e.modifiers & Qt.ControlModifier) !== 0;
                    switch (e.key) {
                    case Qt.Key_Up: case Qt.Key_Backtab: pr.move(-1); break;
                    case Qt.Key_Down: case Qt.Key_Tab:
                        if (ctrl) pr.switchTab(1);
                        else pr.move(1);
                        break;
                    case Qt.Key_Left:
                        if (pr.focusId === "tabs") pr.switchTab(-1);
                        else if (pr.focusId.startsWith("p-") || ["cancel", "apply", "ok"].indexOf(pr.focusId) >= 0) pr.move(-1);
                        break;
                    case Qt.Key_Right:
                        if (pr.focusId === "tabs") pr.switchTab(1);
                        else if (pr.focusId.startsWith("p-") || ["cancel", "apply", "ok"].indexOf(pr.focusId) >= 0) pr.move(1);
                        break;
                    case Qt.Key_PageDown: pr.switchTab(1); break;
                    case Qt.Key_PageUp: pr.switchTab(-1); break;
                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: pr.activate(pr.focusId); break;
                    case Qt.Key_F2: pr.startEditName(); break;
                    case Qt.Key_Escape: case Qt.Key_Backspace: pr.close(); break;
                    default: return;
                    }
                    e.accepted = true;
                }
            }
            Connections {
                target: pr
                function onFocusIdChanged() { Qt.callLater(win.showFocused); }
                function onTabChanged() { if (pr.focusId !== "tabs") pr.focusId = "tabs"; flick.contentY = 0; }
            }
            function showFocused() {
                const it = focusables[pr.focusId];
                if (!it || !it.visible) return;
                const p = it.mapToItem(flick.contentItem, 0, 0);
                if (p.y < flick.contentY) flick.contentY = Math.max(0, p.y - 12);
                else if (p.y + it.height > flick.contentY + flick.height) flick.contentY = p.y + it.height - flick.height + 12;
            }
            property var focusables: ({})
            function reg(id, item) { const f = focusables; f[id] = item; focusables = f; }

            ColumnLayout {
                anchors.fill: parent
                spacing: 0

                // ------------------------------------------------ header: icon, name, type
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 104
                    color: Theme.panel
                    RowLayout {
                        anchors { fill: parent; leftMargin: 22; rightMargin: 22 }
                        spacing: 16
                        Item {
                            Layout.preferredWidth: 64
                            Layout.preferredHeight: 64
                            IconImage {
                                anchors.centerIn: parent
                                visible: thumb.status !== Image.Ready
                                implicitSize: 56
                                source: !pr.info ? "" : pr.multi ? Quickshell.iconPath("folder-documents", "folder")
                                    : Quickshell.iconPath(pr.info.icon || "", pr.info.iconFallback || "text-x-generic")
                            }
                            Image {
                                id: thumb
                                anchors.fill: parent
                                visible: status === Image.Ready
                                source: pr.single && /^image\/(png|jpeg|gif|webp|bmp|svg)/.test(pr.info.mime) && pr.info.size < 40 * 1024 * 1024 ? pr.fileUrl(pr.info.path) : ""
                                sourceSize: Qt.size(128, 128)
                                fillMode: Image.PreserveAspectFit
                                asynchronous: true
                            }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 6
                            Rectangle {
                                id: nameBox
                                Layout.fillWidth: true
                                Layout.preferredHeight: 44
                                radius: 10
                                color: pr.canRename ? Theme.panel3 : "transparent"
                                border.width: 2
                                border.color: nameField.activeFocus || pr.focusId === "name" ? Theme.accent : "transparent"
                                Component.onCompleted: win.reg("name", nameBox)
                                TextInput {
                                    id: nameField
                                    anchors { fill: parent; leftMargin: pr.canRename ? 12 : 0; rightMargin: 12 }
                                    verticalAlignment: TextInput.AlignVCenter
                                    text: !pr.info ? "…" : pr.multi ? pr.plural(pr.info.count, "item") : (pr.newName !== "" ? pr.newName : pr.info.name)
                                    readOnly: !pr.canRename
                                    color: Theme.text
                                    selectionColor: Theme.accent
                                    selectedTextColor: Theme.accentInk
                                    font.family: Theme.font
                                    font.pixelSize: 18
                                    font.weight: Font.DemiBold
                                    selectByMouse: pr.canRename
                                    clip: true
                                    onActiveFocusChanged: if (activeFocus && !pr.editingName && pr.canRename) {
                                        pr.editingName = true;
                                        pr.focusId = "name";
                                    }
                                    Keys.onPressed: e => {
                                        switch (e.key) {
                                        case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Down: case Qt.Key_Tab: win.endEditName(true); break;
                                        case Qt.Key_Escape: win.endEditName(false); break;
                                        case Qt.Key_Up: break;
                                        default: return;
                                        }
                                        e.accepted = true;
                                    }
                                    onEditingFinished: if (pr.editingName) win.endEditName(true)
                                }
                            }
                            Text {
                                Layout.fillWidth: true
                                text: !pr.info ? (pr.loadError || "Looking…") : pr.multi ? (pr.info.desc ? "All " + pr.info.desc : pr.plural(pr.info.files, "file") + ", " + pr.plural(pr.info.folders, "folder")) : pr.typeText(pr.info)
                                color: pr.loadError ? Theme.danger : Theme.muted
                                font.family: Theme.font
                                font.pixelSize: 13
                                elide: Text.ElideRight
                            }
                        }
                    }
                }

                // ------------------------------------------------ tabs
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 54
                    color: Theme.panel
                    Row {
                        id: tabRow
                        anchors { left: parent.left; leftMargin: 18; verticalCenter: parent.verticalCenter }
                        spacing: 6
                        Component.onCompleted: win.reg("tabs", tabRow)
                        Repeater {
                            model: pr.tabs
                            delegate: Rectangle {
                                id: tb
                                required property var modelData
                                readonly property bool on: pr.tab === modelData.id
                                width: tbText.implicitWidth + 32
                                height: 38
                                radius: 10
                                color: on ? (pr.focusId === "tabs" ? Theme.accent : Theme.panel3) : tbMouse.containsMouse ? Theme.panel2 : "transparent"
                                Text {
                                    id: tbText
                                    anchors.centerIn: parent
                                    text: tb.modelData.label
                                    color: tb.on && pr.focusId === "tabs" ? Theme.accentInk : Theme.text
                                    font.family: Theme.font
                                    font.pixelSize: 14
                                    font.weight: tb.on ? Font.DemiBold : Font.Normal
                                }
                                MouseArea {
                                    id: tbMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: { keys.forceActiveFocus(); pr.tab = tb.modelData.id; }
                                }
                            }
                        }
                    }
                }
                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line }

                // ------------------------------------------------ the tab's content
                Flickable {
                    id: flick
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentHeight: body.implicitHeight + 36
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    ColumnLayout {
                        id: body
                        x: 22
                        y: 18
                        width: flick.width - 44
                        spacing: 10

                        // General ------------------------------------------------
                        ColumnLayout {
                            Layout.fillWidth: true
                            visible: pr.tab === "general" && !!pr.info
                            spacing: 10

                            PropRow {
                                id: locRow
                                label: "Location"
                                value: !pr.info ? "" : pr.single ? pr.info.parent : (pr.info.parent || "Different folders")
                                visible: !!pr.info
                                button: "Copy"
                                buttonFocused: pr.focusId === "copyloc"
                                onPressed: pr.activate("copyloc")
                                Component.onCompleted: win.reg("copyloc", locRow)
                            }
                            PropRow {
                                id: targetRow
                                label: "Points to"
                                visible: pr.single && !!pr.info.target
                                value: pr.single ? (pr.info.target || "") + (pr.info.broken ? "  (missing)" : "") : ""
                                button: "Copy"
                                buttonFocused: pr.focusId === "copytarget"
                                onPressed: pr.activate("copytarget")
                                Component.onCompleted: win.reg("copytarget", targetRow)
                            }
                            PropRow {
                                label: pr.multi ? "Total size" : "Size"
                                visible: !!pr.info
                                value: {
                                    const i = pr.info;
                                    if (!i) return "";
                                    if (i.multi || i.dir) {
                                        const c = pr.counted;
                                        if (!c) return "…";
                                        return pr.fmtSize(c.size) + " (" + pr.exact(c.size) + " bytes)" + (c.done ? "" : "  · counting…");
                                    }
                                    return pr.fmtSize(i.size) + " (" + pr.exact(i.size) + " bytes)";
                                }
                            }
                            PropRow {
                                label: "Size on disk"
                                visible: !!pr.info && !(pr.single && pr.info.broken)
                                value: {
                                    const i = pr.info;
                                    if (!i) return "";
                                    if (i.multi || i.dir) return pr.counted ? pr.fmtSize(pr.counted.disk) + " (" + pr.exact(pr.counted.disk) + " bytes)" : "…";
                                    return pr.fmtSize(i.disk) + " (" + pr.exact(i.disk) + " bytes)";
                                }
                            }
                            PropRow {
                                label: "Contains"
                                visible: !!pr.info && (pr.multi || pr.info.dir)
                                value: {
                                    const c = pr.counted;
                                    if (!c) return "…";
                                    return pr.plural(c.files, "file") + ", " + pr.plural(c.folders, "folder")
                                        + (c.unreadable ? " (" + pr.plural(c.unreadable, "folder") + " couldn't be read)" : "")
                                        + (c.done ? "" : "  · counting…");
                                }
                            }
                            PropRow {
                                label: "Picture size"
                                visible: pr.single && !!pr.info.width
                                value: pr.single && pr.info.width ? pr.info.width + " × " + pr.info.height + " pixels" : ""
                            }
                            PropRow {
                                label: "Length"
                                visible: pr.single && !!pr.info.duration
                                value: pr.single ? pr.fmtDuration(pr.info.duration) : ""
                            }
                            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line; visible: pr.single }
                            PropRow { label: "Created"; visible: pr.single; value: pr.single ? pr.fmtWhen(pr.info.btime) : "" }
                            PropRow { label: "Modified"; visible: pr.single; value: pr.single ? pr.fmtWhen(pr.info.mtime) : "" }
                            PropRow { label: "Accessed"; visible: pr.single; value: pr.single ? pr.fmtWhen(pr.info.atime) : "" }
                            PropRow {
                                label: "Original location"
                                visible: pr.single && !!pr.info.origPath
                                value: pr.single ? (pr.info.origPath || "") : ""
                            }
                            PropRow {
                                label: "Moved to Trash"
                                visible: pr.single && !!pr.info.deleted
                                value: pr.single && pr.info.deleted ? pr.info.deleted.replace("T", ", ") : ""
                            }
                            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line; visible: pr.isFile && !pr.info.trashed }
                            PropRow {
                                id: changeRow
                                label: "Opens with"
                                visible: pr.isFile && !pr.info.trashed
                                value: pr.newApp ? pr.newApp.name + "  (after Apply)" : pr.single && pr.info.app ? pr.info.app.name : "No app chosen"
                                icon: pr.newApp ? pr.newApp.icon : pr.single && pr.info.app ? pr.info.app.icon : ""
                                button: "Change…"
                                buttonFocused: pr.focusId === "change"
                                onPressed: pr.activate("change")
                                Component.onCompleted: win.reg("change", changeRow)
                            }
                            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line; visible: !!pr.info && pr.info.total > 0 }
                            PropRow {
                                label: pr.single && pr.info.mount ? "Drive" : "On the drive"
                                visible: !!pr.info && pr.info.total > 0
                                value: !pr.info || !pr.info.total ? "" : pr.fmtSize(pr.info.free) + " free of " + pr.fmtSize(pr.info.total)
                                    + (pr.single && pr.info.device && pr.info.mount ? "  · " + pr.info.device + (pr.info.fs ? " (" + pr.info.fs + ")" : "") : "")
                            }
                            // Used / free, for a drive itself.
                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.leftMargin: 150
                                visible: pr.single && pr.info.mount && pr.info.total > 0
                                spacing: 6
                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: 10
                                    radius: 5
                                    color: Theme.panel3
                                    Rectangle {
                                        width: !pr.single || !pr.info.total ? 0 : parent.width * Math.max(0, Math.min(1, 1 - pr.info.free / pr.info.total))
                                        height: parent.height
                                        radius: 5
                                        color: pr.single && pr.info.total && pr.info.free / pr.info.total < 0.1 ? Theme.danger : Theme.accent
                                    }
                                }
                                Text {
                                    text: !pr.single || !pr.info.total ? "" : pr.fmtSize(pr.info.total - pr.info.free) + " used · " + Math.round((1 - pr.info.free / pr.info.total) * 100) + "% full"
                                    color: Theme.muted
                                    font.family: Theme.font
                                    font.pixelSize: 12
                                }
                            }
                        }

                        // Permissions ------------------------------------------------
                        ColumnLayout {
                            Layout.fillWidth: true
                            visible: pr.tab === "permissions" && pr.single
                            spacing: 12
                            PropRow { label: "Owner"; value: pr.single ? pr.info.owner + (pr.info.mine ? " (you)" : "") : "" }
                            PropRow { label: "Group"; value: pr.single ? pr.info.group : "" }
                            Text {
                                Layout.fillWidth: true
                                visible: pr.single && !pr.info.mine
                                text: pr.single ? "Only " + pr.info.owner + " can change these permissions." : ""
                                color: Theme.muted
                                wrapMode: Text.WordWrap
                                font.family: Theme.font
                                font.pixelSize: 13
                            }
                            GridLayout {
                                Layout.fillWidth: true
                                Layout.topMargin: 6
                                columns: 4
                                columnSpacing: 10
                                rowSpacing: 8
                                Item { Layout.preferredWidth: 140; Layout.preferredHeight: 1 }
                                Repeater {
                                    model: pr.single && pr.info.dir ? ["List files", "Change files", "Open"] : ["Read", "Write", "Execute"]
                                    delegate: Text {
                                        required property string modelData
                                        Layout.fillWidth: true
                                        horizontalAlignment: Text.AlignHCenter
                                        text: modelData
                                        color: Theme.muted
                                        font.family: Theme.font
                                        font.pixelSize: 12
                                        font.weight: Font.DemiBold
                                    }
                                }
                                Repeater {
                                    model: [{ w: "u", label: "Owner" }, { w: "g", label: "Group" }, { w: "o", label: "Others" }]
                                    delegate: Item {
                                        id: permRow
                                        required property var modelData
                                        Layout.columnSpan: 4
                                        Layout.fillWidth: true
                                        implicitHeight: 46
                                        RowLayout {
                                            anchors.fill: parent
                                            spacing: 10
                                            Text {
                                                Layout.preferredWidth: 140
                                                text: permRow.modelData.label
                                                color: Theme.text
                                                font.family: Theme.font
                                                font.pixelSize: 14
                                            }
                                            Repeater {
                                                model: ["r", "w", "x"]
                                                delegate: Item {
                                                    id: cell
                                                    required property string modelData
                                                    readonly property string fid: "p-" + permRow.modelData.w + modelData
                                                    readonly property int bit: pr.bitOf(fid)
                                                    Layout.fillWidth: true
                                                    Layout.preferredHeight: 44
                                                    Component.onCompleted: win.reg(fid, cell)
                                                    Check {
                                                        anchors.centerIn: parent
                                                        checked: (pr.mode & cell.bit) !== 0
                                                        enabled: pr.single && pr.info.mine
                                                        focused: pr.focusId === cell.fid
                                                        onToggled: { keys.forceActiveFocus(); pr.focusId = cell.fid; pr.setBit(cell.bit, !((pr.mode & cell.bit) !== 0)); }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Rectangle {
                                id: execRow
                                Layout.fillWidth: true
                                Layout.preferredHeight: 52
                                visible: pr.single && !pr.info.dir
                                radius: 10
                                color: Theme.panel2
                                border.width: 2
                                border.color: pr.focusId === "exec" ? Theme.accent : "transparent"
                                Component.onCompleted: win.reg("exec", execRow)
                                RowLayout {
                                    anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                                    spacing: 12
                                    Check {
                                        checked: (pr.mode & 0o100) !== 0
                                        enabled: pr.single && pr.info.mine
                                        onToggled: { keys.forceActiveFocus(); pr.focusId = "exec"; pr.setExec(!((pr.mode & 0o100) !== 0)); }
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: "Allow executing as a program"
                                        color: Theme.text
                                        font.family: Theme.font
                                        font.pixelSize: 14
                                    }
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    z: -1
                                    onClicked: { keys.forceActiveFocus(); pr.focusId = "exec"; pr.setExec(!((pr.mode & 0o100) !== 0)); }
                                }
                            }
                            Text {
                                Layout.fillWidth: true
                                text: pr.single ? "Permissions: " + ("000" + pr.mode.toString(8)).slice(-4).replace(/^0/, "") + "  ·  " + pr.permText(pr.mode, pr.info.dir) + (pr.newMode >= 0 && pr.newMode !== pr.info.mode ? "  (after Apply)" : "") : ""
                                color: Theme.muted
                                font.family: Theme.font
                                font.pixelSize: 13
                            }
                        }

                        // Checksums ------------------------------------------------
                        ColumnLayout {
                            Layout.fillWidth: true
                            visible: pr.tab === "checksums" && pr.isFile
                            spacing: 12
                            Text {
                                Layout.fillWidth: true
                                text: "Checksums show whether a download arrived whole: compare them with the ones the website lists."
                                color: Theme.muted
                                wrapMode: Text.WordWrap
                                font.family: Theme.font
                                font.pixelSize: 13
                            }
                            Repeater {
                                model: [{ id: "md5", label: "MD5" }, { id: "sha1", label: "SHA-1" }, { id: "sha256", label: "SHA-256" }]
                                delegate: Rectangle {
                                    id: sumRow
                                    required property var modelData
                                    readonly property var st: pr.sums[modelData.id] || null
                                    readonly property bool lit: pr.focusId === "sum-" + modelData.id || pr.focusId === "copy-" + modelData.id
                                    Layout.fillWidth: true
                                    implicitHeight: sumCol.implicitHeight + 24
                                    radius: 12
                                    color: Theme.panel2
                                    border.width: 2
                                    border.color: lit ? Theme.accent : "transparent"
                                    Component.onCompleted: { win.reg("sum-" + modelData.id, sumRow); win.reg("copy-" + modelData.id, sumRow); }
                                    ColumnLayout {
                                        id: sumCol
                                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12 }
                                        spacing: 8
                                        RowLayout {
                                            Layout.fillWidth: true
                                            spacing: 10
                                            Text {
                                                Layout.fillWidth: true
                                                text: sumRow.modelData.label
                                                color: Theme.text
                                                font.family: Theme.font
                                                font.pixelSize: 14
                                                font.weight: Font.DemiBold
                                            }
                                            SmallButton {
                                                label: sumRow.st && sumRow.st.sum ? "Copy" : sumRow.st && sumRow.st.running ? (sumRow.st.waiting ? "Waiting…" : "Working…") : "Calculate"
                                                focused: sumRow.lit
                                                onPressed: {
                                                    keys.forceActiveFocus();
                                                    if (sumRow.st && sumRow.st.sum) pr.copyText(sumRow.st.sum);
                                                    else pr.computeSum(sumRow.modelData.id);
                                                }
                                            }
                                        }
                                        Rectangle {
                                            Layout.fillWidth: true
                                            Layout.preferredHeight: 6
                                            visible: !!sumRow.st && sumRow.st.running && !sumRow.st.waiting
                                            radius: 3
                                            color: Theme.panel3
                                            Rectangle {
                                                width: sumRow.st && sumRow.st.total ? parent.width * Math.min(1, sumRow.st.done / sumRow.st.total) : 0
                                                height: parent.height
                                                radius: 3
                                                color: Theme.accent
                                            }
                                        }
                                        TextEdit {
                                            Layout.fillWidth: true
                                            visible: !!sumRow.st && (!!sumRow.st.sum || !!sumRow.st.error)
                                            text: sumRow.st ? (sumRow.st.sum || sumRow.st.error || "") : ""
                                            readOnly: true
                                            selectByMouse: true
                                            wrapMode: TextEdit.WrapAnywhere
                                            color: sumRow.st && sumRow.st.error ? Theme.danger : Theme.text
                                            selectionColor: Theme.accent
                                            selectedTextColor: Theme.accentInk
                                            font.family: "monospace"
                                            font.pixelSize: 13
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ------------------------------------------------ problems and buttons
                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.line }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: errText.visible ? 112 : 72
                    color: Theme.panel
                    ColumnLayout {
                        anchors { fill: parent; leftMargin: 18; rightMargin: 18; topMargin: 12; bottomMargin: 12 }
                        spacing: 8
                        Text {
                            id: errText
                            Layout.fillWidth: true
                            visible: pr.error !== ""
                            text: pr.error
                            color: Theme.danger
                            wrapMode: Text.WordWrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                            font.family: Theme.font
                            font.pixelSize: 13
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 10
                            Text {
                                Layout.fillWidth: true
                                text: pr.busy ? "Applying…" : "Ⓐ choose  Ⓑ close"
                                color: Theme.muted
                                font.family: Theme.font
                                font.pixelSize: 12
                                elide: Text.ElideRight
                            }
                            SmallButton {
                                id: cancelBtn
                                label: pr.dirty ? "Cancel" : "Close"
                                focused: pr.focusId === "cancel"
                                onPressed: pr.close()
                                Component.onCompleted: win.reg("cancel", cancelBtn)
                            }
                            SmallButton {
                                id: applyBtn
                                visible: pr.single
                                label: "Apply"
                                dim: !pr.dirty
                                focused: pr.focusId === "apply"
                                onPressed: { keys.forceActiveFocus(); pr.apply(false); }
                                Component.onCompleted: win.reg("apply", applyBtn)
                            }
                            SmallButton {
                                id: okBtn
                                label: "OK"
                                primary: true
                                focused: pr.focusId === "ok"
                                onPressed: { keys.forceActiveFocus(); if (pr.single) pr.apply(true); else pr.close(); }
                                Component.onCompleted: win.reg("ok", okBtn)
                            }
                        }
                    }
                }
            }

            // ------------------------------------------------ Change…: the apps that open this kind of file
            Rectangle {
                anchors.fill: parent
                visible: pr.picking
                color: "#99000000"
                MouseArea { anchors.fill: parent; onClicked: pr.picking = false }
                Rectangle {
                    anchors.centerIn: parent
                    width: Math.min(parent.width - 40, 480)
                    height: Math.min(parent.height - 40, pickCol.implicitHeight + 40)
                    radius: Theme.radius + 2
                    color: Theme.panel
                    border.color: Theme.line
                    MouseArea { anchors.fill: parent }
                    ColumnLayout {
                        id: pickCol
                        anchors { fill: parent; margins: 20 }
                        spacing: 10
                        Text {
                            Layout.fillWidth: true
                            text: pr.single ? "Open " + (pr.info.desc || "this kind of file") + " with" : ""
                            color: Theme.text
                            elide: Text.ElideRight
                            font.family: Theme.font
                            font.pixelSize: 18
                            font.weight: Font.DemiBold
                        }
                        Text {
                            Layout.fillWidth: true
                            visible: pr.apps.length === 0
                            text: "Looking for apps…"
                            color: Theme.muted
                            font.family: Theme.font
                            font.pixelSize: 14
                        }
                        ListView {
                            id: pickList
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            Layout.preferredHeight: Math.min(contentHeight, 420)
                            model: pr.apps.concat([{ id: "", name: pr.pickAll ? "Only the suggested apps" : "Other app…", icon: "", other: true }])
                            spacing: 4
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            currentIndex: pr.pickIndex
                            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                            delegate: Rectangle {
                                id: ap
                                required property var modelData
                                required property int index
                                readonly property bool lit: pr.pickIndex === index
                                width: ListView.view.width
                                height: 48
                                radius: 10
                                color: lit ? Theme.accent : apMouse.containsMouse ? Theme.panel3 : Theme.panel2
                                RowLayout {
                                    anchors { fill: parent; leftMargin: 12; rightMargin: 14 }
                                    spacing: 12
                                    IconImage {
                                        visible: !ap.modelData.other
                                        implicitSize: 28
                                        source: ap.modelData.other ? "" : Quickshell.iconPath(ap.modelData.icon || "", "application-x-executable")
                                    }
                                    Text {
                                        Layout.fillWidth: true
                                        text: ap.modelData.name
                                        color: ap.lit ? Theme.accentInk : Theme.text
                                        elide: Text.ElideRight
                                        font.family: Theme.font
                                        font.pixelSize: 14
                                        font.weight: ap.lit ? Font.DemiBold : Font.Normal
                                    }
                                    Text {
                                        visible: !!ap.modelData.default
                                        text: "Default"
                                        color: ap.lit ? Theme.accentInk : Theme.muted
                                        font.family: Theme.font
                                        font.pixelSize: 12
                                    }
                                }
                                MouseArea {
                                    id: apMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        keys.forceActiveFocus();
                                        if (ap.modelData.other) pr.openPicker(!pr.pickAll);
                                        else pr.pick(ap.modelData);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    function permText(m, dir) {
        let s = dir ? "d" : "-";
        for (const sh of [6, 3, 0]) {
            s += (m >> sh) & 4 ? "r" : "-";
            s += (m >> sh) & 2 ? "w" : "-";
            s += (m >> sh) & 1 ? "x" : "-";
        }
        return s;
    }

    // ================================================ small pieces
    // A label and its value (selectable), with an optional button on the right.
    component PropRow: Item {
        id: row
        property string label
        property string value
        property string icon: ""
        property string button: ""
        property bool buttonFocused: false
        signal pressed()
        Layout.fillWidth: true
        implicitHeight: Math.max(valueText.implicitHeight, button !== "" ? 40 : 0, 22)
        Text {
            id: labelText
            width: 140
            anchors.verticalCenter: parent.verticalCenter
            text: row.label
            color: Theme.muted
            font.family: Theme.font
            font.pixelSize: 13
            wrapMode: Text.WordWrap
        }
        IconImage {
            id: rowIcon
            visible: row.icon !== ""
            x: 150
            anchors.verticalCenter: parent.verticalCenter
            implicitSize: row.icon !== "" ? 22 : 0
            source: row.icon !== "" ? Quickshell.iconPath(row.icon, "application-x-executable") : ""
        }
        TextEdit {
            id: valueText
            x: 150 + (row.icon !== "" ? 30 : 0)
            width: parent.width - x - (btn.visible ? btn.width + 12 : 0)
            anchors.verticalCenter: parent.verticalCenter
            text: row.value
            readOnly: true
            selectByMouse: true
            wrapMode: TextEdit.WrapAnywhere
            color: Theme.text
            selectionColor: Theme.accent
            selectedTextColor: Theme.accentInk
            font.family: Theme.font
            font.pixelSize: 14
        }
        SmallButton {
            id: btn
            visible: row.button !== ""
            anchors { right: parent.right; verticalCenter: parent.verticalCenter }
            label: row.button
            focused: row.buttonFocused
            onPressed: row.pressed()
        }
    }

    component SmallButton: Rectangle {
        id: sb
        property string label
        property bool focused: false
        property bool primary: false
        property bool dim: false
        signal pressed()
        implicitWidth: sbText.implicitWidth + 30
        implicitHeight: 40
        radius: 10
        opacity: dim ? 0.5 : 1
        color: primary ? Theme.accent : sbMouse.containsMouse ? Theme.panel3 : Theme.panel2
        border.width: 2
        border.color: focused ? (primary ? Theme.text : Theme.accent) : "transparent"
        Text {
            id: sbText
            anchors.centerIn: parent
            text: sb.label
            color: sb.primary ? Theme.accentInk : Theme.text
            font.family: Theme.font
            font.pixelSize: 13
            font.weight: sb.primary ? Font.DemiBold : Font.Normal
        }
        MouseArea {
            id: sbMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: sb.pressed()
        }
    }

    component Check: Rectangle {
        id: ck
        property bool checked: false
        property bool focused: false
        signal toggled()
        implicitWidth: 28
        implicitHeight: 28
        width: 28
        height: 28
        radius: 8
        opacity: enabled ? 1 : 0.45
        color: checked ? Theme.accent : Theme.panel3
        border.width: 2
        border.color: focused ? Theme.text : checked ? Theme.accent : Theme.line
        Icon {
            anchors.centerIn: parent
            visible: ck.checked
            name: "check"
            size: 16
            color: Theme.accentInk
        }
        MouseArea {
            anchors.fill: parent
            anchors.margins: -8
            enabled: ck.enabled
            cursorShape: Qt.PointingHandCursor
            onClicked: ck.toggled()
        }
    }
}

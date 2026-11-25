// Every Mantel app made of blocks (the owner, Nov 24: "Modular for all Hearth apps is required as well", and: editing
// an app's layout must feel natural, never buried). One shared piece, put over an app's window after its own content:
//
//   AppBlocks {
//       anchors.fill: parent; app: "files"; appName: "Hearth Files"; page: "window"
//       pages: [{ id: "window", title: "Hearth Files", blocks: [{ id: "sidebar", title: "The navigation pane", x: 0, y: 6, w: 4, h: 18 }, …] }]
//       parts: ({ sidebar: navPane, … })     // the app's own items, by block id, for the page shown
//       focusBack: keys                       // what gets the keys back after Edit layout
//   }
//
// Nothing customised: nothing here does anything; every part is where the app's own layout puts it, exactly as
// before (no item moved, no timer, no polling). What's kept is read from ~/.config/nimbus/apps/<app>.json (the Rust
// apps' own file and format, hosts/hearth-ui/src/blocks.rs; Mantel's pages are "mantel-<page>") and written only by
// `hearth-layout app-keep` (settings_file: atomic, .bak, merge-on-write, one writer); a file that can't be read is
// never written over (the app stays as it is and says so).
//
// The ways in, right where you are:
//   - right-click any part (the toolbar, a pane, the status bar…): "<part>": Edit layout · Move this · Hide this ·
//     Describe it… · Back to the original
//   - the app's own menu (Files: View › LAYOUT, and the empty space's right-click menu): Edit layout, Describe it…
// Edit layout turns the live app into edit mode in place: each part gets a soft outline and its name on hover; drag
// anywhere on a part to move it (let go over another to swap them), drag its edges or corners to size it (the parts
// beside that edge follow, nothing is left empty); ✕ hides it, and hidden parts wait in the tray to be clicked or
// dragged back. Every change is kept as you go, with an Undo toast; Ctrl+Z, Back to the original, Esc or Done.
// The controller: LB / RB (or Tab) pick a part, the D-pad moves it, LT / RT make it smaller / bigger, Shift + the
// arrows size it, X (Menu) hides it, B (Esc) is done. Describe it (your AI) is beside it in the bar.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "app-blocks.js" as L

Item {
    id: ab
    property string app: ""
    property string appName: ""
    property string page: "window"
    property var pages: []
    property var parts: ({})
    property color background: Theme.bg
    property Item focusBack: null
    // The app's menus whose items can be put in order or left out (Files: View and the right-click menu):
    // [{ id: "view", title: "View", items: function () { return [{ id, label }, …] } }]
    property var menus: []
    // false: everything as the app has it (while the app needs its whole window for something else)
    property bool active: true
    signal describeRequested()

    readonly property int cols: L.COLS
    readonly property int rows: L.ROWS
    readonly property int maxBlocks: 16
    readonly property string pageId: "mantel-" + page
    readonly property var def: { for (let i = 0; i < pages.length; i++) if (pages[i].id === page) return pages[i]; return null; }
    readonly property var defs: def ? def.blocks.slice(0, maxBlocks) : []

    // ---------------------------------------------------------------- what's kept
    readonly property string file: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/apps/" + app + ".json"
    property var kept: ({})
    property bool unreadable: false
    FileView {
        id: fv
        path: ab.app !== "" ? ab.file : ""
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: ab.readKept(text())
        onLoadFailed: { ab.kept = {}; ab.unreadable = false; }
    }
    Connections { target: AppBlocksHub; function onChanged(a) { if (a === ab.app) fv.reload(); } }
    function readKept(t) {
        if (String(t).trim() === "") { kept = {}; unreadable = false; return; }
        let o = null;
        try { o = JSON.parse(t); } catch (e) { o = null; }
        if (!o || typeof o !== "object" || Array.isArray(o)) {
            // never written over: the app as it is, and nothing kept until it can be read (hearth-layout puts back
            // the last good copy, FILE.bak, the next time it keeps something)
            console.warn("AppBlocks: " + file + " couldn't be read; " + app + " stays as it is");
            kept = {}; unreadable = true; return;
        }
        kept = o; unreadable = false;
    }
    readonly property var layoutKept: kept && kept.layout && typeof kept.layout === "object" ? kept.layout : ({})
    readonly property var savedPage: layoutKept.blocks && typeof layoutKept.blocks === "object" ? layoutKept.blocks[pageId] : undefined
    readonly property var res: L.resolve(defs, savedPage, layoutKept, cols, rows)
    readonly property bool customised: {
        const b = layoutKept.blocks || {}, z = layoutKept.zones || {};
        for (const k in b) if (k.indexOf("mantel-") === 0 && b[k] && Object.keys(b[k]).length) return true;
        for (const k in z) if (k.indexOf("menu:") === 0 && Array.isArray(z[k]) && z[k].length) return true;
        return res.mode !== "own";
    }

    // ---------------------------------------------------------------- where the parts go
    property bool editing: false
    property var draft: []                 // the page's blocks while editing
    property var start: []                 // as they were when Edit layout opened
    property string startMode: "own"
    property string picked: ""
    property var shownMap: ({})            // block id → is the part showing anything right now (the details pane open)
    readonly property string mode: !active || defs.length === 0 ? "own" : editing ? "grid" : res.mode
    readonly property var placed: editing ? draft : res.placed
    readonly property var rects: mode === "grid" ? (editing ? placed : L.reflow(placed, shownMap)) : []
    readonly property real cw: width / cols
    readonly property real ch: height / rows

    onModeChanged: schedule()
    onPlacedChanged: schedule()
    onPartsChanged: schedule()
    function schedule() { Qt.callLater(ab.applyNow); }
    Component.onCompleted: { if (app !== "") AppBlocksHub.add(app, ab); schedule(); }
    Component.onDestruction: AppBlocksHub.remove(app, ab)

    // Each part's own place (its parent and the next sibling that isn't a part), noted once, before it first moves.
    property var homes: []
    property var catchers: []
    function alive(it) { try { return !!it && it.width !== undefined && it.parent !== undefined; } catch (e) { return false; } }
    function homeOf(it) { for (let i = 0; i < homes.length; i++) if (homes[i].item === it) return homes[i]; return null; }
    function isPartItem(it) { for (const k in parts) if (parts[k] === it) return true; for (let i = 0; i < homes.length; i++) if (homes[i].item === it) return true; return false; }
    function remember(it) {
        let h = homeOf(it);
        if (h) return h;
        const par = it.parent;
        if (!par) return null;
        const kids = par.children;
        let idx = -1;
        for (let i = 0; i < kids.length; i++) if (kids[i] === it) { idx = i; break; }
        let anchor = null;
        for (let i = idx + 1; i < kids.length; i++) if (!isPartItem(kids[i])) { anchor = kids[i]; break; }
        h = { item: it, parent: par, anchor: anchor, index: idx, w: it.width, h: it.height, at: "home" };
        homes.push(h);
        return h;
    }
    function goHome(h) {
        const it = h.item;
        if (!alive(it) || h.at === "home") return;
        it.width = h.w; it.height = h.h;          // the holder's size let go of (its own layout sizes it again)
        it.parent = h.parent;
        if (h.anchor && alive(h.anchor) && h.anchor.parent === h.parent) it.stackBefore(h.anchor);
        h.at = "home";
    }
    function goTo(h, slot, where) {
        const it = h.item;
        if (h.at === where && it.parent === slot) return;
        if (h.at === "home") { h.w = it.width; h.h = it.height; }
        it.parent = slot;
        it.x = 0; it.y = 0;
        if (where !== "park") {
            it.width = Qt.binding(() => slot.width);
            it.height = Qt.binding(() => slot.height);
        }
        h.at = where;
    }
    function applyNow() {
        homes = homes.filter(h => alive(h.item));
        const ids = defs.map(d => d.id);
        for (const id of ids) if (alive(parts[id])) { remember(parts[id]); addCatcher(parts[id], id); }
        // home first (in their own order), then away
        const want = new Map();
        for (let i = 0; i < defs.length; i++) {
            const it = parts[defs[i].id];
            if (!alive(it)) continue;
            const p = placed[i];
            want.set(it, mode === "own" ? "home" : p && p.hidden ? "park" : mode === "natural" ? "home" : "h" + i);
        }
        const going = homes.filter(h => (want.get(h.item) || "home") === "home" && h.at !== "home").sort((a, b) => a.index - b.index);
        for (const h of going) goHome(h);
        for (let i = 0; i < defs.length; i++) {
            const it = parts[defs[i].id];
            const w = alive(it) ? want.get(it) : undefined;
            const hd = holders.itemAt(i);
            if (hd) hd.part = w && w !== "home" && w !== "park" ? it : null;
            if (!w || w === "home") continue;
            const h = homeOf(it);
            if (!h) continue;
            if (w === "park") goTo(h, park, "park");
            else if (hd) goTo(h, hd.slot, w);
        }
    }
    function noteShown(id, on) {
        if (id === "" || shownMap[id] === on) return;
        const m = Object.assign({}, shownMap); m[id] = on; shownMap = m;
    }

    // A right-click on a part that doesn't take it itself: that part's own menu (Edit layout, Move this, Hide this…).
    Component {
        id: catcherComp
        MouseArea {
            property string bid: ""
            anchors.fill: parent
            z: -1000
            acceptedButtons: Qt.RightButton
            onClicked: m => { const q = mapToItem(ab, m.x, m.y); ab.partMenu(bid, q.x, q.y); }
        }
    }
    function addCatcher(it, id) {
        if (it.spacing !== undefined) return;     // a layout or a row lays out its children: it gets none
        for (const c of catchers) if (c.item === it) return;
        const m = catcherComp.createObject(it, { bid: id });
        if (m) catchers.push({ item: it, mouse: m });
    }

    // ---------------------------------------------------------------- the page laid out on the grid
    Rectangle {
        anchors.fill: parent
        visible: ab.mode === "grid"
        color: ab.background
        // while editing, the app under the parts takes no clicks (a click on empty space picks nothing)
        MouseArea { anchors.fill: parent; enabled: ab.editing; acceptedButtons: Qt.AllButtons; onClicked: ab.picked = "" }
    }
    Item { id: park; visible: false }
    Repeater {
        id: holders
        model: ab.maxBlocks
        delegate: Item {
            id: hd
            required property int index
            readonly property var d: index < ab.defs.length ? ab.defs[index] : null
            readonly property var p: d && ab.mode === "grid" ? ab.placed[index] : null
            readonly property var r: p ? ab.rects[index] : null
            readonly property string bid: d ? d.id : ""
            property Item part: null
            readonly property bool showing: part ? part.visible : true
            onShowingChanged: if (part) ab.noteShown(bid, showing)
            onPartChanged: if (part) ab.noteShown(bid, part.visible)
            readonly property bool plain: !p || L.plainLook(p)
            readonly property bool lit: ab.editing && ab.picked === bid
            property bool hovered: body.containsMouse || grip.containsMouse
            visible: !!p && !p.hidden
            x: r ? Math.round(r.x * ab.cw) : 0
            y: r ? Math.round(r.y * ab.ch) : 0
            width: r ? Math.round((r.x + r.w) * ab.cw) - x : 0
            height: r ? Math.round((r.y + r.h) * ab.ch) - y : 0
            z: lit ? 3 : ab.dragId === bid ? 4 : 1
            property alias slot: slotItem
            // the block's card: its own background, or the look it was given
            Rectangle {
                anchors.fill: parent
                visible: !hd.plain || (hd.d && hd.d.bg !== undefined)
                color: hd.plain ? (hd.d && hd.d.bg !== undefined ? hd.d.bg : "transparent") : Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 1 - (hd.p.seeThrough || 0) / 100)
                radius: hd.p && hd.p.corners !== null && hd.p.corners !== undefined ? hd.p.corners : 0
                border.width: hd.plain ? 0 : hd.p.accent ? 2 : 1
                border.color: hd.p && hd.p.accent ? hd.p.accent : Theme.line
            }
            Item {
                id: slotItem
                readonly property real pad: hd.plain ? 0 : Math.max(2, (hd.p.corners || 0) * 0.35)
                anchors { fill: parent; margins: pad }
                clip: true
            }
            // ---- Edit layout: the soft outline, the name on hover, drag to move, edges and corners to size
            Item {
                anchors.fill: parent
                visible: ab.editing
                z: 10
                Rectangle {
                    anchors.fill: parent
                    color: ab.swapId === hd.bid ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16)
                         : hd.hovered || hd.lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.07) : "transparent"
                    border.width: hd.lit ? 2.5 : 1.5
                    border.color: hd.lit || ab.swapId === hd.bid ? Theme.accent : Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, hd.hovered ? 0.8 : 0.45)
                    radius: 4
                }
                Text {
                    visible: !hd.showing
                    anchors.centerIn: parent
                    text: "Not showing now"
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                }
                MouseArea {
                    id: body
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    cursorShape: ab.dragId === hd.bid ? Qt.ClosedHandCursor : Qt.OpenHandCursor
                    onPressed: m => {
                        if (m.button === Qt.RightButton) return;
                        const q = mapToItem(ab, m.x, m.y); ab.grab(hd.bid, null, q.x, q.y);
                    }
                    onPositionChanged: m => { if (pressed && ab.dragId === hd.bid) { const q = mapToItem(ab, m.x, m.y); ab.dragTo(q.x, q.y); } }
                    onReleased: m => { if (m.button !== Qt.RightButton) ab.release(); }
                    onClicked: m => { if (m.button === Qt.RightButton) { const q = mapToItem(ab, m.x, m.y); ab.partMenu(hd.bid, q.x, q.y); } }
                }
                // its name, and ✕ to hide it
                Rectangle {
                    id: tag
                    visible: hd.hovered || hd.lit || ab.dragId === hd.bid
                    x: 6; y: 6
                    width: tagRow.implicitWidth + 16; height: 26; radius: 13
                    color: hd.lit ? Theme.accent : Theme.panel3
                    border.color: hd.lit ? "transparent" : Theme.line
                    Row {
                        id: tagRow
                        anchors.centerIn: parent
                        spacing: 8
                        Text { text: "⠿  " + (hd.d ? hd.d.title : ""); color: hd.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter }
                        Text {
                            text: "✕"; color: hd.lit ? Theme.accentInk : Theme.muted; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter
                            MouseArea { id: grip; anchors { fill: parent; margins: -6 } hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: ab.hide(hd.bid) }
                        }
                    }
                }
                // edges and corners
                Repeater {
                    model: [{ l: 1 }, { r: 1 }, { t: 1 }, { b: 1 }, { l: 1, t: 1 }, { r: 1, t: 1 }, { l: 1, b: 1 }, { r: 1, b: 1 }]
                    delegate: MouseArea {
                        id: edge
                        required property var modelData
                        readonly property bool corner: (modelData.l || modelData.r) && (modelData.t || modelData.b)
                        readonly property int t: 8
                        x: modelData.r ? parent.width - (corner ? 16 : t) : modelData.l ? 0 : 16
                        y: modelData.b ? parent.height - (corner ? 16 : t) : modelData.t ? 0 : 16
                        width: corner ? 16 : (modelData.l || modelData.r) ? t : Math.max(0, parent.width - 32)
                        height: corner ? 16 : (modelData.t || modelData.b) ? t : Math.max(0, parent.height - 32)
                        hoverEnabled: true
                        cursorShape: corner ? ((modelData.l && modelData.t) || (modelData.r && modelData.b) ? Qt.SizeFDiagCursor : Qt.SizeBDiagCursor)
                                            : (modelData.l || modelData.r) ? Qt.SizeHorCursor : Qt.SizeVerCursor
                        onPressed: m => { const q = mapToItem(ab, m.x, m.y); ab.grab(hd.bid, modelData, q.x, q.y); }
                        onPositionChanged: m => { if (pressed) { const q = mapToItem(ab, m.x, m.y); ab.dragTo(q.x, q.y); } }
                        onReleased: ab.release()
                        Rectangle {
                            visible: edge.corner && (hd.hovered || hd.lit || edge.containsMouse)
                            anchors.centerIn: parent
                            width: 10; height: 10; radius: 3
                            color: Theme.accent; border.color: Theme.accentInk
                        }
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------- Edit layout
    function measure() {
        const out = [], shown = {};
        for (let i = 0; i < defs.length; i++) {
            const d = defs[i], it = parts[d.id];
            const p = L.own(d);
            const r = res.placed[i];
            if (r && res.mode === "grid") { out.push(Object.assign({}, r)); shown[d.id] = true; continue; }
            p.hidden = !!(r && r.hidden);
            const h = alive(it) ? homeOf(it) : null;
            const home = alive(it) && (!h || h.at === "home");
            if (!p.hidden && home && it.visible && it.width > 1 && it.height > 1) {
                const q = it.mapToItem(ab, 0, 0);
                const x = L.clamp(Math.round(q.x / cw), 0, cols - 1), y = L.clamp(Math.round(q.y / ch), 0, rows - 1);
                p.x = x; p.y = y;
                p.w = L.clamp(Math.round((q.x + it.width) / cw), x + 1, cols) - x;
                p.h = L.clamp(Math.round((q.y + it.height) / ch), y + 1, rows) - y;
                shown[d.id] = true;
            } else shown[d.id] = false;
            out.push(p);
        }
        if (res.mode === "grid") return out;
        const placedBeside = L.besides(out, defs, shown);
        // the parts inside or beside others give way (only the ones that are there)
        const live = placedBeside.filter((p, i) => !p.hidden && (shown[p.id] || defs[i].beside));
        const carved = L.carve(live);
        return placedBeside.map(p => carved.find(c => c.id === p.id) || p);
    }
    function edit(pick) {
        if (defs.length === 0) return;
        menuShown = false; menuEditor = false;
        if (!editing) {
            startMode = res.mode;
            const m = measure();
            start = L.copy(m);
            draft = m;
            editing = true;
        }
        picked = pick || (draft.find(p => !p.hidden) || {}).id || "";
        editKeys.forceActiveFocus();
    }
    function done() {
        flushKeys();
        editing = false; picked = ""; dragId = ""; menuEditor = false;
        if (focusBack) focusBack.forceActiveFocus();
    }
    function blockIndex(id) { return draft.findIndex(p => p.id === id); }

    // dragging: a part by its body (move) or by an edge / corner (size)
    property string dragId: ""
    property string swapId: ""
    property var dragEdges: null
    property var dragStart: []
    property point dragFrom: Qt.point(0, 0)
    function grab(id, edges, x, y) {
        flushKeys();
        picked = id; dragId = id; dragEdges = edges; dragStart = L.copy(draft); dragFrom = Qt.point(x, y); swapId = "";
        editKeys.forceActiveFocus();
    }
    function dragTo(x, y) {
        if (dragId === "") return;
        const dx = Math.round((x - dragFrom.x) / cw), dy = Math.round((y - dragFrom.y) / ch);
        if (dragEdges) { draft = L.sized(dragStart, dragId, dragEdges, dx, dy, cols, rows); return; }
        const i = dragStart.findIndex(p => p.id === dragId);
        const d = L.copy(dragStart);
        d[i] = L.moved(dragStart[i], dx, dy, cols, rows);
        draft = d;
        swapId = L.swapWith(d, dragId, dragStart);
    }
    function release() {
        if (dragId === "") return;
        let d = draft;
        if (!dragEdges) d = L.dropped(d, dragId, dragStart);
        const before = dragStart;
        dragId = ""; swapId = ""; dragEdges = null;
        if (L.sameGeo(d, before)) { draft = before; return; }
        change(d, before);
    }
    // One change in Edit layout: kept at once, with Undo.
    function change(d, before) {
        pushUndo({ page: pageId, before: savedCopy(), draft: L.copy(before || draft) });
        draft = d;
        keepPage(L.pageJson(draft, start, startMode));
    }
    function hide(id) {
        if (editing) {
            const i = blockIndex(id);
            if (i < 0) return;
            if (draft.filter(p => !p.hidden && p.id !== id).length === 0) return toast("One part stays, or the window would be empty.", false);
            flushKeys();
            const d = L.copy(draft); d[i].hidden = true;
            if (picked === id) picked = (d.find(p => !p.hidden) || {}).id || "";
            change(d);
            return toast(titleOf(id) + " is hidden. It waits in the tray at the bottom.", true);
        }
        // not editing: hidden right where it is (the rest closes up)
        const pl = L.copy(res.placed);
        const i = pl.findIndex(p => p.id === id);
        if (i < 0) return;
        if (pl.filter(p => !p.hidden && p.id !== id).length === 0) return toast("One part stays, or the window would be empty.", false);
        pl[i].hidden = true;
        pushUndo({ page: pageId, before: savedCopy(), draft: null });
        keepPage(L.toJson(pl, res.mode !== "grid"));
        toast(titleOf(id) + " is hidden. Right-click › Edit layout brings it back.", true);
    }
    function show(id, x, y) {
        const i = blockIndex(id);
        if (i < 0) return;
        const d = L.copy(draft);
        d[i].hidden = false;
        if (x !== undefined) { d[i] = L.moved(d[i], Math.floor(x / cw) - d[i].x, Math.floor(y / ch) - d[i].y, cols, rows); d[i].hidden = false; }
        picked = id;
        change(d);
    }
    function look(part) {
        const i = blockIndex(picked);
        if (i < 0) return;
        const d = L.copy(draft); d[i] = L.nextLook(d[i], part);
        change(d);
    }
    function titleOf(id) { const d = defs.find(x => x.id === id); return d ? d.title : id; }

    // the keys and the controller
    property var keysBefore: null
    Timer { id: keysSettle; interval: 700; onTriggered: ab.flushKeys() }
    function flushKeys() {
        if (!keysSettle.running && keysBefore === null) return;
        keysSettle.stop();
        const b = keysBefore; keysBefore = null;
        if (b && !L.sameGeo(b, draft)) {
            pushUndo({ page: pageId, before: savedCopy(), draft: b });
            keepPage(L.pageJson(draft, start, startMode));
        }
    }
    function nudge(fn) {
        const i = blockIndex(picked);
        if (i < 0) return;
        if (keysBefore === null) keysBefore = L.copy(draft);
        draft = fn(i);
        keysSettle.restart();
    }
    function pickStep(dir) {
        const vis = draft.filter(p => !p.hidden);
        if (!vis.length) return;
        flushKeys();
        const at = vis.findIndex(p => p.id === picked);
        picked = vis[(at + dir + vis.length) % vis.length].id;
    }
    FocusScope {
        id: editKeys
        focus: ab.editing
        Keys.onPressed: e => {
            if (!ab.editing) return;
            const ctrl = (e.modifiers & Qt.ControlModifier) !== 0, shift = (e.modifiers & Qt.ShiftModifier) !== 0;
            const mv = (dx, dy) => ab.nudge(i => { const d = L.copy(ab.draft); d[i] = L.moved(d[i], dx, dy, ab.cols, ab.rows); return d; });
            const sz = (e2, dx, dy) => ab.nudge(i => L.sized(ab.draft, ab.picked, e2, dx, dy, ab.cols, ab.rows));
            if (ab.menuEditor) { if (e.key === Qt.Key_Escape) { ab.menuEditor = false; e.accepted = true; } return; }
            if (ctrl && e.key === Qt.Key_Z) ab.undo();
            else if (e.key === Qt.Key_Escape || e.key === Qt.Key_Back) ab.done();
            else if ((ctrl && e.key === Qt.Key_PageDown) || (e.key === Qt.Key_Tab && !shift)) ab.pickStep(1);
            else if ((ctrl && e.key === Qt.Key_PageUp) || e.key === Qt.Key_Backtab || (e.key === Qt.Key_Tab && shift)) ab.pickStep(-1);
            else if (e.key === Qt.Key_PageDown) sz({ r: 1, b: 1 }, 1, 1);
            else if (e.key === Qt.Key_PageUp) sz({ r: 1, b: 1 }, -1, -1);
            else if (shift && e.key === Qt.Key_Left) sz({ r: 1 }, -1, 0);
            else if (shift && e.key === Qt.Key_Right) sz({ r: 1 }, 1, 0);
            else if (shift && e.key === Qt.Key_Up) sz({ b: 1 }, 0, -1);
            else if (shift && e.key === Qt.Key_Down) sz({ b: 1 }, 0, 1);
            else if (e.key === Qt.Key_Left) mv(-1, 0);
            else if (e.key === Qt.Key_Right) mv(1, 0);
            else if (e.key === Qt.Key_Up) mv(0, -1);
            else if (e.key === Qt.Key_Down) mv(0, 1);
            else if (e.key === Qt.Key_Menu || e.key === Qt.Key_Delete) { if (ab.picked !== "") ab.hide(ab.picked); }
            else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) ab.done();
            else return;
            e.accepted = true;
        }
    }

    // ---------------------------------------------------------------- keeping it (hearth-layout app-keep), Undo
    property var undoStack: []
    function savedCopy() { return savedPage === undefined ? null : L.copy(savedPage); }
    function pushUndo(u) { undoStack = undoStack.concat([u]).slice(-60); }
    function undo() {
        flushKeys();
        if (!undoStack.length) return toast("Nothing to undo.", false);
        const u = undoStack[undoStack.length - 1];
        undoStack = undoStack.slice(0, -1);
        if (u.zones) { keepChange({ zones: u.zones }); if (menuEditor) Qt.callLater(ab.loadMenu); }
        if (u.pages) { keepChange({ blocks: u.pages }); }
        if (u.page) {
            if (editing && u.page === pageId && u.draft) draft = u.draft;
            const b = {}; b[u.page] = u.before === null ? {} : u.before;
            keepChange({ blocks: b });
        }
        toast("Undone.", false);
    }
    function keepPage(pj) { keepFor(pageId, pj); }
    // another page of the app made of blocks (Settings' pages: AppBlocksFlow.qml), kept the same way
    property var extraPages: []
    function keepFor(pid, pj) { const b = {}; b[pid] = pj; keepChange({ blocks: b }); }
    // Back to the original: this page as the app has it (in Edit layout), or every Mantel page and menu of the app.
    function backToOriginal() {
        flushKeys();
        if (editing) {
            pushUndo({ page: pageId, before: savedCopy(), draft: L.copy(draft) });
            keepPage({});
            editing = false;
            reopen.restart();      // once the app's own layout has put everything back: Edit layout again, measured
            return toast(appName + " is back to the original. Undo is here.", true);
        }
        const b = layoutKept.blocks || {}, z = layoutKept.zones || {};
        const pagesBefore = {}, pagesNow = {}, zonesBefore = {}, zonesNow = {};
        for (const k in b) if (k.indexOf("mantel-") === 0) { pagesBefore[k] = L.copy(b[k]); pagesNow[k] = {}; }
        for (const k in z) if (k.indexOf("menu:") === 0) { zonesBefore[k] = L.copy(z[k]); zonesNow[k] = []; }
        pushUndo({ pages: pagesBefore, zones: Object.keys(zonesBefore).length ? zonesBefore : null });
        const ch = {};
        if (Object.keys(pagesNow).length) ch.blocks = pagesNow;
        if (Object.keys(zonesNow).length) ch.zones = zonesNow;
        if (Object.keys(ch).length) keepChange(ch);
        toast(appName + " is back to the original. Undo is here.", true);
    }
    Timer { id: reopen; interval: 120; onTriggered: ab.edit(ab.picked) }

    // What hearth-layout is told of the app's blocks: every page, the one shown where its parts are now.
    function catalogNow() {
        const list = pages.map(pg => ({ id: "mantel-" + pg.id, title: pg.title, blocks: pg.blocks }));
        const cur = list.find(p => p.id === pageId);
        if (cur) {
            const now = editing ? draft : res.mode === "grid" ? res.placed : measure();
            cur.blocks = defs.map((d, i) => Object.assign({}, d, now[i] ? { x: now[i].x, y: now[i].y, w: now[i].w, h: now[i].h } : {}));
        }
        return L.catalog(list.concat(extraPages.filter(x => !list.some(p => p.id === x.id))), cols, rows);
    }
    function describe() {
        flushKeys();
        menuShown = false;
        AppBlocksHub.describeAsked(app, appName, JSON.stringify(catalogNow()));
        describeRequested();
    }

    property var queue: []
    property string lastError: ""
    function keepChange(layoutChange) {
        if (unreadable) {
            toast("Your saved layout for " + appName + " couldn't be read, so nothing was changed.", false);
            return;
        }
        // at once here (what's shown follows your hand); the file is read again once hearth-layout has kept it
        const k = L.copy(kept) || {};
        k.layout = k.layout && typeof k.layout === "object" ? k.layout : {};
        for (const part of ["blocks", "zones"]) {
            if (!layoutChange[part]) continue;
            const cur = k.layout[part] && typeof k.layout[part] === "object" ? k.layout[part] : {};
            for (const n in layoutChange[part]) {
                const v = layoutChange[part][n];
                if ((part === "blocks" && v && Object.keys(v).length === 0) || v === null) delete cur[n];
                else cur[n] = v;
            }
            k.layout[part] = cur;
        }
        kept = k;
        queue = queue.concat([{ layout: layoutChange, features: [] }]);
        if (!keeper.running) keepNext();
    }
    function keepNext() {
        if (!queue.length) return;
        const c = queue[0];
        queue = queue.slice(1);
        keeper.out = ""; keeper.err = "";
        keeper.command = ["env", "HEARTH_DESCRIBE_APP=" + appName, "HEARTH_APP_BLOCKS=" + JSON.stringify(catalogNow()),
                          "hearth-layout", "app-keep", app, JSON.stringify(c)];
        keeper.running = true;
    }
    Process {
        id: keeper
        property string out: ""
        property string err: ""
        stdout: StdioCollector { onStreamFinished: keeper.out = text }
        stderr: StdioCollector { onStreamFinished: keeper.err = text }
        onExited: code => {
            if (code !== 0) {
                let why = (keeper.err || keeper.out || "").trim().split("\n").pop();
                try { const j = JSON.parse(why); if (j && j.error) why = j.error; } catch (e) {}
                if (code === 127 || /No such file|not found/.test(why)) why = "keeping it needs hearth-layout, which comes with Hearth OS";
                ab.lastError = why;
                ab.toast("That couldn't be kept: " + why, false);
                ab.queue = [];
            }
            fv.reload();
            ab.keepNext();
        }
    }

    // ---------------------------------------------------------------- menus put in order (zone "menu:<kind>")
    function menuOrder(kind) { const z = layoutKept.zones || {}; return z["menu:" + kind] || []; }
    function arrange(kind, items) { return L.arrangeMenu(items, menuOrder(kind)); }
    property bool menuEditor: false
    property string menuKind: ""
    property var menuIds: []
    property var menuLabels: ({})
    property var menuHidden: ({})
    function openMenus() { if (!menus.length) return; menuKind = menuKind || menus[0].id; menuEditor = true; loadMenu(); }
    function loadMenu() {
        const m = menus.find(x => x.id === menuKind);
        if (!m) return;
        const items = (m.items() || []).filter(i => !i.sep && i.id && String(i.id).indexOf("ab-") !== 0);
        const labels = {}, hid = {};
        for (const i of items) labels[i.id] = String(i.label || i.id).replace(/^[✓\s]+/, "");
        const order = menuOrder(menuKind);
        for (const o of order) if (String(o).charAt(0) === "-") { hid[o.slice(1)] = true; if (!labels[o.slice(1)]) labels[o.slice(1)] = o.slice(1); }
        const shownIds = L.arrangeMenu(items, order).filter(i => !i.sep).map(i => i.id);
        menuIds = shownIds.concat(Object.keys(hid).filter(k => shownIds.indexOf(k) < 0));
        menuLabels = labels; menuHidden = hid;
    }
    function menuChange(ids, hid) {
        const z = layoutKept.zones || {};
        const before = {}; before["menu:" + menuKind] = L.copy(z["menu:" + menuKind] || []);
        pushUndo({ zones: before });
        menuIds = ids; menuHidden = hid;
        const now = {}; now["menu:" + menuKind] = L.menuZone(ids, hid);
        keepChange({ zones: now });
    }
    function menuMove(id, d) {
        const ids = menuIds.slice(), i = ids.indexOf(id), j = i + d;
        if (i < 0 || j < 0 || j >= ids.length) return;
        ids[i] = ids[j]; ids[j] = id;
        menuChange(ids, menuHidden);
    }
    function menuToggle(id) { const h = Object.assign({}, menuHidden); h[id] = !h[id]; if (!h[id]) delete h[id]; menuChange(menuIds, h); }

    // ---------------------------------------------------------------- a part's own menu (right-click)
    property bool menuShown: false
    property string menuFor: ""
    property point menuAt: Qt.point(0, 0)
    function partMenu(id, x, y) {
        if (defs.length === 0) return;
        menuFor = id; menuAt = Qt.point(x, y); menuShown = true;
    }
    function runPartMenu(what) {
        menuShown = false;
        if (what === "edit") edit(menuFor);
        else if (what === "move") { edit(menuFor); toast("Drag " + titleOf(menuFor).toLowerCase() + " where you want it, or use the arrows.", false); }
        else if (what === "hide") hide(menuFor);
        else if (what === "describe") describe();
        else if (what === "reset") backToOriginal();
    }
    // The app's own menus' LAYOUT items run here (Files: View › Edit layout).
    function runMenu(id) {
        if (id === "ab-edit") edit("");
        else if (id === "ab-describe") describe();
        else if (id === "ab-reset") backToOriginal();
        else return false;
        return true;
    }
    function menuItems() {
        const out = [{ id: "ab-edit", label: "Edit layout", icon: "grid" }, { id: "ab-describe", label: "Describe it…", icon: "sparkle" }];
        if (customised) out.push({ id: "ab-reset", label: "Back to the original", icon: "undo" });
        return out;
    }
    MouseArea {
        anchors.fill: parent
        visible: ab.menuShown
        z: 50
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: ab.menuShown = false
    }
    Rectangle {
        id: pmenu
        visible: ab.menuShown
        z: 51
        width: 236
        height: pmCol.implicitHeight + 16
        x: Math.max(8, Math.min(ab.width - width - 8, ab.menuAt.x))
        y: Math.max(8, Math.min(ab.height - height - 8, ab.menuAt.y))
        radius: Theme.r(12); color: Theme.panel; border.color: Theme.line
        Column {
            id: pmCol
            x: 8; y: 8; width: parent.width - 16
            Text { width: parent.width; leftPadding: 10; topPadding: 4; bottomPadding: 6; text: ab.titleOf(ab.menuFor); elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
            Repeater {
                model: [["edit", "Edit layout"], ["move", "Move this"], ["hide", "Hide this"], ["describe", "Describe it…"]].concat(ab.customised ? [["reset", "Back to the original"]] : [])
                delegate: Rectangle {
                    required property var modelData
                    width: pmCol.width; height: 36; radius: Theme.r(8)
                    color: pmMouse.containsMouse ? Theme.accent : "transparent"
                    Text { x: 12; anchors.verticalCenter: parent.verticalCenter; text: parent.modelData[1]; color: pmMouse.containsMouse ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                    MouseArea { id: pmMouse; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: ab.runPartMenu(parent.modelData[0]) }
                }
            }
        }
    }

    // ---------------------------------------------------------------- the bar while editing (at the bottom, or the top
    // when the part you're on is down there), and the hidden parts' tray
    readonly property var pickedPlaced: editing ? (draft.find(p => p.id === picked) || null) : null
    readonly property bool barTop: !!pickedPlaced && pickedPlaced.y + pickedPlaced.h > rows * 0.72
    property string trayDrag: ""
    property point trayAt: Qt.point(0, 0)
    Rectangle {
        id: ebar
        visible: ab.editing
        z: 40
        width: Math.min(ab.width - 24, Math.max(560, ebFlow.implicitWidth + 28))
        height: ebCol.implicitHeight + 24
        anchors.horizontalCenter: parent.horizontalCenter
        y: ab.barTop ? 12 : ab.height - height - 12
        radius: Theme.r(14)
        color: Theme.panel
        border.color: Theme.accent; border.width: 2
        // (a click on the bar is the bar's, not the part under it)
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }
        Column {
            id: ebCol
            x: 14; y: 12; width: parent.width - 28
            spacing: 10
            Flow {
                id: ebFlow
                width: parent.width
                spacing: 8
                Rectangle { width: edT.implicitWidth + 18; height: 30; radius: 15; color: "transparent"; border.color: Theme.accent
                    Text { id: edT; anchors.centerIn: parent; text: "EDITING"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 1 } }
                Text { height: 30; verticalAlignment: Text.AlignVCenter; text: ab.appName + (ab.pages.length > 1 && ab.def ? " · " + ab.def.title : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                Item { width: 8; height: 30 }
                EBtn { label: "Describe it…"; icon: "sparkle"; onClicked: ab.describe() }
                EBtn { visible: ab.menus.length > 0; label: "Menus…"; icon: "menu"; lit: ab.menuEditor; onClicked: { if (ab.menuEditor) ab.menuEditor = false; else ab.openMenus(); } }
                EBtn { label: "Undo"; icon: "undo"; onClicked: ab.undo() }
                EBtn { label: "Back to the original"; onClicked: ab.backToOriginal() }
                EBtn { label: "Done"; primary: true; onClicked: ab.done() }
            }
            Text {
                width: parent.width; wrapMode: Text.WordWrap
                text: "Drag a part to move it (let go on another to swap them) · drag its edges or corners to size it · ✕ hides it · everything is kept as you go"
                    + " · controller: LB / RB pick, D-pad moves, LT / RT size, X hides, B done"
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
            }
            // the picked part's look
            Flow {
                width: parent.width
                spacing: 8
                visible: !!ab.pickedPlaced
                Text { height: 28; verticalAlignment: Text.AlignVCenter; text: ab.pickedPlaced ? ab.titleOf(ab.picked) + ":" : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                Repeater {
                    model: ["corners", "see", "accent"]
                    delegate: EBtn {
                        required property string modelData
                        small: true
                        label: ab.pickedPlaced ? L.lookLabel(ab.pickedPlaced, modelData) : ""
                        swatch: modelData === "accent" && ab.pickedPlaced && ab.pickedPlaced.accent ? ab.pickedPlaced.accent : ""
                        onClicked: ab.look(modelData)
                    }
                }
                EBtn { small: true; label: "Hide it"; onClicked: ab.hide(ab.picked) }
            }
            // the tray: hidden parts, to click or drag back
            Flow {
                width: parent.width
                spacing: 8
                visible: ab.editing && ab.draft.some(p => p.hidden)
                Text { height: 28; verticalAlignment: Text.AlignVCenter; text: "Hidden:"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                Repeater {
                    model: ab.editing ? ab.draft.filter(p => p.hidden) : []
                    delegate: Rectangle {
                        id: chip
                        required property var modelData
                        width: chT.implicitWidth + 22; height: 28; radius: 14
                        color: chM.containsMouse ? Theme.panel3 : Theme.bg2
                        border.color: Theme.line
                        Text { id: chT; anchors.centerIn: parent; text: "+ " + ab.titleOf(chip.modelData.id); color: Theme.text; font.family: Theme.font; font.pixelSize: 12 }
                        MouseArea {
                            id: chM
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.OpenHandCursor
                            property bool moved: false
                            onPressed: m => { moved = false; ab.trayDrag = chip.modelData.id; ab.trayAt = mapToItem(ab, m.x, m.y); }
                            onPositionChanged: m => { if (!pressed) return; moved = true; ab.trayAt = mapToItem(ab, m.x, m.y); }
                            onReleased: m => {
                                const q = mapToItem(ab, m.x, m.y), id = ab.trayDrag;
                                ab.trayDrag = "";
                                const onBar = q.y >= ebar.y && q.y <= ebar.y + ebar.height;
                                if (moved && !onBar) ab.show(id, q.x, q.y); else ab.show(id);
                            }
                        }
                    }
                }
            }
        }
    }
    // a hidden part being dragged back
    Rectangle {
        visible: ab.trayDrag !== ""
        z: 45
        readonly property var p: ab.draft.find(x => x.id === ab.trayDrag) || null
        width: p ? p.w * ab.cw : 0; height: p ? p.h * ab.ch : 0
        x: Math.floor(ab.trayAt.x / ab.cw) * ab.cw; y: Math.floor(ab.trayAt.y / ab.ch) * ab.ch
        color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.18); border.color: Theme.accent; border.width: 2; radius: 4
        Text { anchors.centerIn: parent; text: ab.titleOf(ab.trayDrag); color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
    }

    // ---------------------------------------------------------------- the menus' items, in order or left out
    Rectangle {
        visible: ab.editing && ab.menuEditor
        z: 42
        width: Math.min(460, ab.width - 40)
        height: Math.min(ab.height - ebar.height - 60, meCol.implicitHeight + 28)
        anchors.horizontalCenter: parent.horizontalCenter
        y: ab.barTop ? ebar.y + ebar.height + 12 : ebar.y - height - 12
        radius: Theme.r(14); color: Theme.panel; border.color: Theme.line
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }
        Flickable {
            anchors { fill: parent; margins: 14 }
            contentHeight: meCol.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            Column {
                id: meCol
                width: parent.width
                spacing: 6
                Row {
                    spacing: 8
                    Repeater {
                        model: ab.menus
                        delegate: EBtn { required property var modelData; small: true; label: modelData.title; lit: ab.menuKind === modelData.id; onClicked: { ab.menuKind = modelData.id; ab.loadMenu(); } }
                    }
                }
                Text { width: parent.width; wrapMode: Text.WordWrap; text: "Put the items in the order you want, or leave some out. Edit layout and Describe it always stay."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                Repeater {
                    model: ab.menuIds
                    delegate: Rectangle {
                        required property string modelData
                        required property int index
                        readonly property bool off: !!ab.menuHidden[modelData]
                        width: meCol.width; height: 36; radius: Theme.r(8)
                        color: Theme.bg2
                        Text { x: 12; width: parent.width - 170; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight; text: ab.menuLabels[parent.modelData] || parent.modelData; color: parent.off ? Theme.muted : Theme.text; font.strikeout: parent.off; font.family: Theme.font; font.pixelSize: 13 }
                        Row {
                            anchors { right: parent.right; rightMargin: 6; verticalCenter: parent.verticalCenter }
                            spacing: 4
                            EBtn { small: true; label: "↑"; onClicked: ab.menuMove(parent.parent.modelData, -1) }
                            EBtn { small: true; label: "↓"; onClicked: ab.menuMove(parent.parent.modelData, 1) }
                            EBtn { small: true; label: parent.parent.off ? "Show" : "Hide"; onClicked: ab.menuToggle(parent.parent.modelData) }
                        }
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------- the toast: what happened, with Undo
    property string toastText: ""
    property bool toastUndo: false
    function toast(t, withUndo) { toastText = t; toastUndo = !!withUndo && undoStack.length > 0; toastTimer.restart(); }
    Timer { id: toastTimer; interval: 6000; onTriggered: ab.toastText = "" }
    Rectangle {
        visible: ab.toastText !== ""
        z: 60
        anchors.horizontalCenter: parent.horizontalCenter
        y: ab.editing && !ab.barTop ? ebar.y - height - 10 : ab.height - height - 18
        width: Math.min(ab.width - 40, toastRow.implicitWidth + 32)
        height: 44; radius: 22
        color: Theme.panel3; border.color: Theme.line
        Row {
            id: toastRow
            anchors.centerIn: parent
            spacing: 14
            Text { anchors.verticalCenter: parent.verticalCenter; width: Math.min(implicitWidth, ab.width - 200); elide: Text.ElideRight; text: ab.toastText; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
            EBtn { visible: ab.toastUndo; small: true; label: "Undo"; onClicked: { ab.toastText = ""; ab.undo(); } }
        }
    }

    component EBtn: Rectangle {
        id: eb
        property string label
        property string icon: ""
        property string swatch: ""
        property bool primary: false
        property bool lit: false
        property bool small: false
        signal clicked()
        implicitWidth: ebRow.implicitWidth + (small ? 18 : 26)
        implicitHeight: small ? 28 : 34
        width: implicitWidth; height: implicitHeight
        radius: Theme.r(small ? 14 : 8)
        color: primary ? (ebM.containsMouse ? Qt.lighter(Theme.accent, 1.1) : Theme.accent) : (ebM.containsMouse || lit ? Theme.panel3 : Theme.bg2)
        border.color: lit ? Theme.accent : primary ? "transparent" : Theme.line
        Row {
            id: ebRow
            anchors.centerIn: parent
            spacing: 6
            Icon { visible: eb.icon !== ""; name: eb.icon; size: 15; color: eb.primary ? Theme.accentInk : Theme.text; anchors.verticalCenter: parent.verticalCenter }
            Rectangle { visible: eb.swatch !== ""; width: 12; height: 12; radius: 6; color: eb.swatch || "transparent"; anchors.verticalCenter: parent.verticalCenter }
            Text { text: eb.label; color: eb.primary ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: eb.small ? 12 : 13; font.weight: eb.primary ? Font.DemiBold : Font.Medium; anchors.verticalCenter: parent.verticalCenter }
        }
        MouseArea { id: ebM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: eb.clicked() }
    }
}

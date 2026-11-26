// Every Mantel app made of blocks (the owner, Nov 24: "Modular for all Hearth apps is required as well", and: editing
// an app's layout must feel natural, never buried; Nov 26, drag and drop board 3: "need to expand on it in every app",
// "need 100% customization", "i want to be able to create anything i can think of", "dont want them locked to just 4
// sides"). One shared piece, put over an app's window after its own content:
//
//   AppBlocks {
//       anchors.fill: parent; app: "files"; appName: "Hearth Files"; page: "window"
//       pages: [{ id: "window", title: "Hearth Files", blocks: [{ id: "sidebar", title: "The navigation pane", label: "Sidebar", x: 0, y: 6, w: 4, h: 18 }, …] }]
//       parts: ({ sidebar: navPane, … })     // the app's own items, by block id, for the page shown
//       focusBack: keys                       // what gets the keys back after Edit layout
//       actions: [{ id: "share", label: "Share", icon: "link", run: (x, y) => … }]   // its buttons, as blocks
//       panes: [{ id: "preview", label: "Preview", part: "preview" }, { id: "details", label: "Details", make: comp }]
//       counts: [{ id: "items", label: "Items here", value: () => n }]              // what a Count block can show
//       marks: ({ "Search": searchBox })      // things inside the parts that edges line up with
//   }
//
// Nothing customised: nothing here does anything; every part is where the app's own layout puts it, exactly as
// before (no item moved, no timer, no polling). What's kept is read from two files, each with one writer:
//   ~/.config/nimbus/apps/<app>.json      the parts' cells and what's hidden (the Rust apps' own file and format,
//                                         hosts/hearth-ui/src/blocks.rs; Mantel's pages are "mantel-<page>"), written
//                                         only by `hearth-layout app-keep` (settings_file: atomic, .bak, merged)
//   ~/.config/nimbus/app-looks/<app>.json each page's parts in pixels anywhere on the window, every part's and block's
//                                         own look (Customize), the blocks added from the app's Blocks window and the
//                                         order they're stacked in (app-blocks.js readExtras), written only here
//                                         through SettingsWriter (hearth_settings_io merge: atomic, .bak, merged)
// A file that can't be read is never written over (the app stays as it is and says so).
//
// The ways in, right where you are:
//   - right-click any part (the toolbar, a pane, the status bar…): "<part>": Edit layout · Customize… · Move this ·
//     Hide this · Describe it… · Back to the original
//   - the app's own menu (Files: View › LAYOUT, and the empty space's right-click menu): Edit layout, Describe it…
//   - the Edit layout button in the app's own bar (EditLayoutButton.qml)
// Edit layout (drag and drop board 3; AppBlocksEditor.qml, loaded only while editing): the window marked EDITING,
// every part outlined (dashed, accent) with its name and size; the app's own Blocks window to drag its buttons, panes,
// counts and your own blocks from, into any bar of the app (a lit line where it lands) or anywhere on the window;
// parts dragged anywhere (not only to its sides) and sized by their edges (a guide and a note); right-click any part ›
// Customize (AppBlockCustomize.qml: board 2's side panel); the toolbar: Undo, Redo, Snap to grid · 8 px, Guides,
// Layers, Start from, Menus…, Cancel, Try it, Keep this. Nothing is written until Keep this; Try it puts it in place
// with a way back. Keys: Tab picks a part, the arrows move it, Shift + the arrows size it, Delete hides it, Ctrl+Z /
// Ctrl+Shift+Z undo and redo, Enter keeps, Esc cancels. Describe it (your AI) stays in the part's menu.
import QtQuick
import QtQuick.Layouts
import QtQuick.Window
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
    // Board 3: the app's own blocks (its Blocks window): its actions, its panes, what a count can show, its marks
    property var actions: []
    property var panes: []
    property var counts: []
    property var marks: ({})
    // false: everything as the app has it (while the app needs its whole window for something else)
    property bool active: true
    signal describeRequested()

    readonly property int cols: L.COLS
    readonly property int rows: L.ROWS
    readonly property int maxBlocks: 16
    readonly property int maxPanes: 8
    readonly property string pageId: "mantel-" + page
    readonly property var def: { const ps = pages || []; for (let i = 0; i < ps.length; i++) if (ps[i].id === page) return ps[i]; return null; }
    readonly property var defs: def ? def.blocks.slice(0, maxBlocks) : []
    readonly property string shortAppName: appName.replace(/^Hearth /, "")

    // ---------------------------------------------------------------- what's kept
    readonly property string cfg: Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")
    readonly property string file: cfg + "/nimbus/apps/" + app + ".json"
    readonly property string looksFile: cfg + "/nimbus/app-looks/" + app + ".json"
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
    property var looksKept: ({})
    property bool looksUnreadable: false
    FileView {
        id: lv
        path: ab.app !== "" ? ab.looksFile : ""
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: ab.readLooks(text())
        onLoadFailed: { if (!looksWriter.busy) { ab.looksKept = {}; ab.looksUnreadable = false; } }
    }
    // the one writer of app-looks/<app>.json: only the page you kept, merged into the latest file (atomic, .bak)
    SettingsWriter { id: looksWriter; path: ab.app !== "" ? ab.looksFile : "" }
    Connections { target: AppBlocksHub; function onChanged(a) { if (a === ab.app) { fv.reload(); lv.reload(); } } }
    // read again each time the window shows (a file made while it was closed isn't watched yet: an app kept warm, Files)
    readonly property bool windowShown: ab.Window.window ? ab.Window.window.visible : false
    onWindowShownChanged: if (windowShown && app !== "") { fv.reload(); lv.reload(); }
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
    function readLooks(t) {
        if (String(t).trim() === "") { looksKept = {}; looksUnreadable = false; return; }
        let o = null;
        try { o = JSON.parse(t); } catch (e) { o = null; }
        if (!o || typeof o !== "object" || Array.isArray(o)) {
            console.warn("AppBlocks: " + looksFile + " couldn't be read; " + app + "'s parts keep their own look");
            if (!looksWriter.owns(pageId)) looksKept = {};
            looksUnreadable = true; return;
        }
        // a page still on its way to the file keeps what you just kept (a reload of an older write mustn't put it back)
        const n = Object.assign({}, o);
        for (const k in looksKept) if (looksWriter.owns(k)) n[k] = looksKept[k];
        looksKept = n; looksUnreadable = false;
    }
    readonly property var layoutKept: kept && kept.layout && typeof kept.layout === "object" ? kept.layout : ({})
    readonly property var savedPage: layoutKept.blocks && typeof layoutKept.blocks === "object" ? layoutKept.blocks[pageId] : undefined
    readonly property var res: L.resolve(defs, savedPage, layoutKept, cols, rows)
    readonly property var pageExtras: L.readExtras(looksKept[pageId], defs.map(d => d.id))
    readonly property bool hasExtras: !L.extrasEmpty(pageExtras)
    readonly property bool customised: {
        const b = layoutKept.blocks || {}, z = layoutKept.zones || {};
        for (const k in b) if (k.indexOf("mantel-") === 0 && b[k] && Object.keys(b[k]).length) return true;
        for (const k in z) if (k.indexOf("menu:") === 0 && Array.isArray(z[k]) && z[k].length) return true;
        for (const k in looksKept) if (k.indexOf("mantel-") === 0 && looksKept[k]) return true;
        return res.mode !== "own";
    }

    // ---------------------------------------------------------------- where the parts go
    property bool editing: false
    property bool trying: false            // Try it: what you made is in place, with Go back
    // the page while editing: { parts: [{id, title, x, y, w, h (px), hidden, …}], added: [block], looks: {id: look},
    // z: [ids back to front], zones: {"menu:<kind>": [...]}, mine: [your own blocks] }
    property var draft: ({ parts: [], added: [], looks: ({}), z: [], zones: ({}), mine: [] })
    property var start: null               // the draft as Edit layout opened it
    property string startMode: "own"
    property string picked: ""
    property var shownMap: ({})            // block id → is the part showing anything right now (the details pane open)
    property var panelShown: ({})          // a part shown or hidden by a block's "Show a panel" (this time only)
    readonly property string mode: !active || defs.length === 0 ? "own" : editing || hasExtras ? "grid" : res.mode
    readonly property var restParts: {
        const W = width, H = height, rs = pageExtras.rects;
        return defs.map((d, i) => {
            const p = res.placed[i] || L.own(d);
            const r = rs[d.id] ? L.fromFrac(rs[d.id], W, H) : L.cellPx(p, W, H, cols, rows);
            return Object.assign({}, p, r, { title: d.title });
        });
    }
    readonly property var placed: editing ? draft.parts : restParts
    readonly property var rects: {
        if (mode !== "grid") return [];
        if (editing) return placed;
        const sh = Object.assign({}, shownMap);
        for (const k in panelShown) sh[k] = panelShown[k];
        const pl = placed.map(p => Object.assign({}, p, { hidden: panelShown[p.id] === true ? false : p.hidden }));
        return L.reflowPx(pl, sh, 2);
    }
    readonly property var effAdded: editing ? draft.added : pageExtras.added
    readonly property var effLooks: editing ? draft.looks : pageExtras.looks
    readonly property var effZ: editing ? draft.z : pageExtras.z
    readonly property var effPanes: effAdded.filter(b => b.kind === "pane").slice(0, maxPanes)
    readonly property var effFree: effAdded.filter(b => b.kind !== "pane" && !b.in)
    readonly property var effBarBlocks: mode === "grid" || editing ? effAdded.filter(b => b.kind !== "pane" && !!b.in) : []
    // A part's look: what Customize gave it, over what Describe it gave it (corners, see-through, colour)
    function lookOf(id) {
        const l = Object.assign({}, effLooks[id] || {});
        const p = placed.find(x => x.id === id);
        if (p) {
            if (l.corners === undefined && p.corners !== null && p.corners !== undefined) l.corners = p.corners;
            if (l.see === undefined && p.seeThrough) l.see = p.seeThrough;
            if (l.edge === undefined && p.accent) { l.edge = "line"; l.edgeColor = p.accent; l.edgeW = 2; }
            if (l.fill === undefined && !L.plainLook(p)) l.cardFill = true;
        }
        return l;
    }
    function layerOf(id) { const i = effZ.indexOf(id); return i < 0 ? 0 : i + 1; }
    function titleOf(id) {
        const d = defs.find(x => x.id === id); if (d) return d.title;
        const b = effAdded.find(x => x.id === id) || (editing ? (draft.mine || []).find(x => x.id === id) : null);
        return b ? blockName(b) : id;
    }
    function labelOf(id) { const d = defs.find(x => x.id === id); return d && d.label ? d.label : L.shortName(titleOf(id)); }
    function actionOf(ref) { return (actions || []).find(a => a.id === ref) || null; }
    function paneOf(ref) { return (panes || []).find(p => p.id === ref) || null; }
    function countOf(ref) { return (counts || []).find(c => c.id === ref) || null; }
    function countValue(ref) { const c = countOf(ref); if (!c || !c.value) return ""; try { const v = c.value(); return v === undefined || v === null ? "" : v; } catch (e) { return ""; } }
    function blockName(b) {
        if (!b) return "";
        if (b.label) return b.label;
        if (b.kind === "action") { const a = actionOf(b.ref); return a ? a.label : "A button"; }
        if (b.kind === "pane") { const p = paneOf(b.ref); return p ? p.label : "A pane"; }
        if (b.kind === "count") { const c = countOf(b.ref); return c ? c.label : "A count"; }
        if (b.kind === "words") return "Words";
        if (b.kind === "spacer") return "Spacer";
        return "Your button";
    }
    // What a block or a part does when it's pressed, right-clicked, middle-clicked, scrolled or pointed at
    function runDoes(d, x, y) {
        if (!d || !d.k || d.k === "nothing") return;
        if (d.k === "action") { runAction(d.a, x, y); return; }
        if (d.k === "run" && d.a) { Quickshell.execDetached(["sh", "-c", d.a]); return; }
        if (d.k === "open" && d.a) { Quickshell.execDetached(["xdg-open", d.a]); return; }
        if (d.k === "panel" && d.a) {
            const p = placed.find(q => q.id === d.a);
            if (!p) return;
            const now = panelShown[d.a] !== undefined ? panelShown[d.a] : !p.hidden;
            const m = Object.assign({}, panelShown); m[d.a] = !now; panelShown = m;
        }
    }
    function runAction(ref, x, y) {
        const a = actionOf(ref);
        if (!a || !a.run) return toast("That button's action isn't in " + appName + " any more.", false);
        try { a.run(x, y); } catch (e) { console.warn("AppBlocks: " + app + "'s " + ref + ": " + e); }
    }

    onModeChanged: schedule()
    onPlacedChanged: schedule()
    onPartsChanged: schedule()
    onEffBarBlocksChanged: schedule()
    onPanelShownChanged: schedule()
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
    // Parts going back to their own place, in their own order. (QML can't put an item before another one: setting a
    // parent adds it last, so the siblings after its place are taken off and put back after it, in the order they had.)
    function goHome(list) {
        const byParent = new Map();
        for (const h of list) {
            if (!alive(h.item) || h.at === "home" || !alive(h.parent)) continue;
            if (!byParent.has(h.parent)) byParent.set(h.parent, []);
            byParent.get(h.parent).push(h);
        }
        for (const [par, hs] of byParent) {
            hs.sort((a, b) => a.index - b.index);
            const desired = [];
            for (let i = 0; i < par.children.length; i++) desired.push(par.children[i]);
            for (const h of hs) {
                const it = h.item;
                it.width = h.w; it.height = h.h;          // the holder's size let go of (its own layout sizes it again)
                const at = h.anchor && alive(h.anchor) ? desired.indexOf(h.anchor) : -1;
                if (at < 0) desired.push(it); else desired.splice(at, 0, it);
                h.at = "home";
            }
            reorder(par, desired);
        }
    }
    function reorder(par, desired) {
        let k = 0;
        while (k < desired.length && k < par.children.length && par.children[k] === desired[k]) k++;
        for (let i = k; i < desired.length; i++) if (desired[i].parent === par) desired[i].parent = park;
        for (let i = k; i < desired.length; i++) desired[i].parent = par;
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
            const hid = p && p.hidden && (editing || panelShown[defs[i].id] !== true);
            want.set(it, mode === "own" ? "home" : hid ? "park" : mode === "natural" ? "home" : "h" + i);
        }
        goHome(homes.filter(h => (want.get(h.item) || "home") === "home" && h.at !== "home"));
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
        applyBlocks();
    }
    function noteShown(id, on) {
        if (id === "" || shownMap[id] === on) return;
        const m = Object.assign({}, shownMap); m[id] = on; shownMap = m;
    }

    // ---------------------------------------------------------------- blocks in the app's own bars
    // A block dragged into a bar goes in among the bar's own items (its row: the widest Row or RowLayout in it), at
    // the place it was let go; a bar without a row of its own gets them at its end. The app's own items keep their
    // order (only the ones after a block are taken off and put back); nothing at all happens while none were added.
    property var blockItems: ({})          // block id → its AppBlock
    property Component blockComp: null
    function isRepeater(x) { return !!x && x.model !== undefined && x.delegate !== undefined && x.count !== undefined && x.itemAt !== undefined; }
    function rowOf(it) {
        if (!alive(it)) return null;
        let best = null;
        const visit = (x, d) => {
            if (!x || d > 4) return;
            if (x !== it && x.spacing !== undefined && x.layoutDirection !== undefined && x.flow === undefined && x.columns === undefined
                && x.width >= x.height && x.children && x.children.length > 1 && !isMine(x)) {
                if (!best || x.width > best.width) best = x;
            }
            const k = x.children || [];
            for (let i = 0; i < k.length; i++) visit(k[i], d + 1);
        };
        visit(it, 0);
        return best;
    }
    function isMine(x) { for (const k in blockItems) if (blockItems[k] === x) return true; return false; }
    function ownKids(row) { const o = []; if (!row) return o; for (let i = 0; i < row.children.length; i++) { const c = row.children[i]; if (!isMine(c) && !isRepeater(c)) o.push(c); } return o; }
    function applyBlocks() {
        const want = effBarBlocks.concat(effFree);
        if (!want.length && Object.keys(blockItems).length === 0) return;
        if (!blockComp) {
            blockComp = Qt.createComponent("AppBlock.qml");
            if (blockComp.status === Component.Error) { console.warn("AppBlocks: AppBlock.qml couldn't load: " + blockComp.errorString()); return; }
        }
        if (blockComp.status !== Component.Ready) return;
        const items = Object.assign({}, blockItems);
        // taken out of the bars they were in first (in their own order, nothing of the app's moves)
        for (const id in items) {
            const it = items[id];
            if (!want.some(b => b.id === id)) { it.parent = park; it.destroy(); delete items[id]; }
        }
        for (const b of want) {
            // (one moved between a bar and the window is made again: a bar's layout sizes and places what's in it)
            if (items[b.id] && !!items[b.id].block.in !== !!b.in) { items[b.id].parent = park; items[b.id].destroy(); delete items[b.id]; }
            if (!items[b.id]) { const o = blockComp.createObject(park, { ab: ab, block: b }); if (o) items[b.id] = o; }
            else items[b.id].block = b;
        }
        blockItems = items;
        // into the bars
        const byBar = {};
        for (const b of effBarBlocks) (byBar[b.in] = byBar[b.in] || []).push(b);
        const placedHere = {};
        for (const bar in byBar) {
            const part = parts[bar];
            const hd = holderOf(bar);
            const row = rowOf(part);
            const list = byBar[bar].filter(b => !!items[b.id]);
            if (!row) {
                // no row of its own: at its end, on the bar
                if (hd) for (const b of list) { items[b.id].parent = hd.strip; placedHere[b.id] = true; }
                continue;
            }
            const own = ownKids(row);
            const order = L.barOrder(own.length, list.map(b => ({ id: b.id, at: b.at || 0 })));
            const desired = order.map(x => typeof x === "number" ? own[x] : items[x]);
            const kids = [];
            for (let i = 0; i < row.children.length; i++) if (!isRepeater(row.children[i])) kids.push(row.children[i]);
            let k = 0;
            while (k < desired.length && k < kids.length && kids[k] === desired[k]) k++;
            for (let i = k; i < desired.length; i++) if (desired[i].parent === row) desired[i].parent = park;
            for (let i = k; i < desired.length; i++) desired[i].parent = row;
            for (const b of list) placedHere[b.id] = true;
        }
        // the ones on the window, anywhere
        for (const b of effFree) if (items[b.id]) { items[b.id].parent = freeLayer; placedHere[b.id] = true; }
        for (const id in items) if (!placedHere[id] && items[id].parent !== park) items[id].parent = park;
    }
    function holderOf(id) { const i = defs.findIndex(d => d.id === id); return i >= 0 ? holders.itemAt(i) : null; }

    // A right-click on a part that doesn't take it itself: that part's own menu (Edit layout, Customize, Hide this…).
    Component {
        id: catcherComp
        MouseArea {
            property string bid: ""
            anchors.fill: parent
            z: -1000
            acceptedButtons: Qt.RightButton
            onClicked: m => {
                const l = ab.editing ? null : ab.lookOf(bid);
                if (l && l.does && l.does.right && l.does.right.k) return;        // its own right-click (Customize › Does)
                const q = mapToItem(ab, m.x, m.y); ab.partMenu(bid, q.x, q.y);
            }
        }
    }
    function addCatcher(it, id) {
        if (it.spacing !== undefined) return;     // a layout or a row lays out its children: it gets none
        for (const c of catchers) if (c.item === it) return;
        const m = catcherComp.createObject(it, { bid: id });
        if (m) catchers.push({ item: it, mouse: m });
    }

    // ---------------------------------------------------------------- the page laid out on the window
    Item {
    id: canvas
    anchors.fill: parent
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
        model: ab.maxBlocks + ab.maxPanes
        delegate: Item {
            id: hd
            required property int index
            readonly property bool isPane: index >= ab.maxBlocks
            readonly property var pb: isPane ? (ab.effPanes[index - ab.maxBlocks] || null) : null
            readonly property var d: isPane ? null : index < ab.defs.length ? ab.defs[index] : null
            readonly property var p: isPane ? pb : d && ab.mode === "grid" ? ab.placed[index] : null
            readonly property var r0: isPane ? (pb ? L.fromFrac(pb, ab.width, ab.height) : null) : p ? ab.rects[index] : null
            readonly property string bid: isPane ? (pb ? pb.id : "") : d ? d.id : ""
            readonly property var look: { void ab.effLooks; void ab.placed; return bid ? ab.lookOf(bid) : ({}); }
            // a part's size from its look (Fixed, Fit, Fill)
            readonly property var r: {
                if (!r0) return null;
                if (!look.wMode && !look.hMode) return r0;
                const others = ab.rects.filter((q, i) => i !== index && ab.placed[i] && !ab.placed[i].hidden);
                const need = part ? { w: part.implicitWidth || 0, h: part.implicitHeight || 0 } : null;
                return L.sizedByLook(r0, look, need, others, ab.width, ab.height);
            }
            property Item part: null
            readonly property bool showing: isPane ? true : part ? part.visible : true
            onShowingChanged: if (part) { ab.noteShown(bid, showing); if (showing && !ab.editing) { appear(); if (look.bounce) bounceAnim.restart(); } }
            onPartChanged: if (part) ab.noteShown(bid, part.visible)
            readonly property bool plain: isPane ? false : (!p || L.plainLook(p)) && L.lookEmpty(ab.effLooks[bid] || {})
            readonly property bool hovered: hov.hovered
            readonly property bool wanted: !!p && !!r && (ab.editing ? !p.hidden : ab.panelShown[bid] === true || (!p.hidden && ab.panelShown[bid] !== false))
            readonly property bool whenOk: ab.editing || L.showsNow(look, hovered, showing, ab.page)
            // (only when pointed at: there, see-through, until the pointer comes)
            visible: wanted && (whenOk || look.when === "hover")
            opacity: whenOk ? (look.appear === "fade" || look.appear === "pop" ? appearT : 1) : 0.001
            x: r ? r.x : 0
            y: r ? r.y : 0
            width: r ? r.w : 0
            height: r ? r.h : 0
            z: 1 + ab.layerOf(bid)
            property alias slot: slotItem
            property alias strip: stripRow
            // Motion: how it appears, and a bounce when something new comes
            property real appearT: 1
            property real bounceT: 0
            function appear() { if (look.appear && look.appear !== "none") { appearT = 0; appearAnim.restart(); } }
            NumberAnimation { id: appearAnim; target: hd; property: "appearT"; from: 0; to: 1; duration: 260; easing.type: Easing.OutCubic }
            SequentialAnimation {
                id: bounceAnim
                NumberAnimation { target: hd; property: "bounceT"; from: 0; to: 1; duration: 120; easing.type: Easing.OutQuad }
                NumberAnimation { target: hd; property: "bounceT"; from: 1; to: 0; duration: 260; easing.type: Easing.OutBounce }
            }
            onVisibleChanged: if (visible && !ab.editing) appear()
            transform: [
                Translate { y: (hd.look.appear === "slide" ? (1 - hd.appearT) * 18 : 0) - hd.bounceT * 6 },
                Scale { origin.x: hd.width / 2; origin.y: hd.height / 2; xScale: hd.look.appear === "pop" ? 0.92 + 0.08 * hd.appearT : 1; yScale: xScale }
            ]
            // the space around it (its margins on the window)
            readonly property var m: look.m || [0, 0, 0, 0]
            Item {
                id: box
                x: hd.m[3]; y: hd.m[0]
                width: Math.max(1, hd.width - hd.m[1] - hd.m[3]); height: Math.max(1, hd.height - hd.m[0] - hd.m[2])
                readonly property real rad: hd.look.corners !== undefined ? hd.look.corners : 0
                // Shadow (Soft, Deep)
                Repeater {
                    model: hd.look.shadow === "deep" ? 3 : hd.look.shadow === "soft" ? 2 : 0
                    delegate: Rectangle {
                        required property int index
                        anchors { fill: parent; margins: -(index + 1) * (hd.look.shadow === "deep" ? 4 : 2); topMargin: -(index + 1) * 2 + (index + 1) * (hd.look.shadow === "deep" ? 6 : 3) }
                        radius: box.rad + (index + 1) * 3
                        color: Qt.rgba(0, 0, 0, (hd.look.shadow === "deep" ? 0.22 : 0.16) / (index + 1))
                    }
                }
                // Edge: Glow
                Repeater {
                    model: hd.look.edge === "glow" ? 3 : 0
                    delegate: Rectangle {
                        required property int index
                        anchors { fill: parent; margins: -(index + 1) * 3 }
                        radius: box.rad + (index + 1) * 3
                        color: "transparent"
                        border.width: 3
                        border.color: { const c = hd.look.edgeColor ? Qt.color(hd.look.edgeColor) : Theme.accent; return Qt.rgba(c.r, c.g, c.b, 0.32 / (index + 1)); }
                    }
                }
                // Blur behind it: what's under it, drawn small and stretched back (Frost.qml's way), only when asked
                Loader {
                    anchors.fill: parent
                    active: !!hd.look.blur && hd.wanted
                    sourceComponent: Component {
                        ShaderEffectSource {
                            sourceItem: canvas
                            sourceRect: Qt.rect(hd.x + box.x, hd.y + box.y, box.width, box.height)
                            recursive: true
                            live: true
                            smooth: true
                            textureSize: Qt.size(Math.max(4, Math.round(box.width / 18)), Math.max(4, Math.round(box.height / 18)))
                        }
                    }
                }
                // the card: its own background, or the look it was given
                Rectangle {
                    id: card
                    anchors.fill: parent
                    readonly property color base: hd.look.fill ? Qt.color(hd.look.fill)
                        : hd.d && hd.d.bg !== undefined ? hd.d.bg : (hd.isPane || hd.look.cardFill || hd.look.see !== undefined || hd.look.edge) ? Theme.panel : "transparent"
                    visible: !hd.plain || (hd.d && hd.d.bg !== undefined) || hd.isPane
                    color: base.a === 0 ? "transparent" : Qt.rgba(base.r, base.g, base.b, base.a * (1 - (hd.look.see || 0) / 100))
                    radius: box.rad
                    border.width: hd.look.edge === "line" || hd.look.edge === "glow" ? (hd.look.edgeW || 1) : (hd.isPane ? 1 : 0)
                    border.color: hd.look.edgeColor ? hd.look.edgeColor : hd.look.edge ? Theme.accent : Theme.line
                }
                // Icon and words: the part's own heading
                readonly property bool head: (hd.look.show === "icon" && !!hd.look.icon) || (hd.look.show === "words" && !!hd.look.words) || (hd.look.show === "both" && (!!hd.look.icon || !!hd.look.words))
                Row {
                    id: headRow
                    visible: box.head
                    x: (hd.look.pad || 10); y: (hd.look.pad || 8)
                    spacing: 8
                    Icon {
                        visible: hd.look.show !== "words" && !!hd.look.icon
                        name: hd.look.icon || "dot"; size: hd.look.iconSize || 16
                        color: hd.look.iconColor || Theme.text
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Text {
                        visible: hd.look.show !== "icon" && !!hd.look.words
                        text: L.fillWords(hd.look.words, ab.wordValues)
                        color: hd.look.textColor || Theme.text
                        font.family: hd.look.font || Theme.font
                        font.pixelSize: hd.look.fontSize || 13
                        font.weight: hd.look.weight === "bold" ? Font.Bold : hd.look.weight === "light" ? Font.Light : hd.look.weight === "medium" ? Font.Medium : Font.DemiBold
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }
                Item {
                    id: slotItem
                    readonly property real pad: hd.look.pad !== undefined ? hd.look.pad : hd.plain ? 0 : Math.max(2, box.rad * 0.35)
                    anchors { fill: parent; margins: pad; topMargin: pad + (box.head ? headRow.height + 8 : 0) }
                    clip: true
                    // an added pane (Files' Details): the app's own piece for it
                    Loader {
                        anchors.fill: parent
                        active: hd.isPane && !!hd.pb
                        sourceComponent: { const pn = hd.pb ? ab.paneOf(hd.pb.ref) : null; return pn && pn.make ? pn.make : missingPane; }
                    }
                }
                // a bar without a row of its own: the blocks added to it, at its end
                Row {
                    id: stripRow
                    anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter }
                    spacing: 6
                    z: 5
                }
                // When it's pointed at · pressed: its hover and pressed look
                Rectangle {
                    anchors.fill: parent
                    z: 6
                    radius: box.rad
                    visible: !ab.editing && color.a > 0
                    color: {
                        const l = hd.look;
                        if (press.active) {
                            if (l.pressed === "darker") return Qt.rgba(0, 0, 0, 0.18);
                            if (l.pressed === "own" && l.pressedColor) { const c = Qt.color(l.pressedColor); return Qt.rgba(c.r, c.g, c.b, 0.30); }
                        }
                        if (hov.hovered) {
                            if (l.hover === "lighter") return Qt.rgba(1, 1, 1, 0.06);
                            if (l.hover === "accent") return Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.14);
                            if (l.hover === "own" && l.hoverColor) { const c = Qt.color(l.hoverColor); return Qt.rgba(c.r, c.g, c.b, 0.25); }
                        }
                        return "transparent";
                    }
                }
            }
            // what it does (Customize › Does): over the part, passive, so the app's own clicks still reach it
            Item {
                anchors.fill: parent
                z: 8
                enabled: !ab.editing
                HoverHandler { id: hov; onHoveredChanged: if (hovered && hd.look.does && hd.look.does.hover) ab.runDoes(hd.look.does.hover, point.position.x + hd.x, point.position.y + hd.y) }
                PointHandler { id: press; acceptedButtons: Qt.LeftButton }
                TapHandler {
                    enabled: !!(hd.look.does && hd.look.does.press && hd.look.does.press.k)
                    acceptedButtons: Qt.LeftButton
                    gesturePolicy: TapHandler.DragThreshold
                    onTapped: (ev) => ab.runDoes(hd.look.does.press, hd.x + ev.position.x, hd.y + ev.position.y)
                }
                TapHandler {
                    enabled: !!(hd.look.does && hd.look.does.right && hd.look.does.right.k)
                    acceptedButtons: Qt.RightButton
                    gesturePolicy: TapHandler.DragThreshold
                    onTapped: (ev) => ab.runDoes(hd.look.does.right, hd.x + ev.position.x, hd.y + ev.position.y)
                }
                TapHandler {
                    enabled: !!(hd.look.does && hd.look.does.middle && hd.look.does.middle.k)
                    acceptedButtons: Qt.MiddleButton
                    gesturePolicy: TapHandler.DragThreshold
                    onTapped: (ev) => ab.runDoes(hd.look.does.middle, hd.x + ev.position.x, hd.y + ev.position.y)
                }
                WheelHandler {
                    enabled: !!(hd.look.does && hd.look.does.scroll && hd.look.does.scroll.k)
                    onWheel: (ev) => ab.runDoes(hd.look.does.scroll, hd.x, hd.y)
                }
            }
        }
    }
    }
    Component { id: missingPane; Text { text: "This pane isn't in " + ab.appName + " any more."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap } }
    // what the words of a part or a block can show ({count} and the app's counts by id)
    readonly property var wordValues: {
        const o = {};
        for (const c of (counts || [])) { try { o[c.id] = c.value ? c.value() : ""; } catch (e) { o[c.id] = ""; } }
        if (counts && counts.length) o.count = o[counts[0].id];
        o.app = appName;
        return o;
    }
    // the blocks put anywhere on the window
    Item { id: freeLayer; anchors.fill: parent; z: 20 }

    // ---------------------------------------------------------------- Edit layout (drag and drop, board 3)
    // Where the parts are now, in px on the window (the ones not showing get the side of the parts they open beside).
    function measurePx() {
        const W = width, H = height;
        if (res.mode === "grid" || hasExtras) return restParts.map(p => Object.assign({}, p));
        const out = [], shown = {};
        for (let i = 0; i < defs.length; i++) {
            const d = defs[i], it = parts[d.id];
            const p = Object.assign(L.own(d), { title: d.title });
            const r = res.placed[i];
            p.hidden = !!(r && r.hidden);
            const h = alive(it) ? homeOf(it) : null;
            const home = alive(it) && (!h || h.at === "home");
            if (!p.hidden && home && it.visible && it.width > 1 && it.height > 1) {
                const q = it.mapToItem(ab, 0, 0);
                p.x = Math.round(Math.max(0, q.x)); p.y = Math.round(Math.max(0, q.y));
                p.w = Math.round(Math.min(W - p.x, it.width)); p.h = Math.round(Math.min(H - p.y, it.height));
                shown[d.id] = true;
            } else {
                Object.assign(p, L.cellPx(p, W, H, cols, rows));
                shown[d.id] = false;
            }
            out.push(p);
        }
        const scaled = defs.map(d => Object.assign({}, d, { size: (d.size || 5) * W / cols }));
        const placedBeside = L.besides(out, scaled, shown);
        const live = placedBeside.filter((p, i) => !p.hidden && (shown[p.id] || defs[i].beside));
        const carved = L.carve(live);
        return placedBeside.map(p => carved.find(c => c.id === p.id) || p).map(p => Object.assign({}, p, { x: Math.round(p.x), y: Math.round(p.y), w: Math.max(1, Math.round(p.w)), h: Math.max(1, Math.round(p.h)) }));
    }
    function kindOf(id) { const p = placed.find(x => x.id === id); return p ? L.partKind(p, width, height) : ""; }
    readonly property var barIds: { void placed; return defs.map(d => d.id).filter(id => kindOf(id) === "bar"); }
    property var undoDraft: []
    property var redoDraft: []
    property var before: null              // what was kept when Edit layout opened (Try it's Go back)
    property bool tried: false
    property string customizeFor: ""       // the part or block the side panel is on
    property string customizeTab: "look"
    property var lookClip: null            // Copy its look
    function snapshot() {
        return { parts: restParts.length ? measurePx() : [], added: L.copy(pageExtras.added) || [], looks: L.copy(pageExtras.looks) || {},
                 z: (pageExtras.z || []).slice(), zones: {}, mine: L.readMine(looksKept[pageId]) };
    }
    function edit(pick) {
        if (defs.length === 0) return;
        menuShown = false; menuEditor = false;
        if (!editing) {
            startMode = res.mode;
            const s = snapshot();
            start = L.copy(s);
            draft = s;
            undoDraft = []; redoDraft = [];
            if (!trying) { before = { page: savedCopy(), extras: looksKept[pageId] === undefined ? null : L.copy(looksKept[pageId]), zones: L.copy(layoutKept.zones || {}) }; tried = false; }
            editing = true;            // (first, so the editor stays loaded from trying to editing)
            trying = false;
        }
        picked = pick || (draft.parts.find(p => !p.hidden) || {}).id || "";
        editKeys.forceActiveFocus();
    }
    function focusEditKeys() { editKeys.forceActiveFocus(); }
    function customize(id, tab) { if (!editing) edit(id); picked = id; customizeTab = tab || "look"; customizeFor = id; }
    // One change while editing: on the draft only (Undo, Redo); nothing is written until Keep this.
    function change(d) {
        undoDraft = undoDraft.concat([JSON.stringify(draft)]).slice(-200);
        redoDraft = [];
        draft = d;
        lastLook = "";
    }
    function undo() {
        if (!editing) return restUndo();
        if (!undoDraft.length) return toast("Nothing to undo.", false);
        redoDraft = redoDraft.concat([JSON.stringify(draft)]);
        draft = JSON.parse(undoDraft[undoDraft.length - 1]);
        undoDraft = undoDraft.slice(0, -1);
        lastLook = "";
        if (menuEditor) Qt.callLater(ab.loadMenu);
    }
    function redo() {
        if (!editing || !redoDraft.length) return;
        undoDraft = undoDraft.concat([JSON.stringify(draft)]);
        draft = JSON.parse(redoDraft[redoDraft.length - 1]);
        redoDraft = redoDraft.slice(0, -1);
        lastLook = "";
        if (menuEditor) Qt.callLater(ab.loadMenu);
    }
    function withDraft(fn) { const d = L.copy(draft); fn(d); change(d); }
    // while a part or a block is being dragged: the draft follows the hand, and letting go is one Undo
    property string dragId: ""
    function liveDraft(d) { draft = d; }
    function commitFrom(beforeJson) {
        if (!beforeJson || beforeJson === JSON.stringify(draft)) return;
        undoDraft = undoDraft.concat([beforeJson]).slice(-200); redoDraft = []; lastLook = "";
    }
    function partIndex(id) { return draft.parts.findIndex(p => p.id === id); }
    // a look changed in the side panel: a run of changes to the same setting is one Undo
    property string lastLook: ""
    function setLook(id, patch) {
        const key = id + ":" + Object.keys(patch).join(",");
        const d = L.copy(draft);
        const l = Object.assign({}, d.looks[id] || {}, patch);
        for (const k in patch) if (patch[k] === "" || patch[k] === undefined || patch[k] === null) delete l[k];
        if (L.lookEmpty(l)) delete d.looks[id]; else d.looks[id] = l;
        if (lastLook === key && undoDraft.length) { draft = d; redoDraft = []; return; }
        change(d);
        lastLook = key;
    }
    function resetLook(id) { withDraft(d => { delete d.looks[id]; const p = d.parts.find(x => x.id === id); const s = start.parts.find(x => x.id === id); if (p && s) { p.corners = s.corners; p.seeThrough = s.seeThrough; p.accent = s.accent; } }); }
    function hide(id) {
        if (editing) {
            const i = partIndex(id);
            if (i < 0) return takeOut(id);
            if (draft.parts.filter(p => !p.hidden && p.id !== id).length === 0) return toast("One part stays, or the window would be empty.", false);
            withDraft(d => { d.parts[i].hidden = true; });
            if (picked === id) picked = "";
            if (customizeFor === id) customizeFor = "";
            return toast(titleOf(id) + " is hidden. It waits in Blocks › Panes, and in Layers.", false);
        }
        // not editing: hidden right where it is (the rest closes up), kept at once with Undo (the part's own menu)
        const pl = L.copy(res.placed);
        const i = pl.findIndex(p => p.id === id);
        if (i < 0) return;
        if (pl.filter(p => !p.hidden && p.id !== id).length === 0) return toast("One part stays, or the window would be empty.", false);
        pl[i].hidden = true;
        pushUndo({ page: pageId, before: savedCopy() });
        keepPage(L.toJson(pl, res.mode !== "grid" && !hasExtras));
        toast(titleOf(id) + " is hidden. Right-click › Edit layout brings it back.", true);
    }
    function show(id) { const i = partIndex(id); if (i < 0) return; withDraft(d => { d.parts[i].hidden = false; }); picked = id; }
    function takeOut(id) {
        withDraft(d => { d.added = d.added.filter(b => b.id !== id); d.z = d.z.filter(x => x !== id); delete d.looks[id]; });
        if (picked === id) picked = "";
        if (customizeFor === id) customizeFor = "";
    }
    function raise(id, up) {
        withDraft(d => {
            const all = d.parts.map(p => p.id).concat(d.added.map(b => b.id));
            let z = d.z.filter(x => all.indexOf(x) >= 0);
            for (const x of all) if (z.indexOf(x) < 0) z.unshift(x);
            z = z.filter(x => x !== id);
            if (up) z.push(id); else z.unshift(id);
            d.z = z;
        });
    }
    function startFrom(preset) {
        withDraft(d => {
            for (const q of preset.parts) { const p = d.parts.find(x => x.id === q.id); if (p) Object.assign(p, { x: q.x, y: q.y, w: q.w, h: q.h, hidden: !!q.hidden }); }
            if (preset.id === "hearth") { d.looks = {}; d.added = []; d.z = []; }
        });
        toast("Started from " + preset.name + ". Undo is here; nothing is kept until Keep this.", false);
    }
    function newId() { return "b-" + Date.now().toString(36) + Math.floor(Math.random() * 1296).toString(36); }

    // ---------------------------------------------------------------- Keep this, Try it, Cancel
    function sameDraft(a, b) { return JSON.stringify(a) === JSON.stringify(b); }
    function plan(d) {
        const W = width, H = height;
        const cellsOf = list => list.map((p, i) => Object.assign(L.own(defs[i]), { corners: p.corners === undefined ? null : p.corners, seeThrough: p.seeThrough || 0,
                                                    font: p.font || "", size: p.size || "", accent: p.accent || "" }, L.pxCell(p, W, H, cols, rows), { hidden: !!p.hidden }));
        const geo = !start || d.parts.some((p, i) => { const s = start.parts[i]; return !s || s.x !== p.x || s.y !== p.y || s.w !== p.w || s.h !== p.h; });
        const ex = { rects: {}, looks: d.looks, z: d.z, added: d.added };
        if (geo || hasExtras || Object.keys(d.looks).length || d.added.length || d.z.length) for (const p of d.parts) ex.rects[p.id] = L.toFrac(p, W, H);
        let extras = L.extrasJson(ex);
        if (d.mine && d.mine.length) { extras = extras || { v: 1 }; extras.mine = L.copy(d.mine); }
        const cells = cellsOf(d.parts);
        const pj = extras ? L.toJson(cells, false) : L.pageJson(cells, cellsOf(start ? start.parts : d.parts), startMode);
        return { page: pj, extras: extras };
    }
    function write(d) {
        if (unreadable || looksUnreadable) {
            toast("Your saved layout for " + appName + " couldn't be read, so nothing was changed.", false);
            return false;
        }
        const p = plan(d);
        const ch = { blocks: {} }; ch.blocks[pageId] = p.page;
        if (Object.keys(d.zones || {}).length) ch.zones = L.copy(d.zones);
        keepChange(ch);
        const lk = Object.assign({}, looksKept);
        if (p.extras) lk[pageId] = p.extras; else delete lk[pageId];
        looksKept = lk;
        const m = {}; m[pageId] = p.extras;
        looksWriter.merge(m);
        return true;
    }
    function keepThis() {
        if (!editing && !trying) return;
        if (editing && !sameDraft(draft, start)) { if (!write(draft)) return; toast(appName + ": kept.", false); }
        else if (trying) toast(appName + ": kept.", false);
        close();
    }
    function done() { keepThis(); }
    function tryIt() {
        if (!editing) return;
        if (!sameDraft(draft, start) && !write(draft)) return;
        tried = true;
        trying = true; editing = false; customizeFor = ""; menuEditor = false;
        if (focusBack) focusBack.forceActiveFocus();
    }
    function backToEditing() { if (!trying) return; const d = draft; edit(picked); draft = d; }
    // what was kept before Edit layout opened, back (Try it's Go back, and Cancel after Try it)
    function goBack() {
        if (before) {
            const ch = { blocks: {} }; ch.blocks[pageId] = before.page === null ? {} : before.page;
            const zs = {};
            for (const k in (draft.zones || {})) zs[k] = before.zones && before.zones[k] ? before.zones[k] : [];
            if (Object.keys(zs).length) ch.zones = zs;
            keepChange(ch);
            const lk = Object.assign({}, looksKept);
            if (before.extras) lk[pageId] = before.extras; else delete lk[pageId];
            looksKept = lk;
            const m = {}; m[pageId] = before.extras; looksWriter.merge(m);
        }
        tried = false;
        close();
        toast(appName + " is as it was.", false);
    }
    function cancel() {
        if (tried) return goBack();
        close();
    }
    function close() {
        editing = false; trying = false; picked = ""; customizeFor = ""; menuEditor = false; menuShown = false;
        if (focusBack) focusBack.forceActiveFocus();
    }

    // the keys
    FocusScope {
        id: editKeys
        focus: ab.editing
        Keys.onPressed: e => {
            if (!ab.editing) return;
            const ctrl = (e.modifiers & Qt.ControlModifier) !== 0, shift = (e.modifiers & Qt.ShiftModifier) !== 0;
            const W = ab.width, H = ab.height;
            const mv = (dx, dy) => { const r = L.movePx(ab.draft.parts, ab.picked, dx, dy, W, H, {}); ab.withDraft(d => { d.parts = r.list; }); };
            const sz = (ed, dx, dy) => { const r = L.sizePx(ab.draft.parts, ab.picked, ed, dx, dy, W, H, {}); ab.withDraft(d => { d.parts = r.list; }); };
            if (ab.menuEditor) { if (e.key === Qt.Key_Escape) { ab.menuEditor = false; e.accepted = true; } return; }
            if (ctrl && e.key === Qt.Key_Z) { if (shift) ab.redo(); else ab.undo(); }
            else if (ctrl && e.key === Qt.Key_Y) ab.redo();
            else if (e.key === Qt.Key_Escape || e.key === Qt.Key_Back) { if (ab.customizeFor !== "") ab.customizeFor = ""; else ab.cancel(); }
            else if (e.key === Qt.Key_Tab || e.key === Qt.Key_Backtab) {
                const vis = ab.draft.parts.filter(p => !p.hidden);
                if (vis.length) { const at = vis.findIndex(p => p.id === ab.picked); const dir = e.key === Qt.Key_Backtab || shift ? -1 : 1; ab.picked = vis[(at + dir + vis.length) % vis.length].id; }
            }
            else if (ab.partIndex(ab.picked) < 0) return;
            else if (shift && e.key === Qt.Key_Left) sz({ r: 1 }, -8, 0);
            else if (shift && e.key === Qt.Key_Right) sz({ r: 1 }, 8, 0);
            else if (shift && e.key === Qt.Key_Up) sz({ b: 1 }, 0, -8);
            else if (shift && e.key === Qt.Key_Down) sz({ b: 1 }, 0, 8);
            else if (e.key === Qt.Key_Left) mv(-8, 0);
            else if (e.key === Qt.Key_Right) mv(8, 0);
            else if (e.key === Qt.Key_Up) mv(0, -8);
            else if (e.key === Qt.Key_Down) mv(0, 8);
            else if (e.key === Qt.Key_PageDown) sz({ r: 1, b: 1 }, 8, 8);
            else if (e.key === Qt.Key_PageUp) sz({ r: 1, b: 1 }, -8, -8);
            else if (e.key === Qt.Key_Menu) ab.customize(ab.picked, "look");
            else if (e.key === Qt.Key_Delete) ab.hide(ab.picked);
            else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) ab.keepThis();
            else return;
            e.accepted = true;
        }
    }

    // ---------------------------------------------------------------- keeping it (hearth-layout app-keep); the part
    // menu's Hide this and Back to the original (outside Edit layout) are kept at once, with Undo
    property var undoStack: []
    function savedCopy() { return savedPage === undefined ? null : L.copy(savedPage); }
    function pushUndo(u) { undoStack = undoStack.concat([u]).slice(-60); }
    function restUndo() {
        if (!undoStack.length) return toast("Nothing to undo.", false);
        const u = undoStack[undoStack.length - 1];
        undoStack = undoStack.slice(0, -1);
        if (u.zones) keepChange({ zones: u.zones });
        if (u.pages) keepChange({ blocks: u.pages });
        if (u.looks) { const lk = Object.assign({}, looksKept); for (const k in u.looks) { if (u.looks[k]) lk[k] = u.looks[k]; else delete lk[k]; } looksKept = lk; looksWriter.merge(u.looks); }
        if (u.page) {
            const b = {}; b[u.page] = u.before === null || u.before === undefined ? {} : u.before;
            keepChange({ blocks: b });
        }
        toast("Undone.", false);
    }
    function keepPage(pj) { keepFor(pageId, pj); }
    // another page of the app made of blocks (Settings' pages: AppBlocksFlow.qml), kept the same way
    property var extraPages: []
    function keepFor(pid, pj) { const b = {}; b[pid] = pj; keepChange({ blocks: b }); }
    // Back to the original: this page as the app has it (in Edit layout: on the draft), or every Mantel page, menu
    // and look of the app (kept at once, with Undo).
    function backToOriginal() {
        if (editing) {
            const presets = L.presets(measureOwn(), width, height);
            const d = L.copy(draft);
            d.parts = presets[0].parts.map(q => Object.assign({}, d.parts.find(x => x.id === q.id) || {}, q, { corners: null, seeThrough: 0, font: "", size: "", accent: "" }));
            d.looks = {}; d.added = []; d.z = [];
            const zs = {}; for (const k in (layoutKept.zones || {})) if (k.indexOf("menu:") === 0) zs[k] = [];
            d.zones = zs;
            change(d);
            return toast(appName + " is back to the original. Keep this keeps it; Undo is here.", false);
        }
        const b = layoutKept.blocks || {}, z = layoutKept.zones || {};
        const pagesBefore = {}, pagesNow = {}, zonesBefore = {}, zonesNow = {}, looksBefore = {}, looksNow = {};
        for (const k in b) if (k.indexOf("mantel-") === 0) { pagesBefore[k] = L.copy(b[k]); pagesNow[k] = {}; }
        for (const k in z) if (k.indexOf("menu:") === 0) { zonesBefore[k] = L.copy(z[k]); zonesNow[k] = []; }
        for (const k in looksKept) if (k.indexOf("mantel-") === 0) { looksBefore[k] = L.copy(looksKept[k]); looksNow[k] = null; }
        pushUndo({ pages: pagesBefore, zones: Object.keys(zonesBefore).length ? zonesBefore : null, looks: Object.keys(looksBefore).length ? looksBefore : null });
        const ch = {};
        if (Object.keys(pagesNow).length) ch.blocks = pagesNow;
        if (Object.keys(zonesNow).length) ch.zones = zonesNow;
        if (Object.keys(ch).length) keepChange(ch);
        if (Object.keys(looksNow).length && !looksUnreadable) { const lk = Object.assign({}, looksKept); for (const k in looksNow) delete lk[k]; looksKept = lk; looksWriter.merge(looksNow); }
        toast(appName + " is back to the original. Undo is here.", true);
    }
    // the app's own layout in px (Start from › Hearth): its cells as it has them
    function measureOwn() { return defs.map(d => Object.assign(L.own(d), { title: d.title }, L.cellPx(d, width, height, cols, rows))); }

    // What hearth-layout is told of the app's blocks: every page, the one shown where its parts are now.
    function catalogNow() {
        const list = pages.map(pg => ({ id: "mantel-" + pg.id, title: pg.title, blocks: pg.blocks.map(b => ({ id: b.id, title: b.title, x: b.x, y: b.y, w: b.w, h: b.h })) }));
        const cur = list.find(p => p.id === pageId);
        if (cur) {
            const now = (editing ? draft.parts : restParts).map(p => L.pxCell(p, width || 1, height || 1, cols, rows));
            cur.blocks = defs.map((d, i) => Object.assign({ id: d.id, title: d.title }, now[i] ? { x: now[i].x, y: now[i].y, w: now[i].w, h: now[i].h } : { x: d.x, y: d.y, w: d.w, h: d.h }));
        }
        return L.catalog(list.concat(extraPages.filter(x => !list.some(p => p.id === x.id))), cols, rows);
    }
    function describe() {
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

    // ---------------------------------------------------------------- menus put in order (zone "menu:<kind>"), on the
    // draft while editing (written with Keep this)
    function menuOrder(kind) {
        if (editing && draft.zones && draft.zones["menu:" + kind] !== undefined) return draft.zones["menu:" + kind];
        const z = layoutKept.zones || {}; return z["menu:" + kind] || [];
    }
    function arrange(kind, items) { return L.arrangeMenu(items, menuOrder(kind)); }
    property bool menuEditor: false
    property string menuKind: ""
    property var menuIds: []
    property var menuLabels: ({})
    property var menuHidden: ({})
    function openMenus() { if (!menus.length) { toast(appName + " has no menus of its own to put in order; right-click any part › Customize › Does gives a part its own.", false); return; } menuKind = menuKind || menus[0].id; menuEditor = true; loadMenu(); }
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
        menuIds = ids; menuHidden = hid;
        const kind = menuKind;
        withDraft(d => { d.zones = Object.assign({}, d.zones || {}); d.zones["menu:" + kind] = L.menuZone(ids, hid); });
    }
    function menuMove(id, d) {
        const ids = menuIds.slice(), i = ids.indexOf(id), j = i + d;
        if (i < 0 || j < 0 || j >= ids.length) return;
        ids[i] = ids[j]; ids[j] = id;
        menuChange(ids, menuHidden);
    }
    function menuToggle(id) { const h = Object.assign({}, menuHidden); h[id] = !h[id]; if (!h[id]) delete h[id]; menuChange(menuIds, h); }

    // ---------------------------------------------------------------- a part's own menu (right-click, outside Edit layout)
    property bool menuShown: false
    property string menuFor: ""
    property point menuAt: Qt.point(0, 0)
    function partMenu(id, x, y) {
        if (defs.length === 0) return;
        if (editing && editor.item) { editor.item.partMenu(id, x, y); return; }
        menuFor = id; menuAt = Qt.point(x, y); menuShown = true;
    }
    function runPartMenu(what) {
        menuShown = false;
        if (what === "edit") edit(menuFor);
        else if (what === "customize") customize(menuFor, "look");
        else if (what === "move") { edit(menuFor); toast("Drag " + titleOf(menuFor).toLowerCase() + " anywhere on the window, or use the arrows.", false); }
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
                model: [["edit", "Edit layout"], ["customize", "Customize…"], ["move", "Move this"], ["hide", "Hide this"], ["describe", "Describe it…"]].concat(ab.customised ? [["reset", "Back to the original"]] : [])
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

    // ---------------------------------------------------------------- the editor itself (outlines, Blocks, the toolbar,
    // Customize): its own file, loaded only while editing or trying, so a problem there never stops the app
    Loader {
        id: editor
        anchors.fill: parent
        z: 40
        active: ab.editing || ab.trying
        Component.onCompleted: setSource("AppBlocksEditor.qml", { ab: ab })
        onStatusChanged: if (status === Loader.Error) { console.warn("AppBlocks: the editor couldn't load (AppBlocksEditor.qml); " + ab.app + " stays as it is"); ab.close(); }
    }
    property alias editorItem: editor.item

    // ---------------------------------------------------------------- the menus' items, in order or left out
    Rectangle {
        visible: ab.editing && ab.menuEditor
        z: 45
        width: Math.min(460, ab.width - 40)
        height: Math.min(ab.height - 200, meCol.implicitHeight + 28)
        anchors.horizontalCenter: parent.horizontalCenter
        y: Math.max(12, ab.height - 150 - height)
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
                    EBtn { small: true; label: "Done"; onClicked: ab.menuEditor = false }
                }
                Text { width: parent.width; wrapMode: Text.WordWrap; text: "Put the items in the order you want, or leave some out. Edit layout and Describe it always stay. Kept with Keep this."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
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
        y: ab.editing || ab.trying ? ab.height - height - 150 : ab.height - height - 18
        width: Math.min(ab.width - 40, toastRow.implicitWidth + 32)
        height: 44; radius: 22
        color: Theme.panel3; border.color: Theme.line
        Row {
            id: toastRow
            anchors.centerIn: parent
            spacing: 14
            Text { anchors.verticalCenter: parent.verticalCenter; width: Math.min(implicitWidth, ab.width - 200); elide: Text.ElideRight; text: ab.toastText; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
            EBtn { visible: ab.toastUndo; small: true; label: "Undo"; onClicked: { ab.toastText = ""; ab.restUndo(); } }
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

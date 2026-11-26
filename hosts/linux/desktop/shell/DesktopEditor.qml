import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "desktop-edit.js" as E
import "widget-tools.js" as T
import "widget-layout.js" as L
import "block-look.js" as BL
import "icon-data.js" as IconData

// Edit desktop (drag and drop, board 1; docs/mockups/drag-drop/drag-drop-1.png, checklist
// docs/mockups/checklists/drag-drop.md part 1). Over your real desktop: every bar, panel and widget outlined with what
// it is and its size; the Blocks window (its own window you can move) to drag from; drag any block onto any bar, any
// panel or the desktop, or a placed one anywhere else; panels anywhere on the screen (an edge is only a snap: the
// owner, Nov 26, "dont want them locked to just 4 sides"), sized by their ends and sides; widgets free on the desktop;
// snap to grid and guides; Undo, Redo, Layers, Screen, + Panel, Start from; Cancel, Try it, Keep this. Nothing is
// written until Keep this (or Try it, which can be taken back), and then only through BarLayout and WidgetStore
// (merged, atomic, a .bak kept). The rules are in desktop-edit.js (test_desktop_edit.py).
// Board 2 (docs/mockups/drag-drop/drag-drop-2.png, checklist part 2): click a block to pick it (lit, with handles);
// right-click any block for Customize…, Copy its look, Paste a look, Duplicate, Move to ›, Hide (keep it here) and
// Take it out; the side panel (its own window you can move) has its Look, Shows, Does, Place, When and Motion, Save as
// my own block…, Copy its look and Apply to every bar item; pick the bar itself (its empty space) for its own
// settings. Looks are kept in bar.json "s:<block id>" (block-look.js) and drawn on the real desktop by BlockLook.qml.
// Loaded by shell.qml the first time it's opened, so a problem here only loses the editor, never the desktop.
PanelWindow {
    id: ed
    property bool shown: false
    property int scr: 0
    screen: Quickshell.screens[Math.min(scr, Math.max(0, Quickshell.screens.length - 1))] || null
    readonly property string scrName: screen ? screen.name : ""
    readonly property real sw: width
    readonly property real sh: height

    // ---- the draft (desktop-edit.js), what it was when opened, and Undo / Redo
    property var draft: ({ main: T.barLayout(null), panels: ({}), widgets: ({}), own: ({}), looks: ({}), bar: ({}) })
    property var base: null
    property var undoStack: []
    property var redoStack: []
    property var rawBefore: null          // bar.json and this screen's widgets before Try it or Start from
    property var widgetsBefore: null
    property bool trying: false
    property bool snapOn: true
    property int grid: 8
    property bool guidesOn: true
    property bool layersShown: false
    property bool startShown: false
    property var layouts: []
    property string picked: ""            // "main" | "p:<id>" | "w:<id>" | "b:<block id>"
    property string note: ""

    // ---- board 2: one block picked
    property var blk: null                // the block picked: {id, from, w, h}
    property string tab: "look"           // look | shows | does | place | when | motion
    property var lookClip: null           // Copy its look
    property var menuBlk: null            // the right-click menu's block: {id, from}
    property point menuAt: Qt.point(0, 0)
    property string popup: ""             // "icon" | "font" | "colour:<key>" | "saveas" | "hoverOwn" | "pressedOwn"
    property string lastKey: ""           // a look being changed: one Undo for a run of changes to one setting
    property double lastAt: 0
    readonly property var cur: BL.normalize(blk && draft.looks ? draft.looks[blk.id] : null)
    readonly property string curBase: blk ? T.baseId(blk.id) : ""

    // ---- dragging
    property string dragId: ""            // a block being dragged
    property var dragFrom: null
    property point dragAt: Qt.point(0, 0)
    property var target: null             // where it would land (desktop-edit.js)
    property real lineX: -1; property real lineY: -1; property real lineLen: 0; property bool lineDown: false
    property var guides: []

    visible: shown
    color: "transparent"
    anchors { top: true; bottom: true; left: true; right: true }
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "hearth-desktop-editor"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: shown && !trying ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    // while trying it, only the small pill takes clicks: the desktop works as it will
    mask: trying ? tryMask : null
    Region { id: tryMask; item: tryPill }

    Component.onCompleted: E.use(T)

    function open(n) {
        if (n !== undefined && n !== null) { const i = Quickshell.screens.findIndex(s => s.name === n); if (i >= 0) scr = i; }
        load();
        undoStack = []; redoStack = []; picked = ""; note = ""; trying = false; layersShown = false; startShown = false;
        blk = null; menuBlk = null; popup = ""; tab = "look";
        rawBefore = JSON.parse(JSON.stringify(BarLayout.raw)); widgetsBefore = mineNow();
        shown = true;
        keys.forceActiveFocus();
    }
    function mineNow() {
        const out = {};
        for (const w of L.forScreen(WidgetStore.widgets, scrName, Quickshell.screens.map(s => s.name), Quickshell.screens.length ? Quickshell.screens[0].name : "", Activities.current))
            out[w.id] = { type: w.type, x: w.x, y: w.y, w: w.w, h: w.h };
        return out;
    }
    function load() {
        const panels = {}, own = {}, looks = {};
        for (const k in BarLayout.raw) {
            if (k.startsWith("p:")) { const p = BarLayout.raw[k]; if (p && typeof p === "object" && (!p.screen || p.screen === scrName || Quickshell.screens.length < 2)) panels[k.slice(2)] = JSON.parse(JSON.stringify(p)); }
            if (k.startsWith("b:")) own[k.slice(2)] = JSON.parse(JSON.stringify(BarLayout.raw[k]));
            if (k.startsWith("s:")) { const v = BarLayout.raw[k]; if (v && typeof v === "object" && !Array.isArray(v)) looks[k.slice(2)] = JSON.parse(JSON.stringify(v)); }
        }
        const bar = BarLayout.raw.bar && typeof BarLayout.raw.bar === "object" && !Array.isArray(BarLayout.raw.bar) ? JSON.parse(JSON.stringify(BarLayout.raw.bar)) : {};
        draft = { main: T.barLayout(BarLayout.raw.items), panels: panels, widgets: mineNow(), own: own, looks: looks, bar: bar };
        base = JSON.parse(JSON.stringify(draft));
    }
    function change(d) {
        if (!d) return;
        undoStack = undoStack.concat([JSON.stringify(draft)]).slice(-200);
        redoStack = [];
        draft = d;
        lastKey = "";
    }
    // a run of changes to one setting (a slider dragged, a number typed) is one Undo
    function changeRun(d, key) {
        if (!d) return;
        if (key !== lastKey || Date.now() - lastAt > 1500) { undoStack = undoStack.concat([JSON.stringify(draft)]).slice(-200); redoStack = []; }
        lastKey = key; lastAt = Date.now();
        draft = d;
    }
    function undo() { if (!undoStack.length) return; redoStack = redoStack.concat([JSON.stringify(draft)]); draft = JSON.parse(undoStack[undoStack.length - 1]); undoStack = undoStack.slice(0, -1); }
    function redo() { if (!redoStack.length) return; undoStack = undoStack.concat([JSON.stringify(draft)]); draft = JSON.parse(redoStack[redoStack.length - 1]); redoStack = redoStack.slice(0, -1); }

    // ---- writing (only here, only through the safe writers)
    function write(d, from) {
        const c = E.changes(from, d);
        if (E.nothingChanged(c)) return true;
        // nothing is written over a bar.json that couldn't be read (block-look.js keepPlan)
        const plan = BL.keepPlan(BarLayout.readOk, c.bar);
        if (!plan.write && Object.keys(c.bar).length) { SettingsNav.toast(plan.why); return false; }
        if (Object.keys(c.bar).length) BarLayout.put(c.bar);
        for (const id in c.widgets.put) { const w = c.widgets.put[id]; WidgetStore.place(id, w.x, w.y, w.w, w.h, scrName); }
        for (const a of c.widgets.add) WidgetStore.add(a.rec.type, scrName, Activities.current, "desk", { x: a.rec.x, y: a.rec.y, w: a.rec.w, h: a.rec.h }, {});
        for (const id of c.widgets.remove) WidgetStore.remove(id);
        return true;
    }
    function putBack() {
        // bar.json as it was (keys added since are taken out), and this screen's widgets as they were
        const o = {};
        for (const k in BarLayout.raw) if (rawBefore[k] === undefined) o[k] = null;
        for (const k in rawBefore) if (JSON.stringify(rawBefore[k]) !== JSON.stringify(BarLayout.raw[k])) o[k] = rawBefore[k];
        if (Object.keys(o).length) BarLayout.put(o);
        const now = mineNow();
        for (const id in now) if (!widgetsBefore[id]) WidgetStore.remove(id);
        for (const id in widgetsBefore) {
            const w = widgetsBefore[id];
            if (!now[id]) WidgetStore.put(id, { type: w.type, place: "desk", screen: scrName, x: w.x, y: w.y, w: w.w, h: w.h, activity: Activities.current });
            else WidgetStore.place(id, w.x, w.y, w.w, w.h, scrName);
        }
    }
    function keep() { if (!write(draft, trying ? draft : base)) return; trying = false; shown = false; blk = null; menuBlk = null; popup = ""; SettingsNav.toast("Your desktop is kept"); }
    function cancel() { if (trying || startedFrom) putBack(); trying = false; startedFrom = false; shown = false; blk = null; menuBlk = null; popup = ""; }
    function tryIt() { if (!write(draft, base)) return; base = JSON.parse(JSON.stringify(draft)); trying = true; menuBlk = null; popup = ""; }
    function goBack() { putBack(); trying = false; load(); }
    property bool startedFrom: false

    // ---- names and the Blocks window's list
    function nameOf(id) {
        if (E.isOwn(id)) { const o = draft.own[id.slice(4)]; return o ? (o.label || (o.kind === "spacer" ? "Spacer" : "Your block")) : "Your block"; }
        const wt = E.widgetType(id);
        if (wt) { const i = L.info(wt); return i ? i.name : wt; }
        const b = T.barItem(id) || T.panelItem(id);
        return b ? b.name.replace(/ \(.*\)$/, "") : id;
    }
    function iconOf(id) {
        if (E.isOwn(id)) { const o = draft.own[id.slice(4)]; return o && o.icon ? o.icon : "dot"; }
        const wt = E.widgetType(id);
        if (wt) { const i = L.info(wt); return i ? i.icon : "dot"; }
        const b = T.barItem(id) || T.panelItem(id);
        return b ? b.icon : "dot";
    }
    readonly property var lib: E.library(T.BAR_ITEMS, T.PANEL_ITEMS, L.CATALOG, draft.own)
    property string libKind: "all"
    property string libWords: ""
    readonly property var shownLib: E.find(lib, libKind, libWords)

    // ---- the main bar's box
    // its own settings (board 2: the bar picked), as they are in the draft
    readonly property var mbl: T.barLook(draft.bar)
    readonly property string mainEdge: T.barSide(mbl, Theme.barEdge || DeskState.barEdge || "top")
    readonly property bool mainDown: mbl.edge === "free" ? mbl.orient === "down" : mainEdge === "left" || mainEdge === "right"
    readonly property int nowSize: mainEdge === "top" ? DeskState.barTop : mainEdge === "bottom" ? DeskState.barBottom : mainEdge === "left" ? DeskState.barLeft : DeskState.barRight
    readonly property int mainSize: mbl.thick > 0 ? mbl.thick : Math.max(32, nowSize > 0 ? nowSize : 44)
    readonly property int mainWant: (draft.main.start.length + draft.main.centre.length + draft.main.end.length) * 110 + 60
    readonly property var mainB: E.mainBox(mbl, mainEdge, sw, sh, mainSize, mainWant)
    readonly property rect mainBox: Qt.rect(mainB.x, mainB.y, mainB.w, mainB.h)

    function inBox(p, r, pad) { pad = pad || 0; return p.x >= r.x - pad && p.x <= r.x + r.width + pad && p.y >= r.y - pad && p.y <= r.y + r.height + pad; }
    function panelRect(id) { const p = T.panelOf(id, draft.panels[id]); if (!p) return Qt.rect(0, 0, 0, 0); const b = E.panelBox(p, sw, sh, (p.items.length || 1) * 48 + 24); return Qt.rect(b.x, b.y, b.w, b.h); }
    function othersFor(skip) {
        const o = [];
        for (const id in draft.widgets) if ("w:" + id !== skip) { const w = draft.widgets[id]; o.push({ x: w.x, y: w.y, w: w.w, h: w.h, name: "the " + nameOf("widget:" + w.type).toLowerCase() }); }
        for (const id in draft.panels) if ("p:" + id !== skip && !draft.panels[id].inside) { const r = panelRect(id); o.push({ x: r.x, y: r.y, w: r.width, h: r.height, name: "the panel" }); }
        return o;
    }

    // ---- dragging a block: where it would land
    function startDrag(id, from, p) { dragId = id; dragFrom = from; dragAt = p; picked = ""; blk = null; menuBlk = null; popup = ""; moveDrag(p); }
    function chipCentres(rep, down) {
        const out = [];
        for (let i = 0; i < rep.count; i++) { const it = rep.itemAt(i); if (!it) continue; const c = it.mapToItem(ed.contentItem, it.width / 2, it.height / 2); out.push(down ? c.y : c.x); }
        return out;
    }
    function moveDrag(p) {
        dragAt = p;
        let t = null, why = "";
        lineX = -1;
        if (inBox(p, Qt.rect(libWin.x, libWin.y, libWin.width, libWin.height))) {
            t = { kind: "lib" }; note = E.isOwn(dragId) || dragFrom.kind === "lib" ? "" : "Let go to take it out";
        } else if (inBox(p, mainBox, 6)) {
            // the zone under the pointer: the bar's first third, middle or last third
            const along = mainDown ? p.y - mainBox.y : p.x - mainBox.x, len = mainDown ? mainBox.height : mainBox.width;
            const zone = along < len / 3 ? "start" : along > len * 2 / 3 ? "end" : (draft.main.centre.length || !mainDown ? "centre" : "start");
            const rep = zone === "start" ? mainStart : zone === "end" ? mainEnd : mainCentre;
            const cs = chipCentres(rep, mainDown);
            const i = E.insertAt(cs, mainDown ? p.y : p.x);
            t = { kind: "main", zone: zone, index: i };
            placeLine(rep, i, mainDown, zone === "end" ? mainBox.x + mainBox.width - 12 : zone === "centre" ? mainBox.x + mainBox.width / 2 : mainBox.x + 12, mainBox);
            const after = i > 0 && rep.itemAt(i - 1) ? nameOf(rep.itemAt(i - 1).bid).toLowerCase() : "";
            note = "Drop: " + mainEdge + " bar" + (after ? ", after the " + after : zone === "end" ? ", at the end" : ", at the start");
        } else {
            for (const id in draft.panels) {
                if (draft.panels[id].inside) continue;
                const r = panelRect(id);
                if (!inBox(p, r, 6)) continue;
                const pp = T.panelOf(id, draft.panels[id]);
                const down = pp.free ? pp.orient === "down" : pp.edge === "left" || pp.edge === "right";
                const rep = panelReps.itemAt(Object.keys(draft.panels).filter(k => !draft.panels[k].inside).indexOf(id));
                const chips = rep ? rep.chips : null;
                const cs = chips ? chipCentres(chips, down) : [];
                const i = E.insertAt(cs, down ? p.y : p.x);
                t = { kind: "panel", id: id, index: i };
                if (chips) placeLine(chips, i, down, down ? r.y + 8 : r.x + 8, r);
                note = "Drop: this panel" + (i > 0 && chips && chips.itemAt(i - 1) ? ", after the " + nameOf(chips.itemAt(i - 1).bid).toLowerCase() : "");
                break;
            }
            if (!t) {
                const wt = E.widgetType(dragId) || dragId, info = L.info(wt);
                const w = info ? info.w : 200, h = info ? info.h : 120;
                const s = snapOn || guidesOn ? E.snap({ x: p.x - w / 2, y: p.y - 20, w: w, h: h }, othersFor(dragFrom && dragFrom.kind === "desk" ? "w:" + dragFrom.id : ""), snapOn ? grid : 0, guidesOn) : { x: p.x - w / 2, y: p.y - 20, guides: [], note: "" };
                t = { kind: "desk", x: Math.max(0, Math.min(sw - w, s.x)), y: Math.max(0, Math.min(sh - h, s.y)) };
                guides = s.guides;
                note = s.note || "Drop: on the desktop";
            }
        }
        if (t && t.kind !== "lib") {
            const f = E.fit(dragId, t, L.info);
            if (!f.id) { why = f.why; t = null; lineX = -1; }
        }
        if (t && t.kind !== "desk") guides = [];
        target = t;
        if (why) note = why;
    }
    function placeLine(rep, i, down, emptyAt, r) {
        lineDown = down; lineLen = (down ? r.width : r.height) - 12;
        let at = emptyAt;
        if (rep.count > 0) {
            const it = rep.itemAt(Math.min(i, rep.count - 1));
            if (it) { const c = it.mapToItem(ed.contentItem, 0, 0); at = i >= rep.count ? (down ? c.y + it.height + 3 : c.x + it.width + 3) : (down ? c.y - 4 : c.x - 4); }
        }
        if (down) { lineX = r.x + 6; lineY = at; } else { lineX = at; lineY = r.y + 6; }
    }
    function endDrag() {
        const id = dragId, from = dragFrom, t = target;
        dragId = ""; dragFrom = null; target = null; lineX = -1; guides = [];
        if (!id || !t) { note = ""; return; }
        if (t.kind === "lib") {
            if (from && from.kind !== "lib") { const d = JSON.parse(JSON.stringify(draft)); E.take(d, id, from); change(d); }
            note = ""; return;
        }
        const d = E.drop(draft, id, from, t, L.info, () => "n" + Date.now().toString(36));
        if (d) change(d);
        note = "";
    }
    function makeOwn(kind, p) {
        const d = JSON.parse(JSON.stringify(draft));
        const k = Date.now().toString(36);
        d.own[k] = E.ownBlock(kind);
        change(d);
        startDrag("own:" + k, { kind: "lib" }, p);
    }

    // ---- moving and sizing a panel or a widget
    property var moving: null             // {what: "p"|"w", id, mode: "move"|"len0"|"len1"|"thick"|"corner", start, box}
    function startMove(what, id, mode, p) {
        blk = null; menuBlk = null; popup = "";
        const r = what === "p" ? panelRect(id) : (() => { const w = draft.widgets[id]; return Qt.rect(w.x, w.y, w.w, w.h); })();
        moving = { what: what, id: id, mode: mode, sx: p.x, sy: p.y, box: { x: r.x, y: r.y, w: r.width, h: r.height } };
        undoStack = undoStack.concat([JSON.stringify(draft)]).slice(-200); redoStack = [];
        picked = what + ":" + id;
    }
    function doMove(p) {
        if (!moving) return;
        const m = moving, dx = p.x - m.sx, dy = p.y - m.sy;
        let b = { x: m.box.x, y: m.box.y, w: m.box.w, h: m.box.h };
        const down = m.box.h > m.box.w;
        if (m.mode === "move") { b.x += dx; b.y += dy; }
        else if (m.mode === "len0") { if (down) { b.y += dy; b.h -= dy; } else { b.x += dx; b.w -= dx; } }
        else if (m.mode === "len1") { if (down) b.h += dy; else b.w += dx; }
        else if (m.mode === "thick") { if (down) b.w += dx; else b.h += dy; }
        else if (m.mode === "corner") { b.w += dx; b.h += dy; }
        b.w = Math.max(m.what === "p" ? (down ? 28 : 60) : 80, b.w); b.h = Math.max(m.what === "p" ? (down ? 60 : 28) : 60, b.h);
        if (m.mode === "move" && (snapOn || guidesOn)) { const s = E.snap(b, othersFor(m.what + ":" + m.id), snapOn ? grid : 0, guidesOn); b.x = s.x; b.y = s.y; guides = s.guides; note = s.note; }
        else { guides = []; note = m.what === "p" ? Math.round(b.w) + " × " + Math.round(b.h) : ""; }
        b.x = Math.max(0, Math.min(sw - b.w, b.x)); b.y = Math.max(0, Math.min(sh - b.h, b.y));
        const d = JSON.parse(JSON.stringify(draft));
        if (m.what === "p") d.panels[m.id] = E.placePanel(d.panels[m.id], b, sw, sh, snapOn);
        else d.widgets[m.id] = Object.assign({}, d.widgets[m.id], { x: Math.round(b.x), y: Math.round(b.y), w: Math.round(b.w), h: Math.round(b.h) });
        draft = d;
    }
    function endMove() { moving = null; guides = []; note = ""; }
    function addPanel(edge) {
        const d = JSON.parse(JSON.stringify(draft));
        const k = Date.now().toString(36);
        const box = edge === "right" ? { x: sw - 56, y: sh / 2 - 200, w: 56, h: 400 } : edge === "left" ? { x: 0, y: sh / 2 - 200, w: 56, h: 400 }
            : edge === "bottom" ? { x: sw / 2 - 300, y: sh - 44, w: 600, h: 44 } : edge === "top" ? { x: sw / 2 - 300, y: 0, w: 600, h: 44 }
            : { x: sw / 2 - 200, y: sh / 2 - 22, w: 400, h: 44 };
        d.panels[k] = E.placePanel({ screen: Quickshell.screens.length > 1 ? scrName : "", items: ["spacer"], size: 44 }, box, sw, sh, edge !== "");
        change(d);
        picked = "p:" + k;
    }
    function remove(what) {
        const d = JSON.parse(JSON.stringify(draft));
        if (what.startsWith("p:")) delete d.panels[what.slice(2)];
        else if (what.startsWith("w:")) delete d.widgets[what.slice(2)];
        change(d); picked = "";
    }
    // the edges with nothing on them, for "Drag here for a panel"
    readonly property var freeEdges: {
        const used = {}; used[mainEdge] = true;
        for (const id in draft.panels) { const p = T.panelOf(id, draft.panels[id]); if (p && !p.free && !p.inside) used[p.edge] = true; }
        if (Theme.dockFloats) used.bottom = true;
        return ["top", "bottom", "left", "right"].filter(e => !used[e]);
    }

    // ================================================================== board 2: one block picked
    function copyD() { const d = JSON.parse(JSON.stringify(draft)); if (!d.looks) d.looks = {}; if (!d.bar) d.bar = {}; return d; }
    function say(t) { note = t; noteOff.restart(); }
    Timer { id: noteOff; interval: 2600; onTriggered: if (ed.dragId === "" && !ed.moving) ed.note = "" }
    function pickBlock(id, from, w, h) {
        blk = { id: id, from: from, w: Math.round(w || 0), h: Math.round(h || 0) };
        picked = "b:" + id; menuBlk = null; popup = ""; layersShown = false; startShown = false;
    }
    // where a block is now, picked by its id (the checks, and after a move)
    function pickById(id) {
        const f = E.whereIs(draft, id);
        if (!f) return false;
        const it = chipFor(id);
        pickBlock(id, f, it ? it.width : 0, it ? it.height : 0);
        return true;
    }
    function chipFor(id) {
        const reps = [mainStart, mainCentre, mainEnd];
        for (const r of reps) for (let i = 0; i < r.count; i++) { const it = r.itemAt(i); if (it && it.bid === id) return it; }
        for (let j = 0; j < panelReps.count; j++) {
            const pd = panelReps.itemAt(j);
            if (!pd || !pd.chips) continue;
            for (let i = 0; i < pd.chips.count; i++) { const it = pd.chips.itemAt(i); if (it && it.bid === id) return it; }
        }
        return null;
    }
    // a look changed in the side panel (several changes to one setting are one Undo)
    function setLook(changes, key) {
        if (!blk) return;
        const d = copyD();
        const sp = BL.sparse(BL.merge(d.looks[blk.id], changes));
        if (sp) d.looks[blk.id] = sp; else delete d.looks[blk.id];
        changeRun(d, blk.id + "/" + (key || Object.keys(changes).join(",")));
    }
    function resetLook() { if (!blk) return; const d = copyD(); delete d.looks[blk.id]; change(d); say("Back to its own look"); }
    function copyLookOf(id) { lookClip = BL.copyLook(draft.looks ? draft.looks[id] : null); say("Its look is copied: right-click another block › Paste a look"); }
    function pasteLookOn(id) {
        if (!lookClip) { say("Copy a look first (right-click a block › Copy its look)"); return; }
        const d = copyD();
        const sp = BL.sparse(BL.pasteLook(d.looks[id], lookClip));
        if (sp) d.looks[id] = sp; else delete d.looks[id];
        change(d);
    }
    function applyAll() {
        if (!blk) return;
        const where = blk.from && blk.from.kind === "panel" ? blk.from.id : "main";
        const d = copyD();
        d.looks = BL.applyToAll(d.looks, E.blocksIn(d, where), d.looks[blk.id]);
        change(d);
        say("Its look is on every block in this " + (where === "main" ? "bar" : "panel"));
    }
    function duplicateBlock(b) {
        const d = E.duplicate(draft, b.id, b.from, Date.now().toString(36));
        if (d) change(d); else say("It can't be copied here");
    }
    function moveBlock(b, dest) {
        const d = E.moveTo(draft, b.id, b.from, dest, L.info, () => "n" + Date.now().toString(36), sw / 2 - 150, sh / 2 - 100);
        if (!d) { say("It can't go there"); return; }
        change(d);
        menuBlk = null;
        const nid = E.fit(b.id, dest.kind === "desk" ? { kind: "desk" } : dest, L.info).id;
        if (dest.kind === "desk" || !pickById(nid)) { blk = null; picked = ""; }
    }
    function hideBlock(b) {
        const d = copyD();
        const sp = BL.sparse(BL.merge(d.looks[b.id], { hidden: !BL.normalize(d.looks[b.id]).hidden }));
        if (sp) d.looks[b.id] = sp; else delete d.looks[b.id];
        change(d);
    }
    function takeOutBlock(b) { change(E.takeOut(draft, b.id, b.from)); blk = null; picked = ""; menuBlk = null; }
    // Save as my own block…: a button of yours with this look (Blocks › Your own); what a press does goes with it
    function saveAsOwn(name) {
        if (!blk) return;
        const d = copyD();
        const k = Date.now().toString(36);
        const press = BL.action(cur, "Press");
        d.own[k] = { kind: "button", label: name || nameOf(blk.id), icon: cur.icon || iconOf(blk.id),
                     run: press && press.kind === "run" ? press.arg : "", app: press && press.kind === "app" ? press.arg : "",
                     panel: press && press.kind === "panel" ? press.arg : "" };
        const lk = BL.sparse(BL.merge(cur, { doPress: "default", argPress: "", hidden: false }));
        if (lk) d.looks["own:" + k] = lk;
        change(d);
        popup = ""; libKind = "own";
        say("Saved: it's in Blocks › Your own");
    }
    function blockMenu(id, from, p) { menuBlk = { id: id, from: from }; menuAt = p; popup = ""; }
    // the right-click menu at a block's place (the checks' pictures)
    function menuAtBlock(id) {
        const f = E.whereIs(draft, id), it = chipFor(id);
        if (!f || !it) return false;
        const p = it.mapToItem(ed.contentItem, it.width - 10, it.height);
        blockMenu(id, f, p);
        return true;
    }

    // ---- the bar picked (its empty space): its own settings. The main bar: draft.bar; a panel: its record.
    readonly property string barPicked: picked === "main" ? "main" : picked.startsWith("p:") ? picked.slice(2) : ""
    readonly property var barNow: barPicked === "main" ? mbl : barPicked !== "" && draft.panels[barPicked] ? T.panelOf(barPicked, draft.panels[barPicked]) : null
    readonly property bool barInside: barPicked !== "main" && !!barNow && barNow.inside !== ""
    function setBarKey(k, v) {
        const d = copyD();
        if (barPicked === "main") d.bar = E.setBar(d.bar, k, v);
        else if (d.panels[barPicked]) { if (v === "" || v === null || v === undefined) delete d.panels[barPicked][k]; else d.panels[barPicked][k] = v; }
        else return;
        changeRun(d, "bar/" + barPicked + "/" + k);
    }
    function frac(v) { return Math.round(Math.max(0, Math.min(1, v)) * 1000) / 1000; }
    function setBarEdge(e) {
        const d = copyD();
        if (barPicked === "main") {
            if (e === "free") {
                const b = mainB;
                d.bar.edge = "free"; d.bar.orient = b.down ? "down" : "across";
                if (!d.bar.len || d.bar.len === "full") { d.bar.len = "pct"; d.bar.lenV = 60; }
                d.bar.fx = frac(b.down ? 0.04 : 0.5); d.bar.fy = frac(b.down ? 0.5 : 0.08);
            } else { d.bar.edge = e; delete d.bar.fx; delete d.bar.fy; delete d.bar.orient; }
        } else {
            const p = d.panels[barPicked];
            if (!p) return;
            if (e === "free") {
                const r = panelRect(barPicked);
                p.place = "free"; p.orient = r.height > r.width ? "down" : "across";
                p.fx = frac(r.x / Math.max(1, sw - r.width)); p.fy = frac(Math.max(60, r.y) / Math.max(1, sh - r.height));
                p.length = Math.round(Math.max(60, Math.min(1600, Math.max(r.width, r.height))));
            } else { p.place = ""; p.edge = e; delete p.fx; delete p.fy; delete p.length; delete p.orient; }
        }
        change(d);
    }
    function resetBar() {
        const d = copyD();
        if (barPicked === "main") d.bar = {};
        else if (d.panels[barPicked]) ["len", "lenV", "offset", "corners", "fill", "see", "blur", "gap", "lineup", "islands", "over", "layer", "autoHide"].forEach(k => delete d.panels[barPicked][k]);
        change(d);
    }
    // More than one panel on any edge: another on the same edge, along from the first
    function anotherOnEdge() {
        const e = barPicked === "main" ? (mbl.edge === "free" ? "" : mainEdge) : barNow && !barNow.free ? barNow.edge : "";
        if (!e) { addPanel(""); return; }
        const d = copyD();
        const k = Date.now().toString(36);
        d.panels[k] = { screen: Quickshell.screens.length > 1 ? scrName : "", edge: e, items: ["clock"], size: 40, len: "fit", offset: 60 + Object.keys(d.panels).length * 80, over: true };
        change(d);
        picked = "p:" + k;
        say("Another panel on the " + e + ": drag blocks into it");
    }
    // Panels inside panels: a panel in this bar or panel, with blocks of its own
    function innerPanel() {
        if (barPicked === "") return;
        const k = Date.now().toString(36);
        const d = E.addInner(copyD(), barPicked, k);
        if (!d) return;
        change(d);
        picked = "p:" + k;
        say("A panel inside: right-click a block › Move to › to put it in");
    }

    // ---- what the side panel shows
    property string fontWords: ""
    function iconLabel(n) { const s = String(n || "dot"); return s.length > 14 ? s.slice(0, 13) + "…" : s.charAt(0).toUpperCase() + s.slice(1); }
    function wordsHint() {
        if (!blk) return "";
        const nm = nameOf(blk.id).split(" ")[0];
        return BL.hasValue(curBase) ? nm + " · {" + BL.valueKeys(curBase)[0] + "}" : nm;
    }
    function appsLike(t) {
        const q = String(t || "").trim().toLowerCase();
        const all = AppEntries.values || [];
        const out = [];
        for (const a of all) {
            if (out.length >= 8) break;
            if (a.noDisplay) continue;
            if (q === "" || String(a.name || "").toLowerCase().indexOf(q) >= 0 || String(a.id || "").toLowerCase().indexOf(q) >= 0) out.push({ id: a.id, name: a.name || a.id });
        }
        return out;
    }
    // above or under windows: the whole bar the block is in
    function blkLayer() {
        if (!blk || !blk.from) return "top";
        if (blk.from.kind === "main") return mbl.layer;
        const p = draft.panels[blk.from.id];
        return p && p.layer === "bottom" ? "bottom" : "top";
    }
    function setBlkLayer(v) {
        if (!blk || !blk.from) return;
        const d = copyD();
        if (blk.from.kind === "main") d.bar = E.setBar(d.bar, "layer", v === "bottom" ? "bottom" : "");
        else if (d.panels[blk.from.id]) { if (v === "bottom") d.panels[blk.from.id].layer = "bottom"; else delete d.panels[blk.from.id].layer; }
        change(d);
    }
    function barTitle() {
        if (barPicked === "main") return mbl.edge === "free" ? "Main bar" : mainEdge.charAt(0).toUpperCase() + mainEdge.slice(1) + " bar";
        if (!barNow) return "Panel";
        if (barNow.inside) return "Panel inside";
        return barNow.free ? "Panel" : barNow.edge.charAt(0).toUpperCase() + barNow.edge.slice(1) + " panel";
    }
    function barWhere() {
        if (barPicked === "main") return mbl.edge === "free" ? "the main bar, anywhere" : "the main bar";
        if (!barNow) return "";
        if (barNow.inside) return "inside " + (barNow.inside === "main" ? "the main bar" : "a panel");
        return barNow.free ? "anywhere" : "on the " + barNow.edge + " edge";
    }
    function barEdgeNow() {
        if (barPicked === "main") return mbl.edge === "free" ? "free" : mainEdge;
        return barNow ? (barNow.free ? "free" : barNow.edge) : "";
    }
    function barLenNow() {
        if (!barNow) return "full";
        if (barPicked !== "main" && barNow.free) return barNow.length > 0 ? "px" : "fit";
        return barNow.len;
    }
    function barLenV() {
        if (!barNow) return 0;
        if (barPicked !== "main" && barNow.free) return barNow.length > 0 ? barNow.length : Math.round(Math.max(panelRect(barPicked).width, panelRect(barPicked).height));
        if (barNow.lenV > 0) return barNow.lenV;
        return barNow.len === "pct" ? 50 : Math.round(barPicked === "main" ? (mainDown ? mainBox.height : mainBox.width) : Math.max(panelRect(barPicked).width, panelRect(barPicked).height));
    }
    function setBarLen(v) {
        if (barPicked !== "main" && barNow && barNow.free) { setBarKey("length", v === "fit" ? null : barLenV()); return; }
        const d = copyD();
        if (barPicked === "main") { d.bar = E.setBar(d.bar, "len", v === "full" ? "" : v); if (v === "pct" && !(d.bar.lenV > 0 && d.bar.lenV <= 100)) d.bar.lenV = 60; if (v === "px" && !(d.bar.lenV > 100)) d.bar.lenV = Math.round((mainDown ? sh : sw) * 0.6); }
        else if (d.panels[barPicked]) { const p = d.panels[barPicked]; p.len = v; if (p.style === "centred" && v !== "fit") delete p.style; if (v === "pct" && !(p.lenV > 0 && p.lenV <= 100)) p.lenV = 60; if (v === "px" && !(p.lenV > 100)) p.lenV = 600; }
        change(d);
    }
    function setBarLenV(v) {
        if (barPicked !== "main" && barNow && barNow.free) { setBarKey("length", Math.max(40, v)); return; }
        const d = copyD();
        if (barPicked === "main") { if (!d.bar.len || d.bar.len === "full" || d.bar.len === "fit") d.bar.len = "px"; d.bar.lenV = v; }
        else if (d.panels[barPicked]) { const p = d.panels[barPicked]; if (!p.len || p.len === "full" || p.len === "fit") p.len = "px"; p.lenV = v; }
        changeRun(d, "bar/" + barPicked + "/lenV");
    }
    function barIslands() {
        if (barPicked === "main") return mbl.islands !== "" ? mbl.islands : Theme.mainStyle === "floating" ? "islands" : "strip";
        return barNow && barNow.islands ? "islands" : "strip";
    }
    function barHides() {
        if (barPicked === "main") return mbl.autoHide === "on" || (mbl.autoHide === "" && Theme.barAutoHide);
        return !!barNow && barNow.autoHide;
    }
    function barScreen() { return barNow ? (barNow.screen || "") : ""; }

    // ---- Start from: a ready-made layout (hearth-layout), taken back by Cancel
    Process { id: lister; command: ["hearth-layout", "list", "--json"]; stdout: StdioCollector { onStreamFinished: { try { ed.layouts = JSON.parse(text).layouts || JSON.parse(text); } catch (e) { ed.layouts = []; } } } }
    Process { id: applier; property string lay: ""; command: ["hearth-layout", "apply", lay]; onExited: (code) => { if (code === 0) reloadSoon.start(); else ed.note = "That layout couldn't be put in place"; } }
    Timer { id: reloadSoon; interval: 900; onTriggered: { ed.load(); ed.undoStack = []; ed.redoStack = []; } }
    function startFrom(id) { startShown = false; startedFrom = true; applier.lay = id; applier.running = true; }

    // ================================================================== what you see
    Item {
        id: keys
        anchors.fill: parent
        focus: true
        visible: !ed.trying
        Keys.onEscapePressed: { if (ed.popup !== "") ed.popup = ""; else if (ed.menuBlk) ed.menuBlk = null; else ed.cancel(); }
        Keys.onPressed: e => {
            if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_Z) { if (e.modifiers & Qt.ShiftModifier) ed.redo(); else ed.undo(); e.accepted = true; }
            else if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_Y) { ed.redo(); e.accepted = true; }
            else if (e.key === Qt.Key_Delete && ed.blk) { ed.takeOutBlock(ed.blk); e.accepted = true; }
            else if (e.key === Qt.Key_Delete && ed.picked !== "" && ed.picked !== "main") { ed.remove(ed.picked); e.accepted = true; }
        }

        // the desktop shows through, a little dimmed
        Rectangle { anchors.fill: parent; color: Qt.rgba(0.04, 0.03, 0.02, 0.18) }
        MouseArea { anchors.fill: parent; onClicked: { ed.picked = ""; ed.blk = null; ed.menuBlk = null; ed.popup = ""; ed.layersShown = false; ed.startShown = false; } }

        // ---- the main bar
        Rectangle {
            id: mainR
            x: ed.mainBox.x; y: ed.mainBox.y; width: ed.mainBox.width; height: ed.mainBox.height
            // its own fill, see-through and corners (board 2), as the real bar will have them
            color: ed.mbl.fill !== "" ? BL.argb(ed.mbl.fill, Math.max(0.3, 1 - ed.mbl.see / 100)) : Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, Math.max(0.3, 0.92 - ed.mbl.see / 100))
            radius: ed.mbl.corners >= 99 ? Math.min(width, height) / 2 : ed.mbl.corners >= 0 ? ed.mbl.corners : 0
            border.width: 0
            Dashes { anchors.fill: parent; anchors.margins: 2; lit: ed.picked === "main" || (ed.target && ed.target.kind === "main") }
            // its empty space: the bar itself picked
            MouseArea { anchors.fill: parent; onClicked: { ed.blk = null; ed.menuBlk = null; ed.popup = ""; ed.picked = "main"; } }
            Flow {
                id: startFlow
                anchors { left: parent.left; top: parent.top; margins: 6 }
                flow: ed.mainDown ? Flow.TopToBottom : Flow.LeftToRight
                spacing: 6
                Repeater { id: mainStart; model: ed.draft.main.start; delegate: Chip { required property string modelData; bid: modelData; from: ({ kind: "main", zone: "start" }) } }
            }
            Flow {
                anchors { horizontalCenter: ed.mainDown ? undefined : parent.horizontalCenter; verticalCenter: ed.mainDown ? parent.verticalCenter : undefined; top: ed.mainDown ? undefined : parent.top; topMargin: 6; left: ed.mainDown ? parent.left : undefined; leftMargin: 6 }
                flow: ed.mainDown ? Flow.TopToBottom : Flow.LeftToRight
                spacing: 6
                Repeater { id: mainCentre; model: ed.draft.main.centre; delegate: Chip { required property string modelData; bid: modelData; from: ({ kind: "main", zone: "centre" }) } }
            }
            Flow {
                anchors { right: parent.right; bottom: parent.bottom; margins: 6 }
                flow: ed.mainDown ? Flow.TopToBottom : Flow.LeftToRight
                layoutDirection: Qt.LeftToRight
                spacing: 6
                Repeater { id: mainEnd; model: ed.draft.main.end; delegate: Chip { required property string modelData; bid: modelData; from: ({ kind: "main", zone: "end" }) } }
            }
        }
        Tag { text: E.mainLabel(ed.mainEdge, ed.mainSize); x: ed.mainEdge === "right" ? ed.mainBox.x - width - 10 : ed.mainEdge === "left" ? ed.mainBox.width + 10 : 12; y: ed.mainEdge === "bottom" ? ed.mainBox.y - height - 10 : ed.mainEdge === "top" ? ed.mainBox.height + 10 : 12 }

        // ---- the panels, anywhere
        Repeater {
            id: panelReps
            model: Object.keys(ed.draft.panels).filter(k => !ed.draft.panels[k].inside)
            delegate: Item {
                id: pd
                required property string modelData
                required property int index
                readonly property var p: T.panelOf(modelData, ed.draft.panels[modelData])
                readonly property rect r: ed.panelRect(modelData)
                readonly property bool down: p ? (p.free ? p.orient === "down" : p.edge === "left" || p.edge === "right") : false
                property alias chips: chipsRep
                x: r.x; y: r.y; width: r.width; height: r.height
                Rectangle {
                    anchors.fill: parent
                    radius: pd.p && (pd.p.free || pd.p.style) ? Math.min(18, Math.min(width, height) / 2) : 0
                    color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.92)
                    border.color: Theme.line
                }
                Dashes { anchors.fill: parent; anchors.margins: -3; lit: ed.picked === "p:" + pd.modelData || (ed.target && ed.target.kind === "panel" && ed.target.id === pd.modelData) }
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.SizeAllCursor
                    onPressed: m => ed.startMove("p", pd.modelData, "move", mapToItem(ed.contentItem, m.x, m.y))
                    onPositionChanged: m => ed.doMove(mapToItem(ed.contentItem, m.x, m.y))
                    onReleased: ed.endMove()
                }
                Flow {
                    anchors { fill: parent; margins: 8 }
                    flow: pd.down ? Flow.TopToBottom : Flow.LeftToRight
                    spacing: 8
                    Repeater { id: chipsRep; model: pd.p ? pd.p.items : []; delegate: Chip { required property string modelData; bid: modelData; square: pd.down; from: ({ kind: "panel", id: pd.modelData }) } }
                    Rectangle {
                        width: pd.down ? parent.width : 34; height: pd.down ? 34 : parent.height
                        radius: 8; color: "transparent"; border.color: Theme.muted; border.width: 1
                        Text { anchors.centerIn: parent; text: "+"; color: Theme.muted; font.pixelSize: 16 }
                        MouseArea { anchors.fill: parent; onClicked: { ed.libKind = "all"; ed.note = "Drag a block from Blocks onto this panel"; } }
                    }
                }
                // its ends (length) and its side (thickness)
                Handle { x: pd.down ? pd.width / 2 - 6 : -6; y: pd.down ? -6 : pd.height / 2 - 6; mode: "len0"; what: "p"; tid: pd.modelData; cursor: pd.down ? Qt.SizeVerCursor : Qt.SizeHorCursor }
                Handle { x: pd.down ? pd.width / 2 - 6 : pd.width - 6; y: pd.down ? pd.height - 6 : pd.height / 2 - 6; mode: "len1"; what: "p"; tid: pd.modelData; cursor: pd.down ? Qt.SizeVerCursor : Qt.SizeHorCursor }
                Handle { x: pd.down ? pd.width - 6 : pd.width / 2 - 6; y: pd.down ? pd.height / 2 - 6 : pd.height - 6; mode: "thick"; what: "p"; tid: pd.modelData; cursor: pd.down ? Qt.SizeHorCursor : Qt.SizeVerCursor }
                Tag { text: pd.p ? E.panelLabel(pd.p, { w: pd.width, h: pd.height }) : ""; x: 0; y: pd.height + 10 > ed.sh - 40 ? -height - 10 : pd.height + 10 }
            }
        }

        // ---- widgets, free on the desktop
        Repeater {
            model: Object.keys(ed.draft.widgets)
            delegate: Item {
                id: wd
                required property string modelData
                readonly property var w: ed.draft.widgets[modelData] || { type: "", x: 0, y: 0, w: 100, h: 100 }
                x: w.x; y: w.y; width: w.w; height: w.h
                Rectangle { anchors.fill: parent; radius: 20; color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.85); border.color: Theme.line }
                Column {
                    anchors { left: parent.left; top: parent.top; margins: 18 }
                    spacing: 6
                    Icon { name: ed.iconOf("widget:" + wd.w.type); size: 22; color: Theme.accent }
                    Text { text: ed.nameOf("widget:" + wd.w.type); color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                }
                Dashes { anchors.fill: parent; anchors.margins: -3; lit: ed.picked === "w:" + wd.modelData }
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.SizeAllCursor
                    property bool off: false
                    onPressed: m => { off = false; ed.startMove("w", wd.modelData, "move", mapToItem(ed.contentItem, m.x, m.y)); }
                    onPositionChanged: m => {
                        const p = mapToItem(ed.contentItem, m.x, m.y);
                        // dragged onto a bar or a panel: it becomes that bar's block (or is refused there, with why)
                        if (!off && (ed.inBox(p, ed.mainBox, 4) || Object.keys(ed.draft.panels).some(k => ed.inBox(p, ed.panelRect(k), 4)))) {
                            off = true; ed.moving = null; ed.startDrag("widget:" + wd.w.type, { kind: "desk", id: wd.modelData }, p);
                        }
                        if (off) ed.moveDrag(p); else ed.doMove(p);
                    }
                    onReleased: { if (off) ed.endDrag(); else ed.endMove(); }
                }
                Handle { x: wd.width - 6; y: wd.height - 6; mode: "corner"; what: "w"; tid: wd.modelData; cursor: Qt.SizeFDiagCursor }
                Tag { text: E.widgetLabel(ed.nameOf("widget:" + wd.w.type), wd.w); x: 0; y: wd.height + 10 }
            }
        }

        // ---- an empty edge: drag here for a panel
        Repeater {
            model: ed.freeEdges
            delegate: Item {
                required property string modelData
                readonly property bool down: modelData === "left" || modelData === "right"
                width: down ? 40 : 300; height: down ? 220 : 40
                x: modelData === "left" ? 8 : modelData === "right" ? ed.sw - width - 8 : (ed.sw - width) / 2
                y: modelData === "top" ? 8 : modelData === "bottom" ? ed.sh - height - 100 : (ed.sh - height) / 2
                Rectangle { anchors.fill: parent; radius: 12; color: "transparent"; border.color: Qt.rgba(1, 1, 1, 0.35); border.width: 2 }
                Text { anchors.centerIn: parent; text: "+"; color: Qt.rgba(1, 1, 1, 0.6); font.pixelSize: 20 }
                MouseArea { anchors.fill: parent; hoverEnabled: true; onClicked: ed.addPanel(parent.modelData); onEntered: ed.note = "Drag here for a panel on the " + parent.modelData + " · or click"; onExited: ed.note = "" }
            }
        }

        // ---- snap guides, the drop line, the note
        Repeater {
            model: ed.guides
            delegate: Rectangle {
                required property var modelData
                color: "#5ec8ff"; opacity: 0.8
                x: modelData.x !== undefined ? modelData.x - 1 : 0; y: modelData.y !== undefined ? modelData.y - 1 : 0
                width: modelData.x !== undefined ? 2 : ed.sw; height: modelData.y !== undefined ? 2 : ed.sh
            }
        }
        Rectangle {
            visible: ed.lineX >= 0 && ed.dragId !== ""
            x: ed.lineX; y: ed.lineY
            width: ed.lineDown ? ed.lineLen : 3; height: ed.lineDown ? 3 : ed.lineLen
            radius: 2; color: Theme.accent
        }
        Rectangle {
            visible: ed.note !== ""
            x: Math.min(ed.sw - width - 12, ed.dragId !== "" ? ed.dragAt.x + 30 : ed.sw / 2 - width / 2)
            y: ed.dragId !== "" ? Math.min(ed.sh - 160, ed.dragAt.y + 40) : ed.sh - 150
            width: noteT.implicitWidth + 20; height: 32; radius: 8
            color: Qt.rgba(0.37, 0.78, 1, 0.16); border.color: "#5ec8ff"
            Text { id: noteT; anchors.centerIn: parent; text: ed.note; color: "#ffffff"; font.family: Theme.font; font.pixelSize: 13 }
        }

        // ---- Blocks: its own window, moved by its title
        Rectangle {
            id: libWin
            visible: !side.visible
            x: 150; y: 110; width: 380; height: Math.min(700, ed.sh - 260)
            radius: 14; color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.97); border.color: Theme.line
            clip: true
            Rectangle {
                id: libTitle
                width: parent.width; height: 44; color: "transparent"
                Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
                Row { anchors { left: parent.left; leftMargin: 16; verticalCenter: parent.verticalCenter } spacing: 10
                    Rectangle { width: 14; height: 14; radius: 7; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
                    Text { text: "Blocks"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold } }
                Text { anchors { right: parent.right; rightMargin: 16; verticalCenter: parent.verticalCenter } text: "drag onto anything"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                MouseArea { anchors.fill: parent; cursorShape: Qt.SizeAllCursor; drag.target: libWin; drag.minimumX: 0; drag.minimumY: 0; drag.maximumX: ed.sw - libWin.width; drag.maximumY: ed.sh - 80 }
            }
            Flickable {
                anchors { left: parent.left; right: parent.right; top: libTitle.bottom; bottom: parent.bottom; margins: 16 }
                contentHeight: libCol.implicitHeight
                clip: true
                interactive: ed.dragId === ""
                Column {
                    id: libCol
                    width: parent.width
                    spacing: 10
                    Rectangle {
                        width: parent.width; height: 36; radius: 10; color: Theme.bg; border.color: Theme.line
                        TextInput {
                            id: findIn
                            anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                            verticalAlignment: TextInput.AlignVCenter
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 13
                            onTextChanged: ed.libWords = text
                        }
                        Text { visible: findIn.text === ""; anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter } text: "Find a block (" + ed.lib.length + ")"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                    }
                    Flow {
                        width: parent.width; spacing: 6
                        Repeater {
                            model: [["all", "All"], ["bar", "Bar items"], ["widgets", "Widgets"], ["launchers", "Launchers"], ["info", "Info"], ["media", "Media"], ["own", "Your own"], ["ai", "Made by your AI"]]
                            delegate: Rectangle {
                                required property var modelData
                                readonly property bool on: ed.libKind === modelData[0]
                                width: kt.implicitWidth + 20; height: 28; radius: 8
                                color: on ? Theme.accent : "transparent"; border.color: on ? Theme.accent : Theme.line
                                Text { id: kt; anchors.centerIn: parent; text: parent.modelData[1]; color: parent.on ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: parent.on ? Font.DemiBold : Font.Normal }
                                MouseArea { anchors.fill: parent; onClicked: ed.libKind = parent.modelData[0] }
                            }
                        }
                    }
                    Section { title: "BAR ITEMS"; kinds: ["bar", "launchers", "info"] }
                    Section { title: "WIDGETS · ANYWHERE ON THE DESKTOP"; kinds: ["widgets", "media"] }
                    Section { title: "YOUR OWN"; kinds: ["own"] }
                    Text { visible: ed.libKind === "ai"; width: parent.width; wrapMode: Text.WordWrap; text: "Blocks your AI made with Describe it show here (Settings › Your desktop › Your blocks)."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    Text { text: "MAKE YOUR OWN"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 1.2; visible: ed.libKind === "all" || ed.libKind === "own" }
                    Grid {
                        visible: ed.libKind === "all" || ed.libKind === "own"
                        columns: 2; spacing: 8; width: parent.width
                        LibTile { label: "Button · runs anything"; icon: "rocket"; dashed: true; make: "button" }
                        LibTile { label: "Text · picture · spacer"; icon: "edit"; dashed: true; make: "text" }
                        LibTile { label: "Picture"; icon: "image"; dashed: true; make: "picture" }
                        LibTile { label: "Spacer"; icon: "dot"; dashed: true; make: "spacer" }
                    }
                }
            }
        }

        // ---- Layers: everything here, to pick (and take out)
        Rectangle {
            visible: ed.layersShown
            x: tool.x + layersBtn.x; y: tool.y - height - 10
            width: 300; height: Math.min(420, layersCol.implicitHeight + 24); radius: 12
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.98); border.color: Theme.line
            Column {
                id: layersCol
                anchors { fill: parent; margins: 12 }
                spacing: 4
                Repeater {
                    model: ["main"].concat(Object.keys(ed.draft.panels).map(k => "p:" + k)).concat(Object.keys(ed.draft.widgets).map(k => "w:" + k))
                    delegate: Rectangle {
                        required property string modelData
                        width: parent.width; height: 34; radius: 8
                        color: ed.picked === modelData ? Theme.panel3 : "transparent"
                        Text { anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter }
                               text: parent.modelData === "main" ? "Main bar" : parent.modelData.startsWith("p:") ? "Panel · " + (ed.draft.panels[parent.modelData.slice(2)].items || []).length + " blocks" : ed.nameOf("widget:" + (ed.draft.widgets[parent.modelData.slice(2)] || {}).type)
                               color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                        Text { visible: parent.modelData !== "main"; anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter } text: "Take out"; color: Theme.danger; font.family: Theme.font; font.pixelSize: 12
                               MouseArea { anchors.fill: parent; onClicked: ed.remove(parent.parent.modelData) } }
                        MouseArea { anchors { fill: parent; rightMargin: 80 } onClicked: ed.picked = parent.modelData }
                    }
                }
            }
        }
        // ---- Start from: the ready-made layouts
        Rectangle {
            visible: ed.startShown
            x: tool.x + startBtn.x; y: tool.y - height - 10
            width: 280; height: startCol.implicitHeight + 24; radius: 12
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.98); border.color: Theme.line
            Column {
                id: startCol
                anchors { fill: parent; margins: 12 }
                spacing: 4
                Text { width: parent.width; wrapMode: Text.WordWrap; text: "Puts that layout in place to start from. Cancel brings yours back."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                Repeater {
                    model: ed.layouts
                    delegate: Rectangle {
                        required property var modelData
                        width: parent.width; height: 32; radius: 8; color: sa.containsMouse ? Theme.panel3 : "transparent"
                        Text { anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter } text: parent.modelData.name || parent.modelData.id || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                        MouseArea { id: sa; anchors.fill: parent; hoverEnabled: true; onClicked: ed.startFrom(parent.modelData.id) }
                    }
                }
            }
        }

        // ---- the toolbar
        Text {
            visible: !side.visible
            anchors { horizontalCenter: parent.horizontalCenter; bottom: tool.top; bottomMargin: 12 }
            text: "EDITING YOUR DESKTOP · NOTHING IS KEPT UNTIL KEEP THIS · ESC CANCELS"
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 1.2
            Rectangle { z: -1; anchors { fill: parent; margins: -6 } radius: 6; color: Theme.panel2; border.color: Theme.line }
        }
        Rectangle {
            id: tool
            visible: !side.visible
            anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 28 }
            width: toolRow.implicitWidth + 20; height: 56; radius: 16
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.97); border.color: Theme.line
            Row {
                id: toolRow
                anchors.centerIn: parent
                spacing: 8
                Btn { label: "Undo"; icon: "undo"; enabled: ed.undoStack.length > 0; onClicked: ed.undo() }
                Btn { label: "Redo"; icon: "redo"; enabled: ed.redoStack.length > 0; onClicked: ed.redo() }
                Sep {}
                Btn { label: "Snap to grid" + (ed.snapOn ? " · " + ed.grid + " px" : ""); icon: "grid"; on: ed.snapOn; onClicked: ed.snapOn = !ed.snapOn }
                Btn { label: "Guides"; on: ed.guidesOn; onClicked: ed.guidesOn = !ed.guidesOn }
                Btn { id: layersBtn; label: "Layers"; on: ed.layersShown; onClicked: { ed.layersShown = !ed.layersShown; ed.startShown = false; } }
                Btn { label: "Screen: " + ed.scrName + (Quickshell.screens.length > 1 ? " ▾" : ""); icon: "monitor"; onClicked: if (Quickshell.screens.length > 1) { ed.scr = (ed.scr + 1) % Quickshell.screens.length; ed.load(); } }
                Sep {}
                Btn { label: "Panel"; icon: "plus"; onClicked: ed.addPanel("") }
                Btn { id: startBtn; label: "Start from: " + "a ready-made layout ▾"; onClicked: { ed.startShown = !ed.startShown; ed.layersShown = false; if (ed.startShown) lister.running = true; } }
                Sep {}
                Btn { label: "Cancel"; onClicked: ed.cancel() }
                Btn { label: "Try it"; onClicked: ed.tryIt() }
                Btn { label: "Keep this"; primary: true; onClicked: ed.keep() }
            }
        }

        // ================================================================== board 2: one block picked
        // ---- "Pick the bar itself (click its empty space) and it has its own:" (beside a picked block, as on the board)
        Rectangle {
            id: barHint
            visible: side.forBlock && !ed.trying
            x: 150; y: 300
            width: 500; height: hintCol.implicitHeight + 20
            radius: 12
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.97); border.color: Theme.line
            Column {
                id: hintCol
                anchors { left: parent.left; right: parent.right; top: parent.top }
                Item {
                    width: parent.width; height: 44
                    Text { anchors { left: parent.left; leftMargin: 16; verticalCenter: parent.verticalCenter } text: "Pick the bar itself (click its empty space) and it has its own:"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
                }
                Column {
                    x: 20; topPadding: 8
                    spacing: 6
                    Text { text: "Edge: Top · Bottom · Left · Right · <font color=\"" + Theme.accent + "\"><b>Floating anywhere</b></font>"; textFormat: Text.StyledText; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                    Text { text: "Length: full, fit to its blocks, or any % or px · Offset from the corner"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                    Text { text: "Thickness · Corners · Fill, see-through, blur · Gap between blocks"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                    Text { text: "Line up blocks: start · centre · end · spread · Islands or one strip"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                    Text { text: "Hide until pointed at · Above windows or keep their space · Which screen"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                    Text { text: "More than one panel on any edge, panels inside panels"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                }
            }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { const f = ed.blk ? ed.blk.from : null; ed.blk = null; ed.picked = f && f.kind === "panel" ? "p:" + f.id : "main"; } }
        }

        // ---- right-click any block
        Rectangle {
            id: menuCard
            visible: ed.menuBlk !== null && !ed.trying
            x: Math.max(8, Math.min(ed.sw - width - 12, ed.menuAt.x))
            y: Math.max(8, Math.min(ed.sh - height - 12, ed.menuAt.y + 14))
            width: 250; height: menuCol.implicitHeight + 16
            radius: 12
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.98); border.color: Theme.line
            readonly property bool hid: ed.menuBlk !== null && BL.normalize(ed.draft.looks ? ed.draft.looks[ed.menuBlk.id] : null).hidden
            MouseArea { anchors.fill: parent }   // its gaps aren't the desktop
            Column {
                id: menuCol
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 8 }
                MenuRow { label: "Customize…"; onClicked: { const b = ed.menuBlk; const it = ed.chipFor(b.id); ed.pickBlock(b.id, b.from, it ? it.width : 0, it ? it.height : 0); ed.tab = "look"; } }
                MenuRow { label: "Copy its look"; onClicked: { ed.copyLookOf(ed.menuBlk.id); ed.menuBlk = null; } }
                MenuRow { label: "Paste a look"; dim: ed.lookClip === null; onClicked: { ed.pasteLookOn(ed.menuBlk.id); ed.menuBlk = null; } }
                MenuRow { label: "Duplicate"; onClicked: { ed.duplicateBlock(ed.menuBlk); ed.menuBlk = null; } }
                Item {
                    width: parent.width; height: Math.max(32, moveFlow.implicitHeight + 12)
                    Text { id: moveT; x: 8; anchors.verticalCenter: parent.verticalCenter; text: "Move to ›"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                    Flow {
                        id: moveFlow
                        anchors { left: moveT.right; leftMargin: 6; right: parent.right; verticalCenter: parent.verticalCenter }
                        spacing: 4
                        Repeater {
                            model: ed.menuBlk ? E.moveTargets(ed.draft, ed.menuBlk.id, ed.menuBlk.from, ed.mbl.edge === "free" ? "free" : ed.mainEdge, L.info) : []
                            delegate: Text {
                                required property var modelData
                                required property int index
                                text: (index > 0 ? "· " : "") + modelData.label
                                color: modelData.ok ? (mtA.containsMouse ? Theme.accent : Theme.text) : Theme.muted
                                font.family: Theme.font; font.pixelSize: 13
                                MouseArea { id: mtA; anchors.fill: parent; hoverEnabled: true; cursorShape: parent.modelData.ok ? Qt.PointingHandCursor : Qt.ArrowCursor
                                    onClicked: { if (parent.modelData.ok) ed.moveBlock(ed.menuBlk, parent.modelData.dest); else ed.say(E.fit(ed.menuBlk.id, parent.modelData.dest, L.info).why); } }
                            }
                        }
                    }
                }
                MenuRow { label: menuCard.hid ? "Show it again" : "Hide (keep it here)"; onClicked: { ed.hideBlock(ed.menuBlk); ed.menuBlk = null; } }
                MenuRow { label: "Take it out"; danger: true; onClicked: ed.takeOutBlock(ed.menuBlk) }
            }
        }
        // the note beside the side panel
        Rectangle {
            visible: side.forBlock && !ed.trying
            x: side.x - width - 2; y: side.y + 338
            width: hintT.implicitWidth + 22; height: 30; radius: 8
            color: Qt.rgba(0.37, 0.78, 1, 0.12); border.color: "#5ec8ff"
            Text { id: hintT; anchors.centerIn: parent; text: "Right-click any block · or pick it and use the panel →"; color: "#ffffff"; font.family: Theme.font; font.pixelSize: 13 }
        }

        // ---- the side panel: everything about the block picked, or the bar's own settings (its own window: move it by its title)
        Rectangle {
            id: side
            readonly property bool forBlock: ed.blk !== null
            readonly property bool forBar: !forBlock && ed.barNow !== null
            visible: (forBlock || forBar) && !ed.trying
            x: ed.sw - width - 24; y: 62
            width: 560; height: Math.max(400, ed.sh - 62 - 22)
            radius: 16
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.985); border.color: Theme.line
            clip: true
            MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons; onWheel: w => w.accepted = true }   // nothing behind it is clicked
            Item {
                id: sideHead
                width: parent.width; height: 46
                MouseArea { anchors.fill: parent; cursorShape: Qt.SizeAllCursor; drag.target: side; drag.minimumX: 0; drag.minimumY: 0; drag.maximumX: ed.sw - side.width; drag.maximumY: ed.sh - 80 }
                Row {
                    anchors { left: parent.left; leftMargin: 16; verticalCenter: parent.verticalCenter }
                    spacing: 10
                    Rectangle { width: 16; height: 16; radius: 4; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
                    Text { text: side.forBlock ? ed.nameOf(ed.blk.id) : ed.barTitle(); color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter }
                    Text { text: "· " + (side.forBlock ? E.placeName(ed.draft, ed.blk.from, ed.mbl.edge === "free" ? "free" : ed.mainEdge) : ed.barWhere()); color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
                }
                SmallBtn { anchors { right: parent.right; rightMargin: 14; verticalCenter: parent.verticalCenter } label: "Reset"; onClicked: { if (side.forBlock) ed.resetLook(); else ed.resetBar(); } }
                Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
            }
            Row {
                id: tabs
                visible: side.forBlock
                anchors { left: parent.left; leftMargin: 20; top: sideHead.bottom; topMargin: 16 }
                spacing: 6
                Repeater {
                    model: [["look", "Look"], ["shows", "Shows"], ["does", "Does"], ["place", "Place"], ["when", "When"], ["motion", "Motion"]]
                    delegate: Rectangle {
                        required property var modelData
                        readonly property bool on: ed.tab === modelData[0]
                        width: tt2.implicitWidth + 24; height: 30; radius: 8
                        color: on ? Theme.accent : "transparent"; border.color: on ? Theme.accent : Theme.line
                        Text { id: tt2; anchors.centerIn: parent; text: parent.modelData[1]; color: parent.on ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 13; font.weight: parent.on ? Font.DemiBold : Font.Normal }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { ed.tab = parent.modelData[0]; ed.popup = ""; } }
                    }
                }
            }
            Flickable {
                id: sideFlick
                anchors { left: parent.left; right: parent.right; top: side.forBlock ? tabs.bottom : sideHead.bottom; bottom: sideBtns.top; leftMargin: 20; rightMargin: 20; topMargin: 14; bottomMargin: 10 }
                contentHeight: sideCol.implicitHeight + 10
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                Column {
                    id: sideCol
                    width: sideFlick.width
                    spacing: 4

                    // ======== Look
                    Column {
                        visible: side.forBlock && ed.tab === "look"
                        width: parent.width
                        spacing: 4
                        Sec { text: "SIZE AND PLACE" }
                        LRow { label: "Width"
                            Seg { options: [["fit", "Fit"], ["fixed", "Fixed"], ["fill", "Fill"]]; value: ed.cur.w
                                  onPicked: v => ed.setLook(v === "fixed" ? { w: v, wpx: ed.cur.wpx > 0 ? ed.cur.wpx : Math.max(40, ed.blk ? ed.blk.w : 120) } : { w: v }, "w") }
                            NumBox { value: ed.cur.wpx > 0 ? ed.cur.wpx : (ed.blk ? ed.blk.w : 0); onTyped: v => ed.setLook({ w: "fixed", wpx: v }, "wpx") }
                        }
                        LRow { label: "Height"
                            NumBox { value: ed.cur.h > 0 ? ed.cur.h : (ed.blk ? ed.blk.h : 0); onTyped: v => ed.setLook({ h: v }, "h") }
                        }
                        LRow { label: "Space around it"
                            NumBox { width: 66; prefix: "↑ "; suffix: ""; hi: 200; value: ed.cur.mt; onTyped: v => ed.setLook({ mt: v }, "mt") }
                            NumBox { width: 66; prefix: "→ "; suffix: ""; hi: 200; value: ed.cur.mr; onTyped: v => ed.setLook({ mr: v }, "mr") }
                            NumBox { width: 66; prefix: "↓ "; suffix: ""; hi: 200; value: ed.cur.mb; onTyped: v => ed.setLook({ mb: v }, "mb") }
                            NumBox { width: 66; prefix: "← "; suffix: ""; hi: 200; value: ed.cur.ml; onTyped: v => ed.setLook({ ml: v }, "ml") }
                        }
                        LRow { label: "Space inside"
                            Slide { from: -1; to: 40; value: ed.cur.pad; onMoved: v => ed.setLook({ pad: v }, "pad") }
                            Val { text: ed.cur.pad < 0 ? "its own" : ed.cur.pad + " px" }
                        }
                        Sec { text: "BACKGROUND AND EDGE" }
                        LRow { label: "Fill"
                            Swatches { value: ed.cur.fill; onPicked: h => ed.setLook({ fill: h }, "fill") }
                            Hex { value: ed.cur.fill; onPicked: h => ed.setLook({ fill: h }, "fill") }
                        }
                        LRow { label: "See-through"
                            Slide { from: 0; to: 100; value: ed.cur.see; onMoved: v => ed.setLook({ see: v }, "see") }
                            Val { text: ed.cur.see + " %" }
                        }
                        LRow { label: "Blur behind it"
                            Toggle { on: ed.cur.blur; onToggled: ed.setLook({ blur: !ed.cur.blur }, "blur") }
                        }
                        LRow { label: "Corners"
                            Slide { width: 180; from: -1; to: 21; value: ed.cur.corners >= 99 ? 21 : ed.cur.corners; onMoved: v => ed.setLook({ corners: v >= 21 ? 99 : v }, "corners") }
                            Val { text: BL.cornersLabel(ed.cur.corners) }
                        }
                        LRow { label: "Edge"
                            Seg { options: [["none", "None"], ["line", "Line"], ["glow", "Glow"]]; value: ed.cur.edge; onPicked: v => ed.setLook({ edge: v }, "edge") }
                            ColourDot { value: ed.cur.edgeColor; key: "edgeColor" }
                            NumBox { value: ed.cur.edgeW; lo: 1; hi: 12; onTyped: v => ed.setLook({ edgeW: v }, "edgeW") }
                        }
                        LRow { label: "Shadow"
                            Seg { options: [["none", "None"], ["soft", "Soft"], ["deep", "Deep"]]; value: ed.cur.shadow; onPicked: v => ed.setLook({ shadow: v }, "shadow") }
                        }
                        Sec { text: "ICON AND WORDS" }
                        Text { visible: !BL.simple(ed.curBase); width: parent.width; wrapMode: Text.WordWrap; text: BL.why(ed.curBase); color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                        LRow { label: "Icon"
                            PickBtn { icon: ed.cur.icon !== "" ? ed.cur.icon : ed.iconOf(ed.blk ? ed.blk.id : ""); label: ed.iconLabel(ed.cur.icon !== "" ? ed.cur.icon : ed.iconOf(ed.blk ? ed.blk.id : "")); onClicked: ed.popup = ed.popup === "icon" ? "" : "icon" }
                            ColourDot { value: ed.cur.iconColor; key: "iconColor" }
                            NumBox { value: ed.cur.iconSize > 0 ? ed.cur.iconSize : 16; lo: 8; hi: 128; onTyped: v => ed.setLook({ iconSize: v }, "iconSize") }
                        }
                        LRow { label: "Words"
                            TextBox { width: 220; text: ed.cur.words; placeholder: ed.wordsHint(); onDone: t => ed.setLook({ words: t }, "words") }
                        }
                        Text { visible: BL.hasValue(ed.curBase); width: parent.width; horizontalAlignment: Text.AlignRight; text: "It can show: " + BL.valueKeys(ed.curBase).map(k => "{" + k + "}").join("  "); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                        LRow { label: "Font"
                            PickBtn { label: (ed.cur.font !== "" ? ed.cur.font : "The look's"); onClicked: ed.popup = ed.popup === "font" ? "" : "font" }
                            NumBox { value: ed.cur.fontSize > 0 ? ed.cur.fontSize : 13; suffix: ""; lo: 6; hi: 96; onTyped: v => ed.setLook({ fontSize: v }, "fontSize") }
                            Seg { options: [["light", "Light"], ["medium", "Medium"], ["bold", "Bold"]]; value: ed.cur.weight; onPicked: v => ed.setLook({ weight: v }, "weight") }
                        }
                        LRow { label: "Show"
                            Seg { options: [["icon", "Icon"], ["words", "Words"], ["both", "Both"]]; value: ed.cur.show; onPicked: v => ed.setLook({ show: v }, "show") }
                        }
                        Sec { text: "WHEN IT'S POINTED AT · PRESSED" }
                        LRow { label: "Hover look"
                            Seg { options: [["same", "Same"], ["lighter", "Lighter"], ["accent", "Accent"], ["own", "Own…"]]; value: ed.cur.hover
                                  onPicked: v => { ed.setLook({ hover: v }, "hover"); if (v === "own") ed.popup = "colour:hoverOwn"; } }
                        }
                        LRow { label: "Pressed look"
                            Seg { options: [["same", "Same"], ["darker", "Darker"], ["own", "Own…"]]; value: ed.cur.pressed
                                  onPicked: v => { ed.setLook({ pressed: v }, "pressed"); if (v === "own") ed.popup = "colour:pressedOwn"; } }
                        }
                        Sec { text: "THE OTHER TABS" }
                        Text {
                            width: parent.width; wrapMode: Text.WordWrap; lineHeight: 1.25
                            text: "Shows: what's in it (a count, a list, a picture, a clock…) · Does: press, right-click, middle-click, scroll, hover — open, run, show a panel, nothing · Place: which bar, panel or the desktop, which screen, which desktops, above or under windows · When: always, only on hover, only when there's something to show, only in some apps · Motion: how it appears, bounces when something new comes"
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                        }
                    }

                    // ======== Shows: what's in it
                    Column {
                        visible: side.forBlock && ed.tab === "shows"
                        width: parent.width
                        spacing: 6
                        Sec { text: "WHAT'S IN IT" }
                        Choices { options: [["own", "Its own (what Hearth shows)"], ["count", "A count"], ["list", "A list"], ["picture", "A picture"], ["clock", "A clock"], ["words", "Words"]]; value: ed.cur.shows; onPicked: v => ed.setLook({ shows: v }, "shows") }
                        Text { width: parent.width; wrapMode: Text.WordWrap; text: BL.hasValue(ed.curBase) ? "Its count and values: " + BL.valueKeys(ed.curBase).map(k => "{" + k + "}").join("  ") : "It has no count of its own: a count shows what you write, with {time} and {date}"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                        LRow { visible: ed.cur.shows === "count" || ed.cur.shows === "words"; label: ed.cur.shows === "count" ? "Words with the count" : "Words"
                            TextBox { width: 260; text: ed.cur.words; placeholder: ed.wordsHint(); onDone: t => ed.setLook({ words: t }, "words") } }
                        Column {
                            visible: ed.cur.shows === "list"
                            width: parent.width; spacing: 4
                            Text { text: "Its lines, one on each line"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                            Rectangle {
                                width: parent.width; height: 110; radius: 8; color: Theme.bg; border.color: listEd.activeFocus ? Theme.accent : Theme.line
                                TextEdit {
                                    id: listEd
                                    anchors { fill: parent; margins: 10 }
                                    text: ed.cur.list; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; wrapMode: TextEdit.Wrap; selectByMouse: true
                                    onActiveFocusChanged: if (!activeFocus && text !== ed.cur.list) ed.setLook({ list: text }, "list")
                                }
                            }
                        }
                        LRow { visible: ed.cur.shows === "picture"; label: "Its picture (a file)"
                            TextBox { width: 280; text: ed.cur.pic; placeholder: "/home/you/Pictures/it.png"; onDone: t => ed.setLook({ pic: t.trim() }, "pic") } }
                        LRow { visible: ed.cur.shows === "clock"; label: "Its format"
                            TextBox { width: 200; text: ed.cur.clock; placeholder: Settings.clock24 ? "HH:mm" : "h:mm AP"; onDone: t => ed.setLook({ clock: t }, "clock") } }
                    }

                    // ======== Does: press, right-click, middle-click, scroll, hover
                    Column {
                        visible: side.forBlock && ed.tab === "does"
                        width: parent.width
                        spacing: 6
                        Text { visible: ed.blk !== null && E.isOwn(ed.blk.id); width: parent.width; wrapMode: Text.WordWrap; text: "Your own button: \"Its own\" runs what you gave it."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                        Repeater {
                            model: [["Press", "PRESS"], ["Right", "RIGHT-CLICK"], ["Middle", "MIDDLE-CLICK"], ["Scroll", "SCROLL"], ["Hover", "HOVER (POINTED AT A MOMENT)"]]
                            delegate: Column {
                                id: dk
                                required property var modelData
                                readonly property string k: modelData[0]
                                readonly property string kind: ed.cur["do" + k]
                                readonly property string arg: ed.cur["arg" + k]
                                width: sideCol.width
                                spacing: 6
                                Sec { text: dk.modelData[1] }
                                Choices { options: [["default", "Its own"], ["app", "Open an app"], ["run", "Run a command"], ["panel", "Show a panel"], ["nothing", "Nothing"]]; value: dk.kind
                                          onPicked: v => { const o = {}; o["do" + dk.k] = v; ed.setLook(o, "do" + dk.k); } }
                                TextBox {
                                    visible: dk.kind === "app" || dk.kind === "run"
                                    width: parent.width
                                    text: dk.arg
                                    placeholder: dk.kind === "app" ? "The app (type its name, then pick it below)" : "A command, as you'd type it"
                                    onDone: t => { const o = {}; o["arg" + dk.k] = t.trim(); ed.setLook(o, "arg" + dk.k); }
                                    onTyping: t => dk.typed = t
                                }
                                property string typed: ""
                                Flow {
                                    visible: dk.kind === "app"
                                    width: parent.width; spacing: 6
                                    Repeater {
                                        model: dk.kind === "app" ? ed.appsLike(dk.typed !== "" ? dk.typed : dk.arg) : []
                                        delegate: Rectangle {
                                            required property var modelData
                                            readonly property bool on: dk.arg === modelData.id
                                            width: an.implicitWidth + 20; height: 28; radius: 8
                                            color: on ? Theme.accent : Theme.panel2; border.color: Theme.line
                                            Text { id: an; anchors.centerIn: parent; text: parent.modelData.name; color: parent.on ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 12 }
                                            MouseArea { anchors.fill: parent; onClicked: { const o = {}; o["arg" + dk.k] = parent.modelData.id; ed.setLook(o, "arg" + dk.k); } }
                                        }
                                    }
                                }
                                Choices {
                                    visible: dk.kind === "panel"
                                    options: BL.PANELS; value: dk.arg
                                    onPicked: v => { const o = {}; o["arg" + dk.k] = v; ed.setLook(o, "arg" + dk.k); }
                                }
                            }
                        }
                    }

                    // ======== Place: which bar, panel or the desktop, which screen, which desktops, above or under windows
                    Column {
                        visible: side.forBlock && ed.tab === "place"
                        width: parent.width
                        spacing: 6
                        Sec { text: "WHERE IT IS" }
                        Flow {
                            width: parent.width; spacing: 6
                            Chipish { label: ed.blk ? E.placeName(ed.draft, ed.blk.from, ed.mbl.edge === "free" ? "free" : ed.mainEdge).replace(/^in /, "") : ""; on: true }
                            Repeater {
                                model: ed.blk ? E.moveTargets(ed.draft, ed.blk.id, ed.blk.from, ed.mbl.edge === "free" ? "free" : ed.mainEdge, L.info) : []
                                delegate: Chipish { required property var modelData; label: modelData.label; dim: !modelData.ok
                                    onClicked: { if (modelData.ok) ed.moveBlock(ed.blk, modelData.dest); else ed.say(E.fit(ed.blk.id, modelData.dest, L.info).why); } }
                            }
                        }
                        Sec { text: "WHICH SCREEN" }
                        Choices { options: [["", "Every screen"]].concat(Quickshell.screens.map(s => [s.name, s.name])); value: ed.cur.screen; onPicked: v => ed.setLook({ screen: v }, "screen") }
                        Sec { text: "WHICH DESKTOPS" }
                        Flow {
                            width: parent.width; spacing: 6
                            Chipish { label: "Every desktop"; on: ed.cur.desktops.length === 0; onClicked: ed.setLook({ desktops: [] }, "desktops") }
                            Repeater {
                                model: Math.max(1, DeskState.deskCount)
                                delegate: Chipish {
                                    required property int index
                                    label: "Desktop " + (index + 1)
                                    on: ed.cur.desktops.indexOf(index + 1) >= 0
                                    onClicked: { const l = ed.cur.desktops.slice(); const i = l.indexOf(index + 1); if (i >= 0) l.splice(i, 1); else l.push(index + 1); ed.setLook({ desktops: l.sort((a, b) => a - b) }, "desktops"); }
                                }
                            }
                        }
                        Sec { text: "ABOVE OR UNDER WINDOWS (THE WHOLE BAR IT'S IN)" }
                        Choices { options: [["top", "Above windows"], ["bottom", "Under windows"]]; value: ed.blkLayer(); onPicked: v => ed.setBlkLayer(v) }
                    }

                    // ======== When: always, only on hover, only when there's something to show, only in some apps
                    Column {
                        visible: side.forBlock && ed.tab === "when"
                        width: parent.width
                        spacing: 6
                        Sec { text: "WHEN IT SHOWS" }
                        Choices { options: [["always", "Always"], ["hover", "Only on hover (pointed at)"], ["something", "Only when there's something to show"], ["apps", "Only in some apps"]]; value: ed.cur.when; onPicked: v => ed.setLook({ when: v }, "when") }
                        Text { visible: ed.cur.when === "something" && !BL.hasValue(ed.curBase); width: parent.width; wrapMode: Text.WordWrap; text: "It always has something to show, so it always shows."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                        LRow { visible: ed.cur.when === "apps"; label: "The apps (their ids, with commas)"
                            TextBox { width: 260; text: ed.cur.apps; placeholder: "firefox, org.kde.dolphin"; onDone: t => ed.setLook({ apps: t }, "apps") } }
                        Text { visible: ed.cur.when === "apps"; width: parent.width; wrapMode: Text.WordWrap; text: "In front now: " + (WinList.activeToplevel ? (WinList.activeToplevel.appId || "—") : "—"); color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    }

                    // ======== Motion: how it appears, bounces when something new comes
                    Column {
                        visible: side.forBlock && ed.tab === "motion"
                        width: parent.width
                        spacing: 6
                        Sec { text: "HOW IT APPEARS" }
                        Choices { options: [["none", "At once"], ["fade", "Fades in"], ["slide", "Slides in"], ["grow", "Grows"]]; value: ed.cur.appear; onPicked: v => ed.setLook({ appear: v }, "appear") }
                        LRow { label: "Bounces when something new comes"
                            Toggle { on: ed.cur.bounce; onToggled: ed.setLook({ bounce: !ed.cur.bounce }, "bounce") } }
                        Text { visible: !BL.hasValue(ed.curBase); width: parent.width; wrapMode: Text.WordWrap; text: "It bounces when its count goes up; this block has no count of its own."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                        Text { visible: Settings.reduceMotion; width: parent.width; wrapMode: Text.WordWrap; text: "Fewer animations is on (Settings › Accessibility), so it doesn't move."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    }

                    // ======== the bar itself: its own settings
                    Column {
                        visible: side.forBar
                        width: parent.width
                        spacing: 4
                        Sec { text: "WHERE IT IS" }
                        Text { visible: ed.barInside; width: parent.width; wrapMode: Text.WordWrap; text: "It's inside another bar or panel: it takes its place among the blocks there."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                        LRow { visible: !ed.barInside; label: "Edge"
                            Seg { options: [["top", "Top"], ["bottom", "Bottom"], ["left", "Left"], ["right", "Right"], ["free", "Floating anywhere"]]; value: ed.barEdgeNow(); onPicked: v => ed.setBarEdge(v) } }
                        LRow { label: "Length"
                            Seg { options: ed.barNow && ed.barPicked !== "main" && ed.barNow.free ? [["fit", "Fit to its blocks"], ["px", "px"]] : [["full", "Full"], ["fit", "Fit to its blocks"], ["pct", "%"], ["px", "px"]]
                                  value: ed.barLenNow(); onPicked: v => ed.setBarLen(v) }
                            NumBox { value: ed.barLenV(); suffix: ed.barLenNow() === "pct" ? " %" : " px"; lo: 1; hi: 8000; onTyped: v => ed.setBarLenV(v) }
                        }
                        LRow { visible: !ed.barInside && !(ed.barNow && ed.barNow.free) && !(ed.barPicked === "main" && ed.mbl.edge === "free"); label: "Offset from the corner"
                            Seg { options: [["corner", "From the corner"], ["centre", "Centred"]]; value: ed.barNow && ed.barNow.offset >= 0 ? "corner" : "centre"; onPicked: v => ed.setBarKey("offset", v === "corner" ? 0 : -1) }
                            NumBox { value: ed.barNow ? Math.max(0, ed.barNow.offset) : 0; onTyped: v => ed.setBarKey("offset", v) }
                        }
                        Sec { text: "ITS LOOK" }
                        LRow { label: "Thickness"
                            NumBox { value: ed.barPicked === "main" ? ed.mainSize : ed.barNow ? ed.barNow.size : 44; lo: 20; hi: 120; onTyped: v => { if (ed.barPicked === "main") ed.setBarKey("thick", v); else ed.setBarKey("size", Math.max(28, Math.min(96, v))); } } }
                        LRow { label: "Corners"
                            Slide { width: 180; from: -1; to: 21; value: ed.barNow ? (ed.barNow.corners >= 99 ? 21 : ed.barNow.corners) : -1; onMoved: v => ed.setBarKey("corners", v >= 21 ? 99 : v) }
                            Val { text: BL.cornersLabel(ed.barNow ? ed.barNow.corners : -1) } }
                        LRow { label: "Fill"
                            Swatches { value: ed.barNow ? ed.barNow.fill : ""; onPicked: h => ed.setBarKey("fill", h) }
                            Hex { value: ed.barNow ? ed.barNow.fill : ""; onPicked: h => ed.setBarKey("fill", h) } }
                        LRow { label: "See-through"
                            Slide { from: 0; to: 100; value: ed.barNow ? ed.barNow.see : 0; onMoved: v => ed.setBarKey("see", v) }
                            Val { text: (ed.barNow ? ed.barNow.see : 0) + " %" } }
                        LRow { label: "Blur behind it"
                            Toggle { on: !!ed.barNow && ed.barNow.blur; onToggled: ed.setBarKey("blur", !(ed.barNow && ed.barNow.blur)) } }
                        LRow { label: "Gap between blocks"
                            Slide { from: -1; to: 40; value: ed.barNow ? ed.barNow.gap : -1; onMoved: v => ed.setBarKey("gap", v) }
                            Val { text: ed.barNow && ed.barNow.gap >= 0 ? ed.barNow.gap + " px" : "its own" } }
                        Sec { text: "ITS BLOCKS" }
                        LRow { label: "Line up blocks"
                            Seg { options: [["start", "Start"], ["centre", "Centre"], ["end", "End"], ["spread", "Spread"]]; value: ed.barNow ? ed.barNow.lineup : "spread"; onPicked: v => ed.setBarKey("lineup", v) } }
                        LRow { label: "Islands or one strip"
                            Seg { options: [["islands", "Islands"], ["strip", "One strip"]]; value: ed.barIslands(); onPicked: v => { if (ed.barPicked === "main") ed.setBarKey("islands", v); else ed.setBarKey("islands", v === "islands"); } } }
                        Sec { text: "WINDOWS AND SCREENS" }
                        LRow { visible: !ed.barInside; label: "Hide until pointed at"
                            Toggle { on: ed.barHides(); onToggled: { if (ed.barPicked === "main") ed.setBarKey("autoHide", ed.barHides() ? "off" : "on"); else ed.setBarKey("autoHide", !ed.barHides()); } } }
                        LRow { visible: !ed.barInside; label: "Windows"
                            Seg { options: [["over", "Above windows"], ["keep", "Keep their space"]]; value: ed.barNow && ed.barNow.over ? "over" : "keep"; onPicked: v => ed.setBarKey("over", v === "over") } }
                        LRow { visible: !ed.barInside; label: "Above or under windows"
                            Seg { options: [["top", "Above"], ["bottom", "Under"]]; value: ed.barNow ? ed.barNow.layer : "top"; onPicked: v => ed.setBarKey("layer", v) } }
                        LRow { visible: !ed.barInside; label: "Which screen"
                            Seg { options: [["", "Every screen"]].concat(Quickshell.screens.map(s => [s.name, s.name])); value: ed.barScreen(); onPicked: v => ed.setBarKey("screen", v) } }
                        Sec { text: "MORE PANELS" }
                        Flow {
                            width: parent.width; spacing: 8
                            SmallBtn { label: "+ Another panel on this edge"; onClicked: ed.anotherOnEdge() }
                            SmallBtn { label: "+ A panel inside this one"; onClicked: ed.innerPanel() }
                        }
                        Text { width: parent.width; wrapMode: Text.WordWrap; text: "More than one panel on any edge, panels inside panels: each has all of these settings."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    }
                }
            }
            Row {
                id: sideBtns
                visible: side.forBlock
                anchors { left: parent.left; leftMargin: 20; bottom: parent.bottom; bottomMargin: 20 }
                height: side.forBlock ? 38 : 0
                spacing: 8
                Btn { label: "Save as my own block…"; onClicked: ed.popup = ed.popup === "saveas" ? "" : "saveas" }
                Btn { label: "Copy its look"; onClicked: ed.copyLookOf(ed.blk.id) }
                Btn { label: "Apply to every bar item"; onClicked: ed.applyAll() }
            }
        }

        // ---- the side panel's pickers: an icon, a font, a colour, a name for your own block
        Rectangle {
            id: pop
            visible: ed.popup !== "" && side.visible
            x: Math.max(8, side.x - width - 12); y: Math.min(ed.sh - height - 20, side.y + 120)
            width: ed.popup === "icon" ? 420 : ed.popup === "font" ? 300 : 300
            height: ed.popup === "icon" ? 460 : ed.popup === "font" ? 420 : popCol.implicitHeight + 28
            radius: 14
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.99); border.color: Theme.line
            clip: true
            MouseArea { anchors.fill: parent; onWheel: w => w.accepted = true }
            Column {
                id: popCol
                anchors { fill: parent; margins: 14 }
                spacing: 10
                Row {
                    width: parent.width
                    Text { width: parent.width - 30; text: ed.popup === "icon" ? "Its icon" : ed.popup === "font" ? "Its font" : ed.popup === "saveas" ? "Save as my own block" : "A colour"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                    Text { text: "✕"; color: Theme.muted; font.pixelSize: 14; MouseArea { anchors.fill: parent; anchors.margins: -6; onClicked: ed.popup = "" } }
                }
                // icon: every one of Mantel's, or a picture file or an icon theme's name
                TextBox { visible: ed.popup === "icon"; width: parent.width; text: ""; placeholder: "Or a picture file, or an icon's name"; onDone: t => { if (t.trim() !== "") ed.setLook({ icon: t.trim() }, "icon"); } }
                Flickable {
                    visible: ed.popup === "icon"
                    width: parent.width; height: 370
                    contentHeight: iconGrid.implicitHeight
                    clip: true
                    Flow {
                        id: iconGrid
                        width: parent.width
                        spacing: 6
                        Rectangle {
                            width: 44; height: 44; radius: 8
                            color: ed.cur.icon === "" ? Theme.accent : Theme.panel2; border.color: Theme.line
                            Text { anchors.centerIn: parent; text: "Its\nown"; horizontalAlignment: Text.AlignHCenter; color: ed.cur.icon === "" ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 10 }
                            MouseArea { anchors.fill: parent; onClicked: ed.setLook({ icon: "" }, "icon") }
                        }
                        Repeater {
                            model: Object.keys(IconData.icons)
                            delegate: Rectangle {
                                required property string modelData
                                width: 44; height: 44; radius: 8
                                color: ed.cur.icon === modelData ? Theme.accent : ia.containsMouse ? Theme.panel3 : Theme.panel2
                                border.color: Theme.line
                                Icon { anchors.centerIn: parent; name: parent.modelData; size: 20; color: ed.cur.icon === parent.modelData ? Theme.accentInk : Theme.text }
                                MouseArea { id: ia; anchors.fill: parent; hoverEnabled: true; onClicked: ed.setLook({ icon: parent.modelData }, "icon") }
                            }
                        }
                    }
                }
                // font: every font on this PC
                TextBox { id: fontFind; visible: ed.popup === "font"; width: parent.width; text: ""; placeholder: "Find a font"; onTyping: t => ed.fontWords = t }
                ListView {
                    visible: ed.popup === "font"
                    width: parent.width; height: 320
                    clip: true
                    model: ed.popup === "font" ? [""].concat(Qt.fontFamilies().filter(f => ed.fontWords === "" || f.toLowerCase().indexOf(ed.fontWords.toLowerCase()) >= 0)).slice(0, 400) : []
                    delegate: Rectangle {
                        required property string modelData
                        width: ListView.view ? ListView.view.width : 260; height: 30; radius: 6
                        color: ed.cur.font === modelData ? Theme.accent : fa.containsMouse ? Theme.panel3 : "transparent"
                        Text { anchors { left: parent.left; leftMargin: 8; verticalCenter: parent.verticalCenter } text: parent.modelData === "" ? "The look's own" : parent.modelData; font.family: parent.modelData === "" ? Theme.font : parent.modelData; font.pixelSize: 14; color: ed.cur.font === parent.modelData ? Theme.accentInk : Theme.text }
                        MouseArea { id: fa; anchors.fill: parent; hoverEnabled: true; onClicked: ed.setLook({ font: parent.modelData }, "font") }
                    }
                }
                // a colour: the swatches or any colour
                Row {
                    visible: ed.popup.indexOf("colour:") === 0
                    spacing: 8
                    Swatches { value: ed.popup.indexOf("colour:") === 0 ? ed.cur[ed.popup.slice(7)] : ""; onPicked: h => { const o = {}; o[ed.popup.slice(7)] = h; ed.setLook(o, ed.popup.slice(7)); } }
                    Hex { value: ed.popup.indexOf("colour:") === 0 ? ed.cur[ed.popup.slice(7)] : ""; onPicked: h => { const o = {}; o[ed.popup.slice(7)] = h; ed.setLook(o, ed.popup.slice(7)); } }
                }
                // Save as my own block…
                Column {
                    visible: ed.popup === "saveas"
                    width: parent.width; spacing: 10
                    Text { width: parent.width; wrapMode: Text.WordWrap; text: "A button of yours with this look and what a press does, in Blocks › Your own."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    TextBox { id: saveName; width: parent.width; text: ed.blk ? ed.nameOf(ed.blk.id) : ""; placeholder: "Its name" }
                    Btn { label: "Save"; primary: true; onClicked: ed.saveAsOwn(saveName.current()) }
                }
            }
        }

        // ---- the small toolbar while a block (or the bar) is picked
        Rectangle {
            id: smallTool
            visible: side.visible
            x: Math.max(12, (side.x - width) / 2); anchors { bottom: parent.bottom; bottomMargin: 28 }
            width: smallRow.implicitWidth + 20; height: 56; radius: 16
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.97); border.color: Theme.line
            Row {
                id: smallRow
                anchors.centerIn: parent
                spacing: 8
                Btn { label: "Undo"; icon: "undo"; enabled: ed.undoStack.length > 0; onClicked: ed.undo() }
                Btn { label: "Redo"; icon: "redo"; enabled: ed.redoStack.length > 0; onClicked: ed.redo() }
                Sep {}
                Btn { label: "Cancel"; onClicked: ed.cancel() }
                Btn { label: "Try it"; onClicked: ed.tryIt() }
                Btn { label: "Keep this"; primary: true; onClicked: ed.keep() }
            }
        }

        // ---- the block being dragged: it follows the pointer, tilted, with a shadow
        Rectangle {
            visible: ed.dragId !== ""
            x: ed.dragAt.x - 30; y: ed.dragAt.y - 20
            rotation: -2
            width: ghostRow.implicitWidth + 28; height: 40; radius: 10
            color: Theme.panel2; border.color: Theme.accent; border.width: 1
            Rectangle { z: -1; anchors.fill: parent; anchors.topMargin: 10; radius: 12; color: Qt.rgba(0, 0, 0, 0.45) }
            Row { id: ghostRow; anchors.centerIn: parent; spacing: 8
                Icon { name: ed.iconOf(ed.dragId); size: 16; color: Theme.text; anchors.verticalCenter: parent.verticalCenter }
                Text { text: ed.nameOf(ed.dragId); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; anchors.verticalCenter: parent.verticalCenter } }
        }
    }

    // ---- trying it: your desktop as it'll be, with a way back
    Rectangle {
        id: tryPill
        visible: ed.trying
        anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 28 }
        width: tryRow.implicitWidth + 24; height: 52; radius: 16
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.97); border.color: Theme.accent
        Row {
            id: tryRow
            anchors.centerIn: parent
            spacing: 10
            Text { text: "Trying your desktop"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; anchors.verticalCenter: parent.verticalCenter }
            Btn { label: "Back to editing"; onClicked: ed.trying = false }
            Btn { label: "Go back"; onClicked: ed.goBack() }
            Btn { label: "Keep this"; primary: true; onClicked: ed.keep() }
        }
    }

    // ================================================================== small parts
    // a block in a bar or a panel, in its look: click to pick it, right-click for its menu, drag it anywhere
    component Chip: Item {
        id: c
        property string bid: ""
        property var from: null
        property bool square: false
        readonly property bool own: E.isOwn(bid)
        readonly property var lk: ed.draft.looks ? ed.draft.looks[bid] : undefined
        readonly property bool styled: lk !== undefined && lk !== null
        readonly property bool lit: ed.picked === "b:" + bid
        readonly property bool hiddenLook: styled && lk.hidden === true
        readonly property bool inner: bid.indexOf("panel:") === 0
        width: Math.max(square ? 40 : 40, look.implicitWidth)
        height: Math.max(square ? 40 : 30, look.implicitHeight)
        opacity: ed.dragId === bid ? 0.35 : hiddenLook ? 0.45 : 1
        // the editor's own chip, under a block with no fill or edge of its own
        Rectangle {
            anchors.fill: parent
            visible: !c.styled || (c.lk.fill === undefined && c.lk.edge === undefined)
            radius: 8
            color: Theme.panel2
            border.color: ed.dragId === c.bid && ed.dragFrom && ed.dragFrom === c.from ? Theme.accent : Theme.line
        }
        BlockLook {
            id: look
            anchors.centerIn: parent
            bid: c.bid
            editing: true
            vert: c.square
            side: 30
            lookOverride: c.styled ? c.lk : null
            ownOverride: c.own ? (ed.draft.own[c.bid.slice(4)] || null) : undefined
            Item {
                implicitWidth: c.square ? 40 : c.inner ? innerRow.implicitWidth + 12 : chipRow.implicitWidth + 18
                implicitHeight: c.square ? 40 : 30
                Row {
                    id: chipRow
                    visible: !c.inner
                    anchors.centerIn: parent
                    spacing: 6
                    Icon { id: ci; name: ed.iconOf(c.bid); size: 15; color: Theme.text; anchors.verticalCenter: parent.verticalCenter }
                    Text { id: ct; visible: !c.square; text: T.baseId(c.bid) === "spacer" ? "space" : ed.nameOf(c.bid); color: T.baseId(c.bid) === "spacer" ? Theme.muted : Theme.text; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight; width: Math.min(implicitWidth, 160) }
                }
                // a panel inside: its own blocks, small (pick one, or right-click it)
                Row {
                    id: innerRow
                    visible: c.inner
                    anchors.centerIn: parent
                    spacing: 4
                    Repeater {
                        model: c.inner && ed.draft.panels[c.bid.slice(6)] ? (ed.draft.panels[c.bid.slice(6)].items || []) : []
                        delegate: MiniChip { required property string modelData; bid: modelData; from: ({ kind: "panel", id: c.bid.slice(6) }) }
                    }
                    Text { visible: innerRow.children.length <= 2; text: "panel inside"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; anchors.verticalCenter: parent.verticalCenter }
                }
            }
        }
        Icon { visible: c.hiddenLook; anchors { right: parent.right; top: parent.top; margins: -4 } name: "eye"; size: 12; color: Theme.muted }
        MouseArea {
            anchors.fill: parent
            z: c.inner ? -1 : 0          // a panel inside: its own blocks take their clicks
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            cursorShape: Qt.OpenHandCursor
            preventStealing: true
            property point at: Qt.point(0, 0)
            property bool moved: false
            onPressed: m => { at = mapToItem(ed.contentItem, m.x, m.y); moved = false; }
            onPositionChanged: m => {
                if (!(m.buttons & Qt.LeftButton)) return;
                const p = mapToItem(ed.contentItem, m.x, m.y);
                if (!moved && Math.abs(p.x - at.x) + Math.abs(p.y - at.y) > 6) { moved = true; ed.startDrag(c.bid, c.from, p); }
                if (moved) ed.moveDrag(p);
            }
            onReleased: m => { if (moved) ed.endDrag(); }
            onClicked: m => {
                if (moved) return;
                if (m.button === Qt.RightButton) ed.blockMenu(c.bid, c.from, mapToItem(ed.contentItem, m.x, m.y));
                else ed.pickBlock(c.bid, c.from, c.width, c.height);
            }
        }
        // picked: lit in its place, with handles at its ends (drag one to give it a width)
        Dashes { visible: c.lit; anchors.fill: parent; anchors.margins: -4; lit: c.lit; z: 2 }
        Repeater {
            model: c.lit ? [0, 1] : []
            delegate: Rectangle {
                id: wh
                required property int modelData
                z: 3
                width: 10; height: 10; radius: 2
                color: Theme.accent; border.color: "#ffffff"; border.width: 2
                x: c.square ? (c.width - width) / 2 : modelData === 0 ? -width / 2 - 4 : c.width - width / 2 + 4
                y: c.square ? (modelData === 0 ? -height / 2 - 4 : c.height - height / 2 + 4) : (c.height - height) / 2
                MouseArea {
                    anchors { fill: parent; margins: -6 }
                    cursorShape: c.square ? Qt.SizeVerCursor : Qt.SizeHorCursor
                    preventStealing: true
                    property real sx: 0
                    property real s0: 0
                    onPressed: m => { const p = mapToItem(ed.contentItem, m.x, m.y); sx = c.square ? p.y : p.x; s0 = c.square ? c.height : c.width; }
                    onPositionChanged: m => {
                        const p = mapToItem(ed.contentItem, m.x, m.y);
                        const dv = ((c.square ? p.y : p.x) - sx) * (wh.modelData === 0 ? -1 : 1);
                        const v = Math.max(16, Math.round(s0 + dv));
                        if (c.square) ed.setLook({ h: v }, "h"); else ed.setLook({ w: "fixed", wpx: v }, "w");
                        ed.note = (c.square ? "Height " : "Width ") + v + " px";
                    }
                    onReleased: ed.say(ed.note)
                }
            }
        }
    }
    // a block in a panel inside a bar or a panel: pick it, right-click it
    component MiniChip: Rectangle {
        id: mc
        property string bid: ""
        property var from: null
        width: 26; height: 22; radius: 6
        color: ed.picked === "b:" + bid ? Theme.accent : Theme.panel3
        border.color: Theme.line
        Icon { anchors.centerIn: parent; name: ed.iconOf(mc.bid); size: 13; color: ed.picked === "b:" + mc.bid ? Theme.accentInk : Theme.text }
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: m => { if (m.button === Qt.RightButton) ed.blockMenu(mc.bid, mc.from, mapToItem(ed.contentItem, m.x, m.y)); else ed.pickBlock(mc.bid, mc.from, mc.width, mc.height); }
        }
    }
    // a block in the Blocks window
    component LibTile: Rectangle {
        id: lt
        property string bid: ""
        property string label: ""
        property string icon: ""
        property bool dashed: false
        property string make: ""
        width: (libCol.width - 8) / 2; height: 62; radius: 10
        color: Theme.panel2; border.color: Theme.line
        Rectangle { visible: lt.dashed; anchors.fill: parent; radius: 10; color: "transparent"; border.color: Theme.muted; border.width: 1; opacity: 0.6 }
        Column {
            anchors.centerIn: parent
            spacing: 6
            Icon { anchors.horizontalCenter: parent.horizontalCenter; name: lt.icon; size: 20; color: lt.dashed ? Theme.accent : Theme.text }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: lt.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 12; width: Math.min(implicitWidth, lt.width - 12); elide: Text.ElideRight; horizontalAlignment: Text.AlignHCenter }
        }
        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.OpenHandCursor
            preventStealing: true
            onPressed: m => { const p = mapToItem(ed.contentItem, m.x, m.y); if (lt.make !== "") ed.makeOwn(lt.make, p); else ed.startDrag(lt.bid, { kind: "lib" }, p); }
            onPositionChanged: m => ed.moveDrag(mapToItem(ed.contentItem, m.x, m.y))
            onReleased: ed.endDrag()
        }
    }
    component Section: Column {
        id: sec
        property string title: ""
        property var kinds: []
        readonly property var items: ed.shownLib.filter(b => kinds.indexOf(b.kind) >= 0)
        visible: items.length > 0
        width: libCol.width
        spacing: 8
        Text { text: sec.title; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 1.2 }
        Grid {
            columns: 2; spacing: 8
            Repeater { model: sec.items; delegate: LibTile { required property var modelData; bid: modelData.id; label: modelData.name; icon: modelData.icon } }
        }
    }
    component Handle: Rectangle {
        id: h
        property string mode: ""
        property string what: ""
        property string tid: ""
        property int cursor: Qt.SizeAllCursor
        width: 12; height: 12; radius: 3
        color: Theme.accent; border.color: "#ffffff"; border.width: 2
        MouseArea {
            anchors { fill: parent; margins: -6 }
            cursorShape: h.cursor
            preventStealing: true
            onPressed: m => ed.startMove(h.what, h.tid, h.mode, mapToItem(ed.contentItem, m.x, m.y))
            onPositionChanged: m => ed.doMove(mapToItem(ed.contentItem, m.x, m.y))
            onReleased: ed.endMove()
        }
    }
    component Tag: Rectangle {
        property alias text: tt.text
        width: tt.implicitWidth + 16; height: 22; radius: 6
        color: Theme.accent
        Text { id: tt; anchors.centerIn: parent; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 0.9 }
    }
    // the dashed outline (accent), brighter when it's picked or a block would land in it
    component Dashes: Canvas {
        id: dc
        property bool lit: false
        onLitChanged: requestPaint()
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onPaint: {
            const g = getContext("2d");
            g.reset();
            g.strokeStyle = Theme.accent;
            g.globalAlpha = lit ? 1 : 0.75;
            g.lineWidth = lit ? 3 : 2;
            g.setLineDash([8, 6]);
            g.strokeRect(1, 1, width - 2, height - 2);
        }
    }
    component Sep: Rectangle { width: 1; height: 28; color: Theme.line; anchors.verticalCenter: parent ? parent.verticalCenter : undefined }
    component Btn: Rectangle {
        id: b
        property string label: ""
        property string icon: ""
        property bool primary: false
        property bool on: false
        signal clicked()
        width: br.implicitWidth + 28; height: 38; radius: 10
        opacity: enabled ? 1 : 0.45
        color: primary ? Theme.accent : on ? Theme.panel3 : ba.containsMouse ? Theme.panel3 : Theme.panel2
        border.color: primary ? Theme.accent : Theme.line
        Row { id: br; anchors.centerIn: parent; spacing: 8
            Icon { visible: b.icon !== ""; name: b.icon; size: 16; color: b.primary ? Theme.accentInk : Theme.text; anchors.verticalCenter: parent.verticalCenter }
            Text { text: b.label; color: b.primary ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: b.primary ? Font.DemiBold : Font.Medium; anchors.verticalCenter: parent.verticalCenter } }
        MouseArea { id: ba; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: if (b.enabled) b.clicked() }
    }
    // ---- the side panel's parts
    component Sec: Text {
        topPadding: 12; bottomPadding: 2
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 1.2
    }
    component Val: Text {
        width: 60; height: 30; horizontalAlignment: Text.AlignRight; verticalAlignment: Text.AlignVCenter
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
    }
    // a row: what it is on the left, its controls on the right
    component LRow: Item {
        id: lr
        property string label: ""
        default property alias ctl: lrc.data
        width: sideCol.width
        height: Math.max(34, lrc.implicitHeight + 4)
        Text { anchors { left: parent.left; verticalCenter: parent.verticalCenter } width: Math.max(80, parent.width - lrc.implicitWidth - 12); elide: Text.ElideRight; text: lr.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
        Row { id: lrc; anchors { right: parent.right; verticalCenter: parent.verticalCenter } spacing: 8 }
    }
    // joined choices, the one on lit
    component Seg: Rectangle {
        id: sg
        property var options: []
        property string value: ""
        signal picked(string v)
        width: segRow.implicitWidth + 4; height: 30; radius: 8
        color: "transparent"; border.color: Theme.line
        Row {
            id: segRow
            anchors.centerIn: parent
            Repeater {
                model: sg.options
                delegate: Rectangle {
                    required property var modelData
                    readonly property bool on: String(sg.value) === String(modelData[0])
                    width: segT.implicitWidth + 18; height: 26; radius: 6
                    color: on ? Theme.accent : segA.containsMouse ? Theme.panel3 : "transparent"
                    Text { id: segT; anchors.centerIn: parent; text: parent.modelData[1]; color: parent.on ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: parent.on ? Font.DemiBold : Font.Normal }
                    MouseArea { id: segA; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: sg.picked(String(parent.modelData[0])) }
                }
            }
        }
    }
    // choices that wrap (longer ones)
    component Choices: Flow {
        id: chs
        property var options: []
        property string value: ""
        signal picked(string v)
        width: sideCol.width
        spacing: 6
        Repeater {
            model: chs.options
            delegate: Chipish { required property var modelData; label: modelData[1]; on: String(chs.value) === String(modelData[0]); onClicked: chs.picked(String(modelData[0])) }
        }
    }
    component Chipish: Rectangle {
        id: cp
        property string label: ""
        property bool on: false
        property bool dim: false
        signal clicked()
        width: cpT.implicitWidth + 22; height: 30; radius: 8
        opacity: dim ? 0.5 : 1
        color: on ? Theme.accent : cpA.containsMouse ? Theme.panel3 : Theme.panel2
        border.color: on ? Theme.accent : Theme.line
        Text { id: cpT; anchors.centerIn: parent; text: cp.label; color: cp.on ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 12; font.weight: cp.on ? Font.DemiBold : Font.Normal }
        MouseArea { id: cpA; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: cp.clicked() }
    }
    // a number, typed
    component NumBox: Rectangle {
        id: nb
        property int value: 0
        property string prefix: ""
        property string suffix: " px"
        property int lo: 0
        property int hi: 4000
        signal typed(int v)
        width: 70; height: 30; radius: 8
        color: Theme.bg; border.color: ni.activeFocus ? Theme.accent : Theme.line
        TextInput {
            id: ni
            anchors { fill: parent; leftMargin: 10; rightMargin: 8 }
            verticalAlignment: TextInput.AlignVCenter
            clip: true
            color: Theme.text; font.family: Theme.font; font.pixelSize: 13
            selectByMouse: true
            text: nb.prefix + nb.value + nb.suffix
            onEditingFinished: {
                const v = parseInt(text.replace(/[^0-9-]/g, ""));
                if (!isNaN(v)) nb.typed(Math.max(nb.lo, Math.min(nb.hi, v)));
                text = Qt.binding(() => nb.prefix + nb.value + nb.suffix);
            }
        }
    }
    // words, typed
    component TextBox: Rectangle {
        id: tb
        property string text: ""
        property string placeholder: ""
        signal done(string t)
        signal typing(string t)
        function current() { return ti.text; }
        width: 220; height: 30; radius: 8
        color: Theme.bg; border.color: ti.activeFocus ? Theme.accent : Theme.line
        clip: true
        TextInput {
            id: ti
            anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.text; font.family: Theme.font; font.pixelSize: 13
            selectByMouse: true
            text: tb.text
            onTextChanged: if (activeFocus) tb.typing(text)
            onEditingFinished: { tb.done(text); text = Qt.binding(() => tb.text); }
        }
        Text { visible: ti.text === "" && !ti.activeFocus; anchors { left: parent.left; leftMargin: 10; right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter } elide: Text.ElideRight; text: tb.placeholder; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
    }
    // a slider
    component Slide: Item {
        id: sl
        property real from: 0
        property real to: 100
        property real value: 0
        signal moved(int v)
        width: 150; height: 30
        Rectangle { anchors.verticalCenter: parent.verticalCenter; width: parent.width; height: 4; radius: 2; color: Theme.panel3 }
        Rectangle { anchors.verticalCenter: parent.verticalCenter; width: knob.x + knob.width / 2; height: 4; radius: 2; color: Theme.accent }
        Rectangle {
            id: knob
            anchors.verticalCenter: parent.verticalCenter
            width: 16; height: 16; radius: 8
            color: "#ffffff"; border.color: Theme.accent; border.width: 2
            x: (sl.width - width) * Math.max(0, Math.min(1, (sl.value - sl.from) / Math.max(0.0001, sl.to - sl.from)))
        }
        MouseArea {
            anchors.fill: parent
            preventStealing: true
            cursorShape: Qt.PointingHandCursor
            function at(mx) { const k = Math.max(0, Math.min(1, (mx - 8) / Math.max(1, sl.width - 16))); return Math.round(sl.from + k * (sl.to - sl.from)); }
            onPressed: m => sl.moved(at(m.x))
            onPositionChanged: m => sl.moved(at(m.x))
        }
    }
    component Toggle: Rectangle {
        id: tg
        property bool on: false
        signal toggled()
        width: 38; height: 22; radius: 11
        color: on ? Theme.accent : Theme.panel3; border.color: Theme.line
        Rectangle { width: 16; height: 16; radius: 8; y: 3; x: tg.on ? tg.width - 19 : 3; color: "#ffffff" }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: tg.toggled() }
    }
    // colours: its own (none), dark, the accent, blue, pink
    component Swatches: Row {
        id: sws
        property string value: ""
        signal picked(string h)
        spacing: 8
        Repeater {
            model: ["", "#2a2d36", String(Theme.accent), "#3b5ba5", "#ff6b9a"]
            delegate: Rectangle {
                required property string modelData
                readonly property bool on: String(sws.value).toLowerCase() === modelData.toLowerCase()
                width: 22; height: 22; radius: 5
                color: modelData === "" ? "transparent" : modelData
                border.color: on ? "#ffffff" : Theme.line; border.width: on ? 2 : 1
                Rectangle { visible: parent.modelData === ""; anchors.centerIn: parent; width: 2; height: parent.height - 6; rotation: 45; color: Theme.muted }
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: sws.picked(parent.modelData) }
            }
        }
    }
    // any colour, by its hex
    component Hex: TextBox {
        id: hx
        property string value: ""
        signal picked(string h)
        width: 88
        text: value.toUpperCase()
        placeholder: "Its own"
        onDone: t => {
            const v = t.trim();
            if (v === "") { hx.picked(""); return; }
            const h = v.charAt(0) === "#" ? v : "#" + v;
            if (/^#[0-9a-fA-F]{6}$/.test(h)) hx.picked(h.toLowerCase());
        }
    }
    // a colour of the look (its edge's, its icon's…): click for the swatches and any colour
    component ColourDot: Rectangle {
        id: cd
        property string value: ""
        property string key: ""
        width: 30; height: 30; radius: 8
        color: "transparent"; border.color: ed.popup === "colour:" + key ? Theme.accent : Theme.line
        Rectangle { anchors.centerIn: parent; width: 20; height: 20; radius: 5; color: cd.value !== "" ? cd.value : Theme.accent }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: ed.popup = ed.popup === "colour:" + cd.key ? "" : "colour:" + cd.key }
    }
    // a picker's button: what it is, ▾
    component PickBtn: Rectangle {
        id: pk
        property string icon: ""
        property string label: ""
        signal clicked()
        width: pkRow.implicitWidth + 24; height: 30; radius: 8
        color: pkA.containsMouse ? Theme.panel3 : Theme.panel2; border.color: Theme.line
        Row {
            id: pkRow
            anchors.centerIn: parent
            spacing: 8
            Rectangle { visible: pk.icon !== ""; width: 16; height: 16; radius: 4; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter
                Icon { anchors.centerIn: parent; name: pk.icon || "dot"; size: 12; color: Theme.accentInk } }
            Text { text: pk.label + " ▾"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
        }
        MouseArea { id: pkA; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: pk.clicked() }
    }
    component SmallBtn: Rectangle {
        id: sb
        property string label: ""
        signal clicked()
        width: sbT.implicitWidth + 26; height: 32; radius: 8
        color: sbA.containsMouse ? Theme.panel3 : Theme.panel2; border.color: Theme.line
        Text { id: sbT; anchors.centerIn: parent; text: sb.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.Medium }
        MouseArea { id: sbA; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: sb.clicked() }
    }
    // a line of the right-click menu
    component MenuRow: Rectangle {
        id: mr
        property string label: ""
        property bool danger: false
        property bool dim: false
        signal clicked()
        width: parent ? parent.width : 230; height: 32; radius: 6
        color: mrA.containsMouse ? Theme.panel3 : "transparent"
        opacity: dim ? 0.5 : 1
        Text { anchors { left: parent.left; leftMargin: 8; verticalCenter: parent.verticalCenter } text: mr.label; color: mr.danger ? Theme.danger : Theme.text; font.family: Theme.font; font.pixelSize: 13 }
        MouseArea { id: mrA; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: mr.clicked() }
    }
}

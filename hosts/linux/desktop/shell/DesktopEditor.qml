import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "desktop-edit.js" as E
import "widget-tools.js" as T
import "widget-layout.js" as L

// Edit desktop (drag and drop, board 1; docs/mockups/drag-drop/drag-drop-1.png, checklist
// docs/mockups/checklists/drag-drop.md part 1). Over your real desktop: every bar, panel and widget outlined with what
// it is and its size; the Blocks window (its own window you can move) to drag from; drag any block onto any bar, any
// panel or the desktop, or a placed one anywhere else; panels anywhere on the screen (an edge is only a snap: the
// owner, Nov 26, "dont want them locked to just 4 sides"), sized by their ends and sides; widgets free on the desktop;
// snap to grid and guides; Undo, Redo, Layers, Screen, + Panel, Start from; Cancel, Try it, Keep this. Nothing is
// written until Keep this (or Try it, which can be taken back), and then only through BarLayout and WidgetStore
// (merged, atomic, a .bak kept). The rules are in desktop-edit.js (test_desktop_edit.py).
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
    property var draft: ({ main: T.barLayout(null), panels: ({}), widgets: ({}), own: ({}) })
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
    property string picked: ""            // "main" | "p:<id>" | "w:<id>"
    property string note: ""

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
        const panels = {}, own = {};
        for (const k in BarLayout.raw) {
            if (k.startsWith("p:")) { const p = BarLayout.raw[k]; if (p && typeof p === "object" && (!p.screen || p.screen === scrName || Quickshell.screens.length < 2)) panels[k.slice(2)] = JSON.parse(JSON.stringify(p)); }
            if (k.startsWith("b:")) own[k.slice(2)] = JSON.parse(JSON.stringify(BarLayout.raw[k]));
        }
        draft = { main: T.barLayout(BarLayout.raw.items), panels: panels, widgets: mineNow(), own: own };
        base = JSON.parse(JSON.stringify(draft));
    }
    function change(d) {
        if (!d) return;
        undoStack = undoStack.concat([JSON.stringify(draft)]).slice(-200);
        redoStack = [];
        draft = d;
    }
    function undo() { if (!undoStack.length) return; redoStack = redoStack.concat([JSON.stringify(draft)]); draft = JSON.parse(undoStack[undoStack.length - 1]); undoStack = undoStack.slice(0, -1); }
    function redo() { if (!redoStack.length) return; undoStack = undoStack.concat([JSON.stringify(draft)]); draft = JSON.parse(redoStack[redoStack.length - 1]); redoStack = redoStack.slice(0, -1); }

    // ---- writing (only here, only through the safe writers)
    function write(d, from) {
        const c = E.changes(from, d);
        if (E.nothingChanged(c)) return;
        if (Object.keys(c.bar).length) BarLayout.put(c.bar);
        for (const id in c.widgets.put) { const w = c.widgets.put[id]; WidgetStore.place(id, w.x, w.y, w.w, w.h, scrName); }
        for (const a of c.widgets.add) WidgetStore.add(a.rec.type, scrName, Activities.current, "desk", { x: a.rec.x, y: a.rec.y, w: a.rec.w, h: a.rec.h }, {});
        for (const id of c.widgets.remove) WidgetStore.remove(id);
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
    function keep() { write(draft, trying ? draft : base); trying = false; shown = false; SettingsNav.toast("Your desktop is kept"); }
    function cancel() { if (trying || startedFrom) putBack(); trying = false; startedFrom = false; shown = false; }
    function tryIt() { write(draft, base); base = JSON.parse(JSON.stringify(draft)); trying = true; }
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
    readonly property string mainEdge: DeskState.barEdge || "top"
    readonly property bool mainDown: mainEdge === "left" || mainEdge === "right"
    readonly property int mainSize: Math.max(32, mainEdge === "top" ? DeskState.barTop : mainEdge === "bottom" ? DeskState.barBottom : mainEdge === "left" ? DeskState.barLeft : DeskState.barRight)
    readonly property rect mainBox: mainEdge === "top" ? Qt.rect(0, 0, sw, mainSize) : mainEdge === "bottom" ? Qt.rect(0, sh - mainSize, sw, mainSize)
        : mainEdge === "left" ? Qt.rect(0, 0, mainSize, sh) : Qt.rect(sw - mainSize, 0, mainSize, sh)

    function inBox(p, r, pad) { pad = pad || 0; return p.x >= r.x - pad && p.x <= r.x + r.width + pad && p.y >= r.y - pad && p.y <= r.y + r.height + pad; }
    function panelRect(id) { const p = T.panelOf(id, draft.panels[id]); if (!p) return Qt.rect(0, 0, 0, 0); const b = E.panelBox(p, sw, sh, (p.items.length || 1) * 48 + 24); return Qt.rect(b.x, b.y, b.w, b.h); }
    function othersFor(skip) {
        const o = [];
        for (const id in draft.widgets) if ("w:" + id !== skip) { const w = draft.widgets[id]; o.push({ x: w.x, y: w.y, w: w.w, h: w.h, name: "the " + nameOf("widget:" + w.type).toLowerCase() }); }
        for (const id in draft.panels) if ("p:" + id !== skip) { const r = panelRect(id); o.push({ x: r.x, y: r.y, w: r.width, h: r.height, name: "the panel" }); }
        return o;
    }

    // ---- dragging a block: where it would land
    function startDrag(id, from, p) { dragId = id; dragFrom = from; dragAt = p; picked = ""; moveDrag(p); }
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
                const r = panelRect(id);
                if (!inBox(p, r, 6)) continue;
                const pp = T.panelOf(id, draft.panels[id]);
                const down = pp.free ? pp.orient === "down" : pp.edge === "left" || pp.edge === "right";
                const rep = panelReps.itemAt(Object.keys(draft.panels).indexOf(id));
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
        for (const id in draft.panels) { const p = T.panelOf(id, draft.panels[id]); if (p && !p.free) used[p.edge] = true; }
        if (Theme.dockFloats) used.bottom = true;
        return ["top", "bottom", "left", "right"].filter(e => !used[e]);
    }

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
        Keys.onEscapePressed: ed.cancel()
        Keys.onPressed: e => {
            if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_Z) { if (e.modifiers & Qt.ShiftModifier) ed.redo(); else ed.undo(); e.accepted = true; }
            else if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_Y) { ed.redo(); e.accepted = true; }
            else if (e.key === Qt.Key_Delete && ed.picked !== "" && ed.picked !== "main") { ed.remove(ed.picked); e.accepted = true; }
        }

        // the desktop shows through, a little dimmed
        Rectangle { anchors.fill: parent; color: Qt.rgba(0.04, 0.03, 0.02, 0.18) }
        MouseArea { anchors.fill: parent; onClicked: { ed.picked = ""; ed.layersShown = false; ed.startShown = false; } }

        // ---- the main bar
        Rectangle {
            id: mainR
            x: ed.mainBox.x; y: ed.mainBox.y; width: ed.mainBox.width; height: ed.mainBox.height
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.92)
            border.width: 0
            Dashes { anchors.fill: parent; anchors.margins: 2; lit: ed.picked === "main" || (ed.target && ed.target.kind === "main") }
            MouseArea { anchors.fill: parent; onClicked: ed.picked = "main" }
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
            model: Object.keys(ed.draft.panels)
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
            anchors { horizontalCenter: parent.horizontalCenter; bottom: tool.top; bottomMargin: 12 }
            text: "EDITING YOUR DESKTOP · NOTHING IS KEPT UNTIL KEEP THIS · ESC CANCELS"
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 1.2
            Rectangle { z: -1; anchors { fill: parent; margins: -6 } radius: 6; color: Theme.panel2; border.color: Theme.line }
        }
        Rectangle {
            id: tool
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
    // a block in a bar or a panel: drag it anywhere
    component Chip: Rectangle {
        id: c
        property string bid: ""
        property var from: null
        property bool square: false
        readonly property bool own: E.isOwn(bid)
        width: square ? 40 : Math.max(40, ct.implicitWidth + (ci.visible ? 34 : 18)); height: square ? 40 : 30
        radius: 8
        color: Theme.panel2; border.color: ed.dragId === bid && ed.dragFrom && ed.dragFrom === from ? Theme.accent : Theme.line
        opacity: ed.dragId === bid ? 0.35 : 1
        Row {
            anchors.centerIn: parent
            spacing: 6
            Icon { id: ci; name: ed.iconOf(c.bid); size: 15; color: Theme.text; anchors.verticalCenter: parent.verticalCenter }
            Text { id: ct; visible: !c.square; text: c.bid === "spacer" ? "space" : ed.nameOf(c.bid); color: c.bid === "spacer" ? Theme.muted : Theme.text; font.family: Theme.font; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter; elide: Text.ElideRight; width: Math.min(implicitWidth, 160) }
        }
        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.OpenHandCursor
            preventStealing: true
            onPressed: m => ed.startDrag(c.bid, c.from, mapToItem(ed.contentItem, m.x, m.y))
            onPositionChanged: m => ed.moveDrag(mapToItem(ed.contentItem, m.x, m.y))
            onReleased: ed.endDrag()
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
}

// Edit layout in one of Mantel's apps (drag and drop, board 3: docs/mockups/drag-drop/drag-drop-3.png, checklist
// docs/mockups/checklists/drag-drop.md part 3), over the live app (AppBlocks.qml loads it only while editing or
// trying, so a problem here only loses the editor, never the app). The look and the rules are the desktop editor's
// (DesktopEditor.qml, desktop-edit.js): the window marked EDITING; every part outlined (dashed, accent) with a label
// naming it and its size ("COMMAND BAR · 48 px", "SIDEBAR · 216 px · DRAG ITS EDGE"); the app's own Blocks window
// ("Files' blocks", "drag into Files"; Find a block, All, Buttons, Panes, Info, Your own; Make your own); drag any
// block into any bar of the app (a lit line where it lands, "Drop: command bar, after Sort") or anywhere on the
// window, and a placed one anywhere else; parts dragged anywhere (not only to its sides), panes and the sidebar sized
// by their edges with a guide and a note ("Preview: 514 px · lined up with Search"); right-click any part ›
// Customize (AppBlockCustomize.qml, board 2's side panel); the toolbar: Undo, Redo, Snap to grid · 8 px, Guides,
// Layers, Start from, Menus…, Cancel, Try it, Keep this. Everything here changes the draft only (AppBlocks.qml writes
// it with Keep this); the rules are app-blocks.js's (test_app_blocks.py).
import QtQuick
import "app-blocks.js" as L

Item {
    id: ed
    property var ab: null
    readonly property bool on: !!ab && ab.editing
    readonly property real ew: width
    readonly property real eh: height
    // a window narrower than the board's: the toolbar's buttons a little smaller, so all of them fit
    readonly property bool tight: ew < 1400
    property bool snapOn: true
    property int grid: L.PX_GRID
    property bool guidesOn: true
    property bool layersShown: false
    property bool startShown: false
    property string startName: "Hearth"
    property string note: ""
    property var guides: []
    property real lineX: -1
    property real lineY: 0
    property real lineLen: 0
    property point dragAt: Qt.point(0, 0)
    property var target: null              // where a dragged block would land: {kind: "bar", bar, at, before} | {kind: "window", x, y} | {kind: "lib"}
    property string dragLib: ""            // a block from the Blocks window being dragged ("action:share", "make:button"…)
    property string dragBlock: ""          // a placed block being dragged
    property string beforeJson: ""
    // ---------------------------------------------------------------- the parts and panes on the window, in px
    function canvasList() {
        if (!ab) return [];
        // each part where it's drawn (its look's Fit, Fixed or Fill included)
        const out = ab.draft.parts.map((p, i) => {
            const lk = ab.draft.looks[p.id];
            let r = p;
            if (lk && (lk.wMode || lk.hMode) && !p.hidden) {
                const it = ab.parts[p.id];
                const need = ab.alive(it) ? { w: it.implicitWidth || 0, h: it.implicitHeight || 0 } : null;
                r = Object.assign({}, p, L.sizedByLook(p, lk, need, ab.draft.parts.filter((q, j) => j !== i && !q.hidden), ew, eh));
            }
            return Object.assign({}, r, { name: ab.labelOf(p.id) });
        });
        for (const b of ab.draft.added) if (b.kind === "pane") out.push(Object.assign({ id: b.id, title: ab.blockName(b), name: ab.blockName(b), hidden: false, pane: true }, L.fromFrac(b, ew, eh)));
        return out;
    }
    function commitList(d, list) {
        for (const q of list) {
            if (q.pane) { const b = d.added.find(x => x.id === q.id); if (b) Object.assign(b, L.toFrac(q, ew, eh)); continue; }
            const p = d.parts.find(x => x.id === q.id);
            if (p) { p.x = q.x; p.y = q.y; p.w = q.w; p.h = q.h; p.hidden = !!q.hidden; }
        }
    }
    readonly property var shown: { void ew; void eh; if (!ab) return []; void ab.draft; return canvasList().filter(p => !p.hidden); }
    function marksNow() {
        const out = [];
        if (!ab || !ab.marks) return out;
        for (const n in ab.marks) {
            const it = ab.marks[n];
            if (!ab.alive(it) || !it.visible || it.width <= 0) continue;
            const q = it.mapToItem(ab, 0, 0);
            out.push({ name: n, x: Math.round(q.x), y: Math.round(q.y), w: Math.round(it.width), h: Math.round(it.height) });
        }
        return out;
    }

    // ---------------------------------------------------------------- moving and sizing a part (or an added pane)
    property var moving: null              // {id, edges | null, sx, sy, list, marks}
    function blockAt(p) {
        for (const id in ab.blockItems) {
            const it = ab.blockItems[id];
            if (!it || !it.visible || !it.parent || it.parent === null) continue;
            const q = it.mapToItem(ab, 0, 0);
            if (p.x >= q.x && p.x <= q.x + it.width && p.y >= q.y && p.y <= q.y + it.height && it.width > 0) return id;
        }
        return "";
    }
    function grabPart(id, edges, p) {
        if (!ab) return;
        // sized by hand: it's that size now (a Fit or Fill from Customize gives way)
        const lk = ab.draft.looks[id];
        if (edges && lk && (lk.wMode || lk.hMode)) {
            const now = canvasList().find(q => q.id === id);
            ab.withDraft(d => {
                const l = Object.assign({}, d.looks[id]); delete l.wMode; delete l.wPx; delete l.hMode; delete l.hPx;
                if (L.lookEmpty(l)) delete d.looks[id]; else d.looks[id] = l;
                const p = d.parts.find(x => x.id === id); if (p && now) { p.x = now.x; p.y = now.y; p.w = now.w; p.h = now.h; }
            });
        }
        ab.picked = id; layersShown = false; startShown = false; menuShown = false;
        beforeJson = JSON.stringify(ab.draft);
        moving = { id: id, edges: edges, sx: p.x, sy: p.y, list: canvasList(), marks: marksNow() };
        ab.dragId = id;
    }
    function movePart(p) {
        if (!moving) return;
        const m = moving, dx = p.x - m.sx, dy = p.y - m.sy;
        const opt = { grid: snapOn ? grid : 0, guides: guidesOn, marks: m.marks, title: ab.labelOf(m.id) };
        const r = m.edges ? L.sizePx(m.list, m.id, m.edges, dx, dy, ew, eh, opt) : L.movePx(m.list, m.id, dx, dy, ew, eh, opt);
        guides = r.guides; note = r.note || (m.edges ? "" : "");
        dragAt = p;
        const d = L.copy(ab.draft);
        commitList(d, r.list);
        ab.liveDraft(d);
        moving.last = r.list;
    }
    function letGoPart() {
        if (!moving) return;
        const m = moving;
        moving = null; guides = []; note = ""; ab.dragId = "";
        if (!m.edges && m.last) {
            // let go over another part (more than half of the smaller one covered): the two change places
            const sw = L.dropped(m.last, m.id, m.list);
            const d = L.copy(ab.draft); commitList(d, sw); ab.liveDraft(d);
        }
        ab.commitFrom(beforeJson);
        beforeJson = "";
    }

    // ---------------------------------------------------------------- dragging a block: into a bar, or anywhere
    function grabLib(id, p) {
        if (!ab) return;
        dragLib = id; dragBlock = ""; ab.picked = ""; menuShown = false;
        beforeJson = JSON.stringify(ab.draft);
        moveDrag(p);
    }
    function grabBlock(id, p) {
        if (!ab) return;
        dragBlock = id; dragLib = ""; ab.picked = id; ab.dragId = id; menuShown = false;
        beforeJson = JSON.stringify(ab.draft);
        moveDrag(p);
    }
    readonly property string dragName: {
        if (!ab) return "";
        if (dragBlock) return ab.blockName(ab.draft.added.find(b => b.id === dragBlock));
        const it = lib.find(x => x.id === dragLib);
        return it ? it.name : dragLib.indexOf("make:") === 0 ? ({ "make:button": "My button", "make:count": "A count" })[dragLib] || "Your block" : "";
    }
    readonly property string dragIcon: {
        if (!ab) return "dot";
        const it = lib.find(x => x.id === dragLib);
        if (it) return it.icon;
        const b = ab.draft.added.find(x => x.id === dragBlock);
        if (b && b.kind === "action") { const a = ab.actionOf(b.ref); return a && a.icon ? a.icon : "dot"; }
        return dragLib === "make:button" ? "rocket" : "dot";
    }
    function isPaneDrag() { return dragLib.indexOf("pane:") === 0; }
    function inBox(p, r, pad) { pad = pad || 0; return p.x >= r.x - pad && p.x <= r.x + r.w + pad && p.y >= r.y - pad && p.y <= r.y + r.h + pad; }
    function itemName(it) {
        const clean = t => String(t || "").replace(/[✦⌄▾]/g, "").replace(/\s+/g, " ").trim();
        if (!it) return "";
        if (ab.isMine(it)) return ab.blockName(it.block);
        try { if (it.modelData && it.modelData.label) return clean(it.modelData.label); } catch (e) {}
        if (typeof it.label === "string" && it.label) return clean(it.label);
        if (typeof it.text === "string" && it.text) return clean(it.text);
        for (let i = 0; i < (it.children || []).length; i++) {
            const c = it.children[i];
            if (typeof c.label === "string" && c.label) return clean(c.label);
            if (typeof c.text === "string" && c.text) return clean(c.text);
        }
        return "";
    }
    function visibleKids(row) {
        const out = [];
        for (let i = 0; i < row.children.length; i++) {
            const c = row.children[i];
            if (ab.isRepeater(c) || !c.visible || c.width <= 0) continue;
            if (dragBlock && ab.isMine(c) && c.block.id === dragBlock) continue;
            const q = c.mapToItem(ab, 0, 0);
            out.push({ item: c, x: q.x, w: c.width, cx: q.x + c.width / 2 });
        }
        return out.sort((a, b) => a.x - b.x);
    }
    function moveDrag(p) {
        if (!ab) return;
        dragAt = p;
        lineX = -1; guides = [];
        let t = null;
        if (libWin.visible && inBox(p, { x: libWin.x, y: libWin.y, w: libWin.width, h: libWin.height })) {
            t = { kind: "lib" }; note = dragBlock ? "Let go to take it out" : "";
        } else if (!isPaneDrag()) {
            for (const q of shown) {
                if (q.pane || L.partKind(q, ew, eh) !== "bar" || !inBox(p, q, 4)) continue;
                const part = ab.parts[q.id];
                const row = ab.rowOf(part);
                const hd = ab.holderOf(q.id);
                if (!row) {
                    // a bar without a row of its own: at its end
                    t = { kind: "bar", bar: q.id, at: 0, before: "" };
                    lineX = q.x + q.w - 16; lineY = q.y + 6; lineLen = q.h - 12;
                    note = L.dropNote(ab.labelOf(q.id), "", false).replace(", at the start", ", at the end");
                    break;
                }
                const vis = visibleKids(row);
                const i = L.insertAt(vis.map(v => v.cx), p.x);
                const own = ab.ownKids(row);
                const next = i < vis.length ? vis[i].item : null;
                let at = own.length, before = "";
                if (next && ab.isMine(next)) { before = next.block.id; at = next.block.at || 0; }
                else if (next) at = Math.max(0, own.indexOf(next));
                let after = "";
                for (let k = i - 1; k >= 0 && !after; k--) after = itemName(vis[k].item);
                t = { kind: "bar", bar: q.id, at: at, before: before };
                const rq = row.mapToItem(ab, 0, 0);
                lineX = i < vis.length ? vis[i].x - 3 : (vis.length ? vis[vis.length - 1].x + vis[vis.length - 1].w + 2 : rq.x + 4);
                lineY = Math.max(q.y + 4, rq.y + 2); lineLen = Math.min(q.h - 8, row.height - 4);
                note = L.dropNote(ab.labelOf(q.id), after, false);
                break;
            }
        }
        if (!t) {
            const w = isPaneDrag() ? Math.min(420, ew * 0.3) : 160, h = isPaneDrag() ? eh * 0.6 : 36;
            let x = p.x - w / 2, y = p.y - h / 2;
            if (guidesOn || snapOn) {
                const list = canvasList().concat([{ id: "__drag", x: x, y: y, w: w, h: h }]);
                const r = L.movePx(list, "__drag", 0, 0, ew, eh, { grid: snapOn ? grid : 0, guides: guidesOn, marks: marksNow() });
                const me = r.list.find(z => z.id === "__drag");
                x = me.x; y = me.y; guides = r.guides;
                note = r.note || L.dropNote("", "", true);
            } else note = L.dropNote("", "", true);
            t = { kind: "window", x: Math.max(0, Math.min(ew - w, x)), y: Math.max(0, Math.min(eh - h, y)), w: w, h: h };
        }
        target = t;
    }
    function endDrag() {
        if (!ab) return;
        const t = target, lid = dragLib, bid = dragBlock;
        dragLib = ""; dragBlock = ""; target = null; lineX = -1; guides = []; note = ""; ab.dragId = "";
        if (!t || (!lid && !bid)) { beforeJson = ""; return; }
        const d = L.copy(ab.draft);
        let made = "";
        if (t.kind === "lib") {
            if (bid) { d.added = d.added.filter(b => b.id !== bid); d.z = d.z.filter(x => x !== bid); delete d.looks[bid]; }
        } else if (lid.indexOf("pane:") === 0) {
            // a pane: the app's own part shown (or moved) where it's let go, or a pane of its own added there
            const pn = ab.paneOf(lid.slice(5));
            const r = { x: Math.round(t.x), y: Math.round(t.y), w: Math.round(t.w), h: Math.round(t.h) };
            if (pn && pn.part) {
                const p = d.parts.find(x => x.id === pn.part);
                if (p) { if (!p.hidden) { r.w = p.w; r.h = p.h; r.x = Math.max(0, Math.min(ew - p.w, Math.round(dragAt.x - p.w / 2))); r.y = Math.max(0, Math.min(eh - p.h, Math.round(dragAt.y - p.h / 2))); } Object.assign(p, r, { hidden: false }); made = p.id; }
            } else if (pn) {
                const b = Object.assign(L.newBlock(lid, ab.newId(), ""), L.toFrac(r, ew, eh), { in: "" });
                d.added.push(b); made = b.id;
            }
        } else {
            let b = bid ? d.added.find(x => x.id === bid) : null;
            if (!b) {
                if (lid.indexOf("own:") === 0) {
                    const src = (d.mine || []).find(x => "own:" + x.id === lid) || d.added.find(x => "own:" + x.id === lid);
                    b = src ? Object.assign(L.copy(src), { id: ab.newId() }) : null;
                    if (b && src && d.looks[src.id]) d.looks[b.id] = L.copy(d.looks[src.id]);
                } else b = L.newBlock(lid, ab.newId(), "");
                if (!b) { beforeJson = ""; return; }
                made = b.id;
            } else d.added = d.added.filter(x => x.id !== bid);
            if (t.kind === "bar") {
                b.in = t.bar; b.at = t.at;
                delete b.x; delete b.y; delete b.w; delete b.h;
                let k = t.before ? d.added.findIndex(x => x.id === t.before) : -1;
                if (k < 0) k = d.added.findIndex(x => x.in === t.bar && (x.at || 0) > t.at);
                if (k < 0) d.added.push(b); else d.added.splice(k, 0, b);
            } else {
                const it = ab.blockItems[b.id];
                const w = it ? it.width : 140, h = it ? it.height : 36;
                b.in = ""; delete b.at;
                Object.assign(b, L.toFrac({ x: t.x + (t.w - w) / 2, y: t.y + (t.h - h) / 2, w: w, h: h }, ew, eh));
                d.added.push(b);
            }
        }
        ab.liveDraft(d);
        ab.commitFrom(beforeJson);
        beforeJson = "";
        if (made) { ab.picked = made; if (lid === "make:button") ab.customize(made, "does"); }
    }

    // ---------------------------------------------------------------- the Blocks window's list
    readonly property var lib: { if (!ab) return []; void ab.draft; return L.library(ab.actions, ab.panes, ab.counts, (ab.draft.mine || []).concat(ab.draft.added.filter(b => b.kind === "button" || b.kind === "words"))); }
    property string libKind: "all"
    property string libWords: ""
    readonly property var shownLib: L.find(lib, libKind, libWords)

    // ---------------------------------------------------------------- a part's or a block's menu while editing
    property bool menuShown: false
    property string menuFor: ""
    property point menuAt: Qt.point(0, 0)
    function partMenu(id, x, y) { if (!ab) return; ab.picked = id; menuFor = id; menuAt = Qt.point(x, y); menuShown = true; moveTo = false; }
    property bool moveTo: false
    readonly property bool menuIsBlock: !!ab && ab.draft.added.some(b => b.id === menuFor)
    readonly property var menuRows: {
        if (!ab || menuFor === "") return [];
        const rows = [["customize", "Customize…"], ["copylook", "Copy its look"]];
        if (ab.lookClip) rows.push(["pastelook", "Paste a look"]);
        if (menuIsBlock) rows.push(["duplicate", "Duplicate"]);
        rows.push(["moveto", menuIsBlock ? "Move to ›  " + ab.barIds.map(b => ab.labelOf(b)).concat(["the window"]).join(" · ") : "Move to ›  the front · the back"]);
        if (!menuIsBlock) rows.push(["hide", "Hide (keep it here)"]);
        rows.push(["out", menuIsBlock ? "Take it out" : "Take it out (hidden, in Blocks › Panes)"]);
        rows.push(["describe", "Describe it…"]);
        return rows;
    }
    function runMenu(what) {
        const id = menuFor;
        if (what === "moveto") { moveTo = true; return; }
        menuShown = false;
        if (what === "customize") ab.customize(id, "look");
        else if (what === "copylook") { ab.lookClip = L.lookPart(ab.draft.looks[id] || {}); note = ""; ab.toast("Its look is copied: right-click another part › Paste a look.", false); }
        else if (what === "pastelook") ab.withDraft(d => { d.looks[id] = L.pasteLook(d.looks[id], ab.lookClip); });
        else if (what === "duplicate") ab.withDraft(d => {
            const i = d.added.findIndex(b => b.id === id); if (i < 0) return;
            const n = Object.assign(L.copy(d.added[i]), { id: ab.newId() });
            if (!n.in) { n.x = Math.min(0.95, n.x + 0.01); n.y = Math.min(0.95, n.y + 0.01); }
            d.added.splice(i + 1, 0, n); if (d.looks[id]) d.looks[n.id] = L.copy(d.looks[id]);
        });
        else if (what === "hide") ab.hide(id);
        else if (what === "out") { if (menuIsBlock) ab.takeOut(id); else ab.hide(id); }
        else if (what === "describe") ab.describe();
    }
    function moveBlockTo(bar) {
        menuShown = false; moveTo = false;
        const id = menuFor;
        ab.withDraft(d => {
            const b = d.added.find(x => x.id === id); if (!b) return;
            if (bar === "") { b.in = ""; delete b.at; Object.assign(b, L.toFrac({ x: ew / 2 - 70, y: eh / 2 - 18, w: 140, h: 36 }, ew, eh)); }
            else { b.in = bar; b.at = 400; delete b.x; delete b.y; delete b.w; delete b.h; }      // at the bar's end
        });
    }

    // ================================================================== what you see while editing
    Item {
        anchors.fill: parent
        visible: ed.on

        // ---- every part outlined (dashed, accent), its label, its edges to size it
        // (a fixed number of outlines, each reading its part: the one being dragged is never made again under the hand)
        Repeater {
            model: 24
            delegate: Item {
                id: po
                required property int index
                readonly property var pm: ed.shown[index] || ({ id: "", x: 0, y: 0, w: 0, h: 0 })
                visible: index < ed.shown.length
                readonly property bool lit: ed.ab.picked === pm.id
                readonly property string kind: L.partKind(pm, ed.ew, ed.eh)
                x: pm.x; y: pm.y; width: pm.w; height: pm.h
                z: lit ? 3 : 1
                Rectangle { anchors.fill: parent; anchors.margins: 3; color: body.containsMouse || po.lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.05) : "transparent"; radius: 6 }
                Dashes { anchors.fill: parent; anchors.margins: 3; lit: po.lit || (ed.target && ed.target.kind === "bar" && ed.target.bar === po.pm.id) }
                Text {
                    // a part with nothing to show right now (Files' details pane while it's closed)
                    visible: { const it = ed.ab.parts[po.pm.id]; return !po.pm.pane && ed.ab.alive(it) && !it.visible; }
                    anchors.centerIn: parent
                    text: "Not showing now"
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                }
                MouseArea {
                    id: body
                    anchors.fill: parent; anchors.margins: 8
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    cursorShape: ed.moving && ed.moving.id === po.pm.id ? Qt.ClosedHandCursor : Qt.OpenHandCursor
                    preventStealing: true
                    // (a block added to this part, under the outline, is the one taken: it moves on its own)
                    onPressed: m => {
                        if (m.button !== Qt.LeftButton) return;
                        const q = mapToItem(ed, m.x, m.y), b = ed.blockAt(q);
                        if (b) ed.grabBlock(b, q); else ed.grabPart(po.pm.id, null, q);
                    }
                    onPositionChanged: m => { if (!pressed) return; const q = mapToItem(ed, m.x, m.y); if (ed.dragBlock) ed.moveDrag(q); else if (ed.moving) ed.movePart(q); }
                    onReleased: m => { if (m.button !== Qt.LeftButton) return; if (ed.dragBlock) ed.endDrag(); else ed.letGoPart(); }
                    onClicked: m => { if (m.button === Qt.RightButton) { const q = mapToItem(ed, m.x, m.y); ed.partMenu(ed.blockAt(q) || po.pm.id, q.x, q.y); } }
                    onDoubleClicked: ed.ab.customize(po.pm.id, "look")
                }
                // its edges and corners (the sides that can be dragged get a handle in the middle, as on the board)
                Repeater {
                    model: [{ l: 1 }, { r: 1 }, { t: 1 }, { b: 1 }, { r: 1, b: 1 }]
                    delegate: MouseArea {
                        id: edge
                        required property var modelData
                        readonly property bool corner: !!(modelData.r && modelData.b)
                        readonly property bool across: !!(modelData.l || modelData.r)
                        x: modelData.r ? parent.width - (corner ? 14 : 6) : modelData.l ? -2 : 14
                        y: modelData.b ? parent.height - (corner ? 14 : 6) : modelData.t ? -2 : 14
                        width: corner ? 18 : across ? 8 : Math.max(0, parent.width - 28)
                        height: corner ? 18 : !across ? 8 : Math.max(0, parent.height - 28)
                        hoverEnabled: true
                        preventStealing: true
                        cursorShape: corner ? Qt.SizeFDiagCursor : across ? Qt.SizeHorCursor : Qt.SizeVerCursor
                        onPressed: m => ed.grabPart(po.pm.id, modelData, mapToItem(ed, m.x, m.y))
                        onPositionChanged: m => { if (pressed) ed.movePart(mapToItem(ed, m.x, m.y)); }
                        onReleased: ed.letGoPart()
                        // (an edge on the window's own edge has nothing to give: no handle there)
                        readonly property bool inner: !(modelData.l && po.pm.x < 2) && !(modelData.r && po.pm.x + po.pm.w > ed.ew - 2)
                                                      && !(modelData.t && po.pm.y < 2) && !(modelData.b && po.pm.y + po.pm.h > ed.eh - 2)
                        Rectangle {
                            // a handle on the sides a part sizes by (a sidebar's or a pane's inner edge; any edge of the one picked)
                            visible: edge.inner && (edge.corner ? (po.lit || edge.containsMouse) : (po.kind === "side" && edge.across) || po.lit || edge.containsMouse)
                            anchors.centerIn: parent
                            width: 11; height: 11; radius: 2
                            color: Theme.accent; border.color: "#ffffff"; border.width: 2
                        }
                    }
                }
                // its name and size ("COMMAND BAR · 48 px"): under it, or inside it when it's at the bottom
                Tag {
                    text: L.partLabel(ed.ab.labelOf(po.pm.id), po.pm, ed.ew, ed.eh)
                    lit: po.lit
                    x: 4
                    y: po.pm.y + po.pm.h + 28 <= ed.eh ? po.height + 2 : po.kind === "bar" ? -height - 2 : po.height - height - 6
                }
            }
        }

        // ---- the window marked EDITING
        Rectangle {
            // (top right: clear of the app's own title and the tabs)
            x: ed.ew - width - 12; y: 6; z: 6
            width: edRow.implicitWidth + 20; height: 30; radius: 8
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.96); border.color: Theme.line
            Row {
                id: edRow
                anchors.centerIn: parent
                spacing: 10
                Rectangle { width: 14; height: 14; radius: 4; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
                Text { text: ed.ab ? ed.ab.appName + ((ed.ab.pages || []).length > 1 && ed.ab.def && ed.ab.def.title !== ed.ab.appName ? " — " + ed.ab.def.title : "") : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter }
                Rectangle {
                    width: edT.implicitWidth + 14; height: 20; radius: 5; color: Theme.accent
                    anchors.verticalCenter: parent.verticalCenter
                    Text { id: edT; anchors.centerIn: parent; text: "EDITING"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 1 }
                }
            }
        }

        // ---- snap guides, the drop line, the note
        Repeater {
            model: ed.guides
            delegate: Rectangle {
                required property var modelData
                z: 14
                color: "#5ec8ff"; opacity: 0.85
                x: modelData.x !== undefined ? modelData.x - 1 : 0; y: modelData.y !== undefined ? modelData.y - 1 : 0
                width: modelData.x !== undefined ? 2 : ed.ew; height: modelData.y !== undefined ? 2 : ed.eh
            }
        }
        Rectangle {
            z: 14
            visible: ed.lineX >= 0 && (ed.dragLib !== "" || ed.dragBlock !== "")
            x: ed.lineX; y: ed.lineY; width: 3; height: Math.max(12, ed.lineLen)
            radius: 2; color: Theme.accent
        }
        Rectangle {
            z: 15
            visible: ed.note !== ""
            readonly property bool dragging: ed.dragLib !== "" || ed.dragBlock !== "" || !!ed.moving
            x: Math.max(8, Math.min(ed.ew - width - 12, dragging ? (ed.moving && ed.moving.edges ? ed.dragAt.x - width - 6 : ed.dragAt.x + 26) : ed.ew / 2 - width / 2))
            y: dragging ? Math.min(ed.eh - 160, ed.dragAt.y + (ed.moving && ed.moving.edges ? 0 : 34)) : ed.eh - 150
            width: noteT.implicitWidth + 20; height: 30; radius: 6
            color: Qt.rgba(0.07, 0.12, 0.16, 0.95); border.color: "#5ec8ff"
            Text { id: noteT; anchors.centerIn: parent; text: ed.note; color: "#ffffff"; font.family: Theme.font; font.pixelSize: 13 }
        }

        // ---- the app's own Blocks window, moved by its title
        Rectangle {
            id: libWin
            z: 10
            width: Math.min(380, ed.ew - 40); height: Math.min(620, ed.eh - 220)
            x: ed.custShown && ed.ew - 380 - 560 - 40 >= 0 ? ed.ew - width - 560 - 40 : ed.ew - width - 30
            y: Math.min(160, Math.max(48, ed.eh - height - 150))
            radius: 14; color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.98); border.color: Theme.line
            clip: true
            Rectangle { z: -1; anchors.fill: parent; anchors.margins: -1; anchors.topMargin: 6; radius: 16; color: Qt.rgba(0, 0, 0, 0.35) }
            MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }
            Item {
                id: libTitle
                width: parent.width; height: 44
                Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.line }
                Row { anchors { left: parent.left; leftMargin: 16; verticalCenter: parent.verticalCenter } spacing: 10
                    Rectangle { width: 14; height: 14; radius: 7; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
                    Text { text: ed.ab ? ed.ab.shortAppName + (/s$/.test(ed.ab.shortAppName) ? "' blocks" : "'s blocks") : "Blocks"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold } }
                Text { anchors { right: parent.right; rightMargin: 16; verticalCenter: parent.verticalCenter } text: ed.ab ? "drag into " + ed.ab.shortAppName : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                MouseArea { anchors.fill: parent; cursorShape: Qt.SizeAllCursor; drag.target: libWin; drag.minimumX: 0; drag.minimumY: 0; drag.maximumX: ed.ew - libWin.width; drag.maximumY: ed.eh - 80 }
            }
            Flickable {
                anchors { left: parent.left; right: parent.right; top: libTitle.bottom; bottom: parent.bottom; margins: 16 }
                contentHeight: libCol.implicitHeight
                clip: true
                interactive: ed.dragLib === ""
                boundsBehavior: Flickable.StopAtBounds
                Column {
                    id: libCol
                    width: parent.width
                    spacing: 10
                    Rectangle {
                        width: parent.width; height: 36; radius: 10; color: Theme.bg; border.color: Theme.line
                        TextInput {
                            id: findIn
                            anchors { fill: parent; leftMargin: 28; rightMargin: 12 }
                            verticalAlignment: TextInput.AlignVCenter
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 13
                            onTextChanged: ed.libWords = text
                            Keys.onEscapePressed: { text = ""; if (ed.ab) ed.ab.focusEditKeys(); }
                        }
                        Text { anchors { left: parent.left; leftMargin: 12; verticalCenter: parent.verticalCenter } text: "⌕"; color: Theme.muted; font.pixelSize: 13 }
                        Text { visible: findIn.text === ""; anchors { left: parent.left; leftMargin: 28; verticalCenter: parent.verticalCenter } text: "Find a block (" + (ed.lib.length + 2) + ")"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                    }
                    Flow {
                        width: parent.width; spacing: 6
                        Repeater {
                            model: [["all", "All"], ["buttons", "Buttons"], ["panes", "Panes"], ["info", "Info"], ["own", "Your own"]]
                            delegate: Rectangle {
                                required property var modelData
                                readonly property bool on: ed.libKind === modelData[0]
                                width: kt.implicitWidth + 20; height: 28; radius: 8
                                color: on ? Theme.accent : "transparent"; border.color: on ? Theme.accent : Theme.line
                                Text { id: kt; anchors.centerIn: parent; text: parent.modelData[1]; color: parent.on ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: parent.on ? Font.DemiBold : Font.Normal }
                                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: ed.libKind = parent.modelData[0] }
                            }
                        }
                    }
                    Section { title: "BUTTONS"; kind: "buttons" }
                    Section { title: "PANES"; kind: "panes" }
                    Section { title: "INFO"; kind: "info" }
                    Section { title: "YOUR OWN"; kind: "own" }
                    Text { text: "MAKE YOUR OWN"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 1.2; visible: ed.libKind === "all" || ed.libKind === "own" }
                    Grid {
                        visible: ed.libKind === "all" || ed.libKind === "own"
                        columns: 2; spacing: 8
                        LibTile { label: "Button · runs anything"; icon: "rocket"; dashed: true; bid: "make:button" }
                        LibTile { label: "Count · words · spacer"; icon: "edit"; dashed: true; bid: "make:count" }
                    }
                    Item { width: 1; height: 4 }
                }
            }
        }

        // ---- Layers: every part and block, back to front: pick one, bring it forward or back, show or hide it
        Rectangle {
            z: 12
            visible: ed.layersShown
            x: Math.max(8, Math.min(ed.ew - width - 8, tool.x + layersBtn.x)); y: tool.y - height - 10
            width: 340; height: Math.min(ed.eh - 200, layersCol.implicitHeight + 24); radius: 12
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.98); border.color: Theme.line
            MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }
            Flickable {
                anchors { fill: parent; margins: 12 }
                contentHeight: layersCol.implicitHeight; clip: true; boundsBehavior: Flickable.StopAtBounds
                Column {
                    id: layersCol
                    width: parent.width
                    spacing: 4
                    Text { text: "LAYERS · FRONT FIRST"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 1.2 }
                    Repeater {
                        model: {
                            if (!ed.ab) return [];
                            const ids = ed.ab.draft.parts.map(p => p.id).concat(ed.ab.draft.added.map(b => b.id));
                            return ids.slice().sort((a, b) => ed.ab.draft.z.indexOf(b) - ed.ab.draft.z.indexOf(a));
                        }
                        delegate: Rectangle {
                            required property string modelData
                            readonly property var part: ed.ab.draft.parts.find(p => p.id === modelData) || null
                            width: layersCol.width; height: 32; radius: 8
                            color: ed.ab.picked === modelData ? Theme.panel3 : "transparent"
                            Text { anchors { left: parent.left; leftMargin: 10; right: lr.left; verticalCenter: parent.verticalCenter } elide: Text.ElideRight
                                   text: ed.ab.labelOf(parent.modelData) + (parent.part ? "" : " · block") + (parent.part && parent.part.hidden ? " · hidden" : "")
                                   color: parent.part && parent.part.hidden ? Theme.muted : Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                            MouseArea { anchors.fill: parent; onClicked: ed.ab.picked = parent.modelData }
                            Row {
                                id: lr
                                anchors { right: parent.right; rightMargin: 6; verticalCenter: parent.verticalCenter }
                                spacing: 4
                                Mini { label: "↑"; onClicked: ed.ab.raise(parent.parent.modelData, true) }
                                Mini { label: "↓"; onClicked: ed.ab.raise(parent.parent.modelData, false) }
                                Mini { visible: !!parent.parent.part; label: parent.parent.part && parent.parent.part.hidden ? "Show" : "Hide"; onClicked: { if (parent.parent.part.hidden) ed.ab.show(parent.parent.modelData); else ed.ab.hide(parent.parent.modelData); } }
                                Mini { visible: !parent.parent.part; label: "Take out"; danger: true; onClicked: ed.ab.takeOut(parent.parent.modelData) }
                            }
                        }
                    }
                }
            }
        }
        // ---- Start from: ready-made layouts for this app
        Rectangle {
            z: 12
            visible: ed.startShown
            x: Math.max(8, Math.min(ed.ew - width - 8, tool.x + startBtn.x)); y: tool.y - height - 10
            width: 300; height: startCol.implicitHeight + 24; radius: 12
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.98); border.color: Theme.line
            MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }
            Column {
                id: startCol
                anchors { fill: parent; margins: 12 }
                spacing: 4
                Text { width: parent.width; wrapMode: Text.WordWrap; text: "Puts that layout in place to start from. Undo or Cancel brings yours back."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                Repeater {
                    model: ed.ab && ed.startShown ? L.presets(ed.ab.measureOwn(), ed.ew, ed.eh) : []
                    delegate: Rectangle {
                        required property var modelData
                        width: startCol.width; height: 40; radius: 8; color: sa.containsMouse ? Theme.panel3 : "transparent"
                        Column { anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter }
                            Text { text: parent.parent.modelData.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                            Text { text: parent.parent.modelData.note; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 } }
                        MouseArea { id: sa; anchors.fill: parent; hoverEnabled: true; onClicked: { ed.startShown = false; ed.startName = parent.modelData.name; ed.ab.startFrom(parent.modelData); } }
                    }
                }
            }
        }

        // ---- the toolbar
        Text {
            z: 11
            anchors { horizontalCenter: parent.horizontalCenter; bottom: tool.top; bottomMargin: 14 }
            text: "THE SAME IN EVERY HEARTH APP · SETTINGS · TASK MANAGER · CAPTURE · NOTES · STORE … · RIGHT-CLICK ANY PART › CUSTOMIZE"
            width: Math.min(implicitWidth, ed.ew - 40); elide: Text.ElideMiddle
            color: Theme.muted; font.family: Theme.font; font.pixelSize: ed.tight ? 10 : 11; font.weight: Font.Bold; font.letterSpacing: ed.tight ? 0.6 : 1.2
            Rectangle { z: -1; anchors { fill: parent; margins: -6 } radius: 6; color: Theme.panel2; border.color: Theme.line }
        }
        Rectangle {
            id: tool
            z: 11
            anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 22 }
            width: toolRow.implicitWidth + 20; height: 56; radius: 16
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.98); border.color: Theme.line
            MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }
            Row {
                id: toolRow
                anchors.centerIn: parent
                spacing: ed.tight ? 5 : 8
                Btn { label: "↶ Undo"; enabled: !!ed.ab && ed.ab.undoDraft.length > 0; onClicked: ed.ab.undo() }
                Btn { label: "↷ Redo"; enabled: !!ed.ab && ed.ab.redoDraft.length > 0; onClicked: ed.ab.redo() }
                Sep {}
                Btn { label: "Snap to grid" + (ed.snapOn ? " · " + ed.grid + " px" : ""); icon: "grid"; on: ed.snapOn; onClicked: ed.snapOn = !ed.snapOn }
                Btn { label: "┼ Guides"; on: ed.guidesOn; onClicked: ed.guidesOn = !ed.guidesOn }
                Btn { id: layersBtn; label: "◫ Layers"; on: ed.layersShown; onClicked: { ed.layersShown = !ed.layersShown; ed.startShown = false; } }
                Sep {}
                Btn { id: startBtn; label: "Start from: " + ed.startName + " ▾"; on: ed.startShown; onClicked: { ed.startShown = !ed.startShown; ed.layersShown = false; } }
                Btn { label: "Menus…"; on: !!ed.ab && ed.ab.menuEditor; onClicked: { if (ed.ab.menuEditor) ed.ab.menuEditor = false; else ed.ab.openMenus(); } }
                Sep {}
                Btn { label: "Cancel"; onClicked: ed.ab.cancel() }
                Btn { label: "Try it"; onClicked: ed.ab.tryIt() }
                Btn { label: "Keep this"; primary: true; onClicked: ed.ab.keepThis() }
            }
        }

        // ---- the block being dragged: it follows the pointer, tilted, with a shadow
        Rectangle {
            z: 20
            visible: (ed.dragLib !== "" && !ed.isPaneDrag()) || ed.dragBlock !== ""
            x: ed.dragAt.x - 30; y: ed.dragAt.y - height - 8
            rotation: -2.5
            width: ghostRow.implicitWidth + 28; height: 38; radius: 9
            color: Theme.panel2; border.color: Theme.accent; border.width: 1
            Rectangle { z: -1; anchors.fill: parent; anchors.topMargin: 8; anchors.leftMargin: 3; radius: 11; color: Qt.rgba(0, 0, 0, 0.45) }
            Row { id: ghostRow; anchors.centerIn: parent; spacing: 8
                Rectangle { width: 16; height: 16; radius: 4; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter
                    Icon { anchors.centerIn: parent; name: ed.dragIcon; size: 12; color: Theme.accentInk } }
                Text { text: ed.dragName; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; anchors.verticalCenter: parent.verticalCenter } }
        }
        // a pane being dragged in: where it lands
        Rectangle {
            z: 19
            visible: ed.isPaneDrag() && !!ed.target && ed.target.kind === "window"
            x: ed.target && ed.target.x !== undefined ? ed.target.x : 0; y: ed.target && ed.target.y !== undefined ? ed.target.y : 0
            width: ed.target && ed.target.w ? ed.target.w : 0; height: ed.target && ed.target.h ? ed.target.h : 0
            radius: 10; color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.14); border.color: Theme.accent; border.width: 2
            Text { anchors.centerIn: parent; text: ed.dragName; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
        }

        // ---- a part's or a block's menu
        MouseArea { anchors.fill: parent; z: 30; visible: ed.menuShown; acceptedButtons: Qt.LeftButton | Qt.RightButton; onClicked: ed.menuShown = false }
        Rectangle {
            z: 31
            visible: ed.menuShown
            width: 300; height: emCol.implicitHeight + 16
            x: Math.max(8, Math.min(ed.ew - width - 8, ed.menuAt.x)); y: Math.max(8, Math.min(ed.eh - height - 8, ed.menuAt.y))
            radius: 12; color: Theme.panel; border.color: Theme.line
            Column {
                id: emCol
                x: 8; y: 8; width: parent.width - 16
                Text { width: parent.width; leftPadding: 10; topPadding: 4; bottomPadding: 6; text: ed.ab ? ed.ab.labelOf(ed.menuFor) : ""; elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                Repeater {
                    model: ed.moveTo ? [] : ed.menuRows
                    delegate: Rectangle {
                        required property var modelData
                        width: emCol.width; height: 34; radius: 8
                        color: emM.containsMouse ? Theme.panel3 : "transparent"
                        Text { x: 12; width: parent.width - 20; elide: Text.ElideRight; anchors.verticalCenter: parent.verticalCenter; text: parent.modelData[1]; color: parent.modelData[0] === "out" ? Theme.danger : Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                        MouseArea { id: emM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: ed.runMenu(parent.modelData[0]) }
                    }
                }
                Repeater {
                    model: !ed.moveTo ? [] : ed.menuIsBlock ? ed.ab.barIds.concat([""]) : ["front", "back"]
                    delegate: Rectangle {
                        required property string modelData
                        width: emCol.width; height: 34; radius: 8
                        color: mtM.containsMouse ? Theme.panel3 : "transparent"
                        Text { x: 12; anchors.verticalCenter: parent.verticalCenter; text: "Move to › " + (parent.modelData === "front" ? "the front" : parent.modelData === "back" ? "the back" : parent.modelData === "" ? "the window (anywhere)" : ed.ab.labelOf(parent.modelData)); color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                        MouseArea { id: mtM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                            onClicked: { if (ed.menuIsBlock) ed.moveBlockTo(parent.modelData); else { ed.menuShown = false; ed.ab.raise(ed.menuFor, parent.modelData === "front"); } } }
                    }
                }
            }
        }

        // ---- Right-click any part › Customize: board 2's side panel, for this part
        Loader {
            id: cust
            z: 25
            active: ed.on && !!ed.ab && ed.ab.customizeFor !== ""
            width: Math.min(560, ed.ew - 40)
            height: Math.max(300, ed.eh - 130 - 24)
            x: ed.ew - width - 16; y: 16
            Component.onCompleted: setSource("AppBlockCustomize.qml", { ab: ed.ab })
            onStatusChanged: if (status === Loader.Error) { console.warn("AppBlocks: Customize couldn't load (AppBlockCustomize.qml)"); if (ed.ab) ed.ab.customizeFor = ""; }
        }
    }
    readonly property bool custShown: cust.status === Loader.Ready && cust.active

    // ---- trying it: the app as it'll be, with a way back
    Rectangle {
        visible: !!ed.ab && ed.ab.trying
        z: 50
        anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 22 }
        width: tryRow.implicitWidth + 24; height: 52; radius: 16
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.98); border.color: Theme.accent
        Row {
            id: tryRow
            anchors.centerIn: parent
            spacing: 10
            Text { text: ed.ab ? "Trying " + ed.ab.shortAppName : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; anchors.verticalCenter: parent.verticalCenter }
            Btn { label: "Back to editing"; onClicked: ed.ab.backToEditing() }
            Btn { label: "Go back"; onClicked: ed.ab.goBack() }
            Btn { label: "Keep this"; primary: true; onClicked: ed.ab.keepThis() }
        }
    }

    // ================================================================== small parts
    component LibTile: Rectangle {
        id: lt
        property string bid: ""
        property string label: ""
        property string icon: ""
        property bool dashed: false
        width: (libCol.width - 8) / 2; height: 62; radius: 10
        color: ltM.containsMouse ? Theme.panel3 : Theme.panel2; border.color: dashed ? "transparent" : Theme.line
        Dashes { visible: lt.dashed; anchors.fill: parent; faint: true }
        Column {
            anchors.centerIn: parent
            spacing: 6
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                width: 20; height: 20; radius: 4; color: lt.dashed ? Theme.accent : Qt.rgba(1, 1, 1, 0.82)
                Icon { anchors.centerIn: parent; name: lt.icon || "dot"; size: 13; color: lt.dashed ? Theme.accentInk : "#2a201a" }
            }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: lt.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 12; width: Math.min(implicitWidth, lt.width - 12); elide: Text.ElideRight; horizontalAlignment: Text.AlignHCenter }
        }
        MouseArea {
            id: ltM
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.OpenHandCursor
            preventStealing: true
            onPressed: m => ed.grabLib(lt.bid, mapToItem(ed, m.x, m.y))
            onPositionChanged: m => { if (pressed) ed.moveDrag(mapToItem(ed, m.x, m.y)); }
            onReleased: ed.endDrag()
        }
    }
    component Section: Column {
        id: sec
        property string title: ""
        property string kind: ""
        readonly property var items: ed.shownLib.filter(b => b.kind === kind)
        visible: items.length > 0
        width: libCol.width
        spacing: 8
        Text { text: sec.title; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 1.2 }
        Grid {
            columns: 2; spacing: 8
            Repeater { model: sec.items; delegate: LibTile { required property var modelData; bid: modelData.id; label: modelData.name; icon: modelData.icon } }
        }
    }
    component Tag: Rectangle {
        property alias text: tt.text
        property bool lit: false
        width: tt.implicitWidth + 16; height: 20; radius: 4
        color: Theme.accent
        border.color: lit ? "#ffffff" : "transparent"
        Text { id: tt; anchors.centerIn: parent; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.Bold; font.letterSpacing: 0.9 }
    }
    // the dashed outline (accent), brighter when it's picked or a block would land in it
    component Dashes: Canvas {
        id: dc
        property bool lit: false
        property bool faint: false
        onLitChanged: requestPaint()
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onPaint: {
            const g = getContext("2d");
            g.reset();
            g.strokeStyle = faint ? Theme.muted : Theme.accent;
            g.globalAlpha = faint ? 0.55 : lit ? 1 : 0.8;
            g.lineWidth = lit ? 3 : 2;
            g.setLineDash([8, 5]);
            const r = faint ? 10 : 8;
            g.beginPath();
            g.moveTo(r, 1); g.lineTo(width - r, 1); g.arcTo(width - 1, 1, width - 1, r, r - 1);
            g.lineTo(width - 1, height - r); g.arcTo(width - 1, height - 1, width - r, height - 1, r - 1);
            g.lineTo(r, height - 1); g.arcTo(1, height - 1, 1, height - r, r - 1);
            g.lineTo(1, r); g.arcTo(1, 1, r, 1, r - 1);
            g.closePath();
            g.stroke();
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
        width: br.implicitWidth + (ed.tight ? 18 : 28); height: ed.tight ? 34 : 38; radius: 10
        opacity: enabled ? 1 : 0.45
        color: primary ? Theme.accent : on ? Theme.panel3 : ba.containsMouse ? Theme.panel3 : Theme.panel2
        border.color: primary ? Theme.accent : on ? Theme.accent : Theme.line
        Row { id: br; anchors.centerIn: parent; spacing: 8
            Icon { visible: b.icon !== ""; name: b.icon || "dot"; size: 16; color: b.primary ? Theme.accentInk : Theme.text; anchors.verticalCenter: parent.verticalCenter }
            Text { text: b.label; color: b.primary ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: ed.tight ? 13 : 14; font.weight: b.primary ? Font.DemiBold : Font.Medium; anchors.verticalCenter: parent.verticalCenter } }
        MouseArea { id: ba; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: if (b.enabled) b.clicked() }
    }
    component Mini: Rectangle {
        id: mi
        property string label: ""
        property bool danger: false
        signal clicked()
        width: mt.implicitWidth + 14; height: 24; radius: 6
        color: mm.containsMouse ? Theme.panel3 : Theme.bg2; border.color: Theme.line
        Text { id: mt; anchors.centerIn: parent; text: mi.label; color: mi.danger ? Theme.danger : Theme.text; font.family: Theme.font; font.pixelSize: 11 }
        MouseArea { id: mm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: mi.clicked() }
    }
}

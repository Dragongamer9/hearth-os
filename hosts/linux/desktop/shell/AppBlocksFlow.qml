// A page of Hearth Settings made of blocks (AppBlocks.qml's way, for a page that scrolls): each titled group of cards
// (its SetHeader and the cards under it) is a block. Right-click a group: Edit this page · Move up · Move down · Hide
// this group · Describe it… · Back to the original. Editing it in place: each group outlined with its name; drag a
// group by its name up or down (the others make room as you go), ✕ hides it, the hidden ones wait in the tray to be
// put back; Undo, Back to the original, Done. Kept as you go, through Settings' AppBlocks (hearth-layout app-keep),
// as the page "mantel-page-<page>": {group: {x: 0, y: its place, w: 24, h: 1, hidden}} (only what's hidden, when
// nothing was moved). Keys and the controller: Up / Down pick a group, Shift+Up / Down or LT / RT move it, X hides it,
// B is done.
//
// Nothing kept for the page: nothing here does anything (no group is looked for until a right-click or Edit).
import QtQuick
import Quickshell
import "app-blocks.js" as L

Item {
    id: fl
    property Flickable page: null
    property Item col: null
    property var skip: []                  // the column's own first items (not a group)
    property string blockPage: ""
    property string pageTitle: ""
    readonly property var ctl: { void AppBlocksHub.rev; return AppBlocksHub.get("settings"); }
    readonly property string pid: "mantel-page-" + L.slug(blockPage)
    // (over the page's content, in its coordinates: SetPage's Loader sits at 0, 0 of the content, above the cards)

    // ---------------------------------------------------------------- the groups
    property var orig: []                  // the groups' ids in the page's own order
    property var titles: ({})
    property var parked: ({})              // group id → its items, while it's hidden
    Item { id: park; visible: false }
    function isHeader(it) { return !!it && it.isSetHeader === true; }
    function scan() {
        const out = [];
        if (!col) return out;
        let cur = null;
        const kids = col.children;
        for (let i = 0; i < kids.length; i++) {
            const it = kids[i];
            if (skip.indexOf(it) >= 0) continue;
            if (isHeader(it)) { cur = { id: "", title: it.text, header: it, items: [it] }; out.push(cur); continue; }
            if (!cur) { cur = { id: "top", title: "The top of the page", header: null, items: [] }; out.push(cur); }
            cur.items.push(it);
        }
        const seen = {};
        for (const s of out) {
            if (s.id === "") { let b = L.slug(s.title), id = b, n = 2; while (seen[id]) id = b + "-" + n++; s.id = id; }
            seen[s.id] = true;
        }
        return out;
    }
    function ensureOrig() {
        if (orig.length) return;
        const s = scan();
        const t = {};
        for (const x of s) t[x.id] = x.title;
        orig = s.map(x => x.id);
        titles = t;
        if (ctl) ctl.extraPages = [catalogPage()];
    }
    function defs() { return orig.slice(0, L.ROWS).map((id, i) => ({ id: id, title: titles[id] || id, x: 0, y: i, w: L.COLS, h: 1 })); }
    function catalogPage() { return { id: pid, title: "Settings › " + (pageTitle || blockPage), blocks: defs() }; }
    Component.onDestruction: {
        if (ctl && ctl.extraPages.length && ctl.extraPages[0].id === pid) ctl.extraPages = [];
        if (AppBlocksHub.flow === fl) AppBlocksHub.flow = null;
    }

    // what's kept → the groups' order and which are hidden
    readonly property var saved: ctl && ctl.layoutKept.blocks ? ctl.layoutKept.blocks[pid] : undefined
    property bool everApplied: false
    onSavedChanged: Qt.callLater(fl.applyKept)
    onColChanged: Qt.callLater(fl.applyKept)
    function wanted() {
        const d = defs();
        const r = L.resolve(d, saved, {}, L.COLS, L.ROWS);
        if (r.mode === "own") return { own: true, order: orig.slice(), hidden: {} };
        const idx = {}; d.forEach((x, i) => idx[x.id] = i);
        const pl = r.placed.slice().sort((a, b) => (a.y - b.y) || (idx[a.id] - idx[b.id]));
        const hid = {};
        for (const p of r.placed) if (p.hidden) hid[p.id] = true;
        return { own: false, order: pl.map(p => p.id).concat(orig.slice(L.ROWS)), hidden: hid };
    }
    function applyKept() {
        if (!col) return;
        if (saved === undefined && !everApplied && !editing) return;      // nothing kept, nothing ever moved: as it is
        ensureOrig();
        const w = wanted();
        apply(editing ? order : w.order, editing ? hidden : w.hidden);
    }
    function apply(ord, hid) {
        everApplied = true;
        for (const id in parked) if (!hid[id]) { for (const it of parked[id]) it.parent = col; }
        const keep = {};
        for (const id in parked) if (hid[id]) keep[id] = parked[id];
        parked = keep;
        const secs = scan();
        const p2 = Object.assign({}, parked);
        for (const s of secs) if (hid[s.id]) { for (const it of s.items) it.parent = park; p2[s.id] = s.items; }
        parked = p2;
        let prev = skip.length ? skip[skip.length - 1] : null;
        for (const id of ord) {
            const s = secs.find(x => x.id === id);
            if (!s || hid[id]) continue;
            for (const it of s.items) { if (prev) it.stackAfter(prev); prev = it; }
        }
        rev++;
    }
    property int rev: 0

    // ---------------------------------------------------------------- editing it
    property bool editing: false
    property var order: []
    property var hidden: ({})
    property string picked: ""
    readonly property var shownSecs: { void rev; void editing; return editing ? scan() : []; }
    function edit(pick) {
        ensureOrig();
        const w = wanted();
        order = w.order; hidden = w.hidden; editing = true;
        picked = pick || order.find(id => !hidden[id]) || "";
        menuShown = false;
        keys.forceActiveFocus();
        apply(order, hidden);
    }
    function done() { menuShown = false; editing = false; picked = ""; dragId = ""; if (page && page.focusFirst) page.focusFirst(); }
    function pageJson(ord, hid) {
        const same = ord.length === orig.length && ord.every((id, i) => id === orig[i]);
        const anyHid = Object.keys(hid).some(k => hid[k]);
        if (same && !anyHid) return {};
        const m = {};
        ord.slice(0, L.ROWS).forEach((id, i) => { const o = same ? {} : { x: 0, y: i, w: L.COLS, h: 1 }; if (hid[id]) o.hidden = true; m[id] = o; });
        return m;
    }
    function change(ord, hid) {
        if (!ctl) return;
        ctl.extraPages = [catalogPage()];
        ctl.pushUndo({ page: pid, before: saved === undefined ? null : L.copy(saved), draft: null });
        order = ord; hidden = hid;
        apply(ord, hid);
        ctl.keepFor(pid, pageJson(ord, hid));
    }
    function moveBy(id, d) {
        ensureOrig();
        const w = editing ? { order: order, hidden: hidden } : wanted();
        const vis = w.order.filter(x => !w.hidden[x]);
        const i = vis.indexOf(id), j = i + d;
        if (i < 0 || j < 0 || j >= vis.length) return;
        const other = vis[j];
        const ord = w.order.slice();
        const a = ord.indexOf(id), b = ord.indexOf(other);
        ord[a] = other; ord[b] = id;
        change(ord, w.hidden);
    }
    function hide(id) {
        ensureOrig();
        const w = editing ? { order: order, hidden: hidden } : wanted();
        if (w.order.filter(x => !w.hidden[x] && x !== id).length === 0) return;
        const h = Object.assign({}, w.hidden); h[id] = true;
        if (picked === id) picked = w.order.find(x => !h[x]) || "";
        change(w.order, h);
        if (ctl) ctl.toast((titles[id] || id) + " is hidden. Right-click the page › Edit this page brings it back.", true);
    }
    function show(id) { const h = Object.assign({}, hidden); delete h[id]; picked = id; change(order, h); }
    function reset() {
        if (!ctl) return;
        ctl.pushUndo({ page: pid, before: saved === undefined ? null : L.copy(saved), draft: null });
        order = orig.slice(); hidden = {};
        apply(order, hidden);
        ctl.keepFor(pid, {});
        ctl.toast("This page is back to the original. Undo is here.", true);
    }
    function secAt(y) {
        const secs = scan();
        for (const s of secs) { const r = rectOf(s); if (y >= r.y && y < r.y + r.h) return s.id; }
        return secs.length ? secs[secs.length - 1].id : "";
    }
    function rectOf(s) {
        let t = 1e9, b = -1e9;
        for (const it of s.items) { if (!it.visible || it.height <= 0) continue; t = Math.min(t, it.y); b = Math.max(b, it.y + it.height); }
        if (t > b) return { y: 0, h: 0 };
        return { y: col.y + t - 4, h: b - t + 8 };
    }

    // drag a group up or down: it takes the place of the one it's over as you go
    property string dragId: ""
    property real dragY: 0
    function dragTo(y) {
        dragY = y;
        const over = secAt(y);
        if (over === "" || over === dragId) return;
        const vis = order.filter(x => !hidden[x]);
        const i = vis.indexOf(dragId), j = vis.indexOf(over);
        if (i < 0 || j < 0) return;
        const ord = order.slice();
        const a = ord.indexOf(dragId);
        ord.splice(a, 1);
        ord.splice(ord.indexOf(over) + (j > i ? 1 : 0), 0, dragId);
        order = ord;
        apply(order, hidden);
    }
    property var dragBefore: null
    function grab(id) { picked = id; dragId = id; dragBefore = order.slice(); keys.forceActiveFocus(); }
    function release() {
        if (dragId === "") return;
        dragId = "";
        const before = dragBefore; dragBefore = null;
        if (before && before.join() !== order.join()) {
            const now = order.slice();
            order = before;
            change(now, hidden);
        }
    }

    FocusScope {
        id: keys
        focus: fl.editing
        Keys.onPressed: e => {
            if (!fl.editing) return;
            const shift = (e.modifiers & Qt.ShiftModifier) !== 0, ctrl = (e.modifiers & Qt.ControlModifier) !== 0;
            const vis = fl.order.filter(x => !fl.hidden[x]);
            const i = vis.indexOf(fl.picked);
            if (ctrl && e.key === Qt.Key_Z) { if (fl.ctl) fl.ctl.undo(); }
            else if (e.key === Qt.Key_Escape || e.key === Qt.Key_Return || e.key === Qt.Key_Enter) fl.done();
            else if ((shift && e.key === Qt.Key_Up) || e.key === Qt.Key_PageUp) fl.moveBy(fl.picked, -1);
            else if ((shift && e.key === Qt.Key_Down) || e.key === Qt.Key_PageDown) fl.moveBy(fl.picked, 1);
            else if (e.key === Qt.Key_Up || e.key === Qt.Key_Backtab) fl.picked = vis[Math.max(0, i - 1)] || fl.picked;
            else if (e.key === Qt.Key_Down || e.key === Qt.Key_Tab) fl.picked = vis[Math.min(vis.length - 1, i + 1)] || fl.picked;
            else if (e.key === Qt.Key_Menu || e.key === Qt.Key_Delete) fl.hide(fl.picked);
            else return;
            e.accepted = true;
        }
    }

    // ---------------------------------------------------------------- right-click on the page
    Component {
        id: catcherComp
        MouseArea {
            z: -1
            acceptedButtons: Qt.RightButton
            onClicked: m => { fl.ensureOrig(); fl.menuFor = fl.secAt(m.y + y); fl.menuAt = Qt.point(m.x + x, m.y + y); fl.menuShown = fl.menuFor !== ""; }
        }
    }
    property Item catcher: null
    Component.onCompleted: Qt.callLater(() => {
        AppBlocksHub.flow = fl;
        if (!fl.page || !fl.col || fl.catcher) return;
        fl.catcher = catcherComp.createObject(fl.page.contentItem, { x: Qt.binding(() => fl.col.x), y: Qt.binding(() => fl.col.y), width: Qt.binding(() => fl.col.width), height: Qt.binding(() => fl.col.height) });
    })
    property bool menuShown: false
    property string menuFor: ""
    property point menuAt: Qt.point(0, 0)
    function run(what) {
        menuShown = false;
        if (what === "edit") edit(menuFor);
        else if (what === "up") moveBy(menuFor, -1);
        else if (what === "down") moveBy(menuFor, 1);
        else if (what === "hide") hide(menuFor);
        else if (what === "describe") { if (ctl) ctl.describe(); }
        else if (what === "window") { if (ctl) ctl.edit("page"); }
        else if (what === "reset") reset();
    }
    MouseArea {
        x: 0; y: 0; width: fl.page ? fl.page.contentWidth : 0; height: fl.page ? Math.max(fl.page.contentHeight, fl.page.height) : 0
        visible: fl.menuShown
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: fl.menuShown = false
    }
    Rectangle {
        visible: fl.menuShown
        z: 20
        width: 250; height: fmCol.implicitHeight + 16
        x: Math.max(8, Math.min((fl.page ? fl.page.width : 800) - width - 8, fl.menuAt.x))
        y: fl.menuAt.y
        radius: Theme.r(12); color: Theme.panel; border.color: Theme.line
        Column {
            id: fmCol
            x: 8; y: 8; width: parent.width - 16
            Text { width: parent.width; leftPadding: 10; topPadding: 4; bottomPadding: 6; text: fl.titles[fl.menuFor] || ""; elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
            Repeater {
                model: [["edit", "Edit this page"], ["up", "Move up"], ["down", "Move down"], ["hide", "Hide this group"], ["describe", "Describe it…"], ["window", "Edit Settings' layout"]].concat(fl.saved !== undefined ? [["reset", "Back to the original"]] : [])
                delegate: Rectangle {
                    required property var modelData
                    width: fmCol.width; height: 34; radius: Theme.r(8)
                    color: fmM.containsMouse ? Theme.accent : "transparent"
                    Text { x: 12; anchors.verticalCenter: parent.verticalCenter; text: parent.modelData[1]; color: fmM.containsMouse ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                    MouseArea { id: fmM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: fl.run(parent.modelData[0]) }
                }
            }
        }
    }

    // ---------------------------------------------------------------- the groups outlined while editing
    Repeater {
        model: fl.shownSecs
        delegate: Item {
            id: sg
            required property var modelData
            readonly property var r: { void fl.rev; return fl.rectOf(modelData); }
            readonly property bool lit: fl.picked === modelData.id
            x: fl.col ? fl.col.x - 8 : 0
            y: r.y
            width: fl.col ? fl.col.width + 16 : 0
            height: r.h
            visible: fl.editing && !fl.hidden[modelData.id] && r.h > 0
            Rectangle {
                anchors.fill: parent
                radius: 8
                color: sgM.containsMouse || sg.lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.07) : "transparent"
                border.width: sg.lit ? 2.5 : 1.5
                border.color: sg.lit ? Theme.accent : Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.5)
            }
            MouseArea {
                id: sgM
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: fl.dragId === sg.modelData.id ? Qt.ClosedHandCursor : Qt.OpenHandCursor
                onPressed: fl.grab(sg.modelData.id)
                onPositionChanged: m => { if (pressed) fl.dragTo(mapToItem(fl, m.x, m.y).y); }
                onReleased: fl.release()
            }
            Rectangle {
                x: 8; y: -13
                width: sgRow.implicitWidth + 16; height: 26; radius: 13
                color: sg.lit ? Theme.accent : Theme.panel3
                border.color: sg.lit ? "transparent" : Theme.line
                Row {
                    id: sgRow
                    anchors.centerIn: parent
                    spacing: 8
                    Text { text: "⠿  " + (sg.modelData.title || "The top of the page"); color: sg.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter }
                    Text { text: "↑"; color: sg.lit ? Theme.accentInk : Theme.muted; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter
                        MouseArea { anchors { fill: parent; margins: -5 } cursorShape: Qt.PointingHandCursor; onClicked: fl.moveBy(sg.modelData.id, -1) } }
                    Text { text: "↓"; color: sg.lit ? Theme.accentInk : Theme.muted; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter
                        MouseArea { anchors { fill: parent; margins: -5 } cursorShape: Qt.PointingHandCursor; onClicked: fl.moveBy(sg.modelData.id, 1) } }
                    Text { text: "✕"; color: sg.lit ? Theme.accentInk : Theme.muted; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter
                        MouseArea { anchors { fill: parent; margins: -5 } cursorShape: Qt.PointingHandCursor; onClicked: fl.hide(sg.modelData.id) } }
                }
            }
        }
    }
    // the bar, at the bottom of what's in view
    Rectangle {
        visible: fl.editing
        z: 30
        x: fl.col ? fl.col.x : 16
        width: fl.col ? fl.col.width : 600
        y: fl.page ? fl.page.contentY + fl.page.height - height - 14 : 0
        height: fbCol.implicitHeight + 20
        radius: Theme.r(14); color: Theme.panel; border.color: Theme.accent; border.width: 2
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }
        Column {
            id: fbCol
            x: 12; y: 10; width: parent.width - 24
            spacing: 8
            Flow {
                width: parent.width; spacing: 8
                Text { height: 30; verticalAlignment: Text.AlignVCenter; text: "Editing this page · drag a group up or down, ✕ hides it"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                FBtn { label: "Describe it…"; onClicked: { if (fl.ctl) fl.ctl.describe(); } }
                FBtn { label: "Undo"; onClicked: { if (fl.ctl) fl.ctl.undo(); } }
                FBtn { label: "Back to the original"; onClicked: fl.reset() }
                FBtn { label: "Done"; primary: true; onClicked: fl.done() }
            }
            Flow {
                width: parent.width; spacing: 8
                visible: Object.keys(fl.hidden).some(k => fl.hidden[k])
                Text { height: 28; verticalAlignment: Text.AlignVCenter; text: "Hidden:"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                Repeater {
                    model: Object.keys(fl.hidden).filter(k => fl.hidden[k])
                    delegate: FBtn { required property string modelData; label: "+ " + (fl.titles[modelData] || modelData); onClicked: fl.show(modelData) }
                }
            }
        }
    }
    component FBtn: Rectangle {
        id: fb
        property string label
        property bool primary: false
        signal clicked()
        width: fbT.implicitWidth + 24; height: 30; radius: Theme.r(8)
        color: primary ? Theme.accent : fbM.containsMouse ? Theme.panel3 : Theme.bg2
        border.color: primary ? "transparent" : Theme.line
        Text { id: fbT; anchors.centerIn: parent; text: fb.label; color: fb.primary ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 13 }
        MouseArea { id: fbM; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: fb.clicked() }
    }
}

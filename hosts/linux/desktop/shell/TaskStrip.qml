// Your apps inside the main bar (Islands.qml), for Settings › Appearance › Bar and dock with the floating app menu
// off: the dock's apps attached to the bar, across it (top / bottom) or stacked down it (left / right).
// The same behaviour as the dock (Dock.qml) and the classic taskbar (Bar.qml): pinned apps first (DeskState,
// ~/.config/nimbus/desk.json), windows of the same app share a button; click focuses (again: minimizes; several
// windows: a list to pick from), middle-click opens another one, drag a pinned app to move it, right-click (X on
// the controller) for the app's own actions, Pin / Unpin, Move to desktop, Maximize and Close. Super+1-9 open the
// app at that place (DeskState.mainBar is this strip while it's in use). The thin strip at the end shows the desktop.
// Loaded by Islands.qml through a Loader: if it can't load, the bar still has the app menu button and Game Mode.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Item {
    id: strip
    property var menu: null                 // PopMenu
    property var screen: null
    property bool vertical: false
    property string edge: "bottom"          // the bar's edge: menus open away from it
    property int side: 44                   // a button's size
    property bool primary: false
    property real barW: 0                   // the bar window's size (its position follows from the edge)
    property real barH: 0
    readonly property int iconSize: Math.round(side * 0.68)
    implicitWidth: vertical ? side : lay.implicitWidth
    implicitHeight: vertical ? lay.implicitHeight : side

    function claim() { if (primary) DeskState.mainBar = strip; }
    onPrimaryChanged: claim()
    Component.onCompleted: claim()
    Component.onDestruction: if (DeskState.mainBar === strip) DeskState.mainBar = null

    // ------------------------------------------------------------------ apps (as Dock.qml)
    function ownWindow(t) {
        const a = t.appId || "";
        return /quickshell/i.test(a) || (a === "" && /^(Hearth Settings|Task Manager|Files)$/.test(t.title || ""));
    }
    readonly property var taskItems: {
        const out = [], byKey = {};
        void DesktopEntries.applications.values.length;
        for (const id of DeskState.pins) {
            const entry = DesktopEntries.byId(id);
            if (!entry || byKey[id]) continue;
            byKey[id] = { key: id, id: id, entry: entry, pinned: true, own: false, windows: [] };
            out.push(byKey[id]);
        }
        for (const t of ToplevelManager.toplevels.values) {
            const own = ownWindow(t);
            const entry = own ? null : AppIcons.entryFor(t.appId);
            const key = own ? "own:" + t.title : entry ? entry.id : "app:" + (t.appId || t.title);
            if (!byKey[key]) {
                byKey[key] = { key: key, id: entry ? entry.id : "", entry: entry, pinned: false, own: own, windows: [] };
                out.push(byKey[key]);
            }
            byKey[key].windows.push(t);
        }
        return out;
    }
    function appName(it) {
        if (it.entry) return it.entry.name;
        const w = it.windows[0];
        return w ? (w.title || w.appId) : "";
    }
    function cycleApp(it) {
        const ws = it.windows;
        if (!ws.length) { if (it.entry) it.entry.execute(); return; }
        const i = ws.findIndex(w => w.activated);
        if (ws.length === 1) { if (i === 0) ws[0].minimized = true; else ws[0].activate(); }
        else ws[(i + 1) % ws.length].activate();
    }
    function focusApp(it, btn) {
        if (it.windows.length > 1 && btn && menu) windowList(btn, it);
        else cycleApp(it);
    }
    function windowList(btn, it) {
        const list = it.windows.slice(0, 12).map(w => ({ label: w.title || appName(it), icon: "window", checked: w.activated ? true : undefined, run: () => { w.activate(); } }));
        if (it.entry && !it.own) list.push({ separator: true }, { label: "New window", icon: "plus", run: () => it.entry.execute() });
        popupAt(btn, appName(it), list);
    }
    function newInstance(it) {
        if (it.entry && !it.own) it.entry.execute();
        else if (!it.windows.length) cycleApp(it);
    }
    function activateNth(n) { const it = taskItems[n - 1]; if (it) cycleApp(it); }
    function toggleShowDesktop() {
        const open = ToplevelManager.toplevels.values.filter(t => !t.minimized);
        if (open.length) {
            const front = open.find(t => t.activated) || null;
            DeskState.shownDesktop = { windows: open, front: front };
            open.forEach(t => t.minimized = true);
            return;
        }
        const was = DeskState.shownDesktop;
        DeskState.shownDesktop = null;
        if (!was) return;
        const still = ToplevelManager.toplevels.values;
        was.windows.filter(t => still.indexOf(t) >= 0 && t !== was.front).forEach(t => t.minimized = false);
        if (was.front && still.indexOf(was.front) >= 0) was.front.activate();
    }
    // A menu next to a button, opening away from the bar's edge (into the screen).
    function popupAt(item, title, list) {
        if (!menu) return;
        const sw = screen ? screen.width : 1920, sh = screen ? screen.height : 1080;
        const p = item.mapToItem(null, 0, 0);
        // The bar's window is at its edge, so window coordinates plus the edge give screen coordinates.
        const sx = strip.edge === "right" ? sw - strip.barW + p.x : p.x;
        const sy = strip.edge === "bottom" ? sh - strip.barH + p.y : p.y;
        if (strip.edge === "bottom") menu.popup(screen, Math.max(8, sx + item.width / 2 - menu.cardWidth / 2), sy - 4, { title: title, above: true }, list);
        else if (strip.edge === "top") menu.popup(screen, Math.max(8, sx + item.width / 2 - menu.cardWidth / 2), sy + item.height + 6, { title: title, above: false }, list);
        else if (strip.edge === "left") menu.popup(screen, sx + item.width + 10, sy, { title: title, above: false }, list);
        else menu.popup(screen, Math.max(8, sx - menu.cardWidth - 22), sy, { title: title, above: false }, list);
    }
    function moveToDesktop(win, n) {
        win.activate();
        mover.command = ["sh", "-c", "sleep 0.25; exec wtype -M logo -M alt -k \"$0\" -m alt -m logo", String(n)];
        mover.running = true;
    }
    Process {
        id: mover
        onExited: code => { if (code === 127) SettingsNav.toast("Moving windows needs wtype. The window's title bar menu can move it too"); }
    }
    function taskMenu(btn, it) {
        if (!menu) return;
        const ws = it.windows;
        const win = ws.find(w => w.activated) || ws[0] || null;
        const list = [];
        const acts = it.entry && !it.own && it.entry.actions ? Array.from(it.entry.actions).slice(0, 6) : [];
        for (const a of acts) list.push({ label: a.name, icon: "open", group: "Jump to", run: () => a.execute() });
        if (acts.length) list.push({ separator: true });
        if (it.entry && !it.own) list.push({ label: ws.length ? "New window" : "Open", icon: ws.length ? "plus" : "open", quick: true, run: () => it.entry.execute() });
        if (it.entry && !it.own) {
            const pinned = DeskState.isPinned(it.entry.id);
            list.push({ label: pinned ? "Unpin from the bar" : "Pin to the bar", icon: pinned ? "unpin" : "pin", quick: true, run: () => DeskState.togglePin(it.entry.id) });
        }
        if (win) {
            const sub = [];
            for (let i = 0; i < 4; i++) sub.push({ label: "Desktop " + (i + 1), run: () => moveToDesktop(win, i + 1) });
            if (list.length) list.push({ separator: true });
            list.push({ label: "Move to desktop", icon: "desktops", group: "Window", sub: sub });
            if (!win.maximized) list.push({ label: "Maximize", icon: "window", group: "Window", run: () => { win.activate(); win.maximized = true; } });
            else list.push({ label: "Restore size", icon: "window", group: "Window", run: () => { win.maximized = false; } });
            list.push({ separator: true });
            if (ws.length > 1) list.push({ label: "Close all " + ws.length + " windows", icon: "close", danger: true, quick: true, run: () => ws.forEach(w => w.close()) });
            else list.push({ label: "Close window", icon: "close", danger: true, quick: true, run: () => win.close() });
        }
        popupAt(btn, appName(it), list);
    }
    // Dragging a pinned app along the bar puts it somewhere else.
    property int dragFrom: -1
    property int dragTo: -1
    readonly property int pinnedCount: taskItems.filter(t => t.pinned).length
    function dropIndexAt(pos) {
        let n = 0;
        for (let i = 0; i < pinnedCount; i++) {
            if (i === dragFrom) continue;
            const it = taskRepeater.itemAt(i);
            if (!it) continue;
            const c = it.mapToItem(null, it.width / 2, it.height / 2);
            if ((vertical ? c.y : c.x) < pos) n++;
        }
        return n;
    }
    function finishDrag() {
        if (dragFrom >= 0 && dragTo >= 0 && dragTo !== dragFrom) {
            const order = taskItems.filter(t => t.pinned).map(t => t.id);
            const moved = order.splice(dragFrom, 1)[0];
            order.splice(dragTo, 0, moved);
            DeskState.reorderPins(order.concat(DeskState.pins.filter(id => order.indexOf(id) < 0)));
        }
        dragFrom = -1;
        dragTo = -1;
    }

    // ------------------------------------------------------------------ the strip
    GridLayout {
        id: lay
        anchors.centerIn: parent
        flow: strip.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
        rows: strip.vertical ? -1 : 1
        columns: strip.vertical ? 1 : -1
        rowSpacing: 4
        columnSpacing: 4
        Repeater {
            id: taskRepeater
            model: strip.taskItems
            delegate: TaskButton {
                id: task
                required property var modelData
                required property int index
                readonly property var wins: modelData.windows
                tip: wins.length > 1 ? strip.appName(modelData) + " · " + wins.length + " windows" : wins.length === 1 ? (wins[0].title || strip.appName(modelData)) : strip.appName(modelData)
                active: wins.some(w => w.activated)
                running: wins.length > 0
                count: wins.length
                draggable: modelData.pinned
                dropMark: strip.dragFrom >= 0 && strip.dragTo === index && strip.dragTo !== strip.dragFrom ? (strip.dragTo < strip.dragFrom ? -1 : 1) : 0
                onClicked: strip.focusApp(modelData, task)
                onMiddleClicked: strip.newInstance(modelData)
                onRightClicked: strip.taskMenu(task, modelData)
                onDragStarted: { strip.dragFrom = index; strip.dragTo = index; }
                onDragMoved: pos => strip.dragTo = strip.dropIndexAt(pos)
                onDragEnded: strip.finishDrag()
                AppIcon {
                    anchors.centerIn: parent
                    size: strip.iconSize
                    source: task.modelData.own ? AppIcons.own(task.wins.length ? task.wins[0].title : "") : ""
                    icon: task.modelData.entry ? (task.modelData.entry.icon || "") : ""
                    appId: task.wins.length ? (task.wins[0].appId || "") : (task.modelData.id || "")
                }
            }
        }
        Rectangle {   // Show desktop
            id: showDesk
            implicitWidth: strip.vertical ? strip.side * 0.62 : 8
            implicitHeight: strip.vertical ? 8 : strip.side * 0.62
            Layout.alignment: Qt.AlignCenter
            radius: 3
            color: sdArea.containsMouse ? Theme.panel3 : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.12)
            MouseArea { id: sdArea; anchors.fill: parent; anchors.margins: -5; hoverEnabled: true; onClicked: strip.toggleShowDesktop() }
        }
    }

    component TaskButton: Item {
        id: btn
        property string tip: ""
        property bool active: false
        property bool running: false
        property int count: 0
        property bool draggable: false
        property int dropMark: 0
        property bool dragging: false
        property real dragOffset: 0
        default property alias content: holder.data
        signal clicked()
        signal middleClicked()
        signal rightClicked()
        signal dragStarted()
        signal dragMoved(real pos)
        signal dragEnded()
        z: dragging ? 5 : 0
        transform: Translate { x: strip.vertical ? 0 : btn.dragOffset; y: strip.vertical ? btn.dragOffset : 0 }
        implicitWidth: strip.side
        implicitHeight: strip.side
        Rectangle {
            anchors.fill: parent
            radius: Theme.r(10)
            color: area.containsMouse ? Theme.panel3 : btn.active ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.08) : "transparent"
        }
        Item { id: holder; anchors.fill: parent; opacity: btn.dragging ? 0.75 : 1 }
        Rectangle {
            visible: btn.dropMark !== 0
            color: Theme.accent
            width: strip.vertical ? parent.width - 8 : 3
            height: strip.vertical ? 3 : parent.height - 8
            x: strip.vertical ? 4 : (btn.dropMark < 0 ? -4 : parent.width + 1)
            y: strip.vertical ? (btn.dropMark < 0 ? -4 : parent.height + 1) : 4
        }
        Rectangle {   // running mark, on the bar's outer side (a dock's dot): long and lit for the window in front
            visible: btn.active || btn.running
            readonly property int len: btn.active ? 16 : btn.count > 1 ? 9 : 5
            width: strip.vertical ? 3 : len
            height: strip.vertical ? len : 3
            radius: 1.5
            x: strip.vertical ? (strip.edge === "left" ? 0 : parent.width - 3) : (parent.width - width) / 2
            y: strip.vertical ? (parent.height - height) / 2 : (strip.edge === "top" ? 0 : parent.height - 3)
            color: btn.active ? Theme.accent : Theme.muted
        }
        MouseArea {
            id: area
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
            property real startPos: 0
            property bool dragged: false
            onPressed: mouse => { startPos = strip.vertical ? mouse.y : mouse.x; dragged = false; }
            onPositionChanged: mouse => {
                if (!btn.draggable || !(mouse.buttons & Qt.LeftButton)) return;
                const d = (strip.vertical ? mouse.y : mouse.x) - startPos;
                if (!btn.dragging && Math.abs(d) > 12) { btn.dragging = true; dragged = true; btn.dragStarted(); }
                if (btn.dragging) {
                    btn.dragOffset = d;
                    const p = area.mapToItem(null, mouse.x, mouse.y);
                    btn.dragMoved(strip.vertical ? p.y : p.x);
                }
            }
            onReleased: { if (btn.dragging) { btn.dragging = false; btn.dragOffset = 0; btn.dragEnded(); } }
            onCanceled: { if (btn.dragging) { btn.dragging = false; btn.dragOffset = 0; btn.dragEnded(); } }
            onClicked: mouse => {
                if (dragged) { dragged = false; return; }
                if (mouse.button === Qt.MiddleButton) btn.middleClicked();
                else if (mouse.button === Qt.RightButton) btn.rightClicked();
                else btn.clicked();
            }
        }
        PopupWindow {   // the hover label, toward the screen
            visible: area.containsMouse && btn.tip !== "" && !btn.dragging
            anchor.item: btn
            anchor.rect.x: strip.edge === "left" ? btn.width + 10 : strip.edge === "right" ? -implicitWidth - 10 : Math.round(btn.width / 2 - implicitWidth / 2)
            anchor.rect.y: strip.edge === "top" ? btn.height + 10 : strip.edge === "bottom" ? -implicitHeight - 10 : Math.round(btn.height / 2 - implicitHeight / 2)
            implicitWidth: tipText.implicitWidth + 20
            implicitHeight: tipText.implicitHeight + 12
            color: "transparent"
            Rectangle {
                anchors.fill: parent
                radius: Theme.r(8)
                color: Theme.panel2
                border.color: Theme.line
                Text { id: tipText; anchors.centerIn: parent; text: btn.tip; color: Theme.text; font.pixelSize: 12; font.family: Theme.font }
            }
        }
    }
}

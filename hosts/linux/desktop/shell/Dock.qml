// Mantel 2026: the dock along the bottom. Floating and rounded (most looks), attached to the edge (Steam-style,
// Zen) or Console's shelf of big tiles. Every behaviour of the classic taskbar's apps is kept:
// pinned apps first (DeskState, ~/.config/nimbus/desk.json), windows of the same app share a button; click focuses
// (again: minimizes; several windows: a list to pick from), middle-click opens another one, drag a pinned app to
// put it somewhere else, right-click (X on the controller) for the app's own actions, Pin / Unpin, Move to desktop,
// Maximize and Close. Super+1-9 open the app at that place. The grid button opens the launcher; the thin strip at
// the end shows the desktop (click again: the windows). A running app has a dot, the one in front a long accent dot.
// Auto-hide (Zen's own, or Settings › Appearance › Bar and dock): hidden until the pointer touches the bottom edge
// (the small handle), and while the on-screen keyboard is open the dock steps aside.
// Loaded by shell.qml through a Loader: if this file can't load, the classic taskbar (Bar.qml) comes up instead.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
    id: dock
    signal launcherRequested()
    property var menu: null
    property bool primary: false
    property bool keyboardOpen: false
    onPrimaryChanged: claim()
    Component.onCompleted: claim()
    function claim() { if (!primary) return; DeskState.mainBar = dock; DeskState.dockOwner = dock; DeskState.dockBottom = reserved; }
    // Replaced (Settings › Appearance › Bar layout changed) or gone: its space is given back, unless a new dock
    // already took over.
    Component.onDestruction: { if (DeskState.dockOwner === dock) { DeskState.dockOwner = null; DeskState.dockBottom = 0; } if (DeskState.mainBar === dock) DeskState.mainBar = null; }
    onReservedChanged: { claim(); refit(); }

    readonly property bool shelf: Theme.islandStyle === "shelf"
    readonly property bool attached: Theme.islandStyle === "bar"
    readonly property bool compact: screen && screen.width <= 1440
    readonly property int tile: shelf ? Math.round(Math.min(212, (screen ? screen.height : 1080) * 0.17)) : compact ? 68 : 56
    readonly property int iconSize: shelf ? 64 : compact ? 46 : 38
    readonly property int bodyH: shelf ? tile * 1.18 + 76 : attached ? tile + 8 : tile + 16
    readonly property int hintH: Theme.hints ? 48 : 0       // Console: the controller hints along the very bottom
    readonly property int gap: (shelf ? 24 : attached ? 0 : 10) + hintH
    readonly property bool autoHide: Theme.dockAutoHide && !shelf
    readonly property bool hiddenNow: keyboardOpen || (autoHide && !hov.hovered && !DeskState.dockPeek && !holdOpen.running && !menuOpen)
    readonly property bool menuOpen: DeskState.overlayShown && menuFromDock
    property bool menuFromDock: false
    readonly property int reserved: autoHide || keyboardOpen ? 0 : bodyH + gap + (attached ? 0 : 2)

    anchors { bottom: true; left: true; right: true }
    implicitHeight: bodyH + gap + 2
    implicitWidth: 0
    exclusiveZone: reserved
    // Coming back from Game Mode the shell starts before your settings are read, so the dock used to appear with
    // the look's default (not auto-hide) and reserve the bottom of the screen; labwc kept that space reserved after
    // auto-hide took over, so windows couldn't fill the screen. Now the dock waits for your settings (at most a few
    // seconds, so it always comes up), and when the space it reserves changes it is put back on screen once, which
    // makes labwc work out the free area again and re-fit maximized windows.
    property bool settingsReady: Settings.loaded
    Timer { interval: 4000; running: !dock.settingsReady; onTriggered: dock.settingsReady = true }
    property bool remapping: false
    // Off screen for a moment (not within one frame, or the compositor wouldn't see it go), then back.
    Timer { id: remapTimer; interval: 150; onTriggered: dock.remapping = false }
    visible: settingsReady && !remapping
    property int lastReserved: -1
    onSettingsReadyChanged: refit()
    function refit() {
        if (!settingsReady) return;
        if (lastReserved >= 0 && lastReserved !== reserved) { remapping = true; remapTimer.restart(); }
        lastReserved = reserved;
    }
    color: "transparent"
    WlrLayershell.namespace: "hearth-dock"
    WlrLayershell.layer: WlrLayer.Top
    // Only the dock itself (or, hidden, the handle at the bottom edge) takes clicks; the rest lets them through.
    mask: Region { item: dock.hiddenNow ? handle : dock.hintH > 0 ? whole : body }
    Item { id: whole; anchors.fill: parent }
    Timer { id: holdOpen; interval: 700 }
    Connections { target: DeskState; function onOverlayShownChanged() { if (!DeskState.overlayShown) dock.menuFromDock = false; } }

    // ------------------------------------------------------------------ apps (as the classic taskbar, Bar.qml)
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
        const list = it.windows.slice(0, 12).map(w => ({
            label: w.title || appName(it), icon: "window", checked: w.activated ? true : undefined,
            run: () => { w.activate(); }
        }));
        if (it.entry && !it.own) list.push({ separator: true }, { label: "New window", icon: "plus", run: () => it.entry.execute() });
        popupAt(btn, appName(it), list);
    }
    function newInstance(it) {
        if (it.entry && !it.own) it.entry.execute();
        else if (!it.windows.length) cycleApp(it);
    }
    function activateNth(n) {
        const it = taskItems[n - 1];
        if (it) cycleApp(it);
    }
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
    property int dragFrom: -1
    property int dragTo: -1
    readonly property int pinnedCount: taskItems.filter(t => t.pinned).length
    function dropIndexAt(scenePos) {
        let n = 0;
        for (let i = 0; i < pinnedCount; i++) {
            if (i === dragFrom) continue;
            const it = taskRepeater.itemAt(i);
            if (!it) continue;
            const c = it.mapToItem(null, it.width / 2, it.height / 2);
            if (c.x < scenePos) n++;
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
    function popupAt(item, title, list) {
        if (!menu) return;
        const p = item.mapToItem(null, item.width / 2, 0);
        menuFromDock = true;
        menu.popup(screen, Math.max(8, p.x - 134), (screen ? screen.height : 1080) - dock.height + body.y - 2, { title: title, above: true }, list);
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
        for (const a of acts) list.push({ label: a.name, icon: "open", run: () => a.execute() });
        if (acts.length) list.push({ separator: true });
        if (it.entry && !it.own) list.push({ label: ws.length ? "New window" : "Open", icon: "open", run: () => it.entry.execute() });
        if (it.entry && !it.own) {
            const pinned = DeskState.isPinned(it.entry.id);
            list.push({ label: pinned ? "Unpin from dock" : "Pin to dock", icon: pinned ? "unpin" : "pin", run: () => DeskState.togglePin(it.entry.id) });
        }
        if (win) {
            const sets = deskLoader.item ? deskLoader.item.sets : [];
            const count = sets.length > 1 ? sets.length : 4;
            const sub = [];
            for (let i = 0; i < count; i++) {
                const here = sets.length > 1 && sets[i].active;
                sub.push({ label: (sets.length > 1 && sets[i].name ? sets[i].name : "Desktop " + (i + 1)) + (here ? " (this one)" : ""), enabled: !here, run: () => moveToDesktop(win, i + 1) });
            }
            if (list.length) list.push({ separator: true });
            list.push({ label: "Move to desktop", icon: "desktops", sub: sub });
            if (!win.maximized) list.push({ label: "Maximize", icon: "window", run: () => { win.activate(); win.maximized = true; } });
            else list.push({ label: "Restore size", icon: "window", run: () => { win.maximized = false; } });
            list.push({ separator: true });
            if (ws.length > 1) list.push({ label: "Close all " + ws.length + " windows", icon: "close", danger: true, run: () => ws.forEach(w => w.close()) });
            else list.push({ label: "Close window", icon: "close", danger: true, run: () => win.close() });
        }
        popupAt(btn, appName(it), list);
    }
    function statusLine(it) {
        const n = it.windows.length;
        return n > 1 ? n + " windows" : n === 1 ? "Running" : "";
    }

    // ------------------------------------------------------------------ the dock
    Item {   // auto-hide: the handle at the bottom edge (touch it to bring the dock up)
        id: handle
        width: 220; height: 10
        anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom }
        Rectangle { anchors.centerIn: parent; width: 80; height: 4; radius: 2; color: Theme.muted; opacity: dock.hiddenNow && !dock.keyboardOpen ? 0.6 : 0 }
        HoverHandler { onHoveredChanged: if (hovered) holdOpen.restart() }
    }

    Item {
        id: body
        readonly property real fullW: Math.min(dock.width - 24, row.implicitWidth + (dock.shelf ? 0 : 20))
        width: dock.attached ? dock.width : fullW
        height: dock.bodyH
        anchors.horizontalCenter: parent.horizontalCenter
        y: dock.hiddenNow ? dock.height + 4 : dock.height - height - dock.gap
        Behavior on y { enabled: !Settings.reduceMotion; NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
        HoverHandler { id: hov; onHoveredChanged: if (!hovered) holdOpen.restart() }

        Frame { anchors.fill: parent; visible: !dock.shelf && !dock.attached; rad: Theme.look === "gaming" ? 0 : Theme.r(22) }
        Rectangle {   // attached to the edge: one flat strip
            anchors.fill: parent
            visible: dock.attached
            color: Theme.glass
            Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }
        }

        ColumnLayout {
            id: row
            anchors.centerIn: parent
            spacing: 14
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: dock.shelf ? 20 : 8

                DockButton {   // the app menu: always the Hearth flame
                    visible: !dock.shelf
                    tip: "Apps (Super)"
                    onClicked: dock.launcherRequested()
                    Image { anchors.centerIn: parent; width: 28; height: 28; source: Qt.resolvedUrl("images/hearth-logo.png"); smooth: true; mipmap: true }
                }
                Rectangle { visible: !dock.shelf; implicitWidth: 1; implicitHeight: dock.tile * 0.6; color: Theme.line }

                Repeater {
                    id: taskRepeater
                    model: dock.taskItems
                    delegate: DockButton {
                        id: task
                        required property var modelData
                        required property int index
                        readonly property var wins: modelData.windows
                        readonly property bool focused: wins.some(w => w.activated)
                        tip: wins.length > 1 ? dock.appName(modelData) + " · " + wins.length + " windows" : wins.length === 1 ? (wins[0].title || dock.appName(modelData)) : dock.appName(modelData)
                        label: dock.shelf ? dock.appName(modelData) : ""
                        status: dock.shelf ? dock.statusLine(modelData) : ""
                        active: focused
                        running: wins.length > 0
                        count: wins.length
                        draggable: modelData.pinned
                        dropMark: dock.dragFrom >= 0 && dock.dragTo === index && dock.dragTo !== dock.dragFrom ? (dock.dragTo < dock.dragFrom ? -1 : 1) : 0
                        onClicked: dock.focusApp(modelData, task)
                        onMiddleClicked: dock.newInstance(modelData)
                        onRightClicked: dock.taskMenu(task, modelData)
                        onDragStarted: { dock.dragFrom = index; dock.dragTo = index; }
                        onDragMoved: pos => dock.dragTo = dock.dropIndexAt(pos)
                        onDragEnded: dock.finishDrag()
                        AppIcon {
                            anchors.centerIn: parent
                            anchors.verticalCenterOffset: dock.shelf ? -dock.tile * 0.12 : 0
                            size: dock.iconSize
                            source: task.modelData.own ? AppIcons.own(task.wins.length ? task.wins[0].title : "") : ""
                            icon: task.modelData.entry ? (task.modelData.entry.icon || "") : ""
                            appId: task.wins.length ? (task.wins[0].appId || "") : (task.modelData.id || "")
                        }
                    }
                }

                DockButton {   // Console: All apps and games, the last tile
                    visible: dock.shelf
                    label: "All apps"
                    status: "and games"
                    onClicked: dock.launcherRequested()
                    Icon { anchors.centerIn: parent; anchors.verticalCenterOffset: -dock.tile * 0.12; name: "grid"; size: dock.iconSize }
                }

                // Show desktop: the thin strip at the end.
                Rectangle {
                    id: showDesk
                    visible: !dock.shelf
                    implicitWidth: 8
                    implicitHeight: dock.tile * 0.62
                    radius: 3
                    color: sdArea.containsMouse ? Theme.panel3 : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.12)
                    MouseArea { id: sdArea; anchors.fill: parent; anchors.margins: -5; hoverEnabled: true; onClicked: dock.toggleShowDesktop() }
                    PopupWindow {
                        visible: sdArea.containsMouse
                        anchor.item: showDesk
                        anchor.rect.x: Math.round(showDesk.width / 2 - implicitWidth + 8)
                        anchor.rect.y: -implicitHeight - 14
                        implicitWidth: sdTip.implicitWidth + 20
                        implicitHeight: sdTip.implicitHeight + 12
                        color: "transparent"
                        Rectangle { anchors.fill: parent; radius: Theme.r(8); color: Theme.panel2; border.color: Theme.line
                            Text { id: sdTip; anchors.centerIn: parent; text: "Show desktop (Super+D)"; color: Theme.text; font.pixelSize: 12; font.family: Theme.font } }
                    }
                }
            }
            // Console: the desktops and Show desktop under the tiles (the window title shows on the focused tile).
            RowLayout {
                visible: dock.shelf
                Layout.alignment: Qt.AlignLeft
                spacing: 14
                Rectangle {
                    visible: deskLoader.visible
                    implicitWidth: deskLoader.implicitWidth + 20; implicitHeight: 52; radius: 26
                    color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.75)
                    Loader { id: deskLoader; anchors.centerIn: parent; visible: status === Loader.Ready && !!item && item.wanted; source: "DeskSwitcher.qml"; onLoaded: { item.island = true; item.pillSize = 30; } }
                }
                Chip { icon: "desktops"; text: "Show desktop"; onPicked: dock.toggleShowDesktop() }
                Chip { icon: "window"; text: "Task Manager"; onPicked: Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "tasks", "open"]) }
            }
        }
    }

    // Console: the controller's button hints, always shown, along the very bottom (HintStrip.qml's set).
    Rectangle {
        visible: dock.hintH > 0
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        height: dock.hintH
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.94)
        Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: Theme.line }
        RowLayout {
            anchors.centerIn: parent
            spacing: 22
            Repeater {
                model: [{ b: "A", c: "#3fae5a", t: "Open" }, { b: "X", c: "#3b7de0", t: "Options" }, { b: "Y", c: "#d9a521", t: "Keyboard" },
                        { b: "☰", c: "#6c7480", t: "Apps" }, { b: "⌂", c: Theme.accent, t: "Hearth menu" }, { b: "⧉", c: "#6c7480", t: "Controls" }]
                delegate: RowLayout {
                    required property var modelData
                    spacing: 8
                    Rectangle { implicitWidth: 24; implicitHeight: 24; radius: 12; color: modelData.c
                        Text { anchors.centerIn: parent; text: modelData.b; color: "white"; font.pixelSize: 12; font.bold: true } }
                    Text { text: modelData.t; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 }
                }
            }
        }
    }

    // ------------------------------------------------------------------ building blocks
    component Chip: Rectangle {
        id: ch
        property string icon
        property string text
        signal picked()
        implicitWidth: chr.implicitWidth + 40; implicitHeight: 52; radius: 26
        color: ca.containsMouse ? Theme.panel3 : Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.75)
        RowLayout { id: chr; anchors.centerIn: parent; spacing: 10
            Icon { name: ch.icon; size: 20 }
            Text { text: ch.text; color: Theme.text; font.family: Theme.font; font.pixelSize: 17 } }
        MouseArea { id: ca; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: ch.picked() }
    }

    component DockButton: Item {
        id: btn
        property string tip: ""
        property string label: ""          // Console: the name under the icon
        property string status: ""         // Console: "2 windows", "Running"
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
        transform: Translate { x: btn.dragOffset }
        implicitWidth: dock.tile
        implicitHeight: dock.shelf ? dock.tile * 1.18 : dock.tile
        Rectangle {
            anchors.fill: parent
            radius: Theme.r(dock.shelf ? 22 : 14)
            color: dock.shelf ? (area.containsMouse ? Theme.panel3 : Qt.rgba(Theme.panel2.r, Theme.panel2.g, Theme.panel2.b, 0.9))
                : area.containsMouse ? Theme.panel3 : btn.active ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.08) : "transparent"
            border.width: dock.shelf && btn.active ? 4 : 0
            border.color: Theme.accent
        }
        Item { id: holder; anchors.fill: parent; opacity: btn.dragging ? 0.75 : 1 }
        Column {
            visible: btn.label !== ""
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 16 }
            Text { width: parent.width; text: btn.label; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 18; font.weight: Font.DemiBold }
            Text { width: parent.width; visible: text !== ""; text: btn.status; elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        }
        Rectangle {
            visible: btn.dropMark !== 0
            color: Theme.accent
            radius: 1
            width: 3; height: parent.height - 8
            x: btn.dropMark < 0 ? -5 : parent.width + 2
            y: 4
        }
        Rectangle {   // running dot under the icon: long and lit when in front
            visible: !dock.shelf && (btn.active || btn.running)
            readonly property int len: btn.active ? 18 : btn.count > 1 ? 10 : 5
            x: (parent.width - width) / 2
            y: parent.height - height - 2
            width: len; height: 4; radius: 2
            color: btn.active ? Theme.accent : Theme.muted
            Behavior on width { NumberAnimation { duration: Theme.quick } }
        }
        MouseArea {
            id: area
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
            property real startPos: 0
            property bool dragged: false
            onPressed: mouse => { startPos = mouse.x; dragged = false; }
            onPositionChanged: mouse => {
                if (!btn.draggable || !(mouse.buttons & Qt.LeftButton)) return;
                const d = mouse.x - startPos;
                if (!btn.dragging && Math.abs(d) > 12) { btn.dragging = true; dragged = true; btn.dragStarted(); }
                if (btn.dragging) {
                    btn.dragOffset = d;
                    const p = area.mapToItem(null, mouse.x, mouse.y);
                    btn.dragMoved(p.x);
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
        PopupWindow {   // the hover label above the dock
            visible: area.containsMouse && btn.tip !== "" && !btn.dragging && !dock.shelf
            anchor.item: btn
            anchor.rect.x: Math.round(btn.width / 2 - implicitWidth / 2)
            anchor.rect.y: -implicitHeight - 14
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

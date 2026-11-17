// Arrange widgets: move, resize and remove this screen's widgets with the keyboard, the controller or the mouse.
// It lies over the desktop (the widgets show through) with an outline on each widget; the picked one is lit.
//   D-pad / arrows   move it (Shift: a little at a time)        LB / RB, Tab   the previous / next widget
//   X / Menu key     switch between Move and Resize             A / Enter      its menu: Settings, Every activity,
//   B / Esc          done                                                       Remove…
//   Delete           remove (after asking)                      drag with the mouse: move; the corner: resize
// What you change is drawn at once and saved when you move on to another widget, stop for a moment, or finish:
// one merge of that widget's place (WidgetStore.place), never every step.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import "widget-layout.js" as L

PanelWindow {
    id: ar
    property bool shown: false
    property string screenName: ""
    property var ids: []
    signal doneRequested()
    signal addRequested()
    visible: shown
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "hearth-widget-arrange"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    readonly property var insets: ({ top: DeskState.topInset, bottom: DeskState.bottomInset, left: DeskState.leftInset, right: DeskState.rightInset })
    readonly property var items: { void WidgetStore.rev; void WidgetStore.previewRev; return ids.map(id => WidgetStore.get(id)).filter(w => !!w).map(w => {
        const p = WidgetStore.preview[w.id] || L.shown(w, ar.width, ar.height, ar.insets);
        return { id: w.id, type: w.type, activity: w.activity, x: p.x, y: p.y, w: p.w, h: p.h };
    }); }
    property string picked: ""
    property bool resizing: false
    property bool menuOpen: false
    property int menuAt: 0
    property bool asking: false
    readonly property var cur: items.find(i => i.id === picked) || null
    onShownChanged: {
        if (shown) { picked = L.cycle(items, "", 1); resizing = false; menuOpen = false; asking = false; keys.forceActiveFocus(); }
        else commitAll();
    }
    onPickedChanged: commitAll()
    function commit(id) {
        const p = WidgetStore.preview[id];
        if (!p) return;
        WidgetStore.place(id, p.x, p.y, p.w, p.h, screenName);
        WidgetStore.setPreview(id, null);
    }
    function commitAll() { for (const id in WidgetStore.preview) commit(id); }
    Timer { id: settle; interval: 800; onTriggered: ar.commitAll() }
    function move(dx, dy) {
        if (!cur) return;
        const n = L.nudge(cur, dx, dy, resizing, width, height, insets);
        WidgetStore.setPreview(cur.id, { x: n.x, y: n.y, w: n.w, h: n.h });
        settle.restart();
    }
    readonly property var menuItems: cur ? [
        { id: "settings", label: "Settings", enabled: L.hasSettings(cur.type) },
        { id: "resize", label: resizing ? "Move instead" : "Resize", enabled: true },
        { id: "every", label: cur.activity === "*" ? "Only on " + Activities.name : "Show on every activity", enabled: Activities.many },
        { id: "remove", label: "Remove…", enabled: true }].filter(m => m.enabled) : []
    function act(m) {
        menuOpen = false;
        if (!cur) return;
        if (m === "settings") { const id = cur.id; commitAll(); doneRequested(); WidgetStore.configuring = id; }
        else if (m === "resize") resizing = !resizing;
        else if (m === "every") WidgetStore.setActivity(cur.id, cur.activity === "*" ? Activities.current : "*");
        else if (m === "remove") asking = true;
    }

    Item {
        id: keys
        anchors.fill: parent
        focus: true
        Keys.onPressed: event => {
            const k = event.key, step = (event.modifiers & Qt.ShiftModifier) ? 4 : 24;
            if (ar.asking) {
                if (k === Qt.Key_Return || k === Qt.Key_Enter) { const id = ar.picked; ar.asking = false; ar.picked = L.cycle(ar.items.filter(i => i.id !== id), "", 1); WidgetStore.remove(id); }
                else if (k === Qt.Key_Escape) ar.asking = false;
                event.accepted = true;
                return;
            }
            if (ar.menuOpen) {
                if (k === Qt.Key_Up) ar.menuAt = Math.max(0, ar.menuAt - 1);
                else if (k === Qt.Key_Down) ar.menuAt = Math.min(ar.menuItems.length - 1, ar.menuAt + 1);
                else if (k === Qt.Key_Return || k === Qt.Key_Enter) ar.act(ar.menuItems[ar.menuAt].id);
                else if (k === Qt.Key_Escape || k === Qt.Key_Menu) ar.menuOpen = false;
                event.accepted = true;
                return;
            }
            if (k === Qt.Key_Escape) ar.doneRequested();
            else if (k === Qt.Key_Left) ar.move(-step, 0);
            else if (k === Qt.Key_Right) ar.move(step, 0);
            else if (k === Qt.Key_Up) ar.move(0, -step);
            else if (k === Qt.Key_Down) ar.move(0, step);
            else if (k === Qt.Key_Tab) ar.picked = L.cycle(ar.items, ar.picked, 1);
            else if (k === Qt.Key_Backtab) ar.picked = L.cycle(ar.items, ar.picked, -1);
            else if (k === Qt.Key_Menu || k === Qt.Key_R) ar.resizing = !ar.resizing;
            else if (k === Qt.Key_Return || k === Qt.Key_Enter) { if (ar.cur) { ar.menuAt = 0; ar.menuOpen = true; } else ar.addRequested(); }
            else if (k === Qt.Key_Delete) { if (ar.cur) ar.asking = true; }
            else if (k === Qt.Key_Plus || k === Qt.Key_A) ar.addRequested();
            else return;
            event.accepted = true;
        }
    }

    Rectangle { anchors.fill: parent; color: Qt.rgba(0, 0, 0, 0.22) }
    MouseArea { anchors.fill: parent; onClicked: { ar.menuOpen = false; ar.asking = false; } }
    // A faint grid while arranging, to line widgets up against
    Canvas {
        anchors.fill: parent
        opacity: 0.12
        onPaint: { const g = getContext("2d"); g.reset(); g.strokeStyle = "#ffffff"; g.lineWidth = 1; for (let x = 0; x < width; x += 48) { g.beginPath(); g.moveTo(x, 0); g.lineTo(x, height); g.stroke(); } for (let y = 0; y < height; y += 48) { g.beginPath(); g.moveTo(0, y); g.lineTo(width, y); g.stroke(); } }
    }

    Repeater {
        model: ar.items
        delegate: Rectangle {
            id: box
            required property var modelData
            readonly property bool lit: modelData.id === ar.picked
            readonly property var meta: L.info(modelData.type)
            x: modelData.x; y: modelData.y; width: modelData.w; height: modelData.h
            radius: Theme.r(16)
            color: lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.14) : Qt.rgba(1, 1, 1, 0.04)
            border.width: lit ? 3 : 1.5
            border.color: lit ? Theme.accent : Qt.rgba(1, 1, 1, 0.55)
            Rectangle {
                anchors { top: parent.top; left: parent.left; margins: 8 }
                width: tag.implicitWidth + 18; height: 26; radius: 13
                color: box.lit ? Theme.accent : Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.9)
                Text { id: tag; anchors.centerIn: parent; text: (box.meta ? box.meta.name : box.modelData.type) + (box.lit ? (ar.resizing ? " · resize" : " · move") : "") + (box.modelData.activity === "*" ? " · every activity" : "")
                       color: box.lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
            }
            Text { anchors { bottom: parent.bottom; left: parent.left; margins: 10 } visible: box.lit; text: box.modelData.w + " × " + box.modelData.h; color: Theme.text; font.family: Theme.font; font.pixelSize: 11; style: Text.Outline; styleColor: "#88000000" }
            MouseArea {
                anchors.fill: parent
                cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
                property real ox: 0
                property real oy: 0
                onPressed: mouse => { ar.picked = box.modelData.id; ar.menuOpen = false; ox = mouse.x; oy = mouse.y; }
                onPositionChanged: mouse => {
                    if (!pressed) return;
                    const s = L.shown({ type: box.modelData.type, x: box.x + mouse.x - ox, y: box.y + mouse.y - oy, w: box.width, h: box.height }, ar.width, ar.height, ar.insets);
                    WidgetStore.setPreview(box.modelData.id, s);
                }
                onReleased: ar.commit(box.modelData.id)
                onDoubleClicked: { ar.menuAt = 0; ar.menuOpen = true; }
            }
            Rectangle {
                anchors { right: parent.right; bottom: parent.bottom; margins: -7 }
                width: 18; height: 18; radius: 9
                color: Theme.accent
                border.width: 2; border.color: "white"
                visible: box.lit
                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -6
                    cursorShape: Qt.SizeFDiagCursor
                    onPositionChanged: mouse => {
                        if (!pressed) return;
                        const p = mapToItem(box, mouse.x, mouse.y);
                        const n = L.nudge(box.modelData, p.x - box.width, p.y - box.height, true, ar.width, ar.height, ar.insets);
                        WidgetStore.setPreview(box.modelData.id, { x: box.modelData.x, y: box.modelData.y, w: n.w, h: n.h });
                    }
                    onReleased: ar.commit(box.modelData.id)
                }
            }
            // A / Enter: the picked widget's menu
            Rectangle {
                visible: box.lit && ar.menuOpen
                x: box.x + box.width + 250 > ar.width ? -240 : box.width + 10
                y: 0
                width: 230; height: menuCol.implicitHeight + 12
                radius: Theme.r(12)
                color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.97)
                border.width: 1; border.color: Theme.line
                Column {
                    id: menuCol
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 6 }
                    Repeater {
                        model: ar.menuItems
                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            width: menuCol.width; height: 34; radius: Theme.r(8)
                            color: index === ar.menuAt ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.3) : "transparent"
                            Text { anchors { fill: parent; leftMargin: 10 } verticalAlignment: Text.AlignVCenter; text: modelData.label; color: modelData.id === "remove" ? Theme.danger : Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                            MouseArea { anchors.fill: parent; hoverEnabled: true; onEntered: ar.menuAt = index; onClicked: ar.act(modelData.id) }
                        }
                    }
                }
            }
            Rectangle {
                visible: box.lit && ar.asking
                anchors.centerIn: parent
                width: Math.max(220, askCol.implicitWidth + 30); height: askCol.implicitHeight + 24
                radius: Theme.r(12)
                color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.97)
                border.width: 1; border.color: Theme.danger
                ColumnLayout {
                    id: askCol
                    anchors.centerIn: parent
                    spacing: 8
                    Text { Layout.alignment: Qt.AlignHCenter; text: "Remove " + (box.meta ? box.meta.name.toLowerCase() : "it") + "?"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                    Text { Layout.alignment: Qt.AlignHCenter; text: "A / Enter: remove   B / Esc: keep"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                }
            }
        }
    }

    // ---- what the buttons do, and Add / Done
    Rectangle {
        anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: Math.max(30, DeskState.bottomInset + 20) }
        width: hintRow.implicitWidth + 32; height: 54
        radius: 27
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.95)
        border.width: 1; border.color: Theme.line
        RowLayout {
            id: hintRow
            anchors.centerIn: parent
            spacing: 14
            Text { text: "Arrange widgets"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
            Repeater {
                model: [{ b: "D-pad", c: "#6c7480", t: ar.resizing ? "Resize" : "Move" }, { b: "LB RB", c: "#6c7480", t: "Pick" }, { b: "X", c: "#3c7be0", t: ar.resizing ? "Move" : "Resize" },
                        { b: "A", c: "#3fae5a", t: "Menu" }, { b: "B", c: "#d9483b", t: "Done" }]
                delegate: RowLayout {
                    required property var modelData
                    spacing: 5
                    Rectangle { width: Math.max(24, bl.implicitWidth + 10); height: 24; radius: 12; color: modelData.c; Text { id: bl; anchors.centerIn: parent; text: modelData.b; color: "white"; font.family: Theme.font; font.pixelSize: modelData.b.length > 2 ? 9 : 12; font.bold: true } }
                    Text { text: modelData.t; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                }
            }
            WBtn { text: "Add widgets"; icon: "plus"; onClicked: ar.addRequested() }
            WBtn { text: "Done"; on: true; onClicked: ar.doneRequested() }
        }
    }
    Text {
        visible: ar.items.length === 0
        anchors.centerIn: parent
        text: "No widgets on this screen yet: Add widgets"
        color: "white"; font.family: Theme.font; font.pixelSize: 22; style: Text.Outline; styleColor: "#88000000"
    }
}

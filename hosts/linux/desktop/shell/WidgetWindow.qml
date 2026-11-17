// One desktop widget on the desktop: its own small layer-shell surface on the Bottom layer (above the wallpaper and
// the desktop's icons, under every window, the way Desktop.qml sits), so it takes clicks only where it is.
// Hover it for its handle (drag to move), the gear (its settings) and × (remove, after asking); drag the corner to
// resize. Arrange (WidgetArrange.qml) does the same from the keyboard and the controller. Only letting go after a
// move or resize saves its place (WidgetStore.place); a place pulled inside a smaller screen is only drawn there.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import "widget-layout.js" as L

PanelWindow {
    id: win
    required property var modelData          // the widget's id
    property string screenName: ""
    readonly property string wid: modelData
    readonly property var rec: { void WidgetStore.rev; return WidgetStore.get(wid); }
    readonly property var meta: rec ? L.info(rec.type) : null
    readonly property var insets: ({ top: DeskState.topInset, bottom: DeskState.bottomInset, left: DeskState.leftInset, right: DeskState.rightInset })
    readonly property int sw: screen ? screen.width : 1920
    readonly property int sh: screen ? screen.height : 1080
    property var drag: null                  // {x, y, w, h} while you move or resize it (saved when you let go)
    property bool configuring: false
    property bool asking: false              // "Remove this widget?"
    readonly property var placed: { void WidgetStore.previewRev; const p = WidgetStore.preview[wid]; return p ? p : drag ? drag : rec ? L.shown(rec, sw, sh, insets) : { x: 0, y: 0, w: 200, h: 150 }; }
    // Its settings get a little more room when the widget is small (only while they're open; nothing is saved).
    readonly property var box: configuring ? L.shown({ type: rec ? rec.type : "", x: placed.x, y: placed.y, w: Math.max(placed.w, 360), h: Math.max(placed.h, 340) }, sw, sh, insets) : placed

    WlrLayershell.layer: WlrLayer.Bottom
    WlrLayershell.namespace: "hearth-widget"
    WlrLayershell.keyboardFocus: configuring ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.OnDemand
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; left: true }
    margins { top: box.y; left: box.x }
    implicitWidth: box.w
    implicitHeight: box.h
    color: "transparent"
    visible: rec !== null && !DeskState.locked

    onConfiguringChanged: { if (configuring) WidgetStore.configuring = wid; else if (WidgetStore.configuring === wid) WidgetStore.configuring = ""; }
    Connections {
        target: WidgetStore
        function onConfiguringChanged() { win.configuring = WidgetStore.configuring === win.wid; }
    }

    Item {
        id: body
        anchors.fill: parent
        focus: true
        Keys.onEscapePressed: { win.configuring = false; win.asking = false; }
        HoverHandler { id: hov }

        Frame { anchors.fill: parent; rad: Theme.r(16); solid: win.configuring; visible: !(content.item && content.item.paintsOwn) || win.configuring }
        Loader {
            id: content
            anchors.fill: parent
            source: win.rec ? "Widget" + win.rec.type.charAt(0).toUpperCase() + win.rec.type.slice(1).replace("notes", "Notes") + ".qml" : ""
            onLoaded: {
                item.wid = Qt.binding(() => win.wid);
                item.cfg = Qt.binding(() => win.rec ? win.rec.cfg : ({}));
                item.configuring = Qt.binding(() => win.configuring);
                item.live = Qt.binding(() => win.visible && !DeskState.screenOff);
                item.doneConfiguring.connect(() => { win.configuring = false; });
                item.wantsSettings.connect(() => { win.configuring = true; });
            }
            onStatusChanged: if (status === Loader.Error) console.warn("widgets: the " + (win.rec ? win.rec.type : "?") + " widget couldn't load");
        }
        Text {
            visible: content.status === Loader.Error
            anchors.centerIn: parent
            text: "This widget couldn't load"
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
        }

        // ---- the handle, the gear and ×: on hover (and while its settings are open)
        Rectangle {
            id: tools
            visible: (hov.hovered || win.configuring || win.drag !== null) && !win.asking
            anchors { top: parent.top; right: parent.right; topMargin: 6; rightMargin: 6 }
            width: toolRow.implicitWidth + 8; height: 30
            radius: 15
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.92)
            border.width: 1; border.color: Theme.line
            Row {
                id: toolRow
                anchors.centerIn: parent
                spacing: 2
                Item {
                    width: 26; height: 26
                    Icon { anchors.centerIn: parent; name: "grid"; size: 15; color: Theme.text }
                    MouseArea {
                        id: mover
                        anchors.fill: parent
                        cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
                        property real px: 0
                        property real py: 0
                        onPressed: mouse => { px = mouse.x; py = mouse.y; win.drag = { x: win.placed.x, y: win.placed.y, w: win.placed.w, h: win.placed.h }; }
                        onPositionChanged: mouse => {
                            if (!win.drag) return;
                            const nx = win.drag.x + (mouse.x - px), ny = win.drag.y + (mouse.y - py);
                            const s = L.shown({ type: win.rec.type, x: nx, y: ny, w: win.drag.w, h: win.drag.h }, win.sw, win.sh, win.insets);
                            win.drag = { x: s.x, y: s.y, w: win.drag.w, h: win.drag.h };
                        }
                        onReleased: win.finish()
                        onCanceled: win.drag = null
                    }
                }
                WBtn { visible: win.rec && L.hasSettings(win.rec.type); icon: "gear"; flat: true; size: 26; on: win.configuring; onClicked: win.configuring = !win.configuring }
                WBtn { icon: "close"; flat: true; size: 26; onClicked: win.asking = true }
            }
        }
        // ---- resize from the bottom-right corner
        Item {
            visible: hov.hovered || win.drag !== null
            anchors { right: parent.right; bottom: parent.bottom }
            width: 22; height: 22
            Canvas {
                anchors.fill: parent
                onPaint: { const g = getContext("2d"); g.reset(); g.strokeStyle = Theme.muted; g.lineWidth = 2; for (let i = 0; i < 3; i++) { g.beginPath(); g.moveTo(width - 4 - i * 5, height - 4); g.lineTo(width - 4, height - 4 - i * 5); g.stroke(); } }
            }
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.SizeFDiagCursor
                property real px: 0
                property real py: 0
                onPressed: mouse => { px = mouse.x; py = mouse.y; win.drag = { x: win.placed.x, y: win.placed.y, w: win.placed.w, h: win.placed.h }; }
                onPositionChanged: mouse => {
                    if (!win.drag) return;
                    const n = L.nudge({ type: win.rec.type, x: win.drag.x, y: win.drag.y, w: win.drag.w, h: win.drag.h }, mouse.x - px, mouse.y - py, true, win.sw - win.drag.x, win.sh - win.drag.y, {});
                    win.drag = { x: win.drag.x, y: win.drag.y, w: n.w, h: n.h };
                }
                onReleased: win.finish()
                onCanceled: win.drag = null
            }
        }
        // ---- "Remove this widget?"
        Rectangle {
            visible: win.asking
            anchors.fill: parent
            radius: Theme.r(16)
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.96)
            ColumnLayout {
                anchors.centerIn: parent
                spacing: 10
                Text { Layout.alignment: Qt.AlignHCenter; text: "Remove " + (win.meta ? win.meta.name.toLowerCase() : "this widget") + "?"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                Text { Layout.alignment: Qt.AlignHCenter; visible: win.rec && (win.rec.type === "notes" || win.rec.type === "quicknotes"); text: "What you wrote in it goes too"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                RowLayout {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: 8
                    WBtn { text: "Remove"; on: true; onClicked: WidgetStore.remove(win.wid) }
                    WBtn { text: "Keep"; onClicked: win.asking = false }
                }
            }
        }
    }
    function finish() {
        if (drag && rec) WidgetStore.place(wid, drag.x, drag.y, drag.w, drag.h, screenName);
        drag = null;
    }
}

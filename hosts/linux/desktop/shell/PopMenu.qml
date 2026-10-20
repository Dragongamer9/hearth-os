// Right-click menus in Hearth's look: the taskbar buttons, the desktop and its icons, apps in the launcher.
// One shared menu (shell.qml); callers pass the screen, where to open (screen coordinates) and the items:
//   menu.popup(screen, x, y, { title: "Firefox", above: true }, [
//       { label: "Pin to taskbar", icon: "pin", run: () => ... },
//       { separator: true },
//       { label: "Move to desktop", icon: "desktops", sub: [ { label: "Desktop 2", run: ... }, ... ] },
//       { label: "Show desktop icons", checked: true, run: ... },
//       { label: "Close window", icon: "close", danger: true, enabled: false, run: ... } ])
// Mouse: hover and click, click outside closes. Keyboard and controller: Up/Down, Enter (A) chooses,
// Right opens a submenu, Left / Esc (B) goes back, then closes.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: root
    property bool shown: false
    property var stack: []                 // [{title, items}], the last one is shown
    property int sel: -1
    property real px: 0
    property real py: 0
    property bool above: false             // open upwards from (px, py), e.g. from the taskbar
    readonly property var level: stack.length ? stack[stack.length - 1] : ({ title: "", items: [] })
    readonly property var items: level.items || []

    function popup(scr, x, y, opts, list) {
        if (scr) screen = scr;
        px = x; py = y;
        above = !!(opts && opts.above);
        stack = [{ title: (opts && opts.title) || "", items: list || [] }];
        sel = -1;
        shown = true;
        keys.forceActiveFocus();
    }
    function close() { shown = false; stack = []; }
    function usable(i) { const it = items[i]; return !!it && !it.separator && it.enabled !== false; }
    function move(d) {
        if (!items.length) return;
        let i = sel < 0 ? (d > 0 ? -1 : 0) : sel;
        for (let n = 0; n < items.length; n++) {
            i = (i + d + items.length) % items.length;
            if (usable(i)) { sel = i; return; }
        }
    }
    function pick(i) {
        if (!usable(i)) return;
        const it = items[i];
        if (it.sub) {
            stack = stack.concat([{ title: it.label, items: it.sub }]);
            sel = -1;
            move(1);
            return;
        }
        close();
        if (it.run) Qt.callLater(it.run);
    }
    function back() {
        if (stack.length > 1) { stack = stack.slice(0, -1); sel = -1; move(1); }
        else close();
    }

    visible: shown
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-menu"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        onClicked: root.close()
    }

    Item {
        id: keys
        anchors.fill: parent
        focus: true
        Keys.onPressed: event => {
            switch (event.key) {
            case Qt.Key_Up: root.move(-1); break;
            case Qt.Key_Down: case Qt.Key_Tab: root.move(1); break;
            case Qt.Key_Right: if (root.sel >= 0 && root.items[root.sel].sub) root.pick(root.sel); break;
            case Qt.Key_Left: case Qt.Key_Backspace: case Qt.Key_Escape: root.back(); break;
            case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space:
                if (root.sel < 0) root.move(1); else root.pick(root.sel);
                break;
            case Qt.Key_Menu: root.close(); break;
            default: return;
            }
            event.accepted = true;
        }
    }

    Rectangle {
        id: card
        width: 268
        height: col.implicitHeight + 12
        x: Math.max(8, Math.min(root.width - width - 8, root.px))
        y: Math.max(8, Math.min(root.height - height - 8, root.above ? root.py - height - 8 : root.py))
        radius: Theme.radius
        color: Theme.glass
        border.color: Theme.line
        opacity: root.shown ? 1 : 0
        scale: root.shown ? 1 : 0.96
        transformOrigin: root.above ? Item.Bottom : Item.Top
        Behavior on opacity { NumberAnimation { duration: Theme.quick } }
        Behavior on scale { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutCubic } }
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.LeftButton | Qt.RightButton }   // clicks inside don't close

        ColumnLayout {
            id: col
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 6 }
            spacing: 0

            // Title (the app or file name), with a way back from a submenu
            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 10
                Layout.rightMargin: 10
                Layout.topMargin: 4
                Layout.bottomMargin: 4
                visible: root.level.title !== ""
                spacing: 6
                Icon {
                    visible: root.stack.length > 1
                    name: "back"; size: 14; color: Theme.muted
                    MouseArea { anchors.fill: parent; anchors.margins: -8; onClicked: root.back() }
                }
                Text {
                    Layout.fillWidth: true
                    text: root.level.title
                    color: Theme.muted
                    font.family: Theme.font
                    font.pixelSize: 12
                    elide: Text.ElideRight
                }
            }

            Repeater {
                model: root.items
                delegate: Item {
                    id: row
                    required property var modelData
                    required property int index
                    readonly property bool sep: !!modelData.separator
                    readonly property bool on: modelData.enabled !== false
                    readonly property bool lit: root.sel === index && !sep
                    Layout.fillWidth: true
                    implicitHeight: sep ? 9 : 40
                    Rectangle {
                        visible: row.sep
                        anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 8; rightMargin: 8 }
                        height: 1
                        color: Theme.line
                    }
                    Rectangle {
                        visible: !row.sep
                        anchors.fill: parent
                        radius: 10
                        color: row.lit ? Theme.accent : "transparent"
                    }
                    RowLayout {
                        visible: !row.sep
                        anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                        spacing: 10
                        opacity: row.on ? 1 : 0.45
                        Item {
                            implicitWidth: 18; implicitHeight: 18
                            Icon {
                                anchors.fill: parent
                                visible: row.modelData.checked === undefined && !!row.modelData.icon
                                name: row.modelData.icon || ""
                                size: 18
                                color: row.lit ? Theme.accentInk : (row.modelData.danger ? Theme.accent : Theme.text)
                            }
                            Icon {
                                anchors.fill: parent
                                visible: row.modelData.checked === true
                                name: "check"; size: 18
                                color: row.lit ? Theme.accentInk : Theme.accent
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            text: row.modelData.label || ""
                            color: row.lit ? Theme.accentInk : Theme.text
                            font.family: Theme.font
                            font.pixelSize: 14
                            elide: Text.ElideRight
                        }
                        Icon {
                            visible: !!row.modelData.sub
                            name: "forward"; size: 14
                            color: row.lit ? Theme.accentInk : Theme.muted
                        }
                    }
                    MouseArea {
                        anchors.fill: parent
                        enabled: !row.sep && row.on
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        cursorShape: Qt.PointingHandCursor
                        onEntered: root.sel = row.index
                        onClicked: root.pick(row.index)
                    }
                }
            }
        }
    }
}

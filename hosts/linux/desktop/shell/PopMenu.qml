// Right-click menus in Hearth's look: the taskbar buttons, the desktop and its icons, apps in the launcher.
// One shared menu (shell.qml); callers pass the screen, where to open (screen coordinates) and the items:
//   menu.popup(screen, x, y, { title: "Firefox", above: true }, [
//       { label: "Pin to taskbar", icon: "pin", run: () => ... },
//       { separator: true },
//       { label: "Move to desktop", icon: "desktops", sub: [ { label: "Desktop 2", run: ... }, ... ] },
//       { label: "Show desktop icons", checked: true, run: ... },
//       { label: "Close window", icon: "close", danger: true, enabled: false, run: ... } ])
//       { label: "Copy", icon: "copy", shortcut: "Ctrl+C", quick: true, group: "Edit", run: ... }
// Mouse: hover and click, click outside closes. Keyboard and controller: Up/Down, Enter (A) chooses,
// Right opens a submenu, Left / Esc (B) goes back, then closes. While a controller is in use (DeskState.padSeen, as
// the hint strip) a small A Choose · B Back row shows at the bottom.
// Its look is Settings › Personalization › Bar and dock › Right-click menu (Theme.menuStyle): today's menu is drawn
// here; Windows 11 style, Compact list and Tiles and groups are MenuStyles.qml, loaded by file name, so if it can't
// load this menu shows instead. Items may say quick: true (the quick icons / tiles in those looks; a normal row in
// today's), shortcut: "Ctrl+C" (hint: works too) and group: "Open" (Tiles and groups' headings).
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
    property bool padHint: false           // a controller is in use: the A Choose · B Back row
    // Another look (MenuStyles.qml), when chosen and loaded; otherwise today's menu below.
    readonly property bool styled: Theme.menuStyle !== "today" && styledView.status === Loader.Ready && styledView.item !== null
    // How wide the menu is, for callers that centre it on a button.
    readonly property real cardWidth: styled ? styledView.item.cardWidth : 268

    function popup(scr, x, y, opts, list) {
        if (scr) screen = scr;
        px = x; py = y;
        above = !!(opts && opts.above);
        stack = [{ title: (opts && opts.title) || "", items: list || [] }];
        sel = -1;
        padHint = !!(opts && opts.pad) || (DeskState.padSeen > 0 && Date.now() - DeskState.padSeen < 12000);
        if (styled) styledView.item.open((opts && opts.title) || "", list || []);
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
    // Today's menu scrolls when it's taller than the screen (a handheld with app scaling): the row you're on stays in
    // view. It used to run off the bottom, and Up/Down picked rows you couldn't see.
    onItemsChanged: flick.contentY = 0
    onSelChanged: reveal()
    function reveal() {
        const it = rows.itemAt(sel);
        if (!it) return;
        if (it.y < flick.contentY) flick.contentY = it.y;
        else if (it.y + it.height > flick.contentY + flick.height) flick.contentY = it.y + it.height - flick.height;
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
            if (root.styled) {
                if (styledView.item.key(event)) event.accepted = true;
                return;
            }
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

    // Windows 11 style, Compact list, Tiles and groups (MenuStyles.qml). Loaded by file name: if it can't load, the
    // menu stays today's (a warning goes to the shell's log).
    Loader {
        id: styledView
        active: Theme.menuStyle !== "today"
        source: "MenuStyles.qml"
        visible: root.styled
        width: item ? item.implicitWidth : 0
        height: item ? item.implicitHeight : 0
        x: Math.max(8, Math.min(root.width - width - 8, root.px))
        y: Math.max(8, Math.min(root.height - height - 8, root.above ? root.py - height - 8 : root.py))
        opacity: root.shown ? 1 : 0
        scale: root.shown ? 1 : 0.96
        transformOrigin: root.above ? Item.Bottom : Item.Top
        Behavior on opacity { NumberAnimation { duration: Theme.quick } }
        Behavior on scale { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutCubic } }
        onStatusChanged: if (status === Loader.Error) console.warn("PopMenu: the right-click menu's look (MenuStyles.qml) couldn't load; using today's menu")
        onLoaded: if (root.shown) item.open(root.stack.length ? root.stack[0].title : "", root.stack.length ? root.stack[0].items : [])
    }

    Binding { target: styledView.item; when: styledView.item !== null; property: "look"; value: Theme.menuStyle }
    Binding { target: styledView.item; when: styledView.item !== null; property: "padHint"; value: root.padHint }
    Binding { target: styledView.item; when: styledView.item !== null; property: "spaceRight"; value: root.width - styledView.x - styledView.width - 8 }
    Binding { target: styledView.item; when: styledView.item !== null; property: "maxHeight"; value: root.height - 16 }
    Binding { target: styledView.item; when: styledView.item !== null; property: "maxBottom"; value: root.height - 8 - styledView.y }
    Binding { target: styledView.item; when: styledView.item !== null; property: "minTop"; value: 8 - styledView.y }
    Connections {
        target: styledView.item
        function onChosen(it) { root.close(); if (it.run) Qt.callLater(it.run); }
        function onClosed() { root.close(); }
    }

    Rectangle {
        id: card
        visible: !root.styled
        width: 268
        height: Math.min(col.implicitHeight + 12, root.height - 16)
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

        Flickable {
          id: flick
          anchors { fill: parent; margins: 6 }
          contentWidth: width
          contentHeight: col.implicitHeight
          interactive: contentHeight > height + 1
          boundsBehavior: Flickable.StopAtBounds
          clip: true

          ColumnLayout {
            id: col
            width: flick.width
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
                id: rows
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
                        radius: Theme.r(10)
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
                        Text {                   // the keyboard shortcut for it (Ctrl+C...)
                            visible: text !== "" && !row.modelData.sub
                            text: row.modelData.shortcut || row.modelData.hint || ""
                            color: row.lit ? Theme.accentInk : Theme.muted
                            font.family: Theme.font
                            font.pixelSize: 12
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

            // A controller is in use: what A and B do.
            Item {
                visible: root.padHint
                Layout.fillWidth: true
                implicitHeight: 30
                Rectangle { anchors { left: parent.left; right: parent.right; top: parent.top; topMargin: 3 } height: 1; color: Theme.line }
                MenuHints {
                    anchors { horizontalCenter: parent.horizontalCenter; verticalCenter: parent.verticalCenter; verticalCenterOffset: 2 }
                    k: 0.72
                    model: [{ b: "A", t: "Choose" }, { b: "B", t: "Back" }]
                }
            }
          }
        }
    }
}

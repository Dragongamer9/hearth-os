// The search bar (1.14, like KDE's KRunner): Alt+Space (or Alt+F2) opens a slim bar at the top centre of the screen.
// It searches what the app menus search (Launcher.qml's searchAll: apps, games, Settings pages and single settings,
// files through the file index, hearth-search) and every search plugin (search/*.js through search/runners.js:
// sums, units, currency, web shortcuts, ">commands", windows, recent documents, emoji…), grouped in the order you
// chose in Settings › Search ("pluginOrder" in ~/.config/hearth/search.json; plugins you turned off there stay off).
// Typing "?" lists what you can type, plugin by plugin; Enter on one puts its prefix in the field.
// Keyboard: type, Up / Down (Tab) move, Enter opens, Esc closes. Controller: Y on the desktop opens it with the
// on-screen keyboard, the D-pad moves, A opens, B closes, Y again is the keyboard (Pad.qml: SettingsNav.searchBarShown).
// Typing on the desktop opens it with what you typed (Desktop.qml; Settings › Search › Type on the desktop to search).
// Alt+Space and the keyboard layout switch: when Settings › Keyboard switches layouts with Alt+Space, that wins: labwc
// gets no Alt+Space binding (hearth-system labwc-config), the bar ignores the key under KWin (the layout already
// switched), and Alt+F2 still opens it (search-bar.js layoutTakesAltSpace).
// shell.qml loads this file by name, so a problem here only loses the bar. Opening it writes nothing.
//   quickshell ipc -p <shell folder> call searchbar toggle|open|close|key|type <text>|move <n>|enter|state
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "search-bar.js" as SB

Scope {
    id: sb
    property var launcher: null
    function wire(o) { launcher = o.launcher || null; }

    property bool shown: false
    property string text: ""
    property int sel: 0
    property bool fromPad: false
    Binding { target: SettingsNav; property: "searchBarShown"; value: sb.shown }
    Component.onCompleted: SettingsNav.searchBar = sb

    readonly property var prefs: launcher && launcher.searchPrefs ? launcher.searchPrefs : ({})
    readonly property var savedOrder: Array.isArray(prefs.pluginOrder) ? prefs.pluginOrder : []
    readonly property var off: prefs.pluginsOff && typeof prefs.pluginsOff === "object" ? prefs.pluginsOff : ({})
    readonly property bool typeToSearch: prefs.typeToSearch !== false
    readonly property bool help: SB.isHelp(text)
    readonly property var rows: !shown || !launcher ? [] : help ? SB.helpRows(text, savedOrder, off)
        : text.trim() === "" ? [] : SB.arrange(launcher.searchAll(text, "all"), savedOrder, 4)
    onRowsChanged: if (sel >= rows.length) sel = Math.max(0, rows.length - 1)
    onTextChanged: { sel = 0; if (launcher && !help) launcher.findFiles(text); }

    // Settings › Keyboard's layout switch (read only): Alt+Space is the layout switch's when it's chosen there.
    readonly property string kbPath: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/keyboard.json"
    property var kb: null
    FileView { path: sb.kbPath; watchChanges: true; printErrors: false; onFileChanged: reload(); onLoaded: { try { sb.kb = JSON.parse(text()); } catch (e) { sb.kb = null; } } onLoadFailed: sb.kb = null }
    readonly property bool altSpaceIsLayout: SB.layoutTakesAltSpace(kb)

    function open(t) {
        if (!launcher) return;
        if (typeof launcher.loadGames === "function") launcher.loadGames();
        if (typeof launcher.freshRates === "function") launcher.freshRates();
        if (launcher.shown) launcher.shown = false;
        text = t || "";
        sel = 0;
        shown = true;
        if (field) { field.text = text; field.cursorPosition = text.length; field.forceActiveFocus(); }
    }
    function close() { const kbd = fromPad; shown = false; fromPad = false; if (kbd && SettingsNav.pad) SettingsNav.pad.closeKeyboard(); }
    function toggle() { if (shown) close(); else open(""); }
    // Alt+Space itself: nothing when it's the layout switch (the layout already changed; the bar would be in the way).
    function key() { if (altSpaceIsLayout) return; toggle(); }
    // Typed on the desktop (Desktop.qml): the bar with those letters, if Settings › Search allows it.
    function openWith(t) { if (!typeToSearch) return false; open(t); return true; }
    // Y on the controller with the desktop in front (Pad.qml): the bar and the on-screen keyboard to type in it.
    function padOpen() { open(""); fromPad = true; if (SettingsNav.pad) SettingsNav.pad.openKeyboard(); return true; }
    function moveSel(d) { if (rows.length) sel = (sel + d + rows.length) % rows.length; }
    function enter() {
        const r = rows[sel];
        if (!r) return;
        if (r.kind === "help") { setText(r.fill || ""); return; }
        shown = false;
        launcher.openRow(r);
    }
    function setText(t) { text = t; if (field) { field.text = t; field.cursorPosition = t.length; field.forceActiveFocus(); } }
    function rowTitle(r) { return !r ? "" : r.kind === "help" ? r.title : launcher ? launcher.rowTitle(r) : ""; }
    function rowSub(r) { return !r ? "" : r.kind === "help" ? r.sub + (r.fill ? " · type " + r.fill : "") : launcher ? launcher.rowSub(r) : ""; }
    function rowIcon(r) {
        if (!r) return "search";
        if (r.kind === "help") return r.icon || "search";
        if (r.kind === "page") return r.item.icon || "gear";
        if (r.kind === "answer") return r.item.icon || "search";
        if (r.kind === "game") return "pad";
        return r.item && r.item.dir ? "folder" : "file";
    }
    property var field: null

    LazyLoader {
        active: sb.shown
        PanelWindow {
            id: win
            screen: Quickshell.screens[0]
            visible: true
            color: "transparent"
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "hearth-searchbar"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
            exclusionMode: ExclusionMode.Ignore
            anchors { top: true; bottom: true; left: true; right: true }

            MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons; onClicked: sb.close() }

            Rectangle {
                id: card
                readonly property real sw: win.width > 0 ? win.width : 1920
                width: Math.min(720, sw - 32)
                x: Math.round((sw - width) / 2)
                y: DeskState.topInset + 10
                height: col.implicitHeight + 16
                radius: Theme.r(14)
                color: Theme.panel
                border.color: Theme.line
                border.width: 1
                MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }   // a click inside doesn't close it

                ColumnLayout {
                    id: col
                    x: 8; y: 8
                    width: parent.width - 16
                    spacing: 6

                    // ---- the field
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 48
                        radius: Theme.r(10)
                        color: Theme.panel2
                        border.width: Theme.focusWidth
                        border.color: Theme.accent
                        RowLayout {
                            anchors { fill: parent; leftMargin: 14; rightMargin: 8 }
                            spacing: 10
                            Icon { name: "search"; size: 20; color: Theme.accent }
                            Item {
                                Layout.fillWidth: true
                                implicitHeight: 48
                                TextInput {
                                    id: input
                                    anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter }
                                    color: Theme.text; selectionColor: Theme.accent; selectedTextColor: Theme.accentInk
                                    font.family: Theme.font; font.pixelSize: 18
                                    clip: true
                                    focus: true
                                    Component.onCompleted: { sb.field = input; text = sb.text; cursorPosition = text.length; forceActiveFocus(); }
                                    Component.onDestruction: if (sb.field === input) sb.field = null
                                    onTextEdited: sb.text = text
                                    Keys.onPressed: event => {
                                        switch (event.key) {
                                        case Qt.Key_Escape: sb.close(); break;
                                        case Qt.Key_Down: case Qt.Key_Tab: sb.moveSel(1); break;
                                        case Qt.Key_Up: case Qt.Key_Backtab: sb.moveSel(-1); break;
                                        case Qt.Key_Return: case Qt.Key_Enter: sb.enter(); break;
                                        case Qt.Key_PageDown: sb.moveSel(5); break;
                                        case Qt.Key_PageUp: sb.moveSel(-5); break;
                                        default: return;
                                        }
                                        event.accepted = true;
                                    }
                                }
                                Text {
                                    anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter }
                                    visible: input.text === ""
                                    text: "Search apps, Settings, files, sums, the web…"
                                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 18; elide: Text.ElideRight
                                }
                            }
                            // "?": what you can type
                            Rectangle {
                                implicitWidth: 32; implicitHeight: 32; radius: 16
                                color: sb.help ? Theme.accent : helpMa.containsMouse ? Theme.panel3 : "transparent"
                                border.width: 1; border.color: sb.help ? Theme.accent : Theme.line
                                Text { anchors.centerIn: parent; text: "?"; color: sb.help ? Theme.accentInk : Theme.muted; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.Bold }
                                MouseArea { id: helpMa; anchors.fill: parent; hoverEnabled: true; onClicked: sb.setText(sb.help ? "" : "?") }
                            }
                        }
                    }

                    // ---- the results
                    ListView {
                        id: list
                        Layout.fillWidth: true
                        visible: sb.rows.length > 0
                        implicitHeight: Math.min(contentHeight, 9 * 50 + 24 * 3)
                        clip: true
                        interactive: contentHeight > height
                        model: sb.rows
                        currentIndex: sb.sel
                        highlightFollowsCurrentItem: false
                        onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                        delegate: Column {
                            id: rowItem
                            required property var modelData
                            required property int index
                            readonly property bool lit: index === sb.sel
                            readonly property bool firstOfGroup: index === 0 || sb.rows[index - 1].group !== modelData.group
                            width: list.width
                            Text {
                                visible: rowItem.firstOfGroup
                                height: visible ? 24 : 0
                                leftPadding: 10; verticalAlignment: Text.AlignBottom; bottomPadding: 3
                                text: rowItem.modelData.group || ""
                                color: rowItem.modelData.group === "Best match" ? Theme.accent : Theme.muted
                                font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold; font.letterSpacing: 0.6
                                font.capitalization: Font.AllUppercase
                            }
                            Rectangle {
                                width: parent.width
                                height: 50
                                radius: Theme.r(10)
                                color: rowItem.lit ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : rowMa.containsMouse ? Theme.panel2 : "transparent"
                                border.width: rowItem.lit ? Theme.focusWidth : 0
                                border.color: Theme.accent
                                RowLayout {
                                    anchors { fill: parent; leftMargin: 10; rightMargin: 12 }
                                    spacing: 12
                                    Item {
                                        implicitWidth: 34; implicitHeight: 34
                                        AppIcon { anchors.centerIn: parent; visible: rowItem.modelData.kind === "app"; size: 32; icon: rowItem.modelData.kind === "app" ? (rowItem.modelData.item.icon || "") : ""; appId: rowItem.modelData.kind === "app" ? (rowItem.modelData.item.id || "") : "" }
                                        Image {
                                            anchors.fill: parent; visible: rowItem.modelData.kind === "game" && status === Image.Ready
                                            fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 80
                                            source: rowItem.modelData.kind === "game" ? (rowItem.modelData.item.icon || rowItem.modelData.item.cover || "") : ""
                                        }
                                        Rectangle {
                                            anchors.fill: parent
                                            visible: rowItem.modelData.kind !== "app" && !(rowItem.modelData.kind === "game" && (rowItem.modelData.item.icon || rowItem.modelData.item.cover))
                                            radius: Theme.r(8); color: rowItem.modelData.kind === "help" && rowItem.modelData.off ? Theme.panel2 : Theme.panel3
                                            readonly property string glyph: rowItem.modelData.kind === "answer" ? (rowItem.modelData.item.glyph || "") : ""
                                            Icon { anchors.centerIn: parent; size: 18; visible: parent.glyph === ""; name: sb.rowIcon(rowItem.modelData); color: rowItem.lit ? Theme.accent : Theme.text }
                                            Text { anchors.centerIn: parent; visible: parent.glyph !== ""; text: parent.glyph; font.pixelSize: 20 }
                                        }
                                    }
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 1
                                        Text { Layout.fillWidth: true; text: sb.rowTitle(rowItem.modelData); elide: Text.ElideRight; color: rowItem.modelData.off ? Theme.muted : Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                                        Text { Layout.fillWidth: true; text: sb.rowSub(rowItem.modelData); elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                                    }
                                    Text {
                                        visible: rowItem.lit
                                        text: rowItem.modelData.kind === "help" ? "Enter: type it" : rowItem.modelData.kind === "answer" && rowItem.modelData.item.action && rowItem.modelData.item.action.type === "copy" ? "Enter: copy" : "Enter: open"
                                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                                    }
                                }
                                MouseArea {
                                    id: rowMa
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    onEntered: sb.sel = rowItem.index
                                    onClicked: { sb.sel = rowItem.index; sb.enter(); }
                                }
                            }
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        visible: sb.rows.length === 0
                        leftPadding: 10; topPadding: 2; bottomPadding: 2
                        text: sb.text.trim() === "" ? "Type to search · 2+2 · 10 km to mi · gg:words · >command · :emoji"
                            : "Nothing matches “" + sb.text.trim() + "”. Type ? to see what you can search for."
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight
                    }

                    // ---- keys and buttons
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.leftMargin: 10; Layout.rightMargin: 10
                        spacing: 14
                        Repeater {
                            model: [["↑↓", "move"], ["Enter", "open"], ["?", "help"], ["Esc", "close"]]
                            delegate: Row {
                                required property var modelData
                                spacing: 5
                                Rectangle {
                                    width: k.implicitWidth + 10; height: 18; radius: 4; color: Theme.panel3
                                    Text { id: k; anchors.centerIn: parent; text: modelData[0]; color: Theme.text; font.family: Theme.font; font.pixelSize: 11; font.weight: Font.DemiBold }
                                }
                                Text { text: modelData[1]; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; anchors.verticalCenter: parent.verticalCenter }
                            }
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: "Controller: A open · B close · Y keyboard"
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
                        }
                    }
                }
            }
        }
    }

    IpcHandler {
        target: "searchbar"
        function toggle(): void { sb.toggle(); }
        function open(): void { sb.open(""); }
        function close(): void { sb.close(); }
        function key(): void { sb.key(); }
        function type(t: string): void { if (!sb.shown) sb.open(t); else sb.setText(t); }
        function desk(t: string): string { return sb.openWith(t) ? "opened" : "off"; }
        function pad(): void { sb.padOpen(); }
        function move(n: int): void { sb.moveSel(n); }
        function enter(): void { sb.enter(); }
        function state(): string {
            return JSON.stringify({ shown: sb.shown, text: sb.text, sel: sb.sel, help: sb.help, altSpaceIsLayout: sb.altSpaceIsLayout,
                                    typeToSearch: sb.typeToSearch, order: SB.order(sb.savedOrder, []),
                                    rows: sb.rows.slice(0, 20).map(r => ({ kind: r.kind, group: r.group, plugin: r.plugin, title: sb.rowTitle(r) })) });
        }
    }
}

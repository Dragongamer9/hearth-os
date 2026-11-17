// Arrange the bar (1.14; KDE Plasma's panel edit mode, written for Hearth). Two tabs:
//   The bar's items: every item of the main bar (the flame, the desktops, the window in front, the tray's own
//     buttons…) at the start or the end, in your order, or hidden. ↑ ↓ move one, ⇄ takes it to the other end, the
//     eye hides or shows it. Game Mode always stays, last; the flame can't be hidden. Reset puts the default back.
//   Panels: more bars on any edge of any screen (ExtraPanels.qml), each with what you put in it; Clone the main bar
//     makes one like it (on another screen when there is one).
// Opens from a right-click on the bar or Settings › Bar and dock. Every change is saved at once, only that change
// (BarLayout.qml, bar.json). Keyboard and controller: Tab / arrows move, Enter presses, Esc (B) closes.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import "widget-tools.js" as T

PanelWindow {
    id: root
    readonly property bool shown: BarLayout.arranging
    visible: shown
    screen: Quickshell.screens.length ? Quickshell.screens[0] : null
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-bar-arrange"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: Qt.rgba(0, 0, 0, 0.35)
    onShownChanged: if (shown) keys.forceActiveFocus()
    function close() { BarLayout.arranging = false; }
    readonly property var zones: [{ id: "start", name: "At the start" }, { id: "end", name: "At the end" }, { id: "hidden", name: "Hidden" }]
    readonly property var screens: Quickshell.screens.map(s => s.name)
    function freeEdge(scr) {
        for (const e of ["bottom", "left", "right", "top"])
            if (!T.panelClash(BarLayout.panels, scr, e, screens[0] || "", DeskState.barEdge, Theme.dockFloats)) return e;
        return "";
    }
    function addPanel(clone) {
        let scr = screens[0] || "";
        if (clone && screens.length > 1) scr = screens[1];
        const e = freeEdge(scr);
        if (!e) { SettingsNav.toast("Every edge of this screen has a bar already"); return; }
        BarLayout.addPanel(scr, e, clone ? T.cloneMain(BarLayout.raw.items) : ["menu", "windows", "spacer", "clock", "spacer", "status", "notifications"]);
        SettingsNav.toast((clone ? "A copy of the main bar" : "A new panel") + " is on the " + e + (screens.length > 1 ? " of " + scr : ""));
    }

    MouseArea { anchors.fill: parent; onClicked: root.close() }
    Item {
        id: keys
        anchors.fill: parent
        focus: true
        Keys.onEscapePressed: root.close()
    }

    Rectangle {
        id: card
        anchors.centerIn: parent
        width: Math.min(980, parent.width - 40)
        height: Math.min(680, parent.height - 80)
        radius: Theme.radius + 8
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.97)
        border.color: Theme.line
        MouseArea { anchors.fill: parent }
        Loader { anchors.fill: parent; source: "Frame.qml"; onLoaded: { item.solid = true; item.rad = Qt.binding(() => Theme.radius + 8); } }

        ColumnLayout {
            anchors { fill: parent; margins: 22 }
            spacing: 14
            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                Icon { name: "grid"; size: 22; color: Theme.accent }
                Text { text: "Arrange the bar"; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold }
                Item { Layout.fillWidth: true }
                Tab { label: "The bar's items"; on: BarLayout.arrangeTab !== "panels"; onPicked: BarLayout.arrangeTab = "items" }
                Tab { label: "Panels" + (BarLayout.panels.length ? " (" + BarLayout.panels.length + ")" : ""); on: BarLayout.arrangeTab === "panels"; onPicked: BarLayout.arrangeTab = "panels" }
                Btn { text: "Done"; primary: true; onPicked: root.close() }
            }

            // ---------------------------------------------------------------- the bar's items
            RowLayout {
                visible: BarLayout.arrangeTab !== "panels"
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 14
                Repeater {
                    model: root.zones
                    delegate: Rectangle {
                        id: col
                        required property var modelData
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        radius: Theme.r(14)
                        color: Qt.rgba(Theme.panel2.r, Theme.panel2.g, Theme.panel2.b, 0.75)
                        ColumnLayout {
                            anchors { fill: parent; margins: 12 }
                            spacing: 6
                            Text { text: col.modelData.name; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                            ListView {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                clip: true
                                spacing: 4
                                model: BarLayout.layout[col.modelData.id]
                                delegate: Rectangle {
                                    id: it
                                    required property string modelData
                                    required property int index
                                    readonly property var meta: T.barItem(modelData)
                                    width: ListView.view.width
                                    height: 40
                                    radius: Theme.r(10)
                                    color: ih.hovered ? Theme.panel3 : Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.8)
                                    border.color: Theme.line
                                    HoverHandler { id: ih }
                                    RowLayout {
                                        anchors { fill: parent; leftMargin: 10; rightMargin: 4 }
                                        spacing: 4
                                        Icon { name: it.meta ? it.meta.icon : "dot"; size: 16; color: col.modelData.id === "hidden" ? Theme.muted : Theme.text }
                                        Text { Layout.fillWidth: true; text: it.meta ? it.meta.name : it.modelData; elide: Text.ElideRight; color: col.modelData.id === "hidden" ? Theme.muted : Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                                        Small { visible: ih.hovered && col.modelData.id !== "hidden"; icon: "chevu"; tip: "Earlier"; enabled: it.index > 0; onPicked: BarLayout.move(it.modelData, -1) }
                                        Small { visible: ih.hovered && col.modelData.id !== "hidden"; icon: "chevd"; tip: "Later"; enabled: it.index < BarLayout.layout[col.modelData.id].length - 1; onPicked: BarLayout.move(it.modelData, 1) }
                                        Small { visible: ih.hovered && col.modelData.id !== "hidden"; icon: col.modelData.id === "start" ? "forward" : "back"; tip: col.modelData.id === "start" ? "To the end" : "To the start"; onPicked: BarLayout.move(it.modelData, 0, col.modelData.id === "start" ? "end" : "start") }
                                        Small { visible: (ih.hovered || col.modelData.id === "hidden") && !(it.meta && it.meta.keep); icon: col.modelData.id === "hidden" ? "eye" : "close"; tip: col.modelData.id === "hidden" ? "Show it" : "Hide it"; onPicked: BarLayout.move(it.modelData, 0, col.modelData.id === "hidden" ? (it.meta ? it.meta.zone : "end") : "hidden") }
                                    }
                                }
                            }
                            Text {
                                visible: col.modelData.id === "end"
                                Layout.fillWidth: true
                                text: "Then Game Mode, always: it stays last, one press away"
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; wrapMode: Text.WordWrap
                            }
                            Text {
                                visible: col.modelData.id === "hidden" && BarLayout.layout.hidden.length === 0
                                Layout.fillWidth: true
                                text: "Nothing hidden. Items like the tray's printer or disks button only show while there's something to show"
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 11; wrapMode: Text.WordWrap
                            }
                        }
                    }
                }
            }
            RowLayout {
                visible: BarLayout.arrangeTab !== "panels"
                Layout.fillWidth: true
                Text { Layout.fillWidth: true; text: "Changes show in the bar straight away and are kept."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                Btn { text: "Back to the default"; enabled: BarLayout.raw.items !== undefined; onPicked: BarLayout.resetItems() }
            }

            // ---------------------------------------------------------------- panels
            ListView {
                visible: BarLayout.arrangeTab === "panels"
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                spacing: 10
                model: BarLayout.panels
                delegate: Rectangle {
                    id: pc
                    required property var modelData
                    readonly property var p: modelData
                    width: ListView.view.width
                    implicitHeight: pcol.implicitHeight + 24
                    radius: Theme.r(14)
                    color: Qt.rgba(Theme.panel2.r, Theme.panel2.g, Theme.panel2.b, 0.75)
                    ColumnLayout {
                        id: pcol
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12 }
                        spacing: 8
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 6
                            Icon { name: "window"; size: 18; color: Theme.accent }
                            Text { text: "Panel on the " + pc.p.edge + (root.screens.length > 1 ? " of " + (pc.p.screen || root.screens[0]) : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                            Text { visible: root.screens.indexOf(pc.p.screen || root.screens[0]) < 0; text: "· its screen isn't plugged in"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                            Item { Layout.fillWidth: true }
                            Repeater {
                                model: T.EDGES
                                delegate: Chip {
                                    required property string modelData
                                    label: modelData
                                    on: pc.p.edge === modelData
                                    onPicked: {
                                        if (modelData === pc.p.edge) return;
                                        const clash = T.panelClash(BarLayout.panels.filter(x => x.id !== pc.p.id), pc.p.screen || root.screens[0], modelData, root.screens[0] || "", DeskState.barEdge, Theme.dockFloats);
                                        if (clash) SettingsNav.toast(clash); else BarLayout.setPanel(pc.p.id, { edge: modelData });
                                    }
                                }
                            }
                            Chip { visible: root.screens.length > 1; label: "Next screen"; onPicked: { const i = root.screens.indexOf(pc.p.screen || root.screens[0]); BarLayout.setPanel(pc.p.id, { screen: root.screens[(i + 1) % root.screens.length] }); } }
                            Chip { label: pc.p.size <= 36 ? "Thin" : pc.p.size >= 56 ? "Big" : "Normal"; onPicked: BarLayout.setPanel(pc.p.id, { size: pc.p.size <= 36 ? 44 : pc.p.size >= 56 ? 36 : 56 }) }
                            Small { icon: "trash"; tip: "Remove this panel"; onPicked: BarLayout.removePanel(pc.p.id) }
                        }
                        Flow {
                            Layout.fillWidth: true
                            spacing: 6
                            Repeater {
                                model: pc.p.items
                                delegate: Chip {
                                    required property string modelData
                                    required property int index
                                    readonly property var meta: T.panelItem(modelData)
                                    label: (meta ? meta.name : modelData) + "  ×"
                                    on: true
                                    onPicked: { const l = pc.p.items.slice(); l.splice(index, 1); BarLayout.setPanel(pc.p.id, { items: l }); }
                                }
                            }
                        }
                        Text { text: "Add to it"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                        Flow {
                            Layout.fillWidth: true
                            spacing: 6
                            Repeater {
                                model: T.PANEL_ITEMS.filter(x => x.id === "spacer" || pc.p.items.indexOf(x.id) < 0)
                                delegate: Chip {
                                    required property var modelData
                                    label: "+ " + modelData.name
                                    onPicked: BarLayout.setPanel(pc.p.id, { items: pc.p.items.concat([modelData.id]) })
                                }
                            }
                        }
                    }
                }
                Text {
                    visible: BarLayout.panels.length === 0
                    anchors.centerIn: parent
                    width: parent.width * 0.7
                    horizontalAlignment: Text.AlignHCenter
                    text: "No other panels yet. Add one for a second screen, or a strip down the side with your windows and a clock."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
                }
            }
            RowLayout {
                visible: BarLayout.arrangeTab === "panels"
                Layout.fillWidth: true
                spacing: 8
                Btn { text: "Add a panel"; primary: true; onPicked: root.addPanel(false) }
                Btn { text: "Clone the main bar"; onPicked: root.addPanel(true) }
                Item { Layout.fillWidth: true }
                Text { text: "Right-click a panel to come back here"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            }
        }
    }

    component Btn: Rectangle {
        property string text: ""
        property bool primary: false
        signal picked()
        activeFocusOnTab: true
        implicitWidth: bt.implicitWidth + 28; implicitHeight: 34
        radius: Theme.r(10)
        opacity: enabled ? 1 : 0.45
        color: primary ? (ba.containsMouse ? Qt.lighter(Theme.accent, 1.1) : Theme.accent) : ba.containsMouse || activeFocus ? Theme.panel3 : Theme.panel2
        border.width: activeFocus ? 2 : 0; border.color: Theme.accent
        Text { id: bt; anchors.centerIn: parent; text: parent.text; color: parent.primary ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
        MouseArea { id: ba; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: parent.picked() }
        Keys.onReturnPressed: picked()
        Keys.onSpacePressed: picked()
    }
    component Tab: Btn { property string label: ""; property bool on: false; text: label; primary: on }
    component Chip: Rectangle {
        property string label: ""
        property bool on: false
        signal picked()
        activeFocusOnTab: true
        implicitWidth: ct.implicitWidth + 20; implicitHeight: 28
        radius: Theme.r(14)
        color: on ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22) : ca.containsMouse || activeFocus ? Theme.panel3 : Theme.panel
        border.width: activeFocus ? 2 : 1; border.color: on || activeFocus ? Theme.accent : Theme.line
        Text { id: ct; anchors.centerIn: parent; text: parent.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 12 }
        MouseArea { id: ca; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: parent.picked() }
        Keys.onReturnPressed: picked()
        Keys.onSpacePressed: picked()
    }
    component Small: Rectangle {
        property string icon: ""
        property string tip: ""
        signal picked()
        activeFocusOnTab: true
        implicitWidth: 28; implicitHeight: 28
        radius: 14
        opacity: enabled ? 1 : 0.3
        color: sa.containsMouse || activeFocus ? Theme.panel3 : "transparent"
        border.width: activeFocus ? 2 : 0; border.color: Theme.accent
        Icon { anchors.centerIn: parent; name: parent.icon; size: 14 }
        MouseArea { id: sa; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: parent.picked() }
        Keys.onReturnPressed: picked()
        Keys.onSpacePressed: picked()
    }
}

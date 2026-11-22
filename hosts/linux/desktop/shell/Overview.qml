// The overview for desktop layouts (docs/mockups/desktop-layouts/layouts-3-gnome-style-overview.png), where KWin's
// own Overview isn't there (labwc): "Type to search" at the top, the desktop you're on shrunk with its windows side by
// side and their names under them, the previous and the next desktops at the sides, and the dash (your pinned apps, a
// line, the app grid). Opened by the desktops pill and Task view (LayoutBlock.qml), Super+Tab when a layout has them,
// or `quickshell ipc -p <shell> call overview toggle`. Esc, a click outside or opening something closes it.
// Typing searches the windows here; Enter with nothing matching searches the app menu.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: ov
    property bool shown: false
    property string query: ""
    signal searchRequested(string text)
    visible: shown
    anchors { top: true; bottom: true; left: true; right: true }
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    WlrLayershell.namespace: "hearth-overview"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None

    function toggle() { if (shown) close(); else open(); }
    function open() { query = ""; shown = true; keys.forceActiveFocus(); }
    function close() { shown = false; }

    readonly property var wins: (WinList.toplevels ? WinList.toplevels.values : [])
        .filter(t => !/quickshell|hearth-(launcher|overview)/i.test(t.appId || "") && (t.title || t.appId))
    readonly property var shownWins: query === "" ? wins : wins.filter(t => (String(t.title || "") + " " + String(t.appId || "")).toLowerCase().indexOf(query.toLowerCase()) >= 0)
    readonly property var dash: (DeskState.pins || []).slice(0, 10).map(id => AppIcons.entryFor(id)).filter(e => !!e)

    Loader { id: deskData; active: ov.shown; visible: false; source: "DeskSwitcher.qml" }
    readonly property int deskCount: deskData.item ? deskData.item.sets.length : 1
    readonly property int deskAt: deskData.item ? Math.max(0, deskData.item.activeIndex) : 0

    Rectangle { anchors.fill: parent; color: Qt.rgba(Theme.bg.r, Theme.bg.g, Theme.bg.b, 0.94) }
    MouseArea { anchors.fill: parent; onClicked: ov.close() }

    Item {
        id: keys
        anchors.fill: parent
        focus: true
        Keys.onEscapePressed: ov.close()
        Keys.onPressed: e => {
            if (e.key === Qt.Key_Backspace) { ov.query = ov.query.slice(0, -1); e.accepted = true; }
            else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) {
                if (ov.shownWins.length) { ov.shownWins[0].activate(); ov.close(); }
                else if (ov.query !== "") { const q = ov.query; ov.close(); ov.searchRequested(q); }
                e.accepted = true;
            } else if (e.text && e.text.length === 1 && e.text >= " ") { ov.query += e.text; e.accepted = true; }
        }
    }

    // ---- "Type to search"
    Rectangle {
        id: search
        anchors { horizontalCenter: parent.horizontalCenter; top: parent.top; topMargin: 66 }
        width: Math.min(600, parent.width - 80); height: 46
        radius: Theme.r(23)
        color: Qt.rgba(Theme.bg2.r, Theme.bg2.g, Theme.bg2.b, 0.9)
        border.color: ov.query !== "" ? Theme.accent : Theme.line
        Row {
            anchors { left: parent.left; leftMargin: 20; verticalCenter: parent.verticalCenter }
            spacing: 12
            Icon { name: "search"; size: 18; color: Theme.accent; anchors.verticalCenter: parent.verticalCenter }
            Text { text: ov.query === "" ? "Type to search" : ov.query; color: ov.query === "" ? Theme.muted : Theme.text; font.family: Theme.font; font.pixelSize: 15; anchors.verticalCenter: parent.verticalCenter }
        }
    }

    // ---- the previous and the next desktops, at the sides
    Rectangle {
        visible: ov.deskAt > 0
        anchors { right: stage.left; rightMargin: 30; verticalCenter: stage.verticalCenter }
        width: 220; height: stage.height * 0.8; radius: Theme.r(18)
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.35)
        MouseArea { anchors.fill: parent; onClicked: if (deskData.item) deskData.item.go(ov.deskAt - 1) }
    }
    Rectangle {
        visible: ov.deskAt < ov.deskCount - 1
        anchors { left: stage.right; leftMargin: 30; verticalCenter: stage.verticalCenter }
        width: 220; height: stage.height * 0.8; radius: Theme.r(18)
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.35)
        MouseArea { anchors.fill: parent; onClicked: if (deskData.item) deskData.item.go(ov.deskAt + 1) }
    }

    // ---- the desktop you're on, shrunk: its windows side by side, their names under them
    Rectangle {
        id: stage
        anchors { horizontalCenter: parent.horizontalCenter; top: search.bottom; topMargin: 40; bottom: dashBox.top; bottomMargin: 30 }
        width: Math.min(parent.width - 500, height * 1.6)
        radius: Theme.r(24)
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.45)
        MouseArea { anchors.fill: parent; onClicked: ov.close() }
        Flow {
            id: flow
            anchors.centerIn: parent
            width: parent.width - 120
            spacing: 50
            readonly property real cardW: Math.max(220, Math.min(560, (width - spacing) / Math.max(1, Math.min(3, ov.shownWins.length))))
            Repeater {
                model: ov.shownWins.slice(0, 9)
                Column {
                    id: card
                    required property var modelData
                    spacing: 18
                    Rectangle {
                        width: flow.cardW; height: flow.cardW * 0.66
                        radius: Theme.r(10)
                        color: Theme.bg2
                        border.color: ca.containsMouse || card.modelData.activated ? Theme.accent : Theme.line
                        border.width: ca.containsMouse ? 2 : 1
                        Rectangle {   // its title bar
                            width: parent.width; height: 40; radius: parent.radius
                            color: Theme.panel
                            Text { anchors.centerIn: parent; width: parent.width - 30; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideRight; text: card.modelData.title || ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                        }
                        AppIcon { anchors.centerIn: parent; anchors.verticalCenterOffset: 20; size: 64; appId: card.modelData.appId || ""; icon: "" }
                        MouseArea { id: ca; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { card.modelData.activate(); ov.close(); } }
                    }
                    Text {
                        readonly property var e: AppIcons.entryFor(card.modelData.appId)
                        text: e ? String(e.name) : String(card.modelData.appId || card.modelData.title || "")
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
                    }
                }
            }
        }
        Text { visible: ov.shownWins.length === 0; anchors.centerIn: parent; text: ov.query === "" ? "No windows open" : "No window called “" + ov.query + "”: Enter searches your apps"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 15 }
    }

    // ---- the dash: your pinned apps, a line, the app grid
    Rectangle {
        id: dashBox
        anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 26 }
        width: dashRow.implicitWidth + 24; height: 76
        radius: Theme.r(20)
        color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.85)
        border.color: Theme.line
        Row {
            id: dashRow
            anchors.centerIn: parent
            spacing: 8
            Repeater {
                model: ov.dash
                Rectangle {
                    required property var modelData
                    width: 56; height: 56; radius: Theme.r(14)
                    color: da.containsMouse ? Theme.panel3 : "transparent"
                    AppIcon { anchors.centerIn: parent; size: 44; appId: parent.modelData.id || ""; icon: parent.modelData.icon || "" }
                    MouseArea { id: da; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { parent.modelData.execute(); ov.close(); } }
                }
            }
            Rectangle { visible: ov.dash.length > 0; width: 1; height: 44; color: Theme.line; anchors.verticalCenter: parent.verticalCenter }
            Rectangle {
                width: 56; height: 56; radius: Theme.r(14)
                color: ga.containsMouse ? Theme.panel3 : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.06)
                Icon { anchors.centerIn: parent; name: "grid"; size: 26 }
                MouseArea { id: ga; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { ov.close(); ov.searchRequested(""); } }
            }
        }
    }
}

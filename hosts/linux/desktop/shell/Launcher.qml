// App launcher: a full-screen overlay with search and a grid of installed apps, plus power actions.
// Opens with Super, the Nimbus button on the taskbar, or `quickshell ipc ... call launcher toggle`.
// Type to search, arrows to move, Enter to open, Esc to close.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Quickshell.Widgets

PanelWindow {
    id: root
    property bool shown: false
    signal settingsRequested()
    function toggle() { shown = !shown; }

    visible: shown
    onShownChanged: if (shown) { search.text = ""; grid.currentIndex = 0; search.forceActiveFocus(); }

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-launcher"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "#b0080b10"

    // Every app with a .desktop file, A–Z, filtered by what you type.
    readonly property var apps: Array.from(DesktopEntries.applications.values)
        .filter(a => !a.noDisplay && !/^(qt5ct|qt6ct)$/.test(a.id || ""))
        .sort((a, b) => a.name.localeCompare(b.name))
    readonly property var matches: {
        const q = search.text.trim().toLowerCase();
        if (!q) return apps;
        const hit = a => [a.name, a.genericName, a.comment, Array.from(a.keywords || []).join(" ")].join(" ").toLowerCase().includes(q);
        // Names that start with the query first.
        return apps.filter(hit).sort((a, b) => (b.name.toLowerCase().startsWith(q) ? 1 : 0) - (a.name.toLowerCase().startsWith(q) ? 1 : 0));
    }

    function launch(entry) {
        if (!entry) return;
        entry.execute();
        shown = false;
    }

    MouseArea { anchors.fill: parent; onClicked: root.shown = false } // click outside closes

    Rectangle {
        id: box
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, 880)
        height: Math.min(parent.height - 120, 640)
        radius: 20
        color: Theme.panel
        border.color: Theme.line
        MouseArea { anchors.fill: parent } // clicks inside don't close

        ColumnLayout {
            anchors { fill: parent; margins: 22 }
            spacing: 16

            // Search
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 48
                radius: 12
                color: Theme.bg2
                border.color: search.activeFocus ? Theme.accent : Theme.line
                TextInput {
                    id: search
                    anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
                    verticalAlignment: TextInput.AlignVCenter
                    color: Theme.text
                    selectionColor: Theme.accent
                    font.pixelSize: 17
                    font.family: Theme.font
                    onTextChanged: grid.currentIndex = 0
                    Keys.onEscapePressed: root.shown = false
                    Keys.onReturnPressed: root.launch(root.matches[grid.currentIndex])
                    Keys.onEnterPressed: root.launch(root.matches[grid.currentIndex])
                    Keys.onDownPressed: grid.moveCurrentIndexDown()
                    Keys.onUpPressed: grid.moveCurrentIndexUp()
                    Keys.onRightPressed: event => { if (cursorPosition === text.length) grid.moveCurrentIndexRight(); else event.accepted = false; }
                    Keys.onLeftPressed: event => { if (cursorPosition === text.length && text === "") grid.moveCurrentIndexLeft(); else event.accepted = false; }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: !search.text
                        text: "Search apps"
                        color: Theme.muted
                        font: search.font
                    }
                }
            }

            // Apps
            GridView {
                id: grid
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: root.matches
                cellWidth: Math.floor(width / Math.max(1, Math.floor(width / 120)))
                cellHeight: 112
                keyNavigationEnabled: true
                highlightFollowsCurrentItem: true
                highlight: Rectangle { radius: 14; color: Theme.panel3; border.color: Theme.accent }
                delegate: Item {
                    id: cell
                    required property var modelData
                    required property int index
                    width: grid.cellWidth
                    height: grid.cellHeight
                    Rectangle {
                        anchors { fill: parent; margins: 4 }
                        radius: 14
                        color: hover.containsMouse && grid.currentIndex !== cell.index ? Theme.panel2 : "transparent"
                    }
                    Column {
                        anchors.centerIn: parent
                        spacing: 8
                        IconImage {
                            anchors.horizontalCenter: parent.horizontalCenter
                            implicitSize: 48
                            source: Quickshell.iconPath(cell.modelData.icon, "application-x-executable")
                        }
                        Text {
                            width: grid.cellWidth - 16
                            horizontalAlignment: Text.AlignHCenter
                            elide: Text.ElideRight
                            maximumLineCount: 2
                            wrapMode: Text.Wrap
                            text: cell.modelData.name
                            color: Theme.text
                            font.pixelSize: 12
                            font.family: Theme.font
                        }
                    }
                    MouseArea {
                        id: hover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: root.launch(cell.modelData)
                    }
                }
                Text {
                    anchors.centerIn: parent
                    visible: grid.count === 0
                    text: "No apps match \"" + search.text + "\""
                    color: Theme.muted
                    font.pixelSize: 14
                    font.family: Theme.font
                }
            }

            // Power
            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                ActionButton { label: "Game Mode"; primary: true; command: ["nimbus-mode", "game"] }
                ActionButton { label: "Settings"; command: [] }
                Item { Layout.fillWidth: true }
                ActionButton { label: "Sleep"; command: ["systemctl", "suspend"] }
                ActionButton { label: "Restart"; command: ["systemctl", "reboot"] }
                ActionButton { label: "Shut down"; command: ["systemctl", "poweroff"] }
                ActionButton { label: "Log out"; command: ["nimbus-mode", "logout"] }
            }
        }
    }

    component ActionButton: Rectangle {
        id: ab
        property string label
        property var command: []
        property bool primary: false
        implicitWidth: abText.implicitWidth + 32
        implicitHeight: 40
        radius: 10
        color: primary ? (abArea.containsMouse ? Qt.lighter(Theme.accent, 1.12) : Theme.accent) : (abArea.containsMouse ? Theme.panel3 : Theme.panel2)
        Text { id: abText; anchors.centerIn: parent; text: ab.label; color: ab.primary ? Theme.accentInk : Theme.text; font.pixelSize: 13; font.bold: ab.primary; font.family: Theme.font }
        MouseArea {
            id: abArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: { root.shown = false; if (ab.command.length) Quickshell.execDetached(ab.command); else root.settingsRequested(); }
        }
    }
}

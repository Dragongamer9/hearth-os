// On-screen keyboard for Desktop Mode. Y on the controller (or the keyboard button on the taskbar,
// or a swipe in from the left edge) opens it. Pick keys with the D-pad or left stick, A to type,
// X backspace, LB shift, Menu (☰) Enter, B or Y close; or just tap the keys.
// It takes its own space at the bottom of the screen, so windows shrink above it and the field you're
// typing in stays visible. hearth-pad sends the layout and the highlighted key, and does the typing.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: root
    property bool open: false
    property var rows: []
    property var shifted: ({})
    property int row: 1
    property int col: 0
    property bool shift: false
    signal tapped(string key)
    signal closeRequested()

    function update(m) {
        open = m.open;
        if (!m.open) return;
        rows = m.rows; shifted = m.shifted; row = m.row; col = m.col; shift = m.shift;
    }

    readonly property var labels: ({ shift: "⇧ Shift", space: "Space", bksp: "⌫", tab: "Tab ⇥", enter: "Enter ↵" })
    function label(k) {
        if (labels[k]) return labels[k];
        return shift ? (shifted[k] || k.toUpperCase()) : k;
    }

    visible: open
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-keyboard"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None       // typing goes to the window you're in
    anchors { bottom: true; left: true; right: true }
    implicitHeight: Math.round(Math.min(screen.height * 0.4, Math.max(260, screen.width * 0.22)))
    exclusiveZone: implicitHeight
    color: "transparent"

    Rectangle {
        anchors { fill: parent; leftMargin: 8; rightMargin: 8; topMargin: 6 }
        radius: Theme.radius + 4
        color: Theme.glass
        border.color: Theme.line

        ColumnLayout {
            anchors { fill: parent; margins: 12 }
            spacing: 8

            Repeater {
                model: root.rows
                delegate: RowLayout {
                    id: line
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 8
                    Repeater {
                        model: line.modelData
                        delegate: Rectangle {
                            id: key
                            required property string modelData
                            required property int index
                            readonly property bool wide: root.labels[modelData] !== undefined
                            readonly property bool lit: root.row === line.index && root.col === index
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            Layout.preferredWidth: wide ? 220 : 100
                            radius: 10
                            color: (modelData === "shift" && root.shift) ? Theme.accent
                                 : touch.pressed ? Theme.panel3 : lit ? Theme.panel3 : Theme.panel2
                            border.width: 2
                            border.color: lit ? Theme.accent : "transparent"
                            Behavior on border.color { ColorAnimation { duration: 90 } }
                            Text {
                                anchors.centerIn: parent
                                text: root.label(key.modelData)
                                color: (key.modelData === "shift" && root.shift) ? Theme.accentInk : Theme.text
                                font.family: Theme.font
                                font.pixelSize: Math.max(14, key.height * 0.36)
                                font.weight: Font.Medium
                            }
                            MouseArea { id: touch; anchors.fill: parent; onClicked: root.tapped(key.modelData) }
                        }
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Text {
                    Layout.fillWidth: true
                    text: "A type · X ⌫ · LB shift · ☰ Enter · B close · right stick + RT click a field"
                    color: Theme.muted
                    font.family: Theme.font
                    font.pixelSize: 12
                    elide: Text.ElideRight
                }
                Text {
                    text: "Close"
                    color: Theme.accent
                    font.family: Theme.font
                    font.pixelSize: 13
                    font.weight: Font.DemiBold
                    MouseArea { anchors.fill: parent; anchors.margins: -8; onClicked: root.closeRequested() }
                }
            }
        }
    }
}

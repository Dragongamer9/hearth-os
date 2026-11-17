// A button on the login screen (a chip, or a row in a menu): click or tap it, or pick it with the arrows / D-pad and
// press Enter / A.
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: chip
    property string text: ""
    property string sub: ""
    property string glyph: ""
    property string layer: "main"
    property bool navigable: true
    property bool preferred: false     // where the ring starts on this layer
    property bool primary: false
    property bool checked: false
    property bool wide: false          // a menu row: as wide as its menu, text on the left
    property string name: text         // what the tests and the screen reader call it
    property real fontPx: Theme.px(16)
    signal activated()
    readonly property bool focused: Nav.current === chip && Nav.ring
    readonly property bool hovered: mouse.containsMouse
    implicitWidth: wide ? Theme.px(460) : row.implicitWidth + Theme.px(30)
    implicitHeight: Math.max(Theme.px(sub !== "" ? 64 : 46), row.implicitHeight + Theme.px(16))
    radius: Theme.px(12)
    color: primary ? (mouse.pressed ? Qt.darker(Theme.accent, 1.15) : Theme.accent)
        : (mouse.pressed ? Theme.panel3 : (mouse.containsMouse || focused) ? Theme.panel3 : Theme.panel2)
    border.width: focused ? 3 : 1
    border.color: focused ? (primary ? Theme.text : Theme.accent) : (Theme.contrast ? Theme.line : Qt.rgba(1, 1, 1, 0.06))
    opacity: enabled ? 1 : 0.45
    Accessible.role: Accessible.Button
    Accessible.name: chip.name
    Accessible.description: chip.sub
    Accessible.onPressAction: chip.activated()

    RowLayout {
        id: row
        anchors { left: parent.left; right: chip.wide ? parent.right : undefined; verticalCenter: parent.verticalCenter; leftMargin: Theme.px(15); rightMargin: Theme.px(15) }
        spacing: Theme.px(10)
        Text {
            visible: chip.glyph !== ""
            text: chip.glyph
            color: chip.primary ? Theme.accentInk : Theme.text
            font.family: Theme.font; font.pixelSize: chip.fontPx + 2
        }
        ColumnLayout {
            spacing: 2
            Layout.fillWidth: chip.wide
            Text {
                visible: chip.text !== ""
                text: chip.text
                color: chip.primary ? Theme.accentInk : Theme.text
                font.family: Theme.font; font.pixelSize: chip.fontPx; font.weight: chip.primary ? Font.DemiBold : Font.Medium
            }
            Text {
                visible: chip.sub !== ""
                text: chip.sub
                color: chip.primary ? Theme.accentInk : Theme.muted
                font.family: Theme.font; font.pixelSize: Math.round(chip.fontPx * 0.8)
                Layout.maximumWidth: Theme.px(400)
                wrapMode: Text.WordWrap
            }
        }
        Text {
            visible: chip.checked
            text: "✓"
            color: Theme.accent
            font.family: Theme.font; font.pixelSize: chip.fontPx + 2; font.weight: Font.Bold
        }
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: { Nav.current = chip; chip.activated(); }
    }
    Component.onCompleted: Nav.add(chip)
    Component.onDestruction: Nav.remove(chip)
}

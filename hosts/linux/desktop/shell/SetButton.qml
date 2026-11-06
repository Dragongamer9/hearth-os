// A button on a settings page or card (Rename, Clean up, Switch to Game Mode). Takes focus like a card.
import QtQuick
import QtQuick.Layouts

SetBase {
    id: b
    property string text
    property string icon
    property bool primary: false
    property bool ghost: false
    Layout.fillWidth: false
    implicitWidth: row.implicitWidth + 32
    implicitHeight: 34
    radius: Math.max(6, Theme.r(8))
    color: primary ? Theme.accent : ghost ? "transparent" : SetStyle.card2
    border.width: ghost ? 1 : 0
    border.color: Theme.line
    Accessible.role: Accessible.Button
    Accessible.name: text
    Row {
        id: row
        anchors.centerIn: parent
        spacing: 8
        Icon { visible: b.icon !== ""; anchors.verticalCenter: parent.verticalCenter; name: b.icon; size: 16; color: b.primary ? Theme.accentInk : Theme.text }
        Text { anchors.verticalCenter: parent.verticalCenter; text: b.text; color: b.primary ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: b.primary ? Font.DemiBold : Font.Medium }
    }
}

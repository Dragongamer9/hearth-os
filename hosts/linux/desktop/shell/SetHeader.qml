// A titled group's heading in a settings page ("Brightness & colour", "Media & files"): the cards under it, up to the
// next heading, are one group.
import QtQuick
import QtQuick.Layouts

Text {
    Layout.fillWidth: true
    Layout.topMargin: 19
    Layout.bottomMargin: 6
    color: Theme.text
    font.family: Theme.font
    font.pixelSize: SetStyle.compact ? 14 : 15
    font.weight: Font.DemiBold
    elide: Text.ElideRight
}

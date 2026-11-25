// A titled group's heading in a settings page ("Brightness & colour", "Media & files"): the cards under it, up to the
// next heading, are one group.
import QtQuick
import QtQuick.Layouts

Text {
    // (a page's groups start here: AppBlocksFlow.qml makes each group a block)
    readonly property bool isSetHeader: true
    Layout.fillWidth: true
    Layout.topMargin: 19
    Layout.bottomMargin: 6
    color: Theme.text
    font.family: Theme.font
    font.pixelSize: SetStyle.compact ? 14 : 15
    font.weight: Font.DemiBold
    elide: Text.ElideRight
}

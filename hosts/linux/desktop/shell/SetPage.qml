// A scrolling settings page: title, then the page's rows.
import QtQuick
import QtQuick.Layouts

Flickable {
    id: page
    property string title
    default property alias rows: col.data
    contentHeight: col.implicitHeight + 48
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    Behavior on contentY { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutCubic } }
    function focusFirst() { start.nextItemInFocusChain(true).forceActiveFocus(); }

    ColumnLayout {
        id: col
        width: Math.min(page.width - 48, 820)
        x: 24
        y: 20
        spacing: 8
        Item { id: start; implicitHeight: 0 }
        Text { text: page.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold; Layout.bottomMargin: 6 }
    }
}

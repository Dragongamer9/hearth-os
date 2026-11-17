// The bar's activity button (Activities.qml): the activity you're on; click for the switcher, scroll to step
// through them. Only shown once you have more than one activity (Main alone is the desktop as it always was).
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: chip
    readonly property bool wanted: Activities.many
    property real pillSize: 24
    implicitWidth: row.implicitWidth + 18
    implicitHeight: pillSize + 4
    radius: height / 2
    color: ma.containsMouse ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.3) : Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16)
    border.width: 1
    border.color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.5)
    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: 6
        Icon { name: Activities.cur ? Activities.cur.icon : "home"; size: chip.pillSize * 0.65; color: Theme.accent }
        Text { text: Activities.name; color: Theme.text; font.family: Theme.font; font.pixelSize: Math.round(chip.pillSize * 0.55); font.weight: Font.DemiBold }
    }
    MouseArea {
        id: ma
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: Activities.openSwitcher()
        onWheel: wheel => Activities.step(wheel.angleDelta.y < 0 ? 1 : -1)
    }
}

// A progress bar (downloads, performance).
import QtQuick

Rectangle {
    id: bar
    property real value: 0          // 0..1
    property color fill: Theme.accent
    implicitHeight: Theme.u(6)
    radius: height / 2
    color: Theme.alpha("#000000", 0.35)
    Rectangle {
        width: Math.max(0, Math.min(1, bar.value)) * parent.width
        height: parent.height
        radius: parent.radius
        color: bar.fill
        Behavior on width { NumberAnimation { duration: Theme.calm } }
    }
}

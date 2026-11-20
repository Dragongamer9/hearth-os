// Firehold's slider (volume, brightness), mockups 07-08.
import QtQuick
import "fh.js" as F

Item {
    id: sl
    property real value: 0          // 0..1
    property bool focused: false
    property bool enabled2: true
    signal moved(real v)
    function adjust(d) { const v = Math.max(0, Math.min(1, value + d * 0.05)); moved(v); }
    function activate() {}
    implicitHeight: 28
    Rectangle {
        id: track
        anchors { left: parent.left; right: parent.right; leftMargin: 6; rightMargin: 6; verticalCenter: parent.verticalCenter }
        height: 8; radius: 4
        color: F.dot
        Rectangle { width: Math.max(8, track.width * sl.value); height: parent.height; radius: 4; color: sl.enabled2 ? F.accent : F.muted }
    }
    Rectangle {
        x: track.x + track.width * sl.value - width / 2
        anchors.verticalCenter: parent.verticalCenter
        width: 22; height: 22; radius: 11
        color: "#ffffff"
        border.width: sl.focused ? 3 : 0
        border.color: F.accent
    }
    MouseArea {
        anchors.fill: parent
        enabled: sl.enabled2
        function at(x) { return Math.max(0, Math.min(1, (x - track.x) / Math.max(1, track.width))); }
        onPressed: mouse => sl.moved(at(mouse.x))
        onPositionChanged: mouse => { if (pressed) sl.moved(at(mouse.x)); }
    }
}

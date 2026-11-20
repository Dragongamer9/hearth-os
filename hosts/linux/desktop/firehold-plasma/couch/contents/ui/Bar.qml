// The mockups' .prog: a 7 px track with the accent filling it (0..100).
import QtQuick

Rectangle {
    id: b
    property real k: 1
    property real value: 0
    property color fill: "#ff7a2f"
    height: 7 * k
    radius: 4 * k
    color: "#1affffff"
    Rectangle { width: parent.width * Math.max(0, Math.min(100, b.value)) / 100; height: parent.height; radius: parent.radius; color: b.fill }
}

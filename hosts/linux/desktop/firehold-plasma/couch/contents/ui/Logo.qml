// The Firehold logo tile: the flame on the accent colour.
import QtQuick
import QtQuick.Shapes

Rectangle {
    id: logo
    property real k: 1
    width: 44 * k; height: 44 * k; radius: 10 * k
    color: "#ff8a3d"
    Shape {
        anchors.centerIn: parent
        width: 24; height: 24
        scale: logo.k * 0.75
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: "#2a1206"
            strokeWidth: 2.2
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: "M12 3 C 12 7 7 9 7 14 A 5 5 0 0 0 17 14 C 17 11 15 9.5 14.5 8 C 14 10 13 11 12 11 C 12.5 8 12.5 5 12 3 Z" }
        }
    }
}

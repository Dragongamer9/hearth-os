// One of the Desk bar's tray buttons (the mockups' .tray .t): a line icon (or the download ring) and a short text.
import QtQuick
import QtQuick.Shapes
import "couch"

Rectangle {
    id: tr
    property string glyph: ""
    property string text: ""
    property real ring: -2          // a download's percent (-1: waiting); -2: no ring
    signal clicked()
    height: 34; radius: 9
    width: Math.max(34, row.implicitWidth + 18)
    color: ma.containsMouse ? "#1affffff" : "transparent"
    Row {
        id: row
        anchors.centerIn: parent
        spacing: 6
        Glyph { visible: tr.glyph !== ""; name: tr.glyph || "pad"; size: 17; color: "#dddddd"; anchors.verticalCenter: parent.verticalCenter }
        Shape {
            visible: tr.ring > -2
            width: 18; height: 18
            anchors.verticalCenter: parent.verticalCenter
            preferredRendererType: Shape.CurveRenderer
            ShapePath { strokeColor: "#444444"; strokeWidth: 3; fillColor: "transparent"; PathAngleArc { centerX: 9; centerY: 9; radiusX: 7.5; radiusY: 7.5; startAngle: 0; sweepAngle: 360 } }
            ShapePath { strokeColor: "#ff7a2f"; strokeWidth: 3; fillColor: "transparent"; PathAngleArc { centerX: 9; centerY: 9; radiusX: 7.5; radiusY: 7.5; startAngle: -90; sweepAngle: 360 * Math.max(0.05, tr.ring / 100) } }
        }
        Text { visible: tr.text !== ""; text: tr.text; color: "#dddddd"; font.pixelSize: 14; anchors.verticalCenter: parent.verticalCenter }
    }
    MouseArea { id: ma; anchors.fill: parent; hoverEnabled: true; onClicked: tr.clicked() }
}

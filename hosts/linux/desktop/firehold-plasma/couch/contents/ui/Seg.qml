// The mockups' .seg: a row of choices, the picked one in the accent colour (or the colour given).
import QtQuick
import "fhp.js" as H

Rectangle {
    id: s
    property QtObject couch
    readonly property real k: couch.k
    property var options: []          // labels
    property int current: 0
    property color onColor: H.P.ac
    property real fontSize: 15
    signal picked(int index)
    height: row.implicitHeight + 8 * k
    width: row.implicitWidth + 8 * k
    radius: 12 * k
    color: "#0fffffff"
    Row {
        id: row
        x: 4 * s.k; y: 4 * s.k
        Repeater {
            model: s.options
            delegate: Rectangle {
                required property var modelData
                required property int index
                width: lab.implicitWidth + 32 * s.k; height: lab.implicitHeight + 18 * s.k
                radius: 9 * s.k
                color: index === s.current ? s.onColor : "transparent"
                T { id: lab; couchRef: s.couch; anchors.centerIn: parent; text: modelData; color: index === s.current ? H.P.ink : H.P.mut; font.pixelSize: s.fontSize * s.k; font.weight: Font.DemiBold }
                MouseArea { anchors.fill: parent; onClicked: s.picked(index) }
            }
        }
    }
}

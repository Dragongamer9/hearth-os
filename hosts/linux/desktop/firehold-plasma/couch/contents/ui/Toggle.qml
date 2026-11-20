// The mockups' .tog: a 52 × 30 switch, the accent colour when on.
import QtQuick
import "fhp.js" as H

Rectangle {
    id: t
    property real k: 1
    property bool on: false
    signal toggled()
    width: 52 * k; height: 30 * k; radius: height / 2
    color: on ? H.P.ac : "#3a3d47"
    Rectangle { x: (t.on ? 26 : 4) * t.k; y: 4 * t.k; width: 22 * t.k; height: width; radius: width / 2; color: t.on ? "#ffffff" : "#dddddd" }
    MouseArea { anchors.fill: parent; onClicked: t.toggled() }
}

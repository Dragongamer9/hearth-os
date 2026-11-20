// A row's title (the mockups' .h2, 24 px bold) with its note on the right ("RB for more", "from every store").
import QtQuick
import "fhp.js" as H

Item {
    id: rh
    property QtObject couch
    property string title: ""
    property string note: ""
    height: t.implicitHeight
    T { id: t; couchRef: rh.couch; text: rh.title; font.pixelSize: 24 * rh.couch.k; font.weight: Font.Bold; width: Math.min(implicitWidth, rh.width * 0.7) }
    T { couchRef: rh.couch; anchors.right: parent.right; anchors.verticalCenter: t.verticalCenter; text: rh.note; color: H.P.mut; font.pixelSize: 16 * rh.couch.k; visible: text !== "" }
}

// A hint at the bottom (the mockups' .hint): the button's round badge in its colour and what it does.
import QtQuick
import "fhp.js" as H

Row {
    id: h
    property QtObject couch
    property string key: "A"
    property string label: ""
    property color tint: H.keyColor(key)
    spacing: 10 * couch.k
    Rectangle {
        anchors.verticalCenter: parent.verticalCenter
        height: 32 * h.couch.k
        width: Math.max(height, keyText.implicitWidth + 16 * h.couch.k)
        radius: height / 2
        color: h.tint
        T {
            id: keyText
            couchRef: h.couch
            anchors.centerIn: parent
            text: h.key
            color: "#111111"
            font.pixelSize: 14 * h.couch.k
            font.weight: Font.ExtraBold
        }
    }
    T {
        couchRef: h.couch
        anchors.verticalCenter: parent.verticalCenter
        text: h.label
        font.pixelSize: 19 * h.couch.k
        font.weight: Font.Medium
    }
}

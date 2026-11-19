// A hint at the bottom: the button's round badge and what it does (A Play, X Options...).
import QtQuick

Row {
    id: h
    property QtObject couch
    property string key: "A"
    property string label: ""
    property color tint: "#8fe0a8"
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
            color: "#1a110c"
            font.pixelSize: 15 * h.couch.k
            font.weight: Font.Bold
        }
    }
    T {
        couchRef: h.couch
        anchors.verticalCenter: parent.verticalCenter
        text: h.label
        font.pixelSize: 19 * h.couch.k
    }
}

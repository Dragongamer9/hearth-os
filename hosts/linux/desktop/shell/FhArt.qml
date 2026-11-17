// A game's picture with rounded corners (Firehold's tiles); nothing is drawn until it has loaded.
import QtQuick
import QtQuick.Effects

Item {
    id: a
    property string source: ""
    property real radius: 14
    readonly property bool ready: img.status === Image.Ready && source !== ""
    Image {
        id: img
        anchors.fill: parent
        source: a.source
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        cache: true
        visible: false
        sourceSize.width: Math.max(64, Math.round(a.width))
    }
    Rectangle { id: mask; anchors.fill: parent; radius: a.radius; visible: false; layer.enabled: true }
    MultiEffect {
        anchors.fill: parent
        visible: a.ready
        source: img
        maskEnabled: true
        maskSource: mask
        maskThresholdMin: 0.5
        maskSpreadAtMin: 1.0
    }
}

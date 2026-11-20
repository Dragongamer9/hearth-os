// Rounds what's inside it (a game's art on a tile or the hero): Qt's own mask effect, drawn once into a layer.
import QtQuick
import QtQuick.Effects

Item {
    id: rc
    property real radius: 14
    default property alias content: inner.data
    Item {
        id: inner
        anchors.fill: parent
        layer.enabled: true
        layer.effect: MultiEffect {
            maskEnabled: true
            maskSource: mask
            maskThresholdMin: 0.5
            maskSpreadAtMin: 1.0
        }
    }
    Rectangle {
        id: mask
        anchors.fill: parent
        radius: rc.radius
        visible: false
        layer.enabled: true
        antialiasing: true
    }
}

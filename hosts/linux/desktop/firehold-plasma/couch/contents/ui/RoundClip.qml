// Rounds what's inside it (a game's art on a tile or the hero): Qt's own mask effect, drawn once into a layer.
// (1.16: Qt's software drawing, no graphics card, safe graphics or CI, can't run the effect and would draw nothing:
// there the art is shown square-cornered.)
import QtQuick
import QtQuick.Effects

Item {
    id: rc
    property real radius: 14
    default property alias content: inner.data
    readonly property bool soft: GraphicsInfo.api === GraphicsInfo.Software
    Item {
        id: inner
        anchors.fill: parent
        clip: rc.soft          // (without the effect: at least nothing spills over the edges)
        layer.enabled: !rc.soft
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
        layer.enabled: !rc.soft
        antialiasing: true
    }
}

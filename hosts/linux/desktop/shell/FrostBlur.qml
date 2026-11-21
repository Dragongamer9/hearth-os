// Glass's frost (Frost.qml) cut to its island's round corners, on the graphics card. Its own file so a system without
// QtQuick.Effects (or drawing with the processor) only keeps the frost square under the round milky fill.
import QtQuick
import QtQuick.Effects

MultiEffect {
    maskEnabled: true
    maskSource: shape
    maskThresholdMin: 0.5
    maskSpreadAtMin: 1.0
    Rectangle {
        id: shape
        anchors.fill: parent
        visible: false
        layer.enabled: true
        antialiasing: true
        radius: Math.min(height / 2, Theme.r(18))
    }
}

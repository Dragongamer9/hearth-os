// A game's picture with rounded corners (Firehold's tiles). Nothing at all is made until there's a picture to show (no
// effect, no layer: a tile without art is only its colour), and the picture is drawn once it has loaded.
import QtQuick
import QtQuick.Effects
import QtQuick.Window

Item {
    id: a
    property string source: ""
    property real radius: 14
    // A picture that's taken again and again (Everything open's windows): read at once and not kept, so the card
    // never blinks empty between two pictures.
    property bool live: false
    readonly property bool ready: art.item !== null && art.item.ready
    Loader {
        id: art
        anchors.fill: parent
        active: a.source !== ""
        sourceComponent: Item {
            readonly property bool ready: img.status === Image.Ready
            Image {
                id: img
                anchors.fill: parent
                source: a.source
                fillMode: Image.PreserveAspectCrop
                asynchronous: !a.live
                cache: !a.live
                visible: false
                // Decoded at the screen's own pixels (2.1 on the Ally), filling the tile: never upscaled soft.
                sourceSize: Qt.size(Math.max(64, Math.ceil(a.width * Screen.devicePixelRatio)), Math.max(64, Math.ceil(a.height * Screen.devicePixelRatio)))
                smooth: true; mipmap: true
            }
            Rectangle { id: mask; anchors.fill: parent; radius: a.radius; visible: false; layer.enabled: true }
            MultiEffect {
                anchors.fill: parent
                visible: img.status === Image.Ready
                source: img
                maskEnabled: true
                maskSource: mask
                maskThresholdMin: 0.5
                maskSpreadAtMin: 1.0
            }
        }
    }
}

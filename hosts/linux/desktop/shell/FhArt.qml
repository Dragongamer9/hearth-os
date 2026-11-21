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
    // 1.16 (Firehold's memory: 475 MB with the image's game pictures): a picture is only held while it can be seen:
    // its tab showing and its window up. Off-screen tabs and a hidden Couch let theirs go; Qt keeps the last few in its
    // own small cache, so coming back is quick. Each is decoded at the tile's size (sourceSize below), never bigger.
    readonly property bool seen: a.visible && a.width > 0 && a.height > 0 && a.Window.visibility !== Window.Hidden
    Loader {
        id: art
        anchors.fill: parent
        active: a.source !== "" && a.seen
        sourceComponent: Item {
            readonly property bool ready: img.status === Image.Ready
            Image {
                id: img
                anchors.fill: parent
                source: a.source
                fillMode: Image.PreserveAspectCrop
                asynchronous: !a.live
                cache: !a.live
                visible: parent.soft      // drawn by MultiEffect below, with the GPU
                // Decoded at the screen's own pixels (2.1 on the Ally), filling the tile: never upscaled soft.
                sourceSize: Qt.size(Math.max(64, Math.ceil(a.width * Screen.devicePixelRatio)), Math.max(64, Math.ceil(a.height * Screen.devicePixelRatio)))
                smooth: true; mipmap: true
            }
            // (1.16: the mask's own picture only where it's used, with a graphics card; without one every tile kept an
            // unused full-size copy, part of Firehold's 480 MB on the Fedora image)
            Rectangle { id: mask; anchors.fill: parent; radius: a.radius; visible: false; layer.enabled: !parent.soft }
            // Qt's software renderer (no GPU: CI, safe graphics) can't run MultiEffect's shader: the picture itself, square-cornered.
            readonly property bool soft: GraphicsInfo.api === GraphicsInfo.Software
            MultiEffect {
                anchors.fill: parent
                visible: !parent.soft && img.status === Image.Ready
                source: img
                maskEnabled: true
                maskSource: mask
                maskThresholdMin: 0.5
                maskSpreadAtMin: 1.0
            }
        }
    }
}

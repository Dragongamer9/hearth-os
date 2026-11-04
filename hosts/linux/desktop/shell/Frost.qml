// Glass's faux frost (Mantel 2026): labwc has no background blur for layer surfaces (no ext-background-effect-v1),
// so an island draws its own copy of the wallpaper, lined up with the real one, and blurs it (FrostBlur.qml,
// QtQuick.Effects, only with a GPU; with software rendering the copy is shown as it is, under the milky fill).
// Over windows it is simply a translucent panel.
import QtQuick
import QtQuick.Window

Item {
    id: fr
    property real rad: 18
    clip: true
    readonly property bool gpu: GraphicsInfo.api !== GraphicsInfo.Software && GraphicsInfo.api !== GraphicsInfo.Unknown
    property point at: Qt.point(0, 0)
    function place() { const p = fr.mapToItem(null, 0, 0); at = Qt.point(p.x, p.y); }
    onWidthChanged: Qt.callLater(place)
    onHeightChanged: Qt.callLater(place)
    Component.onCompleted: Qt.callLater(place)
    Timer { interval: 1500; running: fr.visible; repeat: true; onTriggered: fr.place() }

    readonly property string wall: Theme.wall
    readonly property string scene: wall.startsWith("anim:") ? wall.slice(5) : ""
    Item {
        id: copy
        x: -fr.at.x; y: -fr.at.y
        width: fr.Window.window ? fr.Window.window.width : 1920
        height: fr.Window.window ? fr.Window.window.height : 1080
        visible: !blur.active || blur.status !== Loader.Ready
        Rectangle { anchors.fill: parent; color: Theme.bg }
        Loader {
            anchors.fill: parent
            active: ["neon", "zen", "glass", "synth"].indexOf(fr.scene) >= 0
            source: "Scene.qml"
            onLoaded: item.name = Qt.binding(() => fr.scene)
        }
        Image {
            anchors.fill: parent
            visible: status === Image.Ready
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            sourceSize.width: 960
            source: fr.wall.startsWith("builtin:") ? Qt.resolvedUrl("images/wallpapers/" + fr.wall.slice(8) + ".jpg")
                : fr.wall.startsWith("/") ? "file://" + fr.wall.split("/").map(encodeURIComponent).join("/")
                : ["lava", "aurora", "tide", "ridges", "motes"].indexOf(fr.scene) >= 0 ? Qt.resolvedUrl("images/wall-" + fr.scene + ".jpg") : ""
        }
    }
    Loader {
        id: blur
        active: fr.gpu
        anchors.fill: copy
        source: "FrostBlur.qml"
        onLoaded: item.source = copy
    }
}

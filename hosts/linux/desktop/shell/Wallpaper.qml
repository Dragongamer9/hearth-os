// Desktop background: ~/.config/nimbus/wallpaper.jpg (or .png) if you have one, otherwise the Hearth
// fireside: warm darkness lit from below by a fire that slowly breathes, with the odd ember drifting up.
// The motion is slow and cheap (a few animated layers, a handful of particles) and stops when the
// background isn't visible. Sits on the background layer under every window.
import QtQuick
import QtQuick.Particles
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: root
    WlrLayershell.layer: WlrLayer.Background
    WlrLayershell.namespace: "hearth-wallpaper"
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: Theme.bg

    readonly property string dir: Quickshell.env("HOME") + "/.config/nimbus/"
    readonly property bool custom: jpg.status === Image.Ready || png.status === Image.Ready

    Item {
        id: fireside
        anchors.fill: parent
        visible: !root.custom

        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop { position: 0.0; color: "#0b0807" }
                GradientStop { position: 0.6; color: Theme.bg2 }
                GradientStop { position: 1.0; color: "#1f140e" }
            }
        }

        // The fire's light: a wide glow rising from the bottom, breathing slowly...
        Image {
            id: glow
            source: Qt.resolvedUrl("ember-glow.png")
            width: parent.width * 1.5
            height: parent.height * 1.1
            x: (parent.width - width) / 2
            y: parent.height - height * 0.45
            smooth: true
            opacity: 0.32
            SequentialAnimation on opacity {
                loops: Animation.Infinite
                NumberAnimation { to: 0.42; duration: 3600; easing.type: Easing.InOutSine }
                NumberAnimation { to: 0.30; duration: 4200; easing.type: Easing.InOutSine }
            }
        }
        // ...with a smaller, hotter core that flickers and drifts a little.
        Image {
            source: Qt.resolvedUrl("ember-glow.png")
            width: parent.width * 0.7
            height: parent.height * 0.6
            x: (parent.width - width) / 2 + drift
            y: parent.height - height * 0.42
            smooth: true
            opacity: 0.22
            property real drift: 0
            SequentialAnimation on opacity {
                loops: Animation.Infinite
                NumberAnimation { to: 0.30; duration: 700; easing.type: Easing.InOutQuad }
                NumberAnimation { to: 0.20; duration: 1100; easing.type: Easing.InOutQuad }
                NumberAnimation { to: 0.27; duration: 500; easing.type: Easing.InOutQuad }
                NumberAnimation { to: 0.18; duration: 1500; easing.type: Easing.InOutQuad }
            }
            SequentialAnimation on drift {
                loops: Animation.Infinite
                NumberAnimation { to: root.width * 0.02; duration: 5200; easing.type: Easing.InOutSine }
                NumberAnimation { to: -root.width * 0.02; duration: 6100; easing.type: Easing.InOutSine }
            }
        }

        // Embers: a few sparks float up from the fire and fade.
        ParticleSystem { id: sparks; running: fireside.visible }
        Emitter {
            system: sparks
            anchors { bottom: parent.bottom; horizontalCenter: parent.horizontalCenter }
            width: parent.width * 0.5
            height: 20
            emitRate: 2.5
            lifeSpan: 9000
            lifeSpanVariation: 3000
            size: Math.max(3, root.height * 0.004)
            sizeVariation: 2
            velocity: AngleDirection { angle: 270; angleVariation: 14; magnitude: root.height * 0.06; magnitudeVariation: root.height * 0.03 }
            acceleration: AngleDirection { angle: 270; magnitude: 2 }
        }
        Wander { system: sparks; xVariance: 40; pace: 30 }
        ItemParticle {
            system: sparks
            fade: true
            delegate: Rectangle {
                width: 4; height: 4; radius: 2
                color: Math.random() > 0.5 ? Theme.ember : Theme.flame
                opacity: 0.55
            }
        }
    }

    Image {
        id: jpg
        anchors.fill: parent
        source: "file://" + root.dir + "wallpaper.jpg"
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: status === Image.Ready
    }
    Image {
        id: png
        anchors.fill: parent
        source: jpg.status === Image.Error ? "file://" + root.dir + "wallpaper.png" : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: status === Image.Ready
    }
}

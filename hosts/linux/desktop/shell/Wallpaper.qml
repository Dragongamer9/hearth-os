// Desktop background, picked in Hearth Settings › Appearance (Settings.wallpaper): a picture, or one of the
// moving ones: the Hearth fireside (the default: warm darkness lit from below by a fire that slowly breathes,
// with the odd ember drifting up), Lava (slow glowing blobs) or Aurora (warm light ribbons under the stars).
// Older setups' ~/.config/nimbus/wallpaper.jpg/.png still work.
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
    // Hearth Settings › Appearance picks a picture (Settings.wallpaper); older setups used wallpaper.jpg/.png.
    readonly property string anim: Settings.wallpaper.startsWith("anim:") ? Settings.wallpaper.slice(5) : ""
    readonly property string picture: anim ? "" : Settings.wallpaper
    readonly property bool custom: !anim && (chosen.status === Image.Ready || (!picture && (jpg.status === Image.Ready || png.status === Image.Ready)))
    readonly property bool moving: !Settings.reduceMotion

    // ---------------------------------------------------------------- Lava: big soft blobs drifting slowly
    Item {
        id: lava
        anchors.fill: parent
        visible: root.anim === "lava"
        Rectangle { anchors.fill: parent; color: "#120a09" }
        Repeater {
            model: [
                { src: "images/blob-red.png", x: 0.22, y: 0.62, s: 0.62, dx: 0.10, dy: 0.08, t: 23000 },
                { src: "images/blob-orange.png", x: 0.52, y: 0.34, s: 0.55, dx: -0.12, dy: 0.10, t: 29000 },
                { src: "images/blob-gold.png", x: 0.74, y: 0.66, s: 0.58, dx: -0.08, dy: -0.12, t: 31000 },
                { src: "images/blob-plum.png", x: 0.80, y: 0.22, s: 0.40, dx: -0.10, dy: 0.06, t: 26000 },
                { src: "images/blob-orange.png", x: 0.30, y: 0.20, s: 0.36, dx: 0.12, dy: 0.10, t: 34000 },
                { src: "images/blob-red.png", x: 0.58, y: 0.90, s: 0.44, dx: 0.06, dy: -0.12, t: 27000 }
            ]
            delegate: Image {
                id: b
                required property var modelData
                source: Qt.resolvedUrl(modelData.src)
                width: root.height * modelData.s * 1.6 * scaleK
                height: width
                property real scaleK: 1
                property real k: 0
                x: root.width * (modelData.x + modelData.dx * k) - width / 2
                y: root.height * (modelData.y + modelData.dy * k) - height / 2
                opacity: 0.85
                smooth: true
                SequentialAnimation on k {
                    running: lava.visible && root.moving; loops: Animation.Infinite
                    NumberAnimation { to: 1; duration: b.modelData.t; easing.type: Easing.InOutSine }
                    NumberAnimation { to: -1; duration: b.modelData.t * 1.3; easing.type: Easing.InOutSine }
                    NumberAnimation { to: 0; duration: b.modelData.t * 0.7; easing.type: Easing.InOutSine }
                }
                SequentialAnimation on scaleK {
                    running: lava.visible && root.moving; loops: Animation.Infinite
                    NumberAnimation { to: 1.18; duration: b.modelData.t * 0.8; easing.type: Easing.InOutSine }
                    NumberAnimation { to: 0.9; duration: b.modelData.t; easing.type: Easing.InOutSine }
                }
            }
        }
    }

    // ---------------------------------------------------------------- Aurora: warm ribbons, twinkling stars
    Item {
        id: aurora
        anchors.fill: parent
        visible: root.anim === "aurora"
        Rectangle {
            anchors.fill: parent
            gradient: Gradient { GradientStop { position: 0; color: "#0a0609" } GradientStop { position: 1; color: "#1e0f0d" } }
        }
        Repeater {
            model: 90
            delegate: Rectangle {
                id: star
                required property int index
                readonly property real rx: (index * 0.6180339) % 1
                readonly property real ry: (index * 0.3819660 * 1.7) % 1
                x: rx * root.width; y: ry * root.height * 0.85
                width: 2 + (index % 3); height: width; radius: width / 2
                color: "#fff4e0"
                opacity: 0.3 + (index % 5) * 0.12
                SequentialAnimation on opacity {
                    running: aurora.visible && root.moving; loops: Animation.Infinite
                    PauseAnimation { duration: (star.index * 397) % 4000 }
                    NumberAnimation { to: 0.15; duration: 1800 + (star.index % 7) * 300; easing.type: Easing.InOutSine }
                    NumberAnimation { to: 0.9; duration: 1600 + (star.index % 5) * 400; easing.type: Easing.InOutSine }
                }
            }
        }
        Repeater {
            model: [{ src: "images/aurora-a.png", y: 0.20, t: 40000, o: 0.75 }, { src: "images/aurora-b.png", y: 0.30, t: 52000, o: 0.55 }, { src: "images/aurora-c.png", y: 0.38, t: 46000, o: 0.6 }]
            delegate: Image {
                id: rib
                required property var modelData
                source: Qt.resolvedUrl(modelData.src)
                width: root.width * 1.6
                height: root.height * 0.45
                y: root.height * modelData.y
                property real k: 0
                x: -root.width * 0.3 + root.width * 0.15 * k
                opacity: modelData.o
                smooth: true
                SequentialAnimation on k {
                    running: aurora.visible && root.moving; loops: Animation.Infinite
                    NumberAnimation { to: 1; duration: rib.modelData.t; easing.type: Easing.InOutSine }
                    NumberAnimation { to: -1; duration: rib.modelData.t; easing.type: Easing.InOutSine }
                }
                SequentialAnimation on opacity {
                    running: aurora.visible && root.moving; loops: Animation.Infinite
                    NumberAnimation { to: rib.modelData.o * 0.5; duration: rib.modelData.t / 3; easing.type: Easing.InOutSine }
                    NumberAnimation { to: rib.modelData.o; duration: rib.modelData.t / 3; easing.type: Easing.InOutSine }
                }
            }
        }
        Rectangle {          // a dark ridge along the bottom
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
            height: root.height * 0.12
            gradient: Gradient { GradientStop { position: 0; color: "transparent" } GradientStop { position: 0.4; color: "#0c0607" } }
        }
    }

    Item {
        id: fireside
        anchors.fill: parent
        visible: !root.custom && !root.anim

        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop { position: 0.0; color: "#0b0807" }
                GradientStop { position: 0.6; color: Theme.bg2 }
                GradientStop { position: 1.0; color: "#1f140e" }
            }
        }

        // The fire's light: a wide glow rising from the bottom, breathing slowly (still with Reduce motion)...
        Image {
            id: glow
            source: Qt.resolvedUrl("images/ember-glow.png")
            width: parent.width * 1.5
            height: parent.height * 1.1
            x: (parent.width - width) / 2
            y: parent.height - height * 0.45
            smooth: true
            opacity: 0.32
            SequentialAnimation on opacity {
                running: !Settings.reduceMotion
                loops: Animation.Infinite
                NumberAnimation { to: 0.42; duration: 3600; easing.type: Easing.InOutSine }
                NumberAnimation { to: 0.30; duration: 4200; easing.type: Easing.InOutSine }
            }
        }
        // ...with a smaller, hotter core that flickers and drifts a little.
        Image {
            source: Qt.resolvedUrl("images/ember-glow.png")
            width: parent.width * 0.7
            height: parent.height * 0.6
            x: (parent.width - width) / 2 + drift
            y: parent.height - height * 0.42
            smooth: true
            opacity: 0.22
            property real drift: 0
            SequentialAnimation on opacity {
                running: !Settings.reduceMotion
                loops: Animation.Infinite
                NumberAnimation { to: 0.30; duration: 700; easing.type: Easing.InOutQuad }
                NumberAnimation { to: 0.20; duration: 1100; easing.type: Easing.InOutQuad }
                NumberAnimation { to: 0.27; duration: 500; easing.type: Easing.InOutQuad }
                NumberAnimation { to: 0.18; duration: 1500; easing.type: Easing.InOutQuad }
            }
            SequentialAnimation on drift {
                running: !Settings.reduceMotion
                loops: Animation.Infinite
                NumberAnimation { to: root.width * 0.02; duration: 5200; easing.type: Easing.InOutSine }
                NumberAnimation { to: -root.width * 0.02; duration: 6100; easing.type: Easing.InOutSine }
            }
        }

        // Embers: a few sparks float up from the fire and fade.
        ParticleSystem { id: sparks; running: fireside.visible && !Settings.reduceMotion }
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
        id: chosen
        anchors.fill: parent
        // Encoded: a name with # or % in it is otherwise read as part of the address, and nothing shows.
        source: root.picture ? "file://" + root.picture.split("/").map(encodeURIComponent).join("/") : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: status === Image.Ready
    }
    Image {
        id: jpg
        anchors.fill: parent
        source: Settings.wallpaper ? "" : "file://" + root.dir + "wallpaper.jpg"
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: status === Image.Ready
    }
    Image {
        id: png
        anchors.fill: parent
        source: !Settings.wallpaper && jpg.status === Image.Error ? "file://" + root.dir + "wallpaper.png" : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: status === Image.Ready
    }
}

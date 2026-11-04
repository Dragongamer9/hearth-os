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
import Quickshell.Services.UPower

PanelWindow {
    id: root
    WlrLayershell.layer: WlrLayer.Background
    WlrLayershell.namespace: "hearth-wallpaper"
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: Theme.bg

    property bool primary: false          // the main screen: the desktop clock (Mantel 2026) goes here
    readonly property string dir: Quickshell.env("HOME") + "/.config/nimbus/"
    // Hearth Settings › Appearance picks a picture (Settings.wallpaper); older setups used wallpaper.jpg/.png.
    // "" = the look's own (Theme.qml / theme/palettes.json: Hearth's fireside, Steam-style's Slate Tide).
    readonly property string wall: Settings.wallpaper !== "" ? Settings.wallpaper : (Theme.lookDef.wallpaper || "")
    readonly property string anim: wall.startsWith("anim:") ? wall.slice(5) : ""
    readonly property string picture: anim ? "" : wall
    readonly property bool custom: !anim && (chosen.status === Image.Ready || (!picture && (jpg.status === Image.Ready || png.status === Image.Ready)))
    // Moving wallpapers stay cheap: they hold still with Reduce motion, behind a fullscreen app or game, on battery,
    // while the screen is locked or off. The Slate ones also need a GPU (still with software rendering).
    readonly property var front: ToplevelManager.activeToplevel
    readonly property bool onBattery: UPower.onBattery === true
    readonly property bool moving: !Settings.reduceMotion && !(front && front.fullscreen === true) && !onBattery && !DeskState.locked && !DeskState.screenOff
    readonly property bool gpu: gpuProbe.gpu
    Item { id: gpuProbe; readonly property bool gpu: GraphicsInfo.api !== GraphicsInfo.Software && GraphicsInfo.api !== GraphicsInfo.Unknown }
    readonly property bool slateMoving: moving && gpu

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

    // ---------------------------------------------------------------- Slate (the Steam-style look's moving ones)
    // Tide: soft blue light drifting slowly over blue-slate (the Steam-style look's default).
    Item {
        id: tide
        anchors.fill: parent
        visible: root.anim === "tide"
        Rectangle { anchors.fill: parent; gradient: Gradient { GradientStop { position: 0; color: "#0e131b" } GradientStop { position: 1; color: "#1b2838" } } }
        Repeater {
            model: [
                { src: "images/blob-blue.png", x: 0.22, y: 0.64, s: 0.62, dx: 0.10, dy: 0.06, t: 31000, o: 0.55 },
                { src: "images/blob-slate.png", x: 0.52, y: 0.32, s: 0.70, dx: -0.12, dy: 0.08, t: 37000, o: 0.7 },
                { src: "images/blob-sky.png", x: 0.76, y: 0.62, s: 0.46, dx: -0.08, dy: -0.10, t: 41000, o: 0.35 },
                { src: "images/blob-blue.png", x: 0.84, y: 0.20, s: 0.40, dx: -0.10, dy: 0.05, t: 34000, o: 0.45 }
            ]
            delegate: Image {
                id: tb
                required property var modelData
                source: Qt.resolvedUrl(modelData.src)
                width: root.height * modelData.s * 1.7; height: width
                property real k: 0
                x: root.width * (modelData.x + modelData.dx * k) - width / 2
                y: root.height * (modelData.y + modelData.dy * k) - height / 2
                opacity: modelData.o
                smooth: true
                SequentialAnimation on k {
                    running: tide.visible && root.slateMoving; loops: Animation.Infinite
                    NumberAnimation { to: 1; duration: tb.modelData.t; easing.type: Easing.InOutSine }
                    NumberAnimation { to: -1; duration: tb.modelData.t * 1.3; easing.type: Easing.InOutSine }
                    NumberAnimation { to: 0; duration: tb.modelData.t * 0.7; easing.type: Easing.InOutSine }
                }
            }
        }
        Rectangle { anchors.fill: parent; gradient: Gradient { GradientStop { position: 0.55; color: "transparent" } GradientStop { position: 1; color: "#80090c12" } } }
    }
    // Ridges: layered hills drifting past each other at different speeds, a cool light breathing behind them.
    Item {
        id: ridgesWall
        anchors.fill: parent
        visible: root.anim === "ridges"
        Rectangle { anchors.fill: parent; gradient: Gradient { GradientStop { position: 0; color: "#0b1018" } GradientStop { position: 1; color: "#2a475e" } } }
        Image {
            source: Qt.resolvedUrl("images/blob-sky.png")
            width: root.width * 0.9; height: root.height * 0.5
            x: root.width * 0.3; y: root.height * 0.18
            opacity: 0.22
            SequentialAnimation on opacity {
                running: ridgesWall.visible && root.slateMoving; loops: Animation.Infinite
                NumberAnimation { to: 0.32; duration: 9000; easing.type: Easing.InOutSine }
                NumberAnimation { to: 0.18; duration: 11000; easing.type: Easing.InOutSine }
            }
        }
        Repeater {
            model: [{ src: "images/ridge-a.png", y: 0.30, t: 240000 }, { src: "images/ridge-b.png", y: 0.40, t: 160000 }, { src: "images/ridge-c.png", y: 0.50, t: 100000 }]
            delegate: Item {
                id: layer
                required property var modelData
                width: root.width; height: root.height * 0.75
                y: root.height * modelData.y
                property real k: 0
                // Two copies side by side; the layer slides by one width and starts over (the picture tiles).
                Row {
                    x: -layer.k * root.width
                    Image { source: Qt.resolvedUrl(layer.modelData.src); width: root.width; height: layer.height; smooth: true }
                    Image { source: Qt.resolvedUrl(layer.modelData.src); width: root.width; height: layer.height; smooth: true }
                }
                NumberAnimation on k {
                    running: ridgesWall.visible && root.slateMoving; loops: Animation.Infinite
                    from: 0; to: 1; duration: layer.modelData.t
                }
            }
        }
    }
    // Motes: a few points of cool light rising slowly through the dark.
    Item {
        id: motes
        anchors.fill: parent
        visible: root.anim === "motes"
        Rectangle { anchors.fill: parent; gradient: Gradient { GradientStop { position: 0; color: "#0b0f15" } GradientStop { position: 1; color: "#1b2838" } } }
        Image {
            source: Qt.resolvedUrl("images/blob-slate.png")
            width: root.width * 1.4; height: root.height * 0.9
            x: -root.width * 0.2; y: root.height * 0.55
            opacity: 0.55
        }
        ParticleSystem { id: moteSys; running: motes.visible && root.slateMoving }
        Emitter {
            system: moteSys
            anchors.fill: parent
            emitRate: 3
            lifeSpan: 16000
            lifeSpanVariation: 5000
            size: Math.max(3, root.height * 0.004)
            sizeVariation: 3
            velocity: AngleDirection { angle: 275; angleVariation: 20; magnitude: root.height * 0.012; magnitudeVariation: root.height * 0.008 }
        }
        Wander { system: moteSys; xVariance: 30; pace: 12 }
        ItemParticle {
            system: moteSys
            fade: true
            delegate: Rectangle { width: 4; height: 4; radius: 2; color: Math.random() > 0.6 ? "#c7e6fa" : "#66c0f4"; opacity: 0.6 }
        }
        // Still (no motion, battery, software rendering): a fixed scatter of them instead.
        Repeater {
            model: motes.visible && !root.slateMoving ? 60 : 0
            delegate: Rectangle {
                required property int index
                x: ((index * 0.6180339) % 1) * root.width; y: ((index * 0.7548776) % 1) * root.height
                width: 2 + index % 3; height: width; radius: width / 2
                color: "#66c0f4"; opacity: 0.25 + (index % 5) * 0.1
            }
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
                running: fireside.visible && root.moving
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
                running: fireside.visible && root.moving
                loops: Animation.Infinite
                NumberAnimation { to: 0.30; duration: 700; easing.type: Easing.InOutQuad }
                NumberAnimation { to: 0.20; duration: 1100; easing.type: Easing.InOutQuad }
                NumberAnimation { to: 0.27; duration: 500; easing.type: Easing.InOutQuad }
                NumberAnimation { to: 0.18; duration: 1500; easing.type: Easing.InOutQuad }
            }
            SequentialAnimation on drift {
                running: fireside.visible && root.moving
                loops: Animation.Infinite
                NumberAnimation { to: root.width * 0.02; duration: 5200; easing.type: Easing.InOutSine }
                NumberAnimation { to: -root.width * 0.02; duration: 6100; easing.type: Easing.InOutSine }
            }
        }

        // Embers: a few sparks float up from the fire and fade.
        ParticleSystem { id: sparks; running: fireside.visible && root.moving }
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
        source: root.picture.startsWith("builtin:") ? Qt.resolvedUrl("images/wallpapers/" + root.picture.slice(8) + ".jpg")
            : root.picture ? "file://" + root.picture.split("/").map(encodeURIComponent).join("/") : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: status === Image.Ready
    }
    Image {
        id: jpg
        anchors.fill: parent
        source: root.wall ? "" : "file://" + root.dir + "wallpaper.jpg"
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: status === Image.Ready
    }
    Image {
        id: png
        anchors.fill: parent
        source: !root.wall && jpg.status === Image.Error ? "file://" + root.dir + "wallpaper.png" : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: status === Image.Ready
    }

    // ---------------------------------------------------------------- Mantel 2026
    // The new looks' own wallpapers (Scene.qml: anim:neon, anim:zen, anim:glass, anim:synth), loaded on their own so
    // a problem there only loses the picture (the look's background colour stays).
    Loader {
        anchors.fill: parent
        active: ["neon", "zen", "glass", "synth"].indexOf(root.anim) >= 0
        source: "Scene.qml"
        onLoaded: item.name = Qt.binding(() => root.anim)
        onStatusChanged: if (status === Loader.Error) console.warn("Wallpaper: couldn't load the look's picture (Scene.qml)")
    }
    // Gaming: a neon glow along the screen's edges (here on the background layer, under every window, so it never
    // takes a click).
    Item {
        anchors.fill: parent
        visible: Theme.edgeGlow
        Repeater {
            model: ["top", "bottom", "left", "right"]
            delegate: Rectangle {
                id: edge
                required property string modelData
                readonly property bool far: modelData === "bottom" || modelData === "right"
                readonly property bool horiz: modelData === "top" || modelData === "bottom"
                width: horiz ? root.width : 90
                height: horiz ? 90 : root.height
                x: modelData === "right" ? root.width - width : 0
                y: modelData === "bottom" ? root.height - height : 0
                gradient: Gradient {
                    orientation: edge.horiz ? Gradient.Vertical : Gradient.Horizontal
                    GradientStop { position: 0; color: edge.far ? "transparent" : Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) }
                    GradientStop { position: 1; color: edge.far ? Qt.rgba(Theme.flame.r, Theme.flame.g, Theme.flame.b, 0.16) : "transparent" }
                }
            }
        }
        Rectangle { anchors.fill: parent; color: "transparent"; border.width: 2; border.color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.55) }
    }
    // The desktop clock and weather (Settings › Appearance; on by default in some looks), main screen only.
    Loader {
        anchors.fill: parent
        active: root.primary && Theme.deskClock
        source: "DeskClock.qml"
    }
}

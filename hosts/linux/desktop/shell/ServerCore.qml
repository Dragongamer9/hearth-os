// Server mode (docs/SERVER-MODE.md): one AI on this PC drawn as a living "core", the same shape, colour and talking motion
// it has in the Hearth AI Command Center's Live view (src/live.mjs there: gpu = wire globe, comfy = pixel grid, voice =
// sound wave, listen = ripples, hearth = nebula). Draw-only; with Settings › Reduce motion on, it stands still.
import QtQuick

Item {
    id: core
    property string kind: "gpu"
    property string mode: "ready"             // ready | working | speaking | off (not `state`: Item has its own)
    property real size: 96
    width: size; height: size
    readonly property bool talking: mode === "working" || mode === "speaking"
    readonly property bool still: Settings.reduceMotion
    // The Command Center's colours (live.mjs CORES), exactly.
    readonly property var rgb: ({ gpu: [232, 214, 110], comfy: [255, 95, 210], voice: [255, 154, 168], listen: [156, 245, 214], hearth: [255, 138, 76], ollama: [242, 239, 232] })[kind] || [201, 163, 114]
    function a(k) { return Qt.rgba(rgb[0] / 255, rgb[1] / 255, rgb[2] / 255, k); }
    readonly property color c: a(1)
    opacity: mode === "off" ? 0.38 : 1

    // The talking motion, each its own (live.mjs: gpu tHum, comfy tPixel, voice tWobble, listen tSwell, hearth tNebula).
    Item {
        id: body
        anchors.fill: parent
        transformOrigin: Item.Center
        SequentialAnimation on scale {
            running: core.talking && !core.still && (core.kind === "gpu" || core.kind === "listen" || core.kind === "comfy" || core.kind === "hearth")
            loops: Animation.Infinite
            onRunningChanged: if (!running) body.scale = 1
            NumberAnimation { to: core.kind === "gpu" ? 1.04 : core.kind === "listen" ? 1.22 : core.kind === "hearth" ? 1.12 : 1.12; duration: core.kind === "gpu" ? 90 : core.kind === "comfy" ? 125 : 550; easing.type: core.kind === "comfy" ? Easing.Linear : Easing.InOutSine }
            NumberAnimation { to: core.kind === "comfy" ? 0.94 : 1; duration: core.kind === "gpu" ? 90 : core.kind === "comfy" ? 125 : 550; easing.type: Easing.InOutSine }
            NumberAnimation { to: 1; duration: core.kind === "comfy" ? 125 : 0 }
        }
        SequentialAnimation on rotation {
            running: core.talking && !core.still && (core.kind === "voice" || core.kind === "hearth")
            loops: Animation.Infinite
            onRunningChanged: if (!running) body.rotation = 0
            NumberAnimation { to: core.kind === "voice" ? 8 : -10; duration: core.kind === "voice" ? 125 : 550; easing.type: Easing.InOutSine }
            NumberAnimation { to: core.kind === "voice" ? -8 : 0; duration: core.kind === "voice" ? 250 : 550; easing.type: Easing.InOutSine }
            NumberAnimation { to: 0; duration: core.kind === "voice" ? 125 : 0 }
        }

        // Rings sent out while it talks.
        Repeater {
            model: core.talking ? 2 : 0
            delegate: Rectangle {
                required property int index
                anchors.centerIn: parent
                width: core.size; height: width; radius: width / 2
                color: "transparent"; border.width: index === 0 ? 2 : 1; border.color: core.c
                opacity: 0
                SequentialAnimation on scale { running: !core.still; loops: Animation.Infinite
                    PauseAnimation { duration: index * 700 }
                    NumberAnimation { from: 0.7; to: 1.55; duration: 1400; easing.type: Easing.OutQuad }
                    PauseAnimation { duration: 700 - index * 700 } }
                SequentialAnimation on opacity { running: !core.still; loops: Animation.Infinite
                    PauseAnimation { duration: index * 700 }
                    NumberAnimation { from: 0.9; to: 0; duration: 1400 }
                    PauseAnimation { duration: 700 - index * 700 } }
            }
        }

        // ---- gpu: a wireframe globe (rings, two turning)
        Item {
            anchors.fill: parent
            visible: core.kind === "gpu"
            Rectangle { anchors.centerIn: parent; width: parent.width * 0.92; height: width; radius: width / 2; color: "transparent"; border.color: core.a(0.85); border.width: 1 }
            Rectangle { anchors.centerIn: parent; width: parent.width * 0.92; height: parent.height * 0.48; radius: height / 2; color: "transparent"; border.color: core.a(0.5); border.width: 1 }
            Rectangle { anchors.centerIn: parent; width: parent.width * 0.92; height: parent.height * 0.16; radius: height / 2; color: "transparent"; border.color: core.a(0.38); border.width: 1 }
            Repeater {
                model: 2
                delegate: Rectangle {
                    required property int index
                    anchors.centerIn: parent
                    width: core.size * 0.92; height: width; radius: width / 2; color: "transparent"; border.color: core.a(0.7); border.width: 1
                    transform: Scale { id: sy; origin.x: core.size * 0.46; origin.y: core.size * 0.46; xScale: index === 0 ? 0.5 : -0.5
                        SequentialAnimation on xScale { running: !core.still; loops: Animation.Infinite
                            NumberAnimation { from: index === 0 ? 0.5 : -0.5; to: index === 0 ? -1 : 1; duration: 2000 }
                            NumberAnimation { to: index === 0 ? 1 : -1; duration: 4000 }
                            NumberAnimation { to: index === 0 ? 0.5 : -0.5; duration: 2000 } } }
                }
            }
        }
        // ---- comfy: a 4×4 pixel grid that flickers and slowly turns
        Grid {
            anchors.centerIn: parent
            visible: core.kind === "comfy"
            columns: 4; spacing: core.size * 0.04
            RotationAnimation on rotation { running: core.kind === "comfy" && !core.still; from: 0; to: 360; duration: 30000; loops: Animation.Infinite }
            Repeater {
                model: 16
                delegate: Rectangle {
                    required property int index
                    width: core.size * 0.14; height: width
                    color: core.a([0.85, 0.38, 0.7, 0.5][index % 4])
                    SequentialAnimation on opacity { running: core.kind === "comfy" && !core.still; loops: Animation.Infinite
                        NumberAnimation { to: 0.3; duration: 550 + (index * 7 % 17) * 50 }
                        NumberAnimation { to: 1; duration: 550 + (index * 7 % 17) * 50 } }
                }
            }
        }
        // ---- voice: a ring and six moving bars
        Item {
            anchors.fill: parent
            visible: core.kind === "voice"
            Rectangle { anchors.centerIn: parent; width: parent.width * 0.88; height: width; radius: width / 2; color: "transparent"; border.color: core.a(0.38); border.width: 1 }
            Row {
                anchors.centerIn: parent
                spacing: core.size * 0.04
                Repeater {
                    model: [0.5, 0.9, 0.7, 1.0, 0.6, 0.8]
                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        anchors.verticalCenter: parent.verticalCenter
                        width: core.size * 0.05; radius: 2; color: core.c
                        height: core.size * 0.52 * modelData
                        transformOrigin: Item.Center
                        SequentialAnimation on scale { running: core.kind === "voice" && !core.still; loops: Animation.Infinite
                            NumberAnimation { to: 0.2; duration: 250 + index * 40; easing.type: Easing.InOutSine }
                            NumberAnimation { to: 1; duration: 250 + index * 40; easing.type: Easing.InOutSine } }
                    }
                }
            }
        }
        // ---- listen: ripples
        Repeater {
            model: core.kind === "listen" ? 3 : 0
            delegate: Rectangle {
                required property int index
                anchors.centerIn: parent
                width: core.size; height: width; radius: width / 2; color: "transparent"; border.width: 2; border.color: core.a(0.85)
                scale: 0.6
                SequentialAnimation on scale { running: !core.still; loops: Animation.Infinite
                    PauseAnimation { duration: index * 800 }
                    NumberAnimation { from: 0.25; to: 1.15; duration: 2400; easing.type: Easing.OutQuad } }
                opacity: core.still ? 0.6 : Math.max(0, 1 - (scale - 0.25) / 0.9)
            }
        }
        // ---- hearth: a nebula (two soft clouds turning) inside a dotted ring
        Item {
            anchors.fill: parent
            visible: core.kind === "hearth"
            Item {
                anchors.fill: parent
                RotationAnimation on rotation { running: core.kind === "hearth" && !core.still; from: 0; to: 360; duration: 9000; loops: Animation.Infinite }
                Repeater {
                    model: 3
                    delegate: Rectangle {
                        required property int index
                        x: core.size * (0.12 + index * 0.06); y: core.size * (0.26 + index * 0.02)
                        width: core.size * (0.56 - index * 0.1); height: width * 0.8; radius: height / 2
                        color: core.a([0.32, 0.22, 0.4][index])
                    }
                }
                Repeater {
                    model: 3
                    delegate: Rectangle {
                        required property int index
                        x: core.size * (0.38 + index * 0.05); y: core.size * (0.38 + index * 0.03)
                        width: core.size * (0.5 - index * 0.1); height: width * 0.8; radius: height / 2
                        color: core.a([0.18, 0.14, 0.24][index])
                    }
                }
            }
            Rectangle { anchors.centerIn: parent; width: parent.width * 0.8; height: width; radius: width / 2; color: "transparent"; border.color: core.a(0.6); border.width: 1 }
        }
        // ---- anything else: rings that breathe
        Repeater {
            model: ["gpu", "comfy", "voice", "listen", "hearth"].indexOf(core.kind) < 0 ? 3 : 0
            delegate: Rectangle {
                required property int index
                anchors.centerIn: parent
                width: core.size * [0.92, 0.68, 0.44][index]; height: width; radius: width / 2; color: "transparent"; border.width: 1; border.color: core.a([0.5, 0.7, 0.85][index])
            }
        }

        // The iris in the middle (it pulses faster while talking).
        Rectangle {
            id: iris
            anchors.centerIn: parent
            width: core.size * 0.24; height: width; radius: width / 2
            color: "#050302"; border.width: 2; border.color: core.c
            Rectangle { anchors.centerIn: parent; width: parent.width * 1.8; height: width; radius: width / 2; color: core.a(0.12); z: -1 }
            SequentialAnimation on scale { running: !core.still; loops: Animation.Infinite
                NumberAnimation { to: core.talking ? 1.18 : 1.07; duration: core.talking ? 130 : 2000; easing.type: Easing.InOutSine }
                NumberAnimation { to: core.talking ? 0.94 : 1; duration: core.talking ? 160 : 2000; easing.type: Easing.InOutSine }
                NumberAnimation { to: 1; duration: core.talking ? 130 : 0 } }
        }
    }
}

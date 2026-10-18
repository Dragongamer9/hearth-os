// The Hearth menu: press Guide on the controller. Point either stick at an item (or use the D-pad,
// LB/RB, the mouse or touch), A to choose, B or Guide to close. hearth-pad does the stick maths; this
// draws the ring and runs what was picked.
import QtQuick
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: root
    property bool open: false
    property int sel: -1
    property bool mouseOn: true
    signal picked(string action)

    // Clockwise from the top. hearth-pad assumes 8.
    readonly property var items: [
        { action: "apps", label: "Apps", icon: "view-app-grid-symbolic" },
        { action: "keyboard", label: "Keyboard", icon: "input-keyboard-symbolic" },
        { action: "quick", label: "Quick Access", icon: "open-menu-symbolic" },
        { action: "close", label: "Close window", icon: "window-close-symbolic" },
        { action: "game", label: "Game Mode", icon: "input-gaming-symbolic" },
        { action: "screenshot", label: "Screenshot", icon: "camera-photo-symbolic" },
        { action: "mouse", label: root.mouseOn ? "Controller mouse: on" : "Controller mouse: off", icon: "input-mouse-symbolic" },
        { action: "help", label: "Controls", icon: "help-about-symbolic" }
    ]

    function update(m) {
        if (m.open) { open = true; sel = m.sel; return; }
        open = false;
        if (m.pick >= 0 && m.pick < items.length) picked(items[m.pick].action);
    }

    visible: open || closing.running
    Timer { id: closing; interval: Theme.calm + 60 }
    onOpenChanged: if (!open) closing.restart()
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-radial"
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    signal dismissed()
    MouseArea { anchors.fill: parent; onClicked: root.dismissed() }

    Rectangle {
        id: dim
        anchors.fill: parent
        color: "#000000"
        opacity: root.open ? 0.55 : 0
        Behavior on opacity { NumberAnimation { duration: Theme.calm; easing.type: Easing.OutCubic } }
    }

    Item {
        id: ring
        readonly property real size: Math.min(root.width, root.height) * 0.62
        width: size; height: size
        anchors.centerIn: parent
        scale: root.open ? 1 : 0.85
        opacity: root.open ? 1 : 0
        Behavior on scale { NumberAnimation { duration: Theme.calm; easing.type: Easing.OutBack } }
        Behavior on opacity { NumberAnimation { duration: Theme.quick } }

        // Soft disc behind the items, warmed by the fire in the middle.
        Rectangle {
            anchors.fill: parent
            radius: width / 2
            color: Theme.glass
            border.color: Theme.line
            border.width: 1
        }
        Image {
            anchors.centerIn: parent
            width: parent.width * 0.55; height: width
            source: Qt.resolvedUrl("images/ember-glow.png")
            opacity: 0.18
        }

        // Centre: the Hearth mark and the highlighted item's name.
        Column {
            anchors.centerIn: parent
            spacing: ring.size * 0.02
            Image {
                anchors.horizontalCenter: parent.horizontalCenter
                width: ring.size * 0.16; height: width
                source: Qt.resolvedUrl("images/hearth-logo.png")
                smooth: true; mipmap: true
            }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: root.sel >= 0 ? root.items[root.sel].label : "Hearth"
                color: Theme.text
                font.family: Theme.font
                font.pixelSize: Math.max(16, ring.size * 0.05)
                font.weight: Font.DemiBold
            }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "Stick to choose · A select · B close"
                color: Theme.muted
                font.family: Theme.font
                font.pixelSize: Math.max(11, ring.size * 0.028)
            }
        }

        Repeater {
            model: root.items
            delegate: Item {
                id: slot
                required property var modelData
                required property int index
                readonly property real ang: index / root.items.length * 2 * Math.PI
                readonly property real r: ring.size * 0.36
                readonly property bool lit: root.sel === index
                width: ring.size * 0.17; height: width
                x: ring.size / 2 + Math.sin(ang) * r - width / 2
                y: ring.size / 2 - Math.cos(ang) * r - height / 2

                Rectangle {
                    anchors.fill: parent
                    radius: width / 2
                    color: slot.lit ? Theme.accent : (hover.containsMouse ? Theme.panel3 : Theme.panel2)
                    border.color: slot.lit ? Qt.lighter(Theme.accent, 1.2) : Theme.line
                    scale: slot.lit ? 1.14 : 1
                    Behavior on scale { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutCubic } }
                    Behavior on color { ColorAnimation { duration: Theme.quick } }
                }
                Icon {
                    anchors.centerIn: parent
                    size: parent.width * 0.42
                    name: slot.modelData.icon
                    color: slot.lit ? Theme.accentInk : Theme.text
                }
                MouseArea {
                    id: hover
                    anchors.fill: parent
                    hoverEnabled: true
                    onEntered: root.sel = slot.index
                    onClicked: { root.open = false; root.picked(slot.modelData.action); }
                }
            }
        }
    }
}

// The volume bar that shows for a moment when the volume buttons are pressed (Desktop Mode).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: root
    property real volume: 0
    property bool muted: false
    property bool showing: false
    visible: showing || card.opacity > 0
    color: "transparent"
    anchors { top: true }
    margins.top: 28
    implicitWidth: 360
    implicitHeight: 64
    exclusiveZone: 0
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-osd"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    mask: Region {}                      // clicks go through

    function show(v, m) { volume = v; muted = m; showing = true; hide.restart(); }
    Timer { id: hide; interval: 1500; onTriggered: root.showing = false }
    Connections { target: Buttons; function onOsd(v, m) { root.show(v, m); } }

    Rectangle {
        id: card
        anchors.fill: parent
        radius: height / 2
        color: Theme.glass
        border.color: Theme.line
        opacity: root.showing ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: Theme.quick } }
        RowLayout {
            anchors { fill: parent; leftMargin: 20; rightMargin: 22 }
            spacing: 14
            Icon { name: root.muted || root.volume <= 0 ? "vol-mute" : "vol"; size: 22 }
            Item {
                Layout.fillWidth: true
                implicitHeight: 6
                Rectangle { anchors.fill: parent; radius: 3; color: Theme.panel3 }
                Rectangle {
                    height: parent.height; radius: 3
                    width: root.muted ? 0 : parent.width * Math.min(1, root.volume / Buttons.maxVolume)
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop { position: 0; color: Theme.ember }
                        GradientStop { position: 1; color: root.volume > 1 ? Theme.danger : Theme.flame }
                    }
                }
                Rectangle {                  // where 100% is, when it can go past it
                    visible: Buttons.maxVolume > 1
                    x: parent.width / Buttons.maxVolume - 1; y: -4
                    width: 2; height: parent.height + 8; radius: 1
                    color: Qt.rgba(1, 1, 1, 0.5)
                }
            }
            Text {
                Layout.preferredWidth: 52
                horizontalAlignment: Text.AlignRight
                text: root.muted ? "Muted" : Math.round(root.volume * 100) + "%"
                color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold
            }
        }
    }
}

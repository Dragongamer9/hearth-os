// "How do I use this with a controller?": a card with the controls (View on the controller, or Controls
// in the Hearth menu), plus short messages at the top of the screen ("Precise pointer on").
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland

PanelWindow {
    id: root
    property bool shown: false
    property string message: ""
    function toggle() { shown = !shown; }
    function toast(text) { message = text; toastTimer.restart(); }

    visible: shown || message !== ""
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-help"
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    // Only the card catches clicks; a toast alone lets everything through.
    mask: Region { item: root.shown ? full : null }

    Item { id: full; anchors.fill: parent }
    MouseArea { anchors.fill: parent; enabled: root.shown; onClicked: root.shown = false }
    Timer { id: autoHide; interval: 12000; running: root.shown; onTriggered: root.shown = false }
    Timer { id: toastTimer; interval: 1800; onTriggered: root.message = "" }

    readonly property var controls: [
        ["Right stick", "Move the pointer (press it in: precise)"],
        ["Left stick", "Scroll"],
        ["A  /  RT", "Click (hold to drag)"],
        ["X  /  LT", "Right-click"],
        ["B", "Back (Esc)"],
        ["Y", "On-screen keyboard"],
        ["D-pad", "Arrow keys"],
        ["LB  /  RB", "Switch windows"],
        ["Menu ☰", "Apps"],
        ["Guide", "Hearth menu"],
        ["View + D-pad", "Snap left / right, maximize, minimize"],
        ["View + B", "Close the window"],
        ["View", "This list"]
    ]

    Rectangle {
        visible: root.shown
        anchors.centerIn: parent
        width: Math.min(parent.width - 48, 640)
        height: col.implicitHeight + 48
        radius: Theme.radius + 6
        color: Theme.glass
        border.color: Theme.line
        MouseArea { anchors.fill: parent; onClicked: root.shown = false }

        ColumnLayout {
            id: col
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 24 }
            spacing: 10
            RowLayout {
                spacing: 12
                Image { source: Qt.resolvedUrl("images/hearth-logo.png"); Layout.preferredWidth: 36; Layout.preferredHeight: 36; smooth: true; mipmap: true }
                Text { text: "Controller controls"; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold }
            }
            Text {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "The controller works like a mouse and keyboard everywhere in Desktop Mode. It pauses while a game is fullscreen."
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
            }
            GridLayout {
                Layout.fillWidth: true
                Layout.topMargin: 6
                columns: 2
                columnSpacing: 18
                rowSpacing: 8
                Repeater {
                    model: root.controls.length * 2
                    delegate: Text {
                        required property int index
                        readonly property var pair: root.controls[Math.floor(index / 2)]
                        Layout.fillWidth: index % 2 === 1
                        text: pair[index % 2]
                        color: index % 2 === 0 ? Theme.accent : Theme.text
                        font.family: Theme.font
                        font.pixelSize: 14
                        font.weight: index % 2 === 0 ? Font.DemiBold : Font.Normal
                    }
                }
            }
        }
    }

    Rectangle {
        visible: root.message !== ""
        anchors { top: parent.top; horizontalCenter: parent.horizontalCenter; topMargin: 24 }
        width: toastText.implicitWidth + 36
        height: 44
        radius: 22
        color: Theme.glass
        border.color: Theme.line
        Text { id: toastText; anchors.centerIn: parent; text: root.message; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
    }
}

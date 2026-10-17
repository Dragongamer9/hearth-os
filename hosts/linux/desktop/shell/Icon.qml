// A symbolic (one-color) icon from the icon theme, tinted to fit the dark Hearth palette.
// App icons (full color) use IconImage directly.
import QtQuick
import QtQuick.Effects
import Quickshell
import Quickshell.Widgets

Item {
    id: root
    property string name
    property color color: Theme.text
    property real size: 20
    implicitWidth: size
    implicitHeight: size

    IconImage {
        id: img
        anchors.fill: parent
        source: Quickshell.iconPath(root.name, true)
        visible: false
    }
    MultiEffect {
        anchors.fill: img
        source: img
        brightness: 1.0          // the theme draws them dark: lift to white...
        colorization: 1.0        // ...then tint
        colorizationColor: root.color
    }
}

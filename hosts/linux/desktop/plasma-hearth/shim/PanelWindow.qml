// Quickshell's PanelWindow (plasma-hearth), as an Item: Mantel's bar and dock draw inside Plasma's own panel (the
// package's main.qml makes it cover the whole panel window), and its full-screen menus inside a layer window
// (Overlay.qml). The layer-shell lines are taken out of Mantel's files when they're installed.
import QtQuick
import QtQuick.Window

Item {
    id: pw
    property var screen: Hearth.screen
    property color color: "transparent"
    property int exclusiveZone: 0
    property var mask: null
    property bool focusable: false
    // The window's own colour (the menus' darkened desktop behind them), drawn under everything in it.
    Rectangle { anchors.fill: parent; z: -1000; color: pw.color; visible: pw.color.a > 0 }
}

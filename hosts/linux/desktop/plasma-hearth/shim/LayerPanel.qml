// A small Quickshell PanelWindow inside one of Mantel's files (plasma-hearth; Notifications.qml's pop-ups): a real
// layer window over everything, on the edges and margins it asks for (KDE's LayerShellQt), never taking the keyboard.
import QtQuick
import QtQuick.Window
import org.kde.layershell 1.0 as LayerShell

Window {
    id: lp
    property PanelEdges anchors: PanelEdges {}
    property PanelMargins margins: PanelMargins {}
    property real implicitWidth: 100
    property real implicitHeight: 100
    property var screen0: null
    property int exclusiveZone: 0
    property var mask: null
    property string scopeName: "hearth-notifications"
    property bool wantsKeys: false     // typing in it (a notification's Reply box): the keyboard while it's asked for
    width: Math.max(1, implicitWidth)
    height: Math.max(1, implicitHeight)
    flags: wantsKeys ? Qt.FramelessWindowHint : (Qt.FramelessWindowHint | Qt.WindowDoesNotAcceptFocus)
    LayerShell.Window.scope: scopeName
    LayerShell.Window.layer: LayerShell.Window.LayerOverlay
    LayerShell.Window.keyboardInteractivity: wantsKeys ? LayerShell.Window.KeyboardInteractivityOnDemand : LayerShell.Window.KeyboardInteractivityNone
    LayerShell.Window.exclusionZone: -1
    LayerShell.Window.anchors: (anchors.top ? LayerShell.Window.AnchorTop : 0) | (anchors.bottom ? LayerShell.Window.AnchorBottom : 0)
        | (anchors.left ? LayerShell.Window.AnchorLeft : 0) | (anchors.right ? LayerShell.Window.AnchorRight : 0)
    LayerShell.Window.margins.top: margins.top
    LayerShell.Window.margins.bottom: margins.bottom
    LayerShell.Window.margins.left: margins.left
    LayerShell.Window.margins.right: margins.right
}

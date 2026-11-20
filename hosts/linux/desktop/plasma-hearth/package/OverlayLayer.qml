import QtQuick
import QtQuick.Window
import org.kde.layershell 1.0 as LayerShell

Window {
    id: w
    property string name: ""
    property string pieceSource: ""
    readonly property var content: piece.item
    color: "transparent"
    flags: Qt.FramelessWindowHint
    visible: !!piece.item && piece.item.shown === true
    onVisibleChanged: if (visible) requestActivate()
    LayerShell.Window.scope: name
    LayerShell.Window.layer: LayerShell.Window.LayerOverlay
    LayerShell.Window.anchors: LayerShell.Window.AnchorTop | LayerShell.Window.AnchorBottom | LayerShell.Window.AnchorLeft | LayerShell.Window.AnchorRight
    LayerShell.Window.keyboardInteractivity: LayerShell.Window.KeyboardInteractivityExclusive
    LayerShell.Window.exclusionZone: -1
    width: Screen.width
    height: Screen.height
    Loader { id: piece; anchors.fill: parent; source: w.pieceSource; onStatusChanged: if (status === Loader.Error) console.warn("plasma-hearth: " + w.pieceSource + " couldn't load") }
}

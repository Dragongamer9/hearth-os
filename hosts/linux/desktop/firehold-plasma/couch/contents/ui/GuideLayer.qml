// The Guide's window: a layer above everything on the right edge of the screen (KDE's LayerShellQt, which KWin speaks),
// so it shows over a full-screen game. If LayerShellQt's QML can't load, main.qml uses GuideWindow.qml instead.
import QtQuick
import QtQuick.Window
import org.kde.layershell 1.0 as LayerShell

Window {
    id: gw
    property QtObject couch: null
    visible: false
    color: "transparent"
    width: 560 * (couch ? couch.k : 1)
    height: screen ? screen.height : 1080
    LayerShell.Window.scope: "hearth-firehold-guide"
    LayerShell.Window.layer: LayerShell.Window.LayerOverlay
    LayerShell.Window.anchors: LayerShell.Window.AnchorTop | LayerShell.Window.AnchorBottom | LayerShell.Window.AnchorRight
    LayerShell.Window.keyboardInteractivity: LayerShell.Window.KeyboardInteractivityExclusive
    LayerShell.Window.exclusionZone: -1
    Component.onCompleted: { visible = true; requestActivate(); panel.forceActiveFocus(); }
    Guide { id: panel; anchors.fill: parent; couch: gw.couch }
}

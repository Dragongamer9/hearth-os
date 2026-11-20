// Mantel's desktop on Plasma (plasma-hearth): its wallpaper with each look's own picture or moving scene and the
// desktop clock (Wallpaper.qml), and its desktop icons with their right-click menus (Desktop.qml), in one layer window
// just above Plasma's own desktop and under every window (KDE's LayerShellQt). Only while Hearth's panel is in use.
import QtQuick
import QtQuick.Window
import org.kde.layershell 1.0 as LayerShell

Window {
    id: w
    property var menus: null
    color: "transparent"
    flags: Qt.FramelessWindowHint
    visible: true
    LayerShell.Window.scope: "hearth-desktop"
    LayerShell.Window.layer: LayerShell.Window.LayerBottom
    LayerShell.Window.anchors: LayerShell.Window.AnchorTop | LayerShell.Window.AnchorBottom | LayerShell.Window.AnchorLeft | LayerShell.Window.AnchorRight
    LayerShell.Window.keyboardInteractivity: LayerShell.Window.KeyboardInteractivityOnDemand
    LayerShell.Window.exclusionZone: -1
    width: Screen.width
    height: Screen.height
    function backgroundMenu(x, y) { if (desk.item) desk.item.backgroundMenu(x, y); }
    readonly property var scr: ({ width: Screen.width, height: Screen.height, name: Screen.name })
    Loader {
        id: wall
        anchors.fill: parent
        source: "Wallpaper.qml"
        onLoaded: { item.primary = true; item.screen = Qt.binding(() => w.scr); }
        onStatusChanged: if (status === Loader.Error) console.warn("plasma-hearth: Mantel's wallpaper (Wallpaper.qml) couldn't load")
    }
    Loader {
        id: desk
        anchors.fill: parent
        source: "Desktop.qml"
        onLoaded: {
            item.primary = true;
            item.screen = Qt.binding(() => w.scr);
            item.menu = Qt.binding(() => w.menus ? w.menus.popMenu() : null);
            item.dialog = Qt.binding(() => w.menus ? w.menus.deskDialog() : null);
        }
        onStatusChanged: if (status === Loader.Error) console.warn("plasma-hearth: Mantel's desktop (Desktop.qml) couldn't load")
    }
}

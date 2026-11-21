// "A controller is in your hand" (mockup firehold-plasma 12): at the bottom right of the bar's screen, when a
// controller button is pressed on the Desk. hearth-firehold-plasma shows it (ping hint) and takes it away again
// (ping hint-off, a few seconds later); nothing here counts time.
import QtQuick
import QtQuick.Window
import org.kde.layershell 1.0 as LayerShell
import "couch"

Window {
    id: hw
    property QtObject bar: null
    visible: true
    transientParent: null
    color: "transparent"
    screen: {
        const l = Qt.application.screens;
        for (let i = 0; i < l.length; i++) if (bar && l[i].name === bar.myScreen) return l[i];
        return l.length ? l[0] : null;
    }
    width: 540; height: card.height
    LayerShell.Window.scope: "hearth-firehold-hint"
    LayerShell.Window.layer: LayerShell.Window.LayerOverlay
    LayerShell.Window.anchors: LayerShell.Window.AnchorBottom | LayerShell.Window.AnchorRight
    LayerShell.Window.margins.right: 38
    LayerShell.Window.margins.bottom: 116
    LayerShell.Window.keyboardInteractivity: LayerShell.Window.KeyboardInteractivityNone
    LayerShell.Window.exclusionZone: -1
    Rectangle {
        id: card
        width: parent.width; height: words.implicitHeight + 36
        radius: 14; color: "#221e1c"; border.width: 1; border.color: "#1affffff"
        Rectangle {
            x: 18; anchors.verticalCenter: parent.verticalCenter
            width: 40; height: 40; radius: 10; color: "#2e2824"
            Glyph { anchors.centerIn: parent; name: "pad"; size: 18 }
        }
        Column {
            id: words
            x: 72; y: 18; width: parent.width - 90; spacing: 3
            Text { text: "A controller is in your hand"; color: "#f2ede8"; font.pixelSize: 17; font.weight: Font.DemiBold }
            Text { width: parent.width; wrapMode: Text.Wrap; text: "Press ≡ for the Couch on this screen, or View for these menus with the controller"; color: "#b9aea6"; font.pixelSize: 14 }
        }
    }
}

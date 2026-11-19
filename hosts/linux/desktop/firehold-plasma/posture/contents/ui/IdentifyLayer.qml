// Identify (Settings › Couch, Desk and Away): this screen's number in the middle of it, for a few seconds.
import QtQuick
import QtQuick.Window
import org.kde.layershell 1.0 as LayerShell

Window {
    id: iw
    property QtObject bar: null
    visible: true
    color: "transparent"
    screen: {
        const l = Qt.application.screens;
        for (let i = 0; i < l.length; i++) if (bar && l[i].name === bar.myScreen) return l[i];
        return l.length ? l[0] : null;
    }
    width: 260; height: 260
    LayerShell.Window.scope: "hearth-firehold-identify"
    LayerShell.Window.layer: LayerShell.Window.LayerOverlay
    LayerShell.Window.anchors: LayerShell.Window.AnchorNone
    LayerShell.Window.keyboardInteractivity: LayerShell.Window.KeyboardInteractivityNone
    LayerShell.Window.exclusionZone: -1
    Rectangle {
        anchors.fill: parent; radius: 40
        color: "#e6221e1c"; border.width: 4; border.color: "#ff7a2f"
        Text { anchors.centerIn: parent; text: String(Qt.application.screens.map(function (s) { return s.name; }).indexOf(iw.bar ? iw.bar.myScreen : "") + 1); color: "#f2ede8"; font.pixelSize: 150; font.weight: Font.Bold }
    }
}

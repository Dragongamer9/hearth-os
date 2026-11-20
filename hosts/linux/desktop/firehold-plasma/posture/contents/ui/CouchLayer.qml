// This screen's Couch (or Away) over the Desk, while another screen is Desk: the Couch's own page (Couch.qml, copied in
// with install.sh) in a full-screen layer on this screen (KDE's LayerShellQt, which KWin speaks). A window or game
// coming in front hides it; Home and the Guide bring it back.
import QtQuick
import QtQuick.Window
import org.kde.layershell 1.0 as LayerShell
import "couch"

Window {
    id: cw
    property QtObject bar: null
    property string screenName: ""
    visible: false
    color: "#0f0b0a"
    screen: {
        const l = Qt.application.screens;
        for (let i = 0; i < l.length; i++) if (l[i].name === screenName) return l[i];
        return l.length ? l[0] : null;
    }
    width: screen ? screen.width : 1920
    height: screen ? screen.height : 1080
    LayerShell.Window.scope: "hearth-firehold-couch"
    LayerShell.Window.layer: LayerShell.Window.LayerOverlay
    LayerShell.Window.anchors: LayerShell.Window.AnchorTop | LayerShell.Window.AnchorBottom | LayerShell.Window.AnchorLeft | LayerShell.Window.AnchorRight
    LayerShell.Window.keyboardInteractivity: LayerShell.Window.KeyboardInteractivityExclusive
    LayerShell.Window.exclusionZone: -1
    function show() {
        visible = true; requestActivate(); couch.forceActiveFocus();
        console.warn("firehold: the Couch over the Desk on", screenName, "(window on", screen ? screen.name : "no screen", ")");
    }
    Couch {
        id: couch
        anchors.fill: parent
        layered: true
        focus: true
        onStepAside: cw.visible = false
        onComeBack: cw.show()
    }
}

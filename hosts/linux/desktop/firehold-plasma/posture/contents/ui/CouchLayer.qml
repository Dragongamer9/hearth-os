// This screen's Couch (or Away) over the Desk, while another screen is Desk: the Couch's own page (Couch.qml, copied in
// with install.sh) full screen on that screen. A plain window (a layer can't be put on another screen from the bar:
// Wayland places it on the bar's), named "Firehold Couch on NAME" so Firehold's KWin script puts it on its screen, full
// screen, out of the task bar and the window switcher. A window or game coming in front hides it; Home and the
// Guide bring it back.
import QtQuick
import QtQuick.Window
import "couch"

Window {
    id: cw
    property QtObject bar: null
    property string screenName: ""
    title: "Firehold Couch on " + screenName
    visible: false
    flags: Qt.FramelessWindowHint
    color: "#0f0b0a"
    width: 1920; height: 1080
    onVisibleChanged: console.warn("firehold: Couch on", screenName, "visible", visible)
    function show() {
        if (screenName === "") return;
        visible = true;
        raise();
        requestActivate();
        couch.forceActiveFocus();
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

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
    // Its own window, not a child of the bar's panel (a panel is a layer: Qt won't show a window that hangs off one).
    transientParent: null
    flags: Qt.FramelessWindowHint
    color: "#0f0b0a"
    width: 1920; height: 1080
    onVisibleChanged: console.warn("firehold: Couch on", screenName, "visible", visible)
    function readModes() { couch.readModes(); }
    function show() {
        console.warn("firehold: Couch for", screenName);
        if (screenName === "") return;
        // Its own screen's size and place first (so it never shows small); then full screen there. KWin's script puts
        // it on that screen too, by its name.
        const s = Qt.application.screens.filter(function (q) { return q.name === screenName; })[0];
        if (s) { x = s.virtualX; y = s.virtualY; width = s.width; height = s.height; }
        visible = true;
        visibility = Window.FullScreen;
        raise();
        requestActivate();
        couch.forceActiveFocus();
    }
    Couch {
        id: couch
        anchors.fill: parent
        layered: true
        screenFor: cw.screenName
        focus: true
        onStepAside: cw.visible = false
        onComeBack: cw.show()
    }
}

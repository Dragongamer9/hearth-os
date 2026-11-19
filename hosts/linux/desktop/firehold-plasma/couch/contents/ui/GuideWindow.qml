// The Guide without LayerShellQt: a full-screen see-through window (Wayland apps can't place a window themselves), the
// panel on its right; a press on the left part closes it.
import QtQuick
import QtQuick.Window

Window {
    id: gw
    property QtObject couch: null
    visible: false
    color: "transparent"
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    Component.onCompleted: { visibility = Window.FullScreen; requestActivate(); panel.forceActiveFocus(); }
    MouseArea { anchors.fill: parent; onClicked: if (gw.couch) gw.couch.guideShown = false }
    Guide { id: panel; anchors.right: parent.right; width: 560 * (gw.couch ? gw.couch.k : 1); height: parent.height; couch: gw.couch }
}

// A control the selection can land on (Nav.qml): draws Hearth's focus ring (2 px of accent, a gap of the background),
// scrolls itself into view when selected, and is clicked or tapped like a button.
import QtQuick

Item {
    id: f
    property string navKey: ""
    property string navScope: "view"
    property bool navFirst: false
    property var navMove: null            // function(dir) → true when the control used the press itself
    property real radius: Theme.radius
    property bool ring: true
    property bool zoom: false             // tiles grow a little when selected, like the web page's
    property bool hovered: mouse.containsMouse
    readonly property bool isFocused: Nav.focused === f
    signal activated()
    signal revealed()
    signal pressedAndHeld()

    activeFocusOnTab: false
    scale: zoom && isFocused ? 1.045 : 1
    z: isFocused ? 2 : 0
    Behavior on scale { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutCubic } }

    Component.onCompleted: Nav.register(f)
    Component.onDestruction: Nav.unregister(f)
    onRevealed: {
        // Every scrolling area it's in brings it into view (a shelf sideways, the page up and down).
        let p = f.parent;
        while (p) { if (p.hearthScroller === true) p.ensureVisible(f); p = p.parent; }
    }

    Rectangle {
        visible: f.ring && f.isFocused
        anchors.fill: parent
        anchors.margins: -Theme.u(4)
        radius: f.radius + Theme.u(4)
        color: "transparent"
        border.width: Theme.u(2)
        border.color: Theme.accent
        z: 10
        Rectangle {
            anchors.fill: parent
            anchors.margins: Theme.u(2)
            radius: f.radius + Theme.u(2)
            color: "transparent"
            border.width: Theme.u(2)
            border.color: Theme.bg
        }
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        z: -1
        cursorShape: Qt.PointingHandCursor
        pressAndHoldInterval: 600
        onClicked: { App.pointerUsed(); Nav.focus(f, true); f.activated(); }
        onPressAndHold: f.pressedAndHeld()
        onWheel: wheel => { wheel.accepted = false; }
    }
}

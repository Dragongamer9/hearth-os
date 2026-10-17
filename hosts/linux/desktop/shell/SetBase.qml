// Base for every control in Hearth Settings: a rounded row that can take focus. Up/Down move between
// controls, Enter (A on a controller) activates, Left goes back to the page list, Esc (B) too. The page
// scrolls to keep the focused control in view.
import QtQuick

Rectangle {
    id: base
    property bool interactive: true
    signal activated()
    signal adjusted(int step)          // Left/Right on sliders and choices
    property bool usesLeftRight: false

    activeFocusOnTab: interactive
    radius: 12
    color: hover.containsMouse && interactive ? Theme.panel3 : Theme.panel2
    border.width: 2
    border.color: activeFocus ? Theme.accent : "transparent"
    Behavior on border.color { ColorAnimation { duration: Theme.quick } }
    implicitHeight: 64

    function scrollIntoView() {
        let p = base.parent;
        while (p && !(p.contentItem !== undefined && p.contentHeight !== undefined && p.contentY !== undefined)) p = p.parent;
        if (!p) return;
        const y = base.mapToItem(p.contentItem, 0, 0).y;
        if (y < p.contentY + 8) p.contentY = Math.max(0, y - 8);
        else if (y + base.height > p.contentY + p.height - 8) p.contentY = Math.min(p.contentHeight - p.height, y + base.height - p.height + 8);
    }
    onActiveFocusChanged: if (activeFocus) scrollIntoView()

    Keys.onPressed: e => {
        switch (e.key) {
        case Qt.Key_Down: case Qt.Key_Tab: base.nextItemInFocusChain(true).forceActiveFocus(); break;
        case Qt.Key_Up: case Qt.Key_Backtab: base.nextItemInFocusChain(false).forceActiveFocus(); break;
        case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: base.activated(); break;
        case Qt.Key_Right: if (base.usesLeftRight) base.adjusted(1); else return; break;
        case Qt.Key_Left: if (base.usesLeftRight) base.adjusted(-1); else SettingsNav.toNav(); break;
        case Qt.Key_Escape: SettingsNav.toNav(); break;
        default: return;
        }
        e.accepted = true;
    }
    MouseArea {
        id: hover
        anchors.fill: parent
        hoverEnabled: true
        enabled: base.interactive
        cursorShape: Qt.PointingHandCursor
        onClicked: { base.forceActiveFocus(); base.activated(); }
    }
}

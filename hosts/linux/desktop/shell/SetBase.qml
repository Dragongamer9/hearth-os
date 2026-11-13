// Base for every control in Hearth Settings: a setting card that can take focus. Up/Down move between controls,
// Enter (A on a controller) activates, Left goes back to the category list, Esc (B) goes back one level (a subpage to
// its category, a category to the list). The page scrolls to keep the focused card in view, and a big ring shows
// where the controller is. `inset`: a row inside an expander card (SetExpander), drawn flat and indented.
import QtQuick

Rectangle {
    id: base
    property bool interactive: true
    property bool inset: false
    property bool flat: false          // a row inside a Home card: no card of its own
    signal activated()
    signal adjusted(int step)          // Left/Right on sliders and choices
    property bool usesLeftRight: false
    // Esc (B) handled by the control itself (an open dropdown's options close it) instead of going back a level.
    property bool ownsEscape: false
    signal escaped()

    activeFocusOnTab: interactive
    radius: inset ? 0 : SetStyle.radius
    color: inset || flat ? (hover.containsMouse && interactive ? SetStyle.tint(Theme.text, 0.04) : "transparent")
                 : (hover.containsMouse && interactive ? SetStyle.cardHover : SetStyle.card)
    border.width: inset || flat ? 0 : 1
    border.color: SetStyle.tint(Theme.text, 0.05)
    Behavior on color { ColorAnimation { duration: Theme.quick } }
    implicitHeight: SetStyle.cardHeight
    z: activeFocus ? 2 : 0

    // Rows inside an expander: a thin line between them.
    Rectangle { visible: base.inset; anchors { left: parent.left; right: parent.right; top: parent.top } height: 1; color: SetStyle.tint(Theme.text, 0.05) }
    // The controller's focus ring: a thick accent outline with a soft glow, a little outside the card.
    Rectangle {
        visible: base.activeFocus
        anchors { fill: parent; margins: -9 }
        radius: base.radius + 9
        color: "transparent"
        border.width: 6
        border.color: SetStyle.tint(Theme.accent, 0.22)
    }
    Rectangle {
        visible: base.activeFocus
        anchors { fill: parent; margins: -4 }
        radius: base.radius + 4
        color: "transparent"
        border.width: 3
        border.color: Theme.accent
    }

    function scrollIntoView() {
        let p = base.parent;
        while (p && !(p.contentItem !== undefined && p.contentHeight !== undefined && p.contentY !== undefined)) p = p.parent;
        if (!p) return;
        const y = base.mapToItem(p.contentItem, 0, 0).y;
        if (y < p.contentY + 12) p.contentY = Math.max(0, y - 12);
        else if (y + base.height > p.contentY + p.height - 12) p.contentY = Math.min(p.contentHeight - p.height, y + base.height - p.height + 12);
    }
    onActiveFocusChanged: if (activeFocus) scrollIntoView()

    Keys.onPressed: e => {
        switch (e.key) {
        case Qt.Key_Down: case Qt.Key_Tab: base.nextItemInFocusChain(true).forceActiveFocus(); break;
        case Qt.Key_Up: case Qt.Key_Backtab: base.nextItemInFocusChain(false).forceActiveFocus(); break;
        case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: base.activated(); break;
        case Qt.Key_Right: if (base.usesLeftRight) base.adjusted(1); else return; break;
        case Qt.Key_Left: if (base.usesLeftRight) base.adjusted(-1); else SettingsNav.toNav(); break;
        case Qt.Key_Escape: if (base.ownsEscape) base.escaped(); else SettingsNav.back(); break;
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

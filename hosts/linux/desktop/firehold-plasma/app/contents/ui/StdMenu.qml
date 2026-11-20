// One of the Desk bar's standard menus (mockup firehold-plasma 12): the app's own name menu, Window, Help, for an app
// with no menus of its own. Plasma's popup under its button, the mockup's dark menu; a click does it and closes it.
// Window's "Move to screen ›" opens its list of screens beside it (each with its mode), in the same popup.
import QtQuick
import QtQuick.Layouts
import org.kde.plasma.core as PlasmaCore
import "couch"

PlasmaCore.Dialog {
    id: m
    // [label, shortcut or note on the right, action, kind (head | red | sep | sub | pair), what it does, icon]
    property var items: []
    property var subItems: []           // the screens for "Move to screen ›"
    property bool subOpen: false
    property Item anchorItem: null
    signal picked(string action)
    visualParent: anchorItem
    location: PlasmaCore.Types.TopEdge
    type: PlasmaCore.Dialog.PopupMenu
    backgroundHints: PlasmaCore.Dialog.NoBackground
    hideOnWindowDeactivate: true
    flags: Qt.WindowStaysOnTopHint
    mainItem: box
    // (the card itself is the Couch's MenuCard.qml, shared with Firehold on labwc's Desk bar)
    MenuCard {
        id: box
        items: m.items; subItems: m.subItems; subOpen: m.subOpen
        onPicked: function (a) { m.picked(a); m.visible = false; }
    }
}

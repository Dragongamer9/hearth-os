// The Desk bar's first-run tip (mockup firehold-plasma 11): each part of the bar, left to right, and Alt+Space.
// Once (hearth-firehold-plasma tip); Plasma's popup under the bar, gone when you click elsewhere.
import QtQuick
import org.kde.plasma.core as PlasmaCore
import "couch"
import "couch/fhp.js" as H

PlasmaCore.Dialog {
    id: tip
    property QtObject bar: null
    visualParent: bar ? bar.pillItem : null
    location: PlasmaCore.Types.TopEdge
    type: PlasmaCore.Dialog.PopupMenu
    backgroundHints: PlasmaCore.Dialog.NoBackground
    hideOnWindowDeactivate: true
    flags: Qt.WindowStaysOnTopHint
    onVisibleChanged: if (!visible && bar) bar.tipShown = false
    Component.onCompleted: visible = true
    mainItem: card
    // (the card itself is the Couch's BarTipCard.qml, shared with Firehold on labwc)
    BarTipCard { id: card }
}

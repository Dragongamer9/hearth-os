// Modes (mockup firehold-plasma 04), from the Desk bar's mode pill: every screen drawn as placed, each named, with its
// mode; Couch / Desk / Away for each; Both Couch, Both Desk, Away now; the Automatic and Away-after switches (Firehold's
// settings, written only when you flip one); the keys. Switching a screen is live state, never a setting.
import QtQuick
import QtQuick.Window
import org.kde.plasma.core as PlasmaCore
import "couch"
import "couch/fhp.js" as H

// Plasma's own popup under the pill (it places it and closes it when you click elsewhere), drawing the mockup's card.
PlasmaCore.Dialog {
    id: mp
    property QtObject bar: null
    visualParent: bar ? bar.pillItem : null
    location: PlasmaCore.Types.TopEdge
    type: PlasmaCore.Dialog.PopupMenu
    backgroundHints: PlasmaCore.Dialog.NoBackground
    hideOnWindowDeactivate: true
    flags: Qt.WindowStaysOnTopHint
    onVisibleChanged: if (!visible && bar) bar.popupOpen = false
    Component.onCompleted: { visible = true; card.forceActiveFocus(); }
    mainItem: card

    // (the card itself is the Couch's ModesCard.qml, shared with Firehold on labwc)
    ModesCard { id: card; bar: mp.bar }
}

// Hearth Settings › System › Couch, Desk and Away: Firehold on Plasma's page (the Fedora edition's Rust Settings app
// draws it: hosts/hearth-ui/apps/settings/src/cats/couch.rs, mockup docs/mockups/firehold-plasma/05). Mantel has no
// Couch, Desk or Away, so here it only says where it is. Nothing is read or written.
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Couch, Desk and Away"
    Text {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
        text: "Couch for the controller and the TV, Desk for the mouse and keyboard, Away when nobody's here: each screen's mode, starts in, switching by itself, while Away. These are Firehold's, on the Fedora edition's Settings."
    }
}

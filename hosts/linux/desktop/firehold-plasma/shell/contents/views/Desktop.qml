// Firehold's Couch (a Plasma shell package, the way Plasma Bigscreen's is made): each screen's desktop view holds its
// containment, org.hearthos.firehold.couch, which draws the Couch. Nothing else lives here.
import QtQuick

Rectangle {
    id: root
    color: "#120c09"
    property Item containment

    onContainmentChanged: {
        if (!containment) return;
        containment.parent = root;
        containment.visible = true;
        containment.anchors.fill = root;
    }
}

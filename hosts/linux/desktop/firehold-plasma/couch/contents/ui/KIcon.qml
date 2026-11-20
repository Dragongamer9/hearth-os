// An app's icon on the Apps tab (AppsPage.qml) under Plasma: Kirigami's, from the icon theme.
import QtQuick
import org.kde.kirigami as Kirigami

Kirigami.Icon {
    property string name: ""
    // (a menu's line icon: one colour, as the mockups draw them)
    property bool mask: false
    property color tint: "#f2ede8"
    isMask: mask
    color: mask ? tint : "transparent"
    anchors.fill: parent
    source: name || "application-x-executable"
    fallback: "application-x-executable"
}

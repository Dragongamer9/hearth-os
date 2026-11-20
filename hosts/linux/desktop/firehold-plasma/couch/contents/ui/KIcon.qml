// An app's icon on the Apps tab (AppsPage.qml) under Plasma: Kirigami's, from the icon theme.
import QtQuick
import org.kde.kirigami as Kirigami

Kirigami.Icon {
    property string name: ""
    anchors.fill: parent
    source: name || "application-x-executable"
    fallback: "application-x-executable"
}

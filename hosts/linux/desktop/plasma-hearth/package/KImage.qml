// A notification's picture under Plasma (FireholdCards.qml), when KDE's notification service gives the picture
// itself rather than its file: Kirigami draws it (it takes a QImage as it is).
import QtQuick
import org.kde.kirigami as Kirigami

Kirigami.Icon {
    property var picture: null
    source: picture
    anchors.fill: parent
}

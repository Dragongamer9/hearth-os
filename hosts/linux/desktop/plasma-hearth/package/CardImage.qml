// A notification's own picture as Plasma hands it over (a QImage: image-data, or image-path loaded by Plasma), for
// Firehold's cards (FireholdCards.qml). Its own file, loaded by a Loader: without KDE's kquickcontrolsaddons module the
// card just has no picture.
import QtQuick
import org.kde.kquickcontrolsaddons as KQCA

KQCA.QImageItem {
    property var img: null
    image: img
    fillMode: KQCA.QImageItem.PreserveAspectCrop
}

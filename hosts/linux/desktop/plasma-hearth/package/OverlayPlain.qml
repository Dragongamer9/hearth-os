import QtQuick
import QtQuick.Window

Window {
    id: w
    property string name: ""
    property string pieceSource: ""
    readonly property var content: piece.item
    color: "transparent"
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    visibility: !!piece.item && piece.item.shown === true ? Window.FullScreen : Window.Hidden
    Loader { id: piece; anchors.fill: parent; source: w.pieceSource }
}

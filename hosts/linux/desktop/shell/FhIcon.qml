// An app's icon on the shared Couch's Apps tab (AppsPage.qml) under Firehold on labwc: the icon theme's, as Mantel
// finds icons (AppIcons), the same as KIcon.qml under Plasma.
import QtQuick

Image {
    property string name: ""
    property bool mask: false          // (as KIcon.qml: a menu's line icon)
    property color tint: "#f2ede8"
    anchors.fill: parent
    asynchronous: true; smooth: true; mipmap: true
    sourceSize.width: Math.ceil(width * Screen.devicePixelRatio); sourceSize.height: Math.ceil(height * Screen.devicePixelRatio)
    source: AppIcons.find(name || "application-x-executable", "")
}

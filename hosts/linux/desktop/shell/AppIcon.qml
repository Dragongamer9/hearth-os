// An app's picture, anywhere in Mantel: its desktop entry's Icon= and/or a window's app id, found by AppIcons.qml
// (icon theme with hicolor, Flatpak exports, absolute paths, pixmaps, app id matching), Hearth's generic app
// picture when nothing fits or the file can't be read. `source` overrides the lookup (Hearth's own windows).
import QtQuick

Item {
    id: root
    property string icon
    property string appId
    property string source
    property real size: 32
    // Notifications: nothing at all rather than the generic picture.
    property bool genericFallback: true
    implicitWidth: size
    implicitHeight: size

    readonly property string found: source !== "" ? source : AppIcons.find(icon, appId)
    readonly property bool hasIcon: found !== "" && !failed
    property bool failed: false
    onFoundChanged: failed = false

    Image {
        anchors.fill: parent
        source: root.hasIcon ? root.found : root.genericFallback ? AppIcons.generic : ""
        sourceSize: Qt.size(Math.ceil(root.size * 2), Math.ceil(root.size * 2))
        fillMode: Image.PreserveAspectFit
        smooth: true
        mipmap: true
        asynchronous: true
        // A file that isn't there (a stale absolute path, a guessed pixmap): the generic picture instead.
        onStatusChanged: if (status === Image.Error && source == root.found) root.failed = true
    }
}

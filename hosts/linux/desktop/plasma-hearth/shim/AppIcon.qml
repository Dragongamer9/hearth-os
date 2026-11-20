// An app's picture (plasma-hearth's AppIcon.qml, same properties as Mantel's): the icon theme through Kirigami
// (Plasma's own lookup: hicolor, Flatpak exports, pixmaps), a picture file, or Hearth's generic app picture.
import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: root
    property string icon
    property string appId
    property string source
    property real size: 32
    property bool genericFallback: true
    implicitWidth: size
    implicitHeight: size
    readonly property string found: source !== "" ? source : AppIcons.find(icon, appId)
    readonly property bool hasIcon: found !== "" && k.valid
    Kirigami.Icon {
        id: k
        anchors.fill: parent
        source: {
            const f = root.found;
            if (f.startsWith("/")) return "file://" + f;
            return f;
        }
        fallback: root.genericFallback ? "application-x-executable" : ""
        visible: root.found !== "" || root.genericFallback
    }
}

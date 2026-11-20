// Quickshell's IconImage (plasma-hearth): a themed icon or a picture (an app's tray icon).
// Quickshell.iconPath(name, "fallback") (this shim's Quickshell.qml) gives "name|fallback": the theme's icon by that
// name, else the fallback's, the way Quickshell looks them up. Plasma has no image://icon/ provider, so names go
// to Kirigami.Icon as names.
import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: ii
    property var source: ""
    property real implicitSize: 20
    // An app's own tray picture comes as an icon (QIcon), not a name: given to Kirigami.Icon as it is.
    readonly property bool text: typeof source === "string" || source instanceof String
    readonly property string src: text ? String(source || "") : ""
    readonly property bool named: src !== "" && src.indexOf("://") < 0 && src.indexOf("/") !== 0
    implicitWidth: implicitSize
    implicitHeight: implicitSize
    Kirigami.Icon {
        anchors.fill: parent
        source: ii.named ? ii.src.split("|")[0] : (ii.src.indexOf("image://icon/") === 0 ? ii.src.slice(13) : ii.source)
        fallback: ii.named && ii.src.indexOf("|") > 0 ? ii.src.split("|")[1] : "unknown"
    }
}

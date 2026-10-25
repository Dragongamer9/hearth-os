// One of the web page's icons (Icons.js, made from ui/index.html), drawn as lines in any color. An icon is at most four
// paths (gen-icons.py keeps it that way).
import QtQuick
import QtQuick.Shapes
import "Icons.js" as Icons

Item {
    id: ic
    property string name: "dot"
    property color color: Theme.text
    property real size: Theme.u(20)
    readonly property var parts: Icons.I[name] || []
    implicitWidth: size
    implicitHeight: size
    width: size
    height: size

    function d(i) { return i < parts.length ? parts[i][0] : ""; }
    function filled(i) { return i < parts.length && parts[i][1]; }

    Shape {
        width: 24
        height: 24
        scale: ic.size / 24
        transformOrigin: Item.TopLeft
        antialiasing: true
        visible: ic.parts.length > 0
        ShapePath {
            strokeColor: ic.filled(0) || !ic.d(0) ? "transparent" : ic.color; fillColor: ic.filled(0) ? ic.color : "transparent"
            strokeWidth: 1.8; capStyle: ShapePath.RoundCap; joinStyle: ShapePath.RoundJoin
            PathSvg { path: ic.d(0) }
        }
        ShapePath {
            strokeColor: ic.filled(1) || !ic.d(1) ? "transparent" : ic.color; fillColor: ic.filled(1) ? ic.color : "transparent"
            strokeWidth: 1.8; capStyle: ShapePath.RoundCap; joinStyle: ShapePath.RoundJoin
            PathSvg { path: ic.d(1) }
        }
        ShapePath {
            strokeColor: ic.filled(2) || !ic.d(2) ? "transparent" : ic.color; fillColor: ic.filled(2) ? ic.color : "transparent"
            strokeWidth: 1.8; capStyle: ShapePath.RoundCap; joinStyle: ShapePath.RoundJoin
            PathSvg { path: ic.d(2) }
        }
        ShapePath {
            strokeColor: ic.filled(3) || !ic.d(3) ? "transparent" : ic.color; fillColor: ic.filled(3) ? ic.color : "transparent"
            strokeWidth: 1.8; capStyle: ShapePath.RoundCap; joinStyle: ShapePath.RoundJoin
            PathSvg { path: ic.d(3) }
        }
    }
}

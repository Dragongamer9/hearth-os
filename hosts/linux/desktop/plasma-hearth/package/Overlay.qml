// One of Mantel's full-screen pieces in a layer window (plasma-hearth): OverlayLayer.qml (KDE's LayerShellQt, which
// KWin speaks: over everything, the keyboard while it's open) or, without it, a plain full-screen window. The
// window shows while the piece is "shown" (Mantel's own property), and is made only once something asks for it.
import QtQuick

Item {
    id: o
    property string name: ""
    property string source: ""
    property bool wanted: false
    property var pending: null
    readonly property var content: win.item ? win.item.content : null
    readonly property bool ready: content !== null
    onContentChanged: if (content && pending) { const f = pending; pending = null; Qt.callLater(f); }
    Loader {
        id: win
        active: o.wanted
        source: layerOk ? "OverlayLayer.qml" : "OverlayPlain.qml"
        property bool layerOk: true
        onStatusChanged: if (status === Loader.Error && layerOk) { console.warn("plasma-hearth: LayerShellQt's QML isn't here; " + o.name + " opens in a plain window"); layerOk = false; }
        onLoaded: { item.name = o.name; item.pieceSource = o.source; }
    }
}

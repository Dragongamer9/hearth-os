// One of Firehold's shared full-screen pieces on labwc (1.16): Quick Settings (FireholdQuick.qml, mockups 07-08) or
// the notification list (FireholdHistory.qml, mockup 10), the very files Firehold on Plasma shows (from
// desktop/plasma-hearth/package, copied in when the image is built). A layer window over everything on its screen,
// with the keyboard while the piece is open; made only once something asks for it, gone again when it closes.
import QtQuick
import Quickshell
import Quickshell.Wayland

Scope {
    id: fp
    property string source: ""          // FireholdQuick.qml | FireholdHistory.qml
    property string name: ""
    property var screen: null
    property bool wanted: false
    property var pending: null
    readonly property var content: win.item ? win.item.piece : null
    readonly property bool shown: !!content && content.shown === true
    onContentChanged: if (content && pending) { const f = pending; pending = null; Qt.callLater(f); }
    // Open or close it (made first if it isn't yet).
    function toggle() { wanted = true; if (content) content.toggle(); else pending = () => fp.content.toggle(); }
    function close() { if (content && content.shown) content.close(); }
    LazyLoader {
        id: win
        active: fp.wanted
        PanelWindow {
            readonly property var piece: loader.item
            screen: fp.screen
            visible: !!loader.item && loader.item.shown === true
            color: "transparent"
            WlrLayershell.namespace: fp.name
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
            exclusionMode: ExclusionMode.Ignore
            anchors { top: true; bottom: true; left: true; right: true }
            Loader {
                id: loader
                anchors.fill: parent
                focus: true
                source: fp.source
                onStatusChanged: if (status === Loader.Error) console.warn("firehold: " + fp.source + " couldn't load")
            }
            onVisibleChanged: if (visible && loader.item) loader.item.forceActiveFocus()
        }
    }
}

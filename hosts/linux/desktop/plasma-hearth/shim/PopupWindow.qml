// Quickshell's PopupWindow (plasma-hearth): a small window next to `anchor.item` (tips). A Wayland popup of the
// panel's own window, placed by Qt from the item's place.
import QtQuick
import QtQuick.Window

Window {
    id: pw
    readonly property AnchorInfo anchor: AnchorInfo {}
    flags: Qt.ToolTip | Qt.FramelessWindowHint | Qt.WindowDoesNotAcceptFocus | Qt.WindowTransparentForInput
    color: "transparent"
    width: Math.max(1, implicitWidth)
    height: Math.max(1, implicitHeight)
    transientParent: anchor.item ? anchor.item.Window.window : null
    function place() {
        if (!anchor.item) return;
        const g = anchor.item.mapToGlobal(anchor.rect.x, anchor.rect.y);
        x = Math.round(g.x); y = Math.round(g.y);
    }
    onVisibleChanged: if (visible) place()
}

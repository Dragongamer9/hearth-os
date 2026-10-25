// A scrolling area that keeps the selection in view (Focusable.revealed), smoothly, with room around it.
// Vertical by default; `horizontal: true` for a shelf of games.
import QtQuick

Flickable {
    id: sc
    readonly property bool hearthScroller: true
    property bool horizontal: false
    property real pad: Theme.u(48)
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    flickableDirection: horizontal ? Flickable.HorizontalFlick : Flickable.VerticalFlick
    contentWidth: horizontal ? contentItem.childrenRect.width : width
    pixelAligned: true

    NumberAnimation { id: anim; target: sc; duration: Theme.calm; easing.type: Easing.OutCubic }

    function ensureVisible(it) {
        if (!it || !it.visible) return;
        const p = it.mapToItem(sc.contentItem, 0, 0);
        if (horizontal) {
            let x = contentX;
            if (p.x - pad < contentX) x = p.x - pad;
            else if (p.x + it.width + pad > contentX + width) x = p.x + it.width + pad - width;
            x = Math.max(0, Math.min(x, Math.max(0, contentWidth - width)));
            if (Math.abs(x - contentX) > 1) { anim.stop(); anim.property = "contentX"; anim.to = x; anim.start(); }
        } else {
            let y = contentY;
            if (p.y - pad < contentY) y = p.y - pad;
            else if (p.y + it.height + pad > contentY + height) y = p.y + it.height + pad - height;
            y = Math.max(0, Math.min(y, Math.max(0, contentHeight - height)));
            if (Math.abs(y - contentY) > 1) { anim.stop(); anim.property = "contentY"; anim.to = y; anim.start(); }
        }
    }
    function toTop() { anim.stop(); contentY = 0; contentX = 0; }
}

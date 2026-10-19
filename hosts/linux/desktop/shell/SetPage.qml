// A scrolling settings page: title, then the page's rows.
import QtQuick
import QtQuick.Layouts
import QtQuick.Window

Flickable {
    id: page
    property string title
    default property alias rows: col.data
    contentHeight: col.implicitHeight + 48
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    Behavior on contentY { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutCubic } }
    function focusFirst() { start.nextItemInFocusChain(true).forceActiveFocus(); }

    // Lists that refresh by themselves (Wi-Fi networks, Bluetooth devices) rebuild their rows. Without this
    // the selected row disappears with the old ones and nothing has focus any more: the controller and the
    // arrow keys move nothing (stuck). Keeps the selection on the row with the same title, or the page.
    // Nothing changes when the new list is the same as the old one.
    function replaceList(name, list) {
        if (JSON.stringify(page[name]) === JSON.stringify(list)) return;
        const had = page.Window.activeFocusItem;
        let mine = false, key = "";
        for (let it = had; it; it = it.parent) {
            if (it === page) { mine = true; break; }
            if (key === "" && it.title !== undefined && typeof it.title === "string") key = it.title;
        }
        page[name] = list;
        if (!mine) return;
        Qt.callLater(() => {
            const now = page.Window.activeFocusItem;
            let still = false;
            for (let it = now; it; it = it.parent) if (it === page) { still = true; break; }
            if (still && now === had) return;
            const found = page.findRow(col, key);
            if (found) found.forceActiveFocus(); else page.focusFirst();
        });
    }
    function findRow(item, key) {
        if (!item || key === "") return null;
        const kids = item.children || [];
        for (let i = 0; i < kids.length; i++) {
            const k = kids[i];
            if (k.visible && k.activeFocusOnTab && k.title === key) return k;
            const deeper = page.findRow(k, key);
            if (deeper) return deeper;
        }
        return null;
    }

    ColumnLayout {
        id: col
        width: Math.min(page.width - 48, 820)
        x: 24
        y: 20
        spacing: 8
        Item { id: start; implicitHeight: 0 }
        Text { text: page.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 26; font.weight: Font.DemiBold; Layout.bottomMargin: 6 }
    }
}

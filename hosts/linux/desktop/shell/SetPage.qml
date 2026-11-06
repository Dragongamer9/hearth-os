// A scrolling settings page: its titled groups of cards. In Hearth Settings' frame (SettingsFrame.qml) the frame draws
// the breadcrumb title ("Home server › Services") and this page only its cards; on its own (the plain list
// SettingsSimple.qml falls back to) it shows its title itself.
//   part: one part of a page shown on its own (About's Privacy part in Privacy & security); "" shows every part.
//   footerLinks: [page ids] the frame adds at the end as cards (a category's other pages, under a landing page).
import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import "settings-pages.js" as Reg

Flickable {
    id: page
    property string title
    property string part: ""
    property var footerLinks: []
    property string footerTitle: ""   // the heading over footerLinks (settings-pages.js MORE); "More in <category>" if ""
    property bool framed: false       // set by the frame on the page it loads
    function inPart(p) { return part === "" || part === p; }
    default property alias rows: col.data
    contentHeight: col.implicitHeight + (footer.visible ? footer.implicitHeight + 10 : 0) + 56
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    Behavior on contentY { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutCubic } }
    function focusFirst() { const n = start.nextItemInFocusChain(true); if (n) n.forceActiveFocus(); }

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
    // The search's "open this setting": the card with this title (an expander around it opens first).
    function focusTitle(key) {
        const found = page.findRow(col, key) || page.findHidden(col, key);
        if (!found) return false;
        for (let it = found.parent; it && it !== page; it = it.parent) if (it.open === false && it.anchor !== undefined) it.open = true;
        Qt.callLater(() => found.forceActiveFocus());
        return true;
    }
    function findHidden(item, key) {
        const kids = item ? (item.children || []) : [];
        for (let i = 0; i < kids.length; i++) {
            if (kids[i].activeFocusOnTab && kids[i].title === key) return kids[i];
            const deeper = page.findHidden(kids[i], key);
            if (deeper) return deeper;
        }
        return null;
    }

    ColumnLayout {
        id: col
        width: Math.min(page.width - SetStyle.pageMargin * 2, SetStyle.contentMax)
        x: SetStyle.pageMargin
        y: page.framed ? 4 : 24
        spacing: 3
        Item { id: start; implicitHeight: 0 }
        Text { visible: !page.framed; text: page.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 28; font.weight: Font.DemiBold; Layout.bottomMargin: 10 }
    }
    // A category's other pages, after the landing page's own cards.
    ColumnLayout {
        id: footer
        visible: page.footerLinks.length > 0
        width: col.width
        x: col.x
        y: col.y + col.height + 10
        spacing: 3
        SetHeader { text: page.footerTitle !== "" ? page.footerTitle : "More in " + ((Reg.category((Reg.page(page.footerLinks[0] || "") || {}).cat || "") || {}).label || "this category") }
        Repeater {
            model: page.footerLinks
            delegate: SetLinkCard { required property string modelData; target: modelData }
        }
    }
}

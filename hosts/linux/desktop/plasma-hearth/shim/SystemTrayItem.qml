// An app's tray icon (plasma-hearth: hearth-tray-host, SystemTray.qml), as Quickshell's SystemTrayItem.
import QtQuick
QtObject {
    id: sti
    property string itemId: ""
    property string title: ""
    property string tooltipTitle: ""
    property string tooltipDescription: ""
    property var icon: ""
    property bool hasMenu: false
    property bool onlyMenu: false
    property var tray: null
    function activate() { if (tray) tray.op(itemId, "activate", 0, 0); }
    function secondaryActivate() { if (tray) tray.op(itemId, "secondary", 0, 0); }
    function scroll(delta, horizontal) { if (tray) tray.op(itemId, horizontal ? (delta > 0 ? "scroll-right" : "scroll-left") : (delta > 0 ? "scroll-up" : "scroll-down"), 0, 0); }
    function display(win, x, y) { if (tray) { if (hasMenu) tray.showMenu(sti, win, x, y); else tray.op(itemId, "context", x, y); } }
}

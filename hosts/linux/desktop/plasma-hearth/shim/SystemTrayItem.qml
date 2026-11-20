// An app's tray icon (plasma-hearth: Plasma's statusnotifieritem source, SystemTray.qml).
import QtQuick
QtObject {
    property string title: ""
    property string tooltipTitle: ""
    property string tooltipDescription: ""
    property var icon: ""
    property bool hasMenu: false
    property bool onlyMenu: false
    property var tray: null
    property string source: ""
    function activate() { if (tray) tray.op(source, "Activate"); }
    function secondaryActivate() { if (tray) tray.op(source, "SecondaryActivate"); }
    function scroll(delta, horizontal) { if (tray) tray.op(source, "Scroll", { delta: delta, direction: horizontal ? "Horizontal" : "Vertical" }); }
    function display(win, x, y) { if (tray) tray.op(source, "ContextMenu", { x: x, y: y }); }
}

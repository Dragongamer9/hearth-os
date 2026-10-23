pragma Singleton

// Shared by Hearth Settings' pages: focus back to the page list (Left / Esc / B), and the controller
// helper (Pad.qml) for the Controller page. Apps (AppStore.qml) uses the same pages and controls, so its
// own list registers here too: Left / Esc go back to the list of the window you're in.
import QtQuick
import Quickshell

Singleton {
    property var navTarget: null
    property var appsNav: null        // Apps' list (AppStore.qml)
    property var clipboard: null      // the clipboard history picker (Clipboard.qml), for Settings › Clipboard
    property var apps: null           // Apps (AppStore.qml), for links to it
    property var debs: null           // the Install Package window (DebInstaller.qml), for Settings › Apps
    property var debNav: null         // its page while it's open: Left / Esc (B) there close it
    property bool debShown: false
    property var pad: null
    property var help: null
    // Open windows that want the controller to type arrow keys / Enter / Esc (read by Pad.qml).
    property bool appsShown: false    // Apps
    property bool clipShown: false    // the clipboard history picker (Super+V)
    function toNav() {
        if (debNav && debNav.Window.active) { debNav.back(); return; }
        const t = appsNav && appsNav.Window.active ? appsNav : navTarget;
        if (t) t.forceActiveFocus();
    }
    function toast(t) { if (help) help.toast(t); }
}

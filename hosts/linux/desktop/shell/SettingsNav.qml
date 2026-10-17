pragma Singleton

// Shared by Hearth Settings' pages: focus back to the page list (Left / Esc / B), and the controller
// helper (Pad.qml) for the Controller page.
import QtQuick
import Quickshell

Singleton {
    property var navTarget: null
    property var pad: null
    property var help: null
    function toNav() { if (navTarget) navTarget.forceActiveFocus(); }
    function toast(t) { if (help) help.toast(t); }
}

pragma Singleton

// Hearth Settings' card style (the redesign, docs/MANTEL.md "Settings"): sizes and the few colours the looks don't
// have (good / needs a look), shared by SetBase, SetRow, the frame and its pages. Every other colour is a Theme token,
// so all eight looks (and light mode) work. Draw-only: never changes a setting.
import QtQuick
import Quickshell

Singleton {
    // The handheld layout (a 1280×800 screen or a small window): tighter padding, the same labels.
    property bool compact: false
    // 1.14, the new Settings: the controller look (layout B, SettingsFrameB.qml): big rows, ‹ value › choices, bigger
    // text. Set by the frame that's showing; the pages are the same files in both looks.
    property bool big: false
    readonly property int titleSize: big ? 18 : 15
    readonly property int subSize: big ? 14 : 13
    readonly property color ok: Theme.dark ? "#5bd18b" : "#1f8a4c"
    readonly property color warn: Theme.dark ? "#f5b84b" : "#a5640f"
    readonly property color card: Theme.panel2
    readonly property color cardHover: Qt.lighter(Theme.panel2, Theme.dark ? 1.12 : 0.96)
    readonly property color card2: Theme.panel3
    readonly property color sunk: Theme.bg2
    readonly property color nav: Theme.panel
    // Cards follow the look's corners, never sharper than 6 (Steam-style's crisp 4 reads as a table otherwise).
    readonly property real radius: Math.max(6, Theme.r(10))
    readonly property real bigRadius: Math.max(8, Theme.r(12))
    readonly property int cardHeight: big ? 84 : compact ? 62 : 66
    readonly property int pad: big ? 26 : compact ? 16 : 18
    readonly property int pageMargin: compact ? 26 : 44
    readonly property int contentMax: 1240
    function mix(a, b, t) { return Qt.rgba(a.r * (1 - t) + b.r * t, a.g * (1 - t) + b.g * t, a.b * (1 - t) + b.b * t, 1); }
    function tint(c, t) { return Qt.rgba(c.r, c.g, c.b, t); }
}

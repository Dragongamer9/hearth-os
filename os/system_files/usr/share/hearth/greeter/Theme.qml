pragma Singleton

// Hearth's login screen's colours: the fireside look of Hearth's SDDM theme and Mantel's Hearth look (dark, ember
// accent). The greeter runs before anyone signs in, so nobody's own look applies here. Theme.accent, accentInk and font
// are what Mantel's on-screen keyboard (OskBoard.qml, copied in when the image is built) asks for.
import QtQuick
import Quickshell

Singleton {
    property real scale: 1.0          // Bigger text (the accessibility menu)
    property bool contrast: false     // High contrast (the accessibility menu)
    readonly property color bg: "#0f0b09"
    readonly property color bg2: "#17110e"
    readonly property color panel: contrast ? "#000000" : "#1c1512"
    readonly property color panel2: contrast ? "#000000" : "#261d18"
    readonly property color panel3: contrast ? "#2b2b2b" : "#33271f"
    readonly property color line: contrast ? "#ffffff" : "#3b2e26"
    readonly property color text: "#f4ece3"
    readonly property color muted: contrast ? "#f4ece3" : "#a69484"
    readonly property color danger: "#ff6b6b"
    readonly property color warn: "#ffd166"
    readonly property color accent: "#ff8a3d"
    readonly property color accentInk: "#2a1206"
    readonly property string font: Qt.fontFamilies().indexOf("Inter") >= 0 ? "Inter" : "sans-serif"
    function px(n) { return Math.round(n * scale); }
}

pragma Singleton

// The Hearth palette: warm, firelit darks with an ember accent.
// The accent follows your profile when ~/.local/share/nimbus/library.json sets one.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    readonly property color bg: "#0f0b09"
    readonly property color bg2: "#17110e"
    readonly property color panel: "#1c1512"
    readonly property color panel2: "#261d18"
    readonly property color panel3: "#33271f"
    readonly property color line: "#3b2e26"
    readonly property color text: "#f4ece3"
    readonly property color muted: "#a69484"
    readonly property color danger: "#ff5d5d"
    readonly property color ember: "#ff8a3d"
    readonly property color flame: "#ffd166"
    property color accent: ember
    // Text on an accent background: dark on light accents, white on dark ones.
    readonly property color accentInk: (accent.r * 0.2126 + accent.g * 0.7152 + accent.b * 0.0722) > 0.45 ? "#2a1206" : "#ffffff"
    // Frosted panels over the wallpaper.
    readonly property color glass: Qt.rgba(panel.r, panel.g, panel.b, 0.92)

    // Slow, calm motion everywhere.
    readonly property int quick: 140
    readonly property int calm: 260

    readonly property string font: "sans-serif"
    readonly property int radius: 14

    FileView {
        path: Quickshell.env("HOME") + "/.local/share/nimbus/library.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            try {
                const a = JSON.parse(text()).profile?.accent;
                if (/^#[0-9a-fA-F]{6}$/.test(a || "")) accent = a;
            } catch (e) {}
        }
    }
}

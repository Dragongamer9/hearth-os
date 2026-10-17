pragma Singleton

// The Nimbus palette, the same colors as Game Mode (ui/index.html :root).
// The accent follows your Nimbus profile when ~/.local/share/nimbus/library.json sets one.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    readonly property color bg: "#0c1016"
    readonly property color bg2: "#121821"
    readonly property color panel: "#161d27"
    readonly property color panel2: "#1d2531"
    readonly property color panel3: "#273140"
    readonly property color line: "#2a3443"
    readonly property color text: "#e9eef4"
    readonly property color muted: "#7f8b99"
    readonly property color danger: "#ff5d5d"
    property color accent: "#3fd0b8"
    // Text on an accent background: dark on light accents, white on dark ones.
    readonly property color accentInk: (accent.r * 0.2126 + accent.g * 0.7152 + accent.b * 0.0722) > 0.45 ? "#06261f" : "#ffffff"

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

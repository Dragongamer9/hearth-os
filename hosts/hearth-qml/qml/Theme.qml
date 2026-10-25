pragma Singleton
// Hearth's Game Mode look, the same as the web page's (ui/index.html :root and THEMES): the background theme and the
// accent follow your profile (profile.get). Sizes scale with the window (800 px tall is 1.0) and Settings › Display ›
// size (the core's "zoom").
import QtQuick

QtObject {
    id: t
    // [key, name, bg, bg2, panel, panel2, panel3, line], as the web page's THEMES
    readonly property var themes: [
        ["default", "Midnight", "#0c1016", "#121821", "#161d27", "#1d2531", "#273140", "#2a3443"],
        ["preset:1", "Slate", "#101215", "#16191d", "#1b1f24", "#23282e", "#2e343c", "#30363e"],
        ["preset:2", "Plum", "#130f1a", "#191424", "#1f1830", "#271f3b", "#33294b", "#382d4a"],
        ["preset:3", "Forest", "#0b1411", "#101b17", "#15221d", "#1b2b25", "#243830", "#28392f"],
        ["preset:4", "Ember", "#160f0d", "#1e1512", "#251a16", "#2f221d", "#3d2c25", "#40302a"],
        ["preset:5", "Black", "#000000", "#07080a", "#0e1014", "#161a20", "#20252d", "#242a33"]
    ]
    property color bg: "#0c1016"
    property color bg2: "#121821"
    property color panel: "#161d27"
    property color panel2: "#1d2531"
    property color panel3: "#273140"
    property color line: "#2a3443"
    readonly property color text: "#e9eef4"
    readonly property color dim: "#8d99a8"
    readonly property color faint: "#5d6978"
    readonly property color muted: dim
    property color accent: "#3fd0b8"
    property color accent2: "#2aa892"
    property color accentInk: "#06231e"
    readonly property color warn: "#ffb547"
    readonly property color danger: "#ff6b6b"
    readonly property var sourceNames: ({ steam: "Steam", epic: "Epic", gog: "GOG", xbox: "Xbox", manual: "Added by you", beacon: "Beacon", emu: "Emulators", flatpak: "Flathub" })

    readonly property string font: "sans-serif"
    // Anybody for headings when it's built in (main.cpp), else the body face, bold.
    readonly property string display: (typeof hostApp !== "undefined" && hostApp.displayFont) ? hostApp.displayFont : font

    // Window height / 800, times Settings › Display › size.
    property real base: 1.0
    property real zoom: 1.0
    readonly property real s: Math.max(0.75, Math.min(2.2, base * zoom))
    function u(v) { return Math.round(v * s); }
    readonly property int radius: u(10)
    readonly property int gut: u(32)
    readonly property int quick: 140
    readonly property int calm: 240

    function hex(c) { return c; }
    function applyProfile(p) {
        if (!p) return;
        const acc = /^#[0-9a-fA-F]{6}$/.test(p.accent || "") ? p.accent : "#3fd0b8";
        const n = parseInt(acc.slice(1), 16), r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
        accent = acc;
        accent2 = Qt.rgba(r * 0.8 / 255, g * 0.8 / 255, b * 0.8 / 255, 1);
        const lum = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
        accentInk = lum > 0.45 ? Qt.rgba(r * 0.12 / 255, g * 0.12 / 255, b * 0.12 / 255, 1) : "#ffffff";
        const th = themes.find(x => x[0] === p.background) || themes[0];
        bg = th[2]; bg2 = th[3]; panel = th[4]; panel2 = th[5]; panel3 = th[6]; line = th[7];
    }
    function alpha(c, a) { const q = Qt.color(c); return Qt.rgba(q.r, q.g, q.b, a); }
}

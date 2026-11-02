pragma Singleton

// Mantel's palette. Settings › Appearance picks the look (Settings.look: "hearth" = warm, firelit darks with an
// ember accent; "steam" = Steam-style deep blue-slate, flat panels, square-ish corners, a bright blue accent),
// dark or light (Settings.mode) and the accent (Settings.accent: a colour, or "" for the look's own; with the
// Hearth look your profile's accent from ~/.local/share/nimbus/library.json is used then, as before).
// The colours come from ../theme/palettes.json (shared with theme/hearth-theme, which gives labwc's window
// decorations and Qt / GTK / KDE apps the same look); the Hearth dark values below are used until it's read.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root
    property var palettes: ({})
    readonly property string look: palettes[Settings.look] ? Settings.look : "hearth"
    readonly property string mode: Settings.mode === "light" ? "light" : "dark"
    readonly property bool dark: mode === "dark"
    readonly property bool steam: look === "steam"
    readonly property var lookDef: palettes[look] || ({})
    readonly property var p: lookDef[mode] || ({})
    function c(k, d) { return p[k] || d; }

    readonly property color bg: c("bg", "#0f0b09")
    readonly property color bg2: c("bg2", "#17110e")
    readonly property color panel: c("panel", "#1c1512")
    readonly property color panel2: c("panel2", "#261d18")
    readonly property color panel3: c("panel3", "#33271f")
    readonly property color line: c("line", "#3b2e26")
    readonly property color text: c("text", "#f4ece3")
    readonly property color muted: c("muted", "#a69484")
    readonly property color danger: c("danger", "#ff5d5d")
    readonly property color ember: c("ember", "#ff8a3d")       // the look's own accent
    readonly property color flame: c("flame", "#ffd166")       // its second, brighter highlight
    property string profileAccent: ""
    readonly property bool validAccent: /^#[0-9a-fA-F]{6}$/.test(Settings.accent || "")
    readonly property color accent: validAccent ? Settings.accent : (!steam && profileAccent) ? profileAccent : ember
    // Text on an accent background: dark on light accents, white on dark ones.
    readonly property color accentInk: (accent.r * 0.2126 + accent.g * 0.7152 + accent.b * 0.0722) > 0.45 ? ((lookDef.dark || {}).ink || "#2a1206") : "#ffffff"
    // Frosted panels over the wallpaper (Steam-style: nearly solid, flat).
    readonly property color glass: Qt.rgba(panel.r, panel.g, panel.b, p.glass || 0.92)
    // A controller focus ring: crisp, in the accent.
    readonly property int focusWidth: 2

    // Slow, calm motion everywhere.
    readonly property int quick: 140
    readonly property int calm: 260

    readonly property string font: lookDef.font || "sans-serif"
    readonly property int radius: r(14)
    // A corner radius in this look: Hearth's soft corners as they are, Steam-style ones capped at 2–4 px.
    function r(n) { const cap = lookDef.corner || 0; return cap > 0 ? Math.min(n, cap) : n; }

    FileView {
        // (Qt.resolvedUrl can't reach outside the shell's folder: Quickshell answers qrc:/qs-blackhole there.)
        path: Quickshell.shellDir + "/../theme/palettes.json"
        blockLoading: true
        onLoaded: { try { root.palettes = JSON.parse(text()); } catch (e) { console.warn("Theme: palettes.json: " + e); } }
    }

    // Apps and window decorations follow (theme/apply.sh runs theme/hearth-theme, which rewrites the Qt / GTK / KDE
    // colours and labwc's theme and asks labwc to reload). Once things settle, not on every step of a change.
    readonly property string key: look + "/" + mode + "/" + accent
    property string appliedKey: ""
    onKeyChanged: if (Settings.loaded) applyTimer.restart()
    Timer {
        id: applyTimer
        interval: 600
        onTriggered: {
            if (root.appliedKey === "" ) { root.appliedKey = root.key; return; }   // the first settle: labwc's autostart already ran it
            if (root.appliedKey === root.key) return;
            root.appliedKey = root.key;
            Quickshell.execDetached(["sh", Quickshell.shellDir + "/../theme/apply.sh"]);
        }
    }
    Connections {
        target: Settings
        function onLoadedChanged() { if (Settings.loaded) applyTimer.restart(); }
    }

    FileView {
        path: Quickshell.env("HOME") + "/.local/share/nimbus/library.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            try {
                const a = JSON.parse(text()).profile?.accent;
                if (/^#[0-9a-fA-F]{6}$/.test(a || "")) root.profileAccent = a;
            } catch (e) {}
        }
    }
}

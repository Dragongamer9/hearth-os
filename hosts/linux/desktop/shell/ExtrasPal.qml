// Colours for the volume mixer, snap layouts and clipboard popups (Hearth 1.10). In Mantel's Steam-style dark look
// they are the approved mockups' dark blues; in every other look they follow that look (Theme.qml), so the popups
// always match the rest of the desktop. Your accent is always your accent.
import QtQuick

QtObject {
    readonly property bool blue: Theme.steam && Theme.dark
    readonly property color card: blue ? "#1e2f42" : Theme.panel        // the popup
    readonly property color edge: blue ? "#3a5875" : Theme.line         // its border
    readonly property color box: blue ? "#213347" : Theme.panel2        // a group inside it
    readonly property color line: blue ? "#2f4a63" : Theme.line         // borders, slider tracks
    readonly property color well: blue ? "#10151b" : Theme.bg           // inputs, the zone previews
    readonly property color sel: blue ? "#2a475e" : Theme.panel3        // the selected row
    readonly property color zone: blue ? "#3a5875" : Theme.panel3       // a snap zone not lit
    readonly property color text: blue ? "#e6eef5" : Theme.text
    readonly property color muted: blue ? "#9fb2c3" : Theme.muted
    readonly property color off: blue ? "#5d7488" : Theme.muted         // a muted app's slider
    readonly property color warn: blue ? "#ff9c8f" : Theme.danger       // "Muted"
    readonly property color accent: Theme.accent
    readonly property color dim: "#99060a0f"                            // behind a popup
    readonly property string font: Theme.font
    readonly property string mono: "monospace"
}

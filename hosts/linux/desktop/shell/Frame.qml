// An island's background in the look's own style (Mantel 2026; theme/palettes.json "frame"): plain (rounded,
// translucent), chamfer (Gaming's cut corners with a neon edge), neon (Synthwave's glowing edge), wood (Cabin's
// grain) or frost (Glass: a blurred copy of the wallpaper under a milky fill, Frost.qml). Scanlines on top for looks
// that have them. Used by the islands, the dock and the big panels. Nothing here takes clicks.
import QtQuick

Item {
    id: f
    property real rad: Theme.r(18)
    property bool lit: false               // what it opens is open: an accent edge
    property bool flat: false              // part of a full-width bar (draws nothing of its own)
    property bool solid: false             // a big panel (Quick settings, the launcher): a little less see-through
    readonly property string kind: flat ? "none" : Theme.frame
    readonly property color edge: lit ? Theme.accent : Theme.line
    readonly property color fill: solid ? Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, Math.max(Theme.glassAlpha, 0.9)) : Theme.glass

    // plain, neon and the frost's milky fill
    Rectangle {
        anchors.fill: parent
        visible: f.kind === "plain" || f.kind === "neon"
        radius: f.rad
        color: f.fill
        border.width: f.kind === "neon" ? 1.5 : 1
        border.color: f.kind === "neon" ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, f.lit ? 1 : 0.8) : f.edge
    }
    Repeater {                             // neon: a soft glow outside the edge
        model: f.kind === "neon" ? 3 : 0
        delegate: Rectangle {
            required property int index
            anchors.fill: parent
            anchors.margins: -(index + 1) * 2
            radius: f.rad + (index + 1) * 2
            color: "transparent"
            border.width: 2
            border.color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.22 - index * 0.06)
        }
    }

    // frost (Glass): the wallpaper behind, blurred where the GPU allows, then a milky fill and a light top edge
    Loader {
        anchors.fill: parent
        active: f.kind === "frost"
        source: "Frost.qml"
        onLoaded: item.rad = Qt.binding(() => f.rad)
    }
    Rectangle {
        anchors.fill: parent
        visible: f.kind === "frost"
        radius: f.rad
        color: Theme.dark ? Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, f.solid ? 0.78 : Theme.glassAlpha)
                          : Qt.rgba(1, 1, 1, f.solid ? 0.8 : Theme.glassAlpha)
        border.width: 1
        border.color: f.lit ? Theme.accent : Qt.rgba(1, 1, 1, Theme.dark ? 0.18 : 0.7)
        Rectangle { x: f.rad; width: parent.width - 2 * f.rad; height: 1; y: 1; color: Qt.rgba(1, 1, 1, 0.45) }
    }

    // wood (Cabin)
    Rectangle {
        anchors.fill: parent
        visible: f.kind === "wood"
        radius: f.rad
        clip: true
        border.width: 1
        border.color: f.lit ? Theme.accent : (Theme.dark ? "#1a0f08" : "#8a6440")
        gradient: Gradient {
            GradientStop { position: 0; color: Theme.dark ? "#4a2f1b" : "#d8b48a" }
            GradientStop { position: 0.55; color: Theme.dark ? "#3a2414" : "#c99f72" }
            GradientStop { position: 1; color: Theme.dark ? "#2b1a0e" : "#b98b5d" }
        }
        Repeater {
            model: Math.max(2, Math.floor(f.height / 7))
            delegate: Rectangle {
                required property int index
                x: (index * 37) % 23; y: 3 + index * 7
                width: f.width - x - (index * 17) % 31; height: 1
                radius: 1
                color: Theme.dark ? Qt.rgba(0, 0, 0, 0.22 + (index % 3) * 0.06) : Qt.rgba(0.35, 0.2, 0.08, 0.16 + (index % 3) * 0.05)
            }
        }
    }

    // chamfer (Gaming): cut corners and a neon edge
    Canvas {
        id: cham
        anchors.fill: parent
        visible: f.kind === "chamfer"
        readonly property string key: f.fill + "/" + f.edge + "/" + Theme.accent + "/" + width + "x" + height
        onKeyChanged: requestPaint()
        onVisibleChanged: if (visible) requestPaint()
        onPaint: {
            const c = getContext("2d"), w = width, h = height, k = Math.min(12, h / 4);
            c.reset();
            if (w <= 2 || h <= 2) return;
            c.beginPath();
            c.moveTo(k, 1); c.lineTo(w - 1, 1); c.lineTo(w - 1, h - k); c.lineTo(w - k, h - 1); c.lineTo(1, h - 1); c.lineTo(1, k); c.closePath();
            c.fillStyle = Theme.css(f.fill);
            c.fill();
            c.lineWidth = 1.5;
            c.strokeStyle = Theme.css(Theme.accent, f.lit ? 1 : 0.55);
            c.stroke();
        }
    }

    // Scanlines (Synthwave)
    Canvas {
        id: lines
        anchors.fill: parent
        visible: Theme.scanlines && f.kind !== "none"
        opacity: 0.5
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onVisibleChanged: if (visible) requestPaint()
        onPaint: {
            const c = getContext("2d");
            c.reset();
            c.fillStyle = "rgba(0,0,0,0.18)";
            for (let y = 2; y < height - 2; y += 3) c.fillRect(f.rad / 2, y, width - f.rad, 1);
        }
    }
}

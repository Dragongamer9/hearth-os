// A game's picture on a tile or the hero: the game's own art (Hearth's cover, wide or hero picture) when it has one,
// else its two colours with hills and a moon, as the mockups draw a game without art. Rounded by the caller (clip).
import QtQuick
import QtQuick.Window
import QtQuick.Shapes

Item {
    id: art
    property var game: null
    property string want: "wide"          // wide | hero | cover
    property bool motif: true
    readonly property var pairs: [["#3b2a5a", "#c0562a"], ["#24324a", "#8a5a2a"], ["#3a3a24", "#a8742a"], ["#4a2020", "#d08a3a"],
                                  ["#202a3a", "#5a7a9a"], ["#16283a", "#3aa0a0"], ["#2a3a2a", "#c09050"], ["#2a2a20", "#d0b030"],
                                  ["#2a1a2a", "#9a3a4a"], ["#2a1f14", "#c07a2a"], ["#2a1030", "#d04a6a"], ["#1e2a22", "#8a9a5a"],
                                  ["#141a2a", "#5a7aaa"], ["#2a2a30", "#d07a3a"], ["#1a1a24", "#6a5aa0"], ["#22302a", "#a0a060"]]
    function hash(s) { let h = 7; s = String(s || ""); for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 100003; return h; }
    readonly property int seed: hash(game ? (game.title || game.id) : "")
    readonly property var tones: game && game.tones && game.tones.length === 2 ? game.tones : pairs[seed % pairs.length]
    readonly property string src: {
        if (!game) return "";
        const order = want === "hero" ? [game.hero, game.wide, game.cover] : want === "cover" ? [game.cover, game.wide, game.hero] : [game.wide, game.hero, game.cover];
        for (const s of order) if (s) return String(s);
        return "";
    }
    readonly property bool hasArt: src !== "" && img.status === Image.Ready

    Shape {
        anchors.fill: parent
        visible: !art.hasArt
        ShapePath {
            strokeWidth: -1
            fillGradient: LinearGradient {
                x1: 0; y1: 0; x2: art.width; y2: art.height
                GradientStop { position: 0; color: art.tones[0] }
                GradientStop { position: 1; color: art.tones[1] }
            }
            startX: 0; startY: 0
            PathLine { x: art.width; y: 0 }
            PathLine { x: art.width; y: art.height }
            PathLine { x: 0; y: art.height }
            PathLine { x: 0; y: 0 }
        }
    }
    // Hills and a moon (the mockups' "ridge"), only where there's no art.
    Item {
        anchors.fill: parent
        visible: !art.hasArt && art.motif
        readonly property real sx: width / 400
        readonly property real sy: height / 225
        readonly property real m: (art.seed % 7) / 7
        Rectangle {
            x: (80 + parent.m * 240 - 30) * parent.sx; y: (60 - 30) * parent.sy
            width: 60 * parent.sx; height: width; radius: width / 2
            color: "#38ffdcaa"
        }
        Shape {
            anchors.fill: parent
            ShapePath {
                strokeWidth: -1; fillColor: "#47000000"
                startX: 0; startY: 225 * art.height / 225
                PathLine { x: 0; y: 150 * art.height / 225 }
                PathLine { x: 90 * art.width / 400; y: (90 + art.seed % 40) * art.height / 225 }
                PathLine { x: 180 * art.width / 400; y: 160 * art.height / 225 }
                PathLine { x: 280 * art.width / 400; y: (80 + (art.seed >> 3) % 40) * art.height / 225 }
                PathLine { x: art.width; y: 150 * art.height / 225 }
                PathLine { x: art.width; y: art.height }
            }
            ShapePath {
                strokeWidth: -1; fillColor: "#73000000"
                startX: 0; startY: art.height
                PathLine { x: 0; y: 190 * art.height / 225 }
                PathLine { x: 140 * art.width / 400; y: 165 * art.height / 225 }
                PathLine { x: 300 * art.width / 400; y: 195 * art.height / 225 }
                PathLine { x: art.width; y: 175 * art.height / 225 }
                PathLine { x: art.width; y: art.height }
            }
        }
    }
    Image {
        id: img
        anchors.fill: parent
        visible: art.hasArt
        // (1.16: only while it can be seen, a hidden tab or page holds no pictures; decoded at the size it's drawn)
        source: art.src === "" || !art.visible ? "" : (art.src.indexOf("/") === 0 ? "file://" + art.src : art.src)
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        sourceSize.width: Math.min(1600, Math.ceil(art.width * Screen.devicePixelRatio))
    }
}

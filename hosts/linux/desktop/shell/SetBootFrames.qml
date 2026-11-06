// A flip book for Settings › Boot & startup's previews: plays one of the frame sheets tools/make-boot-previews.py
// makes (frames side by side, `columns` to a row) by sliding the sheet behind a window the size of one frame. Plain
// QtQuick: no video player, no Plymouth. Stopped, it shows the poster frame. Looking at it changes nothing.
import QtQuick

Item {
    id: fb
    property string sheet: ""           // the .jpg (absolute path); "" shows the empty slate
    property int frames: 1
    property int columns: 8
    property int frameWidth: 400
    property int frameHeight: 225
    property int fps: 10
    property int poster: 0
    property bool playing: false
    property int frame: poster
    readonly property bool ready: img.status === Image.Ready
    readonly property int rows: Math.max(1, Math.ceil(frames / Math.max(1, columns)))
    readonly property real sx: width / Math.max(1, frameWidth)
    readonly property real sy: height / Math.max(1, frameHeight)
    implicitWidth: frameWidth
    implicitHeight: frameHeight
    clip: true

    onPlayingChanged: if (!playing) frame = poster
    onPosterChanged: if (!playing) frame = poster
    onSheetChanged: frame = poster

    Rectangle { anchors.fill: parent; color: "#0b1016" }
    Image {
        id: img
        source: fb.sheet !== "" ? "file://" + fb.sheet : ""
        asynchronous: true
        cache: false
        smooth: true
        mipmap: fb.sx < 0.75
        // Decoded at the size shown (a small card doesn't hold the whole sheet in memory), never above its own.
        sourceSize.width: Math.round(fb.columns * fb.frameWidth * Math.min(1, fb.sx))
        sourceSize.height: Math.round(fb.rows * fb.frameHeight * Math.min(1, fb.sy))
        width: fb.columns * fb.frameWidth * fb.sx
        height: fb.rows * fb.frameHeight * fb.sy
        x: -(fb.frame % Math.max(1, fb.columns)) * fb.frameWidth * fb.sx
        y: -Math.floor(fb.frame / Math.max(1, fb.columns)) * fb.frameHeight * fb.sy
    }
    Timer {
        interval: Math.round(1000 / Math.max(1, fb.fps))
        repeat: true
        running: fb.playing && fb.visible && fb.ready && fb.frames > 1
        onTriggered: fb.frame = (fb.frame + 1) % fb.frames
    }
}

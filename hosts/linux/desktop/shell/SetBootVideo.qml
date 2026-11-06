// The full-quality startup animation for Settings › Boot & startup's preview: the WebM itself through QtMultimedia
// (qml6-module-qtmultimedia). Loaded only when "Full video" is asked for, through a Loader: without QtMultimedia the
// Loader fails quietly and the flip book (SetBootFrames.qml) stays. `showing` is true only once frames really arrive,
// so a player that can't decode VP9 never leaves a black box over the flip book. Muted (the animations are silent).
import QtQuick
import QtMultimedia

Item {
    id: v
    property string file: ""
    readonly property bool showing: player.playbackState === MediaPlayer.PlayingState && player.hasVideo && player.position > 0
    readonly property bool failed: player.error !== MediaPlayer.NoError
    MediaPlayer {
        id: player
        source: v.file !== "" ? "file://" + v.file : ""
        loops: MediaPlayer.Infinite
        videoOutput: out
    }
    VideoOutput {
        id: out
        anchors.fill: parent
        fillMode: VideoOutput.PreserveAspectFit
        opacity: v.showing ? 1 : 0
    }
    Component.onCompleted: if (v.file !== "") player.play()
    Component.onDestruction: player.stop()
}

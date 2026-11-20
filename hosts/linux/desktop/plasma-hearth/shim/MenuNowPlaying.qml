// What's playing, for the menus' Now playing tile (plasma-hearth: Mpris.qml, Plasma's media players).
import QtQuick
Item {
    readonly property var player: Mpris.players.values.find(p => p.isPlaying) || Mpris.players.values[0] || null
    readonly property bool has: player !== null && (player.trackTitle || "") !== ""
    readonly property bool playing: player !== null && player.isPlaying
    readonly property string title: player ? (player.trackTitle || "") : ""
    readonly property string artist: player ? (player.trackArtist || "") : ""
    readonly property string app: player ? (player.identity || "") : ""
    readonly property string art: player ? (player.trackArtUrl || "") : ""
    function toggle() { if (player) player.togglePlaying(); }
    function next() { if (player) player.next(); }
}

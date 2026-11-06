// What's playing (MPRIS), for the app menus' Now playing tile (MenuTile.qml). Loaded on its own, so a Quickshell
// without the Mpris service only loses this tile's music (it shows its plain icon then).
import QtQuick
import Quickshell
import Quickshell.Services.Mpris

Item {
    readonly property var player: Mpris.players.values.find(p => p.isPlaying) || Mpris.players.values[0] || null
    readonly property bool has: player !== null && (player.trackTitle || "") !== ""
    readonly property bool playing: player !== null && player.isPlaying
    readonly property string title: player ? (player.trackTitle || "") : ""
    readonly property string artist: player ? (player.trackArtist || "") : ""
    readonly property string app: player ? (player.identity || "") : ""
    readonly property string art: player ? (player.trackArtUrl || "") : ""
    function toggle() { if (player && player.canTogglePlaying) player.togglePlaying(); }
    function next() { if (player && player.canGoNext) player.next(); }
}

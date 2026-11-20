// What's playing, for the shared Couch's "Playing on the TV" card (MediaCard.qml) under Firehold on labwc:
// Quickshell's MPRIS, in the names PlasmaMedia.qml gives under Plasma: cp (album, track, artist, identity,
// desktopEntry, artUrl, position and length in microseconds), has, playing, playPause(), next(), updatePosition().
// Loaded on its own, so a Quickshell without the MPRIS service only loses this card.
import QtQuick
import Quickshell.Services.Mpris

Item {
    readonly property var player: Mpris.players.values.find(p => p.isPlaying) || Mpris.players.values.find(p => p.trackTitle) || null
    readonly property bool playing: !!player && player.isPlaying
    readonly property bool has: !!player && String(player.trackTitle || "") !== "" && player.playbackState !== MprisPlaybackState.Stopped
    readonly property QtObject cp: QtObject {
        readonly property string album: player ? String(player.trackAlbum || "") : ""
        readonly property string track: player ? String(player.trackTitle || "") : ""
        readonly property string artist: player ? String(player.trackArtist || "") : ""
        readonly property string identity: player ? String(player.identity || "") : ""
        readonly property string desktopEntry: player ? String(player.desktopEntry || "") : ""
        readonly property string artUrl: player ? String(player.trackArtUrl || "") : ""
        property real position: player && player.positionSupported ? player.position * 1e6 : 0
        readonly property real length: player && player.lengthSupported ? player.length * 1e6 : 0
    }
    function playPause() { if (player && player.canTogglePlaying) player.togglePlaying(); }
    function next() { if (player && player.canGoNext) player.next(); }
    // (when the Couch looks at its cards again, no timer: the owner's rule on lag)
    function updatePosition() { if (player) cp.position = player.position * 1e6; }
}

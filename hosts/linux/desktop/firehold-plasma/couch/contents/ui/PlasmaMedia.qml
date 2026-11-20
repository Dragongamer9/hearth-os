// What's playing, from Plasma's own media model (MPRIS, as Plasma's media controller has it): MediaCard.qml's player
// under Plasma. The same names as desktop/shell/FhMedia.qml (Firehold on labwc): cp (album, track, artist, identity,
// desktopEntry, artUrl, position and length in microseconds), has, playing, playPause(), next(), updatePosition().
import QtQuick
import org.kde.plasma.private.mpris as Mpris

Item {
    Mpris.Mpris2Model { id: mpris }
    readonly property var cp: mpris.currentPlayer
    readonly property bool playing: !!cp && cp.playbackStatus === Mpris.PlaybackStatus.Playing
    readonly property bool has: !!cp && String(cp.track || "") !== "" && cp.playbackStatus !== Mpris.PlaybackStatus.Stopped
    function playPause() { if (cp) cp.PlayPause(); }
    function next() { if (cp) cp.Next(); }
    function updatePosition() { if (cp) cp.updatePosition(); }
}

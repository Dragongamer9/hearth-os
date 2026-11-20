// Plasma's media players (org.kde.plasma.private.mpris), for Mpris.qml (plasma-hearth), in Quickshell's shape for
// Mantel's MediaPill.qml (the clock pill), MenuNowPlaying.qml (the menus' Now playing tile) and MediaCard.qml (the
// dashboard's card): seconds for the position and length, as Quickshell gives them (Plasma's are microseconds).
// Nothing polls: Plasma's player tells when the track or its state changes; the card asks for the position itself
// (positionChanged(), once a second while it's shown and playing, as on Mantel).
import QtQuick
import org.kde.plasma.private.mpris as MprisPkg

Item {
    MprisPkg.Mpris2Model { id: mpris }
    readonly property var cur: mpris.currentPlayer
    readonly property var list: cur && (cur.track || "") !== "" ? [{
        isPlaying: cur.playbackStatus === MprisPkg.PlaybackStatus.Playing, trackTitle: cur.track || "", trackArtist: cur.artist || "",
        identity: cur.identity || "", trackArtUrl: cur.artUrl ? String(cur.artUrl) : "",
        canControl: !!cur.canControl, canTogglePlaying: !!cur.canControl && (!!cur.canPlay || !!cur.canPause),
        canGoNext: !!cur.canGoNext, canGoPrevious: !!cur.canGoPrevious,
        length: Number(cur.length || 0) / 1e6, lengthSupported: Number(cur.length || 0) > 0,
        position: Number(cur.position || 0) / 1e6, positionSupported: !!cur.canSeek || Number(cur.position || 0) > 0,
        positionChanged: function () { if (cur.updatePosition) cur.updatePosition(); },
        togglePlaying: function () { cur.PlayPause(); }, next: function () { cur.Next(); }, previous: function () { cur.Previous(); }
    }] : []
}

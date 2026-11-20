// Plasma's media players (org.kde.plasma.private.mpris), for Mpris.qml (plasma-hearth).
import QtQuick
import org.kde.plasma.private.mpris as Mpris

Item {
    Mpris.Mpris2Model { id: mpris }
    readonly property var cur: mpris.currentPlayer
    readonly property var list: cur && (cur.track || "") !== "" ? [{
        isPlaying: cur.playbackStatus === Mpris.PlaybackStatus.Playing, trackTitle: cur.track || "", trackArtist: cur.artist || "",
        identity: cur.identity || "", trackArtUrl: cur.artUrl ? String(cur.artUrl) : "", canTogglePlaying: true, canGoNext: cur.canGoNext,
        togglePlaying: function () { cur.PlayPause(); }, next: function () { cur.Next(); }
    }] : []
}

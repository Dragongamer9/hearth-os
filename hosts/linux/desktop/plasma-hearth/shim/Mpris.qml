pragma Singleton
// What's playing for Mantel's bar and menus (plasma-hearth): Plasma's media players (MprisPlayers.qml, loaded on
// its own). players.values: {isPlaying, trackTitle, trackArtist, identity, trackArtUrl, togglePlaying(), next()}.
import QtQuick
Item {
    Loader { id: m; source: "MprisPlayers.qml" }
    readonly property var players: ({ values: m.status === Loader.Ready && m.item ? m.item.list : [] })
}

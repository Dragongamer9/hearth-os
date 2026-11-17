// Media player widget: what's playing in any app that says so (MPRIS: Spotify, browsers, Hearth Player…), its
// picture, a position bar, and previous / play-pause / next. With more than one player, ‹ › picks which one.
// HEARTH_WIDGETS_FIXTURES/media.json stands in for a player in the Desktop Mode test.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris

WidgetBase {
    id: w
    property int which: 0
    property var fix: null
    FileView { path: w.fixtures ? w.fixtures + "/media.json" : ""; blockLoading: true; onLoaded: { try { w.fix = JSON.parse(text()); } catch (e) {} } }
    readonly property var players: Mpris.players.values
    readonly property var player: players.length ? (players[which % players.length] || null) : null
    onPlayersChanged: { const i = players.findIndex(p => p.isPlaying); if (i >= 0) which = i; }
    readonly property bool has: player !== null || fix !== null
    readonly property string title: fix ? fix.title : player ? (player.trackTitle || player.identity || "") : ""
    readonly property string artist: fix ? fix.artist : player ? [player.trackArtist, player.identity].filter(Boolean).join(" · ") : ""
    readonly property string art: fix ? (fix.art || "") : player ? (player.trackArtUrl || "") : ""
    readonly property bool playing: fix ? fix.playing === true : player !== null && player.isPlaying
    readonly property real pos: fix ? fix.position : player && player.positionSupported ? player.position : 0
    readonly property real len: fix ? fix.length : player && player.lengthSupported ? player.length : 0
    implicitWidth: 380; implicitHeight: 160
    function fmt(s) { s = Math.max(0, Math.floor(s || 0)); return Math.floor(s / 60) + ":" + ("0" + (s % 60)).slice(-2); }
    Timer { interval: 1000; running: w.live && w.player !== null && w.playing; repeat: true; onTriggered: if (w.player && w.player.positionSupported) w.player.positionChanged() }

    RowLayout {
        visible: w.has
        anchors { fill: parent; margins: 14 }
        spacing: 14
        Rectangle {
            Layout.preferredWidth: Math.min(parent.height, 132); Layout.preferredHeight: Layout.preferredWidth
            radius: Theme.r(12)
            clip: true
            gradient: Gradient { GradientStop { position: 0; color: Theme.flame } GradientStop { position: 1; color: Theme.accent } }
            Icon { anchors.centerIn: parent; name: "headphones"; size: parent.width * 0.4; color: Theme.accentInk; visible: cover.status !== Image.Ready }
            Image { id: cover; anchors.fill: parent; source: w.art; fillMode: Image.PreserveAspectCrop; asynchronous: true; visible: status === Image.Ready; sourceSize.width: 260 }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4
            Text { Layout.fillWidth: true; text: w.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold; elide: Text.ElideRight }
            Text { Layout.fillWidth: true; text: w.artist; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
            Item { Layout.fillHeight: true }
            Rectangle {
                Layout.fillWidth: true
                height: 4; radius: 2
                color: Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.18)
                visible: w.len > 0
                Rectangle { width: parent.width * Math.min(1, w.pos / Math.max(1, w.len)); height: parent.height; radius: 2; color: Theme.accent }
            }
            RowLayout {
                Layout.fillWidth: true
                visible: w.len > 0
                Text { text: w.fmt(w.pos); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                Item { Layout.fillWidth: true }
                Text { text: w.fmt(w.len); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
            }
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 10
                WBtn { visible: w.players.length > 1; icon: "chev"; rotation: 180; flat: true; size: 28; onClicked: w.which = (w.which + w.players.length - 1) % w.players.length }
                WBtn { icon: "back"; size: 34; onClicked: if (w.player && w.player.canGoPrevious) w.player.previous() }
                WBtn { icon: w.playing ? "pause" : "play"; size: 42; on: true; onClicked: if (w.player && w.player.canTogglePlaying) w.player.togglePlaying() }
                WBtn { icon: "forward"; size: 34; onClicked: if (w.player && w.player.canGoNext) w.player.next() }
                WBtn { visible: w.players.length > 1; icon: "chev"; flat: true; size: 28; onClicked: w.which = (w.which + 1) % w.players.length }
            }
        }
    }
    ColumnLayout {
        visible: !w.has
        anchors.centerIn: parent
        spacing: 6
        Icon { Layout.alignment: Qt.AlignHCenter; name: "headphones"; size: 36; color: Theme.muted }
        Text { Layout.alignment: Qt.AlignHCenter; text: "Nothing playing"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 }
        Text { Layout.alignment: Qt.AlignHCenter; text: "Play something in any app and it shows here"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
    }
}

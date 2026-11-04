// What's playing (MPRIS), for the centre pill (Islands.qml): album art, title and artist and a small level meter;
// the play/pause button is the pill's end. Loaded on its own so a Quickshell without the Mpris service only loses
// this. `shownPlayer`: the one playing, else none (the pill is just the clock then).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.Mpris

RowLayout {
    id: m
    property real k: 1
    property bool compact: false
    readonly property var shownPlayer: Mpris.players.values.find(p => p.isPlaying) || null
    readonly property bool playing: shownPlayer !== null
    function toggle() { if (shownPlayer && shownPlayer.canTogglePlaying) shownPlayer.togglePlaying(); }
    spacing: 10 * k
    visible: playing

    Rectangle {
        implicitWidth: 32 * m.k; implicitHeight: 32 * m.k
        radius: Theme.r(8)
        clip: true
        gradient: Gradient { GradientStop { position: 0; color: Theme.flame } GradientStop { position: 1; color: Theme.accent } }
        Image { anchors.fill: parent; source: m.shownPlayer ? (m.shownPlayer.trackArtUrl || "") : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; visible: status === Image.Ready; sourceSize.width: 96 }
    }
    ColumnLayout {
        visible: !m.compact
        spacing: 0
        Layout.maximumWidth: 200 * m.k
        Text { Layout.fillWidth: true; text: m.shownPlayer ? (m.shownPlayer.trackTitle || m.shownPlayer.identity || "") : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: Math.round(13 * m.k); font.weight: Font.DemiBold; elide: Text.ElideRight }
        Text { Layout.fillWidth: true; text: m.shownPlayer ? (m.shownPlayer.trackArtist || "") : ""; visible: text !== ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: Math.round(11 * m.k); elide: Text.ElideRight }
    }
    Row {   // a small level meter (decorative: it moves while playing, holds still with Reduce motion)
        spacing: 3
        Layout.alignment: Qt.AlignVCenter
        visible: !m.compact
        Repeater {
            model: 4
            delegate: Rectangle {
                id: bar
                required property int index
                width: 3 * m.k; radius: 1.5
                anchors.bottom: parent.bottom
                height: (8 + ((index * 5) % 9)) * m.k
                color: Theme.accent
                SequentialAnimation on height {
                    running: m.playing && !Settings.reduceMotion
                    loops: Animation.Infinite
                    NumberAnimation { to: (6 + (bar.index * 7) % 12) * m.k; duration: 380 + bar.index * 70 }
                    NumberAnimation { to: (14 - bar.index * 2) * m.k; duration: 420 + bar.index * 50 }
                }
            }
        }
    }
}

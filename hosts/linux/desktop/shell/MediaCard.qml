// The dashboard's media card (ClockPanel.qml): art, title, artist, a position bar and previous / play-pause / next
// for the one playing (or the last one that can be controlled). MPRIS; its own file so a Quickshell without the
// Mpris service only loses this card.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.Mpris

Item {
    id: card
    readonly property var player: Mpris.players.values.find(p => p.isPlaying) || Mpris.players.values.find(p => p.canControl) || null
    readonly property bool has: player !== null
    implicitHeight: has ? 112 : 0
    visible: has
    function fmt(s) { s = Math.max(0, Math.floor(s || 0)); return Math.floor(s / 60) + ":" + ("0" + (s % 60)).slice(-2); }
    Timer { interval: 1000; running: card.visible && card.player && card.player.isPlaying; repeat: true; onTriggered: if (card.player && card.player.positionSupported) card.player.positionChanged() }

    Frame { anchors.fill: parent; rad: Theme.r(16); solid: true }
    RowLayout {
        anchors { fill: parent; margins: 16 }
        spacing: 14
        Rectangle {
            Layout.preferredWidth: 80; Layout.preferredHeight: 80
            radius: Theme.r(12)
            clip: true
            gradient: Gradient { GradientStop { position: 0; color: Theme.flame } GradientStop { position: 1; color: Theme.accent } }
            Image { anchors.fill: parent; source: card.player ? (card.player.trackArtUrl || "") : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; visible: status === Image.Ready; sourceSize.width: 160 }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4
            Text { Layout.fillWidth: true; text: card.player ? (card.player.trackTitle || card.player.identity || "") : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold; elide: Text.ElideRight }
            Text { Layout.fillWidth: true; text: card.player ? [card.player.trackArtist, card.player.identity].filter(Boolean).join(" · ") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
            Item {
                Layout.fillWidth: true
                implicitHeight: 6
                visible: card.player && card.player.lengthSupported && card.player.length > 0
                Rectangle { anchors.fill: parent; radius: 3; color: Theme.panel3 }
                Rectangle { height: parent.height; radius: 3; color: Theme.accent; width: parent.width * (card.player && card.player.length > 0 ? Math.min(1, card.player.position / card.player.length) : 0) }
            }
            RowLayout {
                Layout.fillWidth: true
                visible: card.player && card.player.lengthSupported && card.player.length > 0
                Text { text: card.fmt(card.player ? card.player.position : 0); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                Item { Layout.fillWidth: true }
                Text { text: card.fmt(card.player ? card.player.length : 0); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
            }
        }
        Round { icon: "back"; onPicked: if (card.player && card.player.canGoPrevious) card.player.previous() }
        Round { icon: card.player && card.player.isPlaying ? "pause" : "play"; big: true; onPicked: if (card.player && card.player.canTogglePlaying) card.player.togglePlaying() }
        Round { icon: "forward"; onPicked: if (card.player && card.player.canGoNext) card.player.next() }
    }
    component Round: Rectangle {
        id: rb
        property string icon
        property bool big: false
        signal picked()
        implicitWidth: big ? 52 : 38; implicitHeight: implicitWidth
        radius: implicitWidth / 2
        color: big ? Theme.accent : ra.containsMouse ? Theme.panel3 : "transparent"
        Text { visible: rb.icon === "pause" || rb.icon === "play"; anchors.centerIn: parent; text: rb.icon === "pause" ? "❚❚" : "▶"; color: rb.big ? Theme.accentInk : Theme.text; font.pixelSize: rb.big ? 16 : 13 }
        Icon { visible: rb.icon !== "pause" && rb.icon !== "play"; anchors.centerIn: parent; name: rb.icon; size: 18 }
        MouseArea { id: ra; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: rb.picked() }
    }
}

// A game in a shelf or the library grid: its cover (or wide picture), where it's from, Not installed / Cloud, a
// download's progress, and its title under it while selected. A selects it (the game's page).
import QtQuick

Focusable {
    id: t
    property var game: null
    property bool wide: false
    property bool caption: true
    property bool loadArt: true
    zoom: true
    radius: Theme.u(10)
    implicitWidth: wide ? Theme.u(400) : Theme.u(160)
    implicitHeight: wide ? Theme.u(250) : Theme.u(240)
    onActivated: if (game) App.navigate("game", { gameId: game.id })
    readonly property bool active: App.isActive(game)
    readonly property bool dimmed: game && !App.playable(game) && !active

    Rectangle {
        id: cover
        anchors.fill: parent
        radius: t.radius
        color: Theme.panel2
        clip: true
        Text {
            visible: !art.ready
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: Theme.u(12) }
            text: t.game ? t.game.title : ""
            color: Theme.text
            wrapMode: Text.WordWrap
            maximumLineCount: 3
            elide: Text.ElideRight
            font.family: Theme.display
            font.pixelSize: t.wide ? Theme.u(26) : Theme.u(17)
            font.weight: Font.Bold
        }
        ArtImage {
            id: art
            anchors.fill: parent
            game: t.game
            kinds: t.wide ? ["wide", "hero", "cover"] : ["cover", "wide"]
            load: t.loadArt
            opacity: t.dimmed ? 0.62 : 1
        }
        Rectangle {
            anchors { top: parent.top; right: parent.right; margins: Theme.u(8) }
            radius: Theme.u(5)
            color: Theme.alpha("#000000", 0.45)
            width: srcText.implicitWidth + Theme.u(12)
            height: srcText.implicitHeight + Theme.u(6)
            Text {
                id: srcText
                anchors.centerIn: parent
                text: App.sourceName(t.game).toUpperCase()
                color: "#dfe6ee"
                font.family: Theme.font
                font.pixelSize: Theme.u(10)
                font.weight: Font.Bold
                font.letterSpacing: Theme.u(0.6)
            }
        }
        Rectangle {
            visible: !!t.game && !t.active && (!t.game.inst)
            anchors { top: parent.top; left: parent.left; margins: Theme.u(8) }
            radius: Theme.u(5)
            color: Theme.alpha("#000000", 0.55)
            width: niText.implicitWidth + Theme.u(12)
            height: niText.implicitHeight + Theme.u(6)
            Text {
                id: niText
                anchors.centerIn: parent
                text: t.game && t.game.cloud ? "Cloud" : "Not installed"
                color: Theme.warn
                font.family: Theme.font
                font.pixelSize: Theme.u(10)
                font.weight: Font.DemiBold
            }
        }
        Column {
            visible: t.active
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: Theme.u(10) }
            spacing: Theme.u(5)
            Text {
                text: t.game ? App.stateLabel(t.game) : ""
                color: Theme.text
                font.family: Theme.font
                font.pixelSize: Theme.u(11)
                font.weight: Font.Bold
                style: Text.Outline
                styleColor: Theme.alpha("#000000", 0.6)
            }
            Bar { width: parent.width; value: t.game && t.game.progress != null ? t.game.progress : 0 }
        }
    }
    Text {
        visible: t.caption && t.isFocused
        anchors { left: parent.left; top: parent.bottom; topMargin: Theme.u(8); leftMargin: Theme.u(4) }
        width: Math.max(parent.width, Theme.u(240))
        text: t.game ? t.game.title + (t.game.last ? " · " + t.lastText(t.game.last) : "") : ""
        color: Theme.text
        elide: Text.ElideRight
        font.family: Theme.font
        font.pixelSize: Theme.u(13)
    }
    function lastText(ms) {
        const d = Math.floor((Date.now() - ms) / 86400000);
        return d <= 0 ? "Today" : d === 1 ? "Yesterday" : d < 7 ? d + " days ago" : new Date(ms).toLocaleDateString();
    }
}

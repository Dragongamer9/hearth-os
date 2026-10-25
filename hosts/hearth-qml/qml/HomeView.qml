// Home, the default layout (the web page's viewHome): the last game's picture behind, Recent games (the newest big),
// Favorites and Recently added. The other Home layouts are the web renderer's for now (a card says so).
import QtQuick

Item {
    id: home
    readonly property var vis: { App.rev; return App.visibleGames().filter(x => x.inst || App.isActive(x) || (x.cloud && (x.last || x.fav))); }
    readonly property var recent: vis.filter(x => x.last).sort((a, b) => b.last - a.last).slice(0, 10)
    readonly property var favs: vis.filter(x => x.fav).slice(0, 24)
    readonly property var added: vis.slice().sort((a, b) => b.added - a.added).slice(0, 10)
    readonly property var first: recent.length ? recent[0] : (added.length ? added[0] : null)

    ArtImage {
        id: hero
        anchors { left: parent.left; right: parent.right; top: parent.top }
        height: Math.max(Theme.u(150), Math.min(Theme.u(260), home.height * 0.3))
        game: home.first
        kinds: ["hero", "wide", "cover"]
        opacity: 0.45
    }
    Rectangle {
        anchors.fill: hero
        gradient: Gradient {
            GradientStop { position: 0; color: Theme.alpha(Theme.bg, 0.1) }
            GradientStop { position: 1; color: Theme.bg }
        }
    }

    Scroller {
        id: page
        anchors.fill: parent
        contentHeight: col.height + Theme.u(40)
        Column {
            id: col
            x: Theme.gut
            width: page.width - Theme.gut * 2
            topPadding: Math.max(Theme.u(80), hero.height - Theme.u(70))
            spacing: Theme.u(4)

            Column {
                visible: home.vis.length === 0
                width: parent.width
                spacing: Theme.u(12)
                topPadding: Theme.u(40)
                Text { text: App.libLoading || !App.libLoaded ? "Loading your games…" : "No games found yet"; color: Theme.text; font.family: Theme.display; font.pixelSize: Theme.u(22); font.weight: Font.Bold }
                Text {
                    visible: App.libLoaded && !App.libLoading
                    width: parent.width
                    wrapMode: Text.WordWrap
                    text: "Install a game in Steam or Heroic (Epic, GOG), stream one with Xbox Cloud Gaming, or add one yourself."
                    color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(14)
                }
                Btn { visible: App.libLoaded && !App.libLoading; text: "Library"; icon: "lib"; navKey: "h-lib"; navFirst: home.vis.length === 0; onActivated: App.navigate("library") }
            }

            Heading { visible: home.recent.length > 0; text: "Recent games" }
            Shelf { visible: home.recent.length > 0; games: home.recent; prefix: "h-r-"; firstWide: true; firstFocus: true }
            Heading { visible: home.favs.length > 0; text: "Favorites" }
            Shelf { visible: home.favs.length > 0; games: home.favs; prefix: "h-f-" }
            Heading { visible: home.added.length > 0; text: "Recently added" }
            Shelf { visible: home.added.length > 0; games: home.added; prefix: "h-a-"; firstFocus: home.recent.length === 0 }

            Heading { text: "Qt renderer" }
            Rectangle {
                width: parent.width
                height: notHere.implicitHeight + Theme.u(32)
                radius: Theme.radius
                color: Theme.panel
                border.color: Theme.line
                Row {
                    id: notHere
                    anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: Theme.u(16) }
                    spacing: Theme.u(16)
                    Icon { name: "info"; color: Theme.warn; size: Theme.u(24); anchors.verticalCenter: parent.verticalCenter }
                    Column {
                        width: parent.width - Theme.u(24) - openWeb.width - Theme.u(32)
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.u(4)
                        Text { text: "Not in the Qt renderer yet"; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(15); font.weight: Font.DemiBold }
                        Text {
                            width: parent.width; wrapMode: Text.WordWrap; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13)
                            text: "Store, Friends, Screenshots, Co-op and Multi-seat, Hearth Link, game settings, the other Home and Library layouts, most of Settings, and signing in to stores. The web renderer has them all."
                        }
                    }
                    Btn { id: openWeb; text: "Open in the web renderer"; icon: "globe"; navKey: "h-web"; anchors.verticalCenter: parent.verticalCenter; onActivated: App.openInWeb() }
                }
            }
        }
    }

    component Shelf: Scroller {
        id: shelf
        property var games: []
        property string prefix: ""
        property bool firstWide: false
        property bool firstFocus: false
        horizontal: true
        width: parent ? parent.width + Theme.gut * 2 : 0
        x: -Theme.gut
        height: Theme.u(250) + Theme.u(44)
        clip: false
        interactive: true
        Row {
            x: Theme.gut
            y: Theme.u(8)
            spacing: Theme.u(14)
            Repeater {
                model: shelf.games
                delegate: GameTile {
                    required property var modelData
                    required property int index
                    game: modelData
                    wide: shelf.firstWide && index === 0
                    navKey: shelf.prefix + modelData.id
                    navFirst: shelf.firstFocus && index === 0
                    y: wide ? 0 : Theme.u(10)
                    height: wide ? Theme.u(250) : Theme.u(240)
                    width: wide ? Theme.u(400) : Theme.u(160)
                }
            }
            Item { width: Theme.gut; height: 1 }
        }
    }
}

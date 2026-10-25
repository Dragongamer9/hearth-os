// A game's page (the web page's viewGame): its picture and logo, Play / Resume / Install / the download, Exit game,
// Favorite, Uninstall, and the details. Game settings, mods and achievements are the web renderer's for now.
import QtQuick

Item {
    id: gp
    readonly property var game: { App.rev; return App.g(App.gameId); }
    readonly property bool running: !!game && App.running === game.id
    readonly property bool active: App.isActive(game)

    Text {
        visible: !gp.game
        anchors.centerIn: parent
        text: "That game isn't in your library anymore."
        color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(16)
    }

    Scroller {
        id: page
        visible: !!gp.game
        anchors.fill: parent
        contentHeight: col.height + Theme.u(40)
        Column {
            id: col
            width: page.width
            Item {
                id: heroBox
                width: parent.width
                height: Math.max(Theme.u(220), Math.min(Theme.u(380), gp.height * 0.46))
                Rectangle { anchors.fill: parent; color: Theme.panel }
                ArtImage { id: heroArt; anchors.fill: parent; game: gp.game; kinds: ["hero", "wide", "cover"] }
                Rectangle {
                    anchors.fill: parent
                    gradient: Gradient {
                        GradientStop { position: 0.35; color: Theme.alpha(Theme.bg, 0) }
                        GradientStop { position: 1; color: Theme.bg }
                    }
                }
                ArtImage {
                    id: logo
                    anchors { left: parent.left; leftMargin: Theme.gut; bottom: parent.bottom; bottomMargin: Theme.u(20) }
                    width: Math.min(Theme.u(420), parent.width * 0.45)
                    height: Theme.u(120)
                    game: gp.game
                    kinds: ["logo"]
                    fill: Image.PreserveAspectFit
                    hAlign: Image.AlignLeft
                    visible: ready
                }
                Text {
                    visible: !logo.ready
                    anchors { left: parent.left; leftMargin: Theme.gut; right: parent.right; rightMargin: Theme.gut; bottom: parent.bottom; bottomMargin: Theme.u(20) }
                    text: gp.game ? gp.game.title : ""
                    color: Theme.text
                    wrapMode: Text.WordWrap
                    font.family: Theme.display
                    font.pixelSize: Theme.u(40)
                    font.weight: Font.Black
                    style: Text.Raised
                    styleColor: Theme.alpha("#000000", 0.5)
                }
            }
            Flow {
                id: barRow
                x: Theme.gut
                width: parent.width - Theme.gut * 2
                spacing: Theme.u(10)
                topPadding: Theme.u(6)
                Btn {
                    id: play
                    navKey: "g-play"
                    navFirst: true
                    kind: "primary"
                    minW: Theme.u(200)
                    height: Theme.u(52)
                    icon: gp.active && !(gp.game && gp.game.inst) ? "dl" : App.playable(gp.game) ? "play" : "dl"
                    text: !gp.game ? "" : gp.active && !gp.game.inst ? App.stateLabel(gp.game) : gp.running ? "Resume" : App.playable(gp.game) ? (gp.game.cloud && !gp.game.inst ? "Stream" : "Play") : gp.game.src === "beacon" ? "Set up" : "Install"
                    onActivated: {
                        const x = gp.game;
                        if (!x) return;
                        if (gp.active && !x.inst) return App.navigate("downloads");
                        if (gp.running) { App.closeAll(); return App.send({ type: "resume" }); }
                        if (App.playable(x)) return App.launch(x);
                        App.install(x);
                    }
                }
                Btn { visible: gp.running; navKey: "g-stop"; icon: "stop"; text: "Exit game"; height: Theme.u(52); onActivated: App.exitGame() }
                Btn { visible: gp.active; navKey: "g-cancel"; icon: "x"; text: "Cancel download"; height: Theme.u(52); onActivated: App.dlCall("game.cancelInstall", gp.game.id) }
                Btn { navKey: "g-fav"; kind: "icon"; icon: "star"; on: !!(gp.game && gp.game.fav); width: Theme.u(52); height: Theme.u(52); onActivated: App.setFav(gp.game) }
                Btn { visible: !!(gp.game && gp.game.inst && gp.game.canUninstall); navKey: "g-uninst"; icon: "trash"; text: "Uninstall"; height: Theme.u(52); onActivated: App.uninstall(gp.game) }
                Btn { navKey: "g-more"; icon: "dots"; text: "Game settings…"; height: Theme.u(52); onActivated: App.notHere("Game settings (Proton, launch options, mods, saves)") }
            }
            Flow {
                x: Theme.gut
                width: parent.width - Theme.gut * 2
                spacing: Theme.u(34)
                topPadding: Theme.u(18)
                bottomPadding: Theme.u(8)
                Repeater {
                    model: gp.game ? [
                        ["Last played", gp.game.last ? gp.ago(gp.game.last) : "Never"],
                        ["Play time", gp.game.hrs ? gp.hours(gp.game.hrs) : "—"],
                        [gp.game.system ? "System" : "Source", App.sourceName(gp.game)],
                        ["Size", gp.game.sizeBytes ? gp.size(gp.game.sizeBytes) : "—"]
                    ] : []
                    delegate: Column {
                        required property var modelData
                        spacing: Theme.u(3)
                        Text { text: modelData[0]; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(12); font.capitalization: Font.AllUppercase }
                        Text { text: modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(16); font.weight: Font.DemiBold }
                    }
                }
            }
            Rectangle {
                x: Theme.gut
                width: parent.width - Theme.gut * 2
                height: about.implicitHeight + Theme.u(32)
                radius: Theme.radius
                color: Theme.panel
                Column {
                    id: about
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.u(16) }
                    spacing: Theme.u(8)
                    Text { text: "About"; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(15); font.weight: Font.DemiBold }
                    Text { width: parent.width; wrapMode: Text.WordWrap; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(14); text: gp.aboutText() }
                    Text { visible: !!(gp.game && gp.game.path); width: parent.width; wrapMode: Text.WrapAnywhere; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13); text: gp.game ? "Location: " + gp.game.path : "" }
                }
            }
        }
    }

    function aboutText() {
        const x = game; if (!x) return "";
        if (x.cloud) return "Streams with Xbox Cloud Gaming. Needs Game Pass Ultimate; sign in with your Xbox account the first time. (Streaming isn't in the Qt renderer yet: open the web renderer to play it.)";
        if (x.system) return "A " + x.system + " game from your ROMs folder, played with RetroArch.";
        if (x.inst) return "Installed through " + App.sourceName(x) + ".";
        if (App.isActive(x)) return (x.installKind === "nimbus" ? "Hearth is installing this game into your games folder." : App.sourceName(x) + " is installing this game.");
        if (x.note) return "In your " + App.sourceName(x) + " library. " + x.note + ", so Hearth can't install it.";
        if (x.installKind === "nimbus") return "In your " + App.sourceName(x) + " library. Select Install and Hearth downloads it, no launcher needed.";
        if (x.installKind === "steam") return "In your Steam library. Select Install and Steam downloads it in the background. Follow it under Downloads.";
        return "In your " + App.sourceName(x) + " library. Select Install and " + App.sourceName(x) + " will ask where to put it.";
    }
    function ago(ms) {
        const m = Math.floor((Date.now() - ms) / 60000);
        if (m < 60) return m <= 1 ? "Just now" : m + " min ago";
        const h = Math.floor(m / 60); if (h < 24) return h + " h ago";
        const d = Math.floor(h / 24); return d === 1 ? "Yesterday" : d < 30 ? d + " days ago" : new Date(ms).toLocaleDateString();
    }
    function hours(h) { return h < 1 ? Math.round(h * 60) + " min" : (h < 10 ? h.toFixed(1) : Math.round(h)) + " h"; }
    function size(b) { return b >= 1e9 ? (b / 1e9).toFixed(1) + " GB" : Math.round(b / 1e6) + " MB"; }
}

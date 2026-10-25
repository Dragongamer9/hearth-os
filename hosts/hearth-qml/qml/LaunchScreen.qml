// The starting screen while a game loads (the web page's #launch): its picture, what's happening, Show it now.
import QtQuick

Item {
    id: ls
    readonly property bool open: App.layers.indexOf("launch") >= 0
    readonly property var game: { App.rev; return App.g(App.launchId); }
    visible: open
    Rectangle { anchors.fill: parent; color: Theme.bg }
    ArtImage { anchors.fill: parent; game: ls.game; kinds: ["hero", "wide", "cover"]; opacity: 0.35 }
    Rectangle { anchors.fill: parent; gradient: Gradient { GradientStop { position: 0.3; color: Theme.alpha(Theme.bg, 0) } GradientStop { position: 1; color: Theme.bg } } }
    Column {
        anchors { left: parent.left; leftMargin: Theme.gut * 2; bottom: parent.bottom; bottomMargin: Theme.u(90) }
        width: parent.width - Theme.gut * 4
        spacing: Theme.u(12)
        Item {
            width: Theme.u(36); height: width
            Rectangle {
                anchors.fill: parent; radius: width / 2; color: "transparent"; border.width: Theme.u(4); border.color: Theme.alpha(Theme.text, 0.15)
            }
            Rectangle {
                width: Theme.u(10); height: width; radius: width / 2; color: Theme.accent
                x: parent.width / 2 - width / 2 + Math.cos(spin.a) * (parent.width / 2 - Theme.u(2)) ; y: parent.height / 2 - height / 2 + Math.sin(spin.a) * (parent.height / 2 - Theme.u(2))
            }
            QtObject { id: spin; property real a: 0 }
            NumberAnimation { target: spin; property: "a"; from: 0; to: Math.PI * 2; duration: 1100; loops: Animation.Infinite; running: ls.open }
        }
        Text { text: ls.game ? ls.game.title : ""; color: Theme.text; font.family: Theme.display; font.pixelSize: Theme.u(34); font.weight: Font.Black; width: parent.width; wrapMode: Text.WordWrap }
        Text { text: App.launchText; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(15); width: parent.width; wrapMode: Text.WordWrap }
        Row {
            spacing: Theme.u(10)
            Btn { visible: App.launchWaiting; navScope: "launch"; navKey: "l-show"; navFirst: true; text: "Show it now"; icon: "play"; onActivated: { App.launchWaiting = false; App.hideLaunch(); App.send({ type: "resume" }); } }
            Btn { navScope: "launch"; navKey: "l-qam"; text: "Quick Access"; icon: "sliders"; onActivated: { if (App.hasFeat("overlayOverLaunch")) App.send({ type: "overlay", action: "open" }); else App.openLayer("qam"); } }
        }
    }
}

// A page the Qt renderer doesn't have yet (Store, Friends...): says so plainly, with the way to the web renderer.
import QtQuick

Item {
    Column {
        anchors.centerIn: parent
        width: Math.min(parent.width - Theme.gut * 2, Theme.u(640))
        spacing: Theme.u(14)
        Icon { name: "info"; color: Theme.warn; size: Theme.u(44) }
        Text { text: (App.missing || "This") + " isn't in the Qt renderer yet"; color: Theme.text; font.family: Theme.display; font.pixelSize: Theme.u(26); font.weight: Font.Bold; wrapMode: Text.WordWrap; width: parent.width }
        Text {
            width: parent.width; wrapMode: Text.WordWrap; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(15)
            text: "The Qt renderer is Experimental: Home, Library, game pages, Downloads, Quick Access, the power menu and the renderer setting are here so far. The web renderer has everything, on the same Hearth: your games, sign-ins and settings are the same in both."
        }
        Row {
            spacing: Theme.u(10)
            Btn { kind: "primary"; text: "Open in the web renderer"; icon: "globe"; navKey: "m-web"; navFirst: true; onActivated: App.openInWeb() }
            Btn { text: "Back"; icon: "home"; navKey: "m-back"; onActivated: { if (!App.goBack()) App.navigate("home", {}, false); } }
        }
    }
}

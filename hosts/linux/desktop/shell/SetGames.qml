// Hearth Settings › Games & stores: Hearth Games (GOG, Epic, Battle.net: hearth-gamestore), store sign-ins (Epic, GOG, Xbox), where your games come from, and Hearth's gaming
// apps. Gathers what lived in the archived Game Mode's Settings › Games (Library sources, Import, Game stores &
// accounts) and its Library, Downloads and Store screens. Apps that exist open themselves; the rest opens in the Hearth
// app's window at that page until it has an app of its own (docs/SETTINGS-MOVE.md, "Interim").
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Games & stores"
    HearthCall { id: core }
    readonly property string signin: decodeURIComponent(Qt.resolvedUrl("hearth-signin").toString().replace(/^file:\/\//, ""))
    function signIn(store, name) {
        Quickshell.execDetached(["python3", page.signin, "open", store]);
        SettingsNav.toast("Opening " + name + "'s sign-in…");
    }
    function app(cmd, name) { Quickshell.execDetached(cmd); SettingsNav.toast("Opening " + name + "…"); }

    SetHeader { text: "Hearth Games" }
    SetRow { chevron: true; icon: "store"; title: "Hearth Games"; sub: "Your GOG, Epic Games and Battle.net games: sign in, install, play and uninstall (docs/HEARTH-GAMES.md)"; onActivated: page.app(["hearth-gamestore"], "Hearth Games") }

    SetHeader { text: "Store accounts" }
    SetRow { icon: "lock"; title: "Epic Games"; sub: "Sign in to see, install and play your Epic games (in Hearth Games)"; onActivated: page.app(["hearth-gamestore", "--store", "epic"], "Hearth Games") }
    SetRow { icon: "lock"; title: "GOG"; sub: "Sign in to see, install and play your GOG games (in Hearth Games)"; onActivated: page.app(["hearth-gamestore", "--store", "gog"], "Hearth Games") }
    SetRow { icon: "lock"; title: "Battle.net"; sub: "Blizzard's own app, set up through Faugus Launcher (in Hearth Games)"; onActivated: page.app(["hearth-gamestore", "--store", "battlenet"], "Hearth Games") }
    SetRow { icon: "lock"; title: "Xbox"; sub: "Sign in for Xbox Cloud Gaming and your Xbox PC games"; onActivated: page.signIn("xbox", "Xbox") }
    Text {
        Layout.fillWidth: true
        text: "Steam signs in in Steam itself. Hearth never signs in to Steam for you."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Your games" }
    SetRow { chevron: true; icon: "grid"; title: "Library sources"; sub: "Steam, Epic, GOG and emulators: which ones Hearth lists, add a game by hand, look for games again"; onActivated: core.openPage("sources", "Opening Library sources in the Hearth app…") }
    SetRow { chevron: true; icon: "update"; title: "Downloads"; sub: "Epic and GOG installs: pause, carry on, cancel (in Hearth Games)"; onActivated: page.app(["hearth-gamestore"], "Hearth Games") }
    SetRow { chevron: true; icon: "store"; title: "Store"; sub: "Deals, new games and search for Steam, GOG, Epic and Xbox"; onActivated: core.openPage("shop", "Opening the Store in the Hearth app…") }
    SetRow { chevron: true; icon: "pad"; title: "Recomps"; sub: "Native PC versions of classic console games: install and update them"; onActivated: core.openPage("recomps", "Opening Recomps in the Hearth app…") }
    SetRow { chevron: true; icon: "folder"; title: "Import"; sub: "Games and files from other drives and systems"; onActivated: core.openPage("import", "Opening Import in the Hearth app…") }
    SetRow { chevron: true; icon: "desktops"; title: "Get a game from another Hearth"; sub: "Copy an installed game from one of your Hearth devices (Settings › Hearth Link pairs them)"; onActivated: core.openPage("library", "Opening your library in the Hearth app: Get from another Hearth…") }

    SetHeader { text: "Apps for your games" }
    SetRow { icon: "grid"; title: "Mods"; sub: "Install mods and Nexus or Wabbajack modlists, turn them on and off, choose their order"; onActivated: page.app(["hearth-mods"], "Mods") }
    SetRow { icon: "folder"; title: "Saves"; sub: "Back up and restore your games' saves, and move them to another PC"; onActivated: page.app(["hearth-saves"], "Saves") }
    SetRow { icon: "pad"; title: "Controller Lab"; sub: "Test every button, stick and trigger; measure stick drift and calibrate"; onActivated: page.app(["hearth-controller-lab"], "Controller Lab") }
    SetRow { icon: "chat"; title: "Discord"; sub: "Friends, chats and invites in Discord; what you're playing on Discord"; onActivated: SettingsNav.openPage("discord") }
    SetRow { icon: "display"; title: "Theater"; sub: "Videos from the web as tiles, full screen with a controller"; onActivated: page.app(["hearth-theater"], "Theater") }
    SetRow { icon: "gear"; title: "Benchmark"; sub: "How fast this device is, with scores and history"; onActivated: page.app(["hearth-tools", "--app", "benchmark"], "Benchmark") }
}

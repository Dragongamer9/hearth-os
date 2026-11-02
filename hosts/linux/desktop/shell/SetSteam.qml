// Hearth Settings › Steam Game Mode: Hearth OS's Game Mode (Steam's own gamepad screens, like SteamOS), Decky Loader and
// Hearth Sync (the Decky plugin that puts your non-Steam games in Steam's library), and why it didn't start last time.
// Moved from the archived Game Mode's Settings › System › Steam Game Mode, with the same calls (hearth-core
// steam_mode.rs through HearthCall). docs/STEAM-GAME-MODE.md, docs/HEARTH-SYNC.md.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Steam Game Mode"
    property var st: null          // steamMode.status: {available, steam, gamescope, decky: {installed, via, plugin, shipped}, lastFailure}
    property string err: ""
    property string msg: ""
    property bool busy: false
    property bool confirm: false   // "Install Decky Loader?" asked
    readonly property var decky: st && st.decky ? st.decky : ({})
    HearthCall { id: core }
    function load() { core.call("steamMode.status", {}, v => { page.st = v; page.err = ""; }, e => page.err = e); }
    Component.onCompleted: load()
    function decide(action) {
        page.busy = true; page.confirm = false; page.msg = action === "install" ? "Installing Decky Loader and Hearth Sync… it asks for an administrator's password" : "Removing Decky Loader…";
        core.call("steamMode.decky", { action: action }, v => { page.busy = false; page.st = v; page.msg = action === "install" ? "Decky Loader and Hearth Sync are installed. Switch to Steam Game Mode to use them." : "Decky Loader removed"; },
            e => { page.busy = false; page.msg = e; page.load(); });
    }

    SetRow {
        icon: "pad"; title: "Switch to Steam Game Mode"
        sub: "Steam's own gamepad screens, like SteamOS: where the device starts. Back here: Steam's Power › Switch to Desktop."
        onActivated: Quickshell.execDetached(["nimbus-mode", "steam"])
    }
    Text {
        visible: !!page.st && !!page.st.lastFailure
        Layout.fillWidth: true
        text: page.st && page.st.lastFailure ? "Steam Game Mode didn't start last time, so Desktop Mode opened instead: " + page.st.lastFailure + ". Switch to Steam Game Mode tries again." : ""
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && page.st.available === false
        Layout.fillWidth: true
        text: "Steam Game Mode can't run here: it needs Steam" + (page.st && page.st.gamescope === false ? " and gamescope" : "") + "."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Your other games in Steam" }
    SetRow {
        icon: "store"; title: "Decky Loader and Hearth Sync"
        sub: page.busy ? "Working on it…"
            : !page.st ? (page.err || "Loading…")
            : page.decky.installed ? (page.decky.via === "decky" ? "Decky Loader (its own install)" : "Installed") + (page.decky.plugin ? " · Hearth Sync in place" : page.decky.shipped ? " · Hearth Sync not added yet" : "") + ". Decky runs only while Steam Game Mode does."
            : "Adds your non-Steam games (Epic, GOG, emulators and more) to Steam's library with covers. Needs an administrator's password; turns on Steam's remote debugging."
        value: page.st && !page.busy ? (page.decky.installed && page.decky.via === "hearth" ? "Remove" : page.decky.installed ? "" : "Install") : ""
        onActivated: {
            if (page.busy || !page.st) return;
            if (page.decky.installed && page.decky.via === "hearth") page.decide("remove");
            else if (!page.decky.installed) page.confirm = !page.confirm;
        }
    }
    SetRow {
        visible: page.confirm
        icon: "check"; title: "Install Decky Loader"
        sub: "Downloads Decky Loader from its GitHub releases and adds Hearth Sync. Decky runs as root while Steam Game Mode runs, and Steam's remote debugging is turned on (any program on this device can then control Steam)."
        onActivated: page.decide("install")
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

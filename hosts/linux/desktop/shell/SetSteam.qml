// Hearth Settings › Steam Game Mode: Hearth OS's Game Mode (Steam's own gamepad screens, like SteamOS), Decky Loader and
// Hearth Sync (the Decky plugin that puts your non-Steam games in Steam's library), and why it didn't start last time.
// Moved from the archived Game Mode's Settings › System › Steam Game Mode, with the same calls (hearth-core
// steam_mode.rs through HearthCall). docs/STEAM-GAME-MODE.md, docs/HEARTH-SYNC.md.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

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

    // 1.14: Steam Game Mode beside the desktop (hearth-side-by-side, docs/SIDE-BY-SIDE.md), off unless you turn it on.
    property var side: null
    Process {
        id: sideProc
        property var after: null
        stdout: StdioCollector { onStreamFinished: { try { page.side = JSON.parse(text.trim().split("\n").pop()); } catch (e) {} } }
        onExited: code => { if (sideProc.after) { const f = sideProc.after; sideProc.after = null; f(code); } }
    }
    function sideRun(args, after) { sideProc.after = after || null; sideProc.command = ["/usr/libexec/nimbus/hearth-side-by-side"].concat(args); sideProc.running = true; }
    Timer { running: true; interval: 50; onTriggered: page.sideRun(["status"]) }
    SetSwitch {
        icon: "desktops"; title: "Keep Steam Game Mode running beside the desktop (test)"
        sub: "Switching only shows the other one: a second or two, and Steam stays open and signed in. It keeps about 1–1.5 GB of memory for Steam. If Steam Game Mode doesn't come up this way, the usual switch happens instead. Still being tried on the Nvidia PC and the Ally."
        checked: !!page.side && page.side.on
        onToggled: v => page.sideRun([v ? "on" : "off"], code => { if (code !== 0) SettingsNav.toast("That didn't work"); else SettingsNav.toast(v ? "On: Steam Game Mode now opens beside the desktop" : "Off: switching is as before"); page.sideRun(["status"]); })
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

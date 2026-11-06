// Hearth Settings › Gaming › Discord (docs/DISCORD.md): which Discord the bar opens, "Playing <game>" on Discord (Rich
// Presence) with its one-step setup (paste the Discord application ID), Ask to join / Join for friends, and the
// invite link for what you're playing. Since 1.6 Discord replaces Hearth Friends (Matrix, archived); its old settings
// and data are left exactly where they were and aren't read any more.
// Everything here changes only when you press it (hearth-discord, which merges one key of ~/.config/nimbus/discord.json).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Discord"
    property var st: null
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-discord").toString().replace(/^file:\/\//, ""))
    readonly property var names: ({ official: "Discord (the app)", web: "Discord in a web window", dorion: "Dorion (unofficial)" })
    Component.onCompleted: load()
    function load() { if (!statusProc.running) statusProc.running = true; }
    function set(key, value) { setProc.command = ["python3", helper, "set", key, value]; setProc.running = true; }

    Process {
        id: statusProc
        command: ["python3", page.helper, "status"]
        stdout: StdioCollector { onStreamFinished: { try { page.st = JSON.parse(text.trim().split("\n").pop()); } catch (e) { } } }
    }
    Process {
        id: setProc
        stdout: StdioCollector {
            onStreamFinished: {
                const t = text.trim();
                if (t.startsWith("error: ")) { SettingsNav.toast(t.slice(7)); return; }
                try { page.st = JSON.parse(t.split("\n").pop()); } catch (e) { }
            }
        }
    }
    Process { id: copyProc }

    SetHeader { text: "Your Discord" }
    SetRow {
        icon: "chat"; chevron: true
        title: "Which Discord"
        sub: page.st && page.st.client ? page.names[page.st.client] + (page.st.clients && !page.st.clients[page.st.client] ? " · not installed yet" : "") + ". Change it" : "Not picked yet: the bar's Discord button asks the first time"
        onActivated: { if (SettingsNav.discord) SettingsNav.discord.openChooser(); }
    }
    SetRow { icon: "window"; title: "Open Discord"; sub: "Full size. The bar's Discord button opens it in a small window you can reply to friends from"; onActivated: { if (SettingsNav.discord) SettingsNav.discord.full(); } }
    Text {
        Layout.fillWidth: true; wrapMode: Text.WordWrap
        text: "Hearth only opens, places and hides Discord's window. It never reads your messages or friends, and never asks for your Discord password. Discord's notifications show as Hearth's."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
    }

    SetHeader { text: "What you're playing" }
    SetRow {
        icon: "key"; chevron: false; interactive: false
        title: "Discord application ID"
        sub: page.st && page.st.appId ? "Set (" + page.st.appId + "). Discord shows \"Playing\" with the game's name and picture" : "One step: make an application at discord.com/developers/applications (its name is what Discord shows, e.g. \"Hearth OS\"), copy its Application ID and paste it here"
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        SetField { id: idField; placeholder: "Paste the Application ID (numbers only)"; onSubmitted: t => page.set("appId", t) }
        SetButton { text: "Save"; primary: true; onActivated: page.set("appId", idField.text) }
        SetButton { visible: !!(page.st && page.st.appId); text: "Remove"; ghost: true; onActivated: { idField.text = ""; page.set("appId", ""); } }
    }
    SetSwitch {
        icon: "play"
        title: "Show what I'm playing on Discord"
        sub: "Rich Presence: the game, its picture and how long you've played, while Discord is running"
        checked: !page.st || page.st.enabled
        onToggled: on => page.set("enabled", on ? "on" : "off")
    }
    SetSwitch {
        icon: "users"
        title: "Let friends ask to join"
        sub: "Friends see Ask to join / Join on your game in Discord. Joining opens the game on their Hearth, or Play together when Remote play (Sunshine) is on"
        checked: !page.st || page.st.askToJoin
        onToggled: on => page.set("askToJoin", on ? "on" : "off")
    }
    SetRow {
        visible: !!(page.st && page.st.invite && page.st.invite.link)
        icon: "link"; chevron: false
        title: page.st && page.st.invite && page.st.invite.playTogether && page.st.invite.pin ? "Invite link · Play together PIN " + page.st.invite.pin : "Invite link"
        sub: "Copy a link to paste in a Discord chat yourself. A Play together PIN in it works once, for 10 minutes"
        onActivated: { copyProc.command = ["wl-copy", page.st.invite.link]; copyProc.running = true; SettingsNav.toast("Invite link copied"); }
    }
}

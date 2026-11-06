// Hearth Settings › Home server › Gaming (docs/HOME-SERVER.md): this PC as the games hub for your Hearth devices. A
// Games folder they can copy installers and mods from (optional), Steam's own downloads over your home network and the
// game archive (Hearth Storage), Remote play over Tailscale, and exactly what Light mode does while you play. Hearth
// never signs in to Steam or touches Steam accounts or tokens: Steam's settings are changed in Steam.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Gaming"
    property var g: null
    property string msg: ""
    property bool bad: false
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["gaming"], r => page.g = r, e => page.g = { error: e }); }
    Component.onCompleted: refresh()
    Timer { interval: 6000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: "Your games, mods and installers in one place for every Hearth device, and what this PC holds back while you play."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Install from home" }
    SetRow {
        visible: !!page.g && !page.g.games
        icon: "folder"; title: "Share a Games folder"
        sub: "Home › Games, read only for your devices: put game installers (GOG, itch.io), mods and emulator files there and copy them to any Hearth device at home-network speed"
        onActivated: act.run(["folder-share", "games"], () => { page.say("Games folder shared ✓ (Shared folders has its settings)"); page.refresh(); }, e => page.say(e, true))
    }
    SetRow {
        visible: !!page.g && !!page.g.games
        icon: "folder"; title: "Games folder"
        sub: page.g && page.g.games ? "\\\\" + (page.g.tsName || "this PC") + "\\" + page.g.games.name + " · " + (page.g.games.write ? "read & write" : "read only") + " · " + page.g.games.path : ""
        onActivated: SettingsNav.openPage("hs-shares")
    }
    SetRow {
        icon: "store"; chevron: false; interactive: false; title: "Steam downloads at home"
        sub: "Steam copies games between your PCs and Steam Decks on the same network by itself (Steam › Settings › Downloads › Local Network Game Transfers, on each device). Hearth never signs in to Steam or changes its settings"
    }
    SetRow {
        icon: "desktops"; title: "Game archive and saves"
        sub: "Hearth Storage keeps games for your Hearth devices (download once, copy at home speed) and lets saves follow you"
        value: page.g && page.g.storage ? "On" : ""
        onActivated: SettingsNav.openPage("link")
    }

    SetHeader { text: "Play from anywhere" }
    SetRow {
        icon: "wired"; title: "Remote play and Use this PC from anywhere"
        sub: "Sunshine on this PC, Moonlight on your laptop, work PC, phone or TV: your games, or your whole desktop. Away from home, add this PC in Moonlight by its Tailscale name"
             + (page.g && page.g.tsName ? " (" + page.g.tsName + ")" : "") + ": nothing is opened to the internet"
        value: page.g && page.g.remote ? "On" : ""
        onActivated: SettingsNav.openPage("remote")
    }
    SetRow {
        icon: "user"; chevron: false; interactive: false; title: "Friends over Tailscale"
        sub: "To play your home-network games with a friend somewhere else, share this PC with them from Tailscale's admin page (Share node). They only reach this PC, nothing else at home"
    }

    SetHeader { text: "Light mode while you play" }
    SetSwitch {
        visible: !!page.g
        icon: "sleep"; title: "Light mode while you play"
        sub: page.g && page.g.playing ? (page.g.resumed ? "You're playing; paused jobs were let through for now (Quick Access)" : "You're playing now: the jobs below wait")
             : "When Game Mode or a game runs, big jobs wait. Also in Game Mode's Quick Access"
        checked: !!page.g && page.g.lightMode
        onToggled: v => act.run(["set", "lightMode", v ? "on" : "off"], () => page.refresh(), e => page.say(e, true))
    }
    Repeater {
        model: page.g ? [
            { t: "Waits", s: "Snapshots and emptying recycle bins" + (page.g.snapshots ? "" : " (snapshots are off)") },
            { t: "Waits", s: "Backups" + (page.g.backups ? " (" + page.g.backups + " set up)" : " (none set up)") + ": they run at the next hour after you stop" },
            { t: page.g.ai.on && page.g.ai.unload ? "Waits" : "Keeps running", s: "AI models: " + (page.g.ai.on ? (page.g.ai.pause ? "the AI service stops while you play" : page.g.ai.unload ? "unloaded from the graphics card within a minute" : "stay loaded (AI › While you play)") : "the AI service is off") },
            { t: page.g.media.on && page.g.media.pause ? "Waits" : "Keeps running", s: "The media server: " + (page.g.media.on ? (page.g.media.pause ? "stops while you play" : "keeps streaming (Services › Pause in Game Mode)") : "off") },
            { t: page.g.pictures && page.g.pictures.on && page.g.pictures.pause ? "Waits" : "Keeps running", s: "AI pictures (ComfyUI): " + (page.g.pictures && page.g.pictures.on ? (page.g.pictures.pause ? "stops while you play, so the graphics card is yours, and starts again after" : "keeps running and holds graphics memory (Services › Stop while you play)") : "off") },
            { t: page.g.voices && page.g.voices.on && page.g.voices.pause ? "Waits" : "Keeps running", s: "AI voices (Kokoro): " + (page.g.voices && page.g.voices.on ? (page.g.voices.pause ? "stops while you play" : "keeps running on the CPU") : "off") },
            { t: "Keeps running", s: "Shared folders, Tailscale, folder sync" + (page.g.syncs ? " (" + page.g.syncs + ")" : "") + ", Files on the web and links" }
        ] : []
        delegate: SetRow {
            required property var modelData
            icon: modelData.t === "Waits" ? "clock" : "check"; chevron: false; interactive: false
            title: modelData.t; sub: modelData.s
        }
    }
    Text {
        Layout.fillWidth: true
        text: "\"Let paused jobs run now\" in Quick Access lets them through for up to 4 hours without changing Light mode."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

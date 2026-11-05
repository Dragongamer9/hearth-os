// Hearth Settings › Home server › Services (docs/HOME-SERVER.md): optional apps this PC runs for your devices, each off
// until you turn it on: the media server (Jellyfin; also your photo library), Files on the web, the AI service (its own
// page), Remote play, folder sync and the game archive. Apps that aren't in Debian are downloaded only when you ask,
// from their makers' releases, checked against the SHA-256 they publish. Each is reachable only on your home network
// or through Tailscale, as you choose (the AI service: Tailscale or this PC only), with a status and an Open link.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Services"
    property var sv: null
    property var st: null
    property string msg: ""
    property bool bad: false
    property bool pwOpen: false
    readonly property var cfg: st ? st.settings : null
    readonly property var media: sv ? sv.media : null
    readonly property var web: sv ? sv.web : null
    readonly property var ai: sv ? sv.ai : null
    readonly property bool aiShown: !!st && !!st.seen && !!st.seen.commandCenter && (st.seen.commandCenter.shown || (!!ai && (ai.on || ai.installed)))
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() {
        job.run(["services"], r => page.sv = r, e => page.sv = { error: e });
        job.run(["status"], r => page.st = r, () => {});
    }
    function set(k, v, ok) { act.run(["set", k, v], () => { if (ok) page.say(ok); page.refresh(); }, e => { page.say(e, true); page.refresh(); }); }
    function openUrl(u) { Quickshell.execDetached(["xdg-open", u]); }
    function appLine(a, label) {
        if (!a) return "";
        if (a.installing) return "Downloading " + label + "… " + (a.progress || 0) + "%";
        if (a.error) return "Didn't install: " + a.error;
        if (!a.installed) return "Not installed yet";
        if (!a.on) return "Off · " + label + " " + (a.version || "");
        if (a.paused) return "Paused while you play";
        return (a.active ? "Running" : "Starting…") + " · " + label + " " + (a.version || "");
    }
    Component.onCompleted: refresh()
    Timer { interval: 4000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: "Optional apps this PC runs for your devices. Each one is off until you turn it on, and can pause in Game Mode. Apps that don't come with Debian are downloaded only when you ask, from their makers, and checked before they're installed."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ---------------------------------------------------------------- media server (Jellyfin)
    SetHeader { text: "Media server" }
    SetRow {
        visible: !!page.media && !page.media.installed
        icon: "display"; title: page.media && page.media.installing ? "Installing Jellyfin…" : "Install the media server (Jellyfin)"
        sub: page.media && page.media.installing ? page.appLine(page.media, "Jellyfin")
             : (page.media && page.media.error ? "Didn't install: " + page.media.error + ". Try again. " : "")
               + "Stream your movies, shows, music and photos to TVs, phones and Hearth devices. About 200 MB from jellyfin.org, its checksum checked"
        onActivated: if (page.media && !page.media.installing) act.run(["app-install", "jellyfin"], () => { page.say("Downloading Jellyfin…"); page.refresh(); }, e => page.say(e, true))
    }
    SetSwitch {
        visible: !!page.media && page.media.installed
        icon: "display"; title: "Media server"
        sub: page.appLine(page.media, "Jellyfin") + ". Jellyfin: your movies, shows, music and photos on TVs, phones and Hearth devices"
        checked: !!page.media && page.media.on
        onToggled: v => page.set("mediaOn", v ? "on" : "off", v ? "Media server on: open it to add your shared folders as libraries" : "Media server off")
    }
    SetChoice {
        visible: !!page.media && page.media.installed && page.media.on
        title: "Reachable"
        sub: page.cfg && page.cfg.tsOnly ? "Only through Tailscale for everything (Tailscale page)" : "Home network: your TVs find it. Tailscale only: just your own devices, anywhere"
        options: [{ value: "home", label: "Home network and Tailscale", disabled: !!page.cfg && page.cfg.tsOnly }, { value: "tailscale", label: "Tailscale only" }]
        current: page.cfg && page.cfg.tsOnly ? "tailscale" : (page.media ? page.media.reach : "home")
        onChosen: v => page.set("mediaReach", v)
    }
    SetSwitch {
        visible: !!page.media && page.media.installed && page.media.on
        icon: "pad"; title: "Pause in Game Mode"
        sub: "Stops the media server while Game Mode or a game runs (with Light mode on), and starts it again after. Off: people can keep watching while you play"
        checked: !!page.media && page.media.pause
        onToggled: v => page.set("mediaPause", v ? "on" : "off")
    }
    Repeater {
        model: page.media && page.media.on && page.media.installed ? page.media.urls : []
        delegate: SetRow {
            required property var modelData
            required property int index
            icon: "open"; title: index === 0 ? "Open the media server" : "Open"
            sub: modelData + (modelData.indexOf("localhost") >= 0 ? " (on this PC)" : modelData.indexOf(".ts.net") >= 0 ? " (through Tailscale)" : " (home network)")
            onActivated: page.openUrl(modelData)
        }
    }
    SetRow {
        visible: !!page.media && page.media.installed
        icon: "folder"; title: "Photo library"
        sub: "Your photos open in Jellyfin too: in its first steps (or Dashboard › Libraries) add your Photos folder as a \"Photos\" library. Phone photos get here with Backups & sync › Phone photos"
        onActivated: SettingsNav.openPage("hs-backups")
    }
    SetRow {
        visible: !!page.media && page.media.installed && !page.media.on
        icon: "trash"; title: "Remove Jellyfin"
        sub: "The app goes; your libraries and its settings (in /var/lib/hearth/jellyfin) stay"
        onActivated: act.run(["app-remove", "jellyfin"], () => { page.say("Jellyfin removed. Its data stays"); page.refresh(); }, e => page.say(e, true))
    }

    // ---------------------------------------------------------------- files on the web
    SetHeader { text: "Files on the web" }
    SetSwitch {
        visible: !!page.web
        icon: "globe"; title: "Files on the web"
        sub: page.web && page.web.on ? (page.web.active ? "On: your shared folders in any web browser, read only, with your user name (" + (page.st ? page.st.user : "") + ") and web password" : "Starting…")
             : "Open your shared folders in a web browser on any of your devices, read only, with a password"
        checked: !!page.web && page.web.on
        onToggled: v => { page.set("webOn", v ? "on" : "off"); if (v && page.web && !page.web.pwSet) page.pwOpen = true; }
    }
    SetRow {
        visible: !!page.web && page.web.on
        icon: "lock"; title: page.web && page.web.pwSet ? "Change the web password" : "Set a web password"
        sub: page.web && page.web.pwSet ? "Set. Only its hash is kept on this PC" : "Needed before it opens: at least 8 characters"
        onActivated: page.pwOpen = !page.pwOpen
    }
    SetField {
        visible: page.pwOpen
        secret: true
        placeholder: "A web password (at least 8 characters), then Enter"
        onSubmitted: t => {
            if (t.length < 8) { page.say("Use at least 8 characters", true); return; }
            act.run(["web-password"], () => { page.say("Web password set ✓"); page.pwOpen = false; page.refresh(); }, e => page.say(e, true), t);
            text = "";
        }
    }
    SetChoice {
        visible: !!page.web && page.web.on
        title: "Reachable (also share links and the drop-off)"
        options: [{ value: "tailscale", label: "Tailscale only" }, { value: "home", label: "Home network and Tailscale", disabled: !!page.cfg && page.cfg.tsOnly }]
        current: page.cfg && page.cfg.tsOnly ? "tailscale" : (page.web ? page.web.reach : "tailscale")
        onChosen: v => page.set("webReach", v)
    }
    Repeater {
        model: page.web && page.web.on ? page.web.urls : []
        delegate: SetRow {
            required property var modelData
            icon: "open"; title: "Open"; sub: modelData
            onActivated: page.openUrl(modelData)
        }
    }
    SetRow {
        visible: !!page.web && page.web.on && page.web.urls.length === 0
        icon: "globe"; chevron: false; interactive: false; title: "Waiting for Tailscale"
        sub: "Turn on Tailscale and sign in (Tailscale page), or pick Home network"
    }

    // ---------------------------------------------------------------- AI (its own page)
    SetHeader { text: "AI voices, pictures & chat"; visible: page.aiShown || (!!page.cfg && page.cfg.showHidden) }
    SetRow {
        visible: page.aiShown || (!!page.cfg && page.cfg.showHidden)
        icon: "mic"; title: "AI service"
        sub: (page.ai ? page.appLine(page.ai, "Ollama") : "") + ". For the Hearth AI Command Center: chat models on the graphics card, reachable only through Tailscale or on this PC"
        value: page.ai && page.ai.on ? (page.ai.active ? "On" : "Starting") : ""
        onActivated: SettingsNav.openPage("hs-ai")
    }

    // ---------------------------------------------------------------- the others, which have their own pages
    SetHeader { text: "Also on this PC" }
    SetRow {
        icon: "wired"; title: "Remote play"
        sub: "Sunshine: play this PC's games on your laptop, phone or TV with Moonlight, at home or through Tailscale"
        value: page.sv && page.sv.remote ? "On" : ""
        onActivated: SettingsNav.openPage("remote")
    }
    SetRow {
        icon: "update"; title: "Folder sync"
        sub: "Syncthing: keep a folder the same on your Hearth devices, like a private Dropbox"
        value: page.sv && page.sv.syncs ? page.sv.syncs + (page.sv.syncs === 1 ? " folder" : " folders") : ""
        onActivated: SettingsNav.openPage("hs-backups")
    }
    SetRow {
        icon: "store"; title: "Game archive"
        sub: "Hearth Storage: move games here to free space on your Hearth devices, and bring them back any time"
        value: page.sv && page.sv.storage ? "On" : ""
        onActivated: SettingsNav.openPage("link")
    }

    Text {
        visible: !!page.st && !!page.st.applied && page.st.applied.ok === false
        Layout.fillWidth: true
        text: page.st && page.st.applied ? "Saved, but the system couldn't be changed to match: " + (page.st.applied.error || "") : ""
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && !!page.st.applied && !!page.st.applied.ok && !!page.st.applied.result && !!page.st.applied.result.apps && !!page.st.applied.result.apps.mediaNote
        Layout.fillWidth: true
        text: page.st && page.st.applied && page.st.applied.result && page.st.applied.result.apps ? (page.st.applied.result.apps.mediaNote || "") : ""
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

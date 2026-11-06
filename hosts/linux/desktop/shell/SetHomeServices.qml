// Hearth Settings › Home server › Services (docs/HOME-SERVER.md): optional apps this PC runs for your devices, each off
// until you turn it on: the media server (Jellyfin; also your photo library), Files on the web, the AI service (its own
// page), Remote play, folder sync and the game archive. Apps that aren't in Debian are downloaded only when you ask,
// from their makers' releases, checked against the SHA-256 they publish. Each is reachable only on your home network
// or through Tailscale, as you choose (the AI service: Tailscale or this PC only), with a status and an Open link.
// AI pictures (ComfyUI, on the graphics card) and AI voices (Kokoro-FastAPI, on the CPU) are always shown here: your own
// user services, installed only when you ask, Tailscale and this PC only; your own copies can be taken over and given back.
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
    readonly property var pic: sv ? sv.pictures : null
    readonly property var voi: sv ? sv.voices : null
    readonly property bool picOwnerRuns: !!pic && !!pic.owner && (pic.owner.active || pic.owner.enabled) && !pic.takenOver
    property int modelsRm: 0
    readonly property bool aiShown: !!st && !!st.seen && !!st.seen.commandCenter && (st.seen.commandCenter.shown || (!!ai && (ai.on || ai.installed)))
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() {
        job.run(["services"], r => page.sv = r, e => page.sv = { error: e });
        job.run(["status"], r => page.st = r, () => {});
    }
    function set(k, v, ok) { act.run(["set", k, v], () => { if (ok) page.say(ok); page.refresh(); }, e => { page.say(e, true); page.refresh(); }); }
    function openUrl(u) { Quickshell.execDetached(["xdg-open", u]); }
    function showPath(p) { Quickshell.execDetached(["python3", decodeURIComponent(Qt.resolvedUrl("hearth-portal").toString().replace(/^file:\/\//, "")), "show", p]); }
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
    onVisibleChanged: if (!visible) modelsRm = 0
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

    // ---------------------------------------------------------------- AI pictures (ComfyUI): always shown
    SetHeader { text: "AI pictures (ComfyUI)" }
    SetRow {
        visible: !!page.pic && page.pic.installing
        icon: "palette"; chevron: false; interactive: false
        title: "Installing ComfyUI… " + (page.pic ? page.pic.progress || 0 : 0) + "%"
        sub: page.pic ? (page.pic.step || "starting") + ". It keeps going if you leave this page" : ""
    }
    SetRow {
        visible: page.picOwnerRuns
        icon: "palette"; title: "Take over your own ComfyUI"
        sub: "Your ComfyUI in ~/ComfyUI runs from your own script (comfyui.service" + (page.pic && page.pic.owner.active ? ", running now" : "") + "). Take it over: Hearth runs the same folder and models, Tailscale only, and stops it while you play. Your service is switched off, never deleted; Give it back undoes this"
        value: "Take over"
        onActivated: act.run(["pictures-takeover"], r => { page.say(r.note || "Hearth runs your ComfyUI now"); page.refresh(); }, e => page.say(e, true))
    }
    SetRow {
        visible: !!page.pic && !page.pic.installed && !page.pic.installing && !page.picOwnerRuns
        icon: "palette"; title: page.pic && page.pic.existing ? "Use your ComfyUI (~/ComfyUI)" : "Install AI pictures (ComfyUI)"
        sub: (page.pic && page.pic.error ? "Didn't install: " + page.pic.error + ". Try again. " : "")
             + (page.pic && page.pic.existing ? "Hearth finishes what's missing (its Python environment, PyTorch for CUDA 12.8) and leaves your files and models where they are"
                : "Make pictures on the RTX graphics card from your other devices. ComfyUI " + (page.pic ? page.pic.tag : "") + " from GitHub (its release checked), PyTorch for CUDA 12.8 from pytorch.org and what ComfyUI needs: about 10 GB, into ~/ComfyUI. No models are downloaded")
             + (page.pic && !page.pic.git ? ". Needs git and Python's venv (the next Hearth OS update brings them)" : "")
        onActivated: act.run(["pictures-install"], () => { page.say("Installing ComfyUI… it takes a while (PyTorch is a few GB)"); page.refresh(); }, e => page.say(e, true))
    }
    SetSwitch {
        visible: !!page.pic && page.pic.installed
        icon: "palette"; title: "AI pictures"
        sub: !page.pic ? "" : !page.pic.on ? "Off · ComfyUI " + (page.pic.version || "") + " is installed in ~/ComfyUI"
             : page.pic.paused ? "Stopped while you play (Light mode); it starts again after"
             : page.pic.active ? (page.pic.healthy ? "Running · " + (page.pic.device || "ComfyUI " + page.pic.version) : "Starting… (ComfyUI takes a moment to load)")
             : "Starting…"
        checked: !!page.pic && page.pic.on
        onToggled: v => page.set("picturesOn", v ? "on" : "off", v ? "AI pictures on: only your devices through Tailscale, and this PC" : "AI pictures off")
    }
    SetRow {
        visible: !!page.pic && page.pic.installed && page.pic.on
        icon: "lock"; chevron: false; interactive: false; title: "Who can reach it"
        sub: "Your devices through Tailscale, and this PC. Never your home network: ComfyUI listens only on this PC" + (page.pic && page.pic.listen.length > 1 ? " and " + page.pic.listen[1] : page.pic && page.pic.listen.length === 1 ? " (Tailscale wasn't up when it started: turn it off and on again once it is)" : " and its Tailscale address") + ", and Hearth's firewall rule lets only Tailscale reach port 8188. ComfyUI has no password, so it stays this way"
    }
    SetRow {
        visible: !!page.pic && page.pic.installed && page.pic.on
        icon: "open"; title: "Open ComfyUI"
        sub: page.pic && page.pic.url ? page.pic.url + " (from your devices on Tailscale) · on this PC: " + page.pic.local : "On this PC: " + (page.pic ? page.pic.local : "") + ". Turn on Tailscale (Tailscale page) for your other devices"
        onActivated: page.openUrl(page.pic && page.pic.url ? page.pic.url : page.pic.local)
    }
    SetRow {
        visible: !!page.pic && page.pic.installed
        icon: "folder"; title: "Models folder"
        sub: (page.pic && page.pic.modelFiles.length ? page.pic.modelFiles.length + (page.pic.modelFiles.length === 1 ? " model: " : " models: ") + page.pic.modelFiles.slice(0, 3).join(", ") + (page.pic.modelFiles.length > 3 ? "…" : "") + ". "
              : "No models yet. ") + "Put checkpoints (.safetensors) in ~/ComfyUI/models/checkpoints and pick one in ComfyUI's Load Checkpoint box. Hearth never downloads models"
        onActivated: if (page.pic) page.showPath(page.pic.models)
    }
    SetSwitch {
        visible: !!page.pic && page.pic.installed
        icon: "pad"; title: "Stop it while I play"
        sub: "ComfyUI holds graphics memory: with Light mode on it stops when Game Mode or a game starts and starts again after. Off: it keeps the memory while you play"
        checked: !!page.pic && page.pic.pause
        onToggled: v => page.set("picturesPause", v ? "on" : "off")
    }
    SetRow {
        visible: !!page.pic && page.pic.installed && !page.pic.installing
        icon: "update"; title: "Update ComfyUI"
        sub: "To " + (page.pic ? page.pic.tag : "") + " (the release this Hearth version checks) and its requirements. Files you changed in ComfyUI stop it instead of being written over"
             + (page.pic && page.pic.torch ? " · PyTorch " + page.pic.torch + (page.pic.gpu ? " on " + page.pic.gpu : " (it didn't see the graphics card)") : "")
        onActivated: act.run(["pictures-update"], () => { page.say("Updating ComfyUI…"); page.refresh(); }, e => page.say(e, true))
    }
    SetRow {
        visible: !!page.pic && page.pic.takenOver
        icon: "back"; title: "Give your own ComfyUI service back"
        sub: "Hearth's turns off and your comfyui.service comes back on, the way it was before"
        onActivated: act.run(["pictures-giveback"], () => { page.say("Your own ComfyUI service is back"); page.refresh(); }, e => page.say(e, true))
    }
    SetRow {
        visible: !!page.pic && page.pic.installed && !page.pic.on && !page.picOwnerRuns
        icon: "trash"; title: "Remove ComfyUI"
        sub: "ComfyUI and its Python environment go. Your models, pictures (output) and workflows stay in ~/ComfyUI"
        onActivated: act.run(["pictures-remove"], () => { page.say("ComfyUI removed. Your models and pictures stay"); page.refresh(); }, e => page.say(e, true))
    }
    SetRow {
        visible: !!page.pic && page.pic.installed && !page.pic.on && !page.picOwnerRuns
        icon: "trash"
        title: page.modelsRm === 0 ? "Remove ComfyUI and its models" : page.modelsRm === 1 ? "Sure? Your models will be deleted" : "Really delete " + (page.pic ? page.pic.modelFiles.length : 0) + " models? This can't be undone"
        sub: page.modelsRm === 0 ? "Also deletes ~/ComfyUI/models (asks twice). Your pictures and workflows still stay"
             : page.modelsRm === 1 ? "Select again to go on, or leave this page to keep them" : "Select once more to delete them now"
        onActivated: {
            if (page.modelsRm < 2) { page.modelsRm += 1; return; }
            page.modelsRm = 0;
            act.run(["pictures-remove", "models", "sure", "really-sure"], () => { page.say("ComfyUI and its models removed"); page.refresh(); }, e => page.say(e, true));
        }
    }

    // ---------------------------------------------------------------- AI voices (Kokoro-FastAPI): always shown
    SetHeader { text: "AI voices (Kokoro)" }
    SetRow {
        visible: !!page.voi && page.voi.installing
        icon: "vol"; chevron: false; interactive: false
        title: "Installing Kokoro… " + (page.voi ? page.voi.progress || 0 : 0) + "%"
        sub: page.voi ? (page.voi.step || "starting") : ""
    }
    SetRow {
        visible: !!page.voi && !!page.voi.owner && page.voi.owner.active && !page.voi.takenOver
        icon: "vol"; title: "Take over your own Kokoro"
        sub: "Your \"kokoro\" container is running, and it answers your home network too. Take it over: it's stopped (never deleted; Give it back starts it again) and Hearth runs its own copy, Tailscale only"
        value: "Take over"
        onActivated: act.run(["voices-takeover"], r => { page.say(r.note || "Hearth runs Kokoro now"); page.refresh(); }, e => page.say(e, true))
    }
    SetRow {
        visible: !!page.voi && !page.voi.installed && !page.voi.installing
        icon: "vol"; title: "Install AI voices (Kokoro)"
        interactive: !!page.voi && page.voi.pinned && page.voi.podman
        sub: (page.voi && page.voi.error ? "Didn't install: " + page.voi.error + ". Try again. " : "")
             + (page.voi && !page.voi.pinned ? "Comes with the next Hearth OS update"
                : "Natural voices for the Hearth AI Command Center. Kokoro-FastAPI " + (page.voi ? page.voi.tag : "") + " for the CPU (no graphics card needed), in Podman, by the exact image Hearth pins (Podman checks it): a few GB")
        onActivated: if (page.voi && page.voi.pinned && page.voi.podman) act.run(["voices-install"], () => { page.say("Downloading Kokoro…"); page.refresh(); }, e => page.say(e, true))
    }
    SetSwitch {
        visible: !!page.voi && page.voi.installed
        icon: "vol"; title: "AI voices"
        sub: !page.voi ? "" : !page.voi.on ? "Off · Kokoro-FastAPI " + (page.voi.version || page.voi.tag) + " is installed"
             : page.voi.paused ? "Stopped while you play (Light mode); it starts again after"
             : page.voi.active ? (page.voi.healthy ? "Running · " + (page.voi.voices ? page.voi.voices + " voices" : "ready") : "Starting… (loading the voice model)")
             : "Starting…"
        checked: !!page.voi && page.voi.on
        onToggled: v => page.set("voicesOn", v ? "on" : "off", v ? "AI voices on: only your devices through Tailscale, and this PC" : "AI voices off")
    }
    SetRow {
        visible: !!page.voi && page.voi.installed
        icon: "clipboard"; title: "Paste this into the Command Center"
        sub: page.voi && page.voi.url ? page.voi.url + " · in Settings › Voice › Speech servers › Kokoro-FastAPI"
             : "Turn on Tailscale and sign in (Tailscale page) to get the address for your other devices. On this PC: " + (page.voi ? page.voi.local : "")
        value: page.voi && page.voi.url ? "Copy" : ""
        onActivated: if (page.voi && page.voi.url) { Quickshell.execDetached(["wl-copy", page.voi.url]); page.say("Copied ✓"); }
    }
    SetRow {
        visible: !!page.voi && page.voi.installed && page.voi.on
        icon: "lock"; chevron: false; interactive: false; title: "Who can reach it"
        sub: "Your devices through Tailscale, and this PC: it listens only on this PC" + (page.voi && page.voi.listen.length > 1 ? " and " + page.voi.listen[1] : page.voi && page.voi.listen.length === 1 ? " (Tailscale wasn't up when it started: turn it off and on again once it is)" : " and its Tailscale address") + ", and Hearth's firewall rule lets only Tailscale reach port 8880"
    }
    SetSwitch {
        visible: !!page.voi && page.voi.installed
        icon: "pad"; title: "Stop it while I play"
        sub: "Kokoro runs on the CPU and leaves the graphics card alone, so it can keep talking while you play. On: it stops when Game Mode or a game starts (with Light mode on)"
        checked: !!page.voi && page.voi.pause
        onToggled: v => page.set("voicesPause", v ? "on" : "off")
    }
    SetRow {
        visible: !!page.voi && page.voi.takenOver
        icon: "back"; title: "Give your own Kokoro back"
        sub: "Hearth's turns off and your \"kokoro\" container starts again if it was running"
        onActivated: act.run(["voices-giveback"], () => { page.say("Your own Kokoro is back"); page.refresh(); }, e => page.say(e, true))
    }
    SetRow {
        visible: !!page.voi && page.voi.installed && !page.voi.on
        icon: "trash"; title: "Remove Kokoro"
        sub: "Hearth's copy and its image go. Your own \"kokoro\" container, if you have one, is never touched"
        onActivated: act.run(["voices-remove"], r => { page.say(r.note || "Kokoro removed"); page.refresh(); }, e => page.say(e, true))
    }

    // ---------------------------------------------------------------- AI (its own page)
    SetHeader { text: "AI chat"; visible: page.aiShown || (!!page.cfg && page.cfg.showHidden) }
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

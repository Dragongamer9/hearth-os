// Hearth Settings › Home server › AI (docs/HOME-SERVER.md): the AI service (Ollama) on this PC's graphics card for the
// Hearth AI Command Center's "Your own GPU server" teammate. Reachable only through Tailscale or on this PC. Ready-made
// models sized for an RTX 5070 (12 GB) with 32 GB RAM, downloaded only when you pick one; the address to paste into the
// Command Center; Light mode (models unloaded, or the service stopped, while Game Mode or a game runs). Command Center
// items are hidden until it was seen on one of your devices within your window (Overview › Show hidden items shows
// them anyway; display only, nothing is turned on or off by hiding). AI pictures (ComfyUI) and AI voices (Kokoro) sit
// next to the chat models and are always shown (they're set up in Services).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "AI"
    property var ai: null
    property string msg: ""
    property bool bad: false
    property string answer: ""
    property string askModel: ""
    readonly property var svc: ai ? ai.ai : null
    readonly property var cc: ai ? ai.commandCenter : null
    readonly property bool ccShown: !!cc && cc.shown
    readonly property bool shown: ccShown || (!!svc && (svc.on || svc.installed))
    readonly property var have: ai ? ai.models.filter(m => m.have) : []
    readonly property var pic: ai ? ai.pictures : null
    readonly property var voi: ai ? ai.voices : null
    function appLine(a, what) {
        if (!a) return "";
        if (a.installing) return "Installing… " + (a.progress || 0) + "%";
        if (!a.installed) return "Not installed yet: install it in Services";
        if (!a.on) return "Off";
        if (a.paused) return "Stopped while you play; it starts again after";
        return a.active ? (a.healthy ? "Running" : "Starting…") : "Starting…";
    }
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["ai"], r => { page.ai = r; if (!page.askModel && page.have.length) page.askModel = page.have[0].id; }, e => page.ai = { error: e, models: [] }); }
    function set(k, v, ok) { act.run(["set", k, v], () => { if (ok) page.say(ok); page.refresh(); }, e => { page.say(e, true); page.refresh(); }); }
    function ago(ms) {
        if (!ms) return "never";
        const h = Math.round((Date.now() - ms) / 3600000);
        return h < 1 ? "just now" : h < 48 ? h + " h ago" : Math.round(h / 24) + " days ago";
    }
    Component.onCompleted: refresh()
    Timer { interval: 3000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: "A local AI on this PC's graphics card for the Hearth AI Command Center. It answers only your own devices (through Tailscale) and this PC. Nothing leaves your home."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ---- Share this device's AI (docs/AI-SHARE.md): its own page, SetHomeAIDevices.qml
    SetRow {
        icon: "desktops"; title: "Share this device's AI · AI devices"
        sub: "Let your other devices and the Command Center use this PC's AI through Tailscale, and see every device's AI in one list"
        onActivated: SettingsNav.openPage("hs-aidevices")
    }
    // ---- end Share this device's AI

    // Hidden until the Command Center was seen (display only).
    SetRow {
        visible: !!page.ai && !page.shown
        icon: "info"; chevron: false; interactive: false; title: "Shows once the Command Center is seen"
        sub: "These items appear when the Hearth AI Command Center was used on one of your devices in the last " + (page.ai ? page.ai.ccDays : 30)
             + " days: on this PC, or when it talks to this PC's AI service. Overview › Show hidden items shows them anyway. Hiding never turns anything off."
    }

    // ---------------------------------------------------------------- the service
    SetHeader { text: "The AI service"; visible: page.shown }
    SetRow {
        visible: page.shown && !!page.svc && !page.svc.installed
        icon: "mic"; title: page.svc && page.svc.installing ? "Installing Ollama… " + (page.svc.progress || 0) + "%" : "Install the AI service (Ollama)"
        sub: page.svc && page.svc.installing ? "From Ollama's own release on GitHub, its SHA-256 checked before anything is unpacked"
             : (page.svc && page.svc.error ? "Didn't install: " + page.svc.error + ". Try again. " : "")
               + "About 2 GB from Ollama's GitHub release, checked against the checksum Ollama publishes. Models come separately, only the ones you pick"
        onActivated: if (page.svc && !page.svc.installing) act.run(["app-install", "ollama"], () => { page.say("Downloading Ollama…"); page.refresh(); }, e => page.say(e, true))
    }
    SetSwitch {
        visible: page.shown && !!page.svc && page.svc.installed
        icon: "mic"; title: "AI service"
        sub: !page.svc ? "" : !page.svc.on ? "Off. Ollama " + (page.svc.version || "") + " is installed"
             : page.svc.paused ? "Stopped while you play (Light mode); it starts again after"
             : page.svc.active ? "Running · Ollama " + (page.svc.version || "") + (page.ai.loaded.length ? " · loaded now: " + page.ai.loaded.join(", ") : " · graphics card idle")
             : "Starting…"
        checked: !!page.svc && page.svc.on
        onToggled: v => page.set("aiOn", v ? "on" : "off", v ? "AI service on: reachable through Tailscale and on this PC only" : "AI service off")
    }
    SetRow {
        visible: page.shown && !!page.svc && page.svc.on
        icon: "lock"; chevron: false; interactive: false; title: "Who can reach it"
        sub: "Your devices through Tailscale, and this PC. Never your home network or the internet: Hearth's firewall rule lets only Tailscale in, and without it the service listens on this PC only"
    }

    // ---------------------------------------------------------------- models
    SetHeader { text: "Models"; visible: page.shown && !!page.svc && page.svc.installed }
    Repeater {
        model: page.shown && page.svc && page.svc.installed ? page.ai.models : []
        delegate: SetRow {
            required property var modelData
            icon: modelData.have ? "check" : "plus"
            title: modelData.label + (modelData.recommended ? " · recommended" : "")
            sub: modelData.note + " · " + modelData.id + " · " + modelData.size
                 + (modelData.pulling ? " · downloading " + modelData.pulling.pct + "%" : modelData.error ? " · didn't download: " + modelData.error : "")
            value: modelData.have ? (modelData.loaded ? "Loaded" : "Downloaded") : modelData.pulling ? modelData.pulling.pct + "%" : "Download"
            onActivated: {
                if (modelData.pulling) return;
                if (!modelData.have) act.run(["ai-pull", modelData.id], () => { page.say("Downloading " + modelData.label + "… (Ollama checks every part)"); page.refresh(); }, e => page.say(e, true));
                else page.askModel = modelData.id;
            }
        }
    }
    SetRow {
        visible: page.shown && !!page.svc && page.svc.installed && !!page.askModel && page.have.some(m => m.id === page.askModel)
        icon: "trash"; title: "Remove " + page.askModel
        sub: "Frees its space; you can download it again any time. Select another downloaded model above to pick it"
        onActivated: act.run(["ai-remove", page.askModel], () => { page.say("Removed " + page.askModel); page.askModel = ""; page.refresh(); }, e => page.say(e, true))
    }
    Text {
        visible: page.shown && !!page.ai && !!page.ai.others && page.ai.others.length > 0
        Layout.fillWidth: true
        text: page.ai && page.ai.others ? "Also here: " + page.ai.others.join(", ") : ""
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }

    // ---------------------------------------------------------------- pictures and voices (always shown; set up in Services)
    SetHeader { text: "Pictures and voices" }
    SetRow {
        visible: !!page.pic
        icon: "palette"; title: "AI pictures (ComfyUI)"
        sub: page.appLine(page.pic) + ". Pictures on the graphics card, from your devices through Tailscale"
             + (page.pic && page.pic.installed ? " · " + page.pic.modelFiles.length + (page.pic.modelFiles.length === 1 ? " model" : " models") : "")
        value: page.pic && page.pic.on ? "On" : ""
        onActivated: SettingsNav.openPage("hs-services")
    }
    SetRow {
        visible: !!page.pic && page.pic.installed && page.pic.on
        icon: "open"; title: "Open ComfyUI"
        sub: page.pic && page.pic.url ? page.pic.url : (page.pic ? page.pic.local + " (on this PC)" : "")
        onActivated: Quickshell.execDetached(["xdg-open", page.pic.url || page.pic.local])
    }
    SetRow {
        visible: !!page.voi
        icon: "vol"; title: "AI voices (Kokoro)"
        sub: page.appLine(page.voi) + ". Natural voices for the Command Center, on the CPU"
        value: page.voi && page.voi.on ? "On" : ""
        onActivated: SettingsNav.openPage("hs-services")
    }
    SetRow {
        visible: !!page.voi && page.voi.installed
        icon: "clipboard"; title: "Voices address for the Command Center"
        sub: page.voi && page.voi.url ? page.voi.url + " · Settings › Voice › Speech servers › Kokoro-FastAPI" : "Turn on Tailscale for the address; on this PC: " + (page.voi ? page.voi.local : "")
        value: page.voi && page.voi.url ? "Copy" : ""
        onActivated: if (page.voi && page.voi.url) { Quickshell.execDetached(["wl-copy", page.voi.url]); page.say("Copied ✓"); }
    }

    // ---------------------------------------------------------------- ask it
    SetHeader { text: "Ask the server"; visible: page.shown && page.have.length > 0 && !!page.svc && page.svc.active }
    SetField {
        visible: page.shown && page.have.length > 0 && !!page.svc && page.svc.active
        placeholder: "Ask " + (page.askModel || "a model") + " something, then Enter"
        onSubmitted: t => {
            page.answer = "Thinking…";
            act.run(["ai-ask", page.askModel], r => page.answer = r.answer || "(no answer)", e => page.answer = e, t);
        }
    }
    Text {
        visible: page.answer !== ""
        Layout.fillWidth: true
        text: page.answer; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ---------------------------------------------------------------- for the Command Center (hidden until seen)
    SetHeader { text: "For the Command Center"; visible: page.ccShown }
    SetRow {
        visible: page.ccShown
        icon: "info"; chevron: false; interactive: false; title: "Seen " + page.ago(page.cc ? page.cc.lastSeen : 0) + (page.cc && page.cc.device ? " on " + page.cc.device : "")
        sub: "Shown because the Command Center was used on one of your devices in the last " + (page.ai ? page.ai.ccDays : 30) + " days"
    }
    SetRow {
        visible: page.ccShown && !!page.ai
        icon: "globe"; title: "Paste this into the Command Center"
        sub: page.ai && page.ai.address ? page.ai.address + " · in Settings › Accounts › Your own GPU server: type \"OpenAI-compatible chat\", no key, model " + (page.askModel || "qwen3:30b-a3b")
             : "Turn on Tailscale and sign in (Tailscale page) to get the address for your other devices. On this PC: " + (page.ai ? page.ai.local : "")
        value: page.ai && page.ai.address ? "Copy" : ""
        onActivated: if (page.ai && page.ai.address) { Quickshell.execDetached(["wl-copy", page.ai.address]); page.say("Copied ✓"); }
    }
    SetRow {
        visible: page.ccShown
        icon: "vol"; title: "Voices for the Command Center"
        sub: page.voi && page.voi.installed ? (page.voi.url ? page.voi.url : page.voi.local) + " in Settings › Voice › Speech servers › Kokoro-FastAPI" + (page.voi.on ? "" : " (turn AI voices on in Services)")
             : "AI voices (Kokoro) in Services gives the Command Center natural voices from this PC"
        onActivated: SettingsNav.openPage("hs-services")
    }

    // ---------------------------------------------------------------- Light mode
    SetHeader { text: "While you play"; visible: page.shown && !!page.svc && page.svc.installed }
    SetSwitch {
        visible: page.shown && !!page.svc && page.svc.installed
        icon: "pad"; title: "Unload models when you play"
        sub: "When Game Mode or a game starts (with Light mode on), loaded models leave the graphics card within a minute; the next question loads them again"
             + (page.ai && page.ai.unloadedAt ? " · last " + page.ago(page.ai.unloadedAt * 1000) : "")
        checked: !!page.svc && page.svc.unload
        onToggled: v => page.set("aiUnload", v ? "on" : "off")
    }
    SetSwitch {
        visible: page.shown && !!page.svc && page.svc.installed
        icon: "sleep"; title: "Stop the AI service while I play"
        sub: "Also stops it (your other devices can't ask it then), and starts it again when you stop playing"
        checked: !!page.svc && page.svc.pause
        onToggled: v => page.set("aiPause", v ? "on" : "off")
    }
    SetRow {
        visible: page.shown && !!page.svc && page.svc.active && (page.ai.loaded || []).length > 0
        icon: "restart"; title: "Unload models now"
        sub: "Frees the graphics card right away"
        onActivated: act.run(["ai-unload"], r => { page.say("Unloaded " + (r.unloaded || []).join(", ")); page.refresh(); }, e => page.say(e, true))
    }
    SetRow {
        visible: page.shown && !!page.svc && page.svc.installed && !page.svc.on
        icon: "trash"; title: "Remove Ollama"
        sub: "The app goes; your downloaded models stay (in /var/lib/hearth-ollama)"
        onActivated: act.run(["app-remove", "ollama"], () => { page.say("Ollama removed. Your models stay"); page.refresh(); }, e => page.say(e, true))
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

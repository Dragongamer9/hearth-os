// Hearth Settings › Remote play: stream this device's games to a TV, phone, tablet or another PC with Moonlight
// (Sunshine, which comes with Hearth OS), and play another PC's games here. Moved from the archived Game Mode's
// Settings › Connections › Remote play, with the same calls (hearth-core remote.rs through HearthCall).
// "Use this PC from anywhere" (docs/REMOTE-DESKTOP.md) is the card at the top: one switch for low-latency remote
// desktop through Tailscale, pairing with Moonlight's PIN, who's connected (direct or relay) and staying awake; its
// helper is hearth-remote-desktop (settings in ~/.config/nimbus/remote-desktop.json, never a password).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Remote play"
    property var st: null          // remote.status: {installed, on, clients, needsLogin, addresses, moonlight}
    property string err: ""
    property string msg: ""
    property string busy: ""       // "on", "pair", "login", "moonlight"
    property bool pairing: false   // the PIN and name boxes are open
    property bool login: false     // Sunshine's own username and password boxes are open
    HearthCall { id: core }
    property var rd: null          // hearth-remote-desktop status (Use this PC from anywhere)
    property string rdMsg: ""
    property bool rdBad: false
    property bool rdPairing: false
    property var rdCheck: null     // the last "Check the connection": {path, ms, relay, hint, tip}
    readonly property bool rdOn: !!page.rd && !!page.rd.on
    SetJob { id: rdJob; helper: "hearth-remote-desktop" }
    SetJob { id: rdAct; helper: "hearth-remote-desktop" }
    function rdLoad() { rdJob.run(["status"], r => page.rd = r, e => page.rd = { error: e }); }
    function rdSay(t, bad) { page.rdMsg = t; page.rdBad = !!bad; }
    function rdRun(args, said, label) {
        rdAct.run(args, r => { page.rdSay(said + (r && r.note ? " " + r.note : ""), !!(r && r.note)); page.rdLoad(); page.load(); },
                  e => { page.rdSay(e, true); page.rdLoad(); page.load(); }, "", label || "");
    }
    function addrLine() {
        if (!page.rd || !page.rd.address) return "";
        const a = page.rd.address;
        if (!a.name && !a.ip) return "";
        return a.name && a.ip ? a.name + " or " + a.ip : (a.name || a.ip);
    }
    function since(t) {
        if (!t) return "";
        const m = Math.max(0, Math.round((Date.now() / 1000 - t) / 60));
        return m < 1 ? "just now" : m < 60 ? m + " min" : Math.floor(m / 60) + " h " + (m % 60) + " min";
    }
    function streamLine(s) {
        const parts = [];
        if (s.width && s.height) parts.push(s.width + "×" + s.height + (s.fps ? " at " + s.fps + " fps" : ""));
        if (s.mbps) parts.push(s.mbps + " Mbps");
        if (s.path !== "unknown") parts.push(s.path === "direct" ? "direct" : s.path === "relay" ? "through a relay" + (s.relay ? " (" + s.relay + ")" : "") : "home network");
        if (s.since) parts.push("for " + page.since(s.since));
        return parts.join(" · ");
    }
    Timer { interval: 5000; running: page.visible && page.rdOn; repeat: true; onTriggered: if (!rdJob.busy && !rdAct.busy) page.rdLoad() }
    function load() { core.call("remote.status", {}, v => { page.st = v; page.err = ""; }, e => page.err = e); }
    function act(method, args, k, said) {
        page.busy = k; page.msg = "";
        core.call(method, args, () => { page.busy = ""; if (said) page.msg = said; page.load(); }, e => { page.busy = ""; page.msg = e; page.load(); });
    }
    Component.onCompleted: { load(); rdLoad(); }

    Text {
        visible: !page.st && page.err === ""
        text: "Loading…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
    Text {
        visible: page.err !== ""
        Layout.fillWidth: true
        text: page.err; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ---- Use this PC from anywhere (docs/REMOTE-DESKTOP.md)
    SetHeader { text: "Use this PC from anywhere"; visible: !!page.rd && !page.rd.error }
    Text {
        visible: !!page.rd && !page.rd.error && !page.rd.installed
        Layout.fillWidth: true
        text: "This uses Sunshine, which comes with Hearth OS on PCs. It isn't on this system."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetSwitch {
        visible: !!page.rd && !!page.rd.installed
        icon: "globe"; title: "Use this PC from anywhere"
        sub: rdAct.busy && rdAct.what === "on" ? "Working on it…"
             : page.rdOn ? (page.rd.running ? "On: this PC's screen in Moonlight on your laptop, work PC or phone, " + (page.rd.reach === "home" ? "through Tailscale and your home network" : "only through Tailscale")
                                            : "On, but Sunshine isn't running: pick Start again below")
             : "Your desktop on another computer or phone with the free Moonlight app, fast and through Tailscale only. Off until you turn it on"
        checked: page.rdOn
        onToggled: on => page.rdRun(["set", "on", on ? "on" : "off"], on ? "On. Now add this PC in Moonlight." : "Off. Sunshine is back as it was.", "on")
    }
    SetRow {
        visible: page.rdOn && !page.rd.running
        icon: "restart"; title: "Start again"
        sub: "Sunshine was stopped (Remote play's switch below, or it couldn't start). Start it with Hearth's settings again"
        onActivated: page.rdRun(["start"], "Started again.", "start")
    }
    SetRow { chevron: true;
        visible: page.rdOn && !!page.rd.tailscale && !page.rd.tailscale.running
        icon: "globe"; title: "Tailscale isn't connected"
        sub: "Away from home, Moonlight finds this PC through Tailscale. Sign in on this PC and on the other device"
        onActivated: SettingsNav.openPage("hs-tailscale")
    }
    SetRow {
        visible: page.rdOn && page.addrLine() !== ""
        icon: "desktops"; chevron: false; interactive: false
        title: "In Moonlight, add " + page.addrLine()
        sub: "On the other device: sign in to Tailscale with the same account, open Moonlight, pick Add PC (+) and type that. Then pick Desktop"
    }
    SetRow {
        visible: page.rdOn && page.rd.running && !page.rd.needsLogin
        icon: "pad"; title: "Type Moonlight's PIN"
        sub: rdAct.busy && rdAct.what === "pin" ? "Pairing…" : "When Moonlight shows 4 numbers, type them here"
        onActivated: { page.rdPairing = !page.rdPairing; if (page.rdPairing) rdPin.focusInput(); }
    }
    SetField { id: rdPin; visible: page.rdPairing; placeholder: "The 4 numbers Moonlight shows"; onSubmitted: rdName.focusInput() }
    SetField {
        id: rdName; visible: page.rdPairing; placeholder: "Name for that device (like Work laptop), then Enter"
        onSubmitted: t => {
            const p = rdPin.text.replace(/\D/g, "");
            if (p.length !== 4) { page.rdSay("The PIN is the 4 numbers Moonlight shows", true); rdPin.focusInput(); return; }
            const n = t.trim() || "Moonlight";
            page.rdRun(["pin", p, n], "Paired " + n + ". In Moonlight, pick Desktop.", "pin");
            rdPin.text = ""; rdName.text = ""; page.rdPairing = false;
        }
    }
    SetRow {
        visible: page.rdOn && page.rd.needsLogin
        icon: "lock"; chevron: false; interactive: false; title: "Sunshine has its own login"
        sub: "Type it once under Sunshine's username and password below, then pair here"
    }
    Repeater {
        model: page.rdOn ? (page.rd.clients || []) : []
        delegate: SetRow {
            required property var modelData
            icon: "display"; title: modelData.name || "Device"; sub: "Paired: can use this PC from anywhere"; value: "Remove"
            onActivated: page.rdRun(["unpair", modelData.uuid], "Removed " + (modelData.name || "it") + ". It can pair again any time.", "unpair")
        }
    }
    SetRow {
        visible: page.rdOn && !!page.rd.stream && page.rd.stream.on
        icon: "wired"; chevron: false; interactive: false
        title: page.rd && page.rd.stream && page.rd.stream.on ? (page.rd.stream.who || page.rd.stream.device || "A device") + " is using this PC now" : ""
        sub: page.rd && page.rd.stream && page.rd.stream.on ? page.streamLine(page.rd.stream) + (page.rd.awake ? " · staying awake" : "") : ""
    }
    Text {
        visible: page.rdOn && !!page.rd.stream && page.rd.stream.on && !!page.rd.stream.tip
        Layout.fillWidth: true
        text: page.rd && page.rd.stream && page.rd.stream.tip ? page.rd.stream.tip + "." : ""
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.rdOn && !!page.rd.stream && page.rd.stream.on && !!page.rd.stream.hint
        Layout.fillWidth: true
        text: page.rd && page.rd.stream ? (page.rd.stream.hint || "") : ""
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    SetRow {
        visible: page.rdOn && !!page.rd.stream && page.rd.stream.on && !!page.rd.stream.peerIp
        icon: "search"; title: "Check the connection"
        sub: rdAct.busy && rdAct.what === "check" ? "Checking…"
             : page.rdCheck ? (page.rdCheck.path === "direct" ? "Direct, " : "Through a relay" + (page.rdCheck.relay ? " (" + page.rdCheck.relay + ")" : "") + ", ") + page.rdCheck.ms + " ms. " + page.rdCheck.hint
             : "One ping through Tailscale: direct or relay, and how long it takes"
        onActivated: rdAct.run(["check"], r => page.rdCheck = r, e => page.rdSay(e, true), "", "check")
    }
    Repeater {
        model: page.rdOn && !(page.rd.stream && page.rd.stream.on) ? (page.rd.peers || []).slice(0, 6) : []
        delegate: SetRow {
            required property var modelData
            icon: "desktops"; chevron: false; interactive: false
            title: modelData.name
            sub: modelData.path === "direct" ? "Online · direct connection" : modelData.path === "relay" ? "Online · through a relay" + (modelData.relay ? " (" + modelData.relay + ")" : "") + ": it will feel laggier"
                 : "Online · not connected to this PC right now"
        }
    }
    SetChoice {
        visible: page.rdOn
        title: "Reachable"
        sub: "Never from the internet. Tailscale only keeps this PC's streaming closed to your home network too"
        options: [{ value: "tailscale", label: "Tailscale only" }, { value: "home", label: "Tailscale and home network" }]
        current: page.rd ? page.rd.reach : "tailscale"
        onChosen: v => page.rdRun(["set", "reach", v], v === "home" ? "Your home network can reach it too." : "Only through Tailscale now.", "reach")
    }
    SetSwitch {
        visible: page.rdOn
        icon: "sleep"; title: "Stay awake while connected"
        sub: "This PC doesn't sleep or go idle while a device streams, and can again 2 minutes after it disconnects"
        checked: !!page.rd && !!page.rd.keepAwake
        onToggled: on => page.rdRun(["set", "keepAwake", on ? "on" : "off"], on ? "Stays awake while connected." : "It can sleep as usual.", "awake")
    }
    SetRow { chevron: true;
        visible: page.rdOn
        icon: "power"; title: "Wake this PC from away"
        sub: "If it's asleep, another device at home (a phone or a Hearth device on Tailscale) can wake it: Home server › Alerts & power"
        onActivated: SettingsNav.openPage("hs-alerts")
    }
    Text {
        visible: page.rdOn
        Layout.fillWidth: true
        text: (page.rd && page.rd.gameMode ? "Game Mode is running: whoever is connected sees and controls Steam's Game Mode, and the stream goes on. " : "")
              + "Moonlight's Desktop shows this PC's screen as it is: in Desktop Mode your desktop, in Game Mode Steam (and a game you start there). Work computers may not allow installing Moonlight or remote access: ask before you do."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.rdMsg !== ""
        Layout.fillWidth: true
        text: page.rdMsg; color: page.rdBad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Stream from this device"; visible: !!page.st }
    Text {
        visible: !!page.st && !page.st.installed
        Layout.fillWidth: true
        text: "Remote play uses Sunshine, which comes with Hearth OS. It isn't on this system."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetSwitch {
        visible: !!page.st && page.st.installed
        icon: "display"; title: "Remote play"
        sub: page.busy === "on" ? "Working on it…" : page.rdOn ? "Sunshine is also on for Use this PC from anywhere (turning this off stops it until you pick Start again)"
             : "Play your games on a TV, phone, tablet or another PC with the free Moonlight app"
        checked: !!page.st && page.st.on
        onToggled: on => page.act("remote.setOn", { on: on }, "on", on ? "Remote play is on. Pair a device to start." : "Remote play is off")
    }
    SetRow {
        visible: !!page.st && page.st.on && page.st.needsLogin
        icon: "lock"; title: "Sunshine's username and password"
        sub: "Sunshine was set up before with its own login. Type it once so Hearth can pair devices."
        onActivated: { page.login = !page.login; if (page.login) sunUser.focusInput(); }
    }
    SetField { id: sunUser; visible: page.login; placeholder: "Sunshine's username, then Enter"; onSubmitted: sunPass.focusInput() }
    SetField {
        id: sunPass; visible: page.login; secret: true; placeholder: "Sunshine's password, then Enter"
        onSubmitted: t => { page.act("remote.login", { user: sunUser.text, pass: t }, "login", "Thanks. Now pair a device."); sunPass.text = ""; page.login = false; }
    }
    SetRow {
        visible: !!page.st && page.st.on && !page.rdOn
        icon: "pad"; title: "Pair a device"
        sub: page.busy === "pair" ? "Pairing…" : "In Moonlight, pick this device (or add it by address" + (page.st && page.st.addresses && page.st.addresses.length ? ": " + page.st.addresses.join(", ") : "") + "). It shows 4 numbers: type them here."
        onActivated: { page.pairing = !page.pairing; if (page.pairing) pin.focusInput(); }
    }
    SetField { id: pin; visible: page.pairing; placeholder: "The 4 numbers Moonlight shows"; onSubmitted: devName.focusInput() }
    SetField {
        id: devName; visible: page.pairing; placeholder: "Name for that device (Moonlight), then Enter"
        onSubmitted: t => {
            const p = pin.text.replace(/\D/g, "");
            if (p.length !== 4) { page.msg = "The PIN is the 4 numbers Moonlight shows"; pin.focusInput(); return; }
            const n = t.trim() || "Moonlight";
            page.act("remote.pair", { pin: p, name: n }, "pair", "Paired " + n + ". Pick a game in Moonlight.");
            pin.text = ""; devName.text = ""; page.pairing = false;
        }
    }
    SetHeader { text: "Paired devices"; visible: !!page.st && page.st.on && !page.rdOn && (page.st.clients || []).length > 0 }
    Repeater {
        model: page.st && page.st.on && !page.rdOn ? (page.st.clients || []) : []
        delegate: SetRow {
            required property var modelData
            icon: "display"; title: modelData.name || "Device"; sub: "Can stream from this device"; value: "Remove"
            onActivated: page.act("remote.unpair", { uuid: modelData.uuid }, "", "Removed " + (modelData.name || "it") + ". It can pair again any time.")
        }
    }
    Text {
        visible: !!page.st && page.st.on
        Layout.fillWidth: true
        text: "Works best on the same home network, with this device plugged in. Streaming uses this device's screen, so whatever plays here plays there."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Play another PC's games here"; visible: !!page.st }
    SetRow {
        visible: !!page.st
        icon: "desktops"; title: "Moonlight"
        sub: page.busy === "moonlight" ? "Installing Moonlight… this takes a few minutes"
            : page.st && page.st.moonlight ? "Stream games from a gaming PC that runs Sunshine or NVIDIA GameStream"
            : "Install it to stream games from a gaming PC (it needs Sunshine on that PC)"
        value: page.st && page.st.moonlight ? "Open" : "Install"
        onActivated: {
            if (page.st && page.st.moonlight) { Quickshell.execDetached(["flatpak", "run", "com.moonlight_stream.Moonlight"]); SettingsNav.toast("Opening Moonlight…"); }
            else if (page.busy !== "moonlight") page.act("moonlight.install", {}, "moonlight", "Moonlight is installed");
        }
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

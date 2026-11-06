// Hearth Settings › Home server › Server mode (docs/SERVER-MODE.md): the third mode next to Game Mode and Desktop Mode
// (same image, a switch, no separate edition). Use Server mode and Open now; Startup and schedule (System › Startup); power
// when idle (lower limits, drives spinning down, the screen dimming then off, sleep); what pauses when you play (Light
// mode's own choices, hearth-home-server: one source of truth); the web page over Tailscale (its PIN, what it may do).
// Server mode's own choices go through hearth-server-mode, Light mode's through hearth-home-server: each saves only the
// key you change.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Server mode"
    property var st: null
    property string msg: ""
    property bool bad: false
    property bool pinOpen: false
    readonly property var s: st ? st.settings : null
    readonly property var p: st ? st.pauses : null
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.say(e, true)); }
    function set(k, v, ok) { act.run(["set", k, v], () => { if (ok) page.say(ok); page.refresh(); }, e => { page.say(e, true); page.refresh(); }); }
    function hsSet(k, v, ok) { hs.run(["set", k, v], () => { if (ok) page.say(ok); page.refresh(); }, e => { page.say(e, true); page.refresh(); }); }
    function web() { act.run(["web-apply"], () => page.refresh(), e => page.say(e, true)); }
    Component.onCompleted: refresh()
    Timer { interval: 6000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy && !hs.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-server-mode" }
    SetJob { id: act; helper: "hearth-server-mode" }
    SetJob { id: hs; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        Layout.bottomMargin: 8
        text: "A third mode next to Game Mode and Desktop Mode: a dashboard on the screen, nothing else using memory, and full power for your services. One press of Play (or Guide) and you're in Game Mode; leaving Game Mode brings you back."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Server mode" }
    SetSwitch {
        icon: "server"; title: "Use Server mode"
        sub: "Show it next to Game Mode and Desktop Mode: in Start in, here and on the dashboard. Off by default; Game Mode stays where your PC starts unless you choose Server mode"
        checked: !!page.s && page.s.on
        onToggled: v => act.run(["set", "on", v ? "on" : "off"], () => { page.say(v ? "Server mode is on ✓ Open it now, or pick it in Start in" : "Server mode is off"); page.web(); page.refresh(); }, e => page.say(e, true))
    }
    SetRow {
        visible: !!page.s && page.s.on
        icon: "play"; title: "Open Server mode now"
        sub: page.st && page.st.now === "server" ? "You're in Server mode" : "Game Mode and Desktop Mode close first (save your work)"
        value: "Open"
        onActivated: if (!(page.st && page.st.now === "server")) Quickshell.execDetached(["nimbus-mode", "server"])
    }
    SetRow {
        chevron: true
        icon: "power"; title: "Startup and schedule"
        sub: page.s && page.s.schedule && page.s.schedule.on ? "Following a schedule" : "Start in: " + ({ game: "Game Mode", desktop: "Desktop Mode", server: "Server mode" })[page.st ? page.st.startIn : "game"] + ". No schedule unless you make one"
        value: "In System"
        onActivated: SettingsNav.openPage("startup")
    }

    SetHeader { text: "Power when idle" }
    SetSwitch {
        visible: !!page.s
        icon: "bolt"; title: "Lower power limits when idle"
        sub: page.s ? "After " + page.s.idleMinutes + " minutes with no jobs: the processor on power-saver" + (page.st && page.st.nvidia ? ", the graphics card at " + page.s.gpuWatts + " W" : "") + ". Full power comes back the moment a job starts, and always before Game Mode" : ""
        checked: !!page.s && page.s.idlePower
        onToggled: v => page.set("idlePower", v ? "on" : "off")
    }
    SetChoice {
        visible: !!page.s && page.s.idlePower && !!page.st && page.st.nvidia
        title: "Graphics card limit when idle"
        options: [80, 100, 120, 150, 200].map(w => ({ value: w, label: w + " W" }))
        current: page.s ? page.s.gpuWatts : 120
        onChosen: v => page.set("gpuWatts", String(v))
    }
    SetChoice {
        visible: !!page.s && page.s.idlePower
        title: "Idle means no jobs for"
        options: [5, 10, 20, 30].map(m => ({ value: m, label: m + " min" }))
        current: page.s ? page.s.idleMinutes : 10
        onChosen: v => page.set("idleMinutes", String(v))
    }
    SetChoice {
        visible: !!page.st
        icon: "drive"; title: "Spin down drives"
        sub: "Data drives only; the system drive stays on (the same setting as Alerts & power)"
        options: [{ value: 0, label: "Never" }, { value: 10, label: "10 min" }, { value: 20, label: "20 min" }, { value: 30, label: "30 min" }, { value: 60, label: "1 hour" }]
        current: page.st ? page.st.spinDown : 0
        onChosen: v => page.hsSet("spinDown", String(v))
    }
    SetChoice {
        visible: !!page.s
        icon: "display"; title: "Screen dims to a clock after"
        sub: "Dim shows the time and the status in dark grey, kind to OLED screens. Any button brings the dashboard back"
        options: [{ value: 0, label: "Never" }, { value: 1, label: "1 min" }, { value: 2, label: "2 min" }, { value: 5, label: "5 min" }, { value: 10, label: "10 min" }]
        current: page.s ? page.s.dim : 2
        onChosen: v => page.set("dim", String(v))
    }
    SetChoice {
        visible: !!page.s
        title: "Then off after"
        sub: page.st && page.st.wlopm ? "The screen turns off" : "The screen goes black (install wlopm to turn it off completely)"
        options: [{ value: 0, label: "Never" }, { value: 5, label: "5 min" }, { value: 10, label: "10 min" }, { value: 20, label: "20 min" }, { value: 30, label: "30 min" }, { value: 60, label: "1 hour" }]
        current: page.s ? page.s.off : 10
        onChosen: v => page.set("off", String(v))
    }
    SetChoice {
        visible: !!page.st
        icon: "sleep"; title: "Sleep"
        sub: "Server mode keeps the PC awake unless you choose a time here. Waking it over Tailscale needs another device at home"
        options: [{ value: 0, label: "Never" }, { value: 30, label: "30 min" }, { value: 60, label: "1 hour" }, { value: 120, label: "2 hours" }, { value: 240, label: "4 hours" }]
        current: page.st ? page.st.sleepIdle : 0
        onChosen: v => page.hsSet("sleepIdle", String(v))
    }

    SetHeader { text: "What pauses when you play" }
    SetSwitch {
        visible: !!page.p
        icon: "pad"; title: "Light mode while you play"
        sub: "The same rules for Server mode's Play button, Guide and any game. Backups wait while it's on"
        checked: !!page.p && page.p.lightMode
        onToggled: v => page.hsSet("lightMode", v ? "on" : "off")
    }
    SetChoice {
        visible: !!page.p && page.p.picturesOn
        icon: "image"; title: "Pictures · ComfyUI"
        options: [{ value: "stop", label: "Stop, keep the queue" }, { value: "keep", label: "Keep running" }]
        current: page.p ? page.p.pictures : "stop"
        onChosen: v => page.hsSet("picturesPause", v === "stop" ? "on" : "off")
    }
    SetChoice {
        visible: !!page.p && page.p.aiOn
        icon: "chat"; title: "AI chat · Ollama"
        sub: page.p ? "The small model is " + page.p.small + " on the processor (only when it's downloaded; nothing is ever deleted)" : ""
        options: [{ value: "small", label: "Switch to the small model" }, { value: "unload", label: "Unload the model" }, { value: "stop", label: "Stop AI chat" }, { value: "keep", label: "Keep it loaded" }]
        current: page.p ? page.p.ai : "small"
        onChosen: v => {
            // One choice, written as Light mode's own keys (only the ones that differ are saved).
            const want = { small: { aiSmall: "on", aiPause: "off", aiUnload: "on" }, unload: { aiSmall: "off", aiPause: "off", aiUnload: "on" },
                           stop: { aiSmall: "off", aiPause: "on", aiUnload: "on" }, keep: { aiSmall: "off", aiPause: "off", aiUnload: "off" } }[v];
            const keys = Object.keys(want);
            keys.forEach((k, i) => hs.run(["set", k, want[k]], () => { if (i === keys.length - 1) page.refresh(); }, e => { page.say(e, true); page.refresh(); }));
        }
    }
    SetChoice {
        visible: !!page.p && page.p.voicesOn
        icon: "wave"; title: "Voices · Kokoro"
        options: [{ value: "pause", label: "Pause" }, { value: "keep", label: "Keep running" }]
        current: page.p ? page.p.voices : "keep"
        onChosen: v => page.hsSet("voicesPause", v === "pause" ? "on" : "off")
    }
    SetChoice {
        visible: !!page.p && page.p.mediaOn
        icon: "film"; title: "Media · Jellyfin"
        options: [{ value: "keep", label: "Keep, convert on the processor" }, { value: "pause", label: "Pause" }]
        current: page.p ? page.p.media : "keep"
        onChosen: v => page.hsSet("mediaPause", v === "pause" ? "on" : "off")
    }
    SetRow {
        icon: "folder"; chevron: false; interactive: false
        title: "Shared folders, Tailscale"; sub: "Always keep working"; value: "Keep"
    }

    SetHeader { text: "Web page" }
    SetSwitch {
        visible: !!page.s
        icon: "globe"; title: "Dashboard web page"
        sub: page.st ? "Open " + page.st.url + " on your phone or laptop. Only over Tailscale, never the internet" + (page.st.pin ? "" : ". Set a PIN first") : ""
        checked: !!page.s && page.s.web.on
        onToggled: v => act.run(["set", "webOn", v ? "on" : "off"], () => { page.web(); if (v && !page.st.pin) page.pinOpen = true; }, e => page.say(e, true))
    }
    SetRow {
        visible: !!page.s
        icon: "lock"; title: "PIN for the web page"
        sub: page.st && page.st.pin ? "Asked once on each device, again after 30 days. Only its hash is kept; a new PIN signs every device out" : "4 to 12 digits. Nobody gets in until you set one"
        value: page.st && page.st.pin ? "●●●●●● · Change" : "Set"
        onActivated: page.pinOpen = !page.pinOpen
    }
    SetField {
        visible: page.pinOpen
        secret: true
        Layout.leftMargin: 56; Layout.rightMargin: 18; Layout.topMargin: 6; Layout.bottomMargin: 6
        placeholder: "A PIN of 4 to 12 digits, then Enter"
        onSubmitted: t => {
            if (!/^\d{4,12}$/.test(t)) { page.say("Use 4 to 12 digits", true); return; }
            act.run(["pin-set"], () => { page.say("PIN set ✓"); page.pinOpen = false; page.web(); }, e => page.say(e, true), t);
            text = "";
        }
    }
    SetSwitch {
        visible: !!page.s && page.s.web.on
        icon: "restart"; title: "Restart and pause services"
        checked: !!page.s && page.s.web.perms.services
        onToggled: v => page.set("permServices", v ? "on" : "off")
    }
    SetSwitch {
        visible: !!page.s && page.s.web.on
        icon: "pad"; title: "Switch modes"
        sub: "The screen at home counts down 10 seconds first (cancel it there), and nothing switches while a game is running or Desktop Mode is open"
        checked: !!page.s && page.s.web.perms.modes
        onToggled: v => page.set("permModes", v ? "on" : "off")
    }
    SetSwitch {
        visible: !!page.s && page.s.web.on
        icon: "sleep"; title: "Sleep or restart the PC"
        checked: !!page.s && page.s.web.perms.power
        onToggled: v => page.set("permPower", v ? "on" : "off")
    }
    SetSwitch {
        visible: !!page.s && page.s.web.on
        icon: "desktops"; title: "Show it on my other Hearth devices"
        sub: "Hearth Link and Quick Access on your Ally can show the status (read only) without the PIN, from your Tailscale devices only"
        checked: !!page.s && page.s.web.perms.devices
        onToggled: v => page.set("permDevices", v ? "on" : "off")
    }

    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

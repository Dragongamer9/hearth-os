// Hearth Settings › Home server › AI devices (docs/AI-SHARE.md): "Share this device's AI" (off by default, per device:
// which services, only when plugged in, not while playing, a share code) and, on your Home server, every Hearth device's
// AI in one list in the order the Hearth AI Command Center tries them (Up/Down on a device, then Move up / Move down),
// and the one address the Command Center needs. Everything goes through hearth-ai-share (settings written per key by
// hearth_settings_io; opening the page never saves anything).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "AI devices"
    property var st: null
    property var list: null
    property string msg: ""
    property bool bad: false
    property string pick: ""
    property bool codeField: false
    readonly property var cfg: st ? st.settings : null
    readonly property bool listShown: !!st && (st.server || (cfg && cfg.hub))
    // The page's rows as laid out (SetPage's column): its height also counts rows its implicit height missed, so the
    // device list at the bottom always scrolls into view (in CI the page stopped scrolling at "Your AI devices").
    // In CI the layout's own height still stopped after "Ask for a share code" (the rows below were drawn and took focus
    // but the page wouldn't scroll to them), so the page also measures where its last shown row really ends.
    readonly property Item rowsColumn: { const k = contentItem.children; for (let i = 0; i < k.length; i++) if (k[i].spacing !== undefined) return k[i]; return null; }
    property real rowsBottom: 0
    function measure() {
        const c = page.rowsColumn;
        if (!c) return;
        let m = 0;
        for (let i = 0; i < c.children.length; i++) { const k = c.children[i]; if (k.visible && k.height > 0) m = Math.max(m, k.y + k.height); }
        if (Math.abs(m - page.rowsBottom) > 1) page.rowsBottom = m;
    }
    contentHeight: rowsColumn ? Math.max(rowsColumn.implicitHeight, rowsColumn.childrenRect.height, rowsBottom) + 48 : 0
    Timer { interval: 400; running: page.visible; repeat: true; onTriggered: page.measure() }
    onActiveFocusChanged: measure()
    // One line in the shell log for CI (desktop-test.yml): the heights the page sees.
    Timer {
        interval: 7000; running: page.visible; repeat: false
        onTriggered: { page.measure(); console.info("AI devices: rows end at", page.rowsBottom, "layout", page.rowsColumn ? page.rowsColumn.implicitHeight : -1, "childrenRect", page.rowsColumn ? page.rowsColumn.childrenRect.height : -1, "contentHeight", page.contentHeight, "height", page.height); }
    }
    readonly property var kindWords: ({ chat: "Chat", voice: "Voices", image: "Pictures", stt: "Speech to text" })
    function say(t, b) { msg = t; bad = !!b; }
    // replaceList (SetPage): rows are rebuilt only when something changed, and the selection stays put (a list that
    // rebuilt under the controller's focus left nothing selected, and Down moved nothing).
    function refresh() { job.run(["status"], r => page.replaceList("st", r), e => page.replaceList("st", { error: e, found: [], settings: null })); }
    function steady(r) {
        // Times change on every answer; the page doesn't show them, so they don't rebuild the list.
        const out = Object.assign({}, r);
        out.updated = 0;
        out.devices = (r.devices || []).map(d => { const c = Object.assign({}, d); c.updated = 0; c.lastSeen = 0; return c; });
        return out;
    }
    function refreshList() { if (page.listShown && !lj.busy) lj.run(["devices"], r => page.replaceList("list", page.steady(r)), e => page.replaceList("list", { error: e, devices: [] })); }
    function set(k, v, ok) { act.run(["set", k, v ? "on" : "off"], r => { page.say(r && r.note ? r.note : (ok || "Saved")); page.refresh(); page.refreshList(); }, e => { page.say(e, true); page.refresh(); }); }
    function reasonWord(r) {
        return ({ off: "Not shared", playing: "Not while you play", battery: "Waiting for a charger", nothing: "No AI found here",
                  offline: "Offline", "not-sharing": "Not sharing its AI" })[r] || "";
    }
    function svcLine(s, here) {
        const bits = [];
        if (s.models && s.models.length) bits.push(s.models.slice(0, 3).join(", ") + (s.models.length > 3 ? " +" + (s.models.length - 3) : ""));
        if (s.voices && s.voices.length) bits.push(s.voices.length + " voices");
        return (here ? (bits.length ? bits.join(", ") : "Found on port " + s.port) : (page.kindWords[s.kind] || s.kind) + " (" + s.engine + ")" + (bits.length ? ": " + bits.join(", ") : ""))
               + (!s.healthy ? " · not answering" : s.reachable === false ? " · only on that device" : s.busy ? " · busy" : "");
    }
    function devLine(d) {
        const bits = [d.online ? "Online" : "Offline"];
        if (d.online && d.gaming) bits.push("playing");
        if (d.battery) bits.push(d.battery.percent + "%" + (d.pluggedIn ? " · plugged in" : " · on battery"));
        else if (d.online) bits.push("plugged in");
        if (d.gpu) bits.push(d.gpu.vendor === "nvidia" ? "Nvidia card" : d.gpu.vendor === "amd" ? "AMD graphics" : "Intel graphics");
        const sv = (d.services || []).map(s => page.svcLine(s, false));   // (map's index isn't "here")
        return bits.join(" · ") + (sv.length ? "\n" + sv.join("\n") : d.via === "hearth link" ? "\nPaired over Hearth Link; turn on Share this device's AI and Tailscale on it" : "");
    }
    Component.onCompleted: { refresh(); }
    onListShownChanged: refreshList()
    Timer { interval: 4000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy) page.refresh() }
    Timer { interval: 15000; running: page.visible && page.listShown; repeat: true; onTriggered: page.refreshList() }
    SetJob { id: job; helper: "hearth-ai-share" }
    SetJob { id: lj; helper: "hearth-ai-share" }
    SetJob { id: act; helper: "hearth-ai-share" }

    Text {
        Layout.fillWidth: true
        text: "Each of your Hearth devices runs its own AI jobs: big chat and pictures on the PC with the strongest card, voices or speech to text on the others. Share a device's AI and your other devices, and the Hearth AI Command Center, can use it through Tailscale. Nothing is shared until you turn it on."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && !!page.st.error
        Layout.fillWidth: true
        text: page.st && page.st.error ? page.st.error : ""; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ---------------------------------------------------------------- this device
    SetHeader { text: "Share this device's AI"; visible: !!page.cfg }
    SetSwitch {
        visible: !!page.cfg
        icon: "mic"; title: "Share this device's AI"
        sub: !page.st || !page.cfg ? "" : !page.cfg.on ? "Off: nothing is shared"
             : !page.st.running ? "Starting…"
             : page.st.reason === "playing" ? "Paused while you play: your devices use another one"
             : page.st.reason === "battery" ? "Paused on battery: shares again when plugged in"
             : page.st.reason === "nothing" ? "On, but no AI service was found on this device"
             : !page.st.tailscale ? "On this device only: turn on Tailscale (Home server › Tailscale) for your other devices"
             : !page.st.firewall && !page.cfg.lan ? "Waiting for Hearth's firewall rule: on this device only until it loads"
             : "Shared with your devices through Tailscale as " + page.st.device
        checked: !!page.cfg && page.cfg.on
        onToggled: v => page.set("on", v, v ? "Sharing this device's AI with your devices" : "Not sharing this device's AI")
    }
    SetRow {
        visible: !!page.cfg && page.cfg.on
        icon: "lock"; chevron: false; interactive: false; title: "Who can see it"
        sub: page.cfg && page.cfg.lan ? "Your devices through Tailscale, this device, and everything on your home network"
             : "Your devices through Tailscale, and this device. Never your home network or the internet: Hearth's firewall rule lets only Tailscale reach it. It's a read-only list of what's here, with no keys or files"
    }

    SetHeader { text: "What to share"; visible: !!page.cfg }
    SetRow {
        visible: !!page.st && !!page.cfg && (page.st.found || []).length === 0
        icon: "info"; chevron: false; interactive: false; title: "No AI services found on this device"
        sub: "Hearth looks for Ollama chat (port 11434), Kokoro voices (8880), ComfyUI pictures (8188) and whisper speech to text (8000 or 9000). On a home server, Home server › AI installs Ollama"
    }
    Repeater {
        model: page.st && page.cfg ? page.st.found : []
        delegate: SetSwitch {
            required property var modelData
            icon: modelData.kind === "voice" ? "vol" : modelData.kind === "image" ? "palette" : modelData.kind === "stt" ? "mic" : "grid"
            title: modelData.label
            sub: page.svcLine(modelData, true) + (modelData.reachable ? "" : " · it listens on this device only, so other devices can't use it yet")
            checked: !!page.cfg && page.cfg.share[modelData.kind] !== false
            onToggled: v => page.set(modelData.kind, v, v ? "Shared: " + modelData.label : "Not shared: " + modelData.label)
        }
    }

    SetHeader { text: "When to share"; visible: !!page.cfg }
    SetSwitch {
        visible: !!page.cfg && !!page.st.battery
        icon: "power"; title: "Share only when plugged in"
        sub: "On battery, your devices use another one" + (page.st && page.st.battery ? " · now " + page.st.battery.percent + "%" + (page.st.pluggedIn ? ", plugged in" : ", on battery") : "")
        checked: !!page.cfg && page.cfg.pluggedOnly
        onToggled: v => page.set("pluggedOnly", v)
    }
    SetSwitch {
        visible: !!page.cfg
        icon: "pad"; title: "Not while I'm playing"
        sub: "While Game Mode or a game runs, this device says it's playing and your devices use another one" + (page.st && page.st.gaming ? " · playing now" : "")
        checked: !!page.cfg && page.cfg.notWhilePlaying
        onToggled: v => page.set("notWhilePlaying", v)
    }
    SetSwitch {
        visible: !!page.cfg && page.cfg.on
        icon: "wired"; title: "Also on my home network"
        sub: "Off: only Tailscale and this device. On: anything on your home network can read the list too (the services keep their own rules)"
        checked: !!page.cfg && page.cfg.lan
        onToggled: v => page.set("lan", v)
    }
    SetSwitch {
        visible: !!page.cfg && (page.cfg.on || page.cfg.hub)
        icon: "lock"; title: "Ask for a share code"
        sub: page.st && page.st.codeSet ? "The Command Center sends it (X-Hearth-Share-Code). Only its hash is kept here. Use the same code on all your devices"
             : "Off. Select to set a code (8 to 64 characters); only its hash is kept"
        checked: !!page.cfg && page.cfg.codeOn
        onToggled: v => {
            if (v && !(page.st && page.st.codeSet)) { page.codeField = true; Qt.callLater(codeBox.focusInput); return; }
            if (v) page.set("codeOn", true);
            else act.run(["code-clear"], () => { page.say("No share code any more"); page.refresh(); }, e => page.say(e, true));
        }
    }

    // ---------------------------------------------------------------- every device (the Home server)
    SetHeader { text: "Your AI devices"; visible: !!page.cfg && (page.st.server || page.cfg.hub) }
    SetSwitch {
        visible: !!page.cfg && (page.st.server || page.cfg.hub)
        icon: "globe"; title: "Let the Command Center find my devices here"
        sub: "This home server lists every device's AI at one address, so the Command Center needs only that"
        checked: !!page.cfg && page.cfg.hub
        onToggled: v => page.set("hub", v, v ? "Listing your devices' AI for the Command Center" : "Not listing your devices any more")
    }
    SetRow {
        visible: !!page.cfg && page.cfg.hub
        icon: "globe"; title: "Command Center address"
        sub: page.st && page.st.commandCenter ? page.st.commandCenter + " · paste it into the Command Center once: it finds every device from there"
             : "Turn on Tailscale and sign in (Home server › Tailscale) to get the address. On this device: " + (page.st ? page.st.local : "")
        value: page.st && page.st.commandCenter ? "Copy" : ""
        onActivated: if (page.st && page.st.commandCenter) { Quickshell.execDetached(["wl-copy", page.st.commandCenter]); page.say("Copied ✓"); }
    }
    SetRow {
        visible: page.listShown && (!page.list || lj.busy && !(page.list.devices || []).length)
        icon: "update"; chevron: false; interactive: false; title: "Looking for your devices…"
    }
    Text {
        visible: page.listShown && !!page.list && (page.list.devices || []).length > 0
        Layout.fillWidth: true
        text: "In the order the Command Center tries them. Select a device to move it up or down."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.listShown && page.list ? page.list.devices : []
        delegate: SetRow {
            required property var modelData
            icon: page.pick === modelData.device ? "check" : modelData.battery ? "batt" : "display"
            title: modelData.rank + ". " + (modelData.name || modelData.device) + (modelData.via === "this device" ? " (this device)" : "") + (page.pick === modelData.device ? "  ◂ selected" : "")
            sub: page.devLine(modelData)
            value: !modelData.online ? "Offline" : modelData.gaming ? "Playing" : modelData.busy ? "Busy" : modelData.available ? "Ready" : page.reasonWord(modelData.reason)
            onActivated: page.pick = page.pick === modelData.device ? "" : modelData.device
        }
    }
    SetRow {
        visible: page.listShown && page.pick !== ""
        icon: "back"; title: "Move " + page.pick + " up"; chevron: false
        onActivated: act.run(["move", page.pick, "up"], () => page.refreshList(), e => page.say(e, true))
    }
    SetRow {
        visible: page.listShown && page.pick !== ""
        icon: "forward"; title: "Move " + page.pick + " down"; chevron: false
        onActivated: act.run(["move", page.pick, "down"], () => page.refreshList(), e => page.say(e, true))
    }
    SetRow {
        visible: page.listShown && !!page.cfg && page.cfg.priority.length > 0
        icon: "restart"; title: "Back to the usual order"; chevron: false
        sub: "A device with an Nvidia card first, then the others by graphics memory"
        onActivated: act.run(["order-reset"], () => { page.say("Usual order"); page.refresh(); page.refreshList(); }, e => page.say(e, true))
    }
    SetRow {
        visible: !!page.st && !page.listShown && !!page.cfg
        icon: "info"; chevron: false; interactive: false; title: "Every device in one list"
        sub: "On your Home server, this page lists every Hearth device's AI and gives the Command Center its address"
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    // The share code's box is last on the page: a hidden text box between rows would stop Down there (a controller
    // couldn't reach the device list). Turning the switch on shows it and puts the cursor in it.
    SetField {
        id: codeBox
        visible: page.codeField
        secret: true
        placeholder: "Type a share code (8 to 64 characters), then Enter"
        onSubmitted: t => act.run(["code-set"], () => { page.codeField = false; page.say("Share code set ✓ (only its hash is kept)"); page.refresh(); }, e => page.say(e, true), t)
    }
}

// Hearth Settings › AI › Describe it (the owner, Nov 24, 2026: Describe it sat on "Your AI is making it…" for minutes on
// his laptop's slow CPU Ollama while his PC's AI was there over Hearth Link). The same choice as the Rust Settings'
// Settings › AI (hosts/hearth-ui/apps/settings/src/cats/ai.rs describe_card), for Mantel's own Settings: where Describe
// it asks first, Fastest available (a slow engine on this device after your other devices) / This device / My other
// devices (hearth-ai-check use describe {"from": …}, ai-uses.json through hearth_settings_io), and which AI that is now.
// Shared from your other devices: every AI your other Hearth devices share (hearth-ai-check shared: the same list as
// Settings › Home server › AI devices, with Hearth Link's), each with its state, engine, models, graphics, speed when
// measured, whether Describe it uses it, and "Use this for Describe it" (hearth-ai-check use-shared: the same pick). A
// device Hearth Link couldn't confirm as yours says so, with Pair again (Settings › Hearth Link). Read when the page
// opens and on Refresh; nothing polls.
// Opening the page writes nothing; the pick is saved only when you make it.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Describe it"
    property var st: null                 // hearth-ai-check status
    property string msg: ""
    readonly property var use: st && st.uses ? st.uses.describe : null
    property var shared: null             // hearth-ai-check shared
    property bool looking: false
    readonly property var devs: shared && shared.devices ? shared.devices : []
    function stateWord(d) {
        return ({ ready: "Ready", playing: "Playing", busy: "Busy", offline: "Offline", unconfirmed: "Can't confirm",
                  off: "Not shared", battery: "Waiting for a charger", nothing: "No AI found here" })[d.state] || "Not sharing";
    }
    function gpuWord(v) { return v === "nvidia" ? "Nvidia card" : v === "amd" ? "AMD graphics" : v ? "Intel graphics" : ""; }
    function line(d) {
        const bits = [];
        if (d.engine) bits.push(d.engine === "ollama" ? "Ollama" : d.engine === "colibri" ? "Colibri" : d.engine);
        if (d.models && d.models.length) bits.push(d.models.slice(0, 2).join(", ") + (d.models.length > 2 ? " +" + (d.models.length - 2) : ""));
        if (d.gpu) bits.push(gpuWord(d.gpu) + (d.vramMB ? " · " + Math.round(d.vramMB / 1024) + " GB" : ""));
        if (d.words) bits.push("about " + Math.round(d.words) + " words a second");
        if (d.via === "tailscale") bits.push("through Tailscale"); else if (d.via === "hearth link") bits.push("over Hearth Link");
        return bits.join(" · ");
    }
    function lookShared() { page.looking = true; look.run(["shared"], r => { page.shared = r; page.looking = false; }, e => { page.msg = e; page.looking = false; }); }

    SetJob { id: job; helper: "hearth-ai-check" }
    SetJob { id: look; helper: "hearth-ai-check" }
    function refresh() { job.run(["status"], r => page.st = r, e => page.msg = e); }
    Component.onCompleted: { refresh(); lookShared(); }

    SetHeader { text: "Describe it uses" }
    SetChoice {
        icon: "sparkle"; title: "Where Describe it asks first"
        sub: !page.use ? "Checking…"
            : page.use.ready ? "Now: " + (page.use.via || page.use.label) + (page.use.model ? " · " + page.use.model : "")
                + (page.use.slow ? " · slow on this device" + (page.use.words ? " (about " + Math.round(page.use.words) + " words a second)" : "") : "")
            : "Nothing answers there right now: the fastest that answers is asked instead"
        options: [{ value: "fastest", label: "Fastest available" }, { value: "here", label: "This device" }, { value: "link", label: "My other devices" }]
        current: page.use && page.use.from ? page.use.from : "fastest"
        // hearth-ai-check says {"ok": false, "error"} when it can't: shown, never taken as done
        onChosen: v => job.run(["use", "describe", JSON.stringify({ from: v })], r => { page.msg = r && r.ok === false ? (r.error || "That didn't work.") : ""; page.refresh(); }, e => page.msg = e)
    }
    Text {
        Layout.fillWidth: true; Layout.leftMargin: 4; wrapMode: Text.WordWrap
        text: "Fastest available: your other devices over Hearth Link before a slow engine on this device (one that writes under 8 words a second, or has no graphics card). Nothing here changes by itself."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }

    SetHeader { text: "Shared from your other devices" }
    SetRow {
        visible: !page.shared || page.devs.length === 0
        interactive: false; chevron: false
        icon: "desktops"; title: !page.shared ? "Looking for your devices…" : "None of your other devices shares its AI yet"
        sub: !page.shared ? "" : "Turn on Share this device's AI on it (Settings › Home server › AI devices), or pair it in Hearth Link"
    }
    Repeater {
        model: page.devs
        SetRow {
            required property var modelData
            readonly property bool unconfirmed: modelData.state === "unconfirmed"
            interactive: unconfirmed || (modelData.canUse && !modelData.inUse)
            chevron: interactive
            icon: unconfirmed ? "lock" : modelData.gpu === "nvidia" ? "display" : "desktops"
            title: modelData.name
            chip: page.stateWord(modelData) + (modelData.inUse ? " · Describe it uses this" : "")
            chipKind: modelData.state === "ready" ? "ok" : unconfirmed ? "warn" : "off"
            sub: unconfirmed ? "Can't confirm this is your " + modelData.name + " — pair it again"
                : page.line(modelData) + (modelData.canUse && !modelData.inUse ? " · select to use this for Describe it" : "")
            onActivated: {
                if (unconfirmed) SettingsNav.openPage("link");
                else job.run(["use-shared", modelData.device], r => { page.msg = r && r.ok === false ? (r.error || "That didn't work.") : ""; page.refresh(); page.lookShared(); }, e => page.msg = e);
            }
        }
    }
    SetRow {
        icon: "restart"; chevron: false
        title: page.looking ? "Looking…" : "Refresh"
        sub: "Looked up when this page opens; nothing is asked in the background"
        onActivated: if (!page.looking) page.lookShared()
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true; wrapMode: Text.WordWrap
        text: page.msg; color: Theme.danger; font.family: Theme.font; font.pixelSize: 14
    }
}

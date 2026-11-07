// Hearth Settings › Home server › Server apps (docs/SERVER-APPS.md): the catalog. One-click game servers (Minecraft,
// Valheim, Terraria, Palworld) and self-hosted apps (Vaultwarden, Immich, Navidrome, ntfy). Each one is off and not
// installed until you press Install, then runs in your own rootless Podman as your own user service: never part of
// starting the PC, the session or Game Mode. Who can reach it (home network and Tailscale, or Tailscale only), its
// options and password, its logs, Update and Remove (your data stays unless you confirm twice). Everything goes through
// hearth-server-apps, which saves only what you change.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Server apps"
    property var st: null
    property string msg: ""
    property bool bad: false
    property string removing: ""      // the app whose data removal is being confirmed
    property int removeStep: 0
    property var logLines: ({})
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["list"], r => page.st = r, e => page.st = { error: e }); }
    function act(args, ok) { run.run(args, r => { page.say(ok || (r && r.note) || "Done ✓"); page.refresh(); }, e => { page.say(e, true); page.refresh(); }); }
    function apps(kind) { return st && st.apps ? st.apps.filter(a => a.kind === kind) : []; }
    function line(a) {
        if (a.installing) return "Downloading… " + (a.progress || 0) + "% · " + (a.step || "starting");
        if (a.error && !a.installed) return "Didn't install: " + a.error + ". Try again";
        if (!a.installed) return a.desc + " · " + a.size;
        if (!a.on) return "Off · installed";
        if (a.failed) return "Stopped: it couldn't start (Logs below say why). Turn it off and on to try again";
        if (!a.running) return "Starting…";
        return "Running · " + (a.reach === "home" ? "your home network and Tailscale" : "Tailscale and this PC only")
               + (a.addresses.length ? " · " + a.addresses[0].url : "");
    }
    Component.onCompleted: refresh()
    Timer { interval: 4000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !run.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-server-apps" }
    SetJob { id: run; helper: "hearth-server-apps" }
    SetJob { id: pick; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        Layout.bottomMargin: 4
        text: "Game servers for you and your friends, and your own versions of apps you'd otherwise pay for. Each one is off until you install it and turn it on, runs in Podman as your own service, and never holds up starting this PC or Game Mode. Your worlds and data live in Home › Server apps."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && (!!page.st.error || page.st.podman === false)
        Layout.fillWidth: true
        text: page.st && page.st.error ? page.st.error : "Podman isn't on this PC: server apps come with the Hearth OS update that brings it."
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    Component {
        id: appCard
        ColumnLayout {
            id: card
            required property var modelData
            readonly property var a: modelData
            Layout.fillWidth: true
            spacing: 0
            SetRow {
                boxed: true
                badge: "New"
                visible: !card.a.installed
                icon: card.a.icon; title: (card.a.installing ? "Installing " : "Install ") + card.a.name
                sub: page.line(card.a)
                value: card.a.installing ? card.a.progress + "%" : "Install"
                interactive: !card.a.installing && !!page.st && page.st.podman
                onActivated: page.act(["install", card.a.id], "Downloading " + card.a.name + "…")
            }
            SetSwitch {
                boxed: true
                visible: card.a.installed
                icon: card.a.icon; title: card.a.name
                sub: page.line(card.a)
                chip: !card.a.on ? "Off" : card.a.failed ? "Stopped" : card.a.running ? "Running" : "Starting"
                chipKind: card.a.on && card.a.running ? "ok" : card.a.failed ? "warn" : "off"
                checked: card.a.on
                onToggled: v => page.act(["set", card.a.id, "on", v ? "on" : "off"], v ? card.a.name + " on" : card.a.name + " off")
            }
            SetExpander {
                visible: card.a.installed
                icon: "gear"; title: card.a.name + ": address, options"
                sub: "Who can reach it, " + (card.a.opts.length ? card.a.opts.map(o => o.label.toLowerCase().replace(/ \(.*$/, "")).join(", ") + ", " : "") + "logs, update, remove"
                anchor: card.a.id
                Repeater {
                    model: card.a.addresses
                    delegate: SetRow {
                        required property var modelData
                        icon: "link"; title: modelData.where; sub: modelData.url
                        value: "Copy"
                        onActivated: { Quickshell.execDetached(["wl-copy", modelData.url]); page.say("Copied ✓"); }
                    }
                }
                SetSwitch {
                    visible: card.a.eula !== null
                    icon: "doc"; title: "I accept Minecraft's EULA"
                    sub: "Mojang's rules for running a server (" + card.a.eulaUrl + "). Needed before it can start"
                    checked: card.a.eula === true
                    onToggled: v => page.act(["set", card.a.id, "eula", v ? "on" : "off"])
                }
                SetChoice {
                    title: "Who can reach it"
                    options: [{ value: "home", label: "Home network and Tailscale" }, { value: "tailscale", label: "Tailscale only" }]
                    current: card.a.reach
                    onChosen: v => page.act(["set", card.a.id, "reach", v], v === "home" ? "Your home network and Tailscale can reach it" : "Only Tailscale and this PC can reach it")
                }
                Repeater {
                    model: card.a.opts.filter(o => o.choices.length)
                    delegate: SetChoice {
                        required property var modelData
                        title: modelData.label
                        options: modelData.choices.map(c => ({ value: c, label: c === "on" ? "On" : c === "off" ? "Off" : c.charAt(0) + c.slice(1).toLowerCase() }))
                        current: modelData.value
                        onChosen: v => page.act(["opt", card.a.id, modelData.key, v], "Saved ✓" + (card.a.on ? " · restarting " + card.a.name : ""))
                    }
                }
                Repeater {
                    model: card.a.opts.filter(o => o.text)
                    delegate: SetField {
                        required property var modelData
                        Layout.leftMargin: 56; Layout.rightMargin: 18; Layout.topMargin: 6; Layout.bottomMargin: 6
                        placeholder: modelData.label + ": " + modelData.value + " (type a new one, then Enter)"
                        onSubmitted: t => { page.act(["opt", card.a.id, modelData.key, t.trim()], modelData.label + " saved ✓"); text = ""; }
                    }
                }
                Repeater {
                    model: card.a.opts.filter(o => o.folder)
                    delegate: SetRow {
                        required property var modelData
                        icon: "folder"; title: modelData.label; sub: modelData.value
                        value: "Change"
                        onActivated: pick.run(["pick-folder"], r => { const p = (r && (r.path || r.folder)) || ""; if (p) page.act(["opt", card.a.id, modelData.key, p], "Folder saved ✓"); }, e => page.say(e, true))
                    }
                }
                SetField {
                    visible: !!card.a.password
                    secret: true
                    Layout.leftMargin: 56; Layout.rightMargin: 18; Layout.topMargin: 6; Layout.bottomMargin: 6
                    placeholder: card.a.password ? (card.a.password.set ? "Password set · type a new one, then Enter" : "A password for joining (at least " + card.a.password.min + " characters), then Enter")
                                                   + (card.a.password.required ? "" : " · empty clears it") : ""
                    onSubmitted: t => { run.run(["password", card.a.id], () => { page.say("Password saved ✓ (kept only on this PC)"); page.refresh(); }, e => page.say(e, true), t); text = ""; }
                }
                SetRow {
                    icon: "folder"; title: "Its data"; sub: card.a.data
                    value: "Open"
                    onActivated: Quickshell.execDetached(["xdg-open", card.a.data])
                }
                SetRow {
                    visible: card.a.on
                    icon: "file"; title: "Logs"
                    sub: (page.logLines[card.a.id] || []).slice(-6).join("\n") || "The last lines it printed"
                    value: "Show"
                    onActivated: run.run(["logs", card.a.id], r => { const l = Object.assign({}, page.logLines); l[card.a.id] = r.lines.length ? r.lines : [r.error || "Nothing yet"]; page.logLines = l; }, e => page.say(e, true))
                }
                SetRow {
                    icon: "update"; title: "Update " + card.a.name
                    sub: "Downloads its newest image" + (card.a.on ? " and restarts it" : "") + "; your data stays"
                    interactive: !card.a.installing
                    onActivated: page.act(["update", card.a.id], "Updating " + card.a.name + "…")
                }
                SetRow {
                    visible: !card.a.on
                    icon: "trash"; title: "Remove " + card.a.name
                    sub: "Its images go; your data stays in " + card.a.data
                    onActivated: page.act(["remove", card.a.id])
                }
                SetRow {
                    visible: !card.a.on
                    danger: true
                    icon: "trash"; title: page.removing === card.a.id && page.removeStep === 1 ? "Really remove its data too?" : "Remove " + card.a.name + " and its data"
                    sub: page.removing === card.a.id && page.removeStep === 1 ? "Press again: " + card.a.data + " is deleted for good" : "Worlds, saves and everything in " + card.a.data + " (asks twice)"
                    onActivated: {
                        if (page.removing !== card.a.id) { page.removing = card.a.id; page.removeStep = 1; return; }
                        page.removing = ""; page.removeStep = 0;
                        page.act(["remove", card.a.id, "data", "sure", "really-sure"]);
                    }
                }
            }
        }
    }

    SetHeader { text: "Game servers"; visible: page.apps("game").length > 0 }
    Repeater { model: page.apps("game"); delegate: appCard }
    SetHeader { text: "Your own apps"; visible: page.apps("app").length > 0 }
    Repeater { model: page.apps("app"); delegate: appCard }

    SetHeader { text: "Also"; visible: !!page.st }
    SetRow { boxed: true; chevron: true; visible: !!page.st
        icon: "bell"; title: "Phone notifications"
        sub: "Hear on your phone (the ntfy app) when an alert comes up or a server app stops"
        onActivated: SettingsNav.openPage("hs-phone")
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

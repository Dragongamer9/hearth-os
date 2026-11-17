// Hearth Settings › Privacy & security › Firewall (1.14): firewalld or ufw, whichever this system has (hearth-firewall).
// On or off, whether other devices can connect in, and the ports let in (yours and the ones Hearth's apps open while
// they run). Guarded: turning it on keeps SSH open while this PC's SSH server runs and keeps Tailscale trusted, so you
// can't lock yourself out of a PC you reach from elsewhere; SSH can't be closed while its server runs. Turning it off,
// letting a port in and letting everything in ask for an administrator's password. Reading it changes nothing.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Firewall"
    property var st: null
    property string message: ""
    property bool messageBad: false
    property string proto: "tcp"
    property string from: "local"
    property string confirm: ""
    readonly property bool none: !!st && st.backend === "none"
    readonly property string name: st ? (st.backend === "firewalld" ? "firewalld" : st.backend === "ufw" ? "ufw" : "") : ""

    function say(t, bad) { message = t; messageBad = !!bad; }
    // As you first; ufw only answers root, so its rules come through hearth-admin (no prompt: Hearth's polkit rule).
    function load() { job.run(["status"], r => { page.st = r; if (r.needsAdmin) admin.run(["firewall-status"], a => page.st = a, e => page.say(e, true)); }, e => page.say(e, true)); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "/usr/libexec/nimbus/hearth-firewall" }
    SetJob { id: admin; helper: "/usr/libexec/nimbus/hearth-admin"; admin: true }
    function act(args, input, ok, label) {
        say("");
        admin.run(args, r => { page.st = r; page.confirm = ""; page.say(ok); }, e => page.say(e, true), input || "", label || "act");
    }

    SetCardGroup {
        title: "Firewall"
        SetRow {
            visible: page.none
            interactive: false; chevron: false; icon: "info"
            title: "There's no firewall on this system"
            sub: "Hearth OS doesn't ship one: your router already keeps the internet out. With firewalld or ufw installed (Peppermint unlocked), it's set here"
        }
        SetSwitch {
            visible: !!page.st && !page.none
            icon: "firewall"; title: "Firewall (" + page.name + ")"
            sub: admin.what === "act" ? "Changing…" : page.st && page.st.on ? "On: only what's listed below can connect to this PC" : "Off: anything on your network can reach this PC's open ports"
            checked: !!page.st && page.st.on
            onToggled: v => page.act([v ? "firewall-on" : "firewall-off"], "", v ? "The firewall is on." + (page.st.ssh ? " SSH stays open (this PC's SSH server runs)." : "") : "The firewall is off.")
        }
        SetChoice {
            visible: !!page.st && !page.none && page.st.on
            icon: "shield"; title: "Other devices connecting in"
            sub: page.st && page.st.backend === "firewalld" ? "firewalld's zone: " + (page.st.zone || "public") : "What happens to a connection nothing below lets in"
            options: [{ value: "deny", label: "Blocked unless listed" }, { value: "allow", label: "All let in" }]
            current: page.st ? page.st.incoming : "deny"
            onChosen: v => { if (v !== page.st.incoming) page.act(["firewall-incoming", v], "", v === "allow" ? "Everything is let in now." : "Only what's listed is let in."); }
        }
        SetRow {
            visible: !!page.st && !page.none && page.st.on
            interactive: false; chevron: false; icon: "tailscale"
            title: page.st && page.st.tailscale ? "Tailscale is trusted" : "Tailscale"
            sub: page.st && page.st.tailscale ? "Your own devices reach this PC through Tailscale whatever the rules say" : "Turn the firewall off and on again to trust it"
        }
    }

    SetCardGroup {
        title: "Let in"
        visible: !!page.st && !page.none && page.st.on
        Repeater {
            model: page.st ? page.st.rules : []
            delegate: ColumnLayout {
                id: rr
                required property var modelData
                Layout.fillWidth: true
                spacing: 6
                SetRow {
                    icon: modelData.hearth ? "sparkle" : "link"
                    title: (modelData.label || "Port " + modelData.port) + (modelData.label && modelData.proto ? "  ·  " + modelData.port : "")
                    sub: (modelData.proto ? modelData.proto.toUpperCase() + " · " : "") + (modelData.from === "local" ? "From this home network" : modelData.from === "any" ? "From anywhere" : "From " + modelData.from)
                         + (modelData.hearth ? " · opened by Hearth while the app runs" : "")
                    chevron: false
                    onActivated: page.confirm = page.confirm === modelData.id ? "" : modelData.id
                }
                SetRow {
                    visible: page.confirm === rr.modelData.id
                    Layout.leftMargin: 52
                    icon: "trash"; danger: true; chevron: false
                    title: "Stop letting this in"
                    onActivated: page.act(["firewall-remove"], JSON.stringify({ id: rr.modelData.id }), "Removed.", "remove")
                }
            }
        }
        Text {
            visible: !!page.st && page.st.rules.length === 0
            Layout.fillWidth: true
            text: page.st && page.st.needsAdmin ? "Reading ufw's rules…" : "Nothing is let in. Nearby Share, Hearth Connect and Hearth Link open their own ports while they run."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }

    SetCardGroup {
        title: "Let a port in"
        visible: !!page.st && !page.none && page.st.on
        SetField { id: port; placeholder: "Port, like 25565 (a Minecraft server) or a range like 6000-6010" }
        SetField { id: label; placeholder: "What it's for (optional), like Minecraft" }
        SetChoice {
            icon: "link"; title: "Kind"
            options: [{ value: "tcp", label: "TCP" }, { value: "udp", label: "UDP" }, { value: "both", label: "Both" }]
            current: page.proto
            onChosen: v => page.proto = v
        }
        SetChoice {
            icon: "globe"; title: "From"
            sub: "Anywhere matters only if your router forwards the port"
            options: [{ value: "local", label: "This home network" }, { value: "any", label: "Anywhere" }]
            current: page.from
            onChosen: v => page.from = v
        }
        SetRow {
            icon: "plus"; chevron: false
            title: admin.what === "allow" ? "Letting it in…" : "Let it in"
            sub: "Asks for an administrator's password"
            onActivated: { page.act(["firewall-allow"], JSON.stringify({ port: port.text.trim(), proto: page.proto, from: page.from, label: label.text.trim() }), "Let in.", "allow"); port.text = ""; label.text = ""; }
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

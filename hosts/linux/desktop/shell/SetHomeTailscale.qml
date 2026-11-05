// Hearth Settings › Home server › Tailscale (docs/HOME-SERVER.md): a private network for your devices only. Sign in
// with a QR code or a link, this PC's name and address, your devices with their status and ping, "Only reachable
// through Tailscale" for the shares (the firewall lets Samba in only on tailscale0), start at boot, the exit node
// (off), Taildrop and waking this PC from away. Sign-in links and keys are never written to a log.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Tailscale"
    property var st: null
    property var login: null
    property var pings: ({})
    property string msg: ""
    property bool bad: false
    readonly property var cfg: st ? st.settings : null
    readonly property var ts: st ? (st.tailscale || {}) : ({})
    readonly property bool connected: ts.state === "Running"
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => { page.st = r; if (page.connected) page.login = null; }, e => page.st = { error: e }); }
    function set(k, v, ok) { act.run(["set", k, v], r => { if (ok) page.say(ok + (r.applied && r.applied.notes && r.applied.notes.length ? " · " + r.applied.notes.join(" · ") : "")); page.refresh(); }, e => { page.say(e, true); page.refresh(); }); }
    function ping() { if (page.connected) pj.run(["ts-ping"], r => page.pings = r.pings || {}, () => {}); }
    Component.onCompleted: { refresh(); }
    onConnectedChanged: ping()
    Timer { interval: 4000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy) page.refresh() }
    Timer { interval: 30000; running: page.visible && page.connected; repeat: true; onTriggered: if (!pj.busy) page.ping() }
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }
    SetJob { id: pj; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: "A private network for your devices only. Reach this PC from work or anywhere, with nothing opened to the internet."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && page.ts.installed === false
        Layout.fillWidth: true
        text: "Tailscale isn't on this PC yet: it comes with the Hearth OS update that brings Home server. Your choices below are kept."
        color: Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetHeader { text: page.connected ? "Signed in" : "Sign in" }
    SetRow {
        visible: page.ts.installed === true && page.ts.running === false
        icon: "power"; title: "Turn on Tailscale"
        sub: "Starts it now. Whether it starts with the PC is the switch below."
        onActivated: act.run(["ts-start"], () => { page.say("Tailscale is on"); page.refresh(); }, e => page.say(e, true))
    }
    SetRow {
        visible: page.ts.installed === true && page.ts.running !== false && !page.connected
        icon: "lock"; title: page.login ? "Waiting for you to sign in…" : "Sign in to Tailscale"
        sub: page.login ? "Scan the code with your phone, or open the link on any device. Then this page updates by itself." : "With Google, Microsoft, GitHub or Apple. Free for personal use."
        onActivated: act.run(["ts-login"], r => { page.login = r; if (!r.authUrl && r.state !== "Running") page.say("Tailscale didn't give a sign-in link yet: try again", true); page.refresh(); }, e => page.say(e, true))
    }
    RowLayout {
        visible: !!page.login && !!page.login.authUrl && !page.connected
        Layout.fillWidth: true
        spacing: 18
        Rectangle {
            visible: !!page.login && page.login.qr !== ""
            Layout.preferredWidth: 220; Layout.preferredHeight: 220
            radius: Theme.r(12); color: "white"
            Image {
                anchors.fill: parent
                anchors.margins: 10
                source: page.login && page.login.qr ? "file://" + page.login.qr : ""
                cache: false; smooth: false; fillMode: Image.PreserveAspectFit
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 6
            Text { Layout.fillWidth: true; text: "1. Scan with your phone, or open:"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
            Text { Layout.fillWidth: true; text: page.login ? page.login.authUrl : ""; color: Theme.flame; font.family: "monospace"; font.pixelSize: 13; wrapMode: Text.WrapAnywhere }
            Text { Layout.fillWidth: true; text: "2. Sign in with Google, Microsoft, GitHub or Apple"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
            Text { Layout.fillWidth: true; text: "3. This PC shows up as " + (page.st ? page.st.host : "this PC"); color: Theme.text; font.family: Theme.font; font.pixelSize: 14 }
        }
    }
    SetRow {
        visible: page.connected
        icon: "user"; chevron: false; interactive: false
        title: "Signed in" + (page.ts.login ? " as " + page.ts.login : "")
        value: "Connected"
        sub: page.ts.tailnet ? "Your tailnet: " + page.ts.tailnet : ""
    }
    SetRow {
        visible: page.connected
        icon: "display"; chevron: false; interactive: false; title: "This PC's name"
        value: (page.ts.dns || page.ts.name) + (page.ts.ips && page.ts.ips.length ? " · " + page.ts.ips[0] : "")
    }
    SetRow {
        visible: page.connected
        icon: "logout"; title: "Sign out"; chevron: false
        sub: "This PC leaves your tailnet until you sign in again"
        onActivated: act.run(["ts-logout"], () => { page.say("Signed out"); page.refresh(); }, e => page.say(e, true))
    }

    SetHeader { text: "Settings" }
    SetSwitch {
        visible: !!page.cfg
        icon: "lock"; title: "Only reachable through Tailscale"
        sub: "Your shares can't be opened from your home network, only from your devices on Tailscale (at home or away). The firewall lets sharing in only through Tailscale."
        checked: !!page.cfg && page.cfg.tsOnly
        onToggled: v => page.set("tsOnly", v ? "on" : "off", v ? "Only through Tailscale now" : "Your home network can open your shares again")
    }
    SetSwitch {
        visible: !!page.cfg
        icon: "power"; title: "Start Tailscale when this PC starts"
        sub: "Off: Tailscale runs only after you turn it on here. Either way, starting the PC never waits for it."
        checked: !!page.cfg && page.cfg.tsBoot
        onToggled: v => page.set("tsBoot", v ? "on" : "off")
    }
    SetSwitch {
        visible: !!page.cfg
        icon: "globe"; title: "Let other devices use this PC's internet (exit node)"
        sub: "Off unless you turn it on. Your devices can send their internet through this PC (you also allow it in Tailscale's admin page)."
        checked: !!page.cfg && page.cfg.exitNode
        onToggled: v => page.set("exitNode", v ? "on" : "off")
    }

    SetHeader { text: "Your devices"; visible: page.connected }
    Repeater {
        model: page.connected ? (page.ts.peers || []) : []
        delegate: SetRow {
            required property var modelData
            icon: /android|ios/i.test(modelData.os) ? "pad" : "display"; title: modelData.name
            value: !modelData.online ? "offline" : page.pings[modelData.name] ? page.pings[modelData.name].ms + " ms" + (page.pings[modelData.name].via === "relay" ? " (relay)" : "") : "online"
            sub: [modelData.os, modelData.ips.length ? modelData.ips[0] : "", modelData.online ? "Select to send it a file (Taildrop)" : ""].filter(x => x).join(" · ")
            onActivated: { if (modelData.online) act.run(["ts-send", modelData.name], r => { if (!r.cancelled) page.say("Sent " + r.sent + " to " + r.to); }, e => page.say(e, true)); }
        }
    }
    Text {
        visible: page.connected && (page.ts.peers || []).length === 0
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "No other devices yet. Install Tailscale on your phone or laptop and sign in with the same account."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Files and waking" }
    SetSwitch {
        visible: !!page.cfg
        icon: "file"; title: "Receive files (Taildrop)"
        sub: "Files your devices send to this PC land in Downloads › Taildrop. A name that's taken gets a new one."
        checked: !!page.cfg && page.cfg.taildropReceive
        onToggled: v => page.set("taildropReceive", v ? "on" : "off")
    }
    SetSwitch {
        visible: !!page.cfg
        icon: "sleep"; title: "Wake this PC from away"
        sub: "Its network card wakes the PC from sleep (Wake-on-LAN). From away, another of your devices at home sends the wake-up"
             + (page.st && page.st.wired && page.st.wired.length ? " · " + page.st.wired.map(w => w.iface + " " + w.mac).join(", ") : " · needs a network cable")
        checked: !!page.cfg && page.cfg.wakeFromAway
        onToggled: v => page.set("wakeFromAway", v ? "on" : "off", v ? "Ready to wake from the network" : "Off")
    }
    Text {
        visible: !!page.st && !!page.st.applied && page.st.applied.ok === false
        Layout.fillWidth: true
        text: page.st && page.st.applied ? "Saved, but the system couldn't be changed to match: " + (page.st.applied.error || "") : ""
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

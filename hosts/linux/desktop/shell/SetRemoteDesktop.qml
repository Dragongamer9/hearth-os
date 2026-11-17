// Hearth Settings › System › Remote desktop: this PC's desktop in any VNC app (TigerVNC, Remmina, KRDC) on your other
// devices, through Tailscale, or your home network too; never the internet. Its helper is hearth-vnc (choices in
// ~/.config/nimbus/remote-vnc.json, written only when you change one; the password in your keyring, never a file).
// Opening this page only reads (hearth-vnc status): nothing is written, started or asked until you press something.
// Settings › Remote play (SetRemote.qml) stays as it was: Moonlight and Sunshine for games and low-latency streaming.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Remote desktop"
    property var st: null
    property string msg: ""
    property bool bad: false
    property bool pwOpen: false
    readonly property bool on: !!page.st && !!page.st.on
    SetJob { id: job; helper: "hearth-vnc" }
    SetJob { id: act; helper: "hearth-vnc" }
    function load() { job.run(["status"], r => page.st = r, e => page.st = { error: e }); }
    function say(t, b) { page.msg = t; page.bad = !!b; }
    function runIt(args, said, label, input) {
        act.run(args, r => { page.say(said + (r && r.note ? " " + r.note : ""), !!(r && r.note)); page.load(); },
                e => { page.say(e, true); page.load(); }, input || "", label || "");
    }
    function since(t) {
        const m = Math.max(0, Math.round((Date.now() / 1000 - t) / 60));
        return m < 1 ? "just now" : m < 60 ? m + " min" : Math.floor(m / 60) + " h " + (m % 60) + " min";
    }
    function address() {
        if (!page.st || !page.st.addresses) return "";
        const t = page.st.addresses.tailscale || {};
        const a = t.name || t.ip || "";
        return a ? a + (page.st.port && page.st.port !== 5900 ? ":" + page.st.port : "") : "";
    }
    function inputLine() {
        if (!page.st) return "";
        if (page.st.viewOnly) return "View only: they see the screen but can't type or click";
        if (page.st.input === "portal") return "They can use the keyboard and mouse, through the desktop's own remote control";
        if (page.st.input === "uinput") return "They can use the keyboard and mouse (a virtual keyboard and mouse: this desktop has no remote control of its own)";
        return "View only for now: " + (page.st.inputWhy || "this desktop can't take typing from another device");
    }
    Timer { interval: 5000; running: page.visible && page.on; repeat: true; onTriggered: if (!job.busy && !act.busy) page.load() }
    Component.onCompleted: load()

    Text {
        visible: !page.st
        text: "Loading…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
    Text {
        visible: !!page.st && !!page.st.error && page.st.on === undefined
        Layout.fillWidth: true
        text: page.st && page.st.error ? page.st.error : ""; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetSwitch {
        visible: !!page.st && page.st.on !== undefined
        icon: "desktops"; title: "Remote desktop"
        sub: act.busy && act.what === "on" ? "Working on it…"
             : page.on ? (page.st.running ? "On: open a VNC app on your other device and connect to this PC" : "On, but the server isn't running. Turn it off and on again")
             : "Use this PC's desktop from a VNC app on your laptop, tablet or phone, through Tailscale. Off until you turn it on"
        checked: page.on
        onToggled: on => page.runIt(["set", "on", on ? "on" : "off"], on ? "On. The first time someone connects, this PC asks you to allow sharing the screen." : "Off. Nobody can connect.", "on")
    }
    Repeater {
        model: page.on ? (page.st.tips || []) : []
        delegate: Text {
            required property var modelData
            Layout.fillWidth: true
            text: modelData + "."; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }

    // ---- who's connected
    SetHeader { text: "Connected now"; visible: page.on && (page.st.clients || []).length > 0 }
    Repeater {
        model: page.on ? (page.st.clients || []) : []
        delegate: SetRow {
            required property var modelData
            icon: "eye"; title: (modelData.name || modelData.ip) + (modelData.viewOnly ? " (view only)" : "")
            sub: (modelData.via === "tailscale" ? "Through Tailscale" : modelData.via === "home" ? "On your home network" : "On this PC") + " · for " + page.since(modelData.since)
            value: "Disconnect"
            onActivated: page.runIt(["disconnect", String(modelData.id)], "Disconnected.", "kick")
        }
    }

    // ---- how to connect
    SetHeader { text: "Connect from another device"; visible: page.on }
    SetRow {
        visible: page.on && page.address() !== ""
        icon: "globe"; chevron: false; interactive: false
        title: "In your VNC app, connect to " + page.address()
        sub: "Sign in to Tailscale on that device with the same account first. Pick VeNCrypt or TLS if the app asks, and use the password below"
    }
    SetRow { chevron: true;
        visible: page.on && page.address() === ""
        icon: "globe"; title: "Tailscale isn't connected"
        sub: "Your other devices reach this PC through Tailscale. Sign in here and on the other device"
        onActivated: SettingsNav.openPage("hs-tailscale")
    }
    SetRow {
        visible: page.on && page.st.reach === "home" && (page.st.addresses.home || []).length > 0
        icon: "wired"; chevron: false; interactive: false
        title: "At home: " + (page.on && page.st.addresses ? (page.st.addresses.home || []).join(" or ") : "")
        sub: "Devices on your home network can connect there too"
    }
    SetRow {
        visible: page.on && !!page.st.fingerprint
        icon: "shield"; chevron: false; interactive: false
        title: "This PC's certificate"
        sub: "If your VNC app asks whether to trust this PC, check it shows: " + (page.st ? page.st.fingerprint : "")
    }

    // ---- the password
    SetHeader { text: "Password"; visible: page.on }
    SetRow {
        visible: page.on
        icon: "key"
        title: page.st && page.st.password ? "Change the password" : "Set a password"
        sub: act.busy && act.what === "pw" ? "Saving…"
             : page.st && page.st.keyring === "locked" ? "Your keyring is locked: sign out and in again, then set it"
             : page.st && page.st.keyring === "missing" ? "The keyring tool (secret-tool) isn't installed, so a password can't be kept"
             : page.st && page.st.password ? "Kept in your keyring, never in a file. Every connection needs it" : "Nobody can connect until there is one: at least 8 characters"
        onActivated: { page.pwOpen = !page.pwOpen; if (page.pwOpen) pw.focusInput(); }
    }
    SetField {
        id: pw; visible: page.pwOpen; secret: true; placeholder: "The new password (at least 8 characters), then Enter"
        onSubmitted: t => {
            if (t.length < 8) { page.say("Use at least 8 characters", true); return; }
            page.runIt(["password"], "The password is set.", "pw", t);
            pw.text = ""; page.pwOpen = false;
        }
    }

    // ---- choices
    SetHeader { text: "Who can connect and what they can do"; visible: page.on }
    SetChoice {
        visible: page.on
        title: "Reachable"
        sub: "Never from the internet. Tailscale only keeps it closed to your home network too"
        options: [{ value: "tailscale", label: "Tailscale only" }, { value: "home", label: "Tailscale and home network" }]
        current: page.st ? page.st.reach : "tailscale"
        onChosen: v => page.runIt(["set", "reach", v], v === "home" ? "Your home network can connect too." : "Only through Tailscale now.", "reach")
    }
    SetSwitch {
        visible: page.on
        icon: "eye"; title: "View only"
        sub: page.inputLine()
        checked: !!page.st && !!page.st.viewOnly
        onToggled: on => page.runIt(["set", "viewOnly", on ? "on" : "off"], on ? "View only: nobody can type or click from another device." : "They can use the keyboard and mouse again.", "view")
    }
    SetRow {
        visible: page.on && (page.st.clients || []).length > 1
        icon: "close"; title: "Disconnect everyone"
        onActivated: page.runIt(["disconnect", "all"], "Everyone is disconnected.", "kick")
    }
    SetRow { chevron: true;
        icon: "play"; title: "Games and low-latency streaming"
        sub: "For playing this PC's games on a TV, phone or another PC, Remote play (Moonlight) is faster"
        onActivated: SettingsNav.openPage("remote")
    }

    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.on
        Layout.fillWidth: true
        text: "Every connection is encrypted (TLS) and shows a notification and a dot on the bar while it lasts; pick it there to disconnect. Five wrong passwords lock that device out for five minutes. The first time, this PC asks you to allow sharing its screen; on KDE's desktop that answer is remembered."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
}

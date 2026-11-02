// Hearth Settings › Remote play: stream this device's games to a TV, phone, tablet or another PC with Moonlight
// (Sunshine, which comes with Hearth OS), and play another PC's games here. Moved from the archived Game Mode's
// Settings › Connections › Remote play, with the same calls (hearth-core remote.rs through HearthCall).
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
    function load() { core.call("remote.status", {}, v => { page.st = v; page.err = ""; }, e => page.err = e); }
    function act(method, args, k, said) {
        page.busy = k; page.msg = "";
        core.call(method, args, () => { page.busy = ""; if (said) page.msg = said; page.load(); }, e => { page.busy = ""; page.msg = e; page.load(); });
    }
    Component.onCompleted: load()

    Text {
        visible: !page.st && page.err === ""
        text: "Loading…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
    Text {
        visible: page.err !== ""
        Layout.fillWidth: true
        text: page.err; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
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
        sub: page.busy === "on" ? "Working on it…" : "Play your games on a TV, phone, tablet or another PC with the free Moonlight app"
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
        visible: !!page.st && page.st.on
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
    SetHeader { text: "Paired devices"; visible: !!page.st && page.st.on && (page.st.clients || []).length > 0 }
    Repeater {
        model: page.st && page.st.on ? (page.st.clients || []) : []
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

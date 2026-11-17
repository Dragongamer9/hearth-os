// Hearth Settings › Network & internet › Mobile hotspot: share this PC's internet over Wi-Fi (NetworkManager's shared
// mode, connection "Hearth Hotspot"): on and off, its name, password and band. The password goes to NetworkManager on
// nmcli's stdin (hearth-network) and is read back from it only when you press Show. Off unless you turn it on; it never
// starts by itself.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Mobile hotspot"
    property var st: null              // hearth-network hotspot
    property string shownPassword: ""
    property bool editing: false
    property string message: ""
    property bool messageBad: false
    property string band: ""
    onVisibleChanged: if (!visible) { shownPassword = ""; editing = false; }

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["hotspot"], r => { page.st = r; page.band = r.band || ""; }, e => page.say(e, true)); }
    Component.onCompleted: load()
    Timer { interval: 15000; repeat: true; running: page.visible && !job.busy && !page.editing; onTriggered: page.load() }
    SetJob { id: job; helper: "hearth-network" }
    readonly property bool made: !!st && st.uuid !== ""

    function turnOn(ssid, pw) {
        say("");
        job.run(["hotspot-on"], r => { page.st = r; page.editing = false; page.shownPassword = ""; page.say("Hotspot on: join \"" + r.ssid + "\" from your other devices."); },
                e => page.say(e, true), JSON.stringify({ ssid: ssid, password: pw || "", band: page.band }), "on");
    }

    SetCardGroup {
        title: "Mobile hotspot"
        SetSwitch {
            icon: "share"; title: "Share my internet"
            sub: job.what === "on" ? "Starting…" : !page.st ? "Checking…" : !page.st.device ? "This PC has no Wi-Fi" : !page.st.canHotspot ? "This PC's Wi-Fi can't be a hotspot"
                 : page.st.on ? "On as \"" + page.st.ssid + "\". Wi-Fi to other networks is off while it's on" : page.made ? "Off · \"" + page.st.ssid + "\"" : "Set a name and password below first"
            enabled: !!page.st && page.st.canHotspot && (page.made || page.st.on)
            checked: !!page.st && page.st.on
            onToggled: v => {
                if (v) page.turnOn(page.st.ssid, "");
                else job.run(["hotspot-off"], r => { page.st = r; page.say("Hotspot off."); }, e => page.say(e, true), "", "off");
            }
        }
        SetRow {
            visible: page.made
            icon: "eye"; chevron: false
            title: page.shownPassword ? page.shownPassword : "Show the password"
            sub: page.shownPassword ? "The password for \"" + page.st.ssid + "\"" : "To type it on your phone"
            onActivated: {
                if (page.shownPassword) { page.shownPassword = ""; return; }
                job.run(["hotspot-password"], r => page.shownPassword = r.password || "", e => page.say(e, true), "", "pw");
            }
        }
    }
    SetCardGroup {
        title: page.made ? "Name and password" : "Set up the hotspot"
        visible: !!page.st && page.st.canHotspot
        SetField { id: ssid; placeholder: page.made ? "Name: " + page.st.ssid : "Network name, like " + (Quickshell.env("HOSTNAME") || "Hearth") + " hotspot"; onSubmitted: pass.focusInput(); onTextChanged: page.editing = text !== "" }
        SetField { id: pass; secret: true; placeholder: page.made ? "New password (leave empty to keep it)" : "Password (8 letters or more)"; onSubmitted: saveBtn.activated(); onTextChanged: page.editing = text !== "" }
        SetChoice {
            icon: "wifi"; title: "Band"; sub: "5 GHz is faster; 2.4 GHz reaches further and works with older devices"
            options: [{ value: "", label: "Any" }, { value: "bg", label: "2.4 GHz" }, { value: "a", label: "5 GHz" }]
            current: page.band
            onChosen: v => page.band = v
        }
        SetRow {
            id: saveBtn
            icon: "check"; chevron: false; title: job.what === "on" ? "Starting…" : page.made ? "Save and turn on" : "Turn on the hotspot"
            onActivated: {
                const n = ssid.text.trim() || (page.made ? page.st.ssid : "");
                if (!n) return page.say("Type a name for the hotspot.", true);
                if (!page.made && pass.text.length < 8) return page.say("The password needs 8 letters or more.", true);
                page.turnOn(n, pass.text);
                ssid.text = ""; pass.text = "";
            }
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}

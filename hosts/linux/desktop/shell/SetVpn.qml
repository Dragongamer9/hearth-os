// Hearth Settings › Network & internet › VPN: your VPNs (WireGuard and OpenVPN, kept by NetworkManager), connect and
// disconnect, add one from its file (.conf or .ovpn, the file your VPN provider gives you), remove one. An OpenVPN
// account's user name and password are asked when you connect and go to NetworkManager on nmcli's stdin
// (hearth-network): Hearth never keeps them. Nothing here changes by itself.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "VPN"
    property var st: null              // hearth-network status
    property string picked: ""         // the VPN whose actions are open
    property string asking: ""         // the OpenVPN asking for its account
    property string confirm: ""
    property string message: ""
    property bool messageBad: false
    readonly property var vpns: st ? st.vpns : []

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["status"], r => page.st = r, e => page.say(e, true)); }
    Component.onCompleted: load()
    Timer { interval: 10000; repeat: true; running: page.visible && !job.busy && page.asking === ""; onTriggered: page.load() }
    SetJob { id: job; helper: "hearth-network" }

    function connect(v, user, pw) {
        say("");
        const input = pw !== undefined ? JSON.stringify({ user: user || "", password: pw }) : "";
        job.run(["vpn-up", v.uuid], () => { page.asking = ""; page.say("Connected to " + v.name + "."); page.load(); },
                e => { page.say(e, true); if (v.vpnType === "OpenVPN") page.asking = v.uuid; page.load(); }, input, "up");
    }
    Process {
        id: picker
        command: ["sh", "-c", "if command -v kdialog >/dev/null; then kdialog --title 'Add a VPN' --getopenfilename \"$HOME/Downloads\" 'VPN files (*.conf *.ovpn)'; elif command -v zenity >/dev/null; then zenity --file-selection --title='Add a VPN' --filename=\"$HOME/Downloads/\" --file-filter='VPN files | *.conf *.ovpn'; else exit 3; fi"]
        stdout: StdioCollector { onStreamFinished: { const f = text.trim(); if (f) page.importFile(f); } }
        onExited: code => { if (code === 3) page.say("No file picker here: type the file's place below instead.", true); }
    }
    function importFile(f) {
        say("");
        job.run(["vpn-import", f], r => { page.say(r.name + " is added. Connect it below."); page.load(); }, e => page.say(e, true), "", "import");
    }

    SetCardGroup {
        title: "Your VPNs"
        Repeater {
            model: page.vpns
            delegate: ColumnLayout {
                id: vr
                required property var modelData
                Layout.fillWidth: true
                spacing: 8
                SetRow {
                    icon: "shield"
                    title: vr.modelData.name
                    sub: vr.modelData.vpnType + (vr.modelData.active ? " · Connected" + (vr.modelData.device ? " (" + vr.modelData.device + ")" : "") : " · Not connected")
                    chip: vr.modelData.active ? "On" : ""
                    onActivated: { page.picked = page.picked === vr.modelData.uuid ? "" : vr.modelData.uuid; page.confirm = ""; }
                }
                ColumnLayout {
                    visible: page.picked === vr.modelData.uuid
                    Layout.fillWidth: true
                    Layout.leftMargin: 52
                    spacing: 8
                    SetRow {
                        visible: !vr.modelData.active
                        icon: "link"; chevron: false; title: job.what === "up" ? "Connecting…" : "Connect"
                        sub: vr.modelData.vpnType === "OpenVPN" ? "Asks for your VPN account if it needs one" : ""
                        onActivated: page.connect(vr.modelData)
                    }
                    SetRow {
                        visible: vr.modelData.active
                        icon: "close"; chevron: false; title: "Disconnect"
                        onActivated: job.run(["vpn-down", vr.modelData.uuid], () => { page.say("Disconnected."); page.load(); }, e => page.say(e, true), "", "down")
                    }
                    SetRow {
                        icon: "gear"; title: "Details"; sub: "DNS, auto-connect and addresses: in Connection details"
                        onActivated: SettingsNav.openPage("net-details")
                    }
                    SetRow {
                        visible: page.confirm !== vr.modelData.uuid
                        icon: "trash"; danger: true; chevron: false; title: "Remove this VPN…"
                        onActivated: page.confirm = vr.modelData.uuid
                    }
                    SetRow {
                        visible: page.confirm === vr.modelData.uuid
                        icon: "trash"; danger: true; chevron: false; title: "Yes, remove " + vr.modelData.name
                        onActivated: job.run(["vpn-remove", vr.modelData.uuid], () => { page.picked = ""; page.confirm = ""; page.say("VPN removed."); page.load(); }, e => page.say(e, true), "", "remove")
                    }
                    // OpenVPN's account, when it asks: straight to NetworkManager, never saved by Hearth.
                    SetField { id: vuser; visible: page.asking === vr.modelData.uuid; placeholder: "VPN user name (if it has one)"; onSubmitted: vpass.focusInput() }
                    SetField { id: vpass; visible: page.asking === vr.modelData.uuid; secret: true; placeholder: "VPN password"; onSubmitted: goBtn.activated() }
                    SetRow {
                        id: goBtn
                        visible: page.asking === vr.modelData.uuid
                        icon: "lock"; chevron: false; title: "Connect with this account"
                        sub: "Handed to NetworkManager only; Hearth doesn't keep it"
                        onActivated: { page.connect(vr.modelData, vuser.text, vpass.text); vpass.text = ""; }
                    }
                }
            }
        }
        Text {
            visible: !!page.st && page.vpns.length === 0
            Layout.fillWidth: true
            text: "No VPNs yet. Add one from the file your VPN gives you (WireGuard .conf or OpenVPN .ovpn)."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
    SetCardGroup {
        title: "Add a VPN"
        SetRow {
            icon: "plus"; title: job.what === "import" ? "Adding…" : "Add from a file…"
            sub: "WireGuard (.conf)" + (page.st && page.st.plugins && page.st.plugins.openvpn ? " or OpenVPN (.ovpn)" : "") + ", from your VPN provider or router"
            onActivated: picker.running = true
        }
        SetField { id: pathField; placeholder: "Or the file's place, like ~/Downloads/home.conf"; onSubmitted: addPath.activated() }
        SetRow {
            id: addPath
            icon: "file"; chevron: false; title: "Add this file"
            onActivated: { const f = pathField.text.trim().replace(/^~(?=\/)/, Quickshell.env("HOME")); if (f) { page.importFile(f); pathField.text = ""; } }
        }
        SetRow {
            visible: !!page.st && page.st.plugins && !page.st.plugins.openvpn
            interactive: false; chevron: false
            icon: "info"; title: "OpenVPN isn't on this system"; sub: "WireGuard works. OpenVPN needs NetworkManager's OpenVPN plugin in the image"
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}

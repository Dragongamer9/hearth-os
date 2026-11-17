// Hearth Settings › Network & internet › VPN: your VPNs (kept by NetworkManager), connect and disconnect, add one from
// its file (.conf or .ovpn, the file your VPN provider gives you), remove one. 1.14: or set one up by hand: IPsec,
// L2TP/IPsec, PPTP, OpenConnect, SSTP and Fortinet, each only when NetworkManager's plugin for it is installed (the
// others say which plugin they need). Passwords go to NetworkManager on nmcli's stdin (hearth-network) and
// NetworkManager keeps them: Hearth never does. Nothing here changes by itself.
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
    property string adding: ""         // the kind of VPN being typed in by hand (1.14)
    property string ocProto: "anyconnect"
    property string message: ""
    property bool messageBad: false
    readonly property var vpns: st ? st.vpns : []
    readonly property var types: st && st.vpnTypes ? st.vpnTypes : []
    readonly property var addType: types.find(t => t.id === adding) || null
    readonly property string addPlugin: addType ? addType.plugin : ""
    // Settings opened at "vpn:add-<kind>" (the pictures, a link): that kind's form open.
    // Opened at a part: once laid out, that part is scrolled into view (the page is a Flickable).
    property var showTarget: null
    function showSoon(it) { showTarget = it; showTimer.restart(); }
    Timer { id: showTimer; interval: 700; onTriggered: { const it = page.showTarget; if (!it || !it.visible) return; const y = it.mapToItem(page.contentItem, 0, 0).y; page.contentY = Math.max(0, Math.min(page.contentHeight - page.height, y - 60)); } }
    // Settings can make a page twice while it opens: the anchor is left a moment for the page that stays, then cleared.
    Timer { id: anchorDone; interval: 1500; onTriggered: SettingsNav.pageAnchor = "" }
    Component.onCompleted: { load(); const m = /^add-(\w+)$/.exec(SettingsNav.pageAnchor); if (m) { adding = m[1]; anchorDone.restart(); showSoon(vname); } }
    function addByHand() {
        say("");
        const req = { type: page.adding, name: vname.text.trim(), gateway: vserver.text.trim(), user: vaddUser.text.trim(),
                      password: vaddPass.text, psk: vpsk.text, group: vgroup.text.trim(), domain: vdomain.text.trim(),
                      realm: vdomain.text.trim(), protocol: page.ocProto };
        job.run(["vpn-add"], r => { page.say(r.name + " is added" + (r.warning ? ". " + r.warning : ". Connect it above.")); page.adding = ""; vaddPass.text = ""; vpsk.text = ""; page.load(); },
                e => page.say(e, true), JSON.stringify(req), "add");
    }

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["status"], r => page.st = r, e => page.say(e, true)); }
    Component.onDestruction: { vaddPass.text = ""; vpsk.text = ""; }
    Timer { interval: 10000; repeat: true; running: page.visible && !job.busy && page.asking === ""; onTriggered: page.load() }
    SetJob { id: job; helper: "hearth-network" }

    function connect(v, user, pw) {
        say("");
        const input = pw !== undefined ? JSON.stringify({ user: user || "", password: pw }) : "";
        job.run(["vpn-up", v.uuid], () => { page.asking = ""; page.say("Connected to " + v.name + "."); page.load(); },
                e => { page.say(e, true); if (v.vpnType !== "WireGuard") page.asking = v.uuid; page.load(); }, input, "up");
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
                        sub: vr.modelData.vpnType !== "WireGuard" ? "Asks for your VPN account if it needs one" : ""
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
    }
    // 1.14: a VPN typed in by hand. Every kind is listed; the ones whose plugin isn't installed say so and can't be picked.
    SetCardGroup {
        title: "Set up a VPN by hand"
        Repeater {
            model: page.types
            delegate: SetRow {
                required property var modelData
                icon: modelData.installed ? "shield" : "info"
                interactive: modelData.installed
                chevron: modelData.installed
                title: modelData.label
                sub: modelData.installed ? (page.adding === modelData.id ? "Fill in the details below" : "Server, user name and password from your work or VPN provider")
                                         : "Not on this system: needs NetworkManager's " + modelData.package + " plugin"
                value: page.adding === modelData.id ? "Setting up" : ""
                onActivated: { page.adding = page.adding === modelData.id ? "" : modelData.id; page.say(""); }
            }
        }
        ColumnLayout {
            visible: !!page.addType && page.addType.installed
            Layout.fillWidth: true
            Layout.leftMargin: 52
            spacing: 8
            SetField { id: vname; placeholder: "A name for it, like Work VPN" }
            SetField { id: vserver; placeholder: page.adding === "openconnect" ? "Server, like vpn.example.com or https://vpn.example.com/group" : "Server (gateway), like vpn.example.com" }
            SetChoice {
                visible: page.adding === "openconnect"
                icon: "globe"; title: "Kind of server"
                options: [{ value: "anyconnect", label: "Cisco AnyConnect" }, { value: "gp", label: "GlobalProtect" }, { value: "pulse", label: "Pulse / Ivanti" },
                          { value: "nc", label: "Juniper" }, { value: "f5", label: "F5" }, { value: "fortinet", label: "Fortinet" }]
                current: page.ocProto
                onChosen: v => page.ocProto = v
            }
            SetField { id: vaddUser; visible: page.adding !== "openconnect"; placeholder: page.adding === "ipsec" ? "User name (leave empty for a pre-shared key only)" : "User name" }
            SetField { id: vaddPass; visible: page.adding !== "openconnect"; secret: true; placeholder: "Password (NetworkManager keeps it, not Hearth)" }
            SetField { id: vpsk; visible: page.adding === "ipsec" || page.adding === "l2tp"; secret: true; placeholder: page.adding === "l2tp" ? "IPsec pre-shared key (if your VPN uses one)" : "Pre-shared key (if your VPN uses one)" }
            SetField { id: vgroup; visible: page.adding === "openconnect" || page.addPlugin === "libreswan"; placeholder: page.adding === "openconnect" ? "Group (if your server has several; optional)" : "Group name (optional)" }
            SetField { id: vdomain; visible: page.adding === "pptp" || page.adding === "fortisslvpn"; placeholder: page.adding === "pptp" ? "Windows domain (optional)" : "Realm (optional)" }
            SetRow {
                icon: "info"; interactive: false; chevron: false
                visible: page.adding === "openconnect"
                title: "Your user name and password are asked when you connect"
                sub: "OpenConnect signs in on the server's own sign-in page each time"
            }
            SetRow {
                icon: "plus"; chevron: false
                title: job.what === "add" ? "Adding…" : "Add this VPN"
                sub: "It connects only when you press Connect"
                onActivated: page.addByHand()
            }
            Text {
                visible: page.message !== ""
                Layout.fillWidth: true; Layout.leftMargin: 4
                text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
                font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
            }
        }
        Text {
            visible: page.message !== "" && page.adding === ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}

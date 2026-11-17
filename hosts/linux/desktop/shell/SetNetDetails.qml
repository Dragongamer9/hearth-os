// Hearth Settings › Network & internet › Connection details: every saved network (Wi-Fi, wired, mobile broadband, VPN),
// and for the one you pick: what it has now (address, gateway, DNS), DNS servers of your own, metered, MAC address
// (random or the real one), auto-connect, IPv4 by hand, IPv6 and the packet size (MTU). 1.14, the rest of KDE's
// connection editor: enterprise sign-in (802.1X: PEAP, TTLS, TLS, PWD) for Wi-Fi and wired, mobile broadband (APN, SIM
// PIN, roaming), a QR code to share a Wi-Fi network, Duplicate, and adding a wired, mobile or enterprise Wi-Fi
// connection. All NetworkManager's (hearth-network), changed only when you press, one setting at a time; passwords go
// to NetworkManager on nmcli's stdin and NetworkManager keeps them.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Connection details"
    property var st: null              // hearth-network status
    property string uuid: ""           // the connection picked
    property var d: null               // its details
    property string message: ""
    property bool messageBad: false
    property bool manual: false        // typing an address by hand
    property string adding: ""         // "wired" | "mobile" | "enterprise": the Add a network form that's open
    property var qr: null              // the picked Wi-Fi's QR code (wifi-qr), only after you ask
    property bool showPw: false
    property bool editEap: false       // the 802.1X form is open
    property string anchorWant: ""     // Settings opened at net-details:<part> (the pictures, links)
    readonly property var conns: st ? st.connections.filter(c => c.kind === "wifi" || c.kind === "ethernet" || c.kind === "vpn" || c.kind === "mobile") : []
    readonly property var picked: conns.find(c => c.uuid === uuid) || null

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() {
        job.run(["status"], r => {
            page.st = r;
            if (!page.uuid) {
                const want = page.anchorWant === "mobile" ? r.connections.find(c => c.kind === "mobile")
                           : page.anchorWant === "wired" ? r.connections.find(c => c.kind === "ethernet") : null;
                const a = want || r.connections.find(c => c.active && c.kind !== "vpn");
                if (a) page.pick(a.uuid);
            }
        }, e => page.say(e, true));
    }
    function pick(u) {
        uuid = u; manual = false; d = null; qr = null; showPw = false; editEap = false; say("");
        job.run(["details", u], r => { page.d = r; if (page.anchorWant === "qr" && r.kind === "wifi") { page.anchorWant = ""; page.showQr(); } }, e => page.say(e, true));
    }
    Component.onCompleted: {
        const a = SettingsNav.pageAnchor;
        if (["qr", "mobile", "wired", "add-wired", "add-mobile", "add-enterprise"].includes(a)) {
            SettingsNav.pageAnchor = "";
            if (a.startsWith("add-")) adding = a.slice(4); else anchorWant = a;
        }
        load();
    }
    Component.onDestruction: { qr = null; eapPw.text = ""; keyPw.text = ""; mobPw.text = ""; mobPin.text = ""; }
    SetJob { id: job; helper: "hearth-network" }
    function change(req, ok) {
        say("");
        job.run(["details-set", page.uuid], r => { page.d = r; page.say(ok); }, e => page.say(e, true), JSON.stringify(req), "set");
    }
    function showQr() {
        say("");
        job.run(["wifi-qr", page.uuid], r => page.qr = r, e => page.say(e, true), "", "qr");
    }
    function eapReq() {
        return { eap: eapKind.current, identity: eapId.text.trim(), anonymous: eapAnon.text.trim(), caCert: eapCa.text.trim(),
                 domain: eapDomain.text.trim(), phase2: eapPhase2.current, password: eapPw.text, askEachTime: eapAsk.checked,
                 clientCert: eapCert.text.trim(), privateKey: eapKey.text.trim(), keyPassword: keyPw.text };
    }
    function kindIcon(k) { return k === "wifi" ? "wifi" : k === "ethernet" ? "ethernet" : k === "vpn" ? "shield" : "globe"; }
    function kindWord(k) { return k === "wifi" ? "Wi-Fi" : k === "ethernet" ? "Wired" : k === "vpn" ? "VPN" : "Mobile broadband"; }

    SetCardGroup {
        title: "Saved networks"
        Repeater {
            model: page.conns
            delegate: SetRow {
                required property var modelData
                icon: page.kindIcon(modelData.kind)
                title: modelData.name
                sub: page.kindWord(modelData.kind) + (modelData.active ? " · Connected" : "")
                value: page.uuid === modelData.uuid ? "Showing" : ""
                onActivated: page.pick(modelData.uuid)
            }
        }
    }

    // ---- Add a network (1.14): a wired profile of your own, mobile broadband, an enterprise Wi-Fi network.
    SetCardGroup {
        title: "Add a network"
        SetRow {
            icon: "ethernet"; title: "Wired connection"; chevron: true
            sub: "A second profile for a cable: another address, DNS or 802.1X sign-in"
            value: page.adding === "wired" ? "Adding" : ""
            onActivated: page.adding = page.adding === "wired" ? "" : "wired"
        }
        ColumnLayout {
            visible: page.adding === "wired"
            Layout.fillWidth: true; Layout.leftMargin: 52; spacing: 8
            SetField { id: wiredName; placeholder: "A name, like Office cable" }
            SetDropdown {
                id: wiredDev
                icon: "ethernet"; title: "For"
                options: [{ value: "", label: "Any wired port" }].concat(page.st ? page.st.devices.filter(x => x.type === "ethernet").map(x => ({ value: x.device, label: x.device })) : [])
                current: ""
                onChosen: v => current = v
            }
            SetRow {
                icon: "plus"; chevron: false; title: job.what === "add" ? "Adding…" : "Add this wired connection"
                sub: "It connects only when you pick it"
                onActivated: job.run(["wired-add"], r => { page.adding = ""; page.say(r.name + " is added."); page.load(); page.pick(r.uuid); }, e => page.say(e, true),
                                     JSON.stringify({ name: wiredName.text.trim(), device: wiredDev.current }), "add")
            }
        }
        SetRow {
            icon: "globe"; title: "Mobile broadband"; chevron: true
            sub: page.st && !page.st.hasModem ? "No modem found right now: you can still set it up for when one is plugged in" : "A SIM in this PC or a USB modem: its APN and PIN"
            value: page.adding === "mobile" ? "Adding" : ""
            onActivated: page.adding = page.adding === "mobile" ? "" : "mobile"
        }
        ColumnLayout {
            visible: page.adding === "mobile"
            Layout.fillWidth: true; Layout.leftMargin: 52; spacing: 8
            SetField { id: addMobName; placeholder: "A name, like My phone plan" }
            SetField { id: addApn; placeholder: "APN, from your mobile provider (like internet)" }
            SetField { id: addMobUser; placeholder: "User name (most providers have none)" }
            SetField { id: addMobPw; secret: true; placeholder: "Password (most providers have none)" }
            SetField { id: addPin; secret: true; placeholder: "SIM PIN (if your SIM asks for one)" }
            SetSwitch { id: addRoam; icon: "globe"; title: "Use it when roaming"; sub: "Abroad or on another network: it can cost more"; checked: false; onToggled: v => checked = v }
            SetRow {
                icon: "plus"; chevron: false; title: job.what === "add" ? "Adding…" : "Add mobile broadband"
                onActivated: {
                    job.run(["mobile-add"], r => { page.adding = ""; page.say(r.name + " is added."); page.load(); page.pick(r.uuid); }, e => page.say(e, true),
                            JSON.stringify({ name: addMobName.text.trim(), apn: addApn.text.trim(), user: addMobUser.text.trim(), password: addMobPw.text, pin: addPin.text, roaming: addRoam.checked }), "add");
                    addMobPw.text = ""; addPin.text = "";
                }
            }
        }
        SetRow {
            icon: "wifi"; title: "Enterprise Wi-Fi (802.1X)"; chevron: true
            sub: "Work, school or eduroam: you sign in with your own account"
            value: page.adding === "enterprise" ? "Adding" : ""
            onActivated: { page.adding = page.adding === "enterprise" ? "" : "enterprise"; page.editEap = false; }
        }
        SetField { id: entSsid; visible: page.adding === "enterprise"; Layout.leftMargin: 52; placeholder: "The network's name (SSID), like eduroam" }
    }

    // The 802.1X form: for a new enterprise Wi-Fi network (Add a network), or the picked Wi-Fi / wired connection.
    SetCardGroup {
        id: eapCard
        readonly property bool forNew: page.adding === "enterprise"
        visible: forNew || (page.editEap && !!page.d)
        title: forNew ? "Sign-in for the new network" : "Enterprise sign-in (802.1X) for " + (page.d ? page.d.name : "")
        SetChoice {
            id: eapKind
            icon: "lock"; title: "How it signs in"; sub: "Ask your IT people if you're not sure: PEAP is the usual one (eduroam too)"
            options: [{ value: "peap", label: "PEAP" }, { value: "ttls", label: "TTLS" }, { value: "tls", label: "Certificate (TLS)" }, { value: "pwd", label: "PWD" }]
            current: page.d && page.d.enterprise && !eapCard.forNew ? page.d.enterprise.eap : "peap"
            onChosen: v => current = v
        }
        SetField { id: eapId; placeholder: page.d && page.d.enterprise && !eapCard.forNew ? "User name: " + page.d.enterprise.identity : "Your user name (identity), like you@school.edu" }
        SetField { id: eapAnon; visible: eapKind.current !== "tls"; placeholder: "Anonymous identity (optional), like anonymous@school.edu" }
        SetChoice {
            id: eapPhase2
            visible: eapKind.current === "peap" || eapKind.current === "ttls"
            icon: "key"; title: "Inner sign-in"
            options: [{ value: "mschapv2", label: "MSCHAPv2" }, { value: "pap", label: "PAP" }, { value: "gtc", label: "GTC" }]
            current: page.d && page.d.enterprise && page.d.enterprise.phase2 && !eapCard.forNew ? page.d.enterprise.phase2 : "mschapv2"
            onChosen: v => current = v
        }
        SetField { id: eapCa; placeholder: "CA certificate file (optional but safer), like ~/Downloads/ca.pem" }
        SetField { id: eapDomain; placeholder: "Server's domain (optional, checks it's really your network), like radius.school.edu" }
        SetField { id: eapCert; visible: eapKind.current === "tls"; placeholder: "Your certificate file, like ~/Documents/me.crt" }
        SetField { id: eapKey; visible: eapKind.current === "tls"; placeholder: "Its private key file, like ~/Documents/me.key" }
        SetField { id: keyPw; visible: eapKind.current === "tls"; secret: true; placeholder: "The private key's password (if it has one)" }
        SetField { id: eapPw; visible: eapKind.current !== "tls" && !eapAsk.checked; secret: true; placeholder: "Password (NetworkManager keeps it, not Hearth)" }
        SetSwitch {
            id: eapAsk
            visible: eapKind.current !== "tls"
            icon: "lock"; title: "Ask for the password each time"; sub: "Off: NetworkManager keeps it, so it connects by itself"
            checked: !!page.d && !!page.d.enterprise && page.d.enterprise.asks && !eapCard.forNew
            onToggled: v => checked = v
        }
        SetRow {
            icon: "check"; chevron: false
            title: job.what === "eap" ? "Saving…" : eapCard.forNew ? "Add this network" : "Save the sign-in"
            onActivated: {
                const req = page.eapReq();
                if (eapCard.forNew) {
                    req.ssid = entSsid.text.trim();
                    job.run(["wifi-enterprise-add"], r => { page.adding = ""; page.say(r.name + " is added. It connects when it's in reach."); page.load(); page.pick(r.uuid); }, e => page.say(e, true), JSON.stringify(req), "eap");
                } else {
                    job.run(["enterprise-set", page.uuid], r => { page.d = r; page.editEap = false; page.say("Sign-in saved."); }, e => page.say(e, true), JSON.stringify(req), "eap");
                }
                eapPw.text = ""; keyPw.text = "";
            }
        }
        SetRow {
            visible: !eapCard.forNew && !!page.d && page.d.kind === "ethernet" && !!page.d.enterprise
            icon: "close"; danger: true; chevron: false; title: "Turn 802.1X off for this cable"
            onActivated: job.run(["enterprise-set", page.uuid], r => { page.d = r; page.editEap = false; page.say("802.1X is off for " + r.name + "."); }, e => page.say(e, true), JSON.stringify({ off: true }), "eap")
        }
    }

    SetCardGroup {
        title: page.d ? page.d.name + " now" : "Details"
        visible: !!page.d
        SetRow {
            interactive: false; chevron: false; icon: "info"
            title: page.d && page.d.now.address ? page.d.now.address : "Not connected"
            sub: page.d ? ["Gateway " + (page.d.now.gateway || "–"), "DNS " + (page.d.now.dns.join(", ") || "–")].concat(page.d.now.address6 ? ["IPv6 " + page.d.now.address6] : []).join(" · ") : ""
        }
    }

    // ---- Share this Wi-Fi network (1.14): a QR code a phone's camera joins from. Made only when you press.
    SetCardGroup {
        title: "Share " + (page.d ? page.d.name : "")
        visible: !!page.d && page.d.kind === "wifi" && page.d.security !== "enterprise"
        SetRow {
            visible: !page.qr
            icon: "qr"; chevron: false
            title: job.what === "qr" ? "Making the code…" : "Show a QR code"
            sub: page.st && !page.st.qr ? "Needs qrencode, which isn't on this system" : "Point a phone's camera at it to join this network. It shows the password"
            onActivated: page.showQr()
        }
        RowLayout {
            visible: !!page.qr
            Layout.fillWidth: true
            spacing: 22
            Rectangle {
                id: qrBox
                readonly property int n: page.qr ? page.qr.size : 21
                implicitWidth: 260; implicitHeight: 260; radius: 16
                color: "#ffffff"
                Canvas {
                    id: qrCanvas
                    anchors.fill: parent; anchors.margins: 18
                    property var rows: page.qr ? page.qr.rows : []
                    onRowsChanged: requestPaint()
                    onPaint: {
                        const g = getContext("2d");
                        g.reset();
                        g.fillStyle = "#ffffff"; g.fillRect(0, 0, width, height);
                        if (!rows.length) return;
                        const cell = Math.floor(Math.min(width, height) / rows.length);
                        const off = Math.floor((width - cell * rows.length) / 2);
                        g.fillStyle = "#111111";
                        for (let y = 0; y < rows.length; y++)
                            for (let x = 0; x < rows[y].length; x++)
                                if (rows[y][x] === "1") g.fillRect(off + x * cell, off + y * cell, cell, cell);
                    }
                }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 8
                Text { text: page.qr ? page.qr.ssid : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold }
                Text { text: page.qr ? (page.qr.security === "nopass" ? "Open network: no password" : page.qr.security + " · scan with the phone's camera") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                Text {
                    visible: !!page.qr && page.qr.password !== ""
                    text: page.showPw && page.qr ? "Password: " + page.qr.password : "Password: " + "•".repeat(10)
                    color: Theme.text; font.family: Theme.font; font.pixelSize: 15
                }
                RowLayout {
                    spacing: 10
                    CaptureBtn { visible: !!page.qr && page.qr.password !== ""; icon: "eye"; text: page.showPw ? "Hide the password" : "Show the password"; onActivated: page.showPw = !page.showPw }
                    CaptureBtn { icon: "close"; text: "Hide the code"; onActivated: { page.qr = null; page.showPw = false; } }
                }
            }
        }
    }

    // ---- Mobile broadband (1.14): APN, user name, password, SIM PIN, roaming.
    SetCardGroup {
        title: "Mobile broadband"
        visible: !!page.d && page.d.kind === "mobile"
        SetRow {
            interactive: false; chevron: false; icon: "globe"
            title: page.d && page.d.mobile ? "APN: " + (page.d.mobile.apn || "automatic") : ""
            sub: page.d && page.d.mobile ? (page.d.mobile.user ? "User " + page.d.mobile.user + " · " : "") + (page.d.mobile.roaming ? "Used when roaming" : "Home network only") : ""
        }
        SetField { id: mobApn; placeholder: "New APN, like internet" }
        SetField { id: mobUser; placeholder: "User name (most providers have none)" }
        SetField { id: mobPw; secret: true; placeholder: "Password (NetworkManager keeps it)" }
        SetField { id: mobPin; secret: true; placeholder: "SIM PIN (4 to 8 numbers)" }
        SetRow {
            icon: "check"; chevron: false; title: job.what === "set" ? "Saving…" : "Save"
            sub: "Only what you typed changes"
            onActivated: {
                const req = {};
                if (mobApn.text.trim()) req.apn = mobApn.text.trim();
                if (mobUser.text.trim()) req.user = mobUser.text.trim();
                if (mobPw.text) req.password = mobPw.text;
                if (mobPin.text) req.pin = mobPin.text;
                job.run(["mobile-set", page.uuid], r => { page.d = r; page.say("Saved."); }, e => page.say(e, true), JSON.stringify(req), "set");
                mobPw.text = ""; mobPin.text = ""; mobApn.text = ""; mobUser.text = "";
            }
        }
        SetSwitch {
            icon: "globe"; title: "Use it when roaming"; sub: "Abroad or on another network: it can cost more"
            checked: !!page.d && !!page.d.mobile && page.d.mobile.roaming
            onToggled: v => job.run(["mobile-set", page.uuid], r => { page.d = r; page.say(v ? "It's used when roaming too." : "Home network only."); }, e => page.say(e, true), JSON.stringify({ roaming: v }), "set")
        }
    }

    SetCardGroup {
        title: "Settings for " + (page.d ? page.d.name : "")
        visible: !!page.d
        SetSwitch {
            icon: "link"; title: "Connect automatically"; sub: "When it's in reach"
            checked: !!page.d && page.d.autoconnect
            onToggled: v => page.change({ autoconnect: v }, v ? "Connects by itself when in reach." : "Connects only when you pick it.")
        }
        SetChoice {
            visible: !!page.d && page.d.kind !== "vpn"
            icon: "cloud"; title: "Metered connection"; sub: "Updates and big downloads wait on a metered network (phone hotspots)"
            options: [{ value: "unknown", label: "Let Hearth guess" }, { value: "yes", label: "Metered" }, { value: "no", label: "Not metered" }]
            current: page.d && (page.d.metered === "yes" || page.d.metered === "no") ? page.d.metered : "unknown"
            onChosen: v => page.change({ metered: v }, "Saved.")
        }
        SetDropdown {
            visible: !!page.d && page.d.mac !== ""
            icon: "shield"; title: "MAC address"; sub: "What this PC tells the network it is. Random is more private"
            options: [{ value: "default", label: "Hearth's default" }, { value: "permanent", label: "The real one" },
                      { value: "stable", label: "Random, the same for this network" }, { value: "random", label: "Random each time" },
                      { value: "preserve", label: "Keep what the device has" }].concat(page.d && page.d.mac === "fixed" ? [{ value: "fixed", label: "Typed in: " + page.d.macFixed }] : [])
            current: page.d ? page.d.mac : "default"
            onChosen: v => { if (v !== "fixed") page.change({ mac: v }, "Saved. It's used the next time you connect."); }
        }
        SetSwitch {
            icon: "globe"; title: "Automatic DNS"; sub: "The DNS servers the network hands out. Off: only yours below"
            checked: !!page.d && page.d.autoDns
            onToggled: v => page.change({ autoDns: v }, "Saved.")
        }
        SetField { id: dns; placeholder: page.d && page.d.dns.length ? "Your DNS: " + page.d.dns.join(", ") : "Your own DNS servers, like 1.1.1.1, 9.9.9.9"; onSubmitted: dnsBtn.activated() }
        SetRow {
            id: dnsBtn
            icon: "check"; chevron: false; title: "Use these DNS servers"
            sub: page.d && page.d.dns.length ? "Empty: back to none of your own" : "Split them with commas"
            onActivated: { page.change({ dns: dns.text.split(/[\s,]+/).filter(x => x) }, "DNS saved."); dns.text = ""; }
        }
        SetChoice {
            visible: !!page.d && page.d.kind !== "vpn"
            icon: "gear"; title: "IP address (IPv4)"; sub: page.d && page.d.ipv4.method === "manual" ? page.d.ipv4.address + (page.d.ipv4.gateway ? " via " + page.d.ipv4.gateway : "") : "Handed out by the router"
            options: [{ value: "auto", label: "Automatic" }, { value: "manual", label: "By hand" }]
            current: page.d && page.d.ipv4.method === "manual" ? "manual" : "auto"
            onChosen: v => { if (v === "auto") page.change({ ipv4: { method: "auto" } }, "Automatic address."); else page.manual = true; }
        }
        SetField { id: addr4; visible: page.manual; placeholder: "Address, like 192.168.1.20/24" }
        SetField { id: gw4; visible: page.manual; placeholder: "Gateway (your router), like 192.168.1.1" }
        SetRow {
            visible: page.manual
            icon: "check"; chevron: false; title: "Use this address"
            onActivated: { page.change({ ipv4: { method: "manual", address: addr4.text.trim(), gateway: gw4.text.trim() } }, "Address saved."); page.manual = false; }
        }
        SetChoice {
            visible: !!page.d && page.d.kind !== "vpn"
            icon: "globe"; title: "IPv6"
            options: [{ value: "auto", label: "On" }, { value: "ignore", label: "Leave it" }, { value: "disabled", label: "Off" }]
            current: page.d ? (page.d.ipv6 === "disabled" ? "disabled" : page.d.ipv6 === "ignore" ? "ignore" : "auto") : "auto"
            onChosen: v => page.change({ ipv6: v }, "Saved.")
        }
        // 1.14: the packet size (MTU), for Wi-Fi and wired.
        SetField { id: mtu; visible: !!page.d && (page.d.kind === "wifi" || page.d.kind === "ethernet"); placeholder: page.d && page.d.mtu ? "Packet size (MTU): " + page.d.mtu + ". Type a new one, or 0 for automatic" : "Packet size (MTU): automatic. Type one, like 1492, only if your provider says so"; onSubmitted: mtuBtn.activated() }
        SetRow {
            id: mtuBtn
            visible: mtu.visible && mtu.text.trim() !== ""
            icon: "check"; chevron: false; title: "Use this packet size"
            onActivated: { page.change({ mtu: parseInt(mtu.text.trim()) || 0 }, "Saved. It's used the next time it connects."); mtu.text = ""; }
        }
        SetRow {
            visible: !!page.d && (page.d.kind === "ethernet" || (page.d.kind === "wifi" && page.d.security === "enterprise"))
            icon: "lock"; chevron: true
            title: page.d && page.d.enterprise ? "Enterprise sign-in (802.1X): " + page.d.enterprise.eap.toUpperCase() : "Enterprise sign-in (802.1X)"
            sub: page.d && page.d.enterprise ? "Signs in as " + page.d.enterprise.identity + ". Change it" : "Off. For networks at work or school that ask for your account"
            onActivated: { page.editEap = !page.editEap; page.adding = ""; }
        }
        SetRow {
            icon: "copy"; chevron: false
            title: job.what === "dup" ? "Copying…" : "Duplicate this connection"
            sub: "A copy to change (it connects only when you pick it)"
            onActivated: job.run(["duplicate", page.uuid], r => { page.say(r.name + " is the copy."); page.load(); page.pick(r.uuid); }, e => page.say(e, true), "", "dup")
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
    Text {
        visible: page.message !== "" && !page.d
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

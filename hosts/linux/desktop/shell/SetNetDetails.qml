// Hearth Settings › Network & internet › Connection details: every saved network (Wi-Fi, wired, VPN), and for the one
// you pick: what it has now (address, gateway, DNS), DNS servers of your own, metered, MAC address (random or the real
// one), auto-connect, IPv4 by hand and IPv6. All NetworkManager's (hearth-network details / details-set), changed only
// when you press, one setting at a time.
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
    readonly property var conns: st ? st.connections.filter(c => c.kind === "wifi" || c.kind === "ethernet" || c.kind === "vpn" || c.kind === "mobile") : []
    readonly property var picked: conns.find(c => c.uuid === uuid) || null

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["status"], r => { page.st = r; if (!page.uuid) { const a = r.connections.find(c => c.active && c.kind !== "vpn"); if (a) page.pick(a.uuid); } }, e => page.say(e, true)); }
    function pick(u) { uuid = u; manual = false; d = null; say(""); job.run(["details", u], r => page.d = r, e => page.say(e, true)); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-network" }
    function change(req, ok) {
        say("");
        job.run(["details-set", page.uuid], r => { page.d = r; page.say(ok); }, e => page.say(e, true), JSON.stringify(req), "set");
    }
    function kindIcon(k) { return k === "wifi" ? "wifi" : k === "ethernet" ? "ethernet" : k === "vpn" ? "shield" : "globe"; }
    function kindWord(k) { return k === "wifi" ? "Wi-Fi" : k === "ethernet" ? "Wired" : k === "vpn" ? "VPN" : "Mobile"; }

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
    SetCardGroup {
        title: page.d ? page.d.name + " now" : "Details"
        visible: !!page.d
        SetRow {
            interactive: false; chevron: false; icon: "info"
            title: page.d && page.d.now.address ? page.d.now.address : "Not connected"
            sub: page.d ? ["Gateway " + (page.d.now.gateway || "–"), "DNS " + (page.d.now.dns.join(", ") || "–")].concat(page.d.now.address6 ? ["IPv6 " + page.d.now.address6] : []).join(" · ") : ""
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
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}

// A category's page in Hearth Settings (System, Network & internet, Bluetooth & devices, Apps, Accounts, Privacy &
// security): its pages as cards, with a live status where one is cheap to read (the Wi-Fi network, Bluetooth, Tailscale),
// then the cards that open a setting's one real home in another category. Only reads: opening it writes nothing.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "settings-pages.js" as Reg

SetPage {
    id: page
    property string category: "system"
    readonly property var cat: Reg.category(category) || ({})
    title: cat.label || ""
    readonly property var subs: Reg.pagesIn(category, SettingsNav.settingsApp ? SettingsNav.settingsApp.live : false)
    readonly property var links: Reg.LINKS[category] || []
    readonly property var leads: ({
        system: "This PC's screen, sound, power, storage and the version it runs.",
        net: "Wi-Fi, and Tailscale: a private network for your own devices, at home and away.",
        bt: "Wireless devices, the keyboard (and the on-screen one), the mouse and printers.",
        apps: "Apps on this PC, which app opens what, the web browser and Claude.",
        accounts: "You and the other people on this PC, and how you sign in.",
        privacy: "What Hearth OS shares, what it keeps, and who can reach this PC."
    })

    // ---- live status, read once when the page opens (and Wi-Fi / Bluetooth every few seconds while it's shown)
    property string ssid: ""
    property bool wifiKnown: false
    property string btText: ""
    property string tsState: ""
    function has(s) { return subs.some(p => p.status === s); }
    Component.onCompleted: { if (has("wifi")) wifi.running = true; if (has("bt")) bt.running = true; if (has("tailscale")) ts.running = true; }
    Timer { interval: 8000; repeat: true; running: page.visible && (page.has("wifi") || page.has("bt")); onTriggered: { if (page.has("wifi") && !wifi.running) wifi.running = true; if (page.has("bt") && !bt.running) bt.running = true; } }
    Process {
        id: wifi
        command: ["sh", "-c", "nmcli -t -f ACTIVE,SSID dev wifi 2>/dev/null | sed -n 's/^yes://p' | head -1"]
        stdout: StdioCollector { onStreamFinished: { page.ssid = text.trim(); page.wifiKnown = true; } }
    }
    Process {
        id: bt
        command: ["sh", "-c", "bluetoothctl --timeout 3 show 2>/dev/null | grep -q 'Powered: yes' || { echo off; exit 0; }; echo on $(bluetoothctl --timeout 3 devices Connected 2>/dev/null | grep -c '^Device')"]
        stdout: StdioCollector { onStreamFinished: { const t = text.trim().split(" "); page.btText = t[0] === "on" ? "On" + (Number(t[1]) > 0 ? " · " + t[1] + " connected" : "") : t[0] === "off" ? "Off" : ""; } }
    }
    Process {
        id: ts
        command: ["sh", "-c", "command -v tailscale >/dev/null || { echo missing; exit 0; }; tailscale status --json 2>/dev/null | sed -n 's/.*\"BackendState\": *\"\\([A-Za-z]*\\)\".*/\\1/p' | head -1"]
        stdout: StdioCollector { onStreamFinished: page.tsState = text.trim() }
    }
    function statusOf(p) {
        switch (p.status) {
        case "wifi": return page.wifiKnown ? { value: page.ssid || "Not connected" } : {};
        case "bt": return { value: page.btText };
        case "tailscale": return page.tsState === "Running" ? { chip: "Connected" } : page.tsState === "missing" ? { value: "Not installed" } : page.tsState ? { value: page.tsState === "NeedsLogin" ? "Sign in" : "Off" } : {};
        case "dnd": return DeskState.dnd ? { chip: "Do not disturb", kind: "warn" } : {};
        case "countMe": return { chip: Settings.countMe ? "On" : "Off", kind: Settings.countMe ? "ok" : "off" };
        case "bar": return { value: Settings.barStyle === "" ? "Look's own" : Settings.barStyle === "bar" ? "Full-width bar" : "Floating islands" };
        default: return {};
        }
    }

    Text {
        visible: (page.leads[page.category] || "") !== ""
        Layout.fillWidth: true
        Layout.bottomMargin: 12
        text: page.leads[page.category] || ""
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.subs
        delegate: SetLinkCard {
            required property var modelData
            readonly property var st: page.statusOf(modelData)
            target: modelData.id
            value: st.value || ""
            chip: st.chip || ""
            chipKind: st.kind || "ok"
        }
    }
    SetHeader { visible: page.links.length > 0; text: page.category === "privacy" ? "Also about privacy" : "Also here" }
    Repeater {
        model: page.links
        delegate: SetLinkCard {
            required property var modelData
            target: modelData.page
            title: modelData.label || (Reg.page(modelData.page) || {}).label || modelData.page
            sub: (modelData.desc || "") + " · " + Reg.pathOf(modelData.page)
        }
    }
}

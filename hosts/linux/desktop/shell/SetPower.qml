// Hearth Settings › Power: battery, performance profile, sleep and restart.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.UPower

SetPage {
    id: page
    title: "Power"
    readonly property var dev: UPower.displayDevice
    readonly property bool battery: !!(dev && dev.isLaptopBattery)
    property string profile: ""
    property var profiles: []

    Process {
        running: true
        command: ["sh", "-c", "powerprofilesctl list 2>/dev/null | sed -n 's/^[* ]*\\([a-z-]*\\):$/\\1/p'; echo '--'; powerprofilesctl get 2>/dev/null"]
        stdout: StdioCollector {
            onStreamFinished: {
                const parts = text.split("--\n");
                page.profiles = (parts[0] || "").split("\n").filter(Boolean);
                page.profile = (parts[1] || "").trim();
            }
        }
    }
    function hm(s) { if (!s || s <= 0) return ""; const h = Math.floor(s / 3600), m = Math.round((s % 3600) / 60); return h ? `${h} h ${m} min` : `${m} min`; }
    readonly property string battText: {
        if (!battery) return "";
        const pct = Math.round(dev.percentage * 100) + "%";
        if (dev.state === UPowerDeviceState.Charging) return pct + " · charging" + (dev.timeToFull > 0 ? ", full in " + hm(dev.timeToFull) : "");
        if (dev.state === UPowerDeviceState.FullyCharged) return pct + " · full";
        return pct + (dev.timeToEmpty > 0 ? " · about " + hm(dev.timeToEmpty) + " left" : "");
    }
    readonly property var labels: ({ "power-saver": "Battery saver", "balanced": "Balanced", "performance": "Performance" })

    SetRow {
        visible: page.battery; interactive: false
        icon: page.dev && page.dev.state === UPowerDeviceState.Charging ? "batt-charging" : "batt"
        title: "Battery"; value: page.battText; chevron: false
    }
    SetChoice {
        visible: page.profiles.length > 1
        title: "Power mode"; sub: "Battery saver lasts longer; Performance is faster and warmer"
        options: page.profiles.map(p => ({ value: p, label: page.labels[p] || p }))
        current: page.profile
        onChosen: v => { page.profile = v; Quickshell.execDetached(["powerprofilesctl", "set", v]); }
    }
    SetHeader { text: "Turn off or switch" }
    SetRow { icon: "pad"; title: "Game Mode"; sub: "Back to Hearth on the full screen"; onActivated: Quickshell.execDetached(["nimbus-mode", "game"]) }
    SetRow { icon: "sleep"; title: "Sleep"; onActivated: Quickshell.execDetached(["systemctl", "suspend"]) }
    SetRow { icon: "restart"; title: "Restart"; onActivated: Quickshell.execDetached(["systemctl", "reboot"]) }
    SetRow { icon: "power"; title: "Shut down"; danger: true; onActivated: Quickshell.execDetached(["systemctl", "poweroff"]) }
}

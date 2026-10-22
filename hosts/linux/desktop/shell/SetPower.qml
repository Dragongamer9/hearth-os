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
    property var tdp: null             // {supported, watts, min, max, via}
    property string powerError: ""
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-power").toString().replace(/^file:\/\//, ""))

    // Power mode and power limit through hearth-power (the same as Hearth's Game Mode settings).
    // One at a time; one asked for while another runs goes next (it was dropped: a power limit picked while the
    // page was still reading the status never got set, and a failed change never showed the real mode again).
    function power(args) {
        if (pw.running) { pw.next = args; return; }
        pw.command = ["python3", helper].concat(args); pw.running = true;
    }
    Component.onCompleted: power(["status"])
    Process {
        id: pw
        property var next: null
        onExited: if (next) { const a = next; next = null; page.power(a); }
        stdout: StdioCollector {
            onStreamFinished: {
                const line = text.trim().split("\n").pop() || "";
                if (line.startsWith("error: ")) { page.powerError = line.slice(7); page.power(["status"]); return; }
                try {
                    const r = JSON.parse(line);
                    if (r.mode) { page.profiles = r.mode.supported ? r.mode.available : []; page.profile = r.mode.active; }
                    if (r.tdp) page.tdp = r.tdp;
                    if (!r.mode || !r.tdp) page.power(["status"]);
                } catch (e) {}
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
    readonly property var powerChoices: [{ value: "sleep", label: "Sleep" }, { value: "menu", label: "Power menu" }, { value: "shutdown", label: "Shut down" }, { value: "nothing", label: "Nothing" }]
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
        onChosen: v => { page.profile = v; page.powerError = ""; page.power(["mode", v]); }
    }
    SetSlider {
        visible: !!(page.tdp && page.tdp.supported)
        icon: "power"; title: "Power limit (TDP)"
        readonly property int lo: page.tdp && page.tdp.min !== undefined ? page.tdp.min : 5
        readonly property int hi: page.tdp && page.tdp.max !== undefined ? page.tdp.max : 30
        value: page.tdp && page.tdp.supported ? (page.tdp.watts - lo) / Math.max(1, hi - lo) : 0
        valueText: (page.tdp && page.tdp.watts !== undefined ? page.tdp.watts : 0) + " W"
        step: 1 / Math.max(1, hi - lo)
        onMoved: v => { const w = Math.round(lo + v * (hi - lo)); if (page.tdp) page.tdp = Object.assign({}, page.tdp, { watts: w }); tdpApply.restart(); }
        Timer { id: tdpApply; interval: 500; onTriggered: page.power(["tdp", String(page.tdp.watts)]) }
    }
    Text {
        visible: page.powerError !== ""
        Layout.fillWidth: true
        text: page.powerError; color: Theme.danger; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    SetHeader { text: "Power button" }
    SetChoice {
        title: "Press"; sub: "A quick press"
        options: page.powerChoices
        current: Buttons.press
        onChosen: v => Buttons.set("press", v)
    }
    SetChoice {
        title: "Hold"; sub: "Held down for a second"
        options: page.powerChoices
        current: Buttons.hold
        onChosen: v => Buttons.set("hold", v)
    }
    Text {
        Layout.fillWidth: true
        text: Buttons.watching.length ? "Holding it for several seconds always turns the device off, whatever is set here."
            : "The power button can't be read right now" + (Buttons.problems.length ? ": " + Buttons.problems.join("; ") : ".")
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    SetHeader { text: "Turn off or switch" }
    SetRow { icon: "pad"; title: "Game Mode"; sub: "Back to Hearth on the full screen"; onActivated: Quickshell.execDetached(["nimbus-mode", "game"]) }
    SetRow { icon: "sleep"; title: "Sleep"; onActivated: Quickshell.execDetached(["systemctl", "suspend"]) }
    SetRow { icon: "restart"; title: "Restart"; onActivated: Quickshell.execDetached(["systemctl", "reboot"]) }
    SetRow { icon: "power"; title: "Shut down"; danger: true; onActivated: Quickshell.execDetached(["systemctl", "poweroff"]) }
}

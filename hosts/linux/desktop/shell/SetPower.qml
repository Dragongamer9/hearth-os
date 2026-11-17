// Hearth Settings › Power: battery, performance profile, sleep and restart. 1.14: the battery's charge limit and health
// (hearth-hwset, the same setting as Task Manager › Battery: /etc/hearth/hardware.json, put back at every start), shown
// only where the battery has a charge limit (charge_control_end_threshold).
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
    property bool canScreenOff: false    // hearth-screen-power can (wlopm under labwc, KWin's DPMS under KWin; IdlePower.qml)
    property string profile: ""
    property var profiles: []
    property var tdp: null             // {supported, watts, min, max, via}
    property string powerError: ""
    property var hw: null              // hearth-hwset status (the battery's charge limit)
    readonly property var bat: hw && hw.batteries && hw.batteries.length ? hw.batteries[0] : null
    SetJob { id: hwStatus; helper: "/usr/libexec/nimbus/hearth-hwset" }
    SetJob { id: hwJob; helper: "/usr/libexec/nimbus/hearth-hwset"; admin: true }
    Component.onCompleted: { power(["status"]); hwStatus.run(["status"], r => page.hw = r, () => page.hw = null); moreJob.run(["status"], r => page.more = r, () => page.more = null); reveal(); }
    // `settings page power:off` opens it at "Turn off or switch" (the Desktop Mode test's picture); only scrolls.
    function reveal() { if (SettingsNav.pageAnchor === "off") { SettingsNav.pageAnchor = ""; revealTimer.start(); } }
    Timer { id: revealTimer; interval: 1500; onTriggered: page.contentY = Math.max(0, Math.min(offCard.mapToItem(page.contentItem, 0, 0).y - 16, page.contentHeight - page.height)) }
    // 1.14 (hearth-powerset): the lid, apps keeping it awake, whether it can hibernate, the battery's health.
    property var more: null
    property bool askHibernate: false
    property string askWhat: "hibernate"     // what the first-time question is for: "hibernate" | "hybrid-sleep"
    readonly property var hybrid: more ? more.hybrid : null
    SetJob { id: moreJob; helper: "hearth-powerset" }
    readonly property var hib: more ? more.hibernate : null
    readonly property int awake: more ? more.inhibitors.filter(i => i.mode === "block").length : 0
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("hearth-power").toString().replace(/^file:\/\//, ""))

    // Power mode and power limit through hearth-power (the same as Hearth's Game Mode settings).
    // One at a time; one asked for while another runs goes next (it was dropped: a power limit picked while the
    // page was still reading the status never got set, and a failed change never showed the real mode again).
    function power(args) {
        if (pw.running) { pw.next = args; return; }
        pw.command = ["python3", helper].concat(args); pw.running = true;
    }
    Process {
        id: pw
        property var next: null
        onExited: if (next) { const a = next; next = null; Qt.callLater(() => page.power(a)); }
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

    SetCardGroup {
        title: "Battery & performance"
        visible: page.battery || (!!page.bat && page.bat.canLimit) || page.profiles.length > 1 || !!(page.tdp && page.tdp.supported) || page.powerError !== ""
        SetRow {
            visible: page.battery; interactive: false
            icon: page.dev && page.dev.state === UPowerDeviceState.Charging ? "batt-charging" : "batt"
            title: "Battery"; value: page.battText; chevron: false
        }
        SetChoice {
            visible: !!page.bat && page.bat.canLimit
            icon: "batt"; title: "Stop charging at"
            sub: (page.bat && page.bat.health ? "Battery health " + page.bat.health + "%" + (page.bat.cycles ? " after " + page.bat.cycles + " charges" : "") + ". " : "") + "A limit keeps the battery healthy longer if it's plugged in a lot"
            options: [{ value: 60, label: "60%" }, { value: 80, label: "80%" }, { value: 100, label: "100% (no limit)" }]
            current: page.bat && page.bat.limit ? (page.bat.limit >= 95 ? 100 : page.bat.limit <= 70 ? 60 : 80) : 100
            onChosen: v => hwJob.run(["set", "battery.limit", String(v)], () => hwStatus.run(["status"], r => page.hw = r), e => page.powerError = e)
        }
        SetChoice {
            visible: page.profiles.length > 1
            icon: "bolt"; title: "Power mode"; sub: "Battery saver lasts longer; Performance is faster and warmer"
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
    }
    Process {
        running: true
        command: ["sh", Quickshell.shellDir + "/hearth-screen-power", "check"]
        onExited: code => page.canScreenOff = code === 0
    }
    SetCardGroup {
        title: "More power settings"
        SetRow { icon: "batt"; title: "Plugged in, battery & lid"; sub: "Different times and power mode on battery, and what closing the lid does"; chevron: true; onActivated: SettingsNav.openPage("power-profiles") }
        SetRow {
            icon: "sleep"; title: "Apps keeping it awake"; chevron: true
            sub: page.awake ? page.awake + (page.awake === 1 ? " app is" : " apps are") + " keeping the screen on or the PC awake" : "Nothing is keeping it awake right now"
            onActivated: SettingsNav.openPage("power-awake")
        }
        SetRow { icon: "bolt"; title: "Battery health & history"; sub: "Charge cycles, how much it holds, graphs of charge and power, your devices' batteries"; chevron: true; onActivated: SettingsNav.openPage("battery") }
    }
    SetCardGroup {
        title: "When you're away"
        SetChoice {
            icon: "display"; title: "Turn the screen off after"
            sub: page.canScreenOff ? "Videos and games that are playing keep it on" : (Quickshell.env("HEARTH_WM") === "kwin" ? "Needs kscreen-doctor (libkscreen-bin), which isn't installed" : "Needs wlopm, which isn't installed")
            enabled: page.canScreenOff
            opacity: page.canScreenOff ? 1 : 0.5
            options: [{ value: 0, label: "Never" }, { value: 2, label: "2 min" }, { value: 5, label: "5 min" }, { value: 10, label: "10 min" }, { value: 15, label: "15 min" }, { value: 30, label: "30 min" }]
            current: Settings.screenOffMinutes
            onChosen: v => Settings.screenOffMinutes = v
        }
        SetChoice {
            icon: "sleep"; title: "Go to sleep after"
            sub: "Downloads stop while it sleeps. Videos and games that are playing keep it awake"
            options: [{ value: 0, label: "Never" }, { value: 10, label: "10 min" }, { value: 15, label: "15 min" }, { value: 30, label: "30 min" }, { value: 60, label: "1 hour" }, { value: 120, label: "2 hours" }]
            current: Settings.sleepMinutes
            onChosen: v => Settings.sleepMinutes = v
        }
    }
    SetCardGroup {
        title: "Power button"
        SetChoice {
            icon: "power"; title: "Press"; sub: "A quick press"
            options: page.powerChoices
            current: Buttons.press
            onChosen: v => Buttons.set("press", v)
        }
        SetChoice {
            icon: "power"; title: "Hold"; sub: "Held down for a second"
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
    }
    SetCardGroup {
        id: offCard
        title: "Turn off or switch"
        SetRow { icon: "pad"; title: "Game Mode"; sub: "The Game Mode you picked in Gaming (Steam's or Hearth's) on the full screen"; onActivated: Quickshell.execDetached(["nimbus-mode", "game"]) }
        SetRow { icon: "sleep"; title: "Sleep"; onActivated: Buttons.sleep("Settings › Power › Sleep") }
        // Hibernate (1.14): only where this PC can (hearth-powerset: the kernel offers it, Secure Boot allows it, a swap
        // file on the drive big enough for what's in memory, logind agrees). Booting is never changed for it. The first
        // time it hasn't come back on this PC yet, so it asks first.
        SetRow {
            visible: !!page.hib && page.hib.can
            icon: "moon"; title: "Hibernate"; chevron: false
            sub: page.hib && page.hib.proven ? "Saves everything to the drive and turns off; turn on to carry on where you were" : "Not tried on this PC yet: save your work first"
            onActivated: { if (page.hib.proven) Quickshell.execDetached(["systemctl", "hibernate"]); else { page.askWhat = "hibernate"; page.askHibernate = true; } }
        }
        // Hybrid sleep (1.14, KDE's): sleeps, with everything also saved to the drive, so a flat battery or a power cut
        // doesn't lose your work. Only where Hibernate is offered and logind says yes (hearth-powerset hybrid); the
        // first time on this PC it asks, like Hibernate.
        SetRow {
            visible: !!page.hib && page.hib.can && !!page.hybrid && page.hybrid.can
            icon: "sleep"; title: "Hybrid sleep"; chevron: false
            sub: page.hib && page.hib.proven ? "Sleeps, and saves everything to the drive too in case the power goes" : "Not tried on this PC yet: save your work first"
            onActivated: { if (page.hib.proven) Quickshell.execDetached(["systemctl", "hybrid-sleep"]); else { page.askWhat = "hybrid-sleep"; page.askHibernate = true; } }
        }
        SetBanner {
            visible: page.askHibernate
            Layout.fillWidth: true
            kind: "warn"; title: page.askWhat === "hybrid-sleep" ? "Hybrid sleep for the first time?" : "Hibernate for the first time?"
            sub: "Hearth hasn't seen this PC come back from hibernating yet. Save your work: if it doesn't come back, it starts fresh and anything not saved is lost. Once it has worked, Hearth remembers."
        }
        RowLayout {
            visible: page.askHibernate
            Layout.leftMargin: 4; spacing: 8
            SetButton { text: page.askWhat === "hybrid-sleep" ? "Hybrid sleep now" : "Hibernate now"; primary: true; onActivated: { page.askHibernate = false; Quickshell.execDetached(["systemctl", page.askWhat === "hybrid-sleep" ? "hybrid-sleep" : "hibernate"]); } }
            SetButton { text: "Cancel"; ghost: true; onActivated: page.askHibernate = false }
        }
        SetRow {
            visible: !!page.hib && !page.hib.can
            interactive: false; chevron: false; icon: "moon"; title: "Hibernate isn't available"; sub: page.hib ? page.hib.why : ""
        }
        SetRow {
            visible: !!page.hib && page.hib.can && !!page.hybrid && !page.hybrid.can
            interactive: false; chevron: false; icon: "sleep"; title: "Hybrid sleep isn't available"; sub: page.hybrid ? page.hybrid.why : ""
        }
        SetRow { icon: "restart"; title: "Restart"; onActivated: Quickshell.execDetached(["systemctl", "reboot"]) }
        SetRow { icon: "power"; title: "Shut down"; danger: true; onActivated: Quickshell.execDetached(["systemctl", "poweroff"]) }
    }
}

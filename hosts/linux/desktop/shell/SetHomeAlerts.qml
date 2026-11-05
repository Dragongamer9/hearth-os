// Hearth Settings › Home server › Alerts & power (docs/HOME-SERVER.md). Alerts: a drive failing its health check
// (SMART) or running hot, a drive filling up, a backup failing or late, a mirror missing a drive; shown as Mantel
// notifications and, if you pick devices, sent to them through Taildrop (nothing leaves your own devices). Power: sleep
// when nobody needs the PC (never while Game Mode runs, a share is open or a backup runs), stay awake while your devices
// use it, Wake-on-LAN, a nightly schedule, spinning down idle data drives (never the system drive), and a UPS (shown
// once one is plugged in, or with Show hidden items). Every one is off until you turn it on.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Alerts & power"
    property var st: null
    property bool peersOpen: false
    property var ups: null
    property string msg: ""
    property bool bad: false
    readonly property var cfg: st ? st.settings : null
    readonly property var al: cfg ? cfg.alerts : null
    readonly property var pw: cfg ? cfg.power : null
    readonly property var active: st && st.alerts ? (st.alerts.active || []) : []
    readonly property bool upsShown: !!st && !!st.seen && !!st.seen.ups && st.seen.ups.shown
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.st = { error: e }); }
    function set(k, v, ok) { act.run(["set", k, v], () => { if (ok) page.say(ok); page.refresh(); }, e => { page.say(e, true); page.refresh(); }); }
    function mins(m) { return m >= 60 ? (m / 60) + (m === 60 ? " hour" : " hours") : m + " minutes"; }
    Component.onCompleted: { refresh(); }
    onUpsShownChanged: if (upsShown && page.pw && page.pw.ups) uj.run(["ups-status"], r => page.ups = r.ups, () => page.ups = null)
    Timer { interval: 8000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }
    SetJob { id: uj; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: "Hear about problems early, and let this PC sleep when nobody needs it. Everything here is off until you turn it on."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ---------------------------------------------------------------- alerts
    SetHeader { text: "Tell me when" }
    Repeater {
        model: page.active
        delegate: SetRow {
            required property var modelData
            icon: "bell"; chevron: false; interactive: false; danger: modelData.kind === "smart" || modelData.kind === "mirror"
            title: modelData.text
            sub: "Now · checked " + (page.st.alerts.at ? new Date(page.st.alerts.at * 1000).toLocaleTimeString(Qt.locale(), "h:mm AP") : "")
        }
    }
    SetSwitch {
        visible: !!page.al
        icon: "bell"; title: "Alerts"
        sub: page.al && page.al.on ? "On: checked every hour, and you're told once a day while something's wrong"
             : "Off. Turn on to hear about a failing drive, a full drive, a failed backup or a broken mirror"
        checked: !!page.al && page.al.on
        onToggled: v => page.set("alertsOn", v ? "on" : "off", v ? "Alerts on" : "Alerts off")
    }
    SetSwitch {
        visible: !!page.al && page.al.on
        icon: "store"; title: "A drive starts to fail"
        sub: "Its health check (SMART) fails, or it runs very hot (65 °C or more)"
        checked: !!page.al && page.al.smart
        onToggled: v => page.set("alertSmart", v ? "on" : "off")
    }
    SetSwitch {
        visible: !!page.al && page.al.on
        icon: "store"; title: "A drive is almost full"
        sub: "A drive with a share or a backup on it"
        checked: !!page.al && page.al.full
        onToggled: v => page.set("alertFull", v ? "on" : "off")
    }
    SetChoice {
        visible: !!page.al && page.al.on && page.al.full
        title: "Almost full means"
        options: [{ value: 80, label: "80%" }, { value: 85, label: "85%" }, { value: 90, label: "90%" }, { value: 95, label: "95%" }]
        current: page.al ? page.al.fullPct : 90
        onChosen: v => page.set("alertFullPct", String(v))
    }
    SetSwitch {
        visible: !!page.al && page.al.on
        icon: "update"; title: "A backup fails or is late"
        sub: "It didn't finish, didn't run for days, or its weekly check found a problem"
        checked: !!page.al && page.al.backup
        onToggled: v => page.set("alertBackup", v ? "on" : "off")
    }
    SetSwitch {
        visible: !!page.al && page.al.on
        icon: "restart"; title: "A mirror is missing a drive"
        sub: "A mirrored drive (Drives & undo) lost one of its drives or has read or write errors"
        checked: !!page.al && page.al.mirror
        onToggled: v => page.set("alertMirror", v ? "on" : "off")
    }
    SetSwitch {
        visible: !!page.al && page.al.on
        icon: "globe"; title: "Also send them to my devices"
        sub: page.al && page.al.peers.length ? "Through Taildrop to " + page.al.peers.join(", ") + " (a small note in their Taildrop files). Nothing goes anywhere else"
             : "Through Taildrop (Tailscale), to the devices you pick below. Nothing goes anywhere else"
        checked: !!page.al && page.al.tailscale
        onToggled: v => { page.set("alertTailscale", v ? "on" : "off"); if (v) page.peersOpen = true; }
    }
    Repeater {
        model: page.al && page.al.on && page.al.tailscale && page.st && page.st.tailscale ? (page.st.tailscale.peers || []) : []
        delegate: SetSwitch {
            required property var modelData
            icon: modelData.os === "android" || modelData.os === "iOS" ? "cam" : "desktops"; title: modelData.name
            sub: (modelData.online ? "Online" : "Offline") + " · " + modelData.os
            checked: page.al.peers.indexOf(modelData.dns || modelData.name) >= 0
            onToggled: v => act.run(["alert-peer", modelData.dns || modelData.name, v ? "on" : "off"], () => page.refresh(), e => page.say(e, true))
        }
    }
    SetRow {
        visible: !!page.al && page.al.on
        icon: "bell"; title: "Send a test alert"
        sub: "A notification here (and a note to the devices you picked)"
        onActivated: act.run(["alerts-test"], () => page.say("Sent ✓"), e => page.say(e, true))
    }

    // ---------------------------------------------------------------- power
    SetHeader { text: "Power" }
    SetChoice {
        visible: !!page.pw
        title: "Sleep when nobody needs it for"
        sub: "Never while Game Mode runs, a share is open or a backup runs. While you're signed in to the desktop, Settings › Power's own timer decides"
        options: [{ value: 0, label: "Never" }, { value: 30, label: "30 min" }, { value: 60, label: "1 hour" }, { value: 120, label: "2 hours" }, { value: 240, label: "4 hours" }]
        current: page.pw ? page.pw.sleepIdle : 0
        onChosen: v => page.set("sleepIdle", String(v), v ? "It sleeps after " + page.mins(v) + " nobody needs it" : "It doesn't sleep by itself")
    }
    SetSwitch {
        visible: !!page.pw
        icon: "sleep"; title: "Stay awake while my devices use it"
        sub: "While a share is open or a backup runs, nothing puts it to sleep (Settings › Power's timer included)"
        checked: !!page.pw && page.pw.keepAwake
        onToggled: v => page.set("keepAwake", v ? "on" : "off")
    }
    SetSwitch {
        visible: !!page.cfg
        icon: "wired"; title: "Wake-on-LAN (wake from your devices)"
        sub: page.st && page.st.wired && page.st.wired.length ? "Your other devices can wake this PC over the network (" + page.st.wired.map(w => w.iface).join(", ") + "). Wi-Fi rarely can: use a cable"
             : "Needs a network cable: no wired network card found"
        checked: !!page.cfg && page.cfg.wakeFromAway
        onToggled: v => page.set("wakeFromAway", v ? "on" : "off")
    }
    SetSwitch {
        visible: !!page.pw
        icon: "clock"; title: "Sleep at night, wake in the morning"
        sub: page.pw && page.pw.schedule ? "Sleeps at " + page.pw.sleepAt + " (unless it's busy) and wakes at " + page.pw.wakeAt + " every day" : "A nightly schedule (backups run once it's awake)"
        checked: !!page.pw && page.pw.schedule
        onToggled: v => page.set("schedule", v ? "on" : "off")
    }
    SetChoice {
        visible: !!page.pw && page.pw.schedule
        title: "Sleep at"
        options: [{ value: "23:00", label: "23:00" }, { value: "00:00", label: "0:00" }, { value: "01:00", label: "1:00" }, { value: "02:00", label: "2:00" }]
        current: page.pw ? page.pw.sleepAt : "01:00"
        onChosen: v => page.set("sleepAt", v)
    }
    SetChoice {
        visible: !!page.pw && page.pw.schedule
        title: "Wake at"
        options: [{ value: "02:30", label: "2:30 (backups)" }, { value: "06:00", label: "6:00" }, { value: "07:00", label: "7:00" }, { value: "08:00", label: "8:00" }]
        current: page.pw ? page.pw.wakeAt : "07:00"
        onChosen: v => page.set("wakeAt", v)
    }
    SetChoice {
        visible: !!page.pw
        title: "Spin down idle storage drives after"
        sub: page.st && page.st.tools && !page.st.tools.hdparm ? "hdparm isn't on this PC yet" : "Spinning hard drives only, never the system drive or an SSD (hdparm)"
        options: [{ value: 0, label: "Never" }, { value: 10, label: "10 min" }, { value: 20, label: "20 min" }, { value: 30, label: "30 min" }, { value: 60, label: "1 hour" }]
        current: page.pw ? page.pw.spinDown : 0
        onChosen: v => page.set("spinDown", String(v))
    }
    SetRow {
        visible: !!page.st && !!page.st.power && !!page.st.power.quietSince
        icon: "info"; chevron: false; interactive: false; title: "Right now"
        sub: page.st && page.st.power ? (page.st.power.quietSince ? "Nobody has needed it since " + new Date(page.st.power.quietSince * 1000).toLocaleTimeString(Qt.locale(), "h:mm AP") : "") : ""
    }

    // ---------------------------------------------------------------- UPS (hidden until one is seen)
    SetHeader { text: "UPS"; visible: page.upsShown }
    SetSwitch {
        visible: page.upsShown && !!page.pw
        icon: "batt"; title: "Shut down cleanly on battery"
        sub: (page.st && page.st.ups ? page.st.ups + " found. " : "No UPS plugged in right now. ")
             + (page.st && page.st.tools && !page.st.tools.nut ? "NUT isn't on this PC yet" : "When its battery runs low, this PC shuts down cleanly (Network UPS Tools, only on this PC)")
        checked: !!page.pw && page.pw.ups
        onToggled: v => page.set("ups", v ? "on" : "off")
    }
    SetRow {
        visible: page.upsShown && !!page.pw && page.pw.ups && !!page.ups
        icon: "batt"; chevron: false; interactive: false; title: "Battery " + (page.ups ? page.ups.charge + "%" : "")
        sub: page.ups ? (page.ups.status.indexOf("OB") >= 0 ? "On battery" : "On mains power") + (page.ups.runtime ? " · about " + Math.round(page.ups.runtime / 60) + " minutes on battery" : "") : ""
    }
    Text {
        visible: !page.upsShown && !!page.st
        Layout.fillWidth: true; Layout.topMargin: 6
        text: "UPS settings show once one is plugged in (or with Show hidden items on the Overview page)."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && !!page.st.applied && page.st.applied.ok === false
        Layout.fillWidth: true
        text: page.st && page.st.applied ? "Saved, but the system couldn't be changed to match: " + (page.st.applied.error || "") : ""
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

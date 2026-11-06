// Hearth Settings › Hearth Link › Your devices: each of your other paired Hearth devices at a glance (online or when it
// was last seen, Game Mode / Desktop Mode / Server mode, what's playing, battery, CPU and GPU use and temperature,
// memory, graphics memory, drives, Hearth OS version, uptime). Pick one for its last hour in small graphs and what you
// can do with it (its shared folders in Hearth Files, wake it, remote desktop).
// The numbers come over Hearth Link from each device (hearth-device-status, status.get: paired devices only, never
// the internet). Opening this page writes no setting; the switch "Let my other devices see this device's status"
// (Settings.shareStatus, on Hearth Link's page) is the only setting here, written only when you flip it.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Your devices"
    property var devices: []
    property string err: ""
    property bool loaded: false
    property string detail: ""              // the device whose details are shown
    readonly property var cur: devices.find(d => d.id === detail) || null
    readonly property var modes: ({ game: "Game Mode", desktop: "Desktop Mode", server: "Server mode" })
    readonly property var kindIcons: ({ handheld: "pad", laptop: "display", desktop: "cpu" })

    SetJob { id: job; helper: "hearth-device-status" }
    // Hearth Files' "Show other devices in Files" (files.json showDevices: on unless you turn it off; only read here
    // until you flip it, then only that key is merged in).
    SetJob { id: filesJob; helper: "hearth-files" }
    property bool showInFiles: true

    function load() {
        if (job.busy) return;
        job.run(["devices"], r => { page.err = ""; page.loaded = true; page.replaceList("devices", (r && r.devices) || []); },
                e => { page.err = e; page.loaded = true; });
    }
    Component.onCompleted: { load(); filesJob.run(["prefs"], r => page.showInFiles = !(r && r.showDevices === false), () => {}); }

    Timer { interval: 15000; repeat: true; running: page.visible; onTriggered: page.load() }
    onVisibleChanged: if (!visible) detail = ""

    // ---- words
    function ago(ms) {
        if (!ms) return "never";
        const s = Math.max(0, (Date.now() - ms) / 1000);
        return s < 90 ? "just now" : s < 3600 ? Math.round(s / 60) + " min ago" : s < 86400 ? Math.round(s / 3600) + " h ago" : Math.round(s / 86400) + " days ago";
    }
    function upText(s) {
        if (!s && s !== 0) return "";
        const d = Math.floor(s / 86400), h = Math.floor(s % 86400 / 3600), m = Math.floor(s % 3600 / 60);
        return d ? d + " d " + h + " h" : h ? h + " h " + m + " min" : m + " min";
    }
    function gb(v) { return v >= 100 ? Math.round(v) + " GB" : (Math.round(v * 10) / 10) + " GB"; }
    function headline(d) {
        if (!d.online) return "Offline or asleep · seen " + ago(d.lastSeen);
        if (d.old) return "Online · update it to Hearth OS 1.6 to see more";
        if (!d.shared) return "Online · doesn't share its status";
        const s = d.status || {};
        return ["Online", modes[s.mode] || "", s.playing ? "Playing " + s.playing.title : ""].filter(x => x).join(" · ");
    }
    function summary(s) {
        if (!s) return "";
        const out = [];
        if (s.battery) out.push("Battery " + s.battery.percent + "%" + (s.battery.charging ? " charging" : ""));
        if (s.cpu && s.cpu.use !== undefined) out.push("CPU " + s.cpu.use + "%" + (s.cpu.temp !== undefined ? " · " + s.cpu.temp + "°C" : ""));
        if (s.gpu && s.gpu.use !== undefined) out.push("GPU " + s.gpu.use + "%" + (s.gpu.temp !== undefined ? " · " + s.gpu.temp + "°C" : ""));
        if (s.ram) out.push("RAM " + s.ram.used + " / " + s.ram.total + " GB");
        return out.join("   ");
    }

    // ------------------------------------------------------------ the list
    ColumnLayout {
        visible: page.detail === ""
        Layout.fillWidth: true
        spacing: 0
        SetCardGroup {
            title: "Your other devices"
            Text {
                visible: page.loaded && page.devices.length === 0
                Layout.fillWidth: true; Layout.leftMargin: 4; Layout.bottomMargin: 6
                wrapMode: Text.WordWrap; color: page.err ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 13
                text: page.err ? page.err : "No other Hearth devices are paired yet. Pair one on Hearth Link's page."
            }
            Text {
                visible: !page.loaded
                Layout.leftMargin: 4
                text: "Asking your devices…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
            }
            Repeater {
                model: page.devices
                delegate: DeviceCard { required property var modelData; dev: modelData; onActivated: { page.detail = modelData.id; Qt.callLater(page.focusFirst); } }
            }
        }
        SetCardGroup {
            title: "This device"
            SetSwitch {
                icon: "eye"; title: "Let my other devices see this device's status"
                sub: "Battery, use, temperatures, drives and what's playing, only for devices paired with this one (Hearth Link). Never the internet."
                checked: Settings.shareStatus
                onToggled: v => Settings.shareStatus = v
            }
            SetSwitch {
                icon: "folder"; title: "Show other devices in Files"
                sub: "Your devices' shared folders, and other computers sharing on this network or through Tailscale, in Hearth Files' sidebar and Open / Save windows"
                checked: page.showInFiles
                onToggled: v => { page.showInFiles = v; filesJob.run(["prefs-set", JSON.stringify({ showDevices: v })], () => {}, e => SettingsNav.toast(e)); }
            }
        }
    }

    // ------------------------------------------------------------ one device
    ColumnLayout {
        visible: page.detail !== ""
        Layout.fillWidth: true
        spacing: 0
        SetRow { icon: "back"; title: "Back to your devices"; chevron: false; onActivated: { page.detail = ""; Qt.callLater(page.focusFirst); } }
        DeviceCard { visible: !!page.cur; dev: page.cur || ({}); interactive: false; Layout.topMargin: 6 }

        SetCardGroup {
            title: "The last hour"
            visible: !!page.cur && !!page.cur.status && (page.cur.status.history || []).length > 1
            GridLayout {
                Layout.fillWidth: true
                columns: page.width > 900 ? 3 : 2
                columnSpacing: 10; rowSpacing: 10
                Graph { label: "CPU"; unit: "%"; key: "cpu"; max: 100 }
                Graph { label: "GPU"; unit: "%"; key: "gpu"; max: 100 }
                Graph { label: "Memory"; unit: "%"; key: "ram"; max: 100 }
                Graph { label: "CPU temperature"; unit: "°C"; key: "cpuTemp"; max: 100 }
                Graph { label: "GPU temperature"; unit: "°C"; key: "gpuTemp"; max: 100 }
                Graph { label: "Battery"; unit: "%"; key: "battery"; max: 100 }
            }
        }

        SetCardGroup {
            title: page.cur && page.cur.stale ? "Last known (it's offline)" : "Now"
            visible: !!page.cur && !!page.cur.status
            Fact { icon: "cpu"; title: "Processor"; value: page.cur && page.cur.status && page.cur.status.cpu ? [page.cur.status.cpu.use !== undefined ? page.cur.status.cpu.use + "% in use" : "", page.cur.status.cpu.temp !== undefined ? page.cur.status.cpu.temp + "°C" : ""].filter(x => x).join(" · ") : "" }
            Fact {
                icon: "display"; title: "Graphics"
                readonly property var g: page.cur && page.cur.status ? page.cur.status.gpu : null
                value: !g ? "" : [g.vendor ? ({ nvidia: "Nvidia", amd: "AMD", intel: "Intel" })[g.vendor] || g.vendor : "", g.use !== undefined ? g.use + "% in use" : "", g.temp !== undefined ? g.temp + "°C" : "",
                                  g.vramTotal ? "graphics memory " + (Math.round(g.vramUsed / 102.4) / 10) + " / " + (Math.round(g.vramTotal / 102.4) / 10) + " GB" : ""].filter(x => x).join(" · ")
            }
            Fact { icon: "grid"; title: "Memory"; value: page.cur && page.cur.status && page.cur.status.ram ? page.cur.status.ram.used + " of " + page.cur.status.ram.total + " GB in use" : "" }
            Fact {
                icon: page.cur && page.cur.status && page.cur.status.battery && page.cur.status.battery.charging ? "batt-charging" : "batt"; title: "Battery"
                value: page.cur && page.cur.status && page.cur.status.battery ? page.cur.status.battery.percent + "%" + (page.cur.status.battery.charging ? ", charging" : page.cur.status.battery.pluggedIn ? ", plugged in" : ", on battery") : ""
            }
            Repeater {
                model: page.cur && page.cur.status ? (page.cur.status.drives || []) : []
                delegate: SetRow {
                    required property var modelData
                    interactive: false; chevron: false
                    icon: "drive"; title: modelData.name
                    sub: page.gb(modelData.used) + " of " + page.gb(modelData.total) + " used"
                    value: Math.round(100 * modelData.used / Math.max(1, modelData.total)) + "%"
                }
            }
            Fact { icon: "update"; title: "Hearth OS"; value: page.cur && page.cur.status && page.cur.status.version ? page.cur.status.version : "" }
            Fact { icon: "clock"; title: "On for"; value: page.cur && page.cur.status ? page.upText(page.cur.status.uptime) : "" }
        }

        SetCardGroup {
            title: "Do something with it"
            visible: !!page.cur
            // Open shared folders: its Home server shares (in its status), in Hearth Files, signed in once.
            Repeater {
                model: page.cur && page.cur.status && page.cur.status.smb ? (page.cur.status.shares || []) : []
                delegate: SetRow {
                    required property var modelData
                    icon: "folder"; title: "Open shared folder " + modelData.name; chevron: true
                    sub: [modelData.badge === "home" ? "Home" : modelData.badge === "drive" ? "Whole drive" : modelData.badge === "system" ? "System drive" : "", modelData.write ? "Read & write" : "Read only", "in Hearth Files"].filter(x => x).join(" · ")
                    onActivated: Quickshell.execDetached(["hearth-filemanager", "hearthnet://" + encodeURIComponent(JSON.stringify({
                        device: page.cur.id, name: page.cur.name, share: modelData.name, home: page.cur.status.smb.home || "", away: page.cur.status.smb.away || "", wake: page.cur.wake === true }))])
                }
            }
            Repeater {
                model: page.cur ? (page.cur.shares || []) : []
                delegate: SetRow {
                    required property var modelData
                    icon: "folder"; title: "Open " + modelData.name; sub: "Its shared folder in Hearth Files (Hearth Link)"; chevron: true
                    onActivated: Quickshell.execDetached(["hearth-filemanager", "hearth://" + page.cur.id + "/" + modelData.id + "/"])
                }
            }
            SetRow {
                visible: !!page.cur && !page.cur.online && page.cur.wake === true
                icon: "power"; title: "Wake it"; sub: "Wake-on-LAN, as Hearth Storage does"; chevron: false
                onActivated: job.run(["wake", page.cur.id], () => SettingsNav.toast("Waking " + page.cur.name + "…"), e => SettingsNav.toast(e))
            }
            SetRow {
                visible: !!page.cur && page.cur.online
                icon: "play"; title: "Remote desktop"; sub: "Use it from here, or play its games (Remote play)"; chevron: true
                onActivated: SettingsNav.openPage("remote")
            }
            SetRow {
                icon: "desktops"; title: "Hearth Link"; sub: "Pairing, sync, copies and Hearth Storage"; chevron: true
                onActivated: SettingsNav.openPage("link")
            }
        }
    }

    // ------------------------------------------------------------ pieces
    component DeviceCard: SetBase {
        id: dc
        property var dev: ({})
        readonly property var s: dev && dev.status ? dev.status : null
        Layout.fillWidth: true
        implicitHeight: dcRow.implicitHeight + 2 * SetStyle.pad
        opacity: dev && dev.online ? 1 : 0.72
        Accessible.name: (dev.name || "") + ": " + page.headline(dev)
        RowLayout {
            id: dcRow
            anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: SetStyle.pad; rightMargin: SetStyle.pad }
            spacing: 14
            Rectangle {
                Layout.preferredWidth: 44; Layout.preferredHeight: 44; radius: Math.max(6, Theme.r(10))
                color: dc.dev && dc.dev.online ? SetStyle.tint(Theme.accent, 0.18) : SetStyle.card2
                Icon { anchors.centerIn: parent; name: page.kindIcons[dc.s ? dc.s.kind : ""] || "desktops"; size: 22; color: dc.dev && dc.dev.online ? Theme.accent : Theme.muted }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3
                RowLayout {
                    spacing: 8
                    Text { text: dc.dev.name || "Device"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; elide: Text.ElideRight }
                    SetChip { text: dc.dev.online ? "Online" : "Offline"; kind: dc.dev.online ? "ok" : "warn" }
                }
                Text { Layout.fillWidth: true; text: page.headline(dc.dev); color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                Text { visible: text !== ""; Layout.fillWidth: true; text: page.summary(dc.s); color: Theme.text; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
            }
            // a small battery or memory meter on the right
            ColumnLayout {
                visible: !!dc.s && (!!dc.s.battery || !!dc.s.ram)
                spacing: 4
                Text { Layout.alignment: Qt.AlignRight; text: dc.s && dc.s.battery ? dc.s.battery.percent + "%" : dc.s && dc.s.ram ? Math.round(100 * dc.s.ram.used / Math.max(1, dc.s.ram.total)) + "% RAM" : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                Rectangle {
                    implicitWidth: 90; implicitHeight: 6; radius: 3; color: SetStyle.card2
                    readonly property real frac: !dc.s ? 0 : dc.s.battery ? dc.s.battery.percent / 100 : dc.s.ram ? dc.s.ram.used / Math.max(1, dc.s.ram.total) : 0
                    Rectangle { width: parent.width * Math.max(0, Math.min(1, parent.frac)); height: parent.height; radius: 3; color: dc.s && dc.s.battery && dc.s.battery.percent < 20 && !dc.s.battery.charging ? SetStyle.warn : Theme.accent }
                }
            }
            Icon { visible: dc.interactive; name: "chev"; size: 16; color: Theme.muted }
        }
    }
    component Fact: SetRow {
        interactive: false; chevron: false
        visible: value !== ""
    }
    component Graph: Rectangle {
        id: gr
        property string label
        property string unit
        property string key
        property real max: 100
        readonly property var pts: page.cur && page.cur.status ? (page.cur.status.history || []).filter(p => p[gr.key] !== undefined) : []
        visible: pts.length > 1
        Layout.fillWidth: true
        implicitHeight: 110
        radius: SetStyle.radius
        color: SetStyle.card
        border.width: 1; border.color: SetStyle.tint(Theme.text, 0.05)
        Text { x: 12; y: 10; text: gr.label; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
        Text { anchors { right: parent.right; rightMargin: 12; top: parent.top; topMargin: 8 } text: gr.pts.length ? gr.pts[gr.pts.length - 1][gr.key] + gr.unit : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
        Canvas {
            id: cv
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; top: parent.top; margins: 12; topMargin: 34 }
            onPaint: {
                const ctx = getContext("2d");
                ctx.reset();
                const p = gr.pts;
                if (p.length < 2) return;
                const t0 = p[0].t, t1 = Math.max(t0 + 1, p[p.length - 1].t);
                const xy = q => [(q.t - t0) / (t1 - t0) * width, height - Math.max(0, Math.min(1, q[gr.key] / gr.max)) * height];
                ctx.beginPath();
                p.forEach((q, i) => { const c = xy(q); if (i) ctx.lineTo(c[0], c[1]); else ctx.moveTo(c[0], c[1]); });
                ctx.lineWidth = 2; ctx.strokeStyle = Theme.css(Theme.accent, 1); ctx.stroke();
                ctx.lineTo(width, height); ctx.lineTo(0, height); ctx.closePath();
                ctx.fillStyle = Theme.css(Theme.accent, 0.16); ctx.fill();
            }
            Connections { target: gr; function onPtsChanged() { cv.requestPaint(); } }
            onWidthChanged: requestPaint()
            Component.onCompleted: requestPaint()
        }
    }
}

// Hearth Settings › Home server › Overview (docs/HOME-SERVER.md): this PC as a home server for your other devices.
// Light mode while you play, what's shared, Tailscale and the drives at a glance, the setup steps, and "Show hidden
// items" (things for the Hearth AI Command Center, cameras or a UPS show only once one was seen; display only: hiding
// never turns anything off). Everything goes through hearth-home-server, which saves only what you change.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Home server"
    property var st: null
    property var drv: null
    property string msg: ""
    property bool bad: false
    readonly property var cfg: st ? st.settings : null
    readonly property var ts: st ? (st.tailscale || {}) : ({})
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.st = { error: e }); }
    function set(k, v, ok) { act.run(["set", k, v], () => { if (ok) page.say(ok); page.refresh(); }, e => { page.say(e, true); page.refresh(); }); }
    function size(b) { return b >= 1e12 ? (b / 1e12).toFixed(1) + " TB" : b >= 1e9 ? Math.round(b / 1e9) + " GB" : Math.round((b || 0) / 1e6) + " MB"; }
    function health() {
        if (!drv || !drv.drives) return { big: "—", sub: "Open Drives & undo to check" };
        const ds = drv.drives.filter(d => d.smart);
        if (!ds.length) return { big: "—", sub: drv.drives.length + " drives · no health data" };
        const failing = ds.filter(d => d.smart.passed === false);
        const temps = ds.map(d => d.smart.temp).filter(t => t > 0);
        return { big: failing.length ? "Check" : "Good", bad: failing.length > 0,
                 sub: drv.drives.length + " drives" + (temps.length ? " · " + Math.min.apply(null, temps) + "–" + Math.max.apply(null, temps) + " °C" : "") };
    }
    Component.onCompleted: { refresh(); drives.run(["drives"], r => page.drv = r, () => page.drv = null); }
    Timer { interval: 5000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }
    SetJob { id: drives; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: page.st && page.st.host ? "This PC (\"" + page.st.host + "\") shares folders with your other devices: Windows PCs, phones, TVs and Hearth devices, at home and anywhere through Tailscale."
            : "This PC shares folders with your other devices: Windows PCs, phones, TVs and Hearth devices."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && (!!page.st.error || page.st.helper === false)
        Layout.fillWidth: true
        text: page.st && page.st.error ? page.st.error : "Parts of Home server come with the Hearth OS update that brings it (Samba, Tailscale): what you choose here is kept."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetSwitch {
        visible: !!page.cfg
        icon: "sleep"; title: "Light mode while you play"
        sub: "When Game Mode or a game runs, big jobs (snapshots, emptying recycle bins) wait. Sharing keeps working."
             + (page.st && page.st.playing && page.cfg && page.cfg.lightMode ? " · On now: you're playing" : "")
        checked: !!page.cfg && page.cfg.lightMode !== false
        onToggled: v => page.set("lightMode", v ? "on" : "off")
    }

    // At a glance
    GridLayout {
        visible: !!page.cfg
        Layout.fillWidth: true
        Layout.topMargin: 6
        columns: page.width > 760 ? 4 : 2
        columnSpacing: 10; rowSpacing: 10
        Repeater {
            model: page.cfg ? [
                { label: "Storage used", big: page.st.usage.total ? page.size(page.st.usage.used) + " / " + page.size(page.st.usage.total) : "—",
                  sub: page.st.usage.total ? "on the drives you share from" : "nothing shared yet", frac: page.st.usage.total ? page.st.usage.used / page.st.usage.total : 0 },
                { label: "Shared folders", big: String(page.st.shares.filter(s => s.on !== false).length),
                  sub: page.st.shares.filter(s => s.on !== false).map(s => s.name).slice(0, 4).join(" · ") || "nothing is shared until you share it", frac: -1 },
                { label: "Tailscale", big: page.ts.state === "Running" ? "Connected" : page.ts.installed === false ? "Not installed" : page.ts.running === false ? "Off" : page.ts.state === "NeedsLogin" ? "Sign in" : (page.ts.state || "—"),
                  sub: page.ts.state === "Running" ? (page.ts.peers || []).filter(p => p.online).length + " of your devices online" : "reach this PC from anywhere", frac: -1, good: page.ts.state === "Running" },
                { label: "Drive health", big: page.health().big, sub: page.health().sub, frac: -1, good: page.health().big === "Good", warn: !!page.health().bad }
            ] : []
            delegate: Rectangle {
                required property var modelData
                Layout.fillWidth: true
                implicitHeight: 104
                radius: Theme.r(12)
                color: Theme.panel2
                ColumnLayout {
                    anchors { fill: parent; margins: 14 }
                    spacing: 4
                    Text { text: modelData.label; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    Text { text: modelData.big; color: modelData.warn ? Theme.danger : modelData.good ? "#57c27a" : Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold; elide: Text.ElideRight; Layout.fillWidth: true }
                    Rectangle {
                        visible: modelData.frac >= 0
                        Layout.fillWidth: true; implicitHeight: 6; radius: 3; color: Theme.panel3
                        Rectangle { width: parent.width * Math.max(0, Math.min(1, modelData.frac)); height: parent.height; radius: 3; color: Theme.accent }
                    }
                    Text { text: modelData.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight; Layout.fillWidth: true }
                }
            }
        }
    }

    SetRow {
        visible: !!page.cfg && !page.cfg.setupDone
        icon: "check"; title: "Set up your home server"
        sub: "A few steps, all with the controller: this PC's name, Tailscale, your first shared folder and your first backup"
        onActivated: SettingsNav.openPage("hs-setup")
    }

    SetHeader { text: "Shared folders"; visible: !!page.cfg }
    Repeater {
        model: page.st ? page.st.shares : []
        delegate: SetRow {
            required property var modelData
            icon: "folder"; title: modelData.name
            sub: (modelData.on === false ? "Not shared now" : (modelData.write ? "Read & write" : "Read only") + " · " + (modelData.reach === "tailscale" || (page.cfg && page.cfg.tsOnly) ? "Tailscale only" : "Home network and Tailscale"))
                 + (modelData.there ? "" : " · its folder isn't there (is the drive plugged in?)")
            value: modelData.path
            onActivated: SettingsNav.openPage("hs-shares")
        }
    }
    SetRow {
        visible: !!page.st && page.st.shares.length === 0
        icon: "folder"; title: "Share a folder"
        sub: "Nothing is shared until you share it. Pick a folder for your Windows PC, phone, TV and Hearth devices"
        onActivated: SettingsNav.openPage("hs-shares")
    }

    SetHeader { text: "Reach it from anywhere"; visible: !!page.cfg }
    SetRow {
        visible: !!page.cfg
        icon: "globe"; title: "Tailscale"
        sub: page.ts.state === "Running" ? "Reach this PC from anywhere as " + (page.ts.dns || page.ts.name) + (page.cfg && page.cfg.tsOnly ? " · only over Tailscale" : "")
           : page.ts.installed === false ? "Comes with the Hearth OS update that brings Home server"
           : "A private network for your devices only. Nothing is opened to the internet"
        value: page.ts.state === "Running" ? "Connected" : ""
        onActivated: SettingsNav.openPage("hs-tailscale")
    }
    SetRow {
        visible: !!page.cfg
        icon: "restart"; title: "Drives & undo"
        sub: "Space, health and temperature of every drive; snapshots to bring back deleted or changed files" + (page.cfg && page.cfg.snapshots ? " · snapshots on" : "")
        onActivated: SettingsNav.openPage("hs-drives")
    }

    SetHeader { text: "Hidden items"; visible: !!page.cfg }
    SetSwitch {
        visible: !!page.cfg
        icon: "info"; title: "Show hidden items"
        sub: "Items for the Hearth AI Command Center show when it was seen on one of your devices in the last " + (page.cfg ? page.cfg.ccDays : 30)
             + " days, and cameras or UPS items once one is found. Hiding never turns anything off." + (page.st && page.st.hiddenCount ? " " + page.st.hiddenCount + " hidden now." : "")
        checked: !!page.cfg && page.cfg.showHidden
        onToggled: v => page.set("showHidden", v ? "on" : "off")
    }
    SetChoice {
        visible: !!page.cfg
        title: "Command Center seen within"
        options: [{ value: 7, label: "7 days" }, { value: 30, label: "30 days" }, { value: 90, label: "90 days" }, { value: 365, label: "A year" }]
        current: page.cfg ? page.cfg.ccDays : 30
        onChosen: v => page.set("ccDays", String(v))
    }
    Text {
        visible: !!page.cfg
        Layout.fillWidth: true; Layout.topMargin: 6
        text: "Coming next to Home server: Files & links, Services, Gaming, Family & home, AI, Backups & sync, Safety, People, and Alerts & power."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && !!page.st.applied && page.st.applied.ok === false
        Layout.fillWidth: true
        text: page.st && page.st.applied ? "Your settings are saved, but the system couldn't be changed to match: " + (page.st.applied.error || "") : ""
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

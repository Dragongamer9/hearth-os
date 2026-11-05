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
    function alertList() { return st && st.alerts ? (st.alerts.active || []) : []; }
    readonly property var svc: st ? (st.services || {}) : ({})
    readonly property bool aiShown: !!st && !!st.seen && !!st.seen.commandCenter && (st.seen.commandCenter.shown || (!!svc.ai && svc.ai.on))
    function servicesBig() {
        const on = [svc.media && svc.media.on ? "Media" : "", svc.web && svc.web.on ? "Web" : "", svc.ai && svc.ai.on ? "AI" : ""].filter(x => x);
        return { big: on.length ? String(on.length) + " on" : "None", sub: on.length ? on.join(" · ") : "media server, files on the web, AI", good: on.length > 0 };
    }
    function backupBig() {
        const jobs = st ? (st.backups || []) : [];
        if (!jobs.length) return { big: "None yet", sub: "a second copy somewhere else", good: false, warn: false };
        const failed = jobs.filter(j => j.state && j.state.last && j.state.last.ok === false);
        if (failed.length) return { big: "Didn't finish", sub: failed.map(j => j.name).join(", "), good: false, warn: true };
        if (jobs.some(j => j.state && j.state.running)) return { big: "Running", sub: "backing up now", good: true, warn: false };
        const last = Math.max.apply(null, jobs.map(j => (j.state && j.state.lastOk) || 0));
        if (!last) return { big: "Not yet", sub: jobs.some(j => j.state && j.state.waiting) ? "waiting for your game" : jobs.length + " set up", good: false, warn: false };
        const h = Math.round((Date.now() / 1000 - last) / 3600);
        return { big: h < 1 ? "Just now" : h < 48 ? h + " h ago" : Math.round(h / 24) + " days ago", sub: jobs.length + (jobs.length === 1 ? " backup" : " backups"), good: h < 48, warn: false };
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
        sub: "When Game Mode or a game runs, big jobs (backups, snapshots, emptying recycle bins) wait. Sharing keeps working. Also in Game Mode's Quick Access (Hearth Link)."
             + (page.st && page.st.playing && page.cfg && page.cfg.lightMode ? " · On now: you're playing" : "")
        checked: !!page.cfg && page.cfg.lightMode !== false
        onToggled: v => page.set("lightMode", v ? "on" : "off")
    }

    // At a glance
    GridLayout {
        visible: !!page.cfg
        Layout.fillWidth: true
        Layout.topMargin: 6
        columns: page.width > 760 ? 3 : 2
        columnSpacing: 10; rowSpacing: 10
        Repeater {
            model: page.cfg ? [
                { label: "Storage used", big: page.st.usage.total ? page.size(page.st.usage.used) + " / " + page.size(page.st.usage.total) : "—",
                  sub: page.st.usage.total ? "on the drives you share from" : "nothing shared yet", frac: page.st.usage.total ? page.st.usage.used / page.st.usage.total : 0 },
                { label: "Shared folders", big: String(page.st.shares.filter(s => s.on !== false).length),
                  sub: page.st.shares.filter(s => s.on !== false).map(s => s.name).slice(0, 4).join(" · ") || "nothing is shared until you share it", frac: -1 },
                { label: "Tailscale", big: page.ts.state === "Running" ? "Connected" : page.ts.installed === false ? "Not installed" : page.ts.running === false ? "Off" : page.ts.state === "NeedsLogin" ? "Sign in" : (page.ts.state || "—"),
                  sub: page.ts.state === "Running" ? (page.ts.peers || []).filter(p => p.online).length + " of your devices online" : "reach this PC from anywhere", frac: -1, good: page.ts.state === "Running" },
                { label: "Drive health", big: page.health().big, sub: page.health().sub, frac: -1, good: page.health().big === "Good", warn: !!page.health().bad },
                { label: "Backups", big: page.backupBig().big, sub: page.backupBig().sub, frac: -1, good: page.backupBig().good, warn: page.backupBig().warn },
                { label: "Alerts", big: page.alertList().length ? String(page.alertList().length) : page.cfg.alerts && page.cfg.alerts.on ? "All good" : "Off",
                  sub: page.alertList().length ? page.alertList()[0].text : page.cfg.alerts && page.cfg.alerts.on ? "checked every hour" : "turn on in Alerts & power",
                  frac: -1, good: !page.alertList().length && !!page.cfg.alerts && page.cfg.alerts.on, warn: page.alertList().length > 0 },
                { label: "Services", big: page.servicesBig().big, sub: page.servicesBig().sub, frac: -1, good: page.servicesBig().good },
                { label: "Safety", big: page.svc.guardTripped ? "Read only" : page.cfg.guard && page.cfg.guard.on ? "Watching" : page.cfg.tsOnly ? "Tailscale only" : "Check",
                  sub: page.svc.guardTripped ? "the ransomware guard stopped writes" : page.cfg.guard && page.cfg.guard.on ? "ransomware guard on" : "see Safety for simple fixes",
                  frac: -1, good: !page.svc.guardTripped && !!page.cfg.guard && page.cfg.guard.on, warn: !!page.svc.guardTripped }
            ].concat(page.aiShown ? [{ label: "AI service", big: page.svc.ai && page.svc.ai.on ? (page.svc.ai.active ? "Ready" : "Starting") : "Off",
                  sub: page.st.seen.commandCenter.device ? "Command Center seen on " + page.st.seen.commandCenter.device : "for the Hearth AI Command Center", frac: -1, good: !!page.svc.ai && page.svc.ai.active }] : []) : []
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
        icon: "search"; title: "Files & links"
        sub: "Search every share, free up space, share links and a drop-off folder" + (page.svc.links ? " · " + page.svc.links + (page.svc.links === 1 ? " link" : " links") : "")
        onActivated: SettingsNav.openPage("hs-files")
    }
    SetRow {
        visible: !!page.cfg
        icon: "restart"; title: "Drives & undo"
        sub: "Space, health and temperature of every drive; snapshots to bring back deleted or changed files" + (page.cfg && page.cfg.snapshots ? " · snapshots on" : "")
        onActivated: SettingsNav.openPage("hs-drives")
    }
    SetRow {
        visible: !!page.cfg
        icon: "update"; title: "Backups & sync"
        sub: "Back up your shares to a drive, another Hearth device or the cloud; phone photos; folders kept the same on your devices"
        value: page.st && page.st.backups && page.st.backups.length ? page.backupBig().big : ""
        onActivated: SettingsNav.openPage("hs-backups")
    }
    SetRow {
        visible: !!page.cfg
        icon: "window"; title: "Services"
        sub: "Optional apps for your devices: the media server (Jellyfin), files on the web" + (page.aiShown ? ", the AI service" : "") + "; each off until you turn it on"
        value: page.servicesBig().good ? page.servicesBig().big : ""
        onActivated: SettingsNav.openPage("hs-services")
    }
    SetRow {
        visible: !!page.cfg
        icon: "pad"; title: "Gaming"
        sub: "A Games folder for your Hearth devices, Remote play, and what Light mode holds back while you play"
        onActivated: SettingsNav.openPage("hs-gaming")
    }
    SetRow {
        visible: !!page.cfg
        icon: "user"; title: "Family & home"
        sub: "A Family folder for everyone" + (page.st && page.st.seen && page.st.seen.camera && page.st.seen.camera.shown ? ", your cameras" : "") + " and who can open what"
        onActivated: SettingsNav.openPage("hs-family")
    }
    SetRow {
        visible: !!page.cfg && page.aiShown
        icon: "mic"; title: "AI"
        sub: "The AI service on this PC's graphics card for the Hearth AI Command Center: models, its address, Light mode"
        value: page.svc.ai && page.svc.ai.on ? "On" : ""
        onActivated: SettingsNav.openPage("hs-ai")
    }
    SetRow {
        visible: !!page.cfg
        icon: "lock"; title: "Safety"
        sub: "Who can reach what, open ports, the ransomware guard and simple fixes you can undo"
        value: page.svc.guardTripped ? "Read only" : ""
        onActivated: SettingsNav.openPage("hs-safety")
    }
    SetRow {
        visible: !!page.cfg
        icon: "user"; title: "People"
        sub: "Family and friends with their own password and only the shares you pick"
        value: page.st && page.st.people && page.st.people.length ? page.st.people.length + (page.st.people.length === 1 ? " person" : " people") : ""
        onActivated: SettingsNav.openPage("hs-people")
    }
    SetRow {
        visible: !!page.cfg
        icon: "bell"; title: "Alerts & power"
        sub: "Hear about failing or full drives and failed backups; sleep, wake, spin down and a UPS"
        value: page.alertList().length ? page.alertList().length + (page.alertList().length === 1 ? " alert" : " alerts") : ""
        onActivated: SettingsNav.openPage("hs-alerts")
    }

    SetHeader { text: "Hidden items"; visible: !!page.cfg }
    SetSwitch {
        visible: !!page.cfg
        icon: "info"; title: "Show hidden items"
        sub: "Items for the Hearth AI Command Center (the AI page and service) show when it was seen on one of your devices in the last " + (page.cfg ? page.cfg.ccDays : 30)
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

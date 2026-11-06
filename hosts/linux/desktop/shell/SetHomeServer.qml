// Hearth Settings › Home server (the category's landing; was Home server › Overview; docs/HOME-SERVER.md): this PC as a home server for your other devices.
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
        const on = [svc.media && svc.media.on ? "Media" : "", svc.web && svc.web.on ? "Web" : "", svc.ai && svc.ai.on ? "AI" : "",
                    svc.pictures && svc.pictures.on ? "Pictures" : "", svc.voices && svc.voices.on ? "Voices" : ""].filter(x => x);
        return { big: on.length ? String(on.length) + " on" : "None", sub: on.length ? on.join(" · ") : "media server, files on the web, AI pictures and voices", good: on.length > 0 };
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

    // The redesign's landing for Home server (always its own category): a banner when something needs a look, four
    // numbers, then its pages in four groups. Tailscale's home is Network & internet now (a link card here); AI chat is
    // under Services.
    readonly property var attention: {
        const out = [];
        if (!st || !cfg) return out;
        const failed = (st.backups || []).filter(j => j.state && j.state.last && j.state.last.ok === false);
        if (failed.length) out.push({ text: "The backup " + failed.map(j => "\u201c" + j.name + "\u201d").join(", ") + " didn't finish" + ((failed[0].state.last || {}).error ? ": " + failed[0].state.last.error : ""), page: "hs-backups", action: "Open Backups" });
        if (svc.guardTripped) out.push({ text: "The ransomware guard stopped writes to your shares", page: "hs-safety", action: "Open Safety" });
        for (const a of alertList()) out.push({ text: a.text || "An alert", page: "hs-alerts", action: "Open Alerts" });
        if (health().bad) out.push({ text: "A drive's health check failed", page: "hs-drives", action: "Open Drives" });
        return out;
    }
    readonly property var backupsChip: {
        const jobs = st ? (st.backups || []) : [];
        const failed = jobs.filter(j => j.state && j.state.last && j.state.last.ok === false).length;
        return failed ? { chip: failed + " failed", kind: "warn" } : jobs.length ? { value: backupBig().big } : { value: "None yet" };
    }
    Text {
        Layout.fillWidth: true
        Layout.bottomMargin: 12
        text: page.st && page.st.host ? "This PC (\"" + page.st.host + "\") shares folders, media and AI with your other devices: Windows PCs, phones, TVs and Hearth devices, at home and anywhere through Tailscale."
            : "This PC shares folders, media and AI with your other devices: Windows PCs, phones, TVs and Hearth devices, at home and through Tailscale."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && (!!page.st.error || page.st.helper === false)
        Layout.fillWidth: true
        text: page.st && page.st.error ? page.st.error : "Parts of Home server come with the Hearth OS update that brings it (Samba, Tailscale): what you choose here is kept."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetBanner {
        visible: page.attention.length > 0
        Layout.bottomMargin: 9
        title: page.attention.length === 1 ? "1 thing needs a look" : page.attention.length + " things need a look"
        sub: page.attention.length ? page.attention[0].text : ""
        action: page.attention.length ? page.attention[0].action : ""
        onActivated: if (page.attention.length) SettingsNav.openPage(page.attention[0].page)
    }
    SetRow {
        visible: !!page.cfg && !page.cfg.setupDone
        chevron: true
        icon: "check"; title: "Set up your home server"
        sub: "A few steps, all with the controller: this PC's name, Tailscale, your first shared folder and your first backup"
        onActivated: SettingsNav.openPage("hs-setup")
    }

    // At a glance
    GridLayout {
        visible: !!page.cfg
        Layout.fillWidth: true
        Layout.topMargin: 6
        Layout.bottomMargin: 4
        columns: page.width > 860 ? 4 : 2
        columnSpacing: 10; rowSpacing: 10
        Repeater {
            model: page.cfg ? [
                { label: "Storage used", big: page.st.usage.total ? page.size(page.st.usage.used) : "—", of: page.st.usage.total ? "of " + page.size(page.st.usage.total) : "",
                  sub: page.st.usage.total ? "" : "nothing shared yet", frac: page.st.usage.total ? page.st.usage.used / page.st.usage.total : -1 },
                { label: "Shared folders", big: String(page.st.shares.filter(s => s.on !== false).length),
                  sub: page.st.shares.filter(s => s.on !== false).map(s => s.name).slice(0, 4).join(" · ") || "nothing is shared until you share it", frac: -1 },
                { label: "Tailscale", big: page.ts.state === "Running" ? "Connected" : page.ts.installed === false ? "Not installed" : page.ts.running === false ? "Off" : page.ts.state === "NeedsLogin" ? "Sign in" : (page.ts.state || "—"),
                  sub: page.ts.state === "Running" ? (page.ts.peers || []).filter(p => p.online).length + " of your devices online" : "reach this PC from anywhere", frac: -1, good: page.ts.state === "Running" },
                { label: "Drive health", big: page.health().big, sub: page.health().sub, frac: -1, good: page.health().big === "Good", warn: !!page.health().bad }
            ] : []
            delegate: Rectangle {
                required property var modelData
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                implicitHeight: 100
                radius: SetStyle.bigRadius
                color: SetStyle.card
                border.width: 1; border.color: SetStyle.tint(Theme.text, 0.05)
                ColumnLayout {
                    anchors { fill: parent; margins: 14; leftMargin: 16; rightMargin: 16 }
                    spacing: 3
                    Text { text: modelData.label; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 6
                        Text { text: modelData.big; color: modelData.warn ? Theme.danger : modelData.good ? SetStyle.ok : Theme.text; font.family: Theme.font; font.pixelSize: 21; font.weight: Font.DemiBold; elide: Text.ElideRight; Layout.maximumWidth: parent.width - ofText.implicitWidth - 6 }
                        Text { id: ofText; visible: (modelData.of || "") !== ""; text: modelData.of || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.Medium }
                        Item { Layout.fillWidth: true }
                    }
                    Rectangle {
                        visible: modelData.frac >= 0
                        Layout.fillWidth: true; Layout.topMargin: 4; implicitHeight: 8; radius: 4; color: SetStyle.card2
                        Rectangle { width: parent.width * Math.max(0, Math.min(1, modelData.frac)); height: parent.height; radius: 4; color: Theme.accent }
                    }
                    Text { visible: modelData.sub !== ""; text: modelData.sub; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight; Layout.fillWidth: true }
                }
            }
        }
    }

    GridLayout {
        visible: !!page.cfg
        Layout.fillWidth: true
        columns: page.width > 900 ? 2 : 1
        columnSpacing: 16; rowSpacing: 0
        ColumnLayout {
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            Layout.alignment: Qt.AlignTop
            spacing: 0
            SetCardGroup {
                title: "Sharing"
                SetLinkCard { target: "hs-shares"; value: page.st && page.st.shares ? String(page.st.shares.filter(s => s.on !== false).length) : ""
                    sub: page.st && page.st.shares && page.st.shares.length === 0 ? "Nothing is shared until you share it. Pick a folder for your devices" : "What your devices can open, and how" }
                SetLinkCard { target: "hs-files"; value: page.svc.links ? page.svc.links + (page.svc.links === 1 ? " link" : " links") : "" }
                SetLinkCard { target: "hs-people"; value: page.st && page.st.people && page.st.people.length ? page.st.people.length + (page.st.people.length === 1 ? " person" : " people") : "" }
                SetLinkCard { target: "hs-family"; sub: "A Family folder" + (page.st && page.st.seen && page.st.seen.camera && page.st.seen.camera.shown ? ", calendar and cameras" : " and calendar") }
            }
            SetCardGroup {
                title: "On this server"
                SetLinkCard { target: "hs-services"; value: page.servicesBig().good ? page.servicesBig().big : ""
                    sub: "Media server, files on the web, AI pictures and voices" + (page.aiShown ? ", AI chat" : "") }
                SetLinkCard { target: "hs-gaming" }
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            Layout.alignment: Qt.AlignTop
            spacing: 0
            SetCardGroup {
                title: "Keep it safe"
                SetLinkCard { target: "hs-drives"; chip: page.health().big === "Good" ? "Good" : page.health().bad ? "Check" : ""; chipKind: page.health().bad ? "warn" : "ok"
                    value: page.cfg && page.cfg.snapshots ? "Snapshots on" : "" }
                SetLinkCard { target: "hs-backups"; chip: page.backupsChip.chip || ""; chipKind: page.backupsChip.kind || "ok"; value: page.backupsChip.value || "" }
                SetLinkCard { target: "hs-safety"; chip: page.svc.guardTripped ? "Read only" : page.cfg && page.cfg.guard && page.cfg.guard.on ? "Watching" : ""; chipKind: page.svc.guardTripped ? "bad" : "ok" }
                SetLinkCard { target: "hs-alerts"; chip: page.alertList().length ? page.alertList().length + (page.alertList().length === 1 ? " alert" : " alerts") : ""; chipKind: "warn"
                    value: !page.alertList().length && page.cfg && page.cfg.alerts && page.cfg.alerts.on ? "All good" : "" }
            }
            SetCardGroup {
                title: "Reach & play"
                SetLinkCard { target: "hs-tailscale"; sub: "Lives in Network & internet now"; value: page.ts.state === "Running" ? "Connected" : page.ts.installed === false ? "Not installed" : "" }
                SetSwitch {
                    icon: "server"; title: "Light mode while you play"
                    sub: "Big jobs (backups, snapshots, emptying recycle bins) wait while a game runs. Sharing keeps working."
                         + (page.st && page.st.playing && page.cfg && page.cfg.lightMode ? " · On now: you're playing" : "")
                    checked: !!page.cfg && page.cfg.lightMode !== false
                    onToggled: v => page.set("lightMode", v ? "on" : "off")
                }
                SetLinkCard { target: "remote"; sub: "Play this PC's games on your laptop, phone or TV · Gaming › Remote play" }
            }
        }
    }

    SetHeader { text: "Hidden items"; visible: !!page.cfg; Layout.topMargin: 28 }
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

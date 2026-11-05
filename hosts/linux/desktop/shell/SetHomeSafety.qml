// Hearth Settings › Home server › Safety (docs/HOME-SERVER.md): who can reach what, what listens on this PC and where,
// Tailscale only, the last backup, Samba's SMB3 and no guests, and simple fixes: each says what it changes and can be
// undone. The ransomware guard (off until you turn it on): when far more files are renamed or deleted on your shares
// than you'd ever do, every share turns read only right away and you're told; "Make them writable again" undoes it.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Safety"
    property var sf: null
    property string msg: ""
    property bool bad: false
    readonly property var guard: sf ? sf.guard : null
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["safety"], r => page.sf = r, e => page.sf = { error: e, findings: [], reach: [], ports: [], done: [], smb: {}, guard: null }); }
    function where(w) { return w === "tailscale" ? "Tailscale only" : w === "local" ? "This PC only" : w === "home" ? "Home network and Tailscale" : "Every network"; }
    function doneFix(id) { return page.sf ? page.sf.done.find(d => d.id === id) : null; }
    Component.onCompleted: refresh()
    Timer { interval: 8000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: "Who can reach what on this PC, and simple fixes. Every fix says what it changes and can be undone."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ---------------------------------------------------------------- the guard tripped
    SetRow {
        visible: !!page.guard && !!page.guard.tripped
        icon: "lock"; danger: true; title: "Your shares are read only"
        sub: page.guard && page.guard.tripped ? page.guard.count + " files were renamed or deleted in 5 minutes at " + new Date(page.guard.tripped * 1000).toLocaleString(Qt.locale(), "d MMM h:mm AP")
             + ". Check your devices first. Snapshots (Drives & undo) bring back what changed" : ""
        chevron: false; interactive: false
    }
    SetRow {
        visible: !!page.guard && !!page.guard.tripped
        icon: "restart"; title: "Make them writable again"
        sub: "It was you (a big move or clean up), or it's dealt with"
        onActivated: act.run(["guard-reset"], () => { page.say("Your shares are writable again"); page.refresh(); }, e => page.say(e, true))
    }

    // ---------------------------------------------------------------- findings and fixes
    SetHeader { text: "Worth a look" }
    Repeater {
        model: page.sf ? page.sf.findings : []
        delegate: SetRow {
            required property var modelData
            icon: modelData.level === "warn" ? "bell" : "info"; danger: modelData.level === "warn" && !modelData.fix && !modelData.page
            title: modelData.title
            sub: modelData.sub + (modelData.fix ? " · Fix: " + modelData.fix : "")
            value: modelData.fix ? "Fix" : modelData.page ? "Open" : ""
            interactive: !!modelData.fix || !!modelData.page
            chevron: !!modelData.page
            onActivated: {
                if (modelData.page) SettingsNav.openPage(modelData.page);
                else if (modelData.fix) act.run(["fix", modelData.id], () => { page.say("Done: " + modelData.fix + ". Undo is below"); page.refresh(); }, e => page.say(e, true));
            }
        }
    }
    SetRow {
        visible: !!page.sf && page.sf.findings.length === 0
        icon: "check"; chevron: false; interactive: false; title: "Nothing to fix"
        sub: "Your shares, services and backups look right"
    }
    Repeater {
        model: page.sf ? page.sf.done : []
        delegate: SetRow {
            required property var modelData
            icon: "restart"; title: "Undo: " + ({ "ts-only": "Everything Tailscale only", "guard": "Ransomware guard on", "media-ts": "Media server Tailscale only", "snapshots": "Snapshots on" }[modelData.id] || modelData.id)
            sub: "Done " + new Date(modelData.at).toLocaleString(Qt.locale(), "d MMM h:mm AP") + " · puts back exactly what was there (" + modelData.steps.length + (modelData.steps.length === 1 ? " change)" : " changes)")
            onActivated: act.run(["fix-undo", modelData.id], () => { page.say("Undone"); page.refresh(); }, e => page.say(e, true))
        }
    }

    // ---------------------------------------------------------------- overview
    SetHeader { text: "At a glance" }
    SetRow {
        visible: !!page.sf
        icon: "globe"; chevron: false; interactive: false; title: page.sf && page.sf.tsOnly ? "Only reachable through Tailscale" : "Home network and Tailscale"
        sub: page.sf ? "Tailscale: " + (page.sf.tailscale === "Running" ? "connected" : (page.sf.tailscale || "off")) + (page.sf.firewall ? " · firewall rule: " + page.sf.firewall : "") : ""
    }
    SetRow {
        visible: !!page.sf
        icon: "update"; title: "Last backup"
        sub: page.sf ? (page.sf.lastBackup ? new Date(page.sf.lastBackup * 1000).toLocaleString(Qt.locale(), "d MMM h:mm AP") : page.sf.backups ? "None finished yet" : "No backup set up") : ""
        onActivated: SettingsNav.openPage("hs-backups")
    }
    SetRow {
        visible: !!page.sf
        icon: "lock"; chevron: false; interactive: false; title: "Shared folders: SMB3, no guests"
        sub: page.sf && page.sf.smb.known ? (page.sf.smb.smb3 ? "SMB3 or newer only" : "Older SMB allowed") + " · " + (page.sf.smb.noGuests ? "no guest access" : "guest access possible") : "Samba's settings couldn't be read"
    }
    SetRow {
        visible: !!page.sf
        icon: "restart"; chevron: false; interactive: false; title: "Snapshots"
        sub: page.sf && page.sf.snapshots ? "On: deleted, changed or encrypted files can be brought back" : "Off"
    }
    SetSwitch {
        visible: !!page.guard
        icon: "lock"; title: "Ransomware guard"
        sub: page.guard && page.guard.on ? "Watching: if " + page.guard.limit + " or more files are renamed or deleted on your shares within 5 minutes, they all turn read only and you're told"
             : "If far more files change at once than you'd ever do, your shares turn read only right away and you're told. Off until you turn it on"
        checked: !!page.guard && page.guard.on
        onToggled: v => act.run(["set", "guardOn", v ? "on" : "off"], () => page.refresh(), e => page.say(e, true))
    }
    SetChoice {
        visible: !!page.guard && page.guard.on
        title: "Turn shares read only after"
        sub: "Files renamed or deleted through Samba within 5 minutes"
        options: [{ value: 500, label: "500" }, { value: 1000, label: "1,000" }, { value: 5000, label: "5,000" }]
        current: page.guard ? page.guard.limit : 1000
        onChosen: v => act.run(["set", "guardLimit", String(v)], () => page.refresh(), e => page.say(e, true))
    }

    SetHeader { text: "Who can reach what" }
    Repeater {
        model: page.sf ? page.sf.reach : []
        delegate: SetRow {
            required property var modelData
            icon: modelData.where === "tailscale" ? "lock" : "wifi"; chevron: false; interactive: false
            title: modelData.what; sub: page.where(modelData.where) + " · " + modelData.who
        }
    }
    SetRow {
        visible: !!page.sf && page.sf.reach.length === 0
        icon: "check"; chevron: false; interactive: false; title: "Nothing is shared or served"; sub: ""
    }

    SetHeader { text: "Open ports" }
    Repeater {
        model: page.sf ? page.sf.ports : []
        delegate: SetRow {
            required property var modelData
            icon: modelData.where === "all" ? "wifi" : modelData.where === "tailscale" ? "lock" : "check"; chevron: false; interactive: false
            title: modelData.what; sub: modelData.proto.toUpperCase() + " " + modelData.port + " · " + page.where(modelData.where)
        }
    }

    SetHeader { text: "Updates for this PC" }
    SetRow {
        icon: "update"; chevron: false; interactive: false; title: "The previous version stays startable"
        sub: "Hearth OS updates always keep the version before them in the boot menu. Settings › Updates"
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

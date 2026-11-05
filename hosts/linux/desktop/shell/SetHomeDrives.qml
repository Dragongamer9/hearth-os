// Hearth Settings › Home server › Drives & undo (docs/HOME-SERVER.md): every drive's space, temperature and SMART
// health (smartctl, through the root helper), snapshots of your shares (btrfs: hourly, daily, weekly, with Browse &
// restore into a new folder), and adding a drive as more storage, a mirror (btrfs RAID1) or a backup target. The
// system drive is never shared, formatted or picked. Formatting asks twice, naming the drive and its size.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Drives & undo"
    property var st: null
    property var drv: null
    property string drvErr: ""
    property string snapShare: ""     // the share whose snapshots are listed
    property var snaps: null
    property string snapSel: ""
    property string pick: ""          // the drive being added (its path)
    property var plan: null
    property string use: "storage"
    property string target: ""
    property int step: 0              // 0 choosing, 1 first confirmation given, 2 done
    property string msg: ""
    property bool bad: false
    readonly property var cfg: st ? st.settings : null
    readonly property var newDrive: drv && pick ? (drv.drives.find(d => d.path === pick) || null) : null
    readonly property var addable: drv ? drv.drives.filter(d => !d.system && !d.mounts.length) : []
    readonly property var mirrorTargets: drv ? [].concat.apply([], drv.drives.filter(d => !d.system).map(d => d.mounts.filter(m => m.indexOf("/var/mnt/") === 0 || m.indexOf("/mnt/") === 0))) : []
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.st = { error: e }); }
    function loadDrives() { dj.run(["drives"], r => { page.drv = r; page.drvErr = ""; }, e => page.drvErr = e); }
    function set(k, v) { act.run(["set", k, v], () => page.refresh(), e => { page.say(e, true); page.refresh(); }); }
    function size(b) { return b >= 1e12 ? (b / 1e12).toFixed(1) + " TB" : b >= 1e9 ? Math.round(b / 1e9) + " GB" : Math.round((b || 0) / 1e6) + " MB"; }
    function when(ms) {
        const d = new Date(ms), now = new Date();
        const day = d.toDateString() === now.toDateString() ? "Today" : new Date(now.getTime() - 86400000).toDateString() === d.toDateString() ? "Yesterday" : d.toLocaleDateString();
        return day + " " + d.toLocaleTimeString(Qt.locale(), "h:mm AP");
    }
    function listSnaps(id) {
        page.snapShare = id; page.snaps = null; page.snapSel = "";
        sj.run(["snapshots", id], r => page.snaps = r, e => page.snaps = { ok: false, why: e, snapshots: [] });
    }
    function choose(path) {
        page.pick = path; page.plan = null; page.step = 0; page.say("");
        pj.run(["format-plan", path], r => page.plan = r, e => page.say(e, true));
    }
    function request(dry) {
        return JSON.stringify({ device: page.pick, use: page.use, label: label.text.trim() || "Storage", token: page.plan ? page.plan.token : "",
                                confirm1: page.step >= 1, confirm2: page.plan ? page.plan.confirm : "", target: page.target, dryRun: dry });
    }
    Component.onCompleted: { refresh(); loadDrives(); }
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }
    SetJob { id: dj; helper: "hearth-home-server" }
    SetJob { id: sj; helper: "hearth-home-server" }
    SetJob { id: pj; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: "Space, health and temperature for every drive, and snapshots to bring back deleted or changed files."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Drives" }
    Text {
        visible: !page.drv && page.drvErr === ""
        text: "Looking at your drives…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
    Text {
        visible: page.drvErr !== ""
        Layout.fillWidth: true
        text: "Couldn't read the drives: " + page.drvErr; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.drv ? page.drv.drives : []
        delegate: SetRow {
            required property var modelData
            icon: "store"; chevron: false
            title: modelData.model + (modelData.system ? " (system)" : "")
            value: !modelData.smart ? "No health data" : modelData.smart.passed === false ? "Failing: back it up" : modelData.smart.passed ? "Good" : "Unknown"
            danger: !!modelData.smart && modelData.smart.passed === false
            sub: [modelData.sizeText + " " + modelData.kind, modelData.smart && modelData.smart.temp ? modelData.smart.temp + " °C" : "",
                  modelData.total ? page.size(modelData.used) + " of " + page.size(modelData.total) + " used" : (modelData.empty ? "empty" : "not in use"),
                  modelData.system ? "Hearth OS lives here: never shared, formatted or picked" : modelData.mounts.join(", ")].filter(x => x).join(" · ")
        }
    }
    Text {
        visible: !!page.st && !!page.st.tools && !page.st.tools.smartctl
        Layout.fillWidth: true
        text: "Drive health (smartmontools) comes with the Hearth OS update that brings Home server."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Snapshots (undo)" }
    SetSwitch {
        visible: !!page.cfg
        icon: "restart"; title: "Snapshots"
        sub: "Read-only copies of your shares' drives, taken every hour, day and week, to bring back what was deleted or changed. Only on btrfs drives; with Light mode they wait while you play."
        checked: !!page.cfg && page.cfg.snapshots
        onToggled: v => page.set("snapshots", v ? "on" : "off")
    }
    SetChoice {
        visible: !!page.cfg && page.cfg.snapshots
        title: "Keep hourly"
        options: [{ value: 0, label: "None" }, { value: 12, label: "12" }, { value: 24, label: "24" }, { value: 48, label: "48" }]
        current: page.cfg ? page.cfg.keep.hourly : 24
        onChosen: v => page.set("keepHourly", String(v))
    }
    SetChoice {
        visible: !!page.cfg && page.cfg.snapshots
        title: "Keep daily"
        options: [{ value: 0, label: "None" }, { value: 7, label: "7" }, { value: 14, label: "14" }, { value: 30, label: "30" }]
        current: page.cfg ? page.cfg.keep.daily : 14
        onChosen: v => page.set("keepDaily", String(v))
    }
    SetChoice {
        visible: !!page.cfg && page.cfg.snapshots
        title: "Keep weekly"
        options: [{ value: 0, label: "None" }, { value: 4, label: "4" }, { value: 8, label: "8" }, { value: 12, label: "12" }]
        current: page.cfg ? page.cfg.keep.weekly : 8
        onChosen: v => page.set("keepWeekly", String(v))
    }
    Repeater {
        model: page.st ? page.st.shares : []
        delegate: SetSwitch {
            required property var modelData
            icon: "folder"; title: "Take snapshots of " + modelData.name
            sub: modelData.snapOk ? modelData.path : modelData.snapWhy
            checked: !!modelData.snapshots
            interactive: modelData.snapOk || !!modelData.snapshots
            onToggled: v => act.run(["share-set", modelData.id, "snapshots", v ? "on" : "off"], () => page.refresh(), e => { page.say(e, true); page.refresh(); })
        }
    }
    Text {
        visible: !!page.st && page.st.shares.length === 0
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "Share a folder first (Shared folders): snapshots are taken of the drives your shares are on."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.st ? page.st.shares.filter(s => s.snapshots) : []
        delegate: SetRow {
            required property var modelData
            icon: "clock"; title: "Browse & restore " + modelData.name
            sub: page.snapShare === modelData.id ? (page.snaps ? (page.snaps.ok ? page.snaps.snapshots.length + " snapshots, newest first" : page.snaps.why) : "Looking…") : "See its snapshots"
            onActivated: page.listSnaps(modelData.id)
        }
    }
    Repeater {
        model: page.snaps && page.snaps.ok ? page.snaps.snapshots.slice(0, 24) : []
        delegate: SetRow {
            required property var modelData
            icon: "clock"; title: page.when(modelData.at); value: modelData.kind
            sub: page.snapSel === modelData.name ? "Below: browse it, or restore it into a new folder" : "Select to browse or restore"
            onActivated: page.snapSel = page.snapSel === modelData.name ? "" : modelData.name
        }
    }
    SetRow {
        visible: page.snapSel !== ""
        icon: "open"; title: "Browse this snapshot"
        sub: "Opens it in Hearth Files (read only): copy back what you need"
        onActivated: {
            const s = page.snaps.snapshots.find(x => x.name === page.snapSel);
            if (s) Quickshell.execDetached(["python3", decodeURIComponent(Qt.resolvedUrl("hearth-portal").toString().replace(/^file:\/\//, "")), "show", s.path]);
        }
    }
    SetRow {
        visible: page.snapSel !== ""
        icon: "restart"; title: "Restore into a new folder"
        sub: "Copies the folder as it was into Home › Restored from snapshots. Nothing of yours is overwritten."
        onActivated: act.run(["restore", page.snapShare, page.snapSel], r => page.say("Restoring into " + r.dest + " (you'll get a notification when it's done)"), e => page.say(e, true))
    }

    SetHeader { text: "Add a drive" }
    Text {
        visible: !!page.drv && page.addable.length === 0
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "No drive to add right now. Plug one in (a drive that's in use has to be ejected in Hearth Files first). The system drive can never be picked."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.addable
        delegate: SetRow {
            required property var modelData
            icon: "plus"; title: modelData.model + " · " + modelData.sizeText + (modelData.empty ? " (empty)" : "")
            sub: page.pick === modelData.path ? "Set it up below" : "Use it as more storage, a mirror or a backup target"
            onActivated: page.choose(modelData.path)
        }
    }
    SetChoice {
        visible: page.pick !== "" && page.step === 0
        title: "Use it as"
        sub: page.use === "mirror" ? "A second copy of a drive: if one breaks, nothing is lost (btrfs RAID1)" : page.use === "backup" ? "Where backups go (Backups & sync, next update)" : "A new drive for your shares and games"
        options: [{ value: "storage", label: "More storage" }, { value: "mirror", label: "Mirror", disabled: page.mirrorTargets.length === 0 }, { value: "backup", label: "Backup target" }]
        current: page.use
        onChosen: v => { page.use = v; if (v === "mirror" && page.mirrorTargets.length) page.target = page.mirrorTargets[0]; }
    }
    SetChoice {
        visible: page.pick !== "" && page.step === 0 && page.use === "mirror"
        title: "Mirror of"
        options: page.mirrorTargets.map(m => ({ value: m, label: m.replace(/^\/(var\/)?mnt\//, "") }))
        current: page.target
        onChosen: v => page.target = v
    }
    SetField {
        id: label
        visible: page.pick !== "" && page.step === 0 && page.use !== "mirror"
        placeholder: "Its name (letters, digits, - or _), like Storage"
    }
    SetRow {
        visible: page.pick !== "" && !!page.plan && !page.plan.canFormat
        icon: "info"; chevron: false; interactive: false; title: "Can't use this drive"
        sub: page.plan ? page.plan.why : ""
    }
    Rectangle {
        visible: page.pick !== "" && !!page.plan && page.plan.canFormat && page.step < 2
        Layout.fillWidth: true
        implicitHeight: warn.implicitHeight + 24
        radius: Theme.r(12)
        color: Qt.rgba(Theme.flame.r, Theme.flame.g, Theme.flame.b, 0.12)
        Text {
            id: warn
            anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 14 }
            text: page.plan ? (page.use === "mirror" ? "Setting it up erases everything on " : "Formatting erases everything on ") + page.plan.confirm + ". Hearth asks twice. The system drive can never be picked." : ""
            color: Theme.text; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
    SetRow {
        visible: page.pick !== "" && !!page.plan && page.plan.canFormat && page.step === 0
        icon: "check"; title: "Set up this drive…"
        sub: "First of two confirmations"
        onActivated: page.step = 1
    }
    SetRow {
        visible: page.pick !== "" && !!page.plan && page.plan.canFormat && page.step === 0
        icon: "info"; title: "Show what it would do"
        sub: "A dry run: nothing is changed"
        onActivated: act.run(["format"], r => page.say("It would run: " + r.steps.map(s => s.join(" ")).join("; ")), e => page.say(e, true), page.request(true))
    }
    SetRow {
        visible: page.pick !== "" && !!page.plan && page.plan.canFormat && page.step === 1
        icon: "trash"; danger: true
        title: "Erase " + (page.plan ? page.plan.confirm : "") + " now"
        sub: "Second confirmation. Everything on this drive is deleted."
        onActivated: act.run(["format"], r => { page.step = 2; page.say(page.use === "mirror" ? "Mirroring started: it copies in the background at low priority" : "Ready ✓ at " + r.mount); page.loadDrives(); },
                             e => { page.step = 0; page.say(e, true); }, page.request(false))
    }
    SetRow {
        visible: page.pick !== "" && page.step < 2
        icon: "close"; title: "Not now"; chevron: false
        onActivated: { page.pick = ""; page.plan = null; page.step = 0; }
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

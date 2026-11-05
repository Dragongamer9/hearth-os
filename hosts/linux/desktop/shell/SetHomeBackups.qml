// Hearth Settings › Home server › Backups & sync (docs/HOME-SERVER.md): backups of the shares you pick (restic,
// encrypted with your password) to a backup drive, another Hearth device over Tailscale, or the cloud (Backblaze B2 or
// S3-compatible, through rclone); each with how often, how many to keep, Back up now, the last run and Restore (always
// into a new folder, never over your files). Backups from your other Hearth devices, phone photos, and folders kept the
// same between Hearth devices (Syncthing, over Tailscale). Light mode makes backups wait while you play. Passwords and
// keys go straight to the root helper's own 0600 files, never into home-server.json.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Backups & sync"
    property var st: null
    property var targets: null
    property var sync: null
    property var cands: []
    property string sel: ""              // the backup whose settings are open
    property bool adding: false
    property var newTarget: null         // {kind, path|peer, label}
    property string provider: "b2"
    property var newShares: []
    property string newEvery: "daily"
    property var snaps: null             // the open backup's backups (restic snapshots)
    property string snapSel: ""
    property string removing: ""
    property bool pwOpen: false
    property string code: ""             // a backup code just made (shown once)
    property bool receivePick: false
    property bool syncAdding: false
    property string syncPath: ""
    property string syncPeer: ""
    property string msg: ""
    property bool bad: false
    readonly property var cfg: st ? st.settings : null
    readonly property var jobs: st ? (st.backups || []) : []
    readonly property var cur: sel ? (jobs.find(j => j.id === sel) || null) : null
    readonly property var rc: cfg ? cfg.receive : null
    readonly property var peers: st && st.tailscale ? (st.tailscale.peers || []) : []
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.st = { error: e }); }
    function run(args, ok) { act.run(args, () => { if (ok) page.say(ok); page.refresh(); }, e => { page.say(e, true); page.refresh(); }); }
    function size(b) { return b >= 1e12 ? (b / 1e12).toFixed(1) + " TB" : b >= 1e9 ? Math.round(b / 1e9) + " GB" : b >= 1e6 ? Math.round(b / 1e6) + " MB" : Math.round((b || 0) / 1e3) + " KB"; }
    function ago(s) {
        if (!s) return "never";
        const m = Math.round((Date.now() / 1000 - s) / 60);
        return m < 2 ? "just now" : m < 60 ? m + " min ago" : m < 1440 ? Math.round(m / 60) + " h ago" : Math.round(m / 1440) + " days ago";
    }
    function shareNames(ids) {
        const all = st ? st.shares : [];
        return (ids || []).map(i => { const s = all.find(x => x.id === i); return s ? s.name : null; }).filter(n => n).join(", ") || "no folders";
    }
    function whereText(t) {
        if (!t) return "";
        if (t.kind === "drive") return t.path;
        if (t.kind === "hearth") return t.peer + " (Hearth, over Tailscale)";
        return (t.provider === "b2" ? "Backblaze B2" : t.remote ? "rclone " + t.remote : "S3") + ": " + t.path;
    }
    function everyText(j) { return j.every === "hourly" ? "every hour" : j.every === "weekly" ? "weekly at " + j.at : "daily at " + j.at; }
    function stateText(j) {
        const s = j.state || {};
        if (s.running) return "Backing up now…";
        if (s.waiting) return "Waiting for your game";
        if (s.last && s.last.ok === false) return "Didn't finish";
        if (s.lastOk) return "✓ " + ago(s.lastOk);
        return j.on === false ? "Off" : "Not yet";
    }
    function lastLine(j) {
        const s = j.state || {};
        if (s.last && s.last.ok === false) return "Last try " + ago(s.last.at) + ": " + (s.last.error || "it didn't finish");
        if (!s.lastOk) return "No backup yet" + (s.waiting ? ": " + s.waiting : "");
        const l = s.last || {};
        return "Last backup " + ago(s.lastOk) + (l.files ? " · " + l.files + " files, " + page.size(l.bytes) : "") + (s.count ? " · " + s.count + " kept" : "")
             + (s.checked ? " · checked " + ago(s.checked.at) + (s.checked.ok ? " (every file readable)" : " (found a problem)") : "");
    }
    function startAdd() {
        page.adding = !page.adding; page.newTarget = null; page.newShares = st ? st.shares.map(s => s.id) : [];
        if (page.adding && !page.targets) tj.run(["backup-targets"], r => page.targets = r, () => page.targets = { drives: [], devices: [] });
    }
    function add(pw) {
        if (!page.newTarget) { page.say("Pick where it goes first", true); return; }
        if (pw.length < 8) { page.say("The password needs at least 8 characters", true); return; }
        const t = page.newTarget;
        const req = { name: (t.kind === "drive" ? "To backup drive" : t.kind === "hearth" ? "To " + t.label : "To the cloud").replace(/[^A-Za-z0-9 _-]/g, "").slice(0, 40),
                      shares: page.newShares, every: page.newEvery, at: "03:00", password: pw,
                      target: t.kind === "cloud" ? { kind: "cloud", provider: page.provider, path: bucket.text.trim() } : t.kind === "drive" ? { kind: "drive", path: t.path } : { kind: "hearth", peer: t.peer } };
        if (t.kind === "hearth") req.code = codeField.text.trim();
        if (t.kind === "cloud") req.cloud = { access: keyId.text.trim(), secret: keySecret.text.trim(), endpoint: endpoint.text.trim(), region: "" };
        act.run(["backup-add"], r => { page.adding = false; page.sel = r.backup.id; page.say("Backup added ✓ It runs " + page.everyText(r.backup) + ". Keep your password somewhere safe: restoring on another PC needs it"); page.refresh(); },
                e => { page.say(e, true); page.refresh(); }, JSON.stringify(req));
    }
    function listSnaps() {
        page.snaps = null; page.snapSel = "";
        sj.run(["backup-snapshots", page.sel], r => page.snaps = r.snapshots || [], e => { page.snaps = []; page.say(e, true); });
    }
    function loadSync() { syj.run(["sync-status"], r => page.sync = r, () => page.sync = null); }
    Component.onCompleted: { refresh(); loadSync(); cj.run(["candidates"], r => page.cands = r.candidates || [], () => page.cands = []); }
    Timer { interval: 6000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }
    SetJob { id: tj; helper: "hearth-home-server" }
    SetJob { id: sj; helper: "hearth-home-server" }
    SetJob { id: cj; helper: "hearth-home-server" }
    SetJob { id: syj; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: "Back up your shared folders to a backup drive, another Hearth device or the cloud, encrypted with your password. Restoring always goes to a new folder: nothing is ever written over your files."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && !!page.st.tools && !page.st.tools.restic
        Layout.fillWidth: true
        text: "restic (backups) isn't on this PC yet: it comes with the Hearth OS update that brings it. What you set up here is kept."
        color: Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.cfg && page.cfg.lightMode !== false && !!page.st && page.st.playing
        Layout.fillWidth: true
        text: "You're playing: backups wait until you stop (Light mode, on the Overview page)."
        color: Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ---------------------------------------------------------------- your backups
    SetHeader { text: "Back up this PC" }
    Repeater {
        model: page.jobs
        delegate: SetRow {
            required property var modelData
            icon: "update"; title: modelData.name
            sub: page.shareNames(modelData.shares) + " → " + page.whereText(modelData.target) + " · " + page.everyText(modelData)
                 + (modelData.on === false ? " · off" : "") + "\n" + page.lastLine(modelData)
            value: page.sel === modelData.id ? "Open below" : page.stateText(modelData)
            onActivated: { page.sel = page.sel === modelData.id ? "" : modelData.id; page.snaps = null; page.removing = ""; page.pwOpen = false; }
        }
    }
    Text {
        visible: !!page.st && page.jobs.length === 0
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "No backups yet. A second copy somewhere else keeps your files safe from a broken drive, a deleted folder or ransomware."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // The open backup
    SetHeader { text: page.cur ? "“" + page.cur.name + "”" : ""; visible: !!page.cur }
    SetSwitch {
        visible: !!page.cur
        icon: "update"; title: "On"
        sub: page.cur ? "Runs " + page.everyText(page.cur) + " (when the PC is on; with Light mode, after you stop playing)" : ""
        checked: !!page.cur && page.cur.on !== false
        onToggled: v => page.run(["backup-set", page.sel, "on", v ? "on" : "off"])
    }
    SetChoice {
        visible: !!page.cur
        title: "How often"
        options: [{ value: "hourly", label: "Every hour" }, { value: "daily", label: "Daily" }, { value: "weekly", label: "Weekly (Sunday)" }]
        current: page.cur ? page.cur.every : "daily"
        onChosen: v => page.run(["backup-set", page.sel, "every", v])
    }
    SetChoice {
        visible: !!page.cur && page.cur.every !== "hourly"
        title: "At"
        options: [{ value: "01:00", label: "1:00" }, { value: "03:00", label: "3:00" }, { value: "05:00", label: "5:00" }, { value: "13:00", label: "13:00" }, { value: "22:00", label: "22:00" }]
        current: page.cur ? page.cur.at : "03:00"
        onChosen: v => page.run(["backup-set", page.sel, "at", v])
    }
    SetChoice {
        visible: !!page.cur
        title: "Keep"
        sub: "Older backups are thinned out: the newest each day, week and month are kept"
        options: [{ value: 7, label: "7 days" }, { value: 14, label: "14 days" }, { value: 30, label: "30 days" }]
        current: page.cur && page.cur.keep ? page.cur.keep.daily : 7
        onChosen: v => page.run(["backup-set", page.sel, "keepDaily", String(v)])
    }
    Repeater {
        model: page.cur && page.st ? page.st.shares : []
        delegate: SetSwitch {
            required property var modelData
            icon: "folder"; title: modelData.name; sub: modelData.path
            checked: !!page.cur && (page.cur.shares || []).indexOf(modelData.id) >= 0
            onToggled: v => page.run(["backup-share", page.sel, modelData.id, v ? "on" : "off"])
        }
    }
    SetRow {
        visible: !!page.cur
        icon: "forward"; title: page.cur && page.cur.state && page.cur.state.running ? "Backing up now…" : "Back up now"
        sub: page.cur ? page.lastLine(page.cur) : ""
        onActivated: page.run(["backup-now", page.sel], page.cfg && page.cfg.lightMode !== false && page.st.playing ? "It runs once you stop playing (Light mode)" : "Backing up ✓ It runs in the background")
    }
    SetRow {
        visible: !!page.cur
        icon: "restart"; title: "Restore…"
        sub: "Pick a backup, then a folder: it's copied into Home › Restored from backups, never over your files"
        onActivated: page.listSnaps()
    }
    Text {
        visible: !!page.cur && page.snaps === null && sj.busy
        text: "Reading the backup…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
    Repeater {
        model: page.cur && page.snaps ? page.snaps.slice(0, 12) : []
        delegate: SetRow {
            required property var modelData
            icon: "clock"; title: new Date(modelData.time).toLocaleString(Qt.locale(), "ddd d MMM yyyy, h:mm AP")
            sub: modelData.paths.length + (modelData.paths.length === 1 ? " folder" : " folders") + " · from " + modelData.host
            value: page.snapSel === modelData.id ? "Pick a folder below" : ""
            onActivated: page.snapSel = page.snapSel === modelData.id ? "" : modelData.id
        }
    }
    Repeater {
        model: page.cur && page.snaps && page.snapSel ? ((page.snaps.find(s => s.id === page.snapSel) || {}).paths || []) : []
        delegate: SetRow {
            required property var modelData
            icon: "folder"; title: "Restore " + modelData.replace(/^.*\//, "")
            sub: modelData + " → a new folder in Home › Restored from backups"
            onActivated: act.run(["backup-restore", page.sel, page.snapSel, modelData], r => page.say("Restoring into " + r.dest + " (a notification says when it's done)"), e => page.say(e, true))
        }
    }
    SetRow {
        visible: !!page.cur
        icon: "lock"; title: "Change its password"
        sub: "The new password is used from the next backup on (restic keeps the old one working too)"
        onActivated: { page.pwOpen = !page.pwOpen; if (page.pwOpen) jobPw.focusInput(); }
    }
    SetField {
        id: jobPw
        visible: !!page.cur && page.pwOpen
        secret: true
        placeholder: "Its password (at least 8 characters), then Enter"
        onSubmitted: t => { act.run(["backup-password", page.sel], () => { page.say("Password saved ✓"); page.pwOpen = false; }, e => page.say(e, true), t); text = ""; }
    }
    SetRow {
        visible: !!page.cur && page.removing !== page.sel
        icon: "close"; title: "Remove this backup"; chevron: false
        onActivated: page.removing = page.sel
    }
    SetRow {
        visible: !!page.cur && page.removing === page.sel
        icon: "close"; danger: true; title: "Remove “" + (page.cur ? page.cur.name : "") + "”"
        sub: "It stops running, and its password and keys are forgotten on this PC. What it backed up stays where it is."
        onActivated: { const id = page.sel; page.sel = ""; page.removing = ""; page.run(["backup-remove", id], "Removed (the backup itself is still where it went)"); }
    }

    // Adding one
    SetRow {
        icon: "plus"; title: "Add a backup"
        sub: "To a backup drive, another Hearth device on Tailscale, or the cloud (Backblaze B2 or S3)"
        onActivated: page.startAdd()
    }
    Text {
        visible: page.adding
        Layout.fillWidth: true; Layout.leftMargin: 4; Layout.topMargin: 4
        text: "1. Where it goes"
        color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold
    }
    Repeater {
        model: page.adding && page.targets ? page.targets.drives : []
        delegate: SetRow {
            required property var modelData
            icon: "store"; title: modelData.label
            sub: modelData.path + (modelData.free > 0 ? " · " + page.size(modelData.free) + " free" : "")
            value: page.newTarget && page.newTarget.path === modelData.path ? "✓" : ""
            onActivated: page.newTarget = modelData
        }
    }
    Text {
        visible: page.adding && !!page.targets && page.targets.drives.length === 0
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "No backup drive yet: add one in Drives & undo (Add a drive › Backup target), or plug one in."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.adding && page.targets ? page.targets.devices : []
        delegate: SetRow {
            required property var modelData
            icon: "desktops"; title: modelData.label + (modelData.online ? "" : " (offline)")
            sub: "Another Hearth device, over Tailscale. Turn on “Backups from your other Hearth devices” there and make a code for this PC"
            value: page.newTarget && page.newTarget.peer === modelData.peer ? "✓" : ""
            onActivated: page.newTarget = modelData
        }
    }
    SetRow {
        visible: page.adding
        icon: "globe"; title: "The cloud"
        sub: "Backblaze B2 or any S3-compatible storage, with your own account. Encrypted here before it leaves"
        value: page.newTarget && page.newTarget.kind === "cloud" ? "✓" : ""
        onActivated: page.newTarget = { kind: "cloud", label: "The cloud" }
    }
    SetField {
        id: codeField
        visible: page.adding && !!page.newTarget && page.newTarget.kind === "hearth"
        placeholder: "The backup code it shows (like ally-3f9a:ABCD-EFGH-JKLM-NPQR)"
    }
    SetChoice {
        visible: page.adding && !!page.newTarget && page.newTarget.kind === "cloud"
        title: "Cloud storage"
        options: [{ value: "b2", label: "Backblaze B2" }, { value: "s3", label: "S3-compatible" }]
        current: page.provider
        onChosen: v => page.provider = v
    }
    SetField { id: bucket; visible: page.adding && !!page.newTarget && page.newTarget.kind === "cloud"; placeholder: "Bucket and folder, like my-bucket/hearth" }
    SetField { id: endpoint; visible: page.adding && !!page.newTarget && page.newTarget.kind === "cloud" && page.provider === "s3"; placeholder: "Endpoint, like s3.eu-central-003.backblazeb2.com (empty for Amazon)" }
    SetField { id: keyId; visible: page.adding && !!page.newTarget && page.newTarget.kind === "cloud"; placeholder: page.provider === "b2" ? "Key ID" : "Access key" }
    SetField { id: keySecret; visible: page.adding && !!page.newTarget && page.newTarget.kind === "cloud"; secret: true; placeholder: page.provider === "b2" ? "Application key (kept only in a root-only file)" : "Secret key (kept only in a root-only file)" }
    Text {
        visible: page.adding
        Layout.fillWidth: true; Layout.leftMargin: 4; Layout.topMargin: 6
        text: "2. What and how often"
        color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold
    }
    Repeater {
        model: page.adding && page.st ? page.st.shares : []
        delegate: SetSwitch {
            required property var modelData
            icon: "folder"; title: modelData.name; sub: modelData.path
            checked: page.newShares.indexOf(modelData.id) >= 0
            onToggled: v => page.newShares = v ? page.newShares.concat([modelData.id]) : page.newShares.filter(i => i !== modelData.id)
        }
    }
    Text {
        visible: page.adding && !!page.st && page.st.shares.length === 0
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "Share a folder first (Shared folders): backups are of your shared folders."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    SetChoice {
        visible: page.adding
        title: "How often"
        options: [{ value: "hourly", label: "Every hour" }, { value: "daily", label: "Daily at 3:00" }, { value: "weekly", label: "Weekly" }]
        current: page.newEvery
        onChosen: v => page.newEvery = v
    }
    Text {
        visible: page.adding
        Layout.fillWidth: true; Layout.leftMargin: 4; Layout.topMargin: 6
        text: "3. A password for it. Write it down: restoring on another PC needs it, and nobody can get it back for you."
        color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold; wrapMode: Text.WordWrap
    }
    SetField {
        id: newPw
        visible: page.adding
        secret: true
        placeholder: "Backup password (at least 8 characters), then Enter to add it"
        onSubmitted: t => { page.add(t); text = ""; }
    }

    // ---------------------------------------------------------------- devices backing up here
    SetHeader { text: "Devices backing up here" }
    SetRow {
        icon: "desktops"; title: "Your Hearth devices' saves and screenshots"
        sub: "They back up to this PC through Hearth Storage (Hearth Link)"
        onActivated: SettingsNav.openPage("link")
    }
    SetSwitch {
        visible: !!page.cfg
        icon: "update"; title: "Backups from your other Hearth devices"
        sub: page.rc && page.rc.on ? "On, over Tailscale only, into " + (page.rc.path || "the folder below") + ". Each device has its own code and its own folder"
             : "Lets your other Hearth devices back up their shares here, over Tailscale only (with a code each)"
        checked: !!page.rc && page.rc.on
        interactive: !!page.rc && (page.rc.on || page.rc.path !== "")
        onToggled: v => page.run(["set", "receiveOn", v ? "on" : "off"], v ? "On: make a code for each device below" : "Off (what they backed up stays)")
    }
    SetRow {
        visible: !!page.cfg
        icon: "folder"; title: "Where they go"
        value: page.rc && page.rc.path ? page.rc.path : "Pick a folder"
        sub: "A folder on a big drive is best (each device gets its own folder inside)"
        onActivated: page.receivePick = !page.receivePick
    }
    Repeater {
        model: page.receivePick ? page.cands.slice(0, 8) : []
        delegate: SetRow {
            required property var modelData
            icon: modelData.drive ? "store" : "folder"; title: modelData.name; sub: modelData.path
            onActivated: { page.receivePick = false; page.run(["receive-path", modelData.path], "Backups from your devices go to " + modelData.path); }
        }
    }
    Repeater {
        model: page.rc ? (page.rc.devices || []) : []
        delegate: SetRow {
            required property var modelData
            icon: "desktops"; title: modelData.name; chevron: false
            sub: "Its code starts with " + modelData.user + " · made " + new Date(modelData.at).toLocaleDateString() + ". Select to take its code away (what it backed up stays)"
            onActivated: page.run(["receive-remove", modelData.user], modelData.name + " can't back up here anymore")
        }
    }
    SetField {
        visible: !!page.rc && page.rc.on
        placeholder: "Make a code for a device: type its name (like rog-ally), then Enter"
        onSubmitted: t => { if (!t.trim()) return; act.run(["receive-add", t.trim()], r => { page.code = r.code; page.refresh(); }, e => page.say(e, true)); text = ""; }
    }
    Rectangle {
        visible: page.code !== ""
        Layout.fillWidth: true
        implicitHeight: codeCol.implicitHeight + 28
        radius: Theme.r(12)
        color: Theme.panel2
        ColumnLayout {
            id: codeCol
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14 }
            spacing: 6
            Text { text: "Type this on the other device (Backups & sync › Add a backup › this PC). It's shown only now."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; Layout.fillWidth: true; wrapMode: Text.WordWrap }
            Text { text: page.code; color: Theme.text; font.family: "monospace"; font.pixelSize: 20; font.weight: Font.DemiBold; Layout.fillWidth: true; wrapMode: Text.WrapAnywhere }
        }
    }

    // ---------------------------------------------------------------- phone photos
    SetHeader { text: "Phone photos" }
    SetRow {
        icon: "cam"; chevron: false; interactive: false; title: "Android"
        sub: "An app that backs up to a network folder (SMB), like FolderSync or PhotoSync: point it at " + (page.st && page.st.shares.length ? page.st.shares[0].how.phone : "smb://this-pc/Photos")
             + " with your user name and sharing password (or a person's, People page). Or Syncthing-Fork, with folder sync below."
    }
    SetRow {
        icon: "cam"; chevron: false; interactive: false; title: "iPhone"
        sub: "PhotoSync (or any app that uploads to SMB): add this PC as an SMB server (" + (page.st ? page.st.host : "this-pc") + "), pick a read & write share, and turn on automatic transfer. Away from home it works through Tailscale too."
    }

    // ---------------------------------------------------------------- folder sync
    SetHeader { text: "Folders kept the same on your Hearth devices" }
    Text {
        visible: !!page.sync && !page.sync.installed
        Layout.fillWidth: true
        text: "Syncthing (folder sync) isn't on this PC yet: it comes with the Hearth OS update that brings it."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetRow {
        visible: !!page.sync && page.sync.installed
        icon: "info"; title: page.sync && page.sync.id ? "This PC's sync ID" : "Show this PC's sync ID"
        sub: page.sync && page.sync.id ? page.sync.id : "Your other device needs it to sync with this PC (this starts folder sync, which only talks over Tailscale)"
        onActivated: if (!page.sync.id) act.run(["sync-id"], () => page.loadSync(), e => page.say(e, true))
    }
    Repeater {
        model: page.sync ? page.sync.folders : []
        delegate: SetRow {
            required property var modelData
            icon: "restart"; title: modelData.label; chevron: false
            sub: modelData.path + " ↔ " + (modelData.peer || (modelData.device || "").slice(0, 7)) + (modelData.connected ? " · connected" : " · not connected")
                 + ". Select to stop syncing (the files stay on both)"
            value: modelData.completion === undefined || modelData.completion === null ? "" : modelData.completion >= 100 ? "Up to date" : "Syncing " + modelData.need + " files"
            onActivated: act.run(["sync-remove", modelData.id], () => { page.say("Not synced anymore (the files stay)"); page.loadSync(); }, e => page.say(e, true))
        }
    }
    Repeater {
        model: page.sync ? page.sync.offers : []
        delegate: SetRow {
            required property var modelData
            icon: "plus"; title: "Accept “" + modelData.label + "”"
            sub: "Offered by " + (modelData.device || "").slice(0, 7) + "…: it's kept in Home › Synced › " + modelData.label
            onActivated: act.run(["sync-accept", modelData.id, modelData.device], () => { page.say("Syncing " + modelData.label); page.loadSync(); }, e => page.say(e, true))
        }
    }
    SetRow {
        visible: !!page.sync && page.sync.installed
        icon: "plus"; title: "Sync a folder"
        sub: "Pick a folder, one of your devices on Tailscale and its sync ID. Changed or deleted files are kept 30 days first"
        onActivated: { page.syncAdding = !page.syncAdding; page.syncPath = ""; page.syncPeer = ""; }
    }
    Repeater {
        model: page.syncAdding ? page.cands.filter(c => !c.drive).slice(0, 8) : []
        delegate: SetRow {
            required property var modelData
            icon: "folder"; title: modelData.name; sub: modelData.path
            value: page.syncPath === modelData.path ? "✓" : ""
            onActivated: page.syncPath = modelData.path
        }
    }
    Repeater {
        model: page.syncAdding ? page.peers.filter(p => p.os === "linux") : []
        delegate: SetRow {
            required property var modelData
            icon: "desktops"; title: modelData.name + (modelData.online ? "" : " (offline)")
            value: page.syncPeer === modelData.name ? "✓" : ""
            onActivated: page.syncPeer = modelData.name
        }
    }
    SetField {
        visible: page.syncAdding
        placeholder: "Its sync ID (Backups & sync on that device), then Enter"
        onSubmitted: t => {
            if (!page.syncPath || !page.syncPeer) { page.say("Pick a folder and a device first", true); return; }
            act.run(["sync-add", page.syncPath, page.syncPeer, t.trim()], () => { page.syncAdding = false; page.say("Offered to " + page.syncPeer + ": accept it there"); page.loadSync(); }, e => page.say(e, true));
            text = "";
        }
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

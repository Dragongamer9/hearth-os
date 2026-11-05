// Hearth Settings › Home server › Shared folders (docs/HOME-SERVER.md): folders your Windows PCs, phones, TVs and
// Hearth devices can open (Samba: SMB3 or newer, no guests, your user and your sharing password). Nothing is shared
// until you share it. Each share: read only or read & write, home network or Tailscale only, a recycle bin, who may
// use it (you, and the people you let in on the People page). Stopping a share never touches its files.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Shared folders"
    property var st: null
    property var cands: []
    property string sel: ""          // the share whose settings are open
    property bool adding: false
    property bool pwOpen: false
    property string removing: ""
    property string msg: ""
    property bool bad: false
    readonly property var cfg: st ? st.settings : null
    readonly property var share: st && sel ? (st.shares.find(s => s.id === sel) || null) : null
    readonly property bool pwSet: !!cfg && cfg.smbPasswordSet > 0
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.st = { error: e }); }
    function run(args, ok) { act.run(args, () => { if (ok) page.say(ok); page.refresh(); }, e => { page.say(e, true); page.refresh(); }); }
    function add(path) {
        page.adding = false;
        act.run(["share-add", path], r => { page.say("Shared ✓ " + r.share.name + (page.pwSet ? "" : ". Set a sharing password below so your devices can sign in")); page.sel = r.share.id; page.refresh(); },
                e => { page.say(e, true); page.refresh(); });
    }
    Component.onCompleted: { refresh(); cj.run(["candidates"], r => page.cands = r.candidates || [], () => page.cands = []); }
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }
    SetJob { id: cj; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: "Folders your other devices can open: Windows (File Explorer), phones, TVs and Hearth devices. Nothing is shared until you share it, and nobody gets in without your sharing password."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && !!page.st.tools && !page.st.tools.samba
        Layout.fillWidth: true
        text: "Samba (sharing) isn't on this PC yet: it comes with the Hearth OS update that brings Home server. What you pick here is kept and starts working then."
        color: Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Your shares" }
    Repeater {
        model: page.st ? page.st.shares : []
        delegate: SetRow {
            required property var modelData
            icon: "folder"; title: modelData.name
            sub: modelData.path + " · " + (modelData.on === false ? "not shared now" : (modelData.write ? "read & write" : "read only") + " · "
                 + (modelData.reach === "tailscale" || (page.cfg && page.cfg.tsOnly) ? "Tailscale only" : "home network and Tailscale")
                 + (modelData.write && modelData.recycle !== false ? " · recycle bin" : ""))
                 + (modelData.there ? "" : " · its folder isn't there")
            value: page.sel === modelData.id ? "Open below" : (modelData.on === false ? "Off" : "On")
            onActivated: { page.sel = page.sel === modelData.id ? "" : modelData.id; page.removing = ""; }
        }
    }
    Text {
        visible: !!page.st && page.st.shares.length === 0
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "Nothing is shared yet."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
    SetRow {
        icon: "plus"; title: "Share a folder"
        sub: "A folder in your Home or on a drive (never the system drive itself). It starts read only, on your home network."
        onActivated: { page.adding = !page.adding; }
    }
    SetRow {
        visible: page.adding
        icon: "folder"; title: "Pick it in Hearth Files"
        onActivated: act.run(["pick-folder"], r => { if (r.path) page.add(r.path); }, e => page.say(e, true))
    }
    Repeater {
        model: page.adding ? page.cands.filter(c => !(page.st ? page.st.shares : []).some(s => s.path === c.path)).slice(0, 10) : []
        delegate: SetRow {
            required property var modelData
            icon: modelData.drive ? "store" : "folder"; title: modelData.name; sub: modelData.path
            onActivated: page.add(modelData.path)
        }
    }
    SetField {
        visible: page.adding
        placeholder: "Or type its full path, like /home/you/Videos"
        onSubmitted: t => { if (t.trim()) page.add(t.trim()); text = ""; }
    }

    // The open share's settings
    SetHeader { text: page.share ? "Share “" + page.share.name + "”" : ""; visible: !!page.share }
    SetSwitch {
        visible: !!page.share
        icon: "folder"; title: "Shared"
        sub: page.share ? page.share.path : ""
        checked: !!page.share && page.share.on !== false
        onToggled: v => page.run(["share-set", page.sel, "on", v ? "on" : "off"], v ? "Shared again" : "Not shared now (its files stay where they are)")
    }
    SetChoice {
        visible: !!page.share
        title: "Access"
        options: [{ value: false, label: "Read only" }, { value: true, label: "Read & write" }]
        current: !!page.share && !!page.share.write
        onChosen: v => page.run(["share-set", page.sel, "write", v ? "on" : "off"])
    }
    SetChoice {
        visible: !!page.share
        title: "Reachable"
        sub: page.cfg && page.cfg.tsOnly ? "“Only reachable through Tailscale” is on for everything (Tailscale page)" : "Tailscale only: your home network can't open it; your devices on Tailscale can, at home or away"
        options: [{ value: "home", label: "Home network", disabled: !!page.cfg && page.cfg.tsOnly }, { value: "tailscale", label: "Tailscale only" }]
        current: page.cfg && page.cfg.tsOnly ? "tailscale" : (page.share ? page.share.reach || "home" : "home")
        onChosen: v => page.run(["share-set", page.sel, "reach", v])
    }
    SetSwitch {
        visible: !!page.share
        icon: "trash"; title: "Recycle bin (30 days)"
        sub: page.share && page.share.write ? "Files deleted from another device go to the share's recycle bin first, kept 30 days" : "For shares your devices can change (read & write)"
        checked: !!page.share && page.share.recycle !== false
        onToggled: v => page.run(["share-set", page.sel, "recycle", v ? "on" : "off"])
    }
    SetRow {
        visible: !!page.share
        icon: "user"; title: "Who"
        value: page.share && page.share.people && Object.keys(page.share.people).length ? String(Object.keys(page.share.people).length + 1) + " people" : "Only you"
        sub: "You (" + (page.st ? page.st.user : "") + ")" + (page.share && page.share.people ? Object.keys(page.share.people).map(n => ", " + n + (page.share.people[n] === "read" || !page.share.write ? " (read only)" : "")).join("") : "")
             + ". Add people and pick what they may open on the People page"
        onActivated: SettingsNav.openPage("hs-people")
    }
    SetSwitch {
        visible: !!page.share
        icon: "restart"; title: "Snapshots"
        sub: page.share ? (page.share.snapOk ? "Undo for this folder: hourly, daily and weekly copies (Drives & undo)" : page.share.snapWhy) : ""
        checked: !!page.share && !!page.share.snapshots
        interactive: !!page.share && (page.share.snapOk || !!page.share.snapshots)
        onToggled: v => page.run(["share-set", page.sel, "snapshots", v ? "on" : "off"])
    }
    SetRow {
        visible: !!page.share && page.removing === ""
        icon: "close"; title: "Stop sharing it"; chevron: false
        onActivated: page.removing = page.sel
    }
    SetRow {
        visible: !!page.share && page.removing === page.sel
        icon: "close"; danger: true; title: "Stop sharing “" + (page.share ? page.share.name : "") + "”"
        sub: "Your devices can't open it anymore. Its files stay where they are: nothing is deleted."
        onActivated: { const id = page.sel; page.sel = ""; page.removing = ""; page.run(["share-remove", id], "Not shared anymore (its files are still there)"); }
    }

    SetHeader { text: "Sharing password" }
    SetRow {
        icon: "lock"; title: page.pwSet ? "Change your sharing password" : "Set your sharing password"
        sub: (page.pwSet ? "Set on " + new Date(page.cfg.smbPasswordSet).toLocaleDateString() + ". " : "Not set yet. ")
             + "Your devices ask for it with your user name (" + (page.st ? page.st.user : "") + "). Hearth never keeps it."
        onActivated: { page.pwOpen = !page.pwOpen; if (page.pwOpen) pw.focusInput(); }
    }
    SetField {
        id: pw
        visible: page.pwOpen
        secret: true
        placeholder: "At least 8 characters, then Enter"
        onSubmitted: t => {
            if (t.length < 8) { page.say("Use at least 8 characters", true); return; }
            act.run(["smb-password"], () => { page.say("Sharing password set ✓"); page.pwOpen = false; page.refresh(); }, e => page.say(e, true), t);
            text = "";
        }
    }

    SetHeader { text: "How to open them" }
    SetRow {
        icon: "display"; chevron: false; interactive: false; title: "Windows"
        sub: "File Explorer › This PC › Map network drive, then " + (page.st && page.st.shares.length ? page.st.shares[0].how.windows : "\\\\" + (page.st ? page.st.host : "this-pc") + "\\Name")
    }
    SetRow {
        icon: "pad"; chevron: false; interactive: false; title: "Phone or TV"
        sub: "Any Files app with network storage (SMB): " + (page.st && page.st.shares.length ? page.st.shares[0].how.phone : "smb://" + (page.st ? page.st.host : "this-pc") + "/Name")
    }
    SetRow {
        icon: "desktops"; chevron: false; interactive: false; title: "Hearth devices"
        sub: "Hearth Files › Network, or type " + (page.st && page.st.shares.length ? page.st.shares[0].how.mac : "smb://" + (page.st ? page.st.host : "this-pc") + "/Name")
    }
    SetRow {
        icon: "globe"; chevron: false; interactive: false; title: "Away from home"
        sub: page.st && page.st.shares.length && page.st.shares[0].how.away ? "The same through Tailscale: " + page.st.shares[0].how.away : "Sign in to Tailscale (Tailscale page) and the same names work from anywhere"
    }
    Text {
        Layout.fillWidth: true; Layout.topMargin: 4
        text: "Sign in with your user name and your sharing password. Hidden files (names starting with a dot) are never shown."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && !!page.st.applied && page.st.applied.ok === false
        Layout.fillWidth: true
        text: page.st && page.st.applied ? "Saved, but sharing couldn't be changed to match: " + (page.st.applied.error || "") : ""
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

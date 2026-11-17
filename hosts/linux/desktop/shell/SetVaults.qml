// Hearth Settings › Privacy & security › Vaults (1.14, like KDE's Plasma Vaults): encrypted folders you open with a
// password (hearth-vaults: gocryptfs, or CryFS when it's installed). Unlocked, a vault's files are in ~/Vaults/<name>;
// locked, they're only scrambled files. Each locks by itself after a while unused and (if you want) when the screen
// locks; the bar's vault button locks and opens them too. The password goes to gocryptfs/CryFS on their stdin only:
// Hearth never keeps it, so a forgotten password can't be brought back. Nothing here changes by itself.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Vaults"
    property var st: null
    property string picked: ""
    property string asking: ""          // the vault whose password is being typed
    property bool creating: false
    property string backend: "gocryptfs"
    property int idle: 15
    property bool lockOnLock: true
    property string confirm: ""
    property string message: ""
    property bool messageBad: false
    readonly property var vaults: st ? st.vaults : []

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["list"], r => page.st = r, e => page.say(e, true)); }
    // Settings opened at vaults:<id> (the bar's "Unlock…"): that vault's password field; vaults:new: the new vault form.
    Component.onCompleted: {
        const a = SettingsNav.pageAnchor;
        if (a === "new") { creating = true; SettingsNav.pageAnchor = ""; }
        else if (/^[a-z0-9-]+$/.test(a)) { picked = a; asking = a; SettingsNav.pageAnchor = ""; }
        load();
    }
    Component.onDestruction: { pw.text = ""; npw.text = ""; npw2.text = ""; }
    Timer { interval: 10000; repeat: true; running: page.visible && !job.busy; onTriggered: page.load() }
    SetJob { id: job; helper: "hearth-vaults" }
    function idleWord(m) { return m === 0 ? "Never by itself" : m < 60 ? m + " minutes unused" : (m / 60) + (m === 60 ? " hour unused" : " hours unused"); }

    SetCardGroup {
        title: "Your vaults"
        Repeater {
            model: page.vaults
            delegate: ColumnLayout {
                id: vr
                required property var modelData
                Layout.fillWidth: true
                spacing: 8
                SetRow {
                    icon: "vault"
                    title: vr.modelData.name
                    sub: (vr.modelData.open ? "Unlocked · " + vr.modelData.mount : "Locked") + " · " + (vr.modelData.backend === "cryfs" ? "CryFS" : "gocryptfs")
                    chip: vr.modelData.open ? "Open" : ""
                    onActivated: { page.picked = page.picked === vr.modelData.id ? "" : vr.modelData.id; page.confirm = ""; page.asking = ""; }
                }
                ColumnLayout {
                    visible: page.picked === vr.modelData.id
                    Layout.fillWidth: true
                    Layout.leftMargin: 52
                    spacing: 8
                    SetRow {
                        visible: !vr.modelData.open && page.asking !== vr.modelData.id
                        icon: "lock"; chevron: false; title: "Unlock…"
                        onActivated: { page.asking = vr.modelData.id; pw.focusInput(); }
                    }
                    SetField { id: pw; visible: page.asking === vr.modelData.id && !vr.modelData.open; secret: true; placeholder: "The vault's password"; onSubmitted: goBtn.activated() }
                    SetRow {
                        id: goBtn
                        visible: page.asking === vr.modelData.id && !vr.modelData.open
                        icon: "lock"; chevron: false; title: job.what === "unlock" ? "Unlocking…" : "Unlock"
                        onActivated: { job.run(["unlock", vr.modelData.id], r => { page.st = r; page.asking = ""; page.say(vr.modelData.name + " is unlocked: its files are in " + vr.modelData.mount + "."); }, e => page.say(e, true), JSON.stringify({ password: pw.text }), "unlock"); pw.text = ""; }
                    }
                    SetRow {
                        visible: vr.modelData.open
                        icon: "folder"; chevron: false; title: "Open in Files"
                        onActivated: Quickshell.execDetached(["xdg-open", vr.modelData.mount])
                    }
                    SetRow {
                        visible: vr.modelData.open
                        icon: "lock"; chevron: false; title: job.what === "lock" ? "Locking…" : "Lock now"
                        onActivated: job.run(["lock", vr.modelData.id], r => { page.st = r; page.say(vr.modelData.name + " is locked."); }, e => page.say(e, true), "", "lock")
                    }
                    SetChoice {
                        icon: "clock"; title: "Lock by itself"; sub: "After nobody used its files for a while (from the next unlock)"
                        options: [{ value: 5, label: "5 min" }, { value: 15, label: "15 min" }, { value: 60, label: "1 hour" }, { value: 0, label: "Never" }]
                        current: vr.modelData.idle
                        onChosen: v => job.run(["set", vr.modelData.id], r => { page.st = r; page.say("Saved: " + page.idleWord(v) + "."); }, e => page.say(e, true), JSON.stringify({ idle: v }), "set")
                    }
                    SetSwitch {
                        icon: "lock"; title: "Lock it when the screen locks"
                        checked: vr.modelData.lockOnLock
                        onToggled: v => job.run(["set", vr.modelData.id], r => page.st = r, e => page.say(e, true), JSON.stringify({ lockOnLock: v }), "set")
                    }
                    SetRow {
                        visible: !vr.modelData.open && page.confirm !== vr.modelData.id
                        icon: "trash"; danger: true; chevron: false; title: "Take it off the list…"
                        onActivated: page.confirm = vr.modelData.id
                    }
                    SetRow {
                        visible: page.confirm === vr.modelData.id
                        icon: "trash"; danger: true; chevron: false; title: "Yes, take " + vr.modelData.name + " off the list"
                        sub: "Its encrypted files stay in " + vr.modelData.data + " (nothing is deleted)"
                        onActivated: job.run(["forget", vr.modelData.id], r => { page.st = r; page.picked = ""; page.confirm = ""; page.say("Taken off the list. Its files are still in " + r.kept + "."); }, e => page.say(e, true), "", "forget")
                    }
                }
            }
        }
        Text {
            visible: !!page.st && page.vaults.length === 0
            Layout.fillWidth: true
            text: "No vaults yet. A vault is a folder only your password opens: for tax papers, passwords exported from a browser, anything private."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }

    SetCardGroup {
        title: "New vault"
        SetRow {
            visible: !page.creating
            icon: "plus"; chevron: false; title: "Make a vault…"
            sub: page.st && page.st.backends.length === 0 ? "Needs gocryptfs or CryFS, which aren't on this system" : "It shows in " + (page.st ? page.st.folder : "~/Vaults") + " while it's unlocked"
            onActivated: page.creating = true
        }
        ColumnLayout {
            visible: page.creating
            Layout.fillWidth: true
            spacing: 8
            SetField { id: nname; placeholder: "Its name, like Private" }
            SetField { id: npw; secret: true; placeholder: "A password (6 letters or more). It can't be brought back if you forget it" }
            SetField { id: npw2; secret: true; placeholder: "The password again" }
            SetChoice {
                icon: "vault"; title: "Kind"
                sub: page.backend === "cryfs" ? "CryFS also hides how big your files are and how many there are" : "gocryptfs is fast and works well with backups"
                options: [{ value: "gocryptfs", label: "gocryptfs", disabled: !page.st || page.st.backends.indexOf("gocryptfs") < 0 },
                          { value: "cryfs", label: "CryFS", disabled: !page.st || page.st.backends.indexOf("cryfs") < 0 }]
                current: page.backend
                onChosen: v => page.backend = v
            }
            SetChoice {
                icon: "clock"; title: "Lock by itself"; sub: "After nobody used its files for a while"
                options: [{ value: 5, label: "5 min" }, { value: 15, label: "15 min" }, { value: 60, label: "1 hour" }, { value: 0, label: "Never" }]
                current: page.idle
                onChosen: v => page.idle = v
            }
            SetSwitch { icon: "lock"; title: "Lock it when the screen locks"; checked: page.lockOnLock; onToggled: v => page.lockOnLock = v }
            SetRow {
                icon: "check"; chevron: false; title: job.what === "create" ? "Making it…" : "Make the vault"
                onActivated: {
                    if (npw.text !== npw2.text) { page.say("The passwords don't match.", true); return; }
                    job.run(["create"], r => { page.st = r; page.creating = false; page.say(nname.text.trim() + " is ready and unlocked. Put your private files in it, then lock it."); nname.text = ""; },
                            e => page.say(e, true), JSON.stringify({ name: nname.text.trim(), backend: page.backend, password: npw.text, idle: page.idle, lockOnLock: page.lockOnLock }), "create");
                    npw.text = ""; npw2.text = "";
                }
            }
            SetRow { icon: "close"; chevron: false; title: "Cancel"; onActivated: { page.creating = false; npw.text = ""; npw2.text = ""; } }
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}

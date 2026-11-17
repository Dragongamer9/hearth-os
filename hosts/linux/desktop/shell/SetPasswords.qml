// Hearth Settings › Passwords (like KDE Wallet Manager): the system's password store (the Secret Service: GNOME Keyring
// on Hearth OS) that browsers, Discord, network shares and other apps keep their sign-ins in. Shows whether it's
// running, each keyring (locked or not, protected with a password or not), and what's saved, by app, with Remove.
// It never shows a password: hearth-secrets only reads names, apps, accounts and dates. Removing, locking, unlocking
// and giving a keyring a password go through the store itself, which asks with its own window when it must.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Passwords"
    property var st: null
    property var list: []
    property var lockedKeyrings: []
    property string confirm: ""
    property string filter: ""
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() {
        job.run(["status"], r => { page.st = r; if (r.running) job.run(["list"], l => { page.replaceList("list", l.items || []); page.lockedKeyrings = l.lockedKeyrings || []; }, e => page.say(e, true)); },
                e => page.say(e, true));
    }
    function act(args, okText) { confirm = ""; job.run(args, r => { page.say(r && r.prompted ? r.message : okText); page.load(); }, e => { page.say(e, true); page.load(); }); }
    function date(ms) { return ms ? new Date(ms).toLocaleDateString(Qt.locale(), Locale.ShortFormat) : ""; }
    readonly property var shown: list.filter(i => !filter || (i.label + " " + i.app + " " + i.account + " " + i.where).toLowerCase().indexOf(filter.toLowerCase()) >= 0)
    readonly property var unprotected: st && st.keyrings ? st.keyrings.filter(k => k.protected === false && !k.session) : []
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-secrets" }

    SetCardGroup {
        title: "Password store"
        SetRow {
            interactive: false; icon: "key"; chevron: false
            title: !page.st ? "Checking…" : page.st.running ? page.st.provider : "No password store is running"
            sub: !page.st ? "" : page.st.running ? "Apps keep their sign-ins here (browsers, Discord, network shares, Hearth's stores). " + page.list.length + (page.list.length === 1 ? " item saved." : " items saved.")
                 : "Apps that keep sign-ins in the system's password store can't save them until it runs. Signing out and in again starts it."
            chip: !page.st || !page.st.running ? "" : page.st.pam && page.st.pam.auth ? "Unlocks when you sign in" : ""
        }
        Repeater {
            model: page.st && page.st.keyrings ? page.st.keyrings.filter(k => !k.session) : []
            delegate: SetRow {
                required property var modelData
                icon: modelData.locked ? "lock" : "key"
                title: (modelData.label || "Keyring") + (modelData.default ? " (default)" : "")
                sub: (modelData.locked ? "Locked" : "Unlocked") + " · " + modelData.count + (modelData.count === 1 ? " item" : " items")
                     + (modelData.protected === false ? " · no password: kept unencrypted on this drive" : modelData.protected ? " · protected with a password" : "")
                chip: modelData.protected === false ? "No password" : ""
                chipKind: "warn"
                value: modelData.locked ? "Unlock" : "Lock"
                chevron: false
                onActivated: page.act([modelData.locked ? "unlock" : "lock", modelData.id], modelData.locked ? "Unlocked." : "Locked.")
            }
        }
        SetRow {
            visible: page.unprotected.length > 0 && !!page.st && page.st.prompter
            icon: "shield"; chevron: false
            title: "Protect “" + (page.unprotected.length ? page.unprotected[0].label : "") + "” with a password…"
            sub: "Use your sign-in password so it unlocks when you sign in. With automatic sign-in, apps ask for it once after each start."
            onActivated: page.act(["protect", page.unprotected[0].id], "Done.")
        }
    }

    SetCardGroup {
        title: "Saved sign-ins"
        visible: !!page.st && page.st.running
        SetField {
            visible: page.list.length > 6
            Layout.fillWidth: true
            placeholder: "Find a saved sign-in"
            onTextChanged: page.filter = text
        }
        SetRow {
            visible: page.lockedKeyrings.length > 0
            interactive: false; icon: "lock"; chevron: false
            title: "Some are in a locked keyring"; sub: "Unlock " + page.lockedKeyrings.join(", ") + " above to see them"
        }
        SetRow {
            visible: page.list.length === 0 && page.lockedKeyrings.length === 0
            interactive: false; icon: "key"; chevron: false; title: "Nothing saved yet"
        }
        Repeater {
            model: page.shown
            delegate: ColumnLayout {
                id: it
                required property var modelData
                Layout.fillWidth: true
                spacing: 6
                SetRow {
                    icon: "key"
                    title: it.modelData.label || it.modelData.app
                    sub: [it.modelData.app, it.modelData.account, it.modelData.where, "saved " + page.date(it.modelData.created), it.modelData.keyring].filter(x => x).join(" · ")
                    value: page.confirm === it.modelData.id ? "" : "Remove…"
                    chevron: false
                    onActivated: page.confirm = page.confirm === it.modelData.id ? "" : it.modelData.id
                }
                SetRow {
                    visible: page.confirm === it.modelData.id
                    Layout.leftMargin: 52
                    danger: true; chevron: false; icon: "trash"
                    title: "Remove it"; sub: "The app forgets this sign-in and asks again next time"
                    onActivated: page.act(["remove", it.modelData.id], "Removed.")
                }
            }
        }
    }

    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        Layout.fillWidth: true; Layout.topMargin: 6
        text: "Hearth never shows or copies a saved password here. Apps' own password managers (like your browser's) are in those apps."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
}

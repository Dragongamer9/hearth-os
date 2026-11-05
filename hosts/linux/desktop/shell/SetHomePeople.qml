// Hearth Settings › Home server › People (docs/HOME-SERVER.md): family and friends who can open your shares, each
// with their own password. A person is a Samba-only account (no login on this PC, no home folder); per share they get
// no access, read, or read & write, and optionally a folder of their own with a size limit (btrfs). Removing a person
// never deletes their files. Passwords go straight to smbpasswd through the root helper; Hearth only remembers when.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "People"
    property var st: null
    property string sel: ""              // the person whose settings are open
    property bool adding: false
    property string newName: ""
    property bool pwOpen: false
    property string removing: ""
    property string msg: ""
    property bool bad: false
    readonly property var cfg: st ? st.settings : null
    readonly property var people: st ? (st.people || []) : []
    readonly property var who: sel ? (people.find(p => p.name === sel) || null) : null
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["status"], r => page.st = r, e => page.st = { error: e }); }
    function run(args, ok) { act.run(args, () => { if (ok) page.say(ok); page.refresh(); }, e => { page.say(e, true); page.refresh(); }); }
    function accessOf(share, name) { return share.people && share.people[name] ? share.people[name] : "none"; }
    function summary(p) {
        const sh = page.st ? page.st.shares : [];
        const parts = sh.filter(s => page.accessOf(s, p.name) !== "none").map(s => s.name + (page.accessOf(s, p.name) === "read" ? " (read only)" : ""));
        if (p.folder) parts.push("own folder" + (p.quotaGb ? " (" + p.quotaGb + " GB)" : ""));
        return parts.join(", ") || "No access yet: pick their shares below";
    }
    Component.onCompleted: refresh()
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: "Family and friends who can open your shares, each with their own name and password and only the shares you pick. They can't sign in to this PC."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && !!page.st.tools && !page.st.tools.samba
        Layout.fillWidth: true
        text: "Samba (sharing) isn't on this PC yet: people come with the Hearth OS update that brings Home server."
        color: Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetHeader { text: "People" }
    SetRow {
        visible: !!page.st
        icon: "user"; chevron: false; interactive: false
        title: "You (" + (page.st ? page.st.user : "") + ")"
        value: "Owner"
        sub: "Everything, with your sharing password (Shared folders)"
    }
    Repeater {
        model: page.people
        delegate: SetRow {
            required property var modelData
            icon: "user"; title: modelData.name
            sub: page.summary(modelData) + (modelData.account === false ? " · not set up on this PC: set their password again" : "")
            value: page.sel === modelData.name ? "Open below" : "Edit"
            onActivated: { page.sel = page.sel === modelData.name ? "" : modelData.name; page.removing = ""; page.pwOpen = false; }
        }
    }
    SetRow {
        icon: "plus"; title: "Add a person"
        sub: "A name (small letters, like sam or mia-k) and their own password"
        onActivated: { page.adding = !page.adding; page.newName = ""; if (page.adding) nameField.focusInput(); }
    }
    SetField {
        id: nameField
        visible: page.adding && page.newName === ""
        placeholder: "Their name: 2 to 24 small letters, digits, - or _, then Enter"
        onSubmitted: t => {
            const n = t.trim().toLowerCase();
            if (!/^[a-z][a-z0-9_-]{1,23}$/.test(n)) { page.say("Use 2 to 24 small letters, digits, - or _, starting with a letter", true); return; }
            page.newName = n; text = ""; newPw.focusInput();
        }
    }
    SetField {
        id: newPw
        visible: page.adding && page.newName !== ""
        secret: true
        placeholder: "A password for " + page.newName + " (at least 8 characters), then Enter"
        onSubmitted: t => {
            if (t.length < 8) { page.say("Use at least 8 characters", true); return; }
            const n = page.newName;
            act.run(["person-add", n], () => { page.adding = false; page.newName = ""; page.sel = n; page.say("Added " + n + " ✓ Now pick what they may open"); page.refresh(); },
                    e => { page.say(e, true); page.refresh(); }, t);
            text = "";
        }
    }

    // The open person
    SetHeader { text: page.who ? page.who.name : ""; visible: !!page.who }
    Repeater {
        model: page.who && page.st ? page.st.shares : []
        delegate: SetChoice {
            required property var modelData
            title: modelData.name
            sub: modelData.write ? modelData.path : modelData.path + " · read only for everyone (make it read & write in Shared folders to let them change it)"
            options: [{ value: "none", label: "No access" }, { value: "read", label: "Read only" }, { value: "write", label: "Read & write", disabled: !modelData.write }]
            current: page.who ? page.accessOf(modelData, page.who.name) : "none"
            onChosen: v => page.run(["person-access", page.sel, modelData.id, v])
        }
    }
    Text {
        visible: !!page.who && !!page.st && page.st.shares.length === 0
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "Share a folder first (Shared folders), then pick it here."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    SetSwitch {
        visible: !!page.who
        icon: "folder"; title: "Their own folder"
        sub: page.who && page.who.folder ? page.who.folder + " · only " + page.who.name + " and you can open it (as \\\\" + page.st.host + "\\" + page.who.name + ")"
             : "Home › People › " + (page.who ? page.who.name : "") + ", shared with only them and you. Turning it off keeps the folder and its files"
        checked: !!page.who && !!page.who.folder
        onToggled: v => page.run(["person-folder", page.sel, v ? "on" : "off"], v ? "Their folder is ready" : "Not shared anymore (the folder and its files stay)")
    }
    SetChoice {
        visible: !!page.who
        title: "Size limit for their folder"
        sub: "On a btrfs drive (your Home on a Hearth install is). Other drives: no limit is possible"
        options: [{ value: 0, label: "None" }, { value: 50, label: "50 GB" }, { value: 200, label: "200 GB" }, { value: 500, label: "500 GB" }, { value: 1000, label: "1 TB" }]
        current: page.who ? (page.who.quotaGb || 0) : 0
        onChosen: v => act.run(["person-quota", page.sel, String(v)], r => { page.say(r.note || (v ? "Limit set: " + v + " GB" : "No limit")); page.refresh(); }, e => { page.say(e, true); page.refresh(); })
    }
    SetRow {
        visible: !!page.who
        icon: "lock"; title: page.who && page.who.account === false ? "Set their password (to finish setting them up)" : "Change their password"
        sub: page.who && page.who.pwSet ? "Set on " + new Date(page.who.pwSet).toLocaleDateString() + ". Hearth never keeps it" : "Hearth never keeps it"
        onActivated: { page.pwOpen = !page.pwOpen; if (page.pwOpen) whoPw.focusInput(); }
    }
    SetField {
        id: whoPw
        visible: !!page.who && page.pwOpen
        secret: true
        placeholder: "Their new password (at least 8 characters), then Enter"
        onSubmitted: t => {
            if (t.length < 8) { page.say("Use at least 8 characters", true); return; }
            act.run(page.who.account === false ? ["person-add", page.sel] : ["person-password", page.sel], () => { page.say("Password set ✓"); page.pwOpen = false; page.refresh(); }, e => page.say(e, true), t);
            text = "";
        }
    }
    SetRow {
        visible: !!page.who && page.removing !== page.sel
        icon: "close"; title: "Remove " + (page.who ? page.who.name : ""); chevron: false
        onActivated: page.removing = page.sel
    }
    SetRow {
        visible: !!page.who && page.removing === page.sel
        icon: "close"; danger: true; title: "Remove " + (page.who ? page.who.name : "") + " now"
        sub: "They can't open anything anymore. Their folder and everything they saved stay where they are."
        onActivated: { const n = page.sel; page.sel = ""; page.removing = ""; page.run(["person-remove", n], n + " can't open your shares anymore (their files are still there)"); }
    }

    SetHeader { text: "How they sign in" }
    SetRow {
        icon: "display"; chevron: false; interactive: false; title: "With their own name and password"
        sub: "The same way as you (Shared folders › How to open them), with their name instead of yours. Hidden files are never shown, and nobody gets in without a password."
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

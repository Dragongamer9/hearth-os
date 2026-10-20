// Hearth Settings › Users: the accounts on this device, your picture and password, adding a standard or
// administrator account, and removing one (keeping or deleting its files). Administrators are the "wheel"
// group, as Hearth's installer sets up. Everything that needs root goes through hearth-admin, which checks
// the names strictly.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Users"
    property var st: null                 // hearth-system users
    property var pictures: []
    property bool picking: false
    property string picked: ""            // the account whose options are showing
    property string confirm: ""           // the account being removed (asks first)
    property string newKind: "standard"
    property string message: ""
    property bool messageBad: false
    readonly property var me: st ? st.users.find(u => u.me) : null
    readonly property bool canManage: !!st && st.canAdmin && st.admin

    function say(t, bad) { message = t; messageBad = !!bad; }
    property int stamp: 0                 // new pictures under the same file name show at once
    function load() { job.run(["users"], r => { page.stamp++; page.st = r; }); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-system" }
    SetJob { id: admin; helper: "/usr/libexec/nimbus/hearth-admin"; admin: true }
    function runAdmin(args, input, label, okText, after) {
        say("");
        admin.run(args, () => { page.say(okText); if (after) after(); page.load(); }, e => { page.say(e, true); page.load(); }, input, label);
    }
    function fileUrl(p) { return p ? "file://" + p.split("/").map(encodeURIComponent).join("/") : ""; }
    function avatarUrl(p) { return p ? fileUrl(p) + "?" + stamp : ""; }

    component Avatar: Rectangle {
        id: av
        property string source
        property string letter
        implicitWidth: 38; implicitHeight: 38
        radius: width / 2
        color: Theme.panel3
        clip: true
        Image {
            id: avImg
            anchors.fill: parent
            source: av.source
            fillMode: Image.PreserveAspectCrop
            sourceSize: Qt.size(96, 96)
            cache: false
            visible: status === Image.Ready
        }
        Text { anchors.centerIn: parent; visible: avImg.status !== Image.Ready; text: av.letter.toUpperCase(); color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
    }

    // ------------------------------------------------ you
    SetBase {
        visible: !!page.me
        interactive: false
        Layout.fillWidth: true
        implicitHeight: 84
        RowLayout {
            anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
            spacing: 14
            Avatar { implicitWidth: 56; implicitHeight: 56; source: page.me ? page.avatarUrl(page.me.picture) : ""; letter: page.me ? (page.me.fullname || page.me.name).charAt(0) : "" }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3
                Text { text: page.me ? (page.me.fullname || page.me.name) : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold }
                Text { text: page.me ? page.me.name + " · " + (page.me.admin ? "Administrator" : "Standard account") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            }
        }
    }
    SetRow {
        visible: !!page.me
        icon: "cam"; title: page.picking ? "Choose a picture below" : "Change my picture"
        sub: "Shown here and on the login screen"
        onActivated: { page.picking = !page.picking; if (page.picking) job.run(["pictures"], r => page.pictures = r.pictures || []); }
    }
    Flow {
        visible: page.picking
        Layout.fillWidth: true
        spacing: 8
        Repeater {
            model: page.picking ? page.pictures : []
            delegate: SetBase {
                required property var modelData
                width: 88; height: 88
                Image {
                    anchors { fill: parent; margins: 6 }
                    source: page.fileUrl(modelData)
                    fillMode: Image.PreserveAspectCrop
                    sourceSize: Qt.size(160, 160)
                    asynchronous: true
                }
                onActivated: job.run(["picture-set", modelData], r => { page.picking = false; page.say(r.loginScreen ? "Picture changed." : "Picture changed (the login screen keeps its own until Hearth's admin helper is there)."); page.load(); }, e => page.say(e, true), "", "picture")
            }
        }
    }
    Text {
        visible: page.picking && page.pictures.length === 0
        Layout.fillWidth: true
        text: job.busy ? "Looking in Pictures…" : "Put a PNG or JPEG picture in your Pictures folder to use it here."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetRow {
        visible: page.picking && !!page.me && page.me.picture !== ""
        icon: "close"; chevron: false; title: "Remove my picture"
        onActivated: job.run(["picture-set", ""], () => { page.picking = false; page.say("Picture removed."); page.load(); }, e => page.say(e, true))
    }

    SetHeader { text: "My password"; visible: !!page.st && page.st.canAdmin }
    SetField { id: oldPw; visible: !!page.st && page.st.canAdmin; placeholder: "Current password"; secret: true }
    SetField { id: newPw; visible: !!page.st && page.st.canAdmin; placeholder: "New password (at least 4 characters)"; secret: true }
    SetField { id: newPw2; visible: !!page.st && page.st.canAdmin; placeholder: "New password again"; secret: true; onSubmitted: pwBtn.activated() }
    SetRow {
        id: pwBtn
        visible: !!page.st && page.st.canAdmin
        icon: "lock"; chevron: false; title: admin.what === "password" ? "Changing…" : "Change my password"
        onActivated: {
            if (newPw.text.length < 4) return page.say("The new password needs at least 4 characters.", true);
            if (newPw.text !== newPw2.text) return page.say("The new passwords don't match.", true);
            page.runAdmin(["password"], JSON.stringify({ old: oldPw.text, new: newPw.text }) + "\n", "password", "Password changed.", () => { oldPw.text = newPw.text = newPw2.text = ""; });
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ------------------------------------------------ everyone
    SetHeader { text: "Accounts on this device" }
    Repeater {
        model: page.st ? page.st.users : []
        delegate: ColumnLayout {
            id: acct
            required property var modelData
            Layout.fillWidth: true
            spacing: 8
            readonly property bool removable: page.canManage && !modelData.me && !modelData.player
            SetBase {
                Layout.fillWidth: true
                implicitHeight: 64
                interactive: true
                onActivated: if (acct.removable) { page.picked = page.picked === acct.modelData.name ? "" : acct.modelData.name; page.confirm = ""; }
                RowLayout {
                    anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
                    spacing: 14
                    Avatar { source: page.avatarUrl(acct.modelData.picture); letter: (acct.modelData.fullname || acct.modelData.name).charAt(0) }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        Text { Layout.fillWidth: true; text: (acct.modelData.fullname || acct.modelData.name) + (acct.modelData.me ? "  (you)" : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; elide: Text.ElideRight }
                        Text { text: acct.modelData.name + " · " + (acct.modelData.player ? "Multi-seat player" : acct.modelData.admin ? "Administrator" : "Standard account"); color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    }
                    Text { visible: acct.removable; text: "›"; color: Theme.muted; font.pixelSize: 20 }
                }
            }
            SetRow {
                visible: page.picked === acct.modelData.name && page.confirm === ""
                Layout.leftMargin: 52
                icon: "trash"; danger: true; chevron: false
                title: "Remove this account…"
                onActivated: page.confirm = acct.modelData.name
            }
            ColumnLayout {
                visible: page.confirm === acct.modelData.name
                Layout.fillWidth: true
                Layout.leftMargin: 52
                spacing: 8
                Text { Layout.fillWidth: true; text: "Remove " + acct.modelData.name + "? They won't be able to sign in any more."; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap }
                SetRow {
                    title: admin.what === "remove" ? "Removing…" : "Remove, but keep their files"; sub: "Their home folder stays (" + acct.modelData.home + ")"; chevron: false
                    onActivated: page.runAdmin(["user-remove", acct.modelData.name, "keep-home"], "", "remove", acct.modelData.name + " was removed. Their files are still in " + acct.modelData.home + ".", () => { page.picked = ""; page.confirm = ""; })
                }
                SetRow {
                    title: "Remove and delete their files"; sub: "Their home folder, games and saves are deleted for good"; danger: true; chevron: false
                    onActivated: page.runAdmin(["user-remove", acct.modelData.name, "delete-home"], "", "remove", acct.modelData.name + " and their files were removed.", () => { page.picked = ""; page.confirm = ""; })
                }
                SetRow { title: "Cancel"; chevron: false; onActivated: page.confirm = "" }
            }
        }
    }

    // ------------------------------------------------ add
    SetHeader { text: "Add an account"; visible: page.canManage }
    SetField { id: newName; visible: page.canManage; placeholder: "Account name (small letters, like alex)" }
    SetField { id: newFull; visible: page.canManage; placeholder: "Full name (optional)" }
    SetField { id: addPw; visible: page.canManage; placeholder: "Password (at least 4 characters)"; secret: true }
    SetField { id: addPw2; visible: page.canManage; placeholder: "Password again"; secret: true; onSubmitted: addBtn.activated() }
    SetChoice {
        visible: page.canManage
        title: "Account type"; sub: "Administrators can change system settings, install system updates and manage accounts"
        options: [{ value: "standard", label: "Standard" }, { value: "admin", label: "Administrator" }]
        current: page.newKind
        onChosen: v => page.newKind = v
    }
    SetRow {
        id: addBtn
        visible: page.canManage
        icon: "user"; chevron: false; title: admin.what === "add" ? "Adding…" : "Add account"
        onActivated: {
            const n = newName.text.trim();
            if (!/^[a-z_][a-z0-9_-]{0,31}$/.test(n)) return page.say("Account names use small letters, numbers, - and _ (up to 32), starting with a letter.", true);
            if (addPw.text.length < 4) return page.say("The password needs at least 4 characters.", true);
            if (addPw.text !== addPw2.text) return page.say("The passwords don't match.", true);
            page.runAdmin(["user-add", n, page.newKind], JSON.stringify({ password: addPw.text, fullname: newFull.text.trim() }) + "\n", "add",
                n + " can now sign in.", () => { newName.text = newFull.text = addPw.text = addPw2.text = ""; page.newKind = "standard"; });
        }
    }
    Text {
        visible: !!page.st && !page.canManage
        Layout.fillWidth: true
        text: !page.st || !page.st.canAdmin ? "Accounts are managed here on Hearth, the OS." : "Only administrators can add or remove accounts."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

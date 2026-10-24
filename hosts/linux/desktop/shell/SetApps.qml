// Hearth Settings › Apps: where apps come from besides Hearth itself.
//  - Hearth's Debian box (.deb packages, hearth-debbox): whether it's made, its size, the packages installed in it
//    (each can be removed), Reset (a fresh, empty box) and Remove (the box and its apps gone).
//  - Flathub (Apps, hearth-apps): opens the app store; says so when your own Flatpak folder can't be used, with the fix.
//  - AppImages: how they work in Files.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Apps"
    property var box: null                // hearth-debbox status
    property var packages: []             // hearth-debbox list
    property var flat: null               // hearth-apps status
    property string confirm: ""           // a package, "reset" or "remove-box"
    property string message: ""
    property bool messageBad: false

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() {
        reader.run(["status"], r => page.box = r, e => page.say(e, true));
        reader.run(["list"], r => page.replaceList("packages", r.packages || []));
        flatJob.run(["status"], r => page.flat = r);
    }
    Component.onCompleted: load()
    SetJob { id: reader; helper: "hearth-debbox" }
    SetJob { id: worker; helper: "hearth-debbox" }
    SetJob { id: flatJob; helper: "hearth-apps" }
    SetJob { id: admin; helper: "/usr/libexec/nimbus/hearth-admin"; admin: true }

    function work(args, okText) {
        if (worker.busy) return;
        confirm = ""; say("");
        worker.run(args, () => { page.say(okText); SettingsNav.toast(okText); page.load(); }, e => { page.say(e, true); page.load(); });
    }
    function boxLine() {
        if (!box) return "Looking…";
        if (box.problem) return box.problem;
        if (!box.box) return "Not set up yet: it's made the first time you install a .deb file (1–2 minutes and a download).";
        return "Ready" + (box.size ? " · " + box.size : "") + " · " + box.count + (box.count === 1 ? " package" : " packages") + " installed through Hearth"
            + (box.nvidia ? " · Nvidia graphics shared" : "");
    }

    SetHeader { text: "Debian packages (.deb)" }
    SetRow {
        icon: "store"; chevron: false; interactive: true
        title: "Hearth's Debian box"
        sub: page.boxLine() + "\n" + (page.box ? page.box.limits : "")
    }
    ColumnLayout {
        visible: worker.busy
        Layout.fillWidth: true
        spacing: 6
        Text { Layout.fillWidth: true; text: (worker.line || "Working…") + (worker.progress >= 0 ? "  " + Math.round(worker.progress * 100) + "%" : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; elide: Text.ElideRight }
        Rectangle {
            Layout.fillWidth: true; implicitHeight: 8; radius: 4; color: Theme.panel3
            Rectangle { width: parent.width * Math.max(0.03, worker.progress); height: parent.height; radius: 4; color: Theme.flame }
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.packages
        delegate: ColumnLayout {
            id: pk
            required property var modelData
            Layout.fillWidth: true
            spacing: 8
            SetRow {
                icon: "grid"
                title: pk.modelData.package + (pk.modelData.version ? "  " + pk.modelData.version : "")
                sub: [pk.modelData.summary, pk.modelData.apps.length ? "In your menu: " + pk.modelData.apps.map(a => a.name).join(", ") : "No app in the menu"].filter(x => x).join(" · ")
                value: page.confirm === pk.modelData.package ? "" : "Remove…"
                onActivated: page.confirm = page.confirm === pk.modelData.package ? "" : pk.modelData.package
            }
            SetRow {
                visible: page.confirm === pk.modelData.package
                Layout.leftMargin: 52
                danger: true; chevron: false; icon: "trash"
                title: "Remove " + pk.modelData.package; sub: "Its apps leave your menu; your files stay"
                onActivated: page.work(["remove", pk.modelData.package], pk.modelData.package + " was removed.")
            }
        }
    }
    SetRow {
        icon: "folder"; title: "Install a .deb file"
        sub: "Open it in Files (double-click, or right-click › Install). Downloads opens now."
        onActivated: Quickshell.execDetached(["hearth-filemanager", (Quickshell.env("HOME") || "") + "/Downloads"])
    }
    SetRow {
        visible: !!page.box && page.box.box && page.confirm !== "reset"
        icon: "restart"; chevron: false; title: "Reset the Debian box…"
        sub: "A fresh, empty box: everything installed in it is removed"
        onActivated: page.confirm = "reset"
    }
    SetRow {
        visible: page.confirm === "reset"
        danger: true; chevron: false; icon: "restart"
        title: "Reset it now"; sub: "Removes every package in the box and makes a new one (1–2 minutes)"
        onActivated: page.work(["reset"], "The Debian box is fresh and empty.")
    }
    SetRow {
        visible: !!page.box && page.box.box && page.confirm !== "remove-box"
        icon: "trash"; danger: true; chevron: false; title: "Remove the Debian box…"
        sub: "Frees its space; it's made again the next time you install a .deb"
        onActivated: page.confirm = "remove-box"
    }
    SetRow {
        visible: page.confirm === "remove-box"
        danger: true; chevron: false; icon: "trash"
        title: "Remove it now"; sub: "The box, everything in it and its apps in your menu"
        onActivated: page.work(["remove-box"], "The Debian box is removed.")
    }
    SetRow {
        visible: page.confirm === "reset" || page.confirm === "remove-box"
        chevron: false; title: "Cancel"
        onActivated: page.confirm = ""
    }

    SetHeader { text: "Flathub" }
    SetRow {
        icon: "store"; title: "Apps"
        sub: "Find, install and update apps from Flathub"
        onActivated: if (SettingsNav.apps) SettingsNav.apps.open("")
    }
    SetRow {
        visible: !!page.flat && page.flat.bazaar
        icon: "store"; title: "Get more apps in Bazaar"
        sub: "A full Flathub store: every app, curated lists and add-ons"
        onActivated: Quickshell.execDetached(["sh", "-c", page.flat.bazaarCommand])
    }
    SetRow {
        visible: !!page.flat && !page.flat.bazaar
        icon: "info"; chevron: false; interactive: true
        title: "Bazaar"
        sub: "Hearth OS installs Bazaar, a full Flathub store, for everyone when the PC is online (it can take a few minutes after an update)."
    }
    SetRow {
        visible: !!page.flat && page.flat.flatpak && !page.flat.userWritable
        danger: true; chevron: false; icon: "info"
        title: "Fix your Flatpak folder"
        sub: page.flat ? page.flat.userProblem : ""
        onActivated: admin.run(["home-repair"], () => { page.say("Your Flatpak folders are yours again."); page.load(); },
            e => page.say("Couldn't fix it: " + e, true))
    }
    SetRow {
        visible: !!page.flat && !!page.flat.lastError && !!page.flat.lastError.error
        chevron: false; icon: "info"; interactive: true
        title: "Last problem downloading Flathub's list"
        sub: page.flat && page.flat.lastError ? page.flat.lastError.error : ""
    }

    SetHeader { text: "AppImages" }
    SetRow {
        icon: "info"; chevron: false; interactive: true
        title: "AppImages run from Files"
        sub: "Double-click one to run it (Hearth asks once, then makes it runnable). Right-click › Add to app menu moves it to the Applications folder in your home folder and puts it in your menu; Remove from app menu takes it out again."
    }
}

// Hearth Settings › Apps › Autostart (1.14, KDE's Autostart page, in Hearth's look): the apps and commands that start
// when you sign in, each with a switch; add an installed app or a command of your own; remove one you added. The same
// XDG autostart files as Task Manager › Startup (hearth-autostart, through hearth-tasks' rules); changed only when you
// press something. Hearth's own parts are in Task Manager › Startup and Background services.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Autostart"
    property var items: null
    property string find: ""
    property bool adding: false
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["list"], r => page.replaceList("items", r.items || []), e => page.say(e, true)); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-autostart" }
    readonly property var apps: {
        void DesktopEntries.applications.values.length;
        const have = (items || []).map(i => i.file);
        const q = find.trim().toLowerCase();
        return Array.from(DesktopEntries.applications.values).filter(e => e && !e.noDisplay && e.id && have.indexOf(e.id + ".desktop") < 0 && have.indexOf(e.id) < 0
            && (q === "" || (e.name || "").toLowerCase().indexOf(q) >= 0)).sort((a, b) => (a.name || "").localeCompare(b.name || "")).slice(0, q === "" ? 8 : 20);
    }

    SetCardGroup {
        title: "Start when you sign in"
        SetRow {
            visible: !!page.items && page.items.length === 0
            interactive: false; icon: "info"; title: "Nothing starts by itself yet"; sub: "Add an app or a command below"
        }
        Repeater {
            model: page.items || []
            delegate: ColumnLayout {
                id: it
                required property var modelData
                Layout.fillWidth: true
                spacing: 3
                SetSwitch {
                    Layout.fillWidth: true
                    icon: it.modelData.added && it.modelData.command ? "doc" : "store"
                    title: it.modelData.name
                    sub: it.modelData.command ? it.modelData.command.replace(/^sh -c "(.*)"$/, "$1").replace(/\\(["\\])/g, "$1").replace(/%%/g, "%") : it.modelData.sub
                    checked: it.modelData.on
                    enabled: it.modelData.canChange
                    onToggled: v => job.run(["set", it.modelData.file, v ? "on" : "off"], () => page.load(), e => { page.say(e, true); page.load(); })
                }
                SetRow {
                    visible: it.modelData.yours
                    icon: "trash"; danger: true; chevron: false; title: "Remove " + it.modelData.name
                    onActivated: job.run(["remove", it.modelData.file], () => { page.say(it.modelData.name + " removed."); page.load(); }, e => page.say(e, true))
                }
            }
        }
    }
    SetCardGroup {
        title: "Add an app"
        SetField { id: findField; placeholder: "Find an app"; onTextChanged: page.find = text }
        Repeater {
            model: page.apps
            delegate: SetRow {
                required property var modelData
                icon: modelData.icon || "store"; chevron: false
                title: modelData.name; sub: modelData.comment || modelData.genericName || ""
                chip: "Add"; chipKind: "new"
                onActivated: job.run(["add-app", modelData.id.endsWith(".desktop") ? modelData.id : modelData.id + ".desktop"], () => { page.say(modelData.name + " starts when you sign in."); page.load(); }, e => page.say(e, true))
            }
        }
    }
    SetCardGroup {
        title: "Add a command"
        SetField { id: cmdName; placeholder: "A name for it, like Mount my NAS" }
        SetField { id: cmdLine; placeholder: "The command, like rclone mount nas: ~/NAS"; onSubmitted: addCmd.activated() }
        SetRow {
            id: addCmd
            icon: "plus"; chevron: false; title: "Add this command"; sub: "Runs once each time you sign in"
            onActivated: job.run(["add-command"], () => { page.say("Added."); cmdName.text = ""; cmdLine.text = ""; page.load(); }, e => page.say(e, true),
                JSON.stringify({ name: cmdName.text, command: cmdLine.text }))
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetRow {
        icon: "gear"; title: "Background services"; sub: "Programs that run in the background and Hearth's own parts"; chevron: true
        onActivated: SettingsNav.openPage("services")
    }
}

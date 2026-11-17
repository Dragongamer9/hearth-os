// Hearth Settings › Apps › App menu editor (1.14, KDE's Menu Editor, in Hearth's look): find an app, change its name,
// description, icon, command and menu category, run it in a terminal, take it off the menu, or put back how it was;
// and add your own command to the menu. Your copy of the app's entry (hearth-menu); the system's is never changed.
// Opened from the app menu's right-click "Edit app…" at that app (SettingsNav.menuEdit).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "App menu editor"
    property var apps: null
    property var cats: []
    property string find: ""
    property string pick: ""           // set once from "Edit app…" or `settings page menu-editor:<app id>`
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["list"], r => { page.apps = r.apps || []; page.cats = r.categories || []; }, e => page.say(e, true)); }
    // Settings can make the page twice in a row as it opens (as SetExpander says): the second one must still see the app.
    // `menu-editor:<app id>` is taken while it's fresh (asked for in the last 4 s, as SetStore does) and left for the
    // second page (closing Settings clears it); "Edit app…"'s choice is cleared a moment later.
    Component.onCompleted: {
        const m = SettingsNav.menuEdit;
        pick = m || (Date.now() - SettingsNav.anchorAt < 4000 ? SettingsNav.pageAnchor : ""); load();
        if (m) Qt.callLater(() => { if (SettingsNav.menuEdit === m) SettingsNav.menuEdit = ""; });
    }
    SetJob { id: job; helper: "hearth-menu" }
    readonly property var cur: apps ? apps.find(a => a.id === pick || a.id === pick + ".desktop") || null : null
    readonly property var shown: {
        if (!apps) return [];
        const q = find.trim().toLowerCase();
        return apps.filter(a => q === "" || a.name.toLowerCase().indexOf(q) >= 0 || a.id.toLowerCase().indexOf(q) >= 0).slice(0, q === "" ? 12 : 30);
    }
    function save(ch, ok) {
        say("");
        job.run(["save", cur.id], () => { page.say(ok || "Saved. The menu shows it in a moment."); page.load(); }, e => page.say(e, true), JSON.stringify(ch));
    }
    readonly property var catOptions: [{ value: "", label: "No category" }].concat(cats.map(c => ({ value: c.id, label: c.label })))

    // ---- the app being edited
    SetPanel {
        visible: !!page.cur
        implicitHeight: 96
        RowLayout {
            anchors { fill: parent; leftMargin: 22; rightMargin: 22 }
            spacing: 18
            AppIcon { size: 56; icon: page.cur ? page.cur.icon : ""; appId: page.cur ? page.cur.id.replace(/\.desktop$/, "") : "" }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Text { Layout.fillWidth: true; text: page.cur ? page.cur.name : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 19; font.weight: Font.DemiBold; elide: Text.ElideRight }
                Text { Layout.fillWidth: true; text: page.cur ? (page.cur.comment || page.cur.id) : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
            }
            SetChip { visible: !!page.cur && page.cur.yours; text: "Changed by you"; kind: "new" }
            SetButton { text: "Other app"; ghost: true; onActivated: page.pick = "" }
        }
    }
    SetCardGroup {
        title: "Change it"
        visible: !!page.cur
        ColumnLayout {
            Layout.fillWidth: true; spacing: 3
            Text { text: "Name"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; Layout.leftMargin: 4 }
            SetField { id: fName; text: page.cur ? page.cur.name : ""; onSubmitted: t => page.save({ name: t }) }
            Text { text: "Description"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; Layout.leftMargin: 4 }
            SetField { id: fComment; text: page.cur ? page.cur.comment : ""; onSubmitted: t => page.save({ comment: t }) }
            Text { text: "Icon (a name from the icon theme, or a picture's place)"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; Layout.leftMargin: 4 }
            SetField { id: fIcon; text: page.cur ? page.cur.icon : ""; onSubmitted: t => page.save({ icon: t }) }
            Text { text: "Command"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; Layout.leftMargin: 4 }
            SetField { id: fExec; text: page.cur ? page.cur.exec : ""; onSubmitted: t => page.save({ exec: t }) }
        }
        SetRow {
            icon: "check"; chevron: false; title: "Save the name, description, icon and command"
            onActivated: page.save({ name: fName.text, comment: fComment.text, icon: fIcon.text, exec: fExec.text })
        }
        SetDropdown {
            icon: "grid"; title: "Menu category"; options: page.catOptions; current: page.cur ? page.cur.category : ""; boxWidth: 240
            onChosen: v => page.save({ category: v })
        }
        SetSwitch {
            icon: "kbd"; title: "Run in a terminal"; sub: "For programs that type in a terminal window"
            checked: !!page.cur && page.cur.terminal
            onToggled: v => page.save({ terminal: v })
        }
        SetSwitch {
            icon: "eye"; title: "Show in the app menu"; sub: "Off takes it off the menu and search; it stays installed"
            checked: !!page.cur && !page.cur.hidden
            onToggled: v => page.save({ hidden: !v }, v ? "It's back on the menu." : "Taken off the menu.")
        }
        SetRow {
            visible: !!page.cur && page.cur.yours
            icon: "undo"; chevron: false; title: page.cur && page.cur.own ? "Remove this entry" : "Put back how it was"
            sub: page.cur && page.cur.own ? "Your own command, taken off the menu" : "Your changes to it taken out"
            onActivated: job.run(["reset", page.cur.id], () => { page.say("Done."); page.pick = ""; page.load(); }, e => page.say(e, true))
        }
    }

    // ---- picking an app
    SetCardGroup {
        title: "Pick an app to change"
        visible: !page.cur
        SetField { placeholder: "Find an app"; onTextChanged: page.find = text }
        Repeater {
            model: page.shown
            delegate: SetRow {
                required property var modelData
                icon: modelData.icon || "store"; chevron: true
                title: modelData.name; sub: modelData.hidden ? "Not on the menu" : (modelData.comment || "")
                chip: modelData.yours ? "Changed" : ""; chipKind: "new"
                onActivated: page.pick = modelData.id
            }
        }
    }
    SetCardGroup {
        title: "Add your own"
        visible: !page.cur
        SetField { id: nName; placeholder: "Name, like Backup script" }
        SetField { id: nExec; placeholder: "Command, like sh ~/bin/backup.sh" }
        SetRow {
            icon: "plus"; chevron: false; title: "Add to the menu"
            onActivated: job.run(["new"], r => { page.say("Added to the menu."); nName.text = ""; nExec.text = ""; page.load(); page.pick = r.id; }, e => page.say(e, true),
                JSON.stringify({ name: nName.text, exec: nExec.text }))
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

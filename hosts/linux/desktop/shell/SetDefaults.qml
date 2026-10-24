// Hearth Settings › Default apps: which installed app opens web links, folders, text, pictures, videos, music, PDFs,
// office files, calendars, contacts, maps and archives, and which terminal starts. The same as Game Mode's
// Settings › System › Default apps: both are hearth-system (defaults, default-set, default-reset), which lists the
// installed apps (.desktop files, Flatpak's too) that can, and saves your choice in ~/.config/mimeapps.list.
// "Reset to Hearth's defaults" takes your choices out, so Hearth's own apps open everything again.
// Mouse, keyboard and controller: a row opens its choices under it, A/Enter picks one, B/Esc goes back.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Widgets

SetPage {
    id: page
    title: "Default apps"
    property var kinds: []            // [{kind, label, current, currentName, yours, hearth, hearthName, isHearth, options: [{id, name, icon, hearth}]}]
    property string open: ""          // the kind whose choices are showing
    property bool confirm: false      // "Reset to Hearth's defaults" asked
    property string message: ""
    property bool messageBad: false
    readonly property var kindIcons: ({ browser: "globe", files: "folder", text: "edit", image: "palette", video: "display", music: "vol",
        ebooks: "file", documents: "file", sheets: "grid", slides: "display", terminal: "kbd", email: "bell", calendar: "calendar",
        contacts: "user", maps: "globe", archives: "folder" })
    // Email only once an app can do it (Hearth has no mail app).
    readonly property var shown: kinds.filter(k => k.options.length > 0 || !!k.current || !!k.hearth)
    readonly property bool anyYours: kinds.some(k => k.yours)

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["defaults"], r => page.replaceList("kinds", r.kinds || []), e => page.say(e, true)); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-system" }
    function subOf(k) {
        if (!k.current) return k.options.length ? "None chosen" : "No installed app can do this yet";
        return k.currentName + (k.isHearth ? " · Hearth's own" : k.yours ? " · your choice" : "");
    }
    function choose(k, o) {
        if (o.id === k.current) { page.open = ""; return; }
        job.run(["default-set", k.kind, o.id], () => { page.say(o.name + " opens " + k.label.toLowerCase() + " now."); page.open = ""; page.load(); },
            e => page.say(e, true), "", "save");
    }
    function resetOne(k) {
        job.run(["default-reset", k.kind], () => { page.say(k.hearthName + " opens " + k.label.toLowerCase() + " again."); page.open = ""; page.load(); },
            e => page.say(e, true), "", "save");
    }
    function resetAll() {
        job.run(["default-reset"], () => { page.confirm = false; page.say("Hearth's default apps are back."); page.open = ""; page.load(); },
            e => page.say(e, true), "", "save");
    }

    component AppChoice: SetBase {
        id: ac
        property string appIcon
        property string title
        property string tag
        property bool current: false
        Layout.fillWidth: true
        implicitHeight: 56
        Accessible.role: Accessible.RadioButton
        Accessible.name: title
        Accessible.checked: current
        Accessible.onPressAction: activated()
        RowLayout {
            anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
            spacing: 14
            IconImage { implicitSize: 32; source: AppIcons.source(ac.appIcon, "") }
            Text { Layout.fillWidth: true; text: ac.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; elide: Text.ElideRight }
            Text { visible: ac.tag !== "" && !ac.current; text: ac.tag; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
            Text { visible: ac.current; text: "In use"; color: Theme.flame; font.family: Theme.font; font.pixelSize: 13 }
        }
    }

    Text {
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "Which app opens links and files, in Desktop Mode and Game Mode. Apps you install that can open them show up here."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.shown
        delegate: ColumnLayout {
            id: kindItem
            required property var modelData
            Layout.fillWidth: true
            spacing: 8
            SetRow {
                icon: page.kindIcons[kindItem.modelData.kind] || "grid"
                title: kindItem.modelData.label
                sub: page.subOf(kindItem.modelData)
                value: page.open === kindItem.modelData.kind ? "Pick one below" : kindItem.modelData.options.length + (kindItem.modelData.options.length === 1 ? " app" : " apps")
                onActivated: page.open = page.open === kindItem.modelData.kind ? "" : kindItem.modelData.kind
            }
            Repeater {
                model: page.open === kindItem.modelData.kind ? kindItem.modelData.options : []
                delegate: AppChoice {
                    required property var modelData
                    Layout.leftMargin: 52
                    appIcon: modelData.icon
                    title: modelData.name
                    tag: modelData.hearth ? "Hearth's default" : ""
                    current: modelData.id === kindItem.modelData.current
                    onActivated: page.choose(kindItem.modelData, modelData)
                }
            }
            SetRow {
                visible: page.open === kindItem.modelData.kind && kindItem.modelData.yours && !!kindItem.modelData.hearth
                Layout.leftMargin: 52
                icon: "restart"; chevron: false
                title: "Reset to Hearth's default"
                sub: kindItem.modelData.hearthName
                onActivated: page.resetOne(kindItem.modelData)
            }
        }
    }
    SetHeader { text: "Hearth's defaults"; visible: page.kinds.length > 0 }
    SetRow {
        visible: page.kinds.length > 0 && !page.confirm
        icon: "restart"; chevron: false
        title: "Reset to Hearth's defaults…"
        sub: page.anyYours ? "Hearth's own apps open everything again; your choices above are forgotten" : "Everything already opens in the app Hearth chose"
        onActivated: page.confirm = true
    }
    RowLayout {
        visible: page.confirm
        Layout.fillWidth: true
        spacing: 8
        SetRow { title: "Yes, reset them"; danger: true; chevron: false; onActivated: page.resetAll() }
        SetRow { title: "Cancel"; chevron: false; onActivated: page.confirm = false }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.kinds.length === 0
        text: job.busy ? "Looking at your apps…" : "Couldn't read the installed apps."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
}

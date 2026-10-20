// Hearth Settings › Default apps: which installed app opens web links, folders, text files, pictures, videos
// and music. The choices are the installed apps (.desktop files) that say they can; saved in
// ~/.config/mimeapps.list by hearth-system (with xdg-settings for the browser, like hearth-browser does).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Widgets

SetPage {
    id: page
    title: "Default apps"
    property var kinds: []            // [{kind, label, current, options: [{id, name, icon}]}]
    property string open: ""          // the kind whose choices are showing
    property string message: ""
    property bool messageBad: false
    readonly property var kindIcons: ({ browser: "globe", files: "grid", text: "kbd", image: "palette", video: "display", music: "vol" })

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["defaults"], r => page.replaceList("kinds", r.kinds || [])); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-system" }
    function nameOf(k) { const o = (k.options || []).find(o => o.id === k.current); return o ? o.name : ""; }
    function choose(k, o) {
        job.run(["default-set", k.kind, o.id], () => { page.say(o.name + " is now the " + k.label.toLowerCase() + "."); page.open = ""; page.load(); },
            e => page.say(e, true), "", "save");
    }

    component AppChoice: SetBase {
        id: ac
        property string appIcon
        property string title
        property bool current: false
        Layout.fillWidth: true
        implicitHeight: 56
        RowLayout {
            anchors { fill: parent; leftMargin: 16; rightMargin: 16 }
            spacing: 14
            IconImage { implicitSize: 32; source: ac.appIcon.startsWith("/") ? "file://" + ac.appIcon : Quickshell.iconPath(ac.appIcon, "application-x-executable") }
            Text { Layout.fillWidth: true; text: ac.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; elide: Text.ElideRight }
            Text { visible: ac.current; text: "In use"; color: Theme.flame; font.family: Theme.font; font.pixelSize: 13 }
        }
    }

    Repeater {
        model: page.kinds
        delegate: ColumnLayout {
            id: kindItem
            required property var modelData
            Layout.fillWidth: true
            spacing: 8
            SetRow {
                icon: page.kindIcons[kindItem.modelData.kind] || "grid"
                title: kindItem.modelData.label
                sub: kindItem.modelData.options.length === 0 ? "No installed app can do this yet" : page.nameOf(kindItem.modelData) || "None chosen"
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
                    current: modelData.id === kindItem.modelData.current
                    onActivated: page.choose(kindItem.modelData, modelData)
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
        visible: page.kinds.length === 0
        text: job.busy ? "Looking at your apps…" : "Couldn't read the installed apps."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
}

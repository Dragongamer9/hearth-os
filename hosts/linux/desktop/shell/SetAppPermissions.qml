// Hearth Settings › Apps › App permissions (1.14, KDE's Flatpak Permissions page, in Hearth's look): pick an app from
// Flathub and see what it may reach (the internet, sound, your files, devices, Bluetooth, its windows), as it asks and
// as you changed it; a switch is Flatpak's own override for you, Reset takes your changes out (hearth-flatpak-perms).
// Changed only when you press; the app sees it the next time it starts.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "App permissions"
    property var apps: null
    property string pick: ""
    property var cur: null
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    SetJob { id: job; helper: "hearth-flatpak-perms" }
    Component.onCompleted: job.run(["list"], r => { page.apps = r.apps || []; if (page.apps.length) page.choose(SettingsNav.pageAnchor && page.apps.some(a => a.id === SettingsNav.pageAnchor) ? SettingsNav.pageAnchor : page.apps[0].id); SettingsNav.pageAnchor = ""; },
                                   e => { page.apps = []; page.say(e, true); })
    function choose(id) { pick = id; cur = null; job.run(["get", id], r => page.cur = r, e => page.say(e, true)); }
    readonly property int changedCount: cur ? cur.perms.filter(p => p.changed).length : 0

    SetRow {
        visible: !!page.apps && page.apps.length === 0
        interactive: false; icon: "store"; title: "No apps from Flathub yet"; sub: "Apps from Flathub show here, each with what it may reach"
    }
    SetPanel {
        visible: !!page.apps && page.apps.length > 0
        implicitHeight: 92
        RowLayout {
            anchors { fill: parent; leftMargin: 20; rightMargin: 14 }
            spacing: 16
            AppIcon { size: 52; appId: page.pick }
            ColumnLayout {
                Layout.fillWidth: true; spacing: 2
                Text { Layout.fillWidth: true; elide: Text.ElideRight; text: (page.apps || []).find(a => a.id === page.pick) ? page.apps.find(a => a.id === page.pick).name : page.pick; color: Theme.text; font.family: Theme.font; font.pixelSize: 18; font.weight: Font.DemiBold }
                Text { Layout.fillWidth: true; elide: Text.ElideRight; text: page.pick + (page.changedCount ? " · " + page.changedCount + " changed by you" : ""); color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            }
            SetDropdown {
                Layout.preferredWidth: 340
                title: "App"; options: (page.apps || []).map(a => ({ value: a.id, label: a.name })); current: page.pick; boxWidth: 260
                onChosen: v => page.choose(v)
            }
        }
    }
    SetCardGroup {
        title: "What it may reach"
        visible: !!page.cur
        Repeater {
            model: page.cur ? page.cur.perms : []
            delegate: SetSwitch {
                required property var modelData
                icon: modelData.key === "network" ? "globe" : modelData.key === "pulseaudio" ? "vol" : modelData.key === "devices" ? "pad" : modelData.key === "dri" ? "cpu"
                    : modelData.key === "bluetooth" ? "bt" : modelData.key === "wayland" || modelData.key === "x11" ? "window" : "folder"
                title: modelData.label
                sub: modelData.sub + (modelData.changed ? " · the app asks for it " + (modelData.app ? "on" : "off") : "")
                chip: modelData.changed ? "Changed" : ""; chipKind: "new"
                checked: modelData.now
                onToggled: v => job.run(["set", page.pick, modelData.key, v ? "on" : "off"], r => { page.cur = r; page.say("Saved. " + page.pick.split(".").pop() + " sees it the next time it starts."); }, e => page.say(e, true))
            }
        }
        SetRow {
            visible: page.changedCount > 0
            icon: "undo"; chevron: false; title: "Reset to what the app asks for"; sub: "Takes out every change you made for this app"
            onActivated: job.run(["reset", page.pick], r => { page.cur = r; page.say("Reset."); }, e => page.say(e, true))
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

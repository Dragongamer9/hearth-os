// Hearth Settings › Apps › File types (1.14, KDE's File Associations editor, in Hearth's look): every kind of file your
// apps open, found by name or ending, with the app it opens in; open one to pick another app, put back the system's,
// or add an ending of your own. Default apps (the big kinds: web, pictures, music…) stays the quick way. Through
// hearth-mime; written only when you pick.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "File types"
    property var all: null
    property string find: ""
    property string group: ""
    property string open: ""
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["types"], r => page.all = r.types || [], e => { page.all = []; page.say(e, true); }); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-mime" }
    readonly property var groups: all ? Array.from(new Set(all.map(t => t.group))).sort() : []
    readonly property var shown: {
        if (!all) return [];
        const q = find.trim().toLowerCase().replace(/^\*?\./, "");
        return all.filter(t => (group === "" || t.group === group) && (q === "" || t.name.toLowerCase().indexOf(q) >= 0 || t.mime.indexOf(q) >= 0
            || t.globs.concat(t.ownGlobs).some(g => g.toLowerCase().replace(/^\*\./, "") === q || g.toLowerCase().indexOf(q) >= 0))).slice(0, 40);
    }
    function after(t) { return r => { page.say(t); page.load(); }; }

    SetField { placeholder: "Find a kind of file, or an ending like .png"; onTextChanged: page.find = text }
    Flow {
        Layout.fillWidth: true
        spacing: 6
        Repeater {
            model: [""].concat(page.groups)
            delegate: Rectangle {
                required property string modelData
                readonly property bool lit: page.group === modelData
                implicitWidth: gl.implicitWidth + 26; implicitHeight: 32; radius: 16
                color: lit ? Theme.accent : gm.containsMouse ? SetStyle.cardHover : SetStyle.card
                Text { id: gl; anchors.centerIn: parent; text: modelData || "All"; color: lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                MouseArea { id: gm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: page.group = modelData }
            }
        }
    }
    SetCardGroup {
        title: !page.all ? "Looking at your apps…" : page.shown.length ? (page.all.length + " kinds of files") : "Nothing matches"
        Repeater {
            model: page.shown
            delegate: SetExpander {
                id: ty
                required property var modelData
                Layout.fillWidth: true
                icon: modelData.group === "Pictures" ? "image" : modelData.group === "Videos" ? "film" : modelData.group === "Music and sound" ? "vol" : "doc"
                title: modelData.name
                sub: (modelData.globs.concat(modelData.ownGlobs).slice(0, 5).join("  ") || modelData.mime)
                value: modelData.currentName
                chip: modelData.yours ? "Yours" : ""; chipKind: "new"
                open: page.open === modelData.mime
                onOpenChanged: if (open) page.open = modelData.mime
                SetDropdown {
                    title: "Opens in"
                    options: ty.modelData.apps.map(a => ({ value: a.id, label: a.name }))
                    current: ty.modelData.current
                    boxWidth: 300
                    onChosen: v => job.run(["set", ty.modelData.mime, v], page.after(ty.modelData.name + " now opens in " + (ty.modelData.apps.find(a => a.id === v) || { name: v }).name + "."), e => page.say(e, true))
                }
                SetRow {
                    visible: ty.modelData.yours
                    icon: "undo"; chevron: false; title: "Use the system's choice"
                    onActivated: job.run(["reset", ty.modelData.mime], page.after("Back to the system's choice."), e => page.say(e, true))
                }
                Repeater {
                    model: ty.modelData.ownGlobs
                    delegate: SetRow {
                        required property string modelData
                        icon: "trash"; chevron: false; title: "Remove the ending " + modelData
                        onActivated: job.run(["remove-glob", ty.modelData.mime, modelData], page.after("Removed."), e => page.say(e, true))
                    }
                }
                SetField { id: globField; placeholder: "Add an ending, like *.myext"; onSubmitted: t => job.run(["add-glob", ty.modelData.mime, t.trim().startsWith("*.") ? t.trim() : "*." + t.trim().replace(/^\./, "")], page.after("Added."), e => page.say(e, true)) }
                Text {
                    Layout.fillWidth: true; Layout.leftMargin: 4
                    text: ty.modelData.mime; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11
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
    SetRow { icon: "grid"; title: "Default apps"; sub: "The browser, music, pictures and other big kinds in one place"; chevron: true; onActivated: SettingsNav.openPage("defaults") }
}

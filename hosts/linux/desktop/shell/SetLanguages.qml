// Hearth Settings › Region & language › Languages in order (1.14, KDE's several languages, in Hearth's look): the
// languages apps try one after another (LANGUAGE): an app without your first language uses the next, before English.
// Your own setting (hearth-region languages-set: ~/.config/environment.d and plasma-localerc), from the next sign-in;
// written only when you change the list. The system's language stays on Region & language.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Languages in order"
    property var list: []
    property var locales: []
    property string message: ""
    property bool messageBad: false
    property bool changed: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    SetJob { id: job; helper: "hearth-region" }
    Component.onCompleted: {
        job.run(["languages"], r => page.list = r.languages || []);
        job.run(["status"], r => page.locales = (r.locales || []).filter(l => !l.missing && l.id !== "C.UTF-8"));
    }
    function idOf(l) { return l.replace(/\..*$/, ""); }
    function nameOf(id) {
        const l = locales.find(x => idOf(x.id) === id);
        return l ? l.name : id;
    }
    function save(l) {
        say("");
        job.run(["languages-set"], r => { page.list = r.languages; page.changed = true; page.say("Saved. Apps use it after you sign out and in."); }, e => page.say(e, true), JSON.stringify({ languages: l }));
    }
    function move(i, d) { const l = list.slice(); const j = i + d; if (j < 0 || j >= l.length) return; const t = l[i]; l[i] = l[j]; l[j] = t; save(l); }

    SetCardGroup {
        title: page.list.length ? "Apps try these in order" : "Only the system's language"
        SetRow {
            visible: page.list.length === 0
            interactive: false; icon: "globe"; title: "Apps use the language from Region & language"
            sub: "Add languages below so an app without your first one uses the next, before falling back to English"
        }
        Repeater {
            model: page.list
            delegate: RowLayout {
                id: lr
                required property string modelData
                required property int index
                Layout.fillWidth: true
                spacing: 6
                Rectangle {
                    implicitWidth: 34; implicitHeight: 34; radius: 17
                    color: lr.index === 0 ? Theme.accent : SetStyle.card2
                    Text { anchors.centerIn: parent; text: lr.index + 1; color: lr.index === 0 ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.Bold }
                }
                SetRow {
                    Layout.fillWidth: true
                    interactive: false; chevron: false; icon: "globe"
                    title: page.nameOf(lr.modelData); sub: lr.index === 0 ? "First choice" : "If an app hasn't got the ones above"
                }
                SetButton { icon: "chevu"; text: ""; enabled: lr.index > 0; opacity: enabled ? 1 : 0.35; onActivated: page.move(lr.index, -1) }
                SetButton { icon: "chevd"; text: ""; enabled: lr.index < page.list.length - 1; opacity: enabled ? 1 : 0.35; onActivated: page.move(lr.index, 1) }
                SetButton { icon: "close"; text: ""; ghost: true; onActivated: page.save(page.list.filter((x, i) => i !== lr.index)) }
            }
        }
    }
    SetCardGroup {
        title: "Add a language"
        SetDropdown {
            icon: "plus"; title: "Language"; sub: "Added at the end of the list"
            options: page.locales.map(l => ({ value: page.idOf(l.id), label: l.name })).filter(o => page.list.indexOf(o.value) < 0)
            current: ""
            boxWidth: 320
            onChosen: v => page.save(page.list.concat([v]))
        }
        SetRow {
            visible: page.changed
            icon: "logout"; chevron: false; title: "Sign out now"; sub: "Apps pick up the new order when you sign in again"
            onActivated: Quickshell.execDetached(["nimbus-mode", "logout"])
        }
        SetRow { icon: "update"; chevron: true; title: "Translations & dictionaries"; sub: "Download what your languages need"; onActivated: SettingsNav.openPage("language-packs") }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

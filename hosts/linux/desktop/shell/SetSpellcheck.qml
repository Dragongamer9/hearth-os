// Hearth Settings › Region & language › Spell check (1.14, KDE's Spell Check page, in Hearth's look): which dictionary,
// whether apps check as you type, guessing the language, and words to leave alone. KDE apps' own setting (Sonnet,
// ~/.config/KDE/Sonnet.conf [General], through hearth-region: only these keys, every other line kept); written only when
// you change one. The dictionaries are the system's (Hunspell).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Spell check"
    property var st: null
    property string message: ""
    property bool messageBad: false
    readonly property var s: st ? st.settings : ({})
    function say(t, bad) { message = t; messageBad = !!bad; }
    SetJob { id: job; helper: "hearth-region" }
    Component.onCompleted: job.run(["spell"], r => page.st = r, e => page.say(e, true))
    function set(ch) { say(""); job.run(["spell-set"], r => page.st = r, e => page.say(e, true), JSON.stringify(ch)); }

    SetBanner {
        visible: !!page.st && page.st.dictionaries.length === 0
        Layout.fillWidth: true
        kind: "info"; icon: "info"; title: "No dictionaries on this system"
        sub: "Spell checking needs a Hunspell dictionary for your language"
    }
    SetCardGroup {
        title: "Checking"
        SetDropdown {
            icon: "globe"; title: "Dictionary"; sub: "The language words are checked in"
            options: page.st ? page.st.dictionaries.map(d => ({ value: d.id, label: d.name })) : []
            current: page.s.defaultLanguage || (page.st && page.st.dictionaries.length ? page.st.dictionaries[0].id : "")
            boxWidth: 300
            onChosen: v => page.set({ defaultLanguage: v })
        }
        SetSwitch { icon: "check"; title: "Check as you type"; sub: "Underlines words that look wrong in apps' text boxes"; checked: !!page.s.backgroundCheckerEnabled; onToggled: v => page.set({ backgroundCheckerEnabled: v }) }
        SetSwitch { icon: "check"; title: "On in every new text box"; sub: "Off: apps turn it on themselves where it helps"; checked: !!page.s.checkerEnabledByDefault; onToggled: v => page.set({ checkerEnabledByDefault: v }) }
        SetSwitch { icon: "globe"; title: "Guess the language"; sub: "Checks a sentence in French with the French dictionary, if you have it"; checked: !!page.s.autodetect; onToggled: v => page.set({ autodetect: v }) }
        SetSwitch { icon: "doc"; title: "Skip words in capitals"; sub: "Like NASA or HTML"; checked: !!page.s.skipUppercase; onToggled: v => page.set({ skipUppercase: v }) }
        SetSwitch { icon: "doc"; title: "Skip run-together words"; sub: "Like setupfile"; checked: !!page.s.skipRunTogether; onToggled: v => page.set({ skipRunTogether: v }) }
    }
    SetCardGroup {
        title: "Words to leave alone"
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: page.s.ignore || []
                delegate: Rectangle {
                    required property string modelData
                    implicitWidth: wl.implicitWidth + 40; implicitHeight: 30; radius: 15; color: SetStyle.card2
                    Text { id: wl; x: 12; anchors.verticalCenter: parent.verticalCenter; text: modelData; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                    Icon { anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter } name: "close"; size: 12; color: Theme.muted }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: page.set({ ignore: (page.s.ignore || []).filter(w => w !== modelData) }) }
                }
            }
        }
        SetField {
            placeholder: "Add a word, like Hearth"
            onSubmitted: t => { const w = t.trim(); if (w) page.set({ ignore: (page.s.ignore || []).concat([w]) }); text = ""; }
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

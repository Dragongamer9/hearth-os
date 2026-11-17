// Hearth Settings › Region & language › Translations & dictionaries (1.14, KDE's downloading of translations and
// language packs, in Hearth's look): for each language you chose (Languages in order, and the system's), how many of
// the system's apps already speak it, whether Flatpak apps get its translations, and its spell check dictionary;
// Download what's missing gets Flatpak's translations and the dictionaries (hearth-langpacks, your own files only).
// Each can be taken back. Opening the page only reads.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Translations & dictionaries"
    property var st: null
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    SetJob { id: job; helper: "hearth-langpacks" }
    Component.onCompleted: job.run(["status"], r => page.st = r, e => page.say(e, true))
    readonly property var missing: st ? st.languages.filter(l => l.dict === "missing" || (st.flatpak && !l.flatpak)) : []

    SetCardGroup {
        title: "Your languages"
        SetRow {
            visible: !!page.st && page.st.languages.length === 0
            icon: "globe"; chevron: true; title: "Choose your languages first"; sub: "Languages in order"
            onActivated: SettingsNav.openPage("languages")
        }
        Repeater {
            model: page.st ? page.st.languages : []
            delegate: SetExpander {
                id: lx
                required property var modelData
                Layout.fillWidth: true
                icon: "globe"
                title: modelData.name
                sub: [modelData.apps ? modelData.apps + " system apps speak it" : "The system's apps: English for now",
                      page.st.flatpak ? (modelData.flatpak ? "Flatpak apps: yes" : "Flatpak apps: not yet") : "",
                      modelData.dict === "missing" ? "No dictionary" : "Dictionary"].filter(x => x).join(" · ")
                chip: modelData.dict !== "missing" && (!page.st.flatpak || modelData.flatpak) ? "Ready" : "Missing parts"
                chipKind: chip === "Ready" ? "ok" : "warn"
                SetRow {
                    interactive: false; chevron: false; icon: "window"
                    title: "The system's apps"
                    sub: lx.modelData.apps ? lx.modelData.apps + " apps came with this language in Hearth OS" : "None came with it; they show the next language in your list"
                }
                SetRow {
                    visible: page.st.flatpak && lx.modelData.flatpak
                    icon: "close"; chevron: false; title: "Flatpak apps: translations downloaded"
                    sub: "Press to stop getting this language's translations for Flatpak apps"
                    onActivated: job.run(["flatpak-off", lx.modelData.id], r => { page.st = r; page.say("Flatpak apps won't get it any more."); }, e => page.say(e, true))
                }
                SetRow {
                    interactive: false; chevron: false; icon: "check"
                    visible: lx.modelData.dict !== "missing"
                    title: lx.modelData.dict === "system" ? "Spell check: came with Hearth OS" : "Spell check: downloaded"
                    sub: "Choose it on Spell check"
                }
                SetRow {
                    visible: lx.modelData.dict === "downloaded"
                    icon: "trash"; chevron: false; title: "Remove the downloaded dictionary"
                    onActivated: job.run(["remove-dict", lx.modelData.id], r => { page.st = r; page.say("Removed."); }, e => page.say(e, true))
                }
            }
        }
    }
    SetCardGroup {
        title: "Download"
        visible: !!page.st && page.st.languages.length > 0
        SetRow {
            icon: "update"; chevron: false
            title: job.what === "get" ? "Downloading… " + (job.progress >= 0 ? Math.round(job.progress * 100) + "%" : "") : page.missing.length ? "Download what's missing" : "Everything's here"
            sub: job.what === "get" ? job.line : page.missing.length ? page.missing.map(l => l.name).join(", ") + ": Flatpak apps' translations and spell check dictionaries" : "Every language you chose has its translations and dictionary"
            onActivated: if (!job.busy && page.missing.length) {
                page.say("");
                job.run(["get"], r => { page.st = r; page.say(r.problems && r.problems.length ? r.problems.join(" · ") : "Downloaded. Apps use it the next time they start.", !!(r.problems && r.problems.length)); },
                        e => page.say(e, true), JSON.stringify({ languages: page.missing.map(l => l.id) }), "get");
            }
        }
        Rectangle {
            visible: job.what === "get"
            Layout.fillWidth: true; Layout.leftMargin: 4; Layout.rightMargin: 4
            implicitHeight: 6; radius: 3; color: Theme.panel3
            Rectangle { width: parent.width * Math.max(0.03, job.progress); height: parent.height; radius: 3; color: Theme.accent }
        }
        SetRow { icon: "globe"; chevron: true; title: "Languages in order"; sub: "Add or reorder your languages"; onActivated: SettingsNav.openPage("languages") }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

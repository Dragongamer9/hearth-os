// Hearth Settings › Sound › Push to talk (1.14, KDE's push-to-talk, in Hearth's look): the microphone stays muted
// and holding a key you pick lets it hear you, in every app and game (KWin's global shortcuts: Hearth never reads the
// keyboard itself). Saved as pttOn / pttKey in desktop.json, only when you change them; PushToTalk.qml runs it.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Push to talk"
    readonly property bool kwin: Quickshell.env("HEARTH_WM") === "kwin"
    property var keys: [{ id: "f13", label: "F13" }, { id: "f14", label: "F14" }, { id: "pause", label: "Pause" },
        { id: "scrolllock", label: "Scroll Lock" }, { id: "ctrl-grave", label: "Ctrl+`" }, { id: "meta-alt-space", label: "Super+Alt+Space" },
        { id: "ctrl-alt-t", label: "Ctrl+Alt+T" }]
    readonly property string keyLabel: (keys.find(k => k.id === Settings.pttKey) || { label: Settings.pttKey }).label

    SetBanner {
        visible: !page.kwin
        Layout.fillWidth: true
        kind: "info"; icon: "info"
        title: "Push to talk needs Mantel on KWin"
        sub: "It uses KWin's global shortcuts. Settings › Your desktop picks the window manager"
    }
    SetBanner {
        visible: page.kwin && Settings.pttOn && SettingsNav.pttProblem !== ""
        Layout.fillWidth: true
        kind: "bad"
        title: "Push to talk isn't working"
        sub: SettingsNav.pttProblem + ". Turn it off and on to try again"
    }
    SetCardGroup {
        title: "Push to talk"
        SetSwitch {
            icon: "mic"; title: "Push to talk"
            sub: Settings.pttOn ? "Your microphone is muted. Hold " + page.keyLabel + " to talk" : "Mute the microphone unless you hold a key"
            checked: Settings.pttOn
            enabled: page.kwin
            onToggled: v => Settings.pttOn = v
        }
        SetDropdown {
            icon: "kbd"; title: "Key to hold"
            sub: "A key games and chats don't use. F13 and F14 are on many gaming keyboards and mice"
            options: page.keys.map(k => ({ value: k.id, label: k.label }))
            current: Settings.pttKey
            boxWidth: 260
            onChosen: v => Settings.pttKey = v
        }
    }
    // How it looks while in use: a picture of the two states.
    SetCardGroup {
        title: "How it works"
        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            Repeater {
                model: [{ icon: "mic", t: "Key held", s: "The microphone hears you", lit: true }, { icon: "vol-mute", t: "Key let go", s: "The microphone is muted", lit: false }]
                delegate: Rectangle {
                    required property var modelData
                    Layout.fillWidth: true
                    implicitHeight: 92
                    radius: SetStyle.radius
                    color: modelData.lit ? SetStyle.tint(Theme.accent, 0.13) : SetStyle.card
                    border.color: modelData.lit ? SetStyle.tint(Theme.accent, 0.45) : SetStyle.tint(Theme.text, 0.07)
                    RowLayout {
                        anchors { fill: parent; leftMargin: 18; rightMargin: 18 }
                        spacing: 14
                        Rectangle {
                            implicitWidth: 46; implicitHeight: 46; radius: 23
                            color: modelData.lit ? Theme.accent : Theme.panel3
                            Icon { anchors.centerIn: parent; name: modelData.icon; size: 22; color: modelData.lit ? Theme.accentInk : Theme.muted }
                        }
                        ColumnLayout {
                            spacing: 2
                            Text { text: modelData.t; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                            Text { text: modelData.s; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                        }
                    }
                }
            }
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "When push to talk is turned off, the microphone goes back to how it was before."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
    }
}

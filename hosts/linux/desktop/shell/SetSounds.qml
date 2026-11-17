// Hearth Settings › Sound › Sounds (1.14): the system sounds (Sounds.qml). Pick the set (Hearth's own, made for
// Hearth and GPL-3.0 like it; the freedesktop set when it's installed; none) and which events play one, each with a
// Play button to hear it. Only what you change is saved (sounds.json); opening the page writes nothing.
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Sounds"

    SetCardGroup {
        title: "Sound set"
        SetChoice {
            icon: "vol"; title: "System sounds"
            sub: Sounds.set === "freedesktop" && !Sounds.fdoInstalled ? "The freedesktop sounds aren't installed: Hearth's own play instead" : "Short sounds for notifications, warnings and devices"
            options: [{ value: "hearth", label: "Hearth" }, { value: "freedesktop", label: "Freedesktop", disabled: !Sounds.fdoInstalled }, { value: "off", label: "None" }]
            current: Sounds.set
            onChosen: v => Sounds.setSet(v)
        }
        SetRow {
            icon: "info"; title: "Where they come from"; chevron: false; interactive: false
            sub: Sounds.set === "freedesktop" ? "The freedesktop sound theme (sound-theme-freedesktop, GPL-2+ and CC-BY / CC-BY-SA by their authors), played from where it's installed"
                : "Hearth's own: made for Hearth from simple tones, nothing recorded or borrowed (GPL-3.0, like Hearth OS)"
        }
    }
    SetCardGroup {
        title: "Events"
        enabled: Sounds.set !== "off"
        opacity: enabled ? 1 : 0.5
        Repeater {
            model: Sounds.events
            delegate: SetSwitch {
                id: ev
                required property var modelData
                icon: modelData.icon
                title: modelData.label
                sub: modelData.sub
                checked: Sounds.isOn(modelData.id)
                trailing: 150
                onToggled: v => Sounds.setOn(modelData.id, v)
                SetButton {
                    anchors { right: parent.right; rightMargin: 104; verticalCenter: parent.verticalCenter }
                    ghost: true; icon: "play"; text: "Play"
                    onActivated: Sounds.preview(ev.modelData.id, Sounds.set)
                }
            }
        }
    }
}

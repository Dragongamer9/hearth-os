// Hearth Settings › Sound › Apps' sound (1.14, KDE's per-app device and the task manager's scroll-for-volume, in
// Hearth's look): each app playing sound with its volume and which speakers or headphones it plays on (moved through
// the volume mixer's "Play on…": WirePlumber moves it and remembers it for that app), and the dock switch. Levels are
// the mixer's, remembered the same way (app-volumes.json, only when you move a slider). Opening the page writes nothing.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Apps' sound"
    readonly property var mixer: SettingsNav.mixer
    readonly property var outs: mixer ? mixer.outList : []

    SetCardGroup {
        title: "Apps playing sound"
        SetRow {
            visible: !page.mixer || page.mixer.appKeys.length === 0
            interactive: false; icon: "vol"; title: "No app is playing sound right now"
            sub: "Start a video, a game or music and it shows up here"
        }
        Repeater {
            model: page.mixer ? page.mixer.appKeys : []
            delegate: ColumnLayout {
                id: app
                required property string modelData
                Layout.fillWidth: true
                spacing: 3
                SetSlider {
                    Layout.fillWidth: true
                    icon: page.mixer.mutedOf(app.modelData) ? "vol-mute" : "vol"
                    title: app.modelData
                    value: page.mixer.volOf(app.modelData)
                    onMoved: v => page.mixer.setVol(app.modelData, v, true)
                }
                SetDropdown {
                    Layout.fillWidth: true
                    visible: page.outs.length > 1
                    icon: page.mixer.devIcon(page.mixer.outOf(app.modelData)); title: "Plays on"
                    sub: "Send " + app.modelData + " to other speakers or headphones; the rest stays where it is"
                    options: page.outs.map(o => ({ value: o.name, label: o.name }))
                    current: page.mixer.outOf(app.modelData)
                    boxWidth: 300
                    onChosen: v => { const o = page.outs.find(x => x.name === v); if (o) page.mixer.sendTo(app.modelData, o); }
                }
            }
        }
    }
    SetCardGroup {
        title: "The dock"
        SetSwitch {
            icon: "vol"; title: "Scroll on an app to change its volume"
            sub: "With the mouse over an app's icon in the dock, scroll up for louder and down for quieter"
            checked: Settings.dockScrollVolume
            onToggled: v => Settings.dockScrollVolume = v
        }
    }
    SetCardGroup {
        title: "More"
        SetRow {
            visible: !!page.mixer
            icon: "vol"; title: "Volume mixer"; sub: "The same apps in a small window, from the sound icon in the bar"; chevron: true
            onActivated: if (page.mixer) page.mixer.open()
        }
    }
}

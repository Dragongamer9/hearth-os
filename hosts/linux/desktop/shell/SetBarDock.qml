// Hearth Settings › Personalization › Bar and dock: floating islands or a full-width bar, the dock always shown or
// auto-hiding. Each starts at the look's own ("") and changes only when you pick one (moved here from Appearance as
// they were).
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Bar and dock"
    Text {
        Layout.fillWidth: true
        Layout.bottomMargin: 8
        text: "Every look has its own bar and dock. Pick yours here and it stays, whichever look you choose."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    SetHeader { text: "Bar and dock" }
    SetChoice {
        icon: "window"
        title: "Bar"
        options: [{ value: "", label: "Look's own" }, { value: "floating", label: "Floating islands" }, { value: "bar", label: "Full-width bar" }]
        current: Settings.barStyle
        onChosen: v => Settings.barStyle = v
    }
    SetChoice {
        icon: "grid"
        title: "Dock"
        sub: "Auto-hide: the dock comes up when the pointer touches the bottom edge."
        options: [{ value: "", label: "Look's own" }, { value: "shown", label: "Always shown" }, { value: "autohide", label: "Auto-hide" }]
        current: Settings.dockMode
        onChosen: v => Settings.dockMode = v
    }
}

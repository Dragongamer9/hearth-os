// Hearth Settings › Personalization › Bar and dock (moved here from Appearance as it was; `settings page appearance:bar`
// opens it): the bar layout, its edge, the floating dock and auto-hide (BarLayoutPicker.qml), the bar style (the look's
// own, floating islands or a full-width bar), the app menu (AppMenuPicker.qml) and the right-click menu's look
// (MenuStylePicker.qml: Today, Windows 11 style, Compact list, Tiles and groups). The pickers are loaded by file name,
// so a problem in one only loses it. Each starts at the default and changes only when you pick one.
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
    SetHeader { text: "Bar and dock"; Layout.topMargin: 0 }
    Loader { Layout.fillWidth: true; source: "BarLayoutPicker.qml" }
    SetChoice {
        icon: "window"
        title: "Bar style"
        options: [{ value: "", label: "Look's own" }, { value: "floating", label: "Floating islands" }, { value: "bar", label: "Full-width bar" }]
        current: Settings.barStyle
        onChosen: v => Settings.barStyle = v
    }
    SetHeader { text: "App menu" }
    Loader { Layout.fillWidth: true; source: "AppMenuPicker.qml" }
    SetHeader { text: "Menus" }
    SetExpander {
        icon: "menu"
        title: "Right-click menu"
        sub: "How the menu looks on files, the desktop, the bar, the dock and the app menus"
        value: ({ today: "Today", windows: "Windows 11 style", compact: "Compact list", tiles: "Tiles and groups" })[Theme.menuStyle] || "Today"
        open: true
        anchor: "rightclick"
        Loader { Layout.fillWidth: true; Layout.margins: 14; source: "MenuStylePicker.qml" }
    }
}

// Settings › Appearance › Bar and dock: the bar layout (Top bar + floating dock, or One main bar), where the one
// main bar goes (Bottom, Top, Left, Right), the floating app menu (the dock) on or off, and auto-hide.
// A component of its own so the coming Settings redesign can move it as it is.
// Settings never change by themselves: each control writes only its own key, and only when you pick something.
// Changes apply live: shell.qml builds the bar again on its new edge.
import QtQuick
import QtQuick.Layouts

ColumnLayout {
    id: blp
    Layout.fillWidth: true
    spacing: 10

    GridLayout {
        Layout.fillWidth: true
        columns: 2
        columnSpacing: 10
        LayoutCard { value: "split"; label: "Top bar + floating dock"; desc: "Mantel's own: islands at the top, your apps in a dock along the bottom" }
        LayoutCard { value: "one"; label: "One main bar"; desc: "Everything in one bar, on the edge you choose" }
    }
    SetChoice {
        visible: Theme.oneBar
        title: "Placement"
        sub: "Where the one main bar goes. Left and right stack the icons; menus open toward the screen."
        options: [{ value: "bottom", label: "Bottom" }, { value: "top", label: "Top" }, { value: "left", label: "Left" }, { value: "right", label: "Right" }]
        current: Theme.barEdge
        onChosen: v => Settings.barEdge = v
    }
    SetSwitch {
        icon: "grid"
        title: "Floating app menu"
        sub: Theme.dockFloats ? "On: the app menu button and your apps float in a dock along the bottom"
                              : "Off: the app menu button and your apps are part of the main bar"
        checked: Theme.dockFloats
        onToggled: on => Settings.dockFloat = on ? "on" : "off"
    }
    SetChoice {
        title: "Auto-hide"
        sub: Theme.oneBar || !Theme.dockFloats ? "Auto-hide: the bar (and the dock) come back when the pointer touches the edge, and leave the whole screen to your windows."
                                              : "Auto-hide: the dock comes back when the pointer touches the bottom edge."
        options: [{ value: "", label: "Look's own" }, { value: "shown", label: "Always shown" }, { value: "autohide", label: "Auto-hide" }]
        current: Settings.dockMode
        onChosen: v => Settings.dockMode = v
    }

    // A layout as a picture card: a sketch of the screen with the bar(s) in your colours.
    component LayoutCard: SetBase {
        id: card
        property string value
        property string label
        property string desc
        readonly property bool lit: (Theme.oneBar ? "one" : "split") === value
        Layout.fillWidth: true
        implicitHeight: 150
        border.color: activeFocus ? Theme.accent : lit ? Qt.lighter(Theme.accent, 1.2) : "transparent"
        onActivated: Settings.barLayout = value
        Rectangle {
            id: scr
            anchors { left: parent.left; top: parent.top; margins: 10 }
            width: 150; height: 88
            radius: Theme.r(6)
            color: Theme.bg2
            border.color: Theme.line
            // Top bar + floating dock
            Rectangle { visible: card.value === "split"; x: 6; y: 5; width: parent.width - 12; height: 8; radius: 4; color: Theme.panel3 }
            Rectangle { visible: card.value === "split"; x: parent.width - 26; y: 6; width: 18; height: 6; radius: 3; color: Theme.accent }
            Rectangle { visible: card.value === "split"; anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 5 } width: 64; height: 10; radius: 5; color: Theme.panel3 }
            // One main bar (on the edge it's on now, or the bottom)
            Rectangle {
                visible: card.value === "one"
                readonly property string e: Theme.oneBar ? Theme.barEdge : "bottom"
                readonly property bool v: e === "left" || e === "right"
                width: v ? 12 : parent.width; height: v ? parent.height : 12
                x: e === "right" ? parent.width - width : 0
                y: e === "bottom" ? parent.height - height : 0
                color: Theme.panel3
                Rectangle { width: 8; height: 8; radius: 4; x: 2; y: 2; color: Theme.flame }
                Rectangle { width: parent.v ? 8 : 16; height: parent.v ? 16 : 8; radius: 3; x: parent.v ? 2 : parent.width - 18; y: parent.v ? parent.height - 18 : 2; color: Theme.accent }
            }
        }
        ColumnLayout {
            anchors { left: scr.right; right: parent.right; top: parent.top; margins: 12; leftMargin: 14 }
            spacing: 4
            RowLayout {
                spacing: 8
                Text { text: card.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                Rectangle {
                    visible: card.lit
                    implicitWidth: inUse.implicitWidth + 14; implicitHeight: 20; radius: 10
                    color: Theme.accent
                    Text { id: inUse; anchors.centerIn: parent; text: "In use"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 10; font.bold: true }
                }
            }
            Text { Layout.fillWidth: true; text: card.desc; wrapMode: Text.WordWrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
        }
    }
}

// Settings › Personalization › Bar and dock › Right-click menu: the four right-click menu looks as picture cards (Today,
// Windows 11 style, Compact list, Tiles and groups), drawn in your look and accent. The menu for files (Hearth Files and
// the desktop), the bar, the dock and the app menus (PopMenu.qml, MenuStyles.qml). Picking one writes only
// Settings.menuStyle and applies straight away; nothing is written until you pick (not chosen = Today).
import QtQuick
import QtQuick.Layouts

ColumnLayout {
    id: msp
    Layout.fillWidth: true
    spacing: 10
    readonly property var styles: [
        { value: "today", label: "Today", desc: "Roomy rows; submenus replace the list" },
        { value: "windows", label: "Windows 11 style", desc: "Quick icons on top, shortcuts, submenus to the side" },
        { value: "compact", label: "Compact list", desc: "The smallest: short rows, small icons" },
        { value: "tiles", label: "Tiles and groups", desc: "Big tiles for the main actions, then short groups" }
    ]
    function choose(v) { Settings.menuStyle = v; }

    Text {
        Layout.fillWidth: true
        text: "How right-click menus look on files, the desktop, the bar, the dock and the app menus. Each works with the mouse, the keyboard and the controller (A choose, B back)."
        wrapMode: Text.WordWrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
    }
    GridLayout {
        Layout.fillWidth: true
        columns: msp.width > 640 ? 4 : 2
        columnSpacing: 10
        rowSpacing: 10
        Repeater {
            model: msp.styles
            delegate: StyleCard {}
        }
    }

    // A sketch of one menu row: icon, name bar, optional shortcut, lit or not.
    component Line: Item {
        id: ln
        property real h: 10
        property real w: 40
        property bool lit: false
        property color fill: "transparent"
        property bool bar: false            // Windows 11: the small accent bar
        property bool keyed: false
        property color ink: Theme.text
        property real ic: 5
        width: parent ? parent.width : 60
        height: h
        Rectangle { anchors.fill: parent; radius: Math.min(3, Theme.r(3)); color: ln.lit ? ln.fill : "transparent"; border.width: 0 }
        Rectangle { visible: ln.bar && ln.lit; x: 1; anchors.verticalCenter: parent.verticalCenter; width: 1.5; height: ln.h * 0.55; radius: 1; color: Theme.accent }
        Rectangle { x: 4; anchors.verticalCenter: parent.verticalCenter; width: ln.ic; height: ln.ic; radius: 1; color: ln.ink; opacity: 0.85 }
        Rectangle { x: 6 + ln.ic; anchors.verticalCenter: parent.verticalCenter; width: ln.w; height: Math.max(2, ln.h * 0.28); radius: 1; color: ln.ink; opacity: 0.85 }
        Rectangle { visible: ln.keyed; anchors { right: parent.right; rightMargin: 4; verticalCenter: parent.verticalCenter } width: 10; height: Math.max(2, ln.h * 0.24); radius: 1; color: Theme.muted; opacity: 0.8 }
    }

    // A look as a picture card: a sketch of the menu in your colours, its name and what it's like.
    component StyleCard: SetBase {
        id: card
        required property var modelData
        readonly property string v: modelData.value
        readonly property bool lit: Theme.menuStyle === v
        Layout.fillWidth: true
        Layout.preferredWidth: 160
        implicitHeight: 206
        border.color: activeFocus ? Theme.accent : lit ? Qt.lighter(Theme.accent, 1.2) : "transparent"
        onActivated: msp.choose(v)
        Accessible.name: modelData.label
        Rectangle {
            id: pic
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 6 }
            height: 110
            radius: Theme.r(8)
            clip: true
            color: Theme.bg2
            // A file list behind the menu, like the mockups.
            Column {
                x: 8; y: 8; spacing: 6
                Repeater { model: 6; delegate: Rectangle { width: 40; height: 4; radius: 2; color: Theme.panel3 } }
            }
            // Today: wide, tall rows, the accent fills the lit row.
            Rectangle {
                visible: card.v === "today"
                x: 26; y: 10; width: parent.width - 40; height: 92; radius: Theme.r(6)
                color: Theme.panel; border.color: Theme.line
                Column {
                    x: 3; y: 4; width: parent.width - 6; spacing: 0
                    Repeater {
                        model: 6
                        delegate: Line { h: 14; w: 30 + (index % 3) * 8; ic: 5; lit: index === 1; fill: Theme.accent; ink: index === 1 ? Theme.accentInk : Theme.text }
                    }
                }
            }
            // Windows 11 style: quick icons, short rows with shortcuts, a flyout.
            Rectangle {
                visible: card.v === "windows"
                x: 22; y: 12; width: (parent.width - 30) * 0.66; height: 86; radius: Theme.r(6)
                color: Theme.panel; border.color: Theme.line
                Row {
                    x: 6; y: 5; spacing: (parent.width - 12 - 5 * 7) / 4
                    Repeater { model: 5; delegate: Rectangle { width: 7; height: 7; radius: 2; color: index === 2 ? Theme.accent : Theme.text; opacity: index === 2 ? 1 : 0.7 } }
                }
                Rectangle { x: 4; y: 16; width: parent.width - 8; height: 1; color: Theme.line }
                Column {
                    x: 3; y: 20; width: parent.width - 6; spacing: 0
                    Repeater {
                        model: 6
                        delegate: Line { h: 10.5; w: 18 + (index % 3) * 6; ic: 4; keyed: index !== 1; lit: index === 1; bar: true; fill: Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.1) }
                    }
                }
            }
            Rectangle {
                visible: card.v === "windows"
                x: 22 + (parent.width - 30) * 0.66 - 3; y: 38; width: (parent.width - 30) * 0.38; height: 34; radius: Theme.r(5)
                color: Theme.panel; border.color: Theme.line
                Column {
                    x: 2; y: 3; width: parent.width - 4
                    Repeater { model: 3; delegate: Line { h: 9.5; w: 14 + index * 4; ic: 3.5; lit: index === 0; bar: true; fill: Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.1) } }
                }
            }
            // Compact list: narrow, short rows, a soft tint with an outline, a red last row.
            Rectangle {
                visible: card.v === "compact"
                x: 30; y: 8; width: (parent.width - 30) * 0.62; height: 96; radius: Theme.r(5)
                color: Theme.panel; border.color: Theme.line
                Column {
                    x: 3; y: 3; width: parent.width - 6; spacing: 0
                    Repeater {
                        model: 11
                        delegate: Item {
                            width: parent.width; height: index === 3 || index === 8 ? 4 : 8.3
                            Rectangle { visible: index === 3 || index === 8; anchors.centerIn: parent; width: parent.width - 6; height: 1; color: Theme.line }
                            Rectangle {
                                visible: index === 4
                                anchors.fill: parent; radius: 2
                                color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.2)
                                border.width: 1; border.color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.45)
                            }
                            Rectangle { visible: index !== 3 && index !== 8; x: 4; anchors.verticalCenter: parent.verticalCenter; width: 3.5; height: 3.5; radius: 1; color: index === 10 ? Theme.danger : index === 4 ? Theme.accent : Theme.muted }
                            Rectangle { visible: index !== 3 && index !== 8; x: 10; anchors.verticalCenter: parent.verticalCenter; width: 16 + (index % 4) * 5; height: 2; radius: 1; color: index === 10 ? Theme.danger : Theme.text; opacity: 0.85 }
                            Rectangle { visible: index !== 3 && index !== 8 && index % 3 !== 0; anchors { right: parent.right; rightMargin: 3; verticalCenter: parent.verticalCenter } width: 8; height: 2; radius: 1; color: Theme.muted; opacity: 0.7 }
                        }
                    }
                }
            }
            // Tiles and groups: four tiles, then two titled groups.
            Rectangle {
                visible: card.v === "tiles"
                x: 24; y: 8; width: (parent.width - 30) * 0.72; height: 96; radius: Theme.r(6)
                color: Theme.panel; border.color: Theme.line
                Row {
                    x: 4; y: 4; spacing: 2
                    Repeater {
                        model: 4
                        delegate: Rectangle {
                            width: (parent.parent.width - 8 - 6) / 4; height: 16; radius: 2
                            color: Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.07)
                            Rectangle { anchors.horizontalCenter: parent.horizontalCenter; y: 3; width: 5; height: 5; radius: 1; color: Theme.text; opacity: 0.85 }
                            Rectangle { anchors.horizontalCenter: parent.horizontalCenter; y: 11; width: 9; height: 2; radius: 1; color: Theme.text; opacity: 0.6 }
                        }
                    }
                }
                Column {
                    x: 4; y: 24; width: parent.width - 8; spacing: 3
                    Repeater {
                        model: [2, 3]
                        delegate: Rectangle {
                            id: grpBox
                            required property var modelData
                            required property int index
                            width: parent.width; height: 7 + modelData * 9; radius: 3
                            color: Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.04)
                            Rectangle { x: 4; y: 2.5; width: 12; height: 2; radius: 1; color: Theme.muted }
                            Column {
                                x: 1; y: 6; width: parent.width - 2
                                Repeater {
                                    model: grpBox.modelData
                                    delegate: Line { h: 9; w: 18 + index * 5; ic: 4; lit: grpBox.index === 0 && index === 1; fill: Theme.accent; ink: lit ? Theme.accentInk : Theme.accent }
                                }
                            }
                        }
                    }
                }
            }
        }
        Rectangle {   // "In use", on the picture's corner
            visible: card.lit
            anchors { right: pic.right; top: pic.top; margins: 6 }
            implicitWidth: inUse.implicitWidth + 14; implicitHeight: 20; radius: 10
            color: Theme.accent
            Text { id: inUse; anchors.centerIn: parent; text: "In use"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 10; font.bold: true }
        }
        ColumnLayout {
            anchors { left: parent.left; right: parent.right; top: pic.bottom; margins: 10; topMargin: 8 }
            spacing: 2
            Text { Layout.fillWidth: true; text: card.modelData.label; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
            Text { Layout.fillWidth: true; text: card.modelData.desc; wrapMode: Text.WordWrap; maximumLineCount: 3; elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
        }
    }
}

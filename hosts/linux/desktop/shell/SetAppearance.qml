// Hearth Settings › Appearance: wallpaper, night light, motion.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Appearance"
    property var pictures: []
    Process {
        running: true
        command: ["sh", "-c", "ls -1 /usr/share/backgrounds/hearth/*.jpg /usr/share/backgrounds/hearth/*.png \"$HOME\"/.config/nimbus/wallpaper.jpg \"$HOME\"/.config/nimbus/wallpaper.png \"$HOME\"/Pictures/Wallpapers/*.jpg \"$HOME\"/Pictures/Wallpapers/*.png 2>/dev/null"]
        stdout: StdioCollector { onStreamFinished: page.pictures = text.split("\n").filter(Boolean) }
    }
    function nameOf(f) { return f.split("/").pop().replace(/\.(jpg|png)$/, "").replace(/^hearth-/, "").replace(/[-_]/g, " ").replace(/\b\w/g, c => c.toUpperCase()); }

    SetHeader { text: "Wallpaper" }
    GridLayout {
        id: grid
        Layout.fillWidth: true
        columns: 3
        columnSpacing: 10
        rowSpacing: 10
        Repeater {
            model: [""].concat(page.pictures)
            delegate: SetBase {
                id: tile
                required property string modelData
                readonly property bool lit: Settings.wallpaper === modelData
                Layout.preferredWidth: (grid.width - 20) / 3
                implicitHeight: 130
                border.color: activeFocus ? Theme.accent : lit ? Qt.lighter(Theme.accent, 1.2) : "transparent"
                onActivated: Settings.wallpaper = modelData
                Rectangle {
                    anchors { fill: parent; margins: 4 }
                    radius: 9
                    clip: true
                    color: "#1a110c"
                    Image { anchors.fill: parent; visible: tile.modelData !== ""; source: tile.modelData ? "file://" + tile.modelData : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 400 }
                    Image { anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom } visible: tile.modelData === ""; width: parent.width * 1.4; height: parent.height; source: Qt.resolvedUrl("ember-glow.png"); opacity: 0.6 }
                    Rectangle {
                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                        height: 30
                        gradient: Gradient { GradientStop { position: 0; color: "transparent" } GradientStop { position: 1; color: "#cc000000" } }
                        Text { anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter } text: tile.modelData ? page.nameOf(tile.modelData) : "Fireside (moving)"; color: "white"; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                        Text { anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter } visible: tile.lit; text: "✓"; color: Theme.flame; font.pixelSize: 14; font.weight: Font.Bold }
                    }
                }
            }
        }
    }
    Text { text: "Add your own pictures to Pictures › Wallpapers."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }

    SetHeader { text: "Comfort" }
    SetSwitch { icon: "moon"; title: "Night light"; sub: "Warmer screen from sunset to sunrise"; checked: Settings.nightLight; onToggled: v => Settings.nightLight = v }
    SetSwitch { icon: "motion"; title: "Reduce motion"; sub: "A still fireside wallpaper and fewer animations"; checked: Settings.reduceMotion; onToggled: v => Settings.reduceMotion = v }
}

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
        command: ["sh", "-c", "ls -1 /usr/share/backgrounds/hearth/*.jpg /usr/share/backgrounds/hearth/*.png \"$HOME\"/.config/nimbus/wallpaper.jpg \"$HOME\"/.config/nimbus/wallpaper.png \"$HOME\"/Pictures/Wallpapers/*.jpg \"$HOME\"/Pictures/Wallpapers/*.jpeg \"$HOME\"/Pictures/Wallpapers/*.png \"$HOME\"/Pictures/Wallpapers/*.webp 2>/dev/null; f=$(sed -n 's/.*\"wallpaper\": *\"\\([^\"]*\\)\".*/\\1/p' \"$HOME/.local/share/nimbus/desktop.json\" 2>/dev/null); case \"$f\" in /*) [ -f \"$f\" ] && echo \"$f\";; esac"]
        stdout: StdioCollector { onStreamFinished: page.pictures = text.split("\n").filter((f, i, a) => f && a.indexOf(f) === i) }
    }
    readonly property var names: ({ "": "Fireside", "anim:lava": "Lava", "anim:aurora": "Aurora", fireside: "Fireside (still)", embers: "Embers",
        ridge: "Dusk Ridge", mark: "The Mark", kiln: "Kiln", cat: "Hearth Cat", cabin: "Night Cabin", lanterns: "Lanterns", bloom: "Ember Bloom",
        peaks: "Low Poly Peaks", forge: "Forge Grid", topo: "Cinder Topo", fox: "Fox Moon" })
    function nameOf(f) {
        if (names[f] !== undefined) return names[f];
        const base = f.split("/").pop().replace(/\.(jpe?g|png|webp)$/i, "");
        const id = base.replace(/^hearth-/, "");
        return f.indexOf("/backgrounds/hearth/") >= 0 && names[id] ? names[id] : base.replace(/[-_]/g, " ").replace(/\b\w/g, c => c.toUpperCase());
    }
    readonly property var moving: ["", "anim:lava", "anim:aurora"]
    function preview(f) { return f === "anim:lava" ? Qt.resolvedUrl("images/wall-lava.jpg") : f === "anim:aurora" ? Qt.resolvedUrl("images/wall-aurora.jpg") : "file://" + f; }

    // "Choose a picture…": the desktop's file picker (KDE's or GNOME's, whichever is installed).
    Process {
        id: picker
        command: ["sh", "-c", "if command -v kdialog >/dev/null; then kdialog --title 'Choose a wallpaper' --getopenfilename \"$HOME/Pictures\" 'image/jpeg image/png image/webp'; elif command -v zenity >/dev/null; then zenity --file-selection --title='Choose a wallpaper' --filename=\"$HOME/Pictures/\" --file-filter='Pictures | *.jpg *.jpeg *.png *.webp'; else exit 3; fi"]
        stdout: StdioCollector { onStreamFinished: { const f = text.trim(); if (f) { Settings.wallpaper = f; if (page.pictures.indexOf(f) < 0) page.pictures = page.pictures.concat([f]); } } }
        onExited: code => { if (code === 3) SettingsNav.toast("No file picker installed: put pictures in Pictures › Wallpapers instead"); }
    }

    SetHeader { text: "Wallpaper" }
    GridLayout {
        id: grid
        Layout.fillWidth: true
        columns: 3
        columnSpacing: 10
        rowSpacing: 10
        Repeater {
            model: page.moving.concat(page.pictures)
            delegate: SetBase {
                id: tile
                required property string modelData
                readonly property bool lit: Settings.wallpaper === modelData
                Layout.fillWidth: true
                Layout.preferredWidth: 200          // equal columns (a width tied to the grid made them 0 wide)
                implicitHeight: 130
                border.color: activeFocus ? Theme.accent : lit ? Qt.lighter(Theme.accent, 1.2) : "transparent"
                onActivated: Settings.wallpaper = modelData
                Rectangle {
                    anchors { fill: parent; margins: 4 }
                    radius: 9
                    clip: true
                    color: "#1a110c"
                    Image { anchors.fill: parent; visible: tile.modelData !== ""; source: tile.modelData ? page.preview(tile.modelData) : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 400 }
                    Image { anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom } visible: tile.modelData === ""; width: parent.width * 1.4; height: parent.height; source: Qt.resolvedUrl("images/ember-glow.png"); opacity: 0.6 }
                    Rectangle {
                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                        height: 30
                        gradient: Gradient { GradientStop { position: 0; color: "transparent" } GradientStop { position: 1; color: "#cc000000" } }
                        Text { anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter } text: page.nameOf(tile.modelData) + (page.moving.indexOf(tile.modelData) >= 0 ? "  ·  moving" : ""); color: "white"; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                        Text { anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter } visible: tile.lit; text: "✓"; color: Theme.flame; font.pixelSize: 14; font.weight: Font.Bold }
                    }
                }
            }
        }
    }
    SetRow { icon: "palette"; title: "Choose a picture…"; sub: "Any picture on this device. In the Files app you can also right-click a picture › Set as Hearth wallpaper."; onActivated: picker.running = true }
    Text { Layout.fillWidth: true; text: "Pictures in Pictures › Wallpapers show up here too."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }

    SetHeader { text: "Comfort" }
    SetSwitch { icon: "moon"; title: "Night light"; sub: "Warmer screen from sunset to sunrise"; checked: Settings.nightLight; onToggled: v => Settings.nightLight = v }
    SetSwitch { icon: "motion"; title: "Reduce motion"; sub: "A still fireside wallpaper and fewer animations"; checked: Settings.reduceMotion; onToggled: v => Settings.reduceMotion = v }
}

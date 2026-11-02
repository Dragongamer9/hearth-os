// Hearth Settings › Appearance: the look (Hearth or Steam-style), dark or light, the accent colour, wallpaper,
// night light, motion. Look, mode and accent change Mantel, window frames and apps (Theme.qml, theme/hearth-theme) live.
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
    readonly property var names: ({ "": Theme.steam ? "Slate Tide (look's own)" : "Fireside", "anim:lava": "Lava", "anim:aurora": "Aurora",
        "anim:tide": "Slate Tide", "anim:ridges": "Slate Ridges", "anim:motes": "Slate Motes",
        "builtin:slate-dusk": "Slate Dusk", "builtin:slate-contours": "Slate Contours", "builtin:slate-horizon": "Slate Horizon",
        "builtin:slate-facets": "Slate Facets", "builtin:slate-nebula": "Slate Nebula", "builtin:slate-ridges": "Slate Ridges (still)", fireside: "Fireside (still)", embers: "Embers",
        ridge: "Dusk Ridge", mark: "The Mark", kiln: "Kiln", cat: "Hearth Cat", cabin: "Night Cabin", lanterns: "Lanterns", bloom: "Ember Bloom",
        peaks: "Low Poly Peaks", forge: "Forge Grid", topo: "Cinder Topo", fox: "Fox Moon" })
    function nameOf(f) {
        if (names[f] !== undefined) return names[f];
        const base = f.split("/").pop().replace(/\.(jpe?g|png|webp)$/i, "");
        const id = base.replace(/^hearth-/, "");
        return f.indexOf("/backgrounds/hearth/") >= 0 && names[id] ? names[id] : base.replace(/[-_]/g, " ").replace(/\b\w/g, c => c.toUpperCase());
    }
    readonly property var moving: ["", "anim:lava", "anim:aurora", "anim:tide", "anim:ridges", "anim:motes"]
    readonly property var builtins: ["builtin:slate-dusk", "builtin:slate-contours", "builtin:slate-horizon", "builtin:slate-facets", "builtin:slate-nebula", "builtin:slate-ridges"]
    function preview(f) {
        if (f === "" && Theme.steam) f = "anim:tide";
        if (f.startsWith("anim:")) return Qt.resolvedUrl("images/wall-" + f.slice(5) + ".jpg");
        if (f.startsWith("builtin:")) return Qt.resolvedUrl("images/wallpapers/" + f.slice(8) + ".jpg");
        return "file://" + f.split("/").map(encodeURIComponent).join("/");
    }
    // Accent colours: the presets (theme/palettes.json) and a custom one from a hue.
    readonly property var accents: (Theme.palettes.accents || [])
    function hueOf(hex) {
        const r = parseInt(hex.substr(1, 2), 16) / 255, g = parseInt(hex.substr(3, 2), 16) / 255, b = parseInt(hex.substr(5, 2), 16) / 255;
        const mx = Math.max(r, g, b), d = mx - Math.min(r, g, b);
        if (d === 0) return 0;
        const h = mx === r ? ((g - b) / d) % 6 : mx === g ? (b - r) / d + 2 : (r - g) / d + 4;
        return ((h / 6) + 1) % 1;
    }
    function hexOf(c) { const t = (x) => ("0" + Math.round(x * 255).toString(16)).slice(-2); return "#" + t(c.r) + t(c.g) + t(c.b); }

    SetHeader { text: "Look" }
    SetChoice {
        title: "Theme"
        sub: "Hearth: warm firelit darks. Steam-style: deep blue-slate, flat panels, crisp corners, made to sit next to Steam's Game Mode."
        options: [{ value: "hearth", label: "Hearth" }, { value: "steam", label: "Steam-style" }]
        current: Theme.look
        onChosen: v => Settings.look = v
    }
    SetChoice {
        title: "Mode"
        options: [{ value: "dark", label: "Dark" }, { value: "light", label: "Light" }]
        current: Theme.mode
        onChosen: v => Settings.mode = v
    }
    SetBase {
        id: accentRow
        Layout.fillWidth: true
        usesLeftRight: true
        implicitHeight: accentCol.implicitHeight + 28
        // "" first (the look's own), then the presets.
        readonly property var values: [""].concat(page.accents.map(a => a.color))
        readonly property int idx: Math.max(0, values.indexOf(Settings.accent))
        onAdjusted: d => Settings.accent = values[(idx + d + values.length) % values.length]
        onActivated: Settings.accent = values[(idx + 1) % values.length]
        ColumnLayout {
            id: accentCol
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14; leftMargin: 16; rightMargin: 16 }
            spacing: 10
            Text { text: "Accent colour"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
            Text { Layout.fillWidth: true; text: "Focus rings, selections and highlights everywhere, in apps too. Left/Right to pick."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }
            Flow {
                Layout.fillWidth: true
                spacing: 10
                Repeater {
                    model: accentRow.values
                    delegate: Rectangle {
                        id: sw
                        required property string modelData
                        required property int index
                        readonly property bool lit: Settings.accent === modelData
                        width: 40; height: 40; radius: Theme.r(20)
                        color: modelData || Theme.ember
                        border.width: lit ? 3 : 1
                        border.color: lit ? Theme.text : Theme.line
                        Text { anchors.centerIn: parent; visible: sw.modelData === ""; text: "A"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.Bold }
                        Text { anchors.centerIn: parent; visible: sw.lit && sw.modelData !== ""; text: "✓"; color: "white"; style: Text.Outline; styleColor: "#66000000"; font.pixelSize: 16; font.weight: Font.Bold }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { accentRow.forceActiveFocus(); Settings.accent = sw.modelData; } }
                    }
                }
                Rectangle {   // a custom colour, when it isn't one of the presets
                    visible: Theme.validAccent && accentRow.values.indexOf(Settings.accent) < 0
                    width: 40; height: 40; radius: Theme.r(20); color: Theme.accent; border.width: 3; border.color: Theme.text
                }
            }
            Text { text: Settings.accent === "" ? "The look's own" + (Theme.steam ? " (Steam blue)" : Theme.profileAccent ? " (your profile's)" : " (Ember)") : (page.accents.find(a => a.color === Settings.accent) || { name: "Custom " + Settings.accent }).name; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
        }
    }
    SetSlider {
        icon: "palette"
        title: "Custom accent"
        value: Theme.validAccent ? page.hueOf(Settings.accent) : 0.58
        step: 1 / 36
        valueText: Theme.validAccent ? Settings.accent : "Pick a hue"
        onMoved: v => Settings.accent = page.hexOf(Qt.hsva(Math.min(0.999, v), 0.85, 1.0, 1.0))
    }
    SetField {
        id: hexField
        placeholder: "Or type a colour, like #1a9fff, then Enter"
        onSubmitted: t => { const v = t.trim(); if (/^#?[0-9a-fA-F]{6}$/.test(v)) { Settings.accent = (v[0] === "#" ? v : "#" + v).toLowerCase(); hexField.text = ""; } else SettingsNav.toast("A colour looks like #1a9fff"); }
    }

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
            model: page.moving.concat(page.builtins).concat(page.pictures)
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
                    radius: Theme.r(9)
                    clip: true
                    color: Theme.bg2
                    Image { anchors.fill: parent; visible: tile.modelData !== "" || Theme.steam; source: tile.modelData || Theme.steam ? page.preview(tile.modelData) : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 400 }
                    Image { anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom } visible: tile.modelData === "" && !Theme.steam; width: parent.width * 1.4; height: parent.height; source: Qt.resolvedUrl("images/ember-glow.png"); opacity: 0.6 }
                    Rectangle {
                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                        height: 30
                        gradient: Gradient { GradientStop { position: 0; color: "transparent" } GradientStop { position: 1; color: "#cc000000" } }
                        Text { anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter } text: page.nameOf(tile.modelData) + (page.moving.indexOf(tile.modelData) >= 0 ? "  ·  animated" : ""); color: "white"; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                        Text { anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter } visible: tile.lit; text: "✓"; color: Theme.accent; font.pixelSize: 14; font.weight: Font.Bold }
                    }
                }
            }
        }
    }
    SetRow { icon: "palette"; title: "Choose a picture…"; sub: "Any picture on this device. In the Files app you can also right-click a picture › Set as Hearth wallpaper."; onActivated: picker.running = true }
    Text { Layout.fillWidth: true; text: "Pictures in Pictures › Wallpapers show up here too."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap }

    SetHeader { text: "Comfort" }
    SetSwitch { icon: "moon"; title: "Night light"; sub: "Warmer screen from sunset to sunrise"; checked: Settings.nightLight; onToggled: v => Settings.nightLight = v }
    SetSwitch { icon: "motion"; title: "Reduce motion"; sub: "Still wallpapers and fewer animations (animated ones also hold still on battery and behind fullscreen games)"; checked: Settings.reduceMotion; onToggled: v => Settings.reduceMotion = v }
}

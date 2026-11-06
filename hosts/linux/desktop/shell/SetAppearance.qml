// Hearth Settings › Personalization (was Appearance; Mantel 2026): the look (eight picture cards: Hearth, Steam-style,
// Gaming, Zen, Glass, Synthwave, Cabin, Console), then Your style (Dark / Light / Sunset, the accent colour: presets,
// From wallpaper, a hue, a typed colour; the wallpaper), then Desktop (Bar and dock, its own page; the desktop clock,
// panel transparency, a way to the on-screen keyboard's style). Night light moved to System › Display, Reduce motion to
// Accessibility. Look, mode and accent change Mantel, window frames and apps (Theme.qml,
// theme/hearth-theme) live.
// Settings never change by themselves: choosing a look sets only the look (its wallpaper, bar and dock are used only
// where you haven't chosen your own: those keys stay ""); Sunset is off unless you turn it on; From wallpaper works
// the colour out once, when you press it or pick a wallpaper with it on.
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
    readonly property var names: ({ "": Theme.lookLabel + "'s own", "anim:neon": "Neon HUD", "anim:zen": "Brush Circle", "anim:glass": "Aurora Glass", "anim:synth": "Sunset Grid", "anim:lava": "Lava", "anim:aurora": "Aurora",
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
    readonly property var scenes: ["anim:neon", "anim:zen", "anim:glass", "anim:synth"]
    readonly property var builtins: ["builtin:slate-dusk", "builtin:slate-contours", "builtin:slate-horizon", "builtin:slate-facets", "builtin:slate-nebula", "builtin:slate-ridges"]
    function preview(f) {
        if (f === "") f = Theme.lookDef.wallpaper || "";
        if (f === "") return "";
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

    // ---- From wallpaper (WallAccent.qml, loaded on its own): only ever run by a press, below.
    Loader { id: wallAccent; source: "WallAccent.qml"; Layout.preferredHeight: 1; Layout.preferredWidth: 1; Layout.topMargin: -8 }
    Connections {
        target: wallAccent.item
        function onFound(sw) { Settings.wallSwatches = sw; Settings.accentFrom = "wallpaper"; Settings.accent = sw[0]; }
        function onFailed() { SettingsNav.toast("Couldn't read colours from this wallpaper"); }
    }
    function fromWallpaper(wall) { if (wallAccent.item) wallAccent.item.compute(wall); else SettingsNav.toast("From wallpaper isn't available here"); }
    // Picking an accent yourself (a preset, a hue, a typed colour) ends From wallpaper.
    function chooseAccent(c) { Settings.accentFrom = ""; Settings.accent = c; }
    // Picking a wallpaper; with From wallpaper on, its colour is worked out from the new one (once, now).
    function chooseWallpaper(f) {
        Settings.wallpaper = f;
        if (Settings.accentFrom === "wallpaper") fromWallpaper(f !== "" ? f : (Theme.lookDef.wallpaper || ""));
    }

    SetHeader { text: "Look"; Layout.topMargin: 0 }
    GridLayout {
        Layout.fillWidth: true
        columns: page.width > 760 ? 4 : 2
        columnSpacing: 12
        rowSpacing: 12
        Repeater {
            model: Theme.looks
            delegate: LookCard {}
        }
    }
    Text {
        Layout.fillWidth: true
        Layout.topMargin: 8
        text: "A look sets the shapes, fonts, wallpaper and panel style. Your mode, accent, wallpaper and Bar and dock choices below still apply on top, and every look keeps the same parts and the Game Mode button."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Your style" }
    SetChoice {
        icon: "sun"
        title: "Mode"
        sub: "Sunset (optional): light by day and dark by night, on the times below. Off unless you choose it."
        options: [{ value: "dark", label: "Dark" }, { value: "light", label: "Light" }, { value: "sunset", label: "Sunset" }]
        current: Settings.sunset ? "sunset" : (Settings.mode === "light" ? "light" : "dark")
        onChosen: v => { if (v === "sunset") Settings.sunset = true; else { Settings.sunset = false; Settings.mode = v; } }
    }
    TimeRow { visible: Settings.sunset; title: "Light from"; key: "sunsetLight" }
    TimeRow { visible: Settings.sunset; title: "Dark from"; key: "sunsetDark" }
    // The accent presets: Left/Right to pick; "A" is the look's own.
    SetBase {
        id: accentRow
        Layout.fillWidth: true
        usesLeftRight: true
        readonly property bool wide: page.width > 900
        implicitHeight: wide ? Math.max(SetStyle.cardHeight, accentText.implicitHeight + 24) : accentText.implicitHeight + swatches.implicitHeight + 40
        // "" first (the look's own), then the presets.
        readonly property var values: [""].concat(page.accents.map(a => a.color))
        readonly property int idx: Math.max(0, values.indexOf(Settings.accent))
        onAdjusted: d => page.chooseAccent(values[(idx + d + values.length) % values.length])
        onActivated: page.chooseAccent(values[(idx + 1) % values.length])
        Accessible.role: Accessible.ComboBox
        Accessible.name: "Accent colour"
        Icon { x: SetStyle.pad; y: accentRow.wide ? (accentRow.height - height) / 2 : 16; name: "palette"; size: 22; color: Theme.text }
        ColumnLayout {
            id: accentText
            x: SetStyle.pad + 38
            y: accentRow.wide ? (accentRow.height - height) / 2 : 14
            width: accentRow.wide ? accentRow.width - x - swatches.width - SetStyle.pad * 2 : accentRow.width - x - SetStyle.pad
            spacing: 2
            Text { text: "Accent colour"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.Medium }
            Text {
                Layout.fillWidth: true; wrapMode: Text.WordWrap
                text: (Settings.accent === "" ? "The look's own (" + Theme.lookAccentName + ")" : Settings.accentFrom === "wallpaper" ? "From wallpaper " + Settings.accent : (page.accents.find(a => a.color === Settings.accent) || { name: "Custom " + Settings.accent }).name)
                      + " · focus rings, selections and highlights, in apps too"
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
            }
        }
        Flow {
            id: swatches
            x: accentRow.wide ? accentRow.width - width - SetStyle.pad : SetStyle.pad + 38
            y: accentRow.wide ? (accentRow.height - height) / 2 : accentText.y + accentText.height + 12
            width: Math.min(accentRow.values.length * 36 + 40, accentRow.width - SetStyle.pad * 2 - 38)
            spacing: 8
            Repeater {
                model: accentRow.values
                delegate: Rectangle {
                    id: sw
                    required property string modelData
                    required property int index
                    readonly property bool lit: Settings.accent === modelData
                    width: 28; height: 28; radius: 14
                    color: modelData || Theme.lookAccent
                    border.width: lit ? 2 : 1
                    border.color: lit ? Theme.text : Theme.line
                    Text { anchors.centerIn: parent; visible: sw.modelData === ""; text: "A"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.Bold }
                    Text { anchors.centerIn: parent; visible: sw.lit && sw.modelData !== ""; text: "✓"; color: "white"; style: Text.Outline; styleColor: "#66000000"; font.pixelSize: 13; font.weight: Font.Bold }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { accentRow.forceActiveFocus(); page.chooseAccent(sw.modelData); } }
                }
            }
            Rectangle {   // a custom colour, when it isn't one of the presets
                visible: Theme.validAccent && accentRow.values.indexOf(Settings.accent) < 0
                width: 28; height: 28; radius: 14; color: Theme.accent; border.width: 2; border.color: Theme.text
            }
        }
    }
    SetExpander {
        icon: "edit"; title: "More accent colours"; sub: "From your wallpaper, any hue, or a colour you type"
        anchor: "accent"
        value: Settings.accentFrom === "wallpaper" ? "From wallpaper" : Theme.validAccent && accentRow.values.indexOf(Settings.accent) < 0 ? Settings.accent : ""
        SetBase {
            id: fromWall
            Layout.fillWidth: true
            implicitHeight: 66
            readonly property bool on: Settings.accentFrom === "wallpaper"
            onActivated: page.fromWallpaper(Theme.wall)
            RowLayout {
                anchors { fill: parent; leftMargin: 56; rightMargin: SetStyle.pad }
                spacing: 14
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Text { text: "From wallpaper"; color: fromWall.on ? Theme.accent : Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.Medium }
                    Text { Layout.fillWidth: true; text: fromWall.on ? "Worked out from your wallpaper when you picked it. It doesn't change by itself." : "Take the accent from the wallpaper (once, now)"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
                }
                Repeater {
                    model: fromWall.on ? Settings.wallSwatches : []
                    delegate: Rectangle {
                        required property string modelData
                        width: 26; height: 26; radius: 13
                        color: modelData
                        border.width: Settings.accent === modelData ? 2 : 1
                        border.color: Settings.accent === modelData ? Theme.text : Theme.line
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Settings.accent = parent.modelData }
                    }
                }
            }
        }
        SetSlider {
            title: "Custom accent"
            value: Theme.validAccent ? page.hueOf(Settings.accent) : 0.58
            step: 1 / 36
            valueText: Theme.validAccent ? Settings.accent : "Pick a hue"
            onMoved: v => page.chooseAccent(page.hexOf(Qt.hsva(Math.min(0.999, v), 0.85, 1.0, 1.0)))
        }
        SetField {
            id: hexField
            Layout.leftMargin: 56; Layout.rightMargin: SetStyle.pad; Layout.topMargin: 8; Layout.bottomMargin: 10
            placeholder: "Or type a colour, like #1a9fff, then Enter"
            onSubmitted: t => { const v = t.trim(); if (/^#?[0-9a-fA-F]{6}$/.test(v)) { page.chooseAccent((v[0] === "#" ? v : "#" + v).toLowerCase()); hexField.text = ""; } else SettingsNav.toast("A colour looks like #1a9fff"); }
        }
    }

    // "Choose a picture…": the desktop's file picker (KDE's or GNOME's, whichever is installed).
    Process {
        id: picker
        command: ["sh", "-c", "if command -v kdialog >/dev/null; then kdialog --title 'Choose a wallpaper' --getopenfilename \"$HOME/Pictures\" 'image/jpeg image/png image/webp'; elif command -v zenity >/dev/null; then zenity --file-selection --title='Choose a wallpaper' --filename=\"$HOME/Pictures/\" --file-filter='Pictures | *.jpg *.jpeg *.png *.webp'; else exit 3; fi"]
        stdout: StdioCollector { onStreamFinished: { const f = text.trim(); if (f) { page.chooseWallpaper(f); if (page.pictures.indexOf(f) < 0) page.pictures = page.pictures.concat([f]); } } }
        onExited: code => { if (code === 3) SettingsNav.toast("No file picker installed: put pictures in Pictures › Wallpapers instead"); }
    }
    // The wallpaper: yours now and a few more to pick at once; every wallpaper inside.
    SetExpander {
        id: wallX
        icon: "image"; title: "Wallpaper"
        sub: page.nameOf(Settings.wallpaper) + " · pictures in Pictures › Wallpapers show up here too"
        anchor: "wallpaper"
        GridLayout {
            id: grid
            Layout.fillWidth: true
            Layout.margins: 14
            columns: page.width > 900 ? 4 : 3
            columnSpacing: 10
            rowSpacing: 10
            Repeater {
                model: page.moving.concat(page.scenes).concat(page.builtins).concat(page.pictures)
                delegate: SetBase {
                    id: tile
                    required property string modelData
                    readonly property bool lit: Settings.wallpaper === modelData
                    Layout.fillWidth: true
                    Layout.preferredWidth: 200          // equal columns (a width tied to the grid made them 0 wide)
                    implicitHeight: 120
                    onActivated: page.chooseWallpaper(modelData)
                    Rectangle {
                        anchors { fill: parent; margins: 4 }
                        radius: Math.max(4, Theme.r(8))
                        clip: true
                        color: Theme.bg2
                        border.width: tile.lit ? 2 : 0
                        border.color: Theme.accent
                        readonly property string shows: tile.modelData !== "" ? tile.modelData : (Theme.lookDef.wallpaper || "")
                        Image { anchors.fill: parent; visible: parent.shows !== "" && page.scenes.indexOf(parent.shows) < 0; source: visible ? page.preview(tile.modelData) : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 400 }
                        Loader { anchors.fill: parent; active: wallX.open && page.scenes.indexOf(parent.shows) >= 0; source: "Scene.qml"; onLoaded: item.name = Qt.binding(() => parent.shows.slice(5)) }
                        Image { anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom } visible: parent.shows === ""; width: parent.width * 1.4; height: parent.height; source: Qt.resolvedUrl("images/ember-glow.png"); opacity: 0.6 }
                        Rectangle {
                            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                            height: 30
                            gradient: Gradient { GradientStop { position: 0; color: "transparent" } GradientStop { position: 1; color: "#cc000000" } }
                            Text { anchors { left: parent.left; leftMargin: 10; verticalCenter: parent.verticalCenter } text: page.nameOf(tile.modelData) + (page.moving.indexOf(tile.modelData) >= 0 && tile.modelData !== "" ? "  ·  animated" : ""); color: "white"; font.family: Theme.font; font.pixelSize: 12; font.weight: Font.DemiBold }
                            Text { anchors { right: parent.right; rightMargin: 10; verticalCenter: parent.verticalCenter } visible: tile.lit; text: "✓"; color: Theme.accent; font.pixelSize: 14; font.weight: Font.Bold }
                        }
                    }
                }
            }
        }
        SetRow { icon: "image"; title: "Choose a picture…"; sub: "Any picture on this device. In the Files app you can also right-click a picture › Set as Hearth wallpaper."; onActivated: picker.running = true }
    }
    // A few wallpapers to pick from without opening it: yours, then the look's own and the moving ones.
    Row {
        Layout.fillWidth: true
        Layout.topMargin: -2
        Layout.bottomMargin: 2
        visible: !wallX.open
        spacing: 8
        leftPadding: 56
        Repeater {
            model: [Settings.wallpaper].concat(page.moving.filter(w => w !== Settings.wallpaper)).slice(0, 5)
            delegate: Rectangle {
                id: thumb
                required property string modelData
                readonly property bool lit: Settings.wallpaper === modelData
                width: 72; height: 42; radius: Math.max(4, Theme.r(6))
                color: Theme.bg2
                clip: true
                border.width: lit ? 2 : 0
                border.color: Theme.accent
                Image { anchors { fill: parent; margins: thumb.lit ? 2 : 0 } source: page.preview(thumb.modelData); fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 160; visible: status === Image.Ready }
                Image { anchors.fill: parent; visible: (thumb.modelData === "" ? (Theme.lookDef.wallpaper || "") : thumb.modelData) === ""; source: Qt.resolvedUrl("images/ember-glow.png"); fillMode: Image.PreserveAspectCrop; opacity: 0.7 }
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: page.chooseWallpaper(thumb.modelData) }
            }
        }
    }

    SetHeader { text: "Desktop" }
    SetLinkCard { target: "bar-dock"; value: Settings.barStyle === "" && Settings.dockMode === "" ? "Look's own" : [Settings.barStyle === "bar" ? "Full-width bar" : Settings.barStyle === "floating" ? "Islands" : "", Settings.dockMode === "autohide" ? "dock auto-hides" : Settings.dockMode === "shown" ? "dock shown" : ""].filter(x => x).join(", ") }
    SetChoice {
        icon: "clock"
        title: "Desktop clock and weather"
        options: [{ value: "", label: "Look's own" }, { value: "on", label: "On" }, { value: "off", label: "Off" }]
        current: Settings.deskClock
        onChosen: v => Settings.deskClock = v
    }
    SetChoice {
        icon: "eye"
        title: "Panel transparency"
        options: [{ value: "", label: "Look's own" }, { value: "solid", label: "Solid" }, { value: "frosted", label: "Frosted" }, { value: "clear", label: "Clear" }]
        current: Settings.panelGlass
        onChosen: v => Settings.panelGlass = v
    }
    SetRow { chevron: true; icon: "kbd"; title: "On-screen keyboard style"; sub: "Its looks (Ember, Frost, Steam, Warm…) are in Bluetooth & devices › Keyboard"; onActivated: SettingsNav.openPage("keyboard:osk") }

    // A look as a picture card: its wallpaper, a sketch of its bar and dock in its own colours, its name and mood.
    // Choosing it changes Settings.look only.
    component LookCard: SetBase {
        id: card
        required property string modelData
        readonly property var def: Theme.palettes[modelData] || ({})
        readonly property var pal: (def[Theme.mode] || def.dark || ({}))
        readonly property bool lit: Theme.look === modelData
        readonly property string wall: def.wallpaper || ""
        Layout.fillWidth: true
        Layout.preferredWidth: 180
        implicitHeight: 196
        radius: SetStyle.bigRadius
        border.width: lit ? 2 : 1
        border.color: lit ? Theme.accent : SetStyle.tint(Theme.text, 0.05)
        onActivated: Settings.look = modelData
        Rectangle {
            id: pic
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 8 }
            height: 120
            radius: Theme.r(9)
            clip: true
            color: card.pal.bg || "#111111"
            Loader { anchors.fill: parent; active: ["anim:neon", "anim:zen", "anim:glass", "anim:synth"].indexOf(card.wall) >= 0; source: "Scene.qml"; onLoaded: item.name = card.wall.slice(5) }
            Image { anchors.fill: parent; visible: status === Image.Ready; fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 360
                source: card.wall.startsWith("/") ? "file://" + card.wall : card.wall === "anim:tide" ? Qt.resolvedUrl("images/wall-tide.jpg") : "" }
            Image { anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom } visible: card.wall === ""; width: parent.width * 1.4; height: parent.height; source: Qt.resolvedUrl("images/ember-glow.png"); opacity: 0.7 }
            // the bar: islands, a full-width strip or Console's big row
            Rectangle {
                visible: card.def.islandStyle === "bar"
                anchors { left: parent.left; right: parent.right; top: parent.top }
                height: card.def.thin ? 6 : 9
                color: card.pal.panel || "#222222"
            }
            Row {
                visible: card.def.islandStyle !== "bar"
                x: 6; y: 5
                spacing: 3
                Rectangle { width: 30; height: card.def.islandStyle === "shelf" ? 12 : 8; radius: card.def.frame === "chamfer" ? 0 : 4; color: card.pal.panel || "#222222"; opacity: 0.92 }
            }
            Rectangle { visible: card.def.islandStyle !== "bar"; anchors.horizontalCenter: parent.horizontalCenter; y: 5; width: 34; height: 8; radius: 4; color: card.pal.panel || "#222222"; opacity: 0.92 }
            Rectangle { visible: card.def.islandStyle !== "bar"; x: parent.width - 66; y: 5; width: 36; height: 8; radius: 4; color: card.pal.panel || "#222222"; opacity: 0.92 }
            Rectangle {   // Game Mode, in every look
                x: parent.width - 26; y: card.def.islandStyle === "bar" ? 1.5 : 4
                width: 20; height: card.def.islandStyle === "bar" ? 6 : 10; radius: card.def.frame === "chamfer" ? 0 : 5
                color: card.pal.ember || "#ff8a3d"
            }
            Rectangle {   // the dock (Console: a shelf of tiles)
                visible: !card.def.dockAutoHide
                anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: card.def.islandStyle === "bar" ? 0 : 5 }
                width: card.def.islandStyle === "shelf" ? parent.width - 24 : card.def.islandStyle === "bar" ? parent.width : 70
                height: card.def.islandStyle === "shelf" ? 26 : 9
                radius: card.def.islandStyle === "bar" ? 0 : 4
                color: card.def.islandStyle === "shelf" ? "transparent" : (card.pal.panel || "#222222")
                opacity: 0.92
                Row {
                    visible: card.def.islandStyle === "shelf"
                    anchors.centerIn: parent
                    spacing: 4
                    Repeater { model: 6; delegate: Rectangle { width: 18; height: 22; radius: 3; color: card.pal.panel2 || "#333333" } }
                }
            }
        }
        RowLayout {
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 12; bottomMargin: 10 }
            spacing: 6
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0
                Text { text: card.def.label || card.modelData; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                Text { Layout.fillWidth: true; text: card.def.desc || ""; elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
            }
            SetChip { visible: card.lit; text: "In use"; kind: "new" }
        }
    }

    // Sunset's times: Left/Right (or the buttons) move it by half an hour.
    component TimeRow: SetBase {
        id: tr
        property string title
        property string key
        readonly property string value: Settings[key] || ""
        Layout.fillWidth: true
        usesLeftRight: true
        implicitHeight: 56
        function shift(d) {
            const m = /^(\d{1,2}):(\d{2})$/.exec(value) || [0, key === "sunsetLight" ? "7" : "19", key === "sunsetLight" ? "00" : "30"];
            let t = (parseInt(m[1]) * 60 + parseInt(m[2]) + d * 30 + 1440) % 1440;
            Settings[key] = ("0" + Math.floor(t / 60)).slice(-2) + ":" + ("0" + (t % 60)).slice(-2);
        }
        onAdjusted: d => shift(d)
        onActivated: shift(1)
        RowLayout {
            anchors { fill: parent; leftMargin: 16; rightMargin: 12 }
            spacing: 10
            Text { Layout.fillWidth: true; text: tr.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 15 }
            Rectangle { implicitWidth: 34; implicitHeight: 34; radius: 17; color: Theme.panel3; Text { anchors.centerIn: parent; text: "−"; color: Theme.text; font.pixelSize: 16 } MouseArea { anchors.fill: parent; onClicked: tr.shift(-1) } }
            Text { text: tr.value; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold; Layout.preferredWidth: 52; horizontalAlignment: Text.AlignHCenter }
            Rectangle { implicitWidth: 34; implicitHeight: 34; radius: 17; color: Theme.panel3; Text { anchors.centerIn: parent; text: "+"; color: Theme.text; font.pixelSize: 16 } MouseArea { anchors.fill: parent; onClicked: tr.shift(1) } }
        }
    }
}

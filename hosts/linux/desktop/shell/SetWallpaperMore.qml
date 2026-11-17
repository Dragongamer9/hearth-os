// Hearth Settings › Personalization › Wallpaper per screen, slideshow & picture of the day (1.14, KDE's in Hearth's look): each screen as a
// little picture with its own wallpaper (or "Same as the others"), and a slideshow from a folder: how often, in order
// or shuffled; the picture of the day (Bing, NASA's APOD or Wikimedia Commons; hearth-potd downloads it once a day and
// keeps it; off unless you turn it on), and a light and a dark wallpaper that follow the look's mode. Saved as
// wallScreens / wallShow / wallPotd / wallPair in desktop.json, only when you pick; WallShow.qml shows them.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Wallpaper per screen, slideshow & picture of the day"
    readonly property var show: Settings.wallShow || ({})
    property string forScreen: ""
    function url(f) {
        if (!f) return "";
        if (f.startsWith("builtin:")) return Qt.resolvedUrl("images/wallpapers/" + f.slice(8) + ".jpg");
        if (f.startsWith("anim:")) return "";
        return "file://" + f;
    }
    function setScreen(name, f) {
        const m = Object.assign({}, Settings.wallScreens || {});
        if (f) m[name] = f; else delete m[name];
        Settings.wallScreens = m;
    }
    function setShow(k, v) { const o = Object.assign({ on: false, folder: "", minutes: 30, shuffle: false }, Settings.wallShow || {}); o[k] = v; Settings.wallShow = o; }
    readonly property var potdCfg: Settings.wallPotd || ({})
    readonly property var pairCfg: Settings.wallPair || ({})
    function setPotd(k, v) { const o = Object.assign({ on: false, source: "bing" }, Settings.wallPotd || {}); o[k] = v; Settings.wallPotd = o; }
    function setPair(k, v) { const o = Object.assign({ on: false, light: "", dark: "" }, Settings.wallPair || {}); o[k] = v; Settings.wallPair = o; }
    // `settings page wallpaper-more:potd` opens it at the picture of the day (the Desktop Mode test's picture); only scrolls.
    function reveal() { if (SettingsNav.pageAnchor === "potd") { SettingsNav.pageAnchor = ""; revealTimer.start(); } }
    Timer { id: revealTimer; interval: 600; onTriggered: page.contentY = Math.max(0, Math.min(potdCard.mapToItem(page.contentItem, 0, 0).y - 16, page.contentHeight - page.height)) }
    Component.onCompleted: reveal()
    function base(f) { return f ? f.replace(/^.*\//, "") : ""; }
    // The desktop's file picker (KDE's or GNOME's), as Personalization's "Choose a picture…".
    Process {
        id: picker
        property bool folder: false
        property string pairFor: ""     // "light" | "dark" while choosing the pair's picture
        command: folder
            ? ["sh", "-c", "if command -v kdialog >/dev/null; then kdialog --title 'Pick a folder of pictures' --getexistingdirectory \"$HOME/Pictures\"; elif command -v zenity >/dev/null; then zenity --file-selection --directory --title='Pick a folder of pictures' --filename=\"$HOME/Pictures/\"; else exit 3; fi"]
            : ["sh", "-c", "if command -v kdialog >/dev/null; then kdialog --title 'Choose a wallpaper' --getopenfilename \"$HOME/Pictures\" 'image/jpeg image/png image/webp'; elif command -v zenity >/dev/null; then zenity --file-selection --title='Choose a wallpaper' --filename=\"$HOME/Pictures/\" --file-filter='Pictures | *.jpg *.jpeg *.png *.webp'; else exit 3; fi"]
        stdout: StdioCollector {
            onStreamFinished: {
                const f = text.trim();
                if (!f) return;
                if (picker.folder) page.setShow("folder", f);
                else if (picker.pairFor !== "") page.setPair(picker.pairFor, f);
                else page.setScreen(page.forScreen, f);
            }
        }
        onExited: code => { if (code === 3) SettingsNav.toast("No file picker installed"); }
    }
    function pick(screenName) { if (picker.running) return; forScreen = screenName; picker.pairFor = ""; picker.folder = false; picker.running = true; }
    function pickFolder() { if (picker.running) return; picker.pairFor = ""; picker.folder = true; picker.running = true; }
    function pickPair(which) { if (picker.running) return; picker.pairFor = which; picker.folder = false; picker.running = true; }

    SetCardGroup {
        title: "Your screens"
        Flow {
            Layout.fillWidth: true
            spacing: 14
            Repeater {
                model: Quickshell.screens
                delegate: SetPanel {
                    id: scr
                    required property var modelData
                    required property int index
                    readonly property string own: Settings.wallScreens && Settings.wallScreens[modelData.name] ? Settings.wallScreens[modelData.name] : ""
                    readonly property string shows: WallShow.forScreen(modelData.name)
                    Layout.fillWidth: false
                    width: 300; implicitHeight: 268
                    Rectangle {
                        id: thumb
                        x: 14; y: 14; width: parent.width - 28; height: width * Math.max(0.4, Math.min(0.8, scr.modelData.height / Math.max(1, scr.modelData.width)))
                        radius: 8; clip: true
                        color: Theme.bg
                        border.color: SetStyle.tint(Theme.text, 0.12)
                        Image { anchors.fill: parent; anchors.margins: 1; source: page.url(scr.shows); fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 400 }
                        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 6; color: Theme.panel; opacity: 0.85 }
                        Text { anchors.centerIn: parent; visible: page.url(scr.shows) === ""; text: "The look's own"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    }
                    Text { x: 16; y: thumb.y + thumb.height + 10; text: (scr.index === 0 ? "Main · " : "") + scr.modelData.name + " · " + scr.modelData.width + "×" + scr.modelData.height; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.Medium }
                    Text { x: 16; y: thumb.y + thumb.height + 30; width: parent.width - 32; elide: Text.ElideMiddle; text: scr.own ? page.base(scr.own) : WallShow.current !== "" ? "Slideshow" : "Same as the others"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    Row {
                        x: 14; y: parent.height - 46
                        spacing: 8
                        SetButton { text: "Choose a picture…"; icon: "image"; onActivated: page.pick(scr.modelData.name) }
                        SetButton { visible: scr.own !== ""; text: "Same as others"; ghost: true; onActivated: page.setScreen(scr.modelData.name, "") }
                    }
                }
            }
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "A screen without its own picture shows the slideshow when it's on, or the wallpaper from Personalization."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
    }
    SetCardGroup {
        title: "Slideshow"
        SetSwitch {
            icon: "image"; title: "Slideshow"
            sub: page.show.folder ? "Pictures from " + page.show.folder + (WallShow.pictures.length ? " (" + WallShow.pictures.length + ")" : "") : "Pick a folder of pictures first"
            checked: !!page.show.on
            onToggled: v => { page.setShow("on", v); if (v && !page.show.folder) page.pickFolder(); }
        }
        SetRow { icon: "folder"; title: page.show.folder ? "Change the folder" : "Pick a folder"; chevron: false; sub: page.show.folder || "JPEG, PNG and WebP pictures in it"; onActivated: page.pickFolder() }
        SetChoice {
            icon: "clock"; title: "Change the picture every"
            options: [{ value: 5, label: "5 min" }, { value: 15, label: "15 min" }, { value: 30, label: "30 min" }, { value: 60, label: "Hour" }, { value: 1440, label: "Day" }]
            current: parseInt(page.show.minutes) || 30
            onChosen: v => page.setShow("minutes", v)
        }
        SetSwitch { icon: "sync"; title: "Shuffle"; sub: "In a random order instead of by name"; checked: !!page.show.shuffle; onToggled: v => page.setShow("shuffle", v) }
        SetRow { visible: WallShow.on && WallShow.pictures.length > 1; icon: "forward"; chevron: false; title: "Next picture now"; onActivated: WallShow.next() }
    }
    SetCardGroup {
        id: potdCard
        title: "Picture of the day"
        SetSwitch {
            icon: "image"; title: "Picture of the day"
            sub: !page.potdCfg.on ? "A new wallpaper every day from the internet. Off until you turn it on"
                : WallShow.potdPath !== "" ? (WallShow.potd.title || "Today's picture") + (WallShow.potd.note ? " (kept from " + WallShow.potd.date + ": " + WallShow.potd.note + ")" : "")
                : WallShow.potdError !== "" ? "Not yet: " + WallShow.potdError : "Downloading today's picture…"
            checked: !!page.potdCfg.on
            onToggled: v => page.setPotd("on", v)
        }
        SetChoice {
            icon: "globe"; title: "From"
            options: [{ value: "bing", label: "Bing" }, { value: "apod", label: "NASA APOD" }, { value: "wikimedia", label: "Wikimedia" }]
            current: WallShow.potdSource
            onChosen: v => page.setPotd("source", v)
        }
        SetPanel {
            visible: !!page.potdCfg.on && WallShow.potdPath !== ""
            implicitHeight: 150
            Rectangle {
                id: potdThumb
                x: 14; y: 14; width: 216; height: parent.height - 28; radius: 8; clip: true; color: Theme.bg
                border.color: SetStyle.tint(Theme.text, 0.12)
                Image { anchors.fill: parent; anchors.margins: 1; source: WallShow.potdPath !== "" ? "file://" + WallShow.potdPath : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 400 }
            }
            Column {
                x: potdThumb.x + potdThumb.width + 16; y: 18; width: parent.width - x - 16
                spacing: 6
                Text { width: parent.width; text: WallShow.potd ? (WallShow.potd.title || "Today's picture") : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.Medium; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight }
                Text { width: parent.width; text: WallShow.potd && WallShow.potd.credit ? WallShow.potd.credit : ""; visible: text !== ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight }
                Text { text: WallShow.potd ? WallShow.potd.date + (WallShow.potd.cached ? " · kept on this PC" : "") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
            }
        }
        SetRow { visible: !!page.potdCfg.on; icon: "sync"; chevron: false; title: "Look again now"; sub: "Downloaded once a day and kept for a week, so it works offline too"; onActivated: WallShow.fetchPotd() }
    }
    SetCardGroup {
        title: "Light and dark"
        SetSwitch {
            icon: "moon"; title: "A wallpaper for light and one for dark"
            sub: page.pairCfg.on ? "Follows the look: now " + (Theme.dark ? "dark" : "light") : "Changes with the look's light and dark mode (and Sunset)"
            checked: !!page.pairCfg.on
            onToggled: v => page.setPair("on", v)
        }
        Row {
            Layout.fillWidth: true
            spacing: 14
            Repeater {
                model: [{ k: "light", t: "Light" }, { k: "dark", t: "Dark" }]
                delegate: SetPanel {
                    id: pp
                    required property var modelData
                    readonly property string f: typeof page.pairCfg[modelData.k] === "string" ? page.pairCfg[modelData.k] : ""
                    readonly property bool now: (modelData.k === "dark") === Theme.dark
                    Layout.fillWidth: false
                    width: 300; implicitHeight: 232
                    border.width: page.pairCfg.on && pp.now ? 2 : 1
                    border.color: page.pairCfg.on && pp.now ? Theme.accent : SetStyle.tint(Theme.text, 0.07)
                    Rectangle {
                        id: pthumb
                        x: 14; y: 14; width: parent.width - 28; height: 140; radius: 8; clip: true
                        color: pp.modelData.k === "dark" ? "#14110f" : "#ece7e1"
                        border.color: SetStyle.tint(Theme.text, 0.12)
                        Image { anchors.fill: parent; anchors.margins: 1; source: page.url(pp.f); fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 400 }
                        Text { anchors.centerIn: parent; visible: pp.f === ""; text: "The wallpaper as it is"; color: pp.modelData.k === "dark" ? "#a89f96" : "#6b625a"; font.family: Theme.font; font.pixelSize: 12 }
                    }
                    Text { x: 16; y: pthumb.y + pthumb.height + 10; width: parent.width - 32; elide: Text.ElideMiddle; text: pp.modelData.t + (pp.f ? " · " + page.base(pp.f) : ""); color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.Medium }
                    Row {
                        x: 14; y: parent.height - 46
                        spacing: 8
                        SetButton { text: "Choose…"; icon: "image"; onActivated: page.pickPair(pp.modelData.k) }
                        SetButton { visible: pp.f !== ""; text: "Clear"; ghost: true; onActivated: page.setPair(pp.modelData.k, "") }
                    }
                }
            }
        }
    }
}

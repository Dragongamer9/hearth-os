// Hearth Settings › Personalization › Wallpaper per screen & slideshow (1.14, KDE's in Hearth's look): each screen as a
// little picture with its own wallpaper (or "Same as the others"), and a slideshow from a folder: how often, in order
// or shuffled. Saved as wallScreens / wallShow in desktop.json, only when you pick; WallShow.qml shows them.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Wallpaper per screen & slideshow"
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
    function base(f) { return f ? f.replace(/^.*\//, "") : ""; }
    // The desktop's file picker (KDE's or GNOME's), as Personalization's "Choose a picture…".
    Process {
        id: picker
        property bool folder: false
        command: folder
            ? ["sh", "-c", "if command -v kdialog >/dev/null; then kdialog --title 'Pick a folder of pictures' --getexistingdirectory \"$HOME/Pictures\"; elif command -v zenity >/dev/null; then zenity --file-selection --directory --title='Pick a folder of pictures' --filename=\"$HOME/Pictures/\"; else exit 3; fi"]
            : ["sh", "-c", "if command -v kdialog >/dev/null; then kdialog --title 'Choose a wallpaper' --getopenfilename \"$HOME/Pictures\" 'image/jpeg image/png image/webp'; elif command -v zenity >/dev/null; then zenity --file-selection --title='Choose a wallpaper' --filename=\"$HOME/Pictures/\" --file-filter='Pictures | *.jpg *.jpeg *.png *.webp'; else exit 3; fi"]
        stdout: StdioCollector {
            onStreamFinished: {
                const f = text.trim();
                if (!f) return;
                if (picker.folder) page.setShow("folder", f); else page.setScreen(page.forScreen, f);
            }
        }
        onExited: code => { if (code === 3) SettingsNav.toast("No file picker installed"); }
    }
    function pick(screenName) { if (picker.running) return; forScreen = screenName; picker.folder = false; picker.running = true; }
    function pickFolder() { if (picker.running) return; picker.folder = true; picker.running = true; }

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
}

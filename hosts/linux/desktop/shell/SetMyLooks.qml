// Hearth Settings › Personalization › My looks (1.14, KDE's "Save as global theme", in Hearth's look): everything about
// how the desktop looks now (the look, light or dark, accent, sub theme, wallpaper, bar and dock, menus, panels, window
// frames, fonts, cursor and icons) saved under a name, and put back in one press. Kept in ~/.config/nimbus/my-looks.json
// (one key per look, through SettingsWriter: merged, atomic, .bak), written only when you save or delete one. Putting
// one back changes each setting the way picking it in Settings would.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "My looks"
    readonly property string path: Quickshell.env("HOME") + "/.config/nimbus/my-looks.json"
    readonly property var keys: ["look", "mode", "accent", "subTheme", "wallpaper", "wallMatch", "barStyle", "barLayout", "barEdge", "dockMode",
        "dockFloat", "panelGlass", "appMenu", "menuStyle", "deskClock", "frameTint", "signinSplash", "wallScreens", "wallShow"]
    property var looks: []
    property var fonts: null           // hearth-lookset's saved choices (fonts, cursor, icons)
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    SettingsWriter { id: writer; path: page.path }
    SetJob { id: lookset; helper: "hearth-lookset" }
    Component.onCompleted: lookset.run(["status"], r => page.fonts = r.saved || {}, () => page.fonts = {})
    FileView {
        id: file
        path: page.path
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            let o = null;
            try { o = JSON.parse(text()); } catch (e) { o = null; }
            if (!o || typeof o !== "object") { if (text().trim() !== "") writer.repair(); page.looks = []; return; }
            page.looks = Object.keys(o).filter(k => k.startsWith("look:") && o[k] && typeof o[k] === "object").map(k => Object.assign({ id: k }, o[k]))
                .sort((a, b) => (b.saved || 0) - (a.saved || 0));
        }
        onLoadFailed: page.looks = []
    }
    function saveNow(name) {
        name = name.trim();
        if (!name) { say("Give it a name first", true); return; }
        const d = {};
        for (const k of keys) d[k] = JSON.parse(JSON.stringify(Settings[k]));
        const id = "look:" + name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 40);
        const ch = {};
        ch[id] = { name: name.slice(0, 60), saved: Date.now(), desktop: d, fonts: page.fonts || {}, accentShown: String(Theme.accent), lookLabel: Theme.lookLabel };
        writer.merge(ch);
        say("Saved as " + name + ".");
    }
    function apply(l) {
        const d = l.desktop || {};
        for (const k of keys) if (d[k] !== undefined && typeof d[k] === typeof Settings[k]) Settings[k] = d[k];
        // Fonts, cursor and icons: what that look had; anything it didn't set goes back to Hearth's default.
        const f = {};
        for (const k of ["font", "fontSize", "mono", "monoSize", "antialias", "hinting", "dpi", "cursor", "cursorSize", "icons", "smallFont", "smallSize", "titleFont", "titleSize"])
            f[k] = l.fonts && l.fonts[k] !== undefined ? l.fonts[k] : null;
        lookset.run(["set"], () => {}, e => page.say("The fonts didn't change: " + e, true), JSON.stringify(f));
        say(l.name + " is back.");
    }
    function remove(l) { const ch = {}; ch[l.id] = null; writer.merge(ch); say(l.name + " deleted."); }

    SetCardGroup {
        title: "Save how it looks now"
        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            SetField { id: nameField; placeholder: "A name, like Cosy evening"; onSubmitted: t => { page.saveNow(t); nameField.text = ""; } }
            SetButton { text: "Save as my look"; primary: true; icon: "check"; onActivated: { page.saveNow(nameField.text); nameField.text = ""; } }
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "Keeps " + Theme.lookLabel + ", " + (Settings.mode === "light" ? "light" : "dark") + " mode, the accent, the wallpaper, the bar and dock, the menus, the window frames and your fonts, cursor and icons."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
    }
    SetCardGroup {
        title: page.looks.length ? "Your looks" : "No looks saved yet"
        Flow {
            Layout.fillWidth: true
            spacing: 12
            Repeater {
                model: page.looks
                delegate: SetPanel {
                    id: lk
                    required property var modelData
                    readonly property var d: modelData.desktop || {}
                    width: 280; implicitHeight: 200
                    Rectangle {
                        id: sw
                        x: 12; y: 12; width: parent.width - 24; height: 96; radius: 8; clip: true
                        color: lk.d.mode === "light" ? "#e9ecef" : "#16191d"
                        Image {
                            anchors.fill: parent
                            fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 400; opacity: 0.85
                            source: typeof lk.d.wallpaper === "string" && lk.d.wallpaper.startsWith("/") ? "file://" + lk.d.wallpaper
                                : typeof lk.d.wallpaper === "string" && lk.d.wallpaper.startsWith("builtin:") ? Qt.resolvedUrl("images/wallpapers/" + lk.d.wallpaper.slice(8) + ".jpg") : ""
                        }
                        // A little bar and window in the look's colours.
                        Rectangle { x: 10; y: parent.height - 22; width: parent.width - 20; height: 14; radius: 7; color: lk.d.mode === "light" ? "#ffffffcc" : "#000000aa" }
                        Rectangle { x: 18; y: parent.height - 19; width: 8; height: 8; radius: 4; color: lk.modelData.accentShown || Theme.accent }
                        Rectangle { x: parent.width * 0.3; y: 14; width: parent.width * 0.42; height: 46; radius: 5; color: lk.d.mode === "light" ? "#fafafa" : "#23272d"; border.color: lk.modelData.accentShown || Theme.accent; border.width: Math.max(1, (lk.d.frameTint || 0) / 30) }
                    }
                    Text { x: 14; y: sw.y + sw.height + 10; width: parent.width - 28; elide: Text.ElideRight; text: lk.modelData.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
                    Text { x: 14; y: sw.y + sw.height + 30; width: parent.width - 28; elide: Text.ElideRight; text: (lk.modelData.lookLabel || lk.d.look || "") + " · " + (lk.d.mode === "light" ? "Light" : "Dark"); color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                    Row {
                        x: 12; y: parent.height - 46; spacing: 8
                        SetButton { text: "Use this look"; primary: true; onActivated: page.apply(lk.modelData) }
                        SetButton { text: "Delete"; ghost: true; icon: "trash"; onActivated: page.remove(lk.modelData) }
                    }
                }
            }
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

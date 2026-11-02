// Hearth Settings › Boot & startup (Hearth OS 1.1): the boot splash preset (Hearth flame, Steam-style, minimal) and
// the startup animation Steam Game Mode plays. Both through /usr/libexec/nimbus/hearth-boot-look: the splash is
// system-wide (hearth-admin boot-splash: the boot menu gets plymouth.splash=, nothing is rebuilt), the animation is
// yours (copied into Steam's override folder as Steam Game Mode starts).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Boot & startup"
    property var st: null
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { look.run(["status"], r => page.st = r, e => page.say(e, true)); }
    Component.onCompleted: load()

    SetJob { id: look; helper: "/usr/libexec/nimbus/hearth-boot-look" }
    SetJob { id: admin; helper: "/usr/libexec/nimbus/hearth-admin"; admin: true }

    readonly property var splashes: st ? st.splash.presets : []
    readonly property var movies: st ? st.movie.list : []

    // ------------------------------------------------ boot splash
    SetHeader { text: "Boot splash" + (admin.what === "splash" ? " · saving…" : "") }
    Text {
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "What the screen shows while Hearth OS starts. It takes effect from the next start; if a splash can't be shown, Hearth OS starts in text as before."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.splashes
        delegate: SetRow {
            required property var modelData
            icon: "display"; chevron: false
            title: modelData.label
            sub: modelData.about
            value: page.st && page.st.splash.current === modelData.id ? "In use" : ""
            interactive: !!page.st && page.st.splash.canChange && modelData.available && admin.what === ""
            onActivated: {
                if (page.st.splash.current === modelData.id) return;
                const id = modelData.id, label = modelData.label;
                page.say("");
                admin.run(["boot-splash", id], () => { page.say(label + " shows from the next start."); page.load(); },
                          e => { page.say(e, true); page.load(); }, "", "splash");
            }
        }
    }

    // ------------------------------------------------ Steam Game Mode's startup animation
    SetHeader { text: "Startup animation (Steam Game Mode)" + (look.what === "movie" ? " · saving…" : "") }
    Text {
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "What Steam Game Mode plays when it starts. Hearth's animations are also in Steam's own Settings › Customization › Startup Movie."
            + (page.st && !page.st.movie.steamReady ? " Steam hasn't been set up yet: your choice is used once it has." : "")
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    SetRow {
        icon: "motion"; chevron: false
        title: "Steam's own"; sub: "The animation Steam picks (or the one you chose in Steam)"
        value: page.st && page.st.movie.current === "steam" ? "In use" : ""
        onActivated: page.pickMovie("steam", "Steam's own animation")
    }
    Repeater {
        model: page.movies
        delegate: SetRow {
            required property var modelData
            icon: "motion"; chevron: false
            title: modelData.label
            sub: modelData.about
            value: page.st && page.st.movie.current === modelData.id ? "In use" : ""
            onActivated: page.pickMovie(modelData.id, modelData.label)
        }
    }
    function pickMovie(id, label) {
        if (st && st.movie.current === id) return;
        say("");
        look.run(["movie", id], r => { page.say(label + (r.later ? " plays once Steam has been set up." : " plays the next time Steam Game Mode starts.")); page.load(); },
                 e => { page.say(e, true); page.load(); }, "", "movie");
    }

    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}

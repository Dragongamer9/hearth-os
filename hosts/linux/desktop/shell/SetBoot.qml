// Hearth Settings › Boot & startup (Hearth OS 1.1; previews and three more of each in the next update): the boot
// splash preset and the startup animation Steam Game Mode plays, each shown as a card with its picture. A card opens
// its preview (SetBootPreview.qml: the splash drawn from its own script, the animation as a flip book or, on request,
// the video itself); only "Use this" / "Use as a set" there chooses. Both through /usr/libexec/nimbus/hearth-boot-look:
// the splash is system-wide (hearth-admin boot-splash: the boot menu gets plymouth.splash=, nothing is rebuilt), the
// animation is yours (copied into Steam's override folder as Steam Game Mode starts).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Boot & startup"
    property var st: null
    property string message: ""
    property bool messageBad: false
    readonly property bool busy: admin.what !== "" || look.what !== ""
    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { look.run(["status"], r => page.st = r, e => page.say(e, true)); }
    Component.onCompleted: load()

    SetJob { id: look; helper: "/usr/libexec/nimbus/hearth-boot-look" }
    SetJob { id: admin; helper: "/usr/libexec/nimbus/hearth-admin"; admin: true }

    readonly property var splashes: st ? st.splash.presets : []
    readonly property var movies: st ? st.movie.list : []
    readonly property var geo: st && st.preview ? st.preview : null

    function labelOf(list, id) { const x = list.find(o => o.id === id); return x ? x.label : ""; }
    function pairAvailable(kind, id) {
        const x = (kind === "splash" ? movies : splashes).find(o => o.id === id);
        return !!x && (kind === "splash" || x.available !== false);
    }

    // ------------------------------------------------ choosing (only from the preview's buttons, or Steam's own)
    function pickSplash(id, label, then) {
        if (st && st.splash.current === id) { if (then) then(); return; }
        say("");
        admin.run(["boot-splash", id], () => { page.say(label + " shows from the next start."); page.load(); if (then) then(); },
                  e => { page.say(e, true); page.load(); }, "", "splash");
    }
    function pickMovie(id, label) {
        if (st && st.movie.current === id) return;
        say("");
        look.run(["movie", id], r => { page.say(label + (r.later ? " plays once Steam has been set up." : " plays the next time Steam Game Mode starts.")); page.load(); },
                 e => { page.say(e, true); page.load(); }, "", "movie");
    }
    // A matching pair: the splash first (it needs the system's permission); the animation only once that worked.
    function pickSet(splashId, movieId, label) {
        pickSplash(splashId, labelOf(splashes, splashId), () => {
            if (st && st.movie.current === movieId) { say(label + ": the splash and the startup animation are in use."); return; }
            look.run(["movie", movieId], r => { page.say(label + ": the splash shows from the next start, and the startup animation "
                                                          + (r.later ? "plays once Steam has been set up." : "plays the next time Steam Game Mode starts.")); page.load(); },
                     e => { page.say(e, true); page.load(); }, "", "movie");
        });
    }

    // ------------------------------------------------ boot splash
    SetHeader { text: "Boot splash" + (admin.what === "splash" ? " · saving…" : "") }
    Text {
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "What the screen shows while Hearth OS starts. Pick one to see it play; it changes only when you choose Use this, and takes effect from the next start. If a splash can't be shown, Hearth OS starts in text as before."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.splashes
        delegate: SetBootCard {
            id: splashCard
            required property var modelData
            item: modelData
            geo: page.geo
            inUse: !!page.st && page.st.splash.current === modelData.id
            pairLabel: modelData.pair ? "Goes with the " + page.labelOf(page.movies, modelData.pair) + " startup animation" : ""
            onActivated: preview.open("splash", modelData, splashCard)
        }
    }

    // ------------------------------------------------ Steam Game Mode's startup animation
    SetHeader { text: "Startup animation (Steam Game Mode)" + (look.what === "movie" ? " · saving…" : "") }
    Text {
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "What Steam Game Mode plays when it starts. Pick one to see it play; it changes only when you choose Use this. Hearth's animations are also in Steam's own Settings › Customization › Startup Movie."
            + (page.st && !page.st.movie.steamReady ? " Steam hasn't been set up yet: your choice is used once it has." : "")
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    SetRow {
        icon: "motion"; chevron: false
        title: "Steam's own"; sub: "The animation Steam picks (or the one you chose in Steam). Choose to use it."
        value: page.st && page.st.movie.current === "steam" ? "In use" : ""
        onActivated: page.pickMovie("steam", "Steam's own animation")
    }
    Repeater {
        model: page.movies
        delegate: SetBootCard {
            id: movieCard
            required property var modelData
            item: modelData
            geo: page.geo
            inUse: !!page.st && page.st.movie.current === modelData.id
            pairLabel: modelData.pair ? "Goes with the " + page.labelOf(page.splashes, modelData.pair) + " boot splash" : ""
            onActivated: preview.open("movie", modelData, movieCard)
        }
    }

    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // Over the page (not part of its list): the preview.
    SetBootPreview {
        id: preview
        parent: page
        anchors.fill: parent
        boot: page
        geo: page.geo
    }
}

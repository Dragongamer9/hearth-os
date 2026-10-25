// A game's picture: the first of `kinds` it has (cover, wide, hero, logo). Store pictures load straight from the web;
// the core's own (nimbus-art://) are asked for (App.art: the core says which file). Until one shows, a soft panel.
import QtQuick

Item {
    id: a
    property var game: null
    property var kinds: ["cover", "wide"]
    property bool load: true               // off while far out of view (big libraries)
    property int fill: Image.PreserveAspectCrop
    property int hAlign: Image.AlignHCenter
    property string url: ""
    readonly property bool ready: img.status === Image.Ready
    readonly property string wanted: {
        if (!game || !game.art) return "";
        for (const k of kinds) if (game.art[k]) return game.art[k];
        return "";
    }
    onWantedChanged: resolve()
    onLoadChanged: resolve()
    Component.onCompleted: resolve()
    function resolve() {
        if (!load || !wanted) { if (!wanted) url = ""; return; }
        const w = wanted;
        const me = a;
        App.art(w, u => { try { if (me && me.wanted === w) me.url = u; } catch (e) { /* gone meanwhile */ } });
    }
    Image {
        id: img
        anchors.fill: parent
        source: a.url
        fillMode: a.fill
        horizontalAlignment: a.hAlign
        asynchronous: true
        cache: true
        smooth: true
        sourceSize.width: Math.min(1280, Math.max(64, Math.round(a.width * 1.25)))
        opacity: status === Image.Ready ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: Theme.calm } }
    }
}

// Playing on the TV (mockup 01): what's playing now, from Plasma's own media model (MPRIS, the same as Plasma's media
// controller): the app, the title and episode, its picture, pause and skip, the time and its bar. Loaded on its own
// (Home's Loader), so a Plasma without this model only loses this card. The time moves when the Couch looks at its
// cards again (no timer: the owner's rule on lag).
// 1.16: the player comes from a small source file: Plasma's (PlasmaMedia.qml) unless the Couch's host has its own
// (Firehold on labwc: desktop/shell/FhMedia.qml, Quickshell's MPRIS). Each gives the same few names.
import QtQuick
import "fhp.js" as H

Card {
    id: mc
    property QtObject couch: null
    k: couch ? couch.k : 1
    Loader {
        id: src
        source: mc.couch && mc.couch.host && mc.couch.host.mediaFile ? mc.couch.host.mediaFile : "PlasmaMedia.qml"
    }
    readonly property var cp: src.item ? src.item.cp : null
    readonly property bool playing: !!src.item && src.item.playing
    readonly property bool has: !!src.item && src.item.has
    readonly property string title: cp ? ((cp.album && String(cp.album) !== String(cp.track) ? cp.album + " · " : "") + cp.track) : ""
    readonly property var info: has ? { app: String(cp.desktopEntry || ""), identity: String(cp.identity || ""), title: title,
                                        short: String(cp.artist || cp.album || cp.track || ""), playing: playing } : null
    onInfoChanged: if (couch) couch.player = info
    visible: has
    width: parent ? parent.width : 508
    height: 92 * k
    Connections {
        target: mc.couch
        function onCardsChanged() { if (src.item && mc.playing) src.item.updatePosition(); }
    }
    RoundClip {
        id: pic
        x: 20 * mc.k; anchors.verticalCenter: parent.verticalCenter
        width: 64 * mc.k; height: width; radius: 11 * mc.k
        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: "#2a3a5a" }
                GradientStop { position: 1; color: "#6a3a2a" }
            }
        }
        Glyph { anchors.centerIn: parent; name: "play"; size: 26 * mc.k; filled: true; visible: cover.status !== Image.Ready }
        Image {
            id: cover
            anchors.fill: parent
            visible: status === Image.Ready
            source: mc.cp && mc.cp.artUrl ? mc.cp.artUrl : ""
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            sourceSize.width: 128
        }
    }
    Column {
        anchors.left: pic.right; anchors.leftMargin: 14 * mc.k
        anchors.right: buttons.left; anchors.rightMargin: 14 * mc.k
        anchors.verticalCenter: parent.verticalCenter
        spacing: 0
        T {
            couchRef: mc.couch
            width: parent.width
            text: "Playing on the " + (mc.couch && mc.couch.screenIsTv ? "TV" : (mc.couch ? mc.couch.screenWord : "screen")) + (mc.cp && mc.cp.identity ? " · " + mc.cp.identity : "")
            color: H.P.mut; font.pixelSize: 14 * mc.k
        }
        Item { width: 1; height: 2 * mc.k }
        T { couchRef: mc.couch; width: parent.width; text: mc.title; font.pixelSize: 17 * mc.k; font.weight: Font.Bold; wrapMode: Text.WordWrap; maximumLineCount: 2 }
        Item { width: 1; height: 8 * mc.k }
        Row {
            width: parent.width
            spacing: 10 * mc.k
            visible: !!mc.cp && mc.cp.length > 0
            Bar { anchors.verticalCenter: parent.verticalCenter; width: parent.width - when.implicitWidth - 10 * mc.k; k: mc.k; value: mc.cp && mc.cp.length > 0 ? 100 * mc.cp.position / mc.cp.length : 0 }
            T { id: when; couchRef: mc.couch; text: mc.cp ? H.clock(mc.cp.position) + " / " + H.clock(mc.cp.length) : ""; color: H.P.mut; font.pixelSize: 14 * mc.k }
        }
    }
    Row {
        id: buttons
        anchors.right: parent.right; anchors.rightMargin: 20 * mc.k
        anchors.verticalCenter: parent.verticalCenter
        spacing: 6 * mc.k
        Repeater {
            model: ["pause", "next"]
            delegate: Rectangle {
                required property var modelData
                width: 50 * mc.k; height: 34 * mc.k; radius: 10 * mc.k; color: H.P.nb
                Glyph { anchors.centerIn: parent; name: modelData === "pause" ? (mc.playing ? "pause" : "play") : "next"; size: 16 * mc.k; filled: modelData !== "pause" || !mc.playing }
                MouseArea { anchors.fill: parent; onClicked: { if (!src.item) return; if (modelData === "pause") src.item.playPause(); else src.item.next(); } }
            }
        }
    }
}

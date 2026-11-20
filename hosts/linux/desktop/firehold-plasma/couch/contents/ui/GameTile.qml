// A game's tile on the Couch home (the mockups' .tile and .tname/.tsub): its art, the store tag at the bottom left,
// a badge at the top right (NEW, DEV, UPDATE READY, UPDATING 68%, CLOUD SAVES, a size), a progress bar along the
// bottom, then its name and a line under it. The controller's focus ring when it's selected.
import QtQuick
import "fhp.js" as H

Item {
    id: t
    property QtObject couch
    readonly property real k: couch.k
    property var game: null
    property real boxW: 230
    property real boxH: 118
    property string tag: game ? (game.tag || (["gog", "epic", "amazon"].indexOf(game.source) >= 0 ? game.store + " · Heroic" : game.store) || "") : ""
    property var badge: null
    property real progress: -1     // 0..100, or none
    property bool focused: false
    property string name: game ? game.title : ""
    property string sub: ""
    property real nameSize: 17
    property real subSize: 13
    property real nameGap: 9
    signal activated()
    width: boxW * k
    height: box.height + nameGap * k + nameText.height + (sub !== "" ? 2 * k + subText.height : 0)

    RoundClip {
        id: box
        width: t.boxW * t.k; height: t.boxH * t.k
        radius: 14 * t.k
        Art { anchors.fill: parent; game: t.game }
        Rectangle {
            visible: t.tag !== ""
            x: 10 * t.k; anchors.bottom: parent.bottom; anchors.bottomMargin: (t.progress >= 0 ? 14 : 9) * t.k
            width: tagText.implicitWidth + 14 * t.k; height: tagText.implicitHeight + 6 * t.k
            radius: 6 * t.k; color: "#59000000"
            T { id: tagText; couchRef: t.couch; anchors.centerIn: parent; text: t.tag; color: "#d9ffffff"; font.pixelSize: 12 * t.k; font.weight: Font.DemiBold }
        }
        Rectangle {
            visible: !!t.badge
            anchors.right: parent.right; anchors.rightMargin: 9 * t.k; y: 9 * t.k
            width: badgeText.implicitWidth + 14 * t.k; height: badgeText.implicitHeight + 6 * t.k
            radius: 6 * t.k; color: "#73000000"
            T { id: badgeText; couchRef: t.couch; anchors.centerIn: parent; text: t.badge ? t.badge.text : ""; color: t.badge ? t.badge.color : "white"; font.pixelSize: 11 * t.k; font.weight: Font.Bold }
        }
        Rectangle {
            visible: t.progress >= 0
            anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
            height: 5 * t.k; color: "#66000000"
            Rectangle { width: parent.width * Math.max(0, Math.min(100, t.progress)) / 100; height: parent.height; color: H.P.ac }
        }
    }
    Ring { visible: t.focused; k: t.k; r: 14; anchors.fill: box }
    T {
        id: nameText
        couchRef: t.couch
        anchors.top: box.bottom; anchors.topMargin: t.nameGap * t.k
        width: parent.width
        text: t.name
        color: H.P.tx
        font.pixelSize: t.nameSize * t.k; font.weight: Font.DemiBold
    }
    T {
        id: subText
        couchRef: t.couch
        visible: t.sub !== ""
        anchors.top: nameText.bottom; anchors.topMargin: 2 * t.k
        width: parent.width
        text: t.sub
        color: H.P.mut
        font.pixelSize: t.subSize * t.k
    }
    MouseArea { anchors.fill: parent; onClicked: t.activated() }
}

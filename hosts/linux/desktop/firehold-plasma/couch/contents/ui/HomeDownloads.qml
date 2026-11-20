// Downloads and updates (mockup 01): each download with its art, name, store, how much is done, the time left, its
// percent and bar; how many run and wait; Hearth OS's update when one is ready. Steam's and Heroic's own files on this
// PC (hearth-firehold-plasma home), never the Steam account. Hidden when nothing downloads and no update waits.
import QtQuick
import "fhp.js" as H

Card {
    id: c
    property QtObject couch
    k: couch.k
    readonly property var list: couch.cards.downloads || []
    readonly property bool os: couch.cards.osUpdate === true
    visible: list.length > 0 || os
    height: col.implicitHeight + 32 * k
    function gameFor(d) {
        for (const g of c.couch.library) if (g.id === d.id || g.title === d.title) return g;
        return { id: d.id, title: d.title };
    }
    Column {
        id: col
        x: 20 * c.k; y: 16 * c.k
        width: parent.width - 40 * c.k
        spacing: 12 * c.k
        Item {
            width: parent.width; height: title.implicitHeight
            T { id: title; couchRef: c.couch; text: "Downloads and updates"; font.pixelSize: 18 * c.k; font.weight: Font.Bold }
            T { couchRef: c.couch; anchors.right: parent.right; anchors.verticalCenter: title.verticalCenter; text: H.downloadsSummary(c.list); color: H.P.mut; font.pixelSize: 14 * c.k }
        }
        Repeater {
            model: c.list.filter(function (d) { return d.state === "running"; }).concat(c.list.filter(function (d) { return d.state !== "running"; })).slice(0, 2)
            delegate: Item {
                required property var modelData
                width: col.width; height: Math.max(40 * c.k, info.implicitHeight)
                RoundClip {
                    id: thumb
                    width: 56 * c.k; height: 40 * c.k; radius: 10 * c.k
                    anchors.verticalCenter: parent.verticalCenter
                    Art { anchors.fill: parent; game: c.gameFor(modelData) }
                }
                Column {
                    id: info
                    anchors.left: thumb.right; anchors.leftMargin: 12 * c.k; anchors.right: parent.right
                    spacing: 0
                    Item {
                        width: parent.width; height: name.implicitHeight
                        T { id: name; couchRef: c.couch; text: modelData.title + " · " + modelData.what; font.pixelSize: 15 * c.k; font.weight: Font.Bold; width: parent.width - 60 * c.k }
                        T { couchRef: c.couch; anchors.right: parent.right; anchors.verticalCenter: name.verticalCenter; visible: modelData.percent >= 0; text: modelData.percent + "%"; color: H.P.mut; font.pixelSize: 14 * c.k }
                    }
                    Item { width: 1; height: 2 * c.k }
                    T { couchRef: c.couch; width: parent.width; text: modelData.state === "waiting" && modelData.line.indexOf(" of ") < 0 ? modelData.line + " · waiting" : modelData.line; color: H.P.mut; font.pixelSize: 14 * c.k }
                    Item { width: 1; height: 6 * c.k }
                    Bar { width: parent.width; k: c.k; value: Math.max(0, modelData.percent) }
                }
            }
        }
        Row {
            visible: c.os
            spacing: 8 * c.k
            Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 9 * c.k; height: width; radius: width / 2; color: H.P.ok }
            T { couchRef: c.couch; text: "Hearth OS update ready · goes in when you restart"; color: "#cfc8c1"; font.pixelSize: 14 * c.k }
        }
    }
}

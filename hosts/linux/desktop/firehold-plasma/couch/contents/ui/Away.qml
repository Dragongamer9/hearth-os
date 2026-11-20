// Away (mockup firehold-plasma 06): the screen when nobody's there. A big clock and the date, cards for what keeps
// working (a download, Hearth Link serving your other devices, the home server), notifications only as a count (so
// nobody else reads them), what Away does to power and the screens, and how to come back. Drawn by the Couch on its
// screen, and over the Desk by AwayWindow.qml. Any button, key or the mouse comes back (hearth-firehold-plasma back).
import QtQuick
import "fhp.js" as H

Item {
    id: aw
    property QtObject couch
    readonly property real k: couch.k
    readonly property var cards: couch.cards
    readonly property var dls: (cards.downloads || []).filter(function (d) { return d.state === "running"; })
    readonly property var waiting: (cards.downloads || []).filter(function (d) { return d.state !== "running"; })
    readonly property var serving: (cards.devices || []).filter(function (d) { return !d.family && (d.savesAt || d.online); })[0] || null
    readonly property var family: (cards.devices || []).filter(function (d) { return d.family && d.backupAt; })[0] || null
    readonly property var server: cards.server || null
    readonly property int notes: couch.notifCount || cards.notifications || 0
    readonly property int off: typeof couch.cfg.awayScreenOff === "number" ? couch.cfg.awayScreenOff : 10
    readonly property int lock: typeof couch.cfg.awayLock === "number" ? couch.cfg.awayLock : 10
    readonly property real since: couch.myMode && couch.myMode.at ? couch.myMode.at : couch.now.getTime()
    readonly property int offIn: Math.max(0, Math.round(off - (couch.now.getTime() - since) / 60000))
    readonly property var others: couch.modes.screens.filter(function (s) { return s.name !== couch.myMode.name; })

    Rectangle { anchors.fill: parent; color: "#0a0807" }
    Rectangle {
        width: 1800 * aw.k; height: 1200 * aw.k
        x: (parent.width - width) / 2; y: -height / 2
        radius: width / 2
        scale: 1
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#002a150b" }
            GradientStop { position: 0.5; color: "#2a150b" }
            GradientStop { position: 1.0; color: "#000a0807" }
        }
        opacity: 0.9
    }
    // The mouse brings you back (not the pointer that was already resting here when Away came).
    MouseArea {
        anchors.fill: parent; hoverEnabled: aw.visible
        onPositionChanged: if (Date.now() - aw.since > 3000) aw.couch.run(["back", "mouse"])
        onClicked: aw.couch.run(["back", "mouse"])
    }
    Column {
        anchors.horizontalCenter: parent.horizontalCenter
        y: 110 * aw.k
        T { couchRef: aw.couch; anchors.horizontalCenter: parent.horizontalCenter; text: Qt.formatTime(aw.couch.now, "h:mm AP").replace(/\s*[AP]M$/i, ""); font.pixelSize: 150 * aw.k; font.weight: Font.ExtraLight; font.letterSpacing: -4.5 * aw.k }
        T { couchRef: aw.couch; anchors.horizontalCenter: parent.horizontalCenter; text: Qt.formatDate(aw.couch.now, "dddd, MMMM d"); color: H.P.mut; font.pixelSize: 26 * aw.k }
        Item { width: 1; height: 34 * aw.k }
        T { couchRef: aw.couch; anchors.horizontalCenter: parent.horizontalCenter; text: "AWAY · THIS PC KEEPS WORKING"; color: H.P.ac2; font.pixelSize: 18 * aw.k; font.weight: Font.Bold; font.letterSpacing: 2.2 * aw.k }
    }
    Row {
        id: cardRow
        x: 160 * aw.k; y: 470 * aw.k
        width: parent.width - 320 * aw.k
        spacing: 22 * aw.k
        readonly property var items: [
            aw.dls.length ? { kind: "Downloading", title: aw.dls[0].title + " " + aw.dls[0].what, bar: aw.dls[0].percent,
                              lines: [aw.dls[0].percent + "%" + (/ · (\d+ (h|min).*)$/.test(aw.dls[0].line) ? " · " + aw.dls[0].line.replace(/^.* · (\d+ (h|min)[^·]*)$/, "$1") + " left" : "") + " · ready when you're back",
                                      (aw.dls.length > 1 ? "Then: " + aw.dls[1].title + " " + aw.dls[1].what : aw.waiting.length ? "Then: " + aw.waiting[0].title + " " + aw.waiting[0].what : "")] } : null,
            aw.serving ? { kind: "Hearth Link", title: "Serving the " + aw.serving.name,
                           lines: [aw.serving.savesAt ? "Its saves came in " + H.agoShort(aw.serving.savesAt, aw.couch.now.getTime()).toLowerCase() : "Connected",
                                   aw.family ? aw.family.name + " backed up at " + Qt.formatTime(new Date(aw.family.backupAt), "h:mm AP") : ""] } : null,
            aw.server ? { kind: "Home server", title: aw.server.title || "Home server on", lines: aw.server.lines || [] } : null,
            { kind: "Waiting for you", title: aw.notes + (aw.notes === 1 ? " notification" : " notifications"), lines: ["Hidden here so nobody else reads them"] }
        ].filter(Boolean)
        Repeater {
            model: cardRow.items
            delegate: Card {
                required property var modelData
                k: aw.k
                width: (cardRow.width - (cardRow.items.length - 1) * cardRow.spacing) / cardRow.items.length
                height: 163 * aw.k
                Column {
                    x: 24 * aw.k; y: 22 * aw.k
                    width: parent.width - 48 * aw.k
                    spacing: 0
                    T { couchRef: aw.couch; text: modelData.kind; color: H.P.mut; font.pixelSize: 14 * aw.k }
                    Item { width: 1; height: 6 * aw.k }
                    T { couchRef: aw.couch; width: parent.width; text: modelData.title; font.pixelSize: 20 * aw.k; font.weight: Font.Bold }
                    Item { width: 1; height: (modelData.bar !== undefined ? 14 : 10) * aw.k }
                    Bar { visible: modelData.bar !== undefined; width: parent.width; k: aw.k; value: modelData.bar || 0 }
                    Item { width: 1; height: 8 * aw.k; visible: modelData.bar !== undefined }
                    Repeater {
                        model: modelData.lines.filter(Boolean)
                        delegate: T { required property var modelData; required property int index; couchRef: aw.couch; text: modelData; color: H.P.mut; font.pixelSize: 14 * aw.k; topPadding: index > 0 ? 4 * aw.k : 0 }
                    }
                }
            }
        }
    }
    T {
        couchRef: aw.couch
        anchors.horizontalCenter: parent.horizontalCenter
        y: 720 * aw.k
        text: ["Fans quiet: Power saver until you're back",
               (aw.couch.screenIsTv ? "TV" : "this screen") + (aw.off ? (aw.offIn > 0 ? " turns off in " + aw.offIn + " min" : " is off") : " stays on"),
               aw.others.length ? (aw.others.every(function (s) { return s.mode === "away"; }) ? "the " + (aw.others.length > 1 ? "other screens are" : "monitor is") + " Away too"
                                  : "the other screen" + (aw.others.length > 1 ? "s are" : " is") + " still on") : ""].filter(Boolean).join(" · ")
        color: H.P.mut; font.pixelSize: 17 * aw.k
    }
    Row {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom; anchors.bottomMargin: 60 * aw.k
        spacing: 40 * aw.k
        Row {
            spacing: 10 * aw.k
            Rectangle { width: 32 * aw.k; height: width; radius: width / 2; color: H.keyColor("A"); anchors.verticalCenter: parent.verticalCenter
                T { couchRef: aw.couch; anchors.centerIn: parent; text: "A"; color: "#111111"; font.pixelSize: 14 * aw.k; font.weight: Font.ExtraBold } }
            T { couchRef: aw.couch; anchors.verticalCenter: parent.verticalCenter; text: "Back to the Couch"; font.pixelSize: 21 * aw.k }
        }
        Row {
            spacing: 10 * aw.k
            Rectangle { width: 32 * aw.k; height: width; radius: width / 2; color: "#bfb6ad"; anchors.verticalCenter: parent.verticalCenter
                Glyph { anchors.centerIn: parent; name: "mouse"; size: 17 * aw.k; color: "#111111" } }
            T { couchRef: aw.couch; anchors.verticalCenter: parent.verticalCenter; text: "Move the mouse: back to the Desk"; font.pixelSize: 21 * aw.k }
        }
        Row {
            visible: aw.lock > 0
            spacing: 10 * aw.k
            Rectangle { width: 32 * aw.k; height: width; radius: width / 2; color: "#bfb6ad"; anchors.verticalCenter: parent.verticalCenter
                Glyph { anchors.centerIn: parent; name: "lock"; size: 17 * aw.k; color: "#111111" } }
            T { couchRef: aw.couch; anchors.verticalCenter: parent.verticalCenter; text: "Asks for your PIN after " + aw.lock + " min"; font.pixelSize: 21 * aw.k }
        }
    }
}

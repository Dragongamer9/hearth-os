// Everything open (mockup 4): games and windows in one list, a live picture of each, three a row. Pick one to jump
// to it, in Couch or Desk.
import QtQuick
import "firehold.js" as F

Item {
    id: sw
    property QtObject couch
    readonly property real k: couch.k
    readonly property var items: couch.openList
    property int current: 0
    onItemsChanged: if (current >= items.length) current = Math.max(0, items.length - 1)
    function key(kk) {
        const n = items.length, it = items[current];
        if (kk === Qt.Key_Left) current = Math.max(0, current - 1);
        else if (kk === Qt.Key_Right) current = Math.min(n - 1, current + 1);
        else if (kk === Qt.Key_Up) current = Math.max(0, current - 3);
        else if (kk === Qt.Key_Down) current = Math.min(n - 1, current + 3);
        else if (kk === Qt.Key_Return || kk === Qt.Key_Enter) { if (it) couch.goTo(it.win, false); }
        else if (kk === Qt.Key_X) { if (it) couch.closeWin(it.win); }
        else if (kk === Qt.Key_Y) { if (it) couch.toggleFull(it.win); }
        else if (kk === Qt.Key_Escape || kk === Qt.Key_Backspace) couch.page = "tabs";
        else return false;
        grid.positionViewAtIndex(current, GridView.Contain);
        return true;
    }

    T { couchRef: sw.couch; x: 96 * sw.k; y: 62 * sw.k; text: "Open now"; font.pixelSize: 46 * sw.k; font.weight: Font.Bold }
    T {
        couchRef: sw.couch
        x: 96 * sw.k; y: 120 * sw.k
        text: "Games and windows are one list. Pick one to jump to it, in Couch or Desk."
        color: "#cdbfb5"; font.pixelSize: 20 * sw.k
    }
    T {
        couchRef: sw.couch
        visible: sw.items.length === 0
        x: 96 * sw.k; y: 220 * sw.k
        text: "Nothing is open. Start a game or an app from Home."
        color: sw.couch.pal.dim; font.pixelSize: 22 * sw.k
    }
    GridView {
        id: grid
        x: 96 * sw.k - 10 * sw.k; y: 200 * sw.k
        width: 3 * cellWidth; height: parent.height - y - 100 * sw.k
        cellWidth: 590 * sw.k; cellHeight: 376 * sw.k
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        model: sw.items
        highlightMoveDuration: 0
        cacheBuffer: 0
        delegate: Item {
            id: card
            required property var modelData
            required property int index
            readonly property bool on: index === sw.current
            readonly property bool game: modelData.kind === "game"
            width: 569 * sw.k; height: 366 * sw.k
            Rectangle {
                visible: card.on
                x: 2 * sw.k; y: 2 * sw.k
                width: 565 * sw.k; height: 316 * sw.k; radius: 24 * sw.k
                color: Qt.rgba(1, 138 / 255, 61 / 255, 0.22)
            }
            Rectangle {
                id: pic
                x: 10 * sw.k; y: 10 * sw.k
                width: 549 * sw.k; height: 300 * sw.k; radius: 18 * sw.k
                clip: true
                gradient: Gradient {
                    orientation: Gradient.Horizontal
                    GradientStop { position: 0.0; color: card.game ? F.GAME_GRADIENT[0] : Qt.darker(F.toneFor(card.modelData.name), 1.35) }
                    GradientStop { position: 1.0; color: card.game ? F.GAME_GRADIENT[2] : Qt.darker(F.toneFor(card.modelData.name), 1.2) }
                }
                Loader {
                    anchors.fill: parent
                    anchors.margins: 3 * sw.k
                    active: sw.visible && card.modelData.win.uuid !== ""
                    source: "Thumb.qml"
                    onLoaded: item.uuid = card.modelData.win.uuid
                }
                Rectangle {
                    anchors.right: parent.right; anchors.rightMargin: 14 * sw.k
                    y: 14 * sw.k
                    height: 30 * sw.k; radius: 8 * sw.k
                    width: tagText.implicitWidth + 24 * sw.k
                    color: "#e0140e0b"
                    T { id: tagText; couchRef: sw.couch; anchors.centerIn: parent; text: card.game ? "GAME" : "WINDOW"; color: card.game ? sw.couch.pal.gold : "#e6d9cf"; font.pixelSize: 14 * sw.k; font.weight: Font.Bold; font.letterSpacing: 0.6 * sw.k }
                }
                Rectangle {
                    anchors.fill: parent; radius: parent.radius
                    color: "transparent"
                    border.width: card.on ? 3 * sw.k : 1
                    border.color: card.on ? sw.couch.pal.accent : "#33ffffff"
                }
            }
            Row {
                x: 10 * sw.k; anchors.top: pic.bottom; anchors.topMargin: 12 * sw.k
                spacing: 10 * sw.k
                T { id: nameText; couchRef: sw.couch; text: card.modelData.name; font.pixelSize: 21 * sw.k; font.weight: Font.Bold; width: Math.min(implicitWidth, 360 * sw.k) }
                T { couchRef: sw.couch; y: nameText.baselineOffset - baselineOffset; text: card.modelData.sub || ""; color: sw.couch.pal.dim; font.pixelSize: 16 * sw.k; width: Math.min(implicitWidth, 180 * sw.k) }
            }
            MouseArea { anchors.fill: parent; onClicked: { sw.current = card.index; sw.couch.goTo(card.modelData.win, false); } }
        }
    }
    Row {
        anchors.right: parent.right; anchors.rightMargin: 96 * sw.k
        y: 1032 * sw.k
        spacing: 30 * sw.k
        Hint { couch: sw.couch; key: "A"; label: "Go to it"; tint: sw.couch.pal.green }
        Hint { couch: sw.couch; key: "X"; label: "Close"; tint: sw.couch.pal.blue }
        Hint { couch: sw.couch; key: "Y"; label: "Window ⇄ full screen"; tint: sw.couch.pal.yellow }
        Hint { couch: sw.couch; key: "B"; label: "Back"; tint: sw.couch.pal.red }
    }
}

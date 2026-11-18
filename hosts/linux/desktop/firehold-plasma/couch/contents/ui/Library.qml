// Library (mockup 5): every game from every store, the search box with the real count, the filter chips with their
// counts, tall tiles six a row with the store's tag and the game's state.
import QtQuick
import "firehold.js" as F

Item {
    id: lib
    property QtObject couch
    readonly property real k: couch.k
    property string filter: "all"
    property string query: ""
    readonly property var chips: F.chips(couch.counts)
    readonly property var games: F.filterGames(couch.library, filter, query, couch.playing)
    property int current: 0
    onGamesChanged: if (current >= games.length) current = Math.max(0, games.length - 1)

    function key(kk) {
        if (search.activeFocus) return false;
        const n = games.length;
        if (kk === Qt.Key_Left) current = Math.max(0, current - 1);
        else if (kk === Qt.Key_Right) current = Math.min(n - 1, current + 1);
        else if (kk === Qt.Key_Up) current = Math.max(0, current - grid.cols);
        else if (kk === Qt.Key_Down) current = Math.min(n - 1, current + grid.cols);
        else if (kk === Qt.Key_Return || kk === Qt.Key_Enter) couch.play(games[current], false);
        else if (kk === Qt.Key_X) { if (games[current]) couch.run(["page", games[current].id]); }
        else if (kk === Qt.Key_Y) search.forceActiveFocus();
        else if (kk === Qt.Key_Comma || kk === Qt.Key_Period) {
            const keys = chips.map(function (c) { return c.key; });
            const i = Math.max(0, Math.min(keys.length - 1, keys.indexOf(filter) + (kk === Qt.Key_Comma ? -1 : 1)));
            filter = keys[i];
        } else return false;
        grid.positionViewAtIndex(current, GridView.Contain);
        return true;
    }

    // ---------------------------------------------------------------- search
    Rectangle {
        x: parent.width - 570 * lib.k; y: 40 * lib.k
        width: 498 * lib.k; height: 54 * lib.k; radius: height / 2
        color: "#1f1714"
        border.width: search.activeFocus ? 2 * lib.k : 1
        border.color: search.activeFocus ? lib.couch.pal.accent : lib.couch.pal.line
        Rectangle {
            x: 26 * lib.k; anchors.verticalCenter: parent.verticalCenter
            width: 14 * lib.k; height: width; radius: width / 2
            color: "transparent"; border.width: 2 * lib.k; border.color: lib.couch.pal.dim
        }
        TextInput {
            id: search
            x: 54 * lib.k; anchors.verticalCenter: parent.verticalCenter
            width: parent.width - 80 * lib.k
            color: lib.couch.pal.text
            font.family: lib.couch.fontName
            font.pixelSize: 20 * lib.k
            clip: true
            onTextChanged: { lib.query = text; lib.current = 0; }
            Keys.onEscapePressed: lib.couch.forceActiveFocus()
            Keys.onReturnPressed: lib.couch.forceActiveFocus()
            Keys.onDownPressed: lib.couch.forceActiveFocus()
        }
        T {
            couchRef: lib.couch
            anchors.verticalCenter: parent.verticalCenter; x: search.x
            visible: search.text === ""
            text: "Search " + (lib.couch.counts.all || lib.couch.library.length) + " games"
            color: lib.couch.pal.dim
            font.pixelSize: 20 * lib.k
        }
        MouseArea { anchors.fill: parent; onClicked: search.forceActiveFocus() }
    }

    // ---------------------------------------------------------------- the chips
    Row {
        x: 72 * lib.k; y: 128 * lib.k
        spacing: 12 * lib.k
        Repeater {
            model: lib.chips
            delegate: Rectangle {
                required property var modelData
                readonly property bool on: lib.filter === modelData.key
                height: 44 * lib.k; radius: height / 2
                width: chipRow.implicitWidth + 40 * lib.k
                color: on ? lib.couch.pal.accent : "#241b17"
                Row {
                    id: chipRow
                    anchors.centerIn: parent
                    spacing: 6 * lib.k
                    T { couchRef: lib.couch; text: modelData.label; font.pixelSize: 18 * lib.k; font.weight: Font.Bold; color: parent.parent.on ? "#2a1206" : lib.couch.pal.text }
                    T { couchRef: lib.couch; text: String(modelData.n); font.pixelSize: 18 * lib.k; color: parent.parent.on ? "#5a2a0c" : lib.couch.pal.dim }
                }
                MouseArea { anchors.fill: parent; onClicked: { lib.filter = modelData.key; lib.current = 0; } }
            }
        }
    }

    // ---------------------------------------------------------------- the tiles
    GridView {
        id: grid
        readonly property int cols: 6
        x: 72 * lib.k; y: 214 * lib.k
        width: cols * cellWidth; height: parent.height - y - 90 * lib.k
        cellWidth: 300 * lib.k; cellHeight: 384 * lib.k
        clip: true
        interactive: true
        boundsBehavior: Flickable.StopAtBounds
        model: lib.games
        currentIndex: lib.current
        highlightMoveDuration: 0
        cacheBuffer: 0
        delegate: Item {
            id: cell
            required property var modelData
            required property int index
            width: 276 * lib.k; height: 370 * lib.k
            readonly property bool ring: index === lib.current
            Rectangle {
                id: tile
                width: parent.width; height: 330 * lib.k; radius: 14 * lib.k
                color: F.toneFor(cell.modelData.id)
                clip: true
                border.width: cell.ring ? 3 * lib.k : 0
                border.color: lib.couch.pal.accent
                readonly property string art: cell.modelData.cover || ""
                Image {
                    anchors.fill: parent; anchors.margins: parent.border.width
                    visible: parent.art !== ""
                    source: parent.art === "" ? "" : (parent.art.indexOf("/") === 0 ? "file://" + parent.art : parent.art)
                    fillMode: Image.PreserveAspectCrop; asynchronous: true
                    sourceSize.width: 400; sourceSize.height: 480
                }
                Rectangle {
                    x: 14 * lib.k; y: 14 * lib.k
                    height: 26 * lib.k; radius: 6 * lib.k
                    width: tag.implicitWidth + 20 * lib.k
                    color: "#d0140e0b"
                    T { id: tag; couchRef: lib.couch; anchors.centerIn: parent; text: cell.modelData.store || ""; font.pixelSize: 14 * lib.k; font.weight: Font.Bold }
                }
                T {
                    couchRef: lib.couch
                    x: 15 * lib.k; anchors.bottom: parent.bottom; anchors.bottomMargin: 14 * lib.k
                    text: F.status(cell.modelData, lib.couch.playing)
                    color: "#e2d6cc"; font.pixelSize: 15 * lib.k
                }
            }
            T {
                couchRef: lib.couch
                anchors.top: tile.bottom; anchors.topMargin: 12 * lib.k
                width: parent.width
                text: cell.modelData.title
                font.pixelSize: 19 * lib.k; font.weight: Font.Bold
            }
            MouseArea { anchors.fill: parent; onClicked: { lib.current = cell.index; lib.couch.play(cell.modelData, false); } }
        }
    }
    T {
        couchRef: lib.couch
        visible: lib.games.length === 0
        x: 72 * lib.k; y: 240 * lib.k
        text: lib.couch.libraryLoaded ? "No games here" : "Reading your games…"
        color: lib.couch.pal.dim; font.pixelSize: 22 * lib.k
    }

    Row {
        anchors.right: parent.right; anchors.rightMargin: 72 * lib.k
        y: 1034 * lib.k
        spacing: 30 * lib.k
        T { couchRef: lib.couch; text: "A Play"; font.pixelSize: 19 * lib.k }
        T { couchRef: lib.couch; text: "X Game options"; font.pixelSize: 19 * lib.k }
        T { couchRef: lib.couch; text: "Y Search"; font.pixelSize: 19 * lib.k }
        T { couchRef: lib.couch; text: "LT RT Filters"; font.pixelSize: 19 * lib.k }
    }
}
